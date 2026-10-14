//! Общие строительные блоки карт: темы, периметр, здания, контейнеры, краны, остовы, деревья.

use super::*;

pub fn half_for(id: MapId) -> Vec2 {
    match id {
        MapId::Port => Vec2::new(160.0, 130.0),
        MapId::Quarry => Vec2::new(150.0, 150.0),
        MapId::Airfield => Vec2::new(200.0, 150.0),
        MapId::Downtown => Vec2::new(150.0, 150.0),
        MapId::Coliseum => Vec2::new(95.0, 95.0),
        MapId::Junkyard => Vec2::new(115.0, 115.0),
    }
}

pub fn theme_for(id: MapId) -> Theme {
    let mut t = theme_base(id);
    // стилизация: ночь светлее и читаемее, лужи скромнее (меньше фото-бликов рядом с плоскими моделями)
    if t.night {
        t.ambient_brightness *= 1.5;
    }
    t.wet *= 0.5;
    // CARBANG_DAYLIGHT: ровный дневной свет для осмотра моделей (галерея машин)
    if std::env::var("CARBANG_DAYLIGHT").is_ok() {
        let p = 42f32.to_radians();
        let y = 210f32.to_radians();
        t.sun_dir = Vec3::new(p.cos() * y.sin(), -p.sin(), p.cos() * y.cos()).normalize();
        t.sun_color = Color::srgb(1.0, 0.96, 0.9);
        t.ambient = Color::srgb(0.7, 0.75, 0.85);
        t.ambient_brightness = 420.0;
        t.fog = Color::srgb(0.62, 0.68, 0.76);
        t.sky_top = Color::srgb(0.25, 0.42, 0.7);
        t.sky_horizon = Color::srgb(0.75, 0.8, 0.86);
        t.night = false;
        t.wet = 0.0;
        t.grade_temp = 0.0;
    }
    t
}

