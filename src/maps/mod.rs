//! Карты: общий строитель, данные для ИИ и пешеходов, навигационный граф, реквизит, опасности.

pub mod builders;
pub mod coliseum;
pub mod downtown;
pub mod junkyard;
pub mod port;
pub mod quarry;
pub mod airfield;

use crate::data::*;
use crate::game::*;
use crate::meshgen::*;
use avian3d::prelude::*;
use bevy::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::BinaryHeap;

// ============================================================ материалы

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Mat {
    Asphalt,
    Concrete,
    Sidewalk,
    Rust,
    Dirt,
    Grass,
    Sand,
    Rock,
    Wood,
    Hazard,
    Facade,
    FacadeDark,
    Brick,
    Water,
    Metal,
    Tiles,
    ContainerRed,
    ContainerBlue,
    ContainerYellow,
    ContainerGreen,
    ContainerGrey,
    Glass,
    Neon1,
    Neon2,
    Neon3,
    Lamp,
    Ramp,
    White,
    Black,
    Checkpoint,
    Mud,
    Ice,
    AsphaltWrecked,
    DecalCrack,
    DecalOil,
    DecalPuddle,
    DecalManhole,
    DecalArrow,
    DecalSlow,
    DecalHatch,
    DecalZebra,
}

#[derive(Resource, Clone)]
pub struct MatLib {
    pub mats: std::collections::HashMap<Mat, Handle<StandardMaterial>>,
    pub tex_tile: std::collections::HashMap<Mat, f32>,
    pub blob: Handle<Image>,
    pub splat: Handle<Image>,
    /// Плакаты и граффити для стен (пакет klod v1)
    pub posters: Vec<Handle<StandardMaterial>>,
    pub graffiti: Vec<Handle<StandardMaterial>>,
}

impl MatLib {
    pub fn get(&self, m: Mat) -> Handle<StandardMaterial> {
        self.mats.get(&m).cloned().unwrap_or_default()
    }
    pub fn tile(&self, m: Mat) -> f32 {
        *self.tex_tile.get(&m).unwrap_or(&4.0)
    }
}

