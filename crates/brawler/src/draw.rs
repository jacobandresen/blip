//! Drawing. The fighters are posed from the same numbers the rules use.
//!
//! Every limb that matters is placed from `move_data()` — the reach and
//! height a punch is judged by are the reach and height it is *drawn*
//! at. A fighting game where the picture and the hitbox disagree teaches
//! the player to stop believing what they see, and there is nothing left
//! to play after that.

use super::*;

fn rgb(c: (f32, f32, f32)) -> BlipColor { BlipColor { r: c.0, g: c.1, b: c.2, a: 1.0 } }

/// Width of a string in pixels at size `sz`.
///
/// Not to be confused with blip's `text_cx()`, which returns the x that
/// centres a string on the *whole canvas* — useful for a banner, useless
/// for right-aligning a name over a health bar, and silently wrong if
/// mistaken for a width. The font is a fixed 6px cell per character.
fn text_w(text: &str, sz: f32) -> f32 { text.len() as f32 * 6.0 * sz }
fn rgba(c: (f32, f32, f32), a: f32) -> BlipColor { BlipColor { r: c.0, g: c.1, b: c.2, a } }

pub fn draw(blip: &Blip, g: &Game) {
    match g.state {
        State::Title => draw_title(blip),
        State::Select => draw_select(blip, g),
        _ => {
            draw_stage(blip, g);
            draw_fight(blip, g);
            draw_hud(blip, g);
            draw_banner(blip, g);
        }
    }
}

// ---- locations -----------------------------------------------------------

/// Two places to fight, and they are only allowed to be scenery: the
/// floor is the same flat line in both, at the same height, with the
/// same walls. A stage that changed the fight would make the ladder's
/// difficulty a matter of where you were standing.
fn draw_stage(blip: &Blip, g: &Game) {
    // A hit shakes the whole picture, which is most of what selling an
    // impact means when the fighters themselves are simple shapes.
    let shake = if g.shake > 0.0 { (g.shake * 60.0).sin() * g.shake * 30.0 } else { 0.0 };
    if g.stage == 0 { draw_dock(blip, shake, g.now); } else { draw_temple(blip, shake, g.now); }

    // The floor line both stages share.
    blip.fill_rect(0.0, FLOOR_Y + shake, WIN_W as f32, WIN_H as f32 - FLOOR_Y,
        BlipColor { r: 0.13, g: 0.11, b: 0.10, a: 1.0 });
    blip.draw_line(0.0, FLOOR_Y + shake, WIN_W as f32, FLOOR_Y + shake,
        BlipColor { r: 0.45, g: 0.40, b: 0.35, a: 1.0 });
}

/// THE DOCKS — sunset, water, cranes, crates.
fn draw_dock(blip: &Blip, shake: f32, t: f32) {
    let bands = 12;
    let h = FLOOR_Y / bands as f32;
    for i in 0..bands {
        let k = i as f32 / (bands - 1) as f32;
        let c = BlipColor {
            r: 0.95 - 0.55 * k, g: 0.45 - 0.22 * k, b: 0.30 + 0.18 * k, a: 1.0,
        };
        blip.fill_rect(0.0, i as f32 * h + shake, WIN_W as f32, h + 1.0, c);
    }
    // Sun low over the water.
    blip.fill_circle(500.0, 190.0 + shake, 46.0, BlipColor { r: 1.0, g: 0.85, b: 0.45, a: 0.9 });
    // Cranes: verticals with a jib, far enough back to read as skyline.
    for (x, hgt) in [(90.0f32, 150.0f32), (160.0, 110.0), (560.0, 130.0)] {
        let base = FLOOR_Y - 64.0 + shake;
        let c = BlipColor { r: 0.18, g: 0.12, b: 0.14, a: 1.0 };
        blip.fill_rect(x, base - hgt, 7.0, hgt, c);
        blip.fill_rect(x - 26.0, base - hgt, 70.0, 6.0, c);
        blip.draw_line(x + 3.0, base - hgt + 6.0, x + 40.0, base - hgt + 34.0, c);
    }
    // Water, with a moving glitter line — and then the quay in front of
    // it. The fighters stand on the planks; drawing the sea right up to
    // the floor line put them ankle-deep in it.
    blip.fill_rect(0.0, FLOOR_Y - 64.0 + shake, WIN_W as f32, 36.0,
        BlipColor { r: 0.20, g: 0.26, b: 0.40, a: 1.0 });
    for i in 0..16 {
        let x = ((i as f32 * 53.0) + (t * 18.0) % 53.0) % WIN_W as f32;
        let y = FLOOR_Y - 58.0 + ((i * 7) % 24) as f32 + shake;
        blip.fill_rect(x, y, 18.0, 1.5, BlipColor { r: 1.0, g: 0.8, b: 0.6, a: 0.28 });
    }
    blip.fill_rect(0.0, FLOOR_Y - 28.0 + shake, WIN_W as f32, 28.0,
        BlipColor { r: 0.33, g: 0.24, b: 0.17, a: 1.0 });
    for i in 0..22 {
        let x = i as f32 * 30.0;
        blip.draw_line(x, FLOOR_Y - 28.0 + shake, x, FLOOR_Y + shake,
            BlipColor { r: 0.22, g: 0.15, b: 0.10, a: 1.0 });
    }
    // Crates stacked at the edges — the only thing telling you where the
    // corner is before you are in it.
    for (x, n) in [(2.0f32, 2), (604.0, 3)] {
        for i in 0..n {
            let y = FLOOR_Y - 24.0 * (i + 1) as f32 + shake;
            blip.fill_rect(x, y, 34.0, 24.0, BlipColor { r: 0.42, g: 0.29, b: 0.16, a: 1.0 });
            blip.draw_rect(x, y, 34.0, 24.0, BlipColor { r: 0.22, g: 0.15, b: 0.08, a: 1.0 });
        }
    }
}

