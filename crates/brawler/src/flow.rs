//! Between the fights: the title and select menus, the billing, the bonus
//! round, the result of a match, the continue count, a challenger, and what
//! each leads to.

use super::*;

/// One player's menu intent this frame, debounced. The menus take this rather
/// than the keyboard, so the whole front of the game (mode, cursors, lock-in,
/// no shared fighter) can be driven by a test.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub(crate) struct MenuIn { pub(crate) back: bool, pub(crate) fwd: bool, pub(crate) fire: bool }

/// What a menu transition asked to be heard. Returned rather than
/// played, for the same reason.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum Cue { Step, Confirm }

/// Both players' stick and button as menu moves, with key repeat.
pub(crate) fn read_menu(g: &mut Game) -> [MenuIn; 2] {
    let mut out = [MenuIn::default(); 2];
    for who in 0..2 {
        out[who] = MenuIn {
            back: menu_step(g, who, true),
            fwd: menu_step(g, who, false),
            fire: menu_fire(g, who),
        };
    }
    out
}

/// The title and the select screen. Returns the sound to make, if any.
pub(crate) fn update_menus(g: &mut Game, m: [MenuIn; 2]) -> Option<Cue> {
    match g.state {
        State::Title => {
            // Player two announcing themselves is the choice, like a cabinet:
            // they reach for their own stick. It cannot be toggled back off.
            if g.menu == 0 && (m[1].back || m[1].fwd || m[1].fire) {
                g.menu = 1;
                web::set_mode(true);
                return Some(Cue::Step);
            }
            // The title is where the mode is chosen, on a stick and one
            // button, because that is all a cabinet has.
            if m[0].back || m[0].fwd {
                g.menu = 1 - g.menu;
                // The second station appears as the cursor lands on 2
                // PLAYERS, not on confirm: choosing is asking what two looks
                // like.
                web::set_players(if g.menu == 1 { 1 } else { TITLE_OPEN });
                return Some(Cue::Step);
            }
            // Once the second station is claimed, either player may
            // start the match — player two has as much right to it as
            // the player who put the coin in.
            if m[0].fire || (g.menu == 1 && m[1].fire) {
                g.mode = if g.menu == 0 { Mode::Solo } else { Mode::Versus };
                web::set_mode(g.mode == Mode::Versus);
                // a two-player game takes two coins: this is player two's
                if g.mode == Mode::Versus { web::spend_coin(); }
                g.state = State::Select;
                g.pick = 0;
                g.pick2 = FIGHTERS.len() - 1;
                g.locked = [false; 2];
                return Some(Cue::Confirm);
            }
            None
        }
        State::Select => {
            let versus = g.mode == Mode::Versus;
            let players = if versus { 2 } else { 1 };
            let mut cue = None;
            for who in 0..players {
                if g.locked[who] { continue; }
                let n = FIGHTERS.len();
                // A cursor steps over a fighter the other player has
                // already taken rather than stopping on one it is not
                // allowed to confirm.
                for (step, moved) in [(n - 1, m[who].back), (1, m[who].fwd)] {
                    if !moved { continue; }
                    let mut at = g.picked(who);
                    for _ in 0..n {
                        at = (at + step) % n;
                        if !g.taken_by_other(who, at) { break; }
                    }
                    g.set_pick(who, at);
                    cue = Some(Cue::Step);
                }
                if m[who].fire && !g.taken_by_other(who, g.picked(who)) {
                    g.locked[who] = true;
                    // Move the other player's cursor off a fighter now taken,
                    // so it is never parked on something it cannot confirm.
                    let other = 1 - who;
                    if versus && !g.locked[other] && g.picked(other) == g.picked(who) {
                        let mut at = g.picked(other);
                        for _ in 0..n {
                            at = (at + 1) % n;
                            if !g.taken_by_other(other, at) { break; }
                        }
                        g.set_pick(other, at);
                    }
                    cue = Some(Cue::Confirm);
                }
            }
            if (0..players).all(|w| g.locked[w]) {
                web::spend_coin();
                g.sess.reset(1);
                g.p[0].rounds = 0;
                g.p[1].rounds = 0;
                if versus { g.start_versus(); } else { g.start_match(0); }
                g.announce();
                cue = Some(Cue::Confirm);
            }
            cue
        }
        _ => None,
    }
}

