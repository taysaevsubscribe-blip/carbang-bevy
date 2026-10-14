//! Гараж: покупка машин, тюнинг уровнями, свободная настройка, оружие, перки, покраска.

use super::menu::{MenuDirty, MenuScreen};
use super::*;
use crate::config::*;
use crate::data::*;
use crate::game::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GarageTab {
    #[default]
    Cars,
    Tuning,
    Setup,
    Weapons,
    Perks,
    Paint,
}

#[derive(Resource, Default)]
pub struct GarageState {
    pub tab: GarageTab,
    /// Машина, которую смотрим во вкладке «Машины»
    pub view: Option<usize>,
    pub mount: Option<Mount>,
}

impl GarageState {
    /// Какую машину показывать в витрине.
    pub fn shown_config(&self, profile: &Profile) -> CarConfig {
        match (self.tab, self.view) {
            (GarageTab::Cars, Some(i)) => {
                let id = CARS[i].id;
                let mut c = profile.config_for(id);
                c.car_id = id.to_string();
                c
            }
            _ => profile.selected_config(),
        }
    }
}

#[derive(Component, Clone, Debug)]
pub enum GarageAction {
    Skin(u8),
    Tab(GarageTab),
    View(usize),
    BuyCar(usize),
    SelectCar(usize),
    BuyUpgrade(Upgrade),
    Setup(usize, i32),
    SetupReset,
    PickMount(Mount),
    Equip(Mount, Option<WeaponKind>),
    BuyWeapon(WeaponKind),
    UpgradeWeapon(WeaponKind),
    TogglePerk(PerkKind),
    Paint([f32; 3]),
    Back,
}

pub const PAINTS: [[f32; 3]; 18] = [
    [0.85, 0.12, 0.1],
    [0.95, 0.45, 0.05],
    [0.98, 0.78, 0.05],
    [0.45, 0.75, 0.1],
    [0.1, 0.55, 0.25],
    [0.05, 0.6, 0.65],
    [0.1, 0.35, 0.85],
    [0.15, 0.15, 0.5],
    [0.5, 0.15, 0.7],
    [0.9, 0.3, 0.6],
    [0.95, 0.95, 0.93],
    [0.6, 0.62, 0.65],
    [0.25, 0.26, 0.28],
    [0.05, 0.05, 0.06],
    [0.45, 0.3, 0.18],
    [0.75, 0.6, 0.4],
    [0.35, 0.4, 0.2],
    [0.55, 0.05, 0.08],
];

pub struct GaragePlugin;

impl Plugin for GaragePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GarageState>().add_systems(Update, garage_actions.run_if(in_state(AppState::Menu)));
    }
}

