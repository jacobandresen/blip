//! Raider assets — a 1942-style vertical dogfighter.
//!
//! Sprites are small silhouettes (plane shapes read fine at arcade
//! resolution); bullets and explosions stay plain shape draw calls in the
//! game itself, so no sprites are needed for those here.

use crate::image::Image;
use std::f32::consts::PI;

use crate::techno::{Rng, MIX_KNEE};
use crate::wav::{encode_pcm16_mono, mix_into, mix_into_f32, ms_to_samples, soft_limit_to_pcm16, SAMPLE_RATE};
use crate::Asset;

// Must match crates/sky_raider/src/main.rs's PLAYER_W / PLAYER_H.
const PLAYER_W: i32 = 36;
const PLAYER_H: i32 = 32;
// Must match crates/sky_raider/src/main.rs's ENEMY_W / ENEMY_H.
const ENEMY_W: i32 = 26;
const ENEMY_H: i32 = 22;
// Must match crates/sky_raider/src/main.rs's BOSS_SIZES.
const BOSS_SIZES: [(i32, i32); 7] = [
    (72, 50), (84, 58), (98, 68), (114, 80), (132, 92), (152, 106), (176, 124),
];
// Must match crates/sky_raider/src/main.rs's POW_W / POW_H.
const POW_W: i32 = 14;
const POW_H: i32 = 14;
// Must match crates/sky_raider/src/main.rs's HEALTH_W / HEALTH_H.
const HEALTH_W: i32 = 14;
const HEALTH_H: i32 = 14;
// Must match crates/sky_raider/src/main.rs's CARRIER_W / CARRIER_H.
const CARRIER_W: i32 = 108;
const CARRIER_H: i32 = 190;
// Must match crates/sky_raider/src/main.rs's BOAT_W / BOAT_H.
const BOAT_W: i32 = 34;
const BOAT_H: i32 = 16;
// Must match crates/sky_raider/src/main.rs's ISLAND_SIZES.
const ISLAND_SIZES: [(i32, i32); 3] = [(64, 44), (98, 68), (140, 96)];

// ---------------------------------------------------------------------- //
// Tone / noise helpers (self-contained, same idiom as the other games)     //
// ---------------------------------------------------------------------- //

fn gen_tone(freq: f32, dur_ms: f32, amp: f32) -> Vec<i16> {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(dur_ms);
    let fade = SAMPLE_RATE as usize / 200;
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let mut e = 1.0_f32;
        if i < fade { e = i as f32 / fade as f32; }
        if i + fade > n { e = (n - i) as f32 / fade as f32; }
        let fund = (2.0 * std::f32::consts::PI * freq * t).sin();
        let third = (2.0 * std::f32::consts::PI * freq * 3.0 * t).sin() / 3.0;
        let shaped = (fund * 0.8 + third * 0.3).tanh();
        s.push((e * amp * 27000.0 * shaped) as i16);
    }
    s
}

/// LCG for deterministic noise.
struct Lcg(u32);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_103_515_245).wrapping_add(12345) & 0x7FFF_FFFF;
        self.0
    }
}

fn gen_noise(dur_ms: f32, amp: f32) -> Vec<i16> {
    let n = ms_to_samples(dur_ms);
    let fade = SAMPLE_RATE as usize / 200;
    let mut rng = Lcg(7);
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let mut e = 1.0_f32;
        if i < fade { e = i as f32 / fade as f32; }
        if i + fade > n { e = (n - i) as f32 / fade as f32; }
        let decay = 1.0 - i as f32 / n as f32;
        let r = rng.next() % 65536;
        let noise = (r as f32 - 32768.0) / 32768.0;
        s.push((e * amp * decay * 32000.0 * noise) as i16);
    }
    s
}

/// A quick run of notes played one after another (each overlapping the next
/// slightly, since `step_ms < dur_ms`), mixed into one buffer. Used for the
/// power-up pickup chimes — more notes and a higher register reads as a
/// bigger pickup, which is how the four weapon-tier chimes escalate.
fn ascending_run(notes: &[f32], step_ms: f32, dur_ms: f32, amp: f32) -> Vec<i16> {
    let n = ms_to_samples(step_ms * notes.len() as f32 + dur_ms);
    let mut buf = vec![0i16; n];
    for (i, f) in notes.iter().enumerate() {
        let t = gen_tone(*f, dur_ms, amp);
        let off = ms_to_samples(step_ms * i as f32);
        for (j, s) in t.iter().enumerate() { mix_into(&mut buf, off + j, *s as f32); }
    }
    buf
}

/// The grand finale for maxing out the weapon: a fast four-note ascending run
/// into a held bright chord, with a fifth harmony under the last note —
/// meant to feel like a proper "fanfare" next to the plain pickup chimes.
fn max_power_sfx() -> Vec<u8> {
    let notes = [523.25, 659.25, 783.99, 1046.50]; // C5 E5 G5 C6
    let step_ms = 90.0;
    let chord_ms = 260.0;
    let n = ms_to_samples(step_ms * notes.len() as f32 + chord_ms + 150.0);
    let mut buf = vec![0i16; n];
    for (i, f) in notes.iter().enumerate() {
        let dur = if i == notes.len() - 1 { chord_ms } else { step_ms * 1.4 };
        let t = gen_tone(*f, dur, 0.5);
        let off = ms_to_samples(step_ms * i as f32);
        for (j, s) in t.iter().enumerate() { mix_into(&mut buf, off + j, *s as f32); }
    }
    // A fifth harmony under the held final chord, for extra sparkle.
    let harmony = gen_tone(1318.51, chord_ms, 0.3); // E6
    let off = ms_to_samples(step_ms * (notes.len() - 1) as f32);
    for (j, s) in harmony.iter().enumerate() { mix_into(&mut buf, off + j, *s as f32); }
    encode_pcm16_mono(&buf)
}

/// A short rising/falling alarm wail — plays once as each boss makes its
/// entrance, tier 1 through 7 alike (the boss's own name banner is what
/// signals which one it is).
fn boss_warning_sfx() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let dur_ms = 500.0;
    let n = ms_to_samples(dur_ms);
    let mut s = Vec::with_capacity(n);
    let mut phase = 0.0f32;
    for i in 0..n {
        let t = i as f32 / n as f32;
        let freq = 500.0 + (t * std::f32::consts::PI * 2.0).sin() * 220.0; // wobbles ~280-720Hz
        phase += freq / sr;
        let env = if t < 0.05 { t / 0.05 } else if t > 0.85 { (1.0 - t) / 0.15 } else { 1.0 };
        let wave = (2.0 * std::f32::consts::PI * phase).sin();
        s.push((env * 0.4 * 27000.0 * wave) as i16);
    }
    encode_pcm16_mono(&s)
}

/// The big finish: clearing all seven waves. A fast run up through an octave
/// into a sustained major triad — the biggest fanfare in the game, longer and
/// grander than the "reached max power" one.
fn victory_sfx() -> Vec<u8> {
    let notes = [392.00, 493.88, 587.33, 698.46, 783.99, 987.77]; // G4 B4 D5 F5 G5 B5
    let step_ms = 85.0;
    let chord_ms = 900.0;
    let n = ms_to_samples(step_ms * notes.len() as f32 + chord_ms + 300.0);
    let mut buf = vec![0i16; n];
    for (i, f) in notes.iter().enumerate() {
        let dur = if i == notes.len() - 1 { chord_ms } else { step_ms * 1.5 };
        let t = gen_tone(*f, dur, 0.5);
        let off = ms_to_samples(step_ms * i as f32);
        for (j, s) in t.iter().enumerate() { mix_into(&mut buf, off + j, *s as f32); }
    }
    // Two more notes layered under the held final chord, for a full triad.
    let off = ms_to_samples(step_ms * (notes.len() - 1) as f32);
    for extra in [1174.66, 1567.98] { // D6, G6
        let t = gen_tone(extra, chord_ms, 0.28);
        for (j, s) in t.iter().enumerate() { mix_into(&mut buf, off + j, *s as f32); }
    }
    encode_pcm16_mono(&buf)
}

/// The player's shot: a soft descending "pew" (sine sweeping down in pitch)
/// rather than a flat repeated beep — this fires many times a second at
/// full-auto, so it needs to be gentle and a little varied, not a shrill tone
/// hit over and over.
fn shoot_sfx() -> Vec<i16> {
    let sr = SAMPLE_RATE as f32;
    let dur_ms = 70.0;
    let n = ms_to_samples(dur_ms);
    let (f0, f1) = (560.0, 220.0); // sweeps down over the note's length
    let mut s = Vec::with_capacity(n);
    let mut phase = 0.0f32;
    for i in 0..n {
        let k = i as f32 / n as f32;
        let freq = f0 + (f1 - f0) * k;
        phase += freq / sr;
        let env = (1.0 - k).powf(1.6); // fast decay, no click since it starts at full volume smoothly
        let wave = (2.0 * std::f32::consts::PI * phase).sin();
        s.push((env * 0.32 * 27000.0 * wave) as i16);
    }
    s
}

/// Descending 3-tone "power-down" sting for game over.
fn game_over_sfx() -> Vec<u8> {
    let notes = [440.0, 330.0, 220.0];
    let step_ms = 180.0;
    let n = ms_to_samples(step_ms * notes.len() as f32 + 200.0);
    let mut buf = vec![0i16; n];
    for (i, f) in notes.iter().enumerate() {
        let t = gen_tone(*f, step_ms * 1.3, 0.55);
        let off = ms_to_samples(step_ms * i as f32);
        for (j, s) in t.iter().enumerate() { mix_into(&mut buf, off + j, *s as f32); }
    }
    encode_pcm16_mono(&buf)
}

/// Ascending 3-tone fanfare for a stage clear.
fn stage_clear_sfx() -> Vec<u8> {
    let notes = [523.25, 659.25, 783.99]; // C5 E5 G5
    let step_ms = 130.0;
    let n = ms_to_samples(step_ms * notes.len() as f32 + 300.0);
    let mut buf = vec![0i16; n];
    for (i, f) in notes.iter().enumerate() {
        let t = gen_tone(*f, step_ms * 1.6, 0.55);
        let off = ms_to_samples(step_ms * i as f32);
        for (j, s) in t.iter().enumerate() { mix_into(&mut buf, off + j, *s as f32); }
    }
    encode_pcm16_mono(&buf)
}

// ---------------------------------------------------------------------- //
// Sprites                                                                  //
// ---------------------------------------------------------------------- //

