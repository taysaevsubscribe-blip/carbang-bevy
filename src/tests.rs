//! Запуск из командной строки: быстрый старт, автотесты физики и боя, скриншоты.

use crate::combat::Combat;
use crate::config::*;
use crate::data::*;
use crate::game::*;
use crate::maps::{builders, new_map_data};
use crate::vehicle::*;
use crate::weapons::Weapons;
use crate::Cli;
use avian3d::prelude::*;
use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

/// Тест на бесконечной плоскости вместо карты.
#[derive(Resource)]
pub struct FlatTest;

/// Витрина всех машин для скриншота.
#[derive(Resource)]
pub struct Gallery;

/// Лобовое столкновение двух машин для скриншота повреждений.
#[derive(Resource)]
pub struct CrashScene;

/// Тест старта: все машины, честный отсчёт, разные варианты ввода на старте.
#[derive(Resource)]
pub struct StartTest;

/// Тест тарана: пары машин, лобовой удар на заданной скорости.
#[derive(Resource)]
pub struct RamTest;

/// Сценарий старта для машины в тесте старта.
#[derive(Component)]
struct StartProbe {
    pattern: u8,
    origin: Vec3,
    fwd: Vec3,
    min_long: f32,
    max_countdown_move: f32,
    reverse_at: Option<f32>,
    long_at_3s: f32,
}

/// Участник теста тарана.
#[derive(Component)]
struct RamProbe {
    pair: usize,
    attacker: bool,
    speed_kmh: f32,
    hp0: f32,
    armor0: f32,
    impact_t: Option<f32>,
    impact_speed: f32,
}

#[derive(Resource, Default)]
struct BattleStats {
    steps: u64,
    wall: f64,
    worst_step: f64,
    last_report: f32,
    last_dbg: f32,
    dist: Vec<f32>,
    stuck: Vec<f32>,
    last_pos: Vec<Vec3>,
    /// последнее известное состояние матча (для итога после выхода в результаты)
    snap: Option<(f32, usize, usize, bool, f32, u32)>,
}

#[derive(Component, Default)]
struct TestDriver {
    phase: u32,
    t: f32,
    total: f32,
    t100: Option<f32>,
    top: f32,
    brake_dist: f32,
    brake_from: Vec3,
    lat15: Vec<f32>,
    lat25: Vec<f32>,
    min_up: f32,
    flipped: bool,
    spun: bool,
}

pub struct TestsPlugin;

impl Plugin for TestsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, launch)
            .add_systems(OnEnter(AppState::Race), flat_setup.run_if(resource_exists::<FlatTest>).after(crate::race::setup_race))
            .add_systems(FixedUpdate, test_driver.before(SimSet::Vehicle).run_if(resource_exists::<FlatTest>).run_if(not(resource_exists::<Gallery>)).run_if(not(resource_exists::<CrashScene>)).run_if(not(resource_exists::<StartTest>)).run_if(not(resource_exists::<RamTest>)).run_if(in_state(AppState::Race)))
            .add_systems(FixedUpdate, crash_driver.before(SimSet::Vehicle).run_if(resource_exists::<CrashScene>).run_if(in_state(AppState::Race)))
            .add_systems(FixedUpdate, start_driver.before(SimSet::Vehicle).run_if(resource_exists::<StartTest>).run_if(in_state(AppState::Race)))
            .add_systems(FixedUpdate, ram_driver.before(SimSet::Vehicle).run_if(resource_exists::<RamTest>).run_if(in_state(AppState::Race)))
            .add_systems(PostUpdate, crash_camera.after(bevy::transform::TransformSystems::Propagate).run_if(resource_exists::<CrashScene>))
            .add_systems(PostUpdate, gallery_camera.after(bevy::transform::TransformSystems::Propagate).run_if(resource_exists::<Gallery>))
            .add_systems(PostUpdate, env_camera.after(bevy::transform::TransformSystems::Propagate))
            .add_systems(Update, (battle_monitor.run_if(resource_exists::<BattleStats>), print_feed.run_if(resource_exists::<BattleStats>), screenshot_timer))
            .add_systems(Update, net_test.run_if(resource_exists::<NetTest>))
            .add_systems(PreUpdate, ui_flow.after(bevy::ui::UiSystems::Focus).run_if(resource_exists::<UiFlow>));
    }
}

pub fn parse_map(s: &str) -> Option<MapId> {
    Some(match s.to_lowercase().as_str() {
        "port" => MapId::Port,
        "quarry" => MapId::Quarry,
        "airfield" => MapId::Airfield,
        "downtown" => MapId::Downtown,
        "coliseum" => MapId::Coliseum,
        "junkyard" => MapId::Junkyard,
        _ => return None,
    })
}

pub fn parse_mode(s: &str) -> Option<GameMode> {
    Some(match s.to_lowercase().as_str() {
        "carnage" => GameMode::Carnage,
        "derby" => GameMode::Derby,
        "deathmatch" | "dm" => GameMode::Deathmatch,
        "pedhunt" | "peds" => GameMode::PedHunt,
        _ => return None,
    })
}

/// Случайная, но разумная сборка бота: машина, оружие, апгрейды по сложности.
pub fn bot_config(seed: u64, difficulty: f32, max_level: u32) -> CarConfig {
    use rand::{Rng, SeedableRng};
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed ^ 0xB07);
    let pool: Vec<&CarDef> = CARS.iter().filter(|c| c.unlock_level <= max_level.max(1) + 2).collect();
    let def = pool[rng.gen_range(0..pool.len())];
    let mut cfg = CarConfig::new_default(def.id);
    let lvl = (difficulty * 4.0).round() as u8;
    for l in cfg.levels.iter_mut() {
        *l = (lvl as i32 + rng.gen_range(-1..=1)).clamp(0, MAX_UPGRADE as i32) as u8;
    }
    cfg.loadout.clear();
    for m in def.slots.iter() {
        let options: Vec<&WeaponDef> = WEAPONS.iter().filter(|w| w.mounts.contains(m)).collect();
        if !options.is_empty() && rng.gen::<f32>() < 0.85 {
            let w = options[rng.gen_range(0..options.len())];
            cfg.loadout.push((*m, w.kind));
            cfg.weapon_levels.push((w.kind, (difficulty * 3.0).round().min(MAX_WEAPON_LEVEL as f32) as u8));
        }
    }
    cfg.perks.clear();
    let nperks = (1.0 + difficulty * 2.0).round() as usize;
    while cfg.perks.len() < nperks.min(PERK_SLOTS) {
        let p = PERKS[rng.gen_range(0..PERKS.len())].kind;
        if !cfg.perks.contains(&p) {
            cfg.perks.push(p);
        }
    }
    cfg.color = [rng.gen_range(0.1..1.0), rng.gen_range(0.1..1.0), rng.gen_range(0.1..1.0)];
    // треть ботов — со скинами
    if rng.gen::<f32>() < 0.33 {
        cfg.skin = rng.gen_range(1..=SKINS.len() as u8);
    }
    cfg.sanitize();
    cfg
}

