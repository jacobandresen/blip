//! Drawing: the dungeon floor, the wall and its shards, the ball and paddle, drops, and the title and level-clear
//! screens.

use super::*;

/// A shard: its face as a fan of triangles, then a darker edge so neighbouring shards read apart.
pub(super) fn draw_shard(blip: &Blip, s: &Shard) {
    let (sn, cs) = s.turn.sin_cos();
    let pts: Vec<(f32, f32)> = s.pts.iter().map(|p| (s.x + p.0 * cs - p.1 * sn, s.y + p.0 * sn + p.1 * cs)).collect();
    let edge = BLIP_INK;
    blip.fill_fan((s.x, s.y), &pts, s.color);
    for k in 0..pts.len() {
        let (a, b) = (pts[k], pts[(k + 1) % pts.len()]);
        blip.draw_line(a.0, a.1, b.0, b.1, edge);
    }
}

/// A tiled floor behind the bricks, a new hue each level: big dusky flagstones, each with a lit and a shaded edge
/// and a small diamond inlay, after the adventure games' dungeon floors. Dim, so the bricks and ball stand out.
pub(super) fn draw_backdrop(blip: &Blip, level: i32) {
    const HUES: [(f32, f32, f32); 5] = [(0.17, 0.19, 0.36), (0.30, 0.17, 0.32), (0.13, 0.27, 0.30), (0.32, 0.22, 0.17), (0.18, 0.27, 0.18)];
    let (r, g, b) = HUES[((level - 1).max(0) as usize) % HUES.len()];
    // Kept dim (0.6), so the floor never competes with the ball.
    let tone = |k: f32| BlipColor { r: r * k * 0.6, g: g * k * 0.6, b: b * k * 0.6, a: 1.0 };
    const TILE: i32 = 40;
    blip.fill_rect(0.0, HUD_H as f32, WIN_W as f32, (WIN_H - HUD_H) as f32, tone(0.62));
    for ty in 0..(WIN_H - HUD_H) / TILE + 1 {
        for tx in 0..WIN_W / TILE {
            let (x, y) = ((tx * TILE) as f32, (HUD_H + ty * TILE) as f32);
            let s = TILE as f32;
            blip.fill_rect(x + 1.0, y + 1.0, s - 2.0, s - 2.0, tone(if (tx + ty) % 2 == 0 { 0.9 } else { 0.82 }));
            blip.fill_rect(x + 1.0, y + 1.0, s - 2.0, 1.0, tone(1.08));
            let (cx, cy) = (x + s / 2.0, y + s / 2.0);
            blip.fill_quad((cx, cy - 4.0), (cx + 4.0, cy), (cx, cy + 4.0), (cx - 4.0, cy), tone(0.76));
        }
    }
}

const DROP_PNGS: [&[u8]; 5] = [
    blip::png!("drop_wide"), blip::png!("drop_narrow"), blip::png!("drop_slow"), blip::png!("drop_life"),
    blip::png!("drop_multi"),
];

/// The pictures, loaded once.
pub(super) struct Art {
    paddle: Texture2D,
    /// The ball's sheet of turned poses (ball_pose picks one).
    ball: Texture2D,
    /// The ball's light and shading, which stay put over the turning ball.
    shade: Texture2D,
    /// One per brick kind (BRICK_COLORS order).
    brick: [Texture2D; 9],
    /// One per DropKind.
    drops: [Texture2D; 5],
}

impl Art {
    pub(super) fn load() -> Art {
        let paddle = load_png_smooth(blip::png!("paddle"));
        let ball = load_png_smooth(blip::png!("ball"));
        let drops = DROP_PNGS.map(load_png_smooth);
        let shade = load_png_smooth(blip::png!("ball_shade"));
        let brick = [
            load_png(blip::png!("brick_red")), load_png(blip::png!("brick_orange")), load_png(blip::png!("brick_yellow")),
            load_png(blip::png!("brick_green")), load_png(blip::png!("brick_blue")), load_png(blip::png!("brick_purple")),
            load_png(blip::png!("brick_steel")), load_png(blip::png!("brick_steel_cracked")),
            load_png(blip::png!("brick_bomb")),
        ];
        Art { paddle, ball, shade, brick, drops }
    }
}

