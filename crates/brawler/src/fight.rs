//! One frame of a round. `step_fight` is the whole of it, given what each
//! side is pressing: movement, specials, blows both ways, the called
//! turtles, projectiles and the bell. `update_fight` reads the keys (or asks
//! the bot and the CPU) and calls it.

use super::*;

/// Is this attack thrown with a leg? The drawing asks to pick the limb, the
/// sound to decide whether a gi cracks.
pub(crate) fn is_kick(f: &Fighter) -> bool {
    matches!(f.mv, MoveId::LowKick | MoveId::HighKick | MoveId::Sweep
        | MoveId::JumpKick | MoveId::FlyingKick)
        || (f.mv == MoveId::Special && f.arch().special == Special::TalonKick)
}

/// How far into an attack's startup the leg stops chambering and starts
/// extending. Shared by the pose and the sound so they cannot drift.
pub(crate) const KICK_SNAP: f32 = 0.42;

/// How long a blow freezes the frame: longer the heavier it is.
pub(crate) fn hitstop_for(damage: i32, knockdown: bool, blocked: bool) -> f32 {
    if blocked { 3.0 * F }
    else if knockdown { 9.0 * F }
    else if damage >= 12 { 7.0 * F }
    else { 5.0 * F }
}

/// Serve this round's fruit when its time comes, and feed it to whoever
/// walks over it. Returns who ate.
pub(crate) fn fruit_step(g: &mut Game, dt: f32) -> Option<usize> {
    if g.fruit_due && g.clock <= FRUIT_AT {
        g.fruit_due = false;
        let kind = (g.opponent_index as i32 * 3 + g.round - 1).rem_euclid(FRUIT_POINTS.len() as i32);
        g.fruit = Fruit { x: WIN_W as f32 / 2.0, ttl: FRUIT_STAYS, kind: kind as usize };
    }
    if g.fruit.ttl <= 0.0 { return None; }
    g.fruit.ttl -= dt;
    let who = (0..2).find(|&i| {
        let f = &g.p[i];
        !f.airborne() && f.act != Act::Knockdown && (f.x - g.fruit.x).abs() < f.width() / 2.0 + 10.0
    })?;
    g.fruit.ttl = 0.0;
    let max = g.p[who].arch().health;
    g.p[who].health = (g.p[who].health + (max as f32 * FRUIT_HEAL).round() as i32).min(max);
    let (points, label) = FRUIT_POINTS[g.fruit.kind];
    g.pop(label, g.fruit.x, FLOOR_Y - 46.0);
    if who == 0 && g.mode == Mode::Solo { g.sess.add_score(points); }
    Some(who)
}

/// Two bolts thrown at each other meet and cancel. A laser is not a thing
/// that can be knocked out of the air: it burns through and carries on.
pub(crate) fn clash_bolts(g: &mut Game) -> Option<(f32, f32)> {
    for i in 0..g.bolts.len() {
        for j in i + 1..g.bolts.len() {
            let (a, b) = (g.bolts[i], g.bolts[j]);
            if !a.active || !b.active || a.owner == b.owner { continue; }
            if !rects_overlap(a.x - 10.0, a.y - 8.0, 20.0, 16.0, b.x - 10.0, b.y - 8.0, 20.0, 16.0) {
                continue;
            }
            for k in [i, j] {
                if g.p[g.bolts[k].owner].arch().special != Special::LaserVision {
                    g.bolts[k].active = false;
                }
            }
            return Some(((a.x + b.x) / 2.0, (a.y + b.y) / 2.0));
        }
    }
    None
}

