//! Рендер-слой GFX v2: пост-процесс (AgX, экспозиция, bloom, цветокор, виньетка), туман,
//! объёмный свет, мокрый асфальт с лужами, процедурные декали и текстуры износа, звёзды.
//! Только визуал: механики, сеть и интерфейс не затрагиваются.

use crate::config::Settings;
use crate::game::*;
use crate::maps::{MapData, Theme};
use crate::meshgen::fbm;
use bevy::post_process::bloom::{Bloom, BloomCompositeMode, BloomPrefilter};
use bevy::anti_alias::fxaa::Fxaa;
use bevy::anti_alias::smaa::Smaa;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::ecs::system::EntityCommands;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::light::EnvironmentMapLight;
use bevy::light::{FogVolume, NotShadowCaster, NotShadowReceiver, VolumetricFog};
use bevy::pbr::{DistanceFog, FogFalloff, ScreenSpaceReflections};
use bevy::prelude::*;
use bevy::camera::Exposure;
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::{ColorGrading, ColorGradingGlobal, ColorGradingSection};

pub struct GfxPlugin;

impl Plugin for GfxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Race), race_gfx.after(crate::camera::spawn_camera).run_if(not_headless));
    }
}

// ============================================================ камера

/// Полный стек пост-обработки на камеру заезда.
pub fn post_stack(ec: &mut EntityCommands, theme: &Theme, quality: u8) {
    let fog_lin = theme.fog.to_linear();
    let sun = theme.sun_color.to_linear();
    ec.insert((
        Tonemapping::AgX,
        Exposure { ev100: theme.exposure },
        DistanceFog {
            color: theme.fog,
            // свечение тумана в сторону низкого солнца
            directional_light_color: Color::linear_rgba(sun.red * 0.35, sun.green * 0.3, sun.blue * 0.25, if theme.night { 0.15 } else { 0.6 }),
            directional_light_exponent: 18.0,
            falloff: FogFalloff::from_visibility_colors(
                theme.fog_vis,
                Color::linear_rgb(fog_lin.red * 0.6, fog_lin.green * 0.7, fog_lin.blue * 0.8),
                theme.fog,
            ),
        },
        ColorGrading {
            global: ColorGradingGlobal {
                temperature: theme.grade_temp,
                post_saturation: theme.grade_sat,
                ..default()
            },
            shadows: ColorGradingSection { contrast: 1.05, saturation: 0.9, lift: 0.0, ..default() },
            midtones: ColorGradingSection { contrast: 1.12, saturation: 1.0, ..default() },
            highlights: ColorGradingSection { saturation: 1.08, gain: 1.05, ..default() },
        },
    ));
    if quality >= 1 {
        ec.insert(Bloom {
            intensity: if theme.night { 0.32 } else { 0.18 },
            low_frequency_boost: 0.6,
            low_frequency_boost_curvature: 0.9,
            high_pass_frequency: 1.0,
            prefilter: BloomPrefilter { threshold: 0.55, threshold_softness: 0.35 },
            composite_mode: BloomCompositeMode::Additive,
            ..Bloom::NATURAL
        });
    }
    if quality >= 2 {
        // Отложенный рендер: отражения в лужах и объёмный туман
        ec.insert((
            Msaa::Off,
            Smaa::default(),
            ScreenSpaceReflections { max_perceptual_roughness: 0.25..0.35, thickness: 0.35, linear_steps: 24, ..default() },
            // контактные тени (Bevy 0.19): машины и пешеходы «стоят» на асфальте
            bevy::pbr::ContactShadows::default(),
            VolumetricFog { ambient_color: theme.ambient, ambient_intensity: 0.0, step_count: 40, jitter: 0.0 },
        ));
    } else if quality == 1 {
        ec.insert((Msaa::Off, Fxaa::default()));
    } else {
        ec.insert(Msaa::Off);
    }
}

// ============================================================ сцена заезда

#[derive(Component)]
pub struct Vignette;

