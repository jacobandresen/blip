//! Raider assets — a 1942-style vertical dogfighter.
//!
//! Sprites are small silhouettes (plane shapes read fine at arcade
//! resolution); bullets and explosions stay plain shape draw calls in the
//! game itself, so no sprites are needed for those here.

use crate::image::Image;
use crate::techno::{Rng, MIX_KNEE};
use crate::wav::{encode_pcm16_mono, env, mix_into, mix_into_f32, ms_to_samples, soft_limit_to_pcm16, SAMPLE_RATE};
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
// Music                                                                    //
// ---------------------------------------------------------------------- //

/// Military march bass drum — a dry, almost-unpitched low thump (unlike the
/// other games' shared `techno::kick`, which glides down in pitch and clicks
/// on the attack). The character of an actual marching-band bass drum, not
/// a synth kick — one of the two things (with `march_snare`) that gives
/// Raider's theme its own identity instead of sharing the kick/clap/hat
/// drum kit every other game's music is built from.
fn march_bass_drum(buf: &mut [f32], off: usize, vol: f32) {
    let sr = SAMPLE_RATE as f32;
    let n = (sr * 0.11) as usize;
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let t = i as f32 / sr;
        let e = (1.0 - i as f32 / n as f32).powf(2.4);
        let body = (2.0 * std::f32::consts::PI * 58.0 * t).sin().tanh();
        mix_into_f32(buf, off + i, body * e * vol * 19_000.0);
    }
}

/// Military snare — a short, dry, sharp noise crack with a little tonal body
/// under it, tighter and higher-pitched than the other games' soft, washy
/// `techno::clap`. Reused at low volume for the steady footfall taps between
/// backbeats, and layered into `march_roll` for fills.
fn march_snare(buf: &mut [f32], off: usize, rng: &mut Rng, vol: f32) {
    let sr = SAMPLE_RATE as f32;
    let n = (sr * 0.075) as usize;
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let t = i as f32 / sr;
        let e = (1.0 - i as f32 / n as f32).powf(1.6);
        let noise = rng.next_f32() * 2.0 - 1.0;
        let body = (2.0 * std::f32::consts::PI * 200.0 * t).sin() * 0.3;
        mix_into_f32(buf, off + i, (noise * 0.85 + body) * e * vol * 12_000.0);
    }
}

/// Brass fanfare stab — additive harmonics (a falling-amplitude stack, not
/// the other games' supersaw pads) driven into gentle saturation for a
/// buzzy, brassy edge on the attack. Used both for the bugle-call melody
/// itself and, at low volume with a long sustain, as the soft pedal drone
/// under the whole piece.
fn brass_stab(buf: &mut [f32], off: usize, freq: f32, ms: f32, vol: f32) {
    let sr = SAMPLE_RATE as f32;
    let n = (sr * ms / 1000.0) as usize;
    let att = ((sr * 0.012) as usize).max(1);
    let rel = (n / 5).max(1);
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let t = i as f32 / sr;
        let e = env(i, n, att, rel);
        let mut tone = 0.0;
        for (k, amp) in [(1.0, 1.0), (2.0, 0.55), (3.0, 0.38), (4.0, 0.22), (5.0, 0.12)] {
            tone += (2.0 * std::f32::consts::PI * freq * k * t).sin() * amp;
        }
        let driven = (tone * 0.32).tanh();
        mix_into_f32(buf, off + i, driven * e * vol * 16_000.0);
    }
}

