//! Bouncer (Breakout) assets.
//!
//! Direct port of `games/bouncer/assets/generate_assets.c`.

use std::f32::consts::PI;

use crate::image::Image;
use crate::techno::{
    bass_note, clap, hat, kick, lead_stab, open_hat, riser, sidechain_duck, supersaw, Rng,
    MIX_KNEE,
};
use crate::wav::{encode_pcm16_mono, env, mix_into_f32, soft_limit_to_pcm16, SAMPLE_RATE};
use crate::Asset;

fn paddle() -> Vec<u8> {
    let w: i32 = 120;
    let h: i32 = 20;
    let mut img = Image::new(w as u32, h as u32);
    for y in 0..h {
        for x in 0..w {
            let mut in_corner = false;
            if (x < 2 || x >= w - 2) && (y < 2 || y >= h - 2) { in_corner = true; }
            if x < 1 || x >= w - 1 { in_corner = true; }
            if in_corner { continue; }
            let t = y as f32 / h as f32;
            let r = 50u8;
            let g = (100.0 + 100.0 * (1.0 - t)) as u8;
            let b = (200.0 + 55.0 * (1.0 - t)) as u8;
            img.set(x, y, r, g, b);
        }
    }
    for x in 4..w - 4 {
        img.set(x, 2, 150, 220, 255);
    }
    img.encode_png()
}

fn ball() -> Vec<u8> {
    let w: i32 = 16;
    let h: i32 = 16;
    let mut img = Image::new(w as u32, h as u32);
    let cx = w / 2;
    let cy = h / 2;
    let r = w as f32 / 2.0 - 1.0;
    for y in 0..h {
        for x in 0..w {
            let dx = (x - cx) as f32 + 0.5;
            let dy = (y - cy) as f32 + 0.5;
            if dx * dx + dy * dy < r * r {
                let shade = (1.0 - (dx * 0.2 + dy * 0.2) / r).clamp(0.5, 1.0);
                let c = (200.0 + 55.0 * shade) as u8;
                img.set(x, y, c, c, c);
            }
        }
    }
    img.set(cx - 2, cy - 2, 255, 255, 255);
    img.set(cx - 1, cy - 2, 255, 255, 255);
    img.set(cx - 2, cy - 1, 255, 255, 255);
    img.encode_png()
}

fn brick(color: (u8, u8, u8)) -> Vec<u8> {
    let w: i32 = 72;
    let h: i32 = 22;
    let mut img = Image::new(w as u32, h as u32);
    let (br, bg, bb) = color;
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let mut shade = 1.0_f32;
            if y < 3 { shade = 1.3; }
            if y > h - 4 { shade = 0.6; }
            if x < 2 { shade *= 1.2; }
            if x > w - 3 { shade *= 0.7; }
            let r = ((br as f32 * shade).min(255.0)) as u8;
            let g = ((bg as f32 * shade).min(255.0)) as u8;
            let b = ((bb as f32 * shade).min(255.0)) as u8;
            img.set(x, y, r, g, b);
        }
    }
    for x in 0..w {
        img.set(x, 0, 20, 20, 20);
        img.set(x, h - 1, 20, 20, 20);
    }
    for y in 0..h {
        img.set(0, y, 20, 20, 20);
        img.set(w - 1, y, 20, 20, 20);
    }
    img.encode_png()
}