/// One frame of the bonus round. Returns what happened to a barrel this
/// frame, for the sound: struck, or broken.
pub(crate) fn update_bonus(g: &mut Game, dt: f32, inp: Input) -> Option<bool> {
    for b in g.barrels.iter_mut() {
        b.hit_t += dt;
        if b.broke_t >= 0.0 { b.broke_t += dt; }
    }
    for s in g.hitspark.iter_mut() { if s.ttl > 0.0 { s.ttl -= dt; } }
    if g.shake > 0.0 { g.shake -= dt; }
    // Decided: the tally shows, then the ladder goes on.
    if g.bonus_done > 0.0 {
        advance(&mut g.p[0], dt);
        g.bonus_done -= dt;
        if g.bonus_done <= 0.0 {
            g.start_match(g.opponent_index + 1);
            g.announce();
        }
        return None;
    }
    if g.hitstop > 0.0 { g.hitstop -= dt; return None; }
    g.clock -= dt;
    // The ones that fall: down under gravity, a bounce, then away along the
    // boards to the nearer edge, and lost if they reach it.
    for b in g.barrels.iter_mut().filter(|b| b.rolls && b.hp > 0) {
        if b.wait > 0.0 { b.wait -= dt; continue; }
        b.vy += 900.0 * dt;
        b.y += b.vy * dt;
        if b.y >= FLOOR_Y {
            b.y = FLOOR_Y;
            b.vy = if b.vy > 140.0 { -b.vy * 0.35 } else { 0.0 };
            if b.vx == 0.0 { b.vx = if b.x < WIN_W as f32 / 2.0 { -BARREL_ROLL } else { BARREL_ROLL }; }
        }
        b.x += b.vx * dt;
        if b.x < -30.0 || b.x > WIN_W as f32 + 30.0 { b.hp = 0; }
    }

    // The fighter faces the nearest barrel still standing.
    let me = g.p[0].x;
    let target = g.barrels.iter().filter(|b| b.hp > 0 && b.wait <= 0.0)
        .min_by(|a, b| (a.x - me).abs().total_cmp(&(b.x - me).abs())).map(|b| b.x);
    g.p[1].x = target.unwrap_or(me + 200.0);
    if g.p[0].free() && !g.p[0].airborne() {
        g.p[0].facing = if g.p[1].x >= g.p[0].x { 1.0 } else { -1.0 };
    }
    face_off(&mut g.p);
    apply_input(&mut g.p[0], inp, false, dt);
    advance(&mut g.p[0], dt);
    g.p[0].x = clamp(g.p[0].x, WALL_MARGIN, WIN_W as f32 - WALL_MARGIN);

    // One barrel a swing: the nearest the blow reaches, whatever its height.
    let mut event = None;
    if let Some((hx, _, hw, _)) = g.p[0].hit_box() {
        let heavy = g.p[0].scaled(move_data(g.p[0].mv)).damage >= 12;
        // (One still high over the fighter's head is out of reach.)
        let hit = g.barrels.iter_mut().filter(|b| b.hp > 0 && b.wait <= 0.0 && b.y > FLOOR_Y - 150.0)
            .filter(|b| hx < b.x + BARREL_W / 2.0 && hx + hw > b.x - BARREL_W / 2.0)
            .min_by(|a, b| (a.x - me).abs().total_cmp(&(b.x - me).abs()));
        if let Some(b) = hit {
            b.hp -= if heavy { 2 } else { 1 };
            b.hit_t = 0.0;
            let (x, y, broke) = (b.x, b.y, b.hp <= 0);
            if broke { b.broke_t = 0.0; }
            g.p[0].hit_done = true;
            g.spark(x - g.p[0].facing * BARREL_W / 2.0, y - 34.0, broke, false);
            g.hitstop = hitstop_for(if heavy { 12 } else { 6 }, broke, false);
            if broke { g.shake = 0.10; }
            g.sess.add_score(if broke { 1000 } else { 100 });
            event = Some(broke);
        }
    }
    let done = g.barrels.iter().all(|b| b.hp <= 0);
    let broken = g.barrels.iter().filter(|b| b.broke_t >= 0.0).count();
    let cleared = broken == g.bonus_total;
    if done || g.clock <= 0.0 {
        // Every barrel inside the time: the clock is paid out as well.
        g.bonus = [if cleared { g.clock.max(0.0) as i32 * 100 + 3000 } else { 0 }, broken as i32];
        g.sess.add_score(g.bonus[0]);
        g.bonus_done = BONUS_TALLY;
        if g.p[0].free() && !g.p[0].airborne() && cleared { g.p[0].act = Act::Victory; g.p[0].t = 0.0; }
    }
    event
}

