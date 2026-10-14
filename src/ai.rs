//! ИИ соперников: личности, выбор цели по полезности, навигация по графу, таран с упреждением,
//! отход для повторного удара, применение оружия, охота на пешеходов, ремонт и бегство.

use crate::config::*;
use crate::data::*;
use crate::game::*;
use crate::maps::MapData;
use crate::peds::Ped;
use crate::powerups::Pickup;
use crate::vehicle::*;
use crate::weapons::Weapons;
use avian3d::prelude::*;
use bevy::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[derive(Clone, Copy, Debug)]
pub struct Personality {
    pub aggression: f32,
    pub skill: f32,
    pub ped_lust: f32,
    pub cowardice: f32,
    pub revenge: f32,
    pub reckless: f32,
    /// Дальше этого цели не интересуют (кроме мести), м
    pub hunt_radius: f32,
    /// С какой дистанции начинается разгон на таран, м
    pub ram_dist: f32,
    /// При каком здоровье едет чиниться
    pub repair_hp: f32,
}

/// Что сейчас делает бот (для оверлея F2 и отчёта).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AiState {
    #[default]
    Roam,
    Hunt,
    Ram,
    Shoot,
    Repair,
    Pickup,
    Peds,
    Flee,
    Backoff,
    Unstuck,
}

impl AiState {
    pub fn label(&self) -> &'static str {
        match self {
            AiState::Roam => "патруль",
            AiState::Hunt => "охота",
            AiState::Ram => "таран",
            AiState::Shoot => "стрельба",
            AiState::Repair => "ремонт",
            AiState::Pickup => "бонус",
            AiState::Peds => "пешеходы",
            AiState::Flee => "бегство",
            AiState::Backoff => "отход",
            AiState::Unstuck => "разворот",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Goal {
    Hunt(Entity),
    Repair(Vec3),
    Pickup(Vec3),
    Peds(Vec3),
    Flee(Entity),
    Roam(Vec3),
}

#[derive(Component)]
pub struct Bot {
    pub p: Personality,
    pub goal: Goal,
    pub retarget_t: f32,
    pub path: Vec<Vec3>,
    pub path_goal: Vec3,
    pub replan_t: f32,
    pub stuck_t: f32,
    pub reverse_t: f32,
    pub reverse_steer: f32,
    pub backoff_t: f32,
    pub noise_t: f32,
    pub noise: f32,
    pub last_hp: f32,
    pub hurt_by: Option<Entity>,
    pub hurt_time: f32,
    pub time: f32,
    pub aim: Vec3,
    pub rng: StdRng,
    pub last_attack_time: f32,
    /// Отладка: целевая скорость, щуп, угол, газ, тормоз, руль
    pub dbg: [f32; 6],
    pub turn_reverse: bool,
    pub state: AiState,
    /// Кулдаун тарана по целям: (цель, до какого времени нельзя таранить)
    pub ram_cd: Vec<(Entity, f32)>,
    /// Цель, на которой застряли: её пропускаем при следующем выборе
    pub blocked: Option<(Entity, f32)>,
    pub slow_t: f32,
    /// Очередь/пауза стрельбы: >0 — стреляем ещё столько, <0 — пауза
    pub burst_t: f32,
}

impl Bot {
    pub fn new(seed: u64, difficulty: f32) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let skill = (difficulty + rng.gen_range(-0.12..0.12)).clamp(0.1, 1.0);
        let p = Personality {
            aggression: (rng.gen_range(0.35..1.0) * (0.7 + difficulty * 0.4)).clamp(0.2, 1.0),
            skill,
            ped_lust: rng.gen_range(0.05..0.6),
            cowardice: rng.gen_range(0.0..0.8),
            revenge: rng.gen_range(0.3..1.0),
            reckless: rng.gen_range(0.0..1.0),
            hunt_radius: rng.gen_range(70.0..170.0),
            ram_dist: rng.gen_range(18.0..50.0),
            repair_hp: rng.gen_range(25.0..55.0),
        };
        Self {
            p,
            goal: Goal::Roam(Vec3::ZERO),
            retarget_t: rng.gen_range(0.0..0.8),
            path: Vec::new(),
            path_goal: Vec3::splat(1e6),
            replan_t: 0.0,
            stuck_t: 0.0,
            reverse_t: 0.0,
            reverse_steer: 0.0,
            backoff_t: 0.0,
            noise_t: 0.0,
            noise: 0.0,
            last_hp: 100.0,
            hurt_by: None,
            hurt_time: -100.0,
            time: 0.0,
            aim: Vec3::ZERO,
            rng,
            last_attack_time: -100.0,
            dbg: [0.0; 6],
            turn_reverse: false,
            state: AiState::Roam,
            ram_cd: Vec::new(),
            blocked: None,
            slow_t: 0.0,
            burst_t: 0.0,
        }
    }
}

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (update_dyn_difficulty, bot_drive).chain().before(SimSet::Vehicle).run_if(is_authority).run_if(in_state(AppState::Race)),
        );
    }
}