/// Steel-plated brick: takes two hits to break. A cool, riveted metal tone
/// keeps it visually distinct from the six single-hit color rows, and the
/// `cracked` variant (shown after the first hit) darkens it and adds a
/// jagged fracture so the damage — and the fact one more hit will do it — is
/// obvious at a glance.
fn brick_steel(cracked: bool) -> Vec<u8> {
    let w: i32 = 72;
    let h: i32 = 22;
    let mut img = Image::new(w as u32, h as u32);
    let base: (u8, u8, u8) = if cracked { (108, 122, 136) } else { (150, 170, 190) };
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let mut shade = 1.0_f32;
            if y < 3 { shade = 1.3; }
            if y > h - 4 { shade = 0.6; }
            if x < 2 { shade *= 1.2; }
            if x > w - 3 { shade *= 0.7; }
            let r = ((base.0 as f32 * shade).min(255.0)) as u8;
            let g = ((base.1 as f32 * shade).min(255.0)) as u8;
            let b = ((base.2 as f32 * shade).min(255.0)) as u8;
            img.set(x, y, r, g, b);
        }
    }
    // corner rivets sell the "reinforced plate" look
    for &(rx, ry) in &[(6, 6), (w - 7, 6), (6, h - 7), (w - 7, h - 7)] {
        img.set(rx, ry, 55, 60, 65);
        img.set(rx + 1, ry, 225, 230, 235);
    }
    if cracked {
        let mut x = 5;
        let mut toggle = 0i32;
        while x < w - 5 {
            let y = (h / 2 + toggle) as i32;
            img.set(x, y, 18, 18, 22);
            img.set(x, (y + 1).min(h - 2), 18, 18, 22);
            toggle = if toggle <= 0 { 3 } else { -3 };
            x += 4;
        }
    }
    for x in 0..w {
        img.set(x, 0, 15, 15, 18);
        img.set(x, h - 1, 15, 15, 18);
    }
    for y in 0..h {
        img.set(0, y, 15, 15, 18);
        img.set(w - 1, y, 15, 15, 18);
    }
    img.encode_png()
}

/// Bouncy plucked lead voice — short, springy, and a little detuned for
/// character. Used for the melodic "bounce" hook over the tech-house groove.
fn pluck(buf: &mut [f32], off: usize, freq: f32, ms: f32, vol: f32) {
    let sr = SAMPLE_RATE as f32;
    let n = (sr * ms / 1000.0) as usize;
    let att = (sr * 0.002) as usize;
    let rel = (n * 3 / 4).max(1);
    for i in 0..n {
        if off + i >= buf.len() { break; }
        let t = i as f32 / sr;
        let e = env(i, n, att.max(1), rel);
        let w = (2.0 * PI * freq * t).sin()
            + 0.5 * (2.0 * PI * freq * 2.003 * t).sin()
            + 0.2 * (2.0 * PI * freq * 4.0 * t).sin();
        mix_into_f32(buf, off + i, w * e * vol * 12000.0);
    }
}

const BPM: f32 = 126.0;
const STEPS_PER_BAR: usize = 16;
const BARS: usize = 16;
const LIFT_BAR: usize = BARS / 2;
const TOTAL_STEPS: usize = BARS * STEPS_PER_BAR;

/// A bouncy tech-house banger: four-on-the-floor kick, backbeat claps,
/// off-beat hats, a springy staccato bassline, and one catchy pluck hook
/// repeated every bar over a simple C-F vamp — the same riff throughout is
/// what makes it stick, not a new melody every couple of bars. The back
/// half (~every 30s) adds an octave-up pluck harmony and an extra open hat
/// for a small lift.
fn music() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let step_ms = 60_000.0 / BPM / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * TOTAL_STEPS + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0xB0DE_1234);
    let mut kick_offsets = Vec::with_capacity(TOTAL_STEPS / 4);

    // C - F, a two-chord vamp, one root every 2 bars.
    let bass_roots = [130.81_f32, 174.61]; // C3, F3
    const BASS_HIT: [bool; STEPS_PER_BAR] = [
        true, false, false, true, false, true, false, false,
        true, false, false, true, false, true, false, true,
    ];
    // The hook: a bright three-note pluck riff, identical every bar.
    const HOOK: [f32; 3] = [523.25, 659.25, 783.99]; // C5 E5 G5

    for step in 0..TOTAL_STEPS {
        let bar = step / STEPS_PER_BAR;
        let pos = step % STEPS_PER_BAR;
        let off = step * step_samples;
        let lifted = bar >= LIFT_BAR;

        if pos % 4 == 0 {
            kick_offsets.push(off);
        }
        if pos == 4 || pos == 12 {
            clap(&mut buf, off, &mut rng, 0.55);
        }
        if pos % 2 == 1 {
            hat(&mut buf, off, &mut rng, 0.24);
        }
        if (bar % 2 == 1 && pos == 14) || (lifted && pos == 6) {
            open_hat(&mut buf, off, &mut rng, 0.20);
        }
        if BASS_HIT[pos] {
            let root = bass_roots[(bar / 2) % bass_roots.len()];
            bass_note(&mut buf, off, root, step_ms * 0.7, 0.58);
        }
        // Pluck hook: three quick notes starting on the "and" of beat 3, every bar.
        if pos == 10 || pos == 11 || pos == 12 {
            let note = HOOK[(pos - 10) as usize];
            pluck(&mut buf, off, note, step_ms * 1.4, 0.22);
            if lifted {
                pluck(&mut buf, off, note * 2.0, step_ms * 1.4, 0.12);
            }
        }
    }

    sidechain_duck(&mut buf, &kick_offsets, 0.55, step_ms * 0.85);
    for &off in &kick_offsets {
        kick(&mut buf, off, 0.92);
    }

    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// A rolling acid-house cut: faster and darker than `music()`, with a
