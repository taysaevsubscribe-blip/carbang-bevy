//! GPU-частицы bevy_hanabi: искры при ударе (пул вспышек), дым из мотора и пар из
//! пробитого радиатора у каждой машины. Всего не больше ~3200 живых частиц (лимит ТЗ — 4000).
//! CPU-частицы effects.rs остаются для огня, взрывов и пыли.

use crate::game::*;
use crate::config::CarStats;
use crate::vehicle::*;
use bevy::prelude::*;
use bevy_hanabi::prelude::*;

const SPARK_POOL: usize = 8;

#[derive(Resource)]
struct FxEffects {
    sparks: Handle<EffectAsset>,
    smoke: Handle<EffectAsset>,
    steam: Handle<EffectAsset>,
    /// Мультяшная «кляксa» дыма: три ступени прозрачности (стилизация), в канале R
    puff: Handle<Image>,
}

/// Текстура клубка дыма для частиц: круг с неровным краем и ступенчатой прозрачностью.
fn puff_image() -> Image {
    let n = 64u32;
    let mut px = vec![0u8; (n * n * 4) as usize];
    for y in 0..n {
        for x in 0..n {
            let dx = (x as f32 + 0.5) / n as f32 * 2.0 - 1.0;
            let dy = (y as f32 + 0.5) / n as f32 * 2.0 - 1.0;
            let a = dy.atan2(dx);
            let r = (dx * dx + dy * dy).sqrt() / (0.82 + 0.1 * (a * 5.0).sin() + 0.05 * (a * 9.0 + 1.3).sin());
            let v = if r < 0.55 { 1.0 } else if r < 0.8 { 0.66 } else if r < 1.0 { 0.33 } else { 0.0 };
            let i = ((y * n + x) * 4) as usize;
            let c = (v * 255.0) as u8;
            px[i..i + 4].copy_from_slice(&[c, c, c, 255]);
        }
    }
    Image::new(
        bevy::render::render_resource::Extent3d { width: n, height: n, depth_or_array_layers: 1 },
        bevy::render::render_resource::TextureDimension::D2,
        px,
        bevy::render::render_resource::TextureFormat::Rgba8Unorm,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    )
}

#[derive(Component)]
struct SparkSlot(usize);

#[derive(Component, Clone, Copy, PartialEq)]
enum CarFx {
    Smoke,
    Steam,
}

#[derive(Resource, Default)]
struct SparkCursor(usize);

pub struct HanabiFxPlugin;

impl Plugin for HanabiFxPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HanabiPlugin)
            .init_resource::<SparkCursor>()
            .add_systems(Startup, make_effects)
            .add_systems(OnEnter(AppState::Race), spawn_spark_pool.after(crate::race::setup_race))
            .add_systems(Update, (attach_car_fx, car_fx_state, spark_bursts).run_if(in_state(AppState::Race)));
    }
}

