//! Внешние CC0-ассеты: PBR-текстуры ambientCG, HDRI Poly Haven, модели Kenney.
//! Всё необязательно: если папки assets нет, игра работает на процедурных текстурах и мешах.

use crate::gfx::add_mips;
use crate::meshgen::fbm;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

/// Папка assets: та же логика, что у загрузчика Bevy (BEVY_ASSET_ROOT, CARGO_MANIFEST_DIR, папка exe).
pub fn assets_dir() -> Option<PathBuf> {
    let mut cands = Vec::new();
    if let Ok(p) = std::env::var("BEVY_ASSET_ROOT") {
        cands.push(PathBuf::from(p).join("assets"));
    }
    if let Ok(p) = std::env::var("CARGO_MANIFEST_DIR") {
        cands.push(PathBuf::from(p).join("assets"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            cands.push(d.join("assets"));
        }
    }
    cands.push(PathBuf::from("assets"));
    cands.into_iter().find(|p| p.join("textures").is_dir() || p.join("kenney").is_dir())
}

type Raw = Arc<(u32, u32, Vec<u8>)>;

fn cache() -> &'static Mutex<HashMap<String, Option<Raw>>> {
    static C: OnceLock<Mutex<HashMap<String, Option<Raw>>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

/// JPG в RGBA8 с кэшем в памяти (между заездами файлы не перечитываются).
/// PNG из папки assets как текстура с повтором и мипами (декали, стадии износа, иконки).
/// Нет файла — понятная ошибка с путём в лог и `None`.
pub fn load_png(images: &mut Assets<Image>, rel: &str, srgb: bool, repeat: bool) -> Option<Handle<Image>> {
    use bevy::image::{CompressedImageFormats, ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor, ImageType};
    let path = assets_dir()?.join(rel);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            error!("нет ассета {}: {}", path.display(), e);
            return None;
        }
    };
    let mode = if repeat { ImageAddressMode::Repeat } else { ImageAddressMode::ClampToEdge };
    let sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: mode,
        address_mode_v: mode,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 8,
        ..default()
    });
    match Image::from_buffer(&bytes, ImageType::Extension("png"), CompressedImageFormats::NONE, srgb, sampler, bevy::asset::RenderAssetUsages::RENDER_WORLD | bevy::asset::RenderAssetUsages::MAIN_WORLD) {
        Ok(mut img) => {
            let fmt = if srgb { bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb } else { bevy::render::render_resource::TextureFormat::Rgba8Unorm };
            img.convert(fmt).map(|i| img = i);
            crate::gfx::add_mips(&mut img);
            Some(images.add(img))
        }
        Err(e) => {
            error!("не читается {}: {}", path.display(), e);
            None
        }
    }
}

/// JPG из папки assets как цветная текстура (sRGB, без повтора, с мипами).
pub fn load_jpg_image(images: &mut Assets<Image>, rel: &str) -> Option<Handle<Image>> {
    let raw = read_jpg(rel)?;
    let mut img = to_image(raw.0, raw.1, raw.2.clone(), true, false);
    add_mips(&mut img);
    Some(images.add(img))
}