fn theme_base(id: MapId) -> Theme {
    let dir = |pitch: f32, yaw: f32| {
        let p = pitch.to_radians();
        let y = yaw.to_radians();
        Vec3::new(p.cos() * y.sin(), -p.sin(), p.cos() * y.cos()).normalize()
    };
    let c = |r: f32, g: f32, b: f32| Color::srgb(r, g, b);
    match id {
        // Вечер после дождя: низкое тёплое солнце, сырой туман с моря, горят фонари
        MapId::Port => Theme {
            sun_dir: dir(7.0, 35.0),
            sun_color: c(1.0, 0.58, 0.32),
            sun_lux: 2600.0,
            ambient: c(0.42, 0.58, 0.72),
            ambient_brightness: 140.0,
            fog: c(0.30, 0.40, 0.44),
            fog_start: 0.0,
            fog_end: 0.0,
            sky_top: c(0.10, 0.16, 0.24),
            sky_horizon: c(0.85, 0.48, 0.30),
            night: true,
            exposure: 8.0,
            fog_vis: 330.0,
            wet: 0.85,
            haze: 0.025,
            stars: false,
            grade_temp: -0.05,
            grade_sat: 1.05,
        },
        // Промзона в смоге: серо-бирюзовый туман, ржавчина, жёлтые фонари
        MapId::Junkyard => Theme {
            sun_dir: dir(11.0, -60.0),
            sun_color: c(1.0, 0.62, 0.36),
            sun_lux: 2400.0,
            ambient: c(0.48, 0.56, 0.56),
            ambient_brightness: 150.0,
            fog: c(0.40, 0.47, 0.46),
            fog_start: 0.0,
            fog_end: 0.0,
            sky_top: c(0.30, 0.36, 0.38),
            sky_horizon: c(0.62, 0.60, 0.54),
            night: true,
            exposure: 8.0,
            fog_vis: 210.0,
            wet: 0.6,
            haze: 0.035,
            stars: false,
            grade_temp: -0.02,
            grade_sat: 0.95,
        },
        // Ночная арена: прожекторы на мачтах, холодная луна, бирюзовый мрак
        MapId::Coliseum => Theme {
            sun_dir: dir(48.0, 20.0),
            sun_color: c(0.72, 0.8, 1.0),
            sun_lux: 260.0,
            ambient: c(0.22, 0.32, 0.42),
            ambient_brightness: 55.0,
            fog: c(0.13, 0.18, 0.21),
            fog_start: 0.0,
            fog_end: 0.0,
            sky_top: c(0.008, 0.012, 0.025),
            sky_horizon: c(0.10, 0.10, 0.11),
            night: true,
            // итерация по mb1: темнее и теплее, лужи и прожекторы читаются лучше
            exposure: 6.2,
            fog_vis: 190.0,
            wet: 0.55,
            haze: 0.045,
            stars: true,
            grade_temp: -0.02,
            grade_sat: 1.05,
        },
        // Карьер на закате: пыль, оранжевое небо, сухо
        MapId::Quarry => Theme {
            sun_dir: dir(9.0, 120.0),
            sun_color: c(1.0, 0.56, 0.30),
            sun_lux: 7000.0,
            ambient: c(0.55, 0.52, 0.62),
            ambient_brightness: 260.0,
            fog: c(0.60, 0.48, 0.40),
            fog_start: 0.0,
            fog_end: 0.0,
            sky_top: c(0.20, 0.27, 0.44),
            sky_horizon: c(1.0, 0.55, 0.30),
            night: false,
            exposure: 9.2,
            fog_vis: 380.0,
            wet: 0.05,
            haze: 0.015,
            stars: false,
            grade_temp: 0.05,
            grade_sat: 1.1,
        },
        // Аэродром ночью: мачтовые прожекторы, огни полосы, мокрый бетон
        MapId::Airfield => Theme {
            sun_dir: dir(35.0, -150.0),
            sun_color: c(0.72, 0.8, 1.0),
            sun_lux: 380.0,
            ambient: c(0.22, 0.30, 0.40),
            ambient_brightness: 60.0,
            fog: c(0.12, 0.16, 0.19),
            fog_start: 0.0,
            fog_end: 0.0,
            sky_top: c(0.01, 0.015, 0.03),
            sky_horizon: c(0.14, 0.12, 0.12),
            night: true,
            exposure: 5.7,
            fog_vis: 260.0,
            wet: 0.65,
            haze: 0.035,
            stars: true,
            grade_temp: -0.06,
            grade_sat: 1.0,
        },
        // Неоновый центр под дождём
        MapId::Downtown => Theme {
            sun_dir: dir(50.0, 60.0),
            sun_color: c(0.72, 0.78, 1.0),
            sun_lux: 320.0,
            ambient: c(0.36, 0.34, 0.55),
            // стилизация: город читается силуэтами на фиолетово-синем небе, а не тонет в чёрном
            ambient_brightness: 95.0,
            fog: c(0.22, 0.2, 0.34),
            fog_start: 0.0,
            fog_end: 0.0,
            sky_top: c(0.05, 0.05, 0.12),
            sky_horizon: c(0.42, 0.22, 0.4),
            night: true,
            exposure: 5.7,
            fog_vis: 170.0,
            wet: 1.0,
            haze: 0.04,
            stars: false,
            grade_temp: -0.04,
            grade_sat: 1.1,
        },
    }
}

/// Окраины за стеной: деревья, камни, кусты и брошенные машины (только декор, силуэты в тумане).
pub fn outskirts(b: &mut MapBuilder) {
    let Some(k) = b.kenney.clone() else { return };
    let half = b.data.half;
    let rocky = matches!(b.data.id, MapId::Quarry);
    let urban = matches!(b.data.id, MapId::Downtown | MapId::Port);
    let mut i = 0u32;
    let perim = 2.0 * (half.x + half.y) * 2.0;
    let n = (perim / 9.0) as u32;
    while i < n {
        let t = i as f32 / n as f32;
        // точка на прямоугольнике периметра
        let (mut p, out) = {
            let l = t * 4.0;
            match l as u32 {
                0 => (Vec3::new(-half.x + (l.fract()) * 2.0 * half.x, 0.0, -half.y), Vec3::NEG_Z),
                1 => (Vec3::new(half.x, 0.0, -half.y + l.fract() * 2.0 * half.y), Vec3::X),
                2 => (Vec3::new(half.x - l.fract() * 2.0 * half.x, 0.0, half.y), Vec3::Z),
                _ => (Vec3::new(-half.x, 0.0, half.y - l.fract() * 2.0 * half.y), Vec3::NEG_X),
            }
        };
        let dist = 6.0 + MapBuilder::hash01(p, 11) * 26.0;
        p += out * dist;
        let r = MapBuilder::hash01(p, 12);
        let list = if rocky {
            if r < 0.7 { &k.rocks } else { &k.trees }
        } else if urban {
            if r < 0.35 { &k.cars } else { &k.trees }
        } else if r < 0.55 {
            &k.trees
        } else if r < 0.8 {
            &k.bushes
        } else {
            &k.cars
        };
        if let Some(m) = MapBuilder::pick(list, p, 13) {
            let is_car = k.cars.iter().any(|c| c.id() == m.id());
            let s = if is_car { 1.5 } else if rocky && r < 0.7 { 9.0 + r * 8.0 } else { 4.5 + MapBuilder::hash01(p, 14) * 2.5 };
            let t = Transform::from_translation(p).with_rotation(Quat::from_rotation_y(MapBuilder::hash01(p, 15) * 6.28)).with_scale(Vec3::splat(s));
            b.scene(m, t, if is_car { Some(0.45) } else { None });
        }
        i += 1;
    }
}

