//! «Мясорубка»: ночной стадион-чаша с наклонными стенами, трибунами и вращающейся лопастью.

use super::builders::*;
use super::*;

const FLOOR_R: f32 = 60.0;
const BANK_R: f32 = 76.0;
const BANK_H: f32 = 8.0;

pub fn build(b: &mut MapBuilder) {
    flat_ground(b, Mat::Concrete);
    // Арена: мокрый асфальт с белой круговой разметкой (по образцу mb1)
    if b.visuals {
        let tag = b.tag();
        let mesh = b.meshes.add(Cylinder::new(FLOOR_R + 1.0, 0.04).mesh().resolution(64));
        b.commands.spawn((Mesh3d(mesh), MeshMaterial3d(b.mats.get(Mat::AsphaltWrecked)), Transform::from_xyz(0.0, 0.01, 0.0), tag));
        let paint = b.std_mats.add(StandardMaterial {
            base_color: Color::srgba(0.86, 0.86, 0.82, 0.85),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.6,
            depth_bias: 8.0,
            ..default()
        });
        for (r0, r1) in [(46.0, 46.7), (47.4, 48.1), (14.0, 14.5)] {
            let tag = b.tag();
            let ring = b.meshes.add(ring_mesh(r0, r1, 0.0, 0.0, 96, 1.0));
            b.commands.spawn((Mesh3d(ring), MeshMaterial3d(paint.clone()), Transform::from_xyz(0.0, 0.045, 0.0), bevy::light::NotShadowCaster, tag));
        }
    }
    // Конусы по кромке арены
    for k in 0..20 {
        let a = (k as f32 + 0.5) / 20.0 * std::f32::consts::TAU;
        let r = FLOOR_R - 3.0;
        b.prop(PropKind::Cone, Vec3::new(a.cos() * r, 0.0, a.sin() * r), a);
    }

    // Наклонная чаша: меш-кольцо + коллизия трёхмерной сеткой
    let ring = ring_mesh(FLOOR_R, BANK_R, 0.0, BANK_H, 64, b.mats.tile(Mat::Concrete));
    let ring_collider = Collider::trimesh_from_mesh(&ring).expect("ring collider");
    {
        let tag = b.tag();
        let mut ec = b.commands.spawn((Transform::default(), RigidBody::Static, ring_collider, CollisionLayers::new(Layer::World, LayerMask::ALL), tag));
        if b.visuals {
            let mesh = b.meshes.add(ring);
            ec.insert((Mesh3d(mesh), MeshMaterial3d(b.mats.get(Mat::Concrete))));
        }
    }
    // Высоты чаши в сетку
    {
        let g = &mut b.data.grid;
        for j in 0..g.nz {
            for i in 0..g.nx {
                let p = g.pos(i, j);
                let r = p.length();
                if r > FLOOR_R && r <= BANK_R {
                    g.height[j * g.nx + i] = (r - FLOOR_R) / (BANK_R - FLOOR_R) * BANK_H;
                }
            }
        }
    }
    // Вертикальная стена над чашей (сегменты)
    let segs = 48;
    for s in 0..segs {
        let a = (s as f32 + 0.5) / segs as f32 * std::f32::consts::TAU;
        let r = BANK_R + 0.8;
        let p = Vec3::new(a.cos() * r, BANK_H + 1.5, a.sin() * r);
        let len = BANK_R * std::f32::consts::TAU / segs as f32 + 0.4;
        let yaw = -a;
        b.solid(p, Vec3::new(1.6, 3.0, len), yaw, Mat::Concrete);
        if s % 2 == 0 {
            b.deco(p + Vec3::Y * 1.7, Vec3::new(1.7, 0.4, len), yaw, Mat::Hazard);
        }
    }
    // Всё снаружи стены вне игры
    {
        let g = &mut b.data.grid;
        for j in 0..g.nz {
            for i in 0..g.nx {
                if g.pos(i, j).length() > BANK_R + 0.5 {
                    g.blocked[j * g.nx + i] = true;
                }
            }
        }
    }

    // Трибуны в углах и по сторонам за стеной: ступени с пешеходами-зрителями
    for k in 0..4 {
        let a = k as f32 * std::f32::consts::FRAC_PI_2 + std::f32::consts::FRAC_PI_4;
        let dir = Vec2::new(a.cos(), a.sin());
        for step in 0..5 {
            let r = BANK_R + 4.0 + step as f32 * 2.2;
            let c = dir * (r + 2.0);
            let h = BANK_H + 1.0 + step as f32 * 1.4;
            b.static_box(Vec3::new(c.x, h * 0.5, c.y), Vec3::new(22.0, h, 2.4), -a + std::f32::consts::FRAC_PI_2, Mat::Concrete, true);
        }
        let zc = dir * (BANK_R + 9.0);
        // Зрители стоят на ступенях: зона в сетке поднята до высоты трибуны
        let zmin = zc - Vec2::splat(6.0);
        let zmax = zc + Vec2::splat(6.0);
        b.ped_zone(zmin, zmax);
        {
            let g = &mut b.data.grid;
            for j in 0..g.nz {
                for i in 0..g.nx {
                    let p = g.pos(i, j);
                    if p.x >= zmin.x && p.x <= zmax.x && p.y >= zmin.y && p.y <= zmax.y {
                        let r = p.length();
                        let step = ((r - BANK_R - 4.0) / 2.2).clamp(0.0, 4.0).floor();
                        let k2 = j * g.nx + i;
                        g.height[k2] = BANK_H + 1.0 + step * 1.4;
                        g.blocked[k2] = false;
                    }
                }
            }
        }
        // Прожекторная мачта
        let mast = dir * (BANK_R + 16.0);
        b.solid(Vec3::new(mast.x, 14.0, mast.y), Vec3::new(1.0, 28.0, 1.0), 0.0, Mat::Metal);
        b.deco(Vec3::new(mast.x, 28.5, mast.y), Vec3::new(5.0, 2.5, 1.0), -a + std::f32::consts::FRAC_PI_2, Mat::Lamp);
        b.spot_light(Vec3::new(mast.x, 27.5, mast.y), Vec3::new(-mast.x * 0.25, 0.0, -mast.y * 0.25), Color::srgb(1.0, 0.78, 0.5), 5_500_000.0, 170.0, 0.62, k % 2 == 0);
    }
    racing_dressing(b);
    // Вспомогательный свет по кругу
    for k in 0..4 {
        let a = k as f32 * std::f32::consts::FRAC_PI_2;
        b.point_light(Vec3::new(a.cos() * 40.0, 14.0, a.sin() * 40.0), Color::srgb(1.0, 0.62, 0.32), 300_000.0, 50.0);
    }

    // Мясорубка в центре: статичная ступица и вращающиеся лопасти
    b.cylinder(Vec3::new(0.0, 1.5, 0.0), 3.2, 3.0, Mat::Hazard, true);
    b.deco(Vec3::new(0.0, 3.4, 0.0), Vec3::new(2.0, 0.8, 2.0), 0.0, Mat::Neon1);
    let blade_len = 48.0;
    let mut parts = Vec::new();
    let mut shapes = Vec::new();
    for k in 0..2 {
        let rot = Quat::from_rotation_y(k as f32 * std::f32::consts::FRAC_PI_2);
        shapes.push((Vec3::new(0.0, 0.0, 0.0), rot, Collider::cuboid(1.6, 1.4, blade_len)));
        parts.push((box_mesh(Vec3::new(1.6, 1.4, blade_len), 2.0), Mat::Hazard, Transform::from_rotation(rot)));
        for side in [-1.0f32, 1.0] {
            for t in [8.0f32, 14.0, 20.0] {
                let local = rot * Vec3::new(side * 0.9, 0.2, t);
                parts.push((Cone { radius: 0.35, height: 1.2 }.mesh().resolution(8).build(), Mat::Metal,
                    Transform::from_translation(local).with_rotation(rot * Quat::from_rotation_z(-side * std::f32::consts::FRAC_PI_2))));
                let local2 = rot * Vec3::new(side * 0.9, 0.2, -t);
                parts.push((Cone { radius: 0.35, height: 1.2 }.mesh().resolution(8).build(), Mat::Metal,
                    Transform::from_translation(local2).with_rotation(rot * Quat::from_rotation_z(-side * std::f32::consts::FRAC_PI_2))));
            }
        }
    }
    b.mover(MoverPath::Spin { center: Vec3::new(0.0, 0.9, 0.0), rate: 0.55 }, Collider::compound(shapes), parts, "Мясорубка");

    // Трамплины по кругу (за пределом лопастей)
    for k in 0..4 {
        let a = k as f32 * std::f32::consts::FRAC_PI_2 + 0.4;
        let p = Vec3::new(a.cos() * 40.0, 0.0, a.sin() * 40.0);
        let tangent = Vec3::new(-a.sin(), 0.0, a.cos());
        let yaw = (-tangent.x).atan2(-tangent.z) + std::f32::consts::PI;
        b.ramp(p, 7.0, 10.0, 2.6, yaw, Mat::Ramp);
    }
    // Пирамидки-кочки
    for k in 0..4 {
        let a = k as f32 * std::f32::consts::FRAC_PI_2 + 1.2;
        let p = Vec3::new(a.cos() * 48.0, 0.0, a.sin() * 48.0);
        let yaw = a;
        b.ramp(p + Quat::from_rotation_y(yaw) * Vec3::new(0.0, 0.0, -3.0), 6.0, 6.0, 1.2, yaw, Mat::Concrete);
        b.ramp(p + Quat::from_rotation_y(yaw) * Vec3::new(0.0, 0.0, 3.0), 6.0, 6.0, 1.2, yaw + std::f32::consts::PI, Mat::Concrete);
    }

    // Реквизит
    for k in 0..6 {
        let a = k as f32 / 6.0 * std::f32::consts::TAU + 0.2;
        let kind = if k % 2 == 0 { PropKind::ExplosiveBarrel } else { PropKind::Tires };
        prop_cluster(b, Vec3::new(a.cos() * 30.0, 0.0, a.sin() * 30.0), kind, 3, 1.5);
    }
    // Пешеходы на арене
    b.ped_zone(Vec2::splat(-45.0), Vec2::splat(45.0));

    for k in 0..8 {
        let a = k as f32 / 8.0 * std::f32::consts::TAU;
        b.pickup(Vec3::new(a.cos() * 52.0, 0.0, a.sin() * 52.0));
    }
    b.pickup(Vec3::new(0.0, 0.0, 30.0));
    b.pickup(Vec3::new(0.0, 0.0, -30.0));
    for k in 0..4 {
        let a = k as f32 * std::f32::consts::FRAC_PI_2 + std::f32::consts::FRAC_PI_4;
        b.checkpoint(Vec3::new(a.cos() * 44.0, 0.0, a.sin() * 44.0));
    }
    checkpoint_visuals(b);
    ring_spawns(b, Vec3::ZERO, 55.0, 12);
}