/// Fighter `i` has just touched down. If that was a rage jump the floor
/// shakes, and the other fighter, if standing on it, goes down wherever they
/// are. Returns whether the floor shook. A jump ended by being hit is not a
/// landing.
pub(crate) fn land_quake(g: &mut Game, i: usize) -> bool {
    let stomp = std::mem::take(&mut g.p[i].stomp);
    if !stomp || !matches!(g.p[i].act, Act::Idle | Act::Attack) { return false; }
    g.quake_x = g.p[i].x;
    g.quake_t = QUAKE_SECS;
    g.shake = 0.3;
    let from = g.p[i].size();
    let d = &mut g.p[1 - i];
    if !d.airborne() && d.act != Act::Knockdown && !invulnerable(d) {
        d.hurt(QUAKE_DAMAGE, from);
        d.act = Act::Knockdown;
        d.t = 0.0;
        d.vx = 0.0;
        if i == 0 && g.mode == Mode::Solo { g.sess.add_score(QUAKE_DAMAGE * 10); }
    }
    true
}

/// Let each bar's ghost drain down to the health it stands for.
pub(crate) fn drain_ghosts(g: &mut Game, dt: f32) {
    for i in 0..2 {
        let (now, max) = (g.p[i].health as f32, g.p[i].arch().health as f32);
        g.ghost[i] = if g.ghost[i] <= now { now } else { (g.ghost[i] - max * 0.5 * dt).max(now) };
    }
}

/// One frame of a round for the keys (or the bot) against the CPU or a
/// second player.
pub(crate) fn update_fight(g: &mut Game, dt: f32) {
    if g.hitstop > 0.0 { return hold_frame(g, dt); }

    #[cfg(not(target_arch = "wasm32"))]
    let p_in = if g.demo { cpu_turn(g, 0, dt) }
        else if blip::bot::active() { bot::fight(g, dt) }
        else { human_input(g, 0) };
    #[cfg(target_arch = "wasm32")]
    let p_in = if g.demo { cpu_turn(g, 0, dt) } else { human_input(g, 0) };

    // In versus the second fighter answers to a person, and the CPU is
    // never consulted.
    let c_in = if g.mode == Mode::Versus { human_input(g, 1) } else { cpu_turn(g, 1, dt) };
    step_fight(g, dt, p_in, c_in);
}

/// The freeze after a blow lands: it holds the fighters and the clock, so a
/// round does not lose time to the impacts that make it worth watching.
fn hold_frame(g: &mut Game, dt: f32) {
    g.hitstop -= dt;
    for s in g.hitspark.iter_mut() { if s.ttl > 0.0 { s.ttl -= dt; } }
}

/// One frame of a round, given what each side is pressing. Everything a
/// round is happens here, and it needs no keyboard and no sound device: the
/// tests can play it.
pub(crate) fn step_fight(g: &mut Game, dt: f32, p_in: Input, c_in: Input) {
    if g.hitstop > 0.0 { return hold_frame(g, dt); }

    // The last ten seconds are counted out loud.
    let before = g.clock.ceil();
    g.clock -= dt;
    if g.clock < 10.0 && g.clock > 0.0 && g.clock.ceil() < before { g.snd(Sfx::Tick); }
    if fruit_step(g, dt).is_some() { g.snd(Sfx::Fruit); }

    // Face each other whenever both are free to turn.
    for i in 0..2 {
        let other = g.p[1 - i].x;
        let hovering = g.p[i].soaring() && g.p[i].act == Act::Air;
        if (g.p[i].free() && !g.p[i].airborne()) || hovering {
            g.p[i].facing = if other >= g.p[i].x { 1.0 } else { -1.0 };
        }
    }

    // Wrapped in a web there is no stick: no step, no guard, no blow.
    let p_in = if g.p[0].webbed > 0.0 { Input::default() } else { p_in };
    let c_in = if g.p[1].webbed > 0.0 { Input::default() } else { c_in };

    let close = face_off(&mut g.p);
    apply_input(&mut g.p[0], p_in, close[0], dt);
    apply_input(&mut g.p[1], c_in, close[1], dt);
    let was_air = [g.p[0].airborne(), g.p[1].airborne()];
    advance(&mut g.p[0], dt);
    advance(&mut g.p[1], dt);
    // Boots on boards: the touchdown is how a player hears they are on
    // the ground again.
    for i in 0..2 {
        if was_air[i] && !g.p[i].airborne() {
            g.snd_at(Sfx::Land, 0.35 + 0.5 * g.p[i].land_force);
            if land_quake(g, i) { g.snd(Sfx::Quake); g.snd(Sfx::Crunch); }
        }
    }

    for i in 0..2 {
        g.p[i].x = clamp(g.p[i].x, WALL_MARGIN, WIN_W as f32 - WALL_MARGIN);
    }
    separate(&mut g.p);

    swing_sounds(g, dt);
    fire_specials(g, dt);
    let holds = [p_in, c_in];
    trade_blows(g, holds);
    // The turtles who were called in.
    for (dmg, blocked, knock) in update_helpers(g, dt, holds) {
        if blocked { g.snd(Sfx::Block); }
        else if knock { g.snd(Sfx::Crunch); g.snd_at(Sfx::Cheer, 0.5); }
        else if dmg > 0 { g.snd(Sfx::HitLight(1)); }
    }

    fly_bolts(g, dt, holds);
    for s in g.hitspark.iter_mut() { if s.ttl > 0.0 { s.ttl -= dt; } }
    for w in g.pops.iter_mut() { if w.ttl > 0.0 { w.ttl -= dt; } }
    drain_ghosts(g, dt);
    if g.shake > 0.0 { g.shake -= dt; }
    if g.quake_t > 0.0 { g.quake_t -= dt; }
    if g.combo_t > 0.0 { g.combo_t -= dt; }

    call_round(g);
}