pub fn build_matlib(images: &mut Assets<Image>, materials: &mut Assets<StandardMaterial>, theme: &Theme) -> MatLib {
    let night = theme.night;
    let wet = theme.wet;
    let mut mats = std::collections::HashMap::new();
    let mut tiles = std::collections::HashMap::new();
    // Мокрые покрытия: своя основа и карта шероховатости с лужами
    let mut wet_pair = |k: TexKind, size: u32, seed: u32, amount: f32, scale: i32| {
        let base = make_texture(k, size, seed);
        let (c, mr) = crate::gfx::wet_variant(&base, amount, seed + 1000, scale);
        (images.add(c), images.add(mr))
    };
    let (asphalt_w, asphalt_mr) = wet_pair(TexKind::Asphalt, 512, 1, wet, 3);
    let (concrete_w, concrete_mr) = wet_pair(TexKind::Concrete, 256, 2, wet * 0.6, 2);
    let (tiles_w, tiles_mr) = wet_pair(TexKind::Tiles, 256, 13, wet * 0.5, 2);
    let (dirt_w, dirt_mr) = wet_pair(TexKind::Dirt, 256, 4, wet * 0.5, 3);
    let mut tex = |k: TexKind, seed: u32| {
        let mut img = make_texture(k, 256, seed);
        crate::gfx::add_mips(&mut img);
        images.add(img)
    };
    let asphalt = tex(TexKind::Asphalt, 1);
    let concrete = tex(TexKind::Concrete, 2);
    let rust = tex(TexKind::Rust, 3);
    let dirt = tex(TexKind::Dirt, 4);
    let grass = tex(TexKind::Grass, 5);
    let sand = tex(TexKind::Sand, 6);
    let container = tex(TexKind::Container, 7);
    let wood = tex(TexKind::Wood, 8);
    let hazard = tex(TexKind::Hazard, 9);
    let facade = tex(TexKind::Facade, 10);
    let facade_e = tex(TexKind::FacadeEmissive, 10);
    let water = tex(TexKind::Water, 11);
    let metal = tex(TexKind::Metal, 12);
    let tiles_t = tex(TexKind::Tiles, 13);
    let brick = tex(TexKind::Brick, 14);
    let rock = tex(TexKind::Rock, 15);
    let blob = images.add(blob_texture(64, 3, false));
    let splat = images.add(blob_texture(128, 9, true));
    let mut wet_mat = |m: Mat, base: Color, t: Handle<Image>, mr: Handle<Image>, tile: f32| {
        mats.insert(
            m,
            materials.add(StandardMaterial {
                base_color: base,
                base_color_texture: Some(t),
                metallic_roughness_texture: Some(mr),
                perceptual_roughness: 1.0,
                metallic: 1.0,
                reflectance: 0.5,
                ..default()
            }),
        );
        tiles.insert(m, tile);
    };
    wet_mat(Mat::Asphalt, Color::WHITE, asphalt_w, asphalt_mr, 9.0);
    wet_mat(Mat::Concrete, Color::WHITE, concrete_w, concrete_mr, 4.0);
    wet_mat(Mat::Sidewalk, Color::srgb(0.95, 0.93, 0.9), tiles_w, tiles_mr, 3.0);
    let mut add = |m: Mat, base: Color, t: Option<Handle<Image>>, rough: f32, metal_v: f32, tile: f32| {
        let mat = StandardMaterial {
            base_color: base,
            base_color_texture: t,
            perceptual_roughness: rough,
            metallic: metal_v,
            ..default()
        };
        mats.insert(m, materials.add(mat));
        tiles.insert(m, tile);
    };
    add(Mat::Rust, Color::WHITE, Some(rust.clone()), 0.75, 0.4, 3.0);
    let _ = (&dirt_w, &dirt_mr);
    add(Mat::Dirt, Color::WHITE, Some(dirt.clone()), 0.95, 0.0, 6.0);
    add(Mat::Grass, Color::WHITE, Some(grass), 0.95, 0.0, 6.0);
    add(Mat::Sand, Color::WHITE, Some(sand), 0.95, 0.0, 6.0);
    add(Mat::Rock, Color::WHITE, Some(rock), 0.95, 0.0, 8.0);
    add(Mat::Wood, Color::WHITE, Some(wood), 0.85, 0.0, 1.5);
    add(Mat::Hazard, Color::WHITE, Some(hazard.clone()), 0.6, 0.1, 2.0);
    add(Mat::Brick, Color::WHITE, Some(brick), 0.9, 0.0, 4.0);
    add(Mat::Metal, Color::WHITE, Some(metal.clone()), 0.45, 0.8, 2.0);
    add(Mat::Tiles, Color::WHITE, Some(tiles_t), 0.7, 0.0, 3.0);
    add(Mat::ContainerRed, Color::srgb(0.75, 0.18, 0.12), Some(container.clone()), 0.6, 0.3, 3.0);
    add(Mat::ContainerBlue, Color::srgb(0.15, 0.32, 0.6), Some(container.clone()), 0.6, 0.3, 3.0);
    add(Mat::ContainerYellow, Color::srgb(0.85, 0.62, 0.12), Some(container.clone()), 0.6, 0.3, 3.0);
    add(Mat::ContainerGreen, Color::srgb(0.2, 0.48, 0.3), Some(container.clone()), 0.6, 0.3, 3.0);
    add(Mat::ContainerGrey, Color::srgb(0.55, 0.55, 0.55), Some(container), 0.6, 0.3, 3.0);
    add(Mat::Ramp, Color::srgb(0.95, 0.8, 0.2), Some(hazard.clone()), 0.6, 0.1, 2.0);
    add(Mat::White, Color::srgb(0.9, 0.9, 0.88), None, 0.7, 0.0, 4.0);
    add(Mat::Black, Color::srgb(0.05, 0.05, 0.05), None, 0.8, 0.0, 4.0);
    add(Mat::Mud, Color::srgb(0.45, 0.35, 0.25), Some(dirt), 0.3, 0.0, 4.0);
    add(Mat::Ice, Color::srgb(0.75, 0.88, 1.0), None, 0.08, 0.0, 4.0);
    add(Mat::Sand, Color::srgb(0.95, 0.85, 0.65), Some(images.add(make_texture(TexKind::Sand, 256, 21))), 0.95, 0.0, 6.0);

    // Фасады: днём окна тёмные, ночью светятся
    let facade_mat = |tint: Color| StandardMaterial {
        base_color: tint,
        base_color_texture: Some(facade.clone()),
        emissive: if night { LinearRgba::rgb(2.0, 1.6, 1.0) } else { LinearRgba::BLACK },
        emissive_texture: if night { Some(facade_e.clone()) } else { None },
        perceptual_roughness: 0.6,
        ..default()
    };
    mats.insert(Mat::Facade, materials.add(facade_mat(Color::srgb(0.85, 0.82, 0.78))));
    tiles.insert(Mat::Facade, 12.0);
    mats.insert(Mat::FacadeDark, materials.add(facade_mat(Color::srgb(0.45, 0.48, 0.55))));
    tiles.insert(Mat::FacadeDark, 12.0);
    mats.insert(
        Mat::Water,
        materials.add(StandardMaterial {
            base_color: Color::srgba(0.6, 0.8, 0.9, 0.92),
            base_color_texture: Some(water),
            perceptual_roughness: 0.05,
            metallic: 0.2,
            reflectance: 0.8,
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
    );
    tiles.insert(Mat::Water, 12.0);
    mats.insert(
        Mat::Glass,
        materials.add(StandardMaterial {
            base_color: Color::srgb(0.1, 0.14, 0.18),
            perceptual_roughness: 0.05,
            metallic: 0.6,
            reflectance: 0.9,
            ..default()
        }),
    );
    let emissive = |c: LinearRgba| StandardMaterial {
        base_color: Color::BLACK,
        emissive: c,
        unlit: false,
        ..default()
    };
    mats.insert(Mat::Neon1, materials.add(emissive(LinearRgba::rgb(8.0, 0.6, 4.0))));
    mats.insert(Mat::Neon2, materials.add(emissive(LinearRgba::rgb(0.5, 4.0, 8.0))));
    mats.insert(Mat::Neon3, materials.add(emissive(LinearRgba::rgb(8.0, 5.0, 0.4))));
    mats.insert(Mat::Lamp, materials.add(emissive(LinearRgba::rgb(14.0, 7.5, 2.6))));
    mats.insert(
        Mat::Checkpoint,
        materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.85, 0.1, 0.25),
            emissive: LinearRgba::rgb(2.0, 1.5, 0.1),
            alpha_mode: AlphaMode::Add,
            unlit: true,
            cull_mode: None,
            ..default()
        }),
    );
    // Декали дороги
    let mut decal = |m: Mat, kind: u32, seed: u32, rough: f32, refl: f32| {
        let t = images.add(crate::gfx::road_decal_texture(kind, 128, seed));
        mats.insert(
            m,
            materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(t),
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: rough,
                reflectance: refl,
                depth_bias: 8.0,
                ..default()
            }),
        );
    };
    decal(Mat::DecalCrack, 0, 41, 0.9, 0.3);
    decal(Mat::DecalOil, 1, 42, 0.05, 0.8);
    decal(Mat::DecalPuddle, 2, 43, 0.02, 0.9);
    decal(Mat::DecalManhole, 3, 44, 0.5, 0.5);
    let _ = (asphalt, concrete, rust, metal);

    // CC0-текстуры ambientCG поверх процедурных, если папка assets на месте
    use crate::extassets::{load_decal, load_pbr, pbr_material, PbrOpts};
    let mut ext = |m: Mat, set: &str, o: PbrOpts, tile: f32, tweak: &dyn Fn(&mut StandardMaterial)| {
        if let Some(maps) = load_pbr(images, set, o) {
            let mut sm = pbr_material(&maps);
            tweak(&mut sm);
            mats.insert(m, materials.add(sm));
            tiles.insert(m, tile);
            true
        } else {
            false
        }
    };
    // Текстуры v3 (env_*), без них — наборы ambientCG из пакетов v1/v2
    let has_ext = ext(Mat::Asphalt, "env_asphalt", PbrOpts { wet: wet * 0.5, puddle_period: 2, tint: [0.5, 0.5, 0.52], ..default() }, 6.0, &|_| {})
        || ext(Mat::Asphalt, "Asphalt033", PbrOpts { wet: wet * 0.5, puddle_period: 2, tint: [0.42, 0.42, 0.43], ..default() }, 6.0, &|_| {})
        || ext(Mat::Asphalt, "Road012A", PbrOpts { wet, puddle_period: 2, ..default() }, 5.0, &|_| {});
    if has_ext {
        let _ = ext(Mat::Concrete, "env_concrete_floor", PbrOpts { wet: wet * 0.15, puddle_period: 2, tint: [0.8, 0.78, 0.74], ..default() }, 5.0, &|_| {})
            || ext(Mat::Concrete, "Concrete034", PbrOpts { wet: wet * 0.15, puddle_period: 2, tint: [0.74, 0.68, 0.6], ..default() }, 4.0, &|_| {});
        ext(Mat::Sidewalk, "env_sidewalk", PbrOpts { wet: wet * 0.3, puddle_period: 2, tint: [0.85, 0.85, 0.85], ..default() }, 3.2, &|_| {});
        let _ = ext(Mat::Dirt, "env_dirt", PbrOpts { wet: wet * 0.5, puddle_period: 2, ..default() }, 6.0, &|_| {})
            || ext(Mat::Dirt, "Ground037", PbrOpts { wet: wet * 0.5, puddle_period: 2, ..default() }, 5.0, &|_| {});
        let _ = ext(Mat::Sand, "env_sand", PbrOpts { wet: wet * 0.5, puddle_period: 2, ..default() }, 6.0, &|_| {})
            || ext(Mat::Sand, "Ground037", PbrOpts { wet, puddle_period: 2, tint: [0.92, 0.8, 0.62], ..default() }, 6.0, &|_| {});
        let _ = ext(Mat::Mud, "env_mud", PbrOpts { wet: 0.6, puddle_period: 3, ..default() }, 5.0, &|_| {})
            || ext(Mat::Mud, "Ground037", PbrOpts { wet: 1.0, puddle_period: 3, tint: [0.75, 0.65, 0.55], ..default() }, 4.0, &|_| {});
        ext(Mat::Grass, "env_grass", PbrOpts { wet: wet * 0.3, puddle_period: 2, ..default() }, 5.0, &|_| {});
        ext(Mat::Rock, "env_rock", PbrOpts::default(), 8.0, &|_| {});
        ext(Mat::Wood, "env_planks", PbrOpts::default(), 2.5, &|_| {});
        ext(Mat::Brick, "env_brick", PbrOpts { wet: wet * 0.2, ..default() }, 3.0, &|_| {});
        let _ = ext(Mat::Rust, "env_rust_plate", PbrOpts { wet: wet * 0.3, ..default() }, 3.0, &|_| {})
            || ext(Mat::Rust, "Metal032", PbrOpts { wet: wet * 0.3, ..default() }, 3.0, &|_| {});
        let _ = ext(Mat::Metal, "env_diamond_plate", PbrOpts { tint: [0.85, 0.85, 0.87], ..default() }, 2.0, &|_| {})
            || ext(Mat::Metal, "Metal032", PbrOpts { tint: [0.8, 0.8, 0.82], ..default() }, 2.0, &|_| {});
        // разбитый асфальт арен, полосы по стали на ограждениях и трамплинах (v3b)
        ext(Mat::AsphaltWrecked, "env_asphalt_wrecked", PbrOpts { wet: wet * 0.25, puddle_period: 2, tint: [0.55, 0.55, 0.57], ..default() }, 7.0, &|_| {});
        ext(Mat::Hazard, "env_hazard", PbrOpts { wet: wet * 0.2, ..default() }, 2.5, &|_| {});
        ext(Mat::Ramp, "env_hazard", PbrOpts { wet: wet * 0.2, ..default() }, 2.5, &|_| {});
        // контейнеры: серая гофра, окрашенная в цвет контейнера
        for (m, c) in [
            (Mat::ContainerRed, [0.85, 0.22, 0.15]),
            (Mat::ContainerBlue, [0.22, 0.4, 0.75]),
            (Mat::ContainerYellow, [0.95, 0.72, 0.18]),
            (Mat::ContainerGreen, [0.28, 0.58, 0.36]),
            (Mat::ContainerGrey, [0.7, 0.7, 0.7]),
        ] {
            ext(m, "env_corrugated", PbrOpts { tint: c, ..default() }, 3.0, &|_| {});
        }
        let wall = |sm: &mut StandardMaterial| {
            if night {
                sm.emissive = LinearRgba::rgb(2.0, 1.6, 1.0);
                sm.emissive_texture = Some(facade_e.clone());
                sm.uv_transform = bevy::math::Affine2::IDENTITY;
            }
        };
        let _ = ext(Mat::FacadeDark, "env_concrete_wall", PbrOpts { tint: [0.58, 0.6, 0.66], streaks: true, ..default() }, 12.0, &wall)
            || ext(Mat::FacadeDark, "Concrete034", PbrOpts { tint: [0.55, 0.58, 0.64], streaks: true, ..default() }, 12.0, &wall);
        // трещины-выбоины и масляные пятна из текстур v3
        for (m, file) in [(Mat::DecalCrack, "decals/decal_crater.png"), (Mat::DecalOil, "decals/decal_oil.png")] {
            if let Some(t) = crate::extassets::load_png(images, file, true, false) {
                let oil = m == Mat::DecalOil;
                mats.insert(
                    m,
                    materials.add(StandardMaterial {
                        base_color_texture: Some(t),
                        alpha_mode: AlphaMode::Blend,
                        perceptual_roughness: if oil { 0.05 } else { 0.9 },
                        reflectance: if oil { 0.8 } else { 0.3 },
                        depth_bias: 8.0,
                        ..default()
                    }),
                );
            }
        }
        // Люк и разметка с настоящей картой прозрачности
        if let Some((tex, nrm)) = load_decal(images, "ManholeCover011", None) {
            mats.insert(
                Mat::DecalManhole,
                materials.add(StandardMaterial {
                    base_color_texture: Some(tex),
                    normal_map_texture: nrm,
                    alpha_mode: AlphaMode::Mask(0.5),
                    perceptual_roughness: 0.45,
                    metallic: 0.6,
                    depth_bias: 8.0,
                    ..default()
                }),
            );
        }
        if let Some((tex, _)) = load_decal(images, "RoadLines019A", Some([235, 235, 228])) {
            let (yellow, _) = load_decal(images, "RoadLines019A", None).unwrap_or((tex.clone(), None));
            for (m, rect, t) in [
                (Mat::DecalArrow, (0.535, 0.07, 0.615, 0.33), tex.clone()),
                (Mat::DecalSlow, (0.665, 0.0, 0.80, 0.10), tex.clone()),
                (Mat::DecalHatch, (0.0, 0.667, 0.333, 1.0), yellow.clone()),
                (Mat::DecalZebra, (0.667, 0.667, 1.0, 1.0), tex.clone()),
            ] {
                let (a, b, c, d) = rect;
                mats.insert(
                    m,
                    materials.add(StandardMaterial {
                        base_color_texture: Some(t),
                        alpha_mode: AlphaMode::Blend,
                        perceptual_roughness: 0.55,
                        depth_bias: 9.0,
                        uv_transform: bevy::math::Affine2::from_scale_angle_translation(Vec2::new(c - a, d - b), 0.0, Vec2::new(a, b)),
                        ..default()
                    }),
                );
            }
        }
    }
    if !mats.contains_key(&Mat::AsphaltWrecked) {
        if let Some(h) = mats.get(&Mat::Asphalt).cloned() {
            mats.insert(Mat::AsphaltWrecked, h);
        }
    }
    // Плакаты (бумага, матовые) и граффити (краска с альфой) для стен
    let mut posters = Vec::new();
    let mut graffiti = Vec::new();
    for i in 1..=4 {
        if let Some(t) = crate::extassets::load_jpg_image(images, &format!("decals/poster_{i}.jpg")) {
            posters.push(materials.add(StandardMaterial { base_color_texture: Some(t), perceptual_roughness: 0.85, depth_bias: 6.0, ..default() }));
        }
    }
    for i in 1..=6 {
        if let Some(t) = crate::extassets::load_png(images, &format!("decals/graffiti_{i}.png"), true, false) {
            graffiti.push(materials.add(StandardMaterial { base_color_texture: Some(t), alpha_mode: AlphaMode::Blend, perceptual_roughness: 0.7, depth_bias: 6.0, ..default() }));
        }
    }
    // новые фасады с окнами (ночью светятся тёплые окна) и брусчатка
    if has_ext {
        for (m, set) in [(Mat::Facade, "env_facade_brick"), (Mat::FacadeDark, "env_facade_concrete")] {
            let em = if night { crate::extassets::load_png(images, &format!("textures/{set}/{set}_emissive.png"), true, true) } else { None };
            if let Some(maps) = load_pbr(images, set, PbrOpts { streaks: false, ..default() }) {
                let mut sm = pbr_material(&maps);
                if let Some(e) = em {
                    sm.emissive = LinearRgba::rgb(5.0, 4.0, 2.8);
                    sm.emissive_texture = Some(e);
                }
                mats.insert(m, materials.add(sm));
                tiles.insert(m, 20.0);
            }
        }
        if let Some(maps) = load_pbr(images, "env_cobble", PbrOpts { wet: wet * 0.4, puddle_period: 2, ..default() }) {
            mats.insert(Mat::Tiles, materials.add(pbr_material(&maps)));
            tiles.insert(Mat::Tiles, 4.0);
        }
    }
    MatLib { mats, tex_tile: tiles, blob, splat, posters, graffiti }
}