pub fn build_garage(root: &mut ChildSpawnerCommands, f: &Fonts, profile: &Profile, gs: &GarageState) {
    root.spawn(Node { flex_direction: FlexDirection::Column, width: Val::Px(700.0), row_gap: Val::Px(10.0), ..default() }).with_children(|c| {
        row(c, 14.0, |r| {
            r.spawn((Text::new("ГАРАЖ"), TextFont { font: f.title.clone().into(), font_size: FontSize::Px(48.0), ..default() }, TextColor(pal::ACCENT)));
            txt(r, money(profile.credits), &f.title, 26.0, pal::GOLD);
            txt(r, format!("ур. {}", profile.level()), &f.bold, 20.0, pal::TEXT);
        });
        row(c, 6.0, |r| {
            for (t, n) in [
                (GarageTab::Cars, "Машины"),
                (GarageTab::Tuning, "Тюнинг"),
                (GarageTab::Setup, "Настройка"),
                (GarageTab::Weapons, "Оружие"),
                (GarageTab::Perks, "Перки"),
                (GarageTab::Paint, "Покраска"),
            ] {
                button_sized(r, f, n, GarageAction::Tab(t), Val::Px(110.0), Val::Px(40.0), 16.0, gs.tab == t, false);
            }
        });
        panel(
            c,
            Node { flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(14.0)), row_gap: Val::Px(8.0), width: Val::Px(690.0), height: Val::Px(600.0), ..default() },
            pal::PANEL,
            |p| match gs.tab {
                GarageTab::Cars => cars_tab(p, f, profile, gs),
                GarageTab::Tuning => tuning_tab(p, f, profile),
                GarageTab::Setup => setup_tab(p, f, profile),
                GarageTab::Weapons => weapons_tab(p, f, profile, gs),
                GarageTab::Perks => perks_tab(p, f, profile),
                GarageTab::Paint => paint_tab(p, f, profile),
            },
        );
        button(c, f, "« НАЗАД", GarageAction::Back, Val::Px(200.0), false, false);
    });
    // Характеристики справа внизу
    let shown = gs.shown_config(profile);
    let def = shown.def();
    root.spawn((
        Node { border_radius: BorderRadius::all(Val::Px(8.0)),
            position_type: PositionType::Absolute,
            right: Val::Px(28.0),
            bottom: Val::Px(28.0),
            width: Val::Px(380.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(16.0)),
            row_gap: Val::Px(6.0),
            ..default()
        },
        BackgroundColor(pal::PANEL),
        ))
    .with_children(|p| {
        txt(p, def.name, &f.title, 28.0, pal::ACCENT);
        let stats = CarStats::compute(&shown);
        txt(
            p,
            format!(
                "{:.0} л.с. · {:.0} кг · {} · до {:.0} км/ч",
                stats.power / 735.5,
                stats.mass,
                match def.drive {
                    Drive::Fwd => "передний привод",
                    Drive::Rwd => "задний привод",
                    Drive::Awd => "полный привод",
                },
                stats.top_speed * 3.6
            ),
            &f.body,
            15.0,
            pal::DIM,
        );
        let base = rating(&CarConfig::new_default(def.id));
        for (i, (name, v)) in rating(&shown).iter().enumerate() {
            row(p, 8.0, |r| {
                txt_node(r, *name, &f.bold, 16.0, pal::TEXT, Node { width: Val::Px(100.0), ..default() });
                bar_delta(r, base[i].1, *v, 230.0, 12.0);
            });
        }
        txt(p, format!("Особенность: {}", def.trait_name), &f.bold, 16.0, pal::GOLD);
        txt(p, def.trait_desc, &f.body, 15.0, pal::DIM);
        let slots: Vec<&str> = def.slots.iter().map(|m| m.name()).collect();
        txt(p, format!("Точки оружия: {}", slots.join(", ")), &f.body, 15.0, pal::DIM);
    });
}

fn cars_tab(p: &mut ChildSpawnerCommands, f: &Fonts, profile: &Profile, gs: &GarageState) {
    let view = gs.view.unwrap_or_else(|| car_index(&profile.selected_car));
    p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(12.0), ..default() }).with_children(|r| {
        r.spawn((
            Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(5.0), width: Val::Px(250.0), height: Val::Px(570.0), overflow: Overflow::scroll_y(), ..default() },
            ScrollPosition::default(),
            Scrollable,
            Interaction::default(),
        ))
        .with_children(|l| {
            for (i, c) in CARS.iter().enumerate() {
                let owned = profile.owned_cars.iter().any(|o| o == c.id);
                let mark = if c.id == profile.selected_car { "» " } else if owned { "" } else { "× " };
                button_sized(l, f, format!("{}{}", mark, c.name), GarageAction::View(i), Val::Px(240.0), Val::Px(40.0), 17.0, i == view, false);
            }
        });
        let c = &CARS[view];
        let owned = profile.owned_cars.iter().any(|o| o == c.id);
        r.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(8.0), width: Val::Px(400.0), ..default() }).with_children(|d| {
            txt(d, c.name, &f.title, 30.0, pal::ACCENT);
            txt(d, c.desc, &f.body, 17.0, pal::TEXT);
            spacer(d, 6.0);
            txt(d, format!("{}: {}", c.trait_name, c.trait_desc), &f.bold, 16.0, pal::GOLD);
            spacer(d, 10.0);
            if owned {
                if c.id == profile.selected_car {
                    txt(d, "Эта машина выбрана для заездов", &f.bold, 18.0, pal::GOOD);
                } else {
                    button(d, f, "ВЫБРАТЬ", GarageAction::SelectCar(view), Val::Px(300.0), true, false);
                }
            } else {
                let lvl_ok = profile.level() >= c.unlock_level;
                let money_ok = profile.credits >= c.price;
                if !lvl_ok {
                    txt(d, format!("Откроется на уровне {}", c.unlock_level), &f.bold, 18.0, pal::BAD);
                }
                button(d, f, format!("КУПИТЬ ЗА {}", money(c.price)), GarageAction::BuyCar(view), Val::Px(360.0), lvl_ok && money_ok, !(lvl_ok && money_ok));
                if lvl_ok && !money_ok {
                    txt(d, "Не хватает кредитов", &f.body, 16.0, pal::BAD);
                }
            }
        });
    });
}

