//! Процедурные модели машин (12 силуэтов), оружие на точках крепления, вмятины по зонам урона.

use crate::config::CarConfig;
use crate::data::*;
use crate::meshgen::*;
use crate::vehicle::*;
use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;

#[derive(Component)]
pub struct CarVisual;

#[derive(Component)]
pub struct CarPaint {
    pub handle: Handle<StandardMaterial>,
    pub base: Color,
    /// краска с картами износа (модели Kenney): шероховатость задаёт текстура, урон её множит
    pub textured: bool,
}

#[derive(Component)]
pub struct BodyDeform {
    pub mesh: Handle<Mesh>,
    pub base: Vec<Vec3>,
    pub half: Vec3,
    pub applied: [f32; 4],
    /// исходные (сглаженные) нормали; пусто — после вмятин плоские нормали
    pub base_n: Vec<Vec3>,
}

#[derive(Component)]
pub struct TurretVisual;

#[derive(Component)]
pub struct DrumSpin;

#[derive(Component)]
pub struct BrakeLight;

#[derive(Component)]
pub struct HeadLight;

#[derive(Component)]
pub struct ExhaustPoint;

/// Параметры формы кузова.
struct Shape {
    hood: f32,
    trunk: f32,
    nose_taper: f32,
    top_taper: f32,
    cabin_front: f32,
    cabin_back: f32,
    cabin_h: f32,
    cabin_taper: f32,
    has_cabin: bool,
}

fn shape_for(style: BodyStyle) -> Shape {
    let base = Shape {
        hood: 0.3,
        trunk: 0.25,
        nose_taper: 0.06,
        top_taper: 0.07,
        cabin_front: -0.35,
        cabin_back: 0.55,
        cabin_h: 0.55,
        cabin_taper: 0.16,
        has_cabin: true,
    };
    match style {
        BodyStyle::Hatch => base,
        BodyStyle::GolfCart => Shape { hood: 0.1, trunk: 0.0, cabin_h: 0.0, has_cabin: false, ..base },
        BodyStyle::Hearse => Shape { hood: 0.3, trunk: 0.0, cabin_front: -0.3, cabin_back: 0.95, cabin_h: 0.62, cabin_taper: 0.08, ..base },
        BodyStyle::Pickup => Shape { hood: 0.25, trunk: 0.0, cabin_front: -0.28, cabin_back: 0.12, cabin_h: 0.62, cabin_taper: 0.1, ..base },
        BodyStyle::IceCream => Shape { hood: 0.0, trunk: 0.0, nose_taper: 0.04, top_taper: 0.03, has_cabin: false, ..base },
        BodyStyle::Buggy => Shape { hood: 0.0, trunk: 0.0, has_cabin: false, ..base },
        BodyStyle::Coupe => Shape { hood: 0.35, trunk: 0.15, cabin_front: -0.12, cabin_back: 0.6, cabin_h: 0.42, cabin_taper: 0.28, ..base },
        BodyStyle::Limo => Shape { hood: 0.3, trunk: 0.25, cabin_front: -0.42, cabin_back: 0.68, cabin_h: 0.5, cabin_taper: 0.06, ..base },
        BodyStyle::ArmoredVan => Shape { hood: 0.25, trunk: 0.0, nose_taper: 0.03, top_taper: 0.12, has_cabin: false, ..base },
        BodyStyle::Monster => Shape { hood: 0.2, trunk: 0.0, cabin_front: -0.3, cabin_back: 0.1, cabin_h: 0.65, cabin_taper: 0.1, ..base },
        BodyStyle::Mixer => Shape { hood: 0.0, trunk: 0.0, has_cabin: false, ..base },
        BodyStyle::Bus => Shape { hood: 0.0, trunk: 0.0, nose_taper: 0.02, top_taper: 0.03, has_cabin: false, ..base },
    }
}

/// Кузов: подразделённая коробка, сжатая к капоту и багажнику.
fn shaped_body(size: Vec3, sh: &Shape) -> (Mesh, Vec<Vec3>) {
    let (nx, ny, nz) = (5u32, 3u32, 14u32);
    let half = size * 0.5;
    let mut p: Vec<Vec3> = Vec::new();
    let mut uv: Vec<[f32; 2]> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();
    // грани: (нормаль, u, v, nu, nv)
    let faces = [
        (Vec3::X, Vec3::NEG_Z, Vec3::Y, nz, ny),
        (Vec3::NEG_X, Vec3::Z, Vec3::Y, nz, ny),
        (Vec3::Y, Vec3::X, Vec3::NEG_Z, nx, nz),
        (Vec3::NEG_Y, Vec3::X, Vec3::Z, nx, nz),
        (Vec3::Z, Vec3::X, Vec3::Y, nx, ny),
        (Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y, nx, ny),
    ];
    for (n, u, v, nu, nv) in faces {
        let base = p.len() as u32;
        let c = n * (half * n.abs()).length();
        let hu = u * (half * u.abs()).length();
        let hv = v * (half * v.abs()).length();
        for j in 0..=nv {
            for i in 0..=nu {
                let fu = i as f32 / nu as f32 * 2.0 - 1.0;
                let fv = j as f32 / nv as f32 * 2.0 - 1.0;
                p.push(c + hu * fu + hv * fv);
                // UV в метрах (1.6 м на повтор), чтобы слой износа не растягивался
                uv.push([i as f32 / nu as f32 * hu.length() * 2.0 / 1.6, j as f32 / nv as f32 * hv.length() * 2.0 / 1.6]);
            }
        }
        for j in 0..nv {
            for i in 0..nu {
                let a = base + j * (nu + 1) + i;
                let b = a + 1;
                let cc = a + nu + 1;
                let d = cc + 1;
                idx.extend_from_slice(&[a, b, d, a, d, cc]);
            }
        }
    }
    for q in p.iter_mut() {
        let t = q.z / half.z; // -1 перед, +1 зад
        let top = (q.y / half.y + 1.0) * 0.5; // 0 низ, 1 верх
        let mut drop = 0.0;
        if t < -0.45 {
            drop = sh.hood * smoothstep_f(-0.45, -1.0, t);
        } else if t > 0.55 {
            drop = sh.trunk * smoothstep_f(0.55, 1.0, t);
        }
        q.y -= drop * size.y * top;
        q.x *= 1.0 - sh.top_taper * top;
        if t.abs() > 0.9 {
            q.x *= 1.0 - sh.nose_taper * (t.abs() - 0.9) * 10.0;
        }
    }
    let mesh = finish_mesh(&p, &uv, &idx);
    (mesh, p)
}

fn smoothstep_f(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn compute_normals(p: &[Vec3], idx: &[u32]) -> Vec<[f32; 3]> {
    let mut n = vec![Vec3::ZERO; p.len()];
    for t in idx.chunks(3) {
        let a = p[t[0] as usize];
        let b = p[t[1] as usize];
        let c = p[t[2] as usize];
        let f = (b - a).cross(c - a);
        for &k in t {
            n[k as usize] += f;
        }
    }
    n.iter().map(|v| v.normalize_or_zero().to_array()).collect()
}

fn finish_mesh(p: &[Vec3], uv: &[[f32; 2]], idx: &[u32]) -> Mesh {
    let normals = compute_normals(p, idx);
    mesh_from(p.iter().map(|v| v.to_array()).collect(), normals, uv.to_vec(), idx.to_vec())
}

/// Кабина: коробка, у которой верх сужается и сдвигается.
fn cabin_mesh(size: Vec3, taper: f32, slant_front: f32, slant_back: f32) -> Mesh {
    let mut m = box_mesh(size, 1.0);
    if let Some(VertexAttributeValues::Float32x3(pos)) = m.attribute_mut(Mesh::ATTRIBUTE_POSITION) {
        for q in pos.iter_mut() {
            if q[1] > 0.0 {
                q[0] *= 1.0 - taper;
                if q[2] < 0.0 {
                    q[2] += slant_front * size.z;
                } else {
                    q[2] -= slant_back * size.z;
                }
            }
        }
    }
    let p: Vec<Vec3> = match m.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(VertexAttributeValues::Float32x3(v)) => v.iter().map(|a| Vec3::from(*a)).collect(),
        _ => Vec::new(),
    };
    let idx: Vec<u32> = match m.indices() {
        Some(bevy::render::mesh::Indices::U32(i)) => i.clone(),
        _ => Vec::new(),
    };
    let n = compute_normals(&p, &idx);
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, n);
    m
}

pub struct Mounts {
    pub front: Vec3,
    pub roof: Vec3,
    pub rear: Vec3,
    pub side: Vec3,
}

/// Высота оси колёс над началом координат машины при штатном клиренсе.
pub fn wheel_axle_y(def: &CarDef) -> f32 {
    crate::config::wheel_mount_y(def) - def.travel * 0.58
}

/// Подъём кузова модели над осью: у «Мамонта» колёса вдвое больше арок.
fn kit_lift(def: &CarDef, fit: &crate::carfit::CarFit) -> f32 {
    if def.wheel_radius > fit.wheel_r[0] * 1.4 {
        def.wheel_radius * 0.75
    } else {
        0.0
    }
}

pub fn mount_points(def: &CarDef) -> Mounts {
    if let Some(f) = crate::carfit::fit(def.id) {
        let y0 = wheel_axle_y(def) + kit_lift(def, f);
        // у миксера посередине барабан: оружие на крыше кабины
        let roof_z = if def.style == BodyStyle::Mixer { f.min.z + 1.0 } else { 0.0 };
        return Mounts {
            front: Vec3::new(0.0, y0 + 0.3, f.min.z - 0.12),
            roof: Vec3::new(0.0, y0 + f.roof_y + 0.02, roof_z),
            rear: Vec3::new(0.0, y0 + 0.35, f.max.z + 0.1),
            side: Vec3::new(f.max.x + 0.08, y0 + 0.45, 0.0),
        };
    }
    let s = def.size;
    let sh = shape_for(def.style);
    let body_top = 0.1 + s.y * 0.5;
    let roof_y = if sh.has_cabin { body_top + sh.cabin_h * s.y + 0.15 } else { body_top };
    let roof_z = if sh.has_cabin { (sh.cabin_front + sh.cabin_back) * 0.5 * s.z * 0.5 } else { 0.0 };
    let roof_y = match def.style {
        BodyStyle::GolfCart => body_top + 1.25,
        BodyStyle::Buggy => body_top + 1.0,
        BodyStyle::Mixer => body_top + 0.9,
        _ => roof_y,
    };
    let roof_z = match def.style {
        BodyStyle::Mixer => -s.z * 0.34,
        BodyStyle::IceCream => s.z * 0.12,
        _ => roof_z,
    };
    Mounts {
        front: Vec3::new(0.0, 0.1 + s.y * 0.05 - sh.hood * s.y * 0.3, -s.z * 0.5 - 0.15),
        roof: Vec3::new(0.0, roof_y, roof_z),
        rear: Vec3::new(0.0, 0.1 + s.y * 0.1, s.z * 0.5 + 0.12),
        side: Vec3::new(s.x * 0.5 + 0.1, 0.1 + s.y * 0.15, 0.0),
    }
}

