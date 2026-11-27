//! Drawing: the sea, boats, islands, clouds and flak, the planes, the boss, the HUD, and the title and end screens.

use super::*;

/// The ocean: a gradient lighter and hazier toward the top (distance), two
/// octaves of wave lines (swell and chop) scrolling with `sea_scroll`, sun
/// glitter in a diagonal band, and each cloud's shadow drifting on the water.
pub(super) fn draw_sea(blip: &Blip, g: &Game) {
    // The blip look's sea: bright and clear, so the ink-rimmed planes stand out on it.
    let far  = (0.30, 0.58, 0.78); // hazy, sun-bleached blue at the horizon (top)
    let near = (0.16, 0.38, 0.64); // deeper blue close up (bottom)
    blip.fill_bands(0.0, 0.0, WIN_W as f32, WIN_H as f32, 14, |t| {
        BlipColor::new(far.0 + (near.0 - far.0) * t, far.1 + (near.1 - far.1) * t, far.2 + (near.2 - far.2) * t, 1.0)
    });

    let spacing = 30.0;
    let rows = (WIN_H as f32 / spacing) as i32 + 2;
    let phase = g.sea_scroll % spacing;
    let segs = 12;
    let seg_w = WIN_W as f32 / segs as f32;
    for i in -1..rows {
        let y = i as f32 * spacing + phase;
        let depth = (y / WIN_H as f32).clamp(0.0, 1.0);
        // Long swell + short chop layered together (two frequencies, not
        // one) — a single sine reads as a uniform ripple; two makes it look
        // like real, irregular open water.
        // Chunky wavelets, a crest and its two shoulders, in staggered rows: pixel-art water.
        let alpha = 0.35 + depth * 0.30;
        let color = BlipColor { a: alpha, ..BLIP_CREAM };
        for s in 0..segs {
            let x0 = s as f32 * seg_w + if i % 2 == 0 { 0.0 } else { seg_w / 2.0 };
            let wob = ((x0 * 0.045 + y * 0.05).sin() * 3.2).round();
            if (s + i as usize * 7).is_multiple_of(3) { continue; }
            let (cx, cy) = (x0.round(), (y + wob).round());
            blip.fill_rect(cx - 4.0, cy, 8.0, 2.0, color);
            blip.fill_rect(cx - 8.0, cy + 2.0, 4.0, 2.0, color);
            blip.fill_rect(cx + 4.0, cy + 2.0, 4.0, 2.0, color);
        }
    }

    // Sun-glitter: a diagonal band of dense, bright glints scrolling with the
    // water (real specular sun-glint off wavelets), plus a light scatter of
    // dim ones everywhere else so the whole sea doesn't look dead outside it.
    for i in 0..26 {
        let seed = i as f32 * 53.7;
        let x = (seed * 13.0) % WIN_W as f32;
        let y = (seed * 29.0 + g.sea_scroll) % (WIN_H as f32 + 40.0) - 20.0;
        let band_center = x * 0.62 + 40.0;
        let dist = (y - band_center).abs();
        let in_band = dist < 60.0;
        let twinkle = (g.sea_scroll * 0.05 + seed).sin() * 0.5 + 0.5;
        let (r, a) = if in_band {
            (1.9 - dist / 60.0, (0.55 + twinkle * 0.35) * (1.0 - dist / 60.0))
        } else {
            (1.1, 0.20 + twinkle * 0.10)
        };
        if r > 0.2 {
            let s = (r * 1.6).round().max(1.0);
            blip.fill_rect(x.round(), y.round(), s, s, BlipColor { a, ..BLIP_CREAM });
        }
    }

    // Cloud shadows on the water — a soft dark smear beneath each cloud,
    // offset the same direction as the plane shadows so the whole scene
    // reads as lit from one consistent direction.
    for c in g.clouds.iter() {
        let sx = c.x + PLANE_SHADOW_DX * 1.5;
        let sy = c.y + PLANE_SHADOW_DY * 1.5;
        blip.fill_circle(sx, sy, c.r * 0.75, BlipColor { a: 0.14, ..BLIP_INK });
    }
}

/// The sprite again, dark and translucent, offset toward the sea
/// (PLANE_SHADOW_DX / DY). Drawn after sea and clouds, before the entity.
pub(super) fn draw_shadow(tex: &Texture2D, x: f32, y: f32, w: f32, h: f32, rotation: f32) {
    draw_texture_ex(tex, x + PLANE_SHADOW_DX, y + PLANE_SHADOW_DY, BlipColor { a: 0.32, ..BLIP_INK }, DrawTextureParams {
        dest_size: Some(vec2(w, h)),
        rotation,
        ..Default::default()
    });
}

pub(super) fn draw_boats(blip: &Blip, g: &Game, boat_tex: &Texture2D) {
    for b in pool_iter(&g.boats) {
        // Rock with the swell: a slow bob (vertical rise/fall) plus a small
        // roll (side-to-side tilt) derived from it, like a hull actually
        // riding waves instead of sliding across flat glass.
        let bob = (g.sea_scroll * 0.05 + b.bob_phase).sin();
        let chop = (g.sea_scroll * 0.13 + b.bob_phase * 1.7).sin();
        let y_off = bob * 1.8 + chop * 0.6;
        let roll = bob * 0.11 + b.vx.signum() * 0.05;

        // V-shaped wake fanning out from the stern, fading with distance —
        // drawn fresh from the boat's current drift each frame rather than
        // baked into the sprite, so it always matches its heading.
        let stern_x = b.x + BOAT_W as f32 / 2.0;
        let stern_y = b.y + BOAT_H as f32 * 0.8 + y_off;
        let drift = (-b.vx * 0.06).clamp(-1.5, 1.5);
        for k in 1..=6 {
            let t = k as f32;
            let back = t * 4.2;
            let spread = t * 2.4;
            let alpha = (0.28 - t * 0.042).max(0.0);
            if alpha <= 0.0 { continue; }
            let wy = stern_y - back;
            let lx = stern_x - spread + drift * back * 0.25;
            let rx = stern_x + spread + drift * back * 0.25;
            blip.fill_circle(lx, wy, 1.1, BlipColor::new(0.85, 0.93, 1.0, alpha));
            blip.fill_circle(rx, wy, 1.1, BlipColor::new(0.85, 0.93, 1.0, alpha));
        }

        draw_texture_ex(boat_tex, b.x, b.y + y_off, BLIP_WHITE, DrawTextureParams {
            dest_size: Some(vec2(BOAT_W as f32, BOAT_H as f32)),
            flip_x: b.vx < 0.0,
            rotation: roll,
            ..Default::default()
        });
    }
}

