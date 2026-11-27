//! Drawing: the room and its tiles, bubbles, candies and toys, the scores in cosy text, the title, and the frame.

use super::*;

// ---- drawing -------------------------------------------------------------

pub(super) fn tint(c: (u8, u8, u8), k: f32) -> (u8, u8, u8) {
    (((c.0 as f32) * k).min(255.0) as u8, ((c.1 as f32) * k).min(255.0) as u8, ((c.2 as f32) * k).min(255.0) as u8)
}

pub(super) fn draw_background(blip: &Blip, g: &Game) {
    let pal = &PALETTES[g.round];
    let ch = |a: u8, b: u8, t: f32| (a as f32 + (b as f32 - a as f32) * t) as u8;
    blip.fill_bands(0.0, 0.0, WIN_W as f32, WIN_H as f32, 18, |t| {
        col((ch(pal.sky0.0, pal.sky1.0, t), ch(pal.sky0.1, pal.sky1.1, t), ch(pal.sky0.2, pal.sky1.2, t)), 1.0)
    });
    draw_bokeh(blip, pal);
}

pub(super) fn draw_bokeh(blip: &Blip, pal: &levels::Palette) {
    use blip::macroquad::shapes::draw_poly;
    let t = now();
    // a faint polka-dot print drifting slowly on the diagonal, like Harajuku fabric
    let drift = (t * 6.0) % 48.0;
    for j in -1..14 {
        for i in -1..15 {
            let x = i as f32 * 48.0 + if j % 2 == 0 { 0.0 } else { 24.0 } + drift;
            let y = j as f32 * 44.0 + drift * 0.9;
            blip.fill_circle(x, y, 5.0, col(pal.glow, 0.05));
        }
    }
    // little hearts and stars floating up behind the play field
    for i in 0..18 {
        let s = i as f32 * 37.7;
        let x = (s * 13.1) % WIN_W as f32 + (t * 0.7 + s).sin() * 8.0;
        let y = WIN_H as f32 - ((s * 7.3 + t * (6.0 + (i % 5) as f32 * 3.0)) % (WIN_H as f32 + 40.0)) + 20.0;
        let r = 3.0 + (i % 3) as f32 * 2.0;
        let a = 0.08 + 0.05 * ((t * 0.8 + s).sin() * 0.5 + 0.5);
        if i % 2 == 0 {
            draw_heart(&Heart { x, y, t: 1.0, sway: 0.0 }, 0.0, 0.0, r / 5.0, col(pal.glow, a));
        } else {
            draw_poly(x, y, 5, r, t * 20.0 + s, col(pal.glow, a));
        }
    }
    // twinkles: four-point sparkles
    for i in 0..26 {
        let s = i as f32 * 91.3;
        let x = (s * 5.7) % WIN_W as f32;
        let y = HUD + (s * 3.1) % (WIN_H as f32 - HUD);
        let a = ((t * 2.0 + s).sin() * 0.5 + 0.5).powi(3) * 0.8;
        sparkle(x, y, 1.0 + 2.5 * a, rgba(1.0, 1.0, 1.0, a));
    }
}

/// A four-point sparkle of radius `r`.
pub(super) fn sparkle(x: f32, y: f32, r: f32, c: BlipColor) {
    use blip::macroquad::{math::vec2, shapes::draw_triangle};
    let w = r * 0.28;
    draw_triangle(vec2(x - r, y), vec2(x + r, y), vec2(x, y - w), c);
    draw_triangle(vec2(x - r, y), vec2(x + r, y), vec2(x, y + w), c);
    draw_triangle(vec2(x, y - r), vec2(x, y + r), vec2(x - w, y), c);
    draw_triangle(vec2(x, y - r), vec2(x, y + r), vec2(x + w, y), c);
}

pub(super) fn draw_tiles(blip: &Blip, art: &art::Art, g: &Game, ox: f32, oy: f32) {
    for r in 0..ROWS {
        for c in 0..COLS {
            if !(g.tiles[r][c] || c == 0 || c == COLS - 1) {
                continue;
            }
            let border = c == 0 || c == COLS - 1 || r == 0 || r == ROWS - 1;
            if (r == 0 || r == ROWS - 1) && !g.tiles[r][c] {
                continue;
            }
            let (x, y) = (c as f32 * TILE + ox, HUD + r as f32 * TILE + oy);
            // a platform's tiles join into one long cushion, rounded at its two ends
            let joins = |c: usize| c > 0 && c < COLS - 1 && g.tiles[r][c];
            let piece = if border {
                4
            } else {
                match (joins(c - 1), joins(c + 1)) {
                    (false, false) => 0,
                    (false, true) => 1,
                    (true, true) => 2,
                    (true, false) => 3,
                }
            };
            art.tile(blip, g.round, piece, x, y, TILE);
        }
    }
}

// ---- plush toys ------------------------------------------------------------

pub(super) const EAT_SECS: f32 = 0.6;
pub(super) const EAT_FLY: f32 = 0.15;
pub(super) const CHOMPS: [f32; 2] = [0.28, 0.45];