fn make_effects(mut commands: Commands, mut effects: ResMut<Assets<EffectAsset>>, mut images: ResMut<Assets<Image>>) {
    // искры: короткая яркая вспышка, гравитация, сопротивление воздуха
    let w = ExprWriter::new();
    let init_pos = SetPositionSphereModifier { center: w.lit(Vec3::ZERO).expr(), radius: w.lit(0.15).expr(), dimension: ShapeDimension::Volume };
    let init_vel = SetVelocitySphereModifier { center: w.lit(Vec3::new(0.0, -0.4, 0.0)).expr(), speed: (w.lit(4.0) + w.rand(ScalarType::Float) * w.lit(9.0)).expr() };
    let init_life = SetAttributeModifier::new(Attribute::LIFETIME, (w.lit(0.25) + w.rand(ScalarType::Float) * w.lit(0.45)).expr());
    let init_age = SetAttributeModifier::new(Attribute::AGE, w.lit(0.0).expr());
    let accel = AccelModifier::new(w.lit(Vec3::Y * -14.0).expr());
    let drag = LinearDragModifier::new(w.lit(2.5).expr());
    let mut grad = bevy_hanabi::Gradient::new();
    grad.add_key(0.0, Vec4::new(9.0, 6.0, 2.5, 1.0));
    grad.add_key(0.5, Vec4::new(5.0, 2.0, 0.4, 1.0));
    grad.add_key(1.0, Vec4::new(1.0, 0.2, 0.0, 0.0));
    let sparks = EffectAsset::new(64, SpawnerSettings::once(56.0.into()).with_emit_on_start(false), w.finish())
        .with_name("искры")
        .init(init_pos)
        .init(init_vel)
        .init(init_life)
        .init(init_age)
        .update(accel)
        .update(drag)
        .render(ColorOverLifetimeModifier { gradient: grad, blend: ColorBlendMode::Overwrite, mask: ColorBlendMask::RGBA })
        .render(SizeOverLifetimeModifier { gradient: bevy_hanabi::Gradient::linear(Vec3::splat(0.07), Vec3::splat(0.02)), screen_space_size: false })
        .render(OrientModifier::new(OrientMode::AlongVelocity))
        .with_alpha_mode(bevy_hanabi::AlphaMode::Add);

    // дым из мотора: поднимается, расползается, темнеет и тает
    let w = ExprWriter::new();
    let init_pos = SetPositionSphereModifier { center: w.lit(Vec3::ZERO).expr(), radius: w.lit(0.25).expr(), dimension: ShapeDimension::Volume };
    let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, (w.lit(Vec3::new(0.0, 1.6, 0.0)) + (w.rand(VectorType::VEC3F) - w.lit(0.5)) * w.lit(0.8)).expr());
    let init_life = SetAttributeModifier::new(Attribute::LIFETIME, (w.lit(1.6) + w.rand(ScalarType::Float) * w.lit(1.2)).expr());
    let init_age = SetAttributeModifier::new(Attribute::AGE, w.lit(0.0).expr());
    let drag = LinearDragModifier::new(w.lit(0.8).expr());
    let rot = SetAttributeModifier::new(Attribute::F32_0, (w.rand(ScalarType::Float) * w.lit(std::f32::consts::TAU)).expr());
    let rot_attr = w.attr(Attribute::F32_0).expr();
    let slot = w.lit(0u32).expr();
    let mut grad = bevy_hanabi::Gradient::new();
    grad.add_key(0.0, Vec4::new(0.25, 0.24, 0.23, 0.0));
    grad.add_key(0.1, Vec4::new(0.22, 0.21, 0.2, 0.55));
    grad.add_key(1.0, Vec4::new(0.08, 0.08, 0.08, 0.0));
    let mut module = w.finish();
    module.add_texture_slot("puff");
    let smoke = EffectAsset::new(128, SpawnerSettings::rate(18.0.into()), module)
        .with_name("дым")
        .with_simulation_space(SimulationSpace::Global)
        .init(init_pos)
        .init(init_vel)
        .init(init_life)
        .init(init_age)
        .init(rot)
        .update(drag)
        .render(ParticleTextureModifier { texture_slot: slot, sample_mapping: ImageSampleMapping::ModulateOpacityFromR })
        .render(OrientModifier { mode: OrientMode::FaceCameraPosition, rotation: Some(rot_attr) })
        .render(ColorOverLifetimeModifier { gradient: grad, blend: ColorBlendMode::Overwrite, mask: ColorBlendMask::RGBA })
        .render(SizeOverLifetimeModifier { gradient: bevy_hanabi::Gradient::linear(Vec3::splat(0.35), Vec3::splat(1.6)), screen_space_size: false });

    // пар из радиатора: белый, быстрый, короткий
    let w = ExprWriter::new();
    let init_pos = SetPositionSphereModifier { center: w.lit(Vec3::ZERO).expr(), radius: w.lit(0.18).expr(), dimension: ShapeDimension::Volume };
    let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, (w.lit(Vec3::new(0.0, 2.4, 0.0)) + (w.rand(VectorType::VEC3F) - w.lit(0.5)) * w.lit(1.4)).expr());
    let init_life = SetAttributeModifier::new(Attribute::LIFETIME, (w.lit(0.6) + w.rand(ScalarType::Float) * w.lit(0.5)).expr());
    let init_age = SetAttributeModifier::new(Attribute::AGE, w.lit(0.0).expr());
    let drag = LinearDragModifier::new(w.lit(1.5).expr());
    let rot = SetAttributeModifier::new(Attribute::F32_0, (w.rand(ScalarType::Float) * w.lit(std::f32::consts::TAU)).expr());
    let rot_attr = w.attr(Attribute::F32_0).expr();
    let slot = w.lit(0u32).expr();
    let mut grad = bevy_hanabi::Gradient::new();
    grad.add_key(0.0, Vec4::new(0.9, 0.92, 0.95, 0.0));
    grad.add_key(0.15, Vec4::new(0.9, 0.92, 0.95, 0.45));
    grad.add_key(1.0, Vec4::new(0.9, 0.92, 0.95, 0.0));
    let mut module = w.finish();
    module.add_texture_slot("puff");
    let steam = EffectAsset::new(96, SpawnerSettings::rate(28.0.into()), module)
        .with_name("пар")
        .with_simulation_space(SimulationSpace::Global)
        .init(init_pos)
        .init(init_vel)
        .init(init_life)
        .init(init_age)
        .init(rot)
        .update(drag)
        .render(ParticleTextureModifier { texture_slot: slot, sample_mapping: ImageSampleMapping::ModulateOpacityFromR })
        .render(OrientModifier { mode: OrientMode::FaceCameraPosition, rotation: Some(rot_attr) })
        .render(ColorOverLifetimeModifier { gradient: grad, blend: ColorBlendMode::Overwrite, mask: ColorBlendMask::RGBA })
        .render(SizeOverLifetimeModifier { gradient: bevy_hanabi::Gradient::linear(Vec3::splat(0.2), Vec3::splat(0.9)), screen_space_size: false });

    commands.insert_resource(FxEffects { sparks: effects.add(sparks), smoke: effects.add(smoke), steam: effects.add(steam), puff: images.add(puff_image()) });
}