/// THE TEMPLE — night, moon, pillars, hanging banners.
fn draw_temple(blip: &Blip, shake: f32, t: f32) {
    blip.fill_rect(0.0, shake, WIN_W as f32, FLOOR_Y,
        BlipColor { r: 0.06, g: 0.07, b: 0.14, a: 1.0 });
    blip.fill_circle(120.0, 96.0 + shake, 34.0, BlipColor { r: 0.90, g: 0.92, b: 0.82, a: 0.95 });
    blip.fill_circle(108.0, 88.0 + shake, 30.0, BlipColor { r: 0.06, g: 0.07, b: 0.14, a: 1.0 });
    for i in 0..40 {
        let x = ((i * 79) % WIN_W as usize) as f32;
        let y = ((i * 43) % 220) as f32 + 10.0 + shake;
        let tw = 0.5 + 0.5 * ((t * 1.7 + i as f32).sin());
        blip.fill_rect(x, y, 1.6, 1.6, BlipColor { r: 0.9, g: 0.95, b: 1.0, a: 0.25 + 0.35 * tw });
    }
    // Pillars, and the roof line they hold up.
    let stone = BlipColor { r: 0.20, g: 0.19, b: 0.24, a: 1.0 };
    let stone_lit = BlipColor { r: 0.30, g: 0.29, b: 0.35, a: 1.0 };
    blip.fill_rect(0.0, 40.0 + shake, WIN_W as f32, 22.0, stone);
    for i in 0..5 {
        let x = 40.0 + i as f32 * 140.0;
        blip.fill_rect(x, 62.0 + shake, 26.0, FLOOR_Y - 62.0, stone);
        blip.fill_rect(x, 62.0 + shake, 6.0, FLOOR_Y - 62.0, stone_lit);
    }
    // Banners that move a little, so the place is not a photograph.
    for i in 0..3 {
        let x = 110.0 + i as f32 * 200.0;
        let sway = (t * 1.1 + i as f32).sin() * 3.0;
        blip.fill_rect(x + sway, 62.0 + shake, 22.0, 120.0,
            BlipColor { r: 0.62, g: 0.12, b: 0.16, a: 1.0 });
        blip.fill_rect(x + 8.0 + sway, 84.0 + shake, 6.0, 60.0,
            BlipColor { r: 0.95, g: 0.85, b: 0.5, a: 0.9 });
    }
}

// ---- fighters ------------------------------------------------------------

