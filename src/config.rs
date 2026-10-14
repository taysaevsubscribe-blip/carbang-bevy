//! Конфигурация машины (то, что уходит по сети), расчёт итоговых характеристик,
//! профиль игрока с сохранением и настройки.

use crate::data::*;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Полное описание машины игрока или бота. Сериализуется и передаётся серверу.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CarConfig {
    pub car_id: String,
    pub levels: [u8; 6],
    pub setup: CarSetup,
    pub loadout: Vec<(Mount, WeaponKind)>,
    pub weapon_levels: Vec<(WeaponKind, u8)>,
    pub perks: Vec<PerkKind>,
    pub color: [f32; 3],
    /// Скин: 0 — нет, 1..=12 — SKINS[n-1]
    #[serde(default)]
    pub skin: u8,
}

impl CarConfig {
    pub fn new_default(car_id: &str) -> Self {
        let def = car_def(car_id);
        Self {
            car_id: def.id.to_string(),
            levels: [0; 6],
            setup: CarSetup::default(),
            loadout: Vec::new(),
            weapon_levels: Vec::new(),
            perks: Vec::new(),
            color: def.color,
            skin: 0,
        }
    }
    pub fn skin_def(&self) -> Option<&'static SkinDef> {
        if self.skin == 0 {
            None
        } else {
            SKINS.get(self.skin as usize - 1)
        }
    }
    pub fn def(&self) -> &'static CarDef {
        car_def(&self.car_id)
    }
    pub fn level(&self, up: Upgrade) -> u8 {
        self.levels[up.index()]
    }
    pub fn weapon_level(&self, kind: WeaponKind) -> u8 {
        self.weapon_levels.iter().find(|(k, _)| *k == kind).map(|(_, l)| *l).unwrap_or(0)
    }
    pub fn has_perk(&self, p: PerkKind) -> bool {
        self.perks.contains(&p)
    }
    pub fn weapon_at(&self, m: Mount) -> Option<WeaponKind> {
        self.loadout.iter().find(|(mm, _)| *mm == m).map(|(_, w)| *w)
    }
    /// Убирает оружие с точек, которых у машины нет, и несовместимое оружие.
    pub fn sanitize(&mut self) {
        let def = self.def();
        self.loadout.retain(|(m, w)| def.slots.contains(m) && weapon_def(*w).mounts.contains(m));
        let mut seen = Vec::new();
        self.loadout.retain(|(m, _)| {
            if seen.contains(m) {
                false
            } else {
                seen.push(*m);
                true
            }
        });
        self.perks.truncate(PERK_SLOTS);
        for l in self.levels.iter_mut() {
            *l = (*l).min(MAX_UPGRADE);
        }
        for i in 0..SETUP_PARAMS.len() {
            let v = self.setup.get(i);
            self.setup.set(i, v);
        }
    }
}

/// Итоговые характеристики после апгрейдов, сетапа, перков и особенности машины.
#[derive(Component, Clone, Debug)]
pub struct CarStats {
    pub mass: f32,
    pub power: f32,
    pub max_drive_force: f32,
    pub top_speed: f32,
    pub grip: f32,
    pub grip_front: f32,
    pub grip_rear: f32,
    pub armor: f32,
    pub ram: f32,
    pub drag: f32,
    pub drive: Drive,
    pub size: Vec3,
    pub wheel_radius: f32,
    pub rest_len: f32,
    /// Высота начала координат машины над землёй в покое
    pub ride_height: f32,
    pub spring_mult: f32,
    pub antiroll: f32,
    pub brake_front: f32,
    pub steer_speed: f32,
    pub stability: f32,
    pub countersteer: f32,
    pub nitro_max: f32,
    pub nitro_regen: f32,
    pub ammo_mult: f32,
    pub air_control: f32,
    pub blast_resist: f32,
    pub knock_resist: f32,
    pub credit_mult: f32,
    pub fire_rate: f32,
    pub trait_kind: CarTrait,
}

