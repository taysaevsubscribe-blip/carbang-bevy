//! CarBang: аркадные гонки на выживание. Bevy 0.19 + avian3d 0.7.

#![allow(clippy::too_many_arguments, clippy::type_complexity)]
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod ai;
mod audio;
mod balance;
mod camera;
mod carfit;
mod carkit;
mod carmodel;
mod combat;
mod config;
mod data;
mod debug;
mod effects;
mod extassets;
mod game;
mod kitbox;
mod gfx;
mod hanabi_fx;
mod maps;
mod meshgen;
mod net;
mod peds;
mod powerups;
mod race;
mod rules;
mod tests;
mod ui;
mod vehicle;
mod weapons;

use avian3d::prelude::*;
use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy::render::settings::{RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;
use bevy::window::{PresentMode, WindowMode};
use bevy::winit::WinitPlugin;
use config::*;
use game::*;

/// Параметры командной строки.
#[derive(Resource, Clone, Default, Debug)]
pub struct Cli {
    pub test: Option<String>,
    pub headless: bool,
    pub host: bool,
    pub connect: Option<String>,
    pub map: Option<String>,
    pub mode: Option<String>,
    pub car: Option<String>,
    pub secs: f32,
    pub bots: usize,
    pub quick: bool,
    pub name: Option<String>,
    pub screenshot: Option<String>,
    pub menu: Option<String>,
    pub tab: Option<String>,
    pub autopilot: bool,
    /// Размер окна (скриншоты 1920×1080)
    pub res: Option<(u32, u32)>,
}

fn parse_cli() -> Cli {
    let mut cli = Cli { secs: 60.0, bots: 7, ..default() };
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        let a = args[i].as_str();
        let val = args.get(i + 1).cloned();
        match a {
            "--test" => {
                cli.test = val;
                i += 1;
            }
            "--headless" => cli.headless = true,
            "--host" => cli.host = true,
            "--quick" => cli.quick = true,
            "--autopilot" => cli.autopilot = true,
            "--connect" => {
                cli.connect = val;
                i += 1;
            }
            "--map" => {
                cli.map = val;
                i += 1;
            }
            "--mode" => {
                cli.mode = val;
                i += 1;
            }
            "--car" => {
                cli.car = val;
                i += 1;
            }
            "--name" => {
                cli.name = val;
                i += 1;
            }
            "--secs" => {
                cli.secs = val.and_then(|v| v.parse().ok()).unwrap_or(60.0);
                i += 1;
            }
            "--bots" => {
                cli.bots = val.and_then(|v| v.parse().ok()).unwrap_or(7);
                i += 1;
            }
            "--menu" => {
                cli.menu = val;
                i += 1;
            }
            "--tab" => {
                cli.tab = val;
                i += 1;
            }
            "--res" => {
                cli.res = val.as_deref().and_then(|v| v.split_once('x')).and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)));
                i += 1;
            }
            "--screenshot" => {
                cli.screenshot = val;
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }
    cli
}

fn main() {
    let cli = parse_cli();
    let headless = cli.headless;
    let settings = Settings::load();
    let profile = if headless { Profile::default() } else { Profile::load() };

    let mut app = App::new();
    if headless {
        app.add_plugins(
            DefaultPlugins
                .build()
                .disable::<WinitPlugin>()
                .disable::<bevy::audio::AudioPlugin>()
                .set(RenderPlugin { render_creation: RenderCreation::Automatic(Box::new(WgpuSettings { backends: None, ..default() })), ..default() })
                .set(bevy::log::LogPlugin { level: bevy::log::Level::WARN, ..default() }),
        )
        // Без GPU-бэкенда Bevy 0.19 не добавляет синхронизацию с render world,
        // а хуки компонентов света и мешей её требуют.
        .add_plugins(bevy::render::sync_world::SyncWorldPlugin);
        let realtime = matches!(cli.test.as_deref(), Some("nethost") | Some("netclient"));
        if realtime {
            app.add_plugins(ScheduleRunnerPlugin::run_loop(std::time::Duration::from_secs_f64(1.0 / 120.0)));
        } else {
            app.add_plugins(ScheduleRunnerPlugin::run_loop(std::time::Duration::ZERO))
                .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(1.0 / 120.0)));
        }
    } else {
        app.add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "CarBang".into(),
                        resolution: { let (w, h) = cli.res.unwrap_or((1600, 900)); bevy::window::WindowResolution::new(w, h) },
                        present_mode: PresentMode::AutoVsync,
                        mode: if settings.fullscreen { WindowMode::BorderlessFullscreen(MonitorSelection::Current) } else { WindowMode::Windowed },
                        ..default()
                    }),
                    ..default()
                })
                .set(bevy::log::LogPlugin { level: bevy::log::Level::INFO, filter: "wgpu=error,naga=warn,bevy_render=warn".into(), ..default() }),
        );
    }

    app.add_plugins(PhysicsPlugins::default().set(PhysicsInterpolationPlugin::interpolate_all()))
        .insert_resource(Time::<Fixed>::from_hz(120.0))
        .init_state::<AppState>()
        .add_message::<GameEvent>()
        .insert_resource(NetMode::Offline)
        .insert_resource(Headless(headless))
        .insert_resource(settings)
        .insert_resource(profile)
        .insert_resource(cli.clone())
        .init_resource::<NetIdAlloc>()
        .init_resource::<LocalPlayer>()
        .init_resource::<MatchResult>()
        .init_resource::<balance::DynDifficulty>()
        .init_resource::<ui::menu::MenuScreen>()
        .init_resource::<ui::menu::MenuDirty>()
        .init_resource::<ui::garage::GarageState>()
        .insert_resource(MatchConfig::default())
        .configure_sets(FixedUpdate, (SimSet::Vehicle, SimSet::Weapons, SimSet::Combat, SimSet::Rules).chain())
        .add_plugins((
            vehicle::VehiclePlugin,
            weapons::WeaponsPlugin,
            combat::CombatPlugin,
            peds::PedsPlugin,
            powerups::PowerupsPlugin,
            ai::AiPlugin,
            rules::RulesPlugin,
            maps::MapsPlugin,
            effects::EffectsPlugin,
            carmodel::CarModelPlugin,
            race::RacePlugin,
            camera::CameraPlugin,
            net::NetPlugin,
            tests::TestsPlugin,
            extassets::ExtAssetsPlugin,
        ));
    if !headless {
        app.add_plugins((ui::UiPlugin, audio::AudioPlugin, gfx::GfxPlugin, debug::DebugPlugin, hanabi_fx::HanabiFxPlugin)).insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.07)));
    }
    app.run();
}
