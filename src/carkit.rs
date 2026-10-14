//! Модели машин из Kenney Car Kit (CC0), подготовленные tools/cars/convert.py в assets/cars/*.cbm.
//! Формат .cbm: "CBM1"/"CBM2", u32 длина JSON, JSON с частями, затем вершины частей
//! (позиция 3, нормаль 3, UV0 2, UV1 2, в CBM2 ещё AO и грязь; f32, без индексов: каждые 3 вершины — треугольник).
//! В CBM2 нормали сглажены, а AO и грязь запечены (tools/cars/bake.py) и идут в цвет вершин.

use bevy::asset::RenderAssetUsages;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Clone, Debug)]
pub struct KitPart {
    /// paint, trim, glass, head, brake, detach, wheel0..wheel3
    pub name: String,
    /// зона отрыва (0 перед, 1 зад) или -1
    pub zone: i32,
    pub center: Vec3,
    pub half: Vec3,
    pub pos: Vec<[f32; 3]>,
    pub nrm: Vec<[f32; 3]>,
    pub uv0: Vec<[f32; 2]>,
    pub uv1: Vec<[f32; 2]>,
    /// (ao 0..1, грязь 0..1) на вершину; пусто для CBM1
    pub ad: Vec<[f32; 2]>,
}

/// Цвет вершины из запечённых AO и грязи: множитель к цвету материала.
pub fn vertex_color(ad: [f32; 2]) -> [f32; 4] {
    let ao = 0.28 + 0.72 * ad[0].powf(1.3);
    let d = (ad[1] * 0.8).min(0.85);
    let dirt = [0.5, 0.43, 0.36];
    [ao * (1.0 + (dirt[0] - 1.0) * d), ao * (1.0 + (dirt[1] - 1.0) * d), ao * (1.0 + (dirt[2] - 1.0) * d), 1.0]
}

#[derive(Debug)]
pub struct KitModel {
    pub parts: Vec<KitPart>,
}

impl KitModel {
    pub fn part(&self, name: &str) -> Option<&KitPart> {
        self.parts.iter().find(|p| p.name == name)
    }
}

impl KitPart {
    /// Отдельный меш на каждую машину: вмятины меняют вершины.
    pub fn mesh(&self) -> Mesh {
        let n = self.pos.len() as u32;
        let m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.pos.clone())
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.nrm.clone())
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uv0.clone())
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, self.uv1.clone())
            .with_inserted_indices(Indices::U32((0..n).collect()));
        self.with_colors(m)
    }

    fn with_colors(&self, m: Mesh) -> Mesh {
        if self.ad.len() == self.pos.len() {
            m.with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.ad.iter().map(|a| vertex_color(*a)).collect::<Vec<_>>())
        } else {
            m
        }
    }

    /// Тот же меш с кубической развёрткой в UV0 (слой износа, трещины стекла).
    /// Канал UV1 в материале на программном рендере давал полосы палитры, поэтому отдельный меш.
    pub fn mesh_box_uv(&self) -> Mesh {
        let n = self.pos.len() as u32;
        let m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.pos.clone())
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.nrm.clone())
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uv1.clone())
            .with_inserted_indices(Indices::U32((0..n).collect()));
        self.with_colors(m)
    }
}

fn parse(bytes: &[u8]) -> Result<KitModel, String> {
    let stride = match bytes.get(0..4) {
        Some(b"CBM1") => 10,
        Some(b"CBM2") => 12,
        _ => return Err("не CBM1/CBM2".into()),
    };
    let hl = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let head: serde_json::Value = serde_json::from_slice(&bytes[8..8 + hl]).map_err(|e| e.to_string())?;
    let base = 8 + hl;
    let mut parts = Vec::new();
    for p in head["parts"].as_array().ok_or("нет parts")? {
        let verts = p["verts"].as_u64().ok_or("verts")? as usize;
        let off = base + p["offset"].as_u64().ok_or("offset")? as usize;
        let end = off + verts * stride * 4;
        if end > bytes.len() {
            return Err("файл обрезан".into());
        }
        let f = |k: usize| f32::from_le_bytes(bytes[off + k * 4..off + k * 4 + 4].try_into().unwrap());
        let v3 = |a: &serde_json::Value| Vec3::new(a[0].as_f64().unwrap_or(0.0) as f32, a[1].as_f64().unwrap_or(0.0) as f32, a[2].as_f64().unwrap_or(0.0) as f32);
        let mut part = KitPart {
            name: p["name"].as_str().unwrap_or("").to_string(),
            zone: p["zone"].as_i64().unwrap_or(-1) as i32,
            center: v3(&p["center"]),
            half: v3(&p["half"]),
            pos: Vec::with_capacity(verts),
            nrm: Vec::with_capacity(verts),
            uv0: Vec::with_capacity(verts),
            uv1: Vec::with_capacity(verts),
            ad: Vec::with_capacity(if stride == 12 { verts } else { 0 }),
        };
        for i in 0..verts {
            let b = i * stride;
            part.pos.push([f(b), f(b + 1), f(b + 2)]);
            part.nrm.push([f(b + 3), f(b + 4), f(b + 5)]);
            part.uv0.push([f(b + 6), f(b + 7)]);
            part.uv1.push([f(b + 8), f(b + 9)]);
            if stride == 12 {
                part.ad.push([f(b + 10), f(b + 11)]);
            }
        }
        parts.push(part);
    }
    Ok(KitModel { parts })
}

/// Модель машины по id (кэш в памяти). Нет файла — ошибка с путём в лог и `None`.
pub fn model(id: &str) -> Option<Arc<KitModel>> {
    static C: OnceLock<Mutex<HashMap<String, Option<Arc<KitModel>>>>> = OnceLock::new();
    let cache = C.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(v) = cache.lock().ok()?.get(id) {
        return v.clone();
    }
    crate::carfit::fit(id)?;
    let res = (|| {
        let path = crate::extassets::assets_dir()?.join("cars").join(format!("{id}.cbm"));
        match std::fs::read(&path).map_err(|e| e.to_string()).and_then(|b| parse(&b)) {
            Ok(m) => Some(Arc::new(m)),
            Err(e) => {
                error!("модель машины {}: {}", path.display(), e);
                None
            }
        }
    })();
    cache.lock().ok()?.insert(id.to_string(), res.clone());
    res
}

#[cfg(test)]
mod tests {
    #[test]
    fn all_fitted_models_load() {
        for f in crate::carfit::FITS {
            let m = super::model(f.id).unwrap_or_else(|| panic!("нет модели {}", f.id));
            for w in 0..4 {
                assert!(m.part(&format!("wheel{w}")).is_some(), "{}: нет колеса {w}", f.id);
            }
            assert!(m.part("trim").is_some(), "{}: нет кузова", f.id);
        }
    }
}