/// Player fighter, nose pointing up (toward the top of the screen, the
/// direction of travel) — a radial-engine prop fighter, 1942-style: a slim
/// tapered fuselage, a big main wing plus a small tail stabiliser (the pair
/// that actually reads as "an airplane" from directly above), a canopy
/// bubble, and a spinner-and-disc propeller at the nose.
fn player_plane() -> Vec<u8> {
    let (w, h) = (PLAYER_W, PLAYER_H);
    let mut img = Image::new(w as u32, h as u32);
    let cx = w / 2;
    let prop_cy = 1;
    let main_wing_y0 = (h as f32 * 0.46) as i32;
    let main_wing_y1 = (h as f32 * 0.60) as i32;
    let tail_wing_y0 = (h as f32 * 0.82) as i32;
    let tail_wing_y1 = (h as f32 * 0.91) as i32;
    let canopy_y0 = (h as f32 * 0.24) as i32;
    let canopy_y1 = (h as f32 * 0.46) as i32;
    for y in 0..h {
        for x in 0..w {
            let from_top = y as f32 / h as f32;
            let body_half = (1.0 + from_top * 2.2) as i32; // slim, longer-looking taper to the nose
            if (x - cx).abs() <= body_half && y >= 3 && y <= h - 1 {
                img.set(x, y, 210, 214, 222);
            }
            // main wing, roughly amidships — a lighter leading-edge stripe
            // and darker wingtips give it real shape instead of a flat slab.
            if y >= main_wing_y0 && y <= main_wing_y1 {
                let wing_half = (w as f32 * 0.48) as i32;
                let wd = (x - cx).abs();
                if wd <= wing_half {
                    img.set(x, y, 50, 100, 220); // BLIP_BLUE — matches the cabinet accent
                }
                if y == main_wing_y0 && wd <= wing_half {
                    img.set(x, y, 96, 150, 235); // leading-edge highlight
                }
                if wd > wing_half - 3 && wd <= wing_half {
                    img.set(x, y, 30, 66, 165); // wingtip shading
                }
            }
            // small tail stabiliser near the rear
            if y >= tail_wing_y0 && y <= tail_wing_y1 {
                let wing_half = (w as f32 * 0.26) as i32;
                if (x - cx).abs() <= wing_half {
                    img.set(x, y, 50, 100, 220);
                }
            }
            // canopy, with a thin frame bar splitting it into two panes
            if (x - cx).abs() <= 3 && y >= canopy_y0 && y <= canopy_y1 {
                img.set(x, y, 40, 220, 255);
            }
            if (x - cx).abs() <= 3 && y == (canopy_y0 + canopy_y1) / 2 {
                img.set(x, y, 30, 40, 48); // canopy frame
            }
            // a highlight down one side of the spine and a shadow down the
            // other — a cheap "rounded fuselage" shading cue instead of a
            // flat-looking silhouette
            if (x - cx) == -1 && y >= 4 && y <= h - 2 {
                img.set(x, y, 232, 236, 244);
            }
            if (x - cx) == 2 && y >= 4 && y <= h - 2 && body_half >= 2 {
                img.set(x, y, 168, 174, 188);
            }
            // a small rudder-stripe accent right at the tail tip
            if (x - cx).abs() <= 1 && y > tail_wing_y1 && y <= h - 1 {
                img.set(x, y, 220, 70, 70);
            }
            // propeller: a blurred spinning disc plus a dark spinner hub at
            // the very nose — drawn after the body so it sits on top of it.
            let pdx = x - cx;
            let pdy = y - prop_cy;
            if pdx * pdx + pdy * pdy <= 13 {
                img.set(x, y, 205, 205, 212);
            }
            if pdx.abs() <= 1 && pdy.abs() <= 1 {
                img.set(x, y, 45, 45, 52);
            }
        }
    }
    img.encode_png()
}

/// Enemy fighter, nose pointing down (diving toward the player) — the same
/// main-wing + tail-stabiliser silhouette as the player, recoloured per
/// kind, front-to-back layout mirrored (tail near the top, nose/propeller
/// at the bottom). `kind`: 0 = grunt (drab green), 1 = weaver (tan),
/// 2 = ace (red, always drops a power-up).
fn enemy_plane(kind: usize) -> Vec<u8> {
    let (w, h) = (ENEMY_W, ENEMY_H);
    let mut img = Image::new(w as u32, h as u32);
    let cx = w / 2;
    let (r, g, b): (u8, u8, u8) = match kind {
        0 => (90, 130, 90),
        1 => (200, 150, 60),
        _ => (225, 45, 45),
    };
    let (dr, dg, db) = (r.saturating_sub(35), g.saturating_sub(35), b.saturating_sub(35));
    let prop_cy = h - 2;
    let tail_wing_y0 = (h as f32 * 0.08) as i32;
    let tail_wing_y1 = (h as f32 * 0.16) as i32;
    let main_wing_y0 = (h as f32 * 0.42) as i32;
    let main_wing_y1 = (h as f32 * 0.54) as i32;
    let cockpit_y0 = (h as f32 * 0.62) as i32;
    let cockpit_y1 = (h as f32 * 0.74) as i32;
    for y in 0..h {
        for x in 0..w {
            let from_top = y as f32 / h as f32;
            let body_half = (1.0 + (1.0 - from_top) * 1.9) as i32; // tapers to a nose at the bottom
            if (x - cx).abs() <= body_half && y >= 1 && y <= h - 2 {
                img.set(x, y, r, g, b);
            }
            // small tail stabiliser near the rear (top, away from the nose)
            if y >= tail_wing_y0 && y <= tail_wing_y1 {
                let wing_half = (w as f32 * 0.24) as i32;
                if (x - cx).abs() <= wing_half {
                    img.set(x, y, dr, dg, db);
                }
            }
            // main wing, roughly amidships
            if y >= main_wing_y0 && y <= main_wing_y1 {
                let wing_half = (w as f32 * 0.46) as i32;
                if (x - cx).abs() <= wing_half {
                    img.set(x, y, dr, dg, db);
                }
            }
            // cockpit, between the main wing and the nose
            if (x - cx).abs() <= 1 && y >= cockpit_y0 && y <= cockpit_y1 {
                img.set(x, y, 20, 20, 30);
            }
            // propeller disc + spinner hub at the nose (the bottom tip,
            // since these planes dive down toward the player).
            let pdx = x - cx;
            let pdy = y - prop_cy;
            if pdx * pdx + pdy * pdy <= 7 {
                img.set(x, y, 55, 55, 62);
            }
            if pdx.abs() <= 1 && pdy.abs() <= 1 {
                img.set(x, y, 15, 15, 20);
            }
        }
    }
    img.encode_png()
}

// (body, wing) colour per tier — brown-red -> orange -> purple -> deep red
// -> magenta -> dark crimson -> molten orange for the finale.
const BOSS_PALETTES: [((u8, u8, u8), (u8, u8, u8)); 7] = [
    ((120, 60, 60),  (90, 40, 40)),
    ((165, 75, 40),  (125, 55, 30)),
    ((120, 55, 150), (90, 40, 115)),
    ((160, 35, 35),  (120, 22, 22)),
    ((185, 45, 130), (145, 28, 100)),
    ((130, 22, 22),  (90, 12, 12)),
    ((225, 60, 30),  (180, 35, 15)),
];

/// End-of-wave bosses, one per level 1-7 (`tier` 0..=6). Unlike the enemy
/// planes (one shared silhouette, recoloured), each boss gets its own hull
/// shape — they should look like different machines, not just bigger ones —
/// while still escalating in size (BOSS_SIZES) and colour (BOSS_PALETTES).
fn boss_plane(tier: usize) -> Vec<u8> {
    let (w, h) = BOSS_SIZES[tier];
    let mut img = Image::new(w as u32, h as u32);
    let (body, wing) = BOSS_PALETTES[tier];
    match tier {
        0 => boss_hull_scout(&mut img, w, h, body, wing),
        1 => boss_hull_interceptor(&mut img, w, h, body, wing),
        2 => boss_hull_gunship(&mut img, w, h, body, wing),
        3 => boss_hull_dreadnought(&mut img, w, h, body, wing),
        4 => boss_hull_cruiser(&mut img, w, h, body, wing),
        5 => boss_hull_carrier(&mut img, w, h, body, wing),
        _ => boss_hull_apex(&mut img, w, h, body, wing),
    }
    img.encode_png()
}

/// Tier 1, SCOUT BOMBER: the baseline shape — tapered fuselage, one pair of
/// swept wings, twin engine pods, single cockpit.
fn boss_hull_scout(img: &mut Image, w: i32, h: i32, body: (u8, u8, u8), wing: (u8, u8, u8)) {
    let cx = w / 2;
    let wing_y0 = (h as f32 * 0.16) as i32;
    let wing_y1 = (h as f32 * 0.36) as i32;
    let pod_y0 = (h as f32 * 0.40) as i32;
    let pod_y1 = (h as f32 * 0.56) as i32;
    let cockpit_y0 = (h as f32 * 0.10) as i32;
    let cockpit_y1 = (h as f32 * 0.20) as i32;
    for y in 0..h {
        let from_top = y as f32 / h as f32;
        let body_half = (2.0 + (1.0 - from_top) * (h as f32 * 0.10)) as i32;
        for x in 0..w {
            let adx = (x - cx).abs();
            if adx <= body_half && y <= h - 3 {
                img.set(x, y, body.0, body.1, body.2);
            }
            if y >= wing_y0 && y <= wing_y1 {
                let half = (w as f32 * 0.47) as i32;
                let taper = ((y - wing_y0) as f32 / (wing_y1 - wing_y0).max(1) as f32 * half as f32) as i32;
                if adx <= half - taper / 3 && adx >= body_half {
                    img.set(x, y, wing.0, wing.1, wing.2);
                }
            }
            if y >= pod_y0 && y <= pod_y1 {
                for &ex in &[-w / 3, w / 3] {
                    if (x - (cx + ex)).abs() <= 3 { img.set(x, y, 60, 60, 70); }
                }
            }
            if y >= cockpit_y0 && y <= cockpit_y1 && adx <= 3 {
                img.set(x, y, 255, 210, 60);
            }
        }
    }
}

/// Tier 2, INTERCEPTOR: a sleek delta — one solid arrow-shaped wing instead
/// of a separate fuselage, with a bright spine and twin tail-engine glow.
fn boss_hull_interceptor(img: &mut Image, w: i32, h: i32, body: (u8, u8, u8), wing: (u8, u8, u8)) {
    let cx = w / 2;
    let spine_w = (w as f32 * 0.07).max(2.0);
    for y in 0..h {
        let ft = y as f32 / h as f32;
        let half_w = if ft < 0.85 {
            (ft / 0.85) * (w as f32 * 0.48)
        } else {
            (w as f32 * 0.48) * (1.0 - (ft - 0.85) / 0.15 * 0.7)
        };
        for x in 0..w {
            let dx = (x - cx) as f32;
            let adx = dx.abs();
            if adx <= half_w {
                let c = if adx <= spine_w { body } else { wing };
                img.set(x, y, c.0, c.1, c.2);
            }
            if adx <= 2.0 && ft > 0.06 && ft < 0.30 {
                img.set(x, y, 255, 220, 80);
            }
            if ft > 0.86 && ft < 0.96 {
                for &ex in &[-(w / 6), w / 6] {
                    if (x - (cx + ex)).abs() <= 3 { img.set(x, y, 255, 160, 60); }
                }
            }
        }
    }
}