pub(super) fn draw_play(blip: &Blip, g: &Game, art: &Art) {
    let Art { paddle, ball, shade, brick, drops } = art;
    draw_backdrop(blip, g.sess.level);
    // The wall's shadow on the floor, then the wall.
    for i in (0..BRICK_TOTAL).filter(|&i| g.bricks[i].alive) {
        let (bx, by) = brick_rect(i);
        blip.fill_rect(bx + 3.0, by + 5.0, BRICK_W as f32, BRICK_H as f32, BLIP_SHADOW);
    }
    for i in 0..BRICK_TOTAL {
        if !g.bricks[i].alive { continue; }
        let w = g.bricks[i].wobble;
        let (bx, by0) = brick_rect(i);
        let by = by0 + (w * 22.0).sin() * w * 1.0;
        blip.draw_texture(&brick[g.bricks[i].kind], bx, by, BRICK_W as f32, BRICK_H as f32);
    }
    for s in &g.shards { draw_shard(blip, s); }

    // Pickups sway as they fall.
    for d in pool_iter(&g.drops) {
        let phase = d.y * 0.045 + d.x;
        blip.draw_texture_cell(&drops[d.kind as usize], 0, 1, 0, 1, d.x, d.y, DROP_W, DROP_H, phase.sin() * 0.16);
    }

    // Each active effect on its own line with its seconds left; it blinks for the last two.
    let widened = g.pad_w > PAD_W as f32;
    let effects = [
        (g.slow_timer.active(), "SLOW", BLIP_CYAN, g.slow_timer.remaining()),
        (g.pad_effect_timer.active(), if widened { "WIDE" } else { "NARROW" },
            if widened { BLIP_GREEN } else { BLIP_RED }, g.pad_effect_timer.remaining()),
    ];
    let mut line_y = (PAD_Y - 20) as f32;
    for (_, label, color, left) in effects.iter().filter(|e| e.0) {
        if *left > EFFECT_WARN || (left * 6.0) as i32 % 2 == 0 {
            blip.draw_centered_outlined(&format!("{label} {}", left.ceil() as i32), (WIN_W / 2) as f32, line_y, 2.0, *color, BLIP_INK);
        }
        line_y -= 18.0;
    }

    // Left cap, stretched middle, right cap: the ends keep their shape at any width.
    let (cap, py) = (PADDLE_CAP as f32, PAD_Y as f32 + g.pad_kick);
    let (sw, cap_w) = (paddle.width(), PADDLE_CAP as f32 * 0.5);
    let ph = PAD_H as f32;
    // Dash afterimages, trailing behind and fading with the boost.
    if g.dash_t > 0.0 {
        let k = g.dash_t / DASH_SECS;
        for i in 1..=3 {
            let x = g.pad_x - g.dash_dir * i as f32 * 9.0;
            let c = BlipColor { r: 0.35, g: 0.75, b: 1.0, a: 0.28 * k / i as f32 };
            blip.fill_rect(x + 2.0, py + 2.0, g.pad_w - 4.0, ph - 4.0, c);
        }
    }
    blip.draw_shadow(g.pad_x + g.pad_w / 2.0 + 3.0, py + ph + 4.0, g.pad_w);
    blip.draw_texture_region(paddle, 0.0, 0.0, cap, 24.0, g.pad_x, py, cap_w, ph);
    blip.draw_texture_region(paddle, cap, 0.0, sw - 2.0 * cap, 24.0, g.pad_x + cap_w, py, g.pad_w - 2.0 * cap_w, ph);
    blip.draw_texture_region(paddle, sw - cap, 0.0, cap, 24.0, g.pad_x + g.pad_w - cap_w, py, cap_w, ph);

    // Seeking: the brick the ball is bending toward pulses.
    if g.seeking() {
        let pulse = 0.5 + 0.5 * (blip::now() * 8.0).sin();
        if let Some((tx, ty)) = g.nearest_brick() {
            let (x, y) = (tx - BRICK_W as f32 / 2.0 - 2.0, ty - BRICK_H as f32 / 2.0 - 2.0);
            blip.draw_rect(x, y, BRICK_W as f32 + 4.0, BRICK_H as f32 + 4.0, BlipColor { r: 1.0, g: 0.95, b: 0.6, a: pulse });
        }
    }

    for b in g.extra.iter().copied().chain([g.ball()]) {
        // Soft drop shadow, offset toward the direction of travel, so the ball
        // reads as rolling across the play field rather than floating over it.
        let ball_cx = b.x + BALL_W as f32 / 2.0;
        let ball_cy = b.y + BALL_H as f32 / 2.0;
        let speed = b.vx.hypot(b.vy).max(1.0);
        // A fading streak behind a fast ball, longer the faster it goes.
        let streak = ((speed - 260.0) / 160.0).clamp(0.0, 1.0);
        for i in (1..=4).rev() {
            if streak <= 0.0 { break; }
            let back = i as f32 * 6.0 * streak;
            blip.fill_circle(ball_cx - b.vx / speed * back, ball_cy - b.vy / speed * back, BALL_W as f32 * (0.5 - i as f32 * 0.07),
                BlipColor { r: 0.6, g: 0.85, b: 1.0, a: 0.22 * streak * (1.0 - i as f32 / 5.0) });
        }
        blip.fill_circle(
            ball_cx + (b.vx / speed) * 4.0, ball_cy + (b.vy / speed) * 4.0 + 3.0, BALL_W as f32 * 0.42,
            BLIP_SHADOW,
        );
        if g.seeking() {
            blip.fill_glow_circle(ball_cx, ball_cy, BALL_W as f32 * 0.55, BlipColor { r: 1.0, g: 0.9, b: 0.5, a: 0.35 });
        }
        let (col, row, roll) = ball_pose(&b.rot);
        let (bx, by, bs) = (b.x - 1.0, b.y - 1.0, BALL_W as f32 + 2.0);
        blip.draw_texture_cell(ball, col, BALL_YAW_N, row, BALL_PITCH_N, bx, by, bs, bs, roll);
        blip.draw_texture(shade, bx, by, bs, bs);
    }

    g.fx.draw(blip);
    blip.draw_hud(g.sess.score, g.sess.lives);
}