// ============================================================ тема освещения

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub sun_dir: Vec3,
    pub sun_color: Color,
    pub sun_lux: f32,
    pub ambient: Color,
    pub ambient_brightness: f32,
    pub fog: Color,
    pub fog_start: f32,
    pub fog_end: f32,
    pub sky_top: Color,
    pub sky_horizon: Color,
    pub night: bool,
    /// Экспозиция камеры, EV100 (меньше — ярче)
    pub exposure: f32,
    /// Дальность видимости в тумане, м
    pub fog_vis: f32,
    /// Насколько мокрый асфальт: 0..1
    pub wet: f32,
    /// Плотность объёмного тумана для лучей света
    pub haze: f32,
    pub stars: bool,
    /// Цветокоррекция: температура (минус — холоднее) и насыщенность
    pub grade_temp: f32,
    pub grade_sat: f32,
}

// ============================================================ данные карты

#[derive(Clone, Copy, Debug)]
pub struct Obstacle {
    pub center: Vec2,
    pub half: Vec2,
    pub yaw: f32,
    pub bottom: f32,
    pub top: f32,
    pub color: Color,
}

#[derive(Clone, Copy, Debug)]
pub struct Zone {
    pub min: Vec2,
    pub max: Vec2,
    pub value: f32,
}

impl Zone {
    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }
}

pub struct Grid {
    pub cell: f32,
    pub nx: usize,
    pub nz: usize,
    pub origin: Vec2,
    pub height: Vec<f32>,
    pub blocked: Vec<bool>,
    pub has_ground: Vec<bool>,
}

impl Grid {
    pub fn new(half: Vec2, cell: f32) -> Self {
        let nx = (half.x * 2.0 / cell).ceil() as usize + 1;
        let nz = (half.y * 2.0 / cell).ceil() as usize + 1;
        Self {
            cell,
            nx,
            nz,
            origin: -half,
            height: vec![0.0; nx * nz],
            blocked: vec![false; nx * nz],
            has_ground: vec![true; nx * nz],
        }
    }
    pub fn idx(&self, x: f32, z: f32) -> Option<usize> {
        let i = ((x - self.origin.x) / self.cell).round();
        let j = ((z - self.origin.y) / self.cell).round();
        if i < 0.0 || j < 0.0 || i as usize >= self.nx || j as usize >= self.nz {
            None
        } else {
            Some(j as usize * self.nx + i as usize)
        }
    }
    pub fn pos(&self, i: usize, j: usize) -> Vec2 {
        self.origin + Vec2::new(i as f32, j as f32) * self.cell
    }
}

#[derive(Default, Clone)]
pub struct NavGraph {
    pub nodes: Vec<Vec3>,
    pub edges: Vec<Vec<(usize, f32)>>,
}

impl NavGraph {
    pub fn add_node(&mut self, p: Vec3) -> usize {
        self.nodes.push(p);
        self.edges.push(Vec::new());
        self.nodes.len() - 1
    }
    pub fn link(&mut self, a: usize, b: usize) {
        if a == b {
            return;
        }
        let d = self.nodes[a].distance(self.nodes[b]);
        if !self.edges[a].iter().any(|(n, _)| *n == b) {
            self.edges[a].push((b, d));
        }
        if !self.edges[b].iter().any(|(n, _)| *n == a) {
            self.edges[b].push((a, d));
        }
    }
    pub fn nearest(&self, p: Vec3) -> Option<usize> {
        let mut best = None;
        let mut bd = f32::MAX;
        for (i, n) in self.nodes.iter().enumerate() {
            if self.edges[i].is_empty() {
                continue;
            }
            let d = n.distance_squared(p) + (n.y - p.y).powi(2) * 8.0;
            if d < bd {
                bd = d;
                best = Some(i);
            }
        }
        best
    }
    /// A* по графу. Возвращает список узлов от старта до цели.
    pub fn path(&self, from: usize, to: usize) -> Option<Vec<usize>> {
        if from == to {
            return Some(vec![from]);
        }
        #[derive(PartialEq)]
        struct S(f32, usize);
        impl Eq for S {}
        impl PartialOrd for S {
            fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(o))
            }
        }
        impl Ord for S {
            fn cmp(&self, o: &Self) -> std::cmp::Ordering {
                o.0.partial_cmp(&self.0).unwrap_or(std::cmp::Ordering::Equal)
            }
        }
        let n = self.nodes.len();
        let mut g = vec![f32::MAX; n];
        let mut came = vec![usize::MAX; n];
        let mut open = BinaryHeap::new();
        g[from] = 0.0;
        open.push(S(self.nodes[from].distance(self.nodes[to]), from));
        let mut iters = 0;
        while let Some(S(_, cur)) = open.pop() {
            iters += 1;
            if iters > 20000 {
                break;
            }
            if cur == to {
                let mut path = vec![to];
                let mut c = to;
                while came[c] != usize::MAX {
                    c = came[c];
                    path.push(c);
                }
                path.reverse();
                return Some(path);
            }
            for &(nb, cost) in &self.edges[cur] {
                let ng = g[cur] + cost;
                if ng < g[nb] {
                    g[nb] = ng;
                    came[nb] = cur;
                    open.push(S(ng + self.nodes[nb].distance(self.nodes[to]), nb));
                }
            }
        }
        None
    }
}

/// Траектория опасности (поезд, лопасть, маятник, пресс): позиция зависит только от времени матча,
/// поэтому у клиентов совпадает без передачи по сети.
#[derive(Clone, Debug)]
pub enum MoverPath {
    /// Прямая A→B со скоростью, затем пауза; период period
    Line { a: Vec3, b: Vec3, speed: f32, period: f32, yaw: f32 },
    /// Вращение вокруг Y
    Spin { center: Vec3, rate: f32 },
    /// Маятник: точка подвеса, длина, амплитуда, период, плоскость (yaw)
    Pendulum { pivot: Vec3, length: f32, amp: f32, period: f32, yaw: f32 },
    /// Пресс: вверх-вниз
    Press { base: Vec3, lift: f32, period: f32 },
    /// Замкнутая ломаная
    Loop { points: Vec<Vec3>, speed: f32 },
}