impl CarStats {
    pub fn compute(cfg: &CarConfig) -> Self {
        let d = cfg.def();
        let lv = |u: Upgrade| cfg.level(u) as f32;
        let s = &cfg.setup;
        let mut armor = d.armor * (1.0 + 0.15 * lv(Upgrade::Armor));
        let mut power = d.power_kw * 1000.0 * (1.0 + 0.12 * lv(Upgrade::Engine));
        if cfg.has_perk(PerkKind::Tank) {
            armor *= 1.2;
            power *= 0.92;
        }
        let mass = d.mass * (1.0 + 0.03 * lv(Upgrade::Armor));
        let grip = d.grip * (1.0 + 0.05 * lv(Upgrade::Tires) + 0.02 * lv(Upgrade::Suspension));
        let torque = d.torque_g * (1.0 + 0.08 * lv(Upgrade::Engine));
        let gearing = s.gearing;
        let max_drive_force = mass * 9.81 * (grip * 1.15).min(torque) / gearing.powf(0.7);
        let top_speed = (power / d.drag).powf(1.0 / 3.0) * gearing.powf(0.5) * 1.05;
        let mut ram = d.ram * (1.0 + 0.22 * lv(Upgrade::Ram));
        if cfg.has_perk(PerkKind::Battering) {
            ram *= 1.25;
        }
        let mut ammo_mult = 1.0;
        if d.trait_kind == CarTrait::ExtraAmmo {
            ammo_mult = 1.35;
        }
        Self {
            mass,
            power,
            max_drive_force,
            top_speed,
            grip,
            grip_front: grip * (1.0 + s.grip_bias),
            grip_rear: grip * (1.08 - s.grip_bias),
            armor,
            ram,
            drag: d.drag,
            drive: d.drive,
            size: d.size,
            wheel_radius: d.wheel_radius,
            rest_len: d.travel * s.ride,
            ride_height: -wheel_mount_y(d) + d.travel * s.ride * 0.58 + d.wheel_radius,
            spring_mult: s.spring * (1.0 + 0.12 * lv(Upgrade::Suspension)),
            antiroll: s.antiroll * (1.0 + 0.3 * lv(Upgrade::Suspension)),
            brake_front: s.brake_bias,
            steer_speed: s.steer_speed,
            stability: s.stability,
            countersteer: s.countersteer,
            nitro_max: 3.0 + 1.5 * lv(Upgrade::Nitro),
            nitro_regen: if d.trait_kind == CarTrait::NitroRegen { 0.08 } else { 0.04 },
            ammo_mult,
            air_control: if d.trait_kind == CarTrait::AirControl { 2.0 } else { 1.0 }
                * if cfg.has_perk(PerkKind::Stuntman) { 1.5 } else { 1.0 },
            blast_resist: if d.trait_kind == CarTrait::BlastResist { 0.6 } else { 1.0 },
            knock_resist: if d.trait_kind == CarTrait::Steady { 0.35 } else { 1.0 },
            credit_mult: if d.trait_kind == CarTrait::Rich { 1.3 } else { 1.0 },
            fire_rate: if cfg.has_perk(PerkKind::HairTrigger) { 1.25 } else { 1.0 },
            trait_kind: d.trait_kind,
        }
    }
}

/// Высота крепления колёс относительно начала координат машины.
pub fn wheel_mount_y(d: &CarDef) -> f32 {
    (0.1 - d.size.y * 0.5) - 0.06 + 0.58 * d.travel
}

/// Шкалы для гаража: 0..1 по четырём осям.
pub fn rating(cfg: &CarConfig) -> [(&'static str, f32); 5] {
    let s = CarStats::compute(cfg);
    [
        ("Разгон", (s.max_drive_force / s.mass / 9.81 / 0.75).clamp(0.05, 1.0)),
        ("Скорость", ((s.top_speed * 3.6 - 90.0) / 170.0).clamp(0.05, 1.0)),
        ("Сцепление", ((s.grip - 0.85) / 0.55).clamp(0.05, 1.0)),
        ("Защита", (s.armor / 4.5).clamp(0.05, 1.0)),
        ("Таран", (s.ram * s.mass / 18000.0).clamp(0.05, 1.0)),
    ]
}

// ============================================================ профиль

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CarProfile {
    pub levels: [u8; 6],
    pub setup: CarSetup,
    pub loadout: Vec<(Mount, WeaponKind)>,
    pub color: Option<[f32; 3]>,
    #[serde(default)]
    pub skin: u8,
}

#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub credits: u32,
    pub xp: u32,
    pub owned_cars: Vec<String>,
    pub owned_weapons: Vec<WeaponKind>,
    pub weapon_levels: HashMap<WeaponKind, u8>,
    pub cars: HashMap<String, CarProfile>,
    pub selected_car: String,
    pub perks: Vec<PerkKind>,
    pub career_done: u32,
    pub best_scores: HashMap<String, u32>,
    pub total_wrecks: u32,
    pub total_peds: u32,
}

