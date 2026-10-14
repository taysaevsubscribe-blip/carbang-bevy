//! Правила режимов: отсчёт, таймеры, очки, комбо, чекпоинты, воскрешение, условия победы и итоги.

use crate::combat::Combat;
use crate::config::*;
use crate::data::*;
use crate::game::*;
use crate::maps::{CheckpointMarker, MapData};
use crate::peds::PedKill;
use crate::vehicle::*;
use crate::weapons::Weapons;
use avian3d::prelude::*;
use bevy::prelude::*;

pub struct RulesPlugin;

impl Plugin for RulesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (tick_match, ped_scoring, wreck_scoring, landing_scoring, checkpoints, respawns, end_conditions)
                .chain()
                .in_set(SimSet::Rules)
                .run_if(is_authority)
                .run_if(in_state(AppState::Race)),
        )
        .add_systems(Update, (checkpoint_markers, finish_to_results).run_if(in_state(AppState::Race)));
    }
}

fn local_combat<'a>(local: &LocalPlayer, q: &'a Query<(&NetId, &mut Combat)>) -> Option<&'a Combat> {
    let id = local.car?;
    q.iter().find(|(n, _)| **n == id).map(|(_, c)| c)
}

fn tick_match(
    time: Res<Time>,
    mut ms: ResMut<MatchState>,
    cfg: Res<MatchConfig>,
    mut combat: Query<(&NetId, &mut Combat)>,
    cars: Query<(&CarState, &NetId)>,
    local: Res<LocalPlayer>,
    mut events: MessageWriter<GameEvent>,
) {
    let dt = time.delta_secs();
    match ms.phase {
        MatchPhase::Countdown => {
            let before = ms.countdown.ceil();
            ms.countdown -= dt;
            if ms.countdown.ceil() < before && ms.countdown > 0.0 {
                events.write(GameEvent::Message { to: None, text: format!("{}", ms.countdown.ceil() as i32), color: [1.0, 0.85, 0.1], big: true });
            }
            if ms.countdown <= 0.0 {
                ms.phase = MatchPhase::Running;
                events.write(GameEvent::Message { to: None, text: "ВПЕРЁД!".into(), color: [1.0, 0.85, 0.1], big: true });
            }
        }
        MatchPhase::Running => {
            ms.elapsed += dt;
            if cfg.mode == GameMode::Carnage {
                for (id, mut c) in combat.iter_mut() {
                    let wrecked = cars.iter().find(|(_, n)| *n == id).map(|(s, _)| s.wrecked).unwrap_or(true);
                    if !c.eliminated && !wrecked {
                        c.time_left -= dt;
                        if c.time_left <= 0.0 {
                            c.time_left = 0.0;
                            c.eliminated = true;
                            events.write(GameEvent::Message { to: Some(*id), text: "ВРЕМЯ ВЫШЛО".into(), color: [1.0, 0.3, 0.2], big: true });
                        }
                    }
                }
                if let Some(c) = local_combat(&local, &combat) {
                    ms.time_left = c.time_left;
                }
            } else {
                ms.time_left = (cfg.time_limit - ms.elapsed).max(0.0);
            }
            for (_, mut c) in combat.iter_mut() {
                c.combo_time -= dt;
                if c.combo_time <= 0.0 {
                    c.combo = 0;
                }
            }
        }
        MatchPhase::Finished => {
            ms.countdown += dt;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn ped_scoring(
    mut ev: MessageReader<PedKill>,
    mut cars: Query<(&Car, &mut CarState, &NetId)>,
    mut combat: Query<&mut Combat>,
    mut events: MessageWriter<GameEvent>,
    cfg: Res<MatchConfig>,
) {
    for k in ev.read() {
        let Some(killer) = k.killer else { continue };
        let Ok((car, mut st, id)) = cars.get_mut(killer) else { continue };
        let Ok(mut c) = combat.get_mut(killer) else { continue };
        if c.combo_time > 0.0 {
            c.combo = (c.combo + 1).min(5);
        } else {
            c.combo = 1;
        }
        c.combo_time = 2.5;
        let mut pts = 120 * c.combo as i32;
        if car.cfg.has_perk(PerkKind::Bloodlust) {
            pts *= 2;
            st.hp = (st.hp + 1.0).min(100.0);
        }
        if car.def().trait_kind == CarTrait::PedVampire {
            st.hp = (st.hp + 3.0).min(100.0);
        }
        c.peds += 1;
        c.score += pts;
        c.credits += 20;
        if cfg.mode == GameMode::Carnage {
            c.time_left += 4.0;
        }
        let text = if c.combo > 1 { format!("КОМБО x{}!  +{}", c.combo, pts) } else { format!("ПЕШЕХОД  +{}", pts) };
        events.write(GameEvent::Message { to: Some(*id), text, color: [1.0, 0.25, 0.2], big: false });
    }
}

fn wreck_scoring(mut ev: MessageReader<GameEvent>, mut combat: Query<(&NetId, &mut Combat)>, cfg: Res<MatchConfig>) {
    for e in ev.read() {
        if let GameEvent::CarWrecked { killer: Some(k), .. } = e {
            if cfg.mode == GameMode::Carnage {
                for (id, mut c) in combat.iter_mut() {
                    if id == k {
                        c.time_left += 30.0;
                    }
                }
            }
        }
        if let GameEvent::Hit { attacker, amount, .. } = e {
            if cfg.mode == GameMode::Carnage {
                for (id, mut c) in combat.iter_mut() {
                    if id == attacker {
                        c.time_left += amount * 0.15;
                    }
                }
            }
        }
    }
}

fn landing_scoring(
    mut events: ResMut<Messages<GameEvent>>,
    mut cursor: Local<bevy::ecs::message::MessageCursor<GameEvent>>,
    cars: Query<(&Car, &NetId)>,
    mut combat: Query<(&NetId, &mut Combat)>,
    mut out: Local<Vec<GameEvent>>,
) {
    out.clear();
    let incoming: Vec<GameEvent> = cursor.read(&events).cloned().collect();
    for e in incoming.iter() {
        if let GameEvent::Landed { car, air } = e {
            if *air < 1.0 {
                continue;
            }
            let mult = cars
                .iter()
                .find(|(_, id)| *id == car)
                .map(|(c, _)| {
                    let mut m = 1.0;
                    if c.cfg.has_perk(PerkKind::Stuntman) {
                        m *= 2.0;
                    }
                    if c.def().trait_kind == CarTrait::AirControl {
                        m *= 2.0;
                    }
                    m
                })
                .unwrap_or(1.0);
            let pts = (air * 300.0 * mult) as i32;
            for (id, mut c) in combat.iter_mut() {
                if id == car {
                    c.score += pts;
                    c.credits += (pts / 20) as u32;
                }
            }
            out.push(GameEvent::Message { to: Some(*car), text: format!("ПОЛЁТ {:.1} С  +{}", air, pts), color: [0.5, 0.8, 1.0], big: false });
        }
    }
    for e in out.drain(..) {
        events.write(e);
    }
}

fn checkpoints(
    cfg: Res<MatchConfig>,
    map: Res<MapData>,
    cars: Query<(Entity, &CarState, &Position, &NetId)>,
    mut combat: Query<&mut Combat>,
    mut events: MessageWriter<GameEvent>,
) {
    if cfg.mode != GameMode::Carnage || map.checkpoints.is_empty() {
        return;
    }
    let n = map.checkpoints.len() as u32;
    for (e, st, pos, id) in cars.iter() {
        if st.wrecked {
            continue;
        }
        let Ok(mut c) = combat.get_mut(e) else { continue };
        if c.finished || c.eliminated {
            continue;
        }
        let cp = map.checkpoints[(c.checkpoint % n) as usize];
        if horiz(pos.0 - cp).length() < 8.0 && (pos.0.y - cp.y).abs() < 6.0 {
            c.checkpoint += 1;
            c.score += 250;
            c.time_left += 12.0;
            if c.checkpoint % n == 0 {
                c.lap += 1;
                if c.lap >= cfg.laps {
                    c.finished = true;
                    events.write(GameEvent::Message { to: Some(*id), text: "ВСЕ КРУГИ ПРОЙДЕНЫ!".into(), color: [0.4, 1.0, 0.5], big: true });
                } else {
                    events.write(GameEvent::Message { to: Some(*id), text: format!("КРУГ {}/{}  +12 С", c.lap + 1, cfg.laps), color: [1.0, 0.85, 0.1], big: true });
                }
            } else {
                events.write(GameEvent::Message { to: Some(*id), text: "КОНТРОЛЬНАЯ ТОЧКА  +12 С".into(), color: [1.0, 0.85, 0.1], big: false });
            }
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn respawns(
    time: Res<Time>,
    cfg: Res<MatchConfig>,
    map: Res<MapData>,
    mut cars: Query<(Entity, &mut CarState, &mut CarSim, &mut Weapons, &CarStats, &mut Position, &mut Rotation, &mut LinearVelocity, &mut AngularVelocity, &NetId)>,
    mut events: MessageWriter<GameEvent>,
) {
    if !cfg.mode.respawns() {
        return;
    }
    let dt = time.delta_secs();
    let positions: Vec<Vec3> = cars.iter().filter(|c| !c.1.wrecked).map(|c| c.5 .0).collect();
    for (_e, mut st, mut sim, mut w, stats, mut p, mut r, mut lv, mut av, id) in cars.iter_mut() {
        if !st.wrecked {
            continue;
        }
        st.respawn_in -= dt;
        if st.respawn_in > 0.0 {
            continue;
        }
        // Самая дальняя от соперников точка старта
        let mut best = map.spawns.first().copied().unwrap_or((Vec3::ZERO, 0.0));
        let mut best_d = -1.0;
        for s in &map.spawns {
            let d = positions.iter().map(|o| o.distance(s.0)).fold(f32::MAX, f32::min);
            if d > best_d {
                best_d = d;
                best = *s;
            }
        }
        p.0 = best.0 + Vec3::Y * (stats.ride_height + 0.3);
        r.0 = Quat::from_rotation_y(best.1);
        lv.0 = Vec3::ZERO;
        av.0 = Vec3::ZERO;
        *st = CarState { nitro: stats.nitro_max, nitro_max: stats.nitro_max, ..default() };
        st.set_effect(PowerupKind::Shield, 2.5);
        w.refill(1.0);
        sim.upside_timer = 0.0;
        sim.last_attacker = None;
        sim.second_wind_used = false;
        events.write(GameEvent::Respawned { car: *id });
    }
}

#[allow(clippy::too_many_arguments)]
fn end_conditions(
    mut ms: ResMut<MatchState>,
    cfg: Res<MatchConfig>,
    cars: Query<(&Car, &CarState, &NetId, &Combat)>,
    mut events: MessageWriter<GameEvent>,
) {
    if ms.phase != MatchPhase::Running {
        return;
    }
    let total = cars.iter().count();
    let alive: Vec<_> = cars.iter().filter(|(_, st, _, c)| !st.wrecked && !c.eliminated).collect();
    let humans_alive = alive.iter().filter(|(car, ..)| car.controller != Controller::Bot).count();
    let humans_total = cars.iter().filter(|(car, ..)| car.controller != Controller::Bot).count();
    let best_score = || cars.iter().max_by_key(|(_, _, _, c)| c.score).map(|(_, _, id, _)| *id);
    match cfg.mode {
        GameMode::Carnage => {
            if let Some((_, _, id, _)) = cars.iter().find(|(_, _, _, c)| c.finished) {
                finish(&mut *ms, Some(*id), "Все круги пройдены");
            } else if alive.len() == 1 && total > 1 {
                finish(&mut *ms, Some(*alive[0].2), "Все соперники разбиты");
            } else if ms.peds_total > 0 && ms.peds_alive == 0 {
                finish(&mut *ms, best_score(), "Улицы опустели");
            } else if humans_total > 0 && humans_alive == 0 {
                finish(&mut *ms, best_score(), "Время вышло или машина разбита");
            } else if alive.is_empty() {
                finish(&mut *ms, best_score(), "Никого не осталось");
            }
        }
        GameMode::Derby => {
            if alive.len() <= 1 && total > 1 {
                finish(&mut *ms, alive.first().map(|a| *a.2), "Последний на ходу");
            } else if humans_total > 0 && humans_alive == 0 && humans_total == total {
                finish(&mut *ms, None, "Все разбиты");
            } else if ms.time_left <= 0.0 {
                let w = alive.iter().max_by(|a, b| a.1.hp.partial_cmp(&b.1.hp).unwrap_or(std::cmp::Ordering::Equal)).map(|a| *a.2);
                finish(&mut *ms, w, "Время вышло: побеждает самый целый");
            }
        }
        GameMode::Deathmatch => {
            if let Some((_, _, id, _)) = cars.iter().find(|(_, _, _, c)| c.wrecks >= cfg.frag_limit) {
                finish(&mut *ms, Some(*id), "Лимит побед набран");
            } else if ms.time_left <= 0.0 {
                let w = cars.iter().max_by_key(|(_, _, _, c)| (c.wrecks, c.score)).map(|(_, _, id, _)| *id);
                finish(&mut *ms, w, "Время вышло");
            }
        }
        GameMode::PedHunt => {
            if ms.time_left <= 0.0 || (ms.peds_total > 0 && ms.peds_alive == 0) {
                let w = cars.iter().max_by_key(|(_, _, _, c)| (c.peds, c.score)).map(|(_, _, id, _)| *id);
                finish(&mut *ms, w, "Охота окончена");
            }
        }
    }
    if ms.phase == MatchPhase::Finished {
        let name = ms.winner.and_then(|w| cars.iter().find(|(_, _, id, _)| **id == w).map(|(c, ..)| c.name.clone())).unwrap_or_default();
        let text = if name.is_empty() { "МАТЧ ОКОНЧЕН".to_string() } else { format!("ПОБЕДИТЕЛЬ: {}", name) };
        events.write(GameEvent::Message { to: None, text, color: [1.0, 0.85, 0.1], big: true });
    }
}

fn finish(ms: &mut MatchState, winner: Option<NetId>, reason: &str) {
    ms.phase = MatchPhase::Finished;
    ms.countdown = 0.0;
    ms.winner = winner;
    ms.end_reason = reason.to_string();
}

/// Показывает только следующий чекпоинт локального игрока.
fn checkpoint_markers(
    local: Res<LocalPlayer>,
    cfg: Option<Res<MatchConfig>>,
    combat: Query<(&NetId, &Combat)>,
    map: Option<Res<MapData>>,
    mut q: Query<(&CheckpointMarker, &mut Visibility)>,
) {
    let (Some(cfg), Some(map)) = (cfg, map) else { return };
    let show = cfg.mode == GameMode::Carnage;
    let n = map.checkpoints.len().max(1) as u32;
    let next = local.car.and_then(|id| combat.iter().find(|(n, _)| **n == id).map(|(_, c)| (c.checkpoint % n) as usize));
    for (m, mut v) in q.iter_mut() {
        let vis = show && Some(m.0) == next;
        *v = if vis { Visibility::Visible } else { Visibility::Hidden };
    }
}

/// Через 3 секунды после конца матча собирает итоги и переходит на экран результатов.
#[allow(clippy::too_many_arguments)]
fn finish_to_results(
    ms: Option<Res<MatchState>>,
    cars: Query<(&Car, &NetId, &Combat)>,
    local: Res<LocalPlayer>,
    cfg: Res<MatchConfig>,
    mut profile: ResMut<Profile>,
    mut result: ResMut<MatchResult>,
    mut next: ResMut<NextState<AppState>>,
    headless: Res<Headless>,
    mut done: Local<bool>,
) {
    let Some(ms) = ms else {
        *done = false;
        return;
    };
    if ms.phase != MatchPhase::Finished {
        *done = false;
        return;
    }
    if ms.countdown < 3.5 || *done {
        return;
    }
    *done = true;
    let mut rows: Vec<ResultRow> = cars
        .iter()
        .map(|(car, id, c)| ResultRow {
            name: car.name.clone(),
            car: car.def().name.to_string(),
            score: c.score,
            wrecks: c.wrecks,
            peds: c.peds,
            deaths: c.deaths,
            damage: c.damage_dealt as u32,
            is_local: Some(*id) == local.car,
        })
        .collect();
    rows.sort_by(|a, b| b.score.cmp(&a.score));
    let won = ms.winner.is_some() && ms.winner == local.car;
    let mine = cars.iter().find(|(_, id, _)| Some(**id) == local.car);
    let mut credits = 0;
    let mut xp = 0;
    if let Some((car, _, c)) = mine {
        let stats = CarStats::compute(&car.cfg);
        let career_mult = 1.0 + cfg.difficulty;
        let mut cr = (c.score.max(0) as f32 * 0.35 * career_mult) as u32 + c.credits;
        if won {
            cr += cfg.career_event.map(|i| CAREER[i].reward).unwrap_or(1200);
        }
        credits = (cr as f32 * stats.credit_mult) as u32;
        xp = (c.score.max(0) / 8) as u32 + if won { 400 } else { 100 } + c.wrecks * 60;
        profile.total_wrecks += c.wrecks;
        profile.total_peds += c.peds;
    }
    let level_before = profile.level();
    profile.credits += credits;
    profile.xp += xp;
    if won {
        if let Some(i) = cfg.career_event {
            if i as u32 + 1 > profile.career_done {
                profile.career_done = i as u32 + 1;
            }
        }
    }
    if !headless.0 {
        profile.save();
    }
    *result = MatchResult {
        rows,
        won,
        reason: ms.end_reason.clone(),
        credits,
        xp,
        career_event: cfg.career_event,
        level_before,
        level_after: profile.level(),
    };
    next.set(AppState::Results);
}