impl MoverPath {
    pub fn eval(&self, t: f32) -> (Vec3, Quat) {
        match self {
            MoverPath::Line { a, b, speed, period, yaw } => {
                let len = a.distance(*b);
                let travel = len / speed;
                let ph = t.rem_euclid(*period);
                if ph < travel {
                    (a.lerp(*b, ph / travel), Quat::from_rotation_y(*yaw))
                } else {
                    (Vec3::new(a.x, -80.0, a.z), Quat::from_rotation_y(*yaw))
                }
            }
            MoverPath::Spin { center, rate } => (*center, Quat::from_rotation_y(t * rate)),
            MoverPath::Pendulum { pivot, length, amp, period, yaw } => {
                let ang = amp * (t / period * std::f32::consts::TAU).sin();
                let rot = Quat::from_rotation_y(*yaw) * Quat::from_rotation_z(ang);
                (*pivot + rot * Vec3::NEG_Y * *length, rot)
            }
            MoverPath::Press { base, lift, period } => {
                let ph = (t / period).fract();
                // быстро вниз (0.0..0.08), пауза внизу, медленно вверх
                let h = if ph < 0.08 {
                    1.0 - ph / 0.08
                } else if ph < 0.2 {
                    0.0
                } else {
                    ((ph - 0.2) / 0.5).min(1.0)
                };
                (*base + Vec3::Y * h * lift, Quat::IDENTITY)
            }
            MoverPath::Loop { points, speed } => {
                let n = points.len();
                let mut total = 0.0;
                for i in 0..n {
                    total += points[i].distance(points[(i + 1) % n]);
                }
                let mut d = (t * speed).rem_euclid(total);
                for i in 0..n {
                    let a = points[i];
                    let b = points[(i + 1) % n];
                    let l = a.distance(b);
                    if d <= l {
                        let p = a.lerp(b, d / l.max(0.001));
                        let dir = b - a;
                        let yaw = (-dir.x).atan2(-dir.z);
                        return (p, Quat::from_rotation_y(yaw));
                    }
                    d -= l;
                }
                (points[0], Quat::IDENTITY)
            }
        }
    }
}

#[derive(Component)]
pub struct Mover {
    pub path: MoverPath,
    pub name: &'static str,
}

/// Реквизит: бочки, ящики, конусы. Снапшоты передают только сдвинутые.
#[derive(Component)]
pub struct Prop {
    pub id: u32,
    pub explosive: bool,
    pub kind: PropKind,
    pub alive: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PropKind {
    Barrel,
    ExplosiveBarrel,
    Crate,
    Cone,
    Tires,
    Pallet,
}

#[derive(Component)]
pub struct CheckpointMarker(pub usize);

#[derive(Resource)]
pub struct MapData {
    pub id: MapId,
    pub half: Vec2,
    pub kill_y: f32,
    pub water: Vec<Zone>,
    pub surfaces: Vec<Zone>,
    pub grid: Grid,
    pub obstacles: Vec<Obstacle>,
    pub nav: NavGraph,
    pub spawns: Vec<(Vec3, f32)>,
    pub pickups: Vec<Vec3>,
    pub checkpoints: Vec<Vec3>,
    pub ped_zones: Vec<Zone>,
    pub theme: Theme,
    pub props_count: u32,
}

impl MapData {
    pub fn surface_grip(&self, p: Vec3) -> f32 {
        let q = Vec2::new(p.x, p.z);
        for z in &self.surfaces {
            if z.contains(q) && p.y < z.value.abs() + 3.0 {
                return if z.value < 0.0 { 1.0 } else { z.value.min(1.0) };
            }
        }
        1.0
    }
    pub fn is_water(&self, p: Vec3) -> bool {
        let q = Vec2::new(p.x, p.z);
        self.water.iter().any(|w| w.contains(q) && p.y < w.value - 0.2)
    }
    pub fn ground_height(&self, x: f32, z: f32) -> f32 {
        self.grid.idx(x, z).map(|i| self.grid.height[i]).unwrap_or(0.0)
    }
    pub fn is_blocked(&self, x: f32, z: f32) -> bool {
        match self.grid.idx(x, z) {
            Some(i) => self.grid.blocked[i] || !self.grid.has_ground[i],
            None => true,
        }
    }
    pub fn in_bounds(&self, p: Vec3) -> bool {
        p.x.abs() < self.half.x - 2.0 && p.z.abs() < self.half.y - 2.0
    }
    /// Ближайшая свободная точка дороги и направление на соседний узел.
    pub fn recover_spot(&self, near: Vec3, occupied: &[Vec3]) -> (Vec3, f32) {
        let mut best: Option<(usize, f32)> = None;
        for (i, n) in self.nav.nodes.iter().enumerate() {
            if self.nav.edges[i].is_empty() {
                continue;
            }
            if occupied.iter().any(|o| o.distance(*n) < 5.0) {
                continue;
            }
            let d = n.distance(near);
            if best.map(|b| d < b.1).unwrap_or(true) {
                best = Some((i, d));
            }
        }
        if let Some((i, _)) = best {
            let p = self.nav.nodes[i];
            let mut yaw = 0.0;
            // Направление на соседа, который дальше от края арены
            let mut best_score = f32::MIN;
            for &(nb, _) in &self.nav.edges[i] {
                let q = self.nav.nodes[nb];
                let score = -q.length();
                if score > best_score {
                    best_score = score;
                    let d = q - p;
                    yaw = (-d.x).atan2(-d.z);
                }
            }
            return (p, yaw);
        }
        if let Some((p, yaw)) = self.spawns.first() {
            return (*p, *yaw);
        }
        (Vec3::ZERO, 0.0)
    }
    /// Видимость между точками по 2D-препятствиям (быстрая проверка без физики).
    pub fn line_clear(&self, a: Vec3, b: Vec3) -> bool {
        let d = Vec2::new(b.x - a.x, b.z - a.z);
        let len = d.length();
        let steps = (len / (self.grid.cell * 0.75)).ceil().max(1.0) as i32;
        let mut prev_h = self.ground_height(a.x, a.z);
        for k in 0..=steps {
            let t = k as f32 / steps as f32;
            let x = a.x + d.x * t;
            let z = a.z + d.y * t;
            if self.is_blocked(x, z) {
                return false;
            }
            let h = self.ground_height(x, z);
            if (h - prev_h).abs() > self.grid.cell * 0.9 + 0.6 {
                return false;
            }
            prev_h = h;
        }
        true
    }
}

// ============================================================ строитель

pub struct MapBuilder<'a, 'w, 's> {
    pub commands: &'a mut Commands<'w, 's>,
    pub meshes: &'a mut Assets<Mesh>,
    pub mats: &'a MatLib,
    pub std_mats: &'a mut Assets<StandardMaterial>,
    pub rng: StdRng,
    pub data: MapData,
    pub authority: bool,
    pub visuals: bool,
    pub light_budget: i32,
    pub prop_id: u32,
    pub manual_nav: Vec<(Vec3, Vec<usize>)>,
    /// Асфальтовые участки (центр верхней грани, половина размера, поворот) для декалей
    pub asphalt: Vec<(Vec3, Vec2, f32)>,
    /// Модели Kenney (если загружены)
    pub kenney: Option<crate::extassets::KenneyLib>,
}

#[derive(Component)]
pub struct MapEntity;

impl<'a, 'w, 's> MapBuilder<'a, 'w, 's> {
    pub fn tag(&mut self) -> (MapEntity, DespawnOnExit<AppState>) {
        (MapEntity, DespawnOnExit(AppState::Race))
    }

    /// Сцена Kenney как декор (без коллизии). weathered: затемнение под ржавчину.
    pub fn scene(&mut self, handle: Handle<WorldAsset>, transform: Transform, weathered: Option<f32>) {
        if !self.visuals {
            return;
        }
        let tag = self.tag();
        let mut ec = self.commands.spawn((WorldAssetRoot(handle), transform, tag));
        if let Some(k) = weathered {
            ec.insert(crate::extassets::Weathered(k));
        }
    }