/// The sky layer, between sea and planes: noise-generated cumulus
/// (gfx.rs cloud_sprite()) in 3 variants.
pub(super) fn draw_clouds(blip: &Blip, g: &Game, cloud_tex: &[Texture2D; 3]) {
    for c in g.clouds.iter() {
        let tex = &cloud_tex[c.variant as usize % cloud_tex.len()];
        // Stretched noticeably wider than tall — long cloud banks rather than
        // round puffs, big enough for a plane to actually vanish into one.
        let w = c.r * 3.6;
        let h = c.r * 1.7;
        blip.draw_texture(tex, c.x - w / 2.0, c.y - h / 2.0, w, h);
    }
}

/// Is (x, y), an entity's centre, inside a cloud's visible body? An ellipse
/// shrunk a little, so a plane must be well under it to count.
pub(super) fn point_in_any_cloud(g: &Game, x: f32, y: f32) -> bool {
    g.clouds.iter().any(|c| {
        let (rw, rh) = (c.r * 3.6 * 0.5 * 0.8, c.r * 1.7 * 0.5 * 0.8);
        let (dx, dy) = ((x - c.x) / rw, (y - c.y) / rh);
        dx * dx + dy * dy <= 1.0
    })
}

/// Islands, part of the sea/sky scenery layer (drawn after the clouds, before
/// the aerial combat) — plus a slim HP bar, same idiom as the boss's, once
/// one has taken a hit.
pub(super) fn draw_islands(blip: &Blip, g: &Game, island_tex: &[Texture2D; 3]) {
    for isl in pool_iter(&g.islands) {
        let (w, h) = ISLAND_SIZES[isl.size as usize];
        // Surf: two pale rings breathing round the shore, out of step, under the island.
        let t = blip::now() + isl.x * 0.01;
        let (cx, cy) = (isl.x + w as f32 / 2.0, isl.y + h as f32 / 2.0);
        for k in 0..2 {
            let p = (t * 0.8 + k as f32 * 0.5).fract();
            let grow = 1.0 + 0.12 * p;
            blip::macroquad::shapes::draw_ellipse_lines(cx, cy, w as f32 * 0.5 * grow, h as f32 * 0.5 * grow, 0.0, 2.5,
                BlipColor { r: 0.88, g: 0.96, b: 1.0, a: 0.55 * (1.0 - p) });
        }
        blip.draw_texture(&island_tex[isl.size as usize], isl.x, isl.y, w as f32, h as f32);
        for k in 0..FLAK_GUNS[isl.size as usize].len() {
            draw_flak_gun(blip, isl, k);
        }
        let max_hp = ISLAND_HP[isl.size as usize];
        if isl.hp < max_hp {
            let frac = (isl.hp as f32 / max_hp as f32).clamp(0.0, 1.0);
            blip.draw_rect(isl.x, isl.y - 6.0, w as f32, 3.0, BLIP_GRAY);
            blip.fill_rect(isl.x, isl.y - 6.0, w as f32 * frac, 3.0, BLIP_RED);
        }
    }
}

/// A flak emplacement: a ring of sandbags round a dark gun, its long barrel
/// on the player.
pub(super) fn draw_flak_gun(blip: &Blip, isl: &Island, k: usize) {
    let (x, y) = flak_gun_pos(isl, k);
    draw_flak_pit(blip, x, y, &isl.guns[k]);
}

/// A flak gun in its pit at (`x`, `y`), wherever it stands.
pub(super) fn draw_flak_pit(blip: &Blip, x: f32, y: f32, gun: &FlakGun) {
    blip.fill_circle(x + 1.0, y + 1.5, 8.5, BlipColor::new(0.0, 0.0, 0.0, 0.3));
    for b in 0..10 {
        let a = b as f32 / 10.0 * std::f32::consts::TAU;
        let shade = if b % 2 == 0 { 0.62 } else { 0.54 };
        blip.fill_circle(x + a.cos() * 6.5, y + a.sin() * 6.5, 2.6, BlipColor::new(shade, shade * 0.88, shade * 0.6, 1.0));
    }
    blip.fill_circle(x, y, 4.5, BlipColor::new(0.2, 0.22, 0.18, 1.0));
    let (s, c) = gun.angle.sin_cos();
    blip.draw_line_ex(x, y, x + s * 11.0, y + c * 11.0, 2.4, BlipColor::new(0.08, 0.08, 0.09, 1.0));
    blip.fill_circle(x, y, 2.4, BlipColor::new(0.32, 0.34, 0.3, 1.0));
    // the flash as it fires
    if gun.flash > 0.0 {
        blip.fill_glow_circle(x + s * 13.0, y + c * 13.0, 3.0, BlipColor::new(1.0, 0.85, 0.45, 1.0));
    }
}

/// Flak bursts: a bright flash and a hard black ball while it can hurt,
/// then a soft grey cloud that drifts back and fades.
pub(super) fn draw_flak_bursts(blip: &Blip, g: &Game) {
    for b in pool_iter(&g.flak_bursts) {
        if b.t < FLAK_LETHAL {
            let k = b.t / FLAK_LETHAL;
            let r = FLAK_BURST_R * (0.45 + 0.55 * k);
            blip.fill_circle(b.x, b.y, r, BlipColor::new(0.08, 0.07, 0.07, 0.9));
            blip.fill_glow_circle(b.x, b.y, r * 0.45 * (1.0 - k), BlipColor::new(1.0, 0.62, 0.25, 1.0 - k));
        } else {
            let k = (b.t - FLAK_LETHAL) / FLAK_SMOKE;
            let r = FLAK_BURST_R * (1.0 + 0.5 * k);
            let a = 0.55 * (1.0 - k);
            for (dx, dy, rr) in [(0.0, 0.0, 1.0), (-0.4, -0.25, 0.7), (0.45, -0.15, 0.65), (0.1, 0.4, 0.6)] {
                blip.fill_circle(b.x + dx * r, b.y + dy * r, r * rr * 0.75, BlipColor::new(0.18, 0.17, 0.17, a));
            }
        }
    }
}