pub struct ModelAssets<'a> {
    pub meshes: &'a mut Assets<Mesh>,
    pub materials: &'a mut Assets<StandardMaterial>,
    /// Текстуры износа и трещин стекла (нет в безоконном режиме)
    pub wear: Option<WearAssets>,
    /// Ночью у машин настоящие фары
    pub headlights: bool,
}

/// Общие текстуры повреждений.
#[derive(Resource, Clone)]
pub struct WearAssets {
    pub wear: Handle<Image>,
    pub crack: Handle<Image>,
    /// Спрайтовые стадии износа damage_stage1..4.png (если пакет ассетов на месте)
    pub stages: Option<[Handle<Image>; 4]>,
    /// Наклейки скинов уровня 2: «Хорёк», «Пломбир»
    pub decals: [Option<Handle<Image>>; 2],
    /// Палитра Kenney и серая карта для перекраски (модели машин assets/cars)
    pub kit: Option<(Handle<Image>, Handle<Image>)>,
    /// Отдельные наклейки повреждений 1..4 (царапины, разрыв, дыры, разбитый лист)
    pub marks: Option<[Handle<Image>; 4]>,
    /// Карты кузовов (assets/cars/tex, tools/cars/car_textures.py)
    pub car_tex: Option<CarTex>,
}

#[derive(Clone)]
pub struct CarTex {
    pub paint: [Handle<Image>; 3],
    pub armor: [Handle<Image>; 3],
    pub palette_mr: Handle<Image>,
}

/// Слой износа поверх кузова: прозрачность растёт по стадиям урона.
#[derive(Component)]
pub struct WearShell(pub Handle<StandardMaterial>);

/// Стекло, на котором появляются трещины.
#[derive(Component)]
pub struct GlassPanel(pub Handle<StandardMaterial>);

/// Деталь, которая отрывается при уроне зоны выше порога.
#[derive(Component)]
pub struct Detachable {
    pub zone: usize,
    pub threshold: f32,
    pub size: Vec3,
}

/// Оторванная деталь, летящая по физике.
#[derive(Component)]
pub struct Debris(pub f32);

/// Фара-прожектор машины.
#[derive(Component)]
pub struct CarHeadlight;

fn mat(a: &mut ModelAssets, color: Color, metal: f32, rough: f32) -> Handle<StandardMaterial> {
    a.materials.add(StandardMaterial { base_color: color, metallic: metal, perceptual_roughness: rough, ..default() })
}

/// Касательные для карт нормалей; при ошибке меш без них (карта нормалей тогда игнорируется).
fn with_tangents(m: Mesh) -> Mesh {
    match m.clone().with_generated_tangents() {
        Ok(t) => t,
        Err(e) => {
            warn!("касательные меша: {e}");
            m
        }
    }
}

fn emissive(a: &mut ModelAssets, color: LinearRgba) -> Handle<StandardMaterial> {
    a.materials.add(StandardMaterial { base_color: Color::BLACK, emissive: color, ..default() })
}

