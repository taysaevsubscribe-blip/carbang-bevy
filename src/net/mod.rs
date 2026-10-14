//! Сеть: сервер-авторитет поверх UDP. Хост считает физику, ИИ и правила,
//! клиенты шлют ввод и получают снапшоты 30 Гц, пешеходов 10 Гц и события.

pub mod client;
pub mod protocol;
pub mod server;

use crate::data::*;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Команды из меню.
#[derive(Message, Clone, Debug)]
pub enum NetCommand {
    Host,
    Join(String),
    Start,
    Leave,
    LobbyChanged,
    ConfigChanged,
}

/// Строка состояния для меню.
#[derive(Resource, Default, Clone)]
pub struct NetStatus {
    pub text: String,
    pub error: bool,
}

impl NetStatus {
    pub fn ok(&mut self, s: impl Into<String>) {
        self.text = s.into();
        self.error = false;
    }
    pub fn err(&mut self, s: impl Into<String>) {
        self.text = s.into();
        self.error = true;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LobbyPlayer {
    pub id: u64,
    pub name: String,
    pub car: String,
    pub ping: u16,
}

/// Состояние лобби: общее для хоста и клиентов.
#[derive(Resource, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LobbyInfo {
    pub players: Vec<LobbyPlayer>,
    pub map: MapId,
    pub mode: GameMode,
    pub bots: u32,
    pub difficulty: f32,
}

impl Default for LobbyInfo {
    fn default() -> Self {
        Self { players: Vec::new(), map: MapId::Port, mode: GameMode::Deathmatch, bots: 3, difficulty: 0.5 }
    }
}

pub const PROTOCOL_VERSION: u32 = 3;

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<NetCommand>()
            .init_resource::<NetStatus>()
            .init_resource::<LobbyInfo>()
            .add_plugins((server::ServerPlugin, client::ClientPlugin));
    }
}
