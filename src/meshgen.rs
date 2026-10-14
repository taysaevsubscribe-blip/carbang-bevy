//! Процедурные меши и текстуры: коробки с UV по размеру, клинья-трамплины, рельеф, детали машин.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

pub fn mesh_from(positions: Vec<[f32; 3]>, normals: Vec<[f32; 3]>, uvs: Vec<[f32; 2]>, indices: Vec<u32>) -> Mesh {
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

/// Коробка с UV, растянутыми по реальному размеру (tile метров на повтор текстуры).
pub fn box_mesh(size: Vec3, tile: f32) -> Mesh {
    let h = size * 0.5;
    let mut p = Vec::new();
    let mut n = Vec::new();
    let mut uv = Vec::new();
    let mut idx = Vec::new();
    // (нормаль, ось u, ось v, размер u, размер v)
    let faces: [(Vec3, Vec3, Vec3, f32, f32); 6] = [
        (Vec3::X, Vec3::NEG_Z, Vec3::Y, size.z, size.y),
        (Vec3::NEG_X, Vec3::Z, Vec3::Y, size.z, size.y),
        (Vec3::Y, Vec3::X, Vec3::NEG_Z, size.x, size.z),
        (Vec3::NEG_Y, Vec3::X, Vec3::Z, size.x, size.z),
        (Vec3::Z, Vec3::X, Vec3::Y, size.x, size.y),
        (Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y, size.x, size.y),
    ];
    for (normal, u, v, su, sv) in faces {
        let base = p.len() as u32;
        let c = normal * (h * normal.abs()).length();
        let hu = u * (h * u.abs()).length();
        let hv = v * (h * v.abs()).length();
        let corners = [c - hu - hv, c + hu - hv, c + hu + hv, c - hu + hv];
        let uvs = [[0.0, sv / tile], [su / tile, sv / tile], [su / tile, 0.0], [0.0, 0.0]];
        for k in 0..4 {
            p.push(corners[k].to_array());
            n.push(normal.to_array());
            uv.push(uvs[k]);
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    mesh_from(p, n, uv, idx)
}

/// Клин-трамплин: низкий край на -Z (въезд), высокий на +Z (обрыв). Основание на y=0.
pub fn wedge_mesh(width: f32, length: f32, height: f32, tile: f32) -> Mesh {
    let hw = width * 0.5;
    let hl = length * 0.5;
    let v = [
        Vec3::new(-hw, 0.0, -hl),
        Vec3::new(hw, 0.0, -hl),
        Vec3::new(hw, 0.0, hl),
        Vec3::new(-hw, 0.0, hl),
        Vec3::new(-hw, height, hl),
        Vec3::new(hw, height, hl),
    ];
    let mut p = Vec::new();
    let mut n = Vec::new();
    let mut uv = Vec::new();
    let mut idx = Vec::new();
    let slope_len = (length * length + height * height).sqrt();
    let add_quad = |pts: [Vec3; 4], su: f32, sv: f32, p: &mut Vec<[f32; 3]>, n: &mut Vec<[f32; 3]>, uv: &mut Vec<[f32; 2]>, idx: &mut Vec<u32>| {
        let nn = (pts[1] - pts[0]).cross(pts[2] - pts[0]).normalize_or_zero();
        let base = p.len() as u32;
        let uvs = [[0.0, sv], [su, sv], [su, 0.0], [0.0, 0.0]];
        for k in 0..4 {
            p.push(pts[k].to_array());
            n.push(nn.to_array());
            uv.push([uvs[k][0] / tile, uvs[k][1] / tile]);
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    };
    let add_tri = |pts: [Vec3; 3], p: &mut Vec<[f32; 3]>, n: &mut Vec<[f32; 3]>, uv: &mut Vec<[f32; 2]>, idx: &mut Vec<u32>| {
        let nn = (pts[1] - pts[0]).cross(pts[2] - pts[0]).normalize_or_zero();
        let base = p.len() as u32;
        for (k, q) in pts.iter().enumerate() {
            p.push(q.to_array());
            n.push(nn.to_array());
            uv.push([[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]][k]);
        }
        idx.extend_from_slice(&[base, base + 1, base + 2]);
    };
    // скат (смотрит вверх и на -Z)
    add_quad([v[0], v[4], v[5], v[1]], width, slope_len, &mut p, &mut n, &mut uv, &mut idx);
    // задняя стенка (+Z)
    add_quad([v[3], v[2], v[5], v[4]], width, height, &mut p, &mut n, &mut uv, &mut idx);
    // дно
    add_quad([v[0], v[1], v[2], v[3]], width, length, &mut p, &mut n, &mut uv, &mut idx);
    // боковины
    add_tri([v[0], v[3], v[4]], &mut p, &mut n, &mut uv, &mut idx);
    add_tri([v[1], v[5], v[2]], &mut p, &mut n, &mut uv, &mut idx);
    mesh_from(p, n, uv, idx)
}

pub fn wedge_points(width: f32, length: f32, height: f32) -> Vec<Vec3> {
    let hw = width * 0.5;
    let hl = length * 0.5;
    vec![
        Vec3::new(-hw, 0.0, -hl),
        Vec3::new(hw, 0.0, -hl),
        Vec3::new(hw, 0.0, hl),
        Vec3::new(-hw, 0.0, hl),
        Vec3::new(-hw, height, hl),
        Vec3::new(hw, height, hl),
    ]
}

/// Сетка рельефа: heights[z][x], шаг step, центр в начале координат.
pub fn terrain_mesh(heights: &[Vec<f32>], step: f32, tile: f32) -> Mesh {
    let nz = heights.len();
    let nx = heights[0].len();
    let ox = -(nx as f32 - 1.0) * step * 0.5;
    let oz = -(nz as f32 - 1.0) * step * 0.5;
    let mut p = Vec::with_capacity(nx * nz);
    let mut uv = Vec::with_capacity(nx * nz);
    for z in 0..nz {
        for x in 0..nx {
            let wx = ox + x as f32 * step;
            let wz = oz + z as f32 * step;
            p.push([wx, heights[z][x], wz]);
            uv.push([wx / tile, wz / tile]);
        }
    }
    let mut idx = Vec::with_capacity((nx - 1) * (nz - 1) * 6);
    for z in 0..nz - 1 {
        for x in 0..nx - 1 {
            let i = (z * nx + x) as u32;
            let r = i + nx as u32;
            idx.extend_from_slice(&[i, r, i + 1, i + 1, r, r + 1]);
        }
    }
    let mut n = vec![[0.0f32; 3]; p.len()];
    for t in idx.chunks(3) {
        let a = Vec3::from(p[t[0] as usize]);
        let b = Vec3::from(p[t[1] as usize]);
        let c = Vec3::from(p[t[2] as usize]);
        let fnorm = (b - a).cross(c - a);
        for &k in t {
            let cur = Vec3::from(n[k as usize]) + fnorm;
            n[k as usize] = cur.to_array();
        }
    }
    for v in n.iter_mut() {
        let nn = Vec3::from(*v).normalize_or_zero();
        *v = if nn.y < 0.0 { (-nn).to_array() } else { nn.to_array() };
    }
    mesh_from(p, n, uv, idx)
}

/// Кольцевой сектор (для чаши стадиона): наклонная полоса между радиусами.
pub fn ring_mesh(r0: f32, r1: f32, h0: f32, h1: f32, segments: u32, tile: f32) -> Mesh {
    let mut p = Vec::new();
    let mut n = Vec::new();
    let mut uv = Vec::new();
    let mut idx = Vec::new();
    for s in 0..=segments {
        let a = s as f32 / segments as f32 * std::f32::consts::TAU;
        let (sa, ca) = a.sin_cos();
        let inner = Vec3::new(ca * r0, h0, sa * r0);
        let outer = Vec3::new(ca * r1, h1, sa * r1);
        let radial = Vec3::new(ca, 0.0, sa);
        let slope = Vec2::new(r1 - r0, h1 - h0).normalize();
        let normal = (Vec3::Y * slope.x - radial * slope.y).normalize();
        p.push(inner.to_array());
        p.push(outer.to_array());
        n.push(normal.to_array());
        n.push(normal.to_array());
        let arc = a * r1 / tile;
        uv.push([arc, 0.0]);
        uv.push([arc, (r1 - r0) / tile]);
    }
    for s in 0..segments {
        let i = s * 2;
        idx.extend_from_slice(&[i, i + 2, i + 1, i + 1, i + 2, i + 3]);
    }
    mesh_from(p, n, uv, idx)
}

// ============================================================ процедурные текстуры

fn hash(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374761393) ^ (y as u32).wrapping_mul(668265263) ^ seed.wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    (h & 0xffff) as f32 / 65535.0
}

fn value_noise(x: f32, y: f32, period: i32, seed: u32) -> f32 {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let w = |a: i32| a.rem_euclid(period);
    let a = hash(w(x0), w(y0), seed);
    let b = hash(w(x0 + 1), w(y0), seed);
    let c = hash(w(x0), w(y0 + 1), seed);
    let d = hash(w(x0 + 1), w(y0 + 1), seed);
    let ab = a + (b - a) * sx;
    let cd = c + (d - c) * sx;
    ab + (cd - ab) * sy
}

/// Бесшовный фрактальный шум 0..1
pub fn fbm(u: f32, v: f32, base_period: i32, octaves: u32, seed: u32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 0.5;
    let mut norm = 0.0;
    let mut period = base_period;
    for o in 0..octaves {
        sum += amp * value_noise(u * period as f32, v * period as f32, period, seed + o * 17);
        norm += amp;
        amp *= 0.5;
        period *= 2;
    }
    sum / norm
}

#[derive(Clone, Copy)]
pub enum TexKind {
    Asphalt,
    Concrete,
    Rust,
    Dirt,
    Grass,
    Sand,
    Container,
    Wood,
    Hazard,
    Facade,
    FacadeEmissive,
    Water,
    Metal,
    Tiles,
    Brick,
    Rock,
}

pub fn make_texture(kind: TexKind, size: u32, seed: u32) -> Image {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let u = x as f32 / size as f32;
            let v = y as f32 / size as f32;
            let n1 = fbm(u, v, 8, 4, seed);
            let n2 = fbm(u, v, 32, 2, seed + 101);
            let speck = hash(x as i32, y as i32, seed + 7);
            let c: [f32; 4] = match kind {
                TexKind::Asphalt => {
                    // зерно, заплатки свежего битума и тонкие трещины
                    let patch = ((fbm(u, v, 3, 3, seed + 40) - 0.62) * 8.0).clamp(0.0, 1.0);
                    let crack = 1.0 - ((fbm(u, v, 4, 5, seed + 50) - 0.5).abs() * 70.0).clamp(0.0, 1.0);
                    let g = 0.15 + n1 * 0.07 + if speck > 0.965 { 0.11 } else { 0.0 } + n2 * 0.05 - patch * 0.06 - crack * 0.08;
                    [g, g * 1.0, g * 1.05, 1.0]
                }
                TexKind::Concrete => {
                    let seam = if x % (size / 2) < 2 || y % (size / 2) < 2 { -0.12 } else { 0.0 };
                    let g = 0.55 + n1 * 0.18 + n2 * 0.05 + seam;
                    [g, g * 0.98, g * 0.95, 1.0]
                }
                TexKind::Rust => {
                    let r = n1 * 0.6 + n2 * 0.4;
                    let spot = fbm(u, v, 4, 3, seed + 5);
                    if spot > 0.55 {
                        [0.45 + r * 0.25, 0.22 + r * 0.12, 0.08 + r * 0.05, 1.0]
                    } else {
                        [0.32 + r * 0.15, 0.28 + r * 0.12, 0.26 + r * 0.1, 1.0]
                    }
                }
                TexKind::Dirt => {
                    let g = 0.3 + n1 * 0.25 + n2 * 0.08;
                    [g * 1.25, g * 0.95, g * 0.62, 1.0]
                }
                TexKind::Grass => {
                    let g = 0.25 + n1 * 0.2 + speck * 0.08;
                    [g * 0.55, g * 1.05, g * 0.35, 1.0]
                }
                TexKind::Sand => {
                    let g = 0.6 + n1 * 0.2 + n2 * 0.06;
                    [g * 1.05, g * 0.88, g * 0.62, 1.0]
                }
                TexKind::Container => {
                    let rib = ((u * 24.0).fract() - 0.5).abs() * 2.0;
                    let g = 0.75 + rib * 0.25 - n1 * 0.15 - if fbm(u, v, 6, 3, seed + 3) > 0.62 { 0.25 } else { 0.0 };
                    [g, g, g, 1.0]
                }
                TexKind::Wood => {
                    let plank = ((v * 6.0).fract() < 0.04) as i32 as f32;
                    let grain = fbm(u * 0.2, v * 3.0, 8, 3, seed);
                    let g = 0.45 + grain * 0.3 - plank * 0.25;
                    [g * 1.1, g * 0.8, g * 0.5, 1.0]
                }
                TexKind::Hazard => {
                    let s = (((u + v) * 8.0).fract() < 0.5) as i32 as f32;
                    let wear = n1 * 0.2;
                    if s > 0.5 {
                        [0.95 - wear, 0.75 - wear, 0.1, 1.0]
                    } else {
                        [0.08 + wear * 0.3, 0.08 + wear * 0.3, 0.08, 1.0]
                    }
                }
                TexKind::Facade | TexKind::FacadeEmissive => {
                    let cx = (u * 8.0).fract();
                    let cy = (v * 8.0).fract();
                    let win = cx > 0.18 && cx < 0.82 && cy > 0.22 && cy < 0.78;
                    let lit = hash((u * 8.0) as i32, (v * 8.0) as i32, seed + 11) > 0.55;
                    if matches!(kind, TexKind::FacadeEmissive) {
                        if win && lit {
                            [1.0, 0.82, 0.5, 1.0]
                        } else {
                            [0.0, 0.0, 0.0, 1.0]
                        }
                    } else if win {
                        [0.18, 0.22, 0.28, 1.0]
                    } else {
                        let g = 0.82 + n2 * 0.18;
                        [g, g, g, 1.0]
                    }
                }
                TexKind::Water => {
                    let g = n1 * 0.5 + n2 * 0.5;
                    [0.05 + g * 0.08, 0.18 + g * 0.12, 0.25 + g * 0.12, 1.0]
                }
                TexKind::Metal => {
                    let g = 0.6 + n2 * 0.15 + (u * 40.0).sin() * 0.02;
                    [g, g, g * 1.02, 1.0]
                }
                TexKind::Tiles => {
                    let line = (u * 8.0).fract() < 0.05 || (v * 8.0).fract() < 0.05;
                    let g = if line { 0.35 } else { 0.62 + n1 * 0.12 };
                    [g, g * 0.97, g * 0.92, 1.0]
                }
                TexKind::Brick => {
                    let row = (v * 16.0) as i32;
                    let off = if row % 2 == 0 { 0.0 } else { 0.5 };
                    let mortar = (v * 16.0).fract() < 0.12 || ((u * 8.0 + off).fract() < 0.06);
                    let g = n2 * 0.15;
                    if mortar {
                        [0.6, 0.58, 0.55, 1.0]
                    } else {
                        [0.55 + g, 0.25 + g * 0.5, 0.18, 1.0]
                    }
                }
                TexKind::Rock => {
                    let g = 0.35 + n1 * 0.3 + n2 * 0.1;
                    let strata = ((v * 12.0 + n1 * 2.0).fract() < 0.1) as i32 as f32 * 0.08;
                    [g * 1.1 - strata, g * 0.95 - strata, g * 0.8 - strata, 1.0]
                }
            };
            for ch in c {
                data.push((ch.clamp(0.0, 1.0) * 255.0) as u8);
            }
        }
    }
    let mut img = Image::new(
        Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 8,
        ..default()
    });
    img
}

/// Мягкое пятно (кровь, масло, дым) с альфой.
pub fn blob_texture(size: u32, seed: u32, splat: bool) -> Image {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    let blobs: Vec<(f32, f32, f32)> = (0..if splat { 9 } else { 1 })
        .map(|i| {
            if splat {
                (
                    0.3 + hash(i, 1, seed) * 0.4,
                    0.3 + hash(i, 2, seed) * 0.4,
                    0.06 + hash(i, 3, seed) * 0.16,
                )
            } else {
                (0.5, 0.5, 0.45)
            }
        })
        .collect();
    for y in 0..size {
        for x in 0..size {
            let u = x as f32 / size as f32;
            let v = y as f32 / size as f32;
            let mut a: f32 = 0.0;
            for (cx, cy, r) in &blobs {
                let d = ((u - cx).powi(2) + (v - cy).powi(2)).sqrt();
                let edge = if splat { 0.015 } else { *r };
                a = a.max(((r - d) / edge).clamp(0.0, 1.0));
            }
            if !splat {
                a = a * a;
            }
            data.extend_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
        }
    }
    Image::new(
        Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}
