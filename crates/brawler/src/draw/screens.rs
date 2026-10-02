//! Everything that is not a fight: the title, the select screen, the
//! billing before a match, the bonus round, the ending and the card's poster.

use super::*;

/// Text as a lit tube. blip's `draw_text_glow` is a one-pixel halo, lost
/// under a big logo: this blooms outward in colour over several passes, lays
/// fattened glass, and puts a near-white core inside.
pub(crate) fn neon(blip: &Blip, text: &str, y: f32, sz: f32, c: BlipColor, lit: f32) {
    let x = blip.text_cx(text, sz as i32) as f32;
    const RING: [(f32, f32); 8] = [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0),
                                   (-0.7, -0.7), (0.7, -0.7), (-0.7, 0.7), (0.7, 0.7)];
    for (r, a) in [(1.05f32, 0.09f32), (0.7, 0.13), (0.42, 0.20)] {
        let col = BlipColor { a: a * lit, ..c };
        for (dx, dy) in RING {
            blip.draw_text(text, x + dx * r * sz, y + dy * r * sz, sz, col);
        }
    }
    let glass = BlipColor { a: lit, ..c };
    for (dx, dy) in RING {
        blip.draw_text(text, x + dx * 0.3 * sz, y + dy * 0.3 * sz, sz, glass);
    }
    blip.draw_text(text, x, y, sz, BlipColor { a: lit, ..blend(c, BLIP_WHITE, 0.78) });
}

/// A tube's glow: a slow swell of a few percent, so a sign reads as lit
/// rather than as coloured text. No dropouts: on a sign this size they strobe.
pub(crate) fn tube(t: f32, phase: f32) -> f32 {
    0.96 + 0.04 * (t * 1.7 + phase).sin()
}

/// How long each pair of fighters holds the title screen.
pub(crate) const TITLE_TURN: f32 = 4.0;

/// One of the title's two fighters, `u` seconds into its turn: stands in,
/// shows a move, and takes the win before the next pair arrives.
pub(crate) fn title_fighter(who: usize, x: f32, facing: f32, u: f32, first: bool) -> Fighter {
    let mut f = Fighter::new(who, x, facing);
    // The left one kicks, then the right one answers with their special.
    let (mv, at) = if first { (MoveId::HighKick, 0.7) } else { (MoveId::Special, 1.7) };
    let m = move_data(mv);
    let len = (m.startup + m.active + m.recovery) * F;
    if u >= at && u < at + len {
        f.act = Act::Attack;
        f.mv = mv;
        f.t = u - at;
    } else if u >= 2.9 {
        f.act = Act::Victory;
        f.t = u - 2.9;
    }
    f
}

