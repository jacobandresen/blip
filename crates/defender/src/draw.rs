//! Drawing: the saucer and its death ray, the boss, chips of broken aliens, the night sky over the town, the play field with its
//! HUD, and the title and level-clear screens.

use super::*;

/// The saucer at `x`,`y`, `w` by `h`: its body, then its rim lights, yellow and
/// white by turns, turned `frame` steps round the disc.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_saucer(blip: &Blip, saucer: &Texture2D, x: f32, y: f32, w: f32, h: f32, frame: usize, tint: BlipColor) {
    blip.draw_texture_tinted(saucer, x, y, w, h, tint);
    let (sx, sy) = (w / UFO_W, h / UFO_H);
    let (cx, cy, rx, ry) = (UFO_W / 2.0, UFO_H * 0.62, (UFO_W / 2.0 - 1.0) * 0.88, UFO_H * 0.30 * 0.88);
    for i in 0..UFO_N_LIGHTS {
        let k = i + frame;
        let a = k as f32 / UFO_N_LIGHTS as f32 * std::f32::consts::TAU;
        let (lx, ly) = ((cx + a.cos() * rx).round(), (cy + a.sin() * ry).round());
        let light = if k.is_multiple_of(2) { BlipColor::new(1.0, 0.9, 0.24, tint.a) } else { BlipColor::new(1.0, 1.0, 1.0, tint.a) };
        blip.fill_rect(x + lx * sx, y + (ly - 1.0) * sy, sx, 2.0 * sy, light);
    }
}

pub(super) fn draw_ufo(blip: &Blip, g: &Game, saucer: &Texture2D) {
    if g.ufo_active {
        match g.ufo_mode {
            UfoMode::Flying => {
                draw_saucer(blip, saucer, g.ufo_x, UFO_Y, UFO_W, UFO_H, g.ufo_frame, BLIP_WHITE);
            }
            UfoMode::Tracking => {
                draw_saucer(blip, saucer, g.ufo_x, UFO_Y, UFO_W, UFO_H, g.ufo_frame, BLIP_WHITE);
                // A faint lock-on reticle down toward the player — a quiet
                // "it's noticed you" tell, well before the loud charge-up.
                let cx = g.ufo_x + UFO_W / 2.0;
                let cy = UFO_Y + UFO_H;
                let px = g.player_x + ALIEN_W as f32 / 2.0;
                let py = (GROUND_Y - 28) as f32;
                blip.draw_line(cx, cy, px, py, BlipColor { r: 1.0, g: 0.3, b: 0.3, a: 0.18 });
            }
            UfoMode::Charging => {
                // The whole ship shakes as it powers up.
                let jx = (rand() % 5) as f32 - 2.0;
                let jy = (rand() % 3) as f32 - 1.0;
                draw_saucer(blip, saucer, g.ufo_x + jx, UFO_Y + jy, UFO_W, UFO_H, g.ufo_frame, BLIP_WHITE);

                let progress = 1.0 - (g.ufo_charge_timer.remaining() / UFO_CHARGE_SECS).max(0.0);
                let cx = g.ufo_x + UFO_W / 2.0;
                let cy = UFO_Y + UFO_H;

                // Growing charge glow beneath it, brightening as it nears firing.
                let glow_r = 8.0 + progress * 16.0;
                let glow_c = BlipColor {
                    r: 1.0, g: 0.25 + progress * 0.5, b: 0.15, a: 0.6 + progress * 0.4,
                };
                blip.fill_glow_circle(cx, cy, glow_r, glow_c);

                // Warning line down to where the beam will land: always
                // visible, pulsing faster and brighter as it nears firing.
                let pulse_rate = 3.0 + progress * 10.0;
                let pulse = 0.5 + 0.5 * (progress * pulse_rate * std::f32::consts::TAU).sin();
                let warn_c = BlipColor {
                    r: 1.0, g: 0.2, b: 0.2, a: 0.5 + progress * 0.4 + pulse * 0.15,
                };
                blip.draw_glow_line(cx, cy, cx, GROUND_Y as f32, warn_c);
            }
            UfoMode::Firing => {
                let cx = g.ufo_laser_x + UFO_W / 2.0;
                let fade = (g.ufo_fire_timer.remaining() / UFO_FIRE_SECS).clamp(0.0, 1.0);
                // Full-width flash for the first instant — the dramatic hit.
                if fade > 0.7 {
                    blip.fill_rect(
                        0.0, PLAY_Y as f32, WIN_W as f32, (GROUND_Y - PLAY_Y) as f32,
                        BlipColor { r: 1.0, g: 1.0, b: 1.0, a: (fade - 0.7) * 0.6 },
                    );
                }
                // The beam: a wide hot core inside a wider red-hot glow.
                let beam_c = BlipColor { r: 1.0, g: 0.3 + fade * 0.4, b: 0.3, a: 1.0 };
                for off in [-10.0_f32, -5.0, 0.0, 5.0, 10.0] {
                    blip.draw_glow_line(cx + off, UFO_Y, cx + off, GROUND_Y as f32, beam_c);
                }
                blip.draw_line_ex(cx - 3.0, UFO_Y, cx - 3.0, GROUND_Y as f32, 6.0, BLIP_WHITE);
                blip.draw_line_ex(cx + 3.0, UFO_Y, cx + 3.0, GROUND_Y as f32, 6.0, BLIP_WHITE);
            }
        }
    }
    if g.ufo_score_timer.active() {
        let text = format!("{} PTS", g.ufo_score);
        blip.draw_centered_outlined(&text, (WIN_W / 2) as f32, UFO_Y, 2.0, BLIP_GOLD, BLIP_INK);
    }
}