    /// Модель из наборов Kenney (assets/kenney/<набор>/<имя>.glb): низ на pos.y, центр габарита в pos.
    /// Коллизия (по габариту из kitbox) создаётся и без графики, чтобы сервер и тесты видели ту же карту.
    /// scale — метров на единицу модели. Возвращает размер в метрах.
    pub fn kit(&mut self, name: &str, pos: Vec3, yaw: f32, scale: f32, collide: bool) -> Vec3 {
        let Some((mn, mx)) = crate::kitbox::bbox(name) else {
            warn!("нет модели набора {name}");
            return Vec3::ZERO;
        };
        let (mn, mx) = (Vec3::from_array(mn) * scale, Vec3::from_array(mx) * scale);
        let size = mx - mn;
        let rot = Quat::from_rotation_y(yaw);
        let mid = Vec3::new((mn.x + mx.x) * 0.5, 0.0, (mn.z + mx.z) * 0.5);
        // начало модели так, чтобы центр габарита оказался в pos, а низ на pos.y
        let origin = pos - rot * mid - Vec3::Y * mn.y;
        let handle = self.kenney.as_ref().and_then(|k| k.kit.get(name).cloned());
        if collide {
            let e = self.solid(pos + Vec3::Y * size.y * 0.5, size, yaw, Mat::Concrete);
            if handle.is_some() {
                self.commands.entity(e).remove::<(Mesh3d, MeshMaterial3d<StandardMaterial>)>();
            }
        } else if handle.is_none() && self.visuals && size.y > 0.5 {
            // без моделей: серая коробка-заглушка, чтобы силуэт карты не пропадал
            self.deco(pos + Vec3::Y * size.y * 0.5, size, yaw, Mat::Concrete);
        }
        if let (Some(h), true) = (handle, self.visuals) {
            let tag = self.tag();
            let mut ec = self.commands.spawn((WorldAssetRoot(h), Transform::from_translation(origin).with_rotation(rot).with_scale(Vec3::splat(scale)), tag));
            // ночью у зданий светятся окна; часть домов тёмные
            if self.data.theme.night && name.contains("building") {
                let kit = name.split('/').next().unwrap_or("");
                let r = MapBuilder::hash01(pos, 81);
                if let (Some(mask), true) = (self.kenney.as_ref().and_then(|k| k.windows.get(kit).cloned()), r > 0.15) {
                    ec.insert(crate::extassets::NightLit(mask, 1.5 + r * 3.5));
                }
            }
        }
        size
    }

    /// Детерминированный выбор из списка по позиции (не трогает ГСЧ карты).
    pub fn pick<T: Clone>(list: &[T], p: Vec3, salt: u32) -> Option<T> {
        if list.is_empty() {
            return None;
        }
        let h = ((p.x * 73.0) as i32 as u32).wrapping_mul(2654435761) ^ ((p.z * 37.0) as i32 as u32).wrapping_mul(40503) ^ salt.wrapping_mul(97);
        Some(list[(h as usize) % list.len()].clone())
    }

    pub fn hash01(p: Vec3, salt: u32) -> f32 {
        let h = ((p.x * 91.0) as i32 as u32).wrapping_mul(2246822519) ^ ((p.z * 53.0) as i32 as u32).wrapping_mul(3266489917) ^ salt.wrapping_mul(668265263);
        (h >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Статичная коробка с коллизией. Регистрирует препятствие (если блокирует проезд).
    pub fn solid(&mut self, center: Vec3, size: Vec3, yaw: f32, mat: Mat) -> Entity {
        let e = self.static_box(center, size, yaw, mat, true);
        self.register_box(center, size, yaw, mat);
        e
    }

    pub fn register_box(&mut self, center: Vec3, size: Vec3, yaw: f32, mat: Mat) {
        let color = match mat {
            Mat::ContainerRed => Color::srgb(0.6, 0.2, 0.15),
            Mat::ContainerBlue => Color::srgb(0.2, 0.3, 0.5),
            Mat::ContainerYellow => Color::srgb(0.6, 0.5, 0.15),
            Mat::ContainerGreen => Color::srgb(0.2, 0.4, 0.25),
            Mat::Water => Color::srgb(0.1, 0.25, 0.4),
            _ => Color::srgb(0.45, 0.45, 0.48),
        };
        self.data.obstacles.push(Obstacle {
            center: Vec2::new(center.x, center.z),
            half: Vec2::new(size.x, size.z) * 0.5,
            yaw,
            bottom: center.y - size.y * 0.5,
            top: center.y + size.y * 0.5,
            color,
        });
    }

    pub fn static_box(&mut self, center: Vec3, size: Vec3, yaw: f32, mat: Mat, collide: bool) -> Entity {
        let tag = self.tag();
        let mut ec = self.commands.spawn((Transform::from_translation(center).with_rotation(Quat::from_rotation_y(yaw)), tag));
        if self.visuals {
            let mesh = self.meshes.add(box_mesh(size, self.mats.tile(mat)));
            ec.insert((Mesh3d(mesh), MeshMaterial3d(self.mats.get(mat))));
        }
        if collide {
            ec.insert((RigidBody::Static, Collider::cuboid(size.x, size.y, size.z), CollisionLayers::new(Layer::World, LayerMask::ALL)));
        }
        ec.id()
    }

    /// Чисто визуальная коробка без коллизии и без регистрации.
    pub fn deco(&mut self, center: Vec3, size: Vec3, yaw: f32, mat: Mat) {
        if !self.visuals {
            return;
        }
        if mat == Mat::Asphalt && size.y < 0.3 && size.x > 4.0 && size.z > 4.0 {
            self.asphalt.push((center + Vec3::Y * size.y * 0.5, Vec2::new(size.x, size.z) * 0.5, yaw));
        }
        let tag = self.tag();
        let mesh = self.meshes.add(box_mesh(size, self.mats.tile(mat)));
        self.commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(self.mats.get(mat)),
            Transform::from_translation(center).with_rotation(Quat::from_rotation_y(yaw)),
            tag,
        ));
    }

    pub fn deco_mesh(&mut self, mesh: Mesh, mat: Mat, transform: Transform) {
        if !self.visuals {
            return;
        }
        let tag = self.tag();
        let h = self.meshes.add(mesh);
        self.commands.spawn((Mesh3d(h), MeshMaterial3d(self.mats.get(mat)), transform, tag));
    }

    /// Плоская поверхность, по которой можно ездить (пол, площадка). Пишет высоту в сетку.
    pub fn floor(&mut self, center: Vec3, size: Vec3, mat: Mat) {
        self.static_box(center, size, 0.0, mat, true);
        let top = center.y + size.y * 0.5;
        self.write_height_rect(Vec2::new(center.x, center.z), Vec2::new(size.x, size.z) * 0.5, 0.0, |_, _| top);
    }

    /// Трамплин: въезд со стороны -Z локально, обрыв на +Z. yaw поворачивает по направлению прыжка.
    pub fn ramp(&mut self, base_center: Vec3, width: f32, length: f32, height: f32, yaw: f32, mat: Mat) {
        let tag = self.tag();
        let rot = Quat::from_rotation_y(yaw);
        let mut ec = self.commands.spawn((Transform::from_translation(base_center).with_rotation(rot), tag));
        if self.visuals {
            let mesh = self.meshes.add(wedge_mesh(width, length, height, self.mats.tile(mat)));
            ec.insert((Mesh3d(mesh), MeshMaterial3d(self.mats.get(mat))));
        }
        let hull = Collider::convex_hull(wedge_points(width, length, height)).unwrap_or(Collider::cuboid(width, height, length));
        ec.insert((RigidBody::Static, hull, CollisionLayers::new(Layer::World, LayerMask::ALL)));
        let b = base_center;
        self.write_height_rect(Vec2::new(b.x, b.z), Vec2::new(width, length) * 0.5, yaw, move |_lx, lz| {
            b.y + ((lz + length * 0.5) / length).clamp(0.0, 1.0) * height
        });
    }

