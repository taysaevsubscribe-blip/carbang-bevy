//! «Аэродром Северный»: взлётная полоса, ангары насквозь, самолёты, диспетчерская, рулящий лайнер.

use super::builders::*;
use super::*;

pub fn build(b: &mut MapBuilder) {
    flat_ground(b, Mat::Grass);
    let half = b.data.half;
    // Полоса, рулёжки, перрон
    road_strip(b, Vec2::new(-185.0, -70.0), Vec2::new(185.0, -70.0), 40.0, Mat::Asphalt);
    let mut x = -170.0;
    while x < 170.0 {
        b.deco(Vec3::new(x, 0.03, -70.0), Vec3::new(14.0, 0.02, 1.0), 0.0, Mat::White);
        x += 26.0;
    }
    for side in [-1.0, 1.0] {
        b.deco(Vec3::new(0.0, 0.03, -70.0 + side * 18.0), Vec3::new(370.0, 0.02, 0.6), 0.0, Mat::White);
    }
    road_strip(b, Vec2::new(-180.0, -10.0), Vec2::new(180.0, -10.0), 18.0, Mat::Asphalt);
    road_strip(b, Vec2::new(-180.0, -70.0), Vec2::new(-180.0, -10.0), 18.0, Mat::Asphalt);
    road_strip(b, Vec2::new(180.0, -70.0), Vec2::new(180.0, -10.0), 18.0, Mat::Asphalt);
    road_strip(b, Vec2::new(-20.0, -70.0), Vec2::new(-20.0, -10.0), 18.0, Mat::Asphalt);
    b.deco(Vec3::new(-20.0, 0.015, 55.0), Vec3::new(300.0, 0.02, 90.0), 0.0, Mat::Concrete);

    // Ангары насквозь вдоль оси Z
    for hx in [-110.0f32, -40.0, 30.0] {
        hangar(b, Vec2::new(hx, 60.0), Vec3::new(42.0, 15.0, 36.0), 0.0, 22.0, Mat::ContainerGrey, Mat::Metal);
        prop_cluster(b, Vec3::new(hx + 12.0, 0.0, 60.0), PropKind::Crate, 5, 3.0);
        b.pickup(Vec3::new(hx, 0.0, 60.0));
        b.street_lamp(Vec3::new(hx + 24.0, 0.0, 38.0), false);
    }
    // Самолёт внутри одного ангара
    plane(b, Vec3::new(-40.0, 0.0, 66.0), 0.0, 0.7);

    // Диспетчерская вышка
    b.solid(Vec3::new(110.0, 12.0, 70.0), Vec3::new(8.0, 24.0, 8.0), 0.0, Mat::Concrete);
    b.solid(Vec3::new(110.0, 27.0, 70.0), Vec3::new(13.0, 6.0, 13.0), 0.0, Mat::Glass);
    b.deco(Vec3::new(110.0, 30.4, 70.0), Vec3::new(14.0, 0.8, 14.0), 0.0, Mat::White);
    // Терминал
    b.solid(Vec3::new(150.0, 6.0, 110.0), Vec3::new(70.0, 12.0, 30.0), 0.0, Mat::Facade);
    b.ped_zone(Vec2::new(110.0, 85.0), Vec2::new(190.0, 94.0));
    b.floor(Vec3::new(150.0, 0.1, 89.0), Vec3::new(72.0, 0.2, 8.0), Mat::Sidewalk);

    // Стоянка самолётов на перроне
    plane(b, Vec3::new(-150.0, 0.0, 20.0), std::f32::consts::FRAC_PI_2, 1.0);
    plane(b, Vec3::new(80.0, 0.0, 22.0), -std::f32::consts::FRAC_PI_2 + 0.2, 0.9);
    // Багажные тележки-трамплины перед самолётами
    b.ramp(Vec3::new(-150.0, 0.0, 2.0), 6.0, 9.0, 2.8, std::f32::consts::PI, Mat::Ramp);
    b.ramp(Vec3::new(62.0, 0.0, 22.0), 6.0, 9.0, 2.6, -std::f32::consts::FRAC_PI_2, Mat::Ramp);
    // Трамплины между полосой и рулёжкой
    b.ramp(Vec3::new(90.0, 0.0, -40.0), 9.0, 13.0, 3.4, std::f32::consts::FRAC_PI_2, Mat::Ramp);
    b.ramp(Vec3::new(-90.0, 0.0, -40.0), 9.0, 13.0, 3.4, -std::f32::consts::FRAC_PI_2, Mat::Ramp);

    // Топливозаправщики и бочки с керосином
    for p in [Vec3::new(-75.0, 0.0, 15.0), Vec3::new(5.0, 0.0, 15.0), Vec3::new(120.0, 0.0, 30.0)] {
        b.solid(p + Vec3::Y * 1.6, Vec3::new(2.6, 3.2, 9.0), 0.0, Mat::White);
        prop_cluster(b, p + Vec3::new(4.0, 0.0, 0.0), PropKind::ExplosiveBarrel, 4, 1.6);
    }
    // Конусы вдоль рулёжки
    let mut x = -170.0;
    while x < 170.0 {
        b.prop(PropKind::Cone, Vec3::new(x, 0.0, -20.0), 0.0);
        x += 20.0;
    }
    // Лес и холмы-трамплины по краям
    for _ in 0..30 {
        let x = b.rand(-190.0, 190.0);
        let z = b.rand(100.0, 145.0);
        if x > 105.0 && x < 195.0 {
            continue;
        }
        let s = b.rand(0.9, 1.5);
        tree(b, Vec3::new(x, 0.0, z), s);
    }
    for _ in 0..18 {
        let x = b.rand(-190.0, 190.0);
        let z = b.rand(-145.0, -100.0);
        let s = b.rand(0.9, 1.5);
        tree(b, Vec3::new(x, 0.0, z), s);
    }

    // Рулящий лайнер: обходит аэродром по кругу через перрон и рулёжку
    let path = vec![
        Vec3::new(-170.0, 3.2, -10.0),
        Vec3::new(170.0, 3.2, -10.0),
        Vec3::new(170.0, 3.2, -70.0),
        Vec3::new(-170.0, 3.2, -70.0),
    ];
    let (parts, collider) = plane_parts(1.0);
    b.mover(MoverPath::Loop { points: path, speed: 13.0 }, collider, parts, "Лайнер");

    // Радар на вышке (вращается, недоступен)
    b.mover(
        MoverPath::Spin { center: Vec3::new(110.0, 32.5, 70.0), rate: 1.2 },
        Collider::cuboid(6.0, 1.2, 0.4),
        vec![(box_mesh(Vec3::new(6.0, 1.2, 0.4), 1.0), Mat::White, Transform::default())],
        "Радар",
    );

    b.ped_zone(Vec2::new(-150.0, 30.0), Vec2::new(60.0, 95.0));
    b.ped_zone(Vec2::new(-180.0, 5.0), Vec2::new(180.0, 30.0));

    for p in [
        Vec3::new(-120.0, 0.0, -70.0),
        Vec3::new(120.0, 0.0, -70.0),
        Vec3::new(0.0, 0.0, -40.0),
        Vec3::new(-150.0, 0.0, 60.0),
        Vec3::new(140.0, 0.0, 45.0),
        Vec3::new(-60.0, 0.0, 115.0),
        Vec3::new(60.0, 0.0, 115.0),
        Vec3::new(-100.0, 0.0, -120.0),
        Vec3::new(100.0, 0.0, -120.0),
        Vec3::new(0.0, 0.0, 30.0),
    ] {
        b.pickup(p);
    }
    for p in [
        Vec3::new(-140.0, 0.0, -70.0),
        Vec3::new(-110.0, 0.0, 60.0),
        Vec3::new(30.0, 0.0, 60.0),
        Vec3::new(140.0, 0.0, -70.0),
        Vec3::new(0.0, 0.0, -10.0),
    ] {
        b.checkpoint(p);
    }
    checkpoint_visuals(b);
    for k in 0..12 {
        let x = -165.0 + k as f32 * 30.0;
        b.spawn_point(Vec3::new(x, 0.0, -40.0), if k % 2 == 0 { std::f32::consts::FRAC_PI_2 } else { -std::f32::consts::FRAC_PI_2 });
    }
    let _ = half;
    // Ночное освещение: мачты с прожекторами и огни полосы
    for (k, x) in [-130.0f32, -65.0, 0.0, 65.0, 130.0].iter().enumerate() {
        let base = Vec3::new(*x, 0.0, -40.0);
        b.solid(base + Vec3::Y * 11.0, Vec3::new(0.8, 22.0, 0.8), 0.0, Mat::Metal);
        b.deco(base + Vec3::Y * 22.3, Vec3::new(3.2, 1.4, 0.8), 0.0, Mat::Lamp);
        b.spot_light(base + Vec3::new(0.0, 21.5, -1.0), Vec3::new(*x, 0.0, -72.0), Color::srgb(1.0, 0.8, 0.55), 6_000_000.0, 110.0, 0.7, k == 2);
        b.spot_light(base + Vec3::new(0.0, 21.5, 1.0), Vec3::new(*x, 0.0, 10.0), Color::srgb(1.0, 0.72, 0.45), 4_000_000.0, 90.0, 0.7, false);
    }
    let mut x = -180.0;
    while x <= 180.0 {
        for side in [-1.0f32, 1.0] {
            b.deco(Vec3::new(x, 0.12, -70.0 + side * 20.5), Vec3::new(0.35, 0.25, 0.35), 0.0, if side > 0.0 { Mat::Lamp } else { Mat::Neon2 });
        }
        x += 18.0;
    }
}