/// One fighter, posed for whatever they are doing.
///
/// Built from a spine: hips and shoulders at fixed fractions of the
/// current body height, limbs drawn as thick lines to wherever the
/// action puts them. Crouching lowers the whole frame rather than
/// shrinking it, which is why a crouch actually ducks under a jab
/// instead of merely looking like it does.
fn draw_fighter(blip: &Blip, g: &Game, i: usize) {
    let f = g.p[i];
    let a = f.arch();
    let body = rgb(a.color);
    let trim = rgb(a.trim);
    let shake = if g.shake > 0.0 { (g.shake * 60.0).sin() * g.shake * 30.0 } else { 0.0 };
    let ground = f.y + shake;

    if f.act == Act::Knockdown {
        // Flat out, head away from the attacker.
        let dir = -f.facing;
        blip.fill_rect(f.x - 50.0, ground - 22.0, 100.0, 20.0, body);
        blip.fill_rect(f.x - 7.0, ground - 25.0, 16.0, 5.0, trim);
        blip.fill_circle(f.x + dir * 55.0, ground - 13.0, 10.0,
            BlipColor { r: 0.78, g: 0.62, b: 0.48, a: 1.0 });
        return;
    }

    let h = f.height();
    let head_r = 11.0;
    let head_y = ground - h + head_r;
    let shoulder_y = ground - h * 0.74;
    let hip_y = ground - h * 0.42;
    let lean = match f.act {
        Act::Hitstun => -f.facing * 7.0,
        Act::Walk => f.facing * 2.0,
        Act::Attack => f.facing * 4.0,
        _ => 0.0,
    };
    let cx = f.x + lean;

    // Legs. Stance widens when crouching, and a jump tucks them up.
    let leg = BlipColor { r: body.r * 0.7, g: body.g * 0.7, b: body.b * 0.7, a: 1.0 };
    if f.airborne() {
        blip.draw_line_ex(cx - 6.0, hip_y, cx - 14.0, hip_y + 18.0, 7.0, leg);
        blip.draw_line_ex(cx + 6.0, hip_y, cx + 16.0, hip_y + 14.0, 7.0, leg);
    } else {
        let spread = if f.crouching() { 16.0 } else { 10.0 };
        blip.draw_line_ex(cx - 4.0, hip_y, f.x - spread, ground, 8.0, leg);
        blip.draw_line_ex(cx + 4.0, hip_y, f.x + spread, ground, 8.0, leg);
    }

    // Torso, belt, head. The trim colour is worn — headband and belt —
    // rather than being the head itself: a head painted in the fighter's
    // accent reads as a coloured ball, and the accent stops meaning
    // "this is who that is" the moment it is the biggest shape on them.
    blip.draw_line_ex(cx, hip_y, cx, shoulder_y, BODY_W * 0.72, body);
    blip.draw_line_ex(cx - BODY_W * 0.34, shoulder_y + 4.0, cx + BODY_W * 0.34, shoulder_y + 4.0,
        11.0, body);
    blip.fill_rect(cx - BODY_W * 0.36, hip_y - 3.0, BODY_W * 0.72, 6.0, trim);
    let skin = BlipColor { r: 0.78, g: 0.62, b: 0.48, a: 1.0 };
    blip.fill_circle(cx + f.facing * 2.0, head_y, head_r, skin);
    blip.fill_rect(cx + f.facing * 2.0 - head_r, head_y - head_r * 0.55, head_r * 2.0, 4.0, trim);
    // Which way they are facing, stated once and unmistakably.
    blip.fill_rect(cx + f.facing * 5.0, head_y - 1.0, 4.0, 3.0,
        BlipColor { r: 0.1, g: 0.1, b: 0.12, a: 1.0 });

    // Arms. Blocking puts both up across the body; attacking puts the
    // striking limb exactly where the hitbox is.
    let arm = body;
    match f.act {
        Act::Block => {
            let guard_y = if f.crouch_block { hip_y - 6.0 } else { shoulder_y + 6.0 };
            blip.draw_line_ex(cx, shoulder_y, cx + f.facing * 16.0, guard_y, 9.0, arm);
            blip.draw_line_ex(cx, shoulder_y + 8.0, cx + f.facing * 14.0, guard_y + 12.0, 9.0, arm);
            blip.draw_line_ex(cx + f.facing * 14.0, guard_y - 8.0, cx + f.facing * 14.0, guard_y + 16.0,
                4.0, trim);
        }
        Act::Attack => {
            let m = f.scaled(move_data(f.mv));
            let start = m.startup * F;
            let end = start + m.active * F;
            // Extension follows the move through its startup, holds
            // while it can hit, and snaps back during recovery — so the
            // picture is a readable tell rather than a pop.
            let ext = if f.t < start { (f.t / start.max(0.0001)).min(1.0) * 0.75 }
                      else if f.t <= end { 1.0 }
                      else { (1.0 - (f.t - end) / (m.recovery * F).max(0.0001)).max(0.0) };
            let tip_x = cx + f.facing * (BODY_W / 2.0 + m.reach * ext);
            let tip_y = ground - m.height;
            let limb_is_leg = matches!(f.mv, MoveId::Kick | MoveId::Sweep | MoveId::JumpKick)
                || (f.mv == MoveId::Special && a.special == Special::TalonKick);
            if limb_is_leg {
                blip.draw_line_ex(cx, hip_y, tip_x, tip_y, 9.0, leg);
                blip.fill_circle(tip_x, tip_y, 6.0, trim);
                blip.draw_line_ex(cx, shoulder_y, cx - f.facing * 12.0, shoulder_y + 16.0, 8.0, arm);
            } else {
                blip.draw_line_ex(cx, shoulder_y, tip_x, tip_y, 9.0, arm);
                blip.fill_circle(tip_x, tip_y, 6.0, trim);
                blip.draw_line_ex(cx, shoulder_y + 6.0, cx - f.facing * 12.0, shoulder_y + 20.0, 8.0, arm);
            }
        }
        _ => {
            // Idle breathes; the guard hand stays up because a fighter
            // who drops their hands reads as one who is not playing.
            let bob = (g.now * 4.0 + i as f32).sin() * 2.0;
            blip.draw_line_ex(cx, shoulder_y, cx + f.facing * 13.0, shoulder_y + 14.0 + bob, 8.0, arm);
            blip.draw_line_ex(cx, shoulder_y + 6.0, cx - f.facing * 11.0, shoulder_y + 22.0, 8.0, arm);
        }
    }

    // A flash while invulnerable on wakeup, so "you cannot hit me yet"
    // is visible rather than something the player has to infer.
    if invulnerable(&f) && ((g.now * 30.0) as i32) % 2 == 0 {
        blip.draw_rect(f.x - BODY_W / 2.0 - 2.0, ground - h - 2.0, BODY_W + 4.0, h + 4.0,
            BlipColor { r: 1.0, g: 1.0, b: 1.0, a: 0.5 });
    }
}

