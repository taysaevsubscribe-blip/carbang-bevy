//! Общие типы: состояния приложения, сетевой режим, идентификаторы, события, параметры матча.

use crate::config::CarConfig;
use crate::data::*;
use avian3d::prelude::*;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(States, Default, Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum AppState {
    #[default]
    Boot,
    Menu,
    Lobby,
    Loading,
    Race,
    Results,
}

/// Кто считает физику и правила.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum NetMode {
    #[default]
    Offline,
    Host,
    Client,
}

pub fn is_authority(mode: Res<NetMode>) -> bool {
    *mode != NetMode::Client
}

pub fn is_client(mode: Res<NetMode>) -> bool {
    *mode == NetMode::Client
}

/// Порядок симуляции внутри фиксированного шага.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SimSet {
    Vehicle,
    Weapons,
    Combat,
    Rules,
}

/// Запуск без окна (тесты и выделенный сервер).
#[derive(Resource, Clone, Copy, Default)]
pub struct Headless(pub bool);

pub fn not_headless(h: Res<Headless>) -> bool {
    !h.0
}

#[derive(PhysicsLayer, Clone, Copy, Debug, Default)]
pub enum Layer {
    #[default]
    World,
    Car,
    Prop,
    Corpse,
    Sensor,
}

/// Сетевой идентификатор: одинаков у сервера и клиентов.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub struct NetId(pub u32);

#[derive(Resource, Default)]
pub struct NetIdAlloc(pub u32);

impl NetIdAlloc {
    pub fn next(&mut self) -> NetId {
        self.0 += 1;
        NetId(self.0)
    }
}

/// Кто управляет машиной.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Controller {
    Local,
    Bot,
    Remote(u64),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerSlot {
    pub name: String,
    pub config: CarConfig,
    pub controller: Controller,
    /// Личность бота (сид), для людей не используется
    pub bot_seed: u64,
}

/// Параметры текущего матча. Одинаковы у сервера и клиентов.
#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct MatchConfig {
    pub map: MapId,
    pub mode: GameMode,
    pub seed: u64,
    pub difficulty: f32,
    pub time_limit: f32,
    pub laps: u32,
    pub frag_limit: u32,
    pub peds: u32,
    pub career_event: Option<usize>,
    pub players: Vec<PlayerSlot>,
    pub zombies: bool,
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self {
            map: MapId::Port,
            mode: GameMode::Carnage,
            seed: 7,
            difficulty: 0.5,
            time_limit: 150.0,
            laps: 2,
            frag_limit: 10,
            peds: 150,
            career_event: None,
            players: Vec::new(),
            zombies: false,
        }
    }
}

impl MatchConfig {
    pub fn apply_map_defaults(&mut self) {
        let info = map_info(self.map);
        self.laps = info.laps;
        self.peds = if self.mode.has_peds() { info.peds } else { 0 };
        self.time_limit = match self.mode {
            GameMode::Carnage => info.time_limit,
            GameMode::Derby => 240.0,
            GameMode::Deathmatch => 300.0,
            GameMode::PedHunt => 180.0,
        };
    }
}

/// Какой машиной управляет этот экземпляр игры.
#[derive(Resource, Default, Clone, Copy)]
pub struct LocalPlayer {
    pub car: Option<NetId>,
}

/// События игры. На сервере порождаются симуляцией и рассылаются клиентам,
/// на клиенте приходят из сети. Визуал и звук слушают только их.
#[derive(Message, Clone, Debug, Serialize, Deserialize)]
pub enum GameEvent {
    Explosion { pos: Vec3, radius: f32 },
    Sparks { pos: Vec3, strength: f32 },
    BulletHit { pos: Vec3, normal: Vec3 },
    Tracer { from: Vec3, to: Vec3 },
    Flame { from: Vec3, dir: Vec3 },
    Crash { pos: Vec3, strength: f32 },
    PedKilled { id: u32, pos: Vec3, vel: Vec3, by: Option<NetId> },
    CarWrecked { victim: NetId, killer: Option<NetId>, pos: Vec3 },
    Respawned { car: NetId },
    Pickup { car: NetId, kind: PowerupKind, pos: Vec3 },
    WeaponFired { car: NetId, kind: WeaponKind, pos: Vec3 },
    Emp { pos: Vec3, radius: f32 },
    Splash { pos: Vec3 },
    Message { to: Option<NetId>, text: String, color: [f32; 3], big: bool },
    KillFeed { text: String },
    Hit { attacker: NetId, victim: NetId, amount: f32 },
    Horn { pos: Vec3 },
    Landed { car: NetId, air: f32 },
}

/// Состояние матча, общее для всех.
#[derive(Resource, Clone, Debug, Serialize, Deserialize, Default)]
pub struct MatchState {
    pub phase: MatchPhase,
    pub countdown: f32,
    pub elapsed: f32,
    pub time_left: f32,
    pub peds_total: u32,
    pub peds_alive: u32,
    pub winner: Option<NetId>,
    pub end_reason: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum MatchPhase {
    #[default]
    Countdown,
    Running,
    Finished,
}

pub fn match_running(ms: Option<Res<MatchState>>) -> bool {
    ms.map(|m| m.phase == MatchPhase::Running).unwrap_or(false)
}

/// Итоги последнего матча для экрана результатов.
#[derive(Resource, Clone, Debug, Default)]
pub struct MatchResult {
    pub rows: Vec<ResultRow>,
    pub won: bool,
    pub reason: String,
    pub credits: u32,
    pub xp: u32,
    pub career_event: Option<usize>,
    pub level_before: u32,
    pub level_after: u32,
}

#[derive(Clone, Debug, Default)]
pub struct ResultRow {
    pub name: String,
    pub car: String,
    pub score: i32,
    pub wrecks: u32,
    pub peds: u32,
    pub deaths: u32,
    pub damage: u32,
    pub is_local: bool,
}

/// Быстрые утилиты.
pub fn rand_f32(rng: &mut impl rand::Rng, a: f32, b: f32) -> f32 {
    rng.gen_range(a..b)
}

pub fn move_toward(cur: f32, target: f32, step: f32) -> f32 {
    if (target - cur).abs() <= step {
        target
    } else {
        cur + step * (target - cur).signum()
    }
}

pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn horiz(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z)
}

pub fn color3(c: [f32; 3]) -> Color {
    Color::srgb(c[0], c[1], c[2])
}