pub fn read_jpg(rel: &str) -> Option<Raw> {
    if let Some(v) = cache().lock().ok()?.get(rel) {
        return v.clone();
    }
    let res = (|| {
        let path = assets_dir()?.join(rel);
        let file = std::fs::File::open(&path).ok()?;
        let mut dec = jpeg_decoder::Decoder::new(std::io::BufReader::new(file));
        let px = dec.decode().ok()?;
        let info = dec.info()?;
        let (w, h) = (info.width as u32, info.height as u32);
        let n = (w * h) as usize;
        let mut out = vec![255u8; n * 4];
        match info.pixel_format {
            jpeg_decoder::PixelFormat::L8 => {
                for i in 0..n {
                    out[i * 4] = px[i];
                    out[i * 4 + 1] = px[i];
                    out[i * 4 + 2] = px[i];
                }
            }
            jpeg_decoder::PixelFormat::L16 => {
                for i in 0..n {
                    let v = px[i * 2];
                    out[i * 4] = v;
                    out[i * 4 + 1] = v;
                    out[i * 4 + 2] = v;
                }
            }
            jpeg_decoder::PixelFormat::RGB24 => {
                for i in 0..n {
                    out[i * 4..i * 4 + 3].copy_from_slice(&px[i * 3..i * 3 + 3]);
                }
            }
            jpeg_decoder::PixelFormat::CMYK32 => {
                for i in 0..n {
                    let k = px[i * 4 + 3] as f32 / 255.0;
                    for c in 0..3 {
                        out[i * 4 + c] = ((255 - px[i * 4 + c]) as f32 * (1.0 - k)) as u8;
                    }
                }
            }
        }
        Some(Arc::new((w, h, out)))
    })();
    if res.is_none() {
        warn!("ассет не найден или не читается: {rel}");
    }
    if let Ok(mut c) = cache().lock() {
        c.insert(rel.to_string(), res.clone());
    }
    res
}

fn sampler(repeat: bool) -> ImageSampler {
    let mode = if repeat { ImageAddressMode::Repeat } else { ImageAddressMode::ClampToEdge };
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: mode,
        address_mode_v: mode,
        address_mode_w: mode,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 16,
        ..default()
    })
}

fn to_image(w: u32, h: u32, data: Vec<u8>, srgb: bool, repeat: bool) -> Image {
    let mut img = Image::new(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        if srgb { TextureFormat::Rgba8UnormSrgb } else { TextureFormat::Rgba8Unorm },
        RenderAssetUsages::RENDER_WORLD,
    );
    add_mips(&mut img);
    img.sampler = sampler(repeat);
    img
}

/// Набор карт PBR-материала.
#[derive(Clone)]
pub struct PbrMaps {
    pub color: Handle<Image>,
    pub normal: Option<Handle<Image>>,
    pub mr: Handle<Image>,
    pub ao: Option<Handle<Image>>,
}

/// Настройки «мокрости» и подмешиваний при сборке набора.
#[derive(Clone, Copy)]
pub struct PbrOpts {
    /// 0..1: общая сырость (темнее, глаже) и лужи по шуму
    pub wet: f32,
    /// Период шума луж на одну плитку текстуры
    pub puddle_period: i32,
    /// Металличность, если нет своей карты
    pub metal: f32,
    /// Множитель цвета
    pub tint: [f32; 3],
    /// Подмешать потёки Leaking012A (стены)
    pub streaks: bool,
}

impl Default for PbrOpts {
    fn default() -> Self {
        Self { wet: 0.0, puddle_period: 2, metal: 0.0, tint: [1.0, 1.0, 1.0], streaks: false }
    }
}

