//! Drawing: the meadow and its day, the snake, food, bonus fruit and shears, the title and the game-over screen.

use super::*;

/// The field (gfx.rs): a sunny lawn in a ring of trees, the board's cells a shade apart so the eye can count them,
/// and two butterflies about it, as in Mabe Village.
pub(super) fn draw_board(blip: &Blip, field: &Texture2D) {
    blip.draw_texture(field, 0.0, HUD_H as f32, field.width(), field.height());
    draw_cloud_shadow(blip, blip::now());
    draw_butterflies(blip, blip::now());
    draw_owl(blip, blip::now());
    // Now and then the chest in the glade below the lawn (field.rs CHEST) twinkles at its lock.
    let k = (blip::now() * 0.5).rem_euclid(5.0);
    if k < 0.6 {
        let (sx, sy) = ((CHEST.0 + 16) as f32, (HUD_H + CHEST.1 + 1) as f32);
        let s = 3.5 * (1.0 - (k / 0.3 - 1.0).abs());
        blip.draw_line_ex(sx - s, sy, sx + s, sy, 1.0, BLIP_WHITE);
        blip.draw_line_ex(sx, sy - s, sx, sy + s, 1.0, BLIP_WHITE);
    }
}

/// A wise old owl of our own on a treetop in the wood above the lawn, watching: brown feathers, a cream face, round
/// eyes that blink now and then and look toward the snake's half of the field.
pub(super) fn draw_owl(blip: &Blip, t: f32) {
    let (x, y) = (WIN_W as f32 * 0.68, HUD_H as f32 + 13.0);
    let brown = BlipColor { r: 0.55, g: 0.36, b: 0.20, a: 1.0 };
    let dark = BlipColor { r: 0.36, g: 0.22, b: 0.12, a: 1.0 };
    let cream = BlipColor { r: 0.98, g: 0.92, b: 0.76, a: 1.0 };
    blip.fill_circle(x + 2.0, y + 9.0, 7.0, BlipColor { a: 0.3, ..BLIP_INK });
    blip.fill_circle(x, y + 2.0, 8.5, BLIP_INK);
    blip.fill_circle(x, y + 2.0, 7.5, brown);
    blip.fill_circle(x, y + 5.0, 4.5, BlipColor { r: 0.80, g: 0.64, b: 0.42, a: 1.0 });
    // ear tufts
    for s in [-1.0f32, 1.0] { blip.fill_triangle((x + s * 3.0, y - 5.0), (x + s * 7.0, y - 3.0), (x + s * 6.5, y - 9.0), dark); }
    for s in [-1.0f32, 1.0] { blip.fill_circle(x + s * 3.0, y - 1.0, 3.2, cream); }
    let blink = t.rem_euclid(4.3) < 0.14;
    let look = (t * 0.3).sin() * 0.9;
    for s in [-1.0f32, 1.0] {
        if blink { blip.fill_rect(x + s * 3.0 - 2.0, y - 1.5, 4.0, 1.0, BLIP_INK); }
        else { blip.fill_circle(x + s * 3.0 + look, y - 0.5, 1.5, BLIP_INK); }
    }
    blip.fill_triangle((x - 1.0, y + 1.5), (x + 1.0, y + 1.5), (x, y + 3.5), BlipColor { r: 0.95, g: 0.70, b: 0.20, a: 1.0 });
}

/// A cloud's shadow drifting slowly over the field, once a minute or so: faint, soft-edged, by the clock.
pub(super) fn draw_cloud_shadow(blip: &Blip, t: f32) {
    let span = WIN_W as f32 + 400.0;
    let x = (t * 9.0).rem_euclid(span) - 200.0;
    let y = HUD_H as f32 + (WIN_H - HUD_H) as f32 * (0.35 + 0.2 * (t * 0.013).sin());
    for (dx, dy, r) in [(0.0, 0.0, 90.0), (70.0, 20.0, 70.0), (-80.0, 25.0, 65.0), (20.0, -40.0, 60.0)] {
        for k in 0..4 {
            blip.fill_circle(x + dx, y + dy, r * (1.0 - k as f32 * 0.12), BlipColor { a: 0.025, ..BLIP_INK });
        }
    }
}

