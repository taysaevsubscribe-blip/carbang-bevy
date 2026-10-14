//! Хост: принимает клиентов, ведёт лобби, применяет их ввод, рассылает снапшоты.

use super::protocol::*;
use super::*;
use crate::combat::Combat;
use crate::config::*;
use crate::game::*;
use crate::maps::Prop;
use crate::peds::Ped;
use crate::powerups::Pickup;
use crate::tests::add_bots;
use crate::ui::menu::{player_slot, MenuDirty, MenuScreen};
use crate::vehicle::*;
use crate::weapons::{Mine, OilSlick, ProjVisual, Projectile};
use avian3d::prelude::*;
use std::collections::{HashMap, HashSet};
use std::net::{SocketAddr, UdpSocket};

pub struct ClientConn {
    pub id: u64,
    pub addr: SocketAddr,
    pub name: String,
    pub config: CarConfig,
    pub last_seen: f64,
    pub input: CarInput,
    pub input_seq: u32,
    pub ping: u16,
}

#[derive(Resource)]
pub struct Server {
    pub socket: UdpSocket,
    pub clients: Vec<ClientConn>,
    pub next_id: u64,
    pub tick: u32,
    pub match_id: u32,
    pub start_cfg: Option<MatchConfig>,
    pub start_until: f64,
    pub events: Vec<(u32, f64, GameEvent)>,
    pub next_event: u32,
    pub snap_t: f64,
    pub ped_t: f64,
    pub lobby_t: f64,
    pub dead_t: f64,
    pub prop_last: HashMap<u32, (Vec3, Quat)>,
    pub known_props: HashSet<u32>,
}

impl Server {
    fn send_all(&self, bytes: &[u8]) {
        for c in &self.clients {
            let _ = self.socket.send_to(bytes, c.addr);
        }
    }
}

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, server_recv.run_if(resource_exists::<Server>))
            .add_systems(Update, (server_commands, server_inputs.run_if(resource_exists::<Server>)).chain())
            .add_systems(PostUpdate, (collect_events, server_send).chain().run_if(resource_exists::<Server>))
            .add_systems(OnEnter(AppState::Menu), back_to_lobby.run_if(resource_exists::<Server>));
    }
}

fn lobby_from(server: &Server, host_name: &str, host_car: &str, lobby: &LobbyInfo) -> LobbyInfo {
    let mut l = lobby.clone();
    l.players = vec![LobbyPlayer { id: 0, name: host_name.to_string(), car: host_car.to_string(), ping: 0 }];
    for c in &server.clients {
        l.players.push(LobbyPlayer { id: c.id, name: c.name.clone(), car: c.config.car_id.clone(), ping: c.ping });
    }
    l
}