fn race_gfx(
    mut commands: Commands,
    map: Option<Res<MapData>>,
    settings: Res<Settings>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    flat: Option<Res<crate::tests::FlatTest>>,
    (mut sky_cache, old_sky, cams): (ResMut<crate::extassets::SkyCache>, Query<Entity, Or<(With<crate::race::SkyDome>, With<crate::race::SunDisc>)>>, Query<Entity, With<crate::camera::ChaseCam>>),
    mut ambient: ResMut<GlobalAmbientLight>,
) {
    // Виньетка поверх 3D, под HUD
    commands.spawn((
        ImageNode::new(images.add(vignette_texture(256))).with_mode(bevy::ui::widget::NodeImageMode::Stretch),
        Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
        GlobalZIndex(-50),
        Vignette,
        DespawnOnExit(AppState::Race),
    ));
    let Some(map) = map else { return };
    let theme = map.theme;
    // Объёмный туман по всей карте: лучи от солнца, прожекторов и фонарей
    if settings.quality >= 2 && theme.haze > 0.0 && std::env::var("CARBANG_NO_VOL").is_err() {
        commands.spawn((
            FogVolume {
                fog_color: {
                    // Почти нейтральный рассеиватель: конусы берут цвет лампы (натрий), бирюзу даёт дистанционный туман
                    let f = theme.fog.to_linear();
                    let l = (f.red + f.green + f.blue) / 3.0;
                    let k = 1.5;
                    Color::linear_rgb((f.red * 0.25 + l * 0.75) * k, (f.green * 0.25 + l * 0.75) * k, (f.blue * 0.25 + l * 0.75) * k)
                },
                density_factor: theme.haze * 0.6,
                absorption: 0.25,
                scattering: 0.45,
                scattering_asymmetry: 0.7,
                ..default()
            },
            // в Bevy 0.15 допустим только один объём тумана на сцену
            if flat.is_some() {
                Transform::from_xyz(0.0, 15.0, 3490.0).with_scale(Vec3::new(160.0, 34.0, 120.0))
            } else {
                Transform::from_xyz(0.0, 22.0, 0.0).with_scale(Vec3::new(map.half.x * 2.0 + 60.0, 50.0, map.half.y * 2.0 + 60.0))
            },
            DespawnOnExit(AppState::Race),
        ));
    }
    // HDRI Poly Haven: купол неба и освещение окружением вместо градиента
    if let Some(sky) = crate::extassets::sky_for(&mut sky_cache, &mut images, theme.night) {
        for e in old_sky.iter() {
            commands.entity(e).despawn();
        }
        // поворачиваем HDRI так, чтобы его солнце/луна совпали по азимуту с нашим светом
        let want = -theme.sun_dir;
        let yaw = want.x.atan2(-want.z) - sky.bright_dir.x.atan2(-sky.bright_dir.z);
        let rot = Quat::from_rotation_y(-yaw);
        let gain = if theme.night { 0.45 } else { 0.55 };
        commands.spawn((
            Mesh3d(meshes.add(sky_dome_mesh(1400.0, 96, 48))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: Color::linear_rgb(gain, gain, gain),
                base_color_texture: Some(sky.dome.clone()),
                unlit: true,
                cull_mode: None,
                fog_enabled: false,
                ..default()
            })),
            Transform::from_rotation(rot),
            NotShadowCaster,
            NotShadowReceiver,
            crate::race::SkyDome,
            DespawnOnExit(AppState::Race),
        ));
        // туманный пояс у горизонта: небо плавно уходит в цвет тумана
        commands.spawn((
            Mesh3d(meshes.add(horizon_band_mesh(1350.0, theme.fog))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: Color::WHITE,
                unlit: true,
                cull_mode: None,
                fog_enabled: false,
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::default(),
            NotShadowCaster,
            NotShadowReceiver,
            crate::race::SkyDome,
            DespawnOnExit(AppState::Race),
        ));
        let intensity = (theme.ambient_brightness * if theme.night { 0.35 } else { 0.7 } / sky.avg).clamp(20.0, 4000.0);
        warn!("HDRI: средняя яркость {:.4}, интенсивность окружения {:.0}", sky.avg, intensity);
        for cam in cams.iter().filter(|_| std::env::var("CARBANG_NO_ENV").is_err()) {
            commands.entity(cam).insert(EnvironmentMapLight { diffuse_map: sky.diffuse.clone(), specular_map: sky.specular.clone(), intensity, rotation: rot, ..default() });
        }
        ambient.brightness = theme.ambient_brightness * 0.35;
        return;
    }
    if theme.stars {
        commands.spawn((
            Mesh3d(meshes.add(stars_mesh(1500, 1350.0, 7))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: Color::BLACK,
                emissive: LinearRgba::rgb(2.2, 2.3, 2.6),
                unlit: true,
                fog_enabled: false,
                ..default()
            })),
            Transform::default(),
            NotShadowCaster,
            NotShadowReceiver,
            crate::race::SkyDome,
            DespawnOnExit(AppState::Race),
        ));
    }
}