/// The day passing over the meadow as the levels go by: the fresh morning of the first levels, a golden afternoon,
/// sunset, then dusk with fireflies. A wash over the field only; the snake, the food and the stones keep their colours,
/// so play reads the same at any hour.
pub(super) fn draw_daylight(blip: &Blip, level: i32, t: f32) {
    let (tint, a) = match level {
        ..=3 => return,
        4..=6 => ((1.0, 0.84, 0.40), 0.16),
        7..=9 => ((1.0, 0.42, 0.50), 0.22),
        _ => ((0.16, 0.20, 0.50), 0.34),
    };
    let (y, w, h) = (HUD_H as f32, WIN_W as f32, (WIN_H - HUD_H) as f32);
    blip.fill_rect(0.0, y, w, h, BlipColor { r: tint.0, g: tint.1, b: tint.2, a });
    if level < 10 { return; }
    // Fireflies: a dozen soft lights drifting and pulsing, by the clock.
    for k in 0..12 {
        let kf = k as f32;
        let x = w * (0.5 + 0.46 * (t * 0.07 * (1.0 + kf * 0.13) + kf * 1.9).sin());
        let fy = y + h * (0.5 + 0.44 * (t * 0.05 * (1.0 + kf * 0.11) + kf * 2.7).cos());
        let glow = 0.5 + 0.5 * (t * (1.3 + kf * 0.2) + kf).sin();
        if glow < 0.25 { continue; }
        blip.fill_glow_circle(x, fy, 2.4, BlipColor { r: 0.95, g: 1.0, b: 0.55, a: glow });
    }
}

/// Two butterflies wandering over the lawn and the wood's edge on slow looping paths, wings beating: by the clock,
/// not the game's random numbers, so play is the same with them. Small and pale, so nobody takes them for food.
pub(super) fn draw_butterflies(blip: &Blip, t: f32) {
    let (w, h) = (WIN_W as f32, (WIN_H - HUD_H) as f32);
    for (k, wing, body) in [(0.0f32, BlipColor { r: 0.62, g: 0.84, b: 1.0, a: 1.0 }, BLIP_INK),
                            (1.0, BlipColor { r: 1.0, g: 0.97, b: 0.86, a: 1.0 }, BLIP_INK)] {
        let s = t * 0.11 + k * 2.7;
        let x = w * (0.5 + 0.42 * (s * 1.3).sin() * (s * 0.37 + k).cos());
        let y = HUD_H as f32 + h * (0.5 + 0.40 * (s * 0.9 + 1.0).sin() * (s * 0.53).sin()) + (t * 5.0 + k).sin() * 3.0;
        let beat = (t * 22.0 + k * 3.0).sin().abs();
        let span = 1.5 + 3.0 * beat;
        blip.fill_circle(x + 3.0, y + 9.0, 2.0, BlipColor { a: 0.18, ..BLIP_INK });
        for side in [-1.0f32, 1.0] {
            blip.fill_circle(x + side * span, y - 1.0, 2.3, wing);
            blip.fill_circle(x + side * span * 0.8, y + 1.8, 1.6, wing);
        }
        blip.fill_rect(x - 0.5, y - 2.0, 1.0, 4.5, body);
    }
}

