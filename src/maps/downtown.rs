//! «Неоновый центр»: ночные небоскрёбы, многоэтажная парковка, эстакада, стройка с ядром-маятником.

use super::builders::*;
use super::*;

const ROADS: [f32; 5] = [-120.0, -60.0, 0.0, 60.0, 120.0];
const BLOCK: f32 = 42.0;

pub fn build(b: &mut MapBuilder) {
    flat_ground(b, Mat::Asphalt);
    let half = b.data.half;
    for &r in &ROADS {
        lane_marks(b, Vec2::new(r, -half.y + 4.0), Vec2::new(r, half.y - 4.0));
        lane_marks(b, Vec2::new(-half.x + 4.0, r), Vec2::new(half.x - 4.0, r));
    }
    for i in 0..4 {
        for j in 0..4 {
            let cx = (ROADS[i] + ROADS[i + 1]) * 0.5;
            let cz = (ROADS[j] + ROADS[j + 1]) * 0.5;
            let c = Vec2::new(cx, cz);
            b.floor(Vec3::new(cx, 0.1, cz), Vec3::new(BLOCK, 0.2, BLOCK), Mat::Sidewalk);
            match (i, j) {
                (1, 1) => parking_garage(b, c),
                (2, 1) => roof_target(b, c),
                (2, 2) => construction(b, c),
                (1, 2) => plaza(b, c),
                _ => towers(b, c),
            }
            b.ped_zone(c - Vec2::splat(BLOCK * 0.5), c + Vec2::splat(BLOCK * 0.5));
        }
    }
    // Фонари со светом вдоль центральных улиц
    for k in 0..9 {
        let t = -120.0 + k as f32 * 30.0;
        b.street_lamp(Vec3::new(9.5, 0.2, t + 5.0), k % 2 == 0);
        b.street_lamp(Vec3::new(t + 5.0, 0.2, -9.5), k % 2 == 1);
    }

    elevated_highway(b);
    street_furniture(b);
    skyline(b);

    // Бонусы на перекрёстках
    for (i, &x) in ROADS.iter().enumerate() {
        for (j, &z) in ROADS.iter().enumerate() {
            if (i + j) % 2 == 0 {
                b.pickup(Vec3::new(x, 0.0, z));
            }
        }
    }
    for _ in 0..16 {
        let r = ROADS[b.rng.gen_range(0..5)];
        let t = b.rand(-130.0, 130.0);
        let p = if b.chance(0.5) { Vec3::new(r + b.rand(-5.0, 5.0), 0.0, t) } else { Vec3::new(t, 0.0, r + b.rand(-5.0, 5.0)) };
        b.prop(PropKind::Cone, p, 0.0);
    }
    for p in [
        Vec3::new(-120.0, 0.0, -120.0),
        Vec3::new(60.0, 0.0, -120.0),
        Vec3::new(120.0, 0.0, 60.0),
        Vec3::new(0.0, 0.0, 120.0),
        Vec3::new(-120.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 0.0),
    ] {
        b.checkpoint(p);
    }
    checkpoint_visuals(b);
    for &r in &[-120.0f32, 120.0] {
        for &t in &[-90.0f32, -30.0, 30.0, 90.0] {
            b.spawn_point(Vec3::new(t, 0.0, r), if r < 0.0 { std::f32::consts::FRAC_PI_2 } else { -std::f32::consts::FRAC_PI_2 });
        }
    }
    for &t in &[-90.0f32, 90.0] {
        b.spawn_point(Vec3::new(-120.0, 0.0, t), 0.0);
        b.spawn_point(Vec3::new(0.0, 0.0, t), std::f32::consts::PI);
    }
}

/// Главная ось направления (фасады стоят по осям улиц).
fn to_center_axis(d: Vec2) -> Vec2 {
    if d.x.abs() > d.y.abs() { Vec2::new(d.x.signum(), 0.0) } else { Vec2::new(0.0, d.y.signum()) }
}

