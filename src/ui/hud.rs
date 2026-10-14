//! HUD заезда: таймер, очки, урон по зонам, скорость, нитро, оружие, миникарта,
//! лента убийств, сообщения, захват цели, таблица, пауза, экран гибели.

use super::*;
use crate::camera::ChaseCam;
use crate::combat::Combat;
use crate::config::Settings;
use crate::data::*;
use crate::game::*;
use crate::maps::MapData;
use crate::race::{Paused, UiToggles};
use crate::vehicle::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use std::collections::HashMap;

#[derive(Component)]
struct HudRoot;
#[derive(Component)]
struct HudTime;
#[derive(Component)]
struct HudScore;
#[derive(Component)]
struct HudInfo;
#[derive(Component)]
struct HudSpeed;
#[derive(Component)]
struct HudGear;
#[derive(Component)]
struct HudNitro;
#[derive(Component)]
struct HudHp;
#[derive(Component)]
struct HudHpText;
#[derive(Component)]
struct HudZone(usize);
#[derive(Component)]
struct HudWeapons;
#[derive(Component)]
struct HudEffects;
#[derive(Component)]
struct HudBig;
#[derive(Component)]
struct HudSmall;
#[derive(Component)]
struct HudFeed;
#[derive(Component)]
struct HudMinimap;
#[derive(Component)]
struct MiniDot(u32);
#[derive(Component)]
struct HudLock;
#[derive(Component)]
struct HudTags;
#[derive(Component)]
struct HudBoard;
#[derive(Component)]
struct HudPause;
#[derive(Component)]
struct HudDead;
#[derive(Component)]
struct HudDeadText;
#[derive(Component)]
struct HudFps;
#[derive(Component)]
struct HudHint;

#[derive(Component, Clone, Copy)]
pub enum HudAction {
    Resume,
    Quit,
}

#[derive(Resource, Default)]
struct HudMessages {
    big: Option<(String, [f32; 3], f32)>,
    small: Vec<(String, [f32; 3], f32)>,
    feed: Vec<(String, f32)>,
    weapons_sig: String,
    effects_sig: String,
    board_sig: String,
    my_name: String,
}

#[derive(Resource)]
struct MiniMapInfo {
    half: Vec2,
    size: Vec2,
}

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }
        app.init_resource::<HudMessages>()
            .init_resource::<WeaponIcons>()
            .add_systems(Update, ui_scale)
            .add_systems(OnEnter(AppState::Race), spawn_hud.after(crate::camera::spawn_camera))
            .add_systems(
                Update,
                (
                    hud_events,
                    hud_top,
                    hud_vehicle,
                    hud_weapons,
                    hud_effects,
                    hud_messages,
                    hud_minimap,
                    hud_world_overlays,
                    hud_board,
                    hud_pause,
                    hud_dead,
                    hud_fps,
                    hud_actions,
                )
                    .run_if(in_state(AppState::Race)),
            );
    }
}

fn local_car<'a>(local: &LocalPlayer, cars: impl Iterator<Item = (&'a NetId, &'a CarState, &'a Combat)>) -> Option<(&'a CarState, &'a Combat)> {
    let id = local.car?;
    cars.into_iter().find(|(n, ..)| **n == id).map(|(_, s, c)| (s, c))
}