/// Динамическая сложность: если игрок разбил 2+ машин, ИИ сильнее и быстрее.
fn update_dyn_difficulty(mut dd: ResMut<crate::balance::DynDifficulty>, humans: Query<(&Car, &CarState, &crate::combat::Combat)>) {
    let mut best = (1.0f32, 1.0f32, 1.0f32);
    for (car, st, c) in humans.iter() {
        if car.controller == Controller::Bot {
            continue;
        }
        let m = crate::balance::dyn_difficulty(c.wrecks, if st.wrecked { 100.0 } else { st.hp });
        if m.0 > best.0 {
            best = (m.0, m.1, best.2.max(m.2));
        }
    }
    dd.damage = best.0;
    dd.speed = best.1;
    dd.aggression = best.2;
}

#[derive(Clone)]
struct CarInfo {
    e: Entity,
    pos: Vec3,
    vel: Vec3,
    fwd: Vec3,
    hp: f32,
    wrecked: bool,
    human: bool,
    invisible: bool,
    ghost: bool,
    low_profile: bool,
    last_attacker: Option<Entity>,
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn bot_drive(
    time: Res<Time>,
    spatial: SpatialQuery,
    map: Option<Res<MapData>>,
    mut bots: Query<(Entity, &mut Bot, &mut CarInput, &Car, &CarState, &CarStats, &CarSim, &Weapons, &Position, &Rotation, &LinearVelocity)>,
    all: Query<(Entity, &Car, &CarState, &CarSim, &Position, &Rotation, &LinearVelocity)>,
    peds: Query<&Transform, With<Ped>>,
    pickups: Query<&Pickup>,
    ms: Option<Res<MatchState>>,
    cfg: Option<Res<MatchConfig>>,
    dd: Res<crate::balance::DynDifficulty>,
) {
    let Some(map) = map else { return };
    let dt = time.delta_secs();
    let running = ms.as_ref().map(|m| m.phase == MatchPhase::Running).unwrap_or(false);
    let mode = cfg.as_ref().map(|c| c.mode).unwrap_or(GameMode::Deathmatch);
    let infos: Vec<CarInfo> = all
        .iter()
        .map(|(e, car, st, sim, p, r, v)| CarInfo {
            e,
            pos: p.0,
            vel: v.0,
            fwd: r.0 * Vec3::NEG_Z,
            hp: st.hp,
            wrecked: st.wrecked,
            human: car.controller != Controller::Bot,
            invisible: st.has(PowerupKind::Invisibility),
            ghost: car.cfg.has_perk(PerkKind::Ghost),
            low_profile: car.def().trait_kind == CarTrait::LowProfile && sim.time - sim.last_hit_time > 0.0,
            last_attacker: sim.last_attacker.filter(|_| sim.time - sim.last_attacker_time < 4.0),
        })
        .collect();
    // Кто на кого охотится (ограничение толпы)
    let mut hunting: Vec<(Entity, Entity)> = bots
        .iter()
        .filter_map(|(e, b, ..)| if let Goal::Hunt(t) = b.goal { Some((e, t)) } else { None })
        .collect();

    for (me, mut bot, mut input, car, st, stats, sim, weapons, pos, rot, vel) in bots.iter_mut() {
        bot.time += dt;
        *input = CarInput::default();
        if st.wrecked || !running {
            input.brake = 1.0;
            input.handbrake = true;
            continue;
        }
        let pos = pos.0;
        let rot = rot.0;
        let fwd = rot * Vec3::NEG_Z;
        let right = rot * Vec3::X;
        let speed = vel.0.dot(fwd);
        let skill = bot.p.skill;

        // Кто меня ранил
        if st.hp < bot.last_hp - 2.0 {
            if let Some(a) = sim.last_attacker {
                bot.hurt_by = Some(a);
                bot.hurt_time = bot.time;
                if bot.rng.gen::<f32>() < bot.p.revenge * 0.6 {
                    bot.retarget_t = 0.0;
                }
            }
        }
        bot.last_hp = st.hp;

        // ---------------------------------------------------- выбор цели
        bot.retarget_t -= dt;
        let goal_invalid = match bot.goal {
            Goal::Hunt(t) | Goal::Flee(t) => infos.iter().find(|i| i.e == t).map(|i| i.wrecked).unwrap_or(true),
            Goal::Repair(p) | Goal::Pickup(p) => pos.distance(p) < 3.0,
            Goal::Peds(p) | Goal::Roam(p) => pos.distance(p) < 8.0,
        };
        if bot.retarget_t <= 0.0 || goal_invalid {
            bot.retarget_t = 1.4 - skill * 0.8 + bot.rng.gen_range(0.0..0.5);
            let mut best = (Goal::Roam(map.nav.nodes.get(bot.rng.gen_range(0..map.nav.nodes.len().max(1))).copied().unwrap_or(Vec3::ZERO)), 0.06);
            let low_hp = st.hp < bot.p.repair_hp;
            let blocked = bot.blocked.filter(|b| bot.time < b.1).map(|b| b.0);
            for t in &infos {
                if t.e == me || t.wrecked || Some(t.e) == blocked {
                    continue;
                }
                let d = t.pos.distance(pos);
                if t.invisible && d > 15.0 {
                    continue;
                }
                let revenge_on = bot.hurt_by == Some(t.e) && bot.time - bot.hurt_time < 8.0;
                if d > bot.p.hunt_radius && !revenge_on {
                    continue;
                }
                let mut s = bot.p.aggression * 60.0 / (d + 30.0);
                s *= 1.0 + (100.0 - t.hp) / 150.0;
                if t.human {
                    // динамическая сложность: доминирующего игрока прессуют сильнее,
                    // раненого (< 30%) не добивают толпой
                    s *= dd.aggression * if t.hp < 30.0 { 0.85 } else { 1.0 };
                }
                if bot.hurt_by == Some(t.e) && bot.time - bot.hurt_time < 8.0 {
                    s *= 1.0 + bot.p.revenge;
                }
                if t.ghost {
                    s *= 0.6;
                }
                if t.low_profile && t.last_attacker != Some(me) && bot.hurt_by != Some(t.e) {
                    s *= 0.25;
                }
                let hunters = hunting.iter().filter(|(h, tt)| *tt == t.e && *h != me).count();
                // на игроке не больше двух охотников — жёстко
                if t.human && hunters >= crate::balance::AI_MAX_ON_PLAYER {
                    continue;
                }
                s /= 1.0 + hunters as f32 * 0.9;
                if hunters >= 3 {
                    s *= 0.2;
                }
                if bot.goal == Goal::Hunt(t.e) {
                    s *= 1.3;
                }
                if low_hp {
                    s *= 0.5 + (1.0 - bot.p.cowardice) * 0.5;
                }
                if s > best.1 {
                    best = (Goal::Hunt(t.e), s);
                }
                // Бегство от того, кто рядом и бьёт, когда здоровья мало
                if st.hp < 30.0 && d < 25.0 && bot.hurt_by == Some(t.e) {
                    let fs = bot.p.cowardice * 1.5;
                    if fs > best.1 {
                        best = (Goal::Flee(t.e), fs);
                    }
                }
            }
            // Бонусы
            let ammo_low = weapons.slots.iter().any(|s| s.max_ammo > 0.0 && s.ammo < s.max_ammo * 0.25);
            for pk in pickups.iter() {
                if !pk.active {
                    continue;
                }
                let d = pk.base.distance(pos);
                if d > 140.0 {
                    continue;
                }
                let mut s = match pk.kind {
                    PowerupKind::Repair | PowerupKind::FullRepair => (1.0 - st.hp / 100.0).powi(2) * 2.2 * (1.0 + bot.p.cowardice) * if st.hp < bot.p.repair_hp { 2.0 } else { 1.0 },
                    PowerupKind::Ammo => {
                        if ammo_low {
                            0.6
                        } else {
                            0.08
                        }
                    }
                    PowerupKind::Mystery => 0.35,
                    PowerupKind::Nitro => 0.15,
                    _ => 0.2,
                } * 50.0
                    / (d + 35.0);
                if matches!(pk.kind, PowerupKind::Repair | PowerupKind::FullRepair) && st.hp > 80.0 {
                    s = 0.0;
                }
                if s > best.1 {
                    best = (if matches!(pk.kind, PowerupKind::Repair | PowerupKind::FullRepair) { Goal::Repair(pk.base) } else { Goal::Pickup(pk.base) }, s);
                }
            }
            // Пешеходы
            if mode.has_peds() {
                let mult = match mode {
                    GameMode::PedHunt => 2.5,
                    GameMode::Carnage => 1.0,
                    _ => 0.4,
                };
                let mut nearest = None;
                let mut nd = 50.0f32;
                for (k, t) in peds.iter().enumerate() {
                    if k % 3 != (bot.rng.gen::<u32>() % 3) as usize && k > 30 {
                        continue;
                    }
                    let d = t.translation.distance(pos);
                    if d < nd {
                        nd = d;
                        nearest = Some(t.translation);
                    }
                }
                if let Some(p) = nearest {
                    let s = bot.p.ped_lust * mult * 0.9 * 40.0 / (nd + 20.0);
                    if s > best.1 {
                        best = (Goal::Peds(p), s);
                    }
                }
            }
            if best.0 != bot.goal {
                bot.path.clear();
            }
            bot.goal = best.0;
            hunting.retain(|(h, _)| *h != me);
            if let Goal::Hunt(t) = bot.goal {
                hunting.push((me, t));
            }
        }

        // ---------------------------------------------------- точка цели
        let (goal_pos, target_info) = match bot.goal {
            Goal::Hunt(t) => {
                let ti = infos.iter().find(|i| i.e == t).cloned();
                match ti {
                    Some(ti) => {
                        let d = ti.pos.distance(pos);
                        let lead = (d / speed.abs().max(8.0)).clamp(0.0, 1.4) * (0.3 + 0.7 * skill);
                        (ti.pos + ti.vel * lead, Some(ti))
                    }
                    None => (pos + fwd * 10.0, None),
                }
            }
            Goal::Flee(t) => {
                let tp = infos.iter().find(|i| i.e == t).map(|i| i.pos).unwrap_or(pos);
                let away = horiz(pos - tp).normalize_or_zero();
                (pos + away * 40.0, None)
            }
            Goal::Repair(p) | Goal::Pickup(p) | Goal::Peds(p) | Goal::Roam(p) => (p, None),
        };

        // ---------------------------------------------------- маршрут
        let direct = pos.distance(goal_pos) < 75.0 && map.line_clear(pos + Vec3::Y * 0.5, goal_pos + Vec3::Y * 0.5);
        bot.replan_t -= dt;
        let mut aim = goal_pos;
        if !direct {
            if bot.path.is_empty() || bot.replan_t <= 0.0 || bot.path_goal.distance(goal_pos) > 14.0 {
                bot.replan_t = 1.2;
                bot.path_goal = goal_pos;
                bot.path.clear();
                if let (Some(a), Some(b)) = (map.nav.nearest(pos), map.nav.nearest(goal_pos)) {
                    if let Some(p) = map.nav.path(a, b) {
                        bot.path = p.iter().map(|i| map.nav.nodes[*i]).collect();
                        bot.path.push(goal_pos);
                    }
                }
            }
            while bot.path.len() > 1 && horiz(bot.path[0] - pos).length() < 7.0 {
                bot.path.remove(0);
            }
            // срезаем углы: если следующий узел виден, целимся в него
            if bot.path.len() > 1 && map.line_clear(pos + Vec3::Y * 0.5, bot.path[1] + Vec3::Y * 0.5) {
                bot.path.remove(0);
            }
            if let Some(p) = bot.path.first() {
                aim = *p;
            }
        } else {
            bot.path.clear();
        }
        bot.aim = aim;

        // ---------------------------------------------------- рулёжка
        bot.noise_t -= dt;
        if bot.noise_t <= 0.0 {
            bot.noise_t = bot.rng.gen_range(0.3..0.9);
            bot.noise = bot.rng.gen_range(-1.0..1.0) * (1.0 - skill) * 0.22;
        }
        let local = rot.inverse() * (aim - pos);
        let angle = local.x.atan2(-local.z);
        let dist = Vec2::new(local.x, local.z).length();
        let gain = 1.5 + 1.1 * skill;
        let mut steer = (angle * gain + bot.noise).clamp(-1.0, 1.0);
        let mut throttle = 0.82 + 0.18 * skill;
        let mut brake = 0.0;

        // Щупы препятствий
        let origin = pos + Vec3::Y * 0.4 + fwd * sim.half_l;
        let aspd = speed.abs();
        let feel = 6.0 + aspd * 1.3;
        let side_len = 4.0 + aspd * 0.4;
        let filter = SpatialQueryFilter::from_mask(Layer::World).with_excluded_entities([me]);
        let probe = |d: Vec3, len: f32| -> f32 {
            match Dir3::new(d) {
                Ok(dir) => spatial.cast_ray(origin, dir, len, true, &filter).map(|h| h.distance / len).unwrap_or(1.0),
                Err(_) => 1.0,
            }
        };
        let flat_fwd = horiz(fwd).normalize_or_zero();
        let center_hit = probe(flat_fwd, feel);
        let left_hit = probe(Quat::from_rotation_y(0.42) * flat_fwd, side_len);
        let right_hit = probe(Quat::from_rotation_y(-0.42) * flat_fwd, side_len);
        if left_hit < 1.0 {
            steer += (1.0 - left_hit) * 0.9;
        }
        if right_hit < 1.0 {
            steer -= (1.0 - right_hit) * 0.9;
        }
        let mut target_speed = 99.0f32;
        if center_hit < 1.0 {
            let hd = center_hit * feel;
            if hd < 14.0 {
                steer += if left_hit < right_hit { 0.5 } else { -0.5 };
            }
            target_speed = target_speed.min((2.0 * 8.0 * (hd - 4.0).max(0.0)).sqrt() + 4.0);
        }
        // Объезд чужих машин, на которые не охотимся: меньше случайных столкновений в толчее
        let hunt_target = if let Goal::Hunt(t) = bot.goal { Some(t) } else { None };
        for o in &infos {
            if o.e == me || o.wrecked || Some(o.e) == hunt_target {
                continue;
            }
            let rel = o.pos - pos;
            let ahead = rel.dot(fwd);
            let lat = rel.dot(right);
            if ahead > 0.0 && ahead < 8.0 + aspd * 0.6 && lat.abs() < 3.2 {
                let k = 1.0 - ahead / (8.0 + aspd * 0.6);
                steer -= lat.signum() * 0.7 * k;
                target_speed = target_speed.min(6.0 + ahead);
            }
        }
        steer = steer.clamp(-1.0, 1.0);

        // Торможение перед поворотом пути
        if bot.path.len() >= 2 {
            let a1 = horiz(bot.path[0] - pos);
            let a2 = horiz(bot.path[1] - bot.path[0]);
            if a1.length() > 0.5 && a2.length() > 0.5 {
                let turn = a1.angle_between(a2);
                if turn > 0.5 {
                    let vt = (22.0 + (11.0 - 22.0) * ((turn - 0.5) / 1.2).clamp(0.0, 1.0)) * (0.85 + 0.2 * skill);
                    let dd = a1.length();
                    target_speed = target_speed.min((vt * vt + 2.0 * 7.0 * (dd - 6.0).max(0.0)).sqrt());
                }
            }
        }
        if speed > target_speed + 1.0 {
            throttle = 0.0;
            brake = ((speed - target_speed) * 0.15).clamp(0.25, 1.0);
        } else if speed > target_speed - 2.0 {
            throttle *= 0.5;
        }
        let aang = angle.abs();
        if aang > 1.1 && speed > 13.0 {
            throttle = 0.25;
            brake = brake.max(0.45);
            if skill > 0.55 && aang < 2.2 {
                input.handbrake = true;
            }
        } else if aang > 0.6 && speed > 22.0 {
            throttle = throttle.min(0.5);
        }
        // Цель внутри круга разворота: вперёд не доехать, крутимся задним ходом
        let wheelbase = sim.half_l * 1.36;
        let min_r = wheelbase / 0.55f32.tan() + 0.5;
        let side = if angle >= 0.0 { 1.0 } else { -1.0 };
        let center = Vec2::new(side * min_r, 0.0);
        let inside = (Vec2::new(local.x, local.z) - center).length() < min_r * 0.92;
        if inside && aang > 0.5 && bot.reverse_t <= 0.0 && bot.backoff_t <= 0.0 {
            if speed < 9.0 {
                bot.reverse_t = 1.4;
                bot.reverse_steer = -side;
                bot.turn_reverse = true;
            } else {
                throttle = 0.0;
                brake = brake.max(0.7);
                input.handbrake = aang < 2.0;
            }
        }
        // Таран: цель прямо по курсу и недалеко — газ в пол и нитро.
        // Одну и ту же цель таранить не чаще раза в 4 с: в кулдауне держим дистанцию и стреляем.
        let now = bot.time;
        bot.ram_cd.retain(|c| c.1 > now);
        let mut charge = false;
        let mut cooling = false;
        if let Some(ti) = &target_info {
            // кулдаун на цель 4 с (ТЗ) и общий 6 с: после тарана бот отходит и стреляет
            cooling = bot.ram_cd.iter().any(|c| c.0 == ti.e) || now - bot.last_attack_time < 6.0;
            // таранят только агрессивные; остальные держат дистанцию и стреляют
            // первые 5 с после старта — разъезд без таранов (иначе куча-мала в центре арены)
            let warmup = ms.as_ref().map(|m| m.elapsed < 5.0).unwrap_or(false);
            if !cooling && !warmup && bot.p.aggression > 0.6 && aang < 0.22 && dist < bot.p.ram_dist && dist > 4.0 {
                charge = true;
                throttle = 1.0;
                brake = 0.0;
                target_speed = 99.0;
            }
            let touch = ti.pos.distance(pos) < sim.half_l + 3.0;
            if touch && !cooling {
                bot.ram_cd.push((ti.e, now + crate::balance::AI_RAM_COOLDOWN));
                bot.last_attack_time = now;
                // после удара отъезжаем для нового захода
                if bot.rng.gen::<f32>() < 0.5 + skill * 0.4 {
                    bot.backoff_t = bot.rng.gen_range(0.7..1.2);
                    bot.reverse_steer = 0.0;
                }
            }
            if (cooling || bot.p.aggression <= 0.6) && ti.pos.distance(pos) < 14.0 {
                // кружим рядом, не лезем в лоб
                throttle = throttle.min(0.6);
                steer = (steer + if angle >= 0.0 { -0.8 } else { 0.8 }).clamp(-1.0, 1.0);
            }
        }
        let _ = target_speed;

        // Застревание: быстрее 2 км/ч не едем дольше 1.5 с при газе — задний ход и смена цели
        if throttle > 0.5 && aspd < 2.0 / 3.6 && bot.reverse_t <= 0.0 && bot.backoff_t <= 0.0 {
            bot.stuck_t += dt;
            if bot.stuck_t > 1.5 {
                bot.stuck_t = 0.0;
                bot.reverse_t = bot.rng.gen_range(0.9..1.5);
                bot.reverse_steer = if steer.abs() > 0.1 { -steer.signum() } else if bot.rng.gen::<bool>() { 1.0 } else { -1.0 };
                bot.path.clear();
                if let Goal::Hunt(t) = bot.goal {
                    bot.blocked = Some((t, bot.time + 6.0));
                }
                bot.retarget_t = 0.0;
                bot.state = AiState::Unstuck;
            }
        } else {
            bot.stuck_t = (bot.stuck_t - dt * 0.5).max(0.0);
        }

        // Нитро на прямой к далёкой цели, на таране или при бегстве
        let nitro = (aang < 0.15 && dist > 30.0 && bot.rng.gen::<f32>() < 0.3 + skill * 0.6)
            || (charge && bot.p.aggression > 0.5 && dist < 30.0)
            || matches!(bot.goal, Goal::Flee(_));

        bot.state = match bot.goal {
            _ if bot.reverse_t > 0.0 => AiState::Unstuck,
            _ if bot.backoff_t > 0.0 => AiState::Backoff,
            Goal::Hunt(_) if charge => AiState::Ram,
            Goal::Hunt(_) if cooling => AiState::Shoot,
            Goal::Hunt(_) => AiState::Hunt,
            Goal::Repair(_) => AiState::Repair,
            Goal::Pickup(_) => AiState::Pickup,
            Goal::Peds(_) => AiState::Peds,
            Goal::Flee(_) => AiState::Flee,
            Goal::Roam(_) => AiState::Roam,
        };
        if bot.turn_reverse && bot.reverse_t > 0.0 && aang < 0.45 {
            bot.reverse_t = 0.0;
        }
        if bot.reverse_t <= 0.0 {
            bot.turn_reverse = false;
        }
        if bot.reverse_t > 0.0 || bot.backoff_t > 0.0 {
            bot.reverse_t -= dt;
            bot.backoff_t -= dt;
            input.throttle = 0.0;
            input.brake = 1.0;
            input.steer = if bot.reverse_t > 0.0 { bot.reverse_steer } else { 0.0 };
            continue;
        }

        bot.dbg = [target_speed, center_hit, angle, throttle, brake, steer];
        input.steer = steer;
        input.throttle = throttle;
        input.brake = brake;
        input.nitro = nitro && st.nitro > 0.5;

        // ---------------------------------------------------- оружие
        // очереди с паузами: «огонь без остановки» превращал любой бой в мясорубку за 30 с
        if bot.burst_t > 0.0 {
            bot.burst_t -= dt;
            if bot.burst_t <= 0.0 {
                bot.burst_t = -bot.rng.gen_range(0.8..2.0) * (1.4 - skill * 0.5);
            }
        } else {
            bot.burst_t += dt;
            if bot.burst_t >= 0.0 {
                bot.burst_t = bot.rng.gen_range(0.5..1.2);
            }
        }
        let reaction_ok = bot.burst_t > 0.0 && bot.rng.gen::<f32>() < 0.25 + skill * 0.75;
        if reaction_ok {
            for slot in &weapons.slots {
                let wd = weapon_def(slot.kind);
                if slot.ammo < 1.0 && slot.kind != WeaponKind::Flamer {
                    continue;
                }
                match slot.kind {
                    WeaponKind::MachineGun | WeaponKind::Rockets | WeaponKind::Flamer => {
                        for t in &infos {
                            if t.e == me || t.wrecked || (t.low_profile && bot.hurt_by != Some(t.e)) {
                                continue;
                            }
                            // упреждение: целимся туда, где цель будет к прилёту снаряда
                            let d0 = (t.pos - pos).length();
                            let flight = if wd.speed > 1.0 { d0 / wd.speed } else { 0.0 };
                            let to = t.pos + (t.vel - vel.0) * flight * skill - pos;
                            let d = to.length();
                            let range = if slot.kind == WeaponKind::Flamer { 11.0 } else { wd.range * 0.8 };
                            if d > range {
                                continue;
                            }
                            let dir = match slot.mount {
                                Mount::Sides => {
                                    if to.dot(right) > 0.0 {
                                        right
                                    } else {
                                        -right
                                    }
                                }
                                Mount::Rear => -fwd,
                                _ => fwd,
                            };
                            let cone = if slot.mount == Mount::Roof && slot.kind == WeaponKind::MachineGun {
                                60.0f32
                            } else if slot.mount == Mount::Sides {
                                30.0
                            } else {
                                14.0
                            };
                            if to.normalize_or_zero().dot(dir) > cone.to_radians().cos() && map.line_clear(pos + Vec3::Y, t.pos + Vec3::Y) {
                                input.fire = true;
                            }
                        }
                    }
                    WeaponKind::Homing => {
                        if st.lock_frac >= 1.0 {
                            input.special = true;
                        }
                    }
                    WeaponKind::Bomb => {
                        if let Some(ti) = &target_info {
                            let to = ti.pos - pos;
                            let d = to.length();
                            if d > 12.0 && d < 30.0 && to.normalize_or_zero().dot(fwd) > 0.95 {
                                input.special = true;
                            }
                        }
                    }
                    WeaponKind::Emp => {
                        let near = infos.iter().filter(|t| t.e != me && !t.wrecked && t.pos.distance(pos) < 12.0).count();
                        if near >= 1 && bot.rng.gen::<f32>() < 0.05 + bot.p.reckless * 0.1 {
                            input.special = true;
                        }
                    }
                    WeaponKind::Mines | WeaponKind::Oil => {
                        for t in &infos {
                            if t.e == me || t.wrecked {
                                continue;
                            }
                            let to = t.pos - pos;
                            let d = to.length();
                            if d < 22.0 && to.normalize_or_zero().dot(fwd) < -0.6 && t.vel.dot(-to.normalize_or_zero()) > 3.0 {
                                input.drop = true;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        if (input.fire || input.special) && matches!(bot.state, AiState::Hunt | AiState::Roam) {
            bot.state = AiState::Shoot;
        }
        let _ = stats;
        let _ = car;
    }
}
