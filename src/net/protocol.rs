//! Сообщения и кодирование. Каждый датаграм: 2 байта метки + bincode.

use super::LobbyInfo;
use crate::combat::Combat;
use crate::config::CarConfig;
use crate::data::*;
use crate::game::*;
use crate::vehicle::{CarInput, CarState};
use crate::weapons::ProjKind;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

const MAGIC: [u8; 2] = [0xCB, 0x42];
/// Безопасный размер полезной нагрузки без фрагментации IP.
pub const MAX_PAYLOAD: usize = 1150;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum C2S {
    Hello { version: u32, name: String, config: CarConfig },
    Config { name: String, config: CarConfig },
    Input { seq: u32, input: CarInput },
    Ping { t: f64, rtt: u16 },
    Bye,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CarSnap {
    pub id: NetId,
    pub pos: Vec3,
    pub rot: Quat,
    pub vel: Vec3,
    pub state: CarState,
    pub combat: Combat,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct ProjSnap {
    pub id: u32,
    pub kind: ProjKind,
    pub pos: Vec3,
    pub vel: Vec3,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct PropSnap {
    pub id: u32,
    pub pos: Vec3,
    pub rot: Quat,
}

/// Пешеход в сжатом виде: позиция в дециметрах, курс в 1/256 оборота.
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct PedSnap {
    pub id: u16,
    pub x: i16,
    pub y: i16,
    pub z: i16,
    pub yaw: u8,
}

impl PedSnap {
    pub fn pack(id: u32, p: Vec3, yaw: f32) -> Self {
        let q = |v: f32| (v * 10.0).round().clamp(-32760.0, 32760.0) as i16;
        let y = (yaw.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * 256.0) as u32 % 256;
        Self { id: id as u16, x: q(p.x), y: q(p.y), z: q(p.z), yaw: y as u8 }
    }
    pub fn pos(&self) -> Vec3 {
        Vec3::new(self.x as f32 / 10.0, self.y as f32 / 10.0, self.z as f32 / 10.0)
    }
    pub fn yaw(&self) -> f32 {
        self.yaw as f32 / 256.0 * std::f32::consts::TAU
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum S2C {
    Welcome { id: u64 },
    Reject { reason: String },
    Lobby(LobbyInfo),
    Start { match_id: u32, cfg: MatchConfig },
    Cars { tick: u32, time: f64, cars: Vec<CarSnap> },
    World { tick: u32, time: f64, ms: MatchState, pickups: Vec<(u16, bool, PowerupKind)>, projs: Vec<ProjSnap> },
    Events(Vec<(u32, GameEvent)>),
    Props(Vec<PropSnap>),
    DeadProps(Vec<u32>),
    Peds(Vec<PedSnap>),
    Pong { t: f64 },
    ToLobby,
    Bye { reason: String },
}

pub fn encode<T: Serialize>(msg: &T) -> Vec<u8> {
    let mut out = MAGIC.to_vec();
    out.extend(bincode::serialize(msg).unwrap_or_default());
    out
}

pub fn decode<T: for<'de> Deserialize<'de>>(buf: &[u8]) -> Option<T> {
    if buf.len() < 2 || buf[0..2] != MAGIC {
        return None;
    }
    bincode::deserialize(&buf[2..]).ok()
}

/// Делит список на пакеты так, чтобы каждый влезал в MAX_PAYLOAD.
pub fn chunk<T: Serialize + Clone>(items: &[T], wrap: impl Fn(Vec<T>) -> S2C) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut cur: Vec<T> = Vec::new();
    let mut size = 24;
    for it in items {
        let s = bincode::serialized_size(it).unwrap_or(64) as usize;
        if !cur.is_empty() && size + s > MAX_PAYLOAD {
            out.push(encode(&wrap(std::mem::take(&mut cur))));
            size = 24;
        }
        size += s;
        cur.push(it.clone());
    }
    if !cur.is_empty() {
        out.push(encode(&wrap(cur)));
    }
    out
}

/// Эти события чисто косметические: шлём один раз.
pub fn cosmetic(e: &GameEvent) -> bool {
    matches!(e, GameEvent::Tracer { .. } | GameEvent::Sparks { .. } | GameEvent::BulletHit { .. } | GameEvent::Flame { .. } | GameEvent::Horn { .. })
}