/// A round in flight: a slug laid along its velocity, a dark jacket with a
/// lighter tip, and a faint streak of its motion. No tracers.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_slug(blip: &Blip, x: f32, y: f32, vx: f32, vy: f32, len: f32, wid: f32, body: BlipColor) {
    let v = vx.hypot(vy).max(1.0);
    let (dx, dy) = (vx / v, vy / v);
    let (tx, ty) = (x - dx * len, y - dy * len);
    blip.draw_line_ex(tx - dx * len, ty - dy * len, tx, ty, wid * 0.6, BlipColor::new(body.r, body.g, body.b, 0.18));
    blip.draw_line_ex(tx, ty, x, y, wid + 1.2, BlipColor::new(0.05, 0.05, 0.06, 0.8));
    blip.draw_line_ex(tx, ty, x, y, wid, body);
    blip.fill_circle(x, y, wid * 0.45, BlipColor::new(1.0, 0.92, 0.7, 1.0));
}

/// Draw a bank-foreshortened aircraft; nose-down sprites rotate by `-heading`.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_plane(tex: &Texture2D, x: f32, y: f32, w: f32, h: f32, heading: f32, bank: f32, tint: BlipColor) {
    let ww = w * (0.55 + 0.45 * bank.cos());
    draw_texture_ex(tex, x + (w - ww) / 2.0, y, tint, DrawTextureParams {
        dest_size: Some(vec2(ww, h)),
        rotation: -heading,
        ..Default::default()
    });
}

pub(super) fn draw_launch(
    blip: &Blip, g: &Game,
    player_tex: &Texture2D, enemy_tex: &[Texture2D; 3], carrier_tex: &Texture2D, boat_tex: &Texture2D,
    cloud_tex: &[Texture2D; 3],
) {
    draw_sea(blip, g);
    land::draw_land(blip, g);
    draw_boats(blip, g, boat_tex);
    let t = g.launch_t;
    let cs = g.carrier_scale;
    let (cw, ch) = (CARRIER_W as f32 * cs, CARRIER_H as f32 * cs);
    let cx0 = WIN_W as f32 / 2.0 - CARRIER_DECK_X * cs;
    let top = g.ship_y;
    // wake: the bow wave peeling off both sides, a churned band astern
    let bow = (WIN_W as f32 / 2.0, top + 2.0 * cs);
    for side in [-1.0f32, 1.0] {
        for k in 0..14 {
            let f = k as f32 / 13.0;
            let (x, y) = (bow.0 + side * (8.0 + f * 70.0) * cs, bow.1 + f * 150.0 * cs);
            let r = (2.0 + f * 6.0) * cs;
            blip.fill_circle(x, y, r, BlipColor::new(0.9, 0.95, 1.0, 0.35 * (1.0 - f)));
        }
    }
    let stern = top + ch - 10.0 * cs;
    for k in 0..26 {
        let f = k as f32 / 25.0;
        let jitter = ((k as f32 * 12.9898 + g.sea_scroll * 0.07).sin() * 43758.5).fract() - 0.5;
        let (x, y) = (WIN_W as f32 / 2.0 + jitter * (40.0 + f * 90.0) * cs, stern + f * 220.0 * cs);
        blip.fill_circle(x, y, (7.0 + f * 12.0) * cs, BlipColor::new(0.88, 0.94, 1.0, 0.3 * (1.0 - f)));
    }
    blip.draw_texture(carrier_tex, cx0, top, cw, ch);

    // deck crew: the launch officer beside the plane, handlers aft
    let deck = |dx: f32, dy: f32| (cx0 + (CARRIER_DECK_X + dx) * cs, top + dy * cs);
    let crew = |x: f32, y: f32, shirt: BlipColor, low: bool| {
        let s = 1.5 * cs * if low { 0.8 } else { 1.0 };
        blip.fill_circle(x + 1.5 * s, y + 2.0 * s, 3.2 * s, BlipColor::new(0.0, 0.0, 0.0, 0.3));
        blip.fill_circle(x, y, 3.0 * s, shirt);
        blip.fill_circle(x, y - 0.6 * s, 1.6 * s, BlipColor::new(0.86, 0.68, 0.52, 1.0));
    };
    let spot_y = CARRIER_BOW + LAUNCH_DECK_POS * CARRIER_DECK_LEN;
    let yellow = BlipColor::new(0.95, 0.8, 0.2, 1.0);
    let blue = BlipColor::new(0.2, 0.35, 0.75, 1.0);
    let red = BlipColor::new(0.8, 0.2, 0.2, 1.0);
    let (ox, oy) = deck(-44.0, spot_y - 58.0);
    crew(ox, oy, yellow, t > LAUNCH_SIGNAL_END);
    // the flag: wound round overhead to rev up, held high, then down the deck
    let (fa, flen) = if t < LAUNCH_START_END { (t * 9.0, 9.0) } else if t < LAUNCH_SIGNAL_END { (-1.57, 11.0) } else { (-0.4, 10.0) };
    let (fx, fy) = (ox + fa.cos() * flen * cs, oy + fa.sin() * flen * cs);
    blip.draw_line_ex(ox, oy, fx, fy, 1.2 * cs, BlipColor::new(0.3, 0.25, 0.2, 1.0));
    blip.fill_rect(fx - 2.5 * cs, fy - 2.0 * cs, 5.0 * cs, 4.0 * cs, BlipColor::new(0.95, 0.2, 0.2, 1.0));
    for (dx, dy, c) in [(-60.0, spot_y + 70.0, blue), (58.0, spot_y + 52.0, blue), (-30.0, spot_y + 150.0, blue),
                        (40.0, spot_y + 160.0, blue), (70.0, 230.0, red), (72.0, 380.0, red)] {
        let (x, y) = deck(dx, dy);
        crew(x, y, c, false);
    }

    // clouds only once there is height under the plane
    if g.launch_climb > 0.05 { draw_clouds(blip, g, cloud_tex); }

    // the opening wave, streaming in from the top while the player climbs
    for e in pool_iter(&g.enemies) {
        draw_shadow(&enemy_tex[e.kind as usize], e.x, e.y, ENEMY_W as f32, ENEMY_H as f32, -e.heading);
    }
    for e in pool_iter(&g.enemies) {
        draw_plane(&enemy_tex[e.kind as usize], e.x, e.y, ENEMY_W as f32, ENEMY_H as f32, e.heading, e.bank, BLIP_WHITE);
    }

    // the plane: its shadow under it on the deck, falling behind as it climbs
    let climb = g.launch_climb;
    // seen from above, the plane grows as it climbs toward the camera
    let scale = 0.62 + 0.38 * climb;
    let pw = PLAYER_W as f32 * scale;
    let ph = PLAYER_H as f32 * scale;
    let px = g.player_x + (PLAYER_W as f32 - pw) / 2.0;
    let py = g.player_y + (PLAYER_H as f32 - ph) / 2.0;
    let (sdx, sdy) = (1.5 + (PLANE_SHADOW_DX - 1.5) * climb, 2.0 + (PLANE_SHADOW_DY - 2.0) * climb);
    draw_texture_ex(player_tex, px + sdx, py + sdy, BlipColor::new(0.0, 0.0, 0.0, 0.35), DrawTextureParams {
        dest_size: Some(vec2(pw, ph)), rotation: g.player_bank, ..Default::default()
    });
    draw_texture_ex(player_tex, px, py, BLIP_WHITE, DrawTextureParams {
        dest_size: Some(vec2(pw, ph)),
        rotation: g.player_bank,
        ..Default::default()
    });
    for p in pool_iter(&g.puffs) {
        let a = p.ttl / p.max_ttl;
        blip.fill_circle(p.x, p.y, p.r, BlipColor::new(0.78, 0.8, 0.84, 0.55 * a));
    }
    blip.draw_hud(g.sess.score, g.sess.lives);
    blip.draw_hud_multiple(g.sess.score, g.streak.multiple());
}

