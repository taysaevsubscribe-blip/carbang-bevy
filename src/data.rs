//! Игровые данные: машины, оружие, апгрейды, перки, бонусы, режимы, карты, карьера.
//! Всё задано в коде, чтобы баланс правился в одном месте и проверялся компилятором.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================ общие перечисления

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Drive {
    Fwd,
    Rwd,
    Awd,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Mount {
    Front,
    Roof,
    Rear,
    Sides,
}

impl Mount {
    pub const ALL: [Mount; 4] = [Mount::Front, Mount::Roof, Mount::Rear, Mount::Sides];
    pub fn name(self) -> &'static str {
        match self {
            Mount::Front => "Перед",
            Mount::Roof => "Крыша",
            Mount::Rear => "Зад",
            Mount::Sides => "Борта",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum BodyStyle {
    Hatch,
    Pickup,
    Coupe,
    ArmoredVan,
    Hearse,
    IceCream,
    Bus,
    Buggy,
    Monster,
    Mixer,
    Limo,
    GolfCart,
}

/// Особенность машины: пассивный эффект, который делает каждую машину непохожей.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum CarTrait {
    None,
    ExtraAmmo,
    NitroRegen,
    BlastResist,
    PedVampire,
    PedLure,
    Unflippable,
    AirControl,
    Crusher,
    Steady,
    Rich,
    LowProfile,
}

// ============================================================ машины

pub struct CarDef {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub trait_name: &'static str,
    pub trait_desc: &'static str,
    pub trait_kind: CarTrait,
    pub price: u32,
    pub unlock_level: u32,
    pub mass: f32,
    pub power_kw: f32,
    /// Ограничение тяги по моменту, в долях g
    pub torque_g: f32,
    pub grip: f32,
    pub armor: f32,
    pub ram: f32,
    /// Коэффициент аэродинамического сопротивления: F = drag * v^2
    pub drag: f32,
    pub drive: Drive,
    /// Ширина, высота кузова (без кабины), длина, м
    pub size: Vec3,
    pub wheel_radius: f32,
    /// Ход подвески, м
    pub travel: f32,
    pub style: BodyStyle,
    pub slots: &'static [Mount],
    pub color: [f32; 3],
    /// Множитель цены апгрейдов
    pub class_mult: f32,
}

pub const CARS: &[CarDef] = &[
    CarDef {
        id: "ferret",
        name: "Хорёк",
        desc: "Злой хэтчбек на полном приводе. Ровный во всём, прощает ошибки.",
        trait_name: "Без сюрпризов",
        trait_desc: "Никаких особенностей, зато нет и слабых мест.",
        trait_kind: CarTrait::None,
        price: 0,
        unlock_level: 1,
        mass: 1100.0,
        power_kw: 150.0,
        torque_g: 0.55,
        grip: 1.08,
        armor: 1.0,
        ram: 1.0,
        drag: 1.2,
        drive: Drive::Awd,
        size: Vec3::new(1.8, 0.75, 3.9),
        wheel_radius: 0.36,
        travel: 0.38,
        style: BodyStyle::Hatch,
        slots: &[Mount::Front, Mount::Roof, Mount::Rear],
        color: [0.1, 0.1, 0.11],
        class_mult: 1.0,
    },
    CarDef {
        id: "golf",
        name: "Дедуля",
        desc: "Заряженный гольф-кар. Медленный, лёгкий, никто его не боится.",
        trait_name: "Божий одуванчик",
        trait_desc: "Соперники почти не выбирают тебя целью, пока ты их не тронешь.",
        trait_kind: CarTrait::LowProfile,
        price: 0,
        unlock_level: 1,
        mass: 520.0,
        power_kw: 48.0,
        torque_g: 0.6,
        grip: 1.1,
        armor: 0.7,
        ram: 0.6,
        drag: 2.2,
        drive: Drive::Rwd,
        size: Vec3::new(1.95, 0.7, 3.7),
        wheel_radius: 0.27,
        travel: 0.25,
        style: BodyStyle::GolfCart,
        slots: &[Mount::Roof, Mount::Rear],
        color: [0.55, 0.3, 0.18],
        class_mult: 0.6,
    },
    CarDef {
        id: "hearse",
        name: "Катафалк",
        desc: "Длинный и тяжёлый универсал. Знает, куда везёт пассажиров.",
        trait_name: "Последний путь",
        trait_desc: "Каждый сбитый пешеход чинит машину на 3 единицы.",
        trait_kind: CarTrait::PedVampire,
        price: 4000,
        unlock_level: 2,
        mass: 1750.0,
        power_kw: 210.0,
        torque_g: 0.5,
        grip: 1.02,
        armor: 1.35,
        ram: 1.15,
        drag: 1.35,
        drive: Drive::Rwd,
        size: Vec3::new(2.05, 0.8, 4.9),
        wheel_radius: 0.38,
        travel: 0.36,
        style: BodyStyle::Hearse,
        slots: &[Mount::Front, Mount::Roof, Mount::Rear],
        color: [0.07, 0.07, 0.08],
        class_mult: 1.2,
    },
    CarDef {
        id: "bison",
        name: "Бизон",
        desc: "Рамный пикап. Кузов набит патронами, морда из чугуна.",
        trait_name: "Полный кузов",
        trait_desc: "Боезапас всего оружия больше на 35%.",
        trait_kind: CarTrait::ExtraAmmo,
        price: 6000,
        unlock_level: 2,
        mass: 1950.0,
        power_kw: 235.0,
        torque_g: 0.5,
        grip: 1.0,
        armor: 1.6,
        ram: 1.35,
        drag: 1.5,
        drive: Drive::Rwd,
        size: Vec3::new(2.0, 0.95, 4.8),
        wheel_radius: 0.42,
        travel: 0.42,
        style: BodyStyle::Pickup,
        slots: &[Mount::Front, Mount::Roof, Mount::Rear],
        color: [0.85, 0.42, 0.12],
        class_mult: 1.3,
    },
    CarDef {
        id: "icecream",
        name: "Пломбир",
        desc: "Фургон мороженщика. Играет весёлую мелодию, на которую сбегаются все.",
        trait_name: "Музыка из динамика",
        trait_desc: "Пешеходы в радиусе 25 м идут к тебе, а не убегают.",
        trait_kind: CarTrait::PedLure,
        price: 7500,
        unlock_level: 3,
        mass: 2150.0,
        power_kw: 200.0,
        torque_g: 0.48,
        grip: 0.98,
        armor: 1.45,
        ram: 1.2,
        drag: 2.1,
        drive: Drive::Rwd,
        size: Vec3::new(2.1, 1.7, 4.8),
        wheel_radius: 0.4,
        travel: 0.36,
        style: BodyStyle::IceCream,
        slots: &[Mount::Roof, Mount::Rear, Mount::Sides],
        color: [0.93, 0.62, 0.7],
        class_mult: 1.3,
    },
    CarDef {
        id: "buggy",
        name: "Блоха",
        desc: "Трубчатая багги. Прыгает выше крыш, ломается от чиха.",
        trait_name: "Акробат",
        trait_desc: "Двойное управление в воздухе, очки за полёт удваиваются.",
        trait_kind: CarTrait::AirControl,
        price: 8000,
        unlock_level: 3,
        mass: 680.0,
        power_kw: 150.0,
        torque_g: 0.75,
        grip: 1.15,
        armor: 0.6,
        ram: 0.7,
        drag: 1.4,
        drive: Drive::Awd,
        size: Vec3::new(1.75, 0.5, 2.75),
        wheel_radius: 0.44,
        travel: 0.55,
        style: BodyStyle::Buggy,
        slots: &[Mount::Roof, Mount::Rear],
        color: [0.85, 0.12, 0.1],
        class_mult: 1.1,
    },
    CarDef {
        id: "wasp",
        name: "Оса",
        desc: "Заднеприводное купе. Самое быстрое и самое капризное.",
        trait_name: "Форсаж",
        trait_desc: "Закись азота восстанавливается в 2 раза быстрее.",
        trait_kind: CarTrait::NitroRegen,
        price: 12000,
        unlock_level: 4,
        mass: 1250.0,
        power_kw: 320.0,
        torque_g: 0.62,
        grip: 1.22,
        armor: 0.85,
        ram: 0.9,
        drag: 1.05,
        drive: Drive::Rwd,
        size: Vec3::new(1.95, 0.62, 4.4),
        wheel_radius: 0.36,
        travel: 0.3,
        style: BodyStyle::Coupe,
        slots: &[Mount::Front, Mount::Roof],
        color: [0.95, 0.75, 0.1],
        class_mult: 1.5,
    },
    CarDef {
        id: "limo",
        name: "Мэр",
        desc: "Бронированный лимузин. Деньги решают всё, даже на арене.",
        trait_name: "Откат",
        trait_desc: "Все заработанные кредиты +30%.",
        trait_kind: CarTrait::Rich,
        price: 15000,
        unlock_level: 5,
        mass: 2450.0,
        power_kw: 300.0,
        torque_g: 0.5,
        grip: 1.05,
        armor: 1.8,
        ram: 1.25,
        drag: 1.3,
        drive: Drive::Rwd,
        size: Vec3::new(2.05, 0.8, 6.4),
        wheel_radius: 0.38,
        travel: 0.32,
        style: BodyStyle::Limo,
        slots: &[Mount::Front, Mount::Roof, Mount::Rear, Mount::Sides],
        color: [0.16, 0.08, 0.15],
        class_mult: 1.6,
    },
    CarDef {
        id: "roller",
        name: "Каток",
        desc: "Инкассаторский броневик. Медленный, но почти неубиваемый.",
        trait_name: "Броневые листы",
        trait_desc: "Урон от взрывов меньше на 40%.",
        trait_kind: CarTrait::BlastResist,
        price: 20000,
        unlock_level: 6,
        mass: 3000.0,
        power_kw: 360.0,
        torque_g: 0.38,
        grip: 0.98,
        armor: 2.5,
        ram: 1.9,
        drag: 1.9,
        drive: Drive::Awd,
        size: Vec3::new(2.3, 1.25, 4.0),
        wheel_radius: 0.45,
        travel: 0.36,
        style: BodyStyle::ArmoredVan,
        slots: &[Mount::Front, Mount::Roof, Mount::Rear, Mount::Sides],
        color: [0.92, 0.7, 0.1],
        class_mult: 2.0,
    },
    CarDef {
        id: "monster",
        name: "Мамонт",
        desc: "Монстр-трак на колёсах в человеческий рост. Ездит по крышам машин.",
        trait_name: "Давилка",
        trait_desc: "Приземление на соперника наносит тройной урон.",
        trait_kind: CarTrait::Crusher,
        price: 24000,
        unlock_level: 7,
        mass: 3200.0,
        power_kw: 420.0,
        torque_g: 0.55,
        grip: 1.05,
        armor: 2.0,
        ram: 1.6,
        drag: 2.0,
        drive: Drive::Awd,
        size: Vec3::new(2.35, 1.0, 5.0),
        wheel_radius: 0.85,
        travel: 0.7,
        style: BodyStyle::Monster,
        slots: &[Mount::Front, Mount::Roof, Mount::Rear],
        color: [0.7, 0.35, 0.12],
        class_mult: 2.0,
    },
    CarDef {
        id: "mixer",
        name: "Миксер",
        desc: "Бетономешалка. Разгоняется до обеда, останавливается к ужину.",
        trait_name: "Тонна бетона",
        trait_desc: "Удары и взрывы почти не отбрасывают машину.",
        trait_kind: CarTrait::Steady,
        price: 28000,
        unlock_level: 8,
        mass: 7200.0,
        power_kw: 400.0,
        torque_g: 0.32,
        grip: 0.97,
        armor: 3.0,
        ram: 2.3,
        drag: 7.0,
        drive: Drive::Rwd,
        size: Vec3::new(2.5, 1.3, 7.6),
        wheel_radius: 0.55,
        travel: 0.32,
        style: BodyStyle::Mixer,
        slots: &[Mount::Front, Mount::Roof, Mount::Rear],
        color: [0.90, 0.45, 0.08],
        class_mult: 2.3,
    },
    CarDef {
        id: "bus",
        name: "Камчатка",
        desc: "Школьный автобус. Огромный таран, много места под оружие.",
        trait_name: "Неваляшка",
        trait_desc: "Автобус почти невозможно перевернуть, он сам встаёт на колёса.",
        trait_kind: CarTrait::Unflippable,
        price: 32000,
        unlock_level: 9,
        mass: 6500.0,
        power_kw: 430.0,
        torque_g: 0.35,
        grip: 0.96,
        armor: 3.2,
        ram: 2.6,
        drag: 6.0,
        drive: Drive::Rwd,
        size: Vec3::new(2.05, 1.1, 3.9),
        wheel_radius: 0.55,
        travel: 0.34,
        style: BodyStyle::Bus,
        slots: &[Mount::Front, Mount::Roof, Mount::Rear, Mount::Sides],
        color: [0.48, 0.5, 0.28],
        class_mult: 2.5,
    },
];

pub fn car_index(id: &str) -> usize {
    CARS.iter().position(|c| c.id == id).unwrap_or(0)
}

pub fn car_def(id: &str) -> &'static CarDef {
    &CARS[car_index(id)]
}

// ============================================================ апгрейды

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Upgrade {
    Engine,
    Armor,
    Tires,
    Suspension,
    Ram,
    Nitro,
}

impl Upgrade {
    pub const ALL: [Upgrade; 6] = [
        Upgrade::Engine,
        Upgrade::Armor,
        Upgrade::Tires,
        Upgrade::Suspension,
        Upgrade::Ram,
        Upgrade::Nitro,
    ];
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn name(self) -> &'static str {
        match self {
            Upgrade::Engine => "Двигатель",
            Upgrade::Armor => "Броня",
            Upgrade::Tires => "Шины",
            Upgrade::Suspension => "Подвеска",
            Upgrade::Ram => "Силовой бампер",
            Upgrade::Nitro => "Закись азота",
        }
    }
    pub fn desc(self) -> &'static str {
        match self {
            Upgrade::Engine => "+12% мощности и +8% тяги за уровень",
            Upgrade::Armor => "+15% к защите, +3% к массе",
            Upgrade::Tires => "+5% сцепления",
            Upgrade::Suspension => "Жёстче пружины, меньше крен, +2% сцепления",
            Upgrade::Ram => "+22% урона при таране",
            Upgrade::Nitro => "+1.5 секунды закиси азота",
        }
    }
    pub fn base_price(self) -> f32 {
        match self {
            Upgrade::Engine => 800.0,
            Upgrade::Armor => 700.0,
            Upgrade::Tires => 600.0,
            Upgrade::Suspension => 500.0,
            Upgrade::Ram => 650.0,
            Upgrade::Nitro => 600.0,
        }
    }
}