/// A militaristic march fanfare over an oom-pah drum pattern — deliberately
/// NOT built from the shared techno.rs toolkit every other game's music
/// uses (no four-on-the-floor kick, no supersaw pads, no claps or hi-hats):
/// a dry march bass drum, a real snare crack, and brass stabs, all built
/// only from the notes of one major triad — the way an actual valveless
/// bugle is limited to the harmonic series.
///
/// The first version of this track played one long, through-composed
/// 8-bar phrase with no repeats — technically a "theme", but nothing in it
/// actually recurred often enough to stick. A catchy tune is a *short*
/// phrase repeated until it's memorable: here that's a 2-bar call-and-answer
/// riff, dotted martial rhythm, plus a genuine "oom-pah" accompaniment
/// (bass drum on the beat, a chord stab on the offbeat) for the walking
/// groove a bare bass-drum-and-snare pattern didn't have. A syncopated
/// snare push and crescendo roll fills keep each pass from feeling like a
/// straight loop.
///
/// Levels run long, so the loop needs to survive more than a couple of
/// repeats: it's a full A-B-A' march form now, not just a straight repeat.
/// Section A plays the riff plain, twice through (4 bars each way — call,
/// answer, call, answer); a 4-bar bridge then drops to half-time and swaps
/// in a legato, minor-tinged countermelody (E-D-B-A-G, the relative minor
/// of the home G major) for the "quiet before the band comes back"; A'
/// then restates the same riff — never changed, since changing the hook is
/// how you lose it — twice through again, now with a harmony layer under
/// it for the fuller "whole band" sound. Crescendo rolls mark all three
/// section joins. An original composition, not a transcription of any real
/// bugle call or march.
fn music() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let bpm = 116.0_f32;
    let bars = 20;
    let steps_per_bar = 16;
    let total_steps = bars * steps_per_bar;
    let step_ms = 60_000.0 / bpm / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * total_steps + SAMPLE_RATE as usize / 3;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0x0B16_10E);

    // The hook: a 2-bar call-and-answer riff, dotted rhythm (quick notes on
    // the "and"s, not just on the beat — the martial "snap"), built only
    // from a G major triad. (step, freq) pairs within the bar.
    const CALL: [(usize, f32); 6] = [
        (0, 392.00), (3, 493.88), (6, 587.33), (8, 783.99), (11, 587.33), (14, 493.88),
    ]; // G4 B4 D5 G5 D5 B4 — the call, climbing
    const ANSWER: [(usize, f32); 6] = [
        (0, 587.33), (3, 493.88), (6, 392.00), (8, 293.66), (11, 392.00), (14, 493.88),
    ]; // D5 B4 G4 D4 G4 B4 — the answer, resolving down then lifting back into the repeat
    const HARMONY_RATIO: f32 = 0.6674; // a perfect fifth below (2^(-7/12))
    const PAH_HZ: f32 = 246.94;        // B3 — the offbeat "pah" chord stab
    const PEDAL_HZ: f32 = 98.00;       // G2 — soft sustained low drone, glues the loop together

    // The bridge: a slow, legato countermelody built on the relative minor
    // (E-G-B, the vi of G major) instead of the call/answer's I chord — a
    // real change of scenery, not just a quieter repeat of the hook.
    const BRIDGE: [(usize, f32); 5] = [
        (0, 329.63), (4, 293.66), (8, 246.94), (10, 220.00), (12, 196.00),
    ]; // E4 D4 B3 A3 G3, descending
    const BRIDGE_START: usize = 8; // A section is bars 0..8
    const BRIDGE_END: usize = 12;  // A' section is bars 12..bars

    for bar in 0..bars {
        let bar_off = (bar * steps_per_bar) * step_samples;
        brass_stab(&mut buf, bar_off, PEDAL_HZ, step_ms * steps_per_bar as f32 * 1.05, 0.09);

        if bar >= BRIDGE_START && bar < BRIDGE_END {
            // Half-time groove: just the downbeat and a couple of quiet
            // taps, no "pah" stab — pulls the energy back before A'
            // brings the full band back in.
            march_bass_drum(&mut buf, bar_off, 0.5);
            for &pos in &[4, 12] {
                march_snare(&mut buf, bar_off + step_samples * pos, &mut rng, 0.15);
            }
            if bar == BRIDGE_END - 1 {
                for h in 0..6 {
                    let off = bar_off + step_samples * 10 + (step_samples / 2) * h;
                    march_snare(&mut buf, off, &mut rng, 0.20 + 0.09 * h as f32);
                }
            }
            for &(step, freq) in &BRIDGE {
                brass_stab(&mut buf, bar_off + step_samples * step, freq, step_ms * 5.0, 0.24);
            }
            continue;
        }

        let full_band = bar >= BRIDGE_END; // A' section: the harmony layer joins in
        let fill_bar = bar == BRIDGE_START - 1 || bar == bars - 1;

        // Oom-pah: bass drum on 1 and 3 ("oom"), a short brass chord stab
        // answering on 2 and 4 ("pah") — the walking groove an EDM-style
        // kick/bass pair gives the other games' tracks, done the marching-
        // band way instead.
        march_bass_drum(&mut buf, bar_off, 0.8);
        march_bass_drum(&mut buf, bar_off + step_samples * 8, 0.8);
        if !fill_bar {
            brass_stab(&mut buf, bar_off + step_samples * 4,  PAH_HZ, step_ms * 2.6, 0.22);
            brass_stab(&mut buf, bar_off + step_samples * 12, PAH_HZ, step_ms * 2.6, 0.22);
        }

        // Snare backbeat on 2 and 4 (coincides with the "pah"), a syncopated
        // push on the "and" of 2 for a bit of swagger, and soft footfall
        // taps on the remaining off-beats; the last bar of each section
        // breaks into a crescendo roll instead — the classic march fill
        // leading into the next section.
        if fill_bar {
            for h in 0..6 {
                let off = bar_off + step_samples * 10 + (step_samples / 2) * h;
                march_snare(&mut buf, off, &mut rng, 0.30 + 0.09 * h as f32);
            }
        } else {
            march_snare(&mut buf, bar_off + step_samples * 4,  &mut rng, 0.55);
            march_snare(&mut buf, bar_off + step_samples * 7,  &mut rng, 0.24); // syncopated push
            march_snare(&mut buf, bar_off + step_samples * 12, &mut rng, 0.55);
        }
        for &pos in &[2, 10, 14] {
            march_snare(&mut buf, bar_off + step_samples * pos, &mut rng, 0.14);
        }

        // The hook — call in even bars, answer in odd bars, so the 2-bar
        // phrase repeats four times over each of the A and A' sections.
        let phrase = if bar % 2 == 0 { &CALL } else { &ANSWER };
        for &(step, freq) in phrase {
            let off = bar_off + step_samples * step;
            brass_stab(&mut buf, off, freq, step_ms * 3.2, 0.42);
            if full_band {
                brass_stab(&mut buf, off, freq * HARMONY_RATIO, step_ms * 3.2, 0.26);
            }
        }
    }

    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// A second, tighter march for the loop rotation — same instrument