/// The fight: the sea, land, boats, islands and clouds, pickups, the planes and the boss, every round in the air,
/// wrecks, and the HUD with the boss's warning.
pub(super) fn draw_play(blip: &Blip, g: &Game, art: &Art) {
    let Art {
        player: player_tex, enemies: enemy_tex, bosses: boss_tex, powerup: pow_tex, health: health_tex,
        boss_names_ja: boss_name_ja_tex, boat: boat_tex, clouds: cloud_tex, islands: island_tex, ..
    } = art;
    draw_sea(blip, g);
    land::draw_land(blip, g);
    draw_boats(blip, g, boat_tex);
    draw_islands(blip, g, island_tex);
    draw_clouds(blip, g, cloud_tex);

    // Tinted by the tier it's about to grant, so the colour is a readable
    // preview of what catching it does — and it brightens as you approach
    // the max tier.
    let next_tier_color = weapon_tier_color((g.weapon_level + 1).min(MAX_WEAPON_LEVEL));
    for p in pool_iter(&g.powerups) {
        blip.draw_texture_tinted(pow_tex, p.x, p.y, POW_W, POW_H, next_tier_color);
    }
    for h in pool_iter(&g.health_pickups) {
        blip.draw_texture(health_tex, h.x, h.y, HEALTH_W, HEALTH_H);
    }

    // Shadows first (a separate pass, not interleaved with the sprites) so a
    // low enemy's shadow never draws over another plane sitting behind it.
    for e in pool_iter(&g.enemies) {
        let (ecx, ecy) = (e.x + ENEMY_W as f32 / 2.0, e.y + ENEMY_H as f32 / 2.0);
        if e.can_hide && point_in_any_cloud(g, ecx, ecy) { continue; }
        let tex = &enemy_tex[e.kind as usize];
        draw_shadow(tex, e.x, e.y, ENEMY_W as f32, ENEMY_H as f32, -e.heading);
    }
    if g.boss.active {
        let (bw, bh) = boss_size(g.boss.tier);
        draw_shadow(&boss_tex[g.boss.tier], g.boss.x, g.boss.y, bw, bh, 0.0);
    }
    if g.state == State::Play && !g.respawn_grace.active() && g.stall_fall == 0.0 {
        draw_shadow(player_tex, g.player_x, g.player_y, PLAYER_W as f32, PLAYER_H as f32, g.player_bank);
    }

    // Speed: streaks of wind rushing past, more and longer the faster the
    // plane flies (they ride the sea scroll, which follows the airspeed).
    {
        let k = g.scroll_k;
        let n = (6.0 + 22.0 * (k - 0.6).max(0.0)) as i32;
        let len = 14.0 + 60.0 * (k - 0.6).max(0.0);
        let a = (0.05 + 0.14 * (k - 0.8).max(0.0)).min(0.22);
        for i in 0..n {
            let seed = i as f32 * 71.3;
            let x = (seed * 17.7) % WIN_W as f32;
            let y = (seed * 31.1 + g.sea_scroll * 7.0) % (WIN_H as f32 + len) - len;
            blip.draw_line_ex(x, y, x, y + len, 1.0, BlipColor::new(0.9, 0.95, 1.0, a));
        }
    }

    // Planes going down: smoke under them, the burning plane shrinking
    // toward the sea with its shadow closing in, fire on top.
    for p in pool_iter(&g.puffs) {
        if p.fire || p.top { continue; }
        let a = p.ttl / p.max_ttl;
        blip.fill_circle(p.x, p.y, p.r, BlipColor::new(0.12, 0.11, 0.10, 0.5 * a));
    }
    for w in pool_iter(&g.wrecks) {
        let k = w.t / w.dur;
        let s = 1.0 - 0.55 * k;
        let (ww, wh) = (ENEMY_W as f32 * s, ENEMY_H as f32 * s);
        let (x, y) = (w.x + (ENEMY_W as f32 - ww) / 2.0, w.y + (ENEMY_H as f32 - wh) / 2.0);
        let tex = &enemy_tex[w.kind as usize];
        draw_texture_ex(tex, x + PLANE_SHADOW_DX * (1.0 - k), y + PLANE_SHADOW_DY * (1.0 - k),
            BlipColor::new(0.0, 0.0, 0.0, 0.3), DrawTextureParams {
                dest_size: Some(vec2(ww, wh)), rotation: -w.heading, ..Default::default()
            });
        let burn = 1.0 - 0.6 * k;
        draw_texture_ex(tex, x, y, BlipColor::new(burn, burn * 0.85, burn * 0.75, 1.0), DrawTextureParams {
            dest_size: Some(vec2(ww, wh)), rotation: -w.heading, ..Default::default()
        });
    }
    for p in pool_iter(&g.puffs) {
        if !p.fire || p.top { continue; }
        let a = p.ttl / p.max_ttl;
        blip.fill_circle(p.x, p.y, p.r, BlipColor::new(1.0, 0.45 + 0.4 * a, 0.1, 0.9 * a));
        blip.fill_circle(p.x, p.y, p.r * 0.5, BlipColor::new(1.0, 0.95, 0.6, a));
    }

    for e in pool_iter(&g.enemies) {
        // Some planes vanish under a cloud they fly through: draw order only;
        // they keep flying, firing and can be hit.
        let (ecx, ecy) = (e.x + ENEMY_W as f32 / 2.0, e.y + ENEMY_H as f32 / 2.0);
        if e.can_hide && point_in_any_cloud(g, ecx, ecy) { continue; }
        let tex = &enemy_tex[e.kind as usize];
        draw_plane(tex, e.x, e.y, ENEMY_W as f32, ENEMY_H as f32, e.heading, e.bank, BLIP_WHITE);
    }

    if g.boss.active {
        let (bw, bh) = boss_size(g.boss.tier);
        let frac = (g.boss.hp as f32 / g.boss.max_hp as f32).clamp(0.0, 1.0);
        // The final boss flashes white-hot once enraged (below half health) —
        // a plain colour tint on the same sprite, no extra art needed.
        let enraged = g.boss.tier == BOSS_SPECS.len() - 1 && frac <= 0.5;
        let tint = if enraged && ((g.boss.t * 8.0) as i32 % 2 == 0) {
            BlipColor::new(1.6, 1.4, 1.2, 1.0)
        } else {
            BlipColor::new(1.0, 1.0, 1.0, 1.0)
        };
        draw_texture_ex(&boss_tex[g.boss.tier], g.boss.x, g.boss.y, tint, DrawTextureParams {
            dest_size: Some(vec2(bw, bh)),
            rotation: boss_bank(g),
            ..Default::default()
        });
        for k in 0..g.boss.n_turrets {
            draw_turret(blip, g, k);
        }
        for i in 0..g.boss.n_lasers {
            draw_laser(blip, g, i);
        }
        blip.draw_rect(g.boss.x, g.boss.y - 8.0, bw, 4.0, BLIP_GRAY);
        blip.fill_rect(g.boss.x, g.boss.y - 8.0, bw * frac, 4.0, BLIP_RED);
    }

    if g.barrier.active {
        let warming = g.barrier.warmup.active();
        let flicker = ((g.barrier.t * 10.0) as i32 % 2) == 0;
        // Dim and flickering while it warms up (harmless, telegraphing),
        // solid and white-hot down the middle once it's actually live.
        let beam_color = if warming {
            if flicker { BlipColor::new(1.0, 0.2, 0.2, 0.5) } else { BlipColor::new(1.0, 0.2, 0.2, 0.15) }
        } else {
            BlipColor::new(1.0, 0.15, 0.15, 0.9)
        };
        let by = g.barrier.y;
        blip.fill_rect(0.0, by - BARRIER_BEAM_H / 2.0, WIN_W as f32, BARRIER_BEAM_H, beam_color);
        if !warming {
            blip.fill_rect(0.0, by - 1.5, WIN_W as f32, 3.0, BLIP_WHITE);
        } else {
            let color = if flicker { BLIP_RED } else { BLIP_WHITE };
            blip.draw_centered("LASER BARRIER", by + 22.0, 2.0, color);
        }

        // The motor: a dark housing with a glowing core and its own health
        // meter above it — the only part of the barrier that's shootable.
        let (mx, my) = (g.barrier.motor_x - MOTOR_W / 2.0, g.barrier.y - MOTOR_H / 2.0);
        blip.fill_rect(mx, my, MOTOR_W, MOTOR_H, BLIP_GRAY);
        blip.draw_rect(mx, my, MOTOR_W, MOTOR_H, BLIP_BLACK);
        blip.fill_glow_circle(g.barrier.motor_x, by, 7.0, BLIP_RED);
        let hp_frac = (g.barrier.hp as f32 / g.barrier.max_hp as f32).clamp(0.0, 1.0);
        blip.draw_rect(mx, my - 8.0, MOTOR_W, 4.0, BLIP_GRAY);
        blip.fill_rect(mx, my - 8.0, MOTOR_W * hp_frac, 4.0, BLIP_RED);
    }

    // Everyone's rounds are slugs: the enemies' (draw_enemy_rounds, on top of everything), brass from the player's guns.
    draw_flak_bursts(blip, g);
    let brass = BlipColor::new(0.82, 0.64, 0.28, 1.0);
    let copper = BlipColor::new(0.72, 0.38, 0.20, 1.0);
    let glint = BlipColor::new(1.0, 0.93, 0.72, 0.9);
    for b in pool_iter(&g.bullets) {
        let v = b.vx.hypot(b.vy).max(1.0);
        let (dx, dy) = (b.vx / v, -b.vy / v); // unit, nose-ward
        let (px, py) = (-dy, dx);
        let (len, wid) = (b.r * 3.4, b.r * 1.15);
        let (nx, ny) = (b.x, b.y);
        let (tx, ty) = (nx - dx * len, ny - dy * len);
        let (sx, sy) = (nx - dx * len * 0.35, ny - dy * len * 0.35); // where the ogive starts
        blip.draw_line_ex(tx, ty, sx, sy, wid, brass);
        blip.fill_triangle((sx + px * wid / 2.0, sy + py * wid / 2.0), (sx - px * wid / 2.0, sy - py * wid / 2.0), (nx, ny), copper);
        blip.draw_line_ex(tx + px * wid * 0.22, ty + py * wid * 0.22,
            sx + px * wid * 0.22, sy + py * wid * 0.22, (wid * 0.28).max(0.6), glint);
    }
    // Spent brass, tumbling off the wings.
    for c in pool_iter(&g.casings) {
        let a = (c.ttl / 0.45).clamp(0.0, 1.0);
        let (s, co) = c.rot.sin_cos();
        blip.draw_line_ex(c.x - co * 2.2, c.y - s * 2.2, c.x + co * 2.2, c.y + s * 2.2, 1.8,
            BlipColor::new(0.86, 0.66, 0.26, a));
    }
    // Muzzle flash at each gun for a frame or three after a burst.
    if g.muzzle_t > 0.0 && g.state == State::Play {
        let (n, fan, r) = weapon_guns(g.weapon_level);
        let cx = g.player_x + PLAYER_W as f32 / 2.0;
        let a = g.muzzle_t / 0.05;
        for i in 0..n {
            let dx = if n > 1 { (i as f32 / (n - 1) as f32 - 0.5) * fan } else { 0.0 };
            blip.fill_glow_circle(cx + dx, g.player_y + 2.0, r * 1.6 * a + 1.0,
                BlipColor::new(1.0, 0.85, 0.45, a));
        }
    }

    if g.state == State::Play && g.stall_fall > 0.0 {
        // Spinning down to the sea, its shadow closing in.
        let k = g.stall_fall / STALL_FALL;
        let s = 1.0 - 0.5 * k;
        let (w, h) = (PLAYER_W as f32 * s, PLAYER_H as f32 * s);
        let (x, y) = (g.player_x + (PLAYER_W as f32 - w) / 2.0, g.player_y + (PLAYER_H as f32 - h) / 2.0);
        draw_texture_ex(player_tex, x + PLANE_SHADOW_DX * (1.0 - k), y + PLANE_SHADOW_DY * (1.0 - k),
            BlipColor::new(0.0, 0.0, 0.0, 0.3), DrawTextureParams {
                dest_size: Some(vec2(w, h)), rotation: g.player_bank, ..Default::default()
            });
        let d = 1.0 - 0.5 * k;
        draw_texture_ex(player_tex, x, y, BlipColor::new(d, d, d, 1.0), DrawTextureParams {
            dest_size: Some(vec2(w, h)), rotation: g.player_bank, ..Default::default()
        });
    } else if g.state == State::Play {
        // Blink the plane while the post-respawn grace period is active.
        let blink = g.respawn_grace.active() && ((g.respawn_grace.remaining() * 12.0) as i32 % 2 == 0);
        if g.airspeed < STALL_WARN && blip::blink(5.0) {
            blip.draw_text("STALL", g.player_x + PLAYER_W as f32 / 2.0 - 15.0, g.player_y - 14.0, 2.0, BLIP_RED);
        }
        if !blink {
            // near a stall the airframe buffets
            let shake = if g.airspeed < STALL_WARN { 2.5 * (1.0 - g.airspeed / STALL_WARN) } else { 0.0 };
            let tt = blip::now();
            let (bx, by) = ((tt * 57.0).sin() * shake, (tt * 43.0).cos() * shake);
            // at full throttle, vapour streams off the wingtips
            if g.airspeed > 1.15 {
                let a = ((g.airspeed - 1.15) / (AIRSPEED_MAX - 1.15)).min(1.0) * 0.35;
                let wy = g.player_y + PLAYER_H as f32 * 0.42;
                for side in [-1.0f32, 1.0] {
                    let wx = g.player_x + PLAYER_W as f32 / 2.0 + side * PLAYER_W as f32 * 0.47;
                    blip.draw_line_ex(wx, wy, wx, wy + 40.0, 1.4, BlipColor::new(1.0, 1.0, 1.0, a));
                }
            }
            // Slow flight is a glide, not a reverse: the camera overtakes a
            // plane riding the wind, wings rocking, engine idling.
            let gl = glide(g);
            let sway = (tt * 1.7).sin() * 0.07 * gl;
            let by = by + (tt * 1.3).sin() * 1.5 * gl;
            draw_glide_wind(blip, g, gl, tt);
            draw_texture_ex(player_tex, g.player_x + bx, g.player_y + by, BLIP_WHITE, DrawTextureParams {
                dest_size: Some(vec2(PLAYER_W as f32, PLAYER_H as f32)),
                rotation: g.player_bank + sway,
                ..Default::default()
            });
            draw_idle_prop(blip, g, g.player_x + bx, g.player_y + by, g.player_bank + sway, gl);
        }
        // the player's own smoke and flames, over the airframe
        for p in pool_iter(&g.puffs) {
            if blink || !p.top || p.fire { continue; }
            let a = p.ttl / p.max_ttl;
            blip.fill_circle(p.x, p.y, p.r, BlipColor::new(0.14, 0.13, 0.12, 0.42 * a));
        }
        for p in pool_iter(&g.puffs) {
            if blink || !p.top || !p.fire { continue; }
            let a = p.ttl / p.max_ttl;
            blip.fill_circle(p.x, p.y, p.r, BlipColor::new(1.0, 0.4 + 0.4 * a, 0.08, 0.95 * a));
            blip.fill_circle(p.x, p.y, p.r * 0.5, BlipColor::new(1.0, 0.95, 0.6, a));
        }
    }

    for e in pool_iter(&g.explosions) {
        let k = (e.ttl / e.max_ttl).clamp(0.0, 1.0);
        let r = 14.0 * e.scale * (0.6 + k * 0.6);
        blip.fill_glow_circle(e.x, e.y, r, BlipColor::new(e.color.r, e.color.g, e.color.b, k));
    }

    g.fx.draw(blip);
    draw_enemy_rounds(blip, g);
    blip.draw_hud(g.sess.score, g.sess.lives);
    blip.draw_hud_multiple(g.sess.score, g.streak.multiple());
    draw_bottom_hud(blip, g);

    if g.max_power_banner.active() {
        let flash = ((g.max_power_banner.remaining() * 14.0) as i32 % 2) == 0;
        let color = if flash { weapon_tier_color(MAX_WEAPON_LEVEL) } else { BLIP_WHITE };
        blip.draw_centered("MAXIMUM POWER", (WIN_H / 2 - 12) as f32, 4.0, color);
    }

    if g.boss_intro.active() {
        let flash = ((g.boss_intro.remaining() * 10.0) as i32 % 2) == 0;
        let color = if flash { BLIP_RED } else { BLIP_WHITE };
        let name = BOSS_SPECS[g.boss.tier].name;
        blip.draw_centered("WARNING", (WIN_H / 2 - 40) as f32, 3.0, color);
        blip.draw_centered(name,      (WIN_H / 2 - 16) as f32, 3.0, color);
        // The Japanese name below it — a texture, not the bitmap font (which
        // only covers A-Z/0-9), tinted the same flashing colour.
        let ja = &boss_name_ja_tex[g.boss.tier];
        let jx = (WIN_W as f32 - ja.width()) / 2.0;
        blip.draw_texture_tinted(ja, jx, (WIN_H / 2 + 8) as f32, ja.width(), ja.height(), color);
    }
}