/// Racing Kit (CC0) за стеной: крытые трибуны по сторонам, рекламные щиты с плакатами
/// и флаги на стене. Только декор: снаружи стены зона вне игры.
fn racing_dressing(b: &mut MapBuilder) {
    if !b.visuals {
        return;
    }
    // фасад моделей Racing Kit смотрит в +Z
    let facing = |d: Vec2| d.x.atan2(d.y);
    for k in 0..4 {
        let a0 = k as f32 * std::f32::consts::FRAC_PI_2;
        for j in -2..=2 {
            let a = a0 + j as f32 * 0.115;
            let out = Vec2::new(a.cos(), a.sin());
            let p = out * (BANK_R + 9.0);
            let name = if j == 0 { "racing/grandStandAwning" } else { "racing/grandStandCovered" };
            b.kit(name, Vec3::new(p.x, 0.0, p.y), facing(-out), 10.5, false);
        }
    }
    // щиты над стеной с плакатами
    for k in 0..8 {
        let a = k as f32 / 8.0 * std::f32::consts::TAU + std::f32::consts::FRAC_PI_4 * 0.5;
        let out = Vec2::new(a.cos(), a.sin());
        let p = out * (BANK_R + 2.6);
        let base = Vec3::new(p.x, BANK_H + 3.0, p.y);
        let size = b.kit("racing/billboard", base, facing(-out), 8.0, false);
        if size != Vec3::ZERO {
            let face = base - Vec3::new(out.x, 0.0, out.y) * (size.z * 0.5 + 0.08) + Vec3::Y * size.y * 0.62;
            wall_art(b, face, facing(-out), size.y * 0.55, false);
        }
    }
    // флаги и баннерные башни по кромке стены
    for k in 0..16 {
        let a = (k as f32 + 0.5) / 16.0 * std::f32::consts::TAU;
        let out = Vec2::new(a.cos(), a.sin());
        let p = out * (BANK_R + 1.2);
        let name = match k % 4 {
            0 => "racing/bannerTowerRed",
            2 => "racing/bannerTowerGreen",
            _ => "racing/flagCheckers",
        };
        b.kit(name, Vec3::new(p.x, BANK_H + 3.0, p.y), facing(-out), 6.0, false);
    }
}
