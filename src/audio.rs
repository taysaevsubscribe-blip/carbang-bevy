//! Звук: файлы из пакета ассетов (assets/sounds: моторы, удары, взрывы, крики, диктор)
//! поверх синтезированных в коде (шины, пулемёт, бонусы, интерфейс). Нет файлов — играет синтез.
//! Атрибуция CC-BY: см. assets/sounds/CREDITS.txt и README.

use crate::camera::ChaseCam;
use crate::config::Settings;
use crate::game::*;
use crate::ui::UiClick;
use crate::vehicle::*;
use bevy::audio::{AudioSinkPlayback, PlaybackMode, Volume};
use bevy::prelude::*;
use std::f32::consts::TAU;

const RATE: u32 = 22050;

#[derive(Resource)]
pub struct Sfx {
    pub engine: Handle<AudioSource>,
    pub engine_big: Handle<AudioSource>,
    pub skid: Handle<AudioSource>,
    pub crash: Handle<AudioSource>,
    pub crash_big: Handle<AudioSource>,
    pub explosion: Handle<AudioSource>,
    pub gun: Handle<AudioSource>,
    pub rocket: Handle<AudioSource>,
    pub pickup: Handle<AudioSource>,
    pub splat: Handle<AudioSource>,
    pub scream: Handle<AudioSource>,
    pub horn: Handle<AudioSource>,
    pub click: Handle<AudioSource>,
    pub beep: Handle<AudioSource>,
    pub go: Handle<AudioSource>,
    pub emp: Handle<AudioSource>,
    pub splash: Handle<AudioSource>,
    pub land: Handle<AudioSource>,
    pub flame: Handle<AudioSource>,
    pub drop: Handle<AudioSource>,
    pub chime: Handle<AudioSource>,
}

/// Звуки из файлов. Пусто, если папки assets/sounds нет.
#[derive(Resource, Default)]
pub struct FileSfx {
    pub engines: Vec<Handle<AudioSource>>,
    pub engine_heavy: Option<Handle<AudioSource>>,
    pub metal_heavy: Vec<Handle<AudioSource>>,
    pub metal_medium: Vec<Handle<AudioSource>>,
    pub metal_light: Vec<Handle<AudioSource>>,
    pub soft: Vec<Handle<AudioSource>>,
    pub glass: Vec<Handle<AudioSource>>,
    pub explosion: Vec<Handle<AudioSource>>,
    pub rocket: Vec<Handle<AudioSource>>,
    pub emp: Vec<Handle<AudioSource>>,
    pub mine_click: Vec<Handle<AudioSource>>,
    pub mine_beep: Vec<Handle<AudioSource>>,
    pub screams_male: Vec<Handle<AudioSource>>,
    pub screams_female: Vec<Handle<AudioSource>>,
    pub voice: Vec<(&'static str, Handle<AudioSource>)>,
}

impl FileSfx {
    fn voice(&self, k: &str) -> Option<Handle<AudioSource>> {
        self.voice.iter().find(|v| v.0 == k).map(|v| v.1.clone())
    }
}

/// Отложенный звук (слой металла после взрыва через 0.2 с).
#[derive(Component)]
struct DelayedSound {
    t: f32,
    h: Handle<AudioSource>,
    vol: f32,
    pitch: f32,
}

#[derive(Component)]
struct EngineSound(Entity);

/// Луп мотора из файла (питч по оборотам 1.0…2.2) или синтезированный.
#[derive(Component)]
struct EngineKind {
    file: bool,
    offset: f32,
}

#[derive(Component)]
struct SkidSound;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (make_sounds, load_file_sounds))
            .add_systems(Update, (master_volume, ui_clicks, delayed_sounds))
            .add_systems(OnEnter(AppState::Results), result_voice)
            .add_systems(Update, (attach_engines, engine_sounds, skid_sound, event_sounds).run_if(in_state(AppState::Race)));
    }
}

// ============================================================ синтез

struct Synth {
    s: Vec<f32>,
    seed: u32,
}