/// Купол с развёрткой в той же проекции, что и выборка HDRI (u = atan2(x, -z)).
fn sky_dome_mesh(r: f32, nu: u32, nv: u32) -> Mesh {
    use crate::meshgen::mesh_from;
    let mut pos = Vec::new();
    let mut nrm = Vec::new();
    let mut uv = Vec::new();
    let mut idx = Vec::new();
    for j in 0..=nv {
        let v = j as f32 / nv as f32;
        let lat = (0.5 - v) * std::f32::consts::PI;
        for i in 0..=nu {
            let u = i as f32 / nu as f32;
            let lon = (u - 0.5) * std::f32::consts::TAU;
            let d = Vec3::new(lat.cos() * lon.sin(), lat.sin(), -lat.cos() * lon.cos());
            pos.push((d * r).to_array());
            nrm.push((-d).to_array());
            uv.push([u, v]);
        }
    }
    for j in 0..nv {
        for i in 0..nu {
            let a = j * (nu + 1) + i;
            let b = a + 1;
            let c = a + nu + 1;
            let d = c + 1;
            idx.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    mesh_from(pos, nrm, uv, idx)
}

/// Кольцо у горизонта с альфой от 1 (низ) до 0 (15° вверх) цветом тумана.
fn horizon_band_mesh(r: f32, fog: Color) -> Mesh {
    let mut m = sky_dome_mesh(r, 64, 32);
    let f = fog.to_linear();
    if let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(p)) = m.attribute(Mesh::ATTRIBUTE_POSITION) {
        let cols: Vec<[f32; 4]> = p
            .iter()
            .map(|q| {
                let h = q[1] / r;
                let a = (1.0 - (h - 0.0) / 0.22).clamp(0.0, 1.0).powf(1.6);
                [f.red, f.green, f.blue, a]
            })
            .collect();
        m.insert_attribute(Mesh::ATTRIBUTE_COLOR, cols);
    }
    m
}

/// Звёзды: крошечные треугольники на верхней полусфере.
fn stars_mesh(n: usize, r: f32, seed: u32) -> Mesh {
    use crate::meshgen::mesh_from;
    let mut pos: Vec<[f32; 3]> = Vec::new();
    let mut nrm: Vec<[f32; 3]> = Vec::new();
    let mut uv: Vec<[f32; 2]> = Vec::new();
    let mut idx = Vec::new();
    let mut h = seed.wrapping_mul(2654435761);
    let mut rnd = || {
        h ^= h << 13;
        h ^= h >> 17;
        h ^= h << 5;
        h as f32 / u32::MAX as f32
    };
    for _ in 0..n {
        let z = rnd() * 0.85 + 0.12;
        let a = rnd() * std::f32::consts::TAU;
        let rr = (1.0 - z * z).sqrt();
        let d = Vec3::new(rr * a.cos(), z, rr * a.sin());
        let size = 1.2 + rnd() * rnd() * 3.5;
        let t1 = d.cross(Vec3::Y).normalize_or(Vec3::X);
        let t2 = d.cross(t1);
        let c = d * r;
        let base = pos.len() as u32;
        for p in [c + t1 * size, c - t1 * size * 0.5 + t2 * size * 0.87, c - t1 * size * 0.5 - t2 * size * 0.87] {
            pos.push(p.to_array());
            nrm.push((-d).to_array());
            uv.push([0.0, 0.0]);
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 1]);
    }
    mesh_from(pos, nrm, uv, idx)
}

// ============================================================ текстуры

fn rgba_image(w: u32, h: u32, data: Vec<u8>, srgb: bool) -> Image {
    let mut img = Image::new(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        if srgb { TextureFormat::Rgba8UnormSrgb } else { TextureFormat::Rgba8Unorm },
        RenderAssetUsages::default(),
    );
    img.sampler = repeat_sampler();
    img
}

pub fn repeat_sampler() -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 16,
        ..default()
    })
}

