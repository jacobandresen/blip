//! The Colosseum: inside the old walls, arches open on the sunlit tiers, rubble on the stone floor, and a kitten on a
//! block watching the fight.

use super::*;

/// THE COLOSSEUM, after Way of the Dragon's last fight: inside the old walls, brick in the shade and arches open on
/// the sunlit tiers beyond, the stone floor scattered with rubble, nobody watching but a kitten on a block, its head
/// following the fight (`focus`).
pub(crate) fn draw_colosseum(blip: &Blip, shake: f32, t: f32, focus: f32, roar: Roar) {
    let c = |r, g, b| BlipColor { r, g, b, a: 1.0 };
    let w = WIN_W as f32;
    let top = FLOOR_Y + shake;
    // The sky over the far side, seen through the arches.
    blip.fill_bands(0.0, shake, w, 220.0, 10, |k| c(0.38 + 0.30 * k, 0.60 + 0.22 * k, 0.86 + 0.06 * k));
    // The far side of the bowl: tier on tier of sunlit arcades, paler with distance.
    let stone = c(0.86, 0.74, 0.56);
    for tier in 0..4 {
        let y = 92.0 + tier as f32 * 34.0 + shake;
        let tone = hazed(stone, c(0.70, 0.80, 0.90), 0.35 - tier as f32 * 0.08);
        blip.fill_rect(0.0, y, w, 34.0, tone);
        for i in 0..32 {
            let x = i as f32 * 21.0 + if tier % 2 == 0 { 0.0 } else { 10.0 };
            blip.fill_rect(x + 5.0, y + 9.0, 9.0, 20.0, shade(tone, 0.62));
            blip.fill_circle(x + 9.5, y + 9.0, 4.5, shade(tone, 0.62));
        }
        blip.fill_rect(0.0, y, w, 3.0, shade(tone, 0.85));
    }
    // The arena floor beyond, in the sun, and the shadow along the foot of the far tiers.
    blip.fill_rect(0.0, 228.0 + shake, w, FLOOR_Y - 228.0, c(0.84, 0.74, 0.58));
    blip.fill_rect(0.0, 228.0 + shake, w, 8.0, c(0.62, 0.52, 0.40));
    // The broken top edge of the far wall against the sky.
    for i in 0..16 {
        let x = i as f32 * 40.0;
        let h = 6.0 + 18.0 * scatter(i, 0xc01);
        blip.fill_rect(x, 92.0 - h + shake, 40.0, h, hazed(stone, c(0.70, 0.80, 0.90), 0.4));
    }
    // The near wall: old brick in shadow, three great arches cut through it, the top crumbling.
    let brick = c(0.46, 0.26, 0.20);
    let mortar = shade(brick, 0.78);
    let arches = [(110.0f32, 62.0f32), (320.0, 74.0), (530.0, 62.0)];
    let wall_top = 40.0 + shake;
    // Wall in vertical strips, leaving the arch openings.
    let mut x = 0.0;
    while x < w {
        let mut open: Option<(f32, f32)> = None;
        for &(ax, ar) in &arches { if (x + 2.0 - ax).abs() < ar { open = Some((ax, ar)); } }
        let crumble = 10.0 * scatter((x / 4.0) as usize, 0xc02) + if (x / 80.0) as usize % 3 == 1 { 22.0 } else { 0.0 };
        let y0 = wall_top + crumble;
        match open {
            Some((ax, ar)) => {
                // Above the round head of the arch only.
                let dx = (x + 2.0 - ax) / ar;
                let spring = top - 150.0;
                let head = spring - ar * (1.0 - dx * dx).max(0.0).sqrt();
                if head > y0 { blip.fill_rect(x, y0, 4.0, head - y0, brick); }
            }
            None => blip.fill_rect(x, y0, 4.0, top - y0, brick),
        }
        x += 4.0;
    }
    // Courses of mortar and the brick joints, only on the brick.
    for row in 0..40 {
        let y = wall_top + 14.0 + row as f32 * 7.0;
        if y > top { break; }
        let off = if row % 2 == 0 { 0.0 } else { 7.0 };
        let mut x = 0.0;
        while x < w {
            let inside = arches.iter().any(|&(ax, ar)| {
                let dx = (x - ax) / ar;
                dx.abs() < 1.0 && y > top - 150.0 - ar * (1.0 - dx * dx).max(0.0).sqrt()
            });
            if !inside {
                blip.fill_rect(x, y, 14.0, 1.0, mortar);
                blip.fill_rect(x + off, y - 6.0, 1.0, 6.0, mortar);
            }
            x += 14.0;
        }
    }
    // Weathering: dark gaps where bricks have fallen out, a few cracks, weeds at the foot of the wall.
    for i in 0..14usize {
        let x = scatter(i, 0xc21) * w;
        let y = wall_top + 30.0 + scatter(i, 0xc22) * (top - wall_top - 60.0);
        let inside = arches.iter().any(|&(ax, ar)| (x - ax).abs() < ar + 8.0 && y > top - 150.0 - ar);
        if inside { continue; }
        if i % 3 == 0 {
            let (cx, cy) = (x, y);
            stroke(blip, V(cx, cy), V(cx + 5.0, cy + 9.0), 0.6, 0.6, shade(brick, 0.55));
            stroke(blip, V(cx + 5.0, cy + 9.0), V(cx + 2.0, cy + 17.0), 0.6, 0.6, shade(brick, 0.55));
        } else {
            blip.fill_rect(x, y, 13.0, 6.0, shade(brick, 0.58));
        }
    }
    for i in 0..10usize {
        let x = scatter(i, 0xc23) * w;
        let green = BlipColor { r: 0.34, g: 0.46, b: 0.22, a: 1.0 };
        for k in 0..3 {
            let lean = (k as f32 - 1.0) * 2.5 + (t * 0.8 + i as f32).sin() * 0.6;
            stroke(blip, V(x + k as f32 * 2.0, top), V(x + k as f32 * 2.0 + lean, top - 5.0 - 3.0 * scatter(i * 3 + k, 0xc24)), 0.7, 0.4, green);
        }
    }
    // Each arch's rim of dressed stone, and its shadowed reveal.
    for &(ax, ar) in &arches {
        let spring = top - 150.0;
        for k in 0..24 {
            let a0 = std::f32::consts::PI * k as f32 / 24.0;
            let a1 = std::f32::consts::PI * (k + 1) as f32 / 24.0;
            let p = |a: f32, r: f32| V(ax - a.cos() * r, spring - a.sin() * r);
            stroke(blip, p(a0, ar + 4.0), p(a1, ar + 4.0), 4.0, 4.0, c(0.62, 0.50, 0.38));
            stroke(blip, p(a0, ar - 2.0), p(a1, ar - 2.0), 2.0, 2.0, shade(brick, 0.60));
        }
        for side in [-1.0f32, 1.0] {
            blip.fill_rect(ax + side * (ar + 2.0) - 4.0, spring, 8.0, 150.0, c(0.62, 0.50, 0.38));
            blip.fill_rect(ax + side * (ar - 3.0) - 2.0, spring, 4.0, 150.0, shade(brick, 0.60));
        }
    }
    // Sun through each arch, slanting down onto the floor, soft.
    for &(ax, ar) in &arches {
        let spring = top - 150.0;
        let sun = BlipColor { r: 1.0, g: 0.90, b: 0.66, a: 0.10 };
        for k in 0..6 {
            let x0 = ax - ar * 0.8 + k as f32 * ar * 0.3;
            stroke(blip, V(x0, spring - ar * 0.5), V(x0 + 70.0, top), 8.0, 14.0, sun);
        }
    }
    // Blocks of fallen stone along the foot of the wall, and the kitten on the left one.
    let block = c(0.58, 0.54, 0.48);
    // (Inside x 42 to 598, what the closest camera keeps.)
    for (bx, bw, bh) in [(52.0f32, 50.0f32, 30.0f32), (250.0, 30.0, 14.0), (556.0, 40.0, 24.0)] {
        blip.fill_rect(bx - 1.0, top - bh - 1.0, bw + 2.0, bh + 2.0, INK);
        blip.fill_rect(bx, top - bh, bw, bh, block);
        blip.fill_rect(bx, top - bh, bw, 3.0, shade(block, 1.12));
        blip.fill_rect(bx + bw * 0.6, top - bh + 6.0, 1.0, bh - 8.0, shade(block, 0.8));
    }
    // A big blow startles it: up off its haunches for half a second, then settling (the crowd's roar, with no crowd).
    let since = t - roar.at;
    let startle = if (0.0..0.6).contains(&since) { roar.loud.min(1.0) * (1.0 - since / 0.6) } else { 0.0 };
    draw_kitten(blip, V(78.0, top - 30.0 - 3.0 * startle), t, focus, startle);
}

