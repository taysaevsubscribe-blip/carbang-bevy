//! Визуальные эффекты: частицы, взрывы, следы, кровь, дым и огонь от повреждений, трассеры.
//! Слушают только GameEvent и CarState, поэтому одинаково работают у сервера и клиентов.

use crate::carmodel::ExhaustPoint;
use crate::game::*;
use crate::vehicle::*;
use bevy::prelude::*;
use rand::Rng;

#[derive(Component)]
pub struct WaterSurface;

#[derive(Component)]
pub struct Particle {
    pub vel: Vec3,
    pub life: f32,
    pub max: f32,
    pub size0: f32,
    pub size1: f32,
    pub gravity: f32,
    pub drag: f32,
}

#[derive(Component)]
pub struct Billboard;

#[derive(Component)]
pub struct Fireball {
    pub life: f32,
    pub max: f32,
    pub radius: f32,
}

#[derive(Component)]
pub struct FadeLight {
    pub life: f32,
    pub max: f32,
    pub intensity: f32,
}

#[derive(Component)]
pub struct Decal {
    pub born: f32,
}

#[derive(Component)]
pub struct Shortlived(pub f32);

#[derive(Component)]
pub struct EmpRing {
    pub life: f32,
    pub radius: f32,
}

#[derive(Resource, Default)]
pub struct CameraShake(pub f32);

/// Варианты спрайтов (пакет klod v1): у материала-«базы» несколько кадров, emit берёт случайный.
static VARIANTS: std::sync::OnceLock<std::collections::HashMap<AssetId<StandardMaterial>, Vec<Handle<StandardMaterial>>>> = std::sync::OnceLock::new();

fn variant(mat: &Handle<StandardMaterial>, rng: &mut impl Rng) -> Handle<StandardMaterial> {
    match VARIANTS.get().and_then(|v| v.get(&mat.id())) {
        Some(list) if !list.is_empty() => list[rng.gen_range(0..list.len())].clone(),
        _ => mat.clone(),
    }
}

#[derive(Resource)]
pub struct FxAssets {
    /// Огненные шары взрыва, ударная волна, вспышка удара, зелёные искры ремонта (спрайты klod v1)
    pub burst: Option<Handle<StandardMaterial>>,
    pub shock: Option<Handle<StandardMaterial>>,
    pub flash: Option<Handle<StandardMaterial>>,
    pub repair: Option<Handle<StandardMaterial>>,
    pub quad: Handle<Mesh>,
    pub flat: Handle<Mesh>,
    pub ball: Handle<Mesh>,
    pub chunk: Handle<Mesh>,
    pub torus: Handle<Mesh>,
    pub beam: Handle<Mesh>,
    pub spark: Handle<StandardMaterial>,
    pub fire: Handle<StandardMaterial>,
    pub blue_fire: Handle<StandardMaterial>,
    pub smoke: Handle<StandardMaterial>,
    pub black_smoke: Handle<StandardMaterial>,
    pub dust: Handle<StandardMaterial>,
    pub blood: Handle<StandardMaterial>,
    pub blood_green: Handle<StandardMaterial>,
    pub blood_decal: Handle<StandardMaterial>,
    pub blood_decal_green: Handle<StandardMaterial>,
    pub scorch: Handle<StandardMaterial>,
    /// Воронка в асфальте под большим взрывом и осколки стекла после сильного удара (текстуры v3)
    pub crater: Option<Handle<StandardMaterial>>,
    pub glass: Option<Handle<StandardMaterial>>,
    pub skid: Handle<StandardMaterial>,
    pub debris: Handle<StandardMaterial>,
    pub fireball: Handle<StandardMaterial>,
    pub tracer: Handle<StandardMaterial>,
    pub emp: Handle<StandardMaterial>,
    pub water: Handle<StandardMaterial>,
    pub steam: Handle<StandardMaterial>,
    pub oil: Handle<StandardMaterial>,
}

#[derive(Resource, Default)]
pub struct FxCounters {
    pub particles: usize,
    pub decals: Vec<Entity>,
    pub skids: Vec<Entity>,
}

pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraShake>()
            .init_resource::<FxCounters>()
            .add_systems(Startup, setup_fx)
            .add_systems(
                Update,
                (
                    event_effects,
                    update_particles,
                    update_fireballs,
                    car_damage_fx,
                    tire_fx,
                    billboard,
                    shortlived,
                    emp_rings,
                )
                    .run_if(in_state(AppState::Race)),
            )
            .add_systems(OnExit(AppState::Race), clear_fx_counters);
    }
}

