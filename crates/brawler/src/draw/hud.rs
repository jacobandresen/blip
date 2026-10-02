//! Over the fight: health bars, the clock, and the banners that call a
//! round.

use super::*;

/// Health, rounds won and the clock. Bars drain toward the centre, so who is
/// ahead reads without reading.
pub(crate) fn draw_hud(blip: &Blip, g: &Game) {
    let pad = 16.0;
    // A portrait at the outer end of each bar.
    let face = 30.0;
    let inset = pad + face + 4.0;
    let bar_w = WIN_W as f32 / 2.0 - 36.0 - inset;
    let bar_h = 16.0;
    let y = 22.0;

    for i in 0..2 {
        let f = g.p[i];
        let frac = (f.health as f32 / f.arch().health as f32).clamp(0.0, 1.0);
        let x = if i == 0 { inset } else { WIN_W as f32 - inset - bar_w };
        let a = f.arch();
        let px = if i == 0 { pad } else { WIN_W as f32 - pad - face };
        let py = y - 7.0;
        blip.fill_rect(px - 2.0, py - 2.0, face + 4.0, face + 4.0, INK);
        blip.fill_rect(px, py, face, face, shade(rgb(a.color), 0.45));
        let fw = if i == 0 { 1.0 } else { -1.0 };
        // Wincing while the last blow is still draining off the bar.
        let mood = if f.health == 0 || g.ghost[i] > f.health as f32 + 0.5 { Mood::Hurt } else { Mood::Calm };
        let look = Look { bulk: a.bulk * 0.82, size: 0.82, wind: (0.0, 0.5), ..look_of(&a, 0.0) };
        let head = V(px + face / 2.0 + fw * 2.0, py + 13.0);
        draw_head(blip, &look, fw, head, V(head.0 - fw * 2.0, head.1 + 13.0), mood);
        // The neck runs out of the frame; the name plate squares it off.
        blip.fill_rect(px - 2.0, py + face, face + 4.0, 3.0, INK);
        blip.draw_rect(px, py, face, face, rgb(a.trim));
        blip.fill_rect(x - 2.0, y - 2.0, bar_w + 4.0, bar_h + 4.0,
            BlipColor { r: 0.10, g: 0.10, b: 0.12, a: 1.0 });
        blip.fill_rect(x, y, bar_w, bar_h, BlipColor { r: 0.35, g: 0.06, b: 0.06, a: 1.0 });
        let w = bar_w * frac;
        // Both bars empty away from the centre of the screen.
        let fx = if i == 0 { x + bar_w - w } else { x };
        let c = if frac > 0.45 { BlipColor { r: 0.95, g: 0.85, b: 0.2, a: 1.0 } }
                else if frac > 0.2 { BlipColor { r: 0.95, g: 0.55, b: 0.15, a: 1.0 } }
                else { BlipColor { r: 0.95, g: 0.25, b: 0.2, a: 1.0 } };
        // What the last blow cost, draining after it.
        let lost = bar_w * ((g.ghost[i] / f.arch().health as f32).clamp(0.0, 1.0) - frac).max(0.0);
        let gx = if i == 0 { fx - lost } else { fx + w };
        blip.fill_rect(gx, y, lost, bar_h, BlipColor { r: 1.0, g: 0.92, b: 0.86, a: 1.0 });
        // Nearly out, the bar beats.
        let c = if frac <= 0.2 && (g.now * 6.0) as i32 % 2 == 0 { blend(c, BLIP_WHITE, 0.35) } else { c };
        blip.fill_rect(fx, y, w, bar_h, c);
        // A lit top edge and a shaded foot: a bar of enamel, not a rectangle.
        blip.fill_rect(fx, y, w, 3.0, blend(c, BLIP_WHITE, 0.45));
        blip.fill_rect(fx, y + bar_h - 4.0, w, 4.0, shade(c, 0.78));
        blip.draw_rect(x, y, bar_w, bar_h, BlipColor { r: 0.8, g: 0.8, b: 0.85, a: 0.7 });

        let name = f.arch().name;
        let nx = if i == 0 { inset } else { WIN_W as f32 - inset - text_w(name, 1.0) };
        blip.draw_text(name, nx, y + bar_h + 6.0, 1.0, BLIP_WHITE);

        // Round pips: a fighter needs two, so two lamps say everything.
        for r in 0..ROUNDS_TO_WIN {
            // Inboard of the name, hard against the clock.
            let px = if i == 0 { x + bar_w - 10.0 - r as f32 * 14.0 } else { x + r as f32 * 14.0 };
            let lit = f.rounds > r;
            blip.fill_circle(px + 5.0, y + bar_h + 10.0, 5.5, INK);
            blip.fill_circle(px + 5.0, y + bar_h + 10.0, 4.0,
                if lit { BLIP_YELLOW } else { BlipColor { r: 0.25, g: 0.25, b: 0.28, a: 1.0 } });
        }
    }

    // The run's score, in a solo game: under the player's own name.
    if g.mode == Mode::Solo && !g.demo {
        blip.draw_text(&format!("{}", g.sess.score), inset, y + bar_h + 17.0, 1.0,
            BlipColor { r: 0.98, g: 0.90, b: 0.40, a: 1.0 });
    }

    let secs = g.clock.max(0.0).ceil() as i32;
    let cx = WIN_W as f32 / 2.0;
    blip.fill_rect(cx - 26.0, y - 4.0, 52.0, bar_h + 12.0,
        BlipColor { r: 0.10, g: 0.10, b: 0.12, a: 1.0 });
    let tc = if secs <= 10 { BlipColor { r: 1.0, g: 0.35, b: 0.3, a: 1.0 } } else { BLIP_WHITE };
    let text = format!("{secs}");
    blip.draw_text(&text, cx - text_w(&text, 2.0) / 2.0, y + 2.0, 2.0, tc);

    let stage = STAGE_NAMES[g.stage % STAGES];
    blip.fill_rect(cx - text_w(stage, 1.0) / 2.0 - 5.0, y + bar_h + 11.0,
        text_w(stage, 1.0) + 10.0, 12.0, BlipColor { r: 0.08, g: 0.08, b: 0.10, a: 0.75 });
    blip.draw_text(stage, cx - text_w(stage, 1.0) / 2.0, y + bar_h + 14.0, 1.0,
        PALE);
}

