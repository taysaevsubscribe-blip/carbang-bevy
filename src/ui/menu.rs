//! Главное меню, карьера, быстрый матч, сетевая игра, лобби, настройки.

use super::*;
use crate::config::*;
use crate::data::*;
use crate::game::*;
use crate::net::{LobbyInfo, NetCommand, NetStatus};
use crate::tests::add_bots;
use bevy::app::AppExit;

#[derive(Resource, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum MenuScreen {
    #[default]
    Main,
    Career,
    Quick,
    Garage,
    Multiplayer,
    Lobby,
    Settings,
}

#[derive(Resource, Default)]
pub struct MenuDirty(pub bool);

#[derive(Resource)]
pub struct QuickSetup {
    pub map: MapId,
    pub mode: GameMode,
    pub bots: u32,
    pub difficulty: usize,
}

impl Default for QuickSetup {
    fn default() -> Self {
        Self { map: MapId::Port, mode: GameMode::Carnage, bots: 6, difficulty: 1 }
    }
}

pub const DIFFICULTIES: [(&str, f32); 4] = [("Лёгкая", 0.25), ("Средняя", 0.5), ("Сложная", 0.75), ("Безумная", 1.0)];

#[derive(Resource, Default)]
pub struct CareerPick(pub usize);

#[derive(Component)]
pub struct MenuRoot;

#[derive(Component, Clone, Debug)]
pub enum MenuAction {
    Screen(MenuScreen),
    Quit,
    CareerSelect(usize),
    CareerStart,
    QuickMap(MapId),
    QuickMode(GameMode),
    QuickBots(i32),
    QuickDiff(usize),
    QuickStart,
    Quality(u8),
    Fullscreen,
    Volume(i32),
    CameraDist(i32),
    ShowFps,
    Zombies,
    Edit(TextField),
    Host,
    Join,
    LobbyMap(i32),
    LobbyMode(i32),
    LobbyBots(i32),
    LobbyDiff,
    LobbyStart,
    LobbyLeave,
}

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuScreen>()
            .init_resource::<MenuDirty>()
            .init_resource::<QuickSetup>()
            .init_resource::<CareerPick>()
            .add_systems(OnEnter(AppState::Menu), |mut d: ResMut<MenuDirty>| d.0 = true)
            .add_systems(Startup, cli_screen)
            .add_systems(Update, (menu_actions, menu_back, rebuild_menu).chain().run_if(in_state(AppState::Menu)));
    }
}

fn cli_screen(cli: Res<crate::Cli>, mut screen: ResMut<MenuScreen>, mut gs: ResMut<super::garage::GarageState>) {
    use super::garage::GarageTab;
    if let Some(m) = cli.menu.as_deref() {
        *screen = match m {
            "career" => MenuScreen::Career,
            "quick" => MenuScreen::Quick,
            "garage" => MenuScreen::Garage,
            "mp" => MenuScreen::Multiplayer,
            "lobby" => MenuScreen::Lobby,
            "settings" => MenuScreen::Settings,
            _ => MenuScreen::Main,
        };
    }
    if let Some(t) = cli.tab.as_deref() {
        gs.tab = match t {
            "tuning" => GarageTab::Tuning,
            "setup" => GarageTab::Setup,
            "weapons" => GarageTab::Weapons,
            "perks" => GarageTab::Perks,
            "paint" => GarageTab::Paint,
            _ => GarageTab::Cars,
        };
    }
}