/// Строит визуальную модель машины дочерними сущностями.
pub fn build_car_model(parent: &mut ChildSpawnerCommands, a: &mut ModelAssets, cfg: &CarConfig) {
    let def = cfg.def();
    if let (Some(fit), Some(kit)) = (crate::carfit::fit(def.id), a.wear.as_ref().and_then(|w| w.kit.clone())) {
        if let Some(model) = crate::carkit::model(def.id) {
            build_kit_model(parent, a, cfg, fit, &model, kit);
            return;
        }
    }
    let s = def.size;
    let sh = shape_for(def.style);
    // скин уровня 1: цвет, металл и шероховатость по палитре
    let skin = cfg.skin_def();
    let col = skin.map(|k| k.color).unwrap_or(cfg.color);
    let paint_color = Color::srgb(col[0], col[1], col[2]);
    let paint = a.materials.add(StandardMaterial {
        base_color: paint_color,
        metallic: skin.map(|k| k.metallic).unwrap_or(0.5),
        perceptual_roughness: skin.map(|k| k.roughness).unwrap_or(0.32),
        clearcoat: if skin.map(|k| k.roughness > 0.6).unwrap_or(false) { 0.0 } else { 0.7 },
        clearcoat_perceptual_roughness: 0.15,
        ..default()
    });
    let glass = a.materials.add(StandardMaterial {
        base_color: Color::srgb(0.04, 0.06, 0.08),
        metallic: 0.6,
        perceptual_roughness: 0.06,
        reflectance: 0.9,
        ..default()
    });
    let dark = mat(a, Color::srgb(0.06, 0.06, 0.07), 0.2, 0.8);
    let chrome = mat(a, Color::srgb(0.85, 0.86, 0.88), 1.0, 0.18);
    let tire = mat(a, Color::srgb(0.04, 0.04, 0.04), 0.0, 0.95);
    let rim = mat(a, Color::srgb(0.7, 0.7, 0.72), 0.9, 0.3);
    let head = emissive(a, LinearRgba::rgb(6.0, 5.6, 4.6));
    let brake = a.materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.0, 0.0),
        emissive: LinearRgba::rgb(0.6, 0.02, 0.01),
        ..default()
    });

    parent.spawn((CarPaint { handle: paint.clone(), base: paint_color, textured: false }, Transform::default(), Visibility::default()));

    let body_y = 0.1;
    // Кузов (кроме багги — у неё рама)
    if def.style != BodyStyle::Buggy {
        let body_size = match def.style {
            BodyStyle::GolfCart => Vec3::new(s.x, s.y * 0.7, s.z),
            BodyStyle::Mixer => Vec3::new(s.x * 0.92, s.y * 0.5, s.z),
            _ => s,
        };
        let (mesh, base) = shaped_body(body_size, &sh);
        let handle = a.meshes.add(mesh);
        let y = if def.style == BodyStyle::Mixer { body_y - s.y * 0.25 } else { body_y };
        parent.spawn((
            Mesh3d(handle.clone()),
            MeshMaterial3d(paint.clone()),
            Transform::from_xyz(0.0, y, 0.0),
            BodyDeform { mesh: handle.clone(), base, half: body_size * 0.5, applied: [0.0; 4], base_n: Vec::new() },
        ));
        if let Some(w) = a.wear.clone() {
            let sprite = w.stages.as_ref().map(|s| s[0].clone());
            let wm = a.materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 1.0, 1.0, if sprite.is_some() { 1.0 } else { 0.0 }),
                base_color_texture: Some(sprite.unwrap_or(w.wear)),
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.85,
                depth_bias: 4.0,
                ..default()
            });
            parent.spawn((
                Mesh3d(handle),
                MeshMaterial3d(wm.clone()),
                Transform::from_xyz(0.0, y, 0.0).with_scale(Vec3::splat(1.004)),
                WearShell(wm),
                bevy::light::NotShadowCaster,
            ));
        }
    }
    let body_top = body_y + s.y * 0.5;
    // скин уровня 2: наклейки-панели на двери и капот
    if let (Some(k), Some(w)) = (skin, a.wear.clone()) {
        if k.decal > 0 && def.style != BodyStyle::Buggy {
            if let Some(tex) = w.decals[(k.decal - 1) as usize].clone() {
                let dm = a.materials.add(StandardMaterial {
                    base_color_texture: Some(tex),
                    alpha_mode: AlphaMode::Blend,
                    perceptual_roughness: k.roughness,
                    metallic: 0.2,
                    depth_bias: 6.0,
                    cull_mode: None,
                    ..default()
                });
                let door = a.meshes.add(Rectangle::new(s.z * 0.46, s.y * 0.52));
                let hood = a.meshes.add(Rectangle::new(s.x * 0.62, s.z * 0.24));
                let mid_y = body_y + s.y * 0.02;
                for side in [-1.0f32, 1.0] {
                    parent.spawn((
                        Mesh3d(door.clone()),
                        MeshMaterial3d(dm.clone()),
                        Transform::from_xyz(side * (s.x * 0.5 + 0.015), mid_y, 0.0).with_rotation(Quat::from_rotation_y(side * std::f32::consts::FRAC_PI_2)),
                        bevy::light::NotShadowCaster,
                    ));
                }
                parent.spawn((
                    Mesh3d(hood),
                    MeshMaterial3d(dm),
                    Transform::from_xyz(0.0, body_top + 0.015, -s.z * 0.32).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
                    bevy::light::NotShadowCaster,
                ));
            }
        }
    }

    // Кабина
    if sh.has_cabin {
        let ch = sh.cabin_h * s.y + 0.15;
        let clen = (sh.cabin_back - sh.cabin_front) * s.z * 0.5;
        let cz = (sh.cabin_front + sh.cabin_back) * 0.5 * s.z * 0.5;
        let cy = body_top + ch * 0.5 - 0.03;
        let slant_f = if def.style == BodyStyle::Coupe { 0.3 } else { 0.18 };
        let slant_b = match def.style {
            BodyStyle::Coupe => 0.35,
            BodyStyle::Hearse | BodyStyle::Pickup | BodyStyle::Monster => 0.03,
            _ => 0.12,
        };
        let cab = cabin_mesh(Vec3::new(s.x * 0.86, ch, clen), sh.cabin_taper, slant_f, slant_b);
        parent.spawn((Mesh3d(a.meshes.add(cab)), MeshMaterial3d(paint.clone()), Transform::from_xyz(0.0, cy, cz)));
        let win = cabin_mesh(Vec3::new(s.x * 0.88, ch * 0.62, clen * 0.94), sh.cabin_taper * 1.1, slant_f * 0.9, slant_b * 0.9);
        parent.spawn((Mesh3d(a.meshes.add(win)), MeshMaterial3d(glass.clone()), Transform::from_xyz(0.0, cy + ch * 0.08, cz), GlassPanel(glass.clone())));
    }

    // Особенности силуэта
    let cube = |a: &mut ModelAssets, size: Vec3| a.meshes.add(box_mesh(size, 1.0));
    match def.style {
        BodyStyle::Hatch => {
            parent.spawn((Mesh3d(cube(a, Vec3::new(s.x * 0.8, 0.06, 0.4))), MeshMaterial3d(dark.clone()),
                Transform::from_xyz(0.0, body_top + sh.cabin_h * s.y + 0.18, s.z * 0.27)));
        }
        BodyStyle::GolfCart => {
            let roof_y = body_top + 1.2;
            for (x, z) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                parent.spawn((Mesh3d(a.meshes.add(Cylinder::new(0.04, 1.25).mesh().resolution(8))), MeshMaterial3d(chrome.clone()),
                    Transform::from_xyz(x * s.x * 0.42, body_top + 0.6, z * s.z * 0.3)));
            }
            parent.spawn((Mesh3d(cube(a, Vec3::new(s.x * 1.02, 0.08, s.z * 0.75))), MeshMaterial3d(paint.clone()), Transform::from_xyz(0.0, roof_y, 0.0)));
            parent.spawn((Mesh3d(cube(a, Vec3::new(s.x * 0.8, 0.5, 0.4))), MeshMaterial3d(dark.clone()), Transform::from_xyz(0.0, body_top + 0.25, 0.15)));
            for k in 0..3 {
                parent.spawn((Mesh3d(a.meshes.add(Cylinder::new(0.03, 1.0).mesh().resolution(6))), MeshMaterial3d(chrome.clone()),
                    Transform::from_xyz(-0.15 + k as f32 * 0.15, body_top + 0.5, s.z * 0.45).with_rotation(Quat::from_rotation_x(0.2))));
            }
        }
        BodyStyle::Hearse => {
            for side in [-1.0f32, 1.0] {
                parent.spawn((Mesh3d(cube(a, Vec3::new(0.04, 0.3, s.z * 0.3))), MeshMaterial3d(chrome.clone()),
                    Transform::from_xyz(side * s.x * 0.44, body_top + 0.45, s.z * 0.28)));
            }
            parent.spawn((Mesh3d(a.meshes.add(Torus::new(0.12, 0.35).mesh())), MeshMaterial3d(mat(a, Color::srgb(0.1, 0.4, 0.15), 0.0, 0.9)),
                Transform::from_xyz(0.0, body_top + sh.cabin_h * s.y + 0.22, s.z * 0.15)));
        }
        BodyStyle::Pickup => {
            let bed_len = s.z * 0.4;
            for side in [-1.0f32, 1.0] {
                parent.spawn((Mesh3d(cube(a, Vec3::new(0.08, 0.45, bed_len))), MeshMaterial3d(paint.clone()),
                    Transform::from_xyz(side * s.x * 0.46, body_top + 0.2, s.z * 0.3)));
            }
            parent.spawn((Mesh3d(cube(a, Vec3::new(s.x * 0.9, 0.45, 0.08))), MeshMaterial3d(paint.clone()), Transform::from_xyz(0.0, body_top + 0.2, s.z * 0.49)));
            // дуга
            for side in [-1.0f32, 1.0] {
                parent.spawn((Mesh3d(a.meshes.add(Cylinder::new(0.05, 0.9).mesh().resolution(8))), MeshMaterial3d(dark.clone()),
                    Transform::from_xyz(side * s.x * 0.42, body_top + 0.45, s.z * 0.12)));
            }
            parent.spawn((Mesh3d(cube(a, Vec3::new(s.x * 0.9, 0.1, 0.1))), MeshMaterial3d(dark.clone()), Transform::from_xyz(0.0, body_top + 0.9, s.z * 0.12)));
        }
        BodyStyle::IceCream => {
            // окно выдачи, полосы, рожок на крыше
            parent.spawn((Mesh3d(cube(a, Vec3::new(0.05, s.y * 0.35, s.z * 0.3))), MeshMaterial3d(glass.clone()), Transform::from_xyz(s.x * 0.48, body_y + s.y * 0.15, s.z * 0.05)));
            for k in 0..3 {
                let c = [Color::srgb(0.9, 0.3, 0.5), Color::srgb(0.4, 0.8, 0.9), Color::srgb(0.95, 0.85, 0.3)][k];
                parent.spawn((Mesh3d(cube(a, Vec3::new(s.x * 1.01, 0.08, s.z * 0.98))), MeshMaterial3d(mat(a, c, 0.0, 0.6)),
                    Transform::from_xyz(0.0, body_y - s.y * 0.3 + k as f32 * 0.12, 0.0)));
            }
            parent.spawn((Mesh3d(cube(a, Vec3::new(s.x * 0.85, 0.08, 0.2))), MeshMaterial3d(glass.clone()), Transform::from_xyz(0.0, body_y + s.y * 0.25, -s.z * 0.5 - 0.01)));
            parent.spawn((Mesh3d(a.meshes.add(Cone { radius: 0.35, height: 1.0 }.mesh().resolution(12))),
                MeshMaterial3d(mat(a, Color::srgb(0.85, 0.6, 0.3), 0.0, 0.8)),
                Transform::from_xyz(0.0, body_top + 0.5, -s.z * 0.25).with_rotation(Quat::from_rotation_x(std::f32::consts::PI))));
            parent.spawn((Mesh3d(a.meshes.add(Sphere::new(0.42).mesh().ico(2).unwrap())), MeshMaterial3d(mat(a, Color::srgb(1.0, 0.75, 0.85), 0.0, 0.6)),
                Transform::from_xyz(0.0, body_top + 1.1, -s.z * 0.25)));
        }
        BodyStyle::Buggy => {
            parent.spawn((Mesh3d(cube(a, Vec3::new(s.x * 0.7, 0.12, s.z * 0.9))), MeshMaterial3d(dark.clone()), Transform::from_xyz(0.0, body_y - 0.1, 0.0)));
            let cage_y = body_top + 0.55;
            let tube = a.meshes.add(Cylinder::new(0.045, 1.0).mesh().resolution(8));
            let tube_mat = paint.clone();
            let bar = |p: &mut ChildSpawnerCommands, from: Vec3, to: Vec3| {
                let d = to - from;
                let rot = Quat::from_rotation_arc(Vec3::Y, d.normalize());
                p.spawn((Mesh3d(tube.clone()), MeshMaterial3d(tube_mat.clone()),
                    Transform::from_translation((from + to) * 0.5).with_rotation(rot).with_scale(Vec3::new(1.0, d.length(), 1.0))));
            };
            let w = s.x * 0.4;
            let fz = -s.z * 0.15;
            let rz = s.z * 0.3;
            for side in [-1.0f32, 1.0] {
                bar(parent, Vec3::new(side * w, body_y, -s.z * 0.45), Vec3::new(side * w * 0.8, cage_y, fz));
                bar(parent, Vec3::new(side * w * 0.8, cage_y, fz), Vec3::new(side * w * 0.8, cage_y, rz));
                bar(parent, Vec3::new(side * w * 0.8, cage_y, rz), Vec3::new(side * w, body_y, s.z * 0.45));
                bar(parent, Vec3::new(side * w, body_y, fz), Vec3::new(side * w * 0.8, cage_y, fz));
            }
            bar(parent, Vec3::new(-w * 0.8, cage_y, fz), Vec3::new(w * 0.8, cage_y, fz));
            bar(parent, Vec3::new(-w * 0.8, cage_y, rz), Vec3::new(w * 0.8, cage_y, rz));
            parent.spawn((Mesh3d(cube(a, Vec3::new(0.5, 0.6, 0.5))), MeshMaterial3d(dark.clone()), Transform::from_xyz(0.0, body_y + 0.25, 0.1)));
            parent.spawn((Mesh3d(cube(a, Vec3::new(0.9, 0.5, 0.7))), MeshMaterial3d(mat(a, Color::srgb(0.3, 0.3, 0.32), 0.7, 0.4)), Transform::from_xyz(0.0, body_y + 0.2, s.z * 0.38)));
        }
        BodyStyle::Coupe => {
            parent.spawn((Mesh3d(cube(a, Vec3::new(s.x * 0.95, 0.06, 0.45))), MeshMaterial3d(dark.clone()), Transform::from_xyz(0.0, body_top + 0.32, s.z * 0.46)));
            for side in [-1.0f32, 1.0] {
                parent.spawn((Mesh3d(cube(a, Vec3::new(0.06, 0.32, 0.1))), MeshMaterial3d(dark.clone()), Transform::from_xyz(side * s.x * 0.38, body_top + 0.16, s.z * 0.46)));
            }
            parent.spawn((Mesh3d(cube(a, Vec3::new(s.x * 0.3, 0.05, s.z * 0.5))), MeshMaterial3d(mat(a, Color::WHITE, 0.3, 0.4)),
                Transform::from_xyz(0.0, body_top - 0.02, -s.z * 0.18)));
        }
        BodyStyle::Limo => {
            for side in [-1.0f32, 1.0] {
                parent.spawn((Mesh3d(cube(a, Vec3::new(0.03, 0.05, s.z * 0.9))), MeshMaterial3d(chrome.clone()), Transform::from_xyz(side * s.x * 0.5, body_y, 0.0)));
            }
            for side in [-1.0f32, 1.0] {
                parent.spawn((Mesh3d(cube(a, Vec3::new(0.08, 0.08, 0.08))), MeshMaterial3d(mat(a, Color::srgb(0.9, 0.1, 0.1), 0.0, 0.6)),
                    Transform::from_xyz(side * s.x * 0.4, body_top + 0.05, -s.z * 0.48)));
            }
        }
        BodyStyle::ArmoredVan => {
            for k in 0..3 {
                parent.spawn((Mesh3d(cube(a, Vec3::new(s.x * 1.01, 0.05, 0.25))), MeshMaterial3d(dark.clone()),
                    Transform::from_xyz(0.0, body_y + s.y * 0.3, -s.z * 0.2 + k as f32 * s.z * 0.2)));
            }
            parent.spawn((Mesh3d(cube(a, Vec3::new(s.x * 0.8, 0.12, 0.05))), MeshMaterial3d(glass.clone()), Transform::from_xyz(0.0, body_y + s.y * 0.32, -s.z * 0.5 - 0.01)));
            parent.spawn((Mesh3d(cube(a, Vec3::new(s.x * 1.04, 0.4, 0.25))), MeshMaterial3d(dark.clone()), Transform::from_xyz(0.0, body_y - s.y * 0.28, -s.z * 0.5 - 0.12)));
        }
        BodyStyle::Monster => {
            for (x, z) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                parent.spawn((Mesh3d(a.meshes.add(Cylinder::new(0.08, 0.9).mesh().resolution(8))), MeshMaterial3d(chrome.clone()),
                    Transform::from_xyz(x * s.x * 0.42, body_y - 0.5, z * s.z * 0.3)));
            }
            parent.spawn((Mesh3d(cube(a, Vec3::new(s.x * 0.9, 0.12, 0.3))), MeshMaterial3d(dark.clone()), Transform::from_xyz(0.0, body_top + sh.cabin_h * s.y + 0.22, -s.z * 0.05)));
        }
        BodyStyle::Mixer => {
            // кабина спереди, рама, вращающийся барабан
            let cab_len = s.z * 0.24;
            parent.spawn((Mesh3d(cube(a, Vec3::new(s.x, s.y * 1.05, cab_len))), MeshMaterial3d(paint.clone()),
                Transform::from_xyz(0.0, body_y + s.y * 0.28, -s.z * 0.5 + cab_len * 0.5)));
            parent.spawn((Mesh3d(cube(a, Vec3::new(s.x * 0.9, s.y * 0.35, 0.05))), MeshMaterial3d(glass.clone()),
                Transform::from_xyz(0.0, body_y + s.y * 0.55, -s.z * 0.5 - 0.01)));
            let drum = a.meshes.add(Cylinder::new(s.x * 0.48, s.z * 0.55).mesh().resolution(18));
            let drum_mat = mat(a, Color::srgb(0.85, 0.85, 0.82), 0.3, 0.5);
            parent
                .spawn((Transform::from_xyz(0.0, body_top + s.y * 0.35, s.z * 0.14).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2 - 0.18)), Visibility::default()))
                .with_children(|d| {
                    d.spawn((Mesh3d(drum), MeshMaterial3d(drum_mat), Transform::default(), DrumSpin));
                    d.spawn((Mesh3d(a.meshes.add(box_mesh(Vec3::new(0.15, s.z * 0.56, s.x * 0.97), 1.0))), MeshMaterial3d(paint.clone()), Transform::default(), DrumSpin));
                });
        }
        BodyStyle::Bus => {
            for side in [-1.0f32, 1.0] {
                parent.spawn((Mesh3d(cube(a, Vec3::new(0.04, s.y * 0.28, s.z * 0.82))), MeshMaterial3d(glass.clone()),
                    Transform::from_xyz(side * s.x * 0.5, body_y + s.y * 0.22, 0.05)));
                parent.spawn((Mesh3d(cube(a, Vec3::new(0.045, 0.08, s.z * 0.98))), MeshMaterial3d(dark.clone()),
                    Transform::from_xyz(side * s.x * 0.5, body_y - s.y * 0.05, 0.0)));
            }
            parent.spawn((Mesh3d(cube(a, Vec3::new(s.x * 0.85, s.y * 0.3, 0.05))), MeshMaterial3d(glass.clone()), Transform::from_xyz(0.0, body_y + s.y * 0.22, -s.z * 0.5 - 0.01)));
            parent.spawn((Mesh3d(a.meshes.add(Cylinder::new(0.22, 0.04).mesh().resolution(8))), MeshMaterial3d(mat(a, Color::srgb(0.85, 0.05, 0.05), 0.0, 0.6)),
                Transform::from_xyz(-s.x * 0.5 - 0.25, body_y + s.y * 0.1, -s.z * 0.35).with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))));
        }
    }

    // Бамперы и двери: отрываются при сильном уроне своей зоны
    let cube = |a: &mut ModelAssets, size: Vec3| a.meshes.add(box_mesh(size, 1.0));
    if !matches!(def.style, BodyStyle::Buggy | BodyStyle::GolfCart) {
        let bsize = Vec3::new(s.x * 1.02, 0.22, 0.22);
        let by = body_y - s.y * 0.32;
        let bm = cube(a, bsize);
        parent.spawn((Mesh3d(bm.clone()), MeshMaterial3d(dark.clone()), Transform::from_xyz(0.0, by, -s.z * 0.5 - 0.06), Detachable { zone: 0, threshold: 32.0, size: bsize }));
        parent.spawn((Mesh3d(bm), MeshMaterial3d(dark.clone()), Transform::from_xyz(0.0, by, s.z * 0.5 + 0.06), Detachable { zone: 1, threshold: 32.0, size: bsize }));
    }
    if sh.has_cabin {
        let dsize = Vec3::new(0.06, s.y * 0.62, s.z * 0.26);
        let dm = cube(a, dsize);
        for (side, zone) in [(-1.0f32, 2usize), (1.0, 3)] {
            parent.spawn((
                Mesh3d(dm.clone()),
                MeshMaterial3d(paint.clone()),
                Transform::from_xyz(side * (s.x * 0.5 * (1.0 - sh.top_taper * 0.5) + 0.01), body_y + s.y * 0.05, -s.z * 0.04),
                Detachable { zone, threshold: 48.0, size: dsize },
            ));
        }
    }
    if a.headlights {
        parent.spawn((
            SpotLight { color: Color::srgb(1.0, 0.93, 0.8), intensity: 900_000.0, range: 48.0, outer_angle: 0.5, inner_angle: 0.3, shadow_maps_enabled: false, ..default() },
            bevy::light::VolumetricLight,
            Transform::from_xyz(0.0, body_y + 0.1, -s.z * 0.5 - 0.2).looking_at(Vec3::new(0.0, -2.0, -s.z * 0.5 - 18.0), Vec3::Y),
            CarHeadlight,
        ));
    }

    // Фары и стопы
    let hood_y = body_top - sh.hood * s.y * 0.9 - 0.12;
    let trunk_y = body_top - sh.trunk * s.y * 0.9 - 0.12;
    let light_mesh = cube(a, Vec3::new(0.3, 0.12, 0.05));
    for side in [-1.0f32, 1.0] {
        parent.spawn((Mesh3d(light_mesh.clone()), MeshMaterial3d(head.clone()), Transform::from_xyz(side * (s.x * 0.5 - 0.28), hood_y.max(body_y), -s.z * 0.5 - 0.02), HeadLight));
        parent.spawn((Mesh3d(light_mesh.clone()), MeshMaterial3d(brake.clone()), Transform::from_xyz(side * (s.x * 0.5 - 0.28), trunk_y.max(body_y), s.z * 0.5 + 0.02), BrakeLight));
    }
    parent.spawn((Transform::from_xyz(s.x * 0.3, body_y - s.y * 0.35, s.z * 0.5 + 0.1), ExhaustPoint));

    // Колёса
    let mounts = wheel_mounts(def);
    let wr = def.wheel_radius;
    let tire_w = 0.24 + s.x * 0.04 + if def.style == BodyStyle::Monster { 0.35 } else { 0.0 };
    let tire_mesh = a.meshes.add(Cylinder::new(wr, tire_w).mesh().resolution(20));
    let rim_mesh = cube(a, Vec3::new(tire_w + 0.02, wr * 1.1, 0.12));
    let hub_mesh = a.meshes.add(Cylinder::new(wr * 0.55, tire_w + 0.03).mesh().resolution(12));
    for (i, m) in mounts.iter().enumerate() {
        let rest = def.travel * cfg.setup.ride;
        parent
            .spawn((Transform::from_translation(*m - Vec3::Y * rest * 0.58), Visibility::default(), WheelVisual(i)))
            .with_children(|w| {
                w.spawn((Mesh3d(tire_mesh.clone()), MeshMaterial3d(tire.clone()), Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))));
                w.spawn((Mesh3d(hub_mesh.clone()), MeshMaterial3d(rim.clone()), Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))));
                w.spawn((Mesh3d(rim_mesh.clone()), MeshMaterial3d(rim.clone()), Transform::default()));
                w.spawn((Mesh3d(rim_mesh.clone()), MeshMaterial3d(rim.clone()), Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))));
            });
    }

    // Оружие
    let mp = mount_points(def);
    for (mount, kind) in &cfg.loadout {
        build_weapon(parent, a, def, &mp, *mount, *kind);
    }
}