fn plane_parts(scale: f32) -> (Vec<(Mesh, Mat, Transform)>, Collider) {
    let s = scale;
    let fus = Vec3::new(4.0 * s, 4.2 * s, 32.0 * s);
    let wing = Vec3::new(34.0 * s, 0.6 * s, 6.0 * s);
    let tail = Vec3::new(12.0 * s, 0.5 * s, 3.5 * s);
    let fin = Vec3::new(0.5 * s, 6.0 * s, 4.0 * s);
    let parts = vec![
        (box_mesh(fus, 4.0), Mat::White, Transform::from_xyz(0.0, 0.0, 0.0)),
        (box_mesh(Vec3::new(3.6 * s, 1.2 * s, 2.0 * s), 1.0), Mat::Glass, Transform::from_xyz(0.0, 1.2 * s, -15.2 * s)),
        (box_mesh(wing, 4.0), Mat::White, Transform::from_xyz(0.0, 0.4 * s, 1.0 * s)),
        (box_mesh(tail, 3.0), Mat::White, Transform::from_xyz(0.0, 1.4 * s, 14.0 * s)),
        (box_mesh(fin, 2.0), Mat::ContainerRed, Transform::from_xyz(0.0, 4.6 * s, 14.0 * s)),
        (Cylinder::new(1.1 * s, 4.0 * s).mesh().resolution(12).build(), Mat::Metal, Transform::from_xyz(-8.0 * s, -0.8 * s, 0.0).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))),
        (Cylinder::new(1.1 * s, 4.0 * s).mesh().resolution(12).build(), Mat::Metal, Transform::from_xyz(8.0 * s, -0.8 * s, 0.0).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))),
        (box_mesh(Vec3::new(0.5, 3.0 * s, 0.5), 1.0), Mat::Black, Transform::from_xyz(0.0, -2.4 * s, -10.0 * s)),
        (box_mesh(Vec3::new(4.0 * s, 1.0, 0.8), 1.0), Mat::Black, Transform::from_xyz(0.0, -2.8 * s, 3.0 * s)),
    ];
    let collider = Collider::compound(vec![
        (Vec3::ZERO, Quat::IDENTITY, Collider::cuboid(fus.x, fus.y, fus.z)),
        (Vec3::new(0.0, 0.4 * s, 1.0 * s), Quat::IDENTITY, Collider::cuboid(wing.x, wing.y, wing.z)),
        (Vec3::new(0.0, 1.4 * s, 14.0 * s), Quat::IDENTITY, Collider::cuboid(tail.x, tail.y, tail.z)),
        (Vec3::new(0.0, -2.6 * s, -4.0 * s), Quat::IDENTITY, Collider::cuboid(4.0 * s, 1.4, 14.0 * s)),
    ]);
    (parts, collider)
}

/// Неподвижный самолёт на шасси. Под крыльями можно проехать.
fn plane(b: &mut MapBuilder, pos: Vec3, yaw: f32, scale: f32) {
    let y = 3.2 * scale;
    let (parts, collider) = plane_parts(scale);
    let tag = b.tag();
    let mut ec = b.commands.spawn((
        Transform::from_translation(pos + Vec3::Y * y).with_rotation(Quat::from_rotation_y(yaw)),
        RigidBody::Static,
        collider,
        CollisionLayers::new(Layer::World, LayerMask::ALL),
        tag,
    ));
    if b.visuals {
        let parts: Vec<_> = parts.into_iter().map(|(m, mat, t)| (b.meshes.add(m), b.mats.get(mat), t)).collect();
        ec.with_children(|c| {
            for (m, mat, t) in parts {
                c.spawn((Mesh3d(m), MeshMaterial3d(mat), t));
            }
        });
    }
    // фюзеляж блокирует проезд
    b.register_box(pos + Vec3::Y * y, Vec3::new(4.0 * scale, 4.2 * scale, 32.0 * scale), yaw, Mat::White);
}