impl Synth {
    fn new(secs: f32) -> Self {
        Self { s: vec![0.0; (secs * RATE as f32) as usize], seed: 0x1234_5678 }
    }
    fn noise(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
    fn len(&self) -> usize {
        self.s.len()
    }
    fn t(i: usize) -> f32 {
        i as f32 / RATE as f32
    }
    /// Нормализация к заданному пику и упаковка в WAV.
    fn wav(mut self, peak: f32) -> AudioSource {
        let m = self.s.iter().fold(0.0f32, |a, b| a.max(b.abs())).max(1e-6);
        for v in self.s.iter_mut() {
            *v = (*v / m * peak).clamp(-1.0, 1.0);
        }
        let n = self.s.len();
        let mut out = Vec::with_capacity(44 + n * 2);
        let data_len = (n * 2) as u32;
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&RATE.to_le_bytes());
        out.extend_from_slice(&(RATE * 2).to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        for v in self.s {
            out.extend_from_slice(&((v * 32000.0) as i16).to_le_bytes());
        }
        AudioSource { bytes: out.into() }
    }
}

/// Однополюсный фильтр низких частот.
fn lowpass(x: &mut [f32], cutoff: f32) {
    let a = (TAU * cutoff / RATE as f32) / (1.0 + TAU * cutoff / RATE as f32);
    let mut y = 0.0;
    for v in x.iter_mut() {
        y += a * (*v - y);
        *v = y;
    }
}

fn highpass(x: &mut [f32], cutoff: f32) {
    let mut lp = x.to_vec();
    lowpass(&mut lp, cutoff);
    for (v, l) in x.iter_mut().zip(lp) {
        *v -= l;
    }
}

/// Мотор: зацикленный фрагмент, частота кратна длине, чтобы шов был незаметен.
fn engine(base: f32, cylinders: f32, rough: f32) -> AudioSource {
    let secs = 1.0;
    let mut s = Synth::new(secs);
    let f = (base * secs).round() / secs;
    for i in 0..s.len() {
        let t = Synth::t(i);
        let ph = t * f * TAU;
        // импульсы зажигания: сумма гармоник с убыванием
        let mut v = 0.0;
        for h in 1..9 {
            let amp = 1.0 / (h as f32).powf(1.1);
            v += (ph * h as f32 + (h as f32 * 0.7)).sin() * amp;
        }
        let fire = ((ph * cylinders / 2.0).sin() * 0.5 + 0.5).powf(3.0);
        let n = s.noise() * rough;
        s.s[i] = v * (0.55 + 0.45 * fire) + n * fire;
    }
    lowpass(&mut s.s, 1800.0);
    s.wav(0.8)
}

fn make_sounds(mut commands: Commands, mut audio: ResMut<Assets<AudioSource>>) {
    let mut add = |a: AudioSource| audio.add(a);

    let engine_s = add(engine(46.0, 4.0, 0.35));
    let engine_big = add(engine(31.0, 8.0, 0.5));

    // Визг шин: полосовой шум с лёгкой модуляцией
    let mut s = Synth::new(1.0);
    for i in 0..s.len() {
        let t = Synth::t(i);
        s.s[i] = s.noise() * (0.7 + 0.3 * (t * TAU * 7.0).sin());
    }
    highpass(&mut s.s, 900.0);
    lowpass(&mut s.s, 3200.0);
    let skid = add(s.wav(0.6));

    // Удар: глухой низ + хруст
    let crash_gen = |len: f32, low: f32, seed: u32| {
        let mut s = Synth::new(len);
        s.seed = seed;
        for i in 0..s.len() {
            let t = Synth::t(i);
            let env = (-t * 9.0 / len).exp();
            let thump = (t * TAU * low * (1.0 - t * 0.8)).sin() * (-t * 14.0).exp();
            let crunch = s.noise() * env * if (t * 37.0).fract() < 0.5 { 1.0 } else { 0.4 };
            s.s[i] = thump * 1.2 + crunch * 0.8;
        }
        lowpass(&mut s.s, 2600.0);
        s.wav(0.95)
    };
    let crash = add(crash_gen(0.35, 70.0, 77));
    let crash_big = add(crash_gen(0.8, 45.0, 99));

    // Взрыв
    let mut s = Synth::new(2.2);
    s.seed = 4242;
    for i in 0..s.len() {
        let t = Synth::t(i);
        let env = (-t * 2.2).exp();
        let crack = if t < 0.05 { s.noise() * 1.5 } else { 0.0 };
        s.s[i] = s.noise() * env + crack + (t * TAU * 38.0).sin() * (-t * 3.5).exp() * 0.8;
    }
    lowpass(&mut s.s, 900.0);
    let explosion = add(s.wav(1.0));

    // Выстрел пулемёта
    let mut s = Synth::new(0.09);
    s.seed = 5;
    for i in 0..s.len() {
        let t = Synth::t(i);
        s.s[i] = s.noise() * (-t * 60.0).exp() + (t * TAU * 140.0).sin() * (-t * 45.0).exp();
    }
    lowpass(&mut s.s, 4000.0);
    let gun = add(s.wav(0.7));

    // Ракета: шипящий запуск
    let mut s = Synth::new(0.7);
    s.seed = 17;
    for i in 0..s.len() {
        let t = Synth::t(i);
        let env = (t * 30.0).min(1.0) * (-t * 3.0).exp();
        s.s[i] = s.noise() * env;
    }
    highpass(&mut s.s, 500.0);
    lowpass(&mut s.s, 5000.0);
    let rocket = add(s.wav(0.7));

    // Бонус: арпеджио
    let mut s = Synth::new(0.42);
    for i in 0..s.len() {
        let t = Synth::t(i);
        let note = if t < 0.12 { 660.0 } else if t < 0.24 { 880.0 } else { 1320.0 };
        let local = t % 0.12;
        s.s[i] = ((t * TAU * note).sin() + 0.3 * (t * TAU * note * 2.0).sin()) * (-local * 12.0).exp();
    }
    let pickup = add(s.wav(0.55));

    // Сбитый пешеход: мокрый шлепок
    let mut s = Synth::new(0.3);
    s.seed = 31;
    for i in 0..s.len() {
        let t = Synth::t(i);
        s.s[i] = (t * TAU * 95.0).sin() * (-t * 18.0).exp() * 1.2 + s.noise() * (-t * 25.0).exp() * 0.7;
    }
    lowpass(&mut s.s, 1500.0);
    let splat = add(s.wav(0.85));

    // Крик: падающий тон с вибрато и формантой
    let mut s = Synth::new(0.55);
    s.seed = 1001;
    let mut ph = 0.0f32;
    for i in 0..s.len() {
        let t = Synth::t(i);
        let f = 620.0 - t * 500.0 + (t * TAU * 9.0).sin() * 25.0;
        ph += f / RATE as f32 * TAU;
        let env = (t * 25.0).min(1.0) * (1.0 - t / 0.55).max(0.0);
        let v = ph.sin() + 0.5 * (ph * 2.0).sin() + 0.25 * (ph * 3.0).sin();
        s.s[i] = (v + s.noise() * 0.15) * env;
    }
    lowpass(&mut s.s, 2400.0);
    let scream = add(s.wav(0.5));

    // Гудок
    let mut s = Synth::new(0.6);
    for i in 0..s.len() {
        let t = Synth::t(i);
        let sq = |f: f32| if (t * f).fract() < 0.5 { 1.0 } else { -1.0 };
        let env = (t * 40.0).min(1.0) * ((0.6 - t) * 20.0).min(1.0);
        s.s[i] = (sq(370.0) + sq(466.0)) * 0.5 * env;
    }
    lowpass(&mut s.s, 2000.0);
    let horn = add(s.wav(0.5));

    let tone = |f: f32, len: f32, decay: f32| {
        let mut s = Synth::new(len);
        for i in 0..s.len() {
            let t = Synth::t(i);
            s.s[i] = (t * TAU * f).sin() * (-t * decay).exp() * (t * 200.0).min(1.0);
        }
        s.wav(0.5)
    };
    let click = add(tone(1800.0, 0.04, 80.0));
    let beep = add(tone(880.0, 0.18, 10.0));
    let go = add(tone(1320.0, 0.5, 4.0));

    // ЭМИ: падающая пила
    let mut s = Synth::new(0.6);
    let mut ph = 0.0f32;
    for i in 0..s.len() {
        let t = Synth::t(i);
        let f = 1600.0 * (-t * 4.0).exp() + 80.0;
        ph += f / RATE as f32;
        s.s[i] = (ph.fract() * 2.0 - 1.0) * (-t * 3.0).exp();
    }
    lowpass(&mut s.s, 3000.0);
    let emp = add(s.wav(0.5));

    // Всплеск
    let mut s = Synth::new(0.8);
    s.seed = 555;
    for i in 0..s.len() {
        let t = Synth::t(i);
        s.s[i] = s.noise() * (-t * 4.0).exp() * (t * 50.0).min(1.0);
    }
    lowpass(&mut s.s, 1200.0);
    let splash = add(s.wav(0.7));

    // Приземление
    let mut s = Synth::new(0.25);
    s.seed = 66;
    for i in 0..s.len() {
        let t = Synth::t(i);
        s.s[i] = (t * TAU * 55.0).sin() * (-t * 16.0).exp() + s.noise() * (-t * 30.0).exp() * 0.4;
    }
    lowpass(&mut s.s, 900.0);
    let land = add(s.wav(0.8));

    // Огнемёт
    let mut s = Synth::new(0.25);
    s.seed = 808;
    for i in 0..s.len() {
        let t = Synth::t(i);
        s.s[i] = s.noise() * (t * 30.0).min(1.0) * ((0.25 - t) * 12.0).min(1.0);
    }
    lowpass(&mut s.s, 700.0);
    let flame = add(s.wav(0.5));

    // Сброс мины
    let drop = add(tone(220.0, 0.15, 25.0));

    // Чекпоинт
    let mut s = Synth::new(0.5);
    for i in 0..s.len() {
        let t = Synth::t(i);
        let f = if t < 0.15 { 988.0 } else { 1318.0 };
        s.s[i] = (t * TAU * f).sin() * (-(t % 0.15) * 6.0).exp();
    }
    let chime = add(s.wav(0.5));

    commands.insert_resource(Sfx {
        engine: engine_s,
        engine_big,
        skid,
        crash,
        crash_big,
        explosion,
        gun,
        rocket,
        pickup,
        splat,
        scream,
        horn,
        click,
        beep,
        go,
        emp,
        splash,
        land,
        flame,
        drop,
        chime,
    });
}

// ============================================================ файлы

fn load_file_sounds(mut commands: Commands, assets: Res<AssetServer>) {
    let Some(dir) = crate::extassets::assets_dir() else {
        commands.insert_resource(FileSfx::default());
        return;
    };
    let exists = |rel: &str| dir.join(rel).exists();
    if !exists("sounds") {
        warn!("нет папки {}: звук только синтезированный", dir.join("sounds").display());
        commands.insert_resource(FileSfx::default());
        return;
    }
    let set = |pat: &str, n: usize| -> Vec<Handle<AudioSource>> {
        (0..n)
            .map(|i| pat.replace("{}", &format!("{:03}", i)))
            .filter(|p| {
                let ok = dir.join(p).exists();
                if !ok {
                    error!("нет звука {}", dir.join(p).display());
                }
                ok
            })
            .map(|p| assets.load(p))
            .collect()
    };
    let list = |paths: &[&str]| -> Vec<Handle<AudioSource>> { paths.iter().filter(|p| dir.join(p).exists()).map(|p| assets.load(p.to_string())).collect() };
    let mut fs = FileSfx {
        engines: list(&["sounds/engine/engine_loop_0.ogg", "sounds/engine/engine_loop_1_0.ogg", "sounds/engine/engine_loop_2_0.ogg"]),
        engine_heavy: list(&["sounds/engine/engine_heavy_loop.ogg"]).into_iter().next(),
        metal_heavy: set("sounds/impact/impactMetal_heavy_{}.ogg", 5),
        metal_medium: set("sounds/impact/impactMetal_medium_{}.ogg", 5),
        metal_light: set("sounds/impact/impactMetal_light_{}.ogg", 5),
        soft: set("sounds/impact/impactSoft_heavy_{}.ogg", 5),
        glass: set("sounds/impact/impactGlass_heavy_{}.ogg", 5),
        explosion: set("sounds/scifi/lowFrequency_explosion_{}.ogg", 2),
        rocket: set("sounds/scifi/thrusterFire_{}.ogg", 5),
        emp: set("sounds/scifi/forceField_{}.ogg", 5),
        mine_click: set("sounds/scifi/doorClose_{}.ogg", 3),
        mine_beep: set("sounds/scifi/computerNoise_{}.ogg", 4),
        screams_male: list(&[
            "sounds/screams/male0_0.ogg",
            "sounds/screams/male0_1.ogg",
            "sounds/screams/male1_0.ogg",
            "sounds/screams/male1_1.ogg",
            "sounds/screams/male2_0.ogg",
            "sounds/screams/male2_1.ogg",
            "sounds/screams/male3_0.ogg",
            "sounds/screams/male3_1.ogg",
            "sounds/screams/male4_0.ogg",
            "sounds/screams/male4_1.ogg",
        ]),
        screams_female: list(&["sounds/screams/female1.ogg", "sounds/screams/female2.ogg", "sounds/screams/female3.ogg", "sounds/screams/female4.ogg"]),
        voice: Vec::new(),
    };
    for k in ["1", "2", "3", "go", "ready", "you_win", "you_lose", "game_over", "war_target_destroyed", "hurry_up"] {
        let p = format!("sounds/voice/{k}.ogg");
        if dir.join(&p).exists() {
            fs.voice.push((k, assets.load(p)));
        }
    }
    commands.insert_resource(fs);
}

/// Диктор на экране итогов: победа или поражение.
fn result_voice(mut commands: Commands, files: Option<Res<FileSfx>>, result: Res<MatchResult>) {
    let Some(f) = files else { return };
    if let Some(h) = f.voice(if result.won { "you_win" } else { "you_lose" }) {
        commands.spawn((AudioPlayer(h), PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.85))));
    }
}

