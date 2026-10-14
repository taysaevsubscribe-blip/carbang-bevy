//! Интерфейс: шрифты, виджеты, меню, гараж, HUD, результаты.

pub mod garage;
pub mod hud;
pub mod menu;
pub mod popups;
pub mod results;
pub mod showroom;

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

/// Встроенные шрифты: игра не зависит от папки assets.
#[derive(Resource, Clone)]
pub struct Fonts {
    pub title: Handle<Font>,
    pub body: Handle<Font>,
    pub bold: Handle<Font>,
    pub narrow: Handle<Font>,
}

pub mod pal {
    use bevy::prelude::Color;
    pub const BG: Color = Color::srgba(0.06, 0.06, 0.08, 0.92);
    pub const PANEL: Color = Color::srgba(0.1, 0.1, 0.12, 0.88);
    pub const PANEL_LIGHT: Color = Color::srgba(0.16, 0.16, 0.19, 0.92);
    pub const ACCENT: Color = Color::srgb(1.0, 0.55, 0.1);
    pub const ACCENT_DARK: Color = Color::srgb(0.55, 0.27, 0.04);
    pub const TEXT: Color = Color::srgb(0.93, 0.93, 0.9);
    pub const DIM: Color = Color::srgb(0.6, 0.6, 0.62);
    pub const GOOD: Color = Color::srgb(0.35, 0.9, 0.4);
    pub const BAD: Color = Color::srgb(1.0, 0.3, 0.25);
    pub const GOLD: Color = Color::srgb(1.0, 0.85, 0.2);
    pub const BTN: Color = Color::srgb(0.18, 0.18, 0.21);
    pub const BTN_HOVER: Color = Color::srgb(0.3, 0.22, 0.12);
    pub const BTN_PRESS: Color = Color::srgb(0.6, 0.32, 0.06);
    pub const BTN_ACTIVE: Color = Color::srgb(0.72, 0.38, 0.06);
    pub const BTN_DISABLED: Color = Color::srgb(0.12, 0.12, 0.13);
}

/// Стиль кнопки: обычный цвет может быть «выбранным».
#[derive(Component, Clone, Copy)]
pub struct BtnStyle {
    pub active: bool,
    pub disabled: bool,
}

/// Текстовое поле, которое сейчас редактируется.
#[derive(Resource, Default)]
pub struct TextEdit {
    pub field: Option<TextField>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TextField {
    Name,
    Ip,
}

/// Список с прокруткой колесом мыши.
#[derive(Component)]
pub struct Scrollable;

/// Звук нажатия кнопки (обрабатывает аудио).
#[derive(Message)]
pub struct UiClick;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TextEdit>()
            .add_message::<UiClick>()
            .add_systems(PreStartup, load_fonts)
            .add_systems(Update, (button_colors, scroll_lists, text_input))
            .add_plugins((menu::MenuPlugin, garage::GaragePlugin, hud::HudPlugin, results::ResultsPlugin, showroom::ShowroomPlugin, popups::PopupPlugin))
            .add_systems(PostUpdate, cartoon_text_shadow);
    }
}

fn load_fonts(mut commands: Commands, mut fonts: ResMut<Assets<Font>>) {
    let load = |bytes: &'static [u8]| Font::from_bytes(bytes.to_vec());
    let title = fonts.add(load(include_bytes!("../../assets/fonts/RussoOne.ttf")));
    let body = fonts.add(load(include_bytes!("../../assets/fonts/PTSans-Regular.ttf")));
    let bold = fonts.add(load(include_bytes!("../../assets/fonts/PTSans-Bold.ttf")));
    let narrow = fonts.add(load(include_bytes!("../../assets/fonts/PTSansNarrow-Bold.ttf")));
    commands.insert_resource(Fonts { title, body, bold, narrow });
}