/// 16th-note bassline (the classic "rolling" acid motion, not held chords)
/// under a bright, sparse `lead_stab` hook instead of `pluck` — a harder,
/// more insistent character than the tech-house original.
fn music2() -> Vec<u8> {
    const BPM: f32 = 132.0;
    const STEPS_PER_BAR: usize = 16;
    const BARS: usize = 16;
    const TOTAL_STEPS: usize = BARS * STEPS_PER_BAR;
    let sr = SAMPLE_RATE as f32;
    let step_ms = 60_000.0 / BPM / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * TOTAL_STEPS + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0xAC1D_7734);
    let mut kick_offsets = Vec::with_capacity(TOTAL_STEPS / 4);

    // One bar of rolling 16ths in A minor, a fifth lower every other bar
    // (A2 .. -> D2 ..) for a two-bar acid vamp. 0.0 = rest.
    const ACID_A: [f32; STEPS_PER_BAR] = [
        110.00, 0.0, 110.00, 130.81, 0.0, 110.00, 0.0, 146.83,
        110.00, 0.0, 110.00, 98.00,  0.0, 98.00,  116.54, 0.0,
    ];
    const ACID_D: [f32; STEPS_PER_BAR] = [
        73.42, 0.0, 73.42, 87.31, 0.0, 73.42, 0.0, 98.00,
        73.42, 0.0, 73.42, 65.41, 0.0, 65.41, 77.78, 0.0,
    ];
    const HOOK: [f32; 3] = [880.00, 1046.50, 987.77]; // A5 C6 B5

    for step in 0..TOTAL_STEPS {
        let bar = step / STEPS_PER_BAR;
        let pos = step % STEPS_PER_BAR;
        let off = step * step_samples;

        if pos % 4 == 0 {
            kick_offsets.push(off);
        }
        if pos == 4 || pos == 12 {
            clap(&mut buf, off, &mut rng, 0.5);
        }
        hat(&mut buf, off, &mut rng, if pos % 2 == 0 { 0.13 } else { 0.25 });
        if bar % 2 == 1 && pos == 14 {
            open_hat(&mut buf, off, &mut rng, 0.22);
        }
        let acid = if bar % 2 == 0 { &ACID_A } else { &ACID_D };
        if acid[pos] > 0.0 {
            bass_note(&mut buf, off, acid[pos], step_ms * 0.85, 0.5);
        }
        // Sparse hook: one stab every other bar, on the "and" of beat 2.
        if bar % 2 == 0 && pos == 6 {
            lead_stab(&mut buf, off, HOOK[(bar / 2) % 3], step_ms * 2.5, 0.16);
        }
    }

    sidechain_duck(&mut buf, &kick_offsets, 0.6, step_ms * 0.8);
    for &off in &kick_offsets {
        kick(&mut buf, off, 0.95);
    }

    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// A laid-back downtempo cut: half-time kick (just beats 1 and 3), long