fn minimap_image(map: &MapData) -> Image {
    let w = 256usize;
    let h = ((256.0 * map.half.y / map.half.x).round() as usize).clamp(64, 512);
    let mut data = vec![0u8; w * h * 4];
    for j in 0..h {
        for i in 0..w {
            let x = (i as f32 + 0.5) / w as f32 * 2.0 * map.half.x - map.half.x;
            let z = (j as f32 + 0.5) / h as f32 * 2.0 * map.half.y - map.half.y;
            let p = Vec3::new(x, -100.0, z);
            let gh = map.ground_height(x, z);
            let water = map.water.iter().any(|wz| wz.contains(Vec2::new(x, z)));
            let blocked = map.is_blocked(x, z);
            let c: [u8; 4] = if water {
                [30, 60, 110, 210]
            } else if blocked {
                [22, 22, 26, 220]
            } else {
                let g = (110.0 + gh * 3.0).clamp(70.0, 200.0) as u8;
                [g, g, (g as f32 * 1.05).min(255.0) as u8, 200]
            };
            let _ = p;
            let k = (j * w + i) * 4;
            data[k..k + 4].copy_from_slice(&c);
        }
    }
    Image::new(
        Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

// ============================================================ HUD v2 по макету hud_16x9.html
// Экран 1600×900, остальные разрешения — через UiScale (высота/900).

// стилизация: плотные панели с толстым тёмным контуром, как у плашек всплывашек
const GLASS: Color = Color::srgba(0.11, 0.11, 0.13, 0.88);
const GLASS_EDGE: Color = Color::srgba(0.01, 0.01, 0.015, 0.95);
const LABEL: Color = Color::srgb(0.60, 0.58, 0.56);
const HUD_ORANGE: Color = Color::srgb(1.0, 0.42, 0.29);
const HUD_GREEN: Color = Color::srgb(0.56, 0.84, 0.58);
const HUD_GOLD: Color = Color::srgb(1.0, 0.82, 0.48);
/// Спидометр: развёртка 240° (-210°..+30°), 0..220 км/ч, цифры на R=76, стрелка 50 px.
const G_MAX: f32 = 220.0;
const G_A0: f32 = -210.0;
const G_A1: f32 = 30.0;
const G_CX: f32 = 108.0;
const G_CY: f32 = 106.0;
const G_R: f32 = 84.0;
const G_RTXT: f32 = 76.0;
const G_RARC: f32 = 64.0;
const ARC_SEGS: usize = 44;
/// Радар: радиус обзора в метрах и в пикселях.
const RADAR_RANGE: f32 = 110.0;
const RADAR_R: f32 = 76.0;

#[derive(Component)]
struct HudKills;
#[derive(Component)]
struct HudAlive;
#[derive(Component)]
struct HudExtraLabel;
#[derive(Component)]
struct HudArmor;
#[derive(Component)]
struct HudNums(u8);
#[derive(Component)]
struct HudCarName;
#[derive(Component)]
struct HudCarSub;
#[derive(Component)]
struct HudAvatar;
#[derive(Component)]
struct HudWreckLine;
#[derive(Component)]
struct HudNeedle;
#[derive(Component)]
struct HudArcSeg(f32);
#[derive(Component)]
struct HudNitroPct;
#[derive(Component)]
struct HudSlot(usize);
#[derive(Component)]
struct HudSlotIcon(usize);
#[derive(Component)]
struct HudSlotAmmo(usize);
#[derive(Component)]
struct HudSlotKey(usize);
#[derive(Component)]
struct HudSlotCd(usize);
#[derive(Component)]
struct RadarMap;
#[derive(Component)]
struct RadarSweep;

/// Иконки оружия (assets/ui/icons), если пакет на месте.
#[derive(Resource, Default)]
struct WeaponIcons(Vec<(WeaponKind, Handle<Image>)>);

fn gauge_angle(v: f32) -> f32 {
    (G_A0 + (G_A1 - G_A0) * (v / G_MAX).clamp(0.0, 1.0)).to_radians()
}

fn glass_node(node: Node) -> (Node, BackgroundColor, BorderColor) {
    (Node { border: UiRect::all(Val::Px(3.0)), border_radius: BorderRadius::all(Val::Px(14.0)), ..node }, BackgroundColor(GLASS), BorderColor::all(GLASS_EDGE))
}

/// Горизонтальный градиент для полос (кузов, броня, нитро).
fn gradient_image(stops: &[(f32, [f32; 3])]) -> Image {
    let w = 128usize;
    let mut data = vec![0u8; w * 4];
    for i in 0..w {
        let t = i as f32 / (w - 1) as f32;
        let mut c = stops[0].1;
        for k in 0..stops.len() - 1 {
            let (t0, c0) = stops[k];
            let (t1, c1) = stops[k + 1];
            if t >= t0 && t <= t1 {
                let f = (t - t0) / (t1 - t0).max(1e-4);
                c = [c0[0] + (c1[0] - c0[0]) * f, c0[1] + (c1[1] - c0[1]) * f, c0[2] + (c1[2] - c0[2]) * f];
            }
        }
        data[i * 4..i * 4 + 4].copy_from_slice(&[(c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8, 255]);
    }
    Image::new(Extent3d { width: w as u32, height: 1, depth_or_array_layers: 1 }, TextureDimension::D2, data, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD)
}

/// Сетка радара и сектор развёртки (152×152).
fn radar_overlay(sweep: bool) -> Image {
    let n = 152usize;
    let mut data = vec![0u8; n * n * 4];
    let c = n as f32 / 2.0;
    for y in 0..n {
        for x in 0..n {
            let dx = x as f32 + 0.5 - c;
            let dy = y as f32 + 0.5 - c;
            let r = (dx * dx + dy * dy).sqrt();
            let mut a = 0.0f32;
            if sweep {
                // яркий край на 0°, затухание на 80°
                let ang = dy.atan2(dx).to_degrees();
                let d = (ang + 360.0) % 360.0;
                if d < 80.0 && r < c {
                    a = 0.10 * (1.0 - d / 80.0);
                }
            } else {
                if (x % 18 == 0 || y % 18 == 0) && r < c {
                    a = 0.05;
                }
                if (r - c * 0.5).abs() < 0.6 {
                    a = 0.07;
                }
            }
            let k = (y * n + x) * 4;
            data[k..k + 4].copy_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
        }
    }
    Image::new(Extent3d { width: n as u32, height: n as u32, depth_or_array_layers: 1 }, TextureDimension::D2, data, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD)
}

fn weapon_icon_file(k: WeaponKind) -> &'static str {
    match k {
        WeaponKind::MachineGun => "machinegun",
        WeaponKind::Rockets => "rockets",
        WeaponKind::Homing => "homing",
        WeaponKind::Mines => "mines",
        WeaponKind::Bomb => "bomb",
        WeaponKind::Spikes => "spikes",
        WeaponKind::Plow => "plow",
        WeaponKind::Emp => "emp",
        WeaponKind::Oil => "oil",
        WeaponKind::Flamer => "flamer",
    }
}

fn short_key(t: crate::data::Trigger) -> &'static str {
    match t {
        crate::data::Trigger::Fire => "ЛКМ",
        crate::data::Trigger::Special => "E",
        crate::data::Trigger::Drop => "Q",
        crate::data::Trigger::Passive => "",
    }
}

/// Масштаб интерфейса: макет нарисован под высоту 900 px.
fn ui_scale(windows: Query<&Window>, mut scale: ResMut<UiScale>) {
    if let Ok(w) = windows.single() {
        let s = (w.height() / 900.0).clamp(0.6, 3.0);
        if (scale.0 - s).abs() > 0.01 {
            scale.0 = s;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_hud(mut commands: Commands, fonts: Res<Fonts>, map: Option<Res<MapData>>, mut images: ResMut<Assets<Image>>, mut msgs: ResMut<HudMessages>, headless: Res<Headless>) {
    if headless.0 {
        return;
    }
    *msgs = HudMessages::default();
    let f = fonts.clone();
    let (mini_img, half) = match map.as_ref() {
        Some(m) => (Some(images.add(minimap_image(m))), m.half),
        None => (None, Vec2::splat(100.0)),
    };
    let scale = RADAR_R / RADAR_RANGE;
    commands.insert_resource(MiniMapInfo { half, size: half * 2.0 * scale });
    let mut icons = WeaponIcons::default();
    for w in WEAPONS.iter() {
        if let Some(h) = crate::extassets::load_png(&mut images, &format!("ui/icons/{}.png", weapon_icon_file(w.kind)), true, false) {
            icons.0.push((w.kind, h));
        }
    }
    commands.insert_resource(icons);
    let hp_grad = images.add(gradient_image(&[(0.0, [1.0, 0.48, 0.18]), (0.55, [1.0, 0.81, 0.29]), (1.0, [0.49, 0.86, 0.53])]));
    let armor_grad = images.add(gradient_image(&[(0.0, [0.44, 0.70, 1.0]), (1.0, [0.73, 0.88, 1.0])]));
    let nitro_grad = images.add(gradient_image(&[(0.0, [0.18, 0.5, 1.0]), (1.0, [0.62, 0.82, 1.0])]));
    let grid = images.add(radar_overlay(false));
    let sweep = images.add(radar_overlay(true));
    let label = |p: &mut ChildSpawnerCommands, t: &str| {
        p.spawn((Text::new(t), TextFont { font: f.bold.clone().into(), font_size: FontSize::Px(10.0), ..default() }, TextColor(LABEL), TextLayout::justify(Justify::Left)));
    };
    commands
        .spawn((Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }, HudRoot, DespawnOnExit(AppState::Race)))
        .with_children(|root| {
            // ---------------- верх слева: таймер, выбито, в игре
            root.spawn(glass_node(Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(1.6),
                top: Val::Percent(2.2),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(11.0)),
                column_gap: Val::Px(22.0),
                align_items: AlignItems::Center,
                ..default()
            }))
            .with_children(|c| {
                c.spawn((Text::new("0:00"), TextFont { font: f.title.clone().into(), font_size: FontSize::Px(30.0), ..default() }, TextColor(pal::TEXT), HudTime));
                for (lbl, col, marker) in [("ВЫБИТО", HUD_ORANGE, 0u8), ("В ИГРЕ", HUD_GREEN, 1), ("ОЧКИ", HUD_GOLD, 2)] {
                    c.spawn(Node { flex_direction: FlexDirection::Column, ..default() }).with_children(|s| {
                        if marker == 2 {
                            s.spawn((Text::new(lbl), TextFont { font: f.bold.clone().into(), font_size: FontSize::Px(10.0), ..default() }, TextColor(LABEL), HudExtraLabel));
                        } else {
                            label(s, lbl);
                        }
                        let mut e = s.spawn((Text::new("0"), TextFont { font: f.title.clone().into(), font_size: FontSize::Px(20.0), ..default() }, TextColor(col)));
                        match marker {
                            0 => e.insert(HudKills),
                            1 => e.insert(HudAlive),
                            _ => e.insert(HudScore),
                        };
                    });
                }
            });
            // ---------------- верх справа: круглая миникарта-радар
            root.spawn(glass_node(Node { position_type: PositionType::Absolute, right: Val::Percent(1.6), top: Val::Percent(2.2), padding: UiRect::all(Val::Px(10.0)), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(4.0), ..default() }))
                .with_children(|c| {
                    c.spawn((
                        Node { width: Val::Px(152.0), height: Val::Px(152.0), border: UiRect::all(Val::Px(1.0)), border_radius: BorderRadius::all(Val::Percent(50.0)), overflow: Overflow::clip(), ..default() },
                        BackgroundColor(Color::srgb(0.08, 0.082, 0.094)),
                        BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.12)),
                        HudMinimap,
                    ))
                    .with_children(|m| {
                        if let Some(img) = mini_img.clone() {
                            m.spawn((ImageNode::new(img).with_color(Color::srgba(1.0, 1.0, 1.0, 0.45)).with_mode(bevy::ui::widget::NodeImageMode::Stretch), Node { position_type: PositionType::Absolute, width: Val::Px(half.x * 2.0 * scale), height: Val::Px(half.y * 2.0 * scale), ..default() }, RadarMap));
                        }
                        m.spawn((ImageNode::new(grid.clone()), Node { position_type: PositionType::Absolute, width: Val::Px(152.0), height: Val::Px(152.0), ..default() }));
                        m.spawn((ImageNode::new(sweep.clone()), Node { position_type: PositionType::Absolute, width: Val::Px(152.0), height: Val::Px(152.0), ..default() }, UiTransform::IDENTITY, RadarSweep));
                    });
                    c.spawn((Text::new(""), TextFont { font: f.body.clone().into(), font_size: FontSize::Px(12.0), ..default() }, TextColor(pal::DIM), HudFps));
                });
            // ---------------- лево-центр: 5 слотов навесного
            root.spawn(Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(1.6),
                top: Val::Percent(50.0),
                margin: UiRect::top(Val::Px(-161.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|c| {
                for i in 0..5 {
                    c.spawn((glass_node(Node { width: Val::Px(58.0), height: Val::Px(58.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, overflow: Overflow::clip(), ..default() }), HudSlot(i)))
                        .with_children(|s| {
                            s.spawn((Node { position_type: PositionType::Absolute, left: Val::Px(0.0), right: Val::Px(0.0), bottom: Val::Px(0.0), height: Val::Percent(0.0), ..default() }, BackgroundColor(Color::srgba(1.0, 0.35, 0.24, 0.30)), HudSlotCd(i)));
                            s.spawn((ImageNode::default(), Node { width: Val::Px(30.0), height: Val::Px(30.0), ..default() }, Visibility::Hidden, HudSlotIcon(i)));
                            s.spawn((Text::new(""), TextFont { font: f.bold.clone().into(), font_size: FontSize::Px(10.0), ..default() }, TextColor(Color::srgb(0.55, 0.53, 0.5)), Node { position_type: PositionType::Absolute, left: Val::Px(5.0), top: Val::Px(2.0), ..default() }, HudSlotKey(i)));
                            s.spawn((Text::new(""), TextFont { font: f.title.clone().into(), font_size: FontSize::Px(11.5), ..default() }, TextColor(HUD_GOLD), Node { position_type: PositionType::Absolute, right: Val::Px(5.0), bottom: Val::Px(2.0), ..default() }, HudSlotAmmo(i)));
                        });
                }
            });
            // ---------------- центр: большие и малые сообщения
            root.spawn(Node { position_type: PositionType::Absolute, top: Val::Percent(18.0), width: Val::Percent(100.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(6.0), ..default() })
                .with_children(|c| {
                    c.spawn((Text::new(""), TextFont { font: f.title.clone().into(), font_size: FontSize::Px(64.0), ..default() }, TextColor(pal::GOLD), TextLayout::justify(Justify::Center), HudBig));
                    c.spawn((Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(2.0), ..default() }, HudSmall));
                });
            // ---------------- низ слева: эффекты и строка повреждений над панелью машины
            root.spawn(Node { position_type: PositionType::Absolute, left: Val::Percent(1.6), bottom: Val::Percent(12.4), flex_direction: FlexDirection::Column, align_items: AlignItems::Start, row_gap: Val::Px(4.0), ..default() })
                .with_children(|c| {
                    c.spawn((Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(3.0), ..default() }, HudEffects));
                    c.spawn((
                        Node { border: UiRect::all(Val::Px(1.0)), border_radius: BorderRadius::all(Val::Px(6.0)), padding: UiRect::axes(Val::Px(8.0), Val::Px(3.0)), display: Display::None, ..default() },
                        BackgroundColor(Color::srgba(1.0, 0.35, 0.24, 0.12)),
                        BorderColor::all(Color::srgba(1.0, 0.35, 0.24, 0.35)),
                        HudWreckLine,
                    ))
                    .with_children(|w| {
                        w.spawn((Text::new(""), TextFont { font: f.bold.clone().into(), font_size: FontSize::Px(11.0), ..default() }, TextColor(Color::srgb(1.0, 0.6, 0.42))));
                    });
                });
            // ---------------- низ слева: панель машины
            root.spawn(glass_node(Node { position_type: PositionType::Absolute, left: Val::Percent(1.6), bottom: Val::Percent(2.4), width: Val::Px(384.0), padding: UiRect::axes(Val::Px(13.0), Val::Px(11.0)), column_gap: Val::Px(11.0), align_items: AlignItems::Center, ..default() }))
                .with_children(|c| {
                    c.spawn((
                        Node { width: Val::Px(58.0), height: Val::Px(58.0), border: UiRect::all(Val::Px(1.0)), border_radius: BorderRadius::all(Val::Px(10.0)), justify_content: JustifyContent::Center, align_items: AlignItems::Center, flex_shrink: 0.0, ..default() },
                        BackgroundColor(Color::srgb(0.33, 0.13, 0.11)),
                        BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.12)),
                        HudAvatar,
                    ))
                    .with_children(|a| {
                        a.spawn((Text::new(""), TextFont { font: f.title.clone().into(), font_size: FontSize::Px(28.0), ..default() }, TextColor(pal::TEXT)));
                    });
                    c.spawn(Node { flex_direction: FlexDirection::Column, width: Val::Px(288.0), row_gap: Val::Px(5.0), ..default() }).with_children(|p| {
                        p.spawn(Node { column_gap: Val::Px(10.0), align_items: AlignItems::End, ..default() }).with_children(|n| {
                            n.spawn((Text::new(""), TextFont { font: f.title.clone().into(), font_size: FontSize::Px(17.0), ..default() }, TextColor(pal::TEXT), HudCarName));
                            n.spawn((Text::new(""), TextFont { font: f.body.clone().into(), font_size: FontSize::Px(11.0), ..default() }, TextColor(LABEL), HudCarSub));
                        });
                        p.spawn((Node { width: Val::Px(288.0), height: Val::Px(13.6), border: UiRect::all(Val::Px(1.0)), border_radius: BorderRadius::all(Val::Px(8.0)), overflow: Overflow::clip(), ..default() }, BackgroundColor(Color::srgb(0.063, 0.067, 0.075)), BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.10))))
                            .with_children(|b| {
                                b.spawn((ImageNode::new(hp_grad.clone()).with_mode(bevy::ui::widget::NodeImageMode::Stretch), Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }, HudHp));
                            });
                        p.spawn((Node { width: Val::Px(288.0), height: Val::Px(8.0), border: UiRect::all(Val::Px(1.0)), border_radius: BorderRadius::all(Val::Px(8.0)), overflow: Overflow::clip(), ..default() }, BackgroundColor(Color::srgb(0.063, 0.067, 0.075)), BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.10))))
                            .with_children(|b| {
                                b.spawn((ImageNode::new(armor_grad.clone()).with_mode(bevy::ui::widget::NodeImageMode::Stretch), Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }, HudArmor));
                            });
                        p.spawn(Node { width: Val::Px(288.0), justify_content: JustifyContent::SpaceBetween, ..default() }).with_children(|n| {
                            n.spawn((Text::new("КУЗОВ 100 / 100"), TextFont { font: f.narrow.clone().into(), font_size: FontSize::Px(11.0), ..default() }, TextColor(Color::srgb(0.73, 0.70, 0.67)), HudNums(0)));
                            n.spawn((Text::new("БРОНЯ 100%"), TextFont { font: f.narrow.clone().into(), font_size: FontSize::Px(11.0), ..default() }, TextColor(Color::srgb(0.73, 0.70, 0.67)), HudNums(1)));
                        });
                    });
                });
            // ---------------- низ-центр: лента событий (не шире 32%)
            root.spawn(Node { position_type: PositionType::Absolute, bottom: Val::Percent(2.6), left: Val::Percent(34.0), width: Val::Percent(32.0), max_width: Val::Px(520.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(5.0), ..default() })
                .with_children(|c| {
                    c.spawn((Node { width: Val::Percent(100.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(5.0), ..default() }, HudFeed));
                });
            // ---------------- низ справа: аналоговый спидометр и нитро
            root.spawn(glass_node(Node { position_type: PositionType::Absolute, right: Val::Percent(1.6), bottom: Val::Percent(2.4), width: Val::Px(244.0), padding: UiRect::new(Val::Px(13.0), Val::Px(13.0), Val::Px(10.0), Val::Px(8.0)), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() }))
                .with_children(|c| {
                    c.spawn(Node { width: Val::Px(216.0), height: Val::Px(192.0), ..default() }).with_children(|g| {
                        let abs = |l: f32, t: f32, w: f32, h: f32| Node { position_type: PositionType::Absolute, left: Val::Px(l), top: Val::Px(t), width: Val::Px(w), height: Val::Px(h), ..default() };
                        // циферблат
                        g.spawn((Node { border: UiRect::all(Val::Px(1.0)), border_radius: BorderRadius::all(Val::Percent(50.0)), ..abs(G_CX - 86.0, G_CY - 86.0, 172.0, 172.0) }, BackgroundColor(Color::srgb(0.075, 0.08, 0.09)), BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.10))));
                        g.spawn((Node { border: UiRect::all(Val::Px(1.0)), border_radius: BorderRadius::all(Val::Percent(50.0)), ..abs(G_CX - 70.0, G_CY - 70.0, 140.0, 140.0) }, BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.06))));
                        // риски и цифры
                        let mut v = 0.0;
                        while v <= G_MAX + 0.1 {
                            let major = (v as i32) % 20 == 0;
                            let a = gauge_angle(v);
                            let len = if major { 11.0 } else { 5.0 };
                            let th = if major { 2.4 } else { 1.2 };
                            let rm = G_R - 1.0 - len * 0.5;
                            let (cx, cy) = (G_CX + a.cos() * rm, G_CY + a.sin() * rm);
                            let red = v >= 160.0;
                            g.spawn((abs(cx - len * 0.5, cy - th * 0.5, len, th), BackgroundColor(if red { Color::srgb(1.0, 0.42, 0.29) } else { Color::srgba(0.9, 0.9, 0.92, 0.75) }), UiTransform::from_rotation(Rot2::radians(a))));
                            if major {
                                let (tx, ty) = (G_CX + a.cos() * G_RTXT, G_CY + a.sin() * G_RTXT);
                                g.spawn((
                                    Text::new(format!("{}", v as i32)),
                                    TextFont { font: f.narrow.clone().into(), font_size: FontSize::Px(10.0), ..default() },
                                    TextColor(if red { Color::srgb(1.0, 0.54, 0.42) } else { Color::srgba(0.9, 0.9, 0.91, 0.88) }),
                                    TextLayout::justify(Justify::Center),
                                    Node { position_type: PositionType::Absolute, left: Val::Px(tx - 14.0), top: Val::Px(ty - 7.0), width: Val::Px(28.0), ..default() },
                                ));
                            }
                            v += 10.0;
                        }
                        // дуга прогресса из сегментов
                        for k in 0..ARC_SEGS {
                            let sv = (k as f32 + 0.5) / ARC_SEGS as f32 * G_MAX;
                            let a = gauge_angle(sv);
                            let (cx, cy) = (G_CX + a.cos() * G_RARC, G_CY + a.sin() * G_RARC);
                            let t = sv / G_MAX;
                            let col = if t < 0.55 { Color::srgb(0.44 + (1.0 - 0.44) * t / 0.55, 0.82, 0.55 - 0.26 * t / 0.55) } else { Color::srgb(1.0, 0.82 - 0.47 * (t - 0.55) / 0.45, 0.29 - 0.05 * (t - 0.55) / 0.45) };
                            g.spawn((abs(cx - 2.5, cy - 4.0, 5.0, 8.0), BackgroundColor(col), UiTransform::from_rotation(Rot2::radians(a)), Visibility::Hidden, HudArcSeg(sv)));
                        }
                        // передача
                        g.spawn((Text::new("N"), TextFont { font: f.title.clone().into(), font_size: FontSize::Px(12.0), ..default() }, TextColor(HUD_GOLD), TextLayout::justify(Justify::Center), Node { position_type: PositionType::Absolute, left: Val::Px(G_CX - 20.0), top: Val::Px(52.0), width: Val::Px(40.0), ..default() }, HudGear));
                        // стрелка: контейнер с центром в оси, поворот вокруг центра
                        g.spawn((abs(G_CX - 60.0, G_CY - 60.0, 120.0, 120.0), UiTransform::from_rotation(Rot2::radians(gauge_angle(0.0) + std::f32::consts::FRAC_PI_2)), HudNeedle)).with_children(|n| {
                            n.spawn((Node { border_radius: BorderRadius::all(Val::Px(4.0)), ..abs(60.0 - 3.75, 10.0 - 3.0, 7.5, 63.0) }, BackgroundColor(Color::srgb(0.04, 0.04, 0.045))));
                            n.spawn((Node { border_radius: BorderRadius::all(Val::Px(2.0)), ..abs(60.0 - 1.7, 10.0, 3.4, 57.0) }, BackgroundColor(Color::srgb(1.0, 0.54, 0.29))));
                        });
                        g.spawn((Node { border: UiRect::all(Val::Px(1.0)), border_radius: BorderRadius::all(Val::Percent(50.0)), ..abs(G_CX - 8.5, G_CY - 8.5, 17.0, 17.0) }, BackgroundColor(Color::srgb(0.063, 0.067, 0.08)), BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.28))));
                        g.spawn((Node { border_radius: BorderRadius::all(Val::Percent(50.0)), ..abs(G_CX - 3.0, G_CY - 3.0, 6.0, 6.0) }, BackgroundColor(HUD_GOLD)));
                        // цифровые км/ч
                        g.spawn(Node { position_type: PositionType::Absolute, left: Val::Px(0.0), right: Val::Px(0.0), bottom: Val::Px(22.0), justify_content: JustifyContent::Center, align_items: AlignItems::End, column_gap: Val::Px(6.0), ..default() }).with_children(|k| {
                            k.spawn((Text::new("0"), TextFont { font: f.title.clone().into(), font_size: FontSize::Px(34.0), ..default() }, TextColor(pal::TEXT), HudSpeed));
                            k.spawn((Text::new("КМ/Ч"), TextFont { font: f.bold.clone().into(), font_size: FontSize::Px(10.0), ..default() }, TextColor(LABEL), Node { margin: UiRect::bottom(Val::Px(6.0)), ..default() }));
                        });
                    });
                    // нитро
                    c.spawn(Node { width: Val::Px(216.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0), margin: UiRect::top(Val::Px(5.0)), ..default() }).with_children(|n| {
                        n.spawn(Node { width: Val::Px(216.0), justify_content: JustifyContent::SpaceBetween, ..default() }).with_children(|l| {
                            label(l, "НИТРО");
                            l.spawn((Text::new("100%"), TextFont { font: f.bold.clone().into(), font_size: FontSize::Px(10.0), ..default() }, TextColor(LABEL), HudNitroPct));
                        });
                        n.spawn((Node { width: Val::Px(216.0), height: Val::Px(13.6), border: UiRect::all(Val::Px(1.0)), border_radius: BorderRadius::all(Val::Px(8.0)), overflow: Overflow::clip(), ..default() }, BackgroundColor(Color::srgb(0.063, 0.067, 0.075)), BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.10))))
                            .with_children(|b| {
                                b.spawn((ImageNode::new(nitro_grad.clone()).with_mode(bevy::ui::widget::NodeImageMode::Stretch), Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }, HudNitro));
                            });
                    });
                });
            // ---------------- оверлеи: имена над машинами, захват
            root.spawn((Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }, HudTags));
            root.spawn((
                Node { border_radius: BorderRadius::all(Val::Px(35.0)), position_type: PositionType::Absolute, width: Val::Px(70.0), height: Val::Px(70.0), border: UiRect::all(Val::Px(3.0)), display: Display::None, ..default() },
                BorderColor::all(pal::BAD),
                HudLock,
            ));
            // ---------------- экран гибели
            root.spawn((Node { position_type: PositionType::Absolute, bottom: Val::Percent(30.0), width: Val::Percent(100.0), justify_content: JustifyContent::Center, display: Display::None, ..default() }, HudDead))
                .with_children(|c| {
                    c.spawn((Text::new(""), TextFont { font: f.title.clone().into(), font_size: FontSize::Px(30.0), ..default() }, TextColor(pal::BAD), TextLayout::justify(Justify::Center), HudDeadText));
                });
            // ---------------- таблица (TAB)
            root.spawn((
                Node {
                    border_radius: BorderRadius::all(Val::Px(10.0)),
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Percent(14.0),
                    margin: UiRect::left(Val::Px(-380.0)),
                    width: Val::Px(760.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(16.0)),
                    row_gap: Val::Px(4.0),
                    display: Display::None,
                    ..default()
                },
                BackgroundColor(pal::BG),
                HudBoard,
            ));
            // ---------------- пауза: управление и правила режима (с экрана убраны)
            root.spawn((
                Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, display: Display::None, ..default() },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
                GlobalZIndex(10),
                HudPause,
            ))
            .with_children(|c| {
                c.spawn((Node { border_radius: BorderRadius::all(Val::Px(10.0)), flex_direction: FlexDirection::Column, row_gap: Val::Px(12.0), padding: UiRect::all(Val::Px(28.0)), align_items: AlignItems::Center, max_width: Val::Px(820.0), ..default() }, BackgroundColor(pal::BG)))
                    .with_children(|p| {
                        txt(p, "ПАУЗА", &f.title, 44.0, pal::ACCENT);
                        button(p, &f, "ПРОДОЛЖИТЬ", HudAction::Resume, Val::Px(320.0), true, false);
                        button(p, &f, "ВЫЙТИ В МЕНЮ", HudAction::Quit, Val::Px(320.0), false, false);
                        p.spawn((Text::new(""), TextFont { font: f.bold.clone().into(), font_size: FontSize::Px(17.0), ..default() }, TextColor(pal::TEXT), TextLayout::justify(Justify::Center), HudHint));
                        txt(
                            p,
                            "W/S: газ и тормоз (задний ход — держать S на месте) · A/D: руль · ПРОБЕЛ: ручник · SHIFT: нитро\nЛКМ/Ctrl: огонь · ПКМ/E: спецоружие · Q: мины и масло · F: прыжок\nR: эвакуация на дорогу · C: камера · V: взгляд назад · H: гудок · TAB: таблица",
                            &f.body,
                            16.0,
                            pal::DIM,
                        );
                    });
            });
        });
}

