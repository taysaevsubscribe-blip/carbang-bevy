//! Применение урона, гибель машин, очки и статистика бойцов.

use crate::balance::*;
use crate::config::*;
use crate::data::*;
use crate::game::*;
use crate::vehicle::*;
use avian3d::prelude::*;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Счёт и статистика участника. Передаётся клиентам для таблицы и HUD.
#[derive(Component, Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Combat {
    pub score: i32,
    pub wrecks: u32,
    pub peds: u32,
    pub deaths: u32,
    pub damage_dealt: f32,
    pub credits: u32,
    pub combo: u32,
    pub combo_time: f32,
    pub checkpoint: u32,
    pub lap: u32,
    pub eliminated: bool,
    pub ping: u16,
    pub time_left: f32,
    pub finished: bool,
}

/// Журнал урона для автотестов баланса: сумма и крупнейший удар по видам, жертвы среди людей.
#[derive(Resource, Default)]
pub struct DamageLog {
    /// (вид, сумма, число попаданий, крупнейшее)
    pub by_kind: Vec<(DamageKind, f32, u32, f32)>,
    /// Попадания по людям (игрок/автопилот): (время, вид, снято кузова, кузов после)
    pub human_hits: Vec<(f32, DamageKind, f32, f32)>,
    pub time: f32,
}

impl DamageLog {
    pub fn add(&mut self, kind: DamageKind, dmg: f32) {
        if let Some(e) = self.by_kind.iter_mut().find(|e| e.0 == kind) {
            e.1 += dmg;
            e.2 += 1;
            e.3 = e.3.max(dmg);
        } else {
            self.by_kind.push((kind, dmg, 1, dmg));
        }
    }
}