/// Собирает материал из набора ambientCG (`Name/Name_1K-JPG_Color.jpg` и т.д.).
pub fn load_pbr(images: &mut Assets<Image>, set: &str, o: PbrOpts) -> Option<PbrMaps> {
    let f = |kind: &str| format!("textures/{set}/{set}_1K-JPG_{kind}.jpg");
    let color = read_jpg(&f("Color"))?;
    let (w, h) = (color.0, color.1);
    let rough = read_jpg(&f("Roughness"));
    let normal = read_jpg(&f("NormalGL"));
    let ao = read_jpg(&f("AmbientOcclusion"));
    let metal = read_jpg(&f("Metalness"));
    let streak = if o.streaks { read_jpg("textures/Leaking012A/Leaking012A_1K-JPG_Roughness.jpg") } else { None };
    let n = (w * h) as usize;
    let mut col = color.2.clone();
    let mut mr = vec![255u8; n * 4];
    let mut nrm = normal.as_ref().filter(|r| r.0 == w && r.1 == h).map(|r| r.2.clone());
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            let u = x as f32 / w as f32;
            let v = y as f32 / h as f32;
            // лужи: низкочастотный бесшовный шум
            let puddle = if o.wet > 0.0 {
                let nz = fbm(u, v, o.puddle_period, 4, 1234);
                let thr = 0.71 - o.wet * 0.12;
                ((nz - thr) / 0.05).clamp(0.0, 1.0) * o.wet.min(1.0)
            } else {
                0.0
            };
            let damp = o.wet * 0.55;
            let mut dark = 1.0 - 0.32 * damp - 0.28 * puddle;
            if let Some(s) = streak.as_ref().filter(|s| s.0 == w && s.1 == h) {
                // потёки: тёмные вертикальные полосы
                let sv = s.2[i * 4] as f32 / 255.0;
                dark *= 0.72 + 0.28 * sv;
            }
            for c in 0..3 {
                col[i * 4 + c] = (col[i * 4 + c] as f32 * dark * o.tint[c]).min(255.0) as u8;
            }
            let r0 = rough.as_ref().filter(|r| r.0 == w && r.1 == h).map(|r| r.2[i * 4] as f32 / 255.0).unwrap_or(0.8);
            let r1 = r0 * (1.0 - 0.22 * damp);
            // стилизация: лужа матовая (0.3), без зеркальных бликов фото-текстуры
            let r = r1 + (0.3 - r1) * puddle;
            mr[i * 4 + 1] = (r.clamp(0.02, 1.0) * 255.0) as u8;
            let m = metal.as_ref().filter(|r| r.0 == w && r.1 == h).map(|r| r.2[i * 4] as f32 / 255.0).unwrap_or(o.metal);
            mr[i * 4 + 2] = (m * 255.0) as u8;
            // в лужах вода заливает рельеф: нормаль выравнивается
            if let Some(nm) = nrm.as_mut() {
                if puddle > 0.0 {
                    let k = puddle;
                    nm[i * 4] = (nm[i * 4] as f32 * (1.0 - k) + 128.0 * k) as u8;
                    nm[i * 4 + 1] = (nm[i * 4 + 1] as f32 * (1.0 - k) + 128.0 * k) as u8;
                    nm[i * 4 + 2] = (nm[i * 4 + 2] as f32 * (1.0 - k) + 255.0 * k) as u8;
                }
            }
        }
    }
    Some(PbrMaps {
        color: images.add(to_image(w, h, col, true, true)),
        normal: nrm.map(|d| images.add(to_image(w, h, d, false, true))),
        mr: images.add(to_image(w, h, mr, false, true)),
        ao: ao.filter(|a| a.0 == w && a.1 == h).map(|a| images.add(to_image(w, h, a.2.clone(), false, true))),
    })
}

pub fn pbr_material(maps: &PbrMaps) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(maps.color.clone()),
        normal_map_texture: maps.normal.clone(),
        metallic_roughness_texture: Some(maps.mr.clone()),
        occlusion_texture: maps.ao.clone(),
        perceptual_roughness: 1.0,
        metallic: 1.0,
        reflectance: 0.5,
        ..default()
    }
}

/// Декаль: цвет + альфа из карты Opacity (или из яркости/шероховатости).
pub fn load_decal(images: &mut Assets<Image>, set: &str, color_override: Option<[u8; 3]>) -> Option<(Handle<Image>, Option<Handle<Image>>)> {
    let f = |kind: &str| format!("textures/{set}/{set}_1K-JPG_{kind}.jpg");
    let color = read_jpg(&f("Color"))?;
    let (w, h) = (color.0, color.1);
    let opacity = read_jpg(&f("Opacity"));
    let mut px = color.2.clone();
    for i in 0..(w * h) as usize {
        if let Some(c) = color_override {
            px[i * 4..i * 4 + 3].copy_from_slice(&c);
        }
        px[i * 4 + 3] = opacity.as_ref().filter(|o| o.0 == w && o.1 == h).map(|o| o.2[i * 4]).unwrap_or(255);
    }
    let normal = read_jpg(&f("NormalGL")).filter(|r| r.0 == w && r.1 == h).map(|r| images.add(to_image(w, h, r.2.clone(), false, false)));
    Some((images.add(to_image(w, h, px, true, false)), normal))
}