    /// Наклонная плоскость (съезд между уровнями): от (start,h0) до (end,h1).
    pub fn slope(&mut self, start: Vec3, end: Vec3, width: f32, thickness: f32, mat: Mat) {
        let d = end - start;
        let horiz_len = Vec2::new(d.x, d.z).length();
        let len = d.length();
        let yaw = (-d.x).atan2(-d.z);
        let pitch = (d.y / horiz_len.max(0.01)).atan();
        let center = (start + end) * 0.5 - Vec3::Y * thickness * 0.5;
        let rot = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch);
        let tag = self.tag();
        let mut ec = self.commands.spawn((Transform::from_translation(center).with_rotation(rot), tag));
        if self.visuals {
            let mesh = self.meshes.add(box_mesh(Vec3::new(width, thickness, len), self.mats.tile(mat)));
            ec.insert((Mesh3d(mesh), MeshMaterial3d(self.mats.get(mat))));
        }
        ec.insert((RigidBody::Static, Collider::cuboid(width, thickness, len), CollisionLayers::new(Layer::World, LayerMask::ALL)));
        let s = start;
        let e = end;
        self.write_height_rect(Vec2::new((s.x + e.x) * 0.5, (s.z + e.z) * 0.5), Vec2::new(width * 0.5, horiz_len * 0.5), yaw, move |_lx, lz| {
            // lz: -half у start? локальная ось -Z направлена от start к end
            let t = ((-lz) / horiz_len + 0.5).clamp(0.0, 1.0);
            s.y + (e.y - s.y) * t
        });
    }

    pub fn cylinder(&mut self, center: Vec3, radius: f32, height: f32, mat: Mat, solid: bool) {
        let tag = self.tag();
        let mut ec = self.commands.spawn((Transform::from_translation(center), tag));
        if self.visuals {
            let mesh = self.meshes.add(Cylinder::new(radius, height).mesh().resolution(16));
            ec.insert((Mesh3d(mesh), MeshMaterial3d(self.mats.get(mat))));
        }
        if solid {
            ec.insert((RigidBody::Static, Collider::cylinder(radius, height), CollisionLayers::new(Layer::World, LayerMask::ALL)));
            self.data.obstacles.push(Obstacle {
                center: Vec2::new(center.x, center.z),
                half: Vec2::splat(radius),
                yaw: 0.0,
                bottom: center.y - height * 0.5,
                top: center.y + height * 0.5,
                color: Color::srgb(0.45, 0.45, 0.48),
            });
        }
    }

    /// Записывает высоту поверхности в сетку для прямоугольника (локальные lx, lz → высота).
    pub fn write_height_rect(&mut self, center: Vec2, half: Vec2, yaw: f32, f: impl Fn(f32, f32) -> f32) {
        let g = &mut self.data.grid;
        let r = half.length() + g.cell;
        let (s, c) = yaw.sin_cos();
        let i0 = (((center.x - r) - g.origin.x) / g.cell).floor().max(0.0) as usize;
        let i1 = ((((center.x + r) - g.origin.x) / g.cell).ceil() as usize).min(g.nx - 1);
        let j0 = (((center.y - r) - g.origin.y) / g.cell).floor().max(0.0) as usize;
        let j1 = ((((center.y + r) - g.origin.y) / g.cell).ceil() as usize).min(g.nz - 1);
        for j in j0..=j1 {
            for i in i0..=i1 {
                let p = g.pos(i, j) - center;
                // поворот мира в локальные: инверсия yaw вокруг Y (x' = x cos - z sin ...)
                let lx = p.x * c - p.y * s;
                let lz = p.x * s + p.y * c;
                if lx.abs() <= half.x && lz.abs() <= half.y {
                    let k = j * g.nx + i;
                    let h = f(lx, lz);
                    if h > g.height[k] - 0.01 || !g.has_ground[k] {
                        g.height[k] = h;
                        g.has_ground[k] = true;
                    }
                }
            }
        }
    }

    pub fn mark_no_ground(&mut self, min: Vec2, max: Vec2) {
        let g = &mut self.data.grid;
        for j in 0..g.nz {
            for i in 0..g.nx {
                let p = g.pos(i, j);
                if p.x >= min.x && p.x <= max.x && p.y >= min.y && p.y <= max.y {
                    g.has_ground[j * g.nx + i] = false;
                }
            }
        }
    }

    pub fn water(&mut self, min: Vec2, max: Vec2, level: f32) {
        self.data.water.push(Zone { min, max, value: level });
        let size = max - min;
        let center = (min + max) * 0.5;
        if self.visuals {
            let tag = self.tag();
            let mesh = self.meshes.add(box_mesh(Vec3::new(size.x, 0.1, size.y), self.mats.tile(Mat::Water)));
            self.commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(self.mats.get(Mat::Water)),
                Transform::from_xyz(center.x, level, center.y),
                tag,
                crate::effects::WaterSurface,
            ));
        }
        // дно
        self.static_box(Vec3::new(center.x, level - 6.0, center.y), Vec3::new(size.x, 1.0, size.y), 0.0, Mat::Dirt, true);
        self.mark_no_ground(min, max);
    }

    pub fn surface(&mut self, min: Vec2, max: Vec2, grip: f32) {
        self.data.surfaces.push(Zone { min, max, value: grip });
    }

    pub fn point_light(&mut self, pos: Vec3, color: Color, intensity: f32, range: f32) {
        if !self.visuals || self.light_budget <= 0 {
            return;
        }
        self.light_budget -= 1;
        let tag = self.tag();
        self.commands.spawn((
            PointLight {
                color,
                intensity,
                range,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(pos),
            tag,
        ));
    }

    /// Прожектор на мачте: конус тёплого света, видимый в объёмном тумане.
    #[allow(clippy::too_many_arguments)]
    pub fn spot_light(&mut self, pos: Vec3, target: Vec3, color: Color, intensity: f32, range: f32, angle: f32, shadows: bool) {
        if !self.visuals || self.light_budget <= 0 {
            return;
        }
        self.light_budget -= 1;
        let tag = self.tag();
        self.commands.spawn((
            SpotLight {
                color,
                intensity,
                range,
                outer_angle: angle,
                inner_angle: angle * 0.7,
                shadow_maps_enabled: shadows,
                contact_shadows_enabled: shadows,
                ..default()
            },
            bevy::light::VolumetricLight,
            Transform::from_translation(pos).looking_at(target, Vec3::Y),
            tag,
        ));
    }

    /// Фонарный столб: визуальная модель и (если хватает бюджета) натриевый свет.
    pub fn street_lamp(&mut self, base: Vec3, light: bool) {
        let lamp_model = self.kenney.as_ref().and_then(|k| k.lamp.clone());
        if let Some(h) = lamp_model {
            // столб Kenney (стрела по -Z, поворачиваем на +X), коллизия — невидимый цилиндр
            let vis = self.visuals;
            self.visuals = false;
            self.cylinder(base + Vec3::Y * 3.5, 0.15, 7.0, Mat::Metal, true);
            self.visuals = vis;
            self.scene(h, Transform::from_translation(base).with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)).with_scale(Vec3::splat(10.8)), None);
            self.deco(base + Vec3::new(2.05, 6.95, 0.0), Vec3::new(0.5, 0.06, 0.3), 0.0, Mat::Lamp);
        } else {
            self.cylinder(base + Vec3::Y * 3.5, 0.12, 7.0, Mat::Metal, true);
            self.deco(base + Vec3::new(0.35, 7.0, 0.0), Vec3::new(1.0, 0.18, 0.45), 0.0, Mat::Metal);
            self.deco(base + Vec3::new(0.6, 6.88, 0.0), Vec3::new(0.55, 0.08, 0.32), 0.0, Mat::Lamp);
        }
        let head = if self.kenney.as_ref().and_then(|k| k.lamp.as_ref()).is_some() { Vec3::new(2.05, 6.85, 0.0) } else { Vec3::new(0.6, 6.7, 0.0) };
        let _ = light;
        if self.data.theme.night {
            if self.light_budget > 8 {
                self.light_budget -= 1;
                let tag = self.tag();
                self.commands.spawn((
                    SpotLight {
                        color: Color::srgb(1.0, 0.62, 0.3),
                        intensity: 260_000.0,
                        range: 26.0,
                        outer_angle: 1.05,
                        inner_angle: 0.6,
                        shadow_maps_enabled: false,
                        ..default()
                    },
                    bevy::light::VolumetricLight,
                    Transform::from_translation(base + head).looking_at(base + head * Vec3::new(1.1, 0.0, 1.0), Vec3::Z),
                    tag,
                ));
            }
        }
    }

    pub fn prop(&mut self, kind: PropKind, pos: Vec3, yaw: f32) {
        let tag = self.tag();
        self.prop_id += 1;
        let (collider, mass, size) = match kind {
            PropKind::Barrel | PropKind::ExplosiveBarrel => (Collider::cylinder(0.4, 1.0), 40.0, Vec3::new(0.8, 1.0, 0.8)),
            PropKind::Crate => (Collider::cuboid(1.1, 1.1, 1.1), 30.0, Vec3::splat(1.1)),
            PropKind::Cone => (Collider::cylinder(0.25, 0.75), 4.0, Vec3::new(0.5, 0.75, 0.5)),
            PropKind::Tires => (Collider::cylinder(0.45, 0.9), 50.0, Vec3::new(0.9, 0.9, 0.9)),
            PropKind::Pallet => (Collider::cuboid(1.2, 0.3, 1.0), 15.0, Vec3::new(1.2, 0.3, 1.0)),
        };
        let center = pos + Vec3::Y * size.y * 0.5;
        let mut ec = self.commands.spawn((
            Transform::from_translation(center).with_rotation(Quat::from_rotation_y(yaw)),
            Prop { id: self.prop_id, explosive: kind == PropKind::ExplosiveBarrel, kind, alive: true },
            NetId(1_000_000 + self.prop_id),
            tag,
        ));
        let cone_model = if kind == PropKind::Cone { self.kenney.as_ref().and_then(|k| k.cone.clone()) } else { None };
        if let Some(h) = cone_model.filter(|_| self.visuals) {
            ec.insert(Visibility::default());
            ec.with_children(|c| {
                c.spawn((WorldAssetRoot(h), Transform::from_xyz(0.0, -size.y * 0.5, 0.0).with_scale(Vec3::splat(1.25))));
            });
        } else if self.visuals {
            let (mesh, mat): (Mesh, Handle<StandardMaterial>) = match kind {
                PropKind::Barrel => (Cylinder::new(0.4, 1.0).mesh().resolution(14).build(), self.mats.get(Mat::Rust)),
                PropKind::ExplosiveBarrel => (
                    Cylinder::new(0.4, 1.0).mesh().resolution(14).build(),
                    self.std_mats.add(StandardMaterial {
                        base_color: Color::srgb(0.85, 0.12, 0.08),
                        emissive: LinearRgba::rgb(0.4, 0.02, 0.0),
                        perceptual_roughness: 0.5,
                        metallic: 0.3,
                        ..default()
                    }),
                ),
                PropKind::Crate => (box_mesh(Vec3::splat(1.1), 1.1), self.mats.get(Mat::Wood)),
                PropKind::Cone => (
                    Cone { radius: 0.25, height: 0.75 }.mesh().resolution(10).build(),
                    self.std_mats.add(StandardMaterial { base_color: Color::srgb(1.0, 0.4, 0.05), ..default() }),
                ),
                PropKind::Tires => (Torus::new(0.22, 0.45).mesh().build(), self.mats.get(Mat::Black)),
                PropKind::Pallet => (box_mesh(Vec3::new(1.2, 0.3, 1.0), 1.0), self.mats.get(Mat::Wood)),
            };
            ec.insert((Mesh3d(self.meshes.add(mesh)), MeshMaterial3d(mat)));
        }
        if self.authority {
            ec.insert((
                RigidBody::Dynamic,
                collider,
                Mass(mass),
                Friction::new(0.6),
                CollisionLayers::new(Layer::Prop, [Layer::World, Layer::Car, Layer::Prop, Layer::Corpse]),
                LinearDamping(0.1),
                AngularDamping(0.3),
            ));
        } else {
            ec.insert(crate::net::client::NetInterp::default());
        }
        self.data.props_count = self.prop_id;
    }

    pub fn mover(&mut self, path: MoverPath, collider: Collider, mesh_parts: Vec<(Mesh, Mat, Transform)>, name: &'static str) {
        let tag = self.tag();
        let (p, r) = path.eval(0.0);
        let mut ec = self.commands.spawn((
            Transform::from_translation(p).with_rotation(r),
            Mover { path, name },
            RigidBody::Kinematic,
            collider,
            CollisionLayers::new(Layer::World, LayerMask::ALL),
            tag,
        ));
        if self.visuals {
            let parts: Vec<(Handle<Mesh>, Handle<StandardMaterial>, Transform)> = mesh_parts
                .into_iter()
                .map(|(m, mat, t)| (self.meshes.add(m), self.mats.get(mat), t))
                .collect();
            ec.with_children(|c| {
                for (m, mat, t) in parts {
                    c.spawn((Mesh3d(m), MeshMaterial3d(mat), t));
                }
            });
        }
    }

    pub fn ped_zone(&mut self, min: Vec2, max: Vec2) {
        self.data.ped_zones.push(Zone { min, max, value: 0.0 });
    }

    pub fn spawn_point(&mut self, pos: Vec3, yaw: f32) {
        self.data.spawns.push((pos, yaw));
    }

    pub fn pickup(&mut self, pos: Vec3) {
        self.data.pickups.push(pos);
    }

    pub fn checkpoint(&mut self, pos: Vec3) {
        self.data.checkpoints.push(pos);
    }

    /// Ручной узел навигации (для этажей, мостов). Связи с другими ручными узлами по индексам в manual_nav.
    pub fn nav_manual(&mut self, pos: Vec3, links: Vec<usize>) -> usize {
        self.manual_nav.push((pos, links));
        self.manual_nav.len() - 1
    }

    pub fn rand(&mut self, a: f32, b: f32) -> f32 {
        self.rng.gen_range(a..b)
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.rng.gen::<f32>() < p
    }

    // -------------------------------------------------------- финализация

    /// Растеризует препятствия в сетку, строит навигационный граф.
    /// Трещины, масляные пятна, лужи и люки на асфальте (только визуал).
    fn scatter_road_decals(&mut self) {
        if !self.visuals || self.asphalt.is_empty() {
            return;
        }
        let mut rng = StdRng::seed_from_u64(0xDECA1 ^ self.asphalt.len() as u64);
        let flat = self.meshes.add(Plane3d::default().mesh().size(1.0, 1.0));
        let mut budget = 420i32;
        let rects = self.asphalt.clone();
        let marks = self.mats.mats.contains_key(&Mat::DecalArrow);
        if marks {
            for (top, half, yaw) in rects.iter().copied() {
                // дорога: длинная узкая полоса, локальная Z вдоль
                if half.y < half.x * 2.5 || half.x > 14.0 {
                    continue;
                }
                let rot = Quat::from_rotation_y(yaw);
                let len = half.y * 2.0;
                let mut t = -half.y + 14.0;
                let mut k = 0;
                while t < half.y - 14.0 && budget > 0 {
                    let lane = if k % 2 == 0 { half.x * 0.5 } else { -half.x * 0.5 };
                    let (mat, sx, sz) = if k % 3 == 2 { (Mat::DecalSlow, 2.6, 1.3) } else { (Mat::DecalArrow, 1.4, 4.2) };
                    let flip = if lane > 0.0 { 0.0 } else { std::f32::consts::PI };
                    let p = top + rot * Vec3::new(lane, 0.0, t);
                    let tag = self.tag();
                    self.commands.spawn((
                        Mesh3d(flat.clone()),
                        MeshMaterial3d(self.mats.get(mat)),
                        Transform::from_translation(p + Vec3::Y * 0.013).with_rotation(rot * Quat::from_rotation_y(flip)).with_scale(Vec3::new(sx, 1.0, sz)),
                        bevy::light::NotShadowCaster,
                        tag,
                    ));
                    budget -= 1;
                    t += 38.0 + (k % 3) as f32 * 7.0;
                    k += 1;
                }
                // зебра у концов дороги
                for end in [-1.0f32, 1.0] {
                    if len < 40.0 {
                        break;
                    }
                    let p = top + rot * Vec3::new(0.0, 0.0, end * (half.y - 5.0));
                    let tag = self.tag();
                    self.commands.spawn((
                        Mesh3d(flat.clone()),
                        MeshMaterial3d(self.mats.get(Mat::DecalZebra)),
                        Transform::from_translation(p + Vec3::Y * 0.013).with_rotation(rot * Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)).with_scale(Vec3::new(4.0, 1.0, half.x * 1.9)),
                        bevy::light::NotShadowCaster,
                        tag,
                    ));
                    budget -= 1;
                }
            }
        }
        for (top, half, yaw) in rects {
            let area = half.x * half.y * 4.0;
            let n = ((area / 140.0) as i32).clamp(1, 40);
            for _ in 0..n {
                if budget <= 0 {
                    return;
                }
                budget -= 1;
                let lx = rng.gen_range(-half.x..half.x) * 0.92;
                let lz = rng.gen_range(-half.y..half.y) * 0.95;
                let p = top + Quat::from_rotation_y(yaw) * Vec3::new(lx, 0.0, lz);
                let r: f32 = rng.gen();
                let (mat, size) = if r < 0.42 {
                    (Mat::DecalCrack, rng.gen_range(2.5..6.0))
                } else if r < 0.62 {
                    (Mat::DecalOil, rng.gen_range(1.2..3.0))
                } else if r < 0.94 {
                    (Mat::DecalPuddle, rng.gen_range(1.5..5.0))
                } else {
                    (Mat::DecalManhole, 1.0)
                };
                let tag = self.tag();
                let stretch = rng.gen_range(0.6..1.0);
                let rot = rng.gen_range(0.0..std::f32::consts::TAU);
                self.commands.spawn((
                    Mesh3d(flat.clone()),
                    MeshMaterial3d(self.mats.get(mat)),
                    Transform::from_translation(p + Vec3::Y * 0.012).with_rotation(Quat::from_rotation_y(rot)).with_scale(Vec3::new(size, 1.0, size * stretch)),
                    bevy::light::NotShadowCaster,
                    tag,
                ));
            }
        }
    }

    pub fn finalize(mut self, nav_step: f32) -> MapData {
        self.scatter_road_decals();
        let obstacles = self.data.obstacles.clone();
        {
            let g = &mut self.data.grid;
            for o in &obstacles {
                let (s, c) = o.yaw.sin_cos();
                let r = o.half.length() + g.cell;
                let i0 = (((o.center.x - r) - g.origin.x) / g.cell).floor().max(0.0) as usize;
                let i1 = ((((o.center.x + r) - g.origin.x) / g.cell).ceil().max(0.0) as usize).min(g.nx - 1);
                let j0 = (((o.center.y - r) - g.origin.y) / g.cell).floor().max(0.0) as usize;
                let j1 = ((((o.center.y + r) - g.origin.y) / g.cell).ceil().max(0.0) as usize).min(g.nz - 1);
                for j in j0..=j1 {
                    for i in i0..=i1 {
                        let p = g.pos(i, j) - o.center;
                        let lx = p.x * c - p.y * s;
                        let lz = p.x * s + p.y * c;
                        if lx.abs() <= o.half.x + 0.3 && lz.abs() <= o.half.y + 0.3 {
                            let k = j * g.nx + i;
                            let ground = g.height[k];
                            if o.top > ground + 0.6 && o.bottom < ground + 2.2 {
                                g.blocked[k] = true;
                            }
                        }
                    }
                }
            }
        }
        self.build_nav(nav_step);
        self.data
    }

    fn build_nav(&mut self, step: f32) {
        let half = self.data.half;
        let mut nav = NavGraph::default();
        let nx = (half.x * 2.0 / step) as i32;
        let nz = (half.y * 2.0 / step) as i32;
        let mut index = vec![usize::MAX; ((nx + 1) * (nz + 1)) as usize];
        let clearance = 3.2;
        for j in 0..=nz {
            for i in 0..=nx {
                let x = -half.x + step * 0.5 + i as f32 * step;
                let z = -half.y + step * 0.5 + j as f32 * step;
                if x.abs() > half.x - 4.0 || z.abs() > half.y - 4.0 {
                    continue;
                }
                // Узел на месте препятствия: пытаемся сдвинуть в ближайшую свободную точку
                let mut placed = None;
                'search: for r in [0.0, 1.5, 3.0] {
                    for (dx, dz) in [(0.0, 0.0), (1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                        let px = x + dx * r;
                        let pz = z + dz * r;
                        if self.clear_around(px, pz, clearance) {
                            placed = Some(Vec2::new(px, pz));
                            break 'search;
                        }
                        if r == 0.0 {
                            break;
                        }
                    }
                }
                if let Some(p) = placed {
                    let h = self.data.ground_height(p.x, p.y);
                    index[(j * (nx + 1) + i) as usize] = nav.add_node(Vec3::new(p.x, h, p.y));
                }
            }
        }
        for j in 0..=nz {
            for i in 0..=nx {
                let a = index[(j * (nx + 1) + i) as usize];
                if a == usize::MAX {
                    continue;
                }
                for (di, dj) in [(1, 0), (0, 1), (1, 1), (1, -1)] {
                    let ii = i + di;
                    let jj = j + dj;
                    if ii < 0 || jj < 0 || ii > nx || jj > nz {
                        continue;
                    }
                    let b = index[(jj * (nx + 1) + ii) as usize];
                    if b == usize::MAX {
                        continue;
                    }
                    if self.segment_drivable(nav.nodes[a], nav.nodes[b]) {
                        nav.link(a, b);
                    }
                }
            }
        }
        // Ручные узлы (этажи, мосты): связываем между собой и с ближайшим автоматическим узлом
        let base = nav.nodes.len();
        let manual = std::mem::take(&mut self.manual_nav);
        for (p, _) in &manual {
            nav.add_node(*p);
        }
        for (k, (p, links)) in manual.iter().enumerate() {
            for &l in links {
                nav.link(base + k, base + l);
            }
            // связь с землёй, если узел на уровне земли
            let ground = self.data.ground_height(p.x, p.z);
            if (p.y - ground).abs() < 1.0 {
                let mut best = None;
                let mut bd = f32::MAX;
                for (i, n) in nav.nodes.iter().enumerate().take(base) {
                    let d = n.distance(*p);
                    if d < bd && d < step * 1.6 {
                        bd = d;
                        best = Some(i);
                    }
                }
                if let Some(b) = best {
                    nav.link(base + k, b);
                }
            }
        }
        // Удаляем маленькие оторванные куски: оставляем только крупнейшую компоненту
        let n = nav.nodes.len();
        let mut comp = vec![usize::MAX; n];
        let mut sizes = Vec::new();
        for s in 0..n {
            if comp[s] != usize::MAX || nav.edges[s].is_empty() {
                continue;
            }
            let id = sizes.len();
            let mut stack = vec![s];
            comp[s] = id;
            let mut count = 0;
            while let Some(c) = stack.pop() {
                count += 1;
                for &(nb, _) in &nav.edges[c] {
                    if comp[nb] == usize::MAX {
                        comp[nb] = id;
                        stack.push(nb);
                    }
                }
            }
            sizes.push(count);
        }
        if let Some((big, _)) = sizes.iter().enumerate().max_by_key(|(_, s)| **s) {
            for i in 0..n {
                if comp[i] != big {
                    nav.edges[i].clear();
                }
            }
            for i in 0..n {
                nav.edges[i].retain(|(nb, _)| comp[*nb] == big);
            }
        }
        self.data.nav = nav;
    }

    fn clear_around(&self, x: f32, z: f32, r: f32) -> bool {
        let g = &self.data.grid;
        let steps = (r / g.cell).ceil() as i32;
        let h0 = self.data.ground_height(x, z);
        for dj in -steps..=steps {
            for di in -steps..=steps {
                let px = x + di as f32 * g.cell;
                let pz = z + dj as f32 * g.cell;
                if (Vec2::new(px - x, pz - z)).length() > r {
                    continue;
                }
                match g.idx(px, pz) {
                    Some(k) => {
                        if g.blocked[k] || !g.has_ground[k] {
                            return false;
                        }
                        if (g.height[k] - h0).abs() > 2.0 {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
        }
        true
    }

    fn segment_drivable(&self, a: Vec3, b: Vec3) -> bool {
        let d = Vec2::new(b.x - a.x, b.z - a.z);
        let len = d.length();
        let steps = (len / (self.data.grid.cell * 0.5)).ceil().max(1.0) as i32;
        let perp = Vec2::new(-d.y, d.x).normalize_or_zero() * 1.4;
        let mut prev = a.y;
        for k in 0..=steps {
            let t = k as f32 / steps as f32;
            let c = Vec2::new(a.x, a.z) + d * t;
            for off in [Vec2::ZERO, perp, -perp] {
                let p = c + off;
                if self.data.is_blocked(p.x, p.y) {
                    return false;
                }
            }
            let h = self.data.ground_height(c.x, c.y);
            let slope = (h - prev).abs() / (len / steps as f32).max(0.01);
            if slope > 0.55 {
                return false;
            }
            prev = h;
        }
        true
    }
}

pub fn new_map_data(id: MapId, half: Vec2, theme: Theme) -> MapData {
    MapData {
        id,
        half,
        kill_y: -25.0,
        water: Vec::new(),
        surfaces: Vec::new(),
        grid: Grid::new(half, 1.0),
        obstacles: Vec::new(),
        nav: NavGraph::default(),
        spawns: Vec::new(),
        pickups: Vec::new(),
        checkpoints: Vec::new(),
        ped_zones: Vec::new(),
        theme,
        props_count: 0,
    }
}

/// Строит карту целиком. Возвращает данные (ресурс ставит вызывающий).
#[allow(clippy::too_many_arguments)]
pub fn build_map(
    id: MapId,
    seed: u64,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    std_mats: &mut Assets<StandardMaterial>,
    mats: &MatLib,
    authority: bool,
    visuals: bool,
    kenney: Option<crate::extassets::KenneyLib>,
) -> MapData {
    let theme = builders::theme_for(id);
    let half = builders::half_for(id);
    let mut b = MapBuilder {
        commands,
        meshes,
        mats,
        std_mats,
        rng: StdRng::seed_from_u64(seed),
        data: new_map_data(id, half, theme),
        authority,
        visuals,
        light_budget: 56,
        prop_id: 0,
        manual_nav: Vec::new(),
        asphalt: Vec::new(),
        kenney: if visuals { kenney } else { None },
    };
    match id {
        MapId::Port => port::build(&mut b),
        MapId::Quarry => quarry::build(&mut b),
        MapId::Airfield => airfield::build(&mut b),
        MapId::Downtown => downtown::build(&mut b),
        MapId::Coliseum => coliseum::build(&mut b),
        MapId::Junkyard => junkyard::build(&mut b),
    }
    builders::perimeter(&mut b);
    builders::outskirts(&mut b);
    let data = b.finalize(9.0);
    info!(
        "карта {:?}: препятствий {}, узлов навигации {}, связанных {}, пикапов {}, спавнов {}",
        id,
        data.obstacles.len(),
        data.nav.nodes.len(),
        data.nav.edges.iter().filter(|e| !e.is_empty()).count(),
        data.pickups.len(),
        data.spawns.len()
    );
    data
}

// ============================================================ движение опасностей

pub struct MapsPlugin;

impl Plugin for MapsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, drive_movers.run_if(in_state(AppState::Race)))
            .add_systems(Update, animate_checkpoints.run_if(in_state(AppState::Race)));
    }
}