fn button_colors(mut q: Query<(&Interaction, &BtnStyle, &mut BackgroundColor, &mut BorderColor), Or<(Changed<Interaction>, Changed<BtnStyle>)>>) {
    for (i, s, mut bg, mut border) in q.iter_mut() {
        let (c, b) = if s.disabled {
            (pal::BTN_DISABLED, Color::srgb(0.2, 0.2, 0.2))
        } else {
            match (*i, s.active) {
                (Interaction::Pressed, _) => (pal::BTN_PRESS, pal::ACCENT),
                (Interaction::Hovered, true) => (pal::BTN_ACTIVE, Color::WHITE),
                (Interaction::Hovered, false) => (pal::BTN_HOVER, pal::ACCENT),
                (Interaction::None, true) => (pal::BTN_ACTIVE, pal::ACCENT),
                (Interaction::None, false) => (pal::BTN, Color::srgb(0.28, 0.28, 0.3)),
            }
        };
        bg.0 = c;
        *border = BorderColor::all(b);
    }
}

fn scroll_lists(mut wheel: MessageReader<MouseWheel>, mut q: Query<(&mut ScrollPosition, &Interaction, &ComputedNode), With<Scrollable>>) {
    let mut dy = 0.0;
    for e in wheel.read() {
        dy += match e.unit {
            MouseScrollUnit::Line => e.y * 40.0,
            MouseScrollUnit::Pixel => e.y,
        };
    }
    if dy == 0.0 {
        return;
    }
    for (mut sp, i, _node) in q.iter_mut() {
        if *i != Interaction::None {
            sp.0.y = (sp.0.y - dy).max(0.0);
        }
    }
}

/// Ввод текста в активное поле: имя игрока или адрес сервера.
fn text_input(
    mut ev: MessageReader<KeyboardInput>,
    mut edit: ResMut<TextEdit>,
    mut profile: ResMut<crate::config::Profile>,
    mut settings: ResMut<crate::config::Settings>,
    mut dirty: ResMut<menu::MenuDirty>,
) {
    let Some(field) = edit.field else {
        ev.clear();
        return;
    };
    for e in ev.read() {
        if !e.state.is_pressed() {
            continue;
        }
        let target: &mut String = match field {
            TextField::Name => &mut profile.name,
            TextField::Ip => &mut settings.last_ip,
        };
        match &e.logical_key {
            Key::Enter | Key::Escape | Key::Tab => {
                edit.field = None;
                profile.save();
                settings.save();
            }
            Key::Backspace => {
                target.pop();
            }
            Key::Character(c) => {
                let max = if field == TextField::Name { 16 } else { 40 };
                for ch in c.chars() {
                    let ok = match field {
                        TextField::Name => !ch.is_control(),
                        TextField::Ip => ch.is_ascii_alphanumeric() || ch == '.' || ch == ':' || ch == '-',
                    };
                    if ok && target.chars().count() < max {
                        target.push(ch);
                    }
                }
            }
            Key::Space if field == TextField::Name => {
                if target.chars().count() < 16 {
                    target.push(' ');
                }
            }
            _ => {}
        }
        dirty.0 = true;
    }
}

// ============================================================ виджеты

pub fn txt(p: &mut ChildSpawnerCommands, s: impl Into<String>, font: &Handle<Font>, size: f32, color: Color) -> Entity {
    p.spawn((Text::new(s.into()), TextFont { font: font.clone().into(), font_size: FontSize::Px(size), ..default() }, TextColor(color))).id()
}

pub fn txt_node(p: &mut ChildSpawnerCommands, s: impl Into<String>, font: &Handle<Font>, size: f32, color: Color, node: Node) -> Entity {
    p.spawn((Text::new(s.into()), TextFont { font: font.clone().into(), font_size: FontSize::Px(size), ..default() }, TextColor(color), node)).id()
}

/// Кнопка с действием A (любой компонент) и подписью.
pub fn button<A: Component>(p: &mut ChildSpawnerCommands, fonts: &Fonts, label: impl Into<String>, action: A, width: Val, active: bool, disabled: bool) -> Entity {
    button_sized(p, fonts, label, action, width, Val::Px(44.0), 20.0, active, disabled)
}