// ============================================================ HDRI

/// Читает Radiance .hdr (RGBE, RLE) в линейные RGB f32.
pub fn read_hdr(rel: &str) -> Option<(u32, u32, Vec<f32>)> {
    let data = std::fs::read(assets_dir()?.join(rel)).ok()?;
    let mut pos = 0;
    // заголовок до пустой строки
    loop {
        let end = data[pos..].iter().position(|b| *b == b'\n')? + pos;
        let line = &data[pos..end];
        pos = end + 1;
        if line.is_empty() {
            break;
        }
    }
    let end = data[pos..].iter().position(|b| *b == b'\n')? + pos;
    let res = std::str::from_utf8(&data[pos..end]).ok()?.to_string();
    pos = end + 1;
    let parts: Vec<&str> = res.split_whitespace().collect();
    if parts.len() < 4 {
        return None;
    }
    let h: u32 = parts[1].parse().ok()?;
    let w: u32 = parts[3].parse().ok()?;
    let mut out = vec![0f32; (w * h * 3) as usize];
    let mut scan = vec![0u8; (w * 4) as usize];
    for y in 0..h as usize {
        if pos + 4 > data.len() {
            return None;
        }
        if data[pos] == 2 && data[pos + 1] == 2 && (data[pos + 2] as u32) << 8 | data[pos + 3] as u32 == w {
            pos += 4;
            for c in 0..4 {
                let mut x = 0usize;
                while x < w as usize {
                    let count = data[pos] as usize;
                    pos += 1;
                    if count > 128 {
                        let n = count - 128;
                        let v = data[pos];
                        pos += 1;
                        for k in 0..n {
                            scan[(x + k) * 4 + c] = v;
                        }
                        x += n;
                    } else {
                        for k in 0..count {
                            scan[(x + k) * 4 + c] = data[pos + k];
                        }
                        pos += count;
                        x += count;
                    }
                }
            }
        } else {
            // плоский RGBE без RLE
            scan.copy_from_slice(&data[pos..pos + (w * 4) as usize]);
            pos += (w * 4) as usize;
        }
        for x in 0..w as usize {
            let e = scan[x * 4 + 3];
            let f = if e == 0 { 0.0 } else { 2f32.powi(e as i32 - 136) };
            let k = (y * w as usize + x) * 3;
            out[k] = scan[x * 4] as f32 * f;
            out[k + 1] = scan[x * 4 + 1] as f32 * f;
            out[k + 2] = scan[x * 4 + 2] as f32 * f;
        }
    }
    Some((w, h, out))
}

fn f16(v: f32) -> u16 {
    let x = v.to_bits();
    let sign = ((x >> 16) & 0x8000) as u16;
    let v = v.abs();
    if v.is_nan() {
        return 0x7e00;
    }
    if v >= 65504.0 {
        return sign | 0x7bff;
    }
    if v < 6.1e-5 {
        return sign | ((v / 5.96e-8) as u16);
    }
    let bits = v.to_bits();
    let exp = ((bits >> 23) & 0xff) as i32 - 127 + 15;
    let mant = (bits >> 13) & 0x3ff;
    sign | ((exp as u16) << 10) | mant as u16
}

fn push_px(out: &mut Vec<u8>, c: [f32; 3]) {
    for v in [c[0], c[1], c[2], 1.0] {
        out.extend_from_slice(&f16(v).to_le_bytes());
    }
}

/// Небо целиком: развёртка для купола и кубы для освещения окружением.
pub struct HdrSky {
    pub w: u32,
    pub h: u32,
    pub rgb: Vec<f32>,
}