fn drive_movers(
    time: Res<Time>,
    ms: Option<Res<MatchState>>,
    mut q: Query<(&Mover, &mut Position, &mut Rotation, &mut LinearVelocity, &mut AngularVelocity, &mut Transform)>,
    mode: Res<NetMode>,
) {
    let dt = time.delta_secs().max(1e-4);
    let t = ms.map(|m| m.elapsed).unwrap_or(0.0);
    for (m, mut pos, mut rot, mut lv, mut av, mut tf) in q.iter_mut() {
        let (p1, r1) = m.path.eval(t + dt);
        if *mode == NetMode::Client {
            let (p0, r0) = m.path.eval(t);
            tf.translation = p0;
            tf.rotation = r0;
            pos.0 = p0;
            rot.0 = r0;
            continue;
        }
        let delta = p1 - pos.0;
        if delta.length() > 20.0 {
            // телепорт (поезд уехал за карту)
            pos.0 = p1;
            rot.0 = r1;
            lv.0 = Vec3::ZERO;
            av.0 = Vec3::ZERO;
            continue;
        }
        lv.0 = delta / dt;
        let dq = r1 * rot.0.inverse();
        let (axis, angle) = dq.to_axis_angle();
        let angle = if angle > std::f32::consts::PI { angle - std::f32::consts::TAU } else { angle };
        av.0 = if axis.is_finite() { axis * angle / dt } else { Vec3::ZERO };
    }
}

fn animate_checkpoints(time: Res<Time>, mut q: Query<&mut Transform, With<CheckpointMarker>>) {
    for mut t in q.iter_mut() {
        t.rotate_y(time.delta_secs() * 0.8);
    }
}

#[allow(dead_code)]
pub fn rng_for(seed: u64, salt: u64) -> StdRng {
    StdRng::seed_from_u64(seed.wrapping_mul(6364136223846793005).wrapping_add(salt))
}