/// Земля под всей картой и стены по краю.
pub fn perimeter(b: &mut MapBuilder) {
    let half = b.data.half;
    let h = 6.0;
    let t = 2.0;
    let mat = match b.data.id {
        MapId::Quarry => Mat::Rock,
        MapId::Junkyard => Mat::Rust,
        MapId::Coliseum => Mat::Concrete,
        _ => Mat::Concrete,
    };
    b.solid(Vec3::new(0.0, h * 0.5, -half.y - t * 0.5), Vec3::new(half.x * 2.0 + t * 2.0, h, t), 0.0, mat);
    b.solid(Vec3::new(0.0, h * 0.5, half.y + t * 0.5), Vec3::new(half.x * 2.0 + t * 2.0, h, t), 0.0, mat);
    b.solid(Vec3::new(-half.x - t * 0.5, h * 0.5, 0.0), Vec3::new(t, h, half.y * 2.0), 0.0, mat);
    b.solid(Vec3::new(half.x + t * 0.5, h * 0.5, 0.0), Vec3::new(t, h, half.y * 2.0), 0.0, mat);
    // Полоса-предупреждение по верху стены
    for (c, s) in [
        (Vec3::new(0.0, h - 0.3, -half.y - t * 0.5), Vec3::new(half.x * 2.0, 0.4, t + 0.05)),
        (Vec3::new(0.0, h - 0.3, half.y + t * 0.5), Vec3::new(half.x * 2.0, 0.4, t + 0.05)),
        (Vec3::new(-half.x - t * 0.5, h - 0.3, 0.0), Vec3::new(t + 0.05, 0.4, half.y * 2.0)),
        (Vec3::new(half.x + t * 0.5, h - 0.3, 0.0), Vec3::new(t + 0.05, 0.4, half.y * 2.0)),
    ] {
        b.deco(c, s, 0.0, Mat::Hazard);
    }
}

/// Плоская земля (большая плита). Высота поверхности 0.
pub fn flat_ground(b: &mut MapBuilder, mat: Mat) {
    let half = b.data.half;
    b.static_box(Vec3::new(0.0, -0.5, 0.0), Vec3::new(half.x * 2.0 + 40.0, 1.0, half.y * 2.0 + 40.0), 0.0, mat, true);
}

/// Дорожное полотно (визуальная накладка без коллизии).
pub fn road_strip(b: &mut MapBuilder, a: Vec2, c: Vec2, width: f32, mat: Mat) {
    let d = c - a;
    let len = d.length();
    let yaw = (-d.x).atan2(-d.y);
    let center = (a + c) * 0.5;
    b.deco(Vec3::new(center.x, 0.01, center.y), Vec3::new(width, 0.02, len), yaw, mat);
}

/// Разметка: пунктир по центру дороги.
pub fn lane_marks(b: &mut MapBuilder, a: Vec2, c: Vec2) {
    if !b.visuals {
        return;
    }
    let d = c - a;
    let len = d.length();
    let dir = d / len;
    let yaw = (-d.x).atan2(-d.y);
    let mut t = 3.0;
    while t < len - 3.0 {
        let p = a + dir * t;
        b.deco(Vec3::new(p.x, 0.025, p.y), Vec3::new(0.22, 0.02, 3.0), yaw, Mat::White);
        t += 7.0;
    }
}