/// sustained chords instead of a staccato bassline, and a slow, singable
/// `pluck` melody — the breather in the rotation, deliberately roomier and
/// quieter than the other tracks rather than another variation on "banger".
fn music3() -> Vec<u8> {
    const BPM: f32 = 100.0;
    const STEPS_PER_BAR: usize = 16;
    const BARS: usize = 12;
    const TOTAL_STEPS: usize = BARS * STEPS_PER_BAR;
    let sr = SAMPLE_RATE as f32;
    let step_ms = 60_000.0 / BPM / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * TOTAL_STEPS + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0xC411_2022);
    let mut kick_offsets = Vec::with_capacity(TOTAL_STEPS / 8);

    // C - Am - F - G, one chord (as a held bass note) every bar.
    let roots = [130.81_f32, 110.00, 87.31, 98.00]; // C3 A2 F2 G2
    // A slow four-bar melody, one note every other bar, repeated over the
    // four-bar progression.
    const MELODY: [f32; 4] = [523.25, 440.00, 349.23, 392.00]; // C5 A4 F4 G4

    for step in 0..TOTAL_STEPS {
        let bar = step / STEPS_PER_BAR;
        let pos = step % STEPS_PER_BAR;
        let off = step * step_samples;

        if pos == 0 || pos == 8 {
            kick_offsets.push(off);
        }
        if pos == 12 {
            clap(&mut buf, off, &mut rng, 0.32);
        }
        if pos % 4 == 2 {
            hat(&mut buf, off, &mut rng, 0.10);
        }
        if pos == 0 {
            bass_note(&mut buf, off, roots[bar % 4], step_ms * 12.0, 0.42);
        }
        if pos == 6 {
            pluck(&mut buf, off, MELODY[bar % 4], step_ms * 6.0, 0.20);
        }
        if pos == 14 && bar % 2 == 1 {
            pluck(&mut buf, off, MELODY[bar % 4] * 1.5, step_ms * 2.5, 0.12);
        }
    }

    sidechain_duck(&mut buf, &kick_offsets, 0.3, step_ms * 2.0);
    for &off in &kick_offsets {
        kick(&mut buf, off, 0.75);
    }

    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// A trance-leaning lift: faster and brighter than the others, a wide
/// `supersaw` hook instead of a plucked one, open hats on every offbeat for
/// that rushing trance hi-hat pattern, and a `riser` sweeping through the
/// last two bars into a hard cut back to the top of the loop — the one
/// track in the rotation built around an obvious "drop" moment.
fn music4() -> Vec<u8> {
    const BPM: f32 = 138.0;
    const STEPS_PER_BAR: usize = 16;
    const BARS: usize = 18;
    const TOTAL_STEPS: usize = BARS * STEPS_PER_BAR;
    let sr = SAMPLE_RATE as f32;
    let step_ms = 60_000.0 / BPM / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * TOTAL_STEPS + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0x7A2C_E015);
    let mut kick_offsets = Vec::with_capacity(TOTAL_STEPS / 4);

    // E - C, a two-chord trance vamp, one root every 2 bars.
    let bass_roots = [82.41_f32, 65.41]; // E2, C2
    const HOOK: [f32; 4] = [659.25, 783.99, 987.77, 880.00]; // E5 G5 B5 A5

    for step in 0..TOTAL_STEPS {
        let bar = step / STEPS_PER_BAR;
        let pos = step % STEPS_PER_BAR;
        let off = step * step_samples;
        let in_lift = bar >= BARS - 2;

        if pos % 4 == 0 {
            kick_offsets.push(off);
        }
        if pos == 4 || pos == 12 {
            clap(&mut buf, off, &mut rng, 0.5);
        }
        if pos % 2 == 1 {
            open_hat(&mut buf, off, &mut rng, if in_lift { 0.26 } else { 0.17 });
        }
        if pos % 4 == 0 {
            bass_note(&mut buf, off, bass_roots[(bar / 2) % 2], step_ms * 3.5, 0.55);
        }
        // Hook: four quick notes starting on beat 3, every other bar (every
        // bar during the lift, for extra energy).
        if bar % 2 == 0 || in_lift {
            if pos == 8 || pos == 9 || pos == 10 || pos == 11 {
                let note = HOOK[(pos - 8) as usize];
                supersaw(&mut buf, off, note, step_ms * 1.4, 0.15, 8.0, 0.01);
            }
        }
        if bar == BARS - 2 && pos == 0 {
            riser(&mut buf, off, step_ms * (STEPS_PER_BAR * 2) as f32, 0.3, &mut rng);
        }
    }

    sidechain_duck(&mut buf, &kick_offsets, 0.6, step_ms * 0.9);
    for &off in &kick_offsets {
        kick(&mut buf, off, 0.95);
    }

    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// A funkier cut: the bassline lands off the beat instead of on it