impl Default for Profile {
    fn default() -> Self {
        let mut cars = HashMap::new();
        cars.insert(
            "ferret".to_string(),
            CarProfile {
                loadout: vec![(Mount::Roof, WeaponKind::MachineGun), (Mount::Rear, WeaponKind::Mines)],
                ..default()
            },
        );
        Self {
            name: "Игрок".to_string(),
            credits: 2500,
            xp: 0,
            owned_cars: vec!["ferret".into(), "golf".into()],
            owned_weapons: vec![WeaponKind::MachineGun, WeaponKind::Mines, WeaponKind::Spikes],
            weapon_levels: HashMap::new(),
            cars,
            selected_car: "ferret".into(),
            perks: vec![PerkKind::Mechanic],
            career_done: 0,
            best_scores: HashMap::new(),
            total_wrecks: 0,
            total_peds: 0,
        }
    }
}

impl Profile {
    pub fn level(&self) -> u32 {
        level_for_xp(self.xp)
    }

    pub fn car_profile(&self, id: &str) -> CarProfile {
        self.cars.get(id).cloned().unwrap_or_default()
    }

    pub fn car_profile_mut(&mut self, id: &str) -> &mut CarProfile {
        self.cars.entry(id.to_string()).or_default()
    }

    /// Собирает конфиг выбранной (или указанной) машины из профиля.
    pub fn config_for(&self, car_id: &str) -> CarConfig {
        let def = car_def(car_id);
        let cp = self.car_profile(car_id);
        let mut cfg = CarConfig {
            car_id: def.id.to_string(),
            levels: cp.levels,
            setup: cp.setup,
            loadout: cp.loadout.clone(),
            weapon_levels: self.weapon_levels.iter().map(|(k, v)| (*k, *v)).collect(),
            perks: self.perks.clone(),
            color: cp.color.unwrap_or(def.color),
            skin: cp.skin,
        };
        cfg.weapon_levels.sort();
        cfg.sanitize();
        cfg
    }

    pub fn selected_config(&self) -> CarConfig {
        self.config_for(&self.selected_car)
    }

    pub fn path() -> PathBuf {
        data_dir().join("profile.json")
    }

    pub fn load() -> Self {
        let p = Self::path();
        match std::fs::read_to_string(&p) {
            Ok(s) => match serde_json::from_str::<Profile>(&s) {
                Ok(mut prof) => {
                    if !prof.owned_cars.iter().any(|c| c == &prof.selected_car) {
                        prof.selected_car = "ferret".into();
                    }
                    prof
                }
                Err(e) => {
                    warn!("профиль повреждён ({e}), создаю новый");
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        let p = Self::path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match serde_json::to_string_pretty(self) {
            Ok(s) => {
                if let Err(e) = std::fs::write(&p, s) {
                    warn!("не удалось сохранить профиль: {e}");
                }
            }
            Err(e) => warn!("не удалось сериализовать профиль: {e}"),
        }
    }
}

pub fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CARBANG_DATA") {
        return PathBuf::from(dir);
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        return PathBuf::from(appdata).join("CarBang");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".local/share/carbang");
    }
    PathBuf::from("carbang_data")
}

// ============================================================ настройки

#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub quality: u8,
    pub fullscreen: bool,
    pub volume: f32,
    pub zombies: bool,
    pub camera_distance: f32,
    pub show_fps: bool,
    pub last_ip: String,
    pub port: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            quality: 2,
            fullscreen: false,
            volume: 0.7,
            zombies: false,
            camera_distance: 1.0,
            show_fps: false,
            last_ip: "127.0.0.1".into(),
            port: 47800,
        }
    }
}

impl Settings {
    pub fn path() -> PathBuf {
        data_dir().join("settings.json")
    }
    pub fn load() -> Self {
        std::fs::read_to_string(Self::path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    pub fn save(&self) {
        let p = Self::path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(p, s);
        }
    }
}