fn draw_fight(blip: &Blip, g: &Game) {
    for i in 0..2 { draw_fighter(blip, g, i); }

    for b in g.bolts.iter() {
        if !b.active { continue; }
        let c = rgb(FIGHTERS[g.p[b.owner].who].trim);
        blip.fill_glow_circle(b.x, b.y, 11.0, rgba(FIGHTERS[g.p[b.owner].who].trim, 0.35));
        blip.fill_circle(b.x, b.y, 7.0, c);
        blip.fill_circle(b.x - b.vx.signum() * 5.0, b.y, 4.0, BLIP_WHITE);
    }

    for s in g.hitspark.iter() {
        if s.2 <= 0.0 { continue; }
        let k = s.2 * 6.0;
        blip.fill_circle(s.0, s.1, 6.0 + 16.0 * k, BlipColor { r: 1.0, g: 0.95, b: 0.6, a: k.min(0.9) });
        blip.fill_circle(s.0, s.1, 3.0 + 8.0 * k, BLIP_WHITE);
    }
}

// ---- HUD -----------------------------------------------------------------

/// Health, rounds won, and the clock.
///
/// The bar drains toward the centre from each side, the way the cabinets
/// did it, because the thing a player needs at a glance is not "how much
/// have I got" but "who is ahead" — and two bars meeting in the middle
/// answer that without being read.
fn draw_hud(blip: &Blip, g: &Game) {
    let pad = 16.0;
    let bar_w = (WIN_W as f32 - pad * 3.0 - 56.0) / 2.0;
    let bar_h = 16.0;
    let y = 22.0;

    for i in 0..2 {
        let f = g.p[i];
        let frac = (f.health as f32 / f.arch().health as f32).clamp(0.0, 1.0);
        let x = if i == 0 { pad } else { WIN_W as f32 - pad - bar_w };
        blip.fill_rect(x - 2.0, y - 2.0, bar_w + 4.0, bar_h + 4.0,
            BlipColor { r: 0.10, g: 0.10, b: 0.12, a: 1.0 });
        blip.fill_rect(x, y, bar_w, bar_h, BlipColor { r: 0.35, g: 0.06, b: 0.06, a: 1.0 });
        let w = bar_w * frac;
        // Both bars empty away from the centre of the screen.
        let fx = if i == 0 { x + bar_w - w } else { x };
        let c = if frac > 0.45 { BlipColor { r: 0.95, g: 0.85, b: 0.2, a: 1.0 } }
                else if frac > 0.2 { BlipColor { r: 0.95, g: 0.55, b: 0.15, a: 1.0 } }
                else { BlipColor { r: 0.95, g: 0.25, b: 0.2, a: 1.0 } };
        blip.fill_rect(fx, y, w, bar_h, c);
        blip.draw_rect(x, y, bar_w, bar_h, BlipColor { r: 0.8, g: 0.8, b: 0.85, a: 0.7 });

        let name = f.arch().name;
        let nx = if i == 0 { pad } else { WIN_W as f32 - pad - text_w(name, 1.0) };
        blip.draw_text(name, nx, y + bar_h + 6.0, 1.0, BLIP_WHITE);

        // Round pips: a fighter needs two, so two lamps say everything.
        for r in 0..ROUNDS_TO_WIN {
            let px = if i == 0 { pad + r as f32 * 14.0 } else { WIN_W as f32 - pad - 10.0 - r as f32 * 14.0 };
            let lit = f.rounds > r;
            blip.fill_circle(px + 5.0, y + bar_h + 22.0, 5.0,
                if lit { BLIP_YELLOW } else { BlipColor { r: 0.25, g: 0.25, b: 0.28, a: 1.0 } });
        }
    }

    let secs = g.clock.max(0.0).ceil() as i32;
    let cx = WIN_W as f32 / 2.0;
    blip.fill_rect(cx - 26.0, y - 4.0, 52.0, bar_h + 12.0,
        BlipColor { r: 0.10, g: 0.10, b: 0.12, a: 1.0 });
    let tc = if secs <= 10 { BlipColor { r: 1.0, g: 0.35, b: 0.3, a: 1.0 } } else { BLIP_WHITE };
    let text = format!("{secs}");
    blip.draw_text(&text, cx - text_w(&text, 2.0) / 2.0, y + 2.0, 2.0, tc);

    let stage = if g.stage == 0 { "THE DOCKS" } else { "MOONLIT TEMPLE" };
    blip.fill_rect(cx - text_w(stage, 1.0) / 2.0 - 5.0, y + bar_h + 11.0,
        text_w(stage, 1.0) + 10.0, 12.0, BlipColor { r: 0.08, g: 0.08, b: 0.10, a: 0.75 });
    blip.draw_text(stage, cx - text_w(stage, 1.0) / 2.0, y + bar_h + 14.0, 1.0,
        BlipColor { r: 0.92, g: 0.92, b: 0.96, a: 1.0 });
}