/// Tier 3, GUNSHIP: a twin-boom airframe — two parallel hulls joined by a
/// connecting wing, with a central gun pod slung underneath.
fn boss_hull_gunship(img: &mut Image, w: i32, h: i32, body: (u8, u8, u8), wing: (u8, u8, u8)) {
    let cx = w / 2;
    let boom_off = w as f32 * 0.28;
    let boom_r = (w as f32 * 0.10).max(4.0);
    let wing_y0 = (h as f32 * 0.30) as i32;
    let wing_y1 = (h as f32 * 0.44) as i32;
    let gun_y0 = (h as f32 * 0.44) as i32;
    let gun_y1 = (h as f32 * 0.64) as i32;
    for y in 0..h {
        let ft = y as f32 / h as f32;
        let taper = if ft < 0.12 { (0.12 - ft) / 0.12 } else if ft > 0.88 { (ft - 0.88) / 0.12 } else { 0.0 };
        let r = boom_r * (1.0 - taper * 0.6);
        for x in 0..w {
            let dx = (x - cx) as f32;
            if (dx - boom_off).abs() <= r || (dx + boom_off).abs() <= r {
                img.set(x, y, body.0, body.1, body.2);
            } else if y >= wing_y0 && y <= wing_y1 && dx.abs() <= boom_off + 2.0 {
                img.set(x, y, wing.0, wing.1, wing.2);
            } else if y >= gun_y0 && y <= gun_y1 && dx.abs() <= w as f32 * 0.07 {
                img.set(x, y, 50, 50, 58);
            }
        }
    }
}

/// Tier 4, DREADNOUGHT: a broad manta/flying-wing — no distinct fuselage,
/// just one wide diamond of a hull with a cockpit blister down the spine.
fn boss_hull_dreadnought(img: &mut Image, w: i32, h: i32, body: (u8, u8, u8), wing: (u8, u8, u8)) {
    let cx = w / 2;
    for y in 0..h {
        let ft = y as f32 / h as f32;
        let half_w = if ft < 0.35 {
            (ft / 0.35) * (w as f32 * 0.49)
        } else {
            (w as f32 * 0.49) * (1.0 - (ft - 0.35) / 0.65)
        };
        for x in 0..w {
            let dx = (x - cx) as f32;
            let adx = dx.abs();
            if adx <= half_w {
                let band = adx / half_w.max(1.0);
                let c = if band < 0.45 { body } else { wing };
                img.set(x, y, c.0, c.1, c.2);
            }
            if adx <= 1.0 && ft > 0.20 && ft < 0.36 {
                img.set(x, y, 255, 220, 80);
            }
        }
    }
}

/// Tier 5, BATTLE CRUISER: a segmented central hull flanked by two wingtip
/// pods on thin struts — reads as a proper "ship" rather than a plane.
fn boss_hull_cruiser(img: &mut Image, w: i32, h: i32, body: (u8, u8, u8), wing: (u8, u8, u8)) {
    let cx = w / 2;
    let hull_hw = w as f32 * 0.20;
    let pod_off = w as f32 * 0.40;
    let pod_r = w as f32 * 0.09;
    for y in 0..h {
        let ft = y as f32 / h as f32;
        let taper = if ft < 0.08 { (0.08 - ft) / 0.08 } else if ft > 0.92 { (ft - 0.92) / 0.08 } else { 0.0 };
        let hw = hull_hw * (1.0 - taper * 0.6);
        let ridge = (y % 9) < 2;
        let pods_here = ft > 0.30 && ft < 0.62;
        let struts_here = ft > 0.42 && ft < 0.50;
        for x in 0..w {
            let dx = (x - cx) as f32;
            let adx = dx.abs();
            if adx <= hw {
                let c = if ridge { wing } else { body };
                img.set(x, y, c.0, c.1, c.2);
            } else if pods_here && (adx - pod_off).abs() <= pod_r {
                img.set(x, y, wing.0, wing.1, wing.2);
            } else if struts_here && adx <= pod_off + pod_r {
                img.set(x, y, wing.0, wing.1, wing.2);
            }
        }
    }
}

/// Tier 6, DOOM CARRIER: a long boxy hull with hangar-bay pods bulging out
/// at regular intervals — the shape itself hints at "launches fighters".
fn boss_hull_carrier(img: &mut Image, w: i32, h: i32, body: (u8, u8, u8), wing: (u8, u8, u8)) {
    let cx = w / 2;
    let hull_hw = w as f32 * 0.30;
    let bay_h = (h / 8).max(3);
    for y in 0..h {
        let ft = y as f32 / h as f32;
        let taper = if ft < 0.06 { (0.06 - ft) / 0.06 } else if ft > 0.94 { (ft - 0.94) / 0.06 } else { 0.0 };
        let hw = hull_hw * (1.0 - taper * 0.7);
        let in_bay_band = ft > 0.14 && ft < 0.86 && (y / bay_h) % 2 == 0;
        for x in 0..w {
            let adx = (x - cx).abs() as f32;
            if adx <= hw {
                img.set(x, y, body.0, body.1, body.2);
            } else if in_bay_band && adx <= hw + w as f32 * 0.09 {
                img.set(x, y, wing.0, wing.1, wing.2);
            }
        }
    }
}

/// Tier 7, APEX DESTROYER: a jagged five-pointed crystal with a molten core
/// — deliberately alien next to the other six, the "totally badass" finale.
fn boss_hull_apex(img: &mut Image, w: i32, h: i32, body: (u8, u8, u8), wing: (u8, u8, u8)) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    for y in 0..h {
        for x in 0..w {
            let ndx = (x as f32 - cx) / cx;
            let ndy = (y as f32 - cy) / cy;
            let spike = (ndy.atan2(ndx) * 5.0).cos().abs() * 0.30 + 0.70;
            let ndist = (ndx * ndx + ndy * ndy).sqrt();
            if ndist <= spike {
                let c = if ndist < spike * 0.55 { body } else { wing };
                img.set(x, y, c.0, c.1, c.2);
            }
        }
    }
    // Molten reactor-glow core, offset toward the "front".
    let core_y = (h as f32 * 0.58) as i32;
    let cxi = w / 2;
    for y in 0..h {
        for x in 0..w {
            let d = (((x - cxi) * (x - cxi) + (y - core_y) * (y - core_y)) as f32).sqrt();
            if d <= w as f32 * 0.05 { img.set(x, y, 255, 240, 180); }
            else if d <= w as f32 * 0.09 { img.set(x, y, 255, 160, 40); }
        }
    }
}

/// Power-up capsule dropped by the ace: a glowing diamond with a bright core.
fn powerup_capsule() -> Vec<u8> {
    let (w, h) = (POW_W, POW_H);
    let mut img = Image::new(w as u32, h as u32);
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    for y in 0..h {
        for x in 0..w {
            let dx = (x as f32 - cx).abs();
            let dy = (y as f32 - cy).abs();
            let d = dx + dy; // diamond metric
            if d <= cx.min(cy) {
                img.set(x, y, 40, 230, 120);
            }
            if d <= cx.min(cy) * 0.45 {
                img.set(x, y, 255, 255, 255);
            }
        }
    }
    img.encode_png()
}

/// Health pickup, dropped occasionally by regular fighters: a white
/// roundel with a red cross — deliberately a different shape from the
/// weapon capsule's diamond (and a fixed colour, not tinted by weapon
/// tier), so the two are never mistaken for each other in the middle of
/// a dogfight.
fn health_pack() -> Vec<u8> {
    let (w, h) = (HEALTH_W, HEALTH_H);
    let mut img = Image::new(w as u32, h as u32);
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let r = cx.min(cy) - 0.5;
    let bar_half = r * 0.34;
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            if dx * dx + dy * dy <= r * r {
                img.set(x, y, 240, 240, 236); // white roundel
            }
            if dx.abs() <= bar_half && dy.abs() <= r * 0.72 {
                img.set(x, y, 220, 50, 50); // the cross's vertical bar
            }
            if dy.abs() <= bar_half && dx.abs() <= r * 0.72 {
                img.set(x, y, 220, 50, 50); // the cross's horizontal bar
            }
        }
    }
    img.encode_png()
}

/// The carrier the player launches from at the start of each level: a
/// top-down flight deck, tapered at bow and stern, with a dashed centre
/// runway, arrestor cables and elevator cutouts marked into the deck, a
/// handful of planes parked to port, and an island superstructure — mast,
/// lit bridge windows — off to starboard. Bigger and longer than the first
/// version, with the extra deck real estate spent on those details instead
/// of just scaling up a plain grey rectangle.
fn carrier_ship() -> Vec<u8> {
    let (w, h) = (CARRIER_W, CARRIER_H);
    let mut img = Image::new(w as u32, h as u32);
    let cx = w / 2;

    // Island superstructure: offset to starboard (the right), the way a
    // real carrier's island sits beside rather than astride the runway.
    let ix0 = (w as f32 * 0.58) as i32;
    let ix1 = (w as f32 * 0.82) as i32;
    let iy0 = (h as f32 * 0.30) as i32;
    let iy1 = (h as f32 * 0.52) as i32;
    let mast_x = (ix0 + ix1) / 2;
    let mast_y0 = (iy0 - (h as f32 * 0.06) as i32).max(0);

    // Two elevator deck cutouts, fore and aft, on the opposite (port) side
    // from the island — just an outline, deck-coloured inside.
    let elev_w = (w as f32 * 0.20) as i32;
    let elev_h = (h as f32 * 0.09) as i32;
    let elev_x0 = (w as f32 * 0.14) as i32;
    let elev_ys = [(h as f32 * 0.22) as i32, (h as f32 * 0.66) as i32];

    for y in 0..h {
        let from_top = y as f32 / h as f32;
        let bow_taper = if from_top < 0.12 { (0.12 - from_top) / 0.12 } else { 0.0 };
        let stern_taper = if from_top > 0.91 { (from_top - 0.91) / 0.09 } else { 0.0 };
        let half_w = (w as f32 * 0.46) * (1.0 - (bow_taper + stern_taper) * 0.75);
        let stripe = (y / 8) % 2 == 0 && from_top > 0.09 && from_top < 0.89;
        // Arrestor wires: a few thin lines crossing the aft deck, just
        // ahead of the stern taper — where the player's plane will catch
        // one on landing, if Raider ever grows a carrier-landing sequence.
        let arrestor = from_top > 0.74 && from_top < 0.88 && y % 6 == 0;

        for x in 0..w {
            let dx = (x - cx) as f32;
            let adx = dx.abs();
            if adx <= half_w {
                img.set(x, y, 72, 76, 82); // deck grey
            }
            if adx > half_w - 2.0 && adx <= half_w {
                img.set(x, y, 38, 40, 44); // deck edge
            }
            if adx <= 2.0 && stripe {
                img.set(x, y, 224, 214, 60); // dashed runway centreline
            }
            if arrestor && adx <= half_w - 4.0 {
                img.set(x, y, 30, 32, 36); // arrestor cable, crossing the centreline
            }
        }

        // Elevator outlines, drawn per-row so they land on top of the deck
        // fill above but under the island and parked planes below.
        for &ey0 in &elev_ys {
            if y >= ey0 && y < ey0 + elev_h {
                let top_or_bottom = y == ey0 || y == ey0 + elev_h - 1;
                for x in elev_x0..(elev_x0 + elev_w).min(w) {
                    if top_or_bottom || x == elev_x0 || x == elev_x0 + elev_w - 1 {
                        img.set(x, y, 46, 48, 54);
                    }
                }
            }
        }
    }

    // The island block itself, a mast rising off its roof, and a strip of
    // lit bridge windows partway down its face.
    for y in iy0..=iy1 {
        for x in ix0..=ix1 {
            img.set(x, y, 42, 46, 52);
        }
    }
    for y in mast_y0..iy0 {
        img.set(mast_x, y, 30, 32, 36);
    }
    let window_y = iy0 + (iy1 - iy0) / 3;
    for x in (ix0 + 1)..ix1 {
        if (x - ix0) % 2 == 1 {
            img.set(x, window_y, 250, 220, 120);
        }
    }

    // A few planes parked to port — small solid silhouettes, not full
    // sprites, just enough to read as a working flight deck rather than an
    // empty one.
    let plane_w = (w as f32 * 0.10) as i32;
    let plane_h = (h as f32 * 0.045) as i32;
    let plane_x = (w as f32 * 0.12) as i32;
    for frac in [0.38, 0.47, 0.56] {
        let py0 = (h as f32 * frac) as i32;
        for y in py0..(py0 + plane_h).min(h) {
            for x in plane_x..(plane_x + plane_w).min(w) {
                img.set(x, y, 58, 62, 68);
            }
        }
    }

    img.encode_png()
}