/// The sounds of an attack on its way: the gi cracking as a leg unfolds, and
/// the air ahead of the big ones.
fn swing_sounds(g: &mut Game, dt: f32) {
    // Kicks crack their gi when the shin starts to unfold, at `KICK_SNAP`,
    // the same split the drawing uses between chamber and extension.
    for i in 0..2 {
        let f = g.p[i];
        if f.act == Act::Attack && is_kick(&f) {
            let at = f.scaled(move_data(f.mv)).startup * F * KICK_SNAP;
            if f.t >= at && f.t - dt < at { g.snd(Sfx::Gi); }
        }
        // And the leap itself. The one move that crosses the stage was
        // the only one that made no sound leaving the ground.
        if f.act == Act::Attack && f.mv == MoveId::FlyingKick && f.t <= dt {
            g.snd(Sfx::Whoosh);
        }
        // The big ones are heard coming: the rage jump, the epic punch, the
        // sweep and the high kick.
        if f.act == Act::Attack && f.t <= dt {
            if f.stomp { g.snd(Sfx::Bellow); }
            else if f.epic_punch() || matches!(f.mv, MoveId::Sweep | MoveId::HighKick | MoveId::Uppercut) {
                g.snd_at(Sfx::Swing, 0.7);
            }
        }
    }
}

/// A special does what it does at the end of its startup: the bolt leaves,
/// the rush starts, the kick takes off, the turtles are called.
fn fire_specials(g: &mut Game, dt: f32) {
    // Specials fire their effect at the end of startup.
    for i in 0..2 {
        let f = g.p[i];
        if f.act == Act::Attack && f.mv == MoveId::Special && !f.hit_done {
            let m = f.scaled(move_data(MoveId::Special));
            let at = m.startup * F;
            // The laser goes where he looked as the eyes lit, not where the
            // opponent has got to since: moving off the spot is the way out.
            if f.arch().special == Special::LaserVision && f.aim.is_none() {
                let foe = g.p[1 - i];
                g.p[i].aim = Some((foe.x, foe.y - foe.height() * 0.5));
            }
            if f.calling {
                if f.t >= at && f.t - dt < at {
                    g.p[i].hit_done = true;
                    g.call_turtles(i);
                    g.pop("TURTLES!", f.x, f.y - f.height() - 18.0);
                    g.snd(Sfx::Whistle);
                }
            } else if f.t >= at && f.t - dt < at {
                // Called by name, as the cabinets shouted them.
                g.pop(f.arch().special_name, f.x, f.y - f.height() - 18.0);
                match f.arch().special {
                    Special::ChiBolt | Special::LaserVision => {
                        g.p[i].hit_done = true; // the bolt carries the hit, not the hand
                        g.spawn_bolt(i, m.damage);
                        g.snd(match f.arch().build {
                            Build::Caped => Sfx::Laser,
                            Build::Spider => Sfx::Thwip,
                            _ => Sfx::Projectile,
                        });
                    }
                    Special::BullRush => {
                        g.p[i].vx = f.facing * 430.0;
                        g.snd(Sfx::Whoosh);
                    }
                    Special::TalonKick => {
                        g.p[i].vy = JUMP_VY * 0.82;
                        g.p[i].y -= 0.5;
                        g.p[i].vx = f.facing * 150.0;
                        g.snd(Sfx::Whoosh);
                    }
                }
            }
        }
    }
}