/// Поворот, при котором фасад модели Kenney (смотрит в -Z) повернут в сторону dir.
fn face(dir: Vec2) -> f32 {
    if dir.x.abs() > dir.y.abs() {
        if dir.x > 0.0 { -std::f32::consts::FRAC_PI_2 } else { std::f32::consts::FRAC_PI_2 }
    } else if dir.y > 0.0 {
        std::f32::consts::PI
    } else {
        0.0
    }
}

/// Здание City Kit плюс граффити и плакат на боковых стенах (по хешу позиции).
fn kit_building(b: &mut MapBuilder, name: &str, pos: Vec3, dir: Vec2, scale: f32) {
    let yaw = face(dir);
    let size = b.kit(name, pos, yaw, scale, true);
    if size == Vec3::ZERO {
        return;
    }
    // габарит в мировых осях
    let (hx, hz) = if dir.x.abs() > dir.y.abs() { (size.z * 0.5, size.x * 0.5) } else { (size.x * 0.5, size.z * 0.5) };
    let side = Vec2::new(dir.y, -dir.x);
    for (k, s) in [1.0f32, -1.0].iter().enumerate() {
        let wd = side * *s;
        let r = MapBuilder::hash01(pos + Vec3::new(wd.x, 0.0, wd.y), 141 + k as u32);
        if r > 0.6 {
            continue;
        }
        let half = if wd.x.abs() > 0.5 { hx } else { hz };
        let base = Vec3::new(pos.x + wd.x * (half + 0.06), 0.0, pos.z + wd.y * (half + 0.06));
        let along = Vec3::new(dir.x, 0.0, dir.y);
        let qyaw = face(wd) + std::f32::consts::PI;
        if r < 0.35 {
            wall_art(b, base + Vec3::Y * 2.2 + along * (r * 6.0 - 1.0), qyaw, 3.6, true);
        } else {
            wall_art(b, base + Vec3::Y * 2.6 - along * 2.0, qyaw, 2.2, false);
        }
    }
}

const SKYSCRAPERS: [&str; 5] = [
    "city_commercial/building-skyscraper-a",
    "city_commercial/building-skyscraper-b",
    "city_commercial/building-skyscraper-c",
    "city_commercial/building-skyscraper-d",
    "city_commercial/building-skyscraper-e",
];
const MIDRISE: [&str; 11] = [
    "city_commercial/building-a",
    "city_commercial/building-b",
    "city_commercial/building-c",
    "city_commercial/building-d",
    "city_commercial/building-f",
    "city_commercial/building-g",
    "city_commercial/building-h",
    "city_commercial/building-i",
    "city_commercial/building-l",
    "city_commercial/building-m",
    "city_commercial/building-d",
];
const WIDE: [&str; 3] = ["city_commercial/building-j", "city_commercial/building-k", "city_commercial/building-n"];

/// Квартал из моделей City Kit: небоскрёб с лавками, четыре дома или два широких здания.
/// Выбор по хешу позиции, ГСЧ карты не трогается (карта одинакова с графикой и без).
fn towers(b: &mut MapBuilder, c: Vec2) {
    let p3 = Vec3::new(c.x, 0.0, c.y);
    let kind = (MapBuilder::hash01(p3, 21) * 3.0) as u32;
    let y = 0.2;
    // сторона к центру карты: туда смотрит главный фасад
    let to_center = if c.length() > 1.0 { -c.normalize() } else { Vec2::NEG_Y };
    match kind {
        0 => {
            let name = MapBuilder::pick(&SKYSCRAPERS, p3, 22).unwrap_or(SKYSCRAPERS[0]);
            kit_building(b, name, Vec3::new(c.x, y, c.y), to_center_axis(to_center), 16.0);
            for (k, (dx, dz)) in [(-1.0f32, 1.0f32), (1.0, -1.0)].iter().enumerate() {
                let q = c + Vec2::new(dx * 14.5, dz * 14.5);
                let name = MapBuilder::pick(&MIDRISE[..7], Vec3::new(q.x, 0.0, q.y), 23 + k as u32).unwrap_or(MIDRISE[0]);
                kit_building(b, name, Vec3::new(q.x, y, q.y), Vec2::new(0.0, *dz), 12.0);
            }
        }
        1 => {
            for (k, (dx, dz)) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)].iter().enumerate() {
                let q = c + Vec2::new(dx * 10.5, dz * 10.5);
                let name = MapBuilder::pick(&MIDRISE, Vec3::new(q.x, 0.0, q.y), 31 + k as u32).unwrap_or(MIDRISE[0]);
                kit_building(b, name, Vec3::new(q.x, y, q.y), Vec2::new(0.0, *dz), 14.0);
            }
        }
        _ => {
            for (k, dz) in [-1.0f32, 1.0].iter().enumerate() {
                let q = c + Vec2::new(0.0, dz * 10.5);
                let name = MapBuilder::pick(&WIDE, Vec3::new(q.x, 0.0, q.y), 41 + k as u32).unwrap_or(WIDE[0]);
                kit_building(b, name, Vec3::new(q.x, y, q.y), Vec2::new(0.0, *dz), 15.5);
            }
        }
    }
    // неон на углу квартала, обращённом к центру
    let corner = c + to_center * 19.0;
    let yaw = face(to_center) + std::f32::consts::PI;
    neon_sign(b, Vec3::new(corner.x, 9.0, corner.y), yaw, 7.0, 2.2);
}