impl HdrSky {
    fn sample(&self, d: Vec3) -> [f32; 3] {
        let d = d.normalize_or(Vec3::Y);
        let u = d.x.atan2(-d.z) / std::f32::consts::TAU + 0.5;
        let v = 0.5 - d.y.clamp(-1.0, 1.0).asin() / std::f32::consts::PI;
        let x = ((u * self.w as f32) as i32).rem_euclid(self.w as i32) as usize;
        let y = ((v * self.h as f32) as i32).clamp(0, self.h as i32 - 1) as usize;
        let k = (y * self.w as usize + x) * 3;
        [self.rgb[k], self.rgb[k + 1], self.rgb[k + 2]]
    }

    /// Усреднение по конусу (грубая свёртка) для размытых уровней.
    fn sample_blur(&self, d: Vec3, spread: f32, n: usize) -> [f32; 3] {
        if spread <= 0.001 {
            return self.sample(d);
        }
        let t1 = d.cross(if d.y.abs() < 0.9 { Vec3::Y } else { Vec3::X }).normalize();
        let t2 = d.cross(t1);
        let mut acc = [0.0; 3];
        let mut wsum = 0.0;
        for i in 0..n {
            // золотой угол
            let a = i as f32 * 2.399963;
            let r = ((i as f32 + 0.5) / n as f32).sqrt() * spread;
            let dir = (d + (t1 * a.cos() + t2 * a.sin()) * r).normalize();
            let w = 1.0 - r / (spread * 1.2);
            let s = self.sample(dir);
            for c in 0..3 {
                acc[c] += s[c] * w;
            }
            wsum += w;
        }
        [acc[0] / wsum, acc[1] / wsum, acc[2] / wsum]
    }

    /// Равнопромежуточная развёртка для купола неба (половинное разрешение, f16).
    pub fn dome_image(&self) -> Image {
        let w = (self.w / 2).max(64);
        let h = (self.h / 2).max(32);
        let mut out = Vec::with_capacity((w * h * 8) as usize);
        for y in 0..h {
            for x in 0..w {
                let sx = (x * 2) as usize;
                let sy = (y * 2) as usize;
                let mut c = [0.0f32; 3];
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let k = ((sy + dy).min(self.h as usize - 1) * self.w as usize + (sx + dx).min(self.w as usize - 1)) * 3;
                    for i in 0..3 {
                        c[i] += self.rgb[k + i] * 0.25;
                    }
                }
                push_px(&mut out, c);
            }
        }
        let mut img = Image::new(
            Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            TextureDimension::D2,
            out,
            TextureFormat::Rgba16Float,
            RenderAssetUsages::RENDER_WORLD,
        );
        img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::ClampToEdge,
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            ..default()
        });
        img
    }

    /// Средняя яркость верхней полусферы и направление на самый яркий пиксель.
    pub fn stats(&self) -> (f32, Vec3) {
        let mut sum = 0.0;
        let mut wsum = 0.0;
        let mut best = (0.0f32, Vec3::Y);
        for y in 0..self.h {
            let lat = std::f32::consts::FRAC_PI_2 - (y as f32 + 0.5) / self.h as f32 * std::f32::consts::PI;
            if lat < 0.0 {
                continue;
            }
            let w = lat.cos();
            for x in (0..self.w).step_by(2) {
                let k = ((y * self.w + x) * 3) as usize;
                let l = 0.2126 * self.rgb[k] + 0.7152 * self.rgb[k + 1] + 0.0722 * self.rgb[k + 2];
                sum += l * w;
                wsum += w;
                if l > best.0 {
                    let lon = ((x as f32 + 0.5) / self.w as f32 - 0.5) * std::f32::consts::TAU;
                    // обратное к sample(): u = atan2(x, -z)
                    let d = Vec3::new(lat.cos() * lon.sin(), lat.sin(), -lat.cos() * lon.cos());
                    best = (l, d);
                }
            }
        }
        ((sum / wsum.max(1e-6)).max(1e-5), best.1)
    }

    fn face_dir(face: usize, u: f32, v: f32) -> Vec3 {
        match face {
            0 => Vec3::new(1.0, -v, -u),
            1 => Vec3::new(-1.0, -v, u),
            2 => Vec3::new(u, 1.0, v),
            3 => Vec3::new(u, -1.0, -v),
            4 => Vec3::new(u, -v, 1.0),
            _ => Vec3::new(-u, -v, -1.0),
        }
    }

    fn cube(&self, size: u32, mips: u32, spread_for: impl Fn(u32) -> f32, samples: usize) -> Image {
        let mut data = Vec::new();
        for face in 0..6 {
            for m in 0..mips {
                let s = (size >> m).max(1);
                let spread = spread_for(m);
                for y in 0..s {
                    for x in 0..s {
                        let u = (x as f32 + 0.5) / s as f32 * 2.0 - 1.0;
                        let v = (y as f32 + 0.5) / s as f32 * 2.0 - 1.0;
                        let d = Self::face_dir(face, u, v).normalize();
                        push_px(&mut data, self.sample_blur(d, spread, samples));
                    }
                }
            }
        }
        let mut img = Image::new(
            Extent3d { width: size, height: size, depth_or_array_layers: 6 },
            TextureDimension::D2,
            vec![0u8; (size * size * 6 * 8) as usize],
            TextureFormat::Rgba16Float,
            RenderAssetUsages::RENDER_WORLD,
        );
        img.data = Some(data);
        img.texture_descriptor.mip_level_count = mips;
        img.texture_view_descriptor = Some(TextureViewDescriptor { dimension: Some(TextureViewDimension::Cube), ..default() });
        img.sampler = sampler(false);
        img
    }

    /// Карты для EnvironmentMapLight: диффузная (сильно размытая) и зеркальная с mip-уровнями по шероховатости.
    pub fn env_maps(&self) -> (Image, Image) {
        let diffuse = self.cube(16, 1, |_| 1.4, 96);
        let mips = 7; // 64..1
        let specular = self.cube(64, mips, |m| (m as f32 / (mips - 1) as f32).powf(1.5) * 1.3, 24);
        (diffuse, specular)
    }
}