pub(super) fn draw_chips(blip: &Blip, g: &Game) {
    for c in &g.chips {
        let a = (c.ttl / CHIP_SECS).min(1.0);
        let s = 2.0 + 2.0 * a;
        let (sn, cs) = c.turn.sin_cos();
        let p = |u: f32, v: f32| (c.x + u * cs - v * sn, c.y + u * sn + v * cs);
        blip.fill_quad(p(-s, -s), p(s, -s), p(s, s), p(-s, s), BlipColor { a, ..c.c });
    }
}

/// The mothership and its health bar, drawn under the ship so it is in the
/// same glance: without it, slowly winning looks like doing nothing.
pub(super) fn draw_boss(blip: &Blip, g: &Game, saucer: &Texture2D) {
    if !g.boss_active { return; }
    let (x, y) = (g.boss_x, g.boss_y());

    // White for a moment on every hit — at this size a small puff at the
    // impact point is easy to lose against the hull.
    let tint = if g.boss_flash > 0.0 {
        BlipColor { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }
    } else {
        // Reddens as it goes: a second reading of the same number the bar
        // carries, for the player whose eyes are on the ship.
        let hurt = 1.0 - (g.boss_hp as f32 / g.boss_hp_max.max(1) as f32);
        BlipColor { r: 1.0, g: 1.0 - hurt * 0.55, b: 1.0 - hurt * 0.65, a: 1.0 }
    };
    draw_saucer(blip, saucer, x, y, BOSS_W, BOSS_H, g.ufo_frame, tint);

    let frac = (g.boss_hp as f32 / g.boss_hp_max.max(1) as f32).clamp(0.0, 1.0);
    let (bw, bh) = (BOSS_W, 4.0);
    let by = y + BOSS_H + 4.0;
    blip.fill_rect(x, by, bw, bh, BlipColor { r: 0.25, g: 0.05, b: 0.05, a: 1.0 });
    let bar = if frac > 0.5 {
        BlipColor { r: 0.35, g: 0.95, b: 0.4, a: 1.0 }
    } else if frac > 0.22 {
        BlipColor { r: 1.0, g: 0.82, b: 0.2, a: 1.0 }
    } else {
        BlipColor { r: 1.0, g: 0.3, b: 0.25, a: 1.0 }
    };
    blip.fill_rect(x, by, bw * frac, bh, bar);
}

/// A small integer hash: the same stars and buildings every frame, without touching the game's random numbers.
pub(super) fn hash(n: u32) -> u32 {
    let mut x = n.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^ (x >> 12)
}

