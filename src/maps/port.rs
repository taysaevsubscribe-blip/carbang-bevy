//! «Бобровая гавань»: город-сетка, контейнерный двор, причалы, грузовой корабль, товарняк.

use super::builders::*;
use super::*;

const XS: [f32; 5] = [-140.0, -88.0, -36.0, 16.0, 68.0];
const ZS: [f32; 5] = [-112.0, -60.0, -8.0, 44.0, 96.0];
const QUAY_X: f32 = 112.0;

pub fn build(b: &mut MapBuilder) {
    let half = b.data.half;
    // Земля до кромки причала
    let gw = QUAY_X + half.x + 20.0;
    b.static_box(Vec3::new((QUAY_X - half.x - 20.0) * 0.5, -0.5, 0.0), Vec3::new(gw, 1.0, half.y * 2.0 + 40.0), 0.0, Mat::Concrete, true);
    b.water(Vec2::new(QUAY_X, -half.y), Vec2::new(half.x + 20.0, half.y), -1.6);
    b.data.kill_y = -12.0;

    // Дороги
    for &x in &XS {
        road_strip(b, Vec2::new(x, -half.y), Vec2::new(x, half.y), 16.0, Mat::Asphalt);
        lane_marks(b, Vec2::new(x, -half.y + 4.0), Vec2::new(x, half.y - 4.0));
    }
    for &z in &ZS {
        road_strip(b, Vec2::new(-half.x, z), Vec2::new(QUAY_X - 2.0, z), 16.0, Mat::Asphalt);
        lane_marks(b, Vec2::new(-half.x + 4.0, z), Vec2::new(QUAY_X - 6.0, z));
    }

    // Кварталы между дорогами
    for i in 0..XS.len() - 1 {
        for j in 0..ZS.len() - 1 {
            let cx = (XS[i] + XS[i + 1]) * 0.5;
            let cz = (ZS[j] + ZS[j + 1]) * 0.5;
            let c = Vec2::new(cx, cz);
            let size = 36.0;
            // тротуар
            b.floor(Vec3::new(cx, 0.09, cz), Vec3::new(size, 0.18, size), Mat::Sidewalk);
            b.ped_zone(c - Vec2::splat(size * 0.5), c + Vec2::splat(size * 0.5));
            let kind = (i * 7 + j * 3) % 6;
            if i == 3 {
                // восточный ряд у порта: склады и контейнеры
                warehouse_block(b, c);
            } else if kind == 0 {
                plaza_block(b, c);
            } else if kind == 3 {
                park_block(b, c);
            } else {
                city_block(b, c);
            }
            // фонари по углам
            for (dx, dz) in [(-1.0, -1.0), (1.0, 1.0)] {
                let p = Vec3::new(cx + dx * (size * 0.5 - 1.2), 0.18, cz + dz * (size * 0.5 - 1.2));
                b.street_lamp(p, false);
            }
        }
    }

    // Контейнерный двор между городом и причалом
    for row in 0..4 {
        let z = -105.0 + row as f32 * 22.0;
        if (z - ZS[1]).abs() < 10.0 || (z - ZS[2]).abs() < 10.0 {
            continue;
        }
        for k in 0..2 {
            let levels = b.rng.gen_range(1..4);
            container_stack(b, Vec2::new(86.0 + k as f32 * 10.0, z), 0.0, levels);
        }
    }
    for row in 0..3 {
        let z = 40.0 + row as f32 * 26.0;
        let levels = b.rng.gen_range(1..3);
        container_stack(b, Vec2::new(90.0, z + 10.0), 0.0, levels);
    }
    gantry_crane(b, Vec2::new(92.0, -70.0), 22.0, 0.0, 12.0);
    gantry_crane(b, Vec2::new(92.0, 20.0), 22.0, 0.0, 12.0);
    prop_cluster(b, Vec3::new(100.0, 0.0, -20.0), PropKind::Barrel, 8, 4.0);
    prop_cluster(b, Vec3::new(100.0, 0.0, 70.0), PropKind::Crate, 7, 4.0);
    prop_cluster(b, Vec3::new(104.0, 0.0, -95.0), PropKind::ExplosiveBarrel, 5, 2.5);
    b.ped_zone(Vec2::new(78.0, -120.0), Vec2::new(110.0, 120.0));

    // Тумбы по кромке причала (между ними легко упасть в воду)
    let mut z = -half.y + 6.0;
    while z < half.y - 6.0 {
        b.cylinder(Vec3::new(QUAY_X - 0.6, 0.4, z), 0.35, 0.8, Mat::Metal, true);
        z += 9.0;
    }

    // Причалы
    for &pz in &[-85.0f32, -10.0, 70.0] {
        b.floor(Vec3::new((QUAY_X + 150.0) * 0.5, -0.4, pz), Vec3::new(150.0 - QUAY_X + 2.0, 0.8, 14.0), Mat::Wood);
        for k in 0..5 {
            let x = QUAY_X + 4.0 + k as f32 * 8.0;
            for side in [-1.0, 1.0] {
                b.deco(Vec3::new(x, -3.0, pz + side * 6.0), Vec3::new(0.6, 5.0, 0.6), 0.0, Mat::Wood);
            }
        }
        b.pickup(Vec3::new(145.0, 0.0, pz));
        b.ped_zone(Vec2::new(QUAY_X + 2.0, pz - 6.0), Vec2::new(148.0, pz + 6.0));
    }
    // Трамплин в конце среднего причала: прыжок в воду для смелых
    b.ramp(Vec3::new(142.0, 0.0, -10.0), 7.0, 9.0, 2.2, -std::f32::consts::FRAC_PI_2, Mat::Ramp);

    // Корабль у северного причала: палуба на высоте 4.5, въезд по пандусу с причала
    let deck_y = 4.5;
    let ship_c = Vec2::new(137.0, 33.0);
    let ship_size = Vec2::new(38.0, 20.0);
    b.solid(Vec3::new(ship_c.x, -1.0, ship_c.y), Vec3::new(ship_size.x, 10.0, ship_size.y), 0.0, Mat::ContainerRed);
    b.floor(Vec3::new(ship_c.x, deck_y - 0.25, ship_c.y), Vec3::new(ship_size.x, 0.5, ship_size.y), Mat::Metal);
    // фальшборт
    for side in [-1.0, 1.0] {
        b.solid(Vec3::new(ship_c.x + 3.0, deck_y + 0.5, ship_c.y + side * (ship_size.y * 0.5 - 0.2)), Vec3::new(ship_size.x - 6.0, 1.0, 0.4), 0.0, Mat::ContainerRed);
    }
    // надстройка на корме
    b.solid(Vec3::new(ship_c.x + 15.0, deck_y + 5.0, ship_c.y), Vec3::new(7.0, 10.0, 14.0), 0.0, Mat::White);
    b.deco(Vec3::new(ship_c.x + 15.0, deck_y + 10.5, ship_c.y), Vec3::new(7.4, 1.2, 14.4), 0.0, Mat::Glass);
    container_stack_at(b, Vec3::new(ship_c.x + 3.0, deck_y, ship_c.y - 4.0), 0.0, 2);
    container_stack_at(b, Vec3::new(ship_c.x - 4.0, deck_y, ship_c.y + 4.5), 0.0, 1);
    // пандус с северного причала на палубу
    b.slope(Vec3::new(125.0, 0.0, 63.0), Vec3::new(125.0, deck_y, ship_c.y + ship_size.y * 0.5 - 0.5), 8.0, 0.6, Mat::Hazard);
    // трамплин с носа обратно на берег
    b.ramp(Vec3::new(ship_c.x - ship_size.x * 0.5 + 6.0, deck_y, ship_c.y - 3.0), 6.0, 7.0, 1.6, std::f32::consts::FRAC_PI_2, Mat::Ramp);
    b.pickup(Vec3::new(ship_c.x - 6.0, deck_y, ship_c.y - 4.0));
    prop_cluster(b, Vec3::new(ship_c.x + 6.0, deck_y, ship_c.y + 5.0), PropKind::ExplosiveBarrel, 4, 2.0);
    let n0 = b.nav_manual(Vec3::new(125.0, 0.2, 64.0), vec![1]);
    let n1 = b.nav_manual(Vec3::new(125.0, deck_y, 42.0), vec![0, 2]);
    let _n2 = b.nav_manual(Vec3::new(ship_c.x - 6.0, deck_y, ship_c.y), vec![1]);
    let _ = (n0, n1);

    // Железная дорога по улице x = 16: товарняк каждые 40 секунд
    let rail_x = XS[3];
    for side in [-0.75, 0.75] {
        b.deco(Vec3::new(rail_x + side, 0.06, 0.0), Vec3::new(0.12, 0.12, half.y * 2.0), 0.0, Mat::Metal);
    }
    let mut z = -half.y;
    while z < half.y {
        b.deco(Vec3::new(rail_x, 0.03, z), Vec3::new(2.6, 0.05, 0.4), 0.0, Mat::Wood);
        z += 1.2;
    }
    // шлагбаумы у перекрёстков
    for &zz in &ZS {
        for side in [-1.0, 1.0] {
            b.deco(Vec3::new(rail_x + side * 9.0, 1.2, zz + side * 9.0), Vec3::new(0.25, 2.4, 0.25), 0.0, Mat::Hazard);
        }
    }
    train(b, rail_x, half.y);

    // Бонусы на перекрёстках
    for (i, &x) in XS.iter().enumerate() {
        for (j, &z) in ZS.iter().enumerate() {
            if (i + j) % 2 == 0 && x != rail_x {
                b.pickup(Vec3::new(x, 0.0, z));
            }
        }
    }
    // Конусы на дорогах
    for _ in 0..14 {
        let x = XS[b.rng.gen_range(0..XS.len())] + b.rand(-5.0, 5.0);
        let z = b.rand(-110.0, 110.0);
        if (x - rail_x).abs() > 4.0 {
            b.prop(PropKind::Cone, Vec3::new(x, 0.0, z), 0.0);
        }
    }

    // Чекпоинты
    for p in [
        Vec3::new(XS[0], 0.0, ZS[4]),
        Vec3::new(XS[2], 0.0, ZS[0]),
        Vec3::new(130.0, 0.0, -10.0),
        Vec3::new(XS[4], 0.0, ZS[3]),
        Vec3::new(XS[1], 0.0, ZS[2]),
    ] {
        b.checkpoint(p);
    }
    checkpoint_visuals(b);

    // Старт: перекрёстки по периметру сетки
    let mut spawns = Vec::new();
    for &x in &XS {
        if x != rail_x {
            spawns.push((Vec3::new(x, 0.0, ZS[0] + 12.0), std::f32::consts::PI));
            spawns.push((Vec3::new(x, 0.0, ZS[4] - 12.0), 0.0));
        }
    }
    for &z in &ZS[1..4] {
        spawns.push((Vec3::new(XS[0] + 12.0, 0.0, z), -std::f32::consts::FRAC_PI_2));
        spawns.push((Vec3::new(XS[4] - 12.0, 0.0, z), std::f32::consts::FRAC_PI_2));
    }
    for (p, yaw) in spawns {
        b.spawn_point(p, yaw);
    }
}