/// A snake's body through `pts` (head first), each drawn joint (`joint(i)`, from point i to the one before it) a
/// tapering capsule: its shadow on the lawn, the ink outline, the body, then (with `marks`) a highlight lit from the
/// upper left like everything on the field and a scale mark on every other segment.
pub(super) fn draw_body(blip: &Blip, pts: &[(f32, f32)], joint: impl Fn(usize) -> bool, radius: &dyn Fn(usize) -> f32,
    shade: &dyn Fn(usize) -> BlipColor, marks: bool) {
    let n = pts.len();
    // Each point's radius and colour once, for the four passes over the body.
    let rs: Vec<f32> = (0..n).map(radius).collect();
    let cs: Vec<BlipColor> = (0..n).map(shade).collect();
    let joints: Vec<usize> = (1..n).rev().filter(|&i| joint(i)).collect();
    let seg = |i: usize, dx: f32, dy: f32, k: f32, add: f32, c: BlipColor| {
        let ((x0, y0), (x1, y1)) = (pts[i], pts[i - 1]);
        blip.fill_capsule(x0 + dx, y0 + dy, x1 + dx, y1 + dy, rs[i] * k + add, rs[i - 1] * k + add, c);
    };
    for &i in &joints { seg(i, 2.0, 4.0, 1.0, 0.0, LAWN_SHADOW); }
    for &i in &joints { seg(i, 0.0, 0.0, 1.0, 1.5, BLIP_INK); }
    for &i in &joints { seg(i, 0.0, 0.0, 1.0, 0.0, cs[i]); }
    if !marks { return; }
    for &i in &joints {
        let c = cs[i];
        let hi = BlipColor { r: (c.r + 0.25).min(1.0), g: (c.g + 0.12).min(1.0), b: (c.b + 0.2).min(1.0), a: 1.0 };
        seg(i, -rs[i] * 0.3, -rs[i] * 0.3, 0.36, 0.0, hi);
        if i % 2 == 0 && i + 2 < n { blip.fill_circle(pts[i].0, pts[i].1, rs[i] * 0.5, blip::shade(c, 0.6)); }
    }
}

