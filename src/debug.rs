//! Отладка: оверлеи по клавишам и лог кадра.
//! F1 — скорости и удары (Δv) своей машины; F2 — состояние и цель ИИ над машинами;
//! F3 — коллайдеры avian и зоны урона. Раз в 5 с — строка `[perf]` в лог.

use crate::ai::{Bot, Goal};
use crate::game::*;
use crate::config::CarStats;
use crate::vehicle::*;
use avian3d::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct DebugOverlays {
    pub speeds: bool,
    pub ai: bool,
    pub colliders: bool,
}

/// Последние удары своей машины для F1: (время, Δv).
#[derive(Resource, Default)]
struct RecentImpacts(Vec<(f32, f32)>);

#[derive(Component)]
struct F1Panel;
#[derive(Component)]
struct AiLabel(Entity);

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }
        if !app.is_plugin_added::<PhysicsDebugPlugin>() {
            app.add_plugins(PhysicsDebugPlugin);
        }
        app.init_resource::<DebugOverlays>()
            .init_resource::<RecentImpacts>()
            .add_systems(Startup, disable_physics_gizmos)
            .add_systems(Update, (toggle_overlays, perf_log))
            .add_systems(Update, (f1_panel, f2_ai_labels, f3_zones, collect_impacts).run_if(in_state(AppState::Race)));
    }
}

fn disable_physics_gizmos(mut store: ResMut<GizmoConfigStore>, mut ov: ResMut<DebugOverlays>) {
    store.config_mut::<PhysicsGizmos>().0.enabled = false;
    // для скриншотов отчёта оверлеи можно включить переменными окружения
    ov.speeds = std::env::var("CARBANG_F1").is_ok();
    ov.ai = std::env::var("CARBANG_F2").is_ok();
}

fn toggle_overlays(keys: Res<ButtonInput<KeyCode>>, mut ov: ResMut<DebugOverlays>, mut store: ResMut<GizmoConfigStore>) {
    if keys.just_pressed(KeyCode::F1) {
        ov.speeds = !ov.speeds;
    }
    if keys.just_pressed(KeyCode::F2) {
        ov.ai = !ov.ai;
    }
    if keys.just_pressed(KeyCode::F3) {
        ov.colliders = !ov.colliders;
        store.config_mut::<PhysicsGizmos>().0.enabled = ov.colliders;
    }
}

/// Раз в 5 с: средний и худший кадр (CPU), FPS. GPU-время без профайлера недоступно.
fn perf_log(diag: Res<DiagnosticsStore>, time: Res<Time<Real>>, mut acc: Local<(f32, f32, u32, f32)>) {
    let dt = time.delta_secs();
    acc.0 += dt;
    acc.1 = acc.1.max(dt);
    acc.2 += 1;
    if time.elapsed_secs() - acc.3 >= 5.0 && acc.2 > 0 {
        let fps = diag.get(&FrameTimeDiagnosticsPlugin::FPS).and_then(|d| d.smoothed()).unwrap_or(0.0);
        info!("[perf] кадр ср {:.2} мс, худший {:.2} мс, FPS {:.0}, кадров {}", acc.0 / acc.2 as f32 * 1000.0, acc.1 * 1000.0, fps, acc.2);
        *acc = (0.0, 0.0, 0, time.elapsed_secs());
    }
}

fn collect_impacts(mut ev: MessageReader<GameEvent>, local: Res<LocalPlayer>, cars: Query<(&NetId, &Transform)>, time: Res<Time>, mut rec: ResMut<RecentImpacts>) {
    let me = local.car.and_then(|id| cars.iter().find(|c| *c.0 == id)).map(|c| c.1.translation);
    for e in ev.read() {
        if let (GameEvent::Crash { pos, strength }, Some(p)) = (e, me) {
            if pos.distance(p) < 4.0 {
                rec.0.push((time.elapsed_secs(), *strength));
            }
        }
    }
    let now = time.elapsed_secs();
    rec.0.retain(|r| now - r.0 < 6.0);
}