/// An enemy boat, viewed from above: a pointed-bow hull (bow to the right;
/// the game flips the sprite horizontally when it's sailing the other way),
/// a small deckhouse amidships, and a wake ripple trailing the stern.
fn boat() -> Vec<u8> {
    let (w, h) = (BOAT_W, BOAT_H);
    let mut img = Image::new(w as u32, h as u32);
    let cy = h / 2;
    let bow_start = w as f32 * 0.72;
    for x in 0..w {
        let half_h = if x as f32 > bow_start {
            let t = (x as f32 - bow_start) / (w as f32 - bow_start);
            (h as f32 * 0.42) * (1.0 - t)
        } else {
            h as f32 * 0.42
        };
        for y in 0..h {
            if (y - cy).abs() as f32 <= half_h {
                img.set(x, y, 92, 86, 78); // drab hull
            }
        }
    }
    // deckhouse, amidships toward the stern
    let dh_x0 = (w as f32 * 0.30) as i32;
    let dh_x1 = (w as f32 * 0.50) as i32;
    for x in dh_x0..dh_x1 {
        for y in (cy - 3)..(cy + 3) {
            img.set(x, y, 58, 56, 53);
        }
    }
    // a couple of wake ripples trailing the stern
    img.set(2, cy, 190, 210, 230);
    img.set(4, cy - 2, 190, 210, 230);
    img.set(4, cy + 2, 190, 210, 230);
    img.encode_png()
}

// ---------------------------------------------------------------------- //
// Clouds — value-noise fBm, not flat circles                                //
// ---------------------------------------------------------------------- //
//
// A real cumulus cloud has a flattish, harder-edged base and a much more
// broken-up, softly-diffused, billowing top — bright where the sun catches
// it, greyer in the "valleys" between puffs. Three flat overlapping circles
// don't read as a cloud at all. Instead each cloud sprite is built from
// fractional Brownian motion (a handful of octaves of value noise, each one
// double the frequency and half the amplitude of the last — the standard
// recipe used to fake terrain, marble, and cloud textures) masked against an
// envelope that's flattened on the underside, with the noise's own
// turbulence driving both the silhouette's edge and a top-lit/base-shadowed
// tint. Baked once per variant at build time, not drawn as live shapes.

const CLOUD_TEX_W: i32 = 40;
const CLOUD_TEX_H: i32 = 26;

/// Cheap 2D hash -> [0, 1). Different `seed`s give unrelated noise fields.
fn cloud_hash(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32)
        .wrapping_mul(374_761_393)
        .wrapping_add((y as u32).wrapping_mul(668_265_263))
        .wrapping_add(seed.wrapping_mul(2_246_822_519));
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0x00FF_FFFF) as f32 / 0x0100_0000 as f32
}

/// Smoothstep-interpolated value noise at a continuous (x, y).
fn value_noise(x: f32, y: f32, seed: u32) -> f32 {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let (xi, yi) = (x0 as i32, y0 as i32);
    let n00 = cloud_hash(xi, yi, seed);
    let n10 = cloud_hash(xi + 1, yi, seed);
    let n01 = cloud_hash(xi, yi + 1, seed);
    let n11 = cloud_hash(xi + 1, yi + 1, seed);
    let nx0 = n00 + (n10 - n00) * sx;
    let nx1 = n01 + (n11 - n01) * sx;
    nx0 + (nx1 - nx0) * sy
}

/// Fractional Brownian motion: `octaves` layers of value noise, each at
/// double the frequency and half the weight of the last, normalised to
/// roughly [0, 1].
fn fbm(x: f32, y: f32, seed: u32, octaves: u32) -> f32 {
    let (mut total, mut amp, mut freq, mut norm) = (0.0, 0.5, 1.0, 0.0);
    for o in 0..octaves {
        total += value_noise(x * freq, y * freq, seed.wrapping_add(o * 101)) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    total / norm
}

/// One cloud sprite variant. `seed` picks the noise field (so each variant
/// is a different cloud, not a recolour of the same one).
fn cloud_sprite(seed: u32) -> Vec<u8> {
    let (w, h) = (CLOUD_TEX_W, CLOUD_TEX_H);
    let mut img = Image::new(w as u32, h as u32);
    let cx = w as f32 * 0.5;
    let cy = h as f32 * 0.58; // envelope centred a little low: flatter base, more room to billow up top

    for y in 0..h {
        let vshade = (y as f32 / h as f32).clamp(0.0, 1.0); // 0 at top (lit) -> 1 at base (shadowed)
        for x in 0..w {
            let nx = (x as f32 - cx) / (w as f32 * 0.46);
            let ny = (y as f32 - cy) / (h as f32 * 0.40);
            // Envelope: an ellipse squashed harder below centre than above —
            // the flat-bottomed cumulus silhouette instead of a round blob.
            let ny_shaped = if ny > 0.0 { ny * 1.7 } else { ny * 0.9 };
            let env = (nx * nx + ny_shaped * ny_shaped).sqrt();

            let n = fbm(x as f32 * 0.22, y as f32 * 0.22, seed, 4);
            // More broken-up/turbulent silhouette toward the top than the base.
            let top_bias = (0.5 - y as f32 / h as f32).max(0.0) * 0.7;
            let shape = (1.0 - env) + (n - 0.5) * (0.5 + top_bias);

            if shape <= 0.05 { continue; }
            let edge = ((shape - 0.05) / 0.22).clamp(0.0, 1.0); // soft, diffused edge

            // Bright, near-white top; greyer, cooler base — the sunlit-top /
            // shadowed-underside look real cumulus has — with the noise
            // value itself brightening the "puffy" high points a little more.
            let lift = (n - 0.5) * 18.0;
            let r = (232.0 - vshade * 60.0 + lift).clamp(0.0, 255.0) as u8;
            let g = (238.0 - vshade * 52.0 + lift).clamp(0.0, 255.0) as u8;
            let b = (248.0 - vshade * 34.0 + lift).clamp(0.0, 255.0) as u8;
            let a = (edge * 235.0) as u8;
            img.set_rgba(x, y, r, g, b, a);
        }
    }
    img.encode_png()
}

// ---------------------------------------------------------------------- //
// Islands — rare, turret-armed landmasses                                  //
// ---------------------------------------------------------------------- //
//
// Same fBm technique as the clouds above, but masked to an opaque landmass
// instead of a soft cloud: an irregular fBm-perturbed coastline (not a
// perfect ellipse — real islands aren't) with a band of sand right at the
// shoreline and mottled green/rock inland, topped with a small turret
// emplacement. Three fixed sizes are baked (small/medium/large); which one
// appears, and where, is picked at runtime.
fn island_sprite(w: i32, h: i32, seed: u32) -> Vec<u8> {
    let mut img = Image::new(w as u32, h as u32);
    let cx = w as f32 * 0.5;
    let cy = h as f32 * 0.5;

    for y in 0..h {
        for x in 0..w {
            let nx = (x as f32 - cx) / (w as f32 * 0.48);
            let ny = (y as f32 - cy) / (h as f32 * 0.44);
            let env = (nx * nx + ny * ny).sqrt();
            let n = fbm(x as f32 * 0.14, y as f32 * 0.14, seed, 4);
            // fBm-perturbed coastline: a lumpy, irregular silhouette rather
            // than a clean ellipse — same idea as the cloud edge above.
            let shape = (1.0 - env) + (n - 0.5) * 0.6;
            if shape <= 0.0 { continue; }

            let (r, g, b) = if shape < 0.16 {
                (200.0 - n * 20.0, 186.0 - n * 20.0, 142.0 - n * 16.0) // sand shoreline
            } else {
                // Mottled green/rock interior — darker in the noise's
                // "valleys" so it doesn't read as one flat colour.
                let dark = n * 46.0;
                (60.0 - dark * 0.5, 98.0 - dark, 50.0 - dark * 0.5)
            };
            img.set(x, y, r.max(20.0) as u8, g.max(30.0) as u8, b.max(18.0) as u8);
        }
    }

    // Turret: a round grey emplacement with a stubby barrel, mounted at the
    // island's high point. Static art — no barrel rotation — since it fires
    // straight at the player procedurally at runtime instead.
    let (tx, ty) = (cx as i32, cy as i32 - 3);
    for yy in -5..5 {
        for xx in -5..5 {
            if (xx * xx) as f32 * 0.7 + (yy * yy) as f32 > 17.0 { continue; }
            let (px, py) = (tx + xx, ty + yy);
            if px < 0 || py < 0 || px >= w || py >= h { continue; }
            img.set(px, py, 96, 98, 102);
        }
    }
    for i in 0..5 {
        let py = ty - 4 - i;
        if py < 0 { break; }
        img.set(tx, py, 62, 64, 68);
    }
    if ty - 6 >= 0 {
        img.set(tx - 1, ty - 6, 40, 42, 46);
        img.set(tx + 1, ty - 6, 40, 42, 46);
    }

    img.encode_png()
}

/// Turret cannon shot — a low tonal thump plus a burst of noise, so it reads
/// as artillery rather than the player's laser-y "pew".
fn turret_fire_sfx() -> Vec<u8> {
    let dur_ms = 140.0;
    let n = ms_to_samples(dur_ms);
    let mut buf = vec![0i16; n];
    let tone = gen_tone(130.0, dur_ms, 0.5);
    let noise = gen_noise(55.0, 0.6);
    for (j, s) in tone.iter().enumerate() { mix_into(&mut buf, j, *s as f32); }
    for (j, s) in noise.iter().enumerate() { mix_into(&mut buf, j, *s as f32); }
    encode_pcm16_mono(&buf)
}

/// A low electrical drone for the laser barrier — looped and dynamically
/// volume-ridden by the game itself as the player nears the beam, so it
/// needs to loop with no audible seam: the 110Hz fundamental (and its
/// harmonics, and the tremolo) all complete a whole number of cycles across
/// the buffer.
fn barrier_hum_sfx() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let dur_ms = 400.0;
    let n = ms_to_samples(dur_ms);
    let f0 = 110.0_f32; // A2 — 44 whole cycles over 400ms
    let trem_hz = 5.0_f32; // 2 whole cycles over 400ms
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let fund = (2.0 * std::f32::consts::PI * f0 * t).sin();
        let harm = (2.0 * std::f32::consts::PI * f0 * 2.0 * t).sin() * 0.4;
        let buzz = (2.0 * std::f32::consts::PI * f0 * 3.0 * t).sin() * 0.22;
        let trem = 0.75 + 0.25 * (2.0 * std::f32::consts::PI * trem_hz * t).sin();
        let shaped = ((fund + harm + buzz) * 0.5).tanh();
        s.push((shaped * trem * 16_000.0) as i16);
    }
    encode_pcm16_mono(&s)
}