/// (syncopated, not four-on-the-floor-locked), backbeat claps every 2 and
/// 4, and a call-and-response `pluck` hook — one short phrase answered by
/// a second, rather than one riff repeated verbatim like `music()`'s.
fn music5() -> Vec<u8> {
    const BPM: f32 = 120.0;
    const STEPS_PER_BAR: usize = 16;
    const BARS: usize = 16;
    const TOTAL_STEPS: usize = BARS * STEPS_PER_BAR;
    let sr = SAMPLE_RATE as f32;
    let step_ms = 60_000.0 / BPM / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * TOTAL_STEPS + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0xF11C_5A11);
    let mut kick_offsets = Vec::with_capacity(TOTAL_STEPS / 4);

    // G mixolydian vamp: root off the beat, not on it.
    let root = 98.00_f32; // G2
    const BASS_HIT: [bool; 16] = [
        false, false, true, false, false, true, false, true,
        false, false, true, false, false, true, false, false,
    ];
    // Call (bar A) and response (bar B), alternating every bar.
    const CALL:     [f32; 3] = [392.00, 493.88, 587.33]; // G4 B4 D5
    const RESPONSE: [f32; 3] = [440.00, 523.25, 392.00]; // A4 C5 G4

    for step in 0..TOTAL_STEPS {
        let bar = step / STEPS_PER_BAR;
        let pos = step % STEPS_PER_BAR;
        let off = step * step_samples;

        if pos % 4 == 0 {
            kick_offsets.push(off);
        }
        if pos == 4 || pos == 12 {
            clap(&mut buf, off, &mut rng, 0.55);
        }
        if pos % 2 == 1 {
            hat(&mut buf, off, &mut rng, 0.20);
        }
        if bar % 4 == 3 && pos == 10 {
            open_hat(&mut buf, off, &mut rng, 0.2);
        }
        if BASS_HIT[pos] {
            bass_note(&mut buf, off, root * if pos == 7 || pos == 13 { 1.5 } else { 1.0 }, step_ms * 0.6, 0.55);
        }
        let phrase = if bar % 2 == 0 { &CALL } else { &RESPONSE };
        if pos == 9 || pos == 10 || pos == 11 {
            pluck(&mut buf, off, phrase[(pos - 9) as usize], step_ms * 1.3, 0.21);
        }
    }

    sidechain_duck(&mut buf, &kick_offsets, 0.5, step_ms * 0.9);
    for &off in &kick_offsets {
        kick(&mut buf, off, 0.9);
    }

    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// The moody one: a slower half-time D minor groove, sparse `lead_stab`
/// accents instead of a running melody, a heavier sidechain pump, and
/// noticeably more silence between hits — tension rather than a hook,
/// so the rotation isn't six tracks all chasing the same upbeat energy.
fn music6() -> Vec<u8> {
    const BPM: f32 = 110.0;
    const STEPS_PER_BAR: usize = 16;
    const BARS: usize = 14;
    const TOTAL_STEPS: usize = BARS * STEPS_PER_BAR;
    let sr = SAMPLE_RATE as f32;
    let step_ms = 60_000.0 / BPM / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * TOTAL_STEPS + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0xDA26_7E55);
    let mut kick_offsets = Vec::with_capacity(TOTAL_STEPS / 4);

    // D minor - Bb, a two-chord tense vamp, one root every 2 bars.
    let bass_roots = [73.42_f32, 58.27]; // D2, Bb1
    const STAB: [f32; 3] = [293.66, 349.23, 311.13]; // D4 F4 D#4 (tense, not resolving)

    for step in 0..TOTAL_STEPS {
        let bar = step / STEPS_PER_BAR;
        let pos = step % STEPS_PER_BAR;
        let off = step * step_samples;

        if pos == 0 || pos == 10 {
            kick_offsets.push(off);
        }
        if pos == 8 {
            clap(&mut buf, off, &mut rng, 0.4);
        }
        if pos % 4 == 2 {
            hat(&mut buf, off, &mut rng, 0.11);
        }
        if pos == 0 {
            bass_note(&mut buf, off, bass_roots[(bar / 2) % 2], step_ms * 6.0, 0.5);
        }
        // A stab only once every other bar — mostly space, not melody.
        if bar % 2 == 1 && pos == 6 {
            lead_stab(&mut buf, off, STAB[(bar / 2) % 3], step_ms * 3.0, 0.17);
        }
    }

    sidechain_duck(&mut buf, &kick_offsets, 0.68, step_ms * 1.4);
    for &off in &kick_offsets {
        kick(&mut buf, off, 0.88);
    }

    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