fn setup_fx(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>, mut images: ResMut<Assets<Image>>) {
    let blob = images.add(crate::meshgen::blob_texture(64, 3, false));
    let splat = images.add(crate::meshgen::blob_texture(128, 9, true));
    let add_blend = |mats: &mut Assets<StandardMaterial>, c: LinearRgba| {
        mats.add(StandardMaterial {
            base_color: Color::BLACK,
            emissive: c,
            base_color_texture: Some(blob.clone()),
            emissive_texture: Some(blob.clone()),
            alpha_mode: AlphaMode::Add,
            unlit: true,
            double_sided: true,
            cull_mode: None,
            ..default()
        })
    };
    let alpha = |mats: &mut Assets<StandardMaterial>, c: Color, tex: &Handle<Image>| {
        mats.add(StandardMaterial {
            base_color: c,
            base_color_texture: Some(tex.clone()),
            alpha_mode: AlphaMode::Blend,
            unlit: false,
            perceptual_roughness: 1.0,
            double_sided: true,
            cull_mode: None,
            ..default()
        })
    };
    let fx = FxAssets {
        burst: None,
        shock: None,
        flash: None,
        repair: None,
        quad: meshes.add(Rectangle::new(1.0, 1.0)),
        flat: meshes.add(Plane3d::default().mesh().size(1.0, 1.0)),
        ball: meshes.add(Sphere::new(1.0).mesh().ico(2).unwrap()),
        chunk: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        torus: meshes.add(Torus::new(0.9, 1.0).mesh()),
        beam: meshes.add(Cuboid::new(0.06, 0.06, 1.0)),
        spark: add_blend(&mut mats, LinearRgba::rgb(12.0, 7.0, 2.0)),
        fire: add_blend(&mut mats, LinearRgba::rgb(9.0, 3.0, 0.5)),
        blue_fire: add_blend(&mut mats, LinearRgba::rgb(1.5, 3.5, 12.0)),
        smoke: alpha(&mut mats, Color::srgba(0.45, 0.45, 0.47, 0.45), &blob),
        black_smoke: alpha(&mut mats, Color::srgba(0.08, 0.08, 0.08, 0.6), &blob),
        dust: alpha(&mut mats, Color::srgba(0.55, 0.45, 0.32, 0.35), &blob),
        blood: alpha(&mut mats, Color::srgba(0.5, 0.0, 0.0, 0.95), &blob),
        blood_green: alpha(&mut mats, Color::srgba(0.25, 0.55, 0.05, 0.95), &blob),
        blood_decal: alpha(&mut mats, Color::srgba(0.42, 0.0, 0.0, 0.9), &splat),
        blood_decal_green: alpha(&mut mats, Color::srgba(0.2, 0.45, 0.05, 0.9), &splat),
        scorch: match crate::extassets::load_png(&mut images, "decals/decal_scorch.png", true, false) {
            Some(t) => alpha(&mut mats, Color::WHITE, &t),
            None => alpha(&mut mats, Color::srgba(0.02, 0.02, 0.02, 0.85), &splat),
        },
        crater: crate::extassets::load_png(&mut images, "decals/decal_crater.png", true, false).map(|t| alpha(&mut mats, Color::WHITE, &t)),
        glass: crate::extassets::load_png(&mut images, "decals/decal_glass.png", true, false).map(|t| {
            mats.add(StandardMaterial { base_color_texture: Some(t), alpha_mode: AlphaMode::Blend, perceptual_roughness: 0.15, reflectance: 0.9, ..default() })
        }),
        skid: mats.add(StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, 0.6),
            // протектор шины (v3b), если файл на месте
            base_color_texture: crate::extassets::load_png(&mut images, "decals/skid_strip.png", true, true),
            uv_transform: bevy::math::Affine2::from_scale(Vec2::new(0.25, 1.0)),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 1.0,
            ..default()
        }),
        debris: mats.add(StandardMaterial { base_color: Color::srgb(0.15, 0.15, 0.16), metallic: 0.6, perceptual_roughness: 0.5, ..default() }),
        fireball: mats.add(StandardMaterial {
            base_color: Color::BLACK,
            emissive: LinearRgba::rgb(14.0, 5.0, 1.0),
            alpha_mode: AlphaMode::Add,
            unlit: true,
            ..default()
        }),
        tracer: mats.add(StandardMaterial { base_color: Color::BLACK, emissive: LinearRgba::rgb(12.0, 9.0, 3.0), unlit: true, ..default() }),
        emp: mats.add(StandardMaterial {
            base_color: Color::BLACK,
            emissive: LinearRgba::rgb(1.0, 3.0, 10.0),
            alpha_mode: AlphaMode::Add,
            unlit: true,
            ..default()
        }),
        water: alpha(&mut mats, Color::srgba(0.85, 0.92, 1.0, 0.7), &blob),
        steam: alpha(&mut mats, Color::srgba(0.9, 0.92, 0.95, 0.35), &blob),
        oil: mats.add(StandardMaterial {
            base_color: Color::srgba(0.01, 0.01, 0.012, 0.92),
            base_color_texture: Some(splat.clone()),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.04,
            reflectance: 0.9,
            ..default()
        }),
    };
    let mut fx = fx;
    // Спрайты klod v1: огонь, искры, дым, пыль, взрывы, ударная волна, вспышка, ремонт
    let load = |images: &mut Assets<Image>, f: &str| crate::extassets::load_png(images, &format!("fx/{f}.png"), true, false);
    let additive = |mats: &mut Assets<StandardMaterial>, t: Handle<Image>, k: f32| {
        mats.add(StandardMaterial {
            base_color: Color::BLACK,
            emissive: LinearRgba::rgb(k, k, k),
            base_color_texture: Some(t.clone()),
            emissive_texture: Some(t),
            alpha_mode: AlphaMode::Add,
            unlit: true,
            double_sided: true,
            cull_mode: None,
            ..default()
        })
    };
    let blended = |mats: &mut Assets<StandardMaterial>, t: Handle<Image>, c: Color| {
        mats.add(StandardMaterial { base_color: c, base_color_texture: Some(t), alpha_mode: AlphaMode::Blend, perceptual_roughness: 1.0, double_sided: true, cull_mode: None, ..default() })
    };
    let mut variants: std::collections::HashMap<AssetId<StandardMaterial>, Vec<Handle<StandardMaterial>>> = std::collections::HashMap::new();
    let mut set4 = |images: &mut Assets<Image>, mats: &mut Assets<StandardMaterial>, base: &Handle<StandardMaterial>, name: &str, mk: &dyn Fn(&mut Assets<StandardMaterial>, Handle<Image>) -> Handle<StandardMaterial>| {
        let list: Vec<Handle<StandardMaterial>> = (0..4).filter_map(|i| load(images, &format!("{name}_{i}"))).map(|t| mk(mats, t)).collect();
        if !list.is_empty() {
            variants.insert(base.id(), list);
        }
    };
    set4(&mut images, &mut mats, &fx.fire, "fire", &|m, t| additive(m, t, 3.0));
    set4(&mut images, &mut mats, &fx.smoke, "smoke", &|m, t| blended(m, t, Color::srgba(0.75, 0.75, 0.77, 0.55)));
    set4(&mut images, &mut mats, &fx.black_smoke, "smoke", &|m, t| blended(m, t, Color::srgba(0.07, 0.07, 0.07, 0.75)));
    set4(&mut images, &mut mats, &fx.dust, "dust", &|m, t| blended(m, t, Color::srgba(0.85, 0.75, 0.6, 0.5)));
    if let Some(t) = load(&mut images, "sparks") {
        variants.insert(fx.spark.id(), vec![additive(&mut mats, t, 4.0)]);
    }
    let bursts: Vec<Handle<StandardMaterial>> = (0..4).filter_map(|i| load(&mut images, &format!("explosion_{i}"))).map(|t| additive(&mut mats, t, 4.0)).collect();
    if let Some(first) = bursts.first().cloned() {
        variants.insert(first.id(), bursts.clone());
        fx.burst = Some(first);
    }
    fx.shock = load(&mut images, "shockwave").map(|t| additive(&mut mats, t, 3.0));
    fx.flash = load(&mut images, "flash").map(|t| additive(&mut mats, t, 6.0));
    fx.repair = load(&mut images, "repair_glow").map(|t| additive(&mut mats, t, 3.0));
    let _ = VARIANTS.set(variants);
    commands.insert_resource(fx);
}