pub fn add_bots(cfg: &mut MatchConfig, count: usize, max_level: u32) {
    use rand::{Rng, SeedableRng};
    let mut rng = rand::rngs::StdRng::seed_from_u64(cfg.seed ^ 0xFACE);
    let mut names: Vec<&str> = BOT_NAMES.to_vec();
    for k in 0..count {
        let i = rng.gen_range(0..names.len().max(1));
        let name = if names.is_empty() { format!("Бот {}", k + 1) } else { names.remove(i).to_string() };
        let seed = cfg.seed.wrapping_mul(31).wrapping_add(k as u64 * 7919 + 13);
        cfg.players.push(PlayerSlot { name, config: bot_config(seed, cfg.difficulty, max_level), controller: Controller::Bot, bot_seed: seed });
    }
}

fn launch(
    mut commands: Commands,
    mut cli: ResMut<Cli>,
    mut cfg: ResMut<MatchConfig>,
    profile: Res<Profile>,
    mut next: ResMut<NextState<AppState>>,
    mut net: MessageWriter<crate::net::NetCommand>,
) {
    let map = cli.map.as_deref().and_then(parse_map).unwrap_or(MapId::Port);
    let mode = cli.mode.as_deref().and_then(parse_mode).unwrap_or(GameMode::Carnage);
    match cli.test.as_deref() {
        Some("physics") => {
            commands.insert_resource(FlatTest);
            *cfg = MatchConfig { map: MapId::Airfield, mode: GameMode::Derby, peds: 0, time_limit: 9999.0, ..default() };
            for def in CARS.iter() {
                if cli.car.as_deref().map(|c| c == def.id).unwrap_or(true) {
                    let mut c = CarConfig::new_default(def.id);
                    c.loadout.clear();
                    cfg.players.push(PlayerSlot { name: def.name.into(), config: c, controller: Controller::Local, bot_seed: 0 });
                }
            }
            next.set(AppState::Race);
        }
        Some("start") => {
            commands.insert_resource(FlatTest);
            commands.insert_resource(StartTest);
            *cfg = MatchConfig { map: MapId::Airfield, mode: GameMode::Derby, peds: 0, time_limit: 9999.0, ..default() };
            for pattern in 0..4u8 {
                for def in CARS.iter() {
                    let mut c = CarConfig::new_default(def.id);
                    c.loadout.clear();
                    c.perks.clear();
                    cfg.players.push(PlayerSlot { name: format!("{}#{}", def.id, pattern), config: c, controller: Controller::Local, bot_seed: pattern as u64 });
                }
            }
            next.set(AppState::Race);
        }
        Some("ram") => {
            commands.insert_resource(FlatTest);
            commands.insert_resource(RamTest);
            *cfg = MatchConfig { map: MapId::Airfield, mode: GameMode::Derby, peds: 0, time_limit: 9999.0, ..default() };
            for (k, kmh) in [40.0f32, 80.0, 120.0, 140.0].iter().enumerate() {
                for def in CARS.iter().filter(|d| cli.car.as_deref().map(|c| c == d.id).unwrap_or(true)) {
                    for role in 0..2u64 {
                        let mut c = CarConfig::new_default(def.id);
                        c.loadout.clear();
                        c.perks.clear();
                        cfg.players.push(PlayerSlot { name: format!("{}@{}", def.id, kmh), config: c, controller: Controller::Local, bot_seed: (k as u64) * 2 + role });
                    }
                }
            }
            next.set(AppState::Race);
        }
        Some("nethost") => {
            commands.insert_resource(NetTest { host: true, ..default() });
            net.write(crate::net::NetCommand::Host);
            next.set(AppState::Menu);
        }
        Some("netclient") => {
            commands.insert_resource(NetTest { host: false, ..default() });
            net.write(crate::net::NetCommand::Join(cli.connect.clone().unwrap_or_else(|| "127.0.0.1".into())));
            next.set(AppState::Menu);
        }
        Some("gallery") => {
            commands.insert_resource(FlatTest);
            commands.insert_resource(Gallery);
            *cfg = MatchConfig { map: cli.map.as_deref().and_then(parse_map).unwrap_or(MapId::Airfield), mode: GameMode::Derby, peds: 0, time_limit: 9999.0, ..default() };
            // --car ferret,golf — только эти машины, в одном ряду
            let only: Option<Vec<String>> = cli.car.as_ref().map(|c| c.split(',').map(|s| s.to_string()).collect());
            for def in CARS.iter().filter(|d| only.as_ref().map(|o| o.iter().any(|x| x == d.id)).unwrap_or(true)) {
                let mut c = CarConfig::new_default(def.id);
                for m in def.slots.iter() {
                    if let Some(w) = WEAPONS.iter().find(|w| w.mounts.contains(m) && !c.loadout.iter().any(|l| l.1 == w.kind)) {
                        c.loadout.push((*m, w.kind));
                    }
                }
                cfg.players.push(PlayerSlot { name: def.name.into(), config: c, controller: Controller::Local, bot_seed: 0 });
            }
            next.set(AppState::Race);
        }
        Some("uiflow") => {
            commands.insert_resource(UiFlow::default());
            next.set(AppState::Menu);
        }
        Some("crash") => {
            commands.insert_resource(FlatTest);
            commands.insert_resource(CrashScene);
            *cfg = MatchConfig { map: MapId::Airfield, mode: GameMode::Derby, peds: 0, time_limit: 9999.0, ..default() };
            for (id, color) in [("ferret", [0.75, 0.2, 0.12]), ("hearse", [0.35, 0.42, 0.45])] {
                let mut c = CarConfig::new_default(id);
                c.color = color;
                cfg.players.push(PlayerSlot { name: id.into(), config: c, controller: Controller::Local, bot_seed: 0 });
            }
            next.set(AppState::Race);
        }
        Some("match") => {
            // Игрок на автопилоте со средним навыком против 11 ботов: выживаемость и длительность
            let seed = std::env::var("CARBANG_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(1234u64);
            *cfg = MatchConfig { map, mode, seed, difficulty: 0.6, ..default() };
            cfg.apply_map_defaults();
            // «средний игрок»: машина и прокачка того же уровня, что у ботов
            let mut me = bot_config(seed ^ 0xA11CE, 0.6, 3);
            if let Some(car) = cli.car.clone() {
                me = CarConfig::new_default(&car);
            }
            cfg.players.push(PlayerSlot { name: "Игрок".into(), config: me, controller: Controller::Local, bot_seed: seed ^ 0x51 });
            add_bots(&mut cfg, cli.bots, 3);
            cli.autopilot = true;
            commands.insert_resource(BattleStats::default());
            commands.insert_resource(crate::combat::DamageLog::default());
            next.set(AppState::Race);
        }
        Some("battle") => {
            *cfg = MatchConfig { map, mode, seed: 1234, difficulty: 0.6, ..default() };
            cfg.apply_map_defaults();
            add_bots(&mut cfg, cli.bots.max(2), 10);
            commands.insert_resource(BattleStats::default());
            next.set(AppState::Race);
        }
        _ if cli.quick => {
            let seed = std::env::var("CARBANG_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(rand::random::<u64>() % 100000);
            *cfg = MatchConfig { map, mode, seed, difficulty: 0.5, ..default() };
            cfg.apply_map_defaults();
            if let Some(tl) = std::env::var("CARBANG_TIME_LIMIT").ok().and_then(|v| v.parse().ok()) {
                cfg.time_limit = tl;
            }
            let mut me = profile.selected_config();
            if let Some(c) = cli.car.as_deref() {
                if CARS.iter().any(|d| d.id == c) {
                    me = profile.config_for(c);
                    me.car_id = c.to_string();
                }
            }
            cfg.players.push(PlayerSlot { name: profile.name.clone(), config: me, controller: Controller::Local, bot_seed: 0 });
            add_bots(&mut cfg, cli.bots, profile.level());
            next.set(AppState::Race);
        }
        _ => {
            if cli.host {
                net.write(crate::net::NetCommand::Host);
            } else if let Some(ip) = cli.connect.clone() {
                net.write(crate::net::NetCommand::Join(ip));
            }
            next.set(AppState::Menu);
        }
    }
}

// ============================================================ тест физики

fn flat_setup(
    mut commands: Commands,
    cars: Query<(Entity, &Car)>,
    mut local: ResMut<LocalPlayer>,
    gallery: Option<Res<Gallery>>,
    crash: Option<Res<CrashScene>>,
    start_t: Option<Res<StartTest>>,
    ram_t: Option<Res<RamTest>>,
    matlib: Option<Res<crate::maps::MatLib>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(new_map_data(MapId::Airfield, Vec2::splat(400.0), builders::theme_for(MapId::Airfield)));
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(6000.0, 1.0, 8000.0),
        Transform::from_xyz(0.0, -0.5, 0.0),
        CollisionLayers::new(Layer::World, [Layer::Car, Layer::Prop, Layer::Corpse]),
        DespawnOnExit(AppState::Race),
    ));
    local.car = None;
    let mut i = 0;
    for (e, car) in cars.iter() {
        let stats = CarStats::compute(&car.cfg);
        if start_t.is_some() {
            // 12 машин в ряд, четыре ряда по вариантам ввода; все смотрят вдоль -Z
            let (col, row) = (i % 12, i / 12);
            let p = Vec3::new(col as f32 * 120.0 - 660.0, stats.ride_height + 0.05, 3300.0 + row as f32 * 150.0);
            commands.entity(e).insert((
                Transform::from_translation(p),
                Position(p),
                Rotation(Quat::IDENTITY),
                StartProbe { pattern: row as u8, origin: p, fwd: Vec3::NEG_Z, min_long: 0.0, max_countdown_move: 0.0, reverse_at: None, long_at_3s: 0.0 },
            ));
        } else if ram_t.is_some() {
            // пары: атакующий в 70 м, жертва стоит мордой к нему; пара = (скорость, модель)
            let k = (car.bot_seed / 2) as usize;
            let attacker = car.bot_seed % 2 == 0;
            let kmh = [40.0f32, 80.0, 120.0, 140.0][k.min(3)];
            let model = CARS.iter().position(|d| d.id == car.def().id).unwrap_or(0);
            let pair = k * CARS.len() + model;
            let (col, row) = (model, k);
            let base = Vec3::new(col as f32 * 120.0 - 660.0, stats.ride_height + 0.05, 2800.0 + row as f32 * 280.0);
            let (p, r) = if attacker { (base + Vec3::Z * 70.0, Quat::IDENTITY) } else { (base, Quat::from_rotation_y(std::f32::consts::PI)) };
            commands.entity(e).insert((
                Transform::from_translation(p).with_rotation(r),
                Position(p),
                Rotation(r),
                RamProbe { pair, attacker, speed_kmh: kmh, hp0: 100.0, armor0: 0.0, impact_t: None, impact_speed: 0.0 },
            ));
        } else if crash.is_some() {
            let side = if i == 0 { -1.0 } else { 1.0 };
            let p = Vec3::new(side * 24.0, stats.ride_height + 0.05, 3500.0 + side * 0.4);
            let r = Quat::from_rotation_y(side * std::f32::consts::FRAC_PI_2);
            commands.entity(e).insert((Transform::from_translation(p).with_rotation(r), Position(p), Rotation(r)));
        } else if gallery.is_some() {
            let (col, row) = (i % 4, i / 4);
            let p = Vec3::new(col as f32 * 9.0 - 13.5, stats.ride_height + 0.05, row as f32 * 11.0 - 11.0 + 3500.0);
            let yaw = std::env::var("CARBANG_GALLERY_YAW").ok().and_then(|v| v.parse().ok()).unwrap_or(-0.7f32);
            let r = Quat::from_rotation_y(yaw);
            commands.entity(e).insert((Transform::from_translation(p).with_rotation(r), Position(p), Rotation(r)));
        } else {
            let x = i as f32 * 300.0 - 1600.0;
            commands.entity(e).insert((
                Transform::from_xyz(x, stats.ride_height + 0.2, 3500.0),
                Position(Vec3::new(x, stats.ride_height + 0.2, 3500.0)),
                Rotation(Quat::IDENTITY),
                TestDriver { min_up: 1.0, ..default() },
            ));
        }
        i += 1;
    }
    if gallery.is_some() || crash.is_some() {
        let floor_mat = matlib.as_ref().map(|m| m.get(crate::maps::Mat::Asphalt)).unwrap_or_else(|| mats.add(StandardMaterial::default()));
        commands.spawn((
            Transform::from_xyz(0.0, -0.004, 3500.0),
            Mesh3d(meshes.add(crate::meshgen::box_mesh(Vec3::new(240.0, 0.01, 240.0), 9.0))),
            MeshMaterial3d(floor_mat),
            DespawnOnExit(AppState::Race),
        ));
        // Фонарь над местом удара и пара дальних огней
        let lamp_mat = matlib.as_ref().map(|m| m.get(crate::maps::Mat::Lamp)).unwrap_or_default();
        let metal = matlib.as_ref().map(|m| m.get(crate::maps::Mat::Metal)).unwrap_or_default();
        for (k, p) in [Vec3::new(4.0, 0.0, 3505.5), Vec3::new(-28.0, 0.0, 3480.0), Vec3::new(30.0, 0.0, 3470.0)].iter().enumerate() {
            commands.spawn((Mesh3d(meshes.add(Cylinder::new(0.12, 8.0).mesh().resolution(8))), MeshMaterial3d(metal.clone()), Transform::from_translation(*p + Vec3::Y * 4.0), DespawnOnExit(AppState::Race)));
            commands.spawn((Mesh3d(meshes.add(Cuboid::new(1.2, 0.25, 0.6))), MeshMaterial3d(lamp_mat.clone()), Transform::from_translation(*p + Vec3::Y * 8.0), DespawnOnExit(AppState::Race)));
            commands.spawn((
                SpotLight { color: Color::srgb(1.0, 0.64, 0.3), intensity: if k == 0 { 2_500_000.0 } else { 900_000.0 }, range: 40.0, outer_angle: 0.85, inner_angle: 0.5, shadow_maps_enabled: k == 0, ..default() },
                bevy::light::VolumetricLight,
                Transform::from_translation(*p + Vec3::Y * 7.8).looking_at(if k == 0 { Vec3::new(0.0, 0.0, 3500.0) } else { *p + Vec3::new(-3.0, 0.0, 6.0) }, Vec3::Z),
                DespawnOnExit(AppState::Race),
            ));
        }
    }
}

