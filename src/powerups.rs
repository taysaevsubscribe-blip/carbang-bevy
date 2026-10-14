//! Бонусы на карте: ящики с ремонтом, нитро, патронами и безумными эффектами.

use crate::combat::Combat;
use crate::config::*;
use crate::data::*;
use crate::game::*;
use crate::peds::GiantPeds;
use crate::vehicle::*;
use crate::weapons::*;
use avian3d::prelude::*;
use bevy::prelude::*;
use rand::Rng;

#[derive(Component)]
pub struct Pickup {
    pub index: u32,
    pub kind: PowerupKind,
    pub active: bool,
    pub respawn: f32,
    pub base: Vec3,
}

#[derive(Component)]
pub struct PickupVisual;

#[derive(Resource)]
pub struct PickupMats(pub std::collections::HashMap<PowerupKind, Handle<StandardMaterial>>, pub Handle<Mesh>, pub Handle<Mesh>);

pub struct PowerupsPlugin;

impl Plugin for PowerupsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_pickup_assets)
            .add_systems(FixedUpdate, collect_pickups.in_set(SimSet::Rules).run_if(is_authority).run_if(in_state(AppState::Race)))
            .add_systems(Update, (animate_pickups, pickup_visual_state).run_if(in_state(AppState::Race)));
    }
}

fn setup_pickup_assets(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    let mut map = std::collections::HashMap::new();
    for p in POWERUPS {
        let c = p.color;
        map.insert(
            p.kind,
            mats.add(StandardMaterial {
                base_color: Color::srgb(c[0] * 0.5, c[1] * 0.5, c[2] * 0.5),
                emissive: LinearRgba::rgb(c[0] * 2.5, c[1] * 2.5, c[2] * 2.5),
                perceptual_roughness: 0.3,
                metallic: 0.2,
                ..default()
            }),
        );
    }
    let cube = meshes.add(Cuboid::new(1.1, 1.1, 1.1));
    let ring = meshes.add(Torus::new(0.9, 1.05).mesh());
    commands.insert_resource(PickupMats(map, cube, ring));
}

pub fn roll_kind(rng: &mut impl Rng, carnage: bool) -> PowerupKind {
    let r: f32 = rng.gen();
    if r < 0.22 {
        PowerupKind::Repair
    } else if r < 0.48 {
        PowerupKind::Mystery
    } else if r < 0.62 {
        PowerupKind::Ammo
    } else if r < 0.74 {
        PowerupKind::Nitro
    } else if r < 0.82 && carnage {
        PowerupKind::Time
    } else if r < 0.86 {
        PowerupKind::Credits
    } else {
        mystery(rng, carnage)
    }
}

pub fn mystery(rng: &mut impl Rng, carnage: bool) -> PowerupKind {
    let total: u32 = POWERUPS.iter().filter(|p| carnage || p.kind != PowerupKind::Time).map(|p| p.mystery_weight).sum();
    let mut x = rng.gen_range(0..total.max(1));
    for p in POWERUPS {
        if !carnage && p.kind == PowerupKind::Time {
            continue;
        }
        if x < p.mystery_weight {
            return p.kind;
        }
        x -= p.mystery_weight;
    }
    PowerupKind::Repair
}

/// Создаёт ящики на точках карты (у сервера и у клиента одинаковые NetId).
pub fn spawn_pickups(commands: &mut Commands, spots: &[Vec3], seed: u64, carnage: bool, assets: Option<&PickupMats>) {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed ^ 0x5151);
    for (i, p) in spots.iter().enumerate() {
        let kind = roll_kind(&mut rng, carnage);
        let base = *p + Vec3::Y * 1.0;
        let mut ec = commands.spawn((
            Transform::from_translation(base),
            Pickup { index: i as u32, kind, active: true, respawn: 0.0, base },
            NetId(3_000_000 + i as u32),
            Visibility::default(),
            DespawnOnExit(AppState::Race),
        ));
        if let Some(a) = assets {
            let mat = a.0.get(&kind).cloned().unwrap_or_default();
            let cube = a.1.clone();
            let ring = a.2.clone();
            let ring_mat = mat.clone();
            ec.with_children(|c| {
                c.spawn((Mesh3d(cube), MeshMaterial3d(mat), Transform::default(), PickupVisual));
                c.spawn((Mesh3d(ring), MeshMaterial3d(ring_mat), Transform::from_xyz(0.0, -0.9, 0.0).with_scale(Vec3::new(1.0, 0.2, 1.0))));
            });
        }
    }
}