fn menu_back(keys: Res<ButtonInput<KeyCode>>, mut screen: ResMut<MenuScreen>, edit: Res<TextEdit>, mut net: MessageWriter<NetCommand>) {
    if edit.field.is_some() {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) && *screen != MenuScreen::Main {
        if *screen == MenuScreen::Lobby {
            net.write(NetCommand::Leave);
            *screen = MenuScreen::Multiplayer;
        } else {
            *screen = MenuScreen::Main;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn rebuild_menu(
    mut commands: Commands,
    fonts: Res<Fonts>,
    screen: Res<MenuScreen>,
    mut dirty: ResMut<MenuDirty>,
    roots: Query<Entity, With<MenuRoot>>,
    profile: Res<Profile>,
    settings: Res<Settings>,
    quick: Res<QuickSetup>,
    career: Res<CareerPick>,
    lobby: Res<LobbyInfo>,
    status: Res<NetStatus>,
    edit: Res<TextEdit>,
    mode: Res<NetMode>,
    garage: Res<super::garage::GarageState>,
) {
    if !dirty.0 && !screen.is_changed() && !status.is_changed() && !lobby.is_changed() && !garage.is_changed() && !profile.is_changed() {
        return;
    }
    dirty.0 = false;
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
    let f = fonts.clone();
    commands
        .spawn((
            Node { width: Val::Percent(100.0), height: Val::Percent(100.0), padding: UiRect::all(Val::Px(28.0)), ..default() },
            MenuRoot,
            DespawnOnExit(AppState::Menu),
        ))
        .with_children(|root| {
            // Профиль в правом верхнем углу
            if *screen != MenuScreen::Garage {
                profile_badge(root, &f, &profile);
            }
            match *screen {
                MenuScreen::Main => main_screen(root, &f),
                MenuScreen::Career => career_screen(root, &f, &profile, career.0),
                MenuScreen::Quick => quick_screen(root, &f, &profile, &quick),
                MenuScreen::Garage => super::garage::build_garage(root, &f, &profile, &garage),
                MenuScreen::Multiplayer => mp_screen(root, &f, &profile, &settings, &status, &edit),
                MenuScreen::Lobby => lobby_screen(root, &f, &lobby, &status, *mode == NetMode::Host),
                MenuScreen::Settings => settings_screen(root, &f, &settings, &profile),
            }
        });
}

fn profile_badge(root: &mut ChildSpawnerCommands, f: &Fonts, profile: &Profile) {
    let lvl = profile.level();
    let lo = xp_for_level(lvl);
    let hi = xp_for_level(lvl + 1).max(lo + 1);
    let frac = (profile.xp - lo) as f32 / (hi - lo) as f32;
    root.spawn((
        Node { border_radius: BorderRadius::all(Val::Px(8.0)),
            position_type: PositionType::Absolute,
            right: Val::Px(28.0),
            top: Val::Px(28.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(14.0)),
            row_gap: Val::Px(6.0),
            min_width: Val::Px(260.0),
            ..default()
        },
        BackgroundColor(pal::PANEL),
        ))
    .with_children(|p| {
        txt(p, profile.name.clone(), &f.title, 22.0, pal::TEXT);
        txt(p, format!("Уровень {}", lvl), &f.bold, 18.0, pal::ACCENT);
        bar(p, frac, 232.0, 8.0, pal::ACCENT);
        txt(p, format!("Опыт {} / {}", profile.xp, hi), &f.body, 15.0, pal::DIM);
        txt(p, money(profile.credits), &f.title, 24.0, pal::GOLD);
    });
}

fn title(root: &mut ChildSpawnerCommands, f: &Fonts, sub: &str) {
    root.spawn(Node { flex_direction: FlexDirection::Column, margin: UiRect::bottom(Val::Px(18.0)), ..default() }).with_children(|t| {
        t.spawn((Text::new("CARBANG"), TextFont { font: f.title.clone().into(), font_size: FontSize::Px(72.0), ..default() }, TextColor(pal::ACCENT)));
        txt(t, sub, &f.bold, 22.0, pal::TEXT);
    });
}

fn left_column(root: &mut ChildSpawnerCommands, width: f32, f: impl FnOnce(&mut ChildSpawnerCommands)) {
    root.spawn(Node { flex_direction: FlexDirection::Column, width: Val::Px(width), row_gap: Val::Px(10.0), ..default() }).with_children(f);
}

fn main_screen(root: &mut ChildSpawnerCommands, f: &Fonts) {
    left_column(root, 420.0, |c| {
        title(c, f, "Гонки на выживание. Без правил.");
        let w = Val::Px(380.0);
        for (label, action) in [
            ("КАРЬЕРА", MenuAction::Screen(MenuScreen::Career)),
            ("БЫСТРЫЙ МАТЧ", MenuAction::Screen(MenuScreen::Quick)),
            ("ГАРАЖ", MenuAction::Screen(MenuScreen::Garage)),
            ("СЕТЕВАЯ ИГРА", MenuAction::Screen(MenuScreen::Multiplayer)),
            ("НАСТРОЙКИ", MenuAction::Screen(MenuScreen::Settings)),
            ("ВЫХОД", MenuAction::Quit),
        ] {
            button_sized(c, f, label, action, w, Val::Px(54.0), 24.0, false, false);
        }
        spacer(c, 20.0);
        txt(c, "W A S D: газ, тормоз, руль · ПРОБЕЛ: ручник · SHIFT: нитро\nЛКМ/Ctrl: огонь · ПКМ/E: спецоружие · Q: сброс мин · F: прыжок\nR: эвакуация · C: камера · V: назад · TAB: таблица · ESC: пауза", &f.body, 16.0, pal::DIM);
        spacer(c, 14.0);
        // титры: атрибуция CC-BY обязательна
        txt(
            c,
            "Звуки CC-BY 3.0: mattvic79 (тяжёлый мотор), jorgebaptista (мужские крики),\nthanvannispen и tcrocker68 (женские крики) — OpenGameArt / Freesound.\nCC0: Kenney, ambientCG, Poly Haven. Шрифты: Russo One, PT Sans (OFL).",
            &f.body,
            12.0,
            Color::srgba(0.6, 0.6, 0.62, 0.8),
        );
    });
}

fn back_button(c: &mut ChildSpawnerCommands, f: &Fonts) {
    button(c, f, "« НАЗАД", MenuAction::Screen(MenuScreen::Main), Val::Px(200.0), false, false);
}

fn career_screen(root: &mut ChildSpawnerCommands, f: &Fonts, profile: &Profile, pick: usize) {
    left_column(root, 760.0, |c| {
        title(c, f, "Карьера: 12 заездов от двора до столицы");
        c.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(16.0), ..default() }).with_children(|r| {
            // список
            r.spawn((
                Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(6.0), width: Val::Px(360.0), max_height: Val::Px(560.0), overflow: Overflow::scroll_y(), ..default() },
                ScrollPosition::default(),
                Scrollable,
                Interaction::default(),
            ))
            .with_children(|l| {
                for (i, ev) in CAREER.iter().enumerate() {
                    let locked = i as u32 > profile.career_done;
                    let done = (i as u32) < profile.career_done;
                    let mark = if done { "+ " } else if locked { "× " } else { "» " };
                    button_sized(l, f, format!("{}{}. {}", mark, i + 1, ev.name), MenuAction::CareerSelect(i), Val::Px(350.0), Val::Px(42.0), 18.0, i == pick, locked);
                }
            });
            // описание
            let ev = &CAREER[pick.min(CAREER.len() - 1)];
            let locked = pick as u32 > profile.career_done;
            panel(r, Node { flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(18.0)), row_gap: Val::Px(10.0), width: Val::Px(380.0), ..default() }, pal::PANEL, |p| {
                txt(p, ev.name, &f.title, 28.0, pal::ACCENT);
                let mi = map_info(ev.map);
                txt(p, format!("Карта: {}", mi.name), &f.bold, 18.0, pal::TEXT);
                txt(p, mi.desc, &f.body, 16.0, pal::DIM);
                txt(p, format!("Режим: {}", ev.mode.name()), &f.bold, 18.0, pal::TEXT);
                txt(p, ev.mode.desc(), &f.body, 16.0, pal::DIM);
                txt(p, format!("Соперников: {}", ev.bots), &f.body, 17.0, pal::TEXT);
                let diff = DIFFICULTIES.iter().min_by(|a, b| (a.1 - ev.difficulty).abs().partial_cmp(&(b.1 - ev.difficulty).abs()).unwrap()).map(|d| d.0).unwrap_or("");
                txt(p, format!("Сложность: {}", diff), &f.body, 17.0, pal::TEXT);
                txt(p, format!("Награда за победу: {}", money(ev.reward)), &f.bold, 18.0, pal::GOLD);
                spacer(p, 8.0);
                let car = car_def(&profile.selected_car);
                txt(p, format!("Твоя машина: {}", car.name), &f.body, 17.0, pal::TEXT);
                button(p, f, if locked { "СНАЧАЛА ПРОЙДИ ПРЕДЫДУЩИЙ" } else { "СТАРТ" }, MenuAction::CareerStart, Val::Px(340.0), !locked, locked);
                button(p, f, "В ГАРАЖ", MenuAction::Screen(MenuScreen::Garage), Val::Px(340.0), false, false);
            });
        });
        back_button(c, f);
    });
}