/// The snake drawn as one body: thick strokes between segment centres,
/// brighter toward the head, eyes on the side it is heading.
pub(super) fn draw_snake(blip: &Blip, g: &Game) {
    // Glide between cells by step progress; frozen fully arrived once dead
    // (move_timer stalls mid-step on the fatal tick).
    let f = if g.state == State::Play {
        blip::clamp(g.move_timer / g.move_interval(), 0.0, 1.0)
    } else {
        1.0
    };
    let dead = g.state == State::Dead;
    let flash = dead && DEAD_SECS - g.dead_timer.remaining() < POP_AFTER && (g.dead_timer.remaining() / 0.08) as i32 % 2 == 0;
    let first = if dead { g.popped.min(g.snake_len) } else { 0 };
    let half = CELL as f32 / 2.0;
    let seg_px = |i: usize| -> (f32, f32) {
        let cur = g.snake_at(i);
        let prev = if i + 1 < g.snake_len {
            g.snake_at(i + 1)
        } else {
            // the tail: extrapolate the cell one step back along the body
            let ahead = g.snake_at(i - 1);
            Cell { c: 2 * cur.c - ahead.c, r: 2 * cur.r - ahead.r }
        };
        let x = BOARD_X as f32 + lerp(prev.c as f32, cur.c as f32, f) * CELL as f32 + half;
        let y = BOARD_Y as f32 + lerp(prev.r as f32, cur.r as f32, f) * CELL as f32 + half;
        (x, y)
    };
    let n = g.snake_len;
    let shade = |i: usize| -> BlipColor {
        if flash { return BLIP_RED; }
        let k = i as f32 / n.max(2) as f32;
        scale_green(k)
    };
    // Body radius tapers toward the tail; a swallowed food bulges it as it travels down.
    let base_r = CELL as f32 / 2.0 - 3.0;
    let radius = |i: usize| {
        let taper = if i + 4 >= n { 0.55 + 0.45 * (n - 1 - i) as f32 / 4.0 } else { 1.0 };
        let bulge = g.gulps.iter().map(|&k| (1.0 - (k - i as f32).abs()).max(0.0)).fold(0.0, f32::max);
        base_r * taper.min(1.0) + bulge * 3.5
    };
    let pts: Vec<(f32, f32)> = (0..n).map(seg_px).collect();
    draw_body(blip, &pts, |i| i > first, &radius, &shade, !flash);
    if first == 0 {
        blip.fill_circle(pts[0].0 + 2.0, pts[0].1 + 4.0, CELL as f32 / 2.0 - 1.0, LAWN_SHADOW);
    }
    // A new level: a sparkle runs down the body from head to tail.
    if g.state == State::Play && g.banner_t < 0.9 && g.sess.level > 1 {
        let at = g.banner_t / 0.9 * n as f32;
        for k in 0..3 {
            let i = (at as usize).saturating_sub(k * 2).min(n - 1);
            let (sx, sy) = pts[i];
            let a = 1.0 - k as f32 * 0.3;
            blip.fill_glow_circle(sx, sy, 7.0 - k as f32 * 1.5, BlipColor { r: 1.0, g: 0.95, b: 0.6, a });
        }
    }
    if first > 0 { return; }
    let (hx, hy) = pts[0];
    let (fx, fy) = match g.cur_dir { Dir::Up => (0.0, -1.0), Dir::Down => (0.0, 1.0), Dir::Left => (-1.0, 0.0), Dir::Right => (1.0, 0.0) };
    let hr = CELL as f32 / 2.0 - 1.0;
    let head = if flash { BLIP_RED } else { HEAD_GREEN };
    // A round head, longer than wide: a capsule from the neck to the snout.
    let (nx, ny) = (hx - fx * 3.0, hy - fy * 3.0);
    let (sx, sy) = (hx + fx * 3.0, hy + fy * 3.0);
    blip.fill_capsule(nx, ny, sx, sy, hr + 1.5, hr * 0.8 + 1.5, BLIP_INK);
    blip.fill_capsule(nx, ny, sx, sy, hr, hr * 0.8, head);
    // A forked tongue flicks out now and then, in the direction of travel.
    if g.state == State::Play && blip::now().rem_euclid(1.6) < 0.18 {
        let (bx, by) = (hx + fx * (hr + 2.0), hy + fy * (hr + 2.0));
        let (tx, ty) = (hx + fx * (hr + 8.0), hy + fy * (hr + 8.0));
        blip.draw_line_ex(bx, by, tx, ty, 2.0, BLIP_RED);
        for side in [-1.0f32, 1.0] {
            blip.draw_line_ex(tx, ty, tx + fx * 3.0 - fy * side * 3.0, ty + fy * 3.0 + fx * side * 3.0, 1.5, BLIP_RED);
        }
    }
    // The mouth opens as food comes within a cell ahead.
    let ahead = Cell { c: g.snake_at(0).c + fx as i32, r: g.snake_at(0).r + fy as i32 };
    if g.state == State::Play && (ahead == g.food || (g.bonus_active() && ahead == g.bonus)) {
        let open = 2.0 + 4.0 * f;
        let (mx, my) = (hx + fx * (hr + 1.0), hy + fy * (hr + 1.0));
        blip.fill_capsule(mx - fy * open, my + fx * open, mx + fy * open, my - fx * open, 2.5, 2.5, BlipColor { r: 0.35, g: 0.0, b: 0.05, a: 1.0 });
    }
    for side in [-1.0f32, 1.0] {
        let (ex, ey) = (hx + fx * 3.0 - fy * side * 5.5, hy + fy * 3.0 + fx * side * 5.5);
        blip.fill_circle(ex, ey, 3.6, BLIP_WHITE);
        blip.fill_circle(ex + fx * 1.4, ey + fy * 1.4, 1.8, BLIP_INK);
    }
}