// ---------------------------------------------------------------------- //
// Japanese boss name banners                                                //
// ---------------------------------------------------------------------- //
//
// blip's shared bitmap font (crates/blip/src/font.rs) only covers A-Z/0-9,
// so the "boss name in Japanese" banner can't go through it. Instead: the
// katakana glyphs actually needed (one set, shared across all seven names)
// were rasterized *once*, offline, from the real "Noto Sans CJK JP Bold"
// font and baked in below as plain bitmap data. That keeps the game itself
// free of any font dependency — this is the only place that font's shape
// data is used, and only at repo-authoring time, never at build or run time.

/// Katakana glyphs needed for the seven boss names, 10 wide x 12 tall.
const KATAKANA_CHARS: [char; 32] = [
    'ス', 'カ', 'ウ', 'ト', 'ボ', 'マ', 'ー', 'イ', 'ン', 'タ', 'セ', 'プ',
    'ガ', 'シ', 'ッ', 'ド', 'レ', 'ノ', 'バ', 'ル', 'ク', 'ザ', 'ゥ', 'ム',
    'キ', 'ャ', 'リ', 'ア', 'ペ', 'デ', 'ロ', 'ヤ',
];
const KATAKANA_GLYPHS: [[u16; 12]; 32] = [
    [0x000, 0x000, 0x000, 0x0FC, 0x018, 0x008, 0x018, 0x038, 0x06C, 0x0C4, 0x080, 0x000], // ス
    [0x000, 0x000, 0x020, 0x020, 0x0FC, 0x064, 0x024, 0x064, 0x044, 0x0DC, 0x098, 0x000], // カ
    [0x000, 0x000, 0x020, 0x030, 0x0FC, 0x084, 0x084, 0x00C, 0x018, 0x030, 0x020, 0x000], // ウ
    [0x000, 0x000, 0x000, 0x060, 0x060, 0x070, 0x07C, 0x06C, 0x060, 0x060, 0x020, 0x000], // ト
    [0x000, 0x000, 0x006, 0x034, 0x0FC, 0x030, 0x030, 0x0B4, 0x1B6, 0x030, 0x060, 0x000], // ボ
    [0x000, 0x000, 0x000, 0x0FC, 0x0FE, 0x00C, 0x048, 0x078, 0x030, 0x018, 0x008, 0x000], // マ
    [0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x0FC, 0x000, 0x000, 0x000, 0x000, 0x000], // ー
    [0x000, 0x000, 0x000, 0x00C, 0x018, 0x030, 0x0F0, 0x090, 0x010, 0x010, 0x010, 0x000], // イ
    [0x000, 0x000, 0x000, 0x0C0, 0x060, 0x006, 0x004, 0x00C, 0x038, 0x0E0, 0x0C0, 0x000], // ン
    [0x000, 0x000, 0x020, 0x03C, 0x07C, 0x0CC, 0x0B8, 0x018, 0x03C, 0x060, 0x040, 0x000], // タ
    [0x000, 0x000, 0x000, 0x040, 0x07C, 0x1FC, 0x0CC, 0x048, 0x040, 0x07C, 0x03C, 0x000], // セ
    [0x000, 0x000, 0x006, 0x0FE, 0x0FC, 0x00C, 0x008, 0x018, 0x010, 0x070, 0x040, 0x000], // プ
    [0x000, 0x000, 0x026, 0x024, 0x0FC, 0x07C, 0x024, 0x064, 0x044, 0x0DC, 0x098, 0x000], // ガ
    [0x000, 0x000, 0x000, 0x060, 0x020, 0x084, 0x0C4, 0x00C, 0x038, 0x0F0, 0x0C0, 0x000], // シ
    [0x000, 0x000, 0x000, 0x000, 0x000, 0x0A4, 0x0D4, 0x00C, 0x008, 0x030, 0x060, 0x000], // ッ
    [0x000, 0x000, 0x000, 0x04C, 0x040, 0x060, 0x078, 0x04C, 0x040, 0x040, 0x040, 0x000], // ド
    [0x000, 0x000, 0x000, 0x0C0, 0x0C0, 0x0C0, 0x0C4, 0x0CC, 0x0D8, 0x0F0, 0x040, 0x000], // レ
    [0x000, 0x000, 0x000, 0x00C, 0x008, 0x008, 0x018, 0x030, 0x060, 0x0C0, 0x000, 0x000], // ノ
    [0x000, 0x000, 0x006, 0x00C, 0x048, 0x048, 0x04C, 0x0C4, 0x084, 0x186, 0x000, 0x000], // バ
    [0x000, 0x000, 0x000, 0x050, 0x050, 0x050, 0x050, 0x052, 0x0DC, 0x098, 0x010, 0x000], // ル
    [0x000, 0x000, 0x020, 0x030, 0x07C, 0x0CC, 0x08C, 0x018, 0x010, 0x070, 0x040, 0x000], // ク
    [0x000, 0x000, 0x000, 0x04C, 0x0FC, 0x1FC, 0x048, 0x048, 0x018, 0x010, 0x020, 0x000], // ザ
    [0x000, 0x000, 0x000, 0x000, 0x030, 0x0FC, 0x0CC, 0x08C, 0x008, 0x018, 0x030, 0x000], // ゥ
    [0x000, 0x000, 0x000, 0x020, 0x020, 0x060, 0x048, 0x04C, 0x0CC, 0x1FE, 0x000, 0x000], // ム
    [0x000, 0x000, 0x020, 0x020, 0x0FC, 0x0F0, 0x03C, 0x0FC, 0x0B0, 0x010, 0x010, 0x000], // キ
    [0x000, 0x000, 0x000, 0x000, 0x040, 0x07C, 0x0FC, 0x028, 0x020, 0x020, 0x030, 0x000], // ャ
    [0x000, 0x000, 0x000, 0x0CC, 0x0CC, 0x0CC, 0x0CC, 0x008, 0x008, 0x038, 0x020, 0x000], // リ
    [0x000, 0x000, 0x000, 0x0FE, 0x004, 0x03C, 0x038, 0x020, 0x020, 0x060, 0x040, 0x000], // ア
    [0x000, 0x000, 0x000, 0x004, 0x06C, 0x070, 0x0D8, 0x18C, 0x004, 0x006, 0x000, 0x000], // ペ
    [0x000, 0x000, 0x006, 0x0FC, 0x000, 0x0FC, 0x0FC, 0x030, 0x020, 0x060, 0x040, 0x000], // デ
    [0x000, 0x000, 0x000, 0x0FC, 0x0FC, 0x084, 0x084, 0x084, 0x084, 0x0FC, 0x084, 0x000], // ロ
    [0x000, 0x000, 0x040, 0x044, 0x07C, 0x1EC, 0x068, 0x020, 0x020, 0x030, 0x030, 0x000], // ヤ
];

/// The seven boss names, transliterated into katakana (they're all foreign
/// loanwords, so katakana — not kanji — is the linguistically correct
/// choice) in the same order as BOSS_SPECS.
const BOSS_NAMES_JA: [&[char]; 7] = [
    &['ス', 'カ', 'ウ', 'ト', 'ボ', 'マ', 'ー'],                                    // SCOUT BOMBER
    &['イ', 'ン', 'タ', 'ー', 'セ', 'プ', 'タ', 'ー'],                              // INTERCEPTOR
    &['ガ', 'ン', 'シ', 'ッ', 'プ'],                                               // GUNSHIP
    &['ド', 'レ', 'ッ', 'ド', 'ノ', 'ー', 'ト'],                                    // DREADNOUGHT
    &['バ', 'ト', 'ル', 'ク', 'ル', 'ー', 'ザ', 'ー'],                              // BATTLE CRUISER
    &['ド', 'ゥ', 'ー', 'ム', 'キ', 'ャ', 'リ', 'ア'],                              // DOOM CARRIER
    &['ア', 'ペ', 'ッ', 'ク', 'ス', 'デ', 'ス', 'ト', 'ロ', 'イ', 'ヤ', 'ー'],       // APEX DESTROYER
];

const KATAKANA_GW: u32 = 10;
const KATAKANA_GH: u32 = 12;
const KATAKANA_GAP: u32 = 1;

/// Render a katakana string as a bitmap: glyphs left to right with a 1px
/// gap, each pixel blown up `scale`x so it reads clearly at game resolution
/// despite the tiny native glyph size.
fn katakana_image(text: &[char], scale: u32) -> Image {
    let n = text.len() as u32;
    let w = (n * (KATAKANA_GW + KATAKANA_GAP)).saturating_sub(KATAKANA_GAP) * scale;
    let h = KATAKANA_GH * scale;
    let mut img = Image::new(w.max(1), h.max(1));
    for (i, c) in text.iter().enumerate() {
        let Some(gi) = KATAKANA_CHARS.iter().position(|k| k == c) else { continue };
        let glyph = &KATAKANA_GLYPHS[gi];
        let ox = i as u32 * (KATAKANA_GW + KATAKANA_GAP) * scale;
        for row in 0..KATAKANA_GH {
            let bits = glyph[row as usize];
            for col in 0..KATAKANA_GW {
                if (bits >> (KATAKANA_GW - 1 - col)) & 1 == 0 { continue; }
                for sy in 0..scale {
                    for sx in 0..scale {
                        img.set((ox + col * scale + sx) as i32, (row * scale + sy) as i32, 255, 255, 255);
                    }
                }
            }
        }
    }
    img
}

fn boss_name_ja(tier: usize) -> Vec<u8> {
    katakana_image(BOSS_NAMES_JA[tier], 2).encode_png()
}