#[allow(clippy::too_many_arguments)]
fn f1_panel(
    mut commands: Commands,
    ov: Res<DebugOverlays>,
    local: Res<LocalPlayer>,
    cars: Query<(&NetId, &CarState, &CarInput)>,
    rec: Res<RecentImpacts>,
    mut panel: Query<(Entity, &mut Text), With<F1Panel>>,
    fonts: Option<Res<crate::ui::Fonts>>,
) {
    if !ov.speeds {
        for (e, _) in panel.iter() {
            commands.entity(e).despawn();
        }
        return;
    }
    let Some(st_in) = local.car.and_then(|id| cars.iter().find(|c| *c.0 == id)) else { return };
    let (_, st, inp) = st_in;
    let n = st.nodes();
    let impacts: Vec<String> = rec.0.iter().rev().take(5).map(|r| format!("{:.1}", r.1)).collect();
    let s = format!(
        "F1  скорость {:.1} км/ч  передача {}  газ {:.2} тормоз {:.2} руль {:.2}\nкузов {:.1}  броня {:.1}  защита {:.1} с  зоны [{:.0} {:.0} {:.0} {:.0}]\nузлы: радиатор {} двигатель {} колесо {}\nпоследние удары Δv, м/с: {}",
        st.speed * 3.6,
        match st.drive {
            -1 => "R".to_string(),
            0 => "N".to_string(),
            _ => format!("D{}", st.gear),
        },
        inp.throttle,
        inp.brake,
        inp.steer,
        st.hp,
        st.armor,
        st.spawn_protect,
        st.zones[0],
        st.zones[1],
        st.zones[2],
        st.zones[3],
        if n.radiator { "да" } else { "нет" },
        if n.engine { "да" } else { "нет" },
        match n.wheel {
            -1 => "лево",
            1 => "право",
            _ => "нет",
        },
        if impacts.is_empty() { "—".into() } else { impacts.join(", ") }
    );
    if let Ok((_, mut t)) = panel.single_mut() {
        if **t != s {
            **t = s;
        }
    } else if let Some(f) = fonts {
        commands.spawn((
            Text::new(s),
            TextFont { font: f.narrow.clone().into(), font_size: FontSize::Px(14.0), ..default() },
            TextColor(Color::srgb(0.6, 1.0, 0.7)),
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            Node { position_type: PositionType::Absolute, left: Val::Percent(24.0), top: Val::Px(14.0), padding: UiRect::all(Val::Px(6.0)), ..default() },
            GlobalZIndex(20),
            F1Panel,
            DespawnOnExit(AppState::Race),
        ));
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn f2_ai_labels(
    mut commands: Commands,
    ov: Res<DebugOverlays>,
    bots: Query<(Entity, &Bot, &Car, &CarState, &Transform)>,
    names: Query<&Car>,
    cam: Query<(&Camera, &GlobalTransform), With<crate::camera::ChaseCam>>,
    mut labels: Query<(Entity, &AiLabel, &mut Text, &mut Node, &mut Visibility)>,
    fonts: Option<Res<crate::ui::Fonts>>,
) {
    if !ov.ai {
        for (e, ..) in labels.iter() {
            commands.entity(e).despawn();
        }
        return;
    }
    let Ok((camera, cam_tf)) = cam.single() else { return };
    let mut seen = Vec::new();
    for (e, label, mut text, mut node, mut vis) in labels.iter_mut() {
        let Ok((_, bot, car, st, tf)) = bots.get(label.0) else {
            commands.entity(e).despawn();
            continue;
        };
        seen.push(label.0);
        let target = match bot.goal {
            Goal::Hunt(t) | Goal::Flee(t) => names.get(t).map(|c| c.name.clone()).unwrap_or_default(),
            _ => String::new(),
        };
        let s = format!("{} · {}{}  {:.0} км/ч", car.name, bot.state.label(), if target.is_empty() { String::new() } else { format!(" > {}", target) }, st.speed.abs() * 3.6);
        if **text != s {
            **text = s;
        }
        match camera.world_to_viewport(cam_tf, tf.translation + Vec3::Y * 2.6) {
            Ok(p) if !st.wrecked => {
                node.left = Val::Px(p.x - 90.0);
                node.top = Val::Px(p.y - 34.0);
                *vis = Visibility::Inherited;
            }
            _ => *vis = Visibility::Hidden,
        }
    }
    let Some(f) = fonts else { return };
    for (e, ..) in bots.iter() {
        if !seen.contains(&e) {
            commands.spawn((
                Text::new(""),
                TextFont { font: f.bold.clone().into(), font_size: FontSize::Px(13.0), ..default() },
                TextColor(Color::srgb(1.0, 0.9, 0.4)),
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
                Node { position_type: PositionType::Absolute, padding: UiRect::axes(Val::Px(4.0), Val::Px(1.0)), ..default() },
                GlobalZIndex(19),
                AiLabel(e),
                Visibility::Hidden,
                DespawnOnExit(AppState::Race),
            ));
        }
    }
}

/// F3: зоны урона рамками вокруг машин (зелёная — цела, красная — 60+ урона).
fn f3_zones(ov: Res<DebugOverlays>, cars: Query<(&CarState, &CarStats, &Transform)>, mut gizmos: Gizmos) {
    if !ov.colliders {
        return;
    }
    for (st, stats, tf) in cars.iter() {
        if st.wrecked {
            continue;
        }
        let hw = stats.size.x * 0.5 + 0.15;
        let hl = stats.size.z * 0.5 + 0.15;
        let y = 0.3;
        let col = |d: f32| {
            let k = (d / 60.0).clamp(0.0, 1.0);
            Color::srgb(0.3 + 0.7 * k, 1.0 - 0.8 * k, 0.3)
        };
        let p = |x: f32, z: f32| tf.transform_point(Vec3::new(x, y, z));
        gizmos.line(p(-hw, -hl), p(hw, -hl), col(st.zones[0]));
        gizmos.line(p(-hw, hl), p(hw, hl), col(st.zones[1]));
        gizmos.line(p(-hw, -hl), p(-hw, hl), col(st.zones[2]));
        gizmos.line(p(hw, -hl), p(hw, hl), col(st.zones[3]));
    }
}