/// Здание с крышей-бортиком. Возвращает высоту.
pub fn building(b: &mut MapBuilder, center: Vec2, size: Vec2, height: f32, mat: Mat, base_y: f32) -> f32 {
    b.solid(Vec3::new(center.x, base_y + height * 0.5, center.y), Vec3::new(size.x, height, size.y), 0.0, mat);
    b.deco(
        Vec3::new(center.x, base_y + height + 0.3, center.y),
        Vec3::new(size.x + 0.4, 0.6, size.y + 0.4),
        0.0,
        Mat::Concrete,
    );
    // крышное оборудование
    if size.x > 10.0 && size.y > 10.0 {
        let n = (size.x * size.y / 150.0).min(3.0) as i32;
        for _ in 0..n {
            let ox = b.rand(-size.x * 0.3, size.x * 0.3);
            let oz = b.rand(-size.y * 0.3, size.y * 0.3);
            let s = b.rand(1.5, 3.5);
            b.deco(Vec3::new(center.x + ox, base_y + height + s * 0.5, center.y + oz), Vec3::new(s, s, s * 1.3), 0.0, Mat::Metal);
        }
    }
    height
}

pub fn container(b: &mut MapBuilder, pos: Vec3, yaw: f32) {
    let mats = [Mat::ContainerRed, Mat::ContainerBlue, Mat::ContainerYellow, Mat::ContainerGreen, Mat::ContainerGrey];
    let m = mats[b.rng.gen_range(0..mats.len())];
    let e = b.solid(pos + Vec3::Y * 1.3, Vec3::new(2.45, 2.6, 6.1), yaw, m);
    // модель контейнера City Kit поверх коллизии (если наборы на месте)
    const KINDS: [&str; 3] = ["city_industrial/shipping-container-a", "city_industrial/shipping-container-b", "city_industrial/shipping-container-c"];
    let name = KINDS[mats.iter().position(|x| *x == m).unwrap_or(0) % 3];
    if b.visuals && b.kenney.as_ref().map(|k| k.kit.contains_key(name)).unwrap_or(false) {
        b.commands.entity(e).remove::<(Mesh3d, MeshMaterial3d<StandardMaterial>)>();
        b.kit(name, pos, yaw, 7.4, false);
    }
}

/// Штабель контейнеров (1-3 яруса).
pub fn container_stack(b: &mut MapBuilder, pos: Vec2, yaw: f32, levels: u32) {
    for l in 0..levels {
        let jitter = b.rand(-0.3, 0.3);
        let p = Vec3::new(pos.x, l as f32 * 2.6, pos.y);
        let dyaw = b.rand(-0.04, 0.04);
        container(b, p + Quat::from_rotation_y(yaw) * Vec3::new(jitter, 0.0, 0.0), yaw + dyaw);
    }
}

/// Портальный кран: две опоры и балка сверху (под ним можно проехать).
pub fn gantry_crane(b: &mut MapBuilder, center: Vec2, span: f32, yaw: f32, height: f32) {
    let rot = Quat::from_rotation_y(yaw);
    for side in [-1.0, 1.0] {
        for along in [-4.0, 4.0] {
            let p = rot * Vec3::new(side * span * 0.5, 0.0, along);
            b.solid(Vec3::new(center.x + p.x, height * 0.5, center.y + p.z), Vec3::new(1.2, height, 1.2), yaw, Mat::ContainerYellow);
        }
    }
    let top = Vec3::new(center.x, height + 0.8, center.y);
    b.static_box(top, Vec3::new(span + 2.0, 1.6, 10.0), yaw, Mat::ContainerYellow, true);
    b.deco(top + Vec3::Y * 2.0, Vec3::new(4.0, 2.4, 3.0), yaw, Mat::Metal);
}

/// Остов машины (статичный, ржавый).
pub fn wreck(b: &mut MapBuilder, pos: Vec3, yaw: f32, flat: bool) {
    let h = if flat { 0.55 } else { 1.0 };
    let model = b.kenney.as_ref().and_then(|k| MapBuilder::pick(&k.cars, pos, 3));
    if let Some(m) = model {
        // остов Kenney: ржавый, слегка перекошенный; коллизия прежняя
        let e = b.solid(pos + Vec3::Y * h * 0.5, Vec3::new(1.85, h, 4.2), yaw, Mat::Rust);
        b.commands.entity(e).remove::<(Mesh3d, MeshMaterial3d<StandardMaterial>)>();
        let roll = (MapBuilder::hash01(pos, 5) - 0.5) * 0.25;
        let sy = if flat { 0.42 } else { 1.0 };
        let t = Transform::from_translation(pos + Vec3::Y * 0.42 * sy)
            .with_rotation(Quat::from_rotation_y(yaw + if MapBuilder::hash01(pos, 6) > 0.5 { std::f32::consts::PI } else { 0.0 }) * Quat::from_rotation_z(roll))
            .with_scale(Vec3::new(1.3, 1.3 * sy, 1.55));
        b.scene(m, t, Some(0.35 + MapBuilder::hash01(pos, 7) * 0.3));
        return;
    }
    b.solid(pos + Vec3::Y * h * 0.5, Vec3::new(1.85, h, 4.2), yaw, Mat::Rust);
    if !flat {
        b.deco(
            pos + Vec3::Y * (h + 0.3) + Quat::from_rotation_y(yaw) * Vec3::new(0.0, 0.0, 0.2),
            Vec3::new(1.5, 0.6, 2.0),
            yaw,
            Mat::Rust,
        );
    }
}