/// Наклейка повреждения: видна, начиная со своей стадии износа.
#[derive(Component)]
pub struct DamageMark(pub u8);

/// Слой износа модели Kenney: материалы стадий 1..4.
#[derive(Component)]
pub struct KitWear(pub [Handle<StandardMaterial>; 4]);

/// Смещение визуального колеса от физической точки подвески (колёса модели стоят в своих арках).
#[derive(Component)]
pub struct WheelOffset(pub Vec3);

/// Машина из Kenney Car Kit: кузов делится на краску (цвет скина), отделку и стекло;
/// бамперы и двери отрываются; колёса модели масштабируются под радиус физики.
fn build_kit_model(
    parent: &mut ChildSpawnerCommands,
    a: &mut ModelAssets,
    cfg: &CarConfig,
    fit: &crate::carfit::CarFit,
    model: &crate::carkit::KitModel,
    (colormap, paintmap): (Handle<Image>, Handle<Image>),
) {
    let def = cfg.def();
    let skin = cfg.skin_def();
    let col = skin.map(|k| k.color).unwrap_or(cfg.color);
    let paint_color = Color::srgb(col[0], col[1], col[2]);
    // серая карта хранит яркость/1.3, поэтому цвет краски умножен на 1.3
    let lin = paint_color.to_linear();
    let tex = a.wear.as_ref().and_then(|w| w.car_tex.clone());
    let (paint_base, paint) = if let Some(t) = &tex {
        // реалистичная краска: лак с бликами, разводы, царапины и сколы из карт износа (кубическая развёртка)
        let base = Color::LinearRgba(LinearRgba::rgb(lin.red * 1.1, lin.green * 1.1, lin.blue * 1.1));
        let m = a.materials.add(StandardMaterial {
            base_color: base,
            base_color_texture: Some(t.paint[0].clone()),
            metallic_roughness_texture: Some(t.paint[1].clone()),
            normal_map_texture: Some(t.paint[2].clone()),
            metallic: skin.map(|k| (k.metallic * 1.4).min(1.0)).unwrap_or(0.8),
            perceptual_roughness: skin.map(|k| (k.roughness / 0.32).clamp(0.6, 2.0)).unwrap_or(1.0),
            clearcoat: 1.0,
            clearcoat_perceptual_roughness: 0.06,
            reflectance: 0.55,
            ..default()
        });
        (base, m)
    } else {
        let base = Color::LinearRgba(LinearRgba::rgb(lin.red * 1.3, lin.green * 1.3, lin.blue * 1.3));
        let m = a.materials.add(StandardMaterial {
            base_color: base,
            base_color_texture: Some(paintmap),
            metallic: skin.map(|k| k.metallic).unwrap_or(0.5) * 0.4,
            perceptual_roughness: skin.map(|k| k.roughness).unwrap_or(0.32).max(0.5),
            clearcoat: 0.2,
            clearcoat_perceptual_roughness: 0.3,
            ..default()
        });
        (base, m)
    };
    // отделка по палитре Kenney: резина, пластик, хром (карта шероховатости/металла по ячейкам)
    let trim = a.materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.9, 0.9),
        base_color_texture: Some(colormap.clone()),
        metallic: if tex.is_some() { 1.0 } else { 0.15 },
        perceptual_roughness: if tex.is_some() { 1.0 } else { 0.6 },
        metallic_roughness_texture: tex.as_ref().map(|t| t.palette_mr.clone()),
        ..default()
    });
    // бамперы и отрывные листы: рифлёная сталь
    let armor = match &tex {
        Some(t) => a.materials.add(StandardMaterial {
            base_color: Color::srgb(0.62, 0.62, 0.64),
            base_color_texture: Some(t.armor[0].clone()),
            metallic_roughness_texture: Some(t.armor[1].clone()),
            normal_map_texture: Some(t.armor[2].clone()),
            metallic: 1.0,
            perceptual_roughness: 1.0,
            ..default()
        }),
        None => trim.clone(),
    };
    let glass = a.materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.07, 0.09),
        metallic: 0.6,
        perceptual_roughness: 0.06,
        reflectance: 0.9,
        ..default()
    });
    let head = emissive(a, LinearRgba::rgb(6.0, 5.6, 4.6));
    let brake = a.materials.add(StandardMaterial { base_color: Color::srgb(0.4, 0.0, 0.0), emissive: LinearRgba::rgb(0.6, 0.02, 0.01), ..default() });
    parent.spawn((CarPaint { handle: paint.clone(), base: paint_base, textured: tex.is_some() }, Transform::default(), Visibility::default()));

    let lift = kit_lift(def, fit);
    let rest = def.travel * cfg.setup.ride;
    let y0 = crate::config::wheel_mount_y(def) - rest * 0.58 + lift;
    let half = (fit.max - fit.min) * 0.5;
    // износ моделей Kenney — наклейки повреждений по стадиям (см. ниже), без слоя на весь кузов
    let wear_mats: Option<[Handle<StandardMaterial>; 4]> = None;
    for (name, m) in [("paint", &paint), ("trim", &trim), ("glass", &glass)] {
        let Some(p) = model.part(name) else { continue };
        let mesh = if name == "glass" || (name == "paint" && tex.is_some()) { with_tangents(p.mesh_box_uv()) } else { p.mesh() };
        let handle = a.meshes.add(mesh);
        let base: Vec<Vec3> = p.pos.iter().map(|v| Vec3::from_array(*v)).collect();
        let base_n: Vec<Vec3> = p.nrm.iter().map(|v| Vec3::from_array(*v)).collect();
        let mut ec = parent.spawn((
            Mesh3d(handle.clone()),
            MeshMaterial3d(m.clone()),
            Transform::from_xyz(0.0, y0, 0.0),
            BodyDeform { mesh: handle.clone(), base, half, applied: [0.0; 4], base_n },
        ));
        if name == "glass" {
            ec.insert(GlassPanel(glass.clone()));
            continue;
        }
        if let Some(wm) = &wear_mats {
            let shell = a.meshes.add(p.mesh_box_uv());
            let base: Vec<Vec3> = p.pos.iter().map(|v| Vec3::from_array(*v)).collect();
            parent.spawn((
                Mesh3d(shell.clone()),
                MeshMaterial3d(wm[0].clone()),
                BodyDeform { mesh: shell, base, half, applied: [0.0; 4], base_n: Vec::new() },
                Transform::from_xyz(0.0, y0, 0.0).with_scale(Vec3::splat(1.004)),
                KitWear(wm.clone()),
                bevy::light::NotShadowCaster,
            ));
        }
    }
    // фары и стопы модели светятся; если их нет — коробочки по краям
    for (name, m, front) in [("head", &head, true), ("brake", &brake, false)] {
        let mut e = if let Some(p) = model.part(name) {
            parent.spawn((Mesh3d(a.meshes.add(p.mesh())), MeshMaterial3d(m.clone()), Transform::from_xyz(0.0, y0, 0.0).with_scale(Vec3::splat(1.006))))
        } else {
            let z = if front { fit.min.z - 0.02 } else { fit.max.z + 0.02 };
            let lm = a.meshes.add(box_mesh(Vec3::new(0.3, 0.12, 0.05), 1.0));
            let x = fit.max.x - 0.32;
            parent.spawn((Mesh3d(lm.clone()), MeshMaterial3d(m.clone()), Transform::from_xyz(-x, y0 + 0.3, z)));
            parent.spawn((Mesh3d(lm), MeshMaterial3d(m.clone()), Transform::from_xyz(x, y0 + 0.3, z)))
        };
        if front {
            e.insert(HeadLight);
        } else {
            e.insert(BrakeLight);
        }
    }
    // отрывные детали: бамперы, двери, ковш
    for p in model.parts.iter().filter(|p| p.name == "detach") {
        let zone = p.zone.clamp(0, 1) as usize;
        let mesh = if tex.is_some() { with_tangents(p.mesh_box_uv()) } else { p.mesh() };
        parent.spawn((
            Mesh3d(a.meshes.add(mesh)),
            MeshMaterial3d(armor.clone()),
            Transform::from_translation(p.center + Vec3::Y * y0),
            Detachable { zone, threshold: if p.half.length() > 0.8 { 48.0 } else { 32.0 }, size: p.half * 2.0 },
        ));
    }
    // наклейки скина на двери
    if let (Some(k), Some(w)) = (skin, a.wear.clone()) {
        if k.decal > 0 {
            if let Some(tex) = w.decals[(k.decal - 1) as usize].clone() {
                let dm = a.materials.add(StandardMaterial {
                    base_color_texture: Some(tex),
                    alpha_mode: AlphaMode::Blend,
                    perceptual_roughness: k.roughness,
                    metallic: 0.2,
                    depth_bias: 6.0,
                    cull_mode: None,
                    ..default()
                });
                let door = a.meshes.add(Rectangle::new(half.z * 0.8, (fit.roof_y * 0.4).min(0.8)));
                for side in [-1.0f32, 1.0] {
                    parent.spawn((
                        Mesh3d(door.clone()),
                        MeshMaterial3d(dm.clone()),
                        Transform::from_xyz(side * (fit.max.x + 0.012), y0 + fit.roof_y * 0.3, 0.0).with_rotation(Quat::from_rotation_y(side * std::f32::consts::FRAC_PI_2)),
                        bevy::light::NotShadowCaster,
                    ));
                }
            }
        }
    }
    // наклейки повреждений: появляются по стадиям износа, накапливаются
    if let Some(mk) = a.wear.as_ref().and_then(|w| w.marks.clone()) {
        let mut hood_y = fit.min.y;
        for name in ["paint", "trim"] {
            if let Some(p) = model.part(name) {
                for v in &p.pos {
                    if v[2] < fit.min.z + half.z * 0.5 && v[0].abs() < half.x * 0.5 {
                        hood_y = hood_y.max(v[1]);
                    }
                }
            }
        }
        let side_x = fit.max.x - 0.04;
        let mats: Vec<Handle<StandardMaterial>> = mk
            .iter()
            .map(|t| a.materials.add(StandardMaterial { base_color: Color::srgb(0.62, 0.6, 0.58), base_color_texture: Some(t.clone()), alpha_mode: AlphaMode::Blend, perceptual_roughness: 0.8, reflectance: 0.3, depth_bias: 7.0, cull_mode: None, ..default() }))
            .collect();
        let quad = a.meshes.add(Rectangle::new(1.0, 1.0));
        let side = |s: f32| Quat::from_rotation_y(s * std::f32::consts::FRAC_PI_2);
        let up = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
        let hy = y0 + 0.35;
        // (стадия, позиция, поворот, ширина, высота)
        let list: [(u8, Vec3, Quat, f32, f32); 9] = [
            (1, Vec3::new(-side_x, hy, -half.z * 0.25), side(-1.0), 1.1, 0.5),
            (1, Vec3::new(side_x, hy, half.z * 0.2), side(1.0), 1.1, 0.5),
            (2, Vec3::new(side_x, hy + 0.05, -half.z * 0.3), side(1.0), 1.3, 0.55),
            (2, Vec3::new(0.0, y0 + hood_y + 0.012, fit.min.z + half.z * 0.3), up, half.x * 1.1, 0.7),
            (3, Vec3::new(-side_x, hy, half.z * 0.25), side(-1.0), 1.2, 0.6),
            (3, Vec3::new(side_x, hy, half.z * 0.35), side(1.0), 1.0, 0.55),
            (4, Vec3::new(-side_x, hy, -half.z * 0.05), side(-1.0), half.z * 1.0, 0.6),
            (4, Vec3::new(side_x, hy, -half.z * 0.05), side(1.0), half.z * 1.0, 0.6),
            (4, Vec3::new(0.0, y0 + hood_y + 0.014, fit.min.z + half.z * 0.3), up, half.x * 1.5, 0.9),
        ];
        for (stage, pos, rot, w, h) in list {
            parent.spawn((
                Mesh3d(quad.clone()),
                MeshMaterial3d(mats[(stage - 1) as usize].clone()),
                Transform::from_translation(pos).with_rotation(rot).with_scale(Vec3::new(w, h, 1.0)),
                Visibility::Hidden,
                DamageMark(stage),
                bevy::light::NotShadowCaster,
            ));
        }
    }
    if a.headlights {
        parent.spawn((
            SpotLight { color: Color::srgb(1.0, 0.93, 0.8), intensity: 900_000.0, range: 48.0, outer_angle: 0.5, inner_angle: 0.3, shadow_maps_enabled: false, ..default() },
            bevy::light::VolumetricLight,
            Transform::from_xyz(0.0, y0 + 0.35, fit.min.z - 0.2).looking_at(Vec3::new(0.0, -2.0, fit.min.z - 18.0), Vec3::Y),
            CarHeadlight,
        ));
    }
    parent.spawn((Transform::from_xyz(fit.max.x * 0.5, y0 + 0.1, fit.max.z + 0.1), ExhaustPoint));
    if def.style == BodyStyle::Mixer {
        // вращающийся барабан на платформе
        let s = def.size;
        let r = s.x * 0.42;
        let drum = a.meshes.add(Cylinder::new(r, s.z * 0.44).mesh().resolution(18));
        let drum_mat = mat(a, Color::srgb(0.6, 0.6, 0.58), 0.2, 0.6);
        parent
            .spawn((Transform::from_xyz(0.0, y0 + 0.75 + r, s.z * 0.2).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2 + 0.16)), Visibility::default()))
            .with_children(|d| {
                d.spawn((Mesh3d(drum), MeshMaterial3d(drum_mat), Transform::default(), DrumSpin));
                d.spawn((Mesh3d(a.meshes.add(box_mesh(Vec3::new(0.15, s.z * 0.45, r * 2.02), 1.0))), MeshMaterial3d(paint.clone()), Transform::default(), DrumSpin));
            });
    }

    // колёса: позиция модели по X/Z, высота и поворот от подвески; радиус как у физики
    let mounts = crate::vehicle::wheel_mounts(def);
    let big = lift > 0.0;
    for i in 0..4 {
        let Some(p) = model.part(&format!("wheel{i}")) else { continue };
        let mr = fit.wheel_r[if i < 2 { 0 } else { 1 }];
        let k = def.wheel_radius / mr.max(0.05);
        let target = if big { mounts[i] } else { Vec3::new(fit.wheels[i].x, mounts[i].y, fit.wheels[i].z) };
        let offset = Vec3::new(target.x - mounts[i].x, 0.0, target.z - mounts[i].z);
        let mesh = a.meshes.add(p.mesh());
        parent
            .spawn((Transform::from_translation(mounts[i] + offset - Vec3::Y * rest * 0.58), Visibility::default(), WheelVisual(i), WheelOffset(offset)))
            .with_children(|w| {
                w.spawn((Mesh3d(mesh), MeshMaterial3d(trim.clone()), Transform::from_scale(Vec3::new(k.min(1.3), k, k))));
            });
    }

    let mp = mount_points(def);
    for (mount, kind) in &cfg.loadout {
        build_weapon(parent, a, def, &mp, *mount, *kind);
    }
}