/// Отложенная гибель (для начисления после всех источников урона в кадре).
#[derive(Message)]
pub struct WreckEvent {
    pub victim: Entity,
    pub killer: Option<Entity>,
}

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<WreckEvent>().add_systems(
            FixedUpdate,
            (apply_damage, handle_wrecks)
                .chain()
                .in_set(SimSet::Combat)
                .run_if(is_authority)
                .run_if(in_state(AppState::Race)),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_damage(
    mut ev: MessageReader<DamageEvent>,
    mut cars: Query<(&Car, &CarStats, &mut CarState, &mut CarSim, &NetId)>,
    mut combat: Query<&mut Combat>,
    mut wreck: MessageWriter<WreckEvent>,
    mut events: MessageWriter<GameEvent>,
    ms: Option<Res<MatchState>>,
    mut log: Option<ResMut<DamageLog>>,
    dyn_diff: Option<Res<DynDifficulty>>,
) {
    if let (Some(l), Some(m)) = (log.as_mut(), ms.as_ref()) {
        l.time = m.elapsed;
    }
    let running = ms.map(|m| m.phase == MatchPhase::Running).unwrap_or(true);
    let list: Vec<DamageEvent> = ev.read().cloned().collect();
    if !running {
        return;
    }
    for d in list {
        let attacker_info = d.attacker.and_then(|a| {
            if a == d.victim {
                return None;
            }
            cars.get(a).ok().map(|(c, _, st, _, id)| (a, c.cfg.has_perk(PerkKind::Vampire), st.has(PowerupKind::DoubleDamage), *id, c.controller == Controller::Bot))
        });
        let (dmg, vid, killed) = {
            let Ok((car, stats, mut st, mut sim, vid)) = cars.get_mut(d.victim) else { continue };
            // Щит и неуязвимость после старта/воскрешения
            if st.wrecked || st.has(PowerupKind::Shield) || st.spawn_protect > 0.0 {
                continue;
            }
            let mut amount = d.amount;
            if let Some((_, _, double, _, bot)) = &attacker_info {
                if *double {
                    amount *= 2.0;
                }
                // динамическая сложность: ИИ бьёт сильнее, когда игрок доминирует
                if *bot && car.controller != Controller::Bot {
                    amount *= dyn_diff.as_ref().map(|d| d.damage).unwrap_or(1.0);
                }
            }
            if matches!(d.kind, DamageKind::Bullet | DamageKind::Explosion | DamageKind::Fire | DamageKind::Emp) {
                amount *= WEAPON_DAMAGE_MULT;
            }
            match d.kind {
                DamageKind::Explosion => amount *= stats.blast_resist,
                DamageKind::Wall => {
                    if car.cfg.has_perk(PerkKind::ThickSkin) {
                        amount *= 0.4;
                    }
                }
                DamageKind::Ram => {
                    if car.cfg.has_perk(PerkKind::Battering) {
                        amount *= 0.85;
                    }
                }
                DamageKind::Fall => {
                    if car.cfg.has_perk(PerkKind::Stuntman) && amount < 20.0 {
                        amount = 0.0;
                    }
                }
                _ => {}
            }
            let zone = if d.local_dir.length_squared() > 0.01 {
                let l = d.local_dir;
                let hw = stats.size.x * 0.5;
                let hl = stats.size.z * 0.5;
                if l.z.abs() / hl > l.x.abs() / hw {
                    if l.z < 0.0 { 0 } else { 1 }
                } else if l.x < 0.0 {
                    2
                } else {
                    3
                }
            } else {
                4
            };
            if zone == 0 && car.cfg.weapon_at(Mount::Front) == Some(WeaponKind::Plow) {
                amount /= 1.6;
            }
            let dmg = amount / stats.armor.max(0.2);
            if dmg <= 0.0 {
                continue;
            }
            if zone < 4 {
                st.zones[zone] += dmg;
            }
            // Броня берёт долю урона; одиночный удар не уничтожает здоровую машину;
            // мгновенная гибель только при здоровье < 25 и ударе быстрее 120 км/ч
            let hit = apply_hit(st.hp, st.armor, dmg, d.impact_speed);
            let effective = hit.hp_loss + (st.armor - hit.armor);
            st.armor = hit.armor;
            st.hp = hit.hp;
            if hit.wrecked {
                st.hp = st.hp.min(0.0);
            }
            if let Some(l) = log.as_mut() {
                l.add(d.kind, hit.hp_loss);
                if car.controller != Controller::Bot {
                    let t = l.time;
                    l.human_hits.push((t, d.kind, hit.hp_loss, st.hp));
                }
            }
            sim.last_hit_time = sim.time;
            if let Some((att, ..)) = &attacker_info {
                sim.last_attacker = Some(*att);
                sim.last_attacker_time = sim.time;
            }
            let mut killed = None;
            if st.hp <= 0.0 {
                if car.cfg.has_perk(PerkKind::SecondWind) && !sim.second_wind_used {
                    sim.second_wind_used = true;
                    st.hp = 35.0;
                    st.zones = [0.0; 4];
                    events.write(GameEvent::Message { to: Some(*vid), text: "ВТОРОЕ ДЫХАНИЕ!".into(), color: [0.4, 1.0, 0.5], big: true });
                } else {
                    let killer = attacker_info
                        .as_ref()
                        .map(|(a, ..)| *a)
                        .or(sim.last_attacker.filter(|_| sim.time - sim.last_attacker_time < 6.0));
                    killed = Some(killer);
                }
            }
            (effective, *vid, killed)
        };
        if let Some((att, vampire, _, att_id, _)) = attacker_info {
            if let Ok(mut c) = combat.get_mut(att) {
                c.damage_dealt += dmg;
                c.score += (dmg * 6.0) as i32;
            }
            if dmg > 1.5 {
                events.write(GameEvent::Hit { attacker: att_id, victim: vid, amount: dmg });
            }
            if vampire {
                if let Ok((_, _, mut ast, _, _)) = cars.get_mut(att) {
                    if !ast.wrecked {
                        ast.hp = (ast.hp + dmg * 0.12).min(100.0);
                    }
                }
            }
        }
        if let Some(killer) = killed {
            wreck.write(WreckEvent { victim: d.victim, killer });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_wrecks(
    mut ev: MessageReader<WreckEvent>,
    mut cars: Query<(&Car, &mut CarState, &NetId, &Position, &mut ExternalImpulse, &CarStats)>,
    mut combat: Query<&mut Combat>,
    mut events: MessageWriter<GameEvent>,
    mode: Option<Res<MatchConfig>>,
) {
    let respawns = mode.as_ref().map(|m| m.mode.respawns()).unwrap_or(false);
    let mut done = Vec::new();
    for w in ev.read() {
        if done.contains(&w.victim) {
            continue;
        }
        let Ok((car, mut st, vid, pos, mut imp, stats)) = cars.get_mut(w.victim) else { continue };
        if st.wrecked {
            continue;
        }
        done.push(w.victim);
        st.wrecked = true;
        st.hp = 0.0;
        st.nitro_active = false;
        st.respawn_in = if respawns { 3.5 } else { 0.0 };
        imp.apply_impulse(Vec3::Y * stats.mass * 6.0);
        let victim_name = car.name.clone();
        let vid = *vid;
        let p = pos.0;
        if let Ok(mut c) = combat.get_mut(w.victim) {
            c.deaths += 1;
            if !respawns {
                c.eliminated = true;
            }
        }
        let mut killer_id = None;
        let mut killer_name = String::new();
        if let Some(k) = w.killer.filter(|k| *k != w.victim) {
            if let Ok((kc, mut kst, kid, _, _, _)) = cars.get_mut(k) {
                killer_id = Some(*kid);
                killer_name = kc.name.clone();
                if kc.cfg.has_perk(PerkKind::Adrenaline) {
                    kst.nitro = kst.nitro_max;
                }
            }
            let adrenaline = cars.get(k).map(|c| c.0.cfg.has_perk(PerkKind::Adrenaline)).unwrap_or(false);
            if let Ok(mut kc) = combat.get_mut(k) {
                if adrenaline {
                    kc.time_left += 5.0;
                }
                kc.wrecks += 1;
                kc.score += 1500;
                kc.credits += 400;
            }
        }
        events.write(GameEvent::Explosion { pos: p + Vec3::Y * 0.5, radius: 5.0 });
        events.write(GameEvent::CarWrecked { victim: vid, killer: killer_id, pos: p });
        let text = if killer_name.is_empty() {
            format!("{} разбился", victim_name)
        } else {
            format!("{} разбил {}", killer_name, victim_name)
        };
        events.write(GameEvent::KillFeed { text });
        if let Some(kid) = killer_id {
            events.write(GameEvent::Message { to: Some(kid), text: format!("{} РАЗБИТ!  +1500", victim_name), color: [1.0, 0.3, 0.2], big: true });
        }
        events.write(GameEvent::Message { to: Some(vid), text: "МАШИНА УНИЧТОЖЕНА".into(), color: [1.0, 0.2, 0.2], big: true });
    }
}