/// The night the cannon defends: a deep blue sky lightening toward the horizon, twinkling stars, a cream moon and a
/// town of rooftops lit window by window, on a strip of grass.
pub(super) fn draw_backdrop(blip: &Blip, now: f32) {
    let (top, bottom) = (PLAY_Y as f32, GROUND_Y as f32);
    blip.fill_bands(0.0, top, WIN_W as f32, bottom - top, 8, |k| blip::blend(BLIP_NIGHT, BLIP_DUSK, k * k));
    for i in 0..70u32 {
        let h = hash(i);
        let (x, y) = ((h % WIN_W as u32) as f32, PLAY_Y as f32 + ((h >> 10) % (GROUND_Y as u32 - 90 - PLAY_Y as u32)) as f32);
        let tw = 0.35 + 0.35 * (now * (0.8 + (h >> 20 & 7) as f32 * 0.3) + i as f32).sin();
        let s = if h >> 24 & 3 == 0 { 2.0 } else { 1.0 };
        blip.fill_rect(x, y, s, s, BlipColor { a: tw * 0.8, ..BLIP_CREAM });
    }
    // A crescent: the cream moon with the sky's colour bitten out of it.
    blip.fill_circle(56.0, PLAY_Y as f32 + 52.0, 19.0, BLIP_INK);
    blip.fill_circle(56.0, PLAY_Y as f32 + 52.0, 18.0, BLIP_CREAM);
    blip.fill_circle(63.0, PLAY_Y as f32 + 47.0, 16.0, BLIP_NIGHT);
    let mut x = 0.0;
    let mut i = 0u32;
    let house = blip::blend(BLIP_DUSK, BLIP_INK, 0.45);
    while x < WIN_W as f32 {
        let h = hash(1000 + i);
        let (w, tall) = (18.0 + (h % 20) as f32, 14.0 + ((h >> 8) % 34) as f32);
        let top = GROUND_Y as f32 - 1.0 - tall;
        blip.fill_rect(x, top, w - 2.0, tall, house);
        blip.fill_rect(x, top, w - 2.0, 2.0, blip::blend(house, BLIP_PLUM, 0.35));
        for wy in (0..(tall as i32 - 6)).step_by(7) {
            for wx in (3..(w as i32 - 5)).step_by(6) {
                let lit = hash(h ^ (wy as u32 * 31 + wx as u32)).is_multiple_of(5);
                if lit { blip.fill_rect(x + wx as f32, top + 4.0 + wy as f32, 2.0, 3.0, BlipColor { a: 0.8, ..BLIP_GOLD }); }
            }
        }
        x += w;
        i += 1;
    }
    // The grass the cannon rolls on, below the ground line.
    blip.fill_rect(0.0, GROUND_Y as f32 + 1.0, WIN_W as f32, (WIN_H - GROUND_Y) as f32, blip::blend(BLIP_LEAF, BLIP_INK, 0.55));
}

/// The pictures, loaded once.
pub(super) struct Art {
    pub(super) player: Texture2D,
    /// Each kind of alien in its two steps.
    pub(super) alien: [[Texture2D; 2]; 3],
    pub(super) explosion: Texture2D,
    pub(super) shield: Texture2D,
    pub(super) bullet: Texture2D,
    pub(super) saucer: Texture2D,
}

impl Art {
    pub(super) fn load() -> Art {
        Art {
            player: load_png(blip::png!("player_ship")),
            alien: [
                [load_png(blip::png!("alien_squid_a")), load_png(blip::png!("alien_squid_b"))],
                [load_png(blip::png!("alien_crab_a")), load_png(blip::png!("alien_crab_b"))],
                [load_png(blip::png!("alien_octopus_a")), load_png(blip::png!("alien_octopus_b"))],
            ],
            explosion: load_png(blip::png!("explosion")),
            shield: load_png(blip::png!("shield_block")),
            bullet: load_png(blip::png!("bullet")),
            saucer: load_png(blip::png!("ufo_saucer")),
        }
    }
}

