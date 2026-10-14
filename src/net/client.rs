//! Клиент: подключение, лобби, отправка ввода, приём снапшотов и плавная интерполяция.

use super::protocol::*;
use super::*;
use crate::combat::Combat;
use crate::config::*;
use crate::game::*;
use crate::maps::Prop;
use crate::peds::Ped;
use crate::powerups::Pickup;
use crate::race::{LocalInput, MyClientId};
use crate::ui::menu::{MenuDirty, MenuScreen};
use crate::vehicle::*;
use crate::weapons::ProjVisual;
use std::collections::{HashMap, HashSet, VecDeque};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};

/// Задержка отображения: рендерим прошлое, чтобы всегда было между чем интерполировать.
const INTERP_DELAY: f64 = 0.1;

/// Буфер снапшотов для плавного движения удалённых объектов.
#[derive(Component, Default)]
pub struct NetInterp {
    pub samples: Vec<(f64, Vec3, Quat)>,
}

impl NetInterp {
    pub fn push(&mut self, t: f64, p: Vec3, r: Quat) {
        if let Some(last) = self.samples.last() {
            if t <= last.0 {
                if (t - last.0).abs() < 1e-6 {
                    return;
                }
                // время пошло назад (новый матч) — сбрасываем
                self.samples.clear();
            }
        }
        self.samples.push((t, p, r));
        if self.samples.len() > 12 {
            self.samples.remove(0);
        }
    }
    pub fn sample(&self, t: f64) -> Option<(Vec3, Quat)> {
        let s = &self.samples;
        if s.is_empty() {
            return None;
        }
        if t <= s[0].0 {
            return Some((s[0].1, s[0].2));
        }
        for w in s.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            if t >= a.0 && t <= b.0 {
                let k = ((t - a.0) / (b.0 - a.0).max(1e-6)) as f32;
                return Some((a.1.lerp(b.1, k), a.2.slerp(b.2, k)));
            }
        }
        // экстраполяция не дальше 0.25 с
        let n = s.len();
        if n >= 2 {
            let (a, b) = (&s[n - 2], &s[n - 1]);
            let dt = (b.0 - a.0).max(1e-3);
            let ahead = (t - b.0).min(0.25);
            let v = (b.1 - a.1) / dt as f32;
            return Some((b.1 + v * ahead as f32, b.2));
        }
        Some((s[n - 1].1, s[n - 1].2))
    }
}

/// Сглаживание пешеходов (10 Гц).
#[derive(Component)]
pub struct PedTarget {
    pub pos: Vec3,
    pub yaw: f32,
}

/// Снаряд, который видит клиент.
#[derive(Component)]
pub struct ClientProj {
    pub vel: Vec3,
    pub last_seen: f64,
}

#[derive(Resource)]
pub struct Client {
    pub socket: UdpSocket,
    pub server: SocketAddr,
    pub id: Option<u64>,
    pub last_recv: f64,
    pub started: f64,
    pub hello_t: f64,
    pub ping_t: f64,
    pub input_seq: u32,
    pub match_id: u32,
    pub seen: HashSet<u32>,
    pub seen_order: VecDeque<u32>,
    pub match_time: f64,
    pub match_time_local: f64,
    pub rtt: u16,
    pub config_dirty: bool,
}

impl Client {
    fn send(&self, msg: &C2S) {
        let _ = self.socket.send_to(&encode(msg), self.server);
    }
    /// Оценка текущего времени матча на сервере.
    pub fn server_time(&self, now: f64) -> f64 {
        self.match_time + (now - self.match_time_local)
    }
}

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, client_recv.run_if(resource_exists::<Client>))
            .add_systems(Update, (client_commands, client_interp.run_if(resource_exists::<Client>)))
            .add_systems(PostUpdate, client_send.run_if(resource_exists::<Client>));
    }
}

fn resolve(addr: &str, port: u16) -> Option<SocketAddr> {
    let a = addr.trim();
    let full = if a.contains(':') { a.to_string() } else { format!("{}:{}", a, port) };
    full.to_socket_addrs().ok()?.find(|s| s.is_ipv4())
}