fn draw_banner(blip: &Blip, g: &Game) {
    let cy = 150.0;
    match g.state {
        State::RoundIntro => {
            let r = format!("ROUND {}", g.round);
            blip.draw_centered(&r, cy, 4.0, BLIP_YELLOW);
            if g.phase.remaining() < 0.7 {
                blip.draw_centered("FIGHT!", cy + 46.0, 5.0, BLIP_WHITE);
            }
        }
        State::RoundEnd => blip.draw_centered(g.banner, cy, 5.0, BLIP_YELLOW),
        State::MatchEnd => blip.draw_centered(g.banner, cy, 4.0, BLIP_YELLOW),
        State::Over => {
            blip.draw_centered("GAME OVER", cy, 5.0, BlipColor { r: 0.95, g: 0.3, b: 0.3, a: 1.0 });
            let s = format!("SCORE {}", g.sess.score);
            blip.draw_centered(&s, cy + 50.0, 2.0, BLIP_WHITE);
            if !g.phase.active() { blip.draw_centered("PRESS FIRE", cy + 86.0, 2.0, BLIP_YELLOW); }
        }
        State::Won => {
            blip.draw_centered("CHAMPION", cy, 5.0, BLIP_YELLOW);
            let s = format!("SCORE {}", g.sess.score);
            blip.draw_centered(&s, cy + 50.0, 2.0, BLIP_WHITE);
            if !g.phase.active() { blip.draw_centered("PRESS FIRE", cy + 86.0, 2.0, BLIP_YELLOW); }
        }
        _ => {}
    }
}