/// The title screen: the sign, the whole roster under it, and two fighters at
/// a time showing what they do. It still says only what is needed to start.
pub(crate) fn draw_title(blip: &Blip, g: &Game) {
    let now = g.now;
    // A night stage, knocked back so the sign is the brightest thing on it.
    draw_temple(blip, 0.0, now, 0.0);
    draw_floor(blip, 1, 0.0);
    blip.fill_rect(0.0, 0.0, WIN_W as f32, FLOOR_Y, BlipColor { r: 0.02, g: 0.01, b: 0.08, a: 0.70 });

    // Rays turning slowly behind the sign, the way a fight poster has them.
    let sign = BlipColor { r: 1.0, g: 0.16, b: 0.38, a: 1.0 };
    let (cx, cy) = (WIN_W as f32 / 2.0, 62.0);
    for j in 0..14 {
        let ang = j as f32 * std::f32::consts::TAU / 14.0 + now * 0.12;
        for k in -2..=2 {
            let a = ang + k as f32 * 0.035;
            blip.draw_line_ex(cx + a.cos() * 60.0, cy + a.sin() * 26.0,
                cx + a.cos() * 520.0, cy + a.sin() * 520.0, 9.0, BlipColor { a: 0.05, ..sign });
        }
    }

    // A new pair every few seconds, so the title shows the whole roster. They
    // arrive in a soft bloom of white.
    let n = FIGHTERS.len();
    let turn = (now / TITLE_TURN) as usize;
    let u = now % TITLE_TURN;
    let pair = [turn % n, (turn + n / 2) % n];
    for (i, (x, face)) in [(66.0f32, 1.0f32), (574.0, -1.0)].into_iter().enumerate() {
        let f = title_fighter(pair[i], x, face, u, i == 0);
        draw_body(blip, &f, now, 0.0, 0.0, i, true);
        if u < 0.3 {
            blip.fill_glow_circle(x, FLOOR_Y - f.height() * 0.5, f.height() * 0.7,
                BlipColor { a: 0.25 * (1.0 - u / 0.3), ..BLIP_WHITE });
        }
    }

    let glow = tube(now, 0.0);
    for (r, a) in [(150.0f32, 0.045f32), (96.0, 0.05)] {
        blip.fill_glow_circle(cx, cy + 6.0, r, BlipColor { a: a * glow, ..sign });
    }
    neon(blip, "BRAWLER", 22.0, 9.0, sign, glow);
    neon(blip, "TEN FIGHTERS   ONE CHAMPION", 100.0, 2.0,
        BlipColor { r: 0.35, g: 0.95, b: 1.0, a: 1.0 }, tube(now, 2.1));

    // The roster, one face each, with the pair on stage lit.
    let gap = 38.0;
    let x0 = cx - gap * (n as f32 - 1.0) / 2.0;
    for (i, a) in FIGHTERS.iter().enumerate() {
        let head = V(x0 + i as f32 * gap - 2.0, 140.0);
        if pair.contains(&i) {
            blip.fill_glow_circle(head.0 + 2.0, head.1, 21.0, rgba(a.trim, 0.55));
        }
        let look = Look { bulk: a.bulk.min(1.15), size: 1.0, ..look_of(a, 0.0) };
        draw_head(blip, &look, 1.0, head, V(head.0 - 3.0, head.1 + 15.0), Mood::Calm);
    }

    // The mode, chosen on a stick and one button because that is all a
    // cabinet has. The line you are on is lit; the other is glass.
    let dark = BlipColor { r: 0.42, g: 0.30, b: 0.46, a: 1.0 };
    for (i, label) in ["1 PLAYER", "2 PLAYERS"].iter().enumerate() {
        let y = 176.0 + i as f32 * 32.0;
        if g.menu == i {
            neon(blip, label, y, 3.0, BlipColor { r: 1.0, g: 0.85, b: 0.25, a: 1.0 },
                0.62 + 0.38 * (now * 3.4).sin().max(0.0));
        } else {
            blip.draw_centered(label, y, 3.0, dark);
        }
    }

    // Both players' controls, padded to the same width so the columns line
    // up.
    let dim = BlipColor { r: 0.74, g: 0.78, b: 0.86, a: 1.0 };
    let p1c = BlipColor { r: 1.0, g: 0.35, b: 0.35, a: 1.0 };
    let p2c = BlipColor { r: 0.40, g: 0.72, b: 1.0, a: 1.0 };
    // Two buttons each: punch, then kick. The stick picks the height.
    const ROWS: [(&str, &str, bool); 2] = [
        ("P1", "W A S D   F PUNCH  G KICK", true),
        ("P2", "ARROWS    J PUNCH  K KICK", false),
    ];
    let x = (WIN_W as f32 - 32.0 * 12.0) / 2.0;
    for (i, (tag, body, one)) in ROWS.iter().enumerate() {
        let y = 250.0 + i as f32 * 22.0;
        // Player two greys out on the one-player line: the row is still
        // there to say the mode exists, without claiming those keys do
        // anything in the game you are about to start.
        let live = *one || g.menu == 1;
        let tint = if live { 1.0 } else { 0.45 };
        let col = if *one { p1c } else { p2c };
        blip.draw_text(tag, x, y, 2.0, BlipColor { a: tint, ..col });
        blip.draw_text(body, x + 48.0, y, 2.0, BlipColor { a: tint, ..dim });
    }
    blip.draw_centered("TOWARD AND BUTTON HITS HIGH   BOTH TOGETHER SPECIAL", 300.0, 1.0, dim);

    // The prompt, on the boards and blinking: the one line that has to be
    // found.
    if (now * 2.2).fract() < 0.7 {
        neon(blip, "PRESS PUNCH TO START", 348.0, 3.0,
            BlipColor { r: 1.0, g: 0.85, b: 0.25, a: 1.0 }, 1.0);
    }
    blip.draw_centered("W S CHOOSE     F OR SPACE START", 382.0, 1.0, dim);
    // The record to beat, when the kiosk knows of one.
    let hi = web::high_score();
    if hi.score > 0 {
        blip.draw_centered(&hi.label("HI"), 322.0, 1.0, BlipColor { r: 0.98, g: 0.90, b: 0.40, a: 1.0 });
    }
}