/// The play field: the night, the ground, shields, the invaders, the boss and the saucer, the shots, the cannon, and the HUD.
pub(super) fn draw_play(blip: &Blip, g: &Game, art: &Art) {
    let Art { player, alien, explosion, shield, bullet, saucer } = art;
    draw_backdrop(blip, blip::now());
    // The ground line pulses red while the invaders are close to landing.
    let lowest = g.aliens.iter().filter(|a| a.alive).map(|a| a.y + ALIEN_H as f32).fold(0.0, f32::max);
    let ground = if g.state == State::Play && lowest > (GROUND_Y - LANDING_WARN) as f32
        && blip::blink(4.0) { BLIP_HEART } else { BLIP_LEAF };
    blip.fill_rect(0.0, GROUND_Y as f32 - 1.0, WIN_W as f32, 3.0, ground);
    blip.fill_rect(0.0, GROUND_Y as f32 + 2.0, WIN_W as f32, 1.0, BLIP_INK);

    for (s, r, c) in blocks() {
        if !g.shields[s].alive[r][c] { continue; }
        let (bx, by) = shield_block_xy(&g.shields[s], r, c);
        blip.draw_texture(shield, bx, by, SHIELD_BLOCK as f32, SHIELD_BLOCK as f32);
    }

    let now = blip::now();
    for (i, a) in g.aliens.iter().enumerate() {
        if !a.alive { continue; }
        // The one about to dive shakes and flashes white: the tell.
        let (shake, tint) = match g.dive_pick {
            Some((idx, _)) if idx == i => ((now * 60.0).sin() * 3.0,
                if (now * 14.0) as i32 % 2 == 0 { BLIP_WHITE } else { alien_color(a.kind) }),
            _ => (0.0, alien_color(a.kind)),
        };
        blip.draw_texture_tinted(
            &alien[a.kind][a.anim as usize],
            a.x + shake, a.y, ALIEN_W as f32, ALIEN_H as f32, tint,
        );
    }
    if let Some(d) = g.diver {
        let frame = (d.t * 12.0) as usize % 2;
        blip.draw_texture_tinted(&alien[d.kind][frame], d.x, d.y, ALIEN_W as f32, ALIEN_H as f32, BLIP_WHITE);
    }

    draw_boss(blip, g, saucer);

    // Hidden through the explosion, then fades back in for the respawn.
    let ship_alpha = if g.state == State::Dead {
        // dead_pause_total, not DEAD_PAUSE: the invasion death has no respawn fade.
        let total = g.dead_pause_total;
        let elapsed = (total - g.dead_timer.remaining()).clamp(0.0, total);
        if elapsed <= DEATH_EXPLOSION_PHASE || total <= DEATH_EXPLOSION_PHASE {
            0.0
        } else {
            ((elapsed - DEATH_EXPLOSION_PHASE) / (total - DEATH_EXPLOSION_PHASE)).clamp(0.0, 1.0)
        }
    } else {
        1.0
    };

    if ship_alpha > 0.0 {
        // Exhaust puffs trail a moving cannon.
        let speed = (g.player_vx / PLAYER_SPEED).clamp(-1.0, 1.0);
        if speed.abs() > 0.2 {
            let back = if speed > 0.0 { g.player_x + 2.0 } else { g.player_x + ALIEN_W as f32 - 2.0 };
            for k in 1..=3 {
                let px = back - speed * k as f32 * 5.0;
                let flick = 0.6 + 0.4 * (now * 40.0 + k as f32).sin();
                blip.fill_circle(px, GROUND_Y as f32 - 6.0, 3.0 - k as f32 * 0.6,
                    BlipColor { r: 1.0, g: 0.6, b: 0.2, a: 0.5 * flick * speed.abs() * ship_alpha / k as f32 });
            }
        }
        blip.draw_texture_tinted(
            player, g.player_x, (GROUND_Y - 28) as f32, ALIEN_W as f32, 28.0,
            BlipColor { r: 1.0, g: 1.0, b: 1.0, a: ship_alpha },
        );
    }

    for b in g.bullets.iter().filter(|b| b.active && b.player) {
        // A fading tail under the shot, and a flash at the muzzle as it leaves.
        for k in 1..=3 {
            blip.fill_rect(b.x + 1.0, b.y + 12.0 + (k - 1) as f32 * 6.0, 2.0, 6.0, BlipColor { r: 0.6, g: 0.9, b: 1.0, a: 0.35 / k as f32 });
        }
        if b.y > (GROUND_Y - 60) as f32 {
            blip.fill_glow_circle(b.x + 2.0, (GROUND_Y - 28) as f32, 8.0, BlipColor { r: 0.8, g: 0.95, b: 1.0, a: 0.6 });
        }
        blip.draw_texture_tinted(bullet, b.x, b.y, 4.0, 12.0, BLIP_CREAM);
    }

    for e in g.explosions.iter() {
        if !e.active { continue; }
        let alpha = (e.ttl / e.max_ttl).clamp(0.0, 1.0);
        let tc = BlipColor { r: 1.0, g: 1.0, b: 1.0, a: alpha };
        // (e.x, e.y) is the top-left of a plain ALIEN_W box at scale 1; grow
        // outward from that box's centre for bigger bursts so callers don't
        // need to know about scale.
        let sz = ALIEN_W as f32 * e.scale;
        let grow = (sz - ALIEN_W as f32) / 2.0;
        blip.draw_texture_tinted(
            explosion,
            e.x - grow, e.y - grow, sz, sz, tc,
        );
    }

    draw_ufo(blip, g, saucer);
    draw_chips(blip, g);
    g.fx.draw(blip);
    // Bombs last, so a kill's burst and score never hide one falling from the formation.
    for b in g.bullets.iter().filter(|b| b.active && !b.player) {
        blip.fill_glow_circle(b.x + 2.0, b.y + 8.0, 7.0, BlipColor { r: 1.0, g: 0.5, b: 0.1, a: 0.35 });
        blip.draw_texture_tinted(bullet, b.x, b.y, 4.0, 12.0, BLIP_ORANGE);
    }
    blip.draw_hud(g.sess.score, g.sess.lives);
    if g.streak.count > 0 {
        let label = streak_label(&g.streak);
        let x = blip.text_cx(&label, 2) as f32;
        blip.draw_text_outlined(&label, x, (GROUND_Y + 10) as f32, 2.0, BLIP_GOLD, BLIP_INK);
    } else if g.lost_run.1 > 0.0 {
        let label = format!("{} IN A ROW", g.lost_run.0);
        let x = blip.text_cx(&label, 2) as f32;
        blip.draw_text_outlined(&label, x, (GROUND_Y + 10) as f32, 2.0, BLIP_HEART, BLIP_INK);
    }
}