/// Улица: светофоры на перекрёстках, мусорные баки, знаки, столбы с проводами.
fn street_furniture(b: &mut MapBuilder) {
    for (i, &x) in ROADS.iter().enumerate() {
        for (j, &z) in ROADS.iter().enumerate() {
            let p = Vec3::new(x, 0.0, z);
            if MapBuilder::hash01(p, 51) < 0.75 {
                // два светофора по диагонали перекрёстка
                for (sx, sz) in [(-1.0f32, -1.0f32), (1.0, 1.0)] {
                    b.kit("city_roads/traffic-light", Vec3::new(x + sx * 9.6, 0.2, z + sz * 9.6), if sx > 0.0 { std::f32::consts::PI } else { 0.0 }, 14.0, true);
                }
            }
            if (i + j) % 3 == 1 {
                b.kit("city_roads/road-sign-warning", Vec3::new(x - 9.8, 0.2, z + 9.8), std::f32::consts::FRAC_PI_2, 12.0, false);
            }
        }
    }
    // баки и строительные ограждения вдоль тротуаров
    for k in 0..24 {
        let p = Vec3::new(-130.0 + k as f32 * 11.3, 0.0, 0.0);
        let h = MapBuilder::hash01(p, 61);
        let road = ROADS[(h * 5.0) as usize % 5];
        let t = -135.0 + MapBuilder::hash01(p, 62) * 270.0;
        let side = if MapBuilder::hash01(p, 63) > 0.5 { 9.9 } else { -9.9 };
        // не на перекрёстках
        if ROADS.iter().any(|r| (t - r).abs() < 13.0) {
            continue;
        }
        let along_x = MapBuilder::hash01(p, 64) > 0.5;
        let pos = if along_x { Vec3::new(t, 0.2, road + side) } else { Vec3::new(road + side, 0.2, t) };
        let yaw = if along_x { 0.0 } else { std::f32::consts::FRAC_PI_2 };
        let name = if h < 0.55 { "city_roads/dumpster" } else { "city_roads/construction-barrier" };
        b.kit(name, pos, yaw, if h < 0.55 { 8.0 } else { 10.0 }, h < 0.55);
    }
}