/// The select screen, laid out like the cabinets': a strip of faces along
/// the bottom, and above it whoever each cursor is on, standing full height
/// beside their numbers.
pub(crate) fn draw_select(blip: &Blip, g: &Game) {
    let versus = g.mode == Mode::Versus;
    // Behind them, dimmed, the home of whoever the cursor is on (in versus,
    // of player two: where that match will be), raised to the preview's floor.
    let ground = 236.0;
    let home = home_of(if versus { g.pick2 } else { g.pick });
    draw_backdrop(blip, home, ground - FLOOR_Y, g.now, 0.0);
    draw_floor(blip, home, ground - FLOOR_Y);
    blip.fill_rect(0.0, 0.0, WIN_W as f32, WIN_H as f32, BlipColor { r: 0.02, g: 0.02, b: 0.05, a: 0.48 });
    blip.fill_rect(0.0, 250.0, WIN_W as f32, WIN_H as f32 - 250.0, BlipColor { r: 0.03, g: 0.03, b: 0.06, a: 1.0 });
    stamp(blip, if versus { "CHOOSE YOUR FIGHTERS" } else { "CHOOSE YOUR FIGHTER" },
        20.0, 3.0, BLIP_YELLOW);
    let p1c = BlipColor { r: 1.0, g: 0.35, b: 0.35, a: 1.0 };
    let p2c = BlipColor { r: 0.40, g: 0.72, b: 1.0, a: 1.0 };
    let grey = BlipColor { r: 0.74, g: 0.77, b: 0.84, a: 1.0 };

    // The chosen fighters, drawn by the same code as in a match, so the
    // screen shows what you will control: player one on the left facing
    // right, player two mirrored.
    let sides: &[(usize, f32, f32, f32, BlipColor)] = if versus {
        &[(0, 84.0, 1.0, 164.0, p1c), (1, 556.0, -1.0, 356.0, p2c)]
    } else {
        &[(0, 214.0, 1.0, 306.0, BLIP_WHITE)]
    };
    for &(side, x, facing, tx, col) in sides {
        let who = g.picked(side);
        let a = FIGHTERS[who];
        let mut f = Fighter::new(who, x, facing);
        // Locked in, they celebrate; until then they stand in their guard.
        if g.locked[side] { f.act = Act::Victory; f.t = 1.0; }
        draw_body(blip, &f, g.now, ground - FLOOR_Y, 0.0, side, true);

        if versus {
            let lit = if g.locked[side] { 1.0 } else { 0.45 + 0.55 * (g.now * 5.0).sin().max(0.0) };
            let tag = if g.locked[side] { ["P1 READY", "P2 READY"][side] } else { ["P1", "P2"][side] };
            blip.draw_text(tag, tx, 112.0, 2.0, BlipColor { a: lit, ..col });
        }
        // A plate under the words: the stage behind them is not black.
        blip.fill_rect(tx - 8.0, 128.0, 142.0, 108.0, BlipColor { r: 0.02, g: 0.02, b: 0.05, a: 0.72 });
        blip.draw_text(a.name, tx, 136.0, 3.0, rgb(a.trim));
        let special = if a.build == Build::Turtle { format!("TURTLE CALL, {}", a.special_name) }
                      else { a.special_name.to_string() };
        blip.draw_text(&special, tx, 164.0, 1.0, grey);
        blip.draw_text(STAGE_NAMES[home_of(who)], tx, 174.0, 1.0, BlipColor { r: 0.55, g: 0.58, b: 0.66, a: 1.0 });
        // The three numbers that actually differ, as bars: a player choosing
        // between fighters needs the trade, not a biography. Full is the best
        // of the fair fighters; the invincible one runs off the end of all
        // three.
        let stats = [("PWR", a.power / 1.45), ("SPD", a.walk / 250.0),
                     ("HP ", if a.invincible { 2.0 } else { a.health as f32 / 130.0 })];
        for (r, (label, v)) in stats.iter().enumerate() {
            let sy = 190.0 + r as f32 * 14.0;
            blip.draw_text(label, tx, sy, 1.0, grey);
            blip.fill_rect(tx + 28.0, sy, 90.0 * v.clamp(0.0, 1.25), 8.0, rgb(a.trim));
            blip.draw_rect(tx + 28.0, sy, 90.0, 8.0, BlipColor { r: 0.3, g: 0.3, b: 0.36, a: 1.0 });
        }
    }

    // The roster: one face each.
    let n = FIGHTERS.len() as f32;
    let (cw, ch, gap) = (58.0, 58.0, 4.0);
    let x0 = (WIN_W as f32 - n * cw - (n - 1.0) * gap) / 2.0;
    let y0 = 258.0;
    for (i, a) in FIGHTERS.iter().enumerate() {
        let x = x0 + i as f32 * (cw + gap);
        let on1 = g.pick == i;
        let on2 = versus && g.pick2 == i;
        blip.fill_rect(x, y0, cw, ch, BlipColor { r: 0.12, g: 0.12, b: 0.17, a: 1.0 });
        let cx = x + cw / 2.0;
        // Faces all one size here, whatever the fighter's.
        let look = Look { bulk: a.bulk, size: 1.0, ..look_of(a, 0.0) };
        let head = V(cx - 2.0, y0 + 24.0);
        draw_head(blip, &look, 1.0, head, V(head.0 - 3.0, head.1 + 16.0), Mood::Calm);
        blip.fill_rect(x, y0 + ch - 14.0, cw, 14.0, BlipColor { r: 0.07, g: 0.07, b: 0.10, a: 1.0 });
        blip.draw_text(a.name, cx - text_w(a.name, 1.0) / 2.0, y0 + ch - 10.0, 1.0,
            if on1 || on2 { BLIP_WHITE } else { BlipColor { r: 0.6, g: 0.6, b: 0.66, a: 1.0 } });
        let frame = if on1 && on2 { BLIP_WHITE }
            else if on1 { if versus { p1c } else { rgb(a.trim) } }
            else if on2 { p2c }
            else { BlipColor { r: 0.3, g: 0.3, b: 0.35, a: 1.0 } };
        blip.draw_rect(x, y0, cw, ch, frame);
        if on1 || on2 {
            blip.draw_rect(x - 1.0, y0 - 1.0, cw + 2.0, ch + 2.0, frame);
            blip.draw_rect(x - 2.0, y0 - 2.0, cw + 4.0, ch + 4.0, frame);
        }
    }

    // The rules a player cannot find by pressing buttons, put where there is
    // nothing else to do but read them.
    blip.draw_centered("PUNCH AND KICK TOGETHER IS THE SPECIAL   DOWN AND PUNCH THE UPPERCUT", 326.0, 1.0, grey);
    blip.draw_centered("HOLD AWAY TO GUARD   WALK IN CLOSE AND PUNCH TO THROW", 340.0, 1.0, grey);
    blip.draw_centered("TAP TOWARD AS A BLOW LANDS TO TURN IT ASIDE", 354.0, 1.0, grey);
    let prompt = if !versus { "PRESS FIRE TO START" }
        else if !g.locked[0] || !g.locked[1] { "PRESS TO LOCK IN" }
        else { "FIGHT" };
    blip.draw_centered(prompt, 374.0, 2.0, BLIP_WHITE);
}