pub const MAX_UPGRADE: u8 = 5;

pub fn upgrade_price(car: &CarDef, up: Upgrade, current_level: u8) -> Option<u32> {
    if current_level >= MAX_UPGRADE {
        return None;
    }
    let p = up.base_price() * (current_level as f32 + 1.0).powf(1.4) * car.class_mult;
    Some(((p / 50.0).round() * 50.0) as u32)
}

/// Свободные настройки (без денег): подвеска, баланс, передачи, ассисты.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct CarSetup {
    pub spring: f32,
    pub ride: f32,
    pub antiroll: f32,
    pub grip_bias: f32,
    pub gearing: f32,
    pub brake_bias: f32,
    pub steer_speed: f32,
    pub stability: f32,
    pub countersteer: f32,
}

impl Default for CarSetup {
    fn default() -> Self {
        Self {
            spring: 1.0,
            ride: 1.0,
            antiroll: 1.0,
            grip_bias: 0.0,
            gearing: 1.0,
            brake_bias: 0.6,
            steer_speed: 1.0,
            stability: 0.8,
            countersteer: 0.6,
        }
    }
}

pub struct SetupParam {
    pub name: &'static str,
    pub desc: &'static str,
    pub min: f32,
    pub max: f32,
    pub step: f32,
}