/// Город за стеной: силуэты упрощённых зданий в тумане (без коллизий).
fn skyline(b: &mut MapBuilder) {
    if !b.visuals {
        return;
    }
    let half = b.data.half;
    const LOW: [&str; 16] = [
        "city_commercial/low-detail-building-a",
        "city_commercial/low-detail-building-b",
        "city_commercial/low-detail-building-c",
        "city_commercial/low-detail-building-d",
        "city_commercial/low-detail-building-e",
        "city_commercial/low-detail-building-f",
        "city_commercial/low-detail-building-g",
        "city_commercial/low-detail-building-h",
        "city_commercial/low-detail-building-i",
        "city_commercial/low-detail-building-j",
        "city_commercial/low-detail-building-k",
        "city_commercial/low-detail-building-l",
        "city_commercial/low-detail-building-m",
        "city_commercial/low-detail-building-wide-a",
        "city_commercial/low-detail-building-wide-b",
        "city_commercial/low-detail-building-n",
    ];
    for ring in 0..2 {
        let d = 26.0 + ring as f32 * 30.0;
        let mut t = -half.x - d;
        while t < half.x + d {
            for (side, along_x) in [(-1.0f32, true), (1.0, true), (-1.0, false), (1.0, false)] {
                let p = if along_x { Vec3::new(t, 0.0, side * (half.y + d)) } else { Vec3::new(side * (half.x + d), 0.0, t) };
                let name = MapBuilder::pick(&LOW, p, 71 + ring).unwrap_or(LOW[0]);
                let s = 20.0 + MapBuilder::hash01(p, 72) * 8.0;
                let dir = if along_x { Vec2::new(0.0, -side) } else { Vec2::new(-side, 0.0) };
                b.kit(name, p, face(dir), s, false);
            }
            t += 13.0 + ring as f32 * 4.0;
        }
    }
}

fn plaza(b: &mut MapBuilder, c: Vec2) {
    b.cylinder(Vec3::new(c.x, 0.8, c.y), 5.0, 1.2, Mat::Concrete, true);
    b.deco(Vec3::new(c.x, 1.5, c.y), Vec3::new(1.0, 2.0, 1.0), 0.0, Mat::Neon2);
    for k in 0..4 {
        let a = k as f32 * std::f32::consts::FRAC_PI_2;
        let p = Vec3::new(c.x + a.cos() * 13.0, 0.2, c.y + a.sin() * 13.0);
        b.ramp(p, 6.0, 8.0, 1.8, a + std::f32::consts::FRAC_PI_2, Mat::Ramp);
    }
    b.point_light(Vec3::new(c.x, 6.0, c.y), Color::srgb(0.6, 0.8, 1.0), 400_000.0, 30.0);
    b.pickup(Vec3::new(c.x + 8.0, 0.2, c.y + 8.0));
}

/// Трёхэтажная парковка: пандусы вдоль западного края, крыша с трамплином на соседнюю крышу.
fn parking_garage(b: &mut MapBuilder, c: Vec2) {
    let x0 = c.x - BLOCK * 0.5;
    let z0 = c.y - BLOCK * 0.5;
    let z1 = c.y + BLOCK * 0.5;
    let strip_w = 7.0;
    let floor_x0 = x0 + strip_w;
    let floor_w = BLOCK - strip_w;
    let levels = [5.0f32, 10.0, 15.0];
    for &y in &levels {
        b.static_box(Vec3::new(floor_x0 + floor_w * 0.5, y - 0.25, c.y), Vec3::new(floor_w, 0.5, BLOCK), 0.0, Mat::Concrete, true);
        // бортики по краям этажа (кроме стороны пандусов)
        b.static_box(Vec3::new(c.x + BLOCK * 0.5 - 0.3, y + 0.5, c.y), Vec3::new(0.6, 1.0, BLOCK), 0.0, Mat::Hazard, true);
        b.static_box(Vec3::new(floor_x0 + floor_w * 0.5, y + 0.5, z0 + 0.3), Vec3::new(floor_w, 1.0, 0.6), 0.0, Mat::Hazard, true);
    }
    // колонны
    for ix in 0..3 {
        for iz in 0..3 {
            let px = floor_x0 + 6.0 + ix as f32 * 13.0;
            let pz = z0 + 6.0 + iz as f32 * 15.0;
            b.solid(Vec3::new(px, 7.5, pz), Vec3::new(0.8, 15.0, 0.8), 0.0, Mat::Concrete);
        }
    }
    // Пандусы: вверх на север, на юг, на север
    let sx = x0 + strip_w * 0.5;
    let near = z1 - 4.0;
    let far = z0 + 4.0;
    b.slope(Vec3::new(sx, 0.2, near), Vec3::new(sx, 5.0, far), strip_w, 0.4, Mat::Concrete);
    b.slope(Vec3::new(sx, 5.0, far), Vec3::new(sx, 10.0, near), strip_w, 0.4, Mat::Concrete);
    b.slope(Vec3::new(sx, 10.0, near), Vec3::new(sx, 15.0, far), strip_w, 0.4, Mat::Concrete);
    // площадки у пандусов
    b.static_box(Vec3::new(sx, 4.75, z0 + 2.0), Vec3::new(strip_w, 0.5, 4.0), 0.0, Mat::Concrete, true);
    b.static_box(Vec3::new(sx, 9.75, z1 - 2.0), Vec3::new(strip_w, 0.5, 4.0), 0.0, Mat::Concrete, true);
    b.static_box(Vec3::new(sx, 14.75, z0 + 2.0), Vec3::new(strip_w, 0.5, 4.0), 0.0, Mat::Concrete, true);
    // Трамплин на крыше в сторону соседнего здания (восток)
    b.ramp(Vec3::new(c.x + BLOCK * 0.5 - 7.0, 15.0, c.y), 7.0, 9.0, 1.4, std::f32::consts::FRAC_PI_2, Mat::Ramp);
    b.pickup(Vec3::new(c.x + 5.0, 15.0, c.y + 8.0));
    b.pickup(Vec3::new(c.x, 5.0, c.y - 6.0));
    neon_sign(b, Vec3::new(c.x + 4.0, 17.5, z0 - 0.2), 0.0, 14.0, 2.5);
    b.point_light(Vec3::new(c.x + 4.0, 13.0, c.y), Color::srgb(1.0, 0.85, 0.6), 250_000.0, 25.0);
}