/// Each fighter's hitbox against the other's body, both ways in the same
/// frame: trades are part of the game. `holds` is what each is pressing, for
/// the guard.
fn trade_blows(g: &mut Game, holds: [Input; 2]) {
    for a in 0..2 {
        let d = 1 - a;
        let (mut atk, mut def) = (g.p[a], g.p[d]);
        let reach = atk.hit_box();
        let (dmg, blocked, knock) = resolve_hit(&mut atk, &mut def, holds[d]);
        if def.dizzy > 0.0 && g.p[d].dizzy <= 0.0 {
            g.pop("DIZZY!", def.x, def.y - def.height() - 34.0);
            g.snd(Sfx::Tweet);
        }
        g.p[a] = atk;
        g.p[d] = def;
        if dmg > 0 || blocked {
            let x = (g.p[a].x + g.p[d].x) / 2.0;
            let (sx, sy) = contact(reach, &g.p[a], &g.p[d]);
            g.spark(sx, sy, knock, blocked);
            g.hitstop = g.hitstop.max(hitstop_for(dmg, knock, blocked));
            let over = g.p[d].y - g.p[d].height() - 18.0;
            // The first blow of a round is worth saying so.
            if dmg > 0 && std::mem::take(&mut g.untouched) {
                g.pop("FIRST HIT!", x, over - 24.0);
                if a == 0 && g.mode == Mode::Solo { g.sess.add_score(FIRST_HIT); }
            }
            if dmg > 0 && g.p[a].countered {
                g.pop("COUNTER!", x, over);
                g.snd(Sfx::Counter);
            }
            if g.p[a].turned {
                g.p[a].turned = false;
                g.pop("GUARD IMPACT!", x, over);
                g.hitstop = g.hitstop.max(8.0 * F);
                g.snd(Sfx::Parry);
                push_apart(&mut g.p, 10.0);
            } else if blocked {
                g.snd(Sfx::Block);
                push_apart(&mut g.p, 6.0);
            } else {
                // The combo counter on the receiving end picks the
                // light hit, so a chain climbs. A knockdown gets the
                // crunch and the crowd with it.
                if knock {
                    g.snd(Sfx::Crunch);
                    g.snd_at(Sfx::Cheer, 0.5);
                } else if dmg >= 12 {
                    g.snd(Sfx::HitHeavy);
                } else {
                    let step = (g.p[d].combo.max(1) as usize - 1).min(2);
                    g.snd(Sfx::HitLight(step));
                }
                // A knockdown throws them clear, so the attacker is not
                // standing over an invulnerable wakeup. A throw pushes
                // little: it already leaves the thrower on top.
                let push = match (knock, g.p[a].mv) {
                    (_, MoveId::Throw) => 18.0,
                    (true, _) => 34.0,
                    _ => 9.0,
                };
                push_apart(&mut g.p, push);
                g.shake = if knock { 0.16 } else { 0.08 };
                if g.p[d].combo > 1 {
                    g.combo_shown = g.p[d].combo;
                    g.combo_t = 1.1;
                    g.combo_side = a;
                    // A combo is worth more than the sum of its hits —
                    // it is the part of the round the player earned.
                    if a == 0 { g.sess.add_score(dmg * 10 * g.p[d].combo); }
                } else if a == 0 {
                    g.sess.add_score(dmg * 10);
                }
            }
        }
    }
}

