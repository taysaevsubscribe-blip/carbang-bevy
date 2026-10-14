//! Пешеходы: толпа на карте, бегство от машин, приманка, наезды, тела-рэгдоллы.

use crate::config::CarStats;
use crate::data::*;
use crate::game::*;
use crate::maps::MapData;
use crate::meshgen::mesh_from;
use crate::vehicle::*;
use crate::weapons::PedHitEvent;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, VertexAttributeValues};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum PedState {
    Walk,
    Flee,
    Lured,
    Cheer,
}

#[derive(Component)]
pub struct Ped {
    pub id: u32,
    pub state: PedState,
    pub dir: Vec3,
    pub timer: f32,
    pub speed: f32,
    pub phase: f32,
    pub zone: usize,
    pub look: u8,
}

/// Убийство пешехода для начисления очков (обрабатывают правила).
#[derive(Message, Clone, Copy)]
pub struct PedKill {
    pub killer: Option<Entity>,
    pub pos: Vec3,
}

#[derive(Resource, Default)]
pub struct GiantPeds(pub f32);

#[derive(Resource)]
pub struct PedAssets {
    pub mesh: Handle<Mesh>,
    pub mats: Vec<Handle<StandardMaterial>>,
    pub zombie_mats: Vec<Handle<StandardMaterial>>,
}

#[derive(Component)]
pub struct Corpse(pub f32);

pub struct PedsPlugin;

impl Plugin for PedsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PedKill>()
            .init_resource::<GiantPeds>()
            .add_systems(Startup, setup_ped_assets)
            .add_systems(
                FixedUpdate,
                (simulate_peds, ped_hits).chain().in_set(SimSet::Rules).run_if(is_authority).run_if(in_state(AppState::Race)),
            )
            .add_systems(Update, (spawn_corpses, corpses_decay, giant_scale).run_if(in_state(AppState::Race)));
    }
}

fn merge(parts: Vec<(Mesh, Transform)>) -> Mesh {
    let mut p: Vec<[f32; 3]> = Vec::new();
    let mut n: Vec<[f32; 3]> = Vec::new();
    let mut uv: Vec<[f32; 2]> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();
    for (m, t) in parts {
        let base = p.len() as u32;
        if let Some(VertexAttributeValues::Float32x3(pos)) = m.attribute(Mesh::ATTRIBUTE_POSITION) {
            for v in pos {
                p.push(t.transform_point(Vec3::from(*v)).to_array());
            }
        }
        if let Some(VertexAttributeValues::Float32x3(nn)) = m.attribute(Mesh::ATTRIBUTE_NORMAL) {
            for v in nn {
                n.push((t.rotation * Vec3::from(*v)).to_array());
            }
        }
        if let Some(VertexAttributeValues::Float32x2(u)) = m.attribute(Mesh::ATTRIBUTE_UV_0) {
            uv.extend_from_slice(u);
        }
        while uv.len() < p.len() {
            uv.push([0.0, 0.0]);
        }
        match m.indices() {
            Some(Indices::U32(i)) => idx.extend(i.iter().map(|k| k + base)),
            Some(Indices::U16(i)) => idx.extend(i.iter().map(|k| *k as u32 + base)),
            None => {}
        }
    }
    mesh_from(p, n, uv, idx)
}

pub fn person_mesh() -> Mesh {
    merge(vec![
        (Capsule3d::new(0.22, 0.55).mesh().rings(2).latitudes(6).longitudes(8).build(), Transform::from_xyz(0.0, 1.05, 0.0)),
        (Sphere::new(0.14).mesh().ico(1).unwrap(), Transform::from_xyz(0.0, 1.62, 0.0)),
        (Cuboid::new(0.12, 0.7, 0.14).mesh().build(), Transform::from_xyz(-0.1, 0.35, 0.0)),
        (Cuboid::new(0.12, 0.7, 0.14).mesh().build(), Transform::from_xyz(0.1, 0.35, 0.0)),
        (Cuboid::new(0.09, 0.55, 0.09).mesh().build(), Transform::from_xyz(-0.3, 1.1, 0.0).with_rotation(Quat::from_rotation_z(0.25))),
        (Cuboid::new(0.09, 0.55, 0.09).mesh().build(), Transform::from_xyz(0.3, 1.1, 0.0).with_rotation(Quat::from_rotation_z(-0.25))),
    ])
}