pub(super) fn draw_play(blip: &Blip, g: &Game, food: &Texture2D, field: &Texture2D) {
    draw_board(blip, field);
    draw_daylight(blip, g.sess.level, blip::now());
    draw_pickups(blip, g, food);
    draw_obstacles(blip, g);
    draw_snake(blip, g);
    g.fx.draw(blip);
    if g.banner_t < BANNER_SECS && g.state == State::Play {
        let a = (1.0 - g.banner_t / BANNER_SECS).min(1.0) * 1.5;
        // Row 5 is clear in every obstacle layout.
        blip.draw_message(&format!("LEVEL {}", g.sess.level), (WIN_W / 2) as f32, (BOARD_Y + 5 * CELL + 16) as f32, 3.0,
            BLIP_GOLD, a.min(1.0));
    }
    blip.draw_hud(g.sess.score, g.sess.lives);
    blip.draw_hud_multiple(g.sess.score, g.streak.multiple());
}

/// A new layout blinks in, see-through, until it is solid.
pub(super) fn draw_obstacles(blip: &Blip, g: &Game) {
    let solid = g.obs_fade >= 1.0;
    let a = if solid { 1.0 } else if (g.obs_fade * 8.0) as i32 % 2 == 0 { 0.55 } else { 0.25 };
    for i in 0..g.obstacle_count {
        let (cx, cy) = cell_centre(g.obstacles[i]);
        // A round grey boulder as on Koholint: lit from the upper left, darker underneath, a soft shadow to the lower
        // right, a nick or two in it. A new one rises out of the lawn as it blinks in.
        let r = (CELL as f32 / 2.0 - 1.0) * if solid { 1.0 } else { 0.4 + 0.6 * g.obs_fade };
        let tint = |c: (f32, f32, f32)| BlipColor { r: c.0, g: c.1, b: c.2, a };
        blip.fill_circle(cx + 3.0, cy + 4.0, r, BlipColor { a: a * 0.35, ..BLIP_INK });
        blip.fill_circle(cx, cy, r + 1.0, BlipColor { a, ..BLIP_INK });
        blip.fill_circle(cx, cy, r, tint((0.30, 0.31, 0.36)));
        blip.fill_circle(cx - r * 0.12, cy - r * 0.16, r * 0.82, tint((0.46, 0.47, 0.52)));
        blip.fill_circle(cx - r * 0.26, cy - r * 0.32, r * 0.48, tint((0.62, 0.63, 0.68)));
        blip.fill_circle(cx - r * 0.36, cy - r * 0.42, r * 0.18, tint((0.84, 0.85, 0.88)));
        let k = g.obstacles[i].c * 7 + g.obstacles[i].r * 3;
        if solid && k % 3 != 0 {
            let s = if k % 2 == 0 { 1.0 } else { -1.0 };
            blip.draw_line_ex(cx + s * r * 0.2, cy + r * 0.1, cx + s * r * 0.5, cy + r * 0.45, 1.5, tint((0.22, 0.23, 0.27)));
        }
    }
}

/// The food, breathing a little so the eye finds it; the bonus and the shears.
pub(super) fn draw_pickups(blip: &Blip, g: &Game, food: &Texture2D) {
    let breathe = 1.0 + 0.08 * (blip::now() * 5.0).sin();
    let fs = CELL as f32 * breathe;
    let (fcx, fcy) = cell_centre(g.food);
    blip.fill_circle(fcx + 2.0, fcy + 8.0, 7.0, LAWN_SHADOW);
    blip.draw_texture(food, fcx - fs / 2.0, fcy - fs / 2.0, fs, fs);
    draw_bonus(blip, g);
    draw_shears(blip, g);
}