pub const SETUP_PARAMS: [SetupParam; 9] = [
    SetupParam { name: "Жёсткость пружин", desc: "Мягко: держит кочки. Жёстко: меньше крен и раскачка.", min: 0.6, max: 1.6, step: 0.1 },
    SetupParam { name: "Клиренс", desc: "Высоко: проходит трамплины и кучи. Низко: устойчивее.", min: 0.8, max: 1.3, step: 0.05 },
    SetupParam { name: "Стабилизаторы", desc: "Сила стабилизаторов поперечной устойчивости.", min: 0.3, max: 2.0, step: 0.1 },
    SetupParam { name: "Баланс сцепления", desc: "Минус: зад скользит, машина дрифтует. Плюс: недостаточная поворачиваемость.", min: -0.15, max: 0.15, step: 0.03 },
    SetupParam { name: "Передачи", desc: "Короткие: быстрый разгон. Длинные: выше максимальная скорость.", min: 0.8, max: 1.25, step: 0.05 },
    SetupParam { name: "Тормозной баланс", desc: "Доля тормозного усилия на передней оси.", min: 0.4, max: 0.75, step: 0.05 },
    SetupParam { name: "Скорость руля", desc: "Как быстро колёса поворачиваются за рулём.", min: 0.6, max: 1.5, step: 0.1 },
    SetupParam { name: "Стабилизация", desc: "Электроника гасит занос. 0: только руки.", min: 0.0, max: 1.0, step: 0.1 },
    SetupParam { name: "Контрруление", desc: "Помощь в заносе: колёса сами ловят машину.", min: 0.0, max: 1.0, step: 0.1 },
];

