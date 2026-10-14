//! Числа баланса и чистые функции, которые проверяются юнит-тестами:
//! урон по Δv, броня, «нет мгновенной смерти», стадии износа, мёртвая зона ввода,
//! правила заднего хода и динамическая сложность.

use bevy::prelude::*;

/// Текущие множители ИИ (динамическая сложность), пересчитываются каждый шаг.
#[derive(Resource, Clone, Copy, Debug)]
pub struct DynDifficulty {
    pub damage: f32,
    pub speed: f32,
    pub aggression: f32,
}

impl Default for DynDifficulty {
    fn default() -> Self {
        Self { damage: 1.0, speed: 1.0, aggression: 1.0 }
    }
}

/// Мёртвая зона осей ввода (газ, тормоз, руль).
pub const INPUT_DEADZONE: f32 = 0.08;
/// После START столько секунд разрешено только движение вперёд.
pub const START_FORWARD_ONLY: f32 = 0.5;
/// Задний ход включается, только если скорость ниже этой (м/с, 5 км/ч)...
pub const REVERSE_MAX_SPEED: f32 = 5.0 / 3.6;
/// ...и тормоз удерживается дольше этого (с).
pub const REVERSE_HOLD: f32 = 0.3;

/// Порог Δv, ниже которого удар не ранит (м/с).
pub const DV_FREE: f32 = 4.0;
/// Коэффициент урона на 1 м/с Δv сверх порога. Подобран тестом тарана:
/// лобовой таран на 80 км/ч в стоящую машину снимает 25–35% кузова.
pub const K_DV: f32 = 7.2;
/// Окно сбора импульса одного удара (с): несколько шагов физики считаются одним ударом.
pub const IMPACT_WINDOW: f32 = 0.2;

pub const START_HP: f32 = 100.0;
pub const START_ARMOR: f32 = 50.0;
/// Доля урона, которую берёт на себя броня, пока она есть.
pub const ARMOR_SHARE: f32 = 0.5;
/// Восстановление брони: через столько секунд без урона, единиц в секунду.
pub const ARMOR_REGEN_DELAY: f32 = 3.0;
pub const ARMOR_REGEN: f32 = 5.0;
/// Неуязвимость после старта и воскрешения (с).
pub const SPAWN_PROTECT: f32 = 1.5;
/// Здоровье, начиная с которого одиночный удар не может уничтожить машину.
pub const ONE_SHOT_GUARD_HP: f32 = 25.0;
/// Скорость удара, после которой при малом здоровье возможна мгновенная гибель (м/с, 120 км/ч).
pub const INSTANT_DEATH_SPEED: f32 = 120.0 / 3.6;

/// Пороги узлов: радиатор (перед), двигатель (перед или общее здоровье), колесо (борт).
pub const RADIATOR_ZONE: f32 = 35.0;
pub const ENGINE_ZONE: f32 = 70.0;
pub const ENGINE_HP: f32 = 30.0;
pub const WHEEL_ZONE: f32 = 40.0;
/// Дебаффы узлов.
pub const RADIATOR_THRUST: f32 = 0.82;
pub const ENGINE_TOP_SPEED: f32 = 0.85;
pub const WHEEL_PULL: f32 = 0.035;

/// Множитель урона оружия (пули, взрывы, огонь, ЭМИ): в дерби на 12 машин без него
/// всё решала стрельба за первые 20–40 секунд.
pub const WEAPON_DAMAGE_MULT: f32 = 0.15;

/// Кулдаун тарана одной цели для ИИ (с).
pub const AI_RAM_COOLDOWN: f32 = 4.0;
/// Не больше стольких ИИ одновременно охотятся на игрока.
pub const AI_MAX_ON_PLAYER: usize = 2;

/// Нормализация оси с мёртвой зоной: |x| < dz → 0, дальше линейно до 1.
pub fn deadzone(x: f32, dz: f32) -> f32 {
    let a = x.abs();
    if a < dz {
        0.0
    } else {
        ((a - dz) / (1.0 - dz)).min(1.0) * x.signum()
    }
}

/// Урон от удара по изменению скорости машины за удар.
pub fn impact_damage(dv: f32) -> f32 {
    K_DV * (dv - DV_FREE).max(0.0)
}

/// Результат применения урона к кузову и броне.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HitResult {
    pub hp: f32,
    pub armor: f32,
    /// Сколько снято с кузова
    pub hp_loss: f32,
    pub wrecked: bool,
}

/// Броня забирает долю урона, затем кузов. Одиночный удар не уничтожает машину
/// со здоровьем ≥ 25: она остаётся на 1. Мгновенная гибель — только если здоровье
/// уже ниже 25 и удар быстрее 120 км/ч.
pub fn apply_hit(hp: f32, armor: f32, dmg: f32, impact_speed: f32) -> HitResult {
    let absorbed = (dmg * ARMOR_SHARE).min(armor.max(0.0));
    let armor_left = armor - absorbed;
    let to_hp = (dmg - absorbed).max(0.0);
    if hp < ONE_SHOT_GUARD_HP && impact_speed > INSTANT_DEATH_SPEED {
        return HitResult { hp: 0.0, armor: armor_left, hp_loss: hp, wrecked: true };
    }
    let mut new_hp = hp - to_hp;
    if hp >= ONE_SHOT_GUARD_HP && new_hp <= 0.0 {
        new_hp = 1.0;
    }
    HitResult { hp: new_hp, armor: armor_left, hp_loss: hp - new_hp.max(0.0), wrecked: new_hp <= 0.0 }
}