fn hud_events(mut ev: MessageReader<GameEvent>, local: Res<LocalPlayer>, mut msgs: ResMut<HudMessages>, time: Res<Time<Real>>, names: Query<(&NetId, &Car)>) {
    let now = time.elapsed_secs();
    if let Some(n) = local.car.and_then(|id| names.iter().find(|x| *x.0 == id)).map(|x| x.1.name.clone()) {
        if msgs.my_name != n {
            msgs.my_name = n;
        }
    }
    for e in ev.read() {
        match e {
            GameEvent::Message { to, text, color, big } => {
                if to.is_some() && *to != local.car {
                    continue;
                }
                if *big {
                    msgs.big = Some((text.clone(), *color, now + 1.8));
                } else {
                    msgs.small.push((text.clone(), *color, now + 2.6));
                    if msgs.small.len() > 5 {
                        msgs.small.remove(0);
                    }
                }
            }
            GameEvent::KillFeed { text } => {
                msgs.feed.push((text.clone(), now + 7.0));
                if msgs.feed.len() > 6 {
                    msgs.feed.remove(0);
                }
            }
            _ => {}
        }
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn hud_top(
    local: Res<LocalPlayer>,
    ms: Option<Res<MatchState>>,
    cfg: Res<MatchConfig>,
    map: Option<Res<MapData>>,
    cars: Query<(&NetId, &CarState, &Combat)>,
    mut t: Query<(&mut Text, &mut TextColor), (With<HudTime>, Without<HudScore>, Without<HudKills>, Without<HudAlive>, Without<HudExtraLabel>, Without<HudHint>)>,
    mut score: Query<&mut Text, (With<HudScore>, Without<HudTime>, Without<HudKills>, Without<HudAlive>, Without<HudExtraLabel>, Without<HudHint>)>,
    mut kills: Query<&mut Text, (With<HudKills>, Without<HudTime>, Without<HudScore>, Without<HudAlive>, Without<HudExtraLabel>, Without<HudHint>)>,
    mut alive_t: Query<&mut Text, (With<HudAlive>, Without<HudTime>, Without<HudScore>, Without<HudKills>, Without<HudExtraLabel>, Without<HudHint>)>,
    mut extra: Query<&mut Text, (With<HudExtraLabel>, Without<HudTime>, Without<HudScore>, Without<HudKills>, Without<HudAlive>, Without<HudHint>)>,
    mut hint: Query<&mut Text, (With<HudHint>, Without<HudTime>, Without<HudScore>, Without<HudKills>, Without<HudAlive>, Without<HudExtraLabel>)>,
) {
    let Some(ms) = ms else { return };
    let me = local_car(&local, cars.iter());
    if let Ok((mut text, mut color)) = t.single_mut() {
        let tl = match (cfg.mode, me) {
            (GameMode::Carnage, Some((_, c))) => c.time_left,
            _ => ms.time_left,
        };
        let tl = if ms.phase == MatchPhase::Countdown { cfg.time_limit } else { tl };
        let s = format!("{:02}:{:02}", (tl.max(0.0) / 60.0) as u32, (tl.max(0.0) % 60.0) as u32);
        if **text != s {
            **text = s;
        }
        color.0 = if tl < 15.0 && ms.phase == MatchPhase::Running { pal::BAD } else { pal::TEXT };
    }
    if let (Ok(mut k), Some((_, c))) = (kills.single_mut(), me) {
        let s = format!("{}", c.wrecks);
        if **k != s {
            **k = s;
        }
    }
    if let Ok(mut a) = alive_t.single_mut() {
        let alive = cars.iter().filter(|(_, s, c)| !s.wrecked && !c.eliminated).count();
        let s = format!("{} / {}", alive, cars.iter().count());
        if **a != s {
            **a = s;
        }
    }
    // третья колонка: по режиму — точка маршрута, пешеходы или очки
    if let (Ok(mut lbl), Ok(mut val)) = (extra.single_mut(), score.single_mut()) {
        let (l, v) = match (cfg.mode, me) {
            (GameMode::Carnage, Some((_, c))) => {
                let n = map.as_ref().map(|m| m.checkpoints.len()).unwrap_or(0).max(1) as u32;
                ("ТОЧКА", format!("{}/{} · {}", c.checkpoint % n + 1, n, c.score))
            }
            (GameMode::PedHunt, Some((_, c))) => ("ОЧКИ", if c.combo > 1 && c.combo_time > 0.0 { format!("{} x{}", c.score, c.combo) } else { format!("{}", c.score) }),
            (GameMode::Deathmatch, Some((_, c))) => ("ОЧКИ", format!("{} · {}/{}", c.score, c.wrecks, cfg.frag_limit)),
            (_, Some((_, c))) => ("ОЧКИ", format!("{}", c.score)),
            _ => ("ОЧКИ", "0".into()),
        };
        if **lbl != l {
            **lbl = l.into();
        }
        if **val != v {
            **val = v;
        }
    }
    if let Ok(mut h) = hint.single_mut() {
        let s = format!("{} · {}\n{}", cfg.mode.name(), map_info(cfg.map).name, cfg.mode.desc());
        if **h != s {
            **h = s;
        }
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn hud_vehicle(
    local: Res<LocalPlayer>,
    cars: Query<(&NetId, &CarState, &Combat, &Car)>,
    mut speed: Query<&mut Text, (With<HudSpeed>, Without<HudGear>, Without<HudNums>, Without<HudNitroPct>, Without<HudCarName>, Without<HudCarSub>)>,
    mut gear: Query<&mut Text, (With<HudGear>, Without<HudSpeed>, Without<HudNums>, Without<HudNitroPct>, Without<HudCarName>, Without<HudCarSub>)>,
    mut nums: Query<(&HudNums, &mut Text), (Without<HudSpeed>, Without<HudGear>, Without<HudNitroPct>, Without<HudCarName>, Without<HudCarSub>)>,
    mut npct: Query<&mut Text, (With<HudNitroPct>, Without<HudSpeed>, Without<HudGear>, Without<HudNums>, Without<HudCarName>, Without<HudCarSub>)>,
    mut cname: Query<&mut Text, (With<HudCarName>, Without<HudSpeed>, Without<HudGear>, Without<HudNums>, Without<HudNitroPct>, Without<HudCarSub>)>,
    mut csub: Query<&mut Text, (With<HudCarSub>, Without<HudSpeed>, Without<HudGear>, Without<HudNums>, Without<HudNitroPct>, Without<HudCarName>)>,
    mut bars: ParamSet<(Query<&mut Node, With<HudHp>>, Query<&mut Node, With<HudArmor>>, Query<&mut Node, With<HudNitro>>)>,
    mut needle: Query<&mut UiTransform, With<HudNeedle>>,
    mut segs: Query<(&HudArcSeg, &mut Visibility)>,
    mut avatar: Query<(&mut BackgroundColor, &Children), With<HudAvatar>>,
    mut avatar_txt: Query<&mut Text, (Without<HudSpeed>, Without<HudGear>, Without<HudNums>, Without<HudNitroPct>, Without<HudCarName>, Without<HudCarSub>, Without<HudTime>, Without<HudScore>)>,
    mut wreck_line: Query<(&mut Node, &Children), (With<HudWreckLine>, Without<HudHp>, Without<HudArmor>, Without<HudNitro>)>,
    mut shown_speed: Local<f32>,
    time: Res<Time>,
) {
    let Some(id) = local.car else { return };
    let Some((_, st, _c, car)) = cars.iter().find(|(n, ..)| **n == id) else { return };
    let kmh = (st.speed.abs() * 3.6).min(G_MAX + 10.0);
    // сглаживание стрелки
    *shown_speed += (kmh - *shown_speed) * (time.delta_secs() * 10.0).min(1.0);
    let v = *shown_speed;
    if let Ok(mut t) = speed.single_mut() {
        let s = format!("{}", kmh.round() as i32);
        if **t != s {
            **t = s;
        }
    }
    if let Ok(mut t) = gear.single_mut() {
        let s = match st.drive {
            -1 => "R".to_string(),
            0 => "N".to_string(),
            _ => format!("D{}", st.gear.clamp(1, 4)),
        };
        if **t != s {
            **t = s;
        }
    }
    if let Ok(mut tr) = needle.single_mut() {
        tr.rotation = Rot2::radians(gauge_angle(v) + std::f32::consts::FRAC_PI_2);
    }
    for (seg, mut vis) in segs.iter_mut() {
        let want = if seg.0 <= v.max(0.5) { Visibility::Inherited } else { Visibility::Hidden };
        if *vis != want {
            *vis = want;
        }
    }
    let hp = st.hp.max(0.0);
    let armor_pct = (st.armor / crate::balance::START_ARMOR * 100.0).clamp(0.0, 100.0);
    if let Ok(mut n) = bars.p0().single_mut() {
        n.width = Val::Percent(hp.clamp(0.0, 100.0));
    }
    if let Ok(mut n) = bars.p1().single_mut() {
        n.width = Val::Percent(armor_pct);
    }
    let nitro = (st.nitro / st.nitro_max.max(0.1)).clamp(0.0, 1.0) * 100.0;
    if let Ok(mut n) = bars.p2().single_mut() {
        n.width = Val::Percent(nitro);
    }
    for (k, mut t) in nums.iter_mut() {
        let s = if k.0 == 0 { format!("КУЗОВ {} / 100", hp.ceil() as i32) } else { format!("БРОНЯ {}%", armor_pct.round() as i32) };
        if **t != s {
            **t = s;
        }
    }
    if let Ok(mut t) = npct.single_mut() {
        let s = format!("{}%", nitro.round() as i32);
        if **t != s {
            **t = s;
        }
    }
    let def = car.def();
    if let Ok(mut t) = cname.single_mut() {
        let s = def.name.to_uppercase();
        if **t != s {
            **t = s;
        }
    }
    if let Ok(mut t) = csub.single_mut() {
        let s = format!("{} · урон {}%", def.trait_name.to_lowercase(), (100.0 - hp).max(0.0).round() as i32);
        if **t != s {
            **t = s;
        }
    }
    if let Ok((mut bg, ch)) = avatar.single_mut() {
        let c = car.cfg.color;
        bg.0 = Color::srgb(c[0] * 0.55 + 0.05, c[1] * 0.55 + 0.04, c[2] * 0.55 + 0.04);
        if let Some(e) = ch.first() {
            if let Ok(mut t) = avatar_txt.get_mut(*e) {
                let s: String = def.name.chars().take(1).collect();
                if **t != s {
                    **t = s;
                }
            }
        }
    }
    // строка повреждений узлов
    let nodes = st.nodes();
    let mut parts = Vec::new();
    if nodes.radiator {
        parts.push("радиатор повреждён · тяга −18%".to_string());
    }
    if nodes.engine {
        parts.push("двигатель · скорость −15%".to_string());
    }
    if nodes.wheel != 0 {
        parts.push(format!("колесо · уводит {}", if nodes.wheel < 0 { "влево" } else { "вправо" }));
    }
    if let Ok((mut n, ch)) = wreck_line.single_mut() {
        let show = !parts.is_empty() && !st.wrecked;
        n.display = if show { Display::Flex } else { Display::None };
        if let Some(e) = ch.first() {
            if let Ok(mut t) = avatar_txt.get_mut(*e) {
                let s = format!("! {}", parts.join("   ! "));
                if **t != s {
                    **t = s;
                }
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn hud_weapons(
    local: Res<LocalPlayer>,
    cars: Query<(&NetId, &CarState, &Combat)>,
    icons: Option<Res<WeaponIcons>>,
    mut slots: Query<(&HudSlot, &mut BorderColor, &mut BackgroundColor)>,
    mut icon_q: Query<(&HudSlotIcon, &mut ImageNode, &mut Visibility)>,
    mut ammo_q: Query<(&HudSlotAmmo, &mut Text), Without<HudSlotKey>>,
    mut key_q: Query<(&HudSlotKey, &mut Text), Without<HudSlotAmmo>>,
    mut cd_q: Query<(&HudSlotCd, &mut Node)>,
) {
    let Some((st, _)) = local_car(&local, cars.iter()) else { return };
    let list = &st.weapons;
    for (s, mut border, mut bg) in slots.iter_mut() {
        let w = list.get(s.0);
        let ready = w.map(|w| w.ready >= 1.0 && (w.ammo > 0 || weapon_def(w.kind).trigger == crate::data::Trigger::Passive)).unwrap_or(false);
        *border = BorderColor::all(if ready { Color::srgba(1.0, 0.7, 0.29, 0.55) } else { GLASS_EDGE });
        bg.0 = if w.is_some() { GLASS } else { Color::srgba(0.07, 0.075, 0.085, 0.45) };
    }
    for (s, mut img, mut vis) in icon_q.iter_mut() {
        let w = list.get(s.0);
        let h = w.and_then(|w| icons.as_ref().and_then(|ic| ic.0.iter().find(|(k, _)| *k == w.kind).map(|x| x.1.clone())));
        match (w, h) {
            (Some(w), Some(h)) => {
                if img.image != h {
                    img.image = h;
                }
                let cool = w.ready < 1.0 || (w.ammo == 0 && weapon_def(w.kind).trigger != crate::data::Trigger::Passive);
                img.color = if cool { Color::srgba(1.0, 1.0, 1.0, 0.3) } else { Color::WHITE };
                *vis = Visibility::Inherited;
            }
            _ => *vis = Visibility::Hidden,
        }
    }
    for (s, mut t) in ammo_q.iter_mut() {
        let txt = list.get(s.0).map(|w| if weapon_def(w.kind).trigger == crate::data::Trigger::Passive { String::new() } else { format!("{}", w.ammo) }).unwrap_or_default();
        if **t != txt {
            **t = txt;
        }
    }
    for (s, mut t) in key_q.iter_mut() {
        let txt = list.get(s.0).map(|w| short_key(weapon_def(w.kind).trigger).to_string()).unwrap_or_default();
        if **t != txt {
            **t = txt;
        }
    }
    for (s, mut n) in cd_q.iter_mut() {
        let h = list.get(s.0).map(|w| if w.ready < 1.0 { (1.0 - w.ready).clamp(0.0, 1.0) * 100.0 } else { 0.0 }).unwrap_or(0.0);
        n.height = Val::Percent(h);
    }
}

fn hud_effects(
    mut commands: Commands,
    fonts: Res<Fonts>,
    local: Res<LocalPlayer>,
    cars: Query<(&NetId, &CarState, &Combat)>,
    panel: Query<Entity, With<HudEffects>>,
    mut msgs: ResMut<HudMessages>,
) {
    let Some((st, _)) = local_car(&local, cars.iter()) else { return };
    let Ok(p) = panel.single() else { return };
    let mut list: Vec<(String, [f32; 3])> = Vec::new();
    for (i, k) in TIMED_EFFECTS.iter().enumerate() {
        let t = st.effects[i];
        if t > 0.0 {
            let d = powerup_def(*k);
            let extra = if *k == PowerupKind::Springs { format!(" ({})", st.springs) } else { String::new() };
            list.push((format!("{}{}  {}", d.name, extra, t.ceil() as i32), d.color));
        }
    }
    if st.burning > 0.0 {
        list.push((format!("ГОРИШЬ  {}", st.burning.ceil() as i32), [1.0, 0.45, 0.1]));
    }
    if st.emp > 0.0 {
        list.push((format!("ЭМИ: МОТОР ЗАГЛОХ  {}", st.emp.ceil() as i32), [0.5, 0.7, 1.0]));
    }
    if st.frozen > 0.0 {
        list.push((format!("ЗАМОРОЖЕН  {}", st.frozen.ceil() as i32), [0.6, 0.95, 1.0]));
    }
    if st.oiled > 0.0 {
        list.push(("СКОЛЬЗКО".into(), [0.6, 0.6, 0.5]));
    }
    let sig: String = list.iter().map(|l| l.0.clone()).collect();
    if sig == msgs.effects_sig {
        return;
    }
    msgs.effects_sig = sig;
    commands.entity(p).despawn_related::<Children>();
    let f = fonts.clone();
    commands.entity(p).with_children(|c| {
        for (t, col) in list {
            c.spawn((Node { border_radius: BorderRadius::all(Val::Px(4.0)), padding: UiRect::axes(Val::Px(8.0), Val::Px(2.0)), ..default() }, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5))))
                .with_children(|b| {
                    txt(b, t, &f.bold, 16.0, Color::srgb(col[0], col[1], col[2]));
                });
        }
    });
}

#[allow(clippy::type_complexity)]
fn hud_messages(
    mut commands: Commands,
    fonts: Res<Fonts>,
    time: Res<Time<Real>>,
    mut msgs: ResMut<HudMessages>,
    mut big: Query<(&mut Text, &mut TextColor, &mut TextFont), With<HudBig>>,
    small: Query<Entity, With<HudSmall>>,
    feed: Query<Entity, With<HudFeed>>,
    mut last_sig: Local<(usize, usize, f32)>,
) {
    let now = time.elapsed_secs();
    msgs.small.retain(|m| m.2 > now);
    msgs.feed.retain(|m| m.1 > now);
    if let Ok((mut t, mut c, mut font)) = big.single_mut() {
        match &msgs.big {
            Some((text, col, until)) if *until > now => {
                if **t != *text {
                    **t = text.clone();
                }
                let left = until - now;
                let a = (left / 0.4).clamp(0.0, 1.0);
                c.0 = Color::srgba(col[0], col[1], col[2], a);
                font.font_size = FontSize::Px(64.0 + (1.8 - left).clamp(0.0, 0.25) * 0.0 + if text.len() <= 2 { 50.0 } else { 0.0 });
            }
            _ => {
                if !t.is_empty() {
                    **t = String::new();
                }
            }
        }
    }
    let sig = (msgs.small.len() + msgs.small.first().map(|m| m.0.len()).unwrap_or(0) * 100, msgs.feed.len() + msgs.feed.first().map(|m| m.0.len()).unwrap_or(0) * 100, 0.0);
    if sig.0 == last_sig.0 && sig.1 == last_sig.1 {
        return;
    }
    *last_sig = sig;
    let f = fonts.clone();
    if let Ok(p) = small.single() {
        commands.entity(p).despawn_related::<Children>();
        let list = msgs.small.clone();
        commands.entity(p).with_children(|c| {
            for (t, col, _) in list {
                c.spawn((Text::new(t), TextFont { font: f.title.clone().into(), font_size: FontSize::Px(26.0), ..default() }, TextColor(Color::srgb(col[0], col[1], col[2]))));
            }
        });
    }
    if let Ok(p) = feed.single() {
        commands.entity(p).despawn_related::<Children>();
        // последние три события; имена — цветом, своё — с оранжевой рамкой
        let list: Vec<_> = msgs.feed.iter().rev().take(3).rev().cloned().collect();
        let me = msgs.my_name.clone();
        commands.entity(p).with_children(|c| {
            for (t, _) in list {
                let mine = !me.is_empty() && t.contains(&me);
                c.spawn((
                    Node { width: Val::Percent(100.0), justify_content: JustifyContent::Center, border: UiRect::all(Val::Px(1.0)), border_radius: BorderRadius::all(Val::Px(8.0)), padding: UiRect::axes(Val::Px(11.0), Val::Px(5.0)), ..default() },
                    BackgroundColor(Color::srgba(0.055, 0.06, 0.067, 0.74)),
                    BorderColor::all(if mine { Color::srgba(1.0, 0.7, 0.29, 0.4) } else { GLASS_EDGE }),
                ))
                .with_children(|b| {
                    let (k, rest) = match t.split_once(' ') {
                        Some((a, r)) => (a.to_string(), r.to_string()),
                        None => (t.clone(), String::new()),
                    };
                    let (verb, victim) = match rest.rsplit_once(' ') {
                        Some((v, w)) if rest.contains("разбил") => (v.to_string(), w.to_string()),
                        _ => (rest.clone(), String::new()),
                    };
                    b.spawn((Text::new(""), TextFont { font: f.bold.clone().into(), font_size: FontSize::Px(13.0), ..default() }, TextColor(pal::TEXT), TextLayout::justify(Justify::Center)))
                        .with_children(|tx| {
                            tx.spawn((TextSpan::new(format!("{} ", k)), TextFont { font: f.bold.clone().into(), font_size: FontSize::Px(13.0), ..default() }, TextColor(Color::srgb(1.0, 0.54, 0.35))));
                            tx.spawn((TextSpan::new(verb), TextFont { font: f.body.clone().into(), font_size: FontSize::Px(13.0), ..default() }, TextColor(pal::TEXT)));
                            if !victim.is_empty() {
                                tx.spawn((TextSpan::new(format!(" {}", victim)), TextFont { font: f.bold.clone().into(), font_size: FontSize::Px(13.0), ..default() }, TextColor(HUD_GOLD)));
                            }
                        });
                });
            }
        });
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn hud_minimap(
    mut commands: Commands,
    info: Option<Res<MiniMapInfo>>,
    local: Res<LocalPlayer>,
    cfg: Res<MatchConfig>,
    map: Option<Res<MapData>>,
    panel: Query<Entity, With<HudMinimap>>,
    cars: Query<(&NetId, &Transform, &CarState, &Car, &Combat)>,
    peds: Query<&Transform, With<crate::peds::Ped>>,
    mut dots: Query<(Entity, &MiniDot, &mut Node, &mut BackgroundColor, &mut Visibility, &mut BorderColor), (Without<RadarMap>, Without<RadarSweep>)>,
    mut radar_map: Query<&mut Node, (With<RadarMap>, Without<MiniDot>)>,
    mut sweep: Query<&mut UiTransform, With<RadarSweep>>,
    time: Res<Time<Real>>,
) {
    let (Some(info), Ok(p)) = (info, panel.single()) else { return };
    let scale = RADAR_R / RADAR_RANGE;
    let center = local.car.and_then(|id| cars.iter().find(|c| *c.0 == id)).map(|c| c.1.translation).unwrap_or(Vec3::ZERO);
    // карта под радаром сдвигается так, чтобы игрок был в центре
    if let Ok(mut n) = radar_map.single_mut() {
        n.left = Val::Px(RADAR_R - (center.x + info.half.x) * scale);
        n.top = Val::Px(RADAR_R - (center.z + info.half.y) * scale);
    }
    if let Ok(mut tr) = sweep.single_mut() {
        tr.rotation = Rot2::radians((time.elapsed_secs() / 4.0).fract() * std::f32::consts::TAU);
    }
    // точка радара: (позиция, цвет, размер, обводка); враги за краем прижимаются к ободу
    let to_px = |pos: Vec3, clamp: bool| -> Option<Vec2> {
        let mut d = Vec2::new(pos.x - center.x, pos.z - center.z) * scale;
        if d.length() > RADAR_R - 5.0 {
            if !clamp {
                return None;
            }
            d = d.normalize_or_zero() * (RADAR_R - 5.0);
        }
        Some(Vec2::splat(RADAR_R) + d)
    };
    let mut wanted: HashMap<u32, (Vec2, Color, f32, bool)> = HashMap::new();
    for (id, tf, st, car, c) in cars.iter() {
        let mine = Some(*id) == local.car;
        if !mine && (st.has(PowerupKind::Invisibility) || car.cfg.has_perk(PerkKind::Ghost)) {
            continue;
        }
        if mine {
            wanted.insert(id.0, (Vec2::splat(RADAR_R), Color::srgb(1.0, 0.87, 0.42), 11.0, true));
        } else if st.wrecked || c.eliminated {
            if let Some(px) = to_px(tf.translation, false) {
                wanted.insert(id.0, (px, Color::srgb(0.35, 0.35, 0.37), 6.0, false));
            }
        } else if let Some(px) = to_px(tf.translation, true) {
            // командных режимов нет: все соперники — враги (красные); союзники (синие) зарезервированы
            wanted.insert(id.0, (px, Color::srgb(1.0, 0.35, 0.29), 7.0, false));
        }
    }
    // пешеходы: ближайшие, без прижима к краю
    let mut pk = 0u32;
    for t in peds.iter() {
        if pk >= 60 {
            break;
        }
        if let Some(px) = to_px(t.translation, false) {
            wanted.insert(3_000_000 + pk, (px, Color::srgba(0.6, 0.63, 0.65, 0.85), 4.0, false));
            pk += 1;
        }
    }
    if cfg.mode == GameMode::Carnage {
        if let (Some(map), Some(c)) = (map.as_ref(), local.car.and_then(|id| cars.iter().find(|x| *x.0 == id)).map(|x| x.4)) {
            if !map.checkpoints.is_empty() {
                let cp = map.checkpoints[(c.checkpoint as usize) % map.checkpoints.len()];
                if let Some(px) = to_px(cp, true) {
                    wanted.insert(4_000_000, (px, Color::srgb(1.0, 0.85, 0.1), 9.0, false));
                }
            }
        }
    }
    for (_e, d, mut n, mut bg, mut vis, mut bc) in dots.iter_mut() {
        if let Some((px, col, size, ring)) = wanted.remove(&d.0) {
            n.left = Val::Px(px.x - size * 0.5);
            n.top = Val::Px(px.y - size * 0.5);
            n.width = Val::Px(size);
            n.height = Val::Px(size);
            bg.0 = col;
            *bc = BorderColor::all(if ring { Color::srgb(1.0, 0.95, 0.77) } else { Color::NONE });
            *vis = Visibility::Inherited;
        } else if *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
    }
    for (id, (px, col, size, ring)) in wanted {
        let dot = commands
            .spawn((
                Node {
                    border_radius: BorderRadius::all(Val::Percent(50.0)),
                    border: UiRect::all(Val::Px(if ring { 2.0 } else { 0.0 })),
                    position_type: PositionType::Absolute,
                    left: Val::Px(px.x - size * 0.5),
                    top: Val::Px(px.y - size * 0.5),
                    width: Val::Px(size),
                    height: Val::Px(size),
                    ..default()
                },
                BackgroundColor(col),
                BorderColor::all(if ring { Color::srgb(1.0, 0.95, 0.77) } else { Color::NONE }),
                MiniDot(id),
                Visibility::Inherited,
            ))
            .id();
        commands.entity(p).add_child(dot);
    }
}

#[derive(Component)]
struct NameTag(u32);
#[derive(Component)]
struct NameTagBar;

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn hud_world_overlays(
    mut commands: Commands,
    fonts: Res<Fonts>,
    local: Res<LocalPlayer>,
    cam: Query<(&Camera, &GlobalTransform), With<ChaseCam>>,
    cars: Query<(&NetId, &Transform, &CarState, &Car)>,
    tags_root: Query<Entity, With<HudTags>>,
    mut tags: Query<(&NameTag, &mut Node, &mut Visibility, &Children), (Without<HudLock>, Without<NameTagBar>)>,
    mut bars: Query<(&mut Node, &mut BackgroundColor), (With<NameTagBar>, Without<NameTag>, Without<HudLock>)>,
    mut lock: Query<(&mut Node, &mut BorderColor), (With<HudLock>, Without<NameTag>, Without<NameTagBar>)>,
) {
    let Ok((camera, cam_tf)) = cam.single() else { return };
    let Ok(root) = tags_root.single() else { return };
    let me = local.car.and_then(|id| cars.iter().find(|c| *c.0 == id));
    let my_pos = me.map(|c| c.1.translation).unwrap_or(cam_tf.translation());
    let mut have: Vec<u32> = Vec::new();
    for (tag, mut node, mut vis, children) in tags.iter_mut() {
        have.push(tag.0);
        let Some((_, tf, st, car)) = cars.iter().find(|c| c.0 .0 == tag.0) else {
            *vis = Visibility::Hidden;
            continue;
        };
        let world = tf.translation + Vec3::Y * (car.def().size.y + 1.8);
        let d = world.distance(my_pos);
        let visible = !st.wrecked && d < 90.0 && !st.has(PowerupKind::Invisibility) && Some(tag.0) != local.car.map(|i| i.0);
        match camera.world_to_viewport(cam_tf, world) {
            Ok(px) if visible => {
                *vis = Visibility::Inherited;
                node.left = Val::Px(px.x - 60.0);
                node.top = Val::Px(px.y - 26.0);
                for c in children.iter() {
                    if let Ok((mut bn, mut bc)) = bars.get_mut(c) {
                        let f = (st.hp / 100.0).clamp(0.0, 1.0);
                        bn.width = Val::Px(120.0 * f);
                        bc.0 = if f > 0.6 { pal::GOOD } else if f > 0.3 { pal::GOLD } else { pal::BAD };
                    }
                }
            }
            _ => *vis = Visibility::Hidden,
        }
    }
    for (id, _, _, car) in cars.iter() {
        if have.contains(&id.0) || Some(*id) == local.car {
            continue;
        }
        let f = fonts.clone();
        let tag = commands
            .spawn((
                Node { position_type: PositionType::Absolute, width: Val::Px(120.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() },
                NameTag(id.0),
                Visibility::Hidden,
            ))
            .with_children(|t| {
                t.spawn((Text::new(car.name.clone()), TextFont { font: f.bold.clone().into(), font_size: FontSize::Px(15.0), ..default() }, TextColor(Color::srgba(1.0, 1.0, 1.0, 0.9))));
                t.spawn((Node { width: Val::Px(120.0), height: Val::Px(4.0), ..default() }, BackgroundColor(pal::GOOD), NameTagBar));
            })
            .id();
        commands.entity(root).add_child(tag);
    }
    // Захват цели самонаводящимися ракетами
    if let Ok((mut node, mut border)) = lock.single_mut() {
        let target = me.and_then(|(_, _, st, _)| st.lock_target.map(|t| (t, st.lock_frac)));
        match target.and_then(|(t, frac)| cars.iter().find(|c| *c.0 == t).map(|c| (c.1.translation, frac))) {
            Some((pos, frac)) => match camera.world_to_viewport(cam_tf, pos + Vec3::Y) {
                Ok(px) => {
                    let size = 90.0 - 40.0 * frac;
                    node.display = Display::Flex;
                    node.left = Val::Px(px.x - size * 0.5);
                    node.top = Val::Px(px.y - size * 0.5);
                    node.width = Val::Px(size);
                    node.height = Val::Px(size);
                    *border = BorderColor::all(if frac >= 1.0 { pal::BAD } else { pal::GOLD });
                }
                Err(_) => node.display = Display::None,
            },
            None => node.display = Display::None,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn hud_board(
    mut commands: Commands,
    fonts: Res<Fonts>,
    toggles: Res<UiToggles>,
    ms: Option<Res<MatchState>>,
    local: Res<LocalPlayer>,
    mut board: Query<(Entity, &mut Node), With<HudBoard>>,
    cars: Query<(&NetId, &Car, &CarState, &Combat)>,
    mut msgs: ResMut<HudMessages>,
) {
    let Ok((e, mut node)) = board.single_mut() else { return };
    let finished = ms.map(|m| m.phase == MatchPhase::Finished).unwrap_or(false);
    let show = toggles.scoreboard || finished;
    node.display = if show { Display::Flex } else { Display::None };
    if !show {
        return;
    }
    let mut rows: Vec<_> = cars.iter().collect();
    rows.sort_by(|a, b| b.3.score.cmp(&a.3.score));
    let sig: String = rows.iter().map(|r| format!("{}{}{}{}{}{}", r.0 .0, r.3.score, r.3.wrecks, r.3.peds, r.2.wrecked, r.3.ping)).collect();
    if sig == msgs.board_sig {
        return;
    }
    msgs.board_sig = sig;
    commands.entity(e).despawn_related::<Children>();
    let f = fonts.clone();
    commands.entity(e).with_children(|p| {
        let header = |p: &mut ChildSpawnerCommands, cols: [&str; 7], font: &Handle<Font>, color: Color| {
            p.spawn(Node { flex_direction: FlexDirection::Row, ..default() }).with_children(|r| {
                let widths = [200.0, 130.0, 100.0, 80.0, 80.0, 80.0, 70.0];
                for (i, c) in cols.iter().enumerate() {
                    txt_node(r, *c, font, 17.0, color, Node { width: Val::Px(widths[i]), ..default() });
                }
            });
        };
        txt(p, "ТАБЛИЦА", &f.title, 26.0, pal::ACCENT);
        header(p, ["Игрок", "Машина", "Очки", "Разбил", "Пеших", "Смертей", "Пинг"], &f.bold, pal::DIM);
        for (id, car, st, c) in rows {
            let mine = Some(*id) == local.car;
            let name = if st.wrecked || c.eliminated { format!("{} (хлам)", car.name) } else { car.name.clone() };
            let ping = if car.controller == Controller::Bot { "бот".to_string() } else { format!("{}", c.ping) };
            let s = [name, car.def().name.to_string(), c.score.to_string(), c.wrecks.to_string(), c.peds.to_string(), c.deaths.to_string(), ping];
            p.spawn(Node { flex_direction: FlexDirection::Row, ..default() }).with_children(|r| {
                let widths = [200.0, 130.0, 100.0, 80.0, 80.0, 80.0, 70.0];
                for (i, c) in s.iter().enumerate() {
                    txt_node(r, c.clone(), &f.body, 17.0, if mine { pal::GOLD } else { pal::TEXT }, Node { width: Val::Px(widths[i]), ..default() });
                }
            });
        }
    });
}

fn hud_pause(paused: Res<Paused>, mut q: Query<&mut Node, With<HudPause>>) {
    if let Ok(mut n) = q.single_mut() {
        n.display = if paused.0 { Display::Flex } else { Display::None };
    }
}

#[allow(clippy::type_complexity)]
fn hud_dead(
    local: Res<LocalPlayer>,
    cfg: Res<MatchConfig>,
    ms: Option<Res<MatchState>>,
    cars: Query<(&NetId, &CarState, &Combat)>,
    mut panel: Query<&mut Node, With<HudDead>>,
    mut text: Query<&mut Text, With<HudDeadText>>,
) {
    let Ok(mut n) = panel.single_mut() else { return };
    let me = local_car(&local, cars.iter());
    let finished = ms.map(|m| m.phase == MatchPhase::Finished).unwrap_or(false);
    let Some((st, c)) = me.filter(|_| !finished) else {
        n.display = Display::None;
        return;
    };
    if !st.wrecked && !c.eliminated {
        n.display = Display::None;
        return;
    }
    n.display = Display::Flex;
    if let Ok(mut t) = text.single_mut() {
        **t = if cfg.mode.respawns() && st.respawn_in > 0.0 {
            format!("МАШИНА УНИЧТОЖЕНА\nснова в бою через {}", st.respawn_in.ceil() as i32)
        } else if cfg.mode.respawns() {
            "МАШИНА УНИЧТОЖЕНА".into()
        } else {
            "ТЫ ВЫБЫЛ\nкамера следит за остальными · ESC: выйти".into()
        };
    }
}

fn hud_fps(diag: Res<DiagnosticsStore>, settings: Res<Settings>, mut q: Query<&mut Text, With<HudFps>>) {
    let Ok(mut t) = q.single_mut() else { return };
    if !settings.show_fps {
        if !t.is_empty() {
            **t = String::new();
        }
        return;
    }
    if let Some(fps) = diag.get(&FrameTimeDiagnosticsPlugin::FPS).and_then(|d| d.smoothed()) {
        **t = format!("{:.0} FPS", fps);
    }
}

fn hud_actions(
    q: Query<(&Interaction, &HudAction), Changed<Interaction>>,
    mut paused: ResMut<Paused>,
    mut next: ResMut<NextState<AppState>>,
    mut net: MessageWriter<crate::net::NetCommand>,
    mode: Res<NetMode>,
    mut click: MessageWriter<UiClick>,
) {
    for (i, a) in q.iter() {
        if *i != Interaction::Pressed {
            continue;
        }
        click.write(UiClick);
        match a {
            HudAction::Resume => paused.0 = false,
            HudAction::Quit => {
                paused.0 = false;
                if *mode != NetMode::Offline {
                    net.write(crate::net::NetCommand::Leave);
                }
                next.set(AppState::Menu);
            }
        }
    }
}