/// How long a broken run stays shown, seconds.
pub(super) const LOST_RUN_SECS: f32 = 0.8;

/// The strip under the ground line: the run of hits, the multiple it earns, and how many more raise it.
pub(super) fn streak_label(s: &blip::Streak) -> String {
    let (n, times) = (s.count, s.multiple());
    match s.to_next() {
        None => format!("{n} IN A ROW  X{times}"),
        Some(to_next) => format!("{n} IN A ROW  X{times}  {to_next} FOR X{}", times + 1),
    }
}

pub(super) fn draw_title(blip: &Blip, alien: &[[Texture2D; 2]; 3]) {
    draw_backdrop(blip, blip::now());
    blip.draw_logo("DEFENDER", (WIN_W / 2) as f32, (WIN_H / 5 + 14) as f32, 6.0, alien_color(0));
    // The title is 6 x 7 = 42px tall: the high score clears it by a real margin.
    blip.draw_attract_bar();

    let dw = (ALIEN_W / 2) as f32;
    let dh = (ALIEN_H / 2) as f32;
    let ax = (blip.text_cx("30 PTS", 2) - ALIEN_W / 2 - 8) as f32;
    let voff = ((7 * 2 - ALIEN_H / 2) / 2) as f32;

    let row0 = (WIN_H / 2 - 40) as f32;
    let row1 = (WIN_H / 2 - 20) as f32;
    let row2 = (WIN_H / 2) as f32;

    blip.draw_panel((WIN_W / 2 - 110) as f32, row0 - 16.0, 220.0, 84.0, 1.0);
    // The invaders' two-frame shuffle, one beat every half second.
    let beat = (blip::now() * 2.0) as i64;
    let frame = beat.rem_euclid(2) as usize;
    blip.draw_texture_tinted(&alien[0][frame], ax, row0 + voff, dw, dh, alien_color(0));
    blip.draw_texture_tinted(&alien[1][frame], ax, row1 + voff, dw, dh, alien_color(1));
    blip.draw_texture_tinted(&alien[2][frame], ax, row2 + voff, dw, dh, alien_color(2));

    blip.draw_centered("30 PTS",        row0,                 2.0, alien_color(0));
    blip.draw_centered("20 PTS",        row1,                 2.0, alien_color(1));
    blip.draw_centered("10 PTS",        row2,                 2.0, alien_color(2));
    blip.draw_start_prompt((WIN_H * 2 / 3) as f32);
    // A patrol steps across the foot of the screen, a stride per beat.
    let sweep = 5i64;
    let pos = beat.rem_euclid(sweep * 2);
    let step = if pos < sweep { pos } else { sweep * 2 - pos - 1 };
    for i in 0..6i64 {
        let kind = (i % 3) as usize;
        let x = 90.0 + (i * 52) as f32 + step as f32 * 12.0;
        blip.draw_texture_tinted(&alien[kind][frame], x, (WIN_H * 5 / 6) as f32, dw, dh, alien_color(kind));
    }
}

pub(super) fn draw_clear(blip: &Blip, level: i32, t: f32) {
    // Fireworks over the town the cannon kept safe.
    draw_backdrop(blip, blip::now());
    blip::draw_fireworks(blip, t, (60.0, (PLAY_Y + 60) as f32, (WIN_W - 120) as f32, 220.0),
        &[alien_color(0), BLIP_SEA, BLIP_LEAF, BLIP_GOLD]);
    let buf = format!("LEVEL {}", level);
    blip.draw_logo("WAVE CLEAR", (WIN_W / 2) as f32, (WIN_H / 3) as f32, 5.0, BLIP_SEA);
    blip.draw_message(&buf, (WIN_W / 2) as f32, (WIN_H / 2 + 10) as f32, 3.0, BLIP_GOLD, 1.0);
}
