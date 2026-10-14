//! «Ржавый рай»: свалка-лабиринт из сплющенных машин, пресс, хафпайп, грязевые ямы.

use super::builders::*;
use super::*;

const CELLS: i32 = 5;
const CELL: f32 = 42.0;

pub fn build(b: &mut MapBuilder) {
    flat_ground(b, Mat::Dirt);
    let origin = -(CELLS as f32) * CELL * 0.5;
    let cell_center = |i: i32, j: i32| Vec2::new(origin + (i as f32 + 0.5) * CELL, origin + (j as f32 + 0.5) * CELL);

    // Лабиринт: случайный обход в глубину по клеткам + лишние проходы
    let n = (CELLS * CELLS) as usize;
    let mut visited = vec![false; n];
    // стены: (i,j,dir) dir 0 = восточная, 1 = южная
    let mut open_e = vec![false; n];
    let mut open_s = vec![false; n];
    let mut stack = vec![(2i32, 2i32)];
    visited[(2 * CELLS + 2) as usize] = true;
    while let Some(&(i, j)) = stack.last() {
        let mut nbs = Vec::new();
        for (di, dj) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (ni, nj) = (i + di, j + dj);
            if ni >= 0 && nj >= 0 && ni < CELLS && nj < CELLS && !visited[(nj * CELLS + ni) as usize] {
                nbs.push((ni, nj, di, dj));
            }
        }
        if nbs.is_empty() {
            stack.pop();
            continue;
        }
        let (ni, nj, di, dj) = nbs[b.rng.gen_range(0..nbs.len())];
        if di == 1 {
            open_e[(j * CELLS + i) as usize] = true;
        } else if di == -1 {
            open_e[(nj * CELLS + ni) as usize] = true;
        } else if dj == 1 {
            open_s[(j * CELLS + i) as usize] = true;
        } else {
            open_s[(nj * CELLS + ni) as usize] = true;
        }
        visited[(nj * CELLS + ni) as usize] = true;
        stack.push((ni, nj));
    }
    for k in 0..n {
        if b.chance(0.45) {
            open_e[k] = true;
        }
        if b.chance(0.45) {
            open_s[k] = true;
        }
    }
    // центр всегда открыт со всех сторон
    for (i, j) in [(1, 2), (2, 2), (2, 1)] {
        open_e[(j * CELLS + i) as usize] = true;
        open_s[(j * CELLS + i) as usize] = true;
    }

    let block_mesh = if b.visuals { Some(b.meshes.add(box_mesh(Vec3::new(2.1, 1.0, 4.4), 2.0))) } else { None };
    let block_mats: Vec<Handle<StandardMaterial>> = if b.visuals {
        [Color::srgb(0.55, 0.32, 0.2), Color::srgb(0.35, 0.38, 0.42), Color::srgb(0.5, 0.15, 0.12), Color::srgb(0.25, 0.35, 0.5), Color::srgb(0.6, 0.55, 0.3)]
            .iter()
            .map(|c| {
                let rust = b.mats.get(Mat::Rust);
                let tex = b.std_mats.get(&rust).and_then(|m| m.base_color_texture.clone());
                b.std_mats.add(StandardMaterial {
                    base_color: *c,
                    base_color_texture: tex,
                    perceptual_roughness: 0.8,
                    metallic: 0.3,
                    ..default()
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    let wall = |b: &mut MapBuilder, a: Vec2, c: Vec2| {
        let d = c - a;
        let len = d.length();
        let dir = d / len;
        let yaw = (-d.x).atan2(-d.y);
        let levels = b.rng.gen_range(2..4);
        let height = levels as f32 * 1.0;
        let center = (a + c) * 0.5;
        b.register_box(Vec3::new(center.x, height * 0.5, center.y), Vec3::new(2.2, height, len), yaw, Mat::Rust);
        let tag = b.tag();
        b.commands.spawn((
            Transform::from_xyz(center.x, height * 0.5, center.y).with_rotation(Quat::from_rotation_y(yaw)),
            RigidBody::Static,
            Collider::cuboid(2.2, height, len),
            CollisionLayers::new(Layer::World, LayerMask::ALL),
            tag,
        ));
        if let Some(mesh) = block_mesh.clone() {
            let count = (len / 2.2) as i32;
            for l in 0..levels {
                for k in 0..count {
                    let p = a + dir * (k as f32 * 2.2 + 1.1);
                    let mat = block_mats[b.rng.gen_range(0..block_mats.len())].clone();
                    let jitter = b.rand(-0.15, 0.15);
                    let tag = b.tag();
                    let dyaw = b.rand(-0.12, 0.12);
                    b.commands.spawn((
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(mat),
                        Transform::from_xyz(p.x + jitter, 0.5 + l as f32 * 1.0, p.y)
                            .with_rotation(Quat::from_rotation_y(yaw + std::f32::consts::FRAC_PI_2 + dyaw)),
                        tag,
                    ));
                }
            }
        }
    };

    let gap = 11.0;
    for j in 0..CELLS {
        for i in 0..CELLS {
            let c = cell_center(i, j);
            let k = (j * CELLS + i) as usize;
            if i < CELLS - 1 && !open_e[k] {
                let x = c.x + CELL * 0.5;
                wall(b, Vec2::new(x, c.y - CELL * 0.5 + gap * 0.5), Vec2::new(x, c.y + CELL * 0.5 - gap * 0.5));
            }
            if j < CELLS - 1 && !open_s[k] {
                let z = c.y + CELL * 0.5;
                wall(b, Vec2::new(c.x - CELL * 0.5 + gap * 0.5, z), Vec2::new(c.x + CELL * 0.5 - gap * 0.5, z));
            }
        }
    }

    // Пресс в центре
    let press_c = cell_center(2, 2);
    b.deco(Vec3::new(press_c.x, 0.02, press_c.y), Vec3::new(14.0, 0.03, 14.0), 0.0, Mat::Hazard);
    for (dx, dz) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        b.solid(Vec3::new(press_c.x + dx * 6.2, 6.0, press_c.y + dz * 6.2), Vec3::new(1.2, 12.0, 1.2), 0.0, Mat::ContainerYellow);
    }
    b.static_box(Vec3::new(press_c.x, 12.6, press_c.y), Vec3::new(14.0, 1.2, 14.0), 0.0, Mat::ContainerYellow, true);
    b.mover(
        MoverPath::Press { base: Vec3::new(press_c.x, 1.1, press_c.y), lift: 9.0, period: 7.0 },
        Collider::cuboid(9.0, 2.2, 9.0),
        vec![(box_mesh(Vec3::new(9.0, 2.2, 9.0), 2.0), Mat::Metal, Transform::default())],
        "Пресс",
    );
    b.pickup(Vec3::new(press_c.x, 0.0, press_c.y));

    // Хафпайп на востоке
    let hp = cell_center(4, 2);
    halfpipe(b, hp);

    // Грязевые ямы на западе
    for (i, j) in [(0, 1), (0, 3), (1, 0)] {
        let c = cell_center(i, j);
        let s = Vec2::new(20.0, 16.0);
        b.deco(Vec3::new(c.x, 0.02, c.y), Vec3::new(s.x, 0.03, s.y), 0.0, Mat::Mud);
        b.surface(c - s * 0.5, c + s * 0.5, 0.55);
    }

    // Кучи хлама-трамплины
    for (i, j, yaw) in [(1, 1, 0.6f32), (3, 3, 2.4), (3, 1, -0.8), (1, 3, 3.6)] {
        let c = cell_center(i, j);
        b.ramp(Vec3::new(c.x, 0.0, c.y), 8.0, 11.0, 2.8, yaw, Mat::Rust);
    }

    // Стены из шин по углам, реквизит
    for (i, j) in [(0, 0), (4, 4), (0, 4), (4, 0)] {
        let c = cell_center(i, j);
        tire_wall(b, c + Vec2::new(-8.0, 6.0), c + Vec2::new(8.0, 6.0), 3);
        prop_cluster(b, Vec3::new(c.x, 0.0, c.y - 6.0), PropKind::Tires, 6, 4.0);
    }
    for _ in 0..10 {
        let i = b.rng.gen_range(0..CELLS);
        let j = b.rng.gen_range(0..CELLS);
        if i == 2 && j == 2 {
            continue;
        }
        let c = cell_center(i, j);
        let kind = if b.chance(0.35) { PropKind::ExplosiveBarrel } else { PropKind::Barrel };
        let cnt = b.rng.gen_range(3..6);
        let (dx, dz) = (b.rand(-10.0, 10.0), b.rand(-10.0, 10.0));
        prop_cluster(b, Vec3::new(c.x + dx, 0.0, c.y + dz), kind, cnt, 3.0);
    }
    // Одиночные остовы для разнообразия
    for _ in 0..12 {
        let i = b.rng.gen_range(0..CELLS);
        let j = b.rng.gen_range(0..CELLS);
        if (i == 2 && j == 2) || (i == 4 && j == 2) {
            continue;
        }
        let c = cell_center(i, j);
        let p = Vec3::new(c.x + b.rand(-12.0, 12.0), 0.0, c.y + b.rand(-12.0, 12.0));
        let flat = b.chance(0.5);
        let yaw = b.rand(0.0, 6.28);
        wreck(b, p, yaw, flat);
    }

    // Пешеходы-сборщики металла по всей свалке
    b.ped_zone(Vec2::splat(origin + 4.0), Vec2::splat(-origin - 4.0));

    // Бонусы: по одному в большинстве клеток
    for j in 0..CELLS {
        for i in 0..CELLS {
            if (i + j) % 2 == 1 || (i == 0 && j == 0) {
                let c = cell_center(i, j);
                b.pickup(Vec3::new(c.x + 8.0, 0.0, c.y - 8.0));
            }
        }
    }

    for (i, j) in [(0, 0), (4, 0), (4, 4), (0, 4), (2, 0)] {
        let c = cell_center(i, j);
        b.checkpoint(Vec3::new(c.x, 0.0, c.y));
    }
    checkpoint_visuals(b);

    for j in [0, 4] {
        for i in 0..CELLS {
            let c = cell_center(i, j);
            let yaw = if j == 0 { std::f32::consts::PI } else { 0.0 };
            b.spawn_point(Vec3::new(c.x - 6.0, 0.0, c.y), yaw);
            b.spawn_point(Vec3::new(c.x + 6.0, 0.0, c.y), yaw);
        }
    }
    for j in 1..4 {
        b.spawn_point(Vec3::new(cell_center(0, j).x, 0.0, cell_center(0, j).y + 10.0), -std::f32::consts::FRAC_PI_2);
    }
    // освещение заката: пара тёплых ламп на кране
    b.street_lamp(Vec3::new(press_c.x + 9.0, 0.0, press_c.y + 9.0), true);
}

/// Хафпайп: две изогнутые стенки из наклонных плит, проезд вдоль оси Z.
fn halfpipe(b: &mut MapBuilder, c: Vec2) {
    let length = 34.0;
    // профиль: (смещение по x от центра, высота) точки ломаной
    let profile = [(6.0, 0.0), (9.0, 0.6), (11.5, 1.9), (13.3, 3.8), (14.4, 6.0)];
    for side in [-1.0f32, 1.0] {
        for w in profile.windows(2) {
            let (x0, h0) = w[0];
            let (x1, h1) = w[1];
            let start = Vec3::new(c.x + side * x0, h0, c.y);
            let end = Vec3::new(c.x + side * x1, h1, c.y);
            let d = end - start;
            let width_across = Vec2::new(d.x, d.y).length();
            let mid = (start + end) * 0.5;
            let pitch = (d.y / d.x.abs()).atan();
            let rot = Quat::from_rotation_z(side * pitch);
            let size = Vec3::new(width_across, 0.5, length);
            let tag = b.tag();
            let mut ec = b.commands.spawn((Transform::from_translation(mid - rot * Vec3::Y * 0.25).with_rotation(rot), tag));
            if b.visuals {
                let mesh = b.meshes.add(box_mesh(size, 3.0));
                ec.insert((Mesh3d(mesh), MeshMaterial3d(b.mats.get(Mat::Concrete))));
            }
            ec.insert((RigidBody::Static, Collider::cuboid(size.x, size.y, size.z), CollisionLayers::new(Layer::World, LayerMask::ALL)));
        }
        // задняя стенка-упор
        b.solid(Vec3::new(c.x + side * 15.2, 3.5, c.y), Vec3::new(1.2, 7.0, length), 0.0, Mat::Rust);
        // наклонные полосы как высоты для сетки
        b.write_height_rect(Vec2::new(c.x + side * 10.5, c.y), Vec2::new(4.0, length * 0.5), 0.0, move |lx, _| {
            let x = (lx * side + 10.5 - 6.0).max(0.0);
            (x / 8.4).powi(2) * 6.0
        });
    }
    b.deco(Vec3::new(c.x, 0.02, c.y), Vec3::new(12.0, 0.03, length), 0.0, Mat::Concrete);
}