/// A second player has pressed their button in the middle of a solo fight.
/// The fight stops where it is for the announcement.
pub(crate) fn challenge(g: &mut Game) {
    g.state = State::Challenger;
    g.phase.start(CHALLENGER_SECS);
    g.snd(Sfx::Gong);
    g.snd(Sfx::Drums);
}

/// Once it has been read, the solo run is over and the two of them choose
/// their fighters, as from the title.
pub(crate) fn update_challenger(g: &mut Game, dt: f32) {
    if !g.phase.tick(dt) { return; }
    let keep = g.pick;
    *g = Game::new();
    g.mode = Mode::Versus;
    g.menu = 1;
    web::set_mode(true);
    // a two-player game takes two coins: this is player two's
    web::spend_coin();
    g.state = State::Select;
    g.pick = keep;
    g.pick2 = if keep == FIGHTERS.len() - 1 { 0 } else { FIGHTERS.len() - 1 };
    g.locked = [false; 2];
    g.fade = FADE;
}

/// Beaten: a press takes the same fight again from nothing, the score with
/// it, for another coin (the kiosk asks for one if there is none); no press
/// by the end of the count and the run is over.
pub(crate) fn update_continue(g: &mut Game, dt: f32, fire: bool) {
    if fire && g.phase.remaining() < CONTINUE_SECS - 0.8 {
        web::spend_coin();
        g.sess.reset(1);
        g.p[0].rounds = 0;
        g.p[1].rounds = 0;
        g.mercy = (g.mercy + MERCY).min(MERCY * 3.0);
        g.continues += 1;
        g.start_match(g.opponent_index);
        g.announce();
    } else if g.phase.tick(dt) {
        web::report_score(g.sess.score);
        g.state = State::Over;
        g.phase.start(1.2);
    }
}

/// The billing runs its time, or a press cuts it short once it has been seen.
pub(crate) fn update_vs(g: &mut Game, dt: f32, fire: bool) {
    let skip = fire && g.phase.remaining() < VS_SECS - 0.8;
    if g.phase.tick(dt) || skip {
        g.state = State::RoundIntro;
        g.phase.start(ROUND_INTRO);
        g.fade = FADE;
    }
}

/// The result of a match on screen. A press moves on once the winner's
/// words have had a second.
pub(crate) fn update_result(g: &mut Game, dt: f32, fire: bool) {
    // Not to zero: a timer at zero is one that was never started, and it
    // never fires.
    if fire && g.phase.remaining() < MATCH_END - 1.2 { g.phase.start(f32::MIN_POSITIVE); }
    update_match_end(g, dt);
}

/// After the result has been read: on up the ladder, to the bonus round, to a
/// continue, or to the ending.
pub(crate) fn update_match_end(g: &mut Game, dt: f32) {
    if !g.phase.tick(dt) { return; }
    // Versus is one match. There is no ladder to climb and no score to
    // report — the result is the whole of it — so it goes back to the
    // title for the next pair.
    if g.mode == Mode::Versus {
        g.state = State::Over;
        g.phase.start(1.2);
        return;
    }
    let won = g.p[0].rounds > g.p[1].rounds;
    if !won {
        g.state = State::Continue;
        g.phase.start(CONTINUE_SECS);
        return;
    }
    if BONUS_AFTER.contains(&g.opponent_index) {
        g.start_bonus();
    } else if g.opponent_index + 1 < RUNGS {
        g.p[0].rounds = 0;
        g.p[1].rounds = 0;
        g.start_match(g.opponent_index + 1);
        g.announce();
    } else {
        g.sess.add_score(5000);
        web::report_score(g.sess.score);
        g.state = State::Won;
        g.phase.start(1.2);
    }
}
