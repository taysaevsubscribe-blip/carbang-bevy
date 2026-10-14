//! Физика машины: лучевая подвеска, модель шины по углу увода, ассисты, урон от столкновений,
//! переворот и эвакуация. Перенос проверенной модели из Godot-прототипа.

use crate::balance::*;
use crate::config::*;
use crate::data::*;
use crate::game::*;
use crate::maps::MapData;
use avian3d::prelude::*;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const G: f32 = 9.81;

// ============================================================ компоненты

#[derive(Component, Clone)]
pub struct Car {
    pub name: String,
    pub controller: Controller,
    pub cfg: CarConfig,
    pub bot_seed: u64,
}

impl Car {
    pub fn def(&self) -> &'static CarDef {
        self.cfg.def()
    }
}

#[derive(Component, Default, Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct CarInput {
    pub throttle: f32,
    pub brake: f32,
    pub steer: f32,
    pub handbrake: bool,
    pub nitro: bool,
    pub fire: bool,
    pub special: bool,
    pub drop: bool,
    pub jump: bool,
    pub recover: bool,
    pub horn: bool,
}

#[derive(Clone, Copy, Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct WheelVis {
    pub comp: f32,
    pub steer: f32,
    pub spin: f32,
    pub slip: f32,
    pub grounded: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct WeaponHud {
    pub kind: WeaponKind,
    pub mount: Mount,
    pub ammo: u32,
    pub max_ammo: u32,
    pub ready: f32,
}

/// Видимое состояние машины. Сервер считает, клиенты получают в снапшотах.
#[derive(Component, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CarState {
    pub hp: f32,
    /// Броня: принимает на себя часть урона, пока не кончится
    pub armor: f32,
    /// Неуязвимость после старта и воскрешения, с
    pub spawn_protect: f32,
    /// Режим коробки: -1 задний ход, 0 нейтраль, 1 вперёд
    pub drive: i8,
    /// Урон по зонам: перед, зад, лево, право
    pub zones: [f32; 4],
    pub wrecked: bool,
    pub nitro: f32,
    pub nitro_max: f32,
    pub nitro_active: bool,
    pub speed: f32,
    pub rpm: f32,
    pub gear: u8,
    pub braking: bool,
    pub wheels: [WheelVis; 4],
    /// Оставшееся время эффектов по порядку TIMED_EFFECTS
    pub effects: [f32; 9],
    pub springs: u8,
    pub emp: f32,
    pub burning: f32,
    pub frozen: f32,
    pub oiled: f32,
    pub weapons: Vec<WeaponHud>,
    pub lock_target: Option<NetId>,
    pub lock_frac: f32,
    pub respawn_in: f32,
    pub air: f32,
    pub turret_yaw: f32,
}

impl Default for CarState {
    fn default() -> Self {
        Self {
            hp: START_HP,
            armor: START_ARMOR,
            spawn_protect: SPAWN_PROTECT,
            drive: 0,
            zones: [0.0; 4],
            wrecked: false,
            nitro: 0.0,
            nitro_max: 0.0,
            nitro_active: false,
            speed: 0.0,
            rpm: 900.0,
            gear: 1,
            braking: false,
            wheels: [WheelVis::default(); 4],
            effects: [0.0; 9],
            springs: 0,
            emp: 0.0,
            burning: 0.0,
            frozen: 0.0,
            oiled: 0.0,
            weapons: Vec::new(),
            lock_target: None,
            lock_frac: 0.0,
            respawn_in: 0.0,
            air: 0.0,
            turret_yaw: 0.0,
        }
    }
}

/// Повреждённые узлы машины.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Nodes {
    pub radiator: bool,
    pub engine: bool,
    /// -1 колесо слева, 1 справа, 0 целы
    pub wheel: i8,
}

impl CarState {
    pub fn nodes(&self) -> Nodes {
        let left = self.zones[2] >= WHEEL_ZONE;
        let right = self.zones[3] >= WHEEL_ZONE;
        Nodes {
            radiator: self.zones[0] >= RADIATOR_ZONE,
            engine: self.zones[0] >= ENGINE_ZONE || (self.hp < ENGINE_HP && !self.wrecked),
            wheel: if left && !right { -1 } else if right && !left { 1 } else if left && right { if self.zones[2] > self.zones[3] { -1 } else { 1 } } else { 0 },
        }
    }