pub(super) fn mouth_pos(p: &Player) -> (f32, f32) {
    (p.cx() + p.face * 10.5 * TOY_SCALE, p.y + P_H - 9.0 * TOY_SCALE)
}

pub(super) fn draw_bubble(art: &art::Art, b: &Bubble, ox: f32, oy: f32, t: f32, round: usize) {
    let (x, y) = (b.x + ox, b.y + oy);
    let life = if b.trapped.is_some() { trap_life(round) } else { FREE_LIFE };
    let escaping = b.trapped.is_some() && b.age > life - 1.6;
    let shake = if escaping { (t * 50.0).sin() * 1.5 } else { 0.0 };
    let grow = if b.phase == Phase::Shoot && b.trapped.is_none() {
        0.35 + 0.65 * blip::ease_out_cubic(b.age / 0.12)
    } else {
        1.0
    };
    let r = (b.radius() + (b.wob * 3.0).sin() * 0.6) * grow;
    let (mut rx, mut ry) = (r * (1.0 + b.squish * 0.25), r * (1.0 - b.squish * 0.25));
    if let Some((kind, angry)) = b.trapped {
        // A plush squished into its bubble: too big for it, so it fills it wall to wall, squashed short and wide,
        // and the bubble bulges round it. The two squeeze against each other in a slow breath.
        let breath = (t * 2.6 + b.wob).sin() * if escaping { 2.0 } else { 1.0 };
        rx *= 1.12 + 0.035 * breath;
        ry *= 1.0 - 0.035 * breath;
        // the whole toy, ears to feet and arm to arm (29 x 31 sprite-grid px), just fits inside the glass
        let (sx, sy) = (rx * 2.0 * 0.96 / 29.0, ry * 2.0 * 0.96 / 31.0);
        // it rocks a little (cute, not scary); about to break free it wriggles fast
        let tilt = if escaping { (t * 26.0).sin() * 0.14 } else { (t * 3.0 + b.wob).sin() * 0.05 };
        // caught: hands pressed up on the glass, patting it; breaking out, they flail
        let pat = if escaping { (t * 20.0 + b.wob).sin() * 0.8 } else { (t * 5.0 + b.wob).sin() * 0.18 };
        let live = art::Live { arms: [2.1 + pat, 2.1 - pat], blink: (t * 0.3 + b.wob * 0.17).fract() < 0.045 };
        art.toy_in_bubble(kind, angry, x + shake, y, sx, sy, tilt, live);
        // cheeks flattened on the glass: a pale pressed patch where each side touches
        for side in [-1.0, 1.0] {
            draw_ellipse(x + shake + side * rx * 0.86, y + ry * 0.1, rx * 0.12, ry * 0.42, 0.0, rgba(1.0, 0.95, 1.0, 0.28));
        }
    }
    let rim = if escaping && (t * 10.0) as i32 % 2 == 0 {
        rgba(1.0, 0.3, 0.3, 0.95)
    } else {
        hsv(b.wob * 0.15 + x * 0.002, 0.45, 1.0, 0.85)
    };
    let body = if b.trapped.is_some() { 0.16 } else { 0.22 };
    draw_glass(x + shake, y, rx, ry, b.owner, body, rim);
}

pub(super) const HEART_SECS: f32 = 1.1;

/// A heart: it pops in big, floats, and fades out at the top of its rise.
pub(super) fn draw_heart(h: &Heart, ox: f32, oy: f32, scale: f32, c: BlipColor) {
    use blip::macroquad::{math::vec2, shapes::{draw_circle, draw_triangle}};
    let pop = 1.0 + 0.5 * (1.0 - blip::ease_out_cubic((h.t / 0.15).min(1.0)));
    let a = ((HEART_SECS - h.t) / 0.4).clamp(0.0, 1.0).max(if scale < 1.0 { 1.0 } else { 0.0 });
    let s = 5.0 * pop * scale;
    let (x, y) = (h.x + ox, h.y + oy);
    let c = BlipColor { a: c.a * a, ..c };
    draw_circle(x - s * 0.55, y, s * 0.62, c);
    draw_circle(x + s * 0.55, y, s * 0.62, c);
    draw_triangle(vec2(x - s * 1.15, y + s * 0.2), vec2(x + s * 1.15, y + s * 0.2), vec2(x, y + s * 1.5), c);
    draw_circle(x - s * 0.6, y - s * 0.15, s * 0.2, rgba(1.0, 1.0, 1.0, 0.8 * c.a));
}

/// A bubble's glass: tinted with its owner's colour, a rim, and the window's shine.
pub(super) fn draw_glass(x: f32, y: f32, rx: f32, ry: f32, owner: usize, body: f32, rim: BlipColor) {
    // pale pastel glass (a deep tint over the dusk sky read as grey)
    let tint = if owner == 0 { (200, 255, 215) } else { (200, 228, 255) };
    draw_ellipse(x, y, rx, ry, 0.0, col(tint, body));
    blip::macroquad::shapes::draw_ellipse_lines(x, y, rx, ry, 0.0, 1.8, rim);
    let r = (rx + ry) / 2.0;
    draw_ellipse(x - r * 0.38, y - r * 0.42, r * 0.3, r * 0.17, -35.0, rgba(1.0, 1.0, 1.0, 0.85));
    blip::macroquad::shapes::draw_circle(x + r * 0.45, y + r * 0.35, 1.2, rgba(1.0, 1.0, 1.0, 0.6));
}