// paddle_hit and brick_hit fire on nearly every bounce — the most frequently
// repeated sounds in the game — so they stay soft and low-key: a plain
// low-order tone with a quick decay, no noise grit or bright high harmonics
// that would turn grating under rapid-fire repetition.
fn paddle_hit() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = SAMPLE_RATE as usize / 16;
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let e = (1.0 - i as f32 / n as f32).powf(1.8);
        let fund = (2.0 * PI * 170.0 * t).sin();
        let second = (2.0 * PI * 170.0 * 2.0 * t).sin() * 0.15;
        s.push((e * 12000.0 * (fund + second)) as i16);
    }
    encode_pcm16_mono(&s)
}

fn brick_hit() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = SAMPLE_RATE as usize / 22;
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let e = (1.0 - i as f32 / n as f32).powf(2.0);
        let fund = (2.0 * PI * 480.0 * t).sin();
        let second = (2.0 * PI * 480.0 * 2.0 * t).sin() * 0.2;
        s.push((e * 10000.0 * (fund + second)) as i16);
    }
    encode_pcm16_mono(&s)
}

fn brick_break() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = SAMPLE_RATE as usize / 10;
    let mut rng = Rng(0xBA11_0003);
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let e = (1.0 - i as f32 / n as f32).powf(1.3);
        let freq = 900.0 - 400.0 * i as f32 / n as f32;
        let tone = (2.0 * PI * freq * t).sin();
        let crackle = (rng.next_f32() * 2.0 - 1.0) * 0.35;
        let shaped = (tone * 0.85 + crackle).tanh();
        s.push((e * 15000.0 * shaped) as i16);
    }
    encode_pcm16_mono(&s)
}

fn life_lost() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = SAMPLE_RATE as usize / 2;
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let freq = 440.0 * (1.0 - 0.5 * i as f32 / n as f32);
        let e = (1.0 - i as f32 / n as f32).powf(1.2);
        let fund = (2.0 * PI * freq * t).sin();
        let detune = (2.0 * PI * freq * 1.008 * t).sin();
        let shaped = (fund * 0.7 + detune * 0.5).tanh();
        s.push((e * 18000.0 * shaped) as i16);
    }
    encode_pcm16_mono(&s)
}

fn win() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let freqs = [440.0_f32, 494.0, 523.0, 587.0, 659.0, 698.0, 784.0, 880.0];
    let seg = SAMPLE_RATE as usize / 6;
    let total = seg * 8;
    let mut buf = vec![0i16; total];
    for (i, f) in freqs.iter().enumerate() {
        for j in 0..seg {
            let t = j as f32 / sr;
            let quarter = seg / 4;
            let e = if j < quarter {
                j as f32 / quarter as f32
            } else {
                (seg - j) as f32 / seg as f32
            };
            buf[i * seg + j] = (e * 20000.0 * (2.0 * PI * f * t).sin()) as i16;
        }
    }
    encode_pcm16_mono(&buf)
}

pub fn generate() -> Vec<Asset> {
    vec![
        ("images/paddle.png", paddle()),
        ("images/ball.png", ball()),
        ("images/brick_red.png",    brick((220, 60, 60))),
        ("images/brick_orange.png", brick((220, 140, 40))),
        ("images/brick_yellow.png", brick((200, 200, 50))),
        ("images/brick_green.png",  brick((50,  200, 80))),
        ("images/brick_blue.png",   brick((50,  100, 220))),
        ("images/brick_purple.png", brick((160, 50,  220))),
        ("images/brick_steel.png",         brick_steel(false)),
        ("images/brick_steel_cracked.png", brick_steel(true)),
        ("sounds/paddle_hit.wav", paddle_hit()),
        ("sounds/brick_hit.wav",  brick_hit()),
        ("sounds/brick_break.wav", brick_break()),
        ("sounds/life_lost.wav",  life_lost()),
        ("sounds/win.wav",        win()),
        ("sounds/music.wav",      music()),
        ("sounds/music2.wav",     music2()),
        ("sounds/music3.wav",     music3()),
        ("sounds/music4.wav",     music4()),
        ("sounds/music5.wav",     music5()),
        ("sounds/music6.wav",     music6()),
    ]
}
