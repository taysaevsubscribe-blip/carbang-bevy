//! Запуск заезда: карта, свет, небо, машины, пешеходы, бонусы. Ввод локального игрока.

use crate::ai::Bot;
use crate::carmodel::{build_car_model, CarVisual, ModelAssets};
use crate::combat::Combat;
use crate::config::*;
use crate::data::*;
use crate::game::*;
use crate::maps::{build_map, build_matlib, builders, MapData};
use crate::peds::{spawn_peds, PedAssets};
use crate::powerups::{spawn_pickups, PickupMats};
use crate::vehicle::*;
use crate::weapons::Weapons;
use bevy::input::gamepad::{Gamepad, GamepadAxis, GamepadButton};
use bevy::light::{CascadeShadowConfigBuilder, NotShadowCaster};
use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;

/// Ввод локального игрока (на клиенте уходит на сервер).
#[derive(Resource, Default, Clone, Copy)]
pub struct LocalInput(pub CarInput);

/// Пауза (только в одиночной игре останавливает время).
#[derive(Resource, Default)]
pub struct Paused(pub bool);

/// Кнопки, которые нужно ловить по нажатию, а не удержанию (камера, таблица).
#[derive(Resource, Default)]
pub struct UiToggles {
    pub scoreboard: bool,
    pub look_back: bool,
    pub camera_mode: u8,
}

/// Идентификатор этого клиента в сетевой игре.
#[derive(Resource, Default, Clone, Copy)]
pub struct MyClientId(pub u64);

#[derive(Component)]
pub struct SkyDome;

#[derive(Component)]
pub struct SunLight;

#[derive(Component)]
pub struct SunDisc;

pub struct RacePlugin;

impl Plugin for RacePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LocalInput>()
            .init_resource::<Paused>()
            .init_resource::<UiToggles>()
            .init_resource::<MyClientId>()
            .add_systems(OnEnter(AppState::Loading), loading_screen)
            .add_systems(Update, loading_tick.run_if(in_state(AppState::Loading)))
            .add_systems(OnEnter(AppState::Race), setup_race)
            .add_systems(OnExit(AppState::Race), cleanup_race)
            .add_systems(
                Update,
                (gather_input, apply_local_input.run_if(is_authority), pause_toggle, sky_follow)
                    .chain()
                    .run_if(in_state(AppState::Race)),
            );
    }
}

/// Экран загрузки: один кадр с надписью, потом тяжёлая постройка карты.
fn loading_screen(mut commands: Commands, cfg: Res<MatchConfig>, fonts: Option<Res<crate::ui::Fonts>>) {
    commands.spawn((Camera2d, DespawnOnExit(AppState::Loading)));
    let Some(f) = fonts else { return };
    commands
        .spawn((
            Node { width: Val::Percent(100.0), height: Val::Percent(100.0), flex_direction: FlexDirection::Column, justify_content: JustifyContent::Center, align_items: AlignItems::Center, row_gap: Val::Px(12.0), ..default() },
            BackgroundColor(Color::srgb(0.04, 0.04, 0.05)),
            DespawnOnExit(AppState::Loading),
        ))
        .with_children(|c| {
            c.spawn((Text::new(map_info(cfg.map).name), TextFont { font: f.title.clone().into(), font_size: FontSize::Px(56.0), ..default() }, TextColor(Color::srgb(1.0, 0.55, 0.1))));
            c.spawn((Text::new(cfg.mode.name()), TextFont { font: f.bold.clone().into(), font_size: FontSize::Px(26.0), ..default() }, TextColor(Color::srgb(0.9, 0.9, 0.88))));
            c.spawn((Text::new(map_info(cfg.map).desc), TextFont { font: f.body.clone().into(), font_size: FontSize::Px(20.0), ..default() }, TextColor(Color::srgb(0.6, 0.6, 0.62))));
            c.spawn((Text::new("Загрузка..."), TextFont { font: f.body.clone().into(), font_size: FontSize::Px(20.0), ..default() }, TextColor(Color::srgb(0.6, 0.6, 0.62))));
        });
}

