//! Оружие: прицеливание, захват цели, пули, ракеты, бомбы, мины, масло, огнемёт, ЭМИ, взрывы.

use crate::carmodel::mount_points;
use crate::config::*;
use crate::data::*;
use crate::effects::{emit, Emit, FxAssets, FxCounters};
use crate::game::*;
use crate::maps::Prop;
use crate::vehicle::*;
use avian3d::prelude::*;
use bevy::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct WeaponSlot {
    pub kind: WeaponKind,
    pub mount: Mount,
    pub level: u8,
    pub ammo: f32,
    pub max_ammo: f32,
    pub cooldown: f32,
    pub lock: Option<Entity>,
    pub lock_time: f32,
    pub side_toggle: bool,
    pub fx_timer: f32,
}

#[derive(Component, Clone, Debug, Default)]
pub struct Weapons {
    pub slots: Vec<WeaponSlot>,
}

impl Weapons {
    pub fn from_config(cfg: &CarConfig, stats: &CarStats) -> Self {
        let slots = cfg
            .loadout
            .iter()
            .map(|(m, k)| {
                let def = weapon_def(*k);
                let lvl = cfg.weapon_level(*k);
                let max = (def.ammo as f32 * weapon_ammo_mult(lvl) * stats.ammo_mult).round();
                WeaponSlot {
                    kind: *k,
                    mount: *m,
                    level: lvl,
                    ammo: max,
                    max_ammo: max,
                    cooldown: 0.0,
                    lock: None,
                    lock_time: 0.0,
                    side_toggle: false,
                    fx_timer: 0.0,
                }
            })
            .collect();
        Self { slots }
    }
    pub fn refill(&mut self, frac: f32) {
        for s in self.slots.iter_mut() {
            s.ammo = (s.ammo + s.max_ammo * frac).min(s.max_ammo);
        }
    }
}

#[derive(Component, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum ProjKind {
    Rocket,
    Missile,
    Bomb,
    Mine,
    Oil,
}

/// Визуальная метка снаряда (есть и у сервера, и у клиентов).
#[derive(Component, Clone, Copy)]
pub struct ProjVisual(pub ProjKind);

#[derive(Component)]
pub struct Projectile {
    pub kind: ProjKind,
    pub owner: Entity,
    pub vel: Vec3,
    pub life: f32,
    pub age: f32,
    pub target: Option<Entity>,
    pub damage: f32,
    pub radius: f32,
}

#[derive(Component)]
pub struct Mine {
    pub owner: Entity,
    pub age: f32,
    pub damage: f32,
    pub radius: f32,
}

#[derive(Component)]
pub struct OilSlick {
    pub radius: f32,
    pub life: f32,
}

#[derive(Component)]
pub struct Detonate(pub f32);

#[derive(Message, Clone, Copy)]
pub struct ExplodeEvent {
    pub pos: Vec3,
    pub radius: f32,
    pub damage: f32,
    pub owner: Option<Entity>,
    pub push: f32,
}

/// Удар по пешеходам в радиусе (обрабатывает модуль пешеходов).
#[derive(Message, Clone, Copy)]
pub struct PedHitEvent {
    pub pos: Vec3,
    pub radius: f32,
    pub attacker: Option<Entity>,
    pub vel: Vec3,
}

pub struct WeaponsPlugin;

impl Plugin for WeaponsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ExplodeEvent>()
            .add_message::<PedHitEvent>()
            .add_systems(
                FixedUpdate,
                (weapon_logic, apply_status, move_projectiles, mines_and_oil, explosive_props, detonations, do_explosions)
                    .chain()
                    .in_set(SimSet::Weapons)
                    .run_if(is_authority)
                    .run_if(in_state(AppState::Race)),
            )
            .add_systems(Update, (projectile_visuals, projectile_trails).run_if(in_state(AppState::Race)));
    }
}

struct TargetInfo {
    e: Entity,
    pos: Vec3,
    vel: Vec3,
    id: NetId,
    invisible: bool,
    wrecked: bool,
}

fn weapon_dir_for(mount: Mount, rot: Quat, side_right: bool) -> Vec3 {
    match mount {
        Mount::Front | Mount::Roof => rot * Vec3::NEG_Z,
        Mount::Rear => rot * Vec3::Z,
        Mount::Sides => {
            if side_right {
                rot * Vec3::X
            } else {
                rot * Vec3::NEG_X
            }
        }
    }
}