fn delayed_sounds(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut DelayedSound)>) {
    for (e, mut d) in q.iter_mut() {
        d.t -= time.delta_secs();
        if d.t <= 0.0 {
            commands.entity(e).despawn();
            commands.spawn((AudioPlayer(d.h.clone()), PlaybackSettings::DESPAWN.with_volume(Volume::Linear(d.vol)).with_speed(d.pitch)));
        }
    }
}

// ============================================================ воспроизведение

fn master_volume(settings: Res<Settings>, mut global: ResMut<GlobalVolume>) {
    if settings.is_changed() {
        *global = GlobalVolume::new(Volume::Linear(settings.volume));
    }
}

fn ui_clicks(mut commands: Commands, mut ev: MessageReader<UiClick>, sfx: Option<Res<Sfx>>) {
    let Some(sfx) = sfx else { return };
    if ev.read().count() > 0 {
        commands.spawn((AudioPlayer(sfx.click.clone()), PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.5))));
    }
}

fn attach_engines(mut commands: Commands, sfx: Option<Res<Sfx>>, files: Option<Res<FileSfx>>, cars: Query<(Entity, &Car), Added<Car>>) {
    let Some(sfx) = sfx else { return };
    for (e, car) in cars.iter() {
        let big = car.def().mass > 2600.0;
        // три лупа из пакета по машинам; Мамонт и Каток — тяжёлый луп
        let idx = crate::data::CARS.iter().position(|d| d.id == car.def().id).unwrap_or(0);
        let heavy = matches!(car.def().id, "monster" | "roller");
        let file = files.as_ref().and_then(|f| if heavy { f.engine_heavy.clone() } else if f.engines.is_empty() { None } else { Some(f.engines[idx % f.engines.len()].clone()) });
        let using_file = file.is_some();
        commands.spawn((
            EngineKind { file: using_file, offset: ((idx * 37) % 9) as f32 * 0.02 - 0.08 },
            AudioPlayer(file.unwrap_or_else(|| if big { sfx.engine_big.clone() } else { sfx.engine.clone() })),
            PlaybackSettings { mode: PlaybackMode::Loop, volume: Volume::Linear(0.0), ..default() },
            EngineSound(e),
            DespawnOnExit(AppState::Race),
        ));
    }
}