pub fn tree(b: &mut MapBuilder, pos: Vec3, scale: f32) {
    let model = b.kenney.as_ref().and_then(|k| MapBuilder::pick(&k.trees, pos, 1));
    if let Some(m) = model {
        let vis = b.visuals;
        b.visuals = false;
        b.cylinder(pos + Vec3::Y * 2.0 * scale, 0.3 * scale, 4.0 * scale, Mat::Wood, true);
        b.visuals = vis;
        let s = 5.0 * scale * (0.85 + MapBuilder::hash01(pos, 2) * 0.3);
        let t = Transform::from_translation(pos).with_rotation(Quat::from_rotation_y(MapBuilder::hash01(pos, 4) * 6.28)).with_scale(Vec3::splat(s));
        b.scene(m, t, None);
        return;
    }
    b.cylinder(pos + Vec3::Y * 2.0 * scale, 0.3 * scale, 4.0 * scale, Mat::Wood, true);
    if b.visuals {
        let tag = b.tag();
        let mesh = b.meshes.add(Sphere::new(2.4 * scale).mesh().ico(1).unwrap());
        let (cr, cg) = (b.rand(0.0, 0.1), b.rand(0.0, 0.12));
        let mat = b.std_mats.add(StandardMaterial {
            base_color: Color::srgb(0.2 + cr, 0.38 + cg, 0.15),
            perceptual_roughness: 0.95,
            ..default()
        });
        b.commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_translation(pos + Vec3::Y * 5.2 * scale).with_scale(Vec3::new(1.0, 1.25, 1.0)),
            tag,
        ));
    }
}

/// Сквозной ангар/склад: стены с проёмами, крыша. Ось проезда по локальному Z.
pub fn hangar(b: &mut MapBuilder, center: Vec2, size: Vec3, yaw: f32, door: f32, wall_mat: Mat, roof_mat: Mat) {
    let rot = Quat::from_rotation_y(yaw);
    let c = Vec3::new(center.x, 0.0, center.y);
    let t = 0.6;
    // боковые стены
    for side in [-1.0, 1.0] {
        let p = c + rot * Vec3::new(side * size.x * 0.5, size.y * 0.5, 0.0);
        b.solid(p, Vec3::new(t, size.y, size.z), yaw, wall_mat);
    }
    // торцы с проёмом посередине
    for end in [-1.0, 1.0] {
        let seg = (size.x - door) * 0.5;
        for side in [-1.0, 1.0] {
            let p = c + rot * Vec3::new(side * (door * 0.5 + seg * 0.5), size.y * 0.5, end * size.z * 0.5);
            b.solid(p, Vec3::new(seg, size.y, t), yaw, wall_mat);
        }
        let lintel_h = size.y - 7.0;
        if lintel_h > 0.5 {
            let p = c + rot * Vec3::new(0.0, 7.0 + lintel_h * 0.5, end * size.z * 0.5);
            b.static_box(p, Vec3::new(door, lintel_h, t), yaw, wall_mat, true);
        }
    }
    // крыша
    b.static_box(c + Vec3::Y * (size.y + 0.3), Vec3::new(size.x + 1.0, 0.6, size.z + 1.0), yaw, roof_mat, true);
    // пол внутри другой
    b.deco(c + Vec3::Y * 0.02, Vec3::new(size.x - 1.0, 0.03, size.z - 1.0), yaw, Mat::Concrete);
}