/// Здание с плоской крышей на 13 м, на которую прилетают с парковки.
fn roof_target(b: &mut MapBuilder, c: Vec2) {
    let h = 13.0;
    b.solid(Vec3::new(c.x, h * 0.5 + 0.2, c.y), Vec3::new(BLOCK - 6.0, h, BLOCK - 6.0), 0.0, Mat::FacadeDark);
    b.static_box(Vec3::new(c.x, h + 0.2 + 0.25, c.y), Vec3::new(BLOCK - 6.0, 0.5, BLOCK - 6.0), 0.0, Mat::Concrete, true);
    for side in [-1.0f32, 1.0] {
        b.static_box(Vec3::new(c.x, h + 1.2, c.y + side * (BLOCK * 0.5 - 3.3)), Vec3::new(BLOCK - 6.0, 1.0, 0.5), 0.0, Mat::Hazard, true);
    }
    b.static_box(Vec3::new(c.x + BLOCK * 0.5 - 3.3, h + 1.2, c.y), Vec3::new(0.5, 1.0, BLOCK - 6.0), 0.0, Mat::Hazard, true);
    b.pickup(Vec3::new(c.x + 5.0, h + 0.7, c.y));
    prop_cluster(b, Vec3::new(c.x + 8.0, h + 0.7, c.y - 8.0), PropKind::ExplosiveBarrel, 4, 2.0);
    neon_sign(b, Vec3::new(c.x, 8.0, c.y - (BLOCK - 6.0) * 0.5 - 0.2), 0.0, 16.0, 3.0);
}