fn clear_fx_counters(mut c: ResMut<FxCounters>) {
    c.particles = 0;
    c.decals.clear();
    c.skids.clear();
}

pub struct Emit {
    pub pos: Vec3,
    pub vel: Vec3,
    pub spread: f32,
    pub count: usize,
    pub life: (f32, f32),
    pub size: (f32, f32),
    pub gravity: f32,
    pub drag: f32,
}

pub fn emit(commands: &mut Commands, counters: &mut FxCounters, mesh: &Handle<Mesh>, mat: &Handle<StandardMaterial>, e: Emit, billboard: bool) {
    let mut rng = rand::thread_rng();
    let budget = 900usize.saturating_sub(counters.particles);
    let n = e.count.min(budget);
    for _ in 0..n {
        let dir = Vec3::new(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0));
        let v = e.vel + dir * e.spread;
        let life = rng.gen_range(e.life.0..e.life.1.max(e.life.0 + 0.01));
        let m = variant(mat, &mut rng);
        let mut ec = commands.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(m),
            Transform::from_translation(e.pos + dir * 0.2).with_scale(Vec3::splat(e.size.0)),
            Particle { vel: v, life, max: life, size0: e.size.0, size1: e.size.1, gravity: e.gravity, drag: e.drag },
            DespawnOnExit(AppState::Race),
        ));
        if billboard {
            ec.insert(Billboard);
        }
        counters.particles += 1;
    }
}