/// Candy colours, picked per drop so a shower of them is a pick-and-mix.
pub(super) const CANDY: [(u8, u8, u8); 6] =
    [(255, 120, 160), (120, 200, 255), (255, 200, 80), (150, 230, 140), (200, 150, 255), (255, 150, 90)];

/// A dropped treat, sweeter the bigger the chain that made it (see VALUE):
/// a wrapped bonbon, a swirled lollipop, a sprinkled donut, a cupcake with
/// a cherry on top. Each shines, and twinkles now and then.
pub(super) fn draw_fruit(f: &Fruit, ox: f32, oy: f32) {
    if f.t > 7.0 && (f.t * 10.0) as i32 % 2 == 0 {
        return;
    }
    let bob = if f.on_ground { (f.t * 4.0).sin() * 1.0 } else { 0.0 };
    draw_candy(f.kind, candy_pick(f), f.t, f.x + 8.0 + ox, f.y + 8.0 + oy + bob, true);
}

/// Which colour a candy is: fixed by where it dropped.
pub(super) fn candy_pick(f: &Fruit) -> usize {
    ((f.x * 7.0 + f.kind as f32 * 13.0) as i32).rem_euclid(CANDY.len() as i32) as usize
}

/// A candy of `kind` in colour `pick`, centred on (`x`, `y`); `t` drives
/// its shine, `halo` the glow it has lying on the floor.
pub(super) fn draw_candy(kind: usize, pick: usize, t: f32, x: f32, y: f32, halo: bool) {
    use blip::macroquad::shapes::{draw_circle, draw_line};
    let c = CANDY[pick];
    let c2 = CANDY[(pick + 2) % CANDY.len()];
    if halo {
        // a warm halo behind, so a treat is found at a glance
        let glow = 0.07 + 0.03 * (t * 5.0).sin();
        draw_circle(x, y + 1.0, 13.0, rgba(1.0, 0.8, 0.6, glow));
        draw_circle(x, y + 1.0, 10.0, rgba(1.0, 0.85, 0.65, glow));
    }
    let shine = rgba(1.0, 1.0, 1.0, 0.85);
    match kind {
        0 => {
            // a wrapped bonbon: a round sweet, twisted cellophane ends
            for s in [-1.0f32, 1.0] {
                draw_triangle(
                    vec2(x + s * 6.0, y),
                    vec2(x + s * 12.0, y - 5.0),
                    vec2(x + s * 12.0, y + 5.0),
                    col(tint(c, 1.15), 1.0),
                );
                draw_line(x + s * 9.0, y - 3.0, x + s * 9.0, y + 3.0, 1.0, col(tint(c, 0.75), 1.0));
            }
            draw_circle(x, y, 7.5, col(tint(c, 0.8), 1.0));
            draw_circle(x, y, 6.6, col(c, 1.0));
            for k in 0..3 {
                // candy stripes
                let dx = -4.0 + k as f32 * 4.0;
                draw_line(x + dx - 1.5, y - 5.0, x + dx + 1.5, y + 5.0, 1.6, col((255, 255, 255), 0.55));
            }
            draw_ellipse(x - 2.6, y - 3.0, 2.4, 1.4, -30.0, shine);
        }
        1 => {
            // a lollipop: a spiral of two colours on a stick
            draw_line(x, y + 4.0, x + 1.0, y + 14.0, 2.2, col((250, 245, 235), 1.0));
            draw_circle(x, y - 1.0, 8.0, col(tint(c, 0.8), 1.0));
            draw_circle(x, y - 1.0, 7.2, col(c, 1.0));
            let turn = t * 1.5;
            for k in 0..28 {
                let u = k as f32 / 28.0;
                let a = u * 12.0 + turn;
                let r = 1.0 + u * 5.6;
                draw_circle(x + a.cos() * r, y - 1.0 + a.sin() * r, 1.2, col(c2, 1.0));
            }
            draw_ellipse(x - 3.0, y - 4.4, 2.2, 1.3, -30.0, shine);
        }
        2 => {
            // a donut: soft dough, glossy icing, sprinkles
            draw_ellipse(x, y + 1.0, 10.0, 7.5, 0.0, col((215, 155, 95), 1.0));
            draw_ellipse(x, y, 9.0, 6.4, 0.0, col(c, 1.0));
            draw_ellipse(x, y + 0.4, 3.2, 2.2, 0.0, col((90, 50, 40), 1.0));
            for k in 0..9 {
                let a = k as f32 * 0.7 + 0.3;
                let r = 5.4 + (k % 2) as f32 * 1.4;
                let (sx, sy) = (x + a.cos() * r, y + a.sin() * r * 0.7);
                let sc = CANDY[(pick + 1 + k) % CANDY.len()];
                draw_line(sx - 1.0, sy - 0.6, sx + 1.0, sy + 0.6, 1.4, col(sc, 1.0));
            }
            draw_ellipse(x - 4.0, y - 3.0, 2.6, 1.2, -15.0, shine);
        }
        _ => {
            // a cupcake: a pleated case, swirled frosting, a cherry
            draw_triangle(vec2(x - 8.0, y + 1.0), vec2(x + 8.0, y + 1.0), vec2(x + 6.0, y + 10.0), col(c2, 1.0));
            draw_triangle(vec2(x - 8.0, y + 1.0), vec2(x - 6.0, y + 10.0), vec2(x + 6.0, y + 10.0), col(c2, 1.0));
            for k in 0..5 {
                let px = x - 6.0 + k as f32 * 3.0;
                draw_line(px, y + 2.0, px + (k as f32 - 2.0) * 0.4, y + 9.5, 1.0, col(tint(c2, 0.75), 1.0));
            }
            for (dy, r) in [(-1.0f32, 8.0f32), (-4.5, 6.0), (-7.5, 3.8)] {
                draw_ellipse(x, y + dy, r, r * 0.6, 0.0, col(tint(c, 0.9), 1.0));
                draw_ellipse(x, y + dy - 0.6, r * 0.9, r * 0.5, 0.0, col(c, 1.0));
            }
            draw_circle(x + 0.5, y - 11.0, 2.8, col((220, 30, 60), 1.0));
            draw_line(x + 0.5, y - 13.5, x + 2.5, y - 16.0, 1.0, col((80, 140, 50), 1.0));
            draw_circle(x - 0.4, y - 11.8, 0.8, shine);
            draw_ellipse(x - 3.0, y - 4.5, 2.0, 1.0, -15.0, shine);
        }
    }
    // a sugar twinkle now and then
    let tw = (t * 2.0 + pick as f32).fract();
    if tw < 0.12 {
        let a = 1.0 - tw / 0.12;
        let (sx, sy) = (x + 6.0, y - 7.0);
        draw_line(sx - 3.0, sy, sx + 3.0, sy, 1.0, rgba(1.0, 1.0, 1.0, a));
        draw_line(sx, sy - 3.0, sx, sy + 3.0, 1.0, rgba(1.0, 1.0, 1.0, a));
    }
}