pub(super) fn draw_title(blip: &Blip, art: &Art) {
    let Art { paddle, ball, shade, brick, .. } = art;
    draw_backdrop(blip, 1);
    // A full wall of the game's bricks, a ripple running along it.
    let t0 = blip::now();
    for row in 0..3 {
        for col in 0..BRICK_COLS {
            let kind = ((row + col) % 6) as usize;
            let x = (BRICK_OX + col * (BRICK_W + BRICK_GAP)) as f32;
            let lift = ((t0 * 3.0 - col as f32 * 0.5).sin().max(0.0)).powi(4) * 4.0;
            let y = (HUD_H + 26 + row * (BRICK_H + BRICK_GAP)) as f32 - lift;
            blip.draw_texture(&brick[kind], x, y, BRICK_W as f32, BRICK_H as f32);
        }
    }
    blip.draw_logo("BOUNCER", (WIN_W / 2) as f32, (WIN_H / 4) as f32, 6.0, BLIP_SEA);
    // BOUNCER is sz=6 (42px tall) — clear its bottom by a real margin.
    blip.draw_attract_bar();
    let by = web::controls();
    blip.draw_start_prompt((WIN_H / 2) as f32);
    let lines = [
        by.pick("LEFT RIGHT ARROW OR AD", "PAD LEFT AND RIGHT", "DRAG TO MOVE"),
        by.pick("SPACE TO LAUNCH", "FIRE TO LAUNCH", "TAP TO LAUNCH"),
        by.pick("DOUBLE TAP TO DASH", "DOUBLE TAP TO DASH", ""),
    ];
    let lines: Vec<&str> = lines.into_iter().filter(|l| !l.is_empty()).collect();
    let top = (WIN_H * 2 / 3 - 14) as f32;
    blip.draw_panel(WIN_W as f32 * 0.14, top, WIN_W as f32 * 0.72, 20.0 * lines.len() as f32 + 22.0, 1.0);
    for (i, line) in lines.iter().enumerate() {
        blip.draw_centered(line, top + 12.0 + 20.0 * i as f32, 2.0, BLIP_CREAM);
    }
    let t = blip::now();
    let cx = WIN_W as f32 * 0.5 + (t * 1.2).sin() * 132.0;
    let bx = cx - BALL_W as f32 * 0.5;
    let by = PAD_Y as f32 - 48.0 + (t * 2.0).sin() * 7.0;
    for i in (1..=3).rev() {
        blip.fill_rect(bx - i as f32 * 8.0, by + 3.0, BALL_W as f32, BALL_H as f32,
            BlipColor { r: 0.25, g: 0.55, b: 1.0, a: 0.11 / i as f32 });
    }
    blip.draw_texture_cell(ball, 0, BALL_YAW_N, 0, BALL_PITCH_N, bx, by, BALL_W as f32, BALL_H as f32, t * 0.7);
    blip.draw_texture(shade, bx, by, BALL_W as f32, BALL_H as f32);
    let pad_x = (cx - PAD_W as f32 * 0.5).clamp(0.0, WIN_W as f32 - PAD_W as f32);
    let cap = PADDLE_CAP as f32;
    let sw = paddle.width();
    blip.draw_texture_region(paddle, 0.0, 0.0, cap, 24.0, pad_x, PAD_Y as f32, cap * 0.5, PAD_H as f32);
    blip.draw_texture_region(paddle, cap, 0.0, sw - 2.0 * cap, 24.0, pad_x + cap * 0.5, PAD_Y as f32, PAD_W as f32 - cap, PAD_H as f32);
    blip.draw_texture_region(paddle, sw - cap, 0.0, cap, 24.0, pad_x + PAD_W as f32 - cap * 0.5, PAD_Y as f32, cap * 0.5, PAD_H as f32);
}

pub(super) fn draw_clear(blip: &Blip, level: i32, t: f32) {
    draw_backdrop(blip, level);
    blip::draw_fireworks(blip, t, (60.0, (HUD_H + 50) as f32, (WIN_W - 120) as f32, 260.0), &BRICK_COLORS[..6]);
    let buf = format!("LEVEL {}", level + 1);  // level just incremented in next_level
    blip.draw_logo("CLEARED", (WIN_W / 2) as f32, (WIN_H / 3) as f32, 5.0, BLIP_LEAF);
    blip.draw_message(&buf, (WIN_W / 2) as f32, (WIN_H / 2 + 10) as f32, 3.0, BLIP_GOLD, 1.0);
}