/// Projectiles: two that meet cancel, the rest fly on until they leave the
/// stage or reach a body, guarded or not.
fn fly_bolts(g: &mut Game, dt: f32, holds: [Input; 2]) {
    // Projectiles.
    if let Some((x, y)) = clash_bolts(g) {
        g.spark(x, y, false, true);
        g.snd(Sfx::Parry);
    }
    for i in 0..g.bolts.len() {
        if !g.bolts[i].active { continue; }
        g.bolts[i].x += g.bolts[i].vx * dt;
        g.bolts[i].y += g.bolts[i].vy * dt;
        // Off the side of the stage, or into the boards.
        if g.bolts[i].x < -20.0 || g.bolts[i].x > WIN_W as f32 + 20.0 || g.bolts[i].y > FLOOR_Y {
            g.bolts[i].active = false;
            continue;
        }
        let d = 1 - g.bolts[i].owner;
        let (dx, dy, dw, dh) = g.p[d].hurt_box();
        let (bx, by) = (g.bolts[i].x - 10.0, g.bolts[i].y - 8.0);
        if !rects_overlap(bx, by, 20.0, 16.0, dx, dy, dw, dh) { continue; }
        g.bolts[i].active = false;
        if invulnerable(&g.p[d]) { continue; }
        let guarding = !g.p[d].airborne()
            && holding_back(holds[d], g.p[d].facing)
            && matches!(g.p[d].act, Act::Idle | Act::Walk | Act::Crouch | Act::Block);
        if guarding {
            g.p[d].act = Act::Block;
            g.p[d].crouch_block = holds[d].down;
            g.p[d].stun = 12.0 * F;
            // A guard stops the laser killing, not hurting: it is ducked or
            // dodged, or it is paid for.
            let laser = g.p[1 - d].arch().special == Special::LaserVision;
            let chip = if laser { LASER_CHIP } else { (g.bolts[i].damage / CHIP_DIVISOR).max(1) };
            if laser || !g.p[d].shelled() { g.p[d].hurt(chip, g.p[1 - d].size()); }
            g.snd(Sfx::Block);
        } else {
            // The laser is the end of the round for whoever it touches.
            let laser = g.p[1 - d].arch().special == Special::LaserVision;
            // A web costs little in itself: what it costs is the next five
            // seconds.
            let web = g.p[1 - d].arch().build == Build::Spider;
            let dmg = if laser { g.p[d].health.max(1) }
                else if web { (g.bolts[i].damage / 3).max(1) }
                else { g.bolts[i].damage };
            g.p[d].hurt(dmg, g.p[1 - d].size());
            g.p[d].act = Act::Hitstun;
            g.p[d].stun = 18.0 * F;
            g.p[d].t = 0.0;
            if web && g.p[d].web() {
                let over = g.p[d].y - g.p[d].height() - 18.0;
                g.pop("WEBBED!", g.p[d].x, over);
                g.snd(Sfx::Thwip);
            }
            g.snd(Sfx::HitLight(0));
            g.shake = 0.08;
            if d == 1 { g.sess.add_score(dmg * 10); }
        }
        g.spark(g.bolts[i].x, g.bolts[i].y, false, guarding);
    }
}