/// Полная цепочка mip-уровней (бокс-фильтр). Без неё мелкие текстуры рябят вдали.
pub fn add_mips(img: &mut Image) {
    let w0 = img.texture_descriptor.size.width;
    let h0 = img.texture_descriptor.size.height;
    if img.texture_descriptor.mip_level_count > 1 || !w0.is_power_of_two() || !h0.is_power_of_two() {
        return;
    }
    let Some(base) = img.data.clone() else { return };
    if base.is_empty() {
        return;
    }
    let srgb = img.texture_descriptor.format == TextureFormat::Rgba8UnormSrgb;
    let to_lin = |v: u8| -> f32 {
        let c = v as f32 / 255.0;
        if srgb {
            c.powf(2.2)
        } else {
            c
        }
    };
    let to_u8 = |c: f32| -> u8 {
        let c = if srgb { c.max(0.0).powf(1.0 / 2.2) } else { c };
        (c.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
    };
    let mut all = base.clone();
    let mut prev = base;
    let (mut w, mut h) = (w0, h0);
    let mut levels = 1;
    while w > 1 || h > 1 {
        let nw = (w / 2).max(1);
        let nh = (h / 2).max(1);
        let mut next = vec![0u8; (nw * nh * 4) as usize];
        for y in 0..nh {
            for x in 0..nw {
                for ch in 0..4 {
                    let mut sum = 0.0;
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let sx = (x * 2 + dx).min(w - 1);
                        let sy = (y * 2 + dy).min(h - 1);
                        let v = prev[((sy * w + sx) * 4 + ch) as usize];
                        sum += if ch == 3 { v as f32 / 255.0 } else { to_lin(v) };
                    }
                    let avg = sum / 4.0;
                    next[((y * nw + x) * 4 + ch) as usize] = if ch == 3 { (avg * 255.0 + 0.5) as u8 } else { to_u8(avg) };
                }
            }
        }
        all.extend_from_slice(&next);
        prev = next;
        w = nw;
        h = nh;
        levels += 1;
    }
    img.data = Some(all);
    img.texture_descriptor.mip_level_count = levels;
}

/// Мокрая версия покрытия: потемневшая основа в лужах и карта шероховатости
/// (G — шероховатость, B — металличность). Лужи почти зеркальные.
pub fn wet_variant(base: &Image, wet: f32, seed: u32, puddle_scale: i32) -> (Image, Image) {
    let w = base.texture_descriptor.size.width;
    let h = base.texture_descriptor.size.height;
    let src = base.data.clone().unwrap_or_default();
    let mut col = vec![0u8; (w * h * 4) as usize];
    let mut mr = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let u = x as f32 / w as f32;
            let v = y as f32 / h as f32;
            let n = fbm(u, v, puddle_scale, 4, seed);
            let fine = fbm(u, v, puddle_scale * 8, 2, seed + 9);
            // лужи там, где шум выше порога; чем мокрее карта, тем ниже порог
            let thr = 0.72 - wet * 0.12;
            let puddle = ((n - thr) / 0.05).clamp(0.0, 1.0) * wet.min(1.0);
            let damp = (wet * 0.6 + fine * 0.25).clamp(0.0, 1.0);
            let k = ((y * w + x) * 4) as usize;
            let dark = 1.0 - 0.35 * damp - 0.3 * puddle;
            for ch in 0..3 {
                col[k + ch] = (src.get(k + ch).copied().unwrap_or(128) as f32 * dark) as u8;
            }
            col[k + 3] = 255;
            let rough = 0.9 - 0.3 * damp;
            let rough = rough + (0.3 - rough) * puddle;
            mr[k] = 255;
            mr[k + 1] = (rough.clamp(0.02, 1.0) * 255.0) as u8;
            mr[k + 2] = 0;
            mr[k + 3] = 255;
        }
    }
    let mut a = rgba_image(w, h, col, true);
    let mut b = rgba_image(w, h, mr, false);
    add_mips(&mut a);
    add_mips(&mut b);
    (a, b)
}

/// Радиальная виньетка: прозрачный центр, тёмные края.
fn vignette_texture(size: u32) -> Image {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let u = x as f32 / (size - 1) as f32 * 2.0 - 1.0;
            let v = y as f32 / (size - 1) as f32 * 2.0 - 1.0;
            let d = (u * u * 0.8 + v * v).sqrt();
            let a = ((d - 0.55) / 0.85).clamp(0.0, 1.0).powf(1.6) * 0.72;
            data.extend_from_slice(&[0, 4, 8, (a * 255.0) as u8]);
        }
    }
    let mut img = rgba_image(size, size, data, true);
    img.sampler = ImageSampler::linear();
    img
}