fn quick_screen(root: &mut ChildSpawnerCommands, f: &Fonts, profile: &Profile, q: &QuickSetup) {
    let lvl = profile.level();
    left_column(root, 860.0, |c| {
        title(c, f, "Быстрый матч");
        txt(c, "Карта", &f.bold, 20.0, pal::TEXT);
        c.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(8.0), row_gap: Val::Px(8.0), ..default() }).with_children(|r| {
            for m in MAPS {
                let locked = m.unlock_level > lvl;
                let label = if locked { format!("{} (ур. {})", m.name, m.unlock_level) } else { m.name.to_string() };
                button_sized(r, f, label, MenuAction::QuickMap(m.id), Val::Px(270.0), Val::Px(42.0), 17.0, q.map == m.id, locked);
            }
        });
        txt(c, map_info(q.map).desc, &f.body, 16.0, pal::DIM);
        spacer(c, 6.0);
        txt(c, "Режим", &f.bold, 20.0, pal::TEXT);
        row(c, 8.0, |r| {
            for m in GameMode::ALL {
                button_sized(r, f, m.name(), MenuAction::QuickMode(m), Val::Px(205.0), Val::Px(42.0), 16.0, q.mode == m, false);
            }
        });
        txt(c, q.mode.desc(), &f.body, 16.0, pal::DIM);
        spacer(c, 6.0);
        row(c, 10.0, |r| {
            txt(r, "Соперники", &f.bold, 20.0, pal::TEXT);
            button(r, f, "–", MenuAction::QuickBots(-1), Val::Px(44.0), false, q.bots <= 1);
            txt_node(r, format!("{}", q.bots), &f.title, 24.0, pal::ACCENT, Node { width: Val::Px(40.0), ..default() });
            button(r, f, "+", MenuAction::QuickBots(1), Val::Px(44.0), false, q.bots >= 11);
            spacer(r, 0.0);
            txt(r, "  Сложность", &f.bold, 20.0, pal::TEXT);
            for (i, d) in DIFFICULTIES.iter().enumerate() {
                button_sized(r, f, d.0, MenuAction::QuickDiff(i), Val::Px(120.0), Val::Px(42.0), 16.0, q.difficulty == i, false);
            }
        });
        spacer(c, 10.0);
        row(c, 10.0, |r| {
            back_button(r, f);
            button_sized(r, f, "СТАРТ", MenuAction::QuickStart, Val::Px(300.0), Val::Px(54.0), 26.0, true, false);
            button(r, f, "ГАРАЖ", MenuAction::Screen(MenuScreen::Garage), Val::Px(160.0), false, false);
        });
    });
}