fn crash_driver(mut ms: ResMut<MatchState>, mut q: Query<(&mut CarInput, &mut CarState)>) {
    if ms.phase == MatchPhase::Countdown {
        ms.countdown = 0.0;
        ms.phase = MatchPhase::Running;
        // сцена для отчёта: машины уже побиты (кузов 45%, пробит радиатор), чтобы были видны
        // стадии износа, пар и дым; CARBANG_CRASH_FRESH — целые машины
        if std::env::var("CARBANG_CRASH_FRESH").is_err() {
            for (_, mut st) in q.iter_mut() {
                st.hp = 45.0;
                st.armor = 10.0;
                st.zones = [40.0, 10.0, 20.0, 15.0];
            }
        }
    }
    for (mut i, st) in q.iter_mut() {
        *i = CarInput { throttle: if ms.elapsed < 3.4 && !st.wrecked { 1.0 } else { 0.0 }, ..default() };
    }
}

/// Ввод на старте по вариантам:
/// 0 — тормоз в отсчёт, газ сразу после START;
/// 1 — газ в отсчёт и после;
/// 2 — шум геймпада ниже мёртвой зоны в отсчёт и первую секунду, затем газ;
/// 3 — тормоз держится 1.5 с после START (проверка блокировки реверса), затем газ.
fn start_driver(
    ms: Res<MatchState>,
    time: Res<Time>,
    mut q: Query<(&Car, &mut CarInput, &mut StartProbe, &Position, &LinearVelocity, &CarState)>,
    mut exit: MessageWriter<AppExit>,
    mut t_run: Local<f32>,
) {
    let running = ms.phase == MatchPhase::Running;
    if running {
        *t_run += time.delta_secs();
    }
    let t = *t_run;
    for (_car, mut inp, mut p, pos, vel, _st) in q.iter_mut() {
        let d = pos.0 - p.origin;
        let long = d.dot(p.fwd);
        *inp = CarInput::default();
        match p.pattern {
            0 => {
                if running { inp.throttle = 1.0 } else { inp.brake = 1.0 }
            }
            1 => inp.throttle = 1.0,
            2 => {
                if !running || t < 1.0 {
                    inp.throttle = 0.06;
                    inp.brake = 0.07;
                    inp.steer = 0.05;
                } else {
                    inp.throttle = 1.0;
                }
            }
            _ => {
                if !running || t < 1.5 { inp.brake = 1.0 } else { inp.throttle = 1.0 }
            }
        }
        if !running {
            p.max_countdown_move = p.max_countdown_move.max(Vec2::new(d.x, d.z).length());
        } else if t <= 3.0 {
            p.min_long = p.min_long.min(long);
            if p.reverse_at.is_none() && vel.0.dot(p.fwd) < -0.3 {
                p.reverse_at = Some(t);
            }
            p.long_at_3s = long;
        }
    }
    if t > 3.0 {
        println!("\nТЕСТ СТАРТА (смещение вдоль исходного «вперёд», м; реверс — момент первой скорости назад < -0,3 м/с)");
        println!("{:<14} {:>7} {:>11} {:>9} {:>9} {:>8}", "машина#вар", "отсчёт", "мин.вперёд", "за 3 с", "реверс", "итог");
        let mut fails = 0;
        let mut rows: Vec<_> = q.iter().map(|(car, _, p, ..)| (p.pattern, car.name.clone(), p.max_countdown_move, p.min_long, p.long_at_3s, p.reverse_at)).collect();
        rows.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        for (pattern, name, cd, minl, l3, rev) in rows {
            // вариант 3 проверяет только то, что реверс не раньше 0,5 с после START
            let ok = if pattern == 3 { rev.map(|r| r >= 0.5).unwrap_or(true) && cd < 0.05 } else { minl > -0.05 && cd < 0.05 && l3 > 1.0 };
            if !ok {
                fails += 1;
            }
            println!(
                "{:<14} {:>6.2}м {:>10.2}м {:>8.1}м {:>9} {:>8}",
                name,
                cd,
                minl,
                l3,
                rev.map(|r| format!("{:.2}с", r)).unwrap_or("нет".into()),
                if ok { "ок" } else { "ПРОВАЛ" }
            );
        }
        println!("провалов: {}", fails);
        exit.write(if fails == 0 { AppExit::Success } else { AppExit::error() });
    }
}

