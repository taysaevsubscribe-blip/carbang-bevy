//! «Карьер Глубокий»: террасная чаша, спиральная дорога вниз, прыжки, самосвал-великан, бочки с динамитом.

use super::builders::*;
use super::*;

const RIM_R: f32 = 104.0;
const BOTTOM_R: f32 = 34.0;
const DEPTH: f32 = 32.0;
const TERRACES: f32 = 4.0;
const SPIRAL_TURNS: f32 = 2.4;
const ROAD_HALF: f32 = 6.5;

/// Профиль террас: ровные кольца со ступенями между ними.
fn terrace_height(r: f32) -> f32 {
    if r >= RIM_R {
        return 0.0;
    }
    if r <= BOTTOM_R {
        return -DEPTH;
    }
    let t = (RIM_R - r) / (RIM_R - BOTTOM_R) * TERRACES;
    let k = t.floor();
    let f = t - k;
    // 70% ширины ровно, 30% ступень
    let step = smoothstep(0.7, 1.0, f);
    -(k + step) * DEPTH / TERRACES
}

fn spiral(theta: f32) -> (f32, f32) {
    let t = theta / (SPIRAL_TURNS * std::f32::consts::TAU);
    let r = RIM_R - 2.0 - (RIM_R - BOTTOM_R - 4.0) * t;
    let h = -DEPTH * t;
    (r, h)
}

/// Высота рельефа с учётом спиральной дороги.
fn height_at(x: f32, z: f32) -> f32 {
    let r = (x * x + z * z).sqrt();
    let base = terrace_height(r);
    let ang = z.atan2(x).rem_euclid(std::f32::consts::TAU);
    let mut best = base;
    let total = SPIRAL_TURNS * std::f32::consts::TAU;
    let mut theta = ang;
    while theta <= total {
        let (sr, sh) = spiral(theta);
        let d = (r - sr).abs();
        if d < ROAD_HALF + 4.0 {
            let w = 1.0 - smoothstep(ROAD_HALF, ROAD_HALF + 4.0, d);
            // дорога либо срезает ступень, либо насыпью поднимается над ней
            best = best + (sh - best) * w;
        }
        theta += std::f32::consts::TAU;
    }
    // центральная гора
    if r < 12.0 {
        let m = -DEPTH + 9.0 * (1.0 - smoothstep(8.0, 12.0, r));
        best = best.max(m);
    }
    best
}