/// The words over the fight for the current state: the round call, the
/// result, the continue count.
pub(crate) fn draw_banner(blip: &Blip, g: &Game) {
    let cy = 150.0;
    match g.state {
        State::RoundIntro => {
            // One round each: this one decides it.
            let last = g.p.iter().all(|f| f.rounds == ROUNDS_TO_WIN - 1);
            let r = if last { "FINAL ROUND".to_string() } else { format!("ROUND {}", g.round) };
            stamp(blip, &r, cy, 4.0, BLIP_YELLOW);
            // How far up the ladder this is.
            if g.mode == Mode::Solo {
                let rung = format!("FIGHT {} OF {}", g.opponent_index + 1, RUNGS);
                stamp(blip, &rung, cy - 28.0, 2.0, PALE);
            }
            let left = g.phase.remaining();
            if left < 0.7 {
                // It lands big and settles.
                let sz = if left > 0.6 { 7.0 } else { 5.0 };
                stamp(blip, "FIGHT!", cy + 46.0 - (sz - 5.0) * 4.0, sz, BLIP_WHITE);
            }
        }
        State::RoundEnd => {
            stamp(blip, g.banner, cy, 5.0, BLIP_YELLOW);
            // The verdict lands once the slow motion is over.
            if !g.call.is_empty() && g.slow <= 0.0 {
                // (Above the banner: below it is where the fighters' heads are.)
                stamp(blip, g.call, cy - 84.0, 3.0, BLIP_WHITE);
            } else if g.slow <= 0.0 && g.result != RoundResult::Draw {
                // Whose round it was, in their colour.
                let a = g.p[(g.result == RoundResult::P2) as usize].arch();
                stamp(blip, &format!("{} WINS", a.name), cy - 84.0, 3.0, rgb(a.trim));
            }
            // The tally, once the fighters have settled.
            if g.bonus != [0, 0] && g.slow <= 0.0 {
                let grey = PALE;
                stamp(blip, &format!("TIME {}", g.bonus[0]), cy - 54.0, 2.0, grey);
                stamp(blip, &format!("VITAL {}", g.bonus[1]), cy - 34.0, 2.0, grey);
            }
        }
        State::MatchEnd => {
            stamp(blip, g.banner, cy, 4.0, BLIP_YELLOW);
            draw_quote(blip, g);
        }
        State::Challenger => {
            blip.fill_rect(0.0, cy - 30.0, WIN_W as f32, 130.0,
                BlipColor { r: 0.02, g: 0.02, b: 0.05, a: 0.82 });
            // It flashes, as it always has.
            let c = if (g.now * 8.0) as i32 % 2 == 0 { BLIP_YELLOW } else { BLIP_WHITE };
            stamp(blip, "HERE COMES", cy - 12.0, 4.0, c);
            stamp(blip, "A NEW CHALLENGER", cy + 34.0, 4.0, c);
        }
        State::Continue => {
            blip.fill_rect(0.0, cy - 30.0, WIN_W as f32, 150.0,
                BlipColor { r: 0.02, g: 0.02, b: 0.05, a: 0.78 });
            stamp(blip, "CONTINUE", cy - 14.0, 4.0, BLIP_YELLOW);
            // The count lands big each second and settles.
            let left = g.phase.remaining();
            let sz = if left.fract() > 0.82 { 9.0 } else { 7.0 };
            let n = format!("{}", left.ceil() as i32);
            stamp(blip, &n, cy + 26.0 - (sz - 7.0) * 4.0, sz, BLIP_WHITE);
            stamp(blip, "ONE COIN - PRESS PUNCH TO FIGHT AGAIN", cy + 96.0, 2.0, PALE);
        }
        State::Over if g.mode == Mode::Versus => {
            // Two people played a match; one of them won it. "GAME
            // OVER" and a score belong to a run against the ladder and
            // say nothing at all about what just happened here.
            let (a, b) = (g.p[0].rounds, g.p[1].rounds);
            let (text, col) = if a > b {
                ("PLAYER 1 WINS", BlipColor { r: 1.0, g: 0.35, b: 0.35, a: 1.0 })
            } else if b > a {
                ("PLAYER 2 WINS", BlipColor { r: 0.40, g: 0.72, b: 1.0, a: 1.0 })
            } else {
                ("DRAW GAME", BLIP_YELLOW)
            };
            // On a plate, because the result sits over two fighters
            // still standing on the stage and yellow on a sunset is
            // not text, it is a smudge.
            blip.fill_rect(0.0, cy - 18.0, WIN_W as f32, 122.0,
                BlipColor { r: 0.02, g: 0.02, b: 0.05, a: 0.78 });
            blip.draw_centered(text, cy, 4.0, col);
            blip.draw_centered(&format!("{a} - {b}"), cy + 46.0, 3.0, BLIP_WHITE);
            if !g.phase.active() {
                blip.draw_centered("PRESS TO PLAY AGAIN", cy + 84.0, 2.0, BLIP_YELLOW);
            }
        }
        State::Over => {
            stamp(blip, "GAME OVER", cy, 5.0, BlipColor { r: 0.95, g: 0.3, b: 0.3, a: 1.0 });
            let s = format!("SCORE {}", g.sess.score);
            stamp(blip, &s, cy + 50.0, 2.0, BLIP_WHITE);
            if !g.phase.active() { blip.draw_centered("PRESS FIRE", cy + 86.0, 2.0, BLIP_YELLOW); }
        }
        State::Won => {
            stamp(blip, "CHAMPION", cy, 5.0, BLIP_YELLOW);
            let s = format!("SCORE {}", g.sess.score);
            stamp(blip, &s, cy + 50.0, 2.0, BLIP_WHITE);
            if !g.phase.active() { blip.draw_centered("PRESS FIRE", cy + 86.0, 2.0, BLIP_YELLOW); }
        }
        _ => {}
    }
}