fn tuning_tab(p: &mut ChildSpawnerCommands, f: &Fonts, profile: &Profile) {
    let def = car_def(&profile.selected_car);
    let cp = profile.car_profile(def.id);
    txt(p, format!("Апгрейды для «{}». Цены зависят от класса машины.", def.name), &f.body, 16.0, pal::DIM);
    for up in Upgrade::ALL {
        let lvl = cp.levels[up.index()];
        p.spawn((
            Node { border_radius: BorderRadius::all(Val::Px(6.0)), flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(12.0), padding: UiRect::all(Val::Px(8.0)), ..default() },
            BackgroundColor(pal::PANEL_LIGHT),
            ))
        .with_children(|r| {
            r.spawn(Node { flex_direction: FlexDirection::Column, width: Val::Px(300.0), row_gap: Val::Px(3.0), ..default() }).with_children(|t| {
                txt(t, up.name(), &f.title, 19.0, pal::TEXT);
                txt(t, up.desc(), &f.body, 14.0, pal::DIM);
            });
            pips(r, lvl, MAX_UPGRADE);
            match upgrade_price(def, up, lvl) {
                Some(price) => {
                    let ok = profile.credits >= price;
                    button_sized(r, f, money(price), GarageAction::BuyUpgrade(up), Val::Px(150.0), Val::Px(40.0), 17.0, false, !ok);
                }
                None => {
                    txt(r, "МАКС", &f.title, 18.0, pal::GOOD);
                }
            }
        });
    }
}

fn setup_tab(p: &mut ChildSpawnerCommands, f: &Fonts, profile: &Profile) {
    let def = car_def(&profile.selected_car);
    let cp = profile.car_profile(def.id);
    txt(p, "Бесплатные настройки. Меняются в любой момент и сохраняются для каждой машины.", &f.body, 16.0, pal::DIM);
    for (i, sp) in SETUP_PARAMS.iter().enumerate() {
        let v = cp.setup.get(i);
        let frac = (v - sp.min) / (sp.max - sp.min);
        p.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(8.0), ..default() }).with_children(|r| {
            r.spawn(Node { flex_direction: FlexDirection::Column, width: Val::Px(340.0), ..default() }).with_children(|t| {
                txt(t, sp.name, &f.bold, 17.0, pal::TEXT);
                txt(t, sp.desc, &f.body, 13.0, pal::DIM);
            });
            button_sized(r, f, "–", GarageAction::Setup(i, -1), Val::Px(36.0), Val::Px(34.0), 18.0, false, v <= sp.min + 1e-4);
            bar(r, frac, 150.0, 10.0, pal::ACCENT);
            button_sized(r, f, "+", GarageAction::Setup(i, 1), Val::Px(36.0), Val::Px(34.0), 18.0, false, v >= sp.max - 1e-4);
            txt_node(r, format!("{:.2}", v), &f.narrow, 18.0, pal::TEXT, Node { width: Val::Px(50.0), ..default() });
        });
    }
    button(p, f, "СБРОСИТЬ НАСТРОЙКИ", GarageAction::SetupReset, Val::Px(280.0), false, false);
}