impl CarSetup {
    pub fn get(&self, i: usize) -> f32 {
        match i {
            0 => self.spring,
            1 => self.ride,
            2 => self.antiroll,
            3 => self.grip_bias,
            4 => self.gearing,
            5 => self.brake_bias,
            6 => self.steer_speed,
            7 => self.stability,
            _ => self.countersteer,
        }
    }
    pub fn set(&mut self, i: usize, v: f32) {
        let p = &SETUP_PARAMS[i];
        let v = v.clamp(p.min, p.max);
        match i {
            0 => self.spring = v,
            1 => self.ride = v,
            2 => self.antiroll = v,
            3 => self.grip_bias = v,
            4 => self.gearing = v,
            5 => self.brake_bias = v,
            6 => self.steer_speed = v,
            7 => self.stability = v,
            _ => self.countersteer = v,
        }
    }
}

// ============================================================ оружие

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub enum WeaponKind {
    MachineGun,
    Rockets,
    Homing,
    Mines,
    Bomb,
    Spikes,
    Plow,
    Emp,
    Oil,
    Flamer,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Trigger {
    Fire,
    Special,
    Drop,
    Passive,
}

impl Trigger {
    pub fn key_hint(self) -> &'static str {
        match self {
            Trigger::Fire => "ЛКМ / Ctrl",
            Trigger::Special => "ПКМ / E",
            Trigger::Drop => "Q",
            Trigger::Passive => "пассивно",
        }
    }
}

pub struct WeaponDef {
    pub kind: WeaponKind,
    pub name: &'static str,
    pub desc: &'static str,
    pub mounts: &'static [Mount],
    pub trigger: Trigger,
    pub ammo: u32,
    pub cooldown: f32,
    pub damage: f32,
    pub radius: f32,
    pub range: f32,
    pub speed: f32,
    pub price: u32,
    pub unlock_level: u32,
    pub color: [f32; 3],
}