/// Готовые картинки неба (строятся один раз на сессию).
#[derive(Clone)]
pub struct SkySet {
    pub dome: Handle<Image>,
    pub diffuse: Handle<Image>,
    pub specular: Handle<Image>,
    /// Средняя яркость неба (для подбора интенсивности окружения)
    pub avg: f32,
    /// Направление на самую яркую точку (солнце/луна)
    pub bright_dir: Vec3,
}

#[derive(Resource, Default)]
pub struct SkyCache(pub HashMap<String, SkySet>);

pub fn sky_for(cache: &mut SkyCache, images: &mut Assets<Image>, night: bool) -> Option<SkySet> {
    let file = if night { "sky/dikhololo_night.hdr" } else { "sky/kloppenheim_02_puresky.hdr" };
    if let Some(s) = cache.0.get(file) {
        return Some(s.clone());
    }
    let (w, h, rgb) = read_hdr(file)?;
    let sky = HdrSky { w, h, rgb };
    let (d, s) = sky.env_maps();
    let (avg, bright_dir) = sky.stats();
    let set = SkySet { dome: images.add(sky.dome_image()), diffuse: images.add(d), specular: images.add(s), avg, bright_dir };
    cache.0.insert(file.to_string(), set.clone());
    Some(set)
}

// ============================================================ модели Kenney

#[derive(Resource, Clone, Default)]
pub struct KenneyLib {
    pub trees: Vec<Handle<WorldAsset>>,
    pub bushes: Vec<Handle<WorldAsset>>,
    pub rocks: Vec<Handle<WorldAsset>>,
    pub cars: Vec<Handle<WorldAsset>>,
    pub debris: Vec<Handle<WorldAsset>>,
    pub tire: Option<Handle<WorldAsset>>,
    pub lamp: Option<Handle<WorldAsset>>,
    pub cone: Option<Handle<WorldAsset>>,
    pub barrier: Option<Handle<WorldAsset>>,
    pub dumpster: Option<Handle<WorldAsset>>,
    pub traffic_light: Option<Handle<WorldAsset>>,
    pub sign: Option<Handle<WorldAsset>>,
    /// Наборы City Kit и Racing Kit: путь без .glb (как в kitbox::KIT) -> сцена
    pub kit: HashMap<&'static str, Handle<WorldAsset>>,
    /// Маски ночных окон наборов City Kit (каталог набора -> текстура свечения)
    pub windows: HashMap<&'static str, Handle<Image>>,
}