/// palette as `music()` (march bass drum, march snare, brass stabs) so it
/// still reads as Raider's theme, but a different key, tempo, and riff so
/// the two don't blur into one loop over a long level. Minor-key (D minor)
/// and a notch faster, no bridge — a straight-ahead A/A' oom-pah march that
/// answers the first track's more developed form with a punchier, more
/// urgent one; the harmony layer still joins for the back half.
fn music2() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let bpm = 124.0_f32;
    let bars = 16;
    let steps_per_bar = 16;
    let total_steps = bars * steps_per_bar;
    let step_ms = 60_000.0 / bpm / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * total_steps + SAMPLE_RATE as usize / 3;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0x0B16_20E);

    // The hook, in D minor this time — same dotted call-and-answer shape as
    // the main theme's riff, transposed and re-contoured, not just pitched
    // up, so it reads as its own tune rather than a key change of the first.
    const CALL: [(usize, f32); 6] = [
        (0, 293.66), (3, 349.23), (6, 440.00), (8, 587.33), (11, 440.00), (14, 349.23),
    ]; // D4 F4 A4 D5 A4 F4 — the call, climbing
    const ANSWER: [(usize, f32); 6] = [
        (0, 440.00), (3, 349.23), (6, 293.66), (8, 220.00), (11, 293.66), (14, 349.23),
    ]; // A4 F4 D4 A3 D4 F4 — the answer, resolving down then lifting back into the repeat
    const HARMONY_RATIO: f32 = 0.6674; // a perfect fifth below (2^(-7/12))
    const PAH_HZ: f32 = 174.61;        // F3 — the offbeat "pah" chord stab
    const PEDAL_HZ: f32 = 73.42;       // D2 — soft sustained low drone, glues the loop together

    for bar in 0..bars {
        let bar_off = (bar * steps_per_bar) * step_samples;
        let full_band = bar >= bars / 2; // second half: the harmony layer joins in
        let fill_bar = bar == bars / 2 - 1 || bar == bars - 1;

        march_bass_drum(&mut buf, bar_off, 0.8);
        march_bass_drum(&mut buf, bar_off + step_samples * 8, 0.8);
        if !fill_bar {
            brass_stab(&mut buf, bar_off + step_samples * 4,  PAH_HZ, step_ms * 2.6, 0.22);
            brass_stab(&mut buf, bar_off + step_samples * 12, PAH_HZ, step_ms * 2.6, 0.22);
        }

        if fill_bar {
            for h in 0..6 {
                let off = bar_off + step_samples * 10 + (step_samples / 2) * h;
                march_snare(&mut buf, off, &mut rng, 0.30 + 0.09 * h as f32);
            }
        } else {
            march_snare(&mut buf, bar_off + step_samples * 4,  &mut rng, 0.55);
            march_snare(&mut buf, bar_off + step_samples * 7,  &mut rng, 0.24); // syncopated push
            march_snare(&mut buf, bar_off + step_samples * 12, &mut rng, 0.55);
        }
        for &pos in &[2, 10, 14] {
            march_snare(&mut buf, bar_off + step_samples * pos, &mut rng, 0.14);
        }

        let phrase = if bar % 2 == 0 { &CALL } else { &ANSWER };
        for &(step, freq) in phrase {
            let off = bar_off + step_samples * step;
            brass_stab(&mut buf, off, freq, step_ms * 3.2, 0.42);
            if full_band {
                brass_stab(&mut buf, off, freq * HARMONY_RATIO, step_ms * 3.2, 0.26);
            }
        }
        brass_stab(&mut buf, bar_off, PEDAL_HZ, step_ms * steps_per_bar as f32 * 1.05, 0.09);
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