    pub fn effect(&self, kind: PowerupKind) -> f32 {
        TIMED_EFFECTS.iter().position(|k| *k == kind).map(|i| self.effects[i]).unwrap_or(0.0)
    }
    pub fn has(&self, kind: PowerupKind) -> bool {
        self.effect(kind) > 0.0
    }
    pub fn set_effect(&mut self, kind: PowerupKind, t: f32) {
        if let Some(i) = TIMED_EFFECTS.iter().position(|k| *k == kind) {
            self.effects[i] = self.effects[i].max(t);
        }
    }
    pub fn clear_effect(&mut self, kind: PowerupKind) {
        if let Some(i) = TIMED_EFFECTS.iter().position(|k| *k == kind) {
            self.effects[i] = 0.0;
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct WheelSim {
    pub mount: Vec3,
    pub front: bool,
    pub driven: bool,
    pub comp: f32,
    pub spin: f32,
    pub slip: f32,
    pub contact: bool,
    pub surface: f32,
}

/// Внутреннее состояние физики (только на сервере).
#[derive(Component)]
pub struct CarSim {
    pub wheels: [WheelSim; 4],
    pub steer: f32,
    pub spring_k: f32,
    pub damper_c: f32,
    pub antiroll_k: f32,
    pub half_w: f32,
    pub half_l: f32,
    pub grounded: u32,
    pub air_time: f32,
    pub upside_timer: f32,
    pub stuck_timer: f32,
    pub last_attacker: Option<Entity>,
    pub last_attacker_time: f32,
    pub last_hit_time: f32,
    pub time: f32,
    pub jump_cooldown: f32,
    pub second_wind_used: bool,
    pub prev_vel: Vec3,
    /// Сколько секунд держится тормоз (для включения заднего хода)
    pub brake_hold: f32,
    pub recover_cooldown: f32,
}

/// Тип урона влияет на модификаторы (броня от взрывов, перки и т.д.).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DamageKind {
    Ram,
    Wall,
    Bullet,
    Explosion,
    Fire,
    Fall,
    Hazard,
    Emp,
}

#[derive(Message, Clone, Copy, Debug)]
pub struct DamageEvent {
    pub victim: Entity,
    pub attacker: Option<Entity>,
    pub amount: f32,
    pub kind: DamageKind,
    /// Направление удара в локальных координатах жертвы (для зоны)
    pub local_dir: Vec3,
    pub pos: Vec3,
    /// Скорость сближения при ударе (м/с), 0 для оружия и прочего
    pub impact_speed: f32,
}

/// Запрос эвакуации машины на дорогу.
#[derive(Message, Clone, Copy)]
pub struct RecoverEvent {
    pub car: Entity,
    pub penalty: bool,
}

/// Метка визуальных колёс.
#[derive(Component)]
pub struct WheelVisual(pub usize);

// ============================================================ геометрия

pub fn wheel_mounts(def: &CarDef) -> [Vec3; 4] {
    let half_l = def.size.z * 0.5;
    let half_w = def.size.x * 0.5;
    let (wz_f, wz_r) = match def.style {
        BodyStyle::Bus => (half_l * 0.72, half_l * 0.62),
        BodyStyle::Mixer => (half_l * 0.74, half_l * 0.66),
        BodyStyle::Limo => (half_l * 0.74, half_l * 0.74),
        BodyStyle::Monster => (half_l * 0.62, half_l * 0.62),
        _ => (half_l * 0.68, half_l * 0.68),
    };
    let wx = match def.style {
        BodyStyle::Monster => half_w + 0.15,
        BodyStyle::Buggy => half_w + 0.05,
        _ => half_w - 0.05,
    };
    let my = wheel_mount_y(def);
    [
        Vec3::new(-wx, my, -wz_f),
        Vec3::new(wx, my, -wz_f),
        Vec3::new(-wx, my, wz_r),
        Vec3::new(wx, my, wz_r),
    ]
}

pub fn collider_for(def: &CarDef) -> Collider {
    let s = def.size;
    let base = Collider::cuboid(s.x, s.y + 0.25, s.z);
    let cabin_h = match def.style {
        BodyStyle::Bus | BodyStyle::IceCream | BodyStyle::Mixer | BodyStyle::ArmoredVan => 0.0,
        BodyStyle::Buggy | BodyStyle::GolfCart => 0.5,
        BodyStyle::Monster => 0.6,
        _ => 0.5,
    };
    if cabin_h > 0.0 {
        Collider::compound(vec![
            (Vec3::new(0.0, 0.12, 0.0), Quat::IDENTITY, base),
            (
                Vec3::new(0.0, 0.12 + (s.y + 0.25) * 0.5 + cabin_h * 0.5, s.z * 0.05),
                Quat::IDENTITY,
                Collider::cuboid(s.x * 0.8, cabin_h, s.z * 0.45),
            ),
        ])
    } else {
        Collider::compound(vec![(Vec3::new(0.0, 0.12, 0.0), Quat::IDENTITY, base)])
    }
}

pub fn car_inertia(def: &CarDef, mass: f32) -> Vec3 {
    let x = def.size.x;
    let y = def.size.y + 0.25;
    let z = def.size.z;
    Vec3::new(
        mass / 12.0 * (y * y + z * z),
        mass / 12.0 * (x * x + z * z),
        mass / 12.0 * (x * x + y * y),
    )
}

pub fn make_sim(def: &CarDef, stats: &CarStats) -> CarSim {
    let mounts = wheel_mounts(def);
    let per_wheel = stats.mass * G / 4.0;
    let spring_k = per_wheel / (stats.rest_len * 0.42) * stats.spring_mult;
    let damper_c = 2.0 * 0.32 * (spring_k * stats.mass / 4.0).sqrt();
    let antiroll_k = spring_k * 0.55 * stats.antiroll;
    let mut wheels = [WheelSim::default(); 4];
    for i in 0..4 {
        let front = i < 2;
        wheels[i] = WheelSim {
            mount: mounts[i],
            front,
            driven: match stats.drive {
                Drive::Awd => true,
                Drive::Fwd => front,
                Drive::Rwd => !front,
            },
            surface: 1.0,
            ..default()
        };
    }
    CarSim {
        wheels,
        steer: 0.0,
        spring_k,
        damper_c,
        antiroll_k,
        half_w: def.size.x * 0.5,
        half_l: def.size.z * 0.5,
        grounded: 0,
        air_time: 0.0,
        upside_timer: 0.0,
        stuck_timer: 0.0,
        last_attacker: None,
        last_attacker_time: -100.0,
        last_hit_time: -100.0,
        time: 0.0,
        jump_cooldown: 0.0,
        second_wind_used: false,
        prev_vel: Vec3::ZERO,
        brake_hold: 0.0,
        recover_cooldown: 0.0,
    }
}

/// Физические компоненты машины (только у того, кто считает физику).
pub fn physics_bundle(def: &CarDef, stats: &CarStats) -> impl Bundle {
    (
        RigidBody::Dynamic,
        collider_for(def),
        Mass(stats.mass),
        AngularInertia::new(car_inertia(def, stats.mass)),
        CenterOfMass::new(0.0, -0.32, 0.0),
        NoAutoMass,
        NoAutoAngularInertia,
        NoAutoCenterOfMass,
        (
            Friction::new(0.35),
            Restitution::new(0.05),
            LinearDamping(0.0),
            AngularDamping(0.5),
            ExternalForce::default(),
            ExternalTorque::default(),
            ExternalImpulse::default(),
            CollisionLayers::new(Layer::Car, [Layer::World, Layer::Car, Layer::Prop]),
            SweptCcd::default(),
            ImpactAccum::default(),
        ),
    )
}


// ============================================================ внешние силы
// avian 0.7 убрал ExternalForce/Torque/Impulse в пользу QueryData `Forces`.
// Игровой код копит силы в этих компонентах, а `flush_external_forces`
// в конце FixedUpdate передаёт их в avian одним вызовом на тело.

/// Сила за шаг (с моментом от точки приложения), сбрасывается после передачи в avian.
#[derive(Component, Default, Clone, Copy)]
pub struct ExternalForce {
    pub force: Vec3,
    pub torque: Vec3,
}

impl ExternalForce {
    pub fn apply_force(&mut self, f: Vec3) -> &mut Self {
        self.force += f;
        self
    }
    /// Сила в мировой точке; `com` — мировой центр масс тела.
    pub fn apply_force_at_point(&mut self, f: Vec3, point: Vec3, com: Vec3) -> &mut Self {
        self.force += f;
        self.torque += (point - com).cross(f);
        self
    }
}

#[derive(Component, Default, Clone, Copy)]
pub struct ExternalTorque(pub Vec3);

impl ExternalTorque {
    pub fn apply_torque(&mut self, t: Vec3) -> &mut Self {
        self.0 += t;
        self
    }
}

/// Мгновенный импульс (удар, взрыв, прыжок).
#[derive(Component, Default, Clone, Copy)]
pub struct ExternalImpulse(pub Vec3);

impl ExternalImpulse {
    pub fn apply_impulse(&mut self, i: Vec3) -> &mut Self {
        self.0 += i;
        self
    }
}

/// Всем динамическим телам нужен накопитель импульса (пропсы, обломки).
pub fn ensure_external_impulse(mut commands: Commands, q: Query<(Entity, &RigidBody), (Added<RigidBody>, Without<ExternalImpulse>)>) {
    for (e, rb) in q.iter() {
        if *rb == RigidBody::Dynamic {
            commands.entity(e).try_insert(ExternalImpulse::default());
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn flush_external_forces(
    mut q: Query<(Forces, Option<&mut ExternalForce>, Option<&mut ExternalTorque>, Option<&mut ExternalImpulse>)>,
) {
    for (mut forces, f, t, i) in q.iter_mut() {
        if let Some(mut f) = f {
            if f.force != Vec3::ZERO {
                forces.apply_force(f.force);
            }
            if f.torque != Vec3::ZERO {
                forces.apply_torque(f.torque);
            }
            *f = ExternalForce::default();
        }
        if let Some(mut t) = t {
            if t.0 != Vec3::ZERO {
                forces.apply_torque(t.0);
            }
            t.0 = Vec3::ZERO;
        }
        if let Some(mut i) = i {
            if i.0 != Vec3::ZERO {
                forces.apply_linear_impulse(i.0);
            }
            i.0 = Vec3::ZERO;
        }
    }
}

// ============================================================ плагин

pub struct VehiclePlugin;

impl Plugin for VehiclePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DamageEvent>()
            .add_message::<RecoverEvent>()
            .add_systems(
                FixedUpdate,
                (
                    tick_effects,
                    drive_cars,
                    collision_damage,
                    check_flip_and_falls,
                    do_recover,
                )
                    .chain()
                    .in_set(SimSet::Vehicle)
                    .run_if(is_authority)
                    .run_if(in_state(AppState::Race)),
            )
            .add_systems(FixedUpdate, (ensure_external_impulse, flush_external_forces).chain().after(SimSet::Rules))
            .add_systems(Update, update_wheel_visuals);
    }
}

// ============================================================ эффекты и таймеры

fn tick_effects(
    time: Res<Time>,
    mut cars: Query<(Entity, &mut CarState, &CarStats, &mut CarSim, &Car)>,
    mut dmg: MessageWriter<DamageEvent>,
    ms: Option<Res<MatchState>>,
) {
    let dt = time.delta_secs();
    let running = ms.map(|m| m.phase == MatchPhase::Running).unwrap_or(true);
    for (e, mut st, stats, mut sim, car) in cars.iter_mut() {
        sim.time += dt;
        sim.jump_cooldown = (sim.jump_cooldown - dt).max(0.0);
        sim.recover_cooldown = (sim.recover_cooldown - dt).max(0.0);
        for e in st.effects.iter_mut() {
            *e = (*e - dt).max(0.0);
        }
        if !st.has(PowerupKind::Springs) {
            st.springs = 0;
        }
        st.emp = (st.emp - dt).max(0.0);
        st.frozen = (st.frozen - dt).max(0.0);
        st.oiled = (st.oiled - dt).max(0.0);
        if running {
            st.spawn_protect = (st.spawn_protect - dt).max(0.0);
            // броня понемногу восстанавливается, если машину 4 с не били
            if !st.wrecked && sim.time - sim.last_hit_time > ARMOR_REGEN_DELAY {
                st.armor = (st.armor + ARMOR_REGEN * dt).min(START_ARMOR);
            }
        }
        st.nitro_max = stats.nitro_max;
        if st.wrecked {
            st.burning = 0.0;
            continue;
        }
        if !st.nitro_active {
            st.nitro = (st.nitro + dt * stats.nitro_regen * stats.nitro_max).min(stats.nitro_max);
        }
        if st.burning > 0.0 {
            st.burning = (st.burning - dt).max(0.0);
            dmg.write(DamageEvent {
                victim: e,
                attacker: sim.last_attacker,
                amount: 6.0 * dt,
                kind: DamageKind::Fire,
                local_dir: Vec3::ZERO,
                pos: Vec3::ZERO,
impact_speed: 0.0,
});
        }
        // Перк «Механик»
        if running && car.cfg.has_perk(PerkKind::Mechanic) && sim.time - sim.last_hit_time > 5.0 && st.hp < 100.0 {
            st.hp = (st.hp + 0.6 * dt).min(100.0);
        }
    }
}

// ============================================================ езда

#[allow(clippy::type_complexity)]
fn drive_cars(
    time: Res<Time>,
    spatial: SpatialQuery,
    map: Option<Res<MapData>>,
    mut q: Query<(
        Entity,
        &Car,
        &CarInput,
        &CarStats,
        &mut CarSim,
        &mut CarState,
        &Position,
        &Rotation,
        &LinearVelocity,
        &AngularVelocity,
        &mut ExternalForce,
        &mut ExternalTorque,
        &mut ExternalImpulse,
    )>,
    ms: Option<Res<MatchState>>,
    mut events: MessageWriter<GameEvent>,
    ids: Query<&NetId>,
    dyn_diff: Option<Res<crate::balance::DynDifficulty>>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let running = ms.as_ref().map(|m| m.phase == MatchPhase::Running).unwrap_or(true);
    let since_start = ms.as_ref().map(|m| m.elapsed).unwrap_or(10.0);
    for (entity, car, input, stats, mut sim, mut st, pos, rot, linvel, angvel, mut force, mut torque, mut impulse) in q.iter_mut() {
        let dyn_speed = if car.controller == Controller::Bot { dyn_diff.as_ref().map(|d| d.speed).unwrap_or(1.0) } else { 1.0 };
        let rot = rot.0;
        let up = rot * Vec3::Y;
        let fwd = rot * Vec3::NEG_Z;
        let right = rot * Vec3::X;
        let v = linvel.0;
        let w = angvel.0;
        let com = pos.0 + rot * Vec3::new(0.0, -0.32, 0.0);
        let speed = v.dot(fwd);
        let aspeed = speed.abs();
        let mass = stats.mass;

        // Ввод с мёртвой зоной: шум геймпада не даёт ни газа, ни тормоза, ни заднего хода
        let mut throttle = deadzone(input.throttle.clamp(0.0, 1.0), INPUT_DEADZONE);
        let mut brake = deadzone(input.brake.clamp(0.0, 1.0), INPUT_DEADZONE);
        let mut steer_in = deadzone(input.steer.clamp(-1.0, 1.0), INPUT_DEADZONE);
        let mut handbrake = input.handbrake;
        // До START: нейтраль, ручник и колёсные тормоза, ввод не применяется
        let hold = !running;
        if hold {
            throttle = 0.0;
            brake = 1.0;
            steer_in = 0.0;
            handbrake = true;
            st.drive = 0;
            sim.brake_hold = 0.0;
        }
        if st.wrecked {
            throttle = 0.0;
            brake = 0.3;
            steer_in = 0.0;
        }
        if st.frozen > 0.0 {
            throttle = 0.0;
            brake = 1.0;
        }
        if st.emp > 0.0 {
            throttle = 0.0;
        }

        // Коробка: N → D по газу; R только после 0.5 с от START, на скорости < 5 км/ч
        // и при тормозе, который держат дольше 0.3 с
        if !hold {
            if brake > 0.0 && throttle < 0.1 {
                sim.brake_hold += dt;
            } else {
                sim.brake_hold = 0.0;
            }
            let can_reverse = may_reverse(since_start, speed, sim.brake_hold);
            match st.drive {
                -1 => {
                    if throttle > 0.1 && speed > -1.0 {
                        st.drive = 1;
                    }
                }
                0 => {
                    if throttle > 0.1 {
                        st.drive = 1;
                    } else if can_reverse {
                        st.drive = -1;
                    }
                }
                _ => {
                    if can_reverse {
                        st.drive = -1;
                    }
                }
            }
        }

        // Узлы: радиатор режет тягу, двигатель — максималку, колесо уводит в сторону
        let nodes = st.nodes();
        let power_mult = if nodes.radiator { RADIATOR_THRUST } else { 1.0 } * dyn_speed;
        let pull = nodes.wheel as f32 * WHEEL_PULL;
        let mut grip_mult = 1.0 - 0.12 * ((100.0 - st.hp) / 100.0).clamp(0.0, 1.0);
        if st.has(PowerupKind::SuperGrip) {
            grip_mult *= 1.6;
        }
        if st.oiled > 0.0 {
            grip_mult *= 0.3;
        }

        // Руль: угол ограничен тем, что шины могут реализовать
        let wheelbase = sim.half_l * 1.36;
        let mu_s = stats.grip * grip_mult;
        let grip_angle = (wheelbase * mu_s * G / (aspeed * aspeed).max(1.0)).atan() + 0.175;
        let mut max_steer = grip_angle.clamp(0.07, 36f32.to_radians());
        if handbrake {
            max_steer = 36f32.to_radians();
        }
        let mut target = steer_in * max_steer + pull.clamp(-0.05, 0.05);
        let ground_now = sim.wheels.iter().filter(|w| w.contact).count();
        if stats.countersteer > 0.0 && aspeed > 4.0 && ground_now >= 3 {
            let slip_ang = v.dot(right).atan2(v.dot(fwd).abs().max(1.0));
            target += (slip_ang * 0.45 * stats.countersteer).clamp(-0.25, 0.25) * (1.0 - steer_in.abs() * 0.5);
        }
        let rate = if target.abs() > sim.steer.abs() { 2.8 } else { 4.5 } * stats.steer_speed;
        sim.steer = move_toward(sim.steer, target, dt * rate);

        // Двигатель
        let mut power = stats.power * power_mult;
        st.nitro_active = false;
        if input.nitro && st.nitro > 0.0 && !st.wrecked && throttle > 0.1 && st.emp <= 0.0 {
            st.nitro_active = true;
            st.nitro = (st.nitro - dt).max(0.0);
            power *= 1.7;
        }
        let mut drive_force = 0.0;
        if hold {
            // ручник и тормоза держат машину на месте
        } else if st.drive == -1 {
            // задний ход: педаль тормоза работает как газ назад, газ — как тормоз
            drive_force = -brake * mass * 5.0 * power_mult;
            if speed < -12.0 {
                drive_force = 0.0;
            }
            brake = if throttle > 0.1 { throttle } else { 0.0 };
        } else if st.drive == 1 {
            let cap = stats.max_drive_force * if st.nitro_active { 1.3 } else { 1.0 };
            drive_force = throttle * cap.min(power / aspeed.max(1.0));
            if speed < -1.0 && throttle > 0.1 {
                brake = brake.max(throttle);
            }
        }
        let top = stats.top_speed * if st.nitro_active { 1.25 } else { 1.0 } * if nodes.engine { ENGINE_TOP_SPEED } else { 1.0 } * dyn_speed;
        if speed > top {
            drive_force *= (1.0 - (speed - top) / 3.0).clamp(0.0, 1.0);
        }
        if st.wrecked {
            drive_force = 0.0;
        }
        st.braking = brake > 0.1 && speed > 0.5;

        let driven_count = sim.wheels.iter().filter(|w| w.driven).count().max(1) as f32;

        // Подвеска: лучи вниз
        let max_len = stats.rest_len + stats.wheel_radius;
        let filter = SpatialQueryFilter::from_mask(Layer::World).with_excluded_entities([entity]);
        let mut new_comp = [0.0f32; 4];
        let mut hit_pos = [Vec3::ZERO; 4];
        let mut hit_n = [up; 4];
        let mut contact = [false; 4];
        let mut surface = [1.0f32; 4];
        let down = Dir3::new(-up).unwrap_or(Dir3::NEG_Y);
        for i in 0..4 {
            let from = pos.0 + rot * sim.wheels[i].mount;
            if let Some(hit) = spatial.cast_ray(from, down, max_len, true, &filter) {
                let comp = (max_len - hit.distance).clamp(0.0, stats.rest_len);
                new_comp[i] = comp;
                hit_pos[i] = from - up * hit.distance;
                hit_n[i] = hit.normal;
                contact[i] = true;
                if let Some(m) = map.as_ref() {
                    surface[i] = m.surface_grip(hit_pos[i]);
                }
            }
        }
        let ar_front = (new_comp[0] - new_comp[1]) * sim.antiroll_k;
        let ar_rear = (new_comp[2] - new_comp[3]) * sim.antiroll_k;
        let unflippable = stats.trait_kind == CarTrait::Unflippable;

        let mut grounded = 0;
        let mut max_slip = 0.0f32;
        for i in 0..4 {
            if !contact[i] {
                sim.wheels[i].comp = 0.0;
                sim.wheels[i].contact = false;
                sim.wheels[i].slip = 0.0;
                continue;
            }
            grounded += 1;
            let comp = new_comp[i];
            let comp_vel = (comp - sim.wheels[i].comp) / dt;
            sim.wheels[i].comp = comp;
            sim.wheels[i].contact = true;
            sim.wheels[i].surface = surface[i];
            let mut fs = sim.spring_k * comp + sim.damper_c * comp_vel;
            if comp > stats.rest_len * 0.85 {
                fs += (comp - stats.rest_len * 0.85) * sim.spring_k * 6.0;
            }
            let ar = if i < 2 { ar_front } else { ar_rear };
            fs += if i % 2 == 0 { ar } else { -ar };
            fs = fs.clamp(0.0, mass * G * 3.0);
            let mount_w = pos.0 + rot * sim.wheels[i].mount;
            force.apply_force_at_point(up * fs, mount_w, com);

            // Шина
            let n = hit_n[i];
            let mut wf = fwd;
            if sim.wheels[i].front {
                wf = Quat::from_axis_angle(up, -sim.steer) * fwd;
            }
            wf = (wf - n * wf.dot(n)).normalize_or_zero();
            let wr = wf.cross(n).normalize_or_zero();
            let rel = hit_pos[i] - com;
            let pv = v + w.cross(rel);
            let vl = pv.dot(wf);
            let vs = pv.dot(wr);
            let base_mu = if sim.wheels[i].front { stats.grip_front } else { stats.grip_rear };
            let mu = base_mu * grip_mult * surface[i];
            let fz = fs.min(mass * G * 1.2);
            let share = mass * 0.25 / dt;
            let rear = !sim.wheels[i].front;
            let mut lat_mu = mu;
            if handbrake && rear {
                lat_mu *= 0.42;
            }
            let slip = vs.abs().atan2(vl.abs() + 0.8);
            let peak = 0.14;
            let curve = if slip < peak {
                slip / peak
            } else {
                1.0 + (0.78 - 1.0) * ((slip - peak) / 0.5).clamp(0.0, 1.0)
            };
            let mut fy = -vs.signum() * curve * lat_mu * fz;
            fy = fy.clamp(-vs.abs() * share, vs.abs() * share);

            let mut fx = 0.0;
            if sim.wheels[i].driven {
                fx += drive_force / driven_count;
            }
            if brake > 0.0 {
                let bias = if sim.wheels[i].front { stats.brake_front * 2.0 } else { (1.0 - stats.brake_front) * 2.0 };
                let bf = brake * mu * fz * 0.95 * bias;
                fx -= vl.signum() * bf.min(vl.abs() * share);
            }
            if handbrake && rear {
                fx -= vl.signum() * (mu * fz * 0.75).min(vl.abs() * share);
            }
            fx -= vl * mass * 0.004;

            let limit = mu * fz * 1.1;
            let mut total = Vec2::new(fx, fy);
            let tl = total.length();
            let mut wheelspin = 0.0;
            if tl > limit && tl > 0.001 {
                wheelspin = ((tl - limit) / limit).clamp(0.0, 1.0);
                total *= limit / tl;
            }
            let slip_vis = wheelspin.max((slip - peak).clamp(0.0, 1.0));
            sim.wheels[i].slip = slip_vis;
            max_slip = max_slip.max(slip_vis);
            let apply_at = hit_pos[i] + up * 0.22;
            force.apply_force_at_point(wf * total.x + wr * total.y, apply_at, com);
            sim.wheels[i].spin += (vl / stats.wheel_radius + wheelspin * 30.0 * drive_force.signum()) * dt;
        }
        // Колёса в воздухе крутятся по инерции
        for i in 0..4 {
            if !contact[i] {
                let drive_spin = if sim.wheels[i].driven { throttle * 40.0 } else { 0.0 };
                sim.wheels[i].spin += (speed / stats.wheel_radius * 0.98 + drive_spin) * dt;
            }
        }
        sim.grounded = grounded;

        // Аэродинамика
        force.apply_force(-v * v.length() * stats.drag);
        if grounded > 0 {
            force.apply_force(-up * speed * speed * 0.35);
        }

        // Стабилизация рыскания: гасит только лишнее вращение
        if stats.stability > 0.0 && grounded >= 3 && !handbrake {
            let yaw = w.dot(up);
            let desired = -sim.steer * speed / (wheelbase + 0.001);
            let excess = yaw - desired;
            if excess.signum() == yaw.signum() || desired.abs() < 0.05 {
                torque.apply_torque(-up * excess * mass * 2.2 * stats.stability);
            }
        }

        if st.nitro_active && grounded > 0 {
            force.apply_force(fwd * mass * 3.0);
        }

        // Лунная гравитация: часть веса снимается
        if st.has(PowerupKind::MoonGravity) {
            force.apply_force(Vec3::Y * mass * G * 0.65);
        }

        // Пружины: прыжок
        if input.jump && st.springs > 0 && grounded >= 2 && sim.jump_cooldown <= 0.0 && !st.wrecked {
            st.springs -= 1;
            sim.jump_cooldown = 0.8;
            impulse.apply_impulse(Vec3::Y * mass * 9.0 + fwd * mass * 2.0);
            if let Ok(id) = ids.get(entity) {
                events.write(GameEvent::Message {
                    to: Some(*id),
                    text: format!("ПРЫЖОК! осталось {}", st.springs),
                    color: [0.9, 1.0, 0.2],
                    big: false,
                });
            }
        }

        // В воздухе: стабилизация и управление
        if grounded == 0 {
            sim.air_time += dt;
            let err = up.cross(Vec3::Y);
            let k = if unflippable { 14.0 } else { 6.0 };
            torque.apply_torque(err * mass * k - w * mass * 0.6);
            if car.controller != Controller::Bot || stats.air_control > 1.0 {
                torque.apply_torque(right * (throttle - input.brake) * mass * 1.2 * stats.air_control);
                torque.apply_torque(up * (-steer_in) * mass * 0.8 * stats.air_control);
            }
        } else {
            if sim.air_time > 0.6 {
                if let Ok(id) = ids.get(entity) {
                    events.write(GameEvent::Landed { car: *id, air: sim.air_time });
                }
            }
            sim.air_time = 0.0;
        }
        st.air = sim.air_time;

        // Неваляшка: даже на земле не даёт завалиться
        if unflippable && up.y < 0.7 {
            let err = up.cross(Vec3::Y);
            torque.apply_torque(err * mass * 20.0);
        }

        // Переворот
        if up.y < 0.25 && v.length() < 3.0 {
            sim.upside_timer += dt;
        } else {
            sim.upside_timer = 0.0;
        }

        // Обороты и передачи для звука и HUD
        let ratios = [0.0, 13.0, 23.0, 33.0, 44.0, 56.0, 70.0];
        let mut gear = 1;
        for g in 1..6 {
            if aspeed > ratios[g] * 0.92 * stats.top_speed / 52.0 {
                gear = g + 1;
            }
        }
        let scale = stats.top_speed / 52.0;
        let lo = ratios[gear - 1] * scale;
        let hi = ratios[gear] * scale;
        let t = ((aspeed - lo * 0.7) / (hi - lo * 0.7).max(1.0)).clamp(0.0, 1.0);
        let mut target_rpm = 1500.0 + t * 5200.0;
        if aspeed < 1.0 {
            target_rpm = 900.0 + throttle * 3000.0;
        }
        if grounded == 0 {
            target_rpm = 1500.0 + throttle * 5500.0;
        }
        st.rpm += (target_rpm - st.rpm) * 0.2;
        st.gear = if speed < -0.5 { 0 } else { gear as u8 };
        st.speed = speed;
        for i in 0..4 {
            st.wheels[i] = WheelVis {
                comp: sim.wheels[i].comp,
                steer: if sim.wheels[i].front { sim.steer } else { 0.0 },
                spin: sim.wheels[i].spin % std::f32::consts::TAU,
                slip: sim.wheels[i].slip,
                grounded: sim.wheels[i].contact,
            };
        }
        let _ = max_slip;
        sim.prev_vel = v;
    }
}

// ============================================================ урон от столкновений

fn zone_of(local: Vec3, half_w: f32, half_l: f32) -> usize {
    if local.z.abs() / half_l.max(0.1) > local.x.abs() / half_w.max(0.1) {
        if local.z < 0.0 { 0 } else { 1 }
    } else if local.x < 0.0 {
        2
    } else {
        3
    }
}

pub fn mount_for_zone(zone: usize) -> Mount {
    match zone {
        0 => Mount::Front,
        1 => Mount::Rear,
        _ => Mount::Sides,
    }
}

/// Один удар с конкретным телом: импульс копится несколько шагов физики,
/// урон считается один раз по суммарному Δv.
#[derive(Clone, Copy, Debug)]
pub struct ImpactEpisode {
    pub other: Entity,
    pub impulse: f32,
    pub start_t: f32,
    pub last_t: f32,
    pub local_normal: Vec3,
    pub point: Vec3,
    /// Скорость сближения до удара (м/с)
    pub closing: f32,
    pub max_step_impulse: f32,
    /// Другой — машина, которая налетела на меня (а не я на неё)
    pub other_attacks: bool,
    /// Другой ударил передом
    pub other_front: bool,
}

#[derive(Component, Default, Clone, Debug)]
pub struct ImpactAccum(pub Vec<ImpactEpisode>, pub Vec3, pub f32);

#[allow(clippy::type_complexity)]
fn collision_damage(
    collisions: Collisions,
    cars: Query<(&Car, &CarStats, &CarSim, &CarState, &LinearVelocity, &Position, &Rotation)>,
    mut accums: Query<(Entity, &mut ImpactAccum, &LinearVelocity)>,
    statics: Query<(), Or<(With<RigidBody>, With<Collider>)>>,
    kinematic: Query<&RigidBody>,
    props: Query<(), With<crate::maps::Prop>>,
    mut dmg: MessageWriter<DamageEvent>,
    mut impulses: Query<&mut ExternalImpulse>,
    mut events: MessageWriter<GameEvent>,
    masses: Query<(&ComputedMass, &RigidBody)>,
) {
    // 1. Копим импульс касаний этого шага по парам «машина — другое тело»
    for contacts in collisions.iter() {
        if contacts.manifolds.is_empty() {
            continue;
        }
        let impulse = contacts.total_normal_impulse_magnitude();
        if impulse <= 0.0 {
            continue;
        }
        let e1 = contacts.body1.unwrap_or(contacts.collider1);
        let e2 = contacts.body2.unwrap_or(contacts.collider2);
        let manifold = &contacts.manifolds[0];
        let world_contact = manifold.points.first().map(|c| c.point);
        for (me, other, world_normal) in [(e1, e2, manifold.normal), (e2, e1, -manifold.normal)] {
            let Ok((_car, _stats, sim, st, _vel, pos, rot)) = cars.get(me) else { continue };
            if st.wrecked {
                continue;
            }
            let Ok((_, mut acc, _)) = accums.get_mut(me) else { continue };
            let my_before = acc.1;
            let now = sim.time;
            if let Some(ep) = acc.0.iter_mut().find(|e| e.other == other) {
                ep.impulse += impulse;
                ep.last_t = now;
                if impulse > ep.max_step_impulse {
                    ep.max_step_impulse = impulse;
                    ep.local_normal = rot.0.inverse() * world_normal;
                    ep.point = world_contact.unwrap_or(pos.0);
                }
                continue;
            }
            // Кто на кого налетел — по скоростям до удара (сохранены шагом раньше)
            let other_before = accums.get(other).map(|o| o.1 .1).unwrap_or(Vec3::ZERO);
            let (mut other_attacks, mut other_front) = (false, false);
            if let Ok((_, _, _osim, _, _, opos, orot)) = cars.get(other) {
                let mut dir = opos.0 - pos.0;
                dir.y = 0.0;
                let dir = dir.normalize_or_zero();
                let mine = my_before.dot(dir);
                let theirs = other_before.dot(-dir);
                other_attacks = theirs > mine;
                other_front = (orot.0 * Vec3::NEG_Z).dot(-dir) > 0.55;
            }
            let Ok((_, mut acc, _)) = accums.get_mut(me) else { continue };
            acc.0.push(ImpactEpisode {
                other,
                impulse,
                start_t: now,
                last_t: now,
                local_normal: rot.0.inverse() * world_normal,
                point: world_contact.unwrap_or(pos.0),
                closing: (my_before - other_before).length(),
                max_step_impulse: impulse,
                other_attacks,
                other_front,
            });
        }
    }

    // 2. Закрываем удары: касание кончилось или окно истекло
    let mut finished: Vec<(Entity, ImpactEpisode)> = Vec::new();
    for (me, mut acc, lv) in accums.iter_mut() {
        // скорость до следующего шага физики — «до удара» для следующего кадра
        acc.1 = lv.0;
        let Ok((_, _, sim, st, ..)) = cars.get(me) else {
            acc.0.clear();
            continue;
        };
        if st.wrecked {
            acc.0.clear();
            continue;
        }
        let now = sim.time;
        acc.0.retain(|ep| {
            let ended = ep.last_t < now - 1e-4 || now - ep.start_t >= IMPACT_WINDOW;
            if ended {
                finished.push((me, *ep));
            }
            !ended
        });
    }

    // 3. Урон по суммарному Δv: damage = k · max(0, |Δv| − 4 м/с) и модификаторы
    for (me, ep) in finished {
        let Ok((_car, stats, sim, _st, _vel, pos, _rot)) = cars.get(me) else { continue };
        // Δv по импульсу контакта, но не больше физически возможного для этой скорости сближения:
        // решатель при глубоком проникновении добавляет «выталкивающий» импульс, который не удар
        let mut dv = ep.impulse / stats.mass;
        let ratio = match masses.get(ep.other) {
            Ok((m, RigidBody::Dynamic)) => {
                let mo = m.value();
                if mo.is_finite() && mo > 0.0 { mo / (mo + stats.mass) } else { 1.0 }
            }
            Ok((_, RigidBody::Kinematic)) => f32::INFINITY,
            _ => 1.0,
        };
        if ratio.is_finite() {
            dv = dv.min(ep.closing * 1.15 * ratio + 0.5);
        } else {
            // ловушки (лопасть, пресс, поезд) отбрасывают и ранят, но не насмерть с одного касания
            dv = dv.min(12.0);
        }
        if dv > 3.0 {
            events.write(GameEvent::Crash { pos: ep.point, strength: dv });
        }
        if dv > 2.5 {
            events.write(GameEvent::Sparks { pos: ep.point, strength: dv });
        }
        let mut base = impact_damage(dv);
        if std::env::var("CARBANG_DMG_DEBUG").is_ok() && dv > 2.0 {
            println!("удар {:?}: другой {:?} dv={:.1} импульс={:.0} длит={:.3} сближение={:.1} база={:.1}", me, ep.other, dv, ep.impulse, ep.last_t - ep.start_t, ep.closing, base);
        }
        if base <= 0.0 {
            continue;
        }
        let other = ep.other;
        let other_is_car = cars.get(other).is_ok();
        let other_is_prop = props.get(other).is_ok();
        let other_kinematic = matches!(kinematic.get(other), Ok(RigidBody::Kinematic));
        let other_static = !other_is_car && !other_is_prop && statics.get(other).is_ok() && !other_kinematic;
        let zone = zone_of(ep.local_normal, sim.half_w, sim.half_l);
        if other_static {
            base *= 0.5;
        }
        let mut attacker = None;
        let mut kind = if other_static { DamageKind::Wall } else { DamageKind::Ram };
        if other_kinematic {
            kind = DamageKind::Hazard;
            base *= 0.35;
            // одна и та же ловушка ранит не чаще раза в 1.2 с (лопасть «пилит» при долгом касании)
            if let Ok((_, mut acc, _)) = accums.get_mut(me) {
                if sim.time - acc.2 < 1.2 && acc.2 > 0.0 {
                    continue;
                }
                acc.2 = sim.time;
            }
        }
        if other_is_prop {
            base *= 0.25;
        }
        if let Ok((ocar, ostats, _osim, ost, _ovel, opos, _orot)) = cars.get(other) {
            if ep.other_attacks {
                attacker = Some(other);
                let mut ram = ostats.ram;
                if ost.has(PowerupKind::MegaRam) {
                    ram *= 3.0;
                }
                if ep.other_front {
                    // разброс «таранности» машин (0.6–2.6) смягчён корнем: тяжёлый таран
                    // остаётся сильнее, но не убивает с одного удара
                    base *= ram.sqrt();
                    if ocar.cfg.weapon_at(Mount::Front) == Some(WeaponKind::Spikes) {
                        base *= 1.6;
                    }
                    if ocar.cfg.weapon_at(Mount::Front) == Some(WeaponKind::Plow) {
                        base *= 1.3;
                    }
                }
                base *= 1.15;
                // Давилка: приземление сверху
                if ostats.trait_kind == CarTrait::Crusher && opos.0.y > pos.0.y + 0.8 {
                    base *= 3.0;
                }
                // Пинбол: жертва отлетает
                if ost.has(PowerupKind::Pinball) {
                    if let Ok(mut imp) = impulses.get_mut(me) {
                        let mut dir = opos.0 - pos.0;
                        dir.y = 0.0;
                        imp.apply_impulse((-dir.normalize_or_zero() + Vec3::Y * 0.4).normalize() * stats.mass * 9.0 * stats.knock_resist);
                    }
                }
            } else {
                base *= 0.55;
                if zone == 0 {
                    base /= stats.ram.max(0.3).sqrt();
                }
            }
        }
        // Шипы на борту/морде жертвы ранят нападающего
        if let (Some(att), Ok((mycar, ..))) = (attacker, cars.get(me)) {
            if mycar.cfg.weapon_at(mount_for_zone(zone)) == Some(WeaponKind::Spikes) {
                dmg.write(DamageEvent { victim: att, attacker: Some(me), amount: base * 0.6, kind: DamageKind::Ram, local_dir: Vec3::NEG_Z, pos: pos.0, impact_speed: 0.0 });
            }
        }
        dmg.write(DamageEvent { victim: me, attacker, amount: base, kind, local_dir: ep.local_normal, pos: ep.point, impact_speed: ep.closing });
    }
}

// ============================================================ переворот, падения, эвакуация

fn check_flip_and_falls(
    mut q: Query<(Entity, &Car, &CarInput, &mut CarSim, &CarState, &Position, &LinearVelocity)>,
    map: Option<Res<MapData>>,
    mut recover: MessageWriter<RecoverEvent>,
    mut dmg: MessageWriter<DamageEvent>,
    mut events: MessageWriter<GameEvent>,
) {
    for (e, car, input, mut sim, st, pos, vel) in q.iter_mut() {
        if st.wrecked {
            continue;
        }
        let auto_limit = if car.controller == Controller::Bot { 2.5 } else { 3.5 };
        if sim.upside_timer > auto_limit {
            sim.upside_timer = 0.0;
            recover.write(RecoverEvent { car: e, penalty: false });
            continue;
        }
        if input.recover && vel.0.length() < 8.0 && sim.recover_cooldown <= 0.0 {
            sim.recover_cooldown = 2.0;
            recover.write(RecoverEvent { car: e, penalty: true });
            continue;
        }
        // Застрял: бот почти не двигается долго
        if let Some(m) = map.as_ref() {
            let in_water = m.is_water(pos.0);
            if pos.0.y < m.kill_y || in_water {
                if in_water {
                    events.write(GameEvent::Splash { pos: pos.0 });
                }
                dmg.write(DamageEvent {
                    victim: e,
                    attacker: sim.last_attacker.filter(|_| sim.time - sim.last_attacker_time < 6.0),
                    amount: 25.0,
                    kind: DamageKind::Fall,
                    local_dir: Vec3::NEG_Y,
                    pos: pos.0,
impact_speed: 0.0,
});
                recover.write(RecoverEvent { car: e, penalty: false });
            }
        }
    }
}

fn do_recover(
    mut ev: MessageReader<RecoverEvent>,
    map: Option<Res<MapData>>,
    mut q: Query<(&mut Position, &mut Rotation, &mut LinearVelocity, &mut AngularVelocity, &mut CarSim, &NetId, &CarStats)>,
    others: Query<(Entity, &Position), (With<Car>, Without<CarSim>)>,
    all_pos: Query<(Entity, &Transform), With<Car>>,
    mut events: MessageWriter<GameEvent>,
) {
    let Some(map) = map else { return };
    for r in ev.read() {
        let occupied: Vec<Vec3> = all_pos.iter().filter(|(e, _)| *e != r.car).map(|(_, t)| t.translation).collect();
        let _ = &others;
        let Ok((mut p, mut rot, mut lv, mut av, mut sim, id, stats)) = q.get_mut(r.car) else { continue };
        let (spot, yaw) = map.recover_spot(p.0, &occupied);
        p.0 = spot + Vec3::Y * (stats.ride_height + 0.4);
        rot.0 = Quat::from_rotation_y(yaw);
        lv.0 = Vec3::ZERO;
        av.0 = Vec3::ZERO;
        sim.upside_timer = 0.0;
        sim.air_time = 0.0;
        if r.penalty {
            events.write(GameEvent::Message {
                to: Some(*id),
                text: "ЭВАКУАЦИЯ".into(),
                color: [1.0, 0.6, 0.3],
                big: false,
            });
        }
    }
}

// ============================================================ визуал колёс

pub fn update_wheel_visuals(
    cars: Query<(&Car, &CarState, &Children)>,
    mut wheels: Query<(&WheelVisual, &mut Transform, Option<&crate::carmodel::WheelOffset>)>,
) {
    for (car, st, children) in cars.iter() {
        let def = car.def();
        let mounts = wheel_mounts(def);
        let rest = def.travel * car.cfg.setup.ride;
        for child in children.iter() {
            if let Ok((wv, mut t, off)) = wheels.get_mut(child) {
                let i = wv.0;
                let wh = st.wheels[i];
                let droop = rest - wh.comp;
                t.translation = mounts[i] + off.map(|o| o.0).unwrap_or(Vec3::ZERO) - Vec3::Y * droop;
                t.rotation = Quat::from_rotation_y(-wh.steer) * Quat::from_rotation_x(-wh.spin);
            }
        }
    }
}