/// Кучка реквизита.
pub fn prop_cluster(b: &mut MapBuilder, center: Vec3, kind: PropKind, count: u32, spread: f32) {
    for i in 0..count {
        let a = i as f32 / count as f32 * std::f32::consts::TAU + b.rand(0.0, 0.8);
        let r = b.rand(0.0, spread);
        let p = center + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
        let yaw = b.rand(0.0, 6.28);
        b.prop(kind, p, yaw);
    }
}

/// Стопка шин (статичная стенка).
pub fn tire_wall(b: &mut MapBuilder, a: Vec2, c: Vec2, height: u32) {
    let d = c - a;
    let len = d.length();
    let n = (len / 0.95) as i32;
    let dir = d / len;
    let yaw = (-d.x).atan2(-d.y);
    b.register_box(
        Vec3::new((a.x + c.x) * 0.5, height as f32 * 0.45 * 0.5, (a.y + c.y) * 0.5),
        Vec3::new(1.0, height as f32 * 0.45, len),
        yaw,
        Mat::Black,
    );
    let center = (a + c) * 0.5;
    let hgt = height as f32 * 0.45;
    let tag = b.tag();
    b.commands.spawn((
        Transform::from_xyz(center.x, hgt * 0.5, center.y).with_rotation(Quat::from_rotation_y(yaw)),
        RigidBody::Static,
        Collider::cuboid(1.0, hgt, len),
        CollisionLayers::new(Layer::World, LayerMask::ALL),
        tag,
    ));
    if !b.visuals {
        return;
    }
    let mesh = b.meshes.add(Torus::new(0.2, 0.46).mesh().build());
    let mat = b.mats.get(Mat::Black);
    for i in 0..n {
        let p = a + dir * (i as f32 * 0.95 + 0.47);
        for l in 0..height {
            let tag = b.tag();
            b.commands.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(p.x, 0.22 + l as f32 * 0.45, p.y),
                tag,
            ));
        }
    }
}

/// Неоновая вывеска на стене.
pub fn neon_sign(b: &mut MapBuilder, pos: Vec3, yaw: f32, w: f32, h: f32) {
    let mats = [Mat::Neon1, Mat::Neon2, Mat::Neon3];
    let m = mats[b.rng.gen_range(0..3)];
    b.deco(pos, Vec3::new(w, h, 0.25), yaw, m);
}

/// Чекпоинт: визуальное кольцо-столб (только визуал).
pub fn checkpoint_visuals(b: &mut MapBuilder) {
    if !b.visuals {
        return;
    }
    let pts = b.data.checkpoints.clone();
    for (i, p) in pts.iter().enumerate() {
        let tag = b.tag();
        let beam = b.meshes.add(Cylinder::new(6.0, 40.0).mesh().resolution(24).segments(1));
        let mat = b.mats.get(Mat::Checkpoint);
        b.commands.spawn((
            Mesh3d(beam),
            MeshMaterial3d(mat),
            Transform::from_translation(*p + Vec3::Y * 20.0),
            Visibility::Hidden,
            CheckpointMarker(i),
            tag,
        ));
    }
}

/// Стандартные стартовые позиции по кругу вокруг центра.
pub fn ring_spawns(b: &mut MapBuilder, center: Vec3, radius: f32, count: usize) {
    for i in 0..count {
        let a = i as f32 / count as f32 * std::f32::consts::TAU;
        let p = center + Vec3::new(a.cos() * radius, 0.0, a.sin() * radius);
        // смотрим по касательной
        let tangent = Vec3::new(-a.sin(), 0.0, a.cos());
        let yaw = (-tangent.x).atan2(-tangent.z);
        let h = b.data.ground_height(p.x, p.z);
        b.spawn_point(Vec3::new(p.x, h, p.z), yaw);
    }
}

/// Плакат или граффити на стене: вертикальный квадрат, повёрнутый лицом в сторону yaw (+Z квадрата).
pub fn wall_art(b: &mut MapBuilder, pos: Vec3, yaw: f32, size: f32, graffiti: bool) {
    if !b.visuals {
        return;
    }
    let list = if graffiti { &b.mats.graffiti } else { &b.mats.posters };
    let Some(m) = MapBuilder::pick(list, pos, 131) else { return };
    let mesh = b.meshes.add(Rectangle::new(size, size));
    let tag = b.tag();
    b.commands.spawn((Mesh3d(mesh), MeshMaterial3d(m), Transform::from_translation(pos).with_rotation(Quat::from_rotation_y(yaw)), bevy::light::NotShadowCaster, tag));
}
