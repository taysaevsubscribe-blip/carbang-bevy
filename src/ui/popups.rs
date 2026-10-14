//! Всплывашки событий (пакет klod v1): удар, разбит, ремонт, бонус, полёт, комбо, старт.
//! Плашка из assets/ui/pop, надпись шрифтом игры поверх, отскок масштаба, подъём и затухание.
//! Показываются только события своей машины, чтобы экран не превращался в салют.

use crate::data::PowerupKind;
use crate::game::*;
use crate::vehicle::Car;
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource, Default)]
struct PopAssets {
    plates: HashMap<&'static str, Handle<Image>>,
    icons: HashMap<&'static str, Handle<Image>>,
}

#[derive(Component)]
struct Popup {
    t: f32,
    life: f32,
    /// центр на экране в момент появления (логические пиксели)
    at: Vec2,
    size: f32,
    rise: f32,
}

#[derive(Component)]
struct PopupText;

/// Накопление ударов по цели за короткое окно, чтобы очередь пуль не давала десять плашек.
#[derive(Resource, Default)]
struct HitAcc {
    pending: HashMap<NetId, (f32, f32, Vec3)>,
    combo: Vec<f32>,
    demo_done: bool,
    red_shown: bool,
}

pub struct PopupPlugin;

impl Plugin for PopupPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PopAssets>()
            .init_resource::<HitAcc>()
            .add_systems(Startup, load_assets)
            .add_systems(Update, (collect_events, animate).run_if(in_state(AppState::Race)));
    }
}

fn load_assets(mut a: ResMut<PopAssets>, mut images: ResMut<Assets<Image>>) {
    for name in [
        "pop_hit_1", "pop_hit_2", "pop_hit_3", "pop_wreck", "pop_repair", "pop_armor", "pop_combo", "pop_air", "pop_ped", "pop_bonus", "pop_win", "pop_lose", "start_light_red",
        "start_light_yellow", "start_light_green", "start_go",
    ] {
        if let Some(h) = crate::extassets::load_png(&mut images, &format!("ui/pop/{name}.png"), true, false) {
            a.plates.insert(name, h);
        }
    }
    for name in [
        "ico_repair", "ico_full_repair", "ico_nitro", "ico_ammo", "ico_time", "ico_credits", "ico_double_damage", "ico_shield", "ico_megaram", "ico_pinball", "ico_moon", "ico_grip",
        "ico_springs", "ico_invisible", "ico_ped_magnet", "ico_giant_peds", "ico_freeze", "ico_barrage", "ico_mine_ring", "ico_mystery",
    ] {
        if let Some(h) = crate::extassets::load_png(&mut images, &format!("ui/powerups/{name}.png"), true, false) {
            a.icons.insert(name, h);
        }
    }
}