/// The kiosk card's picture (see brawler_card.sh): the first fighter of the
/// roster in a flying kick, out of a burst in his own red. Drawn small and
/// enlarged by the script, so it keeps the game's pixels.
pub(crate) fn draw_poster(blip: &Blip, g: &Game) {
    let c = |r: f32, g: f32, b: f32| BlipColor { r, g, b, a: 1.0 };
    let (w, h) = (WIN_W as f32, WIN_H as f32);
    blip.fill_rect(0.0, 0.0, w, h, c(0.30, 0.05, 0.07));
    // The burst comes from behind his back foot, where the kick started.
    let from = V(250.0, 250.0);
    blip.fill_sunburst(from.0, from.1, 18, 0.12, 620.0, [c(0.80, 0.15, 0.12), c(0.56, 0.08, 0.10)]);
    blip.fill_circle(from.0, from.1, 30.0, c(1.0, 0.86, 0.40));
    blip.fill_circle(from.0, from.1, 18.0, BLIP_WHITE);

    let mut f = Fighter::new(0, 300.0, 1.0);
    f.y = FLOOR_Y - 96.0;
    f.act = Act::Attack;
    f.shown = Act::Attack;
    f.mv = MoveId::FlyingKick;
    let m = move_data(MoveId::FlyingKick);
    f.t = (m.startup + m.active * 0.5) * F;
    draw_body(blip, &f, g.now, 0.0, 0.0, 0, false);
    // Where the heel is going.
    let hit = Spark { x: f.x + 96.0, y: f.y - 44.0, ttl: 0.12, life: 0.26, blocked: false };
    draw_splash(blip, &hit);
}

/// A framed head and shoulders, `pw` square with its top-left at
/// (`cx - pw / 2`, `py`), looking away from `side`.
pub(crate) fn draw_bust(blip: &Blip, a: &Archetype, cx: f32, py: f32, pw: f32, side: f32, back: BlipColor, mood: Mood) {
    let z = pw / 128.0;
    blip.fill_rect(cx - pw / 2.0 - 4.0, py - 4.0, pw + 8.0, pw + 8.0, INK);
    blip.fill_rect(cx - pw / 2.0, py, pw, pw, back);
    // No wind in the frame: a headband's tails would fly out of it.
    let look = Look { bulk: a.bulk * 3.0 * z, size: 3.0 * z, wind: (0.0, 1.0), ..look_of(a, 0.0) };
    let head = V(cx - side * 8.0 * z, py + 50.0 * z);
    let neck = V(head.0 + side * 8.0 * z, py + pw - 22.0 * z);
    // Shoulders across the foot of the frame, under the head.
    let (l, r) = (V(cx - 34.0 * z, py + pw + 4.0 * z), V(cx + 34.0 * z, py + pw + 4.0 * z));
    let bare = matches!(a.build, Build::Bare | Build::Giant | Build::Turtle);
    stroke(blip, l, r, 30.0 * z + LINE, 30.0 * z + LINE, INK);
    stroke(blip, l, r, 30.0 * z, 30.0 * z, look.c(if bare { look.skin } else { look.cloth }));
    draw_head(blip, &look, -side, head, neck, mood);
    // The shoulders run out of the frame; the sill squares them off.
    blip.fill_rect(cx - pw / 2.0 - 4.0, py + pw, pw + 8.0, 36.0 * z, INK);
    blip.draw_rect(cx - pw / 2.0, py, pw, pw, rgb(a.trim));
    blip.draw_rect(cx - pw / 2.0 - 1.0, py - 1.0, pw + 2.0, pw + 2.0, rgb(a.trim));
}