/// Декали для дороги: 0 трещины, 1 масляное пятно, 2 лужа (тёмная, гладкая), 3 люк.
pub fn road_decal_texture(kind: u32, size: u32, seed: u32) -> Image {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let u = x as f32 / size as f32;
            let v = y as f32 / size as f32;
            let du = u - 0.5;
            let dv = v - 0.5;
            let r = (du * du + dv * dv).sqrt() * 2.0;
            let n = fbm(u, v, 4, 4, seed);
            let px: [f32; 4] = match kind {
                0 => {
                    // ветвистые трещины: гребни шума
                    let ridge = 1.0 - ((fbm(u, v, 3, 5, seed) - 0.5).abs() * 40.0).clamp(0.0, 1.0);
                    let ridge2 = 1.0 - ((fbm(u, v, 6, 4, seed + 3) - 0.5).abs() * 55.0).clamp(0.0, 1.0);
                    let edge = (1.0 - r).clamp(0.0, 1.0);
                    let a = (ridge.max(ridge2 * 0.7) * edge * 1.3).clamp(0.0, 1.0);
                    [0.03, 0.03, 0.035, a * 0.9]
                }
                1 => {
                    let a = ((1.0 - r + (n - 0.5) * 0.9) * 2.2).clamp(0.0, 1.0);
                    [0.015, 0.012, 0.01, a * 0.85]
                }
                2 => {
                    let a = ((1.0 - r + (n - 0.5) * 1.1) * 3.0).clamp(0.0, 1.0);
                    [0.02, 0.025, 0.03, a * 0.75]
                }
                _ => {
                    let ring = if r < 0.9 && r > 0.8 { 1.0 } else { 0.0 };
                    let grid = if r < 0.8 && (((u * 12.0).fract() < 0.15) || ((v * 12.0).fract() < 0.15)) { 1.0 } else { 0.0 };
                    let a = if r < 0.9 { 1.0 } else { 0.0 };
                    let g = 0.12 + ring * 0.08 + grid * 0.06;
                    [g, g, g * 1.05, a]
                }
            };
            for c in px {
                data.push((c.clamp(0.0, 1.0) * 255.0) as u8);
            }
        }
    }
    let mut img = rgba_image(size, size, data, true);
    img.sampler = ImageSampler::linear();
    img
}

/// Слои износа кузова: царапины, вмятины-тени, ржавые дыры. Альфа задаёт рисунок,
/// общая прозрачность материала — стадию (0..1).
pub fn wear_texture(size: u32, seed: u32) -> Image {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let u = x as f32 / size as f32;
            let v = y as f32 / size as f32;
            // царапины: тонкие наклонные штрихи
            let s1 = ((u * 40.0 + v * 11.0 + fbm(u, v, 4, 2, seed) * 6.0).fract() < 0.04) as i32 as f32;
            let s_mask = (fbm(u, v, 5, 3, seed + 2) > 0.58) as i32 as f32;
            let scratch = s1 * s_mask;
            // пятна грязи и копоти
            let grime = ((fbm(u, v, 3, 4, seed + 5) - 0.45) * 2.5).clamp(0.0, 1.0);
            // дыры от пуль и ржавчины
            let hole_n = fbm(u, v, 12, 2, seed + 7);
            let hole = (hole_n > 0.8) as i32 as f32;
            let rust = ((fbm(u, v, 6, 3, seed + 11) - 0.6) * 4.0).clamp(0.0, 1.0);
            let (c, a) = if hole > 0.0 {
                ([0.02, 0.02, 0.02], 1.0)
            } else if scratch > 0.0 {
                ([0.72, 0.72, 0.7], 0.9)
            } else if rust > 0.2 {
                ([0.36, 0.17, 0.07], rust * 0.85)
            } else {
                ([0.05, 0.045, 0.04], grime * 0.7)
            };
            data.extend_from_slice(&[(c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8, (a * 255.0) as u8]);
        }
    }
    let mut img = rgba_image(size, size, data, true);
    add_mips(&mut img);
    img
}

/// Текстура трещин стекла (белые линии с альфой).
pub fn cracked_glass_texture(size: u32, seed: u32) -> Image {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    let cx = 0.35;
    let cy = 0.45;
    for y in 0..size {
        for x in 0..size {
            let u = x as f32 / size as f32;
            let v = y as f32 / size as f32;
            let du = u - cx;
            let dv = v - cy;
            let ang = dv.atan2(du);
            let r = (du * du + dv * dv).sqrt();
            let spokes = ((ang * 7.0 / std::f32::consts::PI + fbm(u, v, 4, 2, seed) * 2.0).fract() < 0.06) as i32 as f32;
            let rings = (((r * 18.0) + fbm(u, v, 6, 2, seed + 1) * 1.5).fract() < 0.05) as i32 as f32 * (r < 0.45) as i32 as f32;
            let a = (spokes * (1.0 - r * 1.2).clamp(0.0, 1.0) + rings * 0.8).clamp(0.0, 1.0);
            data.extend_from_slice(&[230, 235, 240, (a * 255.0) as u8]);
        }
    }
    let mut img = rgba_image(size, size, data, true);
    add_mips(&mut img);
    img
}