fn listener_pos(cam: &Query<&GlobalTransform, With<ChaseCam>>) -> Vec3 {
    cam.single().map(|t| t.translation()).unwrap_or(Vec3::ZERO)
}

fn atten(d: f32, near: f32) -> f32 {
    1.0 / (1.0 + (d / near).powf(1.4))
}

fn engine_sounds(
    mut commands: Commands,
    local: Res<LocalPlayer>,
    cam: Query<&GlobalTransform, With<ChaseCam>>,
    cars: Query<(&NetId, &CarState, &Transform, &CarInput)>,
    mut sinks: Query<(Entity, &EngineSound, &EngineKind, Option<&mut AudioSink>)>,
    paused: Res<crate::race::Paused>,
) {
    let lp = listener_pos(&cam);
    for (e, es, kind, sink) in sinks.iter_mut() {
        let Ok((id, st, tf, input)) = cars.get(es.0) else {
            commands.entity(e).despawn();
            continue;
        };
        let Some(mut sink) = sink else { continue };
        let mine = Some(*id) == local.car;
        let d = tf.translation.distance(lp);
        let mut vol = if mine { 0.38 } else { 0.5 * atten(d, 12.0) };
        if st.wrecked {
            vol = 0.0;
        }
        let load = if mine { 0.7 + 0.3 * input.throttle } else { 0.85 };
        if paused.0 {
            vol = 0.0;
        }
        sink.set_volume(Volume::Linear(vol * load));
        if kind.file {
            // PlaybackSettings::speed = 0.8 + обороты (900…6700 → 0.2…1.4), в пределах 1.0…2.2
            let rpm_k = ((st.rpm - 900.0) / 5800.0).clamp(0.0, 1.0) * 1.4 + 0.2;
            sink.set_speed((0.8 + rpm_k + kind.offset).clamp(1.0, 2.2) * if st.nitro_active { 1.05 } else { 1.0 });
        } else {
            sink.set_speed((st.rpm / 2400.0).clamp(0.35, 3.2) * if st.nitro_active { 1.08 } else { 1.0 });
        }
    }
}