fn mp_screen(root: &mut ChildSpawnerCommands, f: &Fonts, profile: &Profile, settings: &Settings, status: &NetStatus, edit: &TextEdit) {
    left_column(root, 620.0, |c| {
        title(c, f, "Сетевая игра по IP");
        txt(c, "Имя игрока", &f.bold, 18.0, pal::TEXT);
        let editing_name = edit.field == Some(TextField::Name);
        button(c, f, format!("{}{}", profile.name, if editing_name { "_" } else { "" }), MenuAction::Edit(TextField::Name), Val::Px(420.0), editing_name, false);
        spacer(c, 6.0);
        txt(c, "Адрес сервера (для подключения)", &f.bold, 18.0, pal::TEXT);
        let editing_ip = edit.field == Some(TextField::Ip);
        button(c, f, format!("{}{}", settings.last_ip, if editing_ip { "_" } else { "" }), MenuAction::Edit(TextField::Ip), Val::Px(420.0), editing_ip, false);
        txt(c, format!("Порт UDP {}. Хосту нужно открыть этот порт или играть в одной сети (LAN, Radmin VPN, ZeroTier).", settings.port), &f.body, 15.0, pal::DIM);
        spacer(c, 8.0);
        row(c, 10.0, |r| {
            button_sized(r, f, "СОЗДАТЬ ИГРУ", MenuAction::Host, Val::Px(260.0), Val::Px(52.0), 22.0, false, false);
            button_sized(r, f, "ПОДКЛЮЧИТЬСЯ", MenuAction::Join, Val::Px(260.0), Val::Px(52.0), 22.0, false, false);
        });
        if !status.text.is_empty() {
            txt(c, status.text.clone(), &f.bold, 18.0, if status.error { pal::BAD } else { pal::GOOD });
        }
        spacer(c, 10.0);
        txt(c, "Машина и тюнинг берутся из гаража. Хост выбирает карту, режим и число ботов.", &f.body, 16.0, pal::DIM);
        back_button(c, f);
    });
}