fn weapons_tab(p: &mut ChildSpawnerCommands, f: &Fonts, profile: &Profile, gs: &GarageState) {
    let def = car_def(&profile.selected_car);
    let cp = profile.car_profile(def.id);
    let cur_mount = gs.mount.filter(|m| def.slots.contains(m)).or(def.slots.first().copied());
    txt(p, "Точки крепления. Выбери точку, затем оружие для неё.", &f.body, 16.0, pal::DIM);
    row(p, 6.0, |r| {
        for m in def.slots {
            let w = cp.loadout.iter().find(|(mm, _)| mm == m).map(|(_, w)| weapon_def(*w).name).unwrap_or("пусто");
            button_sized(r, f, format!("{}: {}", m.name(), w), GarageAction::PickMount(*m), Val::Px(162.0), Val::Px(52.0), 14.0, Some(*m) == cur_mount, false);
        }
    });
    let Some(mount) = cur_mount else { return };
    let equipped = cp.loadout.iter().find(|(mm, _)| *mm == mount).map(|(_, w)| *w);
    p.spawn((
        Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(6.0), height: Val::Px(470.0), overflow: Overflow::scroll_y(), ..default() },
        ScrollPosition::default(),
        Scrollable,
        Interaction::default(),
    ))
    .with_children(|l| {
        if equipped.is_some() {
            button(l, f, "СНЯТЬ ОРУЖИЕ С ЭТОЙ ТОЧКИ", GarageAction::Equip(mount, None), Val::Px(320.0), false, false);
        }
        for w in WEAPONS.iter().filter(|w| w.mounts.contains(&mount)) {
            let owned = profile.owned_weapons.contains(&w.kind);
            let lvl = profile.weapon_levels.get(&w.kind).copied().unwrap_or(0);
            let locked = profile.level() < w.unlock_level;
            l.spawn((
                Node { border_radius: BorderRadius::all(Val::Px(6.0)), flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(10.0), padding: UiRect::all(Val::Px(8.0)), ..default() },
                BackgroundColor(if equipped == Some(w.kind) { Color::srgba(1.0, 0.55, 0.1, 0.18) } else { pal::PANEL_LIGHT }),
                ))
            .with_children(|r| {
                r.spawn(Node { flex_direction: FlexDirection::Column, width: Val::Px(330.0), row_gap: Val::Px(2.0), ..default() }).with_children(|t| {
                    txt(t, w.name, &f.title, 18.0, Color::srgb(w.color[0], w.color[1], w.color[2]));
                    txt(t, w.desc, &f.body, 13.0, pal::DIM);
                    txt(t, format!("Кнопка: {}", w.trigger.key_hint()), &f.body, 13.0, pal::TEXT);
                });
                if locked {
                    txt(r, format!("с {} ур.", w.unlock_level), &f.bold, 16.0, pal::BAD);
                } else if !owned {
                    button_sized(r, f, money(w.price), GarageAction::BuyWeapon(w.kind), Val::Px(140.0), Val::Px(38.0), 16.0, false, profile.credits < w.price);
                } else {
                    r.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0), ..default() }).with_children(|cc| {
                        pips(cc, lvl, MAX_WEAPON_LEVEL);
                        match weapon_upgrade_price(w.kind, lvl) {
                            Some(price) => {
                                button_sized(cc, f, format!("+ {}", money(price)), GarageAction::UpgradeWeapon(w.kind), Val::Px(130.0), Val::Px(30.0), 13.0, false, profile.credits < price);
                            }
                            None => {
                                txt(cc, "МАКС", &f.bold, 14.0, pal::GOOD);
                            }
                        }
                    });
                    if equipped != Some(w.kind) {
                        button_sized(r, f, "СТАВИТЬ", GarageAction::Equip(mount, Some(w.kind)), Val::Px(110.0), Val::Px(38.0), 15.0, false, false);
                    } else {
                        txt(r, "СТОИТ", &f.title, 15.0, pal::ACCENT);
                    }
                }
            });
        }
    });
}