fn container_stack_at(b: &mut MapBuilder, base: Vec3, yaw: f32, levels: u32) {
    for l in 0..levels {
        container(b, base + Vec3::Y * l as f32 * 2.6, yaw);
    }
}

/// Фасад модели Kenney смотрит в -Z: поворот к стороне dir.
fn face(dir: Vec2) -> f32 {
    if dir.x.abs() > dir.y.abs() {
        if dir.x > 0.0 { -std::f32::consts::FRAC_PI_2 } else { std::f32::consts::FRAC_PI_2 }
    } else if dir.y > 0.0 {
        std::f32::consts::PI
    } else {
        0.0
    }
}

/// Городской квартал из City Kit: два дома или одно промышленное здание.
fn city_block(b: &mut MapBuilder, c: Vec2) {
    const HOUSES: [&str; 8] = [
        "city_commercial/building-a",
        "city_commercial/building-b",
        "city_commercial/building-c",
        "city_commercial/building-d",
        "city_commercial/building-f",
        "city_commercial/building-g",
        "city_commercial/building-h",
        "city_commercial/building-i",
    ];
    const BIG: [&str; 4] = ["city_industrial/building-a", "city_industrial/building-b", "city_industrial/building-e", "city_industrial/building-g"];
    let p3 = Vec3::new(c.x, 0.0, c.y);
    if MapBuilder::hash01(p3, 91) < 0.4 {
        let name = MapBuilder::pick(&BIG, p3, 92).unwrap_or(BIG[0]);
        let size = b.kit(name, Vec3::new(c.x, 0.18, c.y), face(Vec2::new(0.0, -1.0)), 12.5, true);
        // граффити на боковой стене склада
        if size != Vec3::ZERO && MapBuilder::hash01(p3, 94) < 0.7 {
            wall_art(b, Vec3::new(c.x + size.x * 0.5 + 0.06, 2.4, c.y), std::f32::consts::FRAC_PI_2, 4.0, true);
        }
    } else {
        for (k, dz) in [-1.0f32, 1.0].iter().enumerate() {
            let q = c + Vec2::new(0.0, dz * 8.0);
            let name = MapBuilder::pick(&HOUSES, Vec3::new(q.x, 0.0, q.y), 93 + k as u32).unwrap_or(HOUSES[0]);
            let size = b.kit(name, Vec3::new(q.x, 0.18, q.y), face(Vec2::new(0.0, *dz)), 13.0, true);
            if size != Vec3::ZERO && MapBuilder::hash01(Vec3::new(q.x, 0.0, q.y), 95) < 0.5 {
                let side = if MapBuilder::hash01(Vec3::new(q.x, 0.0, q.y), 96) < 0.5 { 1.0 } else { -1.0 };
                wall_art(b, Vec3::new(q.x + side * (size.x * 0.5 + 0.06), 2.6, q.y), side * std::f32::consts::FRAC_PI_2, 2.2, false);
            }
        }
    }
}