/// The corner readouts: a mini plane and the lives bottom-left, the health
/// bar bottom-right, where a glance mid-fight lands.
/// 0 at cruise, rising to 1 as the throttle closes to the stall warning.
pub(super) fn glide(g: &Game) -> f32 {
    ((1.0 - g.airspeed) / (1.0 - STALL_WARN)).clamp(0.0, 1.0)
}

/// Air still streaming past the wingtips while the plane falls behind the
/// camera: it is flying forward through the air, only slower than the scroll.
pub(super) fn draw_glide_wind(blip: &Blip, g: &Game, gl: f32, tt: f32) {
    if gl <= 0.05 { return; }
    let (cx, top) = (g.player_x + PLAYER_W as f32 / 2.0, g.player_y);
    const RUN: f32 = PLAYER_H as f32 + 8.0; // how far a wisp travels, nose to past the tail
    for side in [-1.0f32, 1.0] {
        for i in 0..3 {
            let k = ((tt * 130.0 + i as f32 * 19.0 + if side > 0.0 { 9.0 } else { 0.0 }) % RUN) / RUN;
            let x = cx + side * (PLAYER_W as f32 * 0.47 + 2.0);
            let y = top + 6.0 + k * RUN;
            let a = gl * 0.7 * (1.0 - k) * (k * 6.0).min(1.0);
            blip.draw_line_ex(x, y, x, y + 14.0, 1.6, BlipColor::new(0.88, 0.94, 1.0, a));
        }
    }
}