/// Таран: атакующий держит скорость до удара, жертва на ручнике. Через 2 с после удара — итог.
fn ram_driver(
    mut ms: ResMut<MatchState>,
    time: Res<Time>,
    mut q: Query<(&Car, &mut CarInput, &mut RamProbe, &Position, &LinearVelocity, &Rotation, &CarState, &CarStats)>,
    mut exit: MessageWriter<AppExit>,
    mut t_run: Local<f32>,
) {
    if ms.phase == MatchPhase::Countdown {
        ms.countdown = 0.0;
        ms.phase = MatchPhase::Running;
    }
    *t_run += time.delta_secs();
    let t = *t_run;
    let pos_of: Vec<(usize, bool, Vec3, f32)> = q.iter().map(|(_, _, p, pos, _, _, _, cs)| (p.pair, p.attacker, pos.0, cs.size.z * 0.5)).collect();
    for (_car, mut inp, mut p, pos, vel, rot, st, cs) in q.iter_mut() {
        if t < 0.05 {
            p.hp0 = st.hp;
            p.armor0 = st.armor;
        }
        *inp = CarInput::default();
        if !p.attacker {
            inp.handbrake = true;
            continue;
        }
        let (other, other_half) = pos_of.iter().find(|(pair, att, ..)| *pair == p.pair && !*att).map(|x| (x.2, x.3)).unwrap_or((pos.0, 2.0));
        // зазор между бамперами
        let gap = horiz(other - pos.0).length() - other_half - cs.size.z * 0.5;
        let speed = vel.0.dot(rot.0 * Vec3::NEG_Z);
        if p.impact_t.is_none() {
            if gap < 2.0 && p.impact_speed == 0.0 {
                p.impact_speed = speed * 3.6;
            }
            if gap < 0.4 {
                p.impact_t = Some(t);
            }
            let target = p.speed_kmh / 3.6;
            // разгон с нитро до нужной скорости, затем удержание
            let e = target - speed;
            inp.throttle = (e * 0.8).clamp(0.0, 1.0);
            inp.nitro = e > 3.0;
            inp.brake = (-e * 0.4).clamp(0.0, 1.0);
        }
    }
    // итог, когда все пары ударились (или вышло время)
    let all = q.iter().filter(|(_, _, p, ..)| p.attacker).all(|(_, _, p, ..)| p.impact_t.map(|it| t - it > 2.0).unwrap_or(false));
    if all || t > 40.0 {
        println!("\nТЕСТ ТАРАНА: лобовой удар в стоящую машину той же модели (снято прочности: кузов+броня)");
        println!("{:<10} {:>6} {:>8} {:>10} {:>10} {:>9} {:>9}", "машина", "км/ч", "удар", "жертва", "атакующий", "ж.хлам", "а.хлам");
        let mut rows = Vec::new();
        for (car, _, p, _, _, _, st, _) in q.iter() {
            if p.attacker {
                let victim = q.iter().find(|(_, _, v, ..)| v.pair == p.pair && !v.attacker);
                if let Some((_, _, v, _, _, _, vst, _)) = victim {
                    rows.push((
                        p.speed_kmh,
                        car.def().id,
                        p.impact_speed,
                        (v.hp0 + v.armor0) - (vst.hp.max(0.0) + vst.armor),
                        (p.hp0 + p.armor0) - (st.hp.max(0.0) + st.armor),
                        vst.wrecked,
                        st.wrecked,
                        v.hp0 - vst.hp.max(0.0),
                    ));
                }
            }
        }
        rows.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(b.1)));
        for r in &rows {
            println!("{:<10} {:>6.0} {:>7.0}  {:>9.1} {:>10.1} {:>9} {:>9}  (кузов {:.1})", r.1, r.0, r.2, r.3, r.4, if r.5 { "ДА" } else { "нет" }, if r.6 { "ДА" } else { "нет" }, r.7);
        }
        for kmh in [40.0f32, 80.0, 120.0, 140.0] {
            let v: Vec<f32> = rows.iter().filter(|r| r.0 == kmh).map(|r| r.7).collect();
            if !v.is_empty() {
                let mean = v.iter().sum::<f32>() / v.len() as f32;
                let min = v.iter().cloned().fold(f32::MAX, f32::min);
                let max = v.iter().cloned().fold(f32::MIN, f32::max);
                let wrecks = rows.iter().filter(|r| r.0 == kmh && r.5).count();
                println!("{:>4.0} км/ч: жертва теряет кузова в среднем {:.1} (мин {:.1}, макс {:.1}), уничтожено {}/{}", kmh, mean, min, max, wrecks, v.len());
            }
        }
        exit.write(AppExit::Success);
    }
}