fn plaza_block(b: &mut MapBuilder, c: Vec2) {
    // Площадь с фонтаном и трамплином
    b.cylinder(Vec3::new(c.x + 9.0, 0.7, c.y + 9.0), 3.0, 1.2, Mat::Concrete, true);
    b.cylinder(Vec3::new(c.x + 9.0, 2.0, c.y + 9.0), 0.5, 2.6, Mat::Metal, true);
    let ryaw = if b.chance(0.5) { 0.0 } else { std::f32::consts::FRAC_PI_2 };
    b.ramp(Vec3::new(c.x - 4.0, 0.18, c.y - 2.0), 7.0, 10.0, 2.4, ryaw, Mat::Ramp);
    b.pickup(Vec3::new(c.x + 6.0, 0.18, c.y - 8.0));
    for _ in 0..3 {
        let p = Vec3::new(c.x + b.rand(-12.0, 12.0), 0.18, c.y + b.rand(-12.0, 12.0));
        b.prop(PropKind::Crate, p, 0.0);
    }
}

fn park_block(b: &mut MapBuilder, c: Vec2) {
    b.deco(Vec3::new(c.x, 0.19, c.y), Vec3::new(32.0, 0.02, 32.0), 0.0, Mat::Grass);
    b.surface(c - Vec2::splat(16.0), c + Vec2::splat(16.0), 0.85);
    for _ in 0..6 {
        let p = Vec3::new(c.x + b.rand(-13.0, 13.0), 0.18, c.y + b.rand(-13.0, 13.0));
        let s = b.rand(0.8, 1.2);
        tree(b, p, s);
    }
}