pub const WEAPONS: &[WeaponDef] = &[
    WeaponDef {
        kind: WeaponKind::MachineGun,
        name: "Швейная машинка",
        desc: "Пулемёт. На крыше сам вращается и наводится на ближайшего врага.",
        mounts: &[Mount::Roof, Mount::Front, Mount::Sides],
        trigger: Trigger::Fire,
        ammo: 240,
        cooldown: 0.08,
        damage: 2.4,
        radius: 0.0,
        range: 80.0,
        speed: 0.0,
        price: 1200,
        unlock_level: 1,
        color: [1.0, 0.85, 0.4],
    },
    WeaponDef {
        kind: WeaponKind::Rockets,
        name: "Гвоздомёт",
        desc: "Неуправляемые ракеты. Прямое попадание плюс взрыв.",
        mounts: &[Mount::Front, Mount::Roof, Mount::Sides],
        trigger: Trigger::Fire,
        ammo: 16,
        cooldown: 0.55,
        damage: 20.0,
        radius: 4.5,
        range: 120.0,
        speed: 75.0,
        price: 2200,
        unlock_level: 2,
        color: [1.0, 0.5, 0.15],
    },
    WeaponDef {
        kind: WeaponKind::Homing,
        name: "Осиное гнездо",
        desc: "Самонаводящиеся ракеты. Держи цель в прицеле, пока не загорится захват.",
        mounts: &[Mount::Roof],
        trigger: Trigger::Special,
        ammo: 6,
        cooldown: 1.4,
        damage: 24.0,
        radius: 5.0,
        range: 110.0,
        speed: 48.0,
        price: 3800,
        unlock_level: 4,
        color: [1.0, 0.3, 0.2],
    },
    WeaponDef {
        kind: WeaponKind::Mines,
        name: "Сюрприз",
        desc: "Мины сзади. Взводятся за секунду и подбрасывают жертву.",
        mounts: &[Mount::Rear],
        trigger: Trigger::Drop,
        ammo: 6,
        cooldown: 1.0,
        damage: 36.0,
        radius: 4.0,
        range: 0.0,
        speed: 0.0,
        price: 1600,
        unlock_level: 1,
        color: [0.9, 0.15, 0.1],
    },
    WeaponDef {
        kind: WeaponKind::Bomb,
        name: "Бабушкин чайник",
        desc: "Бомба навесом вперёд. Взрывается при ударе или через 2 секунды.",
        mounts: &[Mount::Roof],
        trigger: Trigger::Special,
        ammo: 8,
        cooldown: 1.6,
        damage: 32.0,
        radius: 6.5,
        range: 30.0,
        speed: 22.0,
        price: 2600,
        unlock_level: 3,
        color: [0.3, 0.3, 0.3],
    },
    WeaponDef {
        kind: WeaponKind::Spikes,
        name: "Ёж",
        desc: "Шипы. Удары этой стороной наносят +60% урона, а бьющий тебя в шипы ранится сам.",
        mounts: &[Mount::Front, Mount::Sides],
        trigger: Trigger::Passive,
        ammo: 0,
        cooldown: 0.0,
        damage: 0.6,
        radius: 0.0,
        range: 0.0,
        speed: 0.0,
        price: 1400,
        unlock_level: 1,
        color: [0.7, 0.7, 0.72],
    },
    WeaponDef {
        kind: WeaponKind::Plow,
        name: "Плуг",
        desc: "Отвал на морду. Перед машины крепче на 60%, таран сильнее на 30%.",
        mounts: &[Mount::Front],
        trigger: Trigger::Passive,
        ammo: 0,
        cooldown: 0.0,
        damage: 0.3,
        radius: 0.0,
        range: 0.0,
        speed: 0.0,
        price: 1800,
        unlock_level: 2,
        color: [0.85, 0.65, 0.1],
    },
    WeaponDef {
        kind: WeaponKind::Emp,
        name: "Глушилка",
        desc: "Электромагнитный импульс: глушит моторы и оружие всех вокруг на 2.5 секунды.",
        mounts: &[Mount::Roof],
        trigger: Trigger::Special,
        ammo: 4,
        cooldown: 9.0,
        damage: 6.0,
        radius: 16.0,
        range: 0.0,
        speed: 0.0,
        price: 3200,
        unlock_level: 5,
        color: [0.4, 0.7, 1.0],
    },
    WeaponDef {
        kind: WeaponKind::Oil,
        name: "Блин",
        desc: "Масляное пятно сзади. Кто наедет, тот потеряет сцепление.",
        mounts: &[Mount::Rear],
        trigger: Trigger::Drop,
        ammo: 5,
        cooldown: 1.5,
        damage: 0.0,
        radius: 3.5,
        range: 0.0,
        speed: 0.0,
        price: 1000,
        unlock_level: 1,
        color: [0.1, 0.1, 0.12],
    },
    WeaponDef {
        kind: WeaponKind::Flamer,
        name: "Шашлык",
        desc: "Огнемёт. Поджигает всё в 10 метрах перед мордой.",
        mounts: &[Mount::Front],
        trigger: Trigger::Fire,
        ammo: 100,
        cooldown: 0.1,
        damage: 22.0,
        radius: 0.0,
        range: 10.0,
        speed: 0.0,
        price: 2800,
        unlock_level: 4,
        color: [1.0, 0.45, 0.05],
    },
];

pub fn weapon_def(kind: WeaponKind) -> &'static WeaponDef {
    WEAPONS.iter().find(|w| w.kind == kind).expect("weapon def")
}

pub const MAX_WEAPON_LEVEL: u8 = 3;

pub fn weapon_upgrade_price(kind: WeaponKind, level: u8) -> Option<u32> {
    if level >= MAX_WEAPON_LEVEL {
        return None;
    }
    let p = weapon_def(kind).price as f32 * 0.55 * (level as f32 + 1.0);
    Some(((p / 50.0).round() * 50.0) as u32)
}

pub fn weapon_damage_mult(level: u8) -> f32 {
    1.0 + 0.15 * level as f32
}

pub fn weapon_ammo_mult(level: u8) -> f32 {
    1.0 + 0.25 * level as f32
}

// ============================================================ перки

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum PerkKind {
    Vampire,
    Tank,
    Scavenger,
    Bloodlust,
    Pyromaniac,
    Stuntman,
    Adrenaline,
    ThickSkin,
    HairTrigger,
    Ghost,
    Magnet,
    SecondWind,
    Battering,
    Mechanic,
}

pub struct PerkDef {
    pub kind: PerkKind,
    pub name: &'static str,
    pub desc: &'static str,
    pub unlock_level: u32,
}