fn build_weapon(parent: &mut ChildSpawnerCommands, a: &mut ModelAssets, def: &CarDef, mp: &Mounts, mount: Mount, kind: WeaponKind) {
    let wd = weapon_def(kind);
    let accent = Color::srgb(wd.color[0], wd.color[1], wd.color[2]);
    let gun = mat(a, Color::srgb(0.18, 0.18, 0.2), 0.8, 0.35);
    // корпуса оружия — вороненая сталь, цвет оружия только полосой (раньше яркие блоки выглядели игрушкой)
    let acc = mat(a, Color::srgb(0.3, 0.31, 0.33), 0.85, 0.42);
    let stripe = mat(a, accent, 0.3, 0.45);
    let steel = mat(a, Color::srgb(0.7, 0.71, 0.73), 1.0, 0.25);
    let positions: Vec<(Vec3, Quat)> = match mount {
        Mount::Front => vec![(mp.front, Quat::IDENTITY)],
        Mount::Roof => vec![(mp.roof, Quat::IDENTITY)],
        Mount::Rear => vec![(mp.rear, Quat::from_rotation_y(std::f32::consts::PI))],
        Mount::Sides => vec![
            (Vec3::new(-mp.side.x, mp.side.y, mp.side.z), Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
            (mp.side, Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
        ],
    };
    let s = def.size;
    for (pos, rot) in positions {
        let mut ec = parent.spawn((Transform::from_translation(pos).with_rotation(rot), Visibility::default()));
        let barrel = a.meshes.add(Cylinder::new(0.06, 0.9).mesh().resolution(8));
        let cube = |a: &mut ModelAssets, size: Vec3| a.meshes.add(box_mesh(size, 1.0));
        match kind {
            WeaponKind::MachineGun => {
                if mount == Mount::Roof {
                    ec.insert(TurretVisual);
                    let base = a.meshes.add(Cylinder::new(0.35, 0.2).mesh().resolution(12));
                    let body = cube(a, Vec3::new(0.4, 0.3, 0.6));
                    ec.with_children(|c| {
                        c.spawn((Mesh3d(base), MeshMaterial3d(gun.clone()), Transform::from_xyz(0.0, 0.1, 0.0)));
                        c.spawn((Mesh3d(body), MeshMaterial3d(gun.clone()), Transform::from_xyz(0.0, 0.35, 0.0)));
                        for x in [-0.1, 0.1] {
                            c.spawn((Mesh3d(barrel.clone()), MeshMaterial3d(gun.clone()), Transform::from_xyz(x, 0.38, -0.6).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))));
                        }
                    });
                } else {
                    ec.with_children(|c| {
                        for x in [-0.25, 0.25] {
                            c.spawn((Mesh3d(barrel.clone()), MeshMaterial3d(gun.clone()), Transform::from_xyz(x, 0.1, -0.3).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))));
                        }
                    });
                }
            }
            WeaponKind::Rockets => {
                let pod = cube(a, Vec3::new(0.6, 0.4, 1.0));
                let tube = a.meshes.add(Cylinder::new(0.09, 0.05).mesh().resolution(8));
                let red = emissive(a, LinearRgba::rgb(2.0, 0.4, 0.1));
                ec.with_children(|c| {
                    c.spawn((Mesh3d(pod), MeshMaterial3d(acc.clone()), Transform::from_xyz(0.0, 0.25, 0.0)));
                    c.spawn((Mesh3d(cube(a, Vec3::new(0.62, 0.07, 0.3))), MeshMaterial3d(stripe.clone()), Transform::from_xyz(0.0, 0.25, 0.2)));
                    for (x, y) in [(-0.15, 0.15), (0.15, 0.15), (-0.15, 0.35), (0.15, 0.35)] {
                        c.spawn((Mesh3d(tube.clone()), MeshMaterial3d(red.clone()), Transform::from_xyz(x, y, -0.51).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))));
                    }
                });
            }
            WeaponKind::Homing => {
                let pod = cube(a, Vec3::new(0.9, 0.5, 0.8));
                let cell = cube(a, Vec3::new(0.2, 0.15, 0.02));
                let red = emissive(a, LinearRgba::rgb(3.0, 0.3, 0.2));
                ec.with_children(|c| {
                    c.spawn((Mesh3d(pod), MeshMaterial3d(acc.clone()), Transform::from_xyz(0.0, 0.3, 0.0).with_rotation(Quat::from_rotation_x(0.25))));
                    c.spawn((Mesh3d(cube(a, Vec3::new(0.92, 0.08, 0.3))), MeshMaterial3d(stripe.clone()), Transform::from_xyz(0.0, 0.33, 0.12).with_rotation(Quat::from_rotation_x(0.25))));
                    for k in 0..6 {
                        let x = -0.25 + (k % 3) as f32 * 0.25;
                        let y = 0.25 + (k / 3) as f32 * 0.2;
                        c.spawn((Mesh3d(cell.clone()), MeshMaterial3d(red.clone()), Transform::from_xyz(x, y + 0.1, -0.42).with_rotation(Quat::from_rotation_x(0.25))));
                    }
                });
            }
            WeaponKind::Mines => {
                let crate_m = cube(a, Vec3::new(0.9, 0.35, 0.35));
                ec.with_children(|c| {
                    c.spawn((Mesh3d(crate_m), MeshMaterial3d(acc.clone()), Transform::from_xyz(0.0, 0.0, -0.05)));
                    c.spawn((Mesh3d(cube(a, Vec3::new(0.2, 0.37, 0.37))), MeshMaterial3d(stripe.clone()), Transform::from_xyz(0.0, 0.0, -0.05)));
                });
            }
            WeaponKind::Bomb => {
                let tube = a.meshes.add(Cylinder::new(0.2, 0.9).mesh().resolution(12));
                ec.with_children(|c| {
                    c.spawn((Mesh3d(tube), MeshMaterial3d(gun.clone()), Transform::from_xyz(0.0, 0.4, 0.0).with_rotation(Quat::from_rotation_x(0.7))));
                });
            }
            WeaponKind::Spikes => {
                let cone = a.meshes.add(Cone { radius: 0.08, height: 0.4 }.mesh().resolution(6));
                let span = if mount == Mount::Sides { s.z * 0.8 } else { s.x * 0.9 };
                ec.with_children(|c| {
                    for k in 0..6 {
                        let t = k as f32 / 5.0 - 0.5;
                        c.spawn((Mesh3d(cone.clone()), MeshMaterial3d(steel.clone()), Transform::from_xyz(t * span, 0.0, -0.2).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))));
                    }
                });
            }
            WeaponKind::Plow => {
                let blade = cube(a, Vec3::new(s.x * 1.1, 0.6, 0.12));
                ec.with_children(|c| {
                    c.spawn((Mesh3d(blade), MeshMaterial3d(acc.clone()), Transform::from_xyz(0.0, 0.0, -0.25).with_rotation(Quat::from_rotation_x(-0.35))));
                    c.spawn((Mesh3d(cube(a, Vec3::new(s.x * 1.12, 0.08, 0.13))), MeshMaterial3d(stripe.clone()), Transform::from_xyz(0.0, 0.2, -0.32).with_rotation(Quat::from_rotation_x(-0.35))));
                });
            }
            WeaponKind::Emp => {
                let coil = a.meshes.add(Torus::new(0.06, 0.32).mesh());
                let core = a.meshes.add(Sphere::new(0.16).mesh().ico(1).unwrap());
                let glow = emissive(a, LinearRgba::rgb(0.6, 2.0, 6.0));
                ec.with_children(|c| {
                    c.spawn((Mesh3d(barrel.clone()), MeshMaterial3d(gun.clone()), Transform::from_xyz(0.0, 0.3, 0.0)));
                    for y in [0.35, 0.55] {
                        c.spawn((Mesh3d(coil.clone()), MeshMaterial3d(glow.clone()), Transform::from_xyz(0.0, y, 0.0)));
                    }
                    c.spawn((Mesh3d(core), MeshMaterial3d(glow.clone()), Transform::from_xyz(0.0, 0.8, 0.0)));
                });
            }
            WeaponKind::Oil => {
                let drum = a.meshes.add(Cylinder::new(0.28, 0.7).mesh().resolution(12));
                ec.with_children(|c| {
                    c.spawn((Mesh3d(drum), MeshMaterial3d(acc.clone()), Transform::from_xyz(0.0, 0.1, -0.1).with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))));
                    c.spawn((Mesh3d(a.meshes.add(Cylinder::new(0.29, 0.12).mesh().resolution(12))), MeshMaterial3d(stripe.clone()), Transform::from_xyz(0.0, 0.1, -0.1).with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))));
                });
            }
            WeaponKind::Flamer => {
                let nozzle = a.meshes.add(Cylinder::new(0.1, 0.7).mesh().resolution(10));
                let tank = a.meshes.add(Cylinder::new(0.2, 0.6).mesh().resolution(10));
                ec.with_children(|c| {
                    c.spawn((Mesh3d(nozzle), MeshMaterial3d(gun.clone()), Transform::from_xyz(0.0, 0.1, -0.3).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))));
                    c.spawn((Mesh3d(tank), MeshMaterial3d(acc.clone()), Transform::from_xyz(0.35, 0.15, 0.2).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))));
                    c.spawn((Mesh3d(a.meshes.add(Cylinder::new(0.21, 0.1).mesh().resolution(10))), MeshMaterial3d(stripe.clone()), Transform::from_xyz(0.35, 0.15, 0.2).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))));
                });
            }
        }
    }
}