pub(super) fn draw_skull(s: &Skull, ox: f32, oy: f32, t: f32) {
    let (x, y) = (s.x + ox, s.y + oy + (t * 4.0).sin() * 2.0);
    use blip::macroquad::shapes::draw_circle;
    for k in 0..3 {
        draw_circle(x, y, 16.0 + k as f32 * 5.0, rgba(0.6, 0.2, 0.9, 0.12));
    }
    draw_ellipse(x, y, 12.0, 11.0, 0.0, rgba(0.95, 0.95, 1.0, 1.0));
    draw_ellipse(x, y + 8.0, 7.0, 4.0, 0.0, rgba(0.95, 0.95, 1.0, 1.0));
    for sgn in [-1.0f32, 1.0] {
        draw_ellipse(x + sgn * 4.5, y - 1.0, 3.5, 4.2, 0.0, rgba(0.15, 0.0, 0.25, 1.0));
        draw_circle(x + sgn * 4.5, y - 1.0, 1.4, hsv(t * 0.5, 0.7, 1.0, 1.0));
    }
    for k in -1..=1 {
        blip::macroquad::shapes::draw_rectangle(x + k as f32 * 3.0 - 1.0, y + 7.0, 2.0, 4.0, rgba(0.2, 0.1, 0.3, 1.0));
    }
}