fn update_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Particle, &mut Transform)>,
    mut counters: ResMut<FxCounters>,
) {
    let dt = time.delta_secs();
    for (e, mut p, mut t) in q.iter_mut() {
        p.life -= dt;
        if p.life <= 0.0 {
            commands.entity(e).despawn();
            counters.particles = counters.particles.saturating_sub(1);
            continue;
        }
        p.vel.y -= p.gravity * dt;
        let drag = (1.0 - p.drag * dt).max(0.0);
        p.vel *= drag;
        t.translation += p.vel * dt;
        let k = 1.0 - p.life / p.max;
        let s = p.size0 + (p.size1 - p.size0) * k;
        t.scale = Vec3::splat(s.max(0.001));
        if t.translation.y < -50.0 {
            p.life = 0.0;
        }
    }
}

fn billboard(cam: Query<&GlobalTransform, With<Camera3d>>, mut q: Query<&mut Transform, With<Billboard>>) {
    let Some(cam) = cam.iter().next() else { return };
    let cp = cam.translation();
    for mut t in q.iter_mut() {
        let dir = cp - t.translation;
        if dir.length_squared() > 0.0001 {
            t.rotation = Quat::from_rotation_arc(Vec3::Z, dir.normalize());
        }
    }
}

fn update_fireballs(
    mut commands: Commands,
    time: Res<Time>,
    mut balls: Query<(Entity, &mut Fireball, &mut Transform), Without<FadeLight>>,
    mut lights: Query<(Entity, &mut FadeLight, &mut PointLight)>,
) {
    let dt = time.delta_secs();
    for (e, mut f, mut t) in balls.iter_mut() {
        f.life -= dt;
        if f.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        let k = 1.0 - f.life / f.max;
        let s = f.radius * (0.4 + 1.1 * k.sqrt()) * (1.0 - k * k * 0.9);
        t.scale = Vec3::splat(s.max(0.01));
    }
    for (e, mut l, mut pl) in lights.iter_mut() {
        l.life -= dt;
        if l.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        pl.intensity = l.intensity * (l.life / l.max);
    }
}

fn shortlived(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut Shortlived)>) {
    for (e, mut s) in q.iter_mut() {
        s.0 -= time.delta_secs();
        if s.0 <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}

fn emp_rings(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut EmpRing, &mut Transform)>) {
    for (e, mut r, mut t) in q.iter_mut() {
        r.life -= time.delta_secs();
        if r.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        let k = 1.0 - r.life / 0.5;
        t.scale = Vec3::new(r.radius * k, 1.0, r.radius * k);
    }
}