fn crash_camera(mut cam: Query<&mut GlobalTransform, With<Camera3d>>) {
    for mut t in cam.iter_mut() {
        *t = GlobalTransform::from(Transform::from_xyz(-3.0, 1.8, 3511.0).looking_at(Vec3::new(1.0, 0.9, 3500.0), Vec3::Y));
    }
}

/// Камера для скриншотов отчёта: CARBANG_CAM="x,y,z,tx,ty,tz" — позиция и точка взгляда.
fn env_camera(mut cam: Query<&mut GlobalTransform, With<Camera3d>>, mut parsed: Local<Option<Option<(Vec3, Vec3)>>>) {
    let v = parsed.get_or_insert_with(|| {
        let s = std::env::var("CARBANG_CAM").ok()?;
        let n: Vec<f32> = s.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        (n.len() == 6).then(|| (Vec3::new(n[0], n[1], n[2]), Vec3::new(n[3], n[4], n[5])))
    });
    if let Some((p, t)) = *v {
        for mut g in cam.iter_mut() {
            *g = GlobalTransform::from(Transform::from_translation(p).looking_at(t, Vec3::Y));
        }
    }
}

fn gallery_camera(mut cam: Query<&mut GlobalTransform, With<Camera3d>>) {
    // CARBANG_CAM="x,y,z,tx,ty,tz" — своя точка съёмки (машины стоят около z = 3500)
    let custom = std::env::var("CARBANG_CAM").ok().and_then(|s| {
        let v: Vec<f32> = s.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        (v.len() == 6).then(|| Transform::from_xyz(v[0], v[1], v[2]).looking_at(Vec3::new(v[3], v[4], v[5]), Vec3::Y))
    });
    for mut t in cam.iter_mut() {
        *t = GlobalTransform::from(custom.unwrap_or_else(|| Transform::from_xyz(-4.0, 13.0, 3526.0).looking_at(Vec3::new(0.0, 0.5, 3500.0), Vec3::Y)));
    }
}

fn speed_hold(speed: f32, target: f32) -> (f32, f32) {
    let e = target - speed;
    ((e * 0.6).clamp(0.0, 1.0), (-e * 0.4).clamp(0.0, 1.0))
}