fn skid_sound(
    mut commands: Commands,
    sfx: Option<Res<Sfx>>,
    local: Res<LocalPlayer>,
    cars: Query<(&NetId, &CarState)>,
    mut sink: Query<&mut AudioSink, With<SkidSound>>,
    mut spawned: Local<bool>,
    paused: Res<crate::race::Paused>,
) {
    let Some(sfx) = sfx else { return };
    if sink.is_empty() {
        if !*spawned {
            *spawned = true;
            commands.spawn((
                AudioPlayer(sfx.skid.clone()),
                PlaybackSettings { mode: PlaybackMode::Loop, volume: Volume::Linear(0.0), ..default() },
                SkidSound,
                DespawnOnExit(AppState::Race),
            ));
        }
        return;
    }
    *spawned = false;
    let Ok(mut sink) = sink.single_mut() else { return };
    let st = local.car.and_then(|id| cars.iter().find(|c| *c.0 == id)).map(|c| c.1);
    let slip = st
        .map(|s| {
            if s.wrecked || s.speed.abs() < 3.0 {
                0.0
            } else {
                s.wheels.iter().filter(|w| w.grounded).map(|w| w.slip).fold(0.0, f32::max)
            }
        })
        .unwrap_or(0.0);
    let v = ((slip - 0.12) * 1.6).clamp(0.0, 0.45);
    sink.set_volume(Volume::Linear(if paused.0 { 0.0 } else { v }));
}