// ---------------------------------------------------------------------- //
// Music — rock                                                             //
// ---------------------------------------------------------------------- //
//
// Raider's theme is a rock instrumental now, not the parade-ground march
// it grew out of: a distorted rhythm-guitar riff, a live drum kit with a
// cracking backbeat, a picked bass welded to the riff root, and a lead
// guitar that steps out for a full solo in the middle. Like the march, it
// is deliberately NOT assembled from the shared `techno.rs`
// kick/clap/supersaw toolkit every other blip game's music is built from
// — Raider carries its own voices (`power_chord`, `lead_guitar`, and a
// rock kit) so it still reads as its own band on the jukebox rather than
// "the EDM games, plus one".
//
// Every guitar voice runs the signal chain a real rig has: a raw
// oscillator (detuned saws for the rhythm, a saw/square blend for the
// lead) driven hard into a `tanh` clipper — the amp — then a one-pole
// low-pass — the speaker cabinet — to roll the fizz off the top. Voices
// mix into a shared f32 buffer that's soft-limited once at the end, so a
// chord, a kick and the bass all landing on the downbeat compress
// gracefully instead of hard-clipping.

/// Rock kick — tight and dry with a hard beater click, tuned to punch
/// through a wall of distorted guitar. Shorter and with far less "boom"
/// than `techno::kick`; the click, not the body, is what stays audible
/// once the amps are going.
fn rock_kick(buf: &mut [f32], off: usize, vol: f32) {
    let sr = SAMPLE_RATE as f32;
    let n = (sr * 0.11) as usize;
    let click_n = (sr * 0.004) as usize;
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let t = i as f32 / sr;
        let e = (1.0 - i as f32 / n as f32).powf(2.1);
        let freq = 48.0 + 95.0 * (-t / 0.028).exp(); // 143 Hz -> 48 Hz, fast
        let body = (2.0 * PI * freq * t).sin();
        let click = if i < click_n {
            let ce = 1.0 - i as f32 / click_n as f32;
            let cn = ((i as u32).wrapping_mul(2_654_435_761) as f32 / u32::MAX as f32) * 2.0 - 1.0;
            cn * ce * 0.8
        } else {
            0.0
        };
        mix_into_f32(buf, off + i, (body * 1.5 + click).tanh() * e * vol * 20_000.0);
    }
}

/// Rock snare — a bright noise crack over two tuned shell modes, with a
/// real ringing decay (unlike the dry, choked `march_snare`): this is the
/// backbeat the whole groove leans on, so it needs to carry.
fn rock_snare(buf: &mut [f32], off: usize, rng: &mut Rng, vol: f32) {
    let sr = SAMPLE_RATE as f32;
    let n = (sr * 0.15) as usize;
    let mut prev = 0.0f32;
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let t = i as f32 / sr;
        let e = (1.0 - i as f32 / n as f32).powf(1.5);
        let white = rng.next_f32() * 2.0 - 1.0;
        let hp = white - prev; // crude 1st-order highpass -> "snap"
        prev = white;
        let shell = ((2.0 * PI * 185.0 * t).sin() + 0.6 * (2.0 * PI * 331.0 * t).sin()) * 0.3;
        mix_into_f32(buf, off + i, ((hp * 0.9 + shell) * e).tanh() * vol * 14_000.0);
    }
}

/// Hi-hat / ride tick — bright filtered noise. `open` swaps the tight
/// closed-hat blip for a longer, washier decay.
fn rock_hat(buf: &mut [f32], off: usize, rng: &mut Rng, vol: f32, open: bool) {
    let n = (SAMPLE_RATE as f32 * if open { 0.19 } else { 0.038 }) as usize;
    let mut prev = 0.0f32;
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let e = (1.0 - i as f32 / n as f32).powf(if open { 1.4 } else { 3.2 });
        let white = rng.next_f32() * 2.0 - 1.0;
        let hp = white - prev;
        prev = white;
        mix_into_f32(buf, off + i, hp * e * vol * 7_000.0);
    }
}

/// Crash cymbal — a long noise wash with a couple of inharmonic partials
/// for shimmer. Marks the top of a section.
fn crash(buf: &mut [f32], off: usize, rng: &mut Rng, vol: f32) {
    let sr = SAMPLE_RATE as f32;
    let n = (sr * 0.85) as usize;
    let mut prev = 0.0f32;
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let t = i as f32 / sr;
        let e = (1.0 - i as f32 / n as f32).powf(1.7);
        let white = rng.next_f32() * 2.0 - 1.0;
        let hp = white - 0.7 * prev;
        prev = white;
        let shimmer = 0.15 * ((2.0 * PI * 5_300.0 * t).sin() + (2.0 * PI * 7_100.0 * t).sin());
        mix_into_f32(buf, off + i, (hp * 0.85 + shimmer) * e * vol * 6_500.0);
    }
}

/// Distorted rhythm-guitar power chord: root + fifth + octave, each a pair
/// of very slightly detuned saws, summed and driven through the
/// amp/cabinet chain. `palm` picks the articulation — `true` chokes it
/// into a short, dark palm-muted chug; `false` lets it ring open and
/// bright.
fn power_chord(buf: &mut [f32], off: usize, root: f32, ms: f32, vol: f32, palm: bool) {
    let sr = SAMPLE_RATE as f32;
    let n = (sr * ms / 1000.0) as usize;
    if n == 0 { return; }
    let att = (sr * 0.0018) as usize + 1;
    let decay_pow = if palm { 2.6 } else { 0.7 };
    let drive = if palm { 7.5 } else { 11.0 };
    let cutoff = if palm { 2_500.0 } else { 3_400.0 };
    let alpha = 1.0 - (-2.0 * PI * cutoff / sr).exp();
    // root, fifth (3:2), octave — the fifth kept a touch quieter so the
    // chord has a root rather than a hollow parallel-fifths drone.
    const IVL: [(f32, f32); 3] = [(1.0, 1.0), (1.5, 0.7), (2.0, 0.9)];
    const DETUNE: f32 = 0.004;
    let mut ph = [0.0f32; 6];
    let mut lp = 0.0f32;
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let a = if i < att {
            i as f32 / att as f32
        } else {
            (1.0 - (i - att) as f32 / (n - att).max(1) as f32).powf(decay_pow)
        };
        let mut raw = 0.0f32;
        let mut k = 0;
        for &(mult, w) in &IVL {
            for d in [-1.0f32, 1.0] {
                let f = root * mult * (1.0 + d * DETUNE);
                ph[k] += f / sr;
                ph[k] -= ph[k].floor();
                raw += (2.0 * ph[k] - 1.0) * w;
                k += 1;
            }
        }
        let driven = (raw / 5.0 * drive).tanh();
        lp += alpha * (driven - lp);
        mix_into_f32(buf, off + i, lp * a * vol * 15_000.0);
    }
}

/// Lead guitar for the solo — a driven saw/square blend with a delayed
/// finger vibrato, an optional pick-attack bend up into the target pitch,
/// and a slight volume swell toward the tail that stands in for a held
/// note blooming into amp feedback. `bend` is how many semitones the note
/// slides up from on the attack (0.0 = struck clean).
fn lead_guitar(buf: &mut [f32], off: usize, freq: f32, ms: f32, vol: f32, bend: f32) {
    let sr = SAMPLE_RATE as f32;
    let n = (sr * ms / 1000.0) as usize;
    if n == 0 { return; }
    let att = (sr * 0.006) as usize + 1;
    let bend_from = 2f32.powf(-bend / 12.0);
    let bend_n = ((sr * 0.065) as usize).max(1);
    let alpha = 1.0 - (-2.0 * PI * 3_200.0 / sr).exp();
    let mut ph = 0.0f32;
    let mut lp = 0.0f32;
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let t = i as f32 / sr;
        let prog = i as f32 / n as f32;
        let a = if i < att {
            i as f32 / att as f32
        } else {
            let body = (1.0 - prog).powf(0.45);
            let bloom = 1.0 + 0.55 * prog.powf(3.0);
            (body * bloom).min(1.35)
        };
        let vib_on = ((t - 0.11) / 0.06).clamp(0.0, 1.0);
        let vib = 1.0 + vib_on * 0.014 * (2.0 * PI * 5.7 * t).sin();
        let b = if i < bend_n {
            let f = i as f32 / bend_n as f32;
            bend_from + (1.0 - bend_from) * (f * f) // ease-in, like a finger push
        } else {
            1.0
        };
        let f = freq * vib * b;
        ph += f / sr;
        ph -= ph.floor();
        let saw = 2.0 * ph - 1.0;
        let sq = if ph < 0.5 { 1.0 } else { -1.0 };
        let driven = ((saw * 0.7 + sq * 0.3) * 6.0).tanh();
        lp += alpha * (driven - lp);
        let sing = 0.12 * (2.0 * PI * 2.0 * f * t).sin(); // octave-up edge
        mix_into_f32(buf, off + i, (lp + sing) * a * vol * 12_000.0);
    }
}

/// Picked electric bass — a mildly overdriven saw with a reinforcing sine
/// at the fundamental, locked to the rhythm-guitar root. Medium decay, a
/// little pick grit; sits under the guitars without fighting them.
fn bass_guitar(buf: &mut [f32], off: usize, freq: f32, ms: f32, vol: f32) {
    let sr = SAMPLE_RATE as f32;
    let n = (sr * ms / 1000.0) as usize;
    if n == 0 { return; }
    let att = (sr * 0.003) as usize + 1;
    let alpha = 1.0 - (-2.0 * PI * 1_700.0 / sr).exp();
    let mut ph = 0.0f32;
    let mut lp = 0.0f32;
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let t = i as f32 / sr;
        let a = if i < att {
            i as f32 / att as f32
        } else {
            (1.0 - (i - att) as f32 / (n - att).max(1) as f32).powf(1.4)
        };
        ph += freq / sr;
        ph -= ph.floor();
        let driven = ((2.0 * ph - 1.0) * 2.3).tanh();
        lp += alpha * (driven - lp);
        let sub = (2.0 * PI * freq * t).sin();
        mix_into_f32(buf, off + i, (lp * 0.8 + sub * 0.5) * a * vol * 16_000.0);
    }
}

/// Guitar dive bomb — the whammy-bar drop that ends a solo: pitch craters
/// from `freq` toward nothing while the note blooms once and then chokes.
fn dive_bomb(buf: &mut [f32], off: usize, freq: f32, ms: f32, vol: f32) {
    let sr = SAMPLE_RATE as f32;
    let n = (sr * ms / 1000.0) as usize;
    if n == 0 { return; }
    let alpha = 1.0 - (-2.0 * PI * 2_600.0 / sr).exp();
    let mut ph = 0.0f32;
    let mut lp = 0.0f32;
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let prog = i as f32 / n as f32;
        let f = (freq * (1.0 - prog).powf(2.2)).max(18.0);
        let e = (1.0 - prog).powf(1.3) * (1.0 + 0.4 * prog);
        ph += f / sr;
        ph -= ph.floor();
        let saw = 2.0 * ph - 1.0;
        let sq = if ph < 0.5 { 1.0 } else { -1.0 };
        let driven = ((saw * 0.6 + sq * 0.4) * 7.0).tanh();
        lp += alpha * (driven - lp);
        mix_into_f32(buf, off + i, lp * e * vol * 12_000.0);
    }
}