fn lobby_screen(root: &mut ChildSpawnerCommands, f: &Fonts, lobby: &LobbyInfo, status: &NetStatus, is_host: bool) {
    left_column(root, 860.0, |c| {
        title(c, f, if is_host { "Лобби: ты хост" } else { "Лобби: ждём старта" });
        c.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(16.0), ..default() }).with_children(|r| {
            panel(r, Node { flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(16.0)), row_gap: Val::Px(8.0), width: Val::Px(400.0), ..default() }, pal::PANEL, |p| {
                txt(p, format!("Игроки ({})", lobby.players.len()), &f.title, 22.0, pal::ACCENT);
                for pl in &lobby.players {
                    let car = car_def(&pl.car);
                    txt(p, format!("{}  ·  {}  ·  {} мс", pl.name, car.name, pl.ping), &f.bold, 18.0, pal::TEXT);
                }
                if lobby.bots > 0 {
                    txt(p, format!("+ ботов: {}", lobby.bots), &f.body, 17.0, pal::DIM);
                }
            });
            panel(r, Node { flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(16.0)), row_gap: Val::Px(8.0), width: Val::Px(420.0), ..default() }, pal::PANEL, |p| {
                let mi = map_info(lobby.map);
                txt(p, "Карта", &f.bold, 17.0, pal::DIM);
                row(p, 8.0, |rr| {
                    if is_host {
                        button(rr, f, "«", MenuAction::LobbyMap(-1), Val::Px(44.0), false, false);
                    }
                    txt_node(rr, mi.name, &f.title, 20.0, pal::TEXT, Node { width: Val::Px(260.0), ..default() });
                    if is_host {
                        button(rr, f, "»", MenuAction::LobbyMap(1), Val::Px(44.0), false, false);
                    }
                });
                txt(p, "Режим", &f.bold, 17.0, pal::DIM);
                row(p, 8.0, |rr| {
                    if is_host {
                        button(rr, f, "«", MenuAction::LobbyMode(-1), Val::Px(44.0), false, false);
                    }
                    txt_node(rr, lobby.mode.name(), &f.title, 20.0, pal::TEXT, Node { width: Val::Px(260.0), ..default() });
                    if is_host {
                        button(rr, f, "»", MenuAction::LobbyMode(1), Val::Px(44.0), false, false);
                    }
                });
                txt(p, "Боты", &f.bold, 17.0, pal::DIM);
                row(p, 8.0, |rr| {
                    if is_host {
                        button(rr, f, "–", MenuAction::LobbyBots(-1), Val::Px(44.0), false, false);
                    }
                    txt_node(rr, format!("{}", lobby.bots), &f.title, 20.0, pal::TEXT, Node { width: Val::Px(60.0), ..default() });
                    if is_host {
                        button(rr, f, "+", MenuAction::LobbyBots(1), Val::Px(44.0), false, false);
                    }
                    let d = DIFFICULTIES.iter().min_by(|a, b| (a.1 - lobby.difficulty).abs().partial_cmp(&(b.1 - lobby.difficulty).abs()).unwrap()).map(|d| d.0).unwrap_or("");
                    if is_host {
                        button(rr, f, d, MenuAction::LobbyDiff, Val::Px(150.0), false, false);
                    } else {
                        txt(rr, d, &f.body, 18.0, pal::TEXT);
                    }
                });
            });
        });
        if !status.text.is_empty() {
            txt(c, status.text.clone(), &f.bold, 18.0, if status.error { pal::BAD } else { pal::GOOD });
        }
        row(c, 10.0, |r| {
            button(r, f, "« ВЫЙТИ", MenuAction::LobbyLeave, Val::Px(200.0), false, false);
            if is_host {
                button_sized(r, f, "СТАРТ", MenuAction::LobbyStart, Val::Px(300.0), Val::Px(54.0), 26.0, true, false);
            }
            button(r, f, "ГАРАЖ", MenuAction::Screen(MenuScreen::Garage), Val::Px(160.0), false, false);
        });
    });
}