#[allow(clippy::too_many_arguments)]
fn event_sounds(
    mut commands: Commands,
    sfx: Option<Res<Sfx>>,
    mut ev: MessageReader<GameEvent>,
    cam: Query<&GlobalTransform, With<ChaseCam>>,
    local: Res<LocalPlayer>,
    time: Res<Time<Real>>,
    mut last: Local<[f32; 8]>,
    files: Option<Res<FileSfx>>,
) {
    let fsx = files.as_deref();
    let pick = |v: &Vec<Handle<AudioSource>>, r: &mut rand::rngs::ThreadRng| -> Option<Handle<AudioSource>> {
        use rand::Rng;
        if v.is_empty() {
            None
        } else {
            Some(v[r.gen_range(0..v.len())].clone())
        }
    };
    let Some(sfx) = sfx else {
        ev.clear();
        return;
    };
    let lp = listener_pos(&cam);
    let now = time.elapsed_secs();
    let mut rng = rand::thread_rng();
    use rand::Rng;
    let play = |commands: &mut Commands, h: &Handle<AudioSource>, vol: f32, pitch: f32| {
        if vol < 0.01 {
            return;
        }
        commands.spawn((AudioPlayer(h.clone()), PlaybackSettings::DESPAWN.with_volume(Volume::Linear(vol)).with_speed(pitch)));
    };
    // ограничитель частоты по каналам
    let mut gate = |ch: usize, gap: f32| -> bool {
        if now - last[ch] < gap {
            false
        } else {
            last[ch] = now;
            true
        }
    };
    for e in ev.read() {
        match e {
            GameEvent::Explosion { pos, radius } => {
                let v = atten(pos.distance(lp), 30.0) * (0.6 + radius * 0.06);
                // взрыв: низкий бум из пакета + слой металла через 0.2 с
                if let Some(h) = fsx.and_then(|f| pick(&f.explosion, &mut rng)) {
                    play(&mut commands, &h, v.min(1.0), rng.gen_range(0.85..1.05));
                    if let Some(m) = fsx.and_then(|f| pick(&f.metal_heavy, &mut rng)) {
                        commands.spawn((DelayedSound { t: 0.2, h: m, vol: (v * 0.6).min(1.0), pitch: rng.gen_range(0.8..1.0) }, DespawnOnExit(AppState::Race)));
                    }
                } else {
                    play(&mut commands, &sfx.explosion, v.min(1.0), rng.gen_range(0.85..1.1));
                }
            }
            GameEvent::Crash { pos, strength } => {
                // удар: громкость по Δv, при |Δv| < 3 м/с не играем
                if *strength >= 3.0 && gate(0, 0.06) {
                    let v = atten(pos.distance(lp), 15.0) * (strength / 14.0).clamp(0.15, 1.0);
                    let set = fsx.map(|f| if *strength > 11.0 { &f.metal_heavy } else if *strength > 6.0 { &f.metal_medium } else { &f.metal_light });
                    if let Some(h) = set.and_then(|s| pick(s, &mut rng)) {
                        play(&mut commands, &h, v, rng.gen_range(0.9..1.15));
                        if *strength > 12.0 {
                            if let Some(g) = fsx.and_then(|f| pick(&f.glass, &mut rng)) {
                                play(&mut commands, &g, v * 0.5, rng.gen_range(0.9..1.1));
                            }
                        }
                    } else {
                        let h = if *strength > 9.0 { &sfx.crash_big } else { &sfx.crash };
                        play(&mut commands, h, v, rng.gen_range(0.85..1.15));
                    }
                }
            }
            GameEvent::WeaponFired { kind, pos, .. } => {
                use crate::data::WeaponKind::*;
                let v = atten(pos.distance(lp), 18.0);
                match kind {
                    MachineGun => {
                        if gate(1, 0.05) {
                            play(&mut commands, &sfx.gun, v * 0.6, rng.gen_range(0.9..1.1));
                        }
                    }
                    Rockets | Homing => match fsx.and_then(|f| pick(&f.rocket, &mut rng)) {
                        Some(h) => play(&mut commands, &h, v * 0.8, rng.gen_range(0.9..1.1)),
                        None => play(&mut commands, &sfx.rocket, v * 0.8, rng.gen_range(0.9..1.1)),
                    },
                    Mines => {
                        // мина: щелчок и писк взвода
                        match fsx.and_then(|f| pick(&f.mine_click, &mut rng)) {
                            Some(h) => play(&mut commands, &h, v * 0.8, 1.2),
                            None => play(&mut commands, &sfx.drop, v * 0.7, 1.0),
                        }
                        if let Some(h) = fsx.and_then(|f| pick(&f.mine_beep, &mut rng)) {
                            commands.spawn((DelayedSound { t: 0.25, h, vol: v * 0.5, pitch: 1.4 }, DespawnOnExit(AppState::Race)));
                        }
                    }
                    Bomb | Oil => play(&mut commands, &sfx.drop, v * 0.7, 1.0),
                    Emp => match fsx.and_then(|f| pick(&f.emp, &mut rng)) {
                        Some(h) => play(&mut commands, &h, v, 0.9),
                        None => play(&mut commands, &sfx.emp, v, 1.0),
                    },
                    Flamer => {
                        if gate(2, 0.2) {
                            play(&mut commands, &sfx.flame, v * 0.8, rng.gen_range(0.8..1.2));
                        }
                    }
                    _ => {}
                }
            }
            GameEvent::PedKilled { pos, .. } => {
                let v = atten(pos.distance(lp), 14.0);
                match fsx.and_then(|f| pick(&f.soft, &mut rng)) {
                    Some(h) => play(&mut commands, &h, v, rng.gen_range(0.8..1.1)),
                    None => play(&mut commands, &sfx.splat, v, rng.gen_range(0.8..1.2)),
                }
                if gate(3, 0.15) {
                    // крик: мужские и женские голоса, питч 1.1–1.4, пространственный звук
                    let female = rng.gen::<f32>() < 0.4;
                    let h = fsx.and_then(|f| pick(if female { &f.screams_female } else { &f.screams_male }, &mut rng));
                    match h {
                        Some(h) => {
                            commands.spawn((
                                AudioPlayer(h),
                                PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.9)).with_speed(rng.gen_range(1.1..1.4)).with_spatial(true),
                                Transform::from_translation(*pos),
                            ));
                        }
                        None => play(&mut commands, &sfx.scream, v * 0.7, rng.gen_range(0.75..1.4)),
                    }
                }
            }
            GameEvent::Pickup { car, pos, .. } => {
                let v = if Some(*car) == local.car { 0.7 } else { atten(pos.distance(lp), 12.0) * 0.5 };
                play(&mut commands, &sfx.pickup, v, 1.0);
            }
            GameEvent::Horn { pos } => {
                if gate(4, 0.5) {
                    play(&mut commands, &sfx.horn, atten(pos.distance(lp), 20.0) * 0.6, 1.0);
                }
            }
            GameEvent::Emp { pos, .. } => play(&mut commands, &sfx.emp, atten(pos.distance(lp), 25.0), 0.8),
            GameEvent::Splash { pos } => {
                if gate(5, 0.3) {
                    play(&mut commands, &sfx.splash, atten(pos.distance(lp), 20.0), 1.0);
                }
            }
            GameEvent::Landed { car, air } => {
                if Some(*car) == local.car {
                    play(&mut commands, &sfx.land, (air * 0.4).clamp(0.3, 0.9), 1.0);
                }
            }
            GameEvent::Message { to, text, big, .. } => {
                let mine = to.is_none() || *to == local.car;
                if mine && *big {
                    if text.chars().all(|c| c.is_ascii_digit()) {
                        play(&mut commands, &sfx.beep, 0.5, 1.0);
                        if let Some(h) = fsx.and_then(|f| f.voice(text.as_str())) {
                            play(&mut commands, &h, 0.8, 1.0);
                        }
                    } else if text.starts_with("ВПЕРЁД") {
                        play(&mut commands, &sfx.go, 0.6, 1.0);
                        if let Some(h) = fsx.and_then(|f| f.voice("go")) {
                            play(&mut commands, &h, 0.9, 1.0);
                        }
                    } else if text.contains("РАЗБИТ!") {
                        if let Some(h) = fsx.and_then(|f| f.voice("war_target_destroyed")) {
                            play(&mut commands, &h, 0.7, 1.0);
                        }
                    } else if text.starts_with("МАШИНА УНИЧТОЖЕНА") {
                        if let Some(h) = fsx.and_then(|f| f.voice("game_over")) {
                            play(&mut commands, &h, 0.8, 1.0);
                        }
                    } else if text.starts_with("КРУГ") || text.starts_with("ВСЕ КРУГИ") {
                        play(&mut commands, &sfx.chime, 0.6, 1.0);
                    }
                } else if mine && text.starts_with("КОНТРОЛЬНАЯ") {
                    play(&mut commands, &sfx.chime, 0.5, 1.0);
                }
            }
            GameEvent::CarWrecked { pos, .. } => {
                play(&mut commands, &sfx.crash_big, atten(pos.distance(lp), 25.0), 0.7);
            }
            GameEvent::BulletHit { pos, .. } => {
                if gate(6, 0.07) {
                    play(&mut commands, &sfx.crash, atten(pos.distance(lp), 8.0) * 0.25, 2.2);
                }
            }
            _ => {}
        }
    }
}