fn perks_tab(p: &mut ChildSpawnerCommands, f: &Fonts, profile: &Profile) {
    txt(p, format!("Перки открываются с уровнем, ставятся бесплатно. Активно {}/{}.", profile.perks.len(), PERK_SLOTS), &f.body, 16.0, pal::DIM);
    p.spawn((
        Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(5.0), height: Val::Px(530.0), overflow: Overflow::scroll_y(), ..default() },
        ScrollPosition::default(),
        Scrollable,
        Interaction::default(),
    ))
    .with_children(|l| {
        for pk in PERKS {
            let on = profile.perks.contains(&pk.kind);
            let locked = profile.level() < pk.unlock_level;
            l.spawn((
                Node { border_radius: BorderRadius::all(Val::Px(6.0)), flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(10.0), padding: UiRect::all(Val::Px(7.0)), ..default() },
                BackgroundColor(if on { Color::srgba(1.0, 0.55, 0.1, 0.18) } else { pal::PANEL_LIGHT }),
                ))
            .with_children(|r| {
                r.spawn(Node { flex_direction: FlexDirection::Column, width: Val::Px(470.0), ..default() }).with_children(|t| {
                    txt(t, pk.name, &f.title, 17.0, if locked { pal::DIM } else { pal::TEXT });
                    txt(t, pk.desc, &f.body, 14.0, pal::DIM);
                });
                if locked {
                    txt(r, format!("с {} ур.", pk.unlock_level), &f.bold, 15.0, pal::BAD);
                } else {
                    let full = !on && profile.perks.len() >= PERK_SLOTS;
                    button_sized(r, f, if on { "СНЯТЬ" } else { "ВЗЯТЬ" }, GarageAction::TogglePerk(pk.kind), Val::Px(120.0), Val::Px(36.0), 15.0, on, full);
                }
            });
        }
    });
}