// ---- front of house ------------------------------------------------------

fn draw_title(blip: &Blip) {
    blip.draw_centered("BRAWLER", 92.0, 7.0, BLIP_YELLOW);
    blip.draw_centered("TWO FIGHTERS ENTER", 150.0, 2.0, BLIP_WHITE);
    blip.draw_centered("ARROWS MOVE   HOLD BACK TO BLOCK", 206.0, 1.0,
        BlipColor { r: 0.75, g: 0.78, b: 0.85, a: 1.0 });
    blip.draw_centered("SPACE PUNCH   Z KICK", 226.0, 1.0,
        BlipColor { r: 0.75, g: 0.78, b: 0.85, a: 1.0 });
    blip.draw_centered("BOTH TOGETHER = SPECIAL", 246.0, 1.0,
        BlipColor { r: 0.95, g: 0.85, b: 0.4, a: 1.0 });
    blip.draw_centered("DOWN BLOCKS LOW   STANDING BLOCKS HIGH", 272.0, 1.0,
        BlipColor { r: 0.75, g: 0.78, b: 0.85, a: 1.0 });
    blip.draw_centered("PRESS FIRE", 320.0, 3.0, BLIP_WHITE);
}

fn draw_select(blip: &Blip, g: &Game) {
    blip.draw_centered("CHOOSE YOUR FIGHTER", 46.0, 3.0, BLIP_YELLOW);
    for (i, a) in FIGHTERS.iter().enumerate() {
        let x = 70.0 + i as f32 * 180.0;
        let picked = i == g.pick;
        let box_c = if picked { rgb(a.trim) } else { BlipColor { r: 0.3, g: 0.3, b: 0.35, a: 1.0 } };
        blip.draw_rect(x, 96.0, 150.0, 190.0, box_c);
        if picked { blip.draw_rect(x - 2.0, 94.0, 154.0, 194.0, box_c); }

        // The fighter, standing in their box at the size they fight at.
        let cx = x + 75.0;
        let ground = 262.0;
        let body = rgb(a.color);
        let trim = rgb(a.trim);
        blip.draw_line_ex(cx - 4.0, ground - STAND_H * 0.42, cx - 11.0, ground, 8.0,
            BlipColor { r: body.r * 0.7, g: body.g * 0.7, b: body.b * 0.7, a: 1.0 });
        blip.draw_line_ex(cx + 4.0, ground - STAND_H * 0.42, cx + 11.0, ground, 8.0,
            BlipColor { r: body.r * 0.7, g: body.g * 0.7, b: body.b * 0.7, a: 1.0 });
        blip.draw_line_ex(cx, ground - STAND_H * 0.42, cx, ground - STAND_H * 0.74, BODY_W * 0.72, body);
        blip.fill_circle(cx, ground - STAND_H + 11.0, 11.0, trim);
        blip.draw_line_ex(cx, ground - STAND_H * 0.74, cx + 13.0, ground - STAND_H * 0.74 + 14.0, 8.0, body);

        blip.draw_text(a.name, cx - text_w(a.name, 2.0) / 2.0, 104.0, 2.0,
            if picked { BLIP_WHITE } else { BlipColor { r: 0.6, g: 0.6, b: 0.66, a: 1.0 } });

        // The three numbers that actually differ, as bars — a player
        // choosing between archetypes needs the trade, not a biography.
        let stats = [("PWR", a.power / 1.4), ("SPD", a.walk / 150.0), ("HP ", a.health as f32 / 120.0)];
        for (r, (label, v)) in stats.iter().enumerate() {
            let sy = 272.0 + r as f32 * 14.0;
            blip.draw_text(label, x + 8.0, sy, 1.0, BlipColor { r: 0.7, g: 0.7, b: 0.8, a: 1.0 });
            blip.fill_rect(x + 40.0, sy, 100.0 * v.clamp(0.0, 1.0), 7.0, trim);
            blip.draw_rect(x + 40.0, sy, 100.0, 7.0, BlipColor { r: 0.3, g: 0.3, b: 0.36, a: 1.0 });
        }
    }
    let a = FIGHTERS[g.pick];
    let s = format!("SPECIAL: {}", a.special_name);
    blip.draw_centered(&s, 336.0, 2.0, rgb(a.trim));
    blip.draw_centered("PRESS FIRE TO START", 366.0, 2.0, BLIP_WHITE);
}