fn icon_of(k: PowerupKind) -> (&'static str, &'static str) {
    use PowerupKind::*;
    match k {
        Repair => ("ico_repair", "ПОЧИНКА"),
        FullRepair => ("ico_full_repair", "КАК НОВАЯ"),
        Nitro => ("ico_nitro", "НИТРО"),
        Ammo => ("ico_ammo", "ПАТРОНЫ"),
        Time => ("ico_time", "+ВРЕМЯ"),
        Credits => ("ico_credits", "КРЕДИТЫ"),
        DoubleDamage => ("ico_double_damage", "ДВОЙНОЙ УРОН"),
        Shield => ("ico_shield", "ЩИТ"),
        MegaRam => ("ico_megaram", "МЕГАТАРАН"),
        Pinball => ("ico_pinball", "ПИНБОЛ"),
        MoonGravity => ("ico_moon", "ЛУНА"),
        SuperGrip => ("ico_grip", "ПРИСОСКИ"),
        Springs => ("ico_springs", "ПРУЖИНЫ"),
        Invisibility => ("ico_invisible", "НЕВИДИМКА"),
        PedMagnet => ("ico_ped_magnet", "МАГНИТ"),
        GiantPeds => ("ico_giant_peds", "ГИГАНТЫ"),
        Freeze => ("ico_freeze", "ЗАМОРОЗКА"),
        Barrage => ("ico_barrage", "ШКВАЛ"),
        MineRing => ("ico_mine_ring", "КОЛЬЦО МИН"),
        Mystery => ("ico_mystery", "СЮРПРИЗ"),
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_popup(commands: &mut Commands, fonts: &crate::ui::Fonts, plate: Option<Handle<Image>>, icon: Option<Handle<Image>>, text: &str, color: Color, at: Vec2, size: f32, life: f32, rise: f32) {
    spawn_popup_ex(commands, fonts, plate, icon, text, color, at, size, life, rise, None);
}

/// text_slot: (центр текста по высоте в долях плашки, кегль в долях размера) — для плашек с пустым полем под надпись
#[allow(clippy::too_many_arguments)]
fn spawn_popup_ex(commands: &mut Commands, fonts: &crate::ui::Fonts, plate: Option<Handle<Image>>, icon: Option<Handle<Image>>, text: &str, color: Color, at: Vec2, size: f32, life: f32, rise: f32, text_slot: Option<(f32, f32)>) {
    commands
        .spawn((
            Node { position_type: PositionType::Absolute, left: Val::Px(at.x - size * 0.5), top: Val::Px(at.y - size * 0.5), width: Val::Px(size), height: Val::Px(size), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
            UiTransform::from_scale(Vec2::splat(0.4)),
            GlobalZIndex(30),
            Popup { t: 0.0, life, at, size, rise },
            DespawnOnExit(AppState::Race),
        ))
        .with_children(|p| {
            if let Some(img) = plate {
                p.spawn((ImageNode::new(img), Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }));
            }
            if let Some(img) = icon {
                p.spawn((ImageNode::new(img), Node { position_type: PositionType::Absolute, width: Val::Percent(46.0), height: Val::Percent(46.0), top: Val::Percent(8.0), ..default() }));
            }
            if !text.is_empty() {
                let fs = match text_slot { Some((_, k)) => size * k, None => (size * 0.2).clamp(16.0, 56.0) };
                let node = match text_slot {
                    Some((y, _)) => Node { position_type: PositionType::Absolute, top: Val::Px(size * y - fs * 0.62), ..default() },
                    None => Node { position_type: PositionType::Absolute, ..default() },
                };
                p.spawn((
                    Text::new(text),
                    TextFont { font: fonts.title.clone().into(), font_size: FontSize::Px(fs), ..default() },
                    TextColor(color),
                    TextShadow { offset: Vec2::splat(3.0), color: Color::srgba(0.0, 0.0, 0.0, 0.85) },
                    TextLayout::justify(Justify::Center),
                    node,
                    PopupText,
                ));
            }
        });
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn collect_events(
    mut commands: Commands,
    mut events: MessageReader<GameEvent>,
    assets: Res<PopAssets>,
    fonts: Option<Res<crate::ui::Fonts>>,
    local: Res<LocalPlayer>,
    cars: Query<(&NetId, &Transform, &Car)>,
    cam: Query<(&Camera, &GlobalTransform), With<crate::camera::ChaseCam>>,
    windows: Query<&Window>,
    time: Res<Time>,
    mut acc: ResMut<HitAcc>,
    ms: Option<Res<MatchState>>,
) {
    let Some(fonts) = fonts else {
        debug_once("нет шрифтов");
        return;
    };
    let Some(me) = local.car else {
        debug_once("нет своей машины");
        events.clear();
        return;
    };
    let now = time.elapsed_secs();
    let screen = windows.iter().next().map(|w| Vec2::new(w.width(), w.height())).unwrap_or(Vec2::new(1600.0, 900.0));
    let ui = (screen.y / 900.0).max(0.5);
    let to_screen = |p: Vec3| -> Vec2 {
        cam.iter()
            .next()
            .and_then(|(c, gt)| c.world_to_viewport(gt, p + Vec3::Y * 2.2).ok())
            .map(|v| v.clamp(Vec2::splat(80.0), screen - Vec2::splat(80.0)))
            .unwrap_or(screen * Vec2::new(0.5, 0.42))
    };
    let center = screen * Vec2::new(0.5, 0.3);
    let plate = |n: &str| assets.plates.get(n).cloned();
    // красный свет в начале отсчёта (события «3» нет, отсчёт сразу идёт с трёх)
    if let Some(ms) = ms.as_ref() {
        match ms.phase {
            MatchPhase::Countdown if !acc.red_shown && ms.countdown > 1.6 => {
                acc.red_shown = true;
                spawn_popup(&mut commands, &fonts, plate("start_light_red"), None, "", Color::WHITE, screen * Vec2::new(0.5, 0.2), 220.0 * ui, (ms.countdown - 2.0).clamp(0.6, 1.0), 0.0);
            }
            MatchPhase::Running | MatchPhase::Finished => acc.red_shown = false,
            _ => {}
        }
    }
    // CARBANG_POPUP_DEMO: все виды плашек на экране для скриншота отчёта (раз за заезд)
    if std::env::var("CARBANG_POPUP_DEMO").is_ok() && !acc.demo_done && now > 0.3 {
        acc.demo_done = true;
        info!("всплывашки: демо, плашек {}, иконок {}", assets.plates.len(), assets.icons.len());
        let row = |i: usize, n: usize, y: f32| Vec2::new(screen.x * (i as f32 + 0.5) / n as f32, screen.y * y);
        let items = [("pop_hit_1", "БАМ!\n7"), ("pop_hit_2", "ХРЯСЬ!\n16"), ("pop_hit_3", "В ХЛАМ!\n31"), ("pop_wreck", "РАЗБИТ!\nОса"), ("pop_combo", "КОМБО ×3")];
        for (i, (pl, t)) in items.iter().enumerate() {
            spawn_popup(&mut commands, &fonts, plate(pl), None, t, Color::WHITE, row(i, 5, 0.3), 200.0 * ui, 60.0, 0.0);
        }
        let items2 = [("pop_repair", "ico_repair", "ПОЧИНКА"), ("pop_armor", "ico_shield", "ЩИТ"), ("pop_bonus", "ico_nitro", "НИТРО"), ("pop_air", "", "ПОЛЁТ\n2.4 с"), ("pop_ped", "", "+1")];
        for (i, (pl, ico, t)) in items2.iter().enumerate() {
            let at = row(i, 5, 0.62);
            spawn_popup(&mut commands, &fonts, plate(pl), None, if ico.is_empty() { t } else { "" }, Color::WHITE, at, 170.0 * ui, 60.0, 0.0);
            if !ico.is_empty() {
                spawn_popup(&mut commands, &fonts, assets.icons.get(ico).cloned(), None, "", Color::WHITE, at, 120.0 * ui, 60.0, 0.0);
                spawn_popup(&mut commands, &fonts, None, None, t, Color::WHITE, at + Vec2::Y * 85.0 * ui, 150.0 * ui, 60.0, 0.0);
            }
        }
        for (i, n) in ["start_light_red", "start_light_yellow", "start_light_green", "start_go"].iter().enumerate() {
            let go = *n == "start_go";
            spawn_popup_ex(&mut commands, &fonts, plate(n), None, if go { "ВПЕРЁД!" } else { "" }, Color::WHITE, Vec2::new(screen.x * (0.3 + 0.13 * i as f32), screen.y * 0.08 + 60.0 * ui), 120.0 * ui, 60.0, 0.0, go.then_some((0.752, 0.085)));
        }
    }
    for e in events.read() {
        match e {
            GameEvent::Hit { attacker, victim, amount } if *attacker == me && *victim != me => {
                let pos = cars.iter().find(|c| c.0 == victim).map(|c| c.1.translation).unwrap_or_default();
                let entry = acc.pending.entry(*victim).or_insert((0.0, now, pos));
                entry.0 += amount;
                entry.2 = pos;
            }
            GameEvent::CarWrecked { victim, killer, pos } if *killer == Some(me) && *victim != me => {
                acc.pending.remove(victim);
                let name = cars.iter().find(|c| c.0 == victim).map(|c| c.2.name.clone()).unwrap_or_default();
                spawn_popup(&mut commands, &fonts, plate("pop_wreck"), None, &format!("РАЗБИТ!\n{name}"), Color::srgb(1.0, 0.85, 0.3), to_screen(*pos), 230.0 * ui, 1.6, 70.0 * ui);
                acc.combo.push(now);
            }
            GameEvent::Pickup { car, kind, pos } if *car == me => {
                let (ico, label) = icon_of(*kind);
                let (pl, col) = match kind {
                    PowerupKind::Repair | PowerupKind::FullRepair => ("pop_repair", Color::srgb(0.6, 1.0, 0.6)),
                    PowerupKind::Shield => ("pop_armor", Color::srgb(0.7, 0.85, 1.0)),
                    _ => ("pop_bonus", Color::srgb(1.0, 0.95, 0.7)),
                };
                // иконка бонуса крупно, плашка под ней, подпись снизу
                let at = to_screen(*pos);
                spawn_popup(&mut commands, &fonts, plate(pl), None, "", Color::WHITE, at, 150.0 * ui, 1.4, 60.0 * ui);
                spawn_popup(&mut commands, &fonts, assets.icons.get(ico).cloned(), None, "", Color::WHITE, at, 110.0 * ui, 1.4, 60.0 * ui);
                spawn_popup(&mut commands, &fonts, None, None, label, col, at + Vec2::Y * 72.0 * ui, 150.0 * ui, 1.4, 60.0 * ui);
            }
            GameEvent::Landed { car, air } if *car == me && *air > 1.0 => {
                let pos = cars.iter().find(|c| *c.0 == me).map(|c| c.1.translation).unwrap_or_default();
                spawn_popup(&mut commands, &fonts, plate("pop_air"), None, &format!("ПОЛЁТ\n{:.1} с", air), Color::srgb(0.75, 0.95, 1.0), to_screen(pos), 170.0 * ui, 1.3, 50.0 * ui);
            }
            GameEvent::PedKilled { by: Some(b), pos, .. } if *b == me => {
                spawn_popup(&mut commands, &fonts, plate("pop_ped"), None, "+1", Color::WHITE, to_screen(*pos), 90.0 * ui, 0.9, 40.0 * ui);
                acc.combo.push(now);
            }
            GameEvent::Message { to: None, text, big: true, .. } => {
                // старт: светофор над цифрами отсчёта, вспышка с флагом на «ВПЕРЁД!»
                // 3 — красный, 2 и 1 — жёлтый, «ВПЕРЁД!» — зелёный и вспышка с флагом
                let light = match text.as_str() {
                    "3" => plate("start_light_red"),
                    "2" | "1" => plate("start_light_yellow"),
                    "ВПЕРЁД!" => plate("start_light_green"),
                    _ => None,
                };
                if light.is_some() {
                    let go = text == "ВПЕРЁД!";
                    spawn_popup(&mut commands, &fonts, light, None, "", Color::WHITE, screen * Vec2::new(0.5, 0.2), 220.0 * ui, if go { 1.2 } else { 0.95 }, 0.0);
                    if go {
                        spawn_popup_ex(&mut commands, &fonts, plate("start_go"), None, "ВПЕРЁД!", Color::WHITE, screen * Vec2::new(0.5, 0.36), 340.0 * ui, 1.1, 0.0, Some((0.752, 0.085)));
                    }
                }
            }
            _ => {}
        }
    }
    // удары по цели, накопленные за 0.35 с
    let ready: Vec<NetId> = acc.pending.iter().filter(|(_, v)| now - v.1 > 0.35).map(|(k, _)| *k).collect();
    for id in ready {
        let Some((amount, _, pos)) = acc.pending.remove(&id) else { continue };
        if amount < 4.0 {
            continue;
        }
        let (pl, word, size) = if amount < 10.0 {
            ("pop_hit_1", "БАМ!", 150.0)
        } else if amount < 22.0 {
            ("pop_hit_2", "ХРЯСЬ!", 190.0)
        } else {
            ("pop_hit_3", "В ХЛАМ!", 240.0)
        };
        spawn_popup(&mut commands, &fonts, plate(pl), None, &format!("{word}\n{:.0}", amount), Color::WHITE, to_screen(pos), size * ui, 1.0, 45.0 * ui);
    }
    // комбо: 3 и больше разбитых или пешеходов за 4 с
    acc.combo.retain(|t| now - t < 4.0);
    if acc.combo.len() >= 3 {
        let n = acc.combo.len();
        acc.combo.clear();
        spawn_popup(&mut commands, &fonts, plate("pop_combo"), None, &format!("КОМБО ×{n}"), Color::WHITE, center, 300.0 * ui, 1.5, 30.0 * ui);
    }
}

fn animate(
    mut commands: Commands,
    // игровое время: на медленном рендере (скриншоты) и в паузе анимация не убегает
    time: Res<Time>,
    mut q: Query<(Entity, &mut Popup, &mut Node, &mut UiTransform, &Children)>,
    mut imgs: Query<&mut ImageNode>,
    mut texts: Query<&mut TextColor, With<PopupText>>,
) {
    let dt = time.delta_secs();
    for (e, mut p, mut node, mut tf, children) in q.iter_mut() {
        p.t += dt;
        if p.t >= p.life {
            commands.entity(e).despawn();
            continue;
        }
        // отскок: 0.4 -> 1.2 -> 1.0
        let s = if p.t < 0.12 {
            0.4 + (p.t / 0.12) * 0.8
        } else if p.t < 0.24 {
            1.2 - ((p.t - 0.12) / 0.12) * 0.2
        } else {
            1.0
        };
        tf.scale = Vec2::splat(s);
        let k = p.t / p.life;
        node.top = Val::Px(p.at.y - p.size * 0.5 - p.rise * k);
        let a = ((p.life - p.t) / 0.35).clamp(0.0, 1.0);
        for c in children.iter() {
            if let Ok(mut im) = imgs.get_mut(c) {
                im.color.set_alpha(a);
            }
            if let Ok(mut tc) = texts.get_mut(c) {
                tc.0.set_alpha(a);
            }
        }
    }
}

fn debug_once(msg: &str) {
    static SEEN: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
    if let Ok(mut v) = SEEN.lock() {
        if !v.iter().any(|m| m == msg) {
            v.push(msg.to_string());
            info!("всплывашки: {msg}");
        }
    }
}