pub(super) fn draw_world(blip: &Blip, art: &art::Art, g: &Game) {
    let t = now();
    let (ox, oy) =
        if g.shake > 0.0 { ((t * 90.0).sin() * g.shake * 8.0, (t * 77.0).cos() * g.shake * 8.0) } else { (0.0, 0.0) };
    draw_background(blip, g);
    draw_tiles(blip, art, g, ox, oy);
    for f in &g.fruits {
        draw_fruit(f, ox, oy);
    }
    for e in &g.enemies {
        if !e.active {
            continue;
        }
        let s = blip::ease_out_cubic(e.pop_in);
        let shake = if e.windup > 0.0 || e.crouch > 0.0 { (e.t * 70.0).sin() * 1.4 } else { 0.0 };
        let (sx, sy, tilt, bob) = plush_pose(e);
        let live = plush_live(e);
        art.enemy(
            e.kind,
            e.x + ox + shake,
            e.y + oy + bob,
            s.max(0.05) * PLUSH_SCALE,
            s,
            e.angry,
            // the way it is going; a ghost drifting nearly still keeps its heading
            if e.vx.abs() > 1.0 { e.vx.signum() } else { e.dir },
            (sx, sy, tilt),
            live,
        );
    }
    for (i, p) in g.p.iter().enumerate() {
        if !p.joined {
            continue;
        }
        if !p.alive && p.dead_t <= 0.0 {
            continue;
        }
        if p.safe > 0.0 && (p.safe * 12.0) as i32 % 2 == 0 && p.dead_t <= 0.0 {
            continue;
        }
        // a waddle while walking: a small hop each step
        let mut bob = if p.on_ground && p.vx.abs() > 20.0 { -p.walk.sin().abs() * 1.6 * TOY_SCALE } else { 0.0 };
        let mut squash = p.squash;
        if g.state == State::Clear && p.alive && p.on_ground && g.state_t > 0.3 {
            // round cleared: both dragons bounce for joy, squashing as they touch down
            let ph = (g.state_t * 6.5 + i as f32 * 1.3).sin().abs();
            bob -= ph * 9.0;
            squash *= 0.88 + ph * 0.2;
        }
        art.player(blip, p.cx() + ox, p.y + P_H + oy + bob, i, TOY_SCALE, p.face, squash, p.blink < 0.0);
        // the candy flying into the open mouth
        if p.eat >= 0.0 && p.eat < EAT_FLY {
            let (mx, my) = mouth_pos(p);
            let k = blip::ease_out_cubic(p.eat / EAT_FLY);
            let (fx, fy) = (p.eaten.x + 8.0, p.eaten.y + 8.0);
            draw_candy(
                p.eaten.kind,
                candy_pick(&p.eaten),
                1.0,
                fx + (mx - fx) * k + ox,
                fy + (my - fy) * k + oy,
                false,
            );
        }
    }
    for b in &g.bubbles {
        draw_bubble(art, b, ox, oy, t, g.round);
    }
    if g.skull.active {
        draw_skull(&g.skull, ox, oy, t);
    }
    for s in &g.shots {
        let (x, y) = (s.x + ox, s.y + oy);
        if s.spark {
            let a = (4.0 - s.t).clamp(0.0, 1.0);
            blip::macroquad::shapes::draw_circle(x, y, 7.0, rgba(1.0, 0.45, 0.8, 0.25 * a));
            blip::macroquad::shapes::draw_poly(x, y, 4, 5.0, t * 400.0, rgba(1.0, 0.7, 0.95, a));
            blip::macroquad::shapes::draw_circle(x, y, 2.0, rgba(1.0, 1.0, 1.0, a));
        } else {
            // a rolling rock: the highlight turns as it goes
            blip::macroquad::shapes::draw_circle(x + 1.0, y + 2.0, 6.0, rgba(0.1, 0.05, 0.15, 0.4));
            blip::macroquad::shapes::draw_circle(x, y, 6.0, col((150, 120, 110), 1.0));
            let a = s.x / 6.0;
            blip::macroquad::shapes::draw_circle(x + a.cos() * 2.5, y + a.sin() * 2.5, 1.6, col((110, 85, 80), 1.0));
            blip::macroquad::shapes::draw_circle(x - 2.0, y - 2.2, 1.5, rgba(1.0, 0.95, 0.9, 0.7));
        }
    }
    for p in g.parts.iter() {
        let a = (p.life / p.max_life).clamp(0.0, 1.0);
        let c = BlipColor { a: p.data.c.a * a, ..p.data.c };
        if p.data.ring {
            let r = p.data.size + (1.0 - a) * 16.0;
            blip::macroquad::shapes::draw_circle_lines(p.x + ox, p.y + oy, r, 2.0 * a + 0.5, c);
        } else if p.data.star {
            let r = p.data.size * (0.6 + 0.4 * a);
            blip::macroquad::shapes::draw_poly(p.x + ox, p.y + oy, 4, r, t * 200.0, c);
        } else {
            blip::macroquad::shapes::draw_circle(p.x + ox, p.y + oy, p.data.size, c);
        }
    }
    for h in &g.hearts {
        if h.t > 0.0 {
            draw_heart(h, ox, oy, 1.0, col((255, 110, 160), 1.0));
        }
    }
    for p in &g.pops {
        let a = (1.4 - p.t).clamp(0.0, 1.0);
        let s = format!("{}", p.value);
        let sz = 2.0 * (1.0 + 0.5 * (1.0 - blip::ease_out_cubic(p.t / 0.18)));
        let w = s.len() as f32 * 6.0 * sz;
        let x = (p.x - w / 2.0).clamp(TILE + 2.0, WIN_W as f32 - TILE - 2.0 - w);
        blip.draw_text(&s, x + 1.0, p.y + 1.0, sz, col(PLUM, a * 0.8));
        blip.draw_text(&s, x, p.y, sz, BlipColor { a, ..p.c });
    }
}

/// How a free plush toy moves, drawn only (its hit box is still): (width and height squash, tilt about its feet,
/// lift). Walking it waddles, rocking side to side with a little hop each step; it squats before a jump,
/// stretches as it rises and flattens as it falls; a floater sways and jiggles like jelly; all of them breathe.
pub(super) fn plush_pose(e: &Enemy) -> (f32, f32, f32, f32) {
    let breathe = (e.t * 3.1 + e.x * 0.05).sin() * 0.035;
    let squash = |s: f32| (1.0 / s, s);
    if e.crouch > 0.0 {
        // a squat before the spring: wide and low
        let (sx, sy) = squash(0.8);
        return (sx, sy, 0.0, 0.0);
    }
    if e.kind == Kind::Ghost {
        let jiggle = (e.t * 5.0).sin() * 0.06;
        let (sx, sy) = squash(1.0 + jiggle + breathe);
        return (sx, sy, (e.t * 2.0).sin() * 0.14, (e.t * 3.0).sin() * 2.5);
    }
    if !e.on_ground {
        // stretched going up, flattened coming down
        let (sx, sy) = squash((1.0 - e.vy / 1600.0).clamp(0.82, 1.22));
        return (sx, sy, e.vx.signum() * 0.1, 0.0);
    }
    if e.vx.abs() > 5.0 {
        // a waddle: one rock and one little hop a step, squashing as each foot lands
        let step = e.x * 0.22;
        let land = step.cos().abs();
        let (sx, sy) = squash(1.0 + 0.09 * (1.0 - land) - 0.05 * land + breathe);
        return (sx, sy, step.sin() * 0.16, -(1.0 - land) * 3.0);
    }
    let (sx, sy) = squash(1.0 + breathe);
    (sx, sy, (e.t * 1.3).sin() * 0.03, 0.0)
}