fn paint_tab(p: &mut ChildSpawnerCommands, f: &Fonts, profile: &Profile) {
    let def = car_def(&profile.selected_car);
    let cur = profile.car_profile(def.id).color.unwrap_or(def.color);
    txt(p, "Цвет кузова. Бесплатно.", &f.body, 16.0, pal::DIM);
    p.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(10.0), row_gap: Val::Px(10.0), ..default() }).with_children(|r| {
        let mut all: Vec<[f32; 3]> = vec![def.color];
        all.extend(PAINTS.iter().copied());
        for c in all {
            let sel = (c[0] - cur[0]).abs() + (c[1] - cur[1]).abs() + (c[2] - cur[2]).abs() < 0.01;
            r.spawn((
                Button,
                Node { border_radius: BorderRadius::all(Val::Px(8.0)),
                    width: Val::Px(70.0),
                    height: Val::Px(70.0),
                    border: UiRect::all(Val::Px(if sel { 4.0 } else { 2.0 })),
                    ..default()
                },
                BackgroundColor(Color::srgb(c[0], c[1], c[2])),
                BorderColor::all(if sel { Color::WHITE } else { Color::srgb(0.2, 0.2, 0.2) }),
                GarageAction::Paint(c),
            ));
        }
    });
    txt(p, "Первый образец: заводской цвет.", &f.body, 14.0, pal::DIM);
    // скины: палитра (цвет, металл, шероховатость) и наклейки на двери и капот
    let cur_skin = profile.car_profile(def.id).skin;
    txt(p, "Скины. Наклейки — у «Ржавого черепа», «Пломбира» и «Крови».", &f.body, 16.0, pal::DIM);
    p.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(8.0), row_gap: Val::Px(8.0), ..default() }).with_children(|r| {
        for (i, k) in SKINS.iter().enumerate() {
            let n = i as u8 + 1;
            let sel = cur_skin == n;
            r.spawn((
                Button,
                Node {
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    width: Val::Px(118.0),
                    height: Val::Px(44.0),
                    border: UiRect::all(Val::Px(if sel { 3.0 } else { 1.0 })),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(k.color[0] * 0.8, k.color[1] * 0.8, k.color[2] * 0.8)),
                BorderColor::all(if sel { Color::WHITE } else { Color::srgb(0.25, 0.25, 0.25) }),
                GarageAction::Skin(n),
            ))
            .with_children(|b| {
                txt(b, k.name, &f.bold, 13.0, if k.color[0] + k.color[1] + k.color[2] > 1.6 { Color::srgb(0.05, 0.05, 0.05) } else { pal::TEXT });
            });
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn garage_actions(
    q: Query<(&Interaction, &GarageAction, Option<&BtnStyle>), Changed<Interaction>>,
    mut gs: ResMut<GarageState>,
    mut profile: ResMut<Profile>,
    mut dirty: ResMut<MenuDirty>,
    mut screen: ResMut<MenuScreen>,
    mode: Res<NetMode>,
    mut click: MessageWriter<UiClick>,
    mut net: MessageWriter<crate::net::NetCommand>,
) {
    for (i, action, style) in q.iter() {
        if *i != Interaction::Pressed || style.map(|s| s.disabled).unwrap_or(false) {
            continue;
        }
        click.write(UiClick);
        let sel = profile.selected_car.clone();
        match action.clone() {
            GarageAction::Tab(t) => {
                gs.tab = t;
                if t != GarageTab::Cars {
                    gs.view = None;
                }
            }
            GarageAction::View(i) => gs.view = Some(i),
            GarageAction::BuyCar(i) => {
                let c = &CARS[i];
                if profile.credits >= c.price && profile.level() >= c.unlock_level && !profile.owned_cars.iter().any(|o| o == c.id) {
                    profile.credits -= c.price;
                    profile.owned_cars.push(c.id.to_string());
                    profile.selected_car = c.id.to_string();
                }
            }
            GarageAction::SelectCar(i) => {
                let id = CARS[i].id;
                if profile.owned_cars.iter().any(|o| o == id) {
                    profile.selected_car = id.to_string();
                }
            }
            GarageAction::BuyUpgrade(up) => {
                let def = car_def(&sel);
                let lvl = profile.car_profile(&sel).levels[up.index()];
                if let Some(price) = upgrade_price(def, up, lvl) {
                    if profile.credits >= price {
                        profile.credits -= price;
                        profile.car_profile_mut(&sel).levels[up.index()] = lvl + 1;
                    }
                }
            }
            GarageAction::Setup(i, d) => {
                let cp = profile.car_profile_mut(&sel);
                let v = cp.setup.get(i) + SETUP_PARAMS[i].step * d as f32;
                cp.setup.set(i, (v * 1000.0).round() / 1000.0);
            }
            GarageAction::SetupReset => profile.car_profile_mut(&sel).setup = CarSetup::default(),
            GarageAction::PickMount(m) => gs.mount = Some(m),
            GarageAction::Equip(m, w) => {
                let cp = profile.car_profile_mut(&sel);
                cp.loadout.retain(|(mm, _)| *mm != m);
                if let Some(w) = w {
                    cp.loadout.push((m, w));
                }
            }
            GarageAction::BuyWeapon(w) => {
                let price = weapon_def(w).price;
                if profile.credits >= price && !profile.owned_weapons.contains(&w) {
                    profile.credits -= price;
                    profile.owned_weapons.push(w);
                }
            }
            GarageAction::UpgradeWeapon(w) => {
                let lvl = profile.weapon_levels.get(&w).copied().unwrap_or(0);
                if let Some(price) = weapon_upgrade_price(w, lvl) {
                    if profile.credits >= price {
                        profile.credits -= price;
                        profile.weapon_levels.insert(w, lvl + 1);
                    }
                }
            }
            GarageAction::TogglePerk(p) => {
                if let Some(i) = profile.perks.iter().position(|x| *x == p) {
                    profile.perks.remove(i);
                } else if profile.perks.len() < PERK_SLOTS {
                    profile.perks.push(p);
                }
            }
            GarageAction::Paint(c) => {
                profile.car_profile_mut(&sel).color = Some(c);
                profile.car_profile_mut(&sel).skin = 0;
            }
            GarageAction::Skin(k) => profile.car_profile_mut(&sel).skin = k,
            GarageAction::Back => {
                *screen = if *mode == NetMode::Offline { MenuScreen::Main } else { MenuScreen::Lobby };
                gs.view = None;
            }
        }
        profile.save();
        if *mode != NetMode::Offline {
            net.write(crate::net::NetCommand::ConfigChanged);
        }
        dirty.0 = true;
    }
}