fn setup_ped_assets(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    let mesh = meshes.add(person_mesh());
    let palette = [
        Color::srgb(0.8, 0.2, 0.2),
        Color::srgb(0.2, 0.35, 0.75),
        Color::srgb(0.85, 0.75, 0.2),
        Color::srgb(0.2, 0.6, 0.3),
        Color::srgb(0.9, 0.9, 0.88),
        Color::srgb(0.15, 0.15, 0.15),
        Color::srgb(0.6, 0.35, 0.6),
        Color::srgb(0.95, 0.5, 0.15),
    ];
    let m: Vec<_> = palette.iter().map(|c| mats.add(StandardMaterial { base_color: *c, perceptual_roughness: 0.85, ..default() })).collect();
    let z: Vec<_> = palette
        .iter()
        .map(|c| {
            let l = c.to_linear();
            mats.add(StandardMaterial {
                base_color: Color::LinearRgba(LinearRgba::rgb(l.red * 0.35 + 0.08, l.green * 0.35 + 0.25, l.blue * 0.3 + 0.05)),
                perceptual_roughness: 0.95,
                emissive: LinearRgba::rgb(0.02, 0.08, 0.0),
                ..default()
            })
        })
        .collect();
    commands.insert_resource(PedAssets { mesh, mats: m, zombie_mats: z });
}

/// Расставляет пешеходов по зонам карты (на сервере и в одиночной игре).
pub fn spawn_peds(commands: &mut Commands, map: &MapData, count: u32, seed: u64, assets: Option<&PedAssets>, zombies: bool) -> u32 {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed ^ 0x9e3779b97f4a7c15);
    if map.ped_zones.is_empty() {
        return 0;
    }
    let mut spawned = 0;
    let mut tries = 0;
    while spawned < count && tries < count * 40 {
        tries += 1;
        let zi = rng.gen_range(0..map.ped_zones.len());
        let z = map.ped_zones[zi];
        let x = rng.gen_range(z.min.x..z.max.x);
        let zz = rng.gen_range(z.min.y..z.max.y);
        if map.is_blocked(x, zz) || map.is_water(Vec3::new(x, -100.0, zz)) {
            continue;
        }
        let y = map.ground_height(x, zz);
        let a = rng.gen_range(0.0..std::f32::consts::TAU);
        let look = rng.gen_range(0..8u8);
        let mut ec = commands.spawn((
            Transform::from_xyz(x, y, zz),
            Ped {
                id: spawned,
                state: PedState::Walk,
                dir: Vec3::new(a.sin(), 0.0, a.cos()),
                timer: rng.gen_range(1.0..5.0),
                speed: rng.gen_range(1.0..1.7) * if zombies { 0.6 } else { 1.0 },
                phase: rng.gen_range(0.0..6.0),
                zone: zi,
                look,
            },
            NetId(2_000_000 + spawned),
            DespawnOnExit(AppState::Race),
        ));
        if let Some(a) = assets {
            let mat = if zombies { a.zombie_mats[look as usize % a.zombie_mats.len()].clone() } else { a.mats[look as usize % a.mats.len()].clone() };
            ec.insert((Mesh3d(a.mesh.clone()), MeshMaterial3d(mat)));
        }
        spawned += 1;
    }
    spawned
}