/// What each fighter has to say over a beaten opponent, in two lines.
pub(crate) const QUOTES: [[&str; 2]; FIGHTERS.len()] = [
    ["A GOOD FIGHT.", "TRAIN HARD AND FIND ME AGAIN."],
    ["I CRUSH ROCKS FOR BREAKFAST.", "YOU WERE DESSERT."],
    ["TOO SLOW.", "THE WIND WAS ON MY SIDE."],
    ["A LEADER WATCHES FIRST", "AND STRIKES SECOND."],
    ["NEXT TIME", "BRING SOMEBODY BIGGER."],
    ["I WORKED OUT YOUR MOVES", "BEFORE THE BELL."],
    ["THAT WAS FUN!", "WHO WANTS PIZZA."],
    ["STUCK FOR AN ANSWER.", "IT HAPPENS."],
    ["GAMMA STRONGEST.", "GROUND AGREES."],
    ["STAY DOWN.", "I WOULD RATHER NOT DO THAT TWICE."],
];

/// The winner's word on the match, along the foot of the screen.
pub(crate) fn draw_quote(blip: &Blip, g: &Game) {
    let (a, b) = (g.p[0].rounds, g.p[1].rounds);
    if a == b { return; }
    let who = g.p[if a > b { 0 } else { 1 }].who;
    let arch = &FIGHTERS[who];
    // On the floor in front of the fighters, clear of their feet.
    let (top, pw) = (FLOOR_Y + 12.0, 50.0);
    blip.fill_rect(0.0, top - 8.0, WIN_W as f32, WIN_H as f32 - top + 8.0,
        BlipColor { r: 0.02, g: 0.02, b: 0.05, a: 0.90 });
    blip.fill_rect(0.0, top - 8.0, WIN_W as f32, 2.0, rgb(arch.trim));
    draw_bust(blip, arch, 52.0, top, pw, -1.0, shade(rgb(arch.trim), 0.4), Mood::Happy);
    blip.draw_text(arch.name, 96.0, top - 1.0, 1.0, rgb(arch.trim));
    for (k, line) in QUOTES[who].iter().enumerate() {
        blip.draw_text(line, 96.0, top + 12.0 + k as f32 * 20.0, 2.0, BLIP_WHITE);
    }
}