fn loading_tick(mut frames: Local<u32>, mut next: ResMut<NextState<AppState>>) {
    *frames += 1;
    if *frames >= 3 {
        *frames = 0;
        next.set(AppState::Race);
    }
}

/// Позиция старта для машины с индексом i.
pub fn spawn_slot(map: &MapData, i: usize) -> (Vec3, f32) {
    if map.spawns.is_empty() {
        return (Vec3::new(i as f32 * 6.0, 1.0, 0.0), 0.0);
    }
    let n = map.spawns.len();
    let (p, yaw) = map.spawns[i % n];
    // если машин больше, чем точек, ставим вторым рядом
    let row = (i / n) as f32;
    let back = Quat::from_rotation_y(yaw) * Vec3::Z * row * 8.0;
    (p + back, yaw)
}

#[allow(clippy::too_many_arguments)]
pub fn setup_race(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut std_mats: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    cfg: Res<MatchConfig>,
    mode: Res<NetMode>,
    headless: Res<Headless>,
    settings: Res<Settings>,
    my_id: Res<MyClientId>,
    mut ids: ResMut<NetIdAlloc>,
    mut local: ResMut<LocalPlayer>,
    (ped_assets, pickup_mats, wear, kenney): (
        Option<Res<PedAssets>>,
        Option<Res<PickupMats>>,
        Option<Res<crate::carmodel::WearAssets>>,
        Option<Res<crate::extassets::KenneyLib>>,
    ),
    mut paused: ResMut<Paused>,
    mut time: ResMut<Time<Virtual>>,
    cli: Res<crate::Cli>,
) {
    paused.0 = false;
    time.unpause();
    let authority = *mode != NetMode::Client;
    let visuals = !headless.0;
    let theme0 = builders::theme_for(cfg.map);
    let matlib = build_matlib(&mut images, &mut std_mats, &theme0);
    let map = build_map(cfg.map, cfg.seed, &mut commands, &mut meshes, &mut std_mats, &matlib, authority, visuals, kenney.as_deref().cloned());
    let theme = map.theme.clone();

    // ---------------- свет и небо
    if visuals {
        let quality = settings.quality;
        commands.insert_resource(GlobalAmbientLight { color: theme.ambient, brightness: theme.ambient_brightness, ..default() });
        commands.insert_resource(ClearColor(theme.fog));
        let shadow = CascadeShadowConfigBuilder {
            num_cascades: if quality >= 2 { 4 } else { 2 },
            first_cascade_far_bound: 25.0,
            maximum_distance: if quality >= 2 { 260.0 } else { 140.0 },
            ..default()
        }
        .build();
        let mut sun_e = commands.spawn((
            DirectionalLight {
                color: theme.sun_color,
                illuminance: theme.sun_lux,
                shadow_maps_enabled: quality >= 1,
                contact_shadows_enabled: quality >= 2,
                ..default()
            },
            Transform::default().looking_to(theme.sun_dir, Vec3::Y),
            shadow,
            SunLight,
            DespawnOnExit(AppState::Race),
        ));
        // лучи сквозь туман даёт только низкое солнце; ночью — фонари и прожекторы
        if !theme.night {
            sun_e.insert(bevy::light::VolumetricLight);
        }
        // Небесная сфера с вертикальным градиентом
        let mut sky = Sphere::new(1400.0).mesh().uv(96, 48);
        if let Some(VertexAttributeValues::Float32x3(pos)) = sky.attribute(Mesh::ATTRIBUTE_POSITION) {
            let top = theme.sky_top.to_linear();
            let hor = theme.sky_horizon.to_linear();
            let fog = theme.fog.to_linear();
            let sun = -theme.sun_dir;
            let sun_c = theme.sun_color.to_linear();
            let lerp = |a: LinearRgba, b: LinearRgba, t: f32| LinearRgba::rgb(a.red + (b.red - a.red) * t, a.green + (b.green - a.green) * t, a.blue + (b.blue - a.blue) * t);
            let colors: Vec<[f32; 4]> = pos
                .iter()
                .map(|p| {
                    let d = Vec3::from_array(*p).normalize_or_zero();
                    let h = d.y.clamp(-1.0, 1.0);
                    let mut c = if h < 0.0 {
                        fog
                    } else {
                        let t = (h / 0.5).clamp(0.0, 1.0).powf(0.6);
                        let sky = lerp(hor, top, t);
                        // у горизонта небо тонет в тумане
                        lerp(fog, sky, (h / 0.12).clamp(0.0, 1.0))
                    };
                    // рваные облака низкой частоты
                    if h > 0.0 {
                        let az = d.z.atan2(d.x) / std::f32::consts::TAU + 0.5;
                        let n = crate::meshgen::fbm(az, h * 1.5, 6, 4, 77);
                        let k = 0.8 + 0.4 * n;
                        c = LinearRgba::rgb(c.red * k, c.green * k, c.blue * k);
                    }
                    // свечение вокруг солнца или луны
                    let glow = d.dot(sun).max(0.0).powf(if theme.night { 40.0 } else { 6.0 }) * if theme.night { 0.15 } else { 0.9 };
                    c = LinearRgba::rgb(c.red + sun_c.red * glow, c.green + sun_c.green * glow * 0.8, c.blue + sun_c.blue * glow * 0.6);
                    [c.red, c.green, c.blue, 1.0]
                })
                .collect();
            sky.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        }
        let sky_mat = std_mats.add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            cull_mode: None,
            fog_enabled: false,
            ..default()
        });
        commands.spawn((
            Mesh3d(meshes.add(sky)),
            MeshMaterial3d(sky_mat),
            Transform::default(),
            NotShadowCaster,
            SkyDome,
            DespawnOnExit(AppState::Race),
        ));
        // Солнце или луна
        let sun_mat = std_mats.add(StandardMaterial {
            base_color: Color::BLACK,
            emissive: if theme.night { LinearRgba::rgb(2.5, 2.7, 3.2) } else { LinearRgba::rgb(40.0, 22.0, 10.0) },
            unlit: true,
            fog_enabled: false,
            ..default()
        });
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(if theme.night { 28.0 } else { 40.0 }).mesh().uv(16, 8))),
            MeshMaterial3d(sun_mat),
            Transform::from_translation(-theme.sun_dir * 1300.0),
            NotShadowCaster,
            SunDisc,
            DespawnOnExit(AppState::Race),
        ));
    }

    // ---------------- машины
    local.car = None;
    let mut model_assets = ModelAssets {
        meshes: &mut meshes,
        materials: &mut std_mats,
        wear: wear.as_deref().cloned(),
        headlights: theme.night && settings.quality >= 1,
    };
    for (i, slot) in cfg.players.iter().enumerate() {
        let mut car_cfg = slot.config.clone();
        car_cfg.sanitize();
        let def = car_cfg.def();
        let stats = CarStats::compute(&car_cfg);
        let (p, yaw) = spawn_slot(&map, i);
        let id = NetId(1 + i as u32);
        let is_local = match slot.controller {
            Controller::Local => *mode != NetMode::Client,
            Controller::Remote(c) => *mode == NetMode::Client && c == my_id.0,
            Controller::Bot => false,
        };
        if is_local {
            local.car = Some(id);
        }
        let weapons = Weapons::from_config(&car_cfg, &stats);
        let mut st = CarState { nitro: stats.nitro_max, nitro_max: stats.nitro_max, ..default() };
        st.weapons = weapons
            .slots
            .iter()
            .map(|w| WeaponHud { kind: w.kind, mount: w.mount, ammo: w.ammo as u32, max_ammo: w.max_ammo as u32, ready: 1.0 })
            .collect();
        let start = Transform::from_translation(p + Vec3::Y * (stats.ride_height + 0.2)).with_rotation(Quat::from_rotation_y(yaw));
        let combat = Combat { time_left: cfg.time_limit, ..default() };
        let mut ec = commands.spawn((
            Car { name: slot.name.clone(), controller: slot.controller, cfg: car_cfg.clone(), bot_seed: slot.bot_seed },
            CarInput::default(),
            st,
            weapons,
            combat,
            id,
            start,
            Visibility::default(),
            CarVisual,
            DespawnOnExit(AppState::Race),
        ));
        if authority {
            ec.insert((physics_bundle(def, &stats), make_sim(def, &stats)));
            if slot.controller == Controller::Bot || (is_local && cli.autopilot) {
                // автопилот игрока — средний навык (тесты баланса), боты — не ниже 0.7
                let skill = if is_local && cli.autopilot { std::env::var("CARBANG_PLAYER_SKILL").ok().and_then(|v| v.parse().ok()).unwrap_or(0.7) } else { cfg.difficulty.max(0.7) };
                ec.insert(Bot::new(slot.bot_seed ^ 0xA070, skill));
            }
        } else {
            ec.insert(crate::net::client::NetInterp::default());
        }
        ec.insert(stats);
        if visuals {
            ec.with_children(|c| build_car_model(c, &mut model_assets, &car_cfg));
        }
    }
    ids.0 = 10_000;

    // ---------------- пешеходы и бонусы
    let peds = spawn_peds(&mut commands, &map, cfg.peds, cfg.seed, ped_assets.as_deref().filter(|_| visuals), cfg.zombies);
    spawn_pickups(
        &mut commands,
        &map.pickups,
        cfg.seed,
        cfg.mode == GameMode::Carnage,
        pickup_mats.as_deref().filter(|_| visuals),
    );

    commands.insert_resource(MatchState {
        phase: MatchPhase::Countdown,
        countdown: 3.5,
        elapsed: 0.0,
        time_left: cfg.time_limit,
        peds_total: peds,
        peds_alive: peds,
        winner: None,
        end_reason: String::new(),
    });
    commands.insert_resource(map);
    commands.insert_resource(matlib);
}