pub const PERKS: &[PerkDef] = &[
    PerkDef { kind: PerkKind::Mechanic, name: "Механик", desc: "Если тебя не били 5 секунд, машина чинится на 0.6 в секунду.", unlock_level: 1 },
    PerkDef { kind: PerkKind::Battering, name: "Таранщик", desc: "Урон тараном +25%, получаемый от тарана -15%.", unlock_level: 1 },
    PerkDef { kind: PerkKind::Magnet, name: "Магнит", desc: "Бонусы подбираются с расстояния в 2.5 раза больше.", unlock_level: 2 },
    PerkDef { kind: PerkKind::ThickSkin, name: "Толстокожий", desc: "Удары о стены и столбы ранят на 60% меньше.", unlock_level: 2 },
    PerkDef { kind: PerkKind::Bloodlust, name: "Кровопийца", desc: "Очки за пешеходов x2, каждый сбитый чинит на 1.", unlock_level: 3 },
    PerkDef { kind: PerkKind::Scavenger, name: "Барахольщик", desc: "Бонусы действуют на 50% дольше, ящики с патронами дают на 50% больше.", unlock_level: 3 },
    PerkDef { kind: PerkKind::Stuntman, name: "Каскадёр", desc: "Очки за полёт x2, жёсткие приземления не ранят.", unlock_level: 4 },
    PerkDef { kind: PerkKind::HairTrigger, name: "Лёгкий спуск", desc: "Скорострельность всего оружия +25%.", unlock_level: 4 },
    PerkDef { kind: PerkKind::Vampire, name: "Вампир", desc: "12% нанесённого урона возвращается здоровьем.", unlock_level: 5 },
    PerkDef { kind: PerkKind::Adrenaline, name: "Адреналин", desc: "Каждый разбитый соперник полностью заправляет нитро и даёт +5 секунд.", unlock_level: 5 },
    PerkDef { kind: PerkKind::Pyromaniac, name: "Пироман", desc: "Радиус взрывов +30%, огонь горит дольше.", unlock_level: 6 },
    PerkDef { kind: PerkKind::Tank, name: "Танк", desc: "Защита +20%, мощность -8%.", unlock_level: 7 },
    PerkDef { kind: PerkKind::Ghost, name: "Призрак", desc: "Тебя нет на миникарте соперников, боты реже выбирают тебя целью.", unlock_level: 8 },
    PerkDef { kind: PerkKind::SecondWind, name: "Второе дыхание", desc: "Один раз за заезд вместо гибели машина восстанавливается на 35%.", unlock_level: 10 },
];

pub fn perk_def(kind: PerkKind) -> &'static PerkDef {
    PERKS.iter().find(|p| p.kind == kind).expect("perk def")
}

pub const PERK_SLOTS: usize = 3;

// ============================================================ бонусы на карте

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum PowerupKind {
    Repair,
    FullRepair,
    Nitro,
    Ammo,
    Time,
    Credits,
    DoubleDamage,
    Shield,
    MegaRam,
    Pinball,
    MoonGravity,
    SuperGrip,
    Springs,
    Invisibility,
    PedMagnet,
    GiantPeds,
    Freeze,
    Barrage,
    MineRing,
    Mystery,
}

pub struct PowerupDef {
    pub kind: PowerupKind,
    pub name: &'static str,
    pub duration: f32,
    pub color: [f32; 3],
    /// Вес при случайном выборе в ящике «?»
    pub mystery_weight: u32,
}

pub const POWERUPS: &[PowerupDef] = &[
    PowerupDef { kind: PowerupKind::Repair, name: "РЕМОНТ +40", duration: 0.0, color: [0.2, 1.0, 0.35], mystery_weight: 10 },
    PowerupDef { kind: PowerupKind::FullRepair, name: "ПОЛНЫЙ РЕМОНТ", duration: 0.0, color: [0.1, 1.0, 0.7], mystery_weight: 3 },
    PowerupDef { kind: PowerupKind::Nitro, name: "НИТРО", duration: 0.0, color: [0.3, 0.6, 1.0], mystery_weight: 8 },
    PowerupDef { kind: PowerupKind::Ammo, name: "БОЕПРИПАСЫ", duration: 0.0, color: [1.0, 0.8, 0.2], mystery_weight: 8 },
    PowerupDef { kind: PowerupKind::Time, name: "ВРЕМЯ +20 С", duration: 0.0, color: [0.4, 0.8, 1.0], mystery_weight: 6 },
    PowerupDef { kind: PowerupKind::Credits, name: "КРЕДИТЫ +500", duration: 0.0, color: [1.0, 0.9, 0.3], mystery_weight: 6 },
    PowerupDef { kind: PowerupKind::DoubleDamage, name: "ДВОЙНОЙ УРОН", duration: 15.0, color: [1.0, 0.2, 0.2], mystery_weight: 6 },
    PowerupDef { kind: PowerupKind::Shield, name: "НЕУЯЗВИМОСТЬ", duration: 8.0, color: [0.8, 0.9, 1.0], mystery_weight: 4 },
    PowerupDef { kind: PowerupKind::MegaRam, name: "МЕГАТАРАН", duration: 15.0, color: [1.0, 0.4, 0.0], mystery_weight: 6 },
    PowerupDef { kind: PowerupKind::Pinball, name: "ПИНБОЛ", duration: 15.0, color: [1.0, 0.3, 0.9], mystery_weight: 5 },
    PowerupDef { kind: PowerupKind::MoonGravity, name: "ЛУННАЯ ГРАВИТАЦИЯ", duration: 12.0, color: [0.7, 0.7, 0.9], mystery_weight: 5 },
    PowerupDef { kind: PowerupKind::SuperGrip, name: "ЛИПУЧКА", duration: 15.0, color: [0.5, 1.0, 0.3], mystery_weight: 5 },
    PowerupDef { kind: PowerupKind::Springs, name: "ПРУЖИНЫ x3: ПРЫЖОК НА F", duration: 30.0, color: [0.9, 1.0, 0.2], mystery_weight: 5 },
    PowerupDef { kind: PowerupKind::Invisibility, name: "НЕВИДИМКА", duration: 12.0, color: [0.6, 0.6, 0.6], mystery_weight: 4 },
    PowerupDef { kind: PowerupKind::PedMagnet, name: "ПЕШЕХОДЫ-МАГНИТЫ", duration: 15.0, color: [0.9, 0.2, 0.5], mystery_weight: 5 },
    PowerupDef { kind: PowerupKind::GiantPeds, name: "ВЕЛИКАНЫ", duration: 20.0, color: [0.6, 0.3, 0.1], mystery_weight: 3 },
    PowerupDef { kind: PowerupKind::Freeze, name: "ЗАМОРОЗКА СОПЕРНИКОВ", duration: 3.5, color: [0.6, 0.95, 1.0], mystery_weight: 3 },
    PowerupDef { kind: PowerupKind::Barrage, name: "РАКЕТНЫЙ ЗАЛП", duration: 0.0, color: [1.0, 0.25, 0.1], mystery_weight: 4 },
    PowerupDef { kind: PowerupKind::MineRing, name: "КОЛЬЦО МИН", duration: 0.0, color: [0.9, 0.1, 0.1], mystery_weight: 4 },
    PowerupDef { kind: PowerupKind::Mystery, name: "?", duration: 0.0, color: [1.0, 1.0, 1.0], mystery_weight: 0 },
];