/// The propeller turning slowly enough to see its blades, over the sprite's
/// blur disc, at the nose of a plane drawn at (`x`, `y`) rotated `rot`.
pub(super) fn draw_idle_prop(blip: &Blip, g: &Game, x: f32, y: f32, rot: f32, gl: f32) {
    if gl <= 0.1 { return; }
    // In the sprite's 36-unit design the hub is 1 unit below the top edge and
    // a blade 6.3 units long (see gfx::player_plane).
    let s = PLAYER_W as f32 / 36.0;
    let (hw, hh) = (PLAYER_W as f32 / 2.0, PLAYER_H as f32 / 2.0);
    let d = hh - 1.0 * s;
    let (hx, hy) = (x + hw + d * rot.sin(), y + hh - d * rot.cos());
    // Seen from above, a blade's length shows as it swings across the nose.
    let reach = 6.3 * s * g.prop_angle.cos();
    let (dx, dy) = (reach * rot.cos(), reach * rot.sin());
    let c = BlipColor::new(0.08, 0.08, 0.09, ((gl - 0.1) / 0.4).min(1.0));
    blip.draw_line_ex(hx - dx, hy - dy, hx + dx, hy + dy, 2.6, c);
}

/// Draw turret health, aimed barrels, a loaded-gun cue, muzzle flash and reload progress.
pub(super) fn draw_turret(blip: &Blip, g: &Game, k: usize) {
    let t = &g.boss.turrets[k];
    let r = turret_radius(g.boss.tier);
    let (x, y) = turret_pos(g, k);
    blip.fill_circle(x + 1.0, y + 1.5, r + 1.2, BlipColor::new(0.0, 0.0, 0.0, 0.35));
    blip.fill_circle(x, y, r + 1.2, BlipColor::new(0.08, 0.08, 0.09, 1.0));
    blip.fill_circle(x, y, r + 0.4, BlipColor::new(0.52, 0.55, 0.58, 1.0));
    if t.hp <= 0 {
        blip.fill_circle(x, y, r * 0.8, BlipColor::new(0.08, 0.07, 0.06, 1.0));
        return;
    }
    let (s, c) = t.angle.sin_cos();
    let (ox, oy) = (c * 1.9, -s * 1.9);
    let len = r * 2.6;
    let reloading = t.reload_t > 0.0;
    if !reloading {
        // The line of fire: where the next rounds will go.
        let due = if t.burst > 0 { 1.0 } else { (1.0 - t.next_t / 0.5).clamp(0.0, 1.0) };
        let a = 0.10 + 0.35 * due;
        let (mx, my) = (x + s * len, y + c * len);
        for d in 0..9 {
            let (d0, d1) = (6.0 + d as f32 * 12.0, 12.0 + d as f32 * 12.0);
            let fade = 1.0 - d as f32 / 9.0;
            blip.draw_line_ex(mx + s * d0, my + c * d0, mx + s * d1, my + c * d1, 1.2,
                BlipColor::new(1.0, 0.55, 0.3, a * fade));
        }
    }
    for side in [-1.0f32, 1.0] {
        let (bx, by) = (x + ox * side, y + oy * side);
        blip.draw_line_ex(bx, by, bx + s * len, by + c * len, 2.0, BlipColor::new(0.06, 0.06, 0.07, 1.0));
    }
    if t.flash > 0.0 {
        blip.fill_glow_circle(x + s * (len + 2.0), y + c * (len + 2.0), 3.0, BlipColor::new(1.0, 0.85, 0.45, 1.0));
    }
    blip.fill_circle(x, y, r * 0.78, BlipColor::new(0.36, 0.42, 0.34, 1.0));
    blip.fill_circle(x - s * 1.5, y - c * 1.5, r * 0.5, BlipColor::new(0.55, 0.68, 0.72, 0.9));
    blip.fill_circle(x - r * 0.25, y - r * 0.3, r * 0.18, BlipColor::new(0.95, 1.0, 1.0, 0.8));
    if reloading {
        // the drum going on: an amber ring filling round the turret
        let done = 1.0 - t.reload_t / RELOAD_SECS;
        let dots = 14;
        for d in 0..dots {
            let lit = (d as f32 / dots as f32) < done;
            let a = d as f32 / dots as f32 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            blip.fill_circle(x + a.cos() * (r + 3.2), y + a.sin() * (r + 3.2), 1.1,
                if lit { BlipColor::new(1.0, 0.75, 0.2, 1.0) } else { BlipColor::new(0.3, 0.25, 0.15, 0.7) });
        }
    }
}