/// One in-bar step of the core rock beat: kick on 1, the "and" of 2, 3 and
/// the "and" of 4; snare backbeat on 2 and 4; straight 8th-note hats, with
/// an open hat lifting the last off-beat. `busy` doubles the hats to 16ths
/// for the higher-energy solo section.
fn rock_beat_step(buf: &mut [f32], off: usize, pos: usize, rng: &mut Rng, busy: bool) {
    const KICK: [bool; 16] = [
        true, false, false, false, false, false, true, false,
        true, false, false, false, false, false, true, false,
    ];
    if KICK[pos] {
        rock_kick(buf, off, 0.9);
    }
    if pos == 4 || pos == 12 {
        rock_snare(buf, off, rng, 0.6);
    }
    if busy || pos % 2 == 0 {
        rock_hat(buf, off, rng, if pos % 4 == 0 { 0.22 } else { 0.16 }, false);
    }
    if pos == 14 {
        rock_hat(buf, off, rng, 0.16, true);
    }
}

/// A one-bar snare fill: rising 16th-note hits across the back half of the
/// bar, capped with a crash on the downbeat that follows (left to the
/// caller). The march form's crescendo roll, re-scored for a kit.
fn drum_fill(buf: &mut [f32], bar_off: usize, step_samples: usize, rng: &mut Rng) {
    for h in 0..8 {
        let off = bar_off + step_samples * (8 + h);
        rock_snare(buf, off, rng, 0.22 + 0.055 * h as f32);
    }
}

/// The main Raider theme: a driving rock instrumental in E minor.
///
/// Form is intro / verse / chorus / guitar solo / chorus, looped. The
/// verse is a palm-muted gallop riff on the open low E with a short
/// minor-triad answer at the top of every second bar — the hook, and it
/// never changes, because changing the hook is how you lose it. The
/// chorus opens up: rung-out power chords walking Em–C–G–D under a
/// held, singing lead line. The solo takes eight bars over the gallop
/// (the last four moving through the chorus changes for somewhere to go),
/// climbs a pentatonic run to a bent-and-held high note, and drops off a
/// dive bomb straight back into the last chorus. An original composition.
fn music() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let bpm = 150.0_f32;
    let steps_per_bar = 16usize;
    let step_ms = 60_000.0 / bpm / 4.0; // 16th note = 100 ms
    let step_samples = (sr * step_ms / 1000.0) as usize;

    // intro(1) + verse(8) + chorus(4) + solo(8) + chorus(4)
    const INTRO: usize = 1;
    const VERSE: usize = INTRO + 8;
    const CHORUS: usize = VERSE + 4;
    const SOLO: usize = CHORUS + 8;
    const BARS: usize = SOLO + 4; // 25

    let total_steps = BARS * steps_per_bar;
    let total = step_samples * total_steps + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0x5217_9111);

    // E minor. Low roots for the rhythm guitar and bass.
    const E2: f32 = 82.41;
    const G2: f32 = 98.00;
    const A2: f32 = 110.00;
    const B2: f32 = 123.47;
    const C3: f32 = 130.81;
    const D3: f32 = 146.83;

    // The verse hook: a 2-bar phrase, (step-in-phrase, root, dur-in-steps,
    // palm-muted?). Gallop of open-E chugs, then the triad answer.
    const RIFF: [(usize, f32, f32, bool); 20] = [
        (0, E2, 1.3, true), (2, E2, 1.3, true), (3, E2, 1.3, true),
        (4, E2, 1.3, true), (6, E2, 1.3, true), (7, E2, 1.3, true),
        (8, E2, 1.3, true), (10, E2, 1.3, true), (11, E2, 1.3, true),
        (12, G2, 2.0, false), (14, A2, 2.0, false),
        (16, E2, 1.3, true), (18, E2, 1.3, true), (19, E2, 1.3, true),
        (20, E2, 1.3, true), (22, E2, 1.3, true), (23, E2, 1.3, true),
        (24, B2, 2.0, false), (26, A2, 2.0, false), (30, E2, 4.0, false),
    ];
    // Bass under the verse — root notes, a little detached.
    const RIFF_BASS: [(usize, f32); 11] = [
        (0, E2), (4, E2), (8, E2), (12, G2), (14, A2),
        (16, E2), (20, E2), (24, B2), (26, A2), (28, G2), (30, E2),
    ];

    // Chorus changes, one chord per bar, plus the lead line sitting on top.
    const CHORDS: [f32; 4] = [E2, C3, G2, D3]; // Em - C - G - D
    const CH_LEAD: [(f32, f32); 4] = [
        (493.88, 0.0),  // B4
        (523.25, 0.0),  // C5
        (587.33, 0.0),  // D5
        (493.88, 2.0),  // B4, bent up
    ];

    // The solo, as (absolute-step-from-solo-start, freq, dur-in-steps, bend).
    const LEAD: [(usize, f32, f32, f32); 34] = [
        // bar 0 — pickup, then bend up a tone into a held D5
        (0, 440.00, 2.0, 0.0), (2, 493.88, 2.0, 0.0), (4, 587.33, 12.0, 2.0),
        // bar 1 — pentatonic run down
        (16, 659.25, 2.0, 0.0), (18, 587.33, 2.0, 0.0), (20, 493.88, 2.0, 0.0),
        (22, 440.00, 2.0, 0.0), (24, 392.00, 2.0, 0.0), (26, 329.63, 6.0, 0.0),
        // bar 2 — call, ending on a bent E5
        (32, 493.88, 3.0, 0.0), (35, 587.33, 3.0, 0.0), (38, 659.25, 10.0, 2.0),
        // bar 3 — answer
        (48, 587.33, 3.0, 0.0), (51, 493.88, 3.0, 0.0), (54, 440.00, 3.0, 0.0),
        (57, 493.88, 7.0, 0.0),
        // bar 4 — fast run up the scale to a bent A5
        (64, 329.63, 1.0, 0.0), (65, 392.00, 1.0, 0.0), (66, 440.00, 1.0, 0.0),
        (67, 493.88, 1.0, 0.0), (68, 587.33, 1.0, 0.0), (69, 659.25, 1.0, 0.0),
        (70, 783.99, 1.0, 0.0), (71, 880.00, 8.0, 1.0),
        // bar 5 — rhythmic top-note phrase
        (80, 659.25, 2.0, 0.0), (82, 659.25, 2.0, 0.0), (84, 783.99, 2.0, 0.0),
        (86, 659.25, 2.0, 0.0), (88, 587.33, 2.0, 0.0), (90, 493.88, 6.0, 0.0),
        // bar 6 — climax: a huge held bent B5, vibrato and feedback bloom
        (96, 987.77, 16.0, 2.0),
        // bar 7 — resolve down (the dive bomb is placed separately)
        (112, 880.00, 2.0, 0.0), (114, 783.99, 2.0, 0.0), (116, 659.25, 2.0, 0.0),
    ];

    for step in 0..total_steps {
        let bar = step / steps_per_bar;
        let pos = step % steps_per_bar;
        let off = step * step_samples;
        let bar_off = bar * steps_per_bar * step_samples;

        let in_intro = bar < INTRO;
        let in_verse = (INTRO..VERSE).contains(&bar);
        let in_chorus = (VERSE..CHORUS).contains(&bar) || (SOLO..BARS).contains(&bar);
        let in_solo = (CHORUS..SOLO).contains(&bar);

        // Section-top crash.
        if pos == 0 && (bar == VERSE || bar == CHORUS || bar == SOLO || bar == BARS - 4) {
            crash(&mut buf, off, &mut rng, 0.5);
        }

        // ---- Drums ----
        if in_intro {
            // count-in fill only
            if pos >= 8 {
                rock_snare(&mut buf, off, &mut rng, 0.20 + 0.05 * (pos - 8) as f32);
            }
        } else {
            let last_of_section = (in_verse && bar == VERSE - 1)
                || (in_solo && bar == SOLO - 1)
                || (in_chorus && (bar == CHORUS - 1 || bar == BARS - 1));
            if last_of_section {
                if pos == 0 {
                    drum_fill(&mut buf, bar_off, step_samples, &mut rng);
                }
                // keep the kick pulse under the fill
                if pos == 0 || pos == 8 {
                    rock_kick(&mut buf, off, 0.85);
                }
            } else {
                rock_beat_step(&mut buf, off, pos, &mut rng, in_solo);
            }
        }

        // ---- Rhythm guitar + bass ----
        if in_intro {
            if pos == 0 {
                power_chord(&mut buf, off, E2, step_ms * 16.0, 0.42, false);
                bass_guitar(&mut buf, off, E2, step_ms * 14.0, 0.5);
            }
        } else if in_verse {
            let phase_step = ((bar - INTRO) % 2) * steps_per_bar + pos;
            for &(s, root, dur, palm) in &RIFF {
                if s == phase_step {
                    power_chord(&mut buf, off, root, step_ms * dur, if palm { 0.5 } else { 0.44 }, palm);
                }
            }
            for &(s, root) in &RIFF_BASS {
                if s == phase_step {
                    bass_guitar(&mut buf, off, root, step_ms * 3.0, 0.5);
                }
            }
        } else if in_chorus {
            let ch = if bar < SOLO { bar - VERSE } else { bar - SOLO };
            let root = CHORDS[ch % 4];
            if pos == 0 {
                power_chord(&mut buf, off, root, step_ms * 15.5, 0.5, false);
            }
            if pos == 8 {
                power_chord(&mut buf, off, root, step_ms * 7.5, 0.42, false);
            }
            if pos % 2 == 0 {
                bass_guitar(&mut buf, off, root, step_ms * 1.7, 0.5);
            }
            if pos == 0 {
                let (f, bnd) = CH_LEAD[ch % 4];
                lead_guitar(&mut buf, off, f, step_ms * 12.0, 0.42, bnd);
            }
        } else if in_solo {
            // gallop under the solo; last four bars walk the chorus changes
            let sbar = bar - CHORUS;
            let root = if sbar < 4 { E2 } else { CHORDS[(sbar - 4) % 4] };
            const GALLOP: [usize; 12] = [0, 2, 3, 4, 6, 7, 8, 10, 11, 12, 14, 15];
            if GALLOP.contains(&pos) {
                power_chord(&mut buf, off, root, step_ms * 1.3, 0.42, true);
            }
            if pos == 0 || pos == 4 || pos == 8 || pos == 12 {
                bass_guitar(&mut buf, off, root, step_ms * 3.0, 0.48);
            }
        }

        // ---- Lead solo ----
        if in_solo {
            let solo_step = (bar - CHORUS) * steps_per_bar + pos;
            for &(s, f, dur, bend) in &LEAD {
                if s == solo_step {
                    lead_guitar(&mut buf, off, f, step_ms * dur, 0.6, bend);
                }
            }
            if solo_step == 120 {
                dive_bomb(&mut buf, off, 659.25, step_ms * 7.0, 0.55);
            }
        }
    }

    // A soft sustained low-E drone under the whole piece, glueing the loop.
    for bar in 0..BARS {
        let bar_off = bar * steps_per_bar * step_samples;
        power_chord(&mut buf, bar_off, E2, step_ms * steps_per_bar as f32 * 1.02, 0.06, false);
    }

    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// The second loop in Raider's rotation — same band, harder and faster: a