pub fn load_kenney(mut commands: Commands, server: Res<AssetServer>, headless: Res<crate::game::Headless>) {
    if headless.0 {
        return;
    }
    let Some(dir) = assets_dir() else { return };
    if !dir.join("kenney").is_dir() {
        return;
    }
    let s = |p: &str| -> Option<Handle<WorldAsset>> {
        if dir.join(p).is_file() {
            Some(server.load(GltfAssetLabel::Scene(0).from_asset(p.to_string())))
        } else {
            None
        }
    };
    let many = |list: &[&str]| -> Vec<Handle<WorldAsset>> { list.iter().filter_map(|p| s(p)).collect() };
    let lib = KenneyLib {
        trees: many(&[
            "kenney/nature/tree_oak.glb",
            "kenney/nature/tree_default.glb",
            "kenney/nature/tree_detailed.glb",
            "kenney/nature/tree_fat.glb",
            "kenney/nature/tree_tall.glb",
            "kenney/nature/tree_oak_dark.glb",
            "kenney/nature/tree_default_dark.glb",
            "kenney/nature/tree_tall_dark.glb",
            "kenney/nature/tree_pineTallA.glb",
            "kenney/nature/tree_pineRoundA.glb",
        ]),
        bushes: many(&["kenney/nature/plant_bush.glb", "kenney/nature/plant_bushLarge.glb", "kenney/nature/plant_bushDetailed.glb", "kenney/nature/grass_large.glb"]),
        rocks: many(&[
            "kenney/nature/rock_largeA.glb",
            "kenney/nature/rock_largeB.glb",
            "kenney/nature/rock_largeC.glb",
            "kenney/nature/rock_tallA.glb",
            "kenney/nature/rock_tallB.glb",
            "kenney/nature/stone_largeA.glb",
            "kenney/nature/stone_tallA.glb",
        ]),
        cars: many(&[
            "kenney/car/sedan.glb",
            "kenney/car/suv.glb",
            "kenney/car/van.glb",
            "kenney/car/taxi.glb",
            "kenney/car/police.glb",
            "kenney/car/truck.glb",
            "kenney/car/hatchback-sports.glb",
            "kenney/car/sedan-sports.glb",
            "kenney/car/delivery.glb",
            "kenney/car/suv-luxury.glb",
        ]),
        debris: many(&[
            "kenney/car/debris-door.glb",
            "kenney/car/debris-door-window.glb",
            "kenney/car/debris-bumper.glb",
            "kenney/car/debris-plate-a.glb",
            "kenney/car/debris-plate-b.glb",
            "kenney/car/debris-spoiler-a.glb",
            "kenney/car/debris-drivetrain.glb",
        ]),
        tire: s("kenney/car/debris-tire.glb"),
        lamp: s("kenney/roads/light-curved.glb"),
        cone: s("kenney/car/cone.glb"),
        barrier: s("kenney/roads/construction-barrier.glb"),
        dumpster: s("kenney/roads/dumpster.glb"),
        traffic_light: s("kenney/roads/traffic-light.glb"),
        sign: s("kenney/roads/road-sign-warning.glb"),
        kit: HashMap::new(),
        windows: HashMap::new(),
    };
    let mut lib = lib;
    for (name, _, _) in crate::kitbox::KIT {
        let p = format!("kenney/{name}.glb");
        if let Some(h) = s(&p) {
            lib.kit.insert(*name, h);
        }
    }
    for kit in ["city_commercial", "city_industrial", "city_suburban"] {
        let p = format!("kenney/{kit}/Textures/emissive_windows.png");
        if dir.join(&p).is_file() {
            lib.windows.insert(kit, server.load(p));
        }
    }
    info!("модели Kenney: деревьев {}, машин {}, обломков {}, наборов city/racing {}", lib.trees.len(), lib.cars.len(), lib.debris.len(), lib.kit.len());
    commands.insert_resource(lib);
}