/// A free toy's arms and blinks: the arms swing as it waddles, go up in a cheer when it leaps, paddle as a
/// floater drifts, and now and then, standing about, it waves; it blinks at its own random moments.
pub(super) fn plush_live(e: &Enemy) -> art::Live {
    let calm = (e.t * 2.0 + e.seed).sin() * 0.08;
    // its own clock: a wave about one beat in six, a blink every 3 s or so
    let beat = (e.t * 0.23 + e.seed * 0.071).fract();
    let blink = (e.t * 0.31 + e.seed * 0.13).fract() < 0.045;
    let waving = |rest: f32| {
        let w = 2.6 + (e.t * 13.0).sin() * 0.4;
        if (e.seed as i32) % 2 == 0 { [w, rest] } else { [rest, w] }
    };
    let arms = if e.crouch > 0.0 {
        [0.15, 0.15]
    } else if !e.on_ground && e.kind != Kind::Ghost {
        let wig = (e.t * 18.0).sin() * 0.2;
        [2.5 + wig, 2.5 - wig]
    } else if e.kind == Kind::Ghost {
        let p = (e.t * 4.0 + e.seed).sin() * 0.55;
        if beat < 0.15 { waving(1.0) } else { [1.1 + p, 1.1 - p] }
    } else if e.vx.abs() > 5.0 {
        let swing = (e.x * 0.22).sin() * 0.5;
        [0.55 + swing, 0.55 - swing]
    } else if beat < 0.18 {
        waving(0.35 + calm)
    } else {
        [0.35 + calm, 0.35 - calm]
    };
    art::Live { arms, blink }
}

// ---- cosy text -------------------------------------------------------------
// Warm pastels on soft rounded panels, a plum shadow instead of black: the
// text sits on something, never straight over a platform or a monster.
pub(super) const CREAM: (u8, u8, u8) = (255, 244, 222);
pub(super) const PEACH: (u8, u8, u8) = (255, 196, 150);
pub(super) const PINK: (u8, u8, u8) = (255, 170, 200);
pub(super) const MINT: (u8, u8, u8) = (170, 240, 190);
pub(super) const SKY: (u8, u8, u8) = (165, 215, 255);
pub(super) const PLUM: (u8, u8, u8) = (60, 20, 70);

pub(super) fn text_w(text: &str, sz: f32) -> f32 {
    text.chars().count() as f32 * 6.0 * sz - sz
}

/// Text in the blip message box, centred on the screen, popping in with `pop`.
pub(super) fn cosy(blip: &Blip, text: &str, y: f32, sz: f32, c: (u8, u8, u8), pop: f32, a: f32) {
    if a <= 0.01 {
        return;
    }
    let s = sz * (0.7 + 0.3 * blip::ease_out_cubic(pop));
    let (w, h) = (text_w(text, s), 7.0 * s);
    let (px, py) = (8.0 + s * 2.5, 6.0 + s * 1.5);
    let cx = WIN_W as f32 / 2.0;
    let top = y + (7.0 * sz - h) / 2.0;
    blip.draw_panel(cx - w / 2.0 - px, top - py, w + px * 2.0, h + py * 2.0, a);
    blip.draw_text(text, cx - w / 2.0 + s * 0.5, top, s, col(c, a));
}

/// Plain text with the soft plum shadow, left-aligned (HUD, small print).
pub(super) fn soft(blip: &Blip, text: &str, x: f32, y: f32, sz: f32, c: (u8, u8, u8), a: f32) {
    blip.draw_text(text, x + sz * 0.5, y + sz * 0.5, sz, col(PLUM, 0.9 * a));
    blip.draw_text(text, x, y, sz, col(c, a));
}