fn add_decal(commands: &mut Commands, counters: &mut FxCounters, fx: &FxAssets, mat: &Handle<StandardMaterial>, pos: Vec3, size: f32, now: f32) {
    let mut rng = rand::thread_rng();
    let e = commands
        .spawn((
            Mesh3d(fx.flat.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(pos + Vec3::Y * 0.03)
                .with_rotation(Quat::from_rotation_y(rng.gen_range(0.0..6.28)))
                .with_scale(Vec3::new(size, 1.0, size)),
            Decal { born: now },
            DespawnOnExit(AppState::Race),
        ))
        .id();
    counters.decals.push(e);
    if counters.decals.len() > 140 {
        let old = counters.decals.remove(0);
        if let Ok(mut ec) = commands.get_entity(old) {
            ec.despawn();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn event_effects(
    mut commands: Commands,
    mut events: MessageReader<GameEvent>,
    fx: Option<Res<FxAssets>>,
    mut counters: ResMut<FxCounters>,
    mut shake: ResMut<CameraShake>,
    local: Res<LocalPlayer>,
    cars: Query<(&NetId, &Transform), With<Car>>,
    time: Res<Time>,
    cfg: Option<Res<MatchConfig>>,
    map: Option<Res<crate::maps::MapData>>,
) {
    let Some(fx) = fx else { return };
    let now = time.elapsed_secs();
    let my_pos = local.car.and_then(|id| cars.iter().find(|(n, _)| **n == id).map(|(_, t)| t.translation));
    let zombies = cfg.map(|c| c.zombies).unwrap_or(false);
    let mut rng = rand::thread_rng();
    for ev in events.read() {
        match ev {
            GameEvent::Explosion { pos, radius } => {
                let r = *radius;
                commands.spawn((
                    Mesh3d(fx.ball.clone()),
                    MeshMaterial3d(fx.fireball.clone()),
                    Transform::from_translation(*pos).with_scale(Vec3::splat(r * 0.4)),
                    Fireball { life: 0.45, max: 0.45, radius: r * 0.8 },
                    DespawnOnExit(AppState::Race),
                ));
                commands.spawn((
                    PointLight { color: Color::srgb(1.0, 0.6, 0.25), intensity: 4_000_000.0 * (r / 5.0), range: r * 6.0, shadow_maps_enabled: false, ..default() },
                    Transform::from_translation(*pos + Vec3::Y),
                    FadeLight { life: 0.5, max: 0.5, intensity: 4_000_000.0 * (r / 5.0) },
                    DespawnOnExit(AppState::Race),
                ));
                emit(&mut commands, &mut counters, &fx.quad, &fx.fire, Emit { pos: *pos, vel: Vec3::Y * 3.0, spread: r * 1.6, count: 18, life: (0.3, 0.7), size: (r * 0.5, r * 0.15), gravity: -2.0, drag: 2.5 }, true);
                if let Some(b) = fx.burst.as_ref() {
                    emit(&mut commands, &mut counters, &fx.quad, b, Emit { pos: *pos + Vec3::Y * 0.8, vel: Vec3::Y * 2.0, spread: r * 0.6, count: 3, life: (0.25, 0.45), size: (r * 0.9, r * 1.6), gravity: 0.0, drag: 3.0 }, true);
                }
                if let (Some(sh), Some(m)) = (fx.shock.as_ref(), map.as_ref()) {
                    let ground = m.ground_height(pos.x, pos.z);
                    if (pos.y - ground).abs() < 3.0 {
                        commands.spawn((
                            Mesh3d(fx.flat.clone()),
                            MeshMaterial3d(sh.clone()),
                            Transform::from_xyz(pos.x, ground + 0.15, pos.z).with_scale(Vec3::new(0.1, 1.0, 0.1)),
                            EmpRing { life: 0.5, radius: r * 3.2 },
                            DespawnOnExit(AppState::Race),
                        ));
                    }
                }
                emit(&mut commands, &mut counters, &fx.quad, &fx.black_smoke, Emit { pos: *pos + Vec3::Y, vel: Vec3::Y * 4.0, spread: r * 0.8, count: 12, life: (1.2, 2.4), size: (r * 0.4, r * 1.2), gravity: -1.5, drag: 1.2 }, true);
                emit(&mut commands, &mut counters, &fx.quad, &fx.spark, Emit { pos: *pos, vel: Vec3::Y * 6.0, spread: 14.0, count: 20, life: (0.3, 0.9), size: (0.12, 0.04), gravity: 14.0, drag: 0.5 }, true);
                emit(&mut commands, &mut counters, &fx.chunk, &fx.debris, Emit { pos: *pos, vel: Vec3::Y * 9.0, spread: 9.0, count: 8, life: (1.0, 2.0), size: (0.25, 0.2), gravity: 18.0, drag: 0.2 }, false);
                if let Some(m) = map.as_ref() {
                    let ground = m.ground_height(pos.x, pos.z);
                    if (pos.y - ground).abs() < 2.5 {
                        if let (Some(c), true) = (fx.crater.as_ref(), r >= 4.0) {
                            add_decal(&mut commands, &mut counters, &fx, c, Vec3::new(pos.x, ground - 0.01, pos.z), r * 0.55, now);
                        }
                        add_decal(&mut commands, &mut counters, &fx, &fx.scorch, Vec3::new(pos.x, ground, pos.z), r * 1.1, now);
                    }
                }
                if let Some(mp) = my_pos {
                    let d = mp.distance(*pos);
                    shake.0 = (shake.0 + (1.0 - d / (r * 8.0)).clamp(0.0, 1.0) * 0.9).min(1.5);
                }
            }
            GameEvent::Sparks { pos, strength } => {
                let n = (strength * 3.0).clamp(4.0, 30.0) as usize;
                emit(&mut commands, &mut counters, &fx.quad, &fx.spark, Emit { pos: *pos, vel: Vec3::Y * 3.0, spread: 7.0, count: n, life: (0.2, 0.5), size: (0.09, 0.02), gravity: 14.0, drag: 0.6 }, true);
            }
            GameEvent::BulletHit { pos, normal } => {
                emit(&mut commands, &mut counters, &fx.quad, &fx.spark, Emit { pos: *pos, vel: *normal * 4.0, spread: 3.0, count: 3, life: (0.1, 0.25), size: (0.07, 0.02), gravity: 10.0, drag: 0.5 }, true);
                emit(&mut commands, &mut counters, &fx.quad, &fx.dust, Emit { pos: *pos, vel: *normal * 1.5, spread: 0.8, count: 1, life: (0.4, 0.7), size: (0.3, 0.9), gravity: -0.5, drag: 2.0 }, true);
            }
            GameEvent::Tracer { from, to } => {
                let d = *to - *from;
                let len = d.length();
                if len > 0.1 {
                    commands.spawn((
                        Mesh3d(fx.beam.clone()),
                        MeshMaterial3d(fx.tracer.clone()),
                        Transform::from_translation((*from + *to) * 0.5)
                            .with_rotation(Quat::from_rotation_arc(Vec3::Z, d / len))
                            .with_scale(Vec3::new(1.0, 1.0, len)),
                        Shortlived(0.05),
                        DespawnOnExit(AppState::Race),
                    ));
                }
            }
            GameEvent::Flame { from, dir } => {
                emit(&mut commands, &mut counters, &fx.quad, &fx.fire, Emit { pos: *from, vel: *dir * 16.0, spread: 3.0, count: 3, life: (0.35, 0.6), size: (0.4, 1.8), gravity: -3.0, drag: 1.5 }, true);
            }
            GameEvent::Crash { pos, strength } => {
                if let Some(mp) = my_pos {
                    if mp.distance(*pos) < 4.0 {
                        shake.0 = (shake.0 + (strength / 25.0).clamp(0.0, 0.7)).min(1.5);
                    }
                }
                if let (Some(g), Some(m)) = (fx.glass.as_ref(), map.as_ref()) {
                    // осколки стекла на земле после сильного удара
                    if *strength > 11.0 && rand::random::<f32>() < 0.6 {
                        let ground = m.ground_height(pos.x, pos.z);
                        add_decal(&mut commands, &mut counters, &fx, g, Vec3::new(pos.x, ground, pos.z), 2.2, now);
                    }
                }
                if let (Some(fl), true) = (fx.flash.as_ref(), *strength > 9.0) {
                    let sz = (*strength / 4.0).clamp(2.0, 6.0);
                    emit(&mut commands, &mut counters, &fx.quad, fl, Emit { pos: *pos + Vec3::Y * 0.4, vel: Vec3::ZERO, spread: 0.0, count: 1, life: (0.1, 0.14), size: (sz, sz * 0.2), gravity: 0.0, drag: 0.0 }, true);
                }
                if *strength > 6.0 {
                    emit(&mut commands, &mut counters, &fx.chunk, &fx.debris, Emit { pos: *pos, vel: Vec3::Y * 4.0, spread: 4.0, count: (*strength / 4.0) as usize, life: (0.6, 1.4), size: (0.12, 0.1), gravity: 18.0, drag: 0.2 }, false);
                }
            }
            GameEvent::PedKilled { pos, vel, .. } => {
                let mat = if zombies { &fx.blood_green } else { &fx.blood };
                emit(&mut commands, &mut counters, &fx.quad, mat, Emit { pos: *pos, vel: *vel * 0.5 + Vec3::Y * 3.0, spread: 4.0, count: 14, life: (0.4, 0.9), size: (0.25, 0.12), gravity: 12.0, drag: 0.5 }, true);
                if let Some(m) = map.as_ref() {
                    let g = m.ground_height(pos.x, pos.z);
                    let dmat = if zombies { &fx.blood_decal_green } else { &fx.blood_decal };
                    let off = Vec3::new(rng.gen_range(-0.8..0.8), 0.0, rng.gen_range(-0.8..0.8));
                    add_decal(&mut commands, &mut counters, &fx, dmat, Vec3::new(pos.x, g, pos.z) + off + horiz(*vel) * 0.15, rng.gen_range(1.6..2.8), now);
                }
            }
            GameEvent::Splash { pos } => {
                emit(&mut commands, &mut counters, &fx.quad, &fx.water, Emit { pos: *pos, vel: Vec3::Y * 7.0, spread: 4.0, count: 24, life: (0.6, 1.2), size: (0.6, 1.4), gravity: 12.0, drag: 0.4 }, true);
            }
            GameEvent::Emp { pos, radius } => {
                commands.spawn((
                    Mesh3d(fx.torus.clone()),
                    MeshMaterial3d(fx.emp.clone()),
                    Transform::from_translation(*pos + Vec3::Y * 0.8).with_scale(Vec3::new(0.1, 1.0, 0.1)),
                    EmpRing { life: 0.5, radius: *radius },
                    DespawnOnExit(AppState::Race),
                ));
            }
            GameEvent::Pickup { kind, pos, .. } => {
                use crate::data::PowerupKind;
                if let (Some(m), true) = (fx.repair.as_ref(), matches!(kind, PowerupKind::Repair | PowerupKind::FullRepair)) {
                    emit(&mut commands, &mut counters, &fx.quad, m, Emit { pos: *pos + Vec3::Y * 0.5, vel: Vec3::Y * 3.0, spread: 1.2, count: 6, life: (0.6, 1.1), size: (1.4, 2.6), gravity: -1.0, drag: 1.0 }, true);
                }
            }
            GameEvent::WeaponFired { pos, kind, .. } => {
                use crate::data::WeaponKind;
                if matches!(kind, WeaponKind::Rockets | WeaponKind::Homing | WeaponKind::Bomb) {
                    emit(&mut commands, &mut counters, &fx.quad, &fx.smoke, Emit { pos: *pos, vel: Vec3::Y, spread: 1.5, count: 4, life: (0.4, 0.8), size: (0.4, 1.2), gravity: -0.5, drag: 2.0 }, true);
                }
            }
            _ => {}
        }
    }
}

/// Дым и огонь от повреждённых машин, синее пламя нитро.
fn car_damage_fx(
    mut commands: Commands,
    fx: Option<Res<FxAssets>>,
    mut counters: ResMut<FxCounters>,
    time: Res<Time>,
    cars: Query<(Entity, &CarState, &GlobalTransform, &Children, &crate::config::CarStats), With<Car>>,
    exhausts: Query<&GlobalTransform, With<ExhaustPoint>>,
    mut acc: Local<f32>,
    mut drips: Local<std::collections::HashMap<Entity, f32>>,
) {
    let Some(fx) = fx else { return };
    *acc += time.delta_secs();
    if *acc < 0.05 {
        return;
    }
    *acc = 0.0;
    let now = time.elapsed_secs();
    for (car_e, st, gt, children, stats) in cars.iter() {
        let pos = gt.translation();
        let fwd = gt.forward().as_vec3();
        let engine = pos + fwd * 1.4 + Vec3::Y * 0.5;
        // Пар из радиатора и дым мотора — GPU-частицы hanabi_fx; здесь масло на асфальте и огонь
        if st.hp < 40.0 || st.wrecked {
            let t = drips.entry(car_e).or_insert(now);
            if now - *t > if st.wrecked { 3.0 } else { 1.3 } {
                *t = now;
                let ground = pos - Vec3::Y * (stats.ride_height - 0.02);
                add_decal(&mut commands, &mut counters, &fx, &fx.oil, ground, if st.wrecked { 3.2 } else { 1.1 }, now);
            }
        }
        if st.wrecked {
            emit(&mut commands, &mut counters, &fx.quad, &fx.black_smoke, Emit { pos: pos + Vec3::Y, vel: Vec3::Y * 3.0, spread: 0.8, count: 1, life: (1.5, 2.5), size: (0.8, 2.5), gravity: -0.5, drag: 0.8 }, true);
            emit(&mut commands, &mut counters, &fx.quad, &fx.fire, Emit { pos: pos + Vec3::Y * 0.6, vel: Vec3::Y * 2.0, spread: 0.8, count: 1, life: (0.3, 0.6), size: (0.9, 0.3), gravity: -2.0, drag: 1.0 }, true);
            continue;
        }
        if st.hp < 25.0 || st.burning > 0.0 {
            emit(&mut commands, &mut counters, &fx.quad, &fx.fire, Emit { pos: engine, vel: Vec3::Y * 2.5, spread: 0.6, count: 2, life: (0.25, 0.5), size: (0.6, 0.15), gravity: -2.0, drag: 1.0 }, true);
        }
        if st.nitro_active {
            for c in children.iter() {
                if let Ok(ex) = exhausts.get(c) {
                    emit(&mut commands, &mut counters, &fx.quad, &fx.blue_fire, Emit { pos: ex.translation(), vel: -fwd * 8.0, spread: 0.6, count: 2, life: (0.1, 0.2), size: (0.35, 0.1), gravity: 0.0, drag: 1.0 }, true);
                }
            }
        }
        if st.emp > 0.0 {
            emit(&mut commands, &mut counters, &fx.quad, &fx.blue_fire, Emit { pos: pos + Vec3::Y * 1.2, vel: Vec3::ZERO, spread: 2.0, count: 1, life: (0.1, 0.2), size: (0.2, 0.05), gravity: 0.0, drag: 0.0 }, true);
        }
    }
}

/// Дым из-под колёс и чёрные следы шин.
fn tire_fx(
    mut commands: Commands,
    fx: Option<Res<FxAssets>>,
    mut counters: ResMut<FxCounters>,
    time: Res<Time>,
    cars: Query<(Entity, &Car, &CarState, &GlobalTransform)>,
    map: Option<Res<crate::maps::MapData>>,
    mut acc: Local<f32>,
    mut last: Local<std::collections::HashMap<(Entity, usize), Vec3>>,
) {
    let Some(fx) = fx else { return };
    *acc += time.delta_secs();
    if *acc < 0.04 {
        return;
    }
    *acc = 0.0;
    let dusty = map.as_ref().map(|m| matches!(m.id, crate::data::MapId::Quarry | crate::data::MapId::Junkyard)).unwrap_or(false);
    let mats = if dusty { &fx.dust } else { &fx.smoke };
    for (e, car, st, gt) in cars.iter() {
        let mounts = wheel_mounts(car.def());
        let speed = st.speed.abs();
        for i in 0..4 {
            let w = st.wheels[i];
            let key = (e, i);
            if !w.grounded {
                last.remove(&key);
                continue;
            }
            let rest = car.def().travel * car.cfg.setup.ride;
            let local = mounts[i] - Vec3::Y * (rest - w.comp + car.def().wheel_radius - 0.05);
            let p = gt.transform_point(local);
            if w.slip > 0.35 && speed > 3.0 || (dusty && speed > 12.0 && i >= 2) {
                emit(&mut commands, &mut counters, &fx.quad, mats, Emit { pos: p + Vec3::Y * 0.2, vel: Vec3::Y * 1.0, spread: 0.6, count: 1, life: (0.6, 1.2), size: (0.4, 1.6), gravity: -0.3, drag: 1.5 }, true);
            }
            if w.slip > 0.45 && speed > 4.0 {
                if let Some(prev) = last.get(&key).copied() {
                    let d = p - prev;
                    let len = horiz(d).length();
                    if len > 0.25 && len < 3.0 {
                        let mid = (p + prev) * 0.5;
                        let yaw = (-d.x).atan2(-d.z);
                        let e2 = commands
                            .spawn((
                                Mesh3d(fx.flat.clone()),
                                MeshMaterial3d(fx.skid.clone()),
                                Transform::from_translation(mid + Vec3::Y * 0.02)
                                    .with_rotation(Quat::from_rotation_y(yaw))
                                    .with_scale(Vec3::new(0.28, 1.0, len)),
                                DespawnOnExit(AppState::Race),
                            ))
                            .id();
                        counters.skids.push(e2);
                        if counters.skids.len() > 500 {
                            let old = counters.skids.remove(0);
                            if let Ok(mut ec) = commands.get_entity(old) {
                                ec.despawn();
                            }
                        }
                        last.insert(key, p);
                    } else if len >= 3.0 {
                        last.insert(key, p);
                    }
                } else {
                    last.insert(key, p);
                }
            } else {
                last.remove(&key);
            }
        }
    }
}