fn test_driver(
    time: Res<Time>,
    mut ms: ResMut<MatchState>,
    mut q: Query<(&Car, &mut TestDriver, &mut CarInput, &Position, &Rotation, &LinearVelocity, &AngularVelocity)>,
    mut exit: MessageWriter<AppExit>,
) {
    let dt = time.delta_secs();
    if ms.phase == MatchPhase::Countdown {
        ms.countdown = 0.0;
        ms.phase = MatchPhase::Running;
    }
    let mut all_done = true;
    for (_car, mut d, mut inp, pos, rot, lv, av) in q.iter_mut() {
        d.t += dt;
        d.total += dt;
        let fwd = rot.0 * Vec3::NEG_Z;
        let speed = lv.0.dot(fwd);
        let up = rot.0 * Vec3::Y;
        d.min_up = d.min_up.min(up.y);
        if up.y < 0.2 {
            d.flipped = true;
        }
        *inp = CarInput::default();
        match d.phase {
            // разгон
            0 => {
                inp.throttle = 1.0;
                if d.t100.is_none() && speed >= 27.78 {
                    d.t100 = Some(d.t);
                }
                d.top = d.top.max(speed);
                if d.t > 25.0 {
                    d.phase = 1;
                    d.t = 0.0;
                }
            }
            // держим 100 км/ч
            1 => {
                let (t, b) = speed_hold(speed, 27.78);
                inp.throttle = t;
                inp.brake = b;
                if d.t > 4.0 {
                    d.phase = 2;
                    d.t = 0.0;
                    d.brake_from = pos.0;
                }
            }
            // торможение в пол
            2 => {
                inp.brake = 1.0;
                if speed < 0.3 || d.t > 10.0 {
                    d.brake_dist = horiz(pos.0 - d.brake_from).length();
                    d.phase = 3;
                    d.t = 0.0;
                }
            }
            // круг на 15 м/с
            3 | 4 => {
                let target = if d.phase == 3 { 15.0 } else { 25.0 };
                let (t, b) = speed_hold(speed, target);
                inp.throttle = t;
                inp.brake = b * 0.3;
                if d.t > 4.0 {
                    inp.steer = 1.0;
                }
                if d.t > 8.0 {
                    let lat = (speed * av.0.y).abs() / G;
                    if d.phase == 3 {
                        d.lat15.push(lat);
                    } else {
                        d.lat25.push(lat);
                    }
                }
                if d.t > 11.0 {
                    d.phase += 1;
                    d.t = 0.0;
                }
            }
            // слалом на 25 м/с
            5 => {
                let (t, b) = speed_hold(speed, 25.0);
                inp.throttle = t;
                inp.brake = b * 0.3;
                if d.t > 3.0 {
                    inp.steer = if (d.t * std::f32::consts::PI).sin() > 0.0 { 1.0 } else { -1.0 };
                    let vdir = horiz(lv.0).normalize_or_zero();
                    let hf = horiz(fwd).normalize_or_zero();
                    if speed > 8.0 && vdir.dot(hf) < 0.5 {
                        d.spun = true;
                    }
                }
                if d.t > 11.0 {
                    d.phase = 6;
                }
            }
            _ => {}
        }
        if d.phase < 6 {
            all_done = false;
        }
    }
    if all_done {
        println!("\n{:<10} {:>7} {:>8} {:>8} {:>7} {:>7} {:>6} {:>6} {:>5}", "машина", "0-100", "макс", "тормоз", "бок15", "бок25", "minUp", "перев", "занос");
        for (car, d, ..) in q.iter() {
            let avg = |v: &Vec<f32>| if v.is_empty() { 0.0 } else { v.iter().sum::<f32>() / v.len() as f32 };
            println!(
                "{:<10} {:>6.2}с {:>5.0}км/ч {:>7.1}м {:>6.2}g {:>6.2}g {:>6.2} {:>6} {:>5}",
                car.def().id,
                d.t100.unwrap_or(99.0),
                d.top * 3.6,
                d.brake_dist,
                avg(&d.lat15),
                avg(&d.lat25),
                d.min_up,
                if d.flipped { "ДА" } else { "нет" },
                if d.spun { "ДА" } else { "нет" }
            );
        }
        exit.write(AppExit::Success);
    }
}

// ============================================================ тест боя

fn battle_monitor(
    mut stats: ResMut<BattleStats>,
    cli: Res<Cli>,
    ms: Option<Res<MatchState>>,
    state: Res<State<AppState>>,
    cars: Query<(&Car, &CarState, &Combat, &Transform, &Weapons)>,
    result: Res<MatchResult>,
    bots: Query<(&crate::ai::Bot, &Car, &CarState)>,
    mut exit: MessageWriter<AppExit>,
    mut wall_last: Local<Option<std::time::Instant>>,
    log: Option<Res<crate::combat::DamageLog>>,
) {
    let now = std::time::Instant::now();
    if let Some(prev) = *wall_last {
        let dtw = (now - prev).as_secs_f64();
        stats.wall += dtw;
        stats.worst_step = stats.worst_step.max(dtw);
    }
    *wall_last = Some(now);
    stats.steps += 1;
    if *state.get() == AppState::Results {
        println!("\n=== МАТЧ ОКОНЧЕН: {} ===", result.reason);
        for r in &result.rows {
            println!("{:<12} {:<10} очки {:>6}  разбил {:>2}  пеших {:>3}  смертей {:>2}  урон {:>5}", r.name, r.car, r.score, r.wrecks, r.peds, r.deaths, r.damage);
        }
        print_perf(&stats);
        print_match_summary(stats.snap, log.as_deref());
        exit.write(AppExit::Success);
        return;
    }
    let Some(ms) = ms else { return };
    if ms.phase == MatchPhase::Running {
        let alive = cars.iter().filter(|c| !c.1.wrecked).count();
        let player = cars.iter().find(|c| c.0.controller == Controller::Local);
        let (pa, php, pw) = player.map(|p| (!p.1.wrecked, p.1.hp.max(0.0), p.2.wrecks)).unwrap_or((false, 0.0, 0));
        stats.snap = Some((ms.elapsed, alive, cars.iter().count(), pa, php, pw));
    }
    let n = cars.iter().count();
    if stats.dist.len() != n {
        stats.dist = vec![0.0; n];
        stats.stuck = vec![0.0; n];
        stats.last_pos = cars.iter().map(|c| c.3.translation).collect();
    }
    for (i, (_, st, _, tf, _)) in cars.iter().enumerate() {
        let d = tf.translation.distance(stats.last_pos[i]);
        if d < 5.0 {
            stats.dist[i] += d;
        }
        stats.last_pos[i] = tf.translation;
        if !st.wrecked && st.speed.abs() < 1.0 && ms.phase == MatchPhase::Running {
            stats.stuck[i] += 1.0 / 120.0;
        }
    }
    if std::env::var("CARBANG_AI_DEBUG").is_ok() && ms.phase == MatchPhase::Running && ms.elapsed - stats.last_dbg >= 2.0 {
        stats.last_dbg = ms.elapsed;
        println!("t={:.0}", ms.elapsed);
        for (car, st, _, tf, _) in cars.iter().take(4) {
            let _ = (car, st, tf);
        }
        for (b, car, st) in bots.iter() {
            println!(
                "{:<12} {:?} v={:>5.1} путь={} цель={:>5.1}м ts={:>5.1} щуп={:.2} угол={:>5.2} газ={:.2} торм={:.2} руль={:>5.2} rev={:.1} stuck={:.1}",
                car.name, short_goal(&b.goal), st.speed, b.path.len(), (b.aim - st_pos(&cars, car)).length(), b.dbg[0], b.dbg[1], b.dbg[2], b.dbg[3], b.dbg[4], b.dbg[5], b.reverse_t, b.stuck_t
            );
        }
    }
    if ms.elapsed - stats.last_report >= 15.0 || ms.elapsed >= cli.secs {
        stats.last_report = ms.elapsed;
        println!("\n--- t={:.0}с пешеходов {}/{} ---", ms.elapsed, ms.peds_alive, ms.peds_total);
        for (i, (car, st, c, tf, w)) in cars.iter().enumerate() {
            let ammo: Vec<String> = w.slots.iter().map(|s| format!("{:?}:{:.0}", s.kind, s.ammo)).collect();
            println!(
                "{:<12} {:<9} hp {:>5.1} {} очки {:>6} разб {:>2} пеш {:>3} смер {:>2} путь {:>6.0}м стоял {:>5.1}с поз ({:>6.1},{:>5.1},{:>6.1}) [{}]",
                car.name,
                car.def().id,
                st.hp,
                if st.wrecked { "ХЛАМ" } else { "    " },
                c.score,
                c.wrecks,
                c.peds,
                c.deaths,
                stats.dist[i],
                stats.stuck[i],
                tf.translation.x,
                tf.translation.y,
                tf.translation.z,
                ammo.join(" ")
            );
        }
        if ms.elapsed >= cli.secs {
            print_perf(&stats);
            print_match_summary(stats.snap, log.as_deref());
            exit.write(AppExit::Success);
        }
    }
}