/// drop-D thrash riff in D minor at 176 BPM, tremolo-picked chugs with a
/// chromatic breakdown accent, straight 8th-note kicks underneath, and a
/// shred solo that ends on a screaming pinch harmonic and a dive bomb.
/// No chorus, no let-up: one relentless riff either side of the solo, so
/// over a long level it answers the main theme's developed form with a
/// pure adrenaline hit.
fn music2() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let bpm = 176.0_f32;
    let steps_per_bar = 16usize;
    let step_ms = 60_000.0 / bpm / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;

    const INTRO: usize = 1;
    const RIFF_A: usize = INTRO + 6;
    const SOLO: usize = RIFF_A + 6;
    const BARS: usize = SOLO + 3; // 16

    let total_steps = BARS * steps_per_bar;
    let total = step_samples * total_steps + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0x5217_9222);

    const D2: f32 = 73.42;
    const EB2: f32 = 77.78;
    const F2: f32 = 87.31;
    const C3: f32 = 130.81;

    // 1-bar drop-D riff: tremolo chug on the low D, then a
    // chromatic F–D–C–D–Eb–D breakdown accent.
    const RIFF: [(usize, f32, f32, bool); 15] = [
        (0, D2, 1.1, true), (1, D2, 1.1, true), (2, D2, 1.1, true), (3, D2, 1.1, true),
        (4, D2, 1.1, true), (5, D2, 1.1, true), (6, D2, 1.1, true), (7, D2, 1.1, true),
        (8, F2, 2.0, false), (10, D2, 1.1, true), (11, C3, 1.5, false), (12, D2, 1.1, true),
        (13, EB2, 1.3, false), (14, D2, 1.1, true), (15, D2, 1.1, true),
    ];

    // The shred solo, (absolute-step-from-solo-start, freq, dur-in-steps, bend).
    const LEAD: [(usize, f32, f32, f32); 28] = [
        // bar 0 — bend up into a held D5
        (0, 440.00, 2.0, 0.0), (2, 523.25, 2.0, 0.0), (4, 587.33, 10.0, 2.0),
        // bar 1 — fast run down D minor pentatonic
        (16, 587.33, 1.0, 0.0), (17, 523.25, 1.0, 0.0), (18, 440.00, 1.0, 0.0),
        (19, 392.00, 1.0, 0.0), (20, 349.23, 1.0, 0.0), (21, 293.66, 1.0, 0.0),
        (22, 349.23, 2.0, 0.0), (24, 293.66, 8.0, 0.0),
        // bar 2 — bend up into a held F5
        (32, 440.00, 2.0, 0.0), (34, 587.33, 2.0, 0.0), (36, 698.46, 10.0, 3.0),
        // bar 3 — tremolo alternation, then a held D5
        (48, 587.33, 1.0, 0.0), (49, 698.46, 1.0, 0.0), (50, 587.33, 1.0, 0.0),
        (51, 698.46, 1.0, 0.0), (52, 880.00, 1.0, 0.0), (53, 698.46, 1.0, 0.0),
        (54, 587.33, 1.0, 0.0), (55, 440.00, 2.0, 0.0), (57, 587.33, 7.0, 0.0),
        // bar 4 — pinch-harmonic screams way up top
        (64, 880.00, 6.0, 1.0), (70, 1174.66, 6.0, 2.0),
        // bar 5 — descend into the dive (placed separately)
        (80, 1174.66, 1.0, 0.0), (81, 880.00, 1.0, 0.0), (82, 698.46, 1.0, 0.0),
    ];

    for step in 0..total_steps {
        let bar = step / steps_per_bar;
        let pos = step % steps_per_bar;
        let off = step * step_samples;
        let bar_off = bar * steps_per_bar * step_samples;

        let in_intro = bar < INTRO;
        let in_solo = (RIFF_A..SOLO).contains(&bar);
        let last_bar = bar == RIFF_A - 1 || bar == SOLO - 1 || bar == BARS - 1;

        if pos == 0 && (bar == INTRO || bar == RIFF_A || bar == SOLO) {
            crash(&mut buf, off, &mut rng, 0.5);
        }

        // ---- Drums: straight 8th kicks, backbeat, busy hats ----
        if in_intro {
            if pos >= 8 {
                rock_snare(&mut buf, off, &mut rng, 0.22 + 0.05 * (pos - 8) as f32);
            }
        } else if last_bar {
            if pos == 0 {
                drum_fill(&mut buf, bar_off, step_samples, &mut rng);
            }
            if pos % 4 == 0 {
                rock_kick(&mut buf, off, 0.85);
            }
        } else {
            if pos % 2 == 0 {
                rock_kick(&mut buf, off, 0.88);
            }
            if pos == 4 || pos == 12 {
                rock_snare(&mut buf, off, &mut rng, 0.6);
            }
            rock_hat(&mut buf, off, &mut rng, if pos % 4 == 0 { 0.2 } else { 0.14 }, false);
        }

        // ---- Rhythm guitar + bass ----
        if in_intro {
            if pos == 0 {
                power_chord(&mut buf, off, D2, step_ms * 16.0, 0.42, false);
                bass_guitar(&mut buf, off, D2, step_ms * 14.0, 0.5);
            }
        } else if !last_bar || in_solo {
            for &(s, root, dur, palm) in &RIFF {
                if s == pos {
                    power_chord(&mut buf, off, root, step_ms * dur, if palm { 0.5 } else { 0.44 }, palm);
                }
            }
            if pos % 2 == 0 {
                bass_guitar(&mut buf, off, D2, step_ms * 1.4, 0.5);
            }
        } else {
            // the fill bar still needs the downbeat chord
            if pos == 0 {
                power_chord(&mut buf, off, D2, step_ms * 4.0, 0.48, false);
            }
        }

        // ---- Lead ----
        if in_solo {
            let solo_step = (bar - RIFF_A) * steps_per_bar + pos;
            for &(s, f, dur, bend) in &LEAD {
                if s == solo_step {
                    lead_guitar(&mut buf, off, f, step_ms * dur, 0.6, bend);
                }
            }
            if solo_step == 83 {
                dive_bomb(&mut buf, off, 587.33, step_ms * 12.0, 0.55);
            }
        }
    }

    for bar in 0..BARS {
        let bar_off = bar * steps_per_bar * step_samples;
        power_chord(&mut buf, bar_off, D2, step_ms * steps_per_bar as f32 * 1.02, 0.06, false);
    }

    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

pub fn generate() -> Vec<Asset> {
    vec![
        ("images/player_plane.png",   player_plane()),
        ("images/enemy_grunt.png",    enemy_plane(0)),
        ("images/enemy_weaver.png",   enemy_plane(1)),
        ("images/enemy_ace.png",      enemy_plane(2)),
        // One boss per level, 1-7, each bigger and gnarlier than the last.
        ("images/boss_1.png",         boss_plane(0)),
        ("images/boss_2.png",         boss_plane(1)),
        ("images/boss_3.png",         boss_plane(2)),
        ("images/boss_4.png",         boss_plane(3)),
        ("images/boss_5.png",         boss_plane(4)),
        ("images/boss_6.png",         boss_plane(5)),
        ("images/boss_7.png",         boss_plane(6)),
        // Japanese banner for each boss name, shown together with the
        // English one on the "WARNING" intro banner.
        ("images/boss_name_ja_1.png", boss_name_ja(0)),
        ("images/boss_name_ja_2.png", boss_name_ja(1)),
        ("images/boss_name_ja_3.png", boss_name_ja(2)),
        ("images/boss_name_ja_4.png", boss_name_ja(3)),
        ("images/boss_name_ja_5.png", boss_name_ja(4)),
        ("images/boss_name_ja_6.png", boss_name_ja(5)),
        ("images/boss_name_ja_7.png", boss_name_ja(6)),
        ("images/powerup.png",        powerup_capsule()),
        ("images/health_pack.png",    health_pack()),
        ("images/carrier.png",        carrier_ship()),
        ("images/boat.png",           boat()),
        // Three distinct noise-generated cloud shapes, cycled between instances.
        ("images/cloud_1.png",        cloud_sprite(0x1DE7_C10D)),
        ("images/cloud_2.png",        cloud_sprite(0x2ACE_C10D)),
        ("images/cloud_3.png",        cloud_sprite(0x3FAD_C10D)),
        // Three sizes of turret-armed island, each a different noise seed
        // so the coastline shape varies, not just the scale.
        ("images/island_small.png",  island_sprite(ISLAND_SIZES[0].0, ISLAND_SIZES[0].1, 0x9A17_1DE0)),
        ("images/island_medium.png", island_sprite(ISLAND_SIZES[1].0, ISLAND_SIZES[1].1, 0x9A17_2DE0)),
        ("images/island_large.png",  island_sprite(ISLAND_SIZES[2].0, ISLAND_SIZES[2].1, 0x9A17_3DE0)),
        ("sounds/turret_fire.wav",    turret_fire_sfx()),
        ("sounds/barrier_hum.wav",    barrier_hum_sfx()),
        ("sounds/shoot.wav",          encode_pcm16_mono(&shoot_sfx())),
        ("sounds/enemy_explode.wav",  encode_pcm16_mono(&gen_noise(220.0, 0.7))),
        ("sounds/player_explode.wav", encode_pcm16_mono(&gen_noise(650.0, 0.9))),
        // A short, quieter crack for a non-lethal hit — reads as "took a
        // glancing blow" rather than player_explode's full "you're down".
        ("sounds/player_hit.wav",     encode_pcm16_mono(&gen_noise(130.0, 0.55))),
        ("sounds/boss_explode.wav",   encode_pcm16_mono(&gen_noise(1100.0, 1.0))),
        ("sounds/boss_warning.wav",   boss_warning_sfx()),
        // Weapon-tier pickup chimes, escalating: more notes, higher register,
        // and a proper fanfare (with a harmony note) for the last one.
        ("sounds/powerup2.wav",       encode_pcm16_mono(&ascending_run(&[880.00, 1318.51], 55.0, 85.0, 0.5))),
        ("sounds/powerup3.wav",       encode_pcm16_mono(&ascending_run(&[987.77, 1479.98], 50.0, 90.0, 0.5))),
        ("sounds/powerup4.wav",       encode_pcm16_mono(&ascending_run(&[740.00, 987.77, 1318.51], 55.0, 95.0, 0.5))),
        ("sounds/max_power.wav",      max_power_sfx()),
        // A gentle rising chime, lower and warmer than the weapon-tier
        // chimes, for catching a health pickup.
        ("sounds/health_pickup.wav",  encode_pcm16_mono(&ascending_run(&[392.00, 523.25, 659.25], 55.0, 90.0, 0.42))),
        ("sounds/stage_clear.wav",    stage_clear_sfx()),
        ("sounds/victory.wav",        victory_sfx()),
        ("sounds/game_over.wav",      game_over_sfx()),
        ("sounds/music.wav",          music()),
        ("sounds/music2.wav",         music2()),
    ]
}