/// A laser turret: a bigger dark dome with a single heavy emitter. While it
/// charges, a thin line shows exactly where the beam will go, pulsing
/// faster as it nears firing; the beam itself is a white core in red glow.
pub(super) fn draw_laser(blip: &Blip, g: &Game, i: usize) {
    let l = &g.boss.lasers[i];
    let r = laser_radius(g.boss.tier);
    let (x, y) = mount_pos(g, l.fx, l.fy);
    blip.fill_circle(x + 1.0, y + 1.5, r + 1.2, BlipColor::new(0.0, 0.0, 0.0, 0.35));
    blip.fill_circle(x, y, r + 1.2, BlipColor::new(0.08, 0.08, 0.09, 1.0));
    blip.fill_circle(x, y, r + 0.4, BlipColor::new(0.45, 0.2, 0.2, 1.0));
    if l.hp <= 0 {
        blip.fill_circle(x, y, r * 0.8, BlipColor::new(0.08, 0.07, 0.06, 1.0));
        return;
    }
    let (s, c) = l.angle.sin_cos();
    let len = r * 2.4;
    let (mx, my) = (x + s * len, y + c * len);
    let far = 900.0;
    match l.phase {
        LaserPhase::Charging => {
            let k = 1.0 - l.t / LASER_CHARGE;
            let pulse = 0.5 + 0.5 * (g.boss.t * (8.0 + 22.0 * k)).sin();
            blip.draw_line_ex(mx, my, mx + s * far, my + c * far, 1.0 + k,
                BlipColor::new(1.0, 0.2, 0.2, 0.25 + 0.45 * k * pulse));
            blip.fill_glow_circle(mx, my, 1.5 + 3.0 * k, BlipColor::new(1.0, 0.3, 0.3, 0.6 + 0.4 * k));
        }
        LaserPhase::Firing => {
            let flick = 0.85 + 0.15 * (g.boss.t * 60.0).sin();
            blip.draw_line_ex(mx, my, mx + s * far, my + c * far, LASER_HALF_W * 2.8, BlipColor::new(1.0, 0.15, 0.15, 0.35 * flick));
            blip.draw_line_ex(mx, my, mx + s * far, my + c * far, LASER_HALF_W * 1.6, BlipColor::new(1.0, 0.3, 0.25, 0.8 * flick));
            blip.draw_line_ex(mx, my, mx + s * far, my + c * far, LASER_HALF_W * 0.6, BlipColor::new(1.0, 0.95, 0.9, 1.0));
            blip.fill_glow_circle(mx, my, 5.0, BlipColor::new(1.0, 0.6, 0.5, 1.0));
        }
        LaserPhase::Cooling => {}
    }
    blip.draw_line_ex(x, y, mx, my, 4.0, BlipColor::new(0.06, 0.06, 0.07, 1.0));
    blip.fill_circle(x, y, r * 0.75, BlipColor::new(0.22, 0.1, 0.1, 1.0));
    let glow = if l.phase == LaserPhase::Cooling { 0.35 } else { 1.0 };
    blip.fill_circle(x, y, r * 0.35, BlipColor::new(1.0, 0.25, 0.2, glow));
}