pub(super) fn draw_scores(blip: &Blip, g: &Game) {
    // one row: P1 on the left, the round in the middle, P2 on the right
    blip.draw_hud_bar(HUD);
    // A whole-pixel size: at 1.5 the font's rows were uneven, and a phone
    // (the picture at 0.57) showed 6px text that could not be read.
    let sz = 2.0;
    let y = (HUD - 7.0 * sz) / 2.0 - 1.0;
    for (i, colr) in [(0usize, MINT), (1, SKY)] {
        let p = &g.p[i];
        if !p.joined {
            // player two's seat, waiting: a gentle pulse, not a blink
            let a = 0.55 + 0.45 * (now() * 3.0).sin().abs();
            let t = "2P JOIN";
            soft(blip, t, WIN_W as f32 - 10.0 - text_w(t, sz), y, sz, SKY, a);
            continue;
        }
        let score = format!("{}", p.score);
        let (sw, lives) = (text_w(&score, sz), p.lives.max(0));
        let lw = lives as f32 * 12.0;
        let x0 = if i == 0 { 22.0 } else { WIN_W as f32 - 10.0 - sw - 8.0 - lw };
        blip.draw_coin(x0 - 12.0, (HUD / 2.0 - 6.0).round(), 1.0);
        soft(blip, &score, x0, y, sz, colr, 1.0);
        for k in 0..lives {
            // the blip heart, a life each
            blip.draw_heart(x0 + sw + 6.0 + k as f32 * 12.0, (HUD / 2.0 - 5.0).round(), 1.0, true);
        }
    }
    let round = format!("ROUND {}", g.round + 1);
    soft(blip, &round, (WIN_W as f32 - text_w(&round, sz)) / 2.0, y, sz, PEACH, 1.0);
}

pub(super) fn draw_title(blip: &Blip, art: &art::Art, hi: i32) {
    let t = now();
    art.title_background(blip);
    draw_bokeh(blip, &PALETTES[0]);
    // bubbles drifting up behind the logo
    for i in 0..14 {
        let s = i as f32 * 53.1;
        let x = (s * 11.3) % WIN_W as f32;
        let y = WIN_H as f32 - ((s * 9.1 + t * 30.0) % (WIN_H as f32 + 40.0)) + 20.0;
        // the sheen drifts round the colours as each bubble wobbles
        let (wob, r) = (t + s, 12.0 + ((t + s) * 3.0).sin() * 0.6);
        draw_glass(x, y, r, r, i % 2, 0.22, hsv(wob * 0.15 + x * 0.002, 0.45, 1.0, 0.85));
    }
    for letter in 0..7 {
        let bob = (t * 3.0 + letter as f32 * 0.6).sin() * 5.0;
        art.title_letter(blip, letter, 186.0 + letter as f32 * 36.0, 88.0 + DY + bob);
    }
    // the two of them, bouncing
    for (i, x) in [WIN_W as f32 / 2.0 - 70.0, WIN_W as f32 / 2.0 + 70.0].iter().enumerate() {
        let hop = ((t * 3.2 + i as f32 * 1.6).sin()).max(0.0);
        let foot = 278.0 + DY - hop * 34.0;
        // the two of them face each other
        // squashed on touchdown, stretched in the air; each blinks now and then
        let squash = if hop < 0.15 { 0.82 + hop } else { 1.0 + hop * 0.1 };
        let blink = (t + i as f32 * 1.7) % 3.3 < 0.12;
        art.player(blip, *x, foot, i, 2.4, if i == 0 { 1.0 } else { -1.0 }, squash, blink);
    }
    let glow = 0.65 + 0.35 * (t * 2.5).sin().abs();
    art.title_panels(blip, web::controls(), glow);
    if hi > 0 {
        let h = format!("HI {hi}");
        soft(blip, &h, (WIN_W as f32 - text_w(&h, 2.0)) / 2.0, 428.0 + DY, 2.0, PEACH, 1.0);
    }
}

/// Draw the frame: the title, or the world with the words of its moment over it.
/// A round cleared: pastel confetti, hearts and stars among the paper, tumbling down from the top for the
/// clear's first seconds. Placed by the clock and each piece's number, so no random numbers are drawn.
pub(super) fn draw_confetti(t: f32) {
    use blip::macroquad::{math::vec2, shapes::draw_triangle};
    const COLOURS: [(u8, u8, u8); 6] = [(255, 160, 210), (150, 230, 255), (255, 230, 140), (170, 255, 190), (220, 180, 255), (255, 190, 160)];
    for i in 0..70u32 {
        let h = i.wrapping_mul(2_654_435_761).rotate_left(11);
        let start = (h % 900) as f32 / 1000.0;
        let age = t - start;
        if age < 0.0 { continue; }
        let fall = 60.0 + (h >> 10) as f32 % 70.0;
        let y = HUD - 10.0 + age * fall;
        if y > WIN_H as f32 + 10.0 { continue; }
        let x = (h >> 4) as f32 % WIN_W as f32 + (age * 2.5 + i as f32).sin() * 14.0;
        let c = COLOURS[(h >> 20) as usize % COLOURS.len()];
        // Fades as the bubble wipe closes over it.
        let a = (1.0 - ((t - 2.4) / 0.6).clamp(0.0, 1.0)) * 0.95;
        let colour = col(c, a);
        match i % 7 {
            0 => draw_heart(&Heart { x, y, t: 1.0, sway: 0.0 }, 0.0, 0.0, 0.8, colour),
            1 => sparkle(x, y, 5.0, colour),
            _ => {
                // A paper square flipping as it falls: its width follows the flip.
                let (s, w) = (3.5, 3.5 * (age * 6.0 + i as f32).cos().abs().max(0.2));
                let (sn, cs) = (age * 3.0 + i as f32).sin_cos();
                let p = |u: f32, v: f32| vec2(x + u * cs - v * sn, y + u * sn + v * cs);
                draw_triangle(p(-w, -s), p(w, -s), p(w, s), colour);
                draw_triangle(p(-w, -s), p(w, s), p(-w, s), colour);
            }
        }
    }
}