#[allow(clippy::too_many_arguments)]
fn simulate_peds(
    mut commands: Commands,
    time: Res<Time>,
    map: Option<Res<MapData>>,
    mut peds: Query<(Entity, &mut Ped, &mut Transform), Without<Car>>,
    cars: Query<(Entity, &Car, &CarState, &Position, &Rotation, &LinearVelocity, &CarStats)>,
    mut kills: MessageWriter<PedKill>,
    mut events: MessageWriter<GameEvent>,
    ids: Query<&NetId>,
    giant: Res<GiantPeds>,
    mut ms: Option<ResMut<MatchState>>,
    cfg: Option<Res<MatchConfig>>,
) {
    let Some(map) = map else { return };
    let dt = time.delta_secs();
    let mut rng = rand::thread_rng();
    let zombies = cfg.map(|c| c.zombies).unwrap_or(false);
    let giant_on = giant.0 > 0.0;
    let ped_radius = if giant_on { 0.8 } else { 0.35 };
    struct C {
        e: Entity,
        pos: Vec3,
        rot: Quat,
        vel: Vec3,
        half: Vec3,
        lure: f32,
    }
    let cs: Vec<C> = cars
        .iter()
        .filter(|(_, _, st, ..)| !st.wrecked)
        .map(|(e, car, st, p, r, v, stats)| {
            let mut lure = 0.0;
            if car.def().trait_kind == CarTrait::PedLure {
                lure = 25.0;
            }
            if st.has(PowerupKind::PedMagnet) {
                lure = 45.0;
            }
            C { e, pos: p.0, rot: r.0, vel: v.0, half: stats.size * 0.5, lure }
        })
        .collect();
    let mut alive = 0;
    for (pe, mut ped, mut t) in peds.iter_mut() {
        alive += 1;
        let p = t.translation;
        // Угроза и приманка
        let mut threat: Option<&C> = None;
        let mut td = 14.0f32 * 14.0;
        let mut lure: Option<&C> = None;
        let mut ld = f32::MAX;
        for c in &cs {
            let d2 = horiz(c.pos - p).length_squared();
            if c.lure > 0.0 && d2 < c.lure * c.lure && d2 < ld {
                ld = d2;
                lure = Some(c);
            }
            if d2 < td && c.vel.length() > 4.0 {
                td = d2;
                threat = Some(c);
            }
        }
        if let Some(c) = lure {
            ped.state = PedState::Lured;
            ped.dir = horiz(c.pos - p).normalize_or_zero();
            ped.timer = 0.5;
        } else if let Some(c) = threat {
            if ped.state == PedState::Walk && rng.gen::<f32>() < 0.9 {
                ped.state = PedState::Flee;
                ped.timer = rng.gen_range(1.5..3.0);
                let away = horiz(p - c.pos).normalize_or_zero();
                let mut side = c.vel.cross(Vec3::Y).normalize_or_zero();
                if side.dot(away) < 0.0 {
                    side = -side;
                }
                ped.dir = (away * 0.5 + side * 0.8 + Vec3::new(rng.gen_range(-0.3..0.3), 0.0, rng.gen_range(-0.3..0.3))).normalize_or_zero();
            }
        }
        ped.timer -= dt;
        if ped.timer <= 0.0 {
            ped.state = PedState::Walk;
            ped.timer = rng.gen_range(2.0..6.0);
            let a = rng.gen_range(0.0..std::f32::consts::TAU);
            ped.dir = Vec3::new(a.sin(), 0.0, a.cos());
        }
        let mut v = ped.speed
            * match ped.state {
                PedState::Flee => 4.2,
                PedState::Lured => 2.2,
                PedState::Cheer => 0.0,
                PedState::Walk => 1.0,
            };
        if zombies && ped.state == PedState::Flee {
            v *= 0.5;
        }
        if giant_on {
            v *= 0.6;
        }
        let np = p + ped.dir * v * dt;
        let zone = map.ped_zones.get(ped.zone).copied();
        let out_of_zone = zone.map(|z| !z.contains(Vec2::new(np.x, np.z))).unwrap_or(false) && ped.state == PedState::Walk;
        if map.is_blocked(np.x, np.z) || out_of_zone || map.is_water(Vec3::new(np.x, -100.0, np.z)) {
            let a = rng.gen_range(0.0..std::f32::consts::TAU);
            ped.dir = Vec3::new(a.sin(), 0.0, a.cos());
        } else {
            let gy = map.ground_height(np.x, np.z);
            if (gy - p.y).abs() < 1.6 {
                t.translation = Vec3::new(np.x, gy, np.z);
            } else {
                ped.dir = -ped.dir;
            }
        }
        ped.phase += dt * v * 3.5;
        let bob = (ped.phase.sin()).abs() * 0.06;
        let mut rot = Quat::from_rotation_y(ped.dir.x.atan2(ped.dir.z));
        if ped.state == PedState::Flee {
            rot *= Quat::from_rotation_x(0.25);
        }
        t.rotation = rot;
        t.translation.y = map.ground_height(t.translation.x, t.translation.z) + bob;

        // Наезд
        let center = t.translation + Vec3::Y * 0.8;
        for c in &cs {
            if c.vel.length() < 2.5 {
                continue;
            }
            let lp = c.rot.inverse() * (center - c.pos);
            if lp.x.abs() < c.half.x + ped_radius && lp.z.abs() < c.half.z + ped_radius && lp.y > -1.8 && lp.y < 2.2 {
                kills.write(PedKill { killer: Some(c.e), pos: center });
                events.write(GameEvent::PedKilled { id: ped.id, pos: center, vel: c.vel, by: ids.get(c.e).ok().copied() });
                commands.entity(pe).despawn();
                alive -= 1;
                break;
            }
        }
    }
    if let Some(ms) = ms.as_mut() {
        ms.peds_alive = alive;
    }
}