#[allow(clippy::too_many_arguments)]
fn client_commands(
    mut commands: Commands,
    mut ev: MessageReader<NetCommand>,
    client: Option<ResMut<Client>>,
    mut mode: ResMut<NetMode>,
    mut status: ResMut<NetStatus>,
    settings: Res<Settings>,
    time: Res<Time<Real>>,
) {
    let mut client = client;
    for cmd in ev.read() {
        match cmd {
            NetCommand::Join(ip) => {
                if *mode == NetMode::Host {
                    status.err("Сначала останови свой сервер");
                    continue;
                }
                let Some(server) = resolve(ip, settings.port) else {
                    status.err(format!("Не понимаю адрес «{}»", ip));
                    continue;
                };
                match UdpSocket::bind("0.0.0.0:0") {
                    Ok(sock) => {
                        let _ = sock.set_nonblocking(true);
                        let now = time.elapsed_secs_f64();
                        commands.insert_resource(Client {
                            socket: sock,
                            server,
                            id: None,
                            last_recv: now,
                            started: now,
                            hello_t: 0.0,
                            ping_t: 0.0,
                            input_seq: 0,
                            match_id: 0,
                            seen: HashSet::new(),
                            seen_order: VecDeque::new(),
                            match_time: 0.0,
                            match_time_local: now,
                            rtt: 0,
                            config_dirty: false,
                        });
                        *mode = NetMode::Client;
                        status.ok(format!("Подключение к {}...", server));
                    }
                    Err(e) => status.err(format!("Ошибка сети: {}", e)),
                }
            }
            NetCommand::Leave => {
                if let Some(c) = client.as_ref() {
                    c.send(&C2S::Bye);
                    commands.remove_resource::<Client>();
                    *mode = NetMode::Offline;
                    status.ok("Отключено");
                }
            }
            NetCommand::ConfigChanged => {
                if let Some(c) = client.as_mut() {
                    c.config_dirty = true;
                }
            }
            _ => {}
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn client_recv(
    mut commands: Commands,
    mut client: ResMut<Client>,
    time: Res<Time<Real>>,
    (mut status, mut lobby, mut mode): (ResMut<NetStatus>, ResMut<LobbyInfo>, ResMut<NetMode>),
    (mut screen, mut dirty): (ResMut<MenuScreen>, ResMut<MenuDirty>),
    (mut cfg, mut my_id): (ResMut<MatchConfig>, ResMut<MyClientId>),
    (state, mut next): (Res<State<AppState>>, ResMut<NextState<AppState>>),
    ms: Option<ResMut<MatchState>>,
    mut cars: Query<(&NetId, &mut CarState, &mut Combat, &mut NetInterp), With<Car>>,
    mut pickups: Query<&mut Pickup>,
    mut props: Query<(Entity, &NetId, &mut NetInterp), (With<Prop>, Without<Car>)>,
    mut peds: Query<(Entity, &Ped, Option<&mut PedTarget>)>,
    mut projs: Query<(Entity, &NetId, &mut ClientProj, &mut Transform)>,
    mut events: MessageWriter<GameEvent>,
) {
    let now = time.elapsed_secs_f64();
    let in_race = *state.get() == AppState::Race;
    let mut ms = ms;
    let mut buf = [0u8; 4096];
    let mut prop_index: Option<HashMap<u32, Entity>> = None;
    let mut ped_index: Option<HashMap<u32, Entity>> = None;
    loop {
        let (n, from) = match client.socket.recv_from(&mut buf) {
            Ok(x) => x,
            Err(_) => break,
        };
        if from != client.server {
            continue;
        }
        let Some(msg) = decode::<S2C>(&buf[..n]) else { continue };
        client.last_recv = now;
        match msg {
            S2C::Welcome { id } => {
                if client.id.is_none() {
                    client.id = Some(id);
                    my_id.0 = id;
                    status.ok("Подключено. Ждём, пока хост начнёт.");
                    *screen = MenuScreen::Lobby;
                    dirty.0 = true;
                }
            }
            S2C::Reject { reason } => {
                status.err(format!("Сервер отказал: {}", reason));
                commands.remove_resource::<Client>();
                *mode = NetMode::Offline;
                return;
            }
            S2C::Lobby(l) => {
                if *lobby != l {
                    *lobby = l;
                }
            }
            S2C::Start { match_id, cfg: c } => {
                if match_id != client.match_id && client.id.is_some() {
                    client.match_id = match_id;
                    client.seen.clear();
                    client.seen_order.clear();
                    *cfg = c;
                    next.set(AppState::Loading);
                }
            }
            S2C::Cars { time: t, cars: list, .. } => {
                if !in_race {
                    continue;
                }
                client.match_time = t;
                client.match_time_local = now;
                for s in list {
                    for (id, mut st, mut cb, mut interp) in cars.iter_mut() {
                        if *id == s.id {
                            if *st != s.state {
                                *st = s.state.clone();
                            }
                            if *cb != s.combat {
                                *cb = s.combat.clone();
                            }
                            interp.push(t, s.pos, s.rot);
                        }
                    }
                }
            }
            S2C::World { ms: m, pickups: pk, projs: pr, .. } => {
                if !in_race {
                    continue;
                }
                if let Some(cur) = ms.as_mut() {
                    **cur = m;
                }
                for (idx, active, kind) in pk {
                    for mut p in pickups.iter_mut() {
                        if p.index == idx as u32 && (p.active != active || p.kind != kind) {
                            p.active = active;
                            p.kind = kind;
                        }
                    }
                }
                for s in pr {
                    let mut found = false;
                    for (_, id, mut cp, mut tf) in projs.iter_mut() {
                        if id.0 == s.id {
                            found = true;
                            cp.vel = s.vel;
                            cp.last_seen = now;
                            tf.translation = s.pos;
                            if s.vel.length_squared() > 0.1 {
                                tf.look_to(s.vel.normalize(), Vec3::Y);
                            }
                        }
                    }
                    if !found {
                        let mut tf = Transform::from_translation(s.pos);
                        if s.vel.length_squared() > 0.1 {
                            tf.look_to(s.vel.normalize(), Vec3::Y);
                        }
                        commands.spawn((tf, ProjVisual(s.kind), NetId(s.id), ClientProj { vel: s.vel, last_seen: now }, Visibility::default(), DespawnOnExit(AppState::Race)));
                    }
                }
            }
            S2C::Events(list) => {
                if !in_race {
                    continue;
                }
                for (id, e) in list {
                    if client.seen.insert(id) {
                        client.seen_order.push_back(id);
                        if client.seen_order.len() > 4000 {
                            if let Some(old) = client.seen_order.pop_front() {
                                client.seen.remove(&old);
                            }
                        }
                        events.write(e);
                    }
                }
            }
            S2C::Props(list) => {
                if !in_race {
                    continue;
                }
                let idx = prop_index.get_or_insert_with(|| props.iter().map(|(e, id, _)| (id.0, e)).collect());
                let t = client.match_time;
                for s in list {
                    if let Some(e) = idx.get(&s.id) {
                        if let Ok((_, _, mut interp)) = props.get_mut(*e) {
                            interp.push(t, s.pos, s.rot);
                        }
                    }
                }
            }
            S2C::DeadProps(list) => {
                let idx = prop_index.get_or_insert_with(|| props.iter().map(|(e, id, _)| (id.0, e)).collect());
                for id in list {
                    if let Some(e) = idx.remove(&id) {
                        commands.entity(e).despawn();
                    }
                }
            }
            S2C::Peds(list) => {
                if !in_race {
                    continue;
                }
                let idx = ped_index.get_or_insert_with(|| peds.iter().map(|(e, p, _)| (p.id, e)).collect());
                for s in list {
                    if let Some(e) = idx.get(&(s.id as u32)) {
                        if let Ok((_, _, target)) = peds.get_mut(*e) {
                            match target {
                                Some(mut t) => {
                                    t.pos = s.pos();
                                    t.yaw = s.yaw();
                                }
                                None => {
                                    commands.entity(*e).insert(PedTarget { pos: s.pos(), yaw: s.yaw() });
                                }
                            }
                        }
                    }
                }
            }
            S2C::Pong { t } => {
                client.rtt = ((now - t) * 1000.0).clamp(0.0, 9999.0) as u16;
            }
            S2C::ToLobby => {
                if *state.get() != AppState::Menu {
                    *screen = MenuScreen::Lobby;
                    next.set(AppState::Menu);
                }
            }
            S2C::Bye { reason } => {
                status.err(reason);
                commands.remove_resource::<Client>();
                *mode = NetMode::Offline;
                *screen = MenuScreen::Multiplayer;
                if *state.get() != AppState::Menu {
                    next.set(AppState::Menu);
                }
                return;
            }
        }
    }
    // снаряды, которых давно нет в снапшотах, удаляем
    for (e, _, cp, _) in projs.iter() {
        if now - cp.last_seen > 0.35 {
            commands.entity(e).despawn();
        }
    }
    // потеря связи
    let silent = now - client.last_recv;
    if (client.id.is_none() && now - client.started > 8.0) || silent > 8.0 {
        status.err(if client.id.is_none() { "Сервер не отвечает. Проверь адрес и порт." } else { "Связь с сервером потеряна" });
        commands.remove_resource::<Client>();
        *mode = NetMode::Offline;
        *screen = MenuScreen::Multiplayer;
        dirty.0 = true;
        if *state.get() != AppState::Menu {
            next.set(AppState::Menu);
        }
    }
}

fn client_send(mut client: ResMut<Client>, time: Res<Time<Real>>, profile: Res<Profile>, input: Res<LocalInput>, state: Res<State<AppState>>) {
    let now = time.elapsed_secs_f64();
    if client.id.is_none() {
        if now - client.hello_t > 0.4 {
            client.hello_t = now;
            client.send(&C2S::Hello { version: PROTOCOL_VERSION, name: profile.name.clone(), config: profile.selected_config() });
        }
        return;
    }
    if client.config_dirty || profile.is_changed() {
        client.config_dirty = false;
        client.send(&C2S::Config { name: profile.name.clone(), config: profile.selected_config() });
    }
    if *state.get() == AppState::Race {
        client.input_seq += 1;
        let seq = client.input_seq;
        client.send(&C2S::Input { seq, input: input.0 });
    }
    if now - client.ping_t > 1.0 {
        client.ping_t = now;
        let rtt = client.rtt;
        client.send(&C2S::Ping { t: now, rtt });
    }
}

/// Плавное движение машин, пропсов, пешеходов и снарядов у клиента.
#[allow(clippy::type_complexity)]
fn client_interp(
    client: Res<Client>,
    time: Res<Time<Real>>,
    state: Res<State<AppState>>,
    mut q: Query<(&NetInterp, &mut Transform), Without<PedTarget>>,
    mut peds: Query<(&PedTarget, &mut Transform), Without<NetInterp>>,
    mut projs: Query<(&ClientProj, &mut Transform), (Without<NetInterp>, Without<PedTarget>)>,
) {
    if *state.get() != AppState::Race {
        return;
    }
    let now = time.elapsed_secs_f64();
    let dt = time.delta_secs();
    let t = client.server_time(now) - INTERP_DELAY;
    for (interp, mut tf) in q.iter_mut() {
        if let Some((p, r)) = interp.sample(t) {
            tf.translation = p;
            tf.rotation = r;
        }
    }
    let k = 1.0 - (-12.0 * dt).exp();
    for (target, mut tf) in peds.iter_mut() {
        if tf.translation.distance(target.pos) > 6.0 {
            tf.translation = target.pos;
        } else {
            tf.translation = tf.translation.lerp(target.pos, k);
        }
        tf.rotation = tf.rotation.slerp(Quat::from_rotation_y(target.yaw), k);
    }
    for (cp, mut tf) in projs.iter_mut() {
        tf.translation += cp.vel * dt;
    }
}