fn animate_pickups(time: Res<Time>, mut q: Query<(&Pickup, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (p, mut tf) in q.iter_mut() {
        tf.translation = p.base + Vec3::Y * ((t * 2.0 + p.index as f32).sin() * 0.25);
        tf.rotation = Quat::from_rotation_y(t * 1.5 + p.index as f32);
    }
}

fn pickup_visual_state(
    mut q: Query<(&Pickup, &mut Visibility, &Children), Changed<Pickup>>,
    mut vis: Query<&mut MeshMaterial3d<StandardMaterial>, With<PickupVisual>>,
    assets: Option<Res<PickupMats>>,
) {
    for (p, mut v, children) in q.iter_mut() {
        *v = if p.active { Visibility::Inherited } else { Visibility::Hidden };
        if let Some(a) = assets.as_ref() {
            for c in children.iter() {
                if let Ok(mut m) = vis.get_mut(c) {
                    if let Some(h) = a.0.get(&p.kind) {
                        m.0 = h.clone();
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn collect_pickups(
    mut commands: Commands,
    time: Res<Time>,
    mut pickups: Query<&mut Pickup>,
    mut cars: Query<(Entity, &Car, &mut CarState, &mut Weapons, &Position, &Rotation, &NetId, &CarStats)>,
    mut combat: Query<&mut Combat>,
    mut events: MessageWriter<GameEvent>,
    cfg: Option<Res<MatchConfig>>,
    mut giant: ResMut<GiantPeds>,
    mut ids: ResMut<NetIdAlloc>,
) {
    let dt = time.delta_secs();
    let carnage = cfg.as_ref().map(|c| c.mode == GameMode::Carnage).unwrap_or(false);
    let mut rng = rand::thread_rng();
    let positions: Vec<(Entity, Vec3, bool)> = cars.iter().map(|(e, _, st, _, p, ..)| (e, p.0, st.wrecked)).collect();
    let mut freeze_by: Vec<Entity> = Vec::new();
    for mut pk in pickups.iter_mut() {
        if !pk.active {
            pk.respawn -= dt;
            if pk.respawn <= 0.0 {
                pk.active = true;
                pk.kind = roll_kind(&mut rng, carnage);
            }
            continue;
        }
        for (e, car, mut st, mut weapons, pos, rot, id, stats) in cars.iter_mut() {
            if st.wrecked {
                continue;
            }
            let radius = if car.cfg.has_perk(PerkKind::Magnet) { 6.5 } else { 2.8 };
            if pos.0.distance(pk.base) > radius {
                continue;
            }
            let mut kind = pk.kind;
            if kind == PowerupKind::Mystery {
                kind = mystery(&mut rng, carnage);
            }
            pk.active = false;
            pk.respawn = rng.gen_range(18.0..28.0);
            let scav = car.cfg.has_perk(PerkKind::Scavenger);
            let dur_mult = if scav { 1.5 } else { 1.0 };
            let def = powerup_def(kind);
            match kind {
                PowerupKind::Repair => {
                    st.hp = (st.hp + 40.0).min(100.0);
                    st.armor = (st.armor + 15.0).min(crate::balance::START_ARMOR);
                    for z in st.zones.iter_mut() {
                        *z *= 0.5;
                    }
                }
                PowerupKind::FullRepair => {
                    st.hp = 100.0;
                    st.armor = crate::balance::START_ARMOR;
                    st.zones = [0.0; 4];
                    st.burning = 0.0;
                }
                PowerupKind::Nitro => st.nitro = st.nitro_max.max(3.0),
                PowerupKind::Ammo => weapons.refill(if scav { 0.75 } else { 0.5 }),
                PowerupKind::Time => {
                    if let Ok(mut c) = combat.get_mut(e) {
                        c.time_left += 20.0;
                    }
                }
                PowerupKind::Credits => {
                    if let Ok(mut c) = combat.get_mut(e) {
                        c.credits += 500;
                    }
                }
                PowerupKind::Springs => {
                    st.set_effect(kind, def.duration * dur_mult);
                    st.springs = 3;
                }
                PowerupKind::Freeze => freeze_by.push(e),
                PowerupKind::GiantPeds => giant.0 = def.duration * dur_mult,
                PowerupKind::Barrage => {
                    let mut others: Vec<(Entity, f32)> = positions.iter().filter(|(oe, _, w)| *oe != e && !*w).map(|(oe, op, _)| (*oe, op.distance(pos.0))).collect();
                    others.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                    for (k, (target, _)) in others.iter().take(3).enumerate() {
                        let dir = (rot.0 * Vec3::NEG_Z + Vec3::Y * 0.6 + rot.0 * Vec3::X * (k as f32 - 1.0) * 0.5).normalize();
                        spawn_projectile(&mut commands, &mut ids, ProjKind::Missile, e, pos.0 + Vec3::Y * 1.5, dir * 30.0, Some(*target), 26.0, 5.0, 6.0);
                    }
                }
                PowerupKind::MineRing => {
                    for k in 0..6 {
                        let a = k as f32 / 6.0 * std::f32::consts::TAU;
                        let p = pos.0 + Vec3::new(a.cos() * 6.0, -stats.ride_height + 0.05, a.sin() * 6.0);
                        let id = ids.next();
                        commands.spawn((
                            Transform::from_translation(p),
                            Mine { owner: e, age: 0.0, damage: 36.0, radius: 4.0 },
                            ProjVisual(ProjKind::Mine),
                            id,
                            DespawnOnExit(AppState::Race),
                        ));
                    }
                }
                _ => {
                    st.set_effect(kind, def.duration * dur_mult);
                }
            }
            events.write(GameEvent::Pickup { car: *id, kind, pos: pk.base });
            events.write(GameEvent::Message { to: Some(*id), text: def.name.to_string(), color: def.color, big: false });
            break;
        }
    }
    if !freeze_by.is_empty() {
        for (e, _, mut st, ..) in cars.iter_mut() {
            if !freeze_by.contains(&e) && !st.wrecked {
                st.frozen = powerup_def(PowerupKind::Freeze).duration;
            }
        }
    }
}