/// The timed bonus: shrinks with its clock (drawn in-world, where the
/// go-for-it decision is made) and blinks for the last two seconds.
pub(super) fn draw_bonus(blip: &Blip, g: &Game) {
    if !g.bonus_active() { return; }
    // Off for only a third of each cycle so it stays findable.
    let expiring = g.bonus_ttl <= BONUS_WARN;
    if expiring && (g.bonus_ttl * 6.0) as i32 % 3 == 0 { return; }

    let (cx, cy) = cell_centre(g.bonus);
    let life = g.bonus_ttl / BONUS_TTL;
    let r = CELL as f32 * (0.26 + 0.20 * life);
    let gold = GOLD;
    let hot = BlipColor { r: 1.0, g: 0.98, b: 0.72, a: 1.0 };
    // A golden mouse facing the way it last ran: body, head, two round ears, a wiggling tail.
    let (fx, fy) = (g.mouse_dir.0 as f32, g.mouse_dir.1 as f32);
    let t = blip::now();
    let (tx, ty) = (cx - fx * r * 1.0, cy - fy * r * 1.0);
    let wig = (t * 14.0).sin() * r * 0.5;
    blip.draw_line_ex(tx, ty, tx - fx * r * 0.9 - fy * wig, ty - fy * r * 0.9 + fx * wig, 1.5, gold);
    blip.fill_capsule(cx - fx * r * 0.5, cy - fy * r * 0.5, cx + fx * r * 0.3, cy + fy * r * 0.3, r * 0.75, r * 0.6, gold);
    let (hx, hy) = (cx + fx * r * 0.75, cy + fy * r * 0.75);
    for side in [-1.0f32, 1.0] {
        blip.fill_circle(hx - fx * r * 0.2 - fy * side * r * 0.45, hy - fy * r * 0.2 + fx * side * r * 0.45, r * 0.3, hot);
    }
    blip.fill_circle(hx, hy, r * 0.45, gold);
    blip.fill_circle(hx + fx * r * 0.45, hy + fy * r * 0.45, 1.2, BLIP_RED);
    blip.fill_circle(hx + fx * r * 0.1 - fy * r * 0.2, hy + fy * r * 0.1 + fx * r * 0.2, 1.0, BLIP_BLACK);
}

/// The shears: a pale blue X of two blades, flashing as they run out.
pub(super) fn draw_shears(blip: &Blip, g: &Game) {
    if !g.shears_active() { return; }
    if g.shears_ttl <= BONUS_WARN && (g.shears_ttl * 6.0) as i32 % 3 == 0 { return; }
    let (cx, cy) = cell_centre(g.shears);
    let r = CELL as f32 * 0.34;
    blip.draw_line_ex(cx - r, cy - r, cx + r, cy + r, 3.0, SHEARS_BLUE);
    blip.draw_line_ex(cx - r, cy + r, cx + r, cy - r, 3.0, SHEARS_BLUE);
    // the two finger rings
    blip.fill_circle(cx - r, cy + r, 3.0, SHEARS_BLUE);
    blip.fill_circle(cx + r, cy + r, 3.0, SHEARS_BLUE);
    blip.fill_circle(cx, cy, 2.0, BLIP_WHITE);
}

pub(super) fn draw_title(blip: &Blip, field: &Texture2D, wood: &Texture2D) {
    draw_board(blip, field);
    blip.draw_logo("SERPENT", (WIN_W / 2) as f32, (WIN_H / 4) as f32, 6.0, BLIP_LEAF);
    // SERPENT is sz=6 (42px tall) — clear its bottom by a real margin.
    let by = web::controls();
    blip.draw_start_prompt((WIN_H / 2) as f32);
    blip.draw_message(by.pick("ARROW KEYS OR WASD", "STEER WITH THE PAD", "SWIPE TO STEER"), (WIN_W / 2) as f32,
        (WIN_H * 2 / 3 + 7) as f32, 2.0, BLIP_CREAM, 1.0);
    draw_title_snake(blip, blip::now());
    // the snake and the mouse run in under the trees at the sides
    blip.draw_texture(wood, 0.0, HUD_H as f32, wood.width(), wood.height());
    blip.draw_attract_bar();
}