pub(super) fn draw(blip: &Blip, art: &art::Art, g: &Game, hi: i32) {
    blip.clear(BlipColor::new(0.0, 0.0, 0.0, 1.0));
    match g.state {
        State::Title => draw_title(blip, art, hi),
        _ => {
            draw_world(blip, art, g);
            draw_scores(blip, g);
            let st = g.state_t;
            // the end screens dim the world so their words sit on calm
            if matches!(g.state, State::Over | State::Won) {
                blip.fill_rect(
                    0.0,
                    HUD,
                    WIN_W as f32,
                    WIN_H as f32 - HUD,
                    col((20, 8, 28), 0.55 * (st * 2.0).min(1.0)),
                );
            }
            match g.state {
                State::Intro => {
                    cosy(
                        blip,
                        &format!("ROUND {}", g.round + 1),
                        170.0 + DY,
                        3.5,
                        PEACH,
                        st * 3.0,
                        (st * 4.0).min(1.0),
                    );
                    if st > 0.7 {
                        cosy(
                            blip,
                            "READY!",
                            236.0 + DY,
                            2.2,
                            CREAM,
                            (st - 0.7) * 3.0,
                            ((st - 0.7) * 4.0).min(1.0),
                        );
                    }
                }
                State::Play if g.hurry && g.round_t < hurry_at(g.round) + 2.4 => {
                    let a = 0.6 + 0.4 * (g.round_t * 8.0).sin().abs();
                    cosy(blip, "HURRY UP!", 196.0 + DY, 3.2, (255, 150, 150), 1.0, a);
                }
                _ => {}
            }
            if g.chain.1 > 0.0 {
                let text = format!("{} CHAIN!", g.chain.0);
                cosy(blip, &text, 262.0 + DY, 2.6, CREAM, (1.2 - g.chain.1) * 5.0, g.chain.1.min(1.0));
            }
            if g.state == State::Clear { draw_confetti(g.state_t); }
            // bubble wipe: out at the end of a round, back in at the start of the next
            let wipe = match g.state {
                State::Clear => ((g.state_t - 2.6) / 0.8).clamp(0.0, 1.0),
                State::Intro => 1.0 - (g.state_t / 0.7).clamp(0.0, 1.0),
                _ => 0.0,
            };
            if wipe > 0.0 {
                let pal = &PALETTES[g.round];
                for i in 0..12 {
                    for j in 0..12 {
                        let (cx, cy) = (i as f32 * 60.0, j as f32 * 57.0);
                        let r = wipe * 46.0 * (0.8 + 0.2 * ((i * 7 + j * 3) % 5) as f32 / 4.0);
                        blip::macroquad::shapes::draw_circle(cx, cy, r, col(pal.sky0, 1.0));
                        blip::macroquad::shapes::draw_circle_lines(cx, cy, r, 2.0, col(pal.glow, 0.5 * wipe));
                    }
                }
            }
            match g.state {
                State::Clear => {
                    cosy(blip, "CLEAR!", 186.0 + DY, 4.0, CREAM, st * 3.0, (st * 4.0).min(1.0));
                    if g.extend && st > 0.6 {
                        cosy(
                            blip,
                            "EXTEND  +1 LIFE",
                            246.0 + DY,
                            1.8,
                            PEACH,
                            (st - 0.6) * 3.0,
                            ((st - 0.6) * 4.0).min(1.0),
                        );
                    }
                }
                State::Over => {
                    cosy(blip, "GAME OVER", 160.0 + DY, 3.8, PINK, st * 2.0, (st * 3.0).min(1.0));
                    if hi > 0 {
                        cosy(
                            blip,
                            &format!("HI {hi}"),
                            222.0 + DY,
                            1.5,
                            PEACH,
                            1.0,
                            (st - 1.0).clamp(0.0, 1.0),
                        );
                    }
                    if g.cont.showing() {
                        cosy(
                            blip,
                            &format!("PLAY ON? {}", g.cont.seconds()),
                            276.0 + DY,
                            2.0,
                            CREAM,
                            1.0,
                            0.65 + 0.35 * (st * 2.5).sin().abs(),
                        );
                    }
                }
                State::Won => {
                    cosy(blip, "ALL ROUNDS CLEAR!", 120.0 + DY, 2.6, CREAM, st * 2.0, (st * 3.0).min(1.0));
                    cosy(
                        blip,
                        "HAPPY END",
                        192.0 + DY,
                        3.8,
                        PEACH,
                        (st - 0.5) * 2.0,
                        ((st - 0.5) * 3.0).clamp(0.0, 1.0),
                    );
                    if st > 2.5 {
                        cosy(
                            blip,
                            "PRESS BUBBLE",
                            276.0 + DY,
                            2.0,
                            CREAM,
                            1.0,
                            0.65 + 0.35 * (st * 2.5).sin().abs(),
                        );
                    }
                }
                _ => {}
            }
        }
    }
}