fn ped_hits(
    mut commands: Commands,
    mut ev: MessageReader<PedHitEvent>,
    peds: Query<(Entity, &Ped, &Transform)>,
    mut kills: MessageWriter<PedKill>,
    mut events: MessageWriter<GameEvent>,
    ids: Query<&NetId>,
    giant: Res<GiantPeds>,
) {
    let hits: Vec<PedHitEvent> = ev.read().copied().collect();
    if hits.is_empty() {
        return;
    }
    let extra = if giant.0 > 0.0 { 0.6 } else { 0.0 };
    let mut dead = Vec::new();
    for h in &hits {
        for (e, ped, t) in peds.iter() {
            if dead.contains(&e) {
                continue;
            }
            let c = t.translation + Vec3::Y * 0.8;
            if c.distance(h.pos) < h.radius + extra {
                dead.push(e);
                let away = (c - h.pos).normalize_or_zero();
                kills.write(PedKill { killer: h.attacker, pos: c });
                events.write(GameEvent::PedKilled {
                    id: ped.id,
                    pos: c,
                    vel: h.vel + away * 6.0 + Vec3::Y * 3.0,
                    by: h.attacker.and_then(|a| ids.get(a).ok().copied()),
                });
                commands.entity(e).despawn();
            }
        }
    }
}

/// Тела: и у сервера, и у клиента, по событию. Чисто косметическая физика.
#[allow(clippy::too_many_arguments)]
fn spawn_corpses(
    mut commands: Commands,
    mut events: MessageReader<GameEvent>,
    assets: Option<Res<PedAssets>>,
    corpses: Query<(Entity, &Corpse)>,
    headless: Res<Headless>,
    cfg: Option<Res<MatchConfig>>,
    giant: Res<GiantPeds>,
    mode: Res<NetMode>,
    peds: Query<(Entity, &NetId), With<Ped>>,
) {
    let zombies = cfg.map(|c| c.zombies).unwrap_or(false);
    for ev in events.read() {
        let GameEvent::PedKilled { id, pos, vel, .. } = ev else { continue };
        // У клиента живой пешеход удаляется по событию
        if *mode == NetMode::Client {
            for (e, nid) in peds.iter() {
                if nid.0 == 2_000_000 + id {
                    commands.entity(e).despawn();
                }
            }
        }
        if headless.0 {
            continue;
        }
        let Some(a) = assets.as_ref() else { continue };
        let mut list: Vec<(Entity, f32)> = corpses.iter().map(|(e, c)| (e, c.0)).collect();
        if list.len() > 28 {
            list.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap_or(std::cmp::Ordering::Equal));
            if let Some((e, _)) = list.first() {
                commands.entity(*e).despawn();
            }
        }
        let mut rng = rand::thread_rng();
        let look = (*id % 8) as usize;
        let mat = if zombies { a.zombie_mats[look].clone() } else { a.mats[look].clone() };
        let scale = if giant.0 > 0.0 { 2.2 } else { 1.0 };
        commands
            .spawn((
                Transform::from_translation(*pos).with_rotation(Quat::from_rotation_y(rng.gen_range(0.0..6.28))).with_scale(Vec3::splat(scale)),
                RigidBody::Dynamic,
                Collider::capsule(0.25 * scale, 1.1 * scale),
                Mass(70.0),
                CollisionLayers::new(Layer::Corpse, [Layer::World, Layer::Corpse, Layer::Prop]),
                LinearVelocity(*vel * 1.1 + Vec3::Y * (3.0 + vel.length() * 0.3)),
                AngularVelocity(Vec3::new(rng.gen_range(-8.0..8.0), rng.gen_range(-4.0..4.0), rng.gen_range(-8.0..8.0))),
                Friction::new(0.8),
                Corpse(0.0),
                Visibility::default(),
                DespawnOnExit(AppState::Race),
            ))
            .with_children(|c| {
                c.spawn((Mesh3d(a.mesh.clone()), MeshMaterial3d(mat), Transform::from_xyz(0.0, -0.85, 0.0)));
            });
    }
}

fn corpses_decay(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut Corpse)>) {
    for (e, mut c) in q.iter_mut() {
        c.0 += time.delta_secs();
        if c.0 > 30.0 {
            commands.entity(e).despawn();
        }
    }
}

fn giant_scale(time: Res<Time>, mut giant: ResMut<GiantPeds>, mut q: Query<&mut Transform, With<Ped>>) {
    if giant.0 > 0.0 {
        giant.0 = (giant.0 - time.delta_secs()).max(0.0);
    }
    let s = if giant.0 > 0.0 { 2.2 } else { 1.0 };
    for mut t in q.iter_mut() {
        if (t.scale.x - s).abs() > 0.01 {
            t.scale = Vec3::splat(t.scale.x + (s - t.scale.x) * 0.1);
        }
    }
}