#[allow(clippy::too_many_arguments)]
pub fn button_sized<A: Component>(
    p: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    label: impl Into<String>,
    action: A,
    width: Val,
    height: Val,
    font_size: f32,
    active: bool,
    disabled: bool,
) -> Entity {
    let label = label.into();
    p.spawn((
        Button,
        Node { border_radius: BorderRadius::all(Val::Px(6.0)),
            width,
            height,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(2.0)),
            padding: UiRect::horizontal(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(if active { pal::BTN_ACTIVE } else { pal::BTN }),
        BorderColor::all(Color::srgb(0.28, 0.28, 0.3)),
        BtnStyle { active, disabled },
        action,
    ))
    .with_children(|b| {
        b.spawn((
            Text::new(label),
            TextFont { font: fonts.title.clone().into(), font_size: FontSize::Px(font_size), ..default() },
            TextColor(if disabled { pal::DIM } else { pal::TEXT }),
            TextLayout::justify(Justify::Center),
        ));
    })
    .id()
}

pub fn panel(p: &mut ChildSpawnerCommands, node: Node, color: Color, f: impl FnOnce(&mut ChildSpawnerCommands)) -> Entity {
    p.spawn((Node { border_radius: BorderRadius::all(Val::Px(8.0)), ..node }, BackgroundColor(color))).with_children(f).id()
}

pub fn row(p: &mut ChildSpawnerCommands, gap: f32, f: impl FnOnce(&mut ChildSpawnerCommands)) -> Entity {
    p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(gap), align_items: AlignItems::Center, ..default() }).with_children(f).id()
}

pub fn col(p: &mut ChildSpawnerCommands, gap: f32, f: impl FnOnce(&mut ChildSpawnerCommands)) -> Entity {
    p.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(gap), ..default() }).with_children(f).id()
}

/// Полоска значения 0..1 (для характеристик).
pub fn bar(p: &mut ChildSpawnerCommands, frac: f32, width: f32, height: f32, color: Color) -> Entity {
    p.spawn((
        Node { border_radius: BorderRadius::all(Val::Px(3.0)), width: Val::Px(width), height: Val::Px(height), ..default() },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.08)),
        ))
    .with_children(|b| {
        b.spawn((
            Node { border_radius: BorderRadius::all(Val::Px(3.0)), width: Val::Percent(frac.clamp(0.0, 1.0) * 100.0), height: Val::Percent(100.0), ..default() },
            BackgroundColor(color),
            ));
    })
    .id()
}

/// Сравнительная полоса: старое значение и прирост другим цветом.
pub fn bar_delta(p: &mut ChildSpawnerCommands, base: f32, new: f32, width: f32, height: f32) -> Entity {
    p.spawn((
        Node { border_radius: BorderRadius::all(Val::Px(3.0)), width: Val::Px(width), height: Val::Px(height), flex_direction: FlexDirection::Row, ..default() },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.08)),
        ))
    .with_children(|b| {
        let lo = base.min(new).clamp(0.0, 1.0);
        let hi = base.max(new).clamp(0.0, 1.0);
        b.spawn((Node { width: Val::Percent(lo * 100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(pal::ACCENT)));
        b.spawn((
            Node { width: Val::Percent((hi - lo) * 100.0), height: Val::Percent(100.0), ..default() },
            BackgroundColor(if new >= base { pal::GOOD } else { pal::BAD }),
        ));
    })
    .id()
}

/// Ряд квадратиков-уровней.
pub fn pips(p: &mut ChildSpawnerCommands, level: u8, max: u8) -> Entity {
    p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(4.0), ..default() })
        .with_children(|r| {
            for i in 0..max {
                r.spawn((
                    Node { border_radius: BorderRadius::all(Val::Px(3.0)), width: Val::Px(16.0), height: Val::Px(16.0), ..default() },
                    BackgroundColor(if i < level { pal::ACCENT } else { Color::srgba(1.0, 1.0, 1.0, 0.1) }),
                    ));
            }
        })
        .id()
}

pub fn spacer(p: &mut ChildSpawnerCommands, h: f32) {
    p.spawn(Node { height: Val::Px(h), ..default() });
}

pub fn money(v: u32) -> String {
    let s = v.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(' ');
        }
        out.push(c);
    }
    out + " кр"
}

pub fn time_str(t: f32) -> String {
    let t = t.max(0.0);
    format!("{}:{:02}", (t / 60.0) as u32, (t % 60.0) as u32)
}

/// Стилизация: всем надписям интерфейса тёмная тень со сдвигом, как у плашек всплывашек.
fn cartoon_text_shadow(mut commands: Commands, added: Query<Entity, (Added<Text>, Without<TextShadow>)>) {
    for e in added.iter() {
        commands.entity(e).insert(TextShadow { offset: Vec2::new(2.0, 2.0), color: Color::srgba(0.0, 0.0, 0.0, 0.7) });
    }
}