pub fn powerup_def(kind: PowerupKind) -> &'static PowerupDef {
    POWERUPS.iter().find(|p| p.kind == kind).expect("powerup def")
}

/// Эффекты с длительностью, которые висят на машине (порядок важен для сети: биты флагов).
pub const TIMED_EFFECTS: [PowerupKind; 9] = [
    PowerupKind::DoubleDamage,
    PowerupKind::Shield,
    PowerupKind::MegaRam,
    PowerupKind::Pinball,
    PowerupKind::MoonGravity,
    PowerupKind::SuperGrip,
    PowerupKind::Springs,
    PowerupKind::Invisibility,
    PowerupKind::PedMagnet,
];

// ============================================================ режимы

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum GameMode {
    Carnage,
    Derby,
    Deathmatch,
    PedHunt,
}

impl GameMode {
    pub const ALL: [GameMode; 4] = [GameMode::Carnage, GameMode::Derby, GameMode::Deathmatch, GameMode::PedHunt];
    pub fn name(self) -> &'static str {
        match self {
            GameMode::Carnage => "Карнаж",
            GameMode::Derby => "Дерби на выживание",
            GameMode::Deathmatch => "Бойня",
            GameMode::PedHunt => "Охота на пешеходов",
        }
    }
    pub fn desc(self) -> &'static str {
        match self {
            GameMode::Carnage => "Классика: пройди все круги, разбей всех или очисти улицы. Время пополняется за каждое действие. Одна жизнь.",
            GameMode::Derby => "Одна жизнь у каждого, пешеходов нет, побеждает последний на ходу.",
            GameMode::Deathmatch => "Воскрешение через 3 секунды. Побеждает тот, кто первым разобьёт нужное число машин.",
            GameMode::PedHunt => "Три минуты. Побеждает тот, кто собьёт больше пешеходов. Комбо умножают очки.",
        }
    }
    pub fn respawns(self) -> bool {
        matches!(self, GameMode::Deathmatch | GameMode::PedHunt)
    }
    pub fn has_peds(self) -> bool {
        !matches!(self, GameMode::Derby)
    }
}

// ============================================================ карты

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum MapId {
    Port,
    Quarry,
    Airfield,
    Downtown,
    Coliseum,
    Junkyard,
}

pub struct MapInfo {
    pub id: MapId,
    pub name: &'static str,
    pub desc: &'static str,
    pub peds: u32,
    pub laps: u32,
    pub time_limit: f32,
    pub unlock_level: u32,
}

pub const MAPS: &[MapInfo] = &[
    MapInfo { id: MapId::Port, name: "Бобровая гавань", desc: "Портовый город: причалы, контейнеры, грузовой корабль и товарняк, который не тормозит.", peds: 150, laps: 2, time_limit: 150.0, unlock_level: 1 },
    MapInfo { id: MapId::Junkyard, name: "Ржавый рай", desc: "Свалка на закате: лабиринт из машин, грязевые ямы, пресс и хафпайп.", peds: 100, laps: 2, time_limit: 140.0, unlock_level: 1 },
    MapInfo { id: MapId::Coliseum, name: "Мясорубка", desc: "Стадион-чаша с наклонными стенами. В центре крутится лопасть-убийца.", peds: 120, laps: 3, time_limit: 120.0, unlock_level: 2 },
    MapInfo { id: MapId::Quarry, name: "Карьер «Глубокий»", desc: "Спиральная дорога в бездну, террасы, прыжки над пропастью и бочки с динамитом.", peds: 110, laps: 2, time_limit: 160.0, unlock_level: 3 },
    MapInfo { id: MapId::Airfield, name: "Аэродром «Северный»", desc: "Взлётная полоса для разгона, ангары насквозь и самолёт, который рулит прямо по тебе.", peds: 130, laps: 2, time_limit: 170.0, unlock_level: 4 },
    MapInfo { id: MapId::Downtown, name: "Неоновый центр", desc: "Ночной центр: небоскрёбы, многоэтажная парковка, эстакада и стройка с ядром-маятником.", peds: 170, laps: 2, time_limit: 170.0, unlock_level: 5 },
];

pub fn map_info(id: MapId) -> &'static MapInfo {
    MAPS.iter().find(|m| m.id == id).expect("map info")
}

// ============================================================ карьера