fn print_feed(mut ev: MessageReader<GameEvent>, ms: Option<Res<MatchState>>) {
    let t = ms.map(|m| m.elapsed).unwrap_or(0.0);
    for e in ev.read() {
        if let GameEvent::KillFeed { text } = e {
            println!("[{:>5.1}] {}", t, text);
        }
    }
}

fn short_goal(g: &crate::ai::Goal) -> &'static str {
    use crate::ai::Goal::*;
    match g {
        Hunt(_) => "Охота ",
        Repair(_) => "Ремонт",
        Pickup(_) => "Бонус ",
        Peds(_) => "Пешие ",
        Flee(_) => "Бегство",
        Roam(_) => "Бродит",
    }
}

fn st_pos(cars: &Query<(&Car, &CarState, &Combat, &Transform, &Weapons)>, car: &Car) -> Vec3 {
    cars.iter().find(|c| c.0.name == car.name).map(|c| c.3.translation).unwrap_or(Vec3::ZERO)
}

/// Итог матча для теста баланса: одна строка «MATCH» для сводки по сериям.
fn print_match_summary(snap: Option<(f32, usize, usize, bool, f32, u32)>, log: Option<&crate::combat::DamageLog>) {
    let Some(log) = log else { return };
    let (dur, alive, total, survived, hp, wrecks) = snap.unwrap_or((0.0, 0, 0, false, 0.0, 0));
    let death_t = if survived { -1.0 } else { log.human_hits.iter().filter(|h| h.3 <= 0.0).map(|h| h.0).next().unwrap_or(-1.0) };
    let biggest = log.human_hits.iter().map(|h| h.2).fold(0.0f32, f32::max);
    let big_hits = log.human_hits.iter().filter(|h| h.2 >= 30.0).count();
    let mut top: Vec<_> = log.human_hits.iter().filter(|h| h.2 >= 15.0).collect();
    top.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
    for h in top.iter().take(6) {
        println!("  удар по игроку: t={:.1} {:?} -{:.1} → {:.1}", h.0, h.1, h.2, h.3);
    }
    let mut kinds: Vec<(String, f32)> = Vec::new();
    for h in &log.human_hits {
        let k = format!("{:?}", h.1);
        if let Some(e) = kinds.iter_mut().find(|e| e.0 == k) { e.1 += h.2 } else { kinds.push((k, h.2)) }
    }
    println!("  игрок потерял по видам: {:?}", kinds.iter().map(|k| format!("{}={:.0}", k.0, k.1)).collect::<Vec<_>>());
    println!("\nУрон по видам (сумма / попаданий / крупнейший):");
    for (k, sum, n, mx) in &log.by_kind {
        println!("  {:<10} {:>8.0} {:>6} {:>7.1}", format!("{:?}", k), sum, n, mx);
    }
    println!(
        "MATCH dur={:.0} alive={}/{} player_survived={} player_hp={:.0} player_wrecks={} death_t={:.0} biggest_hit={:.1} hits_ge30={}",
        dur, alive, total, survived as u8, hp, wrecks, death_t, biggest, big_hits
    );
}

fn print_perf(s: &BattleStats) {
    println!(
        "\nпроизводительность: шагов {}, среднее {:.2} мс/шаг, худший {:.1} мс",
        s.steps,
        s.wall / s.steps.max(1) as f64 * 1000.0,
        s.worst_step * 1000.0
    );
}

// ============================================================ сетевой тест

#[derive(Resource, Default)]
pub struct NetTest {
    host: bool,
    started: bool,
    t0: f64,
    race_t0: f64,
    events: u32,
    last_print: f64,
}

#[allow(clippy::too_many_arguments)]
fn net_test(
    mut t: ResMut<NetTest>,
    time: Res<Time<Real>>,
    cli: Res<Cli>,
    state: Res<State<AppState>>,
    server: Option<Res<crate::net::server::Server>>,
    client: Option<Res<crate::net::client::Client>>,
    mut lobby: ResMut<crate::net::LobbyInfo>,
    mut net: MessageWriter<crate::net::NetCommand>,
    mut input: ResMut<crate::race::LocalInput>,
    cars: Query<(&Car, &CarState, &Combat, &Transform, &NetId)>,
    local: Res<LocalPlayer>,
    ms: Option<Res<MatchState>>,
    mut ev: MessageReader<GameEvent>,
    mut exit: MessageWriter<AppExit>,
    status: Res<crate::net::NetStatus>,
) {
    let now = time.elapsed_secs_f64();
    if t.t0 == 0.0 {
        t.t0 = now;
    }
    t.events += ev.read().count() as u32;
    // хост стартует, когда подключился клиент
    if t.host && !t.started {
        if let Some(s) = server.as_ref() {
            if !s.clients.is_empty() && now - t.t0 > 1.0 {
                lobby.map = cli.map.as_deref().and_then(parse_map).unwrap_or(MapId::Coliseum);
                lobby.mode = cli.mode.as_deref().and_then(parse_mode).unwrap_or(GameMode::Deathmatch);
                lobby.bots = 2;
                net.write(crate::net::NetCommand::Start);
                t.started = true;
                println!("[хост] клиент подключился, старт");
            }
        }
        if now - t.t0 > 20.0 && !t.started {
            println!("[хост] клиент не подключился: {}", status.text);
            exit.write(AppExit::error());
        }
    }
    if !t.host && client.is_none() && now - t.t0 > 2.0 && *state.get() != AppState::Race {
        println!("[клиент] нет соединения: {}", status.text);
        exit.write(AppExit::error());
        return;
    }
    if *state.get() == AppState::Race {
        if t.race_t0 == 0.0 {
            t.race_t0 = now;
        }
        // клиент рулит по синусоиде
        let rt = (now - t.race_t0) as f32;
        input.0 = CarInput { throttle: 1.0, steer: (rt * 0.7).sin() * 0.6, fire: true, ..default() };
        if now - t.last_print > 3.0 {
            t.last_print = now;
            let who = if t.host { "хост" } else { "клиент" };
            let mine = local.car.and_then(|id| cars.iter().find(|c| *c.4 == id));
            let ms_txt = ms.as_ref().map(|m| format!("{:?} t={:.1}", m.phase, m.elapsed)).unwrap_or_default();
            let rtt = client.as_ref().map(|c| c.rtt).unwrap_or(0);
            println!(
                "[{}] {} машин {} событий {} моя: {:?} rtt {} мс",
                who,
                ms_txt,
                cars.iter().count(),
                t.events,
                mine.map(|c| (c.3.translation.round(), c.1.speed.round(), c.1.hp.round(), c.2.score)),
                rtt
            );
            for (car, st, c, tf, id) in cars.iter() {
                println!("    {:>2} {:<12} поз {:?} v {:>5.1} hp {:>5.1} очки {:>5} пинг {}", id.0, car.name, tf.translation.round(), st.speed, st.hp, c.score, c.ping);
            }
        }
        if now - t.race_t0 > cli.secs as f64 {
            println!("[{}] тест окончен", if t.host { "хост" } else { "клиент" });
            exit.write(AppExit::Success);
        }
    }
}