/// Портовый склад: большое здание City Kit Industrial, баки, трубы, поддоны.
fn warehouse_block(b: &mut MapBuilder, c: Vec2) {
    const WARE: [&str; 4] = ["city_industrial/building-c", "city_industrial/building-l", "city_industrial/building-r", "city_industrial/building-f"];
    let p3 = Vec3::new(c.x, 0.0, c.y);
    let name = MapBuilder::pick(&WARE, p3, 101).unwrap_or(WARE[0]);
    let size = b.kit(name, Vec3::new(c.x - 3.0, 0.18, c.y), face(Vec2::new(1.0, 0.0)), 12.0, true);
    if size == Vec3::ZERO {
        hangar(b, c, Vec3::new(24.0, 9.0, 30.0), 0.0, 12.0, Mat::ContainerGrey, Mat::Metal);
    }
    // баки и трубы у восточного края квартала
    let r = MapBuilder::hash01(p3, 102);
    let extra = if r < 0.33 { "city_industrial/detail-tank" } else if r < 0.66 { "city_industrial/chimney-medium" } else { "city_industrial/water-tower" };
    b.kit(extra, Vec3::new(c.x + 13.0, 0.18, c.y - 11.0), 0.0, if extra.ends_with("tank") { 9.0 } else { 7.0 }, true);
    prop_cluster(b, Vec3::new(c.x + 12.0, 0.18, c.y + 6.0), PropKind::Pallet, 5, 3.0);
    b.pickup(Vec3::new(c.x + 13.0, 0.18, c.y + 12.0));
}

fn train(b: &mut MapBuilder, x: f32, half_z: f32) {
    let wagons = 6;
    let wagon_len = 13.0;
    let gap = 1.0;
    let total = wagons as f32 * (wagon_len + gap);
    let mut shapes = Vec::new();
    let mut parts = Vec::new();
    for k in 0..wagons {
        let z = -total * 0.5 + k as f32 * (wagon_len + gap) + wagon_len * 0.5;
        let h = if k == 0 { 4.2 } else { 3.6 };
        shapes.push((Vec3::new(0.0, h * 0.5 - 0.6, z), Quat::IDENTITY, Collider::cuboid(3.2, h, wagon_len)));
        let mat = if k == 0 { Mat::ContainerRed } else { [Mat::ContainerBlue, Mat::ContainerGreen, Mat::ContainerGrey, Mat::Rust][k % 4] };
        parts.push((box_mesh(Vec3::new(3.2, h, wagon_len), 3.0), mat, Transform::from_xyz(0.0, h * 0.5 - 0.6, z)));
        parts.push((box_mesh(Vec3::new(2.6, 0.8, wagon_len - 2.0), 3.0), Mat::Black, Transform::from_xyz(0.0, -0.9, z)));
    }
    // фара локомотива
    parts.push((box_mesh(Vec3::new(1.6, 0.6, 0.2), 1.0), Mat::Lamp, Transform::from_xyz(0.0, 2.6, -total * 0.5 - 0.1)));
    let a = Vec3::new(x, 1.6, half_z + total * 0.5 + 10.0);
    let c = Vec3::new(x, 1.6, -half_z - total * 0.5 - 10.0);
    b.mover(
        MoverPath::Line { a, b: c, speed: 22.0, period: 42.0, yaw: 0.0 },
        Collider::compound(shapes),
        parts,
        "Товарняк",
    );
}