fn spawn_spark_pool(mut commands: Commands, fx: Option<Res<FxEffects>>) {
    let Some(fx) = fx else { return };
    for i in 0..SPARK_POOL {
        commands.spawn((ParticleEffect::new(fx.sparks.clone()), Transform::from_xyz(0.0, -500.0, 0.0), SparkSlot(i), DespawnOnExit(AppState::Race)));
    }
}

/// Удар с Δv > 4 м/с — вспышка искр в точке контакта (по кругу из пула).
fn spark_bursts(mut ev: MessageReader<GameEvent>, mut cursor: ResMut<SparkCursor>, mut slots: Query<(&SparkSlot, &mut Transform, Option<&mut EffectSpawner>)>) {
    for e in ev.read() {
        let (pos, strength) = match e {
            GameEvent::Crash { pos, strength } if *strength > 4.0 => (*pos, *strength),
            GameEvent::Sparks { pos, strength } if *strength > 6.0 => (*pos, *strength),
            _ => continue,
        };
        let idx = cursor.0 % SPARK_POOL;
        cursor.0 += 1;
        for (s, mut tf, sp) in slots.iter_mut() {
            if s.0 == idx {
                tf.translation = pos + Vec3::Y * 0.2;
                tf.scale = Vec3::splat((strength / 10.0).clamp(0.6, 1.6));
                if let Some(mut sp) = sp {
                    sp.reset();
                }
            }
        }
    }
}

/// Каждой машине — источник дыма (над мотором) и пара (перед радиатором), выключенные.
fn attach_car_fx(mut commands: Commands, fx: Option<Res<FxEffects>>, cars: Query<(Entity, &CarStats), Added<Car>>) {
    let Some(fx) = fx else { return };
    for (e, stats) in cars.iter() {
        let hl = stats.size.z * 0.5;
        commands.entity(e).with_children(|c| {
            c.spawn((ParticleEffect::new(fx.smoke.clone()), EffectMaterial { images: vec![fx.puff.clone()] }, Transform::from_xyz(0.0, 0.9, -hl * 0.55), CarFx::Smoke));
            c.spawn((ParticleEffect::new(fx.steam.clone()), EffectMaterial { images: vec![fx.puff.clone()] }, Transform::from_xyz(0.0, 0.55, -hl * 0.95), CarFx::Steam));
        });
    }
}

/// Дым — при здоровье ниже 50% или разбитом моторе; пар — при пробитом радиаторе.
fn car_fx_state(cars: Query<(&CarState, &Children)>, mut fx: Query<(&CarFx, &mut EffectSpawner)>) {
    for (st, children) in cars.iter() {
        let nodes = st.nodes();
        for c in children.iter() {
            if let Ok((kind, mut sp)) = fx.get_mut(c) {
                let want = match kind {
                    CarFx::Smoke => st.wrecked || st.hp < 50.0 || nodes.engine,
                    CarFx::Steam => !st.wrecked && nodes.radiator,
                };
                if sp.active != want {
                    sp.active = want;
                }
            }
        }
    }
}