// ============================================================ системы визуала

pub struct CarModelPlugin;

impl Plugin for CarModelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_wear_assets)
            .add_systems(Update, (apply_dents, paint_damage, spin_drums, aim_turrets, brake_lights, invisibility, wear_stages, detach_parts, debris_life, rebuild_on_respawn));
    }
}

fn apply_dents(
    cars: Query<(&CarState, &Children), Changed<CarState>>,
    mut bodies: Query<&mut BodyDeform>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (st, children) in cars.iter() {
        for c in children.iter() {
            let Ok(mut bd) = bodies.get_mut(c) else { continue };
            let changed = (0..4).any(|i| (st.zones[i] - bd.applied[i]).abs() > 3.0) || (st.zones.iter().all(|z| *z == 0.0) && bd.applied.iter().any(|z| *z > 0.0));
            if !changed {
                continue;
            }
            bd.applied = st.zones;
            let Some(mut mesh) = meshes.get_mut(&bd.mesh) else { continue };
            let half = bd.half;
            let dents: [f32; 4] = std::array::from_fn(|i| (st.zones[i] / 60.0).clamp(0.0, 1.0) * 0.6);
            let mut local_k: Vec<f32> = Vec::with_capacity(bd.base.len());
            let new_pos: Vec<Vec3> = bd
                .base
                .iter()
                .enumerate()
                .map(|(_, v)| {
                    let k = ((v.x * 37.0).round() as i32 as u32).wrapping_mul(73856093) ^ ((v.y * 37.0).round() as i32 as u32).wrapping_mul(19349663) ^ ((v.z * 37.0).round() as i32 as u32).wrapping_mul(83492791);
                    let n = (k.wrapping_mul(2654435761) >> 24) as f32 / 255.0;
                    let crumple = 0.7 + n * 0.6;
                    let fz = (-v.z / half.z).clamp(0.0, 1.0);
                    let rz = (v.z / half.z).clamp(0.0, 1.0);
                    let lx = (-v.x / half.x).clamp(0.0, 1.0);
                    let rx = (v.x / half.x).clamp(0.0, 1.0);
                    let mut d = Vec3::ZERO;
                    d.z += dents[0] * fz.powi(3) * half.z * 0.5 * crumple;
                    d.z -= dents[1] * rz.powi(3) * half.z * 0.5 * crumple;
                    d.x += dents[2] * lx.powi(3) * half.x * 0.5 * crumple;
                    d.x -= dents[3] * rx.powi(3) * half.x * 0.5 * crumple;
                    d.y -= (dents[0] * fz.powi(4) + dents[1] * rz.powi(4)) * 0.3 * crumple * (v.y / half.y).max(0.0);
                    // мелкая «жевка» металла в зоне удара
                    let n2 = (k.wrapping_mul(1597334677) >> 24) as f32 / 255.0 - 0.5;
                    let local = dents[0] * fz.powi(2) + dents[1] * rz.powi(2) + dents[2] * lx.powi(2) + dents[3] * rx.powi(2);
                    let inward = Vec3::new(-v.x / half.x, 0.0, -v.z / half.z) * 0.12;
                    d += (inward + Vec3::Y * 0.05) * n2 * local * 2.0;
                    local_k.push((local * 4.0).min(1.0));
                    *v + d
                })
                .collect();
            let idx: Vec<u32> = match mesh.indices() {
                Some(bevy::render::mesh::Indices::U32(i)) => i.clone(),
                _ => continue,
            };
            let mut normals = compute_normals(&new_pos, &idx);
            // сглаженные нормали сохраняются, мятые зоны получают гранёные
            if bd.base_n.len() == normals.len() {
                for (i, n) in normals.iter_mut().enumerate() {
                    let t = local_k[i];
                    *n = (bd.base_n[i] * (1.0 - t) + Vec3::from_array(*n) * t).normalize_or_zero().to_array();
                }
            }
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, new_pos.iter().map(|v| v.to_array()).collect::<Vec<_>>());
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        }
    }
}