pub struct CareerEvent {
    pub name: &'static str,
    pub map: MapId,
    pub mode: GameMode,
    pub bots: u32,
    pub difficulty: f32,
    pub reward: u32,
}

pub const CAREER: &[CareerEvent] = &[
    CareerEvent { name: "Первая кровь", map: MapId::Port, mode: GameMode::Carnage, bots: 3, difficulty: 0.25, reward: 1500 },
    CareerEvent { name: "Металлолом", map: MapId::Junkyard, mode: GameMode::Derby, bots: 4, difficulty: 0.3, reward: 1800 },
    CareerEvent { name: "Трибуны ревут", map: MapId::Coliseum, mode: GameMode::Deathmatch, bots: 5, difficulty: 0.35, reward: 2200 },
    CareerEvent { name: "Сезон охоты", map: MapId::Port, mode: GameMode::PedHunt, bots: 4, difficulty: 0.4, reward: 2400 },
    CareerEvent { name: "Спуск в бездну", map: MapId::Quarry, mode: GameMode::Carnage, bots: 5, difficulty: 0.45, reward: 2800 },
    CareerEvent { name: "Взлётный режим", map: MapId::Airfield, mode: GameMode::Deathmatch, bots: 6, difficulty: 0.5, reward: 3200 },
    CareerEvent { name: "Пресс-конференция", map: MapId::Junkyard, mode: GameMode::Carnage, bots: 6, difficulty: 0.55, reward: 3600 },
    CareerEvent { name: "Ночные огни", map: MapId::Downtown, mode: GameMode::Carnage, bots: 6, difficulty: 0.6, reward: 4200 },
    CareerEvent { name: "Гладиаторы", map: MapId::Coliseum, mode: GameMode::Derby, bots: 7, difficulty: 0.7, reward: 4800 },
    CareerEvent { name: "Карьерный рост", map: MapId::Quarry, mode: GameMode::Deathmatch, bots: 7, difficulty: 0.75, reward: 5500 },
    CareerEvent { name: "Нелётная погода", map: MapId::Airfield, mode: GameMode::Carnage, bots: 8, difficulty: 0.85, reward: 6500 },
    CareerEvent { name: "Король асфальта", map: MapId::Downtown, mode: GameMode::Derby, bots: 9, difficulty: 1.0, reward: 9000 },
];

// ============================================================ опыт

pub fn xp_for_level(level: u32) -> u32 {
    if level <= 1 {
        0
    } else {
        (600.0 * ((level - 1) as f32).powf(1.35)) as u32
    }
}

pub fn level_for_xp(xp: u32) -> u32 {
    let mut lvl = 1;
    while lvl < 30 && xp >= xp_for_level(lvl + 1) {
        lvl += 1;
    }
    lvl
}

// ============================================================ имена ботов

pub const BOT_NAMES: &[&str] = &[
    "Бешеный Макс", "Тётя Зина", "Сварщик", "Профессор", "Близнецы", "Гробовщик", "Мясник", "Шиномонтаж",
    "Капитан", "Бабка Ёжка", "Механик Петрович", "Прораб", "Дальнобой", "Кочегар", "Кассирша", "Батюшка",
];


// ============================================================ скины

/// Скин уровня 1 (перекраска по материалу) и 2 (наклейка-панель на двери и капот).
pub struct SkinDef {
    pub name: &'static str,
    pub color: [f32; 3],
    pub metallic: f32,
    pub roughness: f32,
    /// 0 — без наклейки, 1 — «Хорёк» (череп и ключи), 2 — «Пломбир» (рожок)
    pub decal: u8,
}

/// 12 палитр; номер 0 в конфиге — «без скина» (обычная покраска).
pub const SKINS: [SkinDef; 12] = [
    SkinDef { name: "Ржавый череп", color: [0.62, 0.12, 0.09], metallic: 0.35, roughness: 0.55, decal: 1 },
    SkinDef { name: "Пломбир", color: [0.95, 0.62, 0.72], metallic: 0.2, roughness: 0.4, decal: 2 },
    SkinDef { name: "Матовый чёрный", color: [0.05, 0.05, 0.055], metallic: 0.1, roughness: 0.75, decal: 0 },
    SkinDef { name: "Хром", color: [0.78, 0.8, 0.83], metallic: 1.0, roughness: 0.12, decal: 0 },
    SkinDef { name: "Армейский", color: [0.28, 0.32, 0.18], metallic: 0.05, roughness: 0.85, decal: 0 },
    SkinDef { name: "Кислота", color: [0.55, 0.95, 0.1], metallic: 0.3, roughness: 0.35, decal: 0 },
    SkinDef { name: "Полиция", color: [0.08, 0.14, 0.42], metallic: 0.5, roughness: 0.3, decal: 0 },
    SkinDef { name: "Такси", color: [0.98, 0.72, 0.05], metallic: 0.3, roughness: 0.38, decal: 0 },
    SkinDef { name: "Медь", color: [0.72, 0.38, 0.2], metallic: 0.9, roughness: 0.42, decal: 0 },
    SkinDef { name: "Кровь", color: [0.38, 0.0, 0.02], metallic: 0.6, roughness: 0.25, decal: 1 },
    SkinDef { name: "Лёд", color: [0.7, 0.88, 0.98], metallic: 0.4, roughness: 0.2, decal: 0 },
    SkinDef { name: "Сажа", color: [0.18, 0.17, 0.16], metallic: 0.2, roughness: 0.9, decal: 0 },
];