pub fn build(b: &mut MapBuilder) {
    let half = b.data.half;
    let step = 2.0;
    let n = ((half.x * 2.0 + 20.0) / step) as usize + 1;
    let mut heights = vec![vec![0.0f32; n]; n];
    let off = (n as f32 - 1.0) * step * 0.5;
    for zi in 0..n {
        for xi in 0..n {
            let x = xi as f32 * step - off;
            let z = zi as f32 * step - off;
            let noise = (fbm(x / 300.0 + 0.5, z / 300.0 + 0.5, 6, 2, 3) - 0.5) * 1.2;
            let r = (x * x + z * z).sqrt();
            let rim_noise = if r > RIM_R + 6.0 { noise * 2.5 } else { noise * 0.3 };
            heights[zi][xi] = height_at(x, z) + rim_noise;
        }
    }
    let mesh = terrain_mesh(&heights, step, b.mats.tile(Mat::Rock));
    let collider = Collider::trimesh_from_mesh(&mesh).expect("terrain collider");
    {
        let tag = b.tag();
        let mut ec = b.commands.spawn((Transform::default(), RigidBody::Static, collider, CollisionLayers::new(Layer::World, LayerMask::ALL), tag));
        if b.visuals {
            let m = b.meshes.add(mesh);
            ec.insert((Mesh3d(m), MeshMaterial3d(b.mats.get(Mat::Rock))));
        }
    }
    // Высоты в сетку (билинейно из массива)
    {
        let g = &mut b.data.grid;
        for j in 0..g.nz {
            for i in 0..g.nx {
                let p = g.pos(i, j);
                let fx = (p.x + off) / step;
                let fz = (p.y + off) / step;
                let x0 = fx.floor().clamp(0.0, (n - 2) as f32) as usize;
                let z0 = fz.floor().clamp(0.0, (n - 2) as f32) as usize;
                let tx = (fx - x0 as f32).clamp(0.0, 1.0);
                let tz = (fz - z0 as f32).clamp(0.0, 1.0);
                let h = heights[z0][x0] * (1.0 - tx) * (1.0 - tz)
                    + heights[z0][x0 + 1] * tx * (1.0 - tz)
                    + heights[z0 + 1][x0] * (1.0 - tx) * tz
                    + heights[z0 + 1][x0 + 1] * tx * tz;
                g.height[j * g.nx + i] = h;
            }
        }
    }

    // Дорожное покрытие спирали (визуальные плиты-полоски)
    let total = SPIRAL_TURNS * std::f32::consts::TAU;
    let mut theta = 0.0;
    let mut spiral_pts = Vec::new();
    while theta < total {
        let (r, h) = spiral(theta);
        spiral_pts.push(Vec3::new(theta.cos() * r, h, theta.sin() * r));
        theta += 0.12;
    }
    if b.visuals {
        for w in spiral_pts.windows(2) {
            let a = w[0];
            let c = w[1];
            let mid = (a + c) * 0.5;
            let d = c - a;
            let yaw = (-d.x).atan2(-d.z);
            let hlen = Vec2::new(d.x, d.z).length();
            let pitch = (d.y / hlen).atan();
            let y = height_at(mid.x, mid.z) + 0.03;
            let rot = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch);
            let tag = b.tag();
            let mesh = b.meshes.add(box_mesh(Vec3::new(ROAD_HALF * 1.6, 0.04, hlen * 1.05), 6.0));
            b.commands.spawn((Mesh3d(mesh), MeshMaterial3d(b.mats.get(Mat::Dirt)), Transform::from_xyz(mid.x, y, mid.z).with_rotation(rot), tag));
        }
    }

    // Обогатительная фабрика на краю
    let plant = Vec2::new(-125.0, -40.0);
    building(b, plant, Vec2::new(18.0, 30.0), 14.0, Mat::Rust, height_at(plant.x, plant.y));
    building(b, Vec2::new(-125.0, 15.0), Vec2::new(14.0, 14.0), 9.0, Mat::ContainerGrey, 0.0);
    // Транспортёрная эстакада, обрывающаяся над карьером: можно заехать и улететь
    let conv_y = 7.0;
    for k in 0..6 {
        let x = -118.0 + k as f32 * 7.0;
        b.solid(Vec3::new(x, conv_y * 0.5, -70.0), Vec3::new(0.8, conv_y, 0.8), 0.0, Mat::Metal);
    }
    b.static_box(Vec3::new(-100.0, conv_y, -70.0), Vec3::new(46.0, 0.5, 6.0), 0.0, Mat::Metal, true);
    b.slope(Vec3::new(-140.0, 0.0, -70.0), Vec3::new(-123.0, conv_y + 0.25, -70.0), 6.0, 0.5, Mat::Hazard);
    // Самосвалы-остовы и кучи песка
    for p in [Vec2::new(-80.0, 95.0), Vec2::new(95.0, 80.0), Vec2::new(115.0, -60.0)] {
        let h = height_at(p.x, p.y);
        b.solid(Vec3::new(p.x, h + 2.5, p.y), Vec3::new(5.0, 5.0, 11.0), 0.5, Mat::ContainerYellow);
    }
    for k in 0..8 {
        let a = k as f32 / 8.0 * std::f32::consts::TAU + 0.3;
        let r = RIM_R + 18.0;
        let p = Vec3::new(a.cos() * r, height_at(a.cos() * r, a.sin() * r), a.sin() * r);
        b.ramp(p, 9.0, 10.0, 2.2, a + std::f32::consts::FRAC_PI_2, Mat::Sand);
    }

    // Прыжок через пропасть: трамплин на второй террасе в сторону центральной горы
    let jump_r = 68.0;
    let ja = 0.9f32;
    let jp = Vec3::new(ja.cos() * jump_r, terrace_height(jump_r), ja.sin() * jump_r);
    let inward = -Vec3::new(ja.cos(), 0.0, ja.sin());
    let yaw = inward.x.atan2(inward.z);
    b.ramp(jp, 8.0, 12.0, 3.5, yaw, Mat::Ramp);

    // Бочки с динамитом на террасах
    for k in 0..7 {
        let a = k as f32 / 7.0 * std::f32::consts::TAU + 1.1;
        let r = RIM_R - 10.0 - (k % 4) as f32 * (RIM_R - BOTTOM_R) / TERRACES;
        let p = Vec3::new(a.cos() * r, terrace_height(r), a.sin() * r);
        let cnt = 3 + k % 3;
        prop_cluster(b, p, PropKind::ExplosiveBarrel, cnt as u32, 2.0);
    }
    for k in 0..5 {
        let a = k as f32 / 5.0 * std::f32::consts::TAU;
        let p = Vec3::new(a.cos() * 22.0, -DEPTH, a.sin() * 22.0);
        prop_cluster(b, p, PropKind::Crate, 3, 2.0);
    }

    // БелАЗ-великан ездит по спирали вниз и обратно
    let mut route: Vec<Vec3> = spiral_pts.iter().step_by(4).map(|p| Vec3::new(p.x, height_at(p.x, p.z) + 2.6, p.z)).collect();
    let back: Vec<Vec3> = route.iter().rev().skip(1).cloned().collect();
    route.extend(back);
    let truck_parts = vec![
        (box_mesh(Vec3::new(5.5, 3.2, 11.0), 3.0), Mat::ContainerYellow, Transform::from_xyz(0.0, 0.6, 0.0)),
        (box_mesh(Vec3::new(5.0, 2.0, 3.0), 2.0), Mat::Glass, Transform::from_xyz(0.0, 2.4, -3.6)),
        (Cylinder::new(1.4, 1.0).mesh().resolution(16).build(), Mat::Black, Transform::from_xyz(-2.9, -1.2, -3.5).with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))),
        (Cylinder::new(1.4, 1.0).mesh().resolution(16).build(), Mat::Black, Transform::from_xyz(2.9, -1.2, -3.5).with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))),
        (Cylinder::new(1.4, 1.0).mesh().resolution(16).build(), Mat::Black, Transform::from_xyz(-2.9, -1.2, 3.5).with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))),
        (Cylinder::new(1.4, 1.0).mesh().resolution(16).build(), Mat::Black, Transform::from_xyz(2.9, -1.2, 3.5).with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))),
    ];
    b.mover(MoverPath::Loop { points: route, speed: 7.5 }, Collider::cuboid(5.5, 4.4, 11.0), truck_parts, "БелАЗ");

    // Рабочие на краю и на дне
    b.ped_zone(Vec2::new(-140.0, -140.0), Vec2::new(140.0, -RIM_R - 4.0));
    b.ped_zone(Vec2::new(-140.0, RIM_R + 4.0), Vec2::new(140.0, 140.0));
    b.ped_zone(Vec2::new(-140.0, -60.0), Vec2::new(-RIM_R - 4.0, 60.0));
    b.ped_zone(Vec2::new(-25.0, -25.0), Vec2::new(25.0, 25.0));

    // Бонусы: на краю, на террасах, на дне, на горе
    for k in 0..8 {
        let a = k as f32 / 8.0 * std::f32::consts::TAU + 0.2;
        let r = RIM_R + 12.0;
        b.pickup(Vec3::new(a.cos() * r, height_at(a.cos() * r, a.sin() * r), a.sin() * r));
    }
    for k in 0..6 {
        let a = k as f32 / 6.0 * std::f32::consts::TAU + 0.6;
        let r = RIM_R - 8.0 - (k % 3) as f32 * 17.5;
        b.pickup(Vec3::new(a.cos() * r, terrace_height(r), a.sin() * r));
    }
    b.pickup(Vec3::new(0.0, height_at(0.0, 0.0), 0.0));
    b.pickup(Vec3::new(26.0, -DEPTH, 0.0));

    // Чекпоинты: вниз по спирали и обратно на край
    let cps = [0.15f32, 0.45, 0.85, 1.0];
    for t in cps {
        let th = t * total * 0.98;
        let (r, _) = spiral(th);
        let p = Vec3::new(th.cos() * r, 0.0, th.sin() * r);
        b.checkpoint(Vec3::new(p.x, height_at(p.x, p.z), p.z));
    }
    b.checkpoint(Vec3::new(-120.0, 0.0, 120.0));
    checkpoint_visuals(b);

    // Старт по краю
    for k in 0..12 {
        let a = k as f32 / 12.0 * std::f32::consts::TAU;
        let r = RIM_R + 20.0;
        let p = Vec3::new(a.cos() * r, 0.0, a.sin() * r);
        let tangent = Vec3::new(-a.sin(), 0.0, a.cos());
        let yaw = (-tangent.x).atan2(-tangent.z);
        b.spawn_point(Vec3::new(p.x, height_at(p.x, p.z), p.z), yaw);
    }
    b.data.kill_y = -60.0;
}
