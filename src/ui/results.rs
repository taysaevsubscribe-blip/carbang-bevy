//! Экран итогов: таблица, награда, опыт, новый уровень.

use super::menu::{MenuDirty, MenuScreen};
use super::*;
use crate::config::*;
use crate::data::*;
use crate::game::*;

#[derive(Component, Clone, Copy)]
pub enum ResultsAction {
    Continue,
    Retry,
    Garage,
}

pub struct ResultsPlugin;

impl Plugin for ResultsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Results), spawn_results).add_systems(Update, results_actions.run_if(in_state(AppState::Results)));
    }
}

fn spawn_results(mut commands: Commands, fonts: Res<Fonts>, result: Res<MatchResult>, profile: Res<Profile>, mode: Res<NetMode>, cfg: Res<MatchConfig>) {
    commands.insert_resource(ClearColor(Color::srgb(0.04, 0.04, 0.05)));
    commands.spawn((Camera2d, DespawnOnExit(AppState::Results)));
    let f = fonts.clone();
    commands
        .spawn((
            Node { width: Val::Percent(100.0), height: Val::Percent(100.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, padding: UiRect::all(Val::Px(30.0)), row_gap: Val::Px(12.0), ..default() },
            DespawnOnExit(AppState::Results),
        ))
        .with_children(|c| {
            c.spawn((
                Text::new(if result.won { "ПОБЕДА!" } else { "ЗАЕЗД ОКОНЧЕН" }),
                TextFont { font: f.title.clone().into(), font_size: FontSize::Px(64.0), ..default() },
                TextColor(if result.won { pal::GOLD } else { pal::ACCENT }),
            ));
            txt(c, result.reason.clone(), &f.bold, 22.0, pal::TEXT);
            if let Some(i) = result.career_event {
                txt(c, format!("Карьера: «{}»", CAREER[i].name), &f.body, 18.0, pal::DIM);
            }
            txt(c, format!("{} · {}", cfg.mode.name(), map_info(cfg.map).name), &f.body, 18.0, pal::DIM);
            panel(c, Node { flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(16.0)), row_gap: Val::Px(4.0), ..default() }, pal::PANEL, |p| {
                let widths = [50.0, 220.0, 150.0, 110.0, 90.0, 90.0, 90.0, 90.0];
                let hdr = ["#", "Игрок", "Машина", "Очки", "Разбил", "Пеших", "Смертей", "Урон"];
                p.spawn(Node { flex_direction: FlexDirection::Row, ..default() }).with_children(|r| {
                    for (i, h) in hdr.iter().enumerate() {
                        txt_node(r, *h, &f.bold, 17.0, pal::DIM, Node { width: Val::Px(widths[i]), ..default() });
                    }
                });
                for (k, row) in result.rows.iter().enumerate() {
                    let col = if row.is_local { pal::GOLD } else { pal::TEXT };
                    let cells = [
                        format!("{}", k + 1),
                        row.name.clone(),
                        row.car.clone(),
                        row.score.to_string(),
                        row.wrecks.to_string(),
                        row.peds.to_string(),
                        row.deaths.to_string(),
                        row.damage.to_string(),
                    ];
                    p.spawn(Node { flex_direction: FlexDirection::Row, ..default() }).with_children(|r| {
                        for (i, cell) in cells.iter().enumerate() {
                            txt_node(r, cell.clone(), &f.body, 18.0, col, Node { width: Val::Px(widths[i]), ..default() });
                        }
                    });
                }
            });
            row(c, 30.0, |r| {
                txt(r, format!("Заработано: {}", money(result.credits)), &f.title, 26.0, pal::GOLD);
                txt(r, format!("Опыт: +{}", result.xp), &f.title, 26.0, pal::ACCENT);
            });
            if result.level_after > result.level_before {
                txt(c, format!("НОВЫЙ УРОВЕНЬ {}! Открыты новые машины, оружие, перки и карты.", result.level_after), &f.title, 24.0, pal::GOOD);
            }
            txt(c, format!("На счету: {}", money(profile.credits)), &f.bold, 20.0, pal::TEXT);
            spacer(c, 10.0);
            row(c, 12.0, |r| {
                button_sized(r, &f, "ПРОДОЛЖИТЬ", ResultsAction::Continue, Val::Px(260.0), Val::Px(52.0), 22.0, true, false);
                if *mode == NetMode::Offline {
                    button_sized(r, &f, "ЕЩЁ РАЗ", ResultsAction::Retry, Val::Px(220.0), Val::Px(52.0), 22.0, false, false);
                }
                button_sized(r, &f, "ГАРАЖ", ResultsAction::Garage, Val::Px(200.0), Val::Px(52.0), 22.0, false, false);
            });
        });
}

#[allow(clippy::too_many_arguments)]
fn results_actions(
    q: Query<(&Interaction, &ResultsAction), Changed<Interaction>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut next: ResMut<NextState<AppState>>,
    mut screen: ResMut<MenuScreen>,
    mut dirty: ResMut<MenuDirty>,
    mode: Res<NetMode>,
    mut cfg: ResMut<MatchConfig>,
    profile: Res<Profile>,
    mut click: MessageWriter<UiClick>,
) {
    let mut act = None;
    for (i, a) in q.iter() {
        if *i == Interaction::Pressed {
            act = Some(*a);
        }
    }
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Escape) {
        act = Some(ResultsAction::Continue);
    }
    let Some(a) = act else { return };
    click.write(UiClick);
    let back = if *mode == NetMode::Offline { MenuScreen::Main } else { MenuScreen::Lobby };
    match a {
        ResultsAction::Continue => {
            *screen = if cfg.career_event.is_some() && *mode == NetMode::Offline { MenuScreen::Career } else { back };
            next.set(AppState::Menu);
        }
        ResultsAction::Garage => {
            *screen = MenuScreen::Garage;
            next.set(AppState::Menu);
        }
        ResultsAction::Retry => {
            cfg.seed = rand::random::<u64>() % 1_000_000;
            // обновить машину игрока из гаража
            for p in cfg.players.iter_mut() {
                if p.controller == Controller::Local {
                    p.config = profile.selected_config();
                    p.name = profile.name.clone();
                }
            }
            next.set(AppState::Loading);
        }
    }
    dirty.0 = true;
}