/// The bonus round: the fighter, the barrels, the clock and the count.
pub(crate) fn draw_bonus(blip: &Blip, g: &Game) {
    draw_stage(blip, g);
    let shake = g.shake_px();
    let c = |r: f32, g: f32, b: f32| BlipColor { r, g, b, a: 1.0 };
    let (wood, dark, hoop) = (c(0.66, 0.42, 0.22), c(0.44, 0.26, 0.13), c(0.30, 0.31, 0.36));
    let floor = FLOOR_Y + shake;
    for (i, b) in g.barrels.iter().enumerate() {
        if b.hp <= 0 && b.broke_t < 0.0 { continue; }
        if b.hp <= 0 {
            // Staves thrown up and out, and gone before they land twice.
            let t = b.broke_t;
            if t > 0.9 { continue; }
            for j in 0..8 {
                let (vx, vy) = ((scatter(i * 8 + j, 0xd1) - 0.5) * 260.0, 120.0 + scatter(i * 8 + j, 0xd2) * 170.0);
                let (x, y) = (b.x + vx * t, b.y + shake - 26.0 - vy * t + 420.0 * t * t);
                if y > floor { continue; }
                let tall = j % 2 == 0;
                blip.fill_rect(x - 3.0, y - 9.0, if tall { 6.0 } else { 16.0 }, if tall { 18.0 } else { 5.0 }, INK);
                blip.fill_rect(x - 2.0, y - 8.0, if tall { 4.0 } else { 14.0 }, if tall { 16.0 } else { 3.0 },
                    if j % 3 == 0 { hoop } else { wood });
            }
            continue;
        }
        if b.wait > 0.0 { continue; }
        // One that fell is seen end on: a hooped round, turning as it rolls.
        if b.rolls {
            let c = V(b.x, b.y - 20.0 + shake);
            blip.fill_circle(c.0, c.1, 21.0, INK);
            blip.fill_circle(c.0, c.1, 19.0, hoop);
            blip.fill_circle(c.0, c.1, 15.5, wood);
            let turn = b.x * 0.05;
            for j in 0..3 {
                let ang = turn + j as f32 * std::f32::consts::PI / 3.0;
                let (dx, dy) = (ang.cos() * 15.0, ang.sin() * 15.0);
                stroke(blip, V(c.0 - dx, c.1 - dy), V(c.0 + dx, c.1 + dy), 0.8, 0.8, dark);
            }
            blip.fill_circle(c.0, c.1, 3.0, dark);
            continue;
        }
        // It rocks when it is struck.
        let rock = if b.hit_t < 0.22 { (b.hit_t * 60.0).sin() * 3.0 * (1.0 - b.hit_t / 0.22) } else { 0.0 };
        let (x, w, h) = (b.x + rock, BARREL_W, 62.0);
        blip.fill_rect(x - w / 2.0 - 2.0, floor - h - 2.0, w + 4.0, h + 2.0, INK);
        blip.fill_rect(x - w / 2.0 + 2.0, floor - h, w - 4.0, h, wood);
        // The belly: a little wider through the middle.
        blip.fill_rect(x - w / 2.0 - 4.0, floor - h + 12.0, w + 8.0, h - 24.0, INK);
        blip.fill_rect(x - w / 2.0 - 2.0, floor - h + 14.0, w + 4.0, h - 28.0, wood);
        blip.fill_rect(x - w / 2.0, floor - h + 2.0, 5.0, h - 4.0, blend(wood, BLIP_WHITE, 0.25));
        for sx in [-7.0f32, 1.0, 9.0] { blip.fill_rect(x + sx, floor - h, 1.5, h, dark); }
        for hy in [12.0f32, 46.0] { blip.fill_rect(x - w / 2.0 - 2.0, floor - h + hy, w + 4.0, 4.0, hoop); }
        // Cracks for every blow it has taken.
        for k in 0..(BARREL_HP - b.hp) {
            let cx = x - 8.0 + k as f32 * 11.0;
            stroke(blip, V(cx, floor - h + 4.0), V(cx + 5.0, floor - h + 20.0), 1.0, 1.0, INK);
            stroke(blip, V(cx + 5.0, floor - h + 20.0), V(cx - 1.0, floor - h + 34.0), 1.0, 1.0, INK);
        }
    }
    draw_fighter(blip, g, 0);
    for s in g.hitspark.iter().filter(|s| s.ttl > 0.0) { draw_splash(blip, s); }

    stamp(blip, "BONUS ROUND", 14.0, 3.0, BLIP_YELLOW);
    let grey = PALE;
    blip.draw_text(&format!("SCORE {}", g.sess.score), 16.0, 16.0, 1.0, grey);
    let down = g.barrels.iter().filter(|b| b.broke_t >= 0.0).count();
    if g.bonus_done > 0.0 {
        stamp(blip, &format!("{} OF {}", down, g.bonus_total), 110.0, 5.0, BLIP_WHITE);
        if g.bonus[0] > 0 {
            stamp(blip, "PERFECT", 166.0, 4.0, BLIP_YELLOW);
            stamp(blip, &format!("BONUS {}", g.bonus[0]), 206.0, 2.0, grey);
        }
    } else {
        let secs = g.clock.max(0.0).ceil() as i32;
        let hurry = if secs <= 5 { BlipColor { r: 1.0, g: 0.35, b: 0.3, a: 1.0 } } else { BLIP_WHITE };
        stamp(blip, &format!("{secs}"), 46.0, 4.0, hurry);
        if g.clock > BONUS_SECS - 1.8 {
            let falling = g.barrels.iter().any(|b| b.rolls);
            stamp(blip, if falling { "SMASH THEM AS THEY FALL!" } else { "BREAK THE BARRELS!" }, 110.0, 3.0, BLIP_WHITE);
        }
    }
}

/// What became of each champion, in a line.
const ENDINGS: [&str; FIGHTERS.len()] = [
    "THE ROAD GOES ON. THERE IS ALWAYS SOMEBODY STRONGER.",
    "THE BELT COMES HOME TO THE DOCKS.",
    "GROUNDED NO LONGER. SHE FLIES AT DAWN.",
    "HE WALKS HIS BROTHERS HOME. NOBODY SPEAKS OF IT.",
    "HE CARRIES THE TROPHY IN ONE HAND.",
    "THE WIN WAS IN THE NOTEBOOK ALL ALONG.",
    "PIZZA FOR EVERYBODY. EVEN GAMMA.",
    "BACK ON THE ROOF BEFORE THE STREETLIGHTS.",
    "GAMMA SLEEPS. THE VILLAGE DOES NOT.",
    "HE HANGS UP THE CAPE FOR ONE EVENING.",
];