/// Стадия износа по доле кузова: >75% — 1, >50% — 2, >25% — 3, иначе 4.
pub fn damage_stage(hp_frac: f32) -> u8 {
    if hp_frac > 0.75 {
        1
    } else if hp_frac > 0.5 {
        2
    } else if hp_frac > 0.25 {
        3
    } else {
        4
    }
}

/// Состояние коробки: решает, можно ли включить задний ход.
pub fn may_reverse(since_start: f32, speed: f32, brake_hold: f32) -> bool {
    since_start >= START_FORWARD_ONLY && speed.abs() < REVERSE_MAX_SPEED && brake_hold > REVERSE_HOLD
}

/// Множители ИИ от успехов игрока: (урон, скорость, агрессия к игроку).
pub fn dyn_difficulty(player_wrecks: u32, player_hp: f32) -> (f32, f32, f32) {
    let mut m = (1.0, 1.0, 1.0);
    if player_wrecks >= 2 {
        m = (1.10, 1.08, 1.2);
    }
    if player_hp < 30.0 {
        m.2 *= 0.85;
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadzone_kills_noise_and_keeps_full_range() {
        assert_eq!(deadzone(0.05, INPUT_DEADZONE), 0.0);
        assert_eq!(deadzone(-0.079, INPUT_DEADZONE), 0.0);
        assert!((deadzone(1.0, INPUT_DEADZONE) - 1.0).abs() < 1e-6);
        assert!((deadzone(-1.0, INPUT_DEADZONE) + 1.0).abs() < 1e-6);
        let mid = deadzone(0.54, INPUT_DEADZONE);
        assert!((mid - 0.5).abs() < 1e-3, "{mid}");
        // монотонность
        let mut prev = 0.0;
        for i in 0..=100 {
            let v = deadzone(i as f32 / 100.0, INPUT_DEADZONE);
            assert!(v >= prev);
            prev = v;
        }
    }

    #[test]
    fn damage_grows_with_dv_above_threshold() {
        assert_eq!(impact_damage(0.0), 0.0);
        assert_eq!(impact_damage(DV_FREE), 0.0);
        assert!(impact_damage(6.0) > 0.0);
        assert!((impact_damage(14.0) - K_DV * 10.0).abs() < 1e-4);
        assert!(impact_damage(20.0) > impact_damage(10.0));
    }

    #[test]
    fn armor_absorbs_share_then_runs_out() {
        let r = apply_hit(100.0, 50.0, 40.0, 10.0);
        assert_eq!(r.armor, 30.0);
        assert_eq!(r.hp, 80.0);
        let r = apply_hit(100.0, 5.0, 40.0, 10.0);
        assert_eq!(r.armor, 0.0);
        assert_eq!(r.hp, 65.0);
    }

    #[test]
    fn no_one_shot_from_healthy() {
        // огромный урон по здоровой машине оставляет её на 1
        let r = apply_hit(100.0, 0.0, 500.0, 50.0);
        assert!(!r.wrecked);
        assert_eq!(r.hp, 1.0);
        let r = apply_hit(25.0, 0.0, 500.0, 10.0);
        assert!(!r.wrecked);
    }

    #[test]
    fn instant_death_only_low_hp_and_fast() {
        // быстро, но здоровье высокое — не гибель
        assert!(!apply_hit(60.0, 0.0, 10.0, 40.0).wrecked);
        // здоровье низкое, но медленно и урон мал — не гибель
        assert!(!apply_hit(20.0, 0.0, 5.0, 20.0).wrecked);
        // здоровье низкое и > 120 км/ч — гибель даже от малого урона
        assert!(apply_hit(20.0, 0.0, 1.0, 34.0).wrecked);
        // при низком здоровье обычный урон может добить
        assert!(apply_hit(10.0, 0.0, 30.0, 10.0).wrecked);
    }

    #[test]
    fn stage_thresholds() {
        assert_eq!(damage_stage(1.0), 1);
        assert_eq!(damage_stage(0.76), 1);
        assert_eq!(damage_stage(0.75), 2);
        assert_eq!(damage_stage(0.51), 2);
        assert_eq!(damage_stage(0.5), 3);
        assert_eq!(damage_stage(0.26), 3);
        assert_eq!(damage_stage(0.25), 4);
        assert_eq!(damage_stage(0.0), 4);
    }

    #[test]
    fn reverse_rules() {
        assert!(!may_reverse(0.2, 0.0, 1.0), "в первые 0.5 с после START только вперёд");
        assert!(!may_reverse(2.0, 3.0, 1.0), "на скорости выше 5 км/ч реверса нет");
        assert!(!may_reverse(2.0, 0.0, 0.2), "тормоз держится меньше 0.3 с");
        assert!(may_reverse(2.0, 0.5, 0.31));
    }

    #[test]
    fn dynamic_difficulty() {
        assert_eq!(dyn_difficulty(0, 100.0), (1.0, 1.0, 1.0));
        let d = dyn_difficulty(2, 100.0);
        assert!((d.0 - 1.10).abs() < 1e-6 && (d.1 - 1.08).abs() < 1e-6);
        let d = dyn_difficulty(0, 20.0);
        assert!((d.2 - 0.85).abs() < 1e-6);
    }
}