fn construction(b: &mut MapBuilder, c: Vec2) {
    b.deco(Vec3::new(c.x, 0.21, c.y), Vec3::new(BLOCK - 2.0, 0.02, BLOCK - 2.0), 0.0, Mat::Dirt);
    // каркас недостроя
    for ix in 0..3 {
        for iz in 0..3 {
            let p = Vec3::new(c.x - 12.0 + ix as f32 * 12.0, 9.0, c.y - 12.0 + iz as f32 * 12.0);
            if ix == 1 && iz == 1 {
                continue;
            }
            b.solid(p, Vec3::new(0.7, 18.0, 0.7), 0.0, Mat::Rust);
        }
    }
    b.static_box(Vec3::new(c.x, 18.2, c.y), Vec3::new(26.0, 0.4, 26.0), 0.0, Mat::Concrete, true);
    // башенный кран
    let crane = Vec3::new(c.x + 16.0, 0.0, c.y - 16.0);
    b.solid(crane + Vec3::Y * 15.0, Vec3::new(1.6, 30.0, 1.6), 0.0, Mat::ContainerYellow);
    b.static_box(Vec3::new(c.x + 25.0, 30.5, c.y - 16.0), Vec3::new(30.0, 1.2, 1.4), 0.0, Mat::ContainerYellow, false);
    // Ядро-маятник над улицей x = 60
    let pivot = Vec3::new(ROADS[3], 24.0, c.y - 16.0);
    let ball_parts = vec![
        (Sphere::new(2.2).mesh().ico(2).unwrap(), Mat::Metal, Transform::default()),
        (box_mesh(Vec3::new(0.15, 18.0, 0.15), 1.0), Mat::Black, Transform::from_xyz(0.0, 9.0, 0.0)),
    ];
    b.mover(
        MoverPath::Pendulum { pivot, length: 20.0, amp: 0.85, period: 6.5, yaw: 0.0 },
        Collider::sphere(2.2),
        ball_parts,
        "Ядро",
    );
    // кучи песка и плиты как трамплины
    b.ramp(Vec3::new(c.x - 8.0, 0.2, c.y + 10.0), 7.0, 9.0, 2.4, 0.0, Mat::Sand);
    b.ramp(Vec3::new(c.x + 8.0, 0.2, c.y + 4.0), 7.0, 9.0, 2.4, std::f32::consts::PI, Mat::Sand);
    prop_cluster(b, Vec3::new(c.x - 10.0, 0.2, c.y - 6.0), PropKind::Pallet, 5, 3.0);
    prop_cluster(b, Vec3::new(c.x + 4.0, 0.2, c.y - 4.0), PropKind::Barrel, 4, 2.0);
    b.pickup(Vec3::new(c.x, 0.2, c.y));
}

/// Эстакада вдоль восточного края: въезды с обоих концов, колонны под ней.
fn elevated_highway(b: &mut MapBuilder) {
    let x = 138.0;
    let y = 7.0;
    let z0 = -100.0;
    let z1 = 100.0;
    let w = 12.0;
    b.static_box(Vec3::new(x, y - 0.4, 0.0), Vec3::new(w, 0.8, z1 - z0), 0.0, Mat::Asphalt, true);
    for side in [-1.0f32, 1.0] {
        b.static_box(Vec3::new(x + side * (w * 0.5 - 0.3), y + 0.5, 0.0), Vec3::new(0.6, 1.0, z1 - z0), 0.0, Mat::Concrete, true);
    }
    lane_marks_elevated(b, x, y, z0, z1);
    let mut z = z0 + 10.0;
    while z < z1 {
        b.cylinder(Vec3::new(x, (y - 0.8) * 0.5, z), 0.8, y - 0.8, Mat::Concrete, true);
        z += 25.0;
    }
    b.slope(Vec3::new(x, 0.0, -142.0), Vec3::new(x, y, z0), w, 0.6, Mat::Asphalt);
    b.slope(Vec3::new(x, 0.0, 142.0), Vec3::new(x, y, z1), w, 0.6, Mat::Asphalt);
    let a = b.nav_manual(Vec3::new(x, 0.3, -138.0), vec![1]);
    let n1 = b.nav_manual(Vec3::new(x, y, z0), vec![0, 2]);
    let n2 = b.nav_manual(Vec3::new(x, y, 0.0), vec![1, 3]);
    let n3 = b.nav_manual(Vec3::new(x, y, z1), vec![2, 4]);
    let _n4 = b.nav_manual(Vec3::new(x, 0.3, 138.0), vec![3]);
    let _ = (a, n1, n2, n3);
    b.pickup(Vec3::new(x, y, 0.0));
    for k in 0..3 {
        b.point_light(Vec3::new(x, y + 5.0, -60.0 + k as f32 * 60.0), Color::srgb(1.0, 0.7, 0.4), 300_000.0, 26.0);
    }
}

fn lane_marks_elevated(b: &mut MapBuilder, x: f32, y: f32, z0: f32, z1: f32) {
    if !b.visuals {
        return;
    }
    let mut z = z0 + 3.0;
    while z < z1 - 3.0 {
        b.deco(Vec3::new(x, y + 0.02, z), Vec3::new(0.22, 0.02, 3.0), 0.0, Mat::White);
        z += 7.0;
    }
}