/// The kitten that watches the fight at the Colosseum: white with grey patches, sitting up, its tail curling, its
/// head turning to follow the fight and its eyes blinking.
pub(super) fn draw_kitten(blip: &Blip, at: V, t: f32, focus: f32, startle: f32) {
    let white = BlipColor { r: 0.94, g: 0.92, b: 0.88, a: 1.0 };
    let grey = BlipColor { r: 0.46, g: 0.44, b: 0.44, a: 1.0 };
    let look = ((focus - at.0) / 240.0).clamp(-1.0, 1.0);
    // (1.6 times the first drawing: at 14 px tall it was lost against the block.)
    let z = 1.6;
    let at_ = at;
    let p = |dx: f32, dy: f32| V(at_.0 + dx * z, at_.1 + dy * z);
    // Body, sitting.
    shape(blip, &[(p(0.0, -6.0), p(1.0, -2.0), 5.0 * z, 6.0 * z)], white, INK);
    blip.fill_circle(p(2.0, -4.0).0, p(2.0, -4.0).1, 2.4 * z, grey);
    // Tail curled round, the tip lifting now and then.
    let flick = (t * 1.3).sin().max(0.0) * 3.0 + 6.0 * startle;
    stroke(blip, p(-5.0, -1.0), p(-9.0, -4.0 - flick), 1.6 * z, 1.2 * z, INK);
    stroke(blip, p(-5.0, -1.0), p(-9.0, -4.0 - flick), 1.0 * z, 0.7 * z, grey);
    // Head, turned toward the fight.
    let h = p(1.0 + 2.0 * look, -14.0);
    shape(blip, &[(h, h, 4.2 * z, 4.2 * z)], white, INK);
    blip.fill_circle(h.0 + (-1.5 + look) * z, h.1 - 2.0 * z, 2.0 * z, grey);
    for side in [-1.0f32, 1.0] {
        let ear = V(h.0 + (side * 2.6 + look) * z, h.1 - 3.6 * z);
        stroke(blip, V(ear.0, ear.1 + 1.5 * z), V(ear.0 + side * 0.4 * z, ear.1 - 1.6 * z), 1.6 * z, 0.4 * z, INK);
        stroke(blip, V(ear.0, ear.1 + 1.5 * z), V(ear.0 + side * 0.4 * z, ear.1 - 1.2 * z), 1.0 * z, 0.3 * z, if side < 0.0 { grey } else { white });
    }
    // Eyes, shut for a blink every few seconds.
    let blink = (t * 0.37).fract() < 0.04;
    for side in [-1.0f32, 1.0] {
        let e = V(h.0 + (side * 1.6 + 1.2 * look) * z, h.1 - 0.4 * z);
        if blink { blip.fill_rect(e.0 - 1.2, e.1, 2.4, 1.0, INK); }
        else { blip.fill_rect(e.0 - 1.0, e.1 - 1.2, 2.0, 2.4, BlipColor { r: 0.30, g: 0.50, b: 0.30, a: 1.0 }); }
    }
}