fn muzzle_for(def: &CarDef, mount: Mount, pos: Vec3, rot: Quat, side_right: bool) -> Vec3 {
    let mp = mount_points(def);
    let local = match mount {
        Mount::Front => mp.front + Vec3::new(0.0, 0.2, -0.4),
        Mount::Roof => mp.roof + Vec3::new(0.0, 0.45, -0.5),
        Mount::Rear => mp.rear + Vec3::new(0.0, 0.0, 0.6),
        Mount::Sides => {
            let x = if side_right { mp.side.x + 0.4 } else { -mp.side.x - 0.4 };
            Vec3::new(x, mp.side.y + 0.2, mp.side.z)
        }
    };
    pos + rot * local
}

#[allow(clippy::too_many_arguments)]
fn best_target(targets: &[TargetInfo], me: Entity, from: Vec3, dir: Vec3, cone_deg: f32, range: f32, spatial: &SpatialQuery) -> Option<usize> {
    let cos_lim = cone_deg.to_radians().cos();
    let mut best = None;
    let mut best_score = f32::MAX;
    for (i, t) in targets.iter().enumerate() {
        if t.e == me || t.wrecked {
            continue;
        }
        let to = t.pos + Vec3::Y * 0.4 - from;
        let d = to.length();
        if d > range || d < 0.5 {
            continue;
        }
        if t.invisible && d > 15.0 {
            continue;
        }
        let c = to.normalize().dot(dir);
        if c < cos_lim {
            continue;
        }
        let score = d * (2.0 - c);
        if score < best_score {
            let filter = SpatialQueryFilter::from_mask(Layer::World);
            if let Some(dir3) = Dir3::new(to).ok() {
                if spatial.cast_ray(from, dir3, d - 1.0, true, &filter).is_some() {
                    continue;
                }
            }
            best_score = score;
            best = Some(i);
        }
    }
    best
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn weapon_logic(
    mut commands: Commands,
    time: Res<Time>,
    spatial: SpatialQuery,
    mut cars: Query<(Entity, &Car, &CarInput, &CarStats, &mut CarState, &mut Weapons, &Position, &Rotation, &LinearVelocity, &NetId)>,
    mut dmg: MessageWriter<DamageEvent>,
    mut events: MessageWriter<GameEvent>,
    mut ped_hits: MessageWriter<PedHitEvent>,
    mut ids: ResMut<NetIdAlloc>,
    rotations: Query<&Rotation, With<Car>>,
    mut props: Query<&mut ExternalImpulse, (With<Prop>, Without<Car>)>,
    ms: Option<Res<MatchState>>,
) {
    let running = ms.map(|m| m.phase == MatchPhase::Running).unwrap_or(true);
    let dt = time.delta_secs();
    let targets: Vec<TargetInfo> = cars
        .iter()
        .map(|(e, _, _, _, st, _, p, _, v, id)| TargetInfo {
            e,
            pos: p.0,
            vel: v.0,
            id: *id,
            invisible: st.has(PowerupKind::Invisibility),
            wrecked: st.wrecked,
        })
        .collect();
    let mut rng = rand::thread_rng();
    for (me, car, input, stats, mut st, mut weapons, pos, rot, vel, my_id) in cars.iter_mut() {
        let def = car.def();
        let rot = rot.0;
        let pos = pos.0;
        st.lock_target = None;
        st.lock_frac = 0.0;
        let disabled = st.wrecked || st.emp > 0.0 || !running || st.frozen > 0.0;
        let fire_rate = stats.fire_rate;
        let pyro = car.cfg.has_perk(PerkKind::Pyromaniac);
        let mut turret_target_yaw = None;
        for slot in weapons.slots.iter_mut() {
            slot.cooldown = (slot.cooldown - dt).max(0.0);
            slot.fx_timer = (slot.fx_timer - dt).max(0.0);
            let wd = weapon_def(slot.kind);
            let dmg_mult = weapon_damage_mult(slot.level);
            // Наведение турели и захват
            let facing = weapon_dir_for(slot.mount, rot, slot.side_toggle);
            match slot.kind {
                WeaponKind::MachineGun if slot.mount == Mount::Roof => {
                    let from = pos + rot * Vec3::Y * 1.0;
                    if let Some(i) = best_target(&targets, me, from, rot * Vec3::NEG_Z, 179.0, wd.range, &spatial) {
                        let local = rot.inverse() * (targets[i].pos - pos);
                        turret_target_yaw = Some((-local.x).atan2(-local.z));
                    }
                }
                WeaponKind::Homing => {
                    let from = muzzle_for(def, slot.mount, pos, rot, false);
                    let cand = best_target(&targets, me, from, rot * Vec3::NEG_Z, 35.0, wd.range, &spatial).map(|i| targets[i].e);
                    if cand.is_some() && cand == slot.lock {
                        slot.lock_time += dt;
                    } else {
                        slot.lock = cand;
                        slot.lock_time = 0.0;
                    }
                    if let Some(l) = slot.lock {
                        st.lock_target = targets.iter().find(|t| t.e == l).map(|t| t.id);
                        st.lock_frac = (slot.lock_time / 0.7).min(1.0);
                    }
                }
                _ => {}
            }
            if disabled || slot.cooldown > 0.0 || slot.ammo < 1.0 && slot.kind != WeaponKind::Flamer {
                continue;
            }
            let pressed = match wd.trigger {
                crate::data::Trigger::Fire => input.fire,
                crate::data::Trigger::Special => input.special,
                crate::data::Trigger::Drop => input.drop,
                crate::data::Trigger::Passive => false,
            };
            if !pressed {
                continue;
            }
            match slot.kind {
                WeaponKind::MachineGun => {
                    slot.side_toggle = !slot.side_toggle;
                    let muzzle = muzzle_for(def, slot.mount, pos, rot, slot.side_toggle);
                    let mut dir = if slot.mount == Mount::Roof {
                        Quat::from_axis_angle(rot * Vec3::Y, st.turret_yaw) * (rot * Vec3::NEG_Z)
                    } else {
                        weapon_dir_for(slot.mount, rot, slot.side_toggle)
                    };
                    let mut muzzle = muzzle;
                    if slot.mount == Mount::Sides {
                        // бортовые стволы бьют в ту сторону, где цель
                        for side in [true, false] {
                            let d = weapon_dir_for(Mount::Sides, rot, side);
                            let m = muzzle_for(def, Mount::Sides, pos, rot, side);
                            if best_target(&targets, me, m, d, 32.0, wd.range, &spatial).is_some() {
                                dir = d;
                                muzzle = m;
                                break;
                            }
                        }
                    }
                    let cone = match slot.mount {
                        Mount::Roof => 12.0,
                        Mount::Sides => 32.0,
                        _ => 14.0,
                    };
                    if let Some(i) = best_target(&targets, me, muzzle, dir, cone, wd.range, &spatial) {
                        let t = &targets[i];
                        let d = t.pos.distance(muzzle);
                        let lead = t.pos + t.vel * (d / 400.0) + Vec3::Y * 0.3;
                        dir = (lead - muzzle).normalize_or_zero();
                    }
                    let spread = 0.025;
                    dir = (dir + Vec3::new(rng.gen_range(-spread..spread), rng.gen_range(-spread..spread), rng.gen_range(-spread..spread))).normalize_or_zero();
                    fire_hitscan(&spatial, me, muzzle, dir, wd.range, wd.damage * dmg_mult, &mut dmg, &mut events, &mut ped_hits, &rotations, &mut props);
                    slot.ammo -= 1.0;
                    slot.cooldown = wd.cooldown / fire_rate;
                    if slot.fx_timer <= 0.0 {
                        slot.fx_timer = 0.1;
                        events.write(GameEvent::WeaponFired { car: *my_id, kind: slot.kind, pos: muzzle });
                    }
                }
                WeaponKind::Rockets => {
                    slot.side_toggle = !slot.side_toggle;
                    let muzzle = muzzle_for(def, slot.mount, pos, rot, slot.side_toggle);
                    let mut dir = facing;
                    if let Some(i) = best_target(&targets, me, muzzle, dir, 9.0, wd.range, &spatial) {
                        let t = &targets[i];
                        let d = t.pos.distance(muzzle);
                        dir = (t.pos + t.vel * (d / wd.speed) + Vec3::Y * 0.3 - muzzle).normalize_or_zero();
                    }
                    spawn_projectile(&mut commands, &mut ids, ProjKind::Rocket, me, muzzle, dir * wd.speed + vel.0 * 0.5, None, wd.damage * dmg_mult, wd.radius * if pyro { 1.3 } else { 1.0 }, 3.0);
                    slot.ammo -= 1.0;
                    slot.cooldown = wd.cooldown / fire_rate;
                    events.write(GameEvent::WeaponFired { car: *my_id, kind: slot.kind, pos: muzzle });
                }
                WeaponKind::Homing => {
                    let muzzle = muzzle_for(def, slot.mount, pos, rot, false);
                    let target = if slot.lock_time >= 0.7 { slot.lock } else { None };
                    let dir = (rot * Vec3::NEG_Z + rot * Vec3::Y * 0.35).normalize();
                    spawn_projectile(&mut commands, &mut ids, ProjKind::Missile, me, muzzle, dir * wd.speed * 0.6 + vel.0 * 0.5, target, wd.damage * dmg_mult, wd.radius * if pyro { 1.3 } else { 1.0 }, 5.0);
                    slot.ammo -= 1.0;
                    slot.cooldown = wd.cooldown / fire_rate;
                    events.write(GameEvent::WeaponFired { car: *my_id, kind: slot.kind, pos: muzzle });
                }
                WeaponKind::Bomb => {
                    let muzzle = muzzle_for(def, slot.mount, pos, rot, false);
                    let v = rot * Vec3::NEG_Z * wd.speed + Vec3::Y * 9.0 + vel.0;
                    spawn_projectile(&mut commands, &mut ids, ProjKind::Bomb, me, muzzle, v, None, wd.damage * dmg_mult, wd.radius * if pyro { 1.3 } else { 1.0 }, 2.2);
                    slot.ammo -= 1.0;
                    slot.cooldown = wd.cooldown / fire_rate;
                    events.write(GameEvent::WeaponFired { car: *my_id, kind: slot.kind, pos: muzzle });
                }
                WeaponKind::Mines => {
                    let p = muzzle_for(def, slot.mount, pos, rot, false);
                    let id = ids.next();
                    commands.spawn((
                        Transform::from_translation(Vec3::new(p.x, pos.y - stats.ride_height + 0.05, p.z)),
                        Mine { owner: me, age: 0.0, damage: wd.damage * dmg_mult, radius: wd.radius * if pyro { 1.3 } else { 1.0 } },
                        ProjVisual(ProjKind::Mine),
                        id,
                        DespawnOnExit(AppState::Race),
                    ));
                    slot.ammo -= 1.0;
                    slot.cooldown = wd.cooldown / fire_rate;
                    events.write(GameEvent::WeaponFired { car: *my_id, kind: slot.kind, pos: p });
                }
                WeaponKind::Oil => {
                    let p = muzzle_for(def, slot.mount, pos, rot, false) + rot * Vec3::Z * 2.5;
                    let id = ids.next();
                    commands.spawn((
                        Transform::from_translation(Vec3::new(p.x, pos.y - stats.ride_height + 0.02, p.z)),
                        OilSlick { radius: wd.radius, life: 30.0 },
                        ProjVisual(ProjKind::Oil),
                        id,
                        DespawnOnExit(AppState::Race),
                    ));
                    slot.ammo -= 1.0;
                    slot.cooldown = wd.cooldown / fire_rate;
                    events.write(GameEvent::WeaponFired { car: *my_id, kind: slot.kind, pos: p });
                }
                WeaponKind::Emp => {
                    for t in &targets {
                        if t.e == me || t.wrecked {
                            continue;
                        }
                        if t.pos.distance(pos) < wd.radius {
                            dmg.write(DamageEvent { victim: t.e, attacker: Some(me), amount: wd.damage * dmg_mult, kind: DamageKind::Emp, local_dir: Vec3::ZERO, pos: t.pos, impact_speed: 0.0 });
                            commands.entity(t.e).insert(EmpHit(2.5));
                        }
                    }
                    events.write(GameEvent::Emp { pos, radius: wd.radius });
                    slot.ammo -= 1.0;
                    slot.cooldown = wd.cooldown / fire_rate;
                }
                WeaponKind::Flamer => {
                    if slot.ammo <= 0.0 {
                        continue;
                    }
                    let muzzle = muzzle_for(def, slot.mount, pos, rot, false);
                    let dir = rot * Vec3::NEG_Z;
                    slot.ammo = (slot.ammo - 10.0 * dt).max(0.0);
                    for t in &targets {
                        if t.e == me || t.wrecked {
                            continue;
                        }
                        let to = t.pos - muzzle;
                        let d = to.length();
                        if d < wd.range + 1.5 && to.normalize_or_zero().dot(dir) > 25f32.to_radians().cos() {
                            dmg.write(DamageEvent { victim: t.e, attacker: Some(me), amount: wd.damage * dmg_mult * dt, kind: DamageKind::Fire, local_dir: Vec3::ZERO, pos: t.pos, impact_speed: 0.0 });
                            commands.entity(t.e).insert(Ignite(if pyro { 6.0 } else { 4.0 }));
                        }
                    }
                    if slot.fx_timer <= 0.0 {
                        slot.fx_timer = 0.05;
                        events.write(GameEvent::Flame { from: muzzle, dir: dir + vel.0 / 16.0 });
                        ped_hits.write(PedHitEvent { pos: muzzle + dir * 5.0, radius: 4.0, attacker: Some(me), vel: dir * 3.0 });
                    }
                }
                WeaponKind::Spikes | WeaponKind::Plow => {}
            }
        }
        // Турель: плавный поворот
        let target_yaw = turret_target_yaw.unwrap_or(0.0);
        let diff = (target_yaw - st.turret_yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        st.turret_yaw += diff.clamp(-5.0 * dt, 5.0 * dt);
        st.weapons = weapons
            .slots
            .iter()
            .map(|s| WeaponHud {
                kind: s.kind,
                mount: s.mount,
                ammo: s.ammo.ceil() as u32,
                max_ammo: s.max_ammo as u32,
                ready: 1.0 - (s.cooldown / (weapon_def(s.kind).cooldown / fire_rate).max(0.01)).clamp(0.0, 1.0),
            })
            .collect();
    }
}

/// Отложенные эффекты оружия на цель (чтобы не брать второй мутабельный доступ к машинам).
#[derive(Component)]
pub struct EmpHit(pub f32);

#[derive(Component)]
pub struct Ignite(pub f32);

#[allow(clippy::too_many_arguments)]
fn fire_hitscan(
    spatial: &SpatialQuery,
    me: Entity,
    from: Vec3,
    dir: Vec3,
    range: f32,
    damage: f32,
    dmg: &mut MessageWriter<DamageEvent>,
    events: &mut MessageWriter<GameEvent>,
    ped_hits: &mut MessageWriter<PedHitEvent>,
    rotations: &Query<&Rotation, With<Car>>,
    props: &mut Query<&mut ExternalImpulse, (With<Prop>, Without<Car>)>,
) {
    let Ok(d3) = Dir3::new(dir) else { return };
    let filter = SpatialQueryFilter::from_mask([Layer::World, Layer::Car, Layer::Prop]).with_excluded_entities([me]);
    let (end, hit) = match spatial.cast_ray(from, d3, range, true, &filter) {
        Some(h) => (from + dir * h.distance, Some(h)),
        None => (from + dir * range, None),
    };
    events.write(GameEvent::Tracer { from, to: end });
    if let Some(h) = hit {
        if let Ok(r) = rotations.get(h.entity) {
            let local = r.0.inverse() * -dir;
            dmg.write(DamageEvent { victim: h.entity, attacker: Some(me), amount: damage, kind: DamageKind::Bullet, local_dir: local, pos: end, impact_speed: 0.0 });
        } else if let Ok(mut imp) = props.get_mut(h.entity) {
            imp.apply_impulse(dir * 25.0);
        }
        events.write(GameEvent::BulletHit { pos: end, normal: h.normal });
        ped_hits.write(PedHitEvent { pos: end, radius: 0.9, attacker: Some(me), vel: dir * 6.0 });
    }
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_projectile(
    commands: &mut Commands,
    ids: &mut NetIdAlloc,
    kind: ProjKind,
    owner: Entity,
    pos: Vec3,
    vel: Vec3,
    target: Option<Entity>,
    damage: f32,
    radius: f32,
    life: f32,
) {
    let id = ids.next();
    commands.spawn((
        Transform::from_translation(pos).looking_to(vel.normalize_or_zero(), Vec3::Y),
        Projectile { kind, owner, vel, life, age: 0.0, target, damage, radius },
        ProjVisual(kind),
        id,
        DespawnOnExit(AppState::Race),
    ));
}

/// Отложенные ЭМИ и поджоги из weapon_logic.
fn apply_status(mut commands: Commands, mut cars: Query<&mut CarState, With<Car>>, hits: Query<(Entity, &EmpHit)>, ignites: Query<(Entity, &Ignite)>) {
    for (e, h) in hits.iter() {
        if let Ok(mut st) = cars.get_mut(e) {
            st.emp = st.emp.max(h.0);
        }
        commands.entity(e).remove::<EmpHit>();
    }
    for (e, ig) in ignites.iter() {
        if let Ok(mut st) = cars.get_mut(e) {
            st.burning = st.burning.max(ig.0);
        }
        commands.entity(e).remove::<Ignite>();
    }
}

#[allow(clippy::type_complexity)]
fn move_projectiles(
    mut commands: Commands,
    time: Res<Time>,
    spatial: SpatialQuery,
    mut q: Query<(Entity, &mut Projectile, &mut Transform)>,
    cars: Query<(&Position, &LinearVelocity, &CarState), With<Car>>,
    mut explode: MessageWriter<ExplodeEvent>,
) {
    let dt = time.delta_secs();
    for (e, mut p, mut t) in q.iter_mut() {
        p.age += dt;
        p.life -= dt;
        match p.kind {
            ProjKind::Missile => {
                if p.age > 0.25 {
                    if let Some(target) = p.target {
                        if let Ok((tp, tv, tst)) = cars.get(target) {
                            if !tst.wrecked {
                                let speed = 48.0f32.max(p.vel.length());
                                let to = tp.0 + Vec3::Y * 0.4 + tv.0 * 0.15 - t.translation;
                                let desired = to.normalize_or_zero() * speed;
                                let turn = 3.2 * dt;
                                let cur = p.vel.normalize_or_zero();
                                let want = desired.normalize_or_zero();
                                let new_dir = cur.lerp(want, turn.min(1.0)).normalize_or_zero();
                                p.vel = new_dir * (speed + 30.0 * dt).min(70.0);
                            }
                        }
                    } else {
                        let speed = p.vel.length();
                        p.vel = p.vel.normalize_or_zero() * (speed + 40.0 * dt).min(65.0);
                        p.vel.y -= 2.0 * dt;
                    }
                }
            }
            ProjKind::Bomb => {
                p.vel.y -= 18.0 * dt;
            }
            ProjKind::Rocket => {
                p.vel.y -= 1.5 * dt;
            }
            _ => {}
        }
        let step = p.vel * dt;
        let len = step.length();
        let mut exploded = false;
        if len > 0.0001 {
            if let Ok(dir) = Dir3::new(step) {
                let mut filter = SpatialQueryFilter::from_mask([Layer::World, Layer::Car, Layer::Prop]);
                if p.age < 0.3 {
                    filter = filter.with_excluded_entities([p.owner]);
                }
                if let Some(hit) = spatial.cast_ray(t.translation, dir, len, true, &filter) {
                    let pos = t.translation + step.normalize() * hit.distance;
                    explode.write(ExplodeEvent { pos, radius: p.radius, damage: p.damage, owner: Some(p.owner), push: 7.0 });
                    commands.entity(e).despawn();
                    exploded = true;
                }
            }
        }
        if exploded {
            continue;
        }
        t.translation += step;
        if p.vel.length_squared() > 0.01 {
            t.look_to(p.vel.normalize(), Vec3::Y);
        }
        if p.life <= 0.0 || t.translation.y < -40.0 {
            if p.kind == ProjKind::Bomb || p.kind == ProjKind::Missile {
                explode.write(ExplodeEvent { pos: t.translation, radius: p.radius, damage: p.damage, owner: Some(p.owner), push: 7.0 });
            }
            commands.entity(e).despawn();
        }
    }
}

#[allow(clippy::type_complexity)]
fn mines_and_oil(
    mut commands: Commands,
    time: Res<Time>,
    mut mines: Query<(Entity, &mut Mine, &Transform)>,
    mut oils: Query<(Entity, &mut OilSlick, &Transform)>,
    mut cars: Query<(Entity, &Position, &mut CarState), With<Car>>,
    mut explode: MessageWriter<ExplodeEvent>,
) {
    let dt = time.delta_secs();
    for (e, mut m, t) in mines.iter_mut() {
        m.age += dt;
        if m.age > 60.0 {
            commands.entity(e).despawn();
            continue;
        }
        if m.age < 0.8 {
            continue;
        }
        for (ce, p, st) in cars.iter() {
            if st.wrecked || (ce == m.owner && m.age < 3.0) {
                continue;
            }
            if horiz(p.0 - t.translation).length() < 2.6 && (p.0.y - t.translation.y).abs() < 2.5 {
                explode.write(ExplodeEvent { pos: t.translation, radius: m.radius, damage: m.damage, owner: Some(m.owner), push: 12.0 });
                commands.entity(e).despawn();
                break;
            }
        }
    }
    for (e, mut o, t) in oils.iter_mut() {
        o.life -= dt;
        if o.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        for (_, p, mut st) in cars.iter_mut() {
            if horiz(p.0 - t.translation).length() < o.radius && (p.0.y - t.translation.y).abs() < 2.0 {
                st.oiled = 1.2;
            }
        }
    }
}

/// Бочки с динамитом взрываются от сильного удара.
fn explosive_props(collisions: Collisions, props: Query<(&Prop, &Transform)>, mut commands: Commands, armed: Query<(), With<Detonate>>) {
    for c in collisions.iter() {
        if c.total_normal_impulse_magnitude() < 160.0 {
            continue;
        }
        for e in [c.body1.unwrap_or(c.collider1), c.body2.unwrap_or(c.collider2)] {
            if let Ok((p, _)) = props.get(e) {
                if p.explosive && p.alive && armed.get(e).is_err() {
                    commands.entity(e).insert(Detonate(0.05));
                }
            }
        }
    }
}

fn detonations(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Detonate, &Transform, &mut Prop)>,
    mut explode: MessageWriter<ExplodeEvent>,
) {
    for (e, mut d, t, mut p) in q.iter_mut() {
        d.0 -= time.delta_secs();
        if d.0 <= 0.0 && p.alive {
            p.alive = false;
            explode.write(ExplodeEvent { pos: t.translation, radius: 6.5, damage: 30.0, owner: None, push: 10.0 });
            commands.entity(e).despawn();
        }
    }
}

#[allow(clippy::type_complexity)]
fn do_explosions(
    mut commands: Commands,
    mut ev: MessageReader<ExplodeEvent>,
    mut cars: Query<(Entity, &Position, &Rotation, &CarStats, &mut ExternalImpulse), With<Car>>,
    mut props: Query<(Entity, &Transform, &Prop, Option<&mut ExternalImpulse>), Without<Car>>,
    mut dmg: MessageWriter<DamageEvent>,
    mut events: MessageWriter<GameEvent>,
    mut ped_hits: MessageWriter<PedHitEvent>,
    armed: Query<(), With<Detonate>>,
) {
    for x in ev.read() {
        events.write(GameEvent::Explosion { pos: x.pos, radius: x.radius });
        ped_hits.write(PedHitEvent { pos: x.pos, radius: x.radius * 1.2, attacker: x.owner, vel: Vec3::Y * 8.0 });
        for (e, p, r, stats, mut imp) in cars.iter_mut() {
            let to = p.0 - x.pos;
            let d = to.length();
            if d > x.radius {
                continue;
            }
            let fall = 1.0 - d / x.radius;
            let mut amount = x.damage * (0.35 + 0.65 * fall);
            if Some(e) == x.owner {
                amount *= 0.4;
            }
            let local = r.0.inverse() * (-to.normalize_or_zero());
            dmg.write(DamageEvent { victim: e, attacker: x.owner.filter(|o| *o != e), amount, kind: DamageKind::Explosion, local_dir: local, pos: x.pos, impact_speed: 0.0 });
            let dir = (to.normalize_or_zero() + Vec3::Y * 0.8).normalize_or_zero();
            imp.apply_impulse(dir * stats.mass * x.push * fall * stats.knock_resist);
        }
        for (e, t, prop, imp) in props.iter_mut() {
            let to = t.translation - x.pos;
            let d = to.length();
            if d > x.radius * 1.3 {
                continue;
            }
            if prop.explosive && prop.alive && armed.get(e).is_err() {
                commands.entity(e).insert(Detonate(0.15 + d * 0.02));
            }
            if let Some(mut imp) = imp {
                let fall = 1.0 - d / (x.radius * 1.3);
                imp.apply_impulse((to.normalize_or_zero() + Vec3::Y).normalize_or_zero() * 40.0 * x.push * fall);
            }
        }
    }
}

// ============================================================ визуал снарядов

fn projectile_visuals(
    mut commands: Commands,
    q: Query<(Entity, &ProjVisual), Added<ProjVisual>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    headless: Res<Headless>,
    mut cache: Local<Option<[(Handle<Mesh>, Handle<StandardMaterial>); 5]>>,
) {
    if headless.0 {
        return;
    }
    if cache.is_none() {
        let rocket = meshes.add(Cylinder::new(0.09, 0.8).mesh().resolution(8));
        let missile = meshes.add(Cylinder::new(0.13, 1.1).mesh().resolution(8));
        let bomb = meshes.add(Sphere::new(0.32).mesh().ico(1).unwrap());
        let mine = meshes.add(Cylinder::new(0.45, 0.18).mesh().resolution(12));
        let oil = meshes.add(Cylinder::new(1.0, 0.02).mesh().resolution(24));
        let glow = |m: &mut Assets<StandardMaterial>, c: LinearRgba| m.add(StandardMaterial { base_color: Color::srgb(0.2, 0.2, 0.2), emissive: c, ..default() });
        *cache = Some([
            (rocket, glow(&mut mats, LinearRgba::rgb(4.0, 1.5, 0.3))),
            (missile, glow(&mut mats, LinearRgba::rgb(5.0, 0.5, 0.3))),
            (bomb, mats.add(StandardMaterial { base_color: Color::srgb(0.12, 0.12, 0.12), metallic: 0.6, perceptual_roughness: 0.4, ..default() })),
            (mine, glow(&mut mats, LinearRgba::rgb(3.0, 0.1, 0.05))),
            (
                oil,
                mats.add(StandardMaterial {
                    base_color: Color::srgba(0.01, 0.01, 0.02, 0.92),
                    perceptual_roughness: 0.05,
                    metallic: 0.3,
                    reflectance: 0.9,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                }),
            ),
        ]);
    }
    let c = cache.as_ref().unwrap();
    for (e, pv) in q.iter() {
        let (i, xf) = match pv.0 {
            ProjKind::Rocket => (0, Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))),
            ProjKind::Missile => (1, Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))),
            ProjKind::Bomb => (2, Transform::default()),
            ProjKind::Mine => (3, Transform::from_xyz(0.0, 0.09, 0.0)),
            ProjKind::Oil => (4, Transform::from_xyz(0.0, 0.02, 0.0).with_scale(Vec3::new(3.5, 1.0, 3.5))),
        };
        let (m, mat) = c[i].clone();
        commands.entity(e).insert(Visibility::default()).with_children(|p| {
            p.spawn((Mesh3d(m), MeshMaterial3d(mat), xf));
        });
    }
}

fn projectile_trails(
    mut commands: Commands,
    time: Res<Time>,
    fx: Option<Res<FxAssets>>,
    mut counters: ResMut<FxCounters>,
    q: Query<(&ProjVisual, &Transform)>,
    mut acc: Local<f32>,
) {
    let Some(fx) = fx else { return };
    *acc += time.delta_secs();
    if *acc < 0.03 {
        return;
    }
    *acc = 0.0;
    for (pv, t) in q.iter() {
        match pv.0 {
            ProjKind::Rocket | ProjKind::Missile => {
                let back = t.translation - t.forward().as_vec3() * 0.6;
                emit(&mut commands, &mut counters, &fx.quad, &fx.fire, Emit { pos: back, vel: Vec3::ZERO, spread: 0.3, count: 1, life: (0.08, 0.15), size: (0.35, 0.1), gravity: 0.0, drag: 0.0 }, true);
                emit(&mut commands, &mut counters, &fx.quad, &fx.smoke, Emit { pos: back, vel: Vec3::Y * 0.5, spread: 0.3, count: 1, life: (0.5, 0.9), size: (0.3, 1.0), gravity: -0.3, drag: 1.0 }, true);
            }
            _ => {}
        }
    }
}