/// The ladder cleared: the champion at home with a fist in the air, the nine
/// who were beaten bowing in turn along the back, and paper coming down.
pub(crate) fn draw_ending(blip: &Blip, g: &Game) {
    let (w, n) = (WIN_W as f32, FIGHTERS.len());
    let home = home_of(g.pick);
    draw_backdrop(blip, home, 0.0, g.now, 1.0);
    draw_floor(blip, home, 0.0);

    // Five to the left of the champion and four to the right, facing in.
    let xs = [34.0, 90.0, 146.0, 202.0, 252.0, 386.0, 440.0, 512.0, 596.0];
    for (k, who) in (1..n).map(|j| (g.pick + j) % n).enumerate() {
        let x = xs[k];
        let mut f = Fighter::new(who, x, if x < w / 2.0 { 1.0 } else { -1.0 });
        f.act = Act::Bow;
        f.shown = Act::Bow;
        // A wave of bows running along the line.
        f.t = (g.now * 0.8 + k as f32 * 0.22).rem_euclid(2.4);
        draw_body(blip, &f, g.now, 0.0, 0.0, k, true);
    }
    let mut champ = Fighter::new(g.pick, w / 2.0, 1.0);
    champ.act = Act::Victory;
    champ.shown = Act::Victory;
    champ.t = 1.0 + g.now;
    draw_body(blip, &champ, g.now, 0.0, 0.0, 0, true);

    // Paper, in everybody's colours.
    for i in 0..70 {
        let who = &FIGHTERS[i % n];
        let fall = 34.0 + scatter(i, 0xc1) * 40.0;
        let y = (scatter(i, 0xc2) * 420.0 + g.now * fall).rem_euclid(420.0) - 10.0;
        let x = scatter(i, 0xc3) * w + (g.now * 2.0 + i as f32).sin() * 10.0;
        let flat = ((g.now * 6.0 + i as f32 * 1.3).sin() * 0.5 + 0.5) * 4.0 + 1.0;
        blip.fill_rect(x, y, 5.0, flat, rgb(if i % 2 == 0 { who.trim } else { who.color }));
    }

    stamp(blip, "CHAMPION", 34.0, 6.0, BLIP_YELLOW);
    stamp(blip, FIGHTERS[g.pick].name, 92.0, 3.0, rgb(FIGHTERS[g.pick].trim));
    // The score counts from the last continue, so say how the run went.
    let run = match g.continues {
        0 => "ALL NINE WITHOUT A CONTINUE".to_string(),
        1 => "SCORE SINCE THE CONTINUE".to_string(),
        n => format!("{n} CONTINUES"),
    };
    // A run that beats the kiosk's record says so.
    let hi = web::high_score().score;
    let s = if hi > 0 && g.sess.score > hi { format!("NEW RECORD {}", g.sess.score) }
            else { format!("SCORE {}", g.sess.score) };
    stamp(blip, &s, 126.0, 2.0, BLIP_WHITE);
    stamp(blip, ENDINGS[g.pick], 162.0, 1.0, BLIP_WHITE);
    stamp(blip, &run, 148.0, 1.0, if g.continues == 0 { BLIP_YELLOW } else { BlipColor { r: 0.8, g: 0.8, b: 0.86, a: 1.0 } });
    if !g.phase.active() && (g.now * 2.0) as i32 % 2 == 0 {
        stamp(blip, "PRESS FIRE", 372.0, 2.0, BLIP_YELLOW);
    }
}