fn paint_damage(
    cars: Query<(&CarState, &Children), Changed<CarState>>,
    paints: Query<&CarPaint>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    for (st, children) in cars.iter() {
        for c in children.iter() {
            let Ok(p) = paints.get(c) else { continue };
            let Some(mut m) = mats.get_mut(&p.handle) else { continue };
            let dmg = 1.0 - st.hp / 100.0;
            let base = p.base.to_linear();
            let k = if st.wrecked { 0.12 } else { 1.0 - dmg * 0.45 };
            let col = Color::LinearRgba(LinearRgba::rgb(base.red * k, base.green * k, base.blue * k));
            if m.base_color != col {
                m.base_color = col;
                if p.textured {
                    // лак тускнеет, краска матовеет; множитель к карте шероховатости
                    m.perceptual_roughness = 1.0 + dmg * 1.8;
                    m.clearcoat = if st.wrecked { 0.0 } else { (1.0 - dmg).powi(2) };
                } else {
                    m.perceptual_roughness = (0.32 + dmg * 0.5).max(0.5);
                    m.clearcoat = if st.wrecked { 0.0 } else { 0.2 * (1.0 - dmg) };
                }
                let shield = st.has(PowerupKind::Shield);
                let mega = st.has(PowerupKind::MegaRam);
                m.emissive = if shield {
                    LinearRgba::rgb(0.6, 0.8, 1.2)
                } else if mega {
                    LinearRgba::rgb(1.2, 0.3, 0.0)
                } else {
                    LinearRgba::BLACK
                };
            }
        }
    }
}

fn spin_drums(time: Res<Time>, mut q: Query<&mut Transform, With<DrumSpin>>) {
    for mut t in q.iter_mut() {
        t.rotate_local_y(time.delta_secs() * 1.6);
    }
}

fn aim_turrets(cars: Query<(&CarState, &Children)>, mut turrets: Query<&mut Transform, With<TurretVisual>>) {
    for (st, children) in cars.iter() {
        for c in children.iter() {
            if let Ok(mut t) = turrets.get_mut(c) {
                let target = Quat::from_rotation_y(st.turret_yaw);
                t.rotation = t.rotation.slerp(target, 0.3);
            }
        }
    }
}

fn brake_lights(
    cars: Query<(&CarState, &Children), Changed<CarState>>,
    lights: Query<&MeshMaterial3d<StandardMaterial>, With<BrakeLight>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    for (st, children) in cars.iter() {
        for c in children.iter() {
            if let Ok(h) = lights.get(c) {
                if let Some(mut m) = mats.get_mut(&h.0) {
                    let target = if st.braking { LinearRgba::rgb(3.2, 0.1, 0.05) } else { LinearRgba::rgb(0.5, 0.02, 0.01) };
                    if m.emissive != target {
                        m.emissive = target;
                    }
                }
            }
        }
    }
}

/// Невидимка: чужие машины с эффектом не рисуются, своя видна.
fn invisibility(local: Option<Res<crate::game::LocalPlayer>>, mut cars: Query<(&crate::game::NetId, &CarState, &mut Visibility), With<CarVisual>>) {
    let me = local.and_then(|l| l.car);
    for (id, st, mut vis) in cars.iter_mut() {
        let hide = st.has(PowerupKind::Invisibility) && Some(*id) != me;
        let want = if hide { Visibility::Hidden } else { Visibility::Inherited };
        if *vis != want {
            *vis = want;
        }
    }
}

fn setup_wear_assets(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let wear = images.add(crate::gfx::wear_texture(256, 31));
    let crack = images.add(crate::gfx::cracked_glass_texture(256, 5));
    let loaded: Vec<Handle<Image>> = (1..=4).filter_map(|i| crate::extassets::load_png(&mut images, &format!("damage/damage_stage{i}.png"), true, true)).collect();
    let stages = if loaded.len() == 4 { Some([loaded[0].clone(), loaded[1].clone(), loaded[2].clone(), loaded[3].clone()]) } else { None };
    let decals = [
        crate::extassets::load_png(&mut images, "skins/skin_horek_decal.png", true, false),
        crate::extassets::load_png(&mut images, "skins/skin_plombir_decal.png", true, false),
    ];
    let kit = match (
        crate::extassets::load_png(&mut images, "cars/colormap.png", true, false),
        crate::extassets::load_png(&mut images, "cars/colormap_paint.png", false, false),
    ) {
        (Some(a), Some(b)) => Some((a, b)),
        _ => None,
    };
    let m: Vec<Handle<Image>> = (1..=4).filter_map(|i| crate::extassets::load_png(&mut images, &format!("damage/mark_{i}.png"), true, false)).collect();
    let marks = if m.len() == 4 { Some([m[0].clone(), m[1].clone(), m[2].clone(), m[3].clone()]) } else { None };
    let mut t = |rel: &str, srgb: bool, repeat: bool| crate::extassets::load_png(&mut images, &format!("cars/tex/{rel}.png"), srgb, repeat);
    let car_tex = match (
        t("paint_color", true, true),
        t("paint_mr", false, true),
        t("paint_n", false, true),
        t("armor_color", true, true),
        t("armor_mr", false, true),
        t("armor_n", false, true),
        t("colormap_mr", false, false),
    ) {
        (Some(a), Some(b), Some(c), Some(d), Some(e), Some(f), Some(g)) => Some(CarTex { paint: [a, b, c], armor: [d, e, f], palette_mr: g }),
        _ => None,
    };
    commands.insert_resource(WearAssets { wear, crack, stages, decals, kit, marks, car_tex });
}