fn settings_screen(root: &mut ChildSpawnerCommands, f: &Fonts, s: &Settings, profile: &Profile) {
    left_column(root, 760.0, |c| {
        title(c, f, "Настройки");
        let line = |c: &mut ChildSpawnerCommands, name: &str, f: &Fonts, inner: &dyn Fn(&mut ChildSpawnerCommands)| {
            row(c, 10.0, |r| {
                txt_node(r, name, &f.bold, 19.0, pal::TEXT, Node { width: Val::Px(230.0), ..default() });
                inner(r);
            });
        };
        line(c, "Графика", f, &|r| {
            for (i, n) in ["Низкая", "Средняя", "Высокая"].iter().enumerate() {
                button_sized(r, f, *n, MenuAction::Quality(i as u8), Val::Px(140.0), Val::Px(42.0), 17.0, s.quality == i as u8, false);
            }
        });
        line(c, "Полный экран", f, &|r| {
            button(r, f, if s.fullscreen { "ВКЛ" } else { "ВЫКЛ" }, MenuAction::Fullscreen, Val::Px(140.0), s.fullscreen, false);
        });
        line(c, "Громкость", f, &|r| {
            button(r, f, "–", MenuAction::Volume(-1), Val::Px(44.0), false, false);
            txt_node(r, format!("{:.0}%", s.volume * 100.0), &f.title, 20.0, pal::ACCENT, Node { width: Val::Px(70.0), ..default() });
            button(r, f, "+", MenuAction::Volume(1), Val::Px(44.0), false, false);
        });
        line(c, "Дистанция камеры", f, &|r| {
            button(r, f, "–", MenuAction::CameraDist(-1), Val::Px(44.0), false, false);
            txt_node(r, format!("{:.1}", s.camera_distance), &f.title, 20.0, pal::ACCENT, Node { width: Val::Px(70.0), ..default() });
            button(r, f, "+", MenuAction::CameraDist(1), Val::Px(44.0), false, false);
        });
        line(c, "Счётчик FPS", f, &|r| {
            button(r, f, if s.show_fps { "ВКЛ" } else { "ВЫКЛ" }, MenuAction::ShowFps, Val::Px(140.0), s.show_fps, false);
        });
        line(c, "Пешеходы-зомби", f, &|r| {
            button(r, f, if s.zombies { "ВКЛ" } else { "ВЫКЛ" }, MenuAction::Zombies, Val::Px(140.0), s.zombies, false);
            txt(r, "зелёная кровь вместо красной", &f.body, 15.0, pal::DIM);
        });
        spacer(c, 8.0);
        txt(c, format!("Статистика: разбито машин {}, сбито пешеходов {}, пройдено заездов карьеры {}", profile.total_wrecks, profile.total_peds, profile.career_done), &f.body, 16.0, pal::DIM);
        txt(c, format!("Сохранения: {}", data_dir().display()), &f.body, 14.0, pal::DIM);
        back_button(c, f);
    });
}

// ============================================================ действия

pub fn player_slot(profile: &Profile) -> PlayerSlot {
    PlayerSlot { name: profile.name.clone(), config: profile.selected_config(), controller: Controller::Local, bot_seed: 0 }
}