fn cleanup_race(mut commands: Commands, mut paused: ResMut<Paused>, mut time: ResMut<Time<Virtual>>) {
    commands.remove_resource::<MapData>();
    commands.remove_resource::<MatchState>();
    paused.0 = false;
    time.unpause();
}

/// Небо и солнце следуют за камерой, чтобы не упираться в край сферы.
#[allow(clippy::type_complexity)]
fn sky_follow(
    cam: Query<&Transform, (With<Camera3d>, Without<SkyDome>, Without<SunDisc>)>,
    mut sky: Query<&mut Transform, (With<SkyDome>, Without<SunDisc>)>,
    mut sun: Query<&mut Transform, (With<SunDisc>, Without<SkyDome>)>,
    map: Option<Res<MapData>>,
) {
    let Ok(c) = cam.single() else { return };
    let sun_dir = map.map(|m| m.theme.sun_dir).unwrap_or(Vec3::NEG_Y);
    for mut t in sky.iter_mut() {
        t.translation = c.translation;
    }
    for mut t in sun.iter_mut() {
        t.translation = c.translation - sun_dir * 1300.0;
    }
}

// ============================================================ ввод

fn axis(keys: &ButtonInput<KeyCode>, pos: &[KeyCode], neg: &[KeyCode]) -> f32 {
    let p = pos.iter().any(|k| keys.pressed(*k));
    let n = neg.iter().any(|k| keys.pressed(*k));
    (p as i32 - n as i32) as f32
}