/// The billing before a match: the two fighters, where they meet, and in a
/// solo run the whole ladder with the ones already beaten struck out.
pub(crate) fn draw_vs(blip: &Blip, g: &Game) {
    let (w, h) = (WIN_W as f32, WIN_H as f32);
    let age = VS_SECS - g.phase.remaining();
    let who = [g.p[0].arch(), g.p[1].arch()];
    let grey = PALE;

    // Each side leans to its fighter's colour, the left warm and the right
    // cold, so two of a kind still stand on different ground.
    let warm = BlipColor { r: 0.75, g: 0.16, b: 0.12, a: 1.0 };
    let cold = BlipColor { r: 0.12, g: 0.26, b: 0.70, a: 1.0 };
    let tint = [blend(rgb(who[0].trim), warm, 0.6), blend(rgb(who[1].trim), cold, 0.6)];
    // Under the colours, where they are about to meet: the stage itself,
    // its floor under the two of them.
    draw_backdrop(blip, g.stage, 302.0 - FLOOR_Y, g.now, 0.0);
    draw_floor(blip, g.stage, 302.0 - FLOOR_Y);
    let slant = 0.24;
    let edge = |y: f32| w / 2.0 + (h / 2.0 - y) * slant;
    for row in 0..(h as i32 / 2) {
        let y = row as f32 * 2.0;
        let k = 0.22 + 0.26 * y / h;
        blip.fill_rect(0.0, y, edge(y), 2.0, BlipColor { a: 0.80, ..shade(tint[0], k) });
        blip.fill_rect(edge(y), y, w - edge(y), 2.0, BlipColor { a: 0.80, ..shade(tint[1], k) });
    }
    stroke(blip, V(edge(0.0), 0.0), V(edge(h), h), 5.0, 5.0, INK);
    stroke(blip, V(edge(0.0), 0.0), V(edge(h), h), 1.5, 1.5, BLIP_WHITE);

    // A portrait each, sliding in from the wings, and the fighters themselves
    // squaring up under the letters.
    let off = (1.0 - (age / 0.35).min(1.0)).powi(2) * 260.0;
    let (pw, py) = (128.0, 44.0);
    let ground = 302.0;
    for i in 0..2 {
        let side = if i == 0 { -1.0 } else { 1.0 };
        let a = &who[i];
        let cx = w / 2.0 + side * (218.0 + off);
        draw_bust(blip, a, cx, py, pw, side, shade(tint[i], 0.75), Mood::Calm);
        blip.fill_rect(cx - pw / 2.0 - 4.0, py + pw, pw + 8.0, 34.0, INK);
        if g.mode == Mode::Versus {
            let label = if i == 0 { "PLAYER 1" } else { "PLAYER 2" };
            blip.draw_text(label, cx - text_w(label, 1.0) / 2.0, py - 16.0, 1.0, grey);
        }
        blip.draw_text(a.name, cx - text_w(a.name, 3.0) / 2.0, py + pw + 6.0, 3.0, BLIP_WHITE);

        let f = Fighter::new(g.p[i].who, w / 2.0 + side * (64.0 + off), -side);
        draw_body(blip, &f, g.now, ground - FLOOR_Y, 0.0, i, true);
    }

    // It lands big and settles.
    if age > 0.35 {
        let sz = if age < 0.45 { 11.0 } else { 8.0 };
        stamp_at(blip, "VS", w / 2.0, 62.0 - (sz - 8.0) * 4.0, sz, BLIP_YELLOW);
    }
    stamp(blip, STAGE_NAMES[g.stage % STAGES], 14.0, 2.0, grey);
    if g.mode != Mode::Solo { return; }

    // The ladder: everyone still to come, and a cross through the beaten.
    let (cw, gap) = (40.0, 6.0);
    let x0 = (w - RUNGS as f32 * cw - (RUNGS as f32 - 1.0) * gap) / 2.0;
    let y0 = 342.0;
    for (rung, foe) in g.ladder().into_iter().enumerate() {
        let a = &FIGHTERS[foe];
        let x = x0 + rung as f32 * (cw + gap);
        let (beaten, next) = (rung < g.opponent_index, rung == g.opponent_index);
        blip.fill_rect(x - 2.0, y0 - 2.0, cw + 4.0, cw + 4.0, INK);
        blip.fill_rect(x, y0, cw, cw, shade(rgb(a.color), 0.4));
        // No wind in the frame: a headband's tails would fly out of it.
        let look = Look { bulk: a.bulk * 0.9, size: 0.9, wind: (0.0, 1.0), ..look_of(a, 0.0) };
        let head = V(x + cw / 2.0 - 1.0, y0 + 18.0);
        draw_head(blip, &look, -1.0, head, V(head.0 + 3.0, head.1 + 15.0),
            if beaten { Mood::Hurt } else { Mood::Calm });
        blip.fill_rect(x - 2.0, y0 + cw, cw + 4.0, 4.0, INK);
        if beaten {
            blip.fill_rect(x, y0, cw, cw, BlipColor { a: 0.55, ..INK });
            let red = BlipColor { r: 0.95, g: 0.25, b: 0.2, a: 1.0 };
            stroke(blip, V(x + 7.0, y0 + 7.0), V(x + cw - 7.0, y0 + cw - 7.0), 2.2, 2.2, red);
            stroke(blip, V(x + cw - 7.0, y0 + 7.0), V(x + 7.0, y0 + cw - 7.0), 2.2, 2.2, red);
        }
        let lit = next && (g.now * 5.0) as i32 % 2 == 0;
        let frame = if lit { BLIP_YELLOW } else if next { BLIP_WHITE }
            else { BlipColor { r: 0.3, g: 0.3, b: 0.35, a: 1.0 } };
        blip.draw_rect(x, y0, cw, cw, frame);
        if next { blip.draw_rect(x - 1.0, y0 - 1.0, cw + 2.0, cw + 2.0, frame); }
    }
    let rung = format!("FIGHT {} OF {}", g.opponent_index + 1, RUNGS);
    stamp(blip, &rung, y0 - 24.0, 2.0, BLIP_YELLOW);
}