/// The bell: somebody is out, or the clock is, and the round is given.
fn call_round(g: &mut Game) {
    // Round over?
    let ko = g.p[0].health <= 0 || g.p[1].health <= 0;
    let time = g.clock <= 0.0;
    if ko || time {
        g.result = round_result(g, ko);
        match g.result {
            RoundResult::P1 => { g.p[0].rounds += 1; g.p[1].act = Act::Defeat; g.p[0].act = Act::Victory;
                g.p[0].t = 0.0; g.p[1].t = 0.0; }
            RoundResult::P2 => { g.p[1].rounds += 1; g.p[0].act = Act::Defeat; g.p[1].act = Act::Victory;
                g.p[0].t = 0.0; g.p[1].t = 0.0; }
            // A double KO gives the round to both, the way the cabinets
            // did. It cannot loop forever: the match ends as soon as
            // either fighter reaches two, and a draw takes both there.
            RoundResult::Draw => { g.p[0].rounds += 1; g.p[1].rounds += 1; }
        }
        if ko {
            g.snd(Sfx::Ko);
            g.snd_at(Sfx::Roar, 0.75);
            g.slow = SLOW_MO_SECS;
        }
        g.fruit.ttl = 0.0;
        g.call = round_call(g);
        g.bonus = [0, 0];
        if g.result == RoundResult::P1 && g.mode == Mode::Solo && !g.demo {
            g.sess.add_score(match g.call { "PERFECT" => 3000, "GREAT" => 1000, _ => 0 });
            // What was left on the clock and on the bar is worth points.
            g.bonus = round_bonus(g.clock, g.p[0].health, g.p[0].arch().health);
            g.sess.add_score(g.bonus[0] + g.bonus[1]);
        }
        g.banner = if ko { "K.O." } else if survived(g) { "SURVIVED" } else { "TIME UP" };
        g.state = State::RoundEnd;
        g.phase.start(2.2);
        // Surviving a round is worth something, and so is surviving it
        // untouched — a player who wins 100-0 has done more than one who
        // wins 100-99.
        if g.result == RoundResult::P1 { g.sess.add_score(1000 + g.p[0].health * 20); }
        let side = match g.result { RoundResult::P1 => "won", RoundResult::P2 => "lost", RoundResult::Draw => "drew" };
        blip::bot::add(&format!("round_{side}_vs{}_{}", g.opponent_index, if ko { "ko" } else { "time" }), 1.0);
    }
}

/// The winner's bonus for a round: a hundred a second left on the clock, and
/// up to three thousand for the health kept.
pub(crate) fn round_bonus(clock: f32, health: i32, full: i32) -> [i32; 2] {
    [clock.max(0.0) as i32 * 100, health.max(0) * 30 / full.max(1) * 100]
}

/// Where a blow met the body: on the near face of the defender, at the
/// height the fist or foot came in.
pub(crate) fn contact(reach: Option<(f32, f32, f32, f32)>, atk: &Fighter, def: &Fighter) -> (f32, f32) {
    let (dx, dy, dw, dh) = def.hurt_box();
    let face = if atk.x < def.x { dx } else { dx + dw };
    let Some((hx, hy, hw, hh)) = reach else { return (face, dy + dh * 0.45) };
    (face.clamp(hx, hx + hw), (hy + hh / 2.0).clamp(dy + 6.0, dy + dh - 6.0))
}

/// Run the turtles who were called in: each waits his turn, runs at the
/// opponent, strikes once under the same rules as anyone, and runs on off
/// the far side. Returns what each blow did: (damage, blocked, knockdown).
pub(crate) fn update_helpers(g: &mut Game, dt: f32, holds: [Input; 2]) -> Vec<(i32, bool, bool)> {
    let mut landed = vec![];
    for k in 0..g.helpers.len() {
        let Some(mut h) = g.helpers[k] else { continue };
        let d = 1 - h.side;
        h.f.foe_size = g.p[d].size();
        if h.wait > 0.0 {
            h.wait -= dt;
        } else if h.f.act == Act::Attack {
            advance(&mut h.f, dt);
            let reach = h.f.hit_box();
            let mut def = g.p[d];
            let (dmg, blocked, knock) = resolve_hit(&mut h.f, &mut def, holds[d]);
            g.p[d] = def;
            if dmg > 0 || blocked {
                let (x, y) = contact(reach, &h.f, &g.p[d]);
                g.spark(x, y, knock, blocked);
                g.hitstop = g.hitstop.max(hitstop_for(dmg, knock, blocked));
                if d == 1 { g.sess.add_score(dmg * 10); }
                landed.push((dmg, blocked, knock));
            }
        } else {
            h.f.act = Act::Walk;
            h.f.t += dt;
            h.f.x += h.f.facing * HELPER_RUN * dt;
            let ahead = (g.p[d].x - h.f.x) * h.f.facing;
            if !h.struck && ahead <= attack_range(&h.f, h.mv) * 0.9 {
                h.struck = true;
                h.f.start_attack(h.mv);
            }
            if h.f.x < -40.0 || h.f.x > WIN_W as f32 + 40.0 {
                g.helpers[k] = None;
                continue;
            }
        }
        note_handover(&mut h.f, dt);
        g.helpers[k] = Some(h);
    }
    landed
}