/// Стадии износа: чистый, царапины, вмятины и грязь, дыры и ржавчина.
fn wear_stages(
    cars: Query<(&CarState, &Children), Changed<CarState>>,
    shells: Query<&WearShell>,
    mut kit: Query<(&KitWear, &mut MeshMaterial3d<StandardMaterial>)>,
    mut marks: Query<(&DamageMark, &mut Visibility)>,
    glass: Query<&GlassPanel>,
    wear: Option<Res<WearAssets>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    for (st, children) in cars.iter() {
        // спрайтовые стадии: >75% — 1, >50% — 2, >25% — 3, иначе 4 (смена текстуры)
        let sprite_stage = if st.wrecked { 4 } else { crate::balance::damage_stage(st.hp / 100.0) };
        let stage = if st.wrecked {
            1.0
        } else if st.hp > 80.0 {
            0.0
        } else if st.hp > 55.0 {
            0.4
        } else if st.hp > 30.0 {
            0.7
        } else {
            0.95
        };
        for c in children.iter() {
            if let Ok((dm, mut vis)) = marks.get_mut(c) {
                // стадия 1 — целая машина: наклейки с 1-й видны со 2-й стадии и т.д.
                let want = if sprite_stage >= (dm.0 + 1).min(4) { Visibility::Inherited } else { Visibility::Hidden };
                if *vis != want {
                    *vis = want;
                }
            }
            if let Ok((kw, mut mm)) = kit.get_mut(c) {
                let want = &kw.0[(sprite_stage - 1) as usize];
                if mm.0 != *want {
                    mm.0 = want.clone();
                }
            }
            if let Ok(w) = shells.get(c) {
                if let Some(stages) = wear.as_ref().and_then(|w| w.stages.as_ref()) {
                    let want = &stages[(sprite_stage - 1) as usize];
                    let differs = mats.get(&w.0).map(|m| m.base_color_texture.as_ref() != Some(want)).unwrap_or(false);
                    if differs {
                        if let Some(mut m) = mats.get_mut(&w.0) {
                            m.base_color_texture = Some(want.clone());
                        }
                    }
                } else if let Some(mut m) = mats.get_mut(&w.0) {
                    if (m.base_color.alpha() - stage).abs() > 0.01 {
                        m.base_color.set_alpha(stage);
                    }
                }
            }
            if let Ok(g) = glass.get(c) {
                let cracked = st.hp < 60.0 || st.zones[0] > 25.0;
                if let Some(mut m) = mats.get_mut(&g.0) {
                    let has = m.emissive_texture.is_some();
                    if cracked && !has {
                        m.emissive_texture = wear.as_ref().map(|w| w.crack.clone());
                        m.emissive = LinearRgba::rgb(0.5, 0.52, 0.55);
                        m.perceptual_roughness = 0.25;
                    } else if !cracked && has {
                        m.emissive_texture = None;
                        m.emissive = LinearRgba::BLACK;
                        m.perceptual_roughness = 0.06;
                    }
                }
            }
        }
    }
}

/// Отрыв бамперов и дверей, при гибели отлетают колёса.
#[allow(clippy::type_complexity)]
fn detach_parts(
    mut commands: Commands,
    cars: Query<(Entity, &CarState, &Children, &GlobalTransform, &Transform), (With<Car>, Changed<CarState>)>,
    parts: Query<(Entity, &Detachable, &GlobalTransform, &Mesh3d, &MeshMaterial3d<StandardMaterial>)>,
    mut wheels: Query<(&WheelVisual, &GlobalTransform, &Children, &mut Visibility)>,
    wheel_meshes: Query<(&Mesh3d, &MeshMaterial3d<StandardMaterial>)>,
    mut prev: Local<std::collections::HashMap<Entity, (bool, Vec3)>>,
    time: Res<Time>,
    kenney: Option<Res<crate::extassets::KenneyLib>>,
) {
    let mut rng_state = (time.elapsed_secs() * 1000.0) as u32 | 1;
    let mut rnd = move || {
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 17;
        rng_state ^= rng_state << 5;
        rng_state as f32 / u32::MAX as f32 - 0.5
    };
    for (car_e, st, children, gt, tf) in cars.iter() {
        let car_pos = gt.translation();
        let entry = prev.entry(car_e).or_insert((st.wrecked, car_pos));
        let vel = (car_pos - entry.1) / time.delta_secs().max(1e-3);
        let vel = if vel.length() > 60.0 { Vec3::ZERO } else { vel };
        let just_wrecked = st.wrecked && !entry.0;
        *entry = (st.wrecked, car_pos);
        for c in children.iter() {
            let Ok((e, d, pgt, mesh, mat)) = parts.get(c) else { continue };
            if st.zones[d.zone] < d.threshold && !(st.wrecked && d.zone < 2) {
                continue;
            }
            let (_, rot, pos) = pgt.to_scale_rotation_translation();
            let out = (pos - car_pos).normalize_or_zero();
            commands.entity(e).despawn();
            spawn_debris(&mut commands, mesh.0.clone(), mat.0.clone(), pos, rot, d.size, vel * 0.6 + out * 5.0 + Vec3::Y * 4.0, Vec3::new(rnd(), rnd(), rnd()) * 12.0);
        }
        if just_wrecked {
            let _ = tf;
            // разлёт обломков Kenney: двери, пластины, бампер
            if let Some(k) = kenney.as_ref() {
                for n in 0..3 {
                    if k.debris.is_empty() {
                        break;
                    }
                    let h = k.debris[((rnd() + 0.5) * k.debris.len() as f32) as usize % k.debris.len()].clone();
                    let dir = Vec3::new(rnd() * 2.0, 0.6 + rnd().abs(), rnd() * 2.0).normalize();
                    use avian3d::prelude::*;
                    commands.spawn((
                        WorldAssetRoot(h),
                        Transform::from_translation(car_pos + Vec3::Y * (1.0 + n as f32 * 0.3)).with_scale(Vec3::splat(1.6)),
                        RigidBody::Dynamic,
                        Collider::cuboid(0.8, 0.25, 0.8),
                        Mass(10.0),
                        LinearVelocity(vel * 0.4 + dir * 9.0),
                        AngularVelocity(Vec3::new(rnd(), rnd(), rnd()) * 16.0),
                        CollisionLayers::new(crate::game::Layer::Corpse, [crate::game::Layer::World]),
                        Debris(18.0),
                        crate::extassets::Weathered(0.5),
                        DespawnOnExit(crate::game::AppState::Race),
                    ));
                }
            }
            for c in children.iter() {
                let Ok((wv, wgt, wch, mut vis)) = wheels.get_mut(c) else { continue };
                if wv.0 != 0 && wv.0 != 3 {
                    continue;
                }
                *vis = Visibility::Hidden;
                if let Some((m, mt)) = wch.first().and_then(|w| wheel_meshes.get(*w).ok()) {
                    let (_, rot, pos) = wgt.to_scale_rotation_translation();
                    let out = (pos - car_pos).normalize_or_zero();
                    spawn_debris(
                        &mut commands,
                        m.0.clone(),
                        mt.0.clone(),
                        pos,
                        rot * Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                        Vec3::new(0.7, 0.3, 0.7),
                        vel * 0.5 + out * 7.0 + Vec3::Y * 6.0,
                        Vec3::new(rnd(), rnd(), rnd()) * 20.0,
                    );
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_debris(commands: &mut Commands, mesh: Handle<Mesh>, mat: Handle<StandardMaterial>, pos: Vec3, rot: Quat, size: Vec3, vel: Vec3, spin: Vec3) {
    use avian3d::prelude::*;
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Transform::from_translation(pos).with_rotation(rot),
        RigidBody::Dynamic,
        Collider::cuboid(size.x.max(0.05), size.y.max(0.05), size.z.max(0.05)),
        Mass(8.0),
        LinearVelocity(vel),
        AngularVelocity(spin),
        CollisionLayers::new(crate::game::Layer::Corpse, [crate::game::Layer::World]),
        Debris(20.0),
        DespawnOnExit(crate::game::AppState::Race),
    ));
}

fn debris_life(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut Debris)>) {
    let n = q.iter().count();
    for (e, mut d) in q.iter_mut() {
        d.0 -= time.delta_secs() * if n > 60 { 4.0 } else { 1.0 };
        if d.0 <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}

/// После воскрешения модель собирается заново: детали и колёса на месте.
#[allow(clippy::type_complexity)]
fn rebuild_on_respawn(
    mut commands: Commands,
    cars: Query<(Entity, &Car, &CarState), Changed<CarState>>,
    mut prev: Local<std::collections::HashMap<Entity, bool>>,
    wear: Option<Res<WearAssets>>,
    headless: Option<Res<crate::game::Headless>>,
    map: Option<Res<crate::maps::MapData>>,
    settings: Option<Res<crate::config::Settings>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if headless.map(|h| h.0).unwrap_or(false) {
        return;
    }
    for (e, car, st) in cars.iter() {
        let was = prev.insert(e, st.wrecked).unwrap_or(false);
        if was && !st.wrecked {
            let night = map.as_ref().map(|m| m.theme.night).unwrap_or(false);
            let q = settings.as_ref().map(|s| s.quality).unwrap_or(1);
            let mut a = ModelAssets { meshes: &mut meshes, materials: &mut materials, wear: wear.as_deref().cloned(), headlights: night && q >= 1 };
            commands.entity(e).despawn_related::<Children>();
            let cfg = car.cfg.clone();
            commands.entity(e).with_children(|c| build_car_model(c, &mut a, &cfg));
        }
    }
}