/// The enemies' rounds, drawn after explosions and score popups so nothing hides a danger (a kill's "+300" and its
/// blast sat over the rounds coming at the plane): steel for the fighters' and gunners', heavier flak shells.
pub(super) fn draw_enemy_rounds(blip: &Blip, g: &Game) {
    for b in pool_iter(&g.enemy_bullets) {
        draw_slug(blip, b.x, b.y, b.vx, b.vy, 6.0, 2.2, BlipColor::new(0.78, 0.74, 0.66, 1.0));
    }
    for s in pool_iter(&g.flak) {
        draw_slug(blip, s.x, s.y, s.vx, s.vy, 8.0, 3.2, BlipColor::new(0.62, 0.58, 0.5, 1.0));
    }
}

pub(super) fn draw_bottom_hud(blip: &Blip, g: &Game) {
    // Lives are the HUD's hearts; here only the health, bottom-right, in a blip box. It empties with hits, is full
    // again each life, and pickups restore some. Kept in from the edge, clear of the curved glass's rim.
    let (bar_w, bar_h) = (64.0, 8.0);
    let bar_x = WIN_W as f32 - bar_w - 30.0;
    let bar_y = (WIN_H - 22) as f32;
    let hp_frac = (g.health as f32 / PLAYER_HEALTH_MAX as f32).clamp(0.0, 1.0);
    let hp_color = if hp_frac > 0.6 { BLIP_LEAF } else if hp_frac > 0.3 { BLIP_GOLD } else { BLIP_HEART };
    blip.draw_panel(bar_x - 40.0, bar_y - 9.0, bar_w + 50.0, bar_h + 18.0, 1.0);
    blip.draw_text("HP", bar_x - 30.0, bar_y - 3.0, 2.0, BLIP_CREAM);
    blip.fill_rect(bar_x, bar_y, bar_w, bar_h, blip::shade(BLIP_HEART, 0.35));
    blip.fill_rect(bar_x, bar_y, bar_w * hp_frac, bar_h, hp_color);
    blip.fill_rect(bar_x, bar_y, bar_w * hp_frac, 2.0, blip::blend(hp_color, BLIP_CREAM, 0.5));
}

pub(super) fn draw_title(
    blip: &Blip, backdrop: &Texture2D, player_tex: &Texture2D,
) {
    blip.draw_texture(backdrop, 0.0, 0.0, WIN_W as f32, WIN_H as f32);
    blip.draw_logo("RAIDER", (WIN_W / 2) as f32, (WIN_H / 4 - 10) as f32, 6.0, BLIP_GOLD);
    // RAIDER is sz=6 (42px tall) — clear its bottom by a real margin.
    blip.draw_attract_bar();
    let px = (WIN_W as f32 - PLAYER_W as f32 * 2.0) / 2.0;
    blip.draw_texture(player_tex, px, (WIN_H / 2 - 70) as f32, PLAYER_W as f32 * 2.0, PLAYER_H as f32 * 2.0);
    blip.draw_start_prompt((WIN_H * 2 / 3) as f32);
    blip.draw_panel(WIN_W as f32 * 0.17, (WIN_H * 2 / 3 + 30) as f32, WIN_W as f32 * 0.66, 56.0, 1.0);
    blip.draw_centered("UP THROTTLE  DOWN SLOW", (WIN_H * 2 / 3 + 42) as f32, 2.0, BLIP_CREAM);
    blip.draw_centered("TOO SLOW AND YOU STALL", (WIN_H * 2 / 3 + 60) as f32, 2.0, BLIP_CREAM);
}

pub(super) fn draw_win(blip: &Blip, level: i32) {
    let buf = format!("WAVE {level}");
    blip.draw_logo("STAGE CLEAR", (WIN_W / 2) as f32, (WIN_H / 3) as f32, 5.0, BLIP_LEAF);
    blip.draw_message(&buf, (WIN_W / 2) as f32, (WIN_H / 2 + 10) as f32, 3.0, BLIP_GOLD, 1.0);
}

pub(super) fn draw_won(blip: &Blip, score: i32, hi: i32, waiting: bool) {
    blip.draw_end_screen("YOU WON!!", score, hi, BLIP_YELLOW, BLIP_GREEN, BLIP_YELLOW, !waiting);
    blip.draw_game_over_note(&format!("ALL {MAX_LEVEL} WAVES CLEARED"), BLIP_GREEN);
}