/// Метка: материалы этой сцены после загрузки нужно «состарить» (ржавые остовы).
#[derive(Component, Clone, Copy)]
pub struct Weathered(pub f32);

/// Затемняет и делает шершавыми материалы загруженных сцен-остовов.
pub fn weather_scenes(
    added: Query<(Entity, &MeshMaterial3d<StandardMaterial>), Added<MeshMaterial3d<StandardMaterial>>>,
    parents: Query<&ChildOf>,
    weathered: Query<&Weathered>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
    mut cache: Local<HashMap<(AssetId<StandardMaterial>, u32), Handle<StandardMaterial>>>,
) {
    for (e, m) in added.iter() {
        let mut cur = e;
        let mut found = None;
        for _ in 0..6 {
            if let Ok(w) = weathered.get(cur) {
                found = Some(w.0);
                break;
            }
            match parents.get(cur) {
                Ok(p) => cur = p.parent(),
                Err(_) => break,
            }
        }
        let Some(k) = found else { continue };
        let key = (m.0.id(), (k * 100.0) as u32);
        let h = if let Some(h) = cache.get(&key) {
            h.clone()
        } else {
            let Some(src) = mats.get(&m.0) else { continue };
            let mut nm = src.clone();
            let c = nm.base_color.to_linear();
            nm.base_color = Color::linear_rgb(c.red * k + 0.06 * (1.0 - k), c.green * k + 0.035 * (1.0 - k), c.blue * k + 0.02 * (1.0 - k));
            nm.perceptual_roughness = 0.9;
            nm.metallic = 0.2;
            let h = mats.add(nm);
            cache.insert(key, h.clone());
            h
        };
        commands.entity(e).insert(MeshMaterial3d(h));
    }
}

/// Метка: у сцены ночью светятся окна (маска по палитре набора), k — яркость.
#[derive(Component, Clone)]
pub struct NightLit(pub Handle<Image>, pub f32);

/// Подменяет материалы зданий City Kit на копии со светящимися окнами.
pub fn night_windows(
    added: Query<(Entity, &MeshMaterial3d<StandardMaterial>), Added<MeshMaterial3d<StandardMaterial>>>,
    parents: Query<&ChildOf>,
    lit: Query<&NightLit>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
    mut cache: Local<HashMap<(AssetId<StandardMaterial>, AssetId<Image>, u32), Handle<StandardMaterial>>>,
) {
    for (e, m) in added.iter() {
        let mut cur = e;
        let mut found = None;
        for _ in 0..6 {
            if let Ok(w) = lit.get(cur) {
                found = Some(w.clone());
                break;
            }
            match parents.get(cur) {
                Ok(p) => cur = p.parent(),
                Err(_) => break,
            }
        }
        let Some(NightLit(mask, k)) = found else { continue };
        let key = (m.0.id(), mask.id(), (k * 20.0) as u32);
        let h = if let Some(h) = cache.get(&key) {
            h.clone()
        } else {
            let Some(src) = mats.get(&m.0) else { continue };
            let mut nm = src.clone();
            nm.emissive_texture = Some(mask.clone());
            nm.emissive = LinearRgba::rgb(k, k, k);
            let h = mats.add(nm);
            cache.insert(key, h.clone());
            h
        };
        commands.entity(e).insert(MeshMaterial3d(h));
    }
}

pub struct ExtAssetsPlugin;

impl Plugin for ExtAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SkyCache>().add_systems(Startup, load_kenney).add_systems(Update, (weather_scenes, night_windows));
    }
}