#[allow(clippy::too_many_arguments)]
fn gather_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    pads: Query<&Gamepad>,
    mut input: ResMut<LocalInput>,
    mut toggles: ResMut<UiToggles>,
    paused: Res<Paused>,
    time: Res<Time>,
    mut steer_state: Local<f32>,
) {
    if paused.0 {
        input.0 = CarInput::default();
        return;
    }
    use KeyCode::*;
    let dt = time.delta_secs();
    let mut i = CarInput {
        throttle: axis(&keys, &[KeyW, ArrowUp], &[]),
        brake: axis(&keys, &[KeyS, ArrowDown], &[]),
        steer: 0.0,
        handbrake: keys.pressed(Space),
        nitro: keys.pressed(ShiftLeft) || keys.pressed(ShiftRight),
        fire: keys.pressed(ControlLeft) || keys.pressed(KeyJ) || mouse.pressed(MouseButton::Left),
        special: keys.pressed(KeyE) || keys.pressed(KeyK) || mouse.pressed(MouseButton::Right),
        drop: keys.pressed(KeyQ) || keys.pressed(KeyL),
        jump: keys.pressed(KeyF),
        recover: keys.pressed(KeyR),
        horn: keys.pressed(KeyH),
    };
    // Клавиатурный руль: плавное нарастание, быстрый возврат
    let target = axis(&keys, &[KeyD, ArrowRight], &[KeyA, ArrowLeft]);
    let rate = if target == 0.0 || target.signum() != steer_state.signum() { 8.0 } else { 4.5 };
    *steer_state = move_toward(*steer_state, target, rate * dt);
    i.steer = *steer_state;

    for pad in pads.iter() {
        let sx = pad.get(GamepadAxis::LeftStickX).unwrap_or(0.0);
        if sx.abs() > 0.12 {
            let s = (sx.abs() - 0.12) / 0.88 * sx.signum();
            i.steer = s * s.abs().powf(0.4);
        }
        let rt = pad.get(GamepadButton::RightTrigger2).unwrap_or(0.0);
        let lt = pad.get(GamepadButton::LeftTrigger2).unwrap_or(0.0);
        if rt > 0.05 {
            i.throttle = i.throttle.max(rt);
        }
        if lt > 0.05 {
            i.brake = i.brake.max(lt);
        }
        i.handbrake |= pad.pressed(GamepadButton::South);
        i.nitro |= pad.pressed(GamepadButton::East) || pad.pressed(GamepadButton::LeftTrigger);
        i.fire |= pad.pressed(GamepadButton::West) || pad.pressed(GamepadButton::RightTrigger);
        i.special |= pad.pressed(GamepadButton::North);
        i.drop |= pad.pressed(GamepadButton::DPadDown);
        i.jump |= pad.pressed(GamepadButton::DPadUp);
        i.recover |= pad.pressed(GamepadButton::Select);
        i.horn |= pad.pressed(GamepadButton::LeftThumb);
        if pad.just_pressed(GamepadButton::RightThumb) {
            toggles.camera_mode = (toggles.camera_mode + 1) % 3;
        }
        toggles.look_back = pad.pressed(GamepadButton::DPadLeft);
    }
    if keys.just_pressed(KeyC) {
        toggles.camera_mode = (toggles.camera_mode + 1) % 3;
    }
    toggles.look_back |= keys.pressed(KeyV);
    if !keys.pressed(KeyV) && pads.iter().all(|p| !p.pressed(GamepadButton::DPadLeft)) {
        toggles.look_back = false;
    }
    toggles.scoreboard = keys.pressed(Tab) || pads.iter().any(|p| p.pressed(GamepadButton::Start) && p.pressed(GamepadButton::Select));
    input.0 = i;
}

fn apply_local_input(input: Res<LocalInput>, local: Res<LocalPlayer>, mut cars: Query<(&NetId, &Car, &mut CarInput)>, cli: Res<crate::Cli>) {
    if cli.autopilot {
        return;
    }
    let Some(id) = local.car else { return };
    for (nid, car, mut ci) in cars.iter_mut() {
        if *nid == id && car.controller == Controller::Local {
            *ci = input.0;
        }
    }
}

fn pause_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    mut paused: ResMut<Paused>,
    mode: Res<NetMode>,
    mut time: ResMut<Time<Virtual>>,
) {
    let pressed = keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::KeyP) || pads.iter().any(|p| p.just_pressed(GamepadButton::Start));
    if pressed {
        paused.0 = !paused.0;
    }
    if *mode == NetMode::Offline {
        if paused.0 && !time.is_paused() {
            time.pause();
        } else if !paused.0 && time.is_paused() {
            time.unpause();
        }
    }
}