/// A snake winding across the title screen after a golden mouse, wrapping round: drawn as in play, an outlined body
/// tapering to the tail with a belly stripe and scale marks.
pub(super) fn draw_title_snake(blip: &Blip, t: f32) {
    const SEGS: usize = 22;
    let span = WIN_W as f32 + 260.0;
    let at = |d: f32| {
        let x = (t * 90.0 - d).rem_euclid(span) - 160.0;
        (x, WIN_H as f32 * 0.83 + ((t * 90.0 - d) * 0.035).sin() * 22.0)
    };
    let shade = |i: usize| {
        let k = i as f32 / SEGS as f32;
        scale_green(k)
    };
    let r = |i: usize| 11.0 * if i + 5 >= SEGS { 0.5 + 0.5 * (SEGS - 1 - i) as f32 / 5.0 } else { 1.0 };
    let pts: Vec<(f32, f32)> = (0..SEGS).map(|i| at(i as f32 * 10.0)).collect();
    // Skip a joint that wraps across the screen edge.
    draw_body(blip, &pts, |i| (pts[i].0 - pts[i - 1].0).abs() < 40.0, &r, &shade, true);
    let (hx, hy) = pts[0];
    blip.fill_circle(hx + 2.0, hy, 13.5, BLIP_INK);
    blip.fill_circle(hx + 2.0, hy, 12.0, HEAD_GREEN);
    for side in [-1.0f32, 1.0] {
        blip.fill_circle(hx + 5.0, hy + side * 5.5, 3.6, BLIP_WHITE);
        blip.fill_circle(hx + 6.4, hy + side * 5.5, 1.8, BLIP_BLACK);
    }
    if t.rem_euclid(1.2) < 0.2 {
        blip.draw_line_ex(hx + 14.0, hy, hx + 22.0, hy, 2.0, BLIP_RED);
        for side in [-1.0f32, 1.0] { blip.draw_line_ex(hx + 22.0, hy, hx + 25.0, hy + side * 3.0, 1.5, BLIP_RED); }
    }
    // The golden mouse, always a little out of reach.
    let mx = (t * 90.0 + 70.0).rem_euclid(span) - 160.0;
    let my = WIN_H as f32 * 0.83 + ((t * 90.0 + 70.0) * 0.035).sin() * 22.0 + (t * 18.0).sin().abs() * -3.0;
    let wig = (t * 14.0).sin() * 4.0;
    blip.draw_line_ex(mx - 9.0, my, mx - 17.0, my + wig, 1.5, GOLD);
    blip.fill_capsule(mx - 5.0, my, mx + 3.0, my, 8.0, 6.5, GOLD);
    blip.fill_circle(mx + 2.0, my - 6.0, 3.5, BlipColor { r: 1.0, g: 0.98, b: 0.72, a: 1.0 });
    blip.fill_circle(mx + 8.0, my, 5.0, GOLD);
    blip.fill_circle(mx + 12.5, my, 1.3, BLIP_RED);
    blip.fill_circle(mx + 9.0, my - 2.0, 1.1, BLIP_BLACK);
}

pub(super) fn draw_over(blip: &Blip, g: &Game, hi: i32, field: &Texture2D) {
    // The field as it ended, dimmed under the score.
    draw_board(blip, field);
    draw_obstacles(blip, g);
    draw_snake(blip, g);
    blip.fill_rect(0.0, HUD_H as f32, WIN_W as f32, (WIN_H - HUD_H) as f32, BlipColor { a: 0.45, ..BLIP_INK });
    let (title, colour) = if g.snake_len + g.obstacle_count == MAX_LEN { ("FIELD CLEARED", BLIP_GREEN) } else { ("GAME OVER", BLIP_RED) };
    blip.draw_end_screen(title, g.sess.score, hi, colour, BLIP_GREEN, BLIP_YELLOW, !g.dead_timer.active());
    blip.draw_game_over_note(&format!("LEVEL {}   LENGTH {}", g.sess.level, g.snake_len), BLIP_GRAY);
}