#[allow(clippy::too_many_arguments)]
fn server_commands(
    mut commands: Commands,
    mut ev: MessageReader<NetCommand>,
    server: Option<ResMut<Server>>,
    mut mode: ResMut<NetMode>,
    mut status: ResMut<NetStatus>,
    mut lobby: ResMut<LobbyInfo>,
    settings: Res<Settings>,
    profile: Res<Profile>,
    mut screen: ResMut<MenuScreen>,
    mut dirty: ResMut<MenuDirty>,
    mut cfg: ResMut<MatchConfig>,
    mut next: ResMut<NextState<AppState>>,
    time: Res<Time<Real>>,
) {
    let mut server = server;
    for cmd in ev.read() {
        match cmd {
            NetCommand::Host => {
                if server.is_some() {
                    continue;
                }
                match UdpSocket::bind(("0.0.0.0", settings.port)) {
                    Ok(sock) => {
                        let _ = sock.set_nonblocking(true);
                        commands.insert_resource(Server {
                            socket: sock,
                            clients: Vec::new(),
                            next_id: 1,
                            tick: 0,
                            match_id: 0,
                            start_cfg: None,
                            start_until: 0.0,
                            events: Vec::new(),
                            next_event: 1,
                            snap_t: 0.0,
                            ped_t: 0.0,
                            lobby_t: 0.0,
                            dead_t: 0.0,
                            prop_last: HashMap::new(),
                            known_props: HashSet::new(),
                        });
                        *mode = NetMode::Host;
                        lobby.players = vec![LobbyPlayer { id: 0, name: profile.name.clone(), car: profile.selected_car.clone(), ping: 0 }];
                        status.ok(format!("Сервер запущен на порту {}. Сообщи друзьям свой IP.", settings.port));
                        *screen = MenuScreen::Lobby;
                        dirty.0 = true;
                    }
                    Err(e) => status.err(format!("Не удалось открыть порт {}: {}", settings.port, e)),
                }
            }
            NetCommand::Start => {
                let Some(s) = server.as_mut() else { continue };
                let mut c = MatchConfig {
                    map: lobby.map,
                    mode: lobby.mode,
                    seed: rand::random::<u64>() % 1_000_000,
                    difficulty: lobby.difficulty,
                    zombies: settings.zombies,
                    ..default()
                };
                c.apply_map_defaults();
                c.players.push(player_slot(&profile));
                for cl in &s.clients {
                    c.players.push(PlayerSlot { name: cl.name.clone(), config: cl.config.clone(), controller: Controller::Remote(cl.id), bot_seed: 0 });
                }
                add_bots(&mut c, lobby.bots as usize, profile.level());
                s.match_id += 1;
                s.start_cfg = Some(c.clone());
                s.start_until = time.elapsed_secs_f64() + 3.0;
                s.events.clear();
                s.prop_last.clear();
                s.known_props.clear();
                let bytes = encode(&S2C::Start { match_id: s.match_id, cfg: c.clone() });
                s.send_all(&bytes);
                *cfg = c;
                next.set(AppState::Loading);
            }
            NetCommand::Leave => {
                if let Some(s) = server.as_ref() {
                    s.send_all(&encode(&S2C::Bye { reason: "Хост закрыл игру".into() }));
                    commands.remove_resource::<Server>();
                    *mode = NetMode::Offline;
                    status.ok("Сервер остановлен");
                }
            }
            NetCommand::LobbyChanged | NetCommand::ConfigChanged => {
                if let Some(s) = server.as_mut() {
                    let l = lobby_from(s, &profile.name, &profile.selected_car, &lobby);
                    s.send_all(&encode(&S2C::Lobby(l.clone())));
                    *lobby = l;
                }
            }
            NetCommand::Join(_) => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn server_recv(
    mut server: ResMut<Server>,
    time: Res<Time<Real>>,
    mut lobby: ResMut<LobbyInfo>,
    profile: Res<Profile>,
    state: Res<State<AppState>>,
    mut status: ResMut<NetStatus>,
    mut feed: MessageWriter<GameEvent>,
) {
    let now = time.elapsed_secs_f64();
    let mut buf = [0u8; 2048];
    let mut lobby_dirty = false;
    loop {
        let (n, addr) = match server.socket.recv_from(&mut buf) {
            Ok(x) => x,
            Err(_) => break,
        };
        let Some(msg) = decode::<C2S>(&buf[..n]) else { continue };
        let idx = server.clients.iter().position(|c| c.addr == addr);
        match msg {
            C2S::Hello { version, name, mut config } => {
                if version != PROTOCOL_VERSION {
                    let _ = server.socket.send_to(&encode(&S2C::Reject { reason: "Другая версия игры".into() }), addr);
                    continue;
                }
                if server.clients.len() >= 7 && idx.is_none() {
                    let _ = server.socket.send_to(&encode(&S2C::Reject { reason: "Сервер заполнен".into() }), addr);
                    continue;
                }
                config.sanitize();
                let id = match idx {
                    Some(i) => {
                        server.clients[i].last_seen = now;
                        server.clients[i].id
                    }
                    None => {
                        let id = server.next_id;
                        server.next_id += 1;
                        let name: String = name.chars().take(16).collect();
                        status.ok(format!("{} подключился", name));
                        server.clients.push(ClientConn { id, addr, name, config, last_seen: now, input: CarInput::default(), input_seq: 0, ping: 0 });
                        lobby_dirty = true;
                        id
                    }
                };
                let _ = server.socket.send_to(&encode(&S2C::Welcome { id }), addr);
                // опоздавший к уже идущему матчу посмотрит итоги и попадёт в следующий
            }
            C2S::Config { name, mut config } => {
                if let Some(i) = idx {
                    config.sanitize();
                    server.clients[i].config = config;
                    server.clients[i].name = name.chars().take(16).collect();
                    server.clients[i].last_seen = now;
                    lobby_dirty = true;
                }
            }
            C2S::Input { seq, input } => {
                if let Some(i) = idx {
                    let c = &mut server.clients[i];
                    c.last_seen = now;
                    if seq > c.input_seq || c.input_seq.wrapping_sub(seq) > 1_000_000 {
                        c.input_seq = seq;
                        c.input = input;
                    }
                }
            }
            C2S::Ping { t, rtt } => {
                if let Some(i) = idx {
                    server.clients[i].last_seen = now;
                    server.clients[i].ping = rtt;
                }
                let _ = server.socket.send_to(&encode(&S2C::Pong { t }), addr);
            }
            C2S::Bye => {
                if let Some(i) = idx {
                    let c = server.clients.remove(i);
                    feed.write(GameEvent::KillFeed { text: format!("{} отключился", c.name) });
                    status.ok(format!("{} отключился", c.name));
                    lobby_dirty = true;
                }
            }
        }
    }
    // таймауты
    let before = server.clients.len();
    let gone: Vec<String> = server.clients.iter().filter(|c| now - c.last_seen > 8.0).map(|c| c.name.clone()).collect();
    server.clients.retain(|c| now - c.last_seen <= 8.0);
    for g in gone {
        feed.write(GameEvent::KillFeed { text: format!("{} потерял связь", g) });
    }
    if server.clients.len() != before {
        lobby_dirty = true;
    }
    if *state.get() == AppState::Menu && (lobby_dirty || now - server.lobby_t > 1.0) {
        server.lobby_t = now;
        let l = lobby_from(&server, &profile.name, &profile.selected_car, &lobby);
        server.send_all(&encode(&S2C::Lobby(l.clone())));
        if *lobby != l {
            *lobby = l;
        }
    }
    // повтор старта для потерявших пакет
    if now < server.start_until {
        if let Some(cfg) = server.start_cfg.clone() {
            if (now * 5.0) as i64 != ((now - time.delta_secs_f64()) * 5.0) as i64 {
                let bytes = encode(&S2C::Start { match_id: server.match_id, cfg });
                server.send_all(&bytes);
            }
        }
    }
}

/// Ввод клиентов в машины.
fn server_inputs(server: Res<Server>, mut cars: Query<(&Car, &mut CarInput, &mut Combat)>) {
    for (car, mut input, mut combat) in cars.iter_mut() {
        if let Controller::Remote(id) = car.controller {
            match server.clients.iter().find(|c| c.id == id) {
                Some(c) => {
                    *input = c.input;
                    combat.ping = c.ping;
                }
                None => *input = CarInput::default(),
            }
        }
    }
}

fn collect_events(mut server: ResMut<Server>, mut ev: MessageReader<GameEvent>, time: Res<Time<Real>>) {
    let now = time.elapsed_secs_f64();
    for e in ev.read() {
        let id = server.next_event;
        server.next_event += 1;
        server.events.push((id, now, e.clone()));
    }
    server.events.retain(|(_, t, e)| now - t < if cosmetic(e) { 0.02 } else { 0.25 });
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn server_send(
    mut server: ResMut<Server>,
    time: Res<Time<Real>>,
    state: Res<State<AppState>>,
    ms: Option<Res<MatchState>>,
    cars: Query<(&NetId, &Position, &Rotation, &LinearVelocity, &CarState, &Combat), With<Car>>,
    pickups: Query<&Pickup>,
    projs: Query<(&NetId, &Transform, &ProjVisual, Option<&Projectile>), Or<(With<Projectile>, With<Mine>, With<OilSlick>)>>,
    props: Query<(&Prop, &NetId, &Transform)>,
    peds: Query<(&Ped, &Transform)>,
) {
    if *state.get() != AppState::Race || server.clients.is_empty() {
        return;
    }
    let Some(ms) = ms else { return };
    let now = time.elapsed_secs_f64();
    if now - server.snap_t < 1.0 / 30.0 {
        return;
    }
    server.snap_t = now;
    server.tick += 1;
    let tick = server.tick;
    let mtime = ms.elapsed as f64 + if ms.phase == MatchPhase::Countdown { -(ms.countdown as f64) } else { 0.0 };
    let mut packets: Vec<Vec<u8>> = Vec::new();
    // машины
    let snaps: Vec<CarSnap> = cars
        .iter()
        .map(|(id, p, r, v, st, c)| CarSnap { id: *id, pos: p.0, rot: r.0, vel: v.0, state: st.clone(), combat: c.clone() })
        .collect();
    packets.extend(chunk(&snaps, |cars| S2C::Cars { tick, time: mtime, cars }));
    // мир
    let pk: Vec<(u16, bool, PowerupKind)> = pickups.iter().map(|p| (p.index as u16, p.active, p.kind)).collect();
    let pr: Vec<ProjSnap> = projs
        .iter()
        .map(|(id, t, v, p)| ProjSnap { id: id.0, kind: v.0, pos: t.translation, vel: p.map(|p| p.vel).unwrap_or(Vec3::ZERO) })
        .collect();
    packets.push(encode(&S2C::World { tick, time: mtime, ms: ms.clone(), pickups: pk, projs: pr.iter().take(40).copied().collect() }));
    if pr.len() > 40 {
        // остаток снарядов отдельными пакетами мира без пикапов
        for part in pr[40..].chunks(40) {
            packets.push(encode(&S2C::World { tick, time: mtime, ms: ms.clone(), pickups: Vec::new(), projs: part.to_vec() }));
        }
    }
    // события
    let evs: Vec<(u32, GameEvent)> = server.events.iter().map(|(i, _, e)| (*i, e.clone())).collect();
    if !evs.is_empty() {
        packets.extend(chunk(&evs, S2C::Events));
    }
    // сдвинутые пропсы
    let mut moved = Vec::new();
    let mut alive_ids = HashSet::new();
    for (p, id, t) in props.iter() {
        alive_ids.insert(id.0);
        server.known_props.insert(id.0);
        let last = server.prop_last.get(&id.0).copied();
        let changed = match last {
            None => false,
            Some((lp, lr)) => lp.distance(t.translation) > 0.02 || lr.angle_between(t.rotation) > 0.02,
        };
        if last.is_none() {
            server.prop_last.insert(id.0, (t.translation, t.rotation));
        } else if changed {
            server.prop_last.insert(id.0, (t.translation, t.rotation));
            moved.push(PropSnap { id: id.0, pos: t.translation, rot: t.rotation });
        }
        let _ = p;
    }
    if !moved.is_empty() {
        packets.extend(chunk(&moved, S2C::Props));
    }
    if now - server.dead_t > 0.5 {
        server.dead_t = now;
        let dead: Vec<u32> = server.known_props.iter().filter(|i| !alive_ids.contains(i)).copied().collect();
        if !dead.is_empty() {
            packets.extend(chunk(&dead, S2C::DeadProps));
        }
    }
    // пешеходы 10 Гц
    if now - server.ped_t > 0.1 {
        server.ped_t = now;
        let list: Vec<PedSnap> = peds
            .iter()
            .map(|(p, t)| {
                let (yaw, _, _) = t.rotation.to_euler(EulerRot::YXZ);
                PedSnap::pack(p.id, t.translation, yaw)
            })
            .collect();
        packets.extend(chunk(&list, S2C::Peds));
    }
    for p in &packets {
        server.send_all(p);
    }
}

fn back_to_lobby(server: Res<Server>, mut screen: ResMut<MenuScreen>) {
    server.send_all(&encode(&S2C::ToLobby));
    if *screen == MenuScreen::Main {
        *screen = MenuScreen::Lobby;
    }
}