// ============================================================ проверка интерфейса

#[derive(Resource, Default)]
pub struct UiFlow {
    step: u32,
    t: f32,
    credits0: u32,
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn ui_flow(
    mut flow: ResMut<UiFlow>,
    time: Res<Time<Real>>,
    state: Res<State<AppState>>,
    mut menu: Query<(&crate::ui::menu::MenuAction, &mut Interaction), (Without<crate::ui::garage::GarageAction>, Without<crate::ui::results::ResultsAction>)>,
    mut garage: Query<(&crate::ui::garage::GarageAction, &mut Interaction), (Without<crate::ui::menu::MenuAction>, Without<crate::ui::results::ResultsAction>)>,
    mut results: Query<(&crate::ui::results::ResultsAction, &mut Interaction), (Without<crate::ui::menu::MenuAction>, Without<crate::ui::garage::GarageAction>)>,
    ms: Option<ResMut<MatchState>>,
    local: Res<LocalPlayer>,
    profile: Res<Profile>,
    result: Res<MatchResult>,
    mut exit: MessageWriter<AppExit>,
) {
    use crate::ui::garage::{GarageAction, GarageTab};
    use crate::ui::menu::{MenuAction, MenuScreen};
    use crate::ui::results::ResultsAction;
    flow.t += time.delta_secs();
    if flow.t < 1.0 {
        return;
    }
    let mut press_menu = |f: &dyn Fn(&MenuAction) -> bool| -> bool {
        for (a, mut i) in menu.iter_mut() {
            if f(a) {
                *i = Interaction::Pressed;
                return true;
            }
        }
        false
    };
    let st = *state.get();
    let step = flow.step;
    let mut advance = false;
    match (step, st) {
        (0, AppState::Menu) => advance = press_menu(&|a| matches!(a, MenuAction::Screen(MenuScreen::Quick))),
        (1, AppState::Menu) => {
            flow.credits0 = profile.credits;
            advance = press_menu(&|a| matches!(a, MenuAction::QuickStart));
        }
        (2, AppState::Race) => {
            if let Some(mut ms) = ms {
                if ms.phase == MatchPhase::Running && ms.elapsed > 3.0 {
                    ms.phase = MatchPhase::Finished;
                    ms.countdown = 0.0;
                    ms.winner = local.car;
                    ms.end_reason = "тест".into();
                    advance = true;
                }
            }
        }
        (3, AppState::Results) => {
            println!("[ui] итоги: победа {} кредиты +{} опыт +{} строк {}", result.won, result.credits, result.xp, result.rows.len());
            for (a, mut i) in results.iter_mut() {
                if matches!(a, ResultsAction::Continue) {
                    *i = Interaction::Pressed;
                    advance = true;
                }
            }
        }
        (4, AppState::Menu) => advance = press_menu(&|a| matches!(a, MenuAction::Screen(MenuScreen::Garage))),
        (5..=9, AppState::Menu) => {
            let want: GarageAction = match step {
                5 => GarageAction::Tab(GarageTab::Tuning),
                6 => GarageAction::BuyUpgrade(Upgrade::Engine),
                7 => GarageAction::Tab(GarageTab::Weapons),
                8 => GarageAction::Tab(GarageTab::Perks),
                _ => GarageAction::TogglePerk(PerkKind::Battering),
            };
            let key = format!("{:?}", want);
            for (a, mut i) in garage.iter_mut() {
                if format!("{:?}", a) == key {
                    *i = Interaction::Pressed;
                    advance = true;
                    break;
                }
            }
        }
        (10, AppState::Menu) => {
            let cp = profile.car_profile(&profile.selected_car);
            println!(
                "[ui] профиль: кредиты {} -> {}, двигатель ур. {}, перки {:?}, опыт {}",
                flow.credits0,
                profile.credits,
                cp.levels[Upgrade::Engine.index()],
                profile.perks,
                profile.xp
            );
            exit.write(AppExit::Success);
        }
        _ => {}
    }
    if advance {
        println!("[ui] шаг {} выполнен в состоянии {:?}", step, st);
        flow.step += 1;
        flow.t = 0.0;
    }
    if flow.t > 120.0 {
        println!("[ui] застряли на шаге {} в состоянии {:?}", flow.step, st);
        exit.write(AppExit::error());
    }
}

// ============================================================ скриншот

fn screenshot_timer(
    mut commands: Commands,
    cli: Res<Cli>,
    time: Res<Time<Real>>,
    mut stage: Local<u32>,
    mut exit: MessageWriter<AppExit>,
    state: Res<State<AppState>>,
    ms: Option<Res<MatchState>>,
    mut res_frames: Local<u32>,
) {
    let Some(path) = cli.screenshot.clone() else { return };
    if *state.get() == AppState::Results {
        *res_frames += 1;
    }
    // в заезде считаем игровое время: программный рендер может быть очень медленным
    let t = match (state.get(), ms.as_ref()) {
        (AppState::Race, Some(m)) if cli.quick || cli.test.is_some() => m.elapsed,
        _ => time.elapsed_secs(),
    };
    // снимки для отчёта: во время отсчёта (CARBANG_SHOT_COUNTDOWN=секунд до старта) и на экране итогов (CARBANG_SHOT_RESULTS)
    let countdown_shot = std::env::var("CARBANG_SHOT_COUNTDOWN").ok().and_then(|v| v.parse::<f32>().ok());
    let results_shot = std::env::var("CARBANG_SHOT_RESULTS").is_ok();
    let (want, ready) = if let Some(cd) = countdown_shot {
        let ok = ms.as_ref().map(|m| m.phase == MatchPhase::Countdown && m.countdown < cd).unwrap_or(false);
        (*state.get() == AppState::Race, ok)
    } else if results_shot {
        // экран итогов строится не в первый кадр
        (*state.get() == AppState::Results, *res_frames > 4)
    } else {
        (if cli.quick { *state.get() == AppState::Race } else { true }, t > cli.secs.min(30.0))
    };
    if *stage == 0 && want && ready {
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
        *stage = 1;
    } else if *stage >= 1 {
        *stage += 1;
        if *stage > 30 {
            exit.write(AppExit::Success);
        }
    }
}