#[allow(clippy::too_many_arguments)]
fn menu_actions(
    q: Query<(&Interaction, &MenuAction, &BtnStyle), Changed<Interaction>>,
    mut screen: ResMut<MenuScreen>,
    mut dirty: ResMut<MenuDirty>,
    mut quick: ResMut<QuickSetup>,
    mut career: ResMut<CareerPick>,
    profile: ResMut<Profile>,
    mut settings: ResMut<Settings>,
    mut cfg: ResMut<MatchConfig>,
    mut next: ResMut<NextState<AppState>>,
    mut net_mode: ResMut<NetMode>,
    mut edit: ResMut<TextEdit>,
    mut lobby: ResMut<LobbyInfo>,
    mut net: MessageWriter<NetCommand>,
    mut exit: MessageWriter<AppExit>,
    mut click: MessageWriter<UiClick>,
    mut windows: Query<&mut Window>,
) {
    for (i, action, style) in q.iter() {
        if *i != Interaction::Pressed || style.disabled {
            continue;
        }
        click.write(UiClick);
        if !matches!(action, MenuAction::Edit(_)) && edit.field.is_some() {
            edit.field = None;
            profile.save();
            settings.save();
        }
        match action.clone() {
            MenuAction::Screen(s) => *screen = s,
            MenuAction::Quit => {
                exit.write(AppExit::Success);
            }
            MenuAction::CareerSelect(i) => career.0 = i,
            MenuAction::CareerStart => {
                let i = career.0;
                let ev = &CAREER[i];
                *net_mode = NetMode::Offline;
                *cfg = MatchConfig { map: ev.map, mode: ev.mode, seed: rand::random::<u64>() % 1_000_000, difficulty: ev.difficulty, career_event: Some(i), zombies: settings.zombies, ..default() };
                cfg.apply_map_defaults();
                cfg.players.push(player_slot(&profile));
                add_bots(&mut cfg, ev.bots as usize, profile.level() + i as u32 / 2);
                next.set(AppState::Loading);
            }
            MenuAction::QuickMap(m) => quick.map = m,
            MenuAction::QuickMode(m) => quick.mode = m,
            MenuAction::QuickBots(d) => quick.bots = (quick.bots as i32 + d).clamp(1, 11) as u32,
            MenuAction::QuickDiff(d) => quick.difficulty = d,
            MenuAction::QuickStart => {
                *net_mode = NetMode::Offline;
                *cfg = MatchConfig {
                    map: quick.map,
                    mode: quick.mode,
                    seed: rand::random::<u64>() % 1_000_000,
                    difficulty: DIFFICULTIES[quick.difficulty].1,
                    zombies: settings.zombies,
                    ..default()
                };
                cfg.apply_map_defaults();
                cfg.players.push(player_slot(&profile));
                add_bots(&mut cfg, quick.bots as usize, profile.level() + quick.difficulty as u32 * 2);
                next.set(AppState::Loading);
            }
            MenuAction::Quality(q) => {
                settings.quality = q;
                settings.save();
            }
            MenuAction::Fullscreen => {
                settings.fullscreen = !settings.fullscreen;
                settings.save();
                for mut w in windows.iter_mut() {
                    w.mode = if settings.fullscreen { bevy::window::WindowMode::BorderlessFullscreen(MonitorSelection::Current) } else { bevy::window::WindowMode::Windowed };
                }
            }
            MenuAction::Volume(d) => {
                settings.volume = ((settings.volume * 10.0).round() + d as f32).clamp(0.0, 10.0) / 10.0;
                settings.save();
            }
            MenuAction::CameraDist(d) => {
                settings.camera_distance = ((settings.camera_distance * 10.0).round() + d as f32).clamp(6.0, 16.0) / 10.0;
                settings.save();
            }
            MenuAction::ShowFps => {
                settings.show_fps = !settings.show_fps;
                settings.save();
            }
            MenuAction::Zombies => {
                settings.zombies = !settings.zombies;
                settings.save();
            }
            MenuAction::Edit(field) => {
                edit.field = if edit.field == Some(field) { None } else { Some(field) };
            }
            MenuAction::Host => {
                net.write(NetCommand::Host);
            }
            MenuAction::Join => {
                net.write(NetCommand::Join(settings.last_ip.clone()));
            }
            MenuAction::LobbyMap(d) => {
                let i = MAPS.iter().position(|m| m.id == lobby.map).unwrap_or(0) as i32;
                lobby.map = MAPS[(i + d).rem_euclid(MAPS.len() as i32) as usize].id;
                net.write(NetCommand::LobbyChanged);
            }
            MenuAction::LobbyMode(d) => {
                let i = GameMode::ALL.iter().position(|m| *m == lobby.mode).unwrap_or(0) as i32;
                lobby.mode = GameMode::ALL[(i + d).rem_euclid(4) as usize];
                net.write(NetCommand::LobbyChanged);
            }
            MenuAction::LobbyBots(d) => {
                lobby.bots = (lobby.bots as i32 + d).clamp(0, 10) as u32;
                net.write(NetCommand::LobbyChanged);
            }
            MenuAction::LobbyDiff => {
                let i = DIFFICULTIES.iter().position(|x| (x.1 - lobby.difficulty).abs() < 0.01).unwrap_or(1);
                lobby.difficulty = DIFFICULTIES[(i + 1) % DIFFICULTIES.len()].1;
                net.write(NetCommand::LobbyChanged);
            }
            MenuAction::LobbyStart => {
                net.write(NetCommand::Start);
            }
            MenuAction::LobbyLeave => {
                net.write(NetCommand::Leave);
                *screen = MenuScreen::Multiplayer;
            }
        }
        dirty.0 = true;
    }
}