/// Against the CPU fighter nothing can hurt, the player has only to be
/// standing at the bell.
pub(crate) fn survived(g: &Game) -> bool {
    g.mode == Mode::Solo && g.p[1].arch().invincible && !g.p[0].arch().invincible
        && g.p[0].health > 0 && g.p[1].health > 0
}

/// Who took the round: whoever has more health left, by knockout or at the
/// bell. The one exception is `survived`: a round nobody could have won on
/// health goes to the player for lasting it out.
pub(crate) fn round_result(g: &Game, ko: bool) -> RoundResult {
    if !ko && survived(g) { return RoundResult::P1; }
    if g.p[0].health == g.p[1].health { RoundResult::Draw }
    else if g.p[0].health > g.p[1].health { RoundResult::P1 }
    else { RoundResult::P2 }
}

/// The announcer's verdict, Tekken's: PERFECT for a round won untouched,
/// GREAT for one won on the last sliver of the bar. Nothing for the fighter
/// who cannot be hurt; it is no achievement of his.
pub(crate) fn round_call(g: &Game) -> &'static str {
    let winner = match g.result {
        RoundResult::P1 => g.p[0],
        RoundResult::P2 => g.p[1],
        RoundResult::Draw => return "",
    };
    let max = winner.arch().health;
    if winner.arch().invincible { "" }
    else if winner.health >= max { "PERFECT" }
    else if winner.health * 10 <= max { "GREAT" }
    else { "" }
}

/// Between the last blow and the next round or the match result.
pub(crate) fn update_round_end(g: &mut Game, dt: f32) {
    let dt = if g.slow > 0.0 { g.slow -= dt; dt * SLOW_MO } else { dt };
    // Keep the fighters moving after the round ends, or a mid-air finisher
    // hangs in the sky and the victory pose (driven by the action timer)
    // never plays.
    for i in 0..2 {
        advance(&mut g.p[i], dt);
        g.p[i].x = clamp(g.p[i].x, WALL_MARGIN, WIN_W as f32 - WALL_MARGIN);
    }
    // A bolt in flight at the bell sails on off screen, harmless, rather than hang there.
    for b in g.bolts.iter_mut().filter(|b| b.active) {
        b.x += b.vx * dt;
        b.y += b.vy * dt;
        if b.x < -20.0 || b.x > WIN_W as f32 + 20.0 || b.y > FLOOR_Y { b.active = false; }
    }
    // The finishing blow's spark fades through the round end too.
    for s in g.hitspark.iter_mut() { if s.ttl > 0.0 { s.ttl -= dt; } }
    for w in g.pops.iter_mut() { if w.ttl > 0.0 { w.ttl -= dt; } }
    drain_ghosts(g, dt);
    if g.shake > 0.0 { g.shake -= dt; }
    if g.quake_t > 0.0 { g.quake_t -= dt; }
    if !g.phase.tick(dt) { return; }
    let (a, b) = (g.p[0].rounds, g.p[1].rounds);
    if a >= ROUNDS_TO_WIN || b >= ROUNDS_TO_WIN {
        g.state = State::MatchEnd;
        g.phase.start(MATCH_END);
        g.banner = if g.mode == Mode::Versus {
            if a > b { "PLAYER 1 WINS" } else if b > a { "PLAYER 2 WINS" } else { "DRAW GAME" }
        } else if a > b { "WINNER" } else if b > a { "YOU LOSE" } else { "DRAW GAME" };
    } else {
        g.round += 1;
        g.start_round();
    }
}
