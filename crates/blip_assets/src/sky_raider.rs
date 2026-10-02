//! Raider assets: a 1942-style vertical dogfighter. Planes are rendered from
//! their real planforms; bullets and explosions are drawn by the game.

use crate::image::Image;
use std::f32::consts::PI;

use crate::wav::{low_pass, tame, warm, Rng, MIX_KNEE};
use crate::wav::{encode_pcm16_mono, encode_pcm16_music, mix_into, mix_into_f32, ms_to_samples, soft_limit_to_pcm16, SAMPLE_RATE};
use crate::Asset;

// Must match crates/sky_raider/src/main.rs's PLAYER_W / PLAYER_H.
const SR_F: f32 = SAMPLE_RATE as f32;
const PLAYER_W: i32 = 66;
const PLAYER_H: i32 = 58;
// Must match crates/sky_raider/src/main.rs's ENEMY_W / ENEMY_H.
const ENEMY_W: i32 = 62;
const ENEMY_H: i32 = 52;
// Must match crates/sky_raider/src/main.rs's BOSS_SIZES.
// (span, length), growing with the level; the last, the Fugaku, is twice
// the first.
const BOSS_SIZES: [(i32, i32); 7] = [
    (96, 76), (104, 84), (116, 92), (150, 106), (164, 122), (182, 134), (192, 152),
];
// Gun positions per boss: (x as a fraction of the half span from the
// centreline, y as a fraction of the length from the nose). The game fires
// from the same points (crates/sky_raider/src/main.rs BOSS_TURRETS).
const BOSS_TURRETS: [&[(f32, f32)]; 7] = [
    &[(0.0, 0.06), (0.0, 0.45), (0.0, 0.97)],
    &[(0.0, 0.06), (0.0, 0.50), (0.0, 0.97)],
    &[(0.0, 0.07), (0.0, 0.36), (-0.09, 0.62), (0.09, 0.62), (0.0, 0.98)],
    &[(0.0, 0.06), (0.0, 0.30), (0.0, 0.60), (0.0, 0.97)],
    &[(0.0, 0.07), (0.0, 0.36), (-0.07, 0.60), (0.07, 0.60), (0.0, 0.97)],
    &[(0.0, 0.06), (0.0, 0.28), (0.0, 0.55), (0.0, 0.97)],
    &[(0.0, 0.05), (0.0, 0.97)],
];
// Must match crates/sky_raider/src/main.rs's POW_W / POW_H.
const POW_W: i32 = 14;
const POW_H: i32 = 14;
// Must match crates/sky_raider/src/main.rs's HEALTH_W / HEALTH_H.
const HEALTH_W: i32 = 14;
const HEALTH_H: i32 = 14;
// Must match crates/sky_raider/src/main.rs's CARRIER_W / CARRIER_H.
const CARRIER_W: i32 = 186;
const CARRIER_H: i32 = 760;
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

/// Notes one after another, each overlapping the next (`step_ms < dur_ms`):
/// the power-up chimes, more notes and higher for a bigger tier.
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

/// One report from a wing gun: an overdriven crack of band-passed noise (the
/// muzzle blast), a short saturated body, the bolt cycling twice, and two
/// reflections off the airframe. `pitch` and `seed` vary the takes so
/// autofire rattles rather than loops.
fn gun_report(seed: u32, pitch: f32) -> Vec<f32> {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(120.0);
    let mut rng = Rng(seed | 1);
    let mut dry = vec![0.0f32; n];
    let (mut lo, mut bp_lo, mut bp_hi, mut phase) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    for (i, out) in dry.iter_mut().enumerate() {
        let t = i as f32 / sr;
        let w = rng.next_f32() * 2.0 - 1.0;
        // crack: noise band-passed round 1.5-5 kHz, driven hard, ~4ms
        bp_hi += (w - bp_hi) * 0.55 * pitch.min(1.2);
        bp_lo += (bp_hi - bp_lo) * 0.18;
        let crack = ((bp_hi - bp_lo) * 6.0).tanh() * (-t / 0.004).exp();
        // body: low-passed blast, ~25ms
        lo += (w - lo) * 0.09 * pitch;
        let body = (lo * 5.0).tanh() * (-t / 0.025).exp();
        // thump: 180 -> 90 Hz, saturated so a phone speaker keeps it
        phase += pitch * (90.0 + 90.0 * (-t / 0.01).exp()) / sr;
        let thump = ((2.0 * PI * phase).sin() * 4.0).tanh() * (-t / 0.02).exp();
        // bolt: two short metallic clacks
        let mut bolt = 0.0;
        for (at, amp) in [(0.022f32, 1.0f32), (0.034, 0.6)] {
            let ct = t - at / pitch;
            if ct > 0.0 { bolt += amp * (2.0 * PI * 2900.0 * pitch * ct).sin() * (-ct / 0.0018).exp(); }
        }
        *out = crack * 1.0 + body * 0.7 + thump * 0.55 + bolt * 0.12;
    }
    // Early reflections, darker than the direct sound.
    let mut buf = dry.clone();
    for (ms, g) in [(14.0f32, 0.30f32), (27.0, 0.16)] {
        let d = ms_to_samples(ms);
        let mut lp = 0.0f32;
        for i in d..n {
            lp += (dry[i - d] - lp) * 0.25;
            buf[i] += lp * g;
        }
    }
    buf
}

/// `guns` guns firing one burst: each report a few ms off the others and a
/// little off pitch, summed and then limited, so more guns come out denser
/// and louder rather than the same shot turned up.
fn gun_burst(guns: usize, seed: u32) -> Vec<i16> {
    let mut rng = Rng(seed | 1);
    let pad = ms_to_samples(16.0);
    let mut mix = vec![0.0f32; ms_to_samples(120.0) + pad];
    for g in 0..guns {
        let delay = if g == 0 { 0 } else { (rng.next_f32() * pad as f32) as usize };
        let pitch = 0.9 + rng.next_f32() * 0.2;
        let shot = gun_report(seed.wrapping_add((g as u32).wrapping_mul(0x9E37_79B9)), pitch);
        for (i, v) in shot.iter().enumerate() { mix[delay + i] += v * 16_000.0; }
    }
    soft_limit_to_pcm16(&mix, MIX_KNEE)
}

/// An enemy fighter's burst, heard from a distance: three rounds from a
/// pair of guns, lower and duller than the player's (further off, smaller
/// calibre on most of them), with the highs rolled off by the air between.
fn enemy_gun_sfx() -> Vec<u8> {
    let mut rng = Rng(0xE6_6E1);
    let n = ms_to_samples(360.0);
    let mut mix = vec![0.0f32; n];
    for k in 0..3 {
        for g in 0..2 {
            let off = ms_to_samples(k as f32 * 80.0 + g as f32 * 9.0 + rng.next_f32() * 6.0);
            let shot = gun_report(0x1234 + (k * 2 + g) as u32 * 977, 0.78 + rng.next_f32() * 0.08);
            for (i, v) in shot.iter().enumerate() {
                if off + i < n { mix[off + i] += v * 12_000.0; }
            }
        }
    }
    warm(&mut mix);
    warm(&mut mix);
    encode_pcm16_mono(&soft_limit_to_pcm16(&mix, MIX_KNEE))
}

/// A backfire from an engine starved near a stall: a sharp pop in the
/// exhaust, a low thump, and a crackle as it clears.
fn backfire_sfx() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(280.0);
    let mut rng = Rng(0xBAC_F1);
    let mut buf = vec![0.0f32; n];
    let (mut lp, mut crack) = (0.0f32, 0.0f32);
    for (i, out) in buf.iter_mut().enumerate() {
        let t = i as f32 / sr;
        let w = rng.next_f32() * 2.0 - 1.0;
        lp += (w - lp) * 0.2;
        let pop = lp * (-t / 0.012).exp();
        let thump = (2.0 * PI * (70.0 + 60.0 * (-t / 0.02).exp()) * t).sin() * (-t / 0.05).exp();
        if rng.next_f32() < 0.004 * (1.0 - t / 0.28) { crack = 1.0; }
        crack *= 0.9;
        *out = (pop * 1.6 + thump * 0.9 + lp * crack * 0.8) * 20_000.0;
    }
    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// A ricochet: a hard metallic tick where the round strikes, then the
/// classic "pyeww" of a spinning slug tearing away, a whistle falling from
/// `f0` with a wavering pitch, a little air noise round it.
fn ricochet_sfx(f0: f32, seed: u32) -> Vec<i16> {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(420.0);
    let mut rng = Rng(seed | 1);
    let mut buf = vec![0.0f32; n];
    let (mut phase, mut hp_prev) = (0.0f32, 0.0f32);
    for (i, out) in buf.iter_mut().enumerate() {
        let t = i as f32 / sr;
        let w = rng.next_f32() * 2.0 - 1.0;
        let hp = w - hp_prev;
        hp_prev = w;
        let tick = hp * (-t / 0.003).exp() + (2.0 * PI * 3100.0 * t).sin() * (-t / 0.006).exp() * 0.5;
        // whistle: falls to ~40% of f0 and wavers as the slug tumbles
        let f = f0 * (0.4 + 0.6 * (-t / 0.16).exp()) * (1.0 + 0.035 * (2.0 * PI * 23.0 * t).sin());
        phase += f / sr;
        let env = (t / 0.012).min(1.0) * (-t / 0.13).exp();
        let whistle = (2.0 * PI * phase).sin() * env;
        let air = w * env * 0.12;
        *out = (tick * 0.7 + whistle * 0.55 + air) * 20_000.0;
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// A round striking the fuselage: a sharp tick, a short metallic ring from
/// two inharmonic partials, and a dull knock underneath.
fn hit_sfx() -> Vec<i16> {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(160.0);
    let mut rng = Rng(0x417C_0DE5);
    let mut buf = vec![0.0f32; n];
    for (i, out) in buf.iter_mut().enumerate() {
        let t = i as f32 / sr;
        let w = rng.next_f32() * 2.0 - 1.0;
        let tick = w * (-t / 0.004).exp();
        let ring = ((2.0 * PI * 1180.0 * t).sin() + 0.6 * (2.0 * PI * 1730.0 * t).sin()) * (-t / 0.035).exp();
        let knock = (2.0 * PI * 180.0 * t).sin() * (-t / 0.02).exp();
        *out = (tick * 0.35 + ring * 0.35 + knock * 0.8) * 20_000.0;
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// An explosion's shape: how long, how deep, how much goes off after.
#[derive(Clone, Copy)]
struct Blast {
    secs: f32,
    /// The boom's starting pitch; it falls to a third. Lower is bigger.
    boom_hz: f32,
    /// Fuel and ammunition cooking off after the main blast.
    secondaries: usize,
    /// Metal pinging away as debris.
    debris: usize,
    amp: f32,
    seed: u32,
}

/// An explosion, built the way a real one sounds: a sharp blast front, a
/// boom falling in pitch, a fireball roar that swells and gutters rather
/// than fading smoothly, secondary pops, metal debris, and a faint echo
/// off the sea. Each seed is a different take of the same size.
fn explosion_sfx(b: Blast) -> Vec<i16> {
    let sr = SAMPLE_RATE as f32;
    let n = (b.secs * sr) as usize;
    let mut rng = Rng(b.seed | 1);
    let mut buf = vec![0.0f32; n + (0.3 * sr) as usize];
    let (mut lp1, mut lp2, mut phase, mut turb, mut turb_lp) = (0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32);
    for i in 0..n {
        let t = i as f32 / sr;
        let k = t / b.secs;
        let attack = (t / 0.002).min(1.0);
        let decay = (1.0 - k).max(0.0).powf(1.6) * (-t / (b.secs * 0.4)).exp();
        let w = rng.next_f32() * 2.0 - 1.0;
        // the fireball's roar: dark noise whose filter closes as it dies,
        // its level wandering at a few hertz the way turbulence does
        let cut = 0.22 * (1.0 - k * 0.85) + 0.015;
        lp1 += (w - lp1) * cut;
        lp2 += (lp1 - lp2) * cut;
        turb += (rng.next_f32() * 2.0 - 1.0 - turb) * 0.0015;
        turb_lp += (turb - turb_lp) * 0.002;
        let roar = lp2 * 3.4 * (0.65 + 4.0 * turb_lp.abs());
        // the boom: a saturated low sine falling from boom_hz to a third
        phase += b.boom_hz * (0.33 + 0.67 * (-t / 0.08).exp()) / sr;
        let boom = ((2.0 * PI * phase).sin() * 3.0).tanh() * (-t / (b.secs * 0.22)).exp();
        // the blast front: a hard crack of bright noise in the first ms
        let front = w * (-t / 0.0025).exp() * 1.4 + lp1 * (-t / 0.015).exp() * 1.2;
        buf[i] += (roar * decay + boom * 0.8 * decay + front) * attack * b.amp;
    }
    // secondary explosions: smaller, later, each its own crack and thump
    for _ in 0..b.secondaries {
        let at = (b.secs * (0.12 + 0.5 * rng.next_f32()) * sr) as usize;
        let size = 0.35 + 0.4 * rng.next_f32();
        let f = b.boom_hz * (1.3 + 0.6 * rng.next_f32());
        let mut lp = 0.0f32;
        for i in 0..(0.18 * sr) as usize {
            let j = at + i;
            if j >= buf.len() { break; }
            let t = i as f32 / sr;
            let w = rng.next_f32() * 2.0 - 1.0;
            lp += (w - lp) * 0.25;
            buf[j] += size * b.amp * ((2.0 * PI * f * t).sin() * (-t / 0.05).exp() + lp * 1.5 * (-t / 0.03).exp());
        }
    }
    // debris: short metal clinks scattering away, two inharmonic partials
    // each so they read as torn panels rather than tones
    for _ in 0..b.debris {
        let at = (b.secs * (0.08 + 0.6 * rng.next_f32()) * sr) as usize;
        let f = 1200.0 + 2200.0 * rng.next_f32();
        let ring = 0.004 + 0.008 * rng.next_f32();
        let vol = 0.10 + 0.10 * rng.next_f32();
        for i in 0..(ring * 6.0 * sr) as usize {
            let j = at + i;
            if j >= buf.len() { break; }
            let t = i as f32 / sr;
            let tone = (2.0 * PI * f * t).sin() + 0.6 * (2.0 * PI * f * 2.37 * t).sin();
            buf[j] += tone * (-t / ring).exp() * vol * b.amp;
        }
    }
    // the sea and the sky give a little back: two soft, darker echoes
    let dry = buf.clone();
    for &(delay, gain) in &[(0.11f32, 0.28f32), (0.24, 0.16)] {
        let d = (delay * sr) as usize;
        let mut lp = 0.0f32;
        for i in d..buf.len() {
            lp += (dry[i - d] - lp) * 0.15;
            buf[i] += lp * gain;
        }
    }
    let end = buf.iter().rposition(|v| v.abs() > 1e-4).map_or(1, |i| i + 1);
    buf.truncate(end);
    let fade = (0.02 * sr) as usize;
    let len = buf.len();
    for i in 0..fade.min(len) { buf[len - 1 - i] *= i as f32 / fade as f32; }
    let scaled: Vec<f32> = buf.iter().map(|v| v * 22_000.0).collect();
    soft_limit_to_pcm16(&scaled, MIX_KNEE)
}

/// Takes of each explosion size: enemy planes and boats, the player, bosses.
const ENEMY_BLASTS: [Blast; 4] = [
    Blast { secs: 0.65, boom_hz: 100.0, secondaries: 1, debris: 3, amp: 0.8, seed: 0xE1E1 },
    Blast { secs: 0.75, boom_hz: 88.0, secondaries: 2, debris: 2, amp: 0.8, seed: 0xE2E2 },
    Blast { secs: 0.55, boom_hz: 112.0, secondaries: 0, debris: 4, amp: 0.75, seed: 0xE3E3 },
    Blast { secs: 0.8, boom_hz: 92.0, secondaries: 1, debris: 1, amp: 0.85, seed: 0xE4E4 },
];
const PLAYER_BLASTS: [Blast; 2] = [
    Blast { secs: 1.2, boom_hz: 70.0, secondaries: 3, debris: 5, amp: 1.0, seed: 0x9A7E },
    Blast { secs: 1.3, boom_hz: 64.0, secondaries: 2, debris: 4, amp: 1.0, seed: 0x9B7F },
];
const BOSS_BLASTS: [Blast; 2] = [
    Blast { secs: 2.0, boom_hz: 55.0, secondaries: 6, debris: 8, amp: 1.0, seed: 0xB055 },
    Blast { secs: 2.2, boom_hz: 50.0, secondaries: 7, debris: 6, amp: 1.0, seed: 0xB156 },
];

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

// ---- planform renderer -------------------------------------------------
// Planes drawn from their real planforms (three-views of the P-51D, A6M Zero,
// Ki-43, Ki-84 and the bombers), rasterised 4x supersampled. Coordinates are
// sprite pixels, nose at y = 0, pointing up.

const SS: usize = 4;

/// `k` is design units to pixels (a design drawn at 1 can be rendered at 2
/// for twice the size and twice the detail); `clip`, in design units, is
/// the only area a fill scans, which keeps a big sprite quick to build.
struct Canvas { w: usize, h: usize, k: f32, clip: (f32, f32, f32, f32), buf: Vec<[f32; 4]> }

impl Canvas {
    fn new(w: i32, h: i32) -> Self { Self::scaled(w, h, 1.0) }
    fn scaled(w: i32, h: i32, k: f32) -> Self {
        let (w, h) = (w as usize, h as usize);
        Self { w, h, k, clip: (0.0, 0.0, 1e9, 1e9), buf: vec![[0.0; 4]; w * h * SS * SS] }
    }
    fn clip(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) { self.clip = (x0, y0, x1, y1); }
    fn unclip(&mut self) { self.clip = (0.0, 0.0, 1e9, 1e9); }
    /// Paint `col` (straight RGBA, 0..1) wherever `inside(x, y)` holds,
    /// with `shade(x, y)` scaling its RGB.
    fn fill(&mut self, inside: impl Fn(f32, f32) -> bool, shade: impl Fn(f32, f32) -> f32, col: [f32; 4]) {
        let sw = self.w * SS;
        let unit = self.k * SS as f32;
        let (cx0, cy0, cx1, cy1) = self.clip;
        let sx0 = ((cx0 * unit).floor().max(0.0) as usize).min(sw);
        let sx1 = ((cx1 * unit).ceil().max(0.0) as usize).min(sw);
        let sy0 = ((cy0 * unit).floor().max(0.0) as usize).min(self.h * SS);
        let sy1 = ((cy1 * unit).ceil().max(0.0) as usize).min(self.h * SS);
        for sy in sy0..sy1 {
            for sx in sx0..sx1 {
                let (x, y) = ((sx as f32 + 0.5) / unit, (sy as f32 + 0.5) / unit);
                if !inside(x, y) { continue; }
                let k = shade(x, y);
                let d = &mut self.buf[sy * sw + sx];
                let a = col[3];
                for c in 0..3 { d[c] = d[c] * (1.0 - a) + (col[c] * k).min(1.0) * a; }
                d[3] = d[3] + a * (1.0 - d[3]);
            }
        }
    }
    fn solid(&mut self, inside: impl Fn(f32, f32) -> bool, col: [f32; 4]) {
        self.fill(inside, |_, _| 1.0, col);
    }
    /// Box-filter down to the sprite, add a soft dark outline so the
    /// silhouette holds against the sea, optionally flipped nose-down.
    fn finish(&self, flip: bool) -> Vec<u8> {
        let (w, h) = (self.w, self.h);
        let sw = w * SS;
        let mut px = vec![[0.0f32; 4]; w * h];
        for y in 0..h {
            for x in 0..w {
                let mut acc = [0.0f32; 4];
                for j in 0..SS {
                    for i in 0..SS {
                        let d = self.buf[(y * SS + j) * sw + x * SS + i];
                        for c in 0..3 { acc[c] += d[c] * d[3]; }
                        acc[3] += d[3];
                    }
                }
                let a = acc[3] / (SS * SS) as f32;
                px[y * w + x] = if acc[3] > 0.0 {
                    [acc[0] / acc[3], acc[1] / acc[3], acc[2] / acc[3], a]
                } else { [0.0; 4] };
            }
        }
        let mut img = Image::new(w as u32, h as u32);
        for y in 0..h {
            for x in 0..w {
                let p = px[y * w + x];
                let mut near = 0.0f32;
                for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < h {
                        near = near.max(px[ny as usize * w + nx as usize][3]);
                    }
                }
                let outline = (near - p[3]).max(0.0) * 0.55;
                let a = p[3] + outline * (1.0 - p[3]);
                let rgb = if a > 0.0 { [p[0] * p[3] / a, p[1] * p[3] / a, p[2] * p[3] / a] } else { [0.0; 3] };
                let oy = if flip { h - 1 - y } else { y } as i32;
                img.set_rgba(x as i32, oy, (rgb[0] * 255.0) as u8, (rgb[1] * 255.0) as u8,
                    (rgb[2] * 255.0) as u8, (a * 255.0).round() as u8);
            }
        }
        img.encode_png()
    }
}

fn rgb(c: (u8, u8, u8)) -> [f32; 4] { [c.0 as f32 / 255.0, c.1 as f32 / 255.0, c.2 as f32 / 255.0, 1.0] }
fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }

/// A lifting surface either side of the centreline `cx`: leading / trailing
/// edge at the root and at the tip, and how much of the outer span is
/// rounded off (0 = square tip, ~0.35 = the Japanese elliptical tips).
#[derive(Clone, Copy)]
struct Surface { half_span: f32, root: (f32, f32), tip: (f32, f32), round: f32 }

impl Surface {
    fn contains(&self, cx: f32, x: f32, y: f32) -> bool {
        let t = (x - cx).abs() / self.half_span;
        if t > 1.0 { return false; }
        let (mut le, mut te) = (lerp(self.root.0, self.tip.0, t), lerp(self.root.1, self.tip.1, t));
        if self.round > 0.0 && t > 1.0 - self.round {
            let u = (t - (1.0 - self.round)) / self.round;
            let (mid, half) = ((le + te) / 2.0, (te - le) / 2.0 * (1.0 - u * u).max(0.0).sqrt());
            le = mid - half;
            te = mid + half;
        }
        y >= le && y <= te
    }
    /// Lit leading edge, darker toward the trailing edge and the tip.
    fn shade(&self, cx: f32, x: f32, y: f32) -> f32 {
        let t = (x - cx).abs() / self.half_span;
        let (le, te) = (lerp(self.root.0, self.tip.0, t), lerp(self.root.1, self.tip.1, t));
        let c = ((y - le) / (te - le).max(0.1)).clamp(0.0, 1.0);
        1.12 - 0.22 * c - 0.08 * t
    }
}

/// One fighter seen from above, nose up.
struct Fighter {
    w: i32,
    /// fuselage: half width at its widest, where that is, and the tail's
    body_half: f32, widest: f32, tail_half: f32, tail_y: f32,
    /// radial cowl (round, blunt) or inline nose (pointed), half width, length
    radial: bool, nose_half: f32, nose_len: f32,
    wing: Surface, tailplane: Surface,
    /// canopy centre y, half length, half width
    canopy: (f32, f32, f32),
    body: (u8, u8, u8), wing_col: (u8, u8, u8), nose_col: (u8, u8, u8), spinner: (u8, u8, u8),
    /// gun ports along the leading edge, as fractions of the half span
    guns: &'static [f32],
}

impl Fighter {
    fn fuselage_half(&self, y: f32) -> f32 {
        if y < 0.0 || y > self.tail_y { return -1.0; }
        if y < self.widest {
            let s = y / self.widest;
            if self.radial {
                // a blunt round cowl that is nearly full width at once
                lerp(self.nose_half, self.body_half, s) * (1.0 - (1.0 - (s * 4.0).min(1.0)).powi(2) * 0.55)
            } else {
                lerp(0.9, self.body_half, s.sqrt())
            }
        } else {
            lerp(self.body_half, self.tail_half, (y - self.widest) / (self.tail_y - self.widest))
        }
    }

    fn paint(&self, c: &mut Canvas) {
        let cx = self.w as f32 / 2.0;
        // propeller disc: a faint blur ahead of the nose
        c.solid(|x, y| ((x - cx) / (self.wing.half_span * 0.36)).powi(2) + ((y - 1.0) / 0.9).powi(2) <= 1.0,
            [0.75, 0.75, 0.72, 0.28]);
        let (wing, tp) = (self.wing, self.tailplane);
        c.fill(|x, y| tp.contains(cx, x, y), |x, y| tp.shade(cx, x, y), rgb(self.wing_col));
        c.fill(|x, y| wing.contains(cx, x, y), |x, y| wing.shade(cx, x, y), rgb(self.wing_col));
        // control-surface hinge line along the trailing edge
        c.solid(|x, y| {
            let t = (x - cx).abs() / wing.half_span;
            let te = lerp(wing.root.1, wing.tip.1, t) - (lerp(wing.root.1, wing.tip.1, t) - lerp(wing.root.0, wing.tip.0, t)) * 0.22;
            t > 0.18 && t < 0.9 && (y - te).abs() < 0.28 && wing.contains(cx, x, y)
        }, [0.0, 0.0, 0.0, 0.22]);
        // fuselage, shaded as a cylinder lit from above
        c.fill(|x, y| (x - cx).abs() <= self.fuselage_half(y), |x, y| {
            let hw = self.fuselage_half(y).max(0.3);
            let u = ((x - cx) / hw).clamp(-1.0, 1.0);
            0.72 + 0.42 * (1.0 - u * u).sqrt()
        }, rgb(self.body));
        // nose / cowl colour over the front of the fuselage
        c.fill(|x, y| y < self.nose_len && (x - cx).abs() <= self.fuselage_half(y), |x, y| {
            let hw = self.fuselage_half(y).max(0.3);
            let u = ((x - cx) / hw).clamp(-1.0, 1.0);
            0.7 + 0.45 * (1.0 - u * u).sqrt()
        }, rgb(self.nose_col));
        // spinner
        // spinner: small in the middle of a radial's cowl ring, long on an inline nose
        let (sw, sl) = if self.radial { (0.75, 0.9) } else { (1.1, 1.5) };
        c.solid(|x, y| ((x - cx) / sw).powi(2) + ((y - sl * 0.7) / sl).powi(2) <= 1.0, rgb(self.spinner));
        // canopy glass with a highlight
        let (cy, cl, cw) = self.canopy;
        c.solid(|x, y| ((x - cx) / cw).powi(2) + ((y - cy) / cl).powi(2) <= 1.0, [0.16, 0.24, 0.32, 1.0]);
        c.solid(|x, y| ((x - cx + cw * 0.3) / (cw * 0.35)).powi(2) + ((y - cy + cl * 0.25) / (cl * 0.45)).powi(2) <= 1.0,
            [0.75, 0.88, 1.0, 0.85]);
        self.details(c);
    }

    /// The small things that make it read as a machine: panel lines,
    /// ailerons and elevators, gun ports, exhausts and their soot, the
    /// canopy frame, the fin seen edge-on.
    fn details(&self, c: &mut Canvas) {
        let cx = self.w as f32 / 2.0;
        let (wing, tp) = (self.wing, self.tailplane);
        let line = [0.0, 0.0, 0.0, 0.2];
        // chordwise panel lines and the main spar
        for t in [0.34f32, 0.62, 0.82] {
            c.solid(|x, y| ((x - cx).abs() / wing.half_span - t).abs() * wing.half_span < 0.13
                && wing.contains(cx, x, y), line);
        }
        c.solid(|x, y| {
            let t = (x - cx).abs() / wing.half_span;
            let (le, te) = (lerp(wing.root.0, wing.tip.0, t), lerp(wing.root.1, wing.tip.1, t));
            (y - (le + (te - le) * 0.3)).abs() < 0.12 && t > 0.12 && wing.contains(cx, x, y)
        }, [0.0, 0.0, 0.0, 0.12]);
        // elevator hinge
        c.solid(|x, y| {
            let t = (x - cx).abs() / tp.half_span;
            let (le, te) = (lerp(tp.root.0, tp.tip.0, t), lerp(tp.root.1, tp.tip.1, t));
            (y - (te - (te - le) * 0.35)).abs() < 0.14 && t > 0.1 && tp.contains(cx, x, y)
        }, [0.0, 0.0, 0.0, 0.25]);
        // the fin, edge-on down the tail
        let (fy0, fy1) = (tp.root.0 - 1.0, self.tail_y);
        c.solid(|x, y| y > fy0 && y < fy1 && (x - cx).abs() < 0.35, [0.1, 0.1, 0.1, 0.45]);
        // fuselage panel bands
        for yb in [self.widest + 0.5, self.widest + (self.tail_y - self.widest) * 0.5] {
            c.solid(|x, y| (y - yb).abs() < 0.12 && (x - cx).abs() <= self.fuselage_half(y), line);
        }
        // gun ports: dark muzzles just behind the leading edge
        for &t in self.guns {
            for sgn in [-1.0f32, 1.0] {
                let gx = cx + sgn * t * wing.half_span;
                let le = lerp(wing.root.0, wing.tip.0, t);
                c.solid(|x, y| ((x - gx) / 0.3).powi(2) + ((y - le - 0.3) / 0.45).powi(2) <= 1.0, [0.05, 0.05, 0.05, 0.9]);
            }
        }
        // exhausts: stubs down the nose of an inline, a soot streak behind a radial's cowl
        if self.radial {
            for sgn in [-1.0f32, 1.0] {
                c.solid(|x, y| y > self.nose_len && y < self.nose_len + 3.5
                    && ((x - cx) * sgn - self.fuselage_half(y) * 0.72).abs() < 0.35, [0.08, 0.07, 0.06, 0.5]);
            }
        } else {
            for k in 0..4 {
                let ey = 3.2 + k as f32 * 0.9;
                for sgn in [-1.0f32, 1.0] {
                    let ex = cx + sgn * (self.fuselage_half(ey) + 0.1);
                    c.solid(|x, y| ((x - ex) / 0.35).powi(2) + ((y - ey) / 0.3).powi(2) <= 1.0, [0.12, 0.1, 0.08, 0.9]);
                }
            }
            for sgn in [-1.0f32, 1.0] {
                c.solid(|x, y| y > 6.5 && y < 10.0 && ((x - cx) * sgn - self.fuselage_half(y) * 0.85).abs() < 0.3,
                    [0.1, 0.09, 0.08, 0.35]);
            }
        }
        // canopy frame
        let (cy, cl, cw) = self.canopy;
        for fy in [cy - cl * 0.35, cy + cl * 0.25] {
            c.solid(|x, y| (y - fy).abs() < 0.14 && ((x - cx) / cw).powi(2) + ((y - cy) / cl).powi(2) <= 1.0,
                [0.1, 0.1, 0.12, 0.8]);
        }
        // the propeller: two faint blade arcs in the disc
        let r = wing.half_span * 0.36;
        c.solid(|x, y| {
            let (dx, dy) = ((x - cx) / r, (y - 1.0) / 0.9);
            let d = dx * dx + dy * dy;
            d <= 1.0 && d > 0.8
        }, [0.9, 0.9, 0.88, 0.25]);
    }
}

/// A US star-and-bar on the port wing, as the P-51 carried it.
fn us_star(c: &mut Canvas, x0: f32, y0: f32) {
    let r = 2.1;
    let blue = [0.10, 0.16, 0.42, 1.0];
    c.solid(|x, y| (x - x0).abs() <= r + 1.8 && (y - y0).abs() <= 0.75, blue);
    c.solid(|x, y| (x - x0).abs() <= r + 1.4 && (y - y0).abs() <= 0.42, [1.0, 1.0, 1.0, 1.0]);
    c.solid(|x, y| (x - x0).powi(2) + (y - y0).powi(2) <= r * r, blue);
    c.solid(|x, y| star(x - x0, y - y0, r * 0.95), [1.0, 1.0, 1.0, 1.0]);
}

/// Inside a five-pointed star of outer radius `r`, point up.
fn star(x: f32, y: f32, r: f32) -> bool {
    let pts: Vec<(f32, f32)> = (0..10).map(|i| {
        let a = -std::f32::consts::FRAC_PI_2 + i as f32 * PI / 5.0;
        let rr = if i % 2 == 0 { r } else { r * 0.4 };
        (a.cos() * rr, a.sin() * rr)
    }).collect();
    let mut inside = false;
    let mut j = pts.len() - 1;
    for i in 0..pts.len() {
        let ((xi, yi), (xj, yj)) = (pts[i], pts[j]);
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi { inside = !inside; }
        j = i;
    }
    inside
}

/// A hinomaru with a thin white surround, on each wing.
fn hinomaru(c: &mut Canvas, cx: f32, dx: f32, y0: f32, r: f32) {
    for x0 in [cx - dx, cx + dx] {
        c.solid(|x, y| (x - x0).powi(2) + (y - y0).powi(2) <= (r + 0.55).powi(2), [0.97, 0.97, 0.95, 1.0]);
        c.solid(|x, y| (x - x0).powi(2) + (y - y0).powi(2) <= r * r, [0.80, 0.08, 0.10, 1.0]);
    }
}

/// Player: a P-51D Mustang in natural metal — straight tapered wings with
/// squarish tips, a long inline nose with an olive anti-glare panel, a
/// bubble canopy, red spinner and yellow nose band, star on the port wing.
fn player_plane() -> Vec<u8> {
    // designed at 36x32, rendered at 1.5x
    let (w, h) = (36, 32);
    let mut c = Canvas::scaled(PLAYER_W, PLAYER_H, PLAYER_W as f32 / 36.0);
    let _ = h;
    let cx = w as f32 / 2.0;
    let metal = (196, 202, 212);
    let f = Fighter {
        w,
        body_half: 2.5, widest: 11.0, tail_half: 0.7, tail_y: 31.5,
        radial: false, nose_half: 1.2, nose_len: 4.2,
        wing: Surface { half_span: 17.6, root: (10.0, 18.6), tip: (11.4, 15.0), round: 0.06 },
        tailplane: Surface { half_span: 6.6, root: (25.6, 30.4), tip: (26.9, 29.4), round: 0.12 },
        canopy: (13.2, 2.6, 1.45),
        body: metal, wing_col: (206, 212, 222), nose_col: (228, 196, 40), spinner: (200, 40, 36),
        guns: &[0.38, 0.43, 0.48], // six .50s, three a side
    };
    f.paint(&mut c);
    // olive-drab anti-glare panel from the nose band to the windscreen
    c.solid(|x, y| y > 4.2 && y < 10.8 && (x - cx).abs() <= 0.8, [0.34, 0.36, 0.22, 1.0]);
    us_star(&mut c, cx - 11.5, 13.6);
    c.finish(false)
}

/// Enemy fighters, drawn nose-up and flipped to dive at the player. `kind`: 0
/// = grunt, an A6M Zero (IJN green, black cowl, wide rounded wings); 1 =
/// weaver, a Ki-43 (khaki, slim tapered wings); 2 = ace, a Ki-84 in natural
/// metal with yellow ID stripes (drops a power-up).
fn enemy_plane(kind: usize) -> Vec<u8> {
    // designed at 26x22, rendered at 2x
    let w = 26;
    let mut c = Canvas::scaled(ENEMY_W, ENEMY_H, ENEMY_W as f32 / 26.0);
    let cx = w as f32 / 2.0;
    let (body, wing) = enemy_colors(kind);
    let f = match kind {
        0 => Fighter {
            w,
            body_half: 1.9, widest: 4.5, tail_half: 0.5, tail_y: 20.5,
            radial: true, nose_half: 1.8, nose_len: 3.4,
            wing: Surface { half_span: 12.8, root: (5.6, 11.4), tip: (7.4, 10.4), round: 0.38 },
            tailplane: Surface { half_span: 5.0, root: (16.6, 20.0), tip: (17.6, 19.4), round: 0.45 },
            canopy: (8.6, 2.3, 1.1),
            body, wing_col: wing, nose_col: (38, 38, 42), spinner: (150, 110, 70),
            guns: &[0.42], // a 20 mm cannon in each wing
        },
        1 => Fighter {
            w,
            body_half: 1.6, widest: 4.2, tail_half: 0.45, tail_y: 19.8,
            radial: true, nose_half: 1.5, nose_len: 2.8,
            wing: Surface { half_span: 11.8, root: (5.4, 10.8), tip: (7.8, 9.8), round: 0.3 },
            tailplane: Surface { half_span: 4.4, root: (16.2, 19.2), tip: (17.0, 18.8), round: 0.4 },
            canopy: (8.4, 2.4, 0.95),
            body, wing_col: wing, nose_col: (60, 60, 58), spinner: (150, 110, 70),
            guns: &[],
        },
        _ => Fighter {
            w,
            body_half: 2.1, widest: 5.0, tail_half: 0.55, tail_y: 21.0,
            radial: true, nose_half: 1.6, nose_len: 3.2,
            wing: Surface { half_span: 12.0, root: (6.0, 11.6), tip: (7.6, 10.4), round: 0.32 },
            tailplane: Surface { half_span: 4.8, root: (16.8, 20.4), tip: (17.8, 19.8), round: 0.4 },
            canopy: (9.0, 2.2, 1.15),
            body, wing_col: wing, nose_col: (52, 56, 44), spinner: (160, 120, 70),
            guns: &[0.36, 0.44],
        },
    };
    f.paint(&mut c);
    // IJN/IJA yellow ID stripe on the inboard leading edges
    let wg = f.wing;
    c.solid(|x, y| {
        let t = (x - cx).abs() / wg.half_span;
        t > 0.14 && t < 0.5 && wg.contains(cx, x, y) && y < lerp(wg.root.0, wg.tip.0, t) + 0.9
    }, [0.95, 0.72, 0.12, if kind == 2 { 1.0 } else { 0.8 }]);
    hinomaru(&mut c, cx, wg.half_span * 0.62, lerp(wg.root.0, wg.root.1, 0.5) + 0.4, 1.7);
    c.finish(true)
}

/// Body and wing colours for an enemy kind, chosen against the sky they are
/// seen on (rgb(25,61,117) at the horizon to rgb(7,28,76) near; draw_sea() in
/// the game). Every kind clears 3:1 against both ends, body and wing, the
/// WCAG non-text contrast floor, pinned by
/// enemy_planes_are_visible_against_the_sky(). Wings are lit from above,
/// brighter than the fuselage: the widest surface, the first picked out.
fn enemy_colors(kind: usize) -> ((u8, u8, u8), (u8, u8, u8)) {
    let body: (u8, u8, u8) = match kind {
        0 => (125, 175, 115),
        1 => (205, 158, 68),
        _ => (214, 218, 224), // natural metal: the ace's Ki-84 is left unpainted
    };
    let wing = (
        body.0.saturating_add(38),
        body.1.saturating_add(38),
        body.2.saturating_add(38),
    );
    (body, wing)
}

/// A heavy aircraft seen from above, nose up, in design px (= sprite px).
/// Every length is laid out from real planforms of the type.
struct Bomber {
    w: i32,
    fus_half: f32, fus_widest: f32, tail_y: f32,
    nose_glaze: f32, hull: bool, floats: bool,
    wing: Surface,
    engines: &'static [f32], // nacelle positions, fractions of the half span
    cowl: f32, nacelle_len: f32,
    tailplane: Surface,
    fins: &'static [f32],    // fin positions, fractions of the tailplane half span
    turrets: &'static [(f32, f32)],
}

impl Bomber {
    fn fuselage_half(&self, y: f32) -> f32 {
        if y < 0.0 || y > self.tail_y { return -1.0; }
        let hw = if self.hull { self.fus_half * 1.25 } else { self.fus_half };
        if y < self.fus_widest {
            let s = y / self.fus_widest;
            hw * (1.0 - (1.0 - s).powi(2)).sqrt().max(0.25)
        } else {
            lerp(hw, hw * 0.3, ((y - self.fus_widest) / (self.tail_y - self.fus_widest)).powf(1.3))
        }
    }

    fn paint(&self, c: &mut Canvas) {
        let cx = self.w as f32 / 2.0;
        let hs = self.wing.half_span;
        let (wing, tp) = (self.wing, self.tailplane);
        let green = (104, 132, 92);
        let green_w = (116, 146, 102);
        let le = |t: f32| lerp(wing.root.0, wing.tip.0, t);
        let te = |t: f32| lerp(wing.root.1, wing.tip.1, t);
        // propeller discs ahead of each engine
        for &t in self.engines {
            for sgn in [-1.0f32, 1.0] {
                let (ex, ey) = (cx + sgn * t * hs, le(t) - self.cowl * 1.4);
                let r = self.cowl * 2.3;
                c.solid(|x, y| ((x - ex) / r).powi(2) + ((y - ey) / (r * 0.18)).powi(2) <= 1.0, [0.8, 0.8, 0.78, 0.22]);
            }
        }
        // tail surfaces and fins
        c.fill(|x, y| tp.contains(cx, x, y), |x, y| tp.shade(cx, x, y), rgb(green_w));
        c.solid(|x, y| {
            let t = (x - cx).abs() / tp.half_span;
            let (l, e) = (lerp(tp.root.0, tp.tip.0, t), lerp(tp.root.1, tp.tip.1, t));
            (y - (e - (e - l) * 0.35)).abs() < 0.4 && t > 0.1 && tp.contains(cx, x, y)
        }, [0.0, 0.0, 0.0, 0.25]);
        // wings, with floats under the tips of a flying boat
        c.fill(|x, y| wing.contains(cx, x, y), |x, y| wing.shade(cx, x, y), rgb(green_w));
        if self.floats {
            for sgn in [-1.0f32, 1.0] {
                let (fx, fy) = (cx + sgn * 0.84 * hs, (le(0.84) + te(0.84)) / 2.0);
                c.solid(|x, y| ((x - fx) / (self.cowl * 0.8)).powi(2) + ((y - fy) / (self.cowl * 2.4)).powi(2) <= 1.0,
                    [0.55, 0.6, 0.52, 1.0]);
            }
        }
        // panel lines, spar, flaps and ailerons
        let line = [0.0, 0.0, 0.0, 0.18];
        for k in 1..8 {
            let t = k as f32 / 8.0;
            c.solid(|x, y| ((x - cx).abs() / hs - t).abs() * hs < 0.35 && wing.contains(cx, x, y), line);
        }
        c.solid(|x, y| {
            let t = (x - cx).abs() / hs;
            (y - (le(t) + (te(t) - le(t)) * 0.3)).abs() < 0.3 && wing.contains(cx, x, y)
        }, [0.0, 0.0, 0.0, 0.1]);
        c.solid(|x, y| {
            let t = (x - cx).abs() / hs;
            (y - (te(t) - (te(t) - le(t)) * 0.25)).abs() < 0.4 && t > 0.12 && wing.contains(cx, x, y)
        }, [0.0, 0.0, 0.0, 0.28]);
        // yellow ID stripe on the inboard leading edge
        c.solid(|x, y| {
            let t = (x - cx).abs() / hs;
            t > 0.08 && t < 0.3 && wing.contains(cx, x, y) && y < le(t) + 1.6
        }, [0.95, 0.72, 0.12, 0.85]);
        // engine nacelles: cowl ring forward, nacelle over the wing, soot aft
        for &t in self.engines {
            for sgn in [-1.0f32, 1.0] {
                let ex = cx + sgn * t * hs;
                let (y0, y1) = (le(t) - self.cowl * 1.3, le(t) + self.nacelle_len);
                c.fill(|x, y| y > y0 && y < y1 && ((x - ex).abs() / self.cowl) <= (1.0 - ((y - y0) / (y1 - y0)).powi(3) * 0.7),
                    |x, _| 0.7 + 0.4 * (1.0 - ((x - ex) / self.cowl).powi(2)).max(0.0).sqrt(), rgb(green));
                c.solid(|x, y| ((x - ex) / self.cowl).powi(2) + ((y - y0 - self.cowl * 0.3) / (self.cowl * 0.45)).powi(2) <= 1.0,
                    [0.12, 0.12, 0.13, 1.0]);
                c.solid(|x, y| ((x - ex) / (self.cowl * 0.3)).powi(2) + ((y - y0 - self.cowl * 0.1) / (self.cowl * 0.3)).powi(2) <= 1.0,
                    [0.5, 0.45, 0.35, 1.0]);
                c.solid(|x, y| y > y1 - 1.0 && y < y1 + self.nacelle_len * 0.18 && (x - ex).abs() < self.cowl * 0.3,
                    [0.08, 0.07, 0.06, 0.16]);
            }
        }
        // fins, edge-on
        for &f in self.fins {
            for sgn in [-1.0f32, 1.0] {
                let fx = cx + sgn * f * tp.half_span;
                if f == 0.0 && sgn > 0.0 { continue; }
                c.solid(|x, y| y > tp.root.0 - 3.0 && y < self.tail_y + 1.0 && (x - fx).abs() < 0.8, [0.2, 0.26, 0.18, 0.95]);
            }
        }
        // fuselage (a boat hull is wider, with a keel line)
        c.fill(|x, y| (x - cx).abs() <= self.fuselage_half(y), |x, y| {
            let hw = self.fuselage_half(y).max(0.3);
            let u = ((x - cx) / hw).clamp(-1.0, 1.0);
            0.7 + 0.42 * (1.0 - u * u).sqrt()
        }, rgb(green));
        if self.hull {
            c.solid(|x, y| (x - cx).abs() < 0.5 && y > self.fus_widest && y < self.tail_y * 0.8, [0.0, 0.0, 0.0, 0.2]);
        }
        for k in 1..6 {
            let yb = self.tail_y * k as f32 / 6.0;
            c.solid(|x, y| (y - yb).abs() < 0.3 && (x - cx).abs() <= self.fuselage_half(y), line);
        }
        // glazed nose with its frames, then the cockpit
        c.solid(|x, y| y < self.nose_glaze && (x - cx).abs() <= self.fuselage_half(y) * 0.92, [0.3, 0.42, 0.5, 1.0]);
        for k in 1..4 {
            let fy = self.nose_glaze * k as f32 / 4.0;
            c.solid(|x, y| (y - fy).abs() < 0.3 && (x - cx).abs() <= self.fuselage_half(y) * 0.92, [0.15, 0.18, 0.2, 0.9]);
        }
        c.solid(|x, y| y < self.nose_glaze && (x - cx).abs() < 0.3, [0.15, 0.18, 0.2, 0.9]);
        let (cy, cl) = (self.nose_glaze + self.fus_half * 1.6, self.fus_half * 1.1);
        c.solid(|x, y| ((x - cx) / (self.fus_half * 0.55)).powi(2) + ((y - cy) / cl).powi(2) <= 1.0, [0.2, 0.28, 0.36, 1.0]);
        c.solid(|x, y| ((x - cx + self.fus_half * 0.18) / (self.fus_half * 0.2)).powi(2) + ((y - cy + cl * 0.3) / (cl * 0.4)).powi(2) <= 1.0,
            [0.8, 0.9, 1.0, 0.7]);
        // gun turrets: glazed domes with twin barrels
        let h = self.tail_y / 0.99;
        for &(tx, ty) in self.turrets {
            let (gx, gy) = (cx + tx * hs, ty * h);
            let r = (self.fus_half * 0.55).max(2.2);
            let back = ty > 0.9;
            let dir = if back { 1.0 } else { -1.0 };
            for off in [-0.9f32, 0.9] {
                c.solid(|x, y| (x - gx - off).abs() < 0.45 && (y - gy) * dir > 0.0 && (y - gy).abs() < r * 2.4,
                    [0.08, 0.08, 0.08, 1.0]);
            }
            c.solid(|x, y| ((x - gx) / r).powi(2) + ((y - gy) / r).powi(2) <= 1.0, [0.35, 0.48, 0.56, 1.0]);
            c.solid(|x, y| ((x - gx + r * 0.3) / (r * 0.35)).powi(2) + ((y - gy - r * 0.3) / (r * 0.35)).powi(2) <= 1.0,
                [0.85, 0.95, 1.0, 0.7]);
        }
        // hinomaru on each wing
        let t = 0.68;
        let chord = te(t) - le(t);
        hinomaru(c, cx, t * hs, le(t) + chord * 0.5, chord * 0.28);
    }
}

/// The seven bosses, smallest to largest: Ki-49 Donryu, Ki-67 Hiryu,
/// G4M "Betty", G8N Renzan, H8K flying boat, G5N Shinzan and the six-engine
/// Fugaku. Drawn nose-up and flipped so they fly at the player.
fn boss_plane(tier: usize) -> Vec<u8> {
    let (w, h) = BOSS_SIZES[tier];
    let (hs, l) = (w as f32 / 2.0, h as f32);
    let wing = |rle: f32, rte: f32, tle: f32, tte: f32, round: f32| Surface {
        half_span: hs * 0.98, root: (l * rle, l * rte), tip: (l * tle, l * tte), round };
    let tailp = |span: f32, y0: f32, y1: f32| Surface {
        half_span: hs * span, root: (l * y0, l * y1), tip: (l * (y0 + 0.03), l * (y1 - 0.02)), round: 0.3 };
    let b = match tier {
        0 => Bomber { w, fus_half: 5.5, fus_widest: l * 0.3, tail_y: l * 0.98, nose_glaze: l * 0.1, hull: false, floats: false,
            wing: wing(0.30, 0.52, 0.37, 0.46, 0.2), engines: &[0.3], cowl: 4.2, nacelle_len: l * 0.2,
            tailplane: tailp(0.34, 0.84, 0.95), fins: &[0.0], turrets: BOSS_TURRETS[0] },
        1 => Bomber { w, fus_half: 5.2, fus_widest: l * 0.3, tail_y: l * 0.98, nose_glaze: l * 0.1, hull: false, floats: false,
            wing: wing(0.30, 0.50, 0.37, 0.45, 0.25), engines: &[0.3], cowl: 4.4, nacelle_len: l * 0.22,
            tailplane: tailp(0.34, 0.84, 0.95), fins: &[0.0], turrets: BOSS_TURRETS[1] },
        2 => Bomber { w, fus_half: 7.5, fus_widest: l * 0.35, tail_y: l * 0.99, nose_glaze: l * 0.12, hull: false, floats: false,
            wing: wing(0.32, 0.54, 0.40, 0.48, 0.35), engines: &[0.28], cowl: 5.0, nacelle_len: l * 0.2,
            tailplane: tailp(0.3, 0.84, 0.95), fins: &[0.0], turrets: BOSS_TURRETS[2] },
        3 => Bomber { w, fus_half: 6.5, fus_widest: l * 0.3, tail_y: l * 0.98, nose_glaze: l * 0.1, hull: false, floats: false,
            wing: wing(0.30, 0.52, 0.38, 0.46, 0.15), engines: &[0.22, 0.5], cowl: 4.8, nacelle_len: l * 0.18,
            tailplane: tailp(0.3, 0.84, 0.95), fins: &[0.0], turrets: BOSS_TURRETS[3] },
        4 => Bomber { w, fus_half: 7.5, fus_widest: l * 0.28, tail_y: l * 0.98, nose_glaze: l * 0.07, hull: true, floats: true,
            wing: wing(0.30, 0.46, 0.34, 0.41, 0.2), engines: &[0.2, 0.42], cowl: 5.0, nacelle_len: l * 0.14,
            tailplane: tailp(0.26, 0.86, 0.96), fins: &[0.0], turrets: BOSS_TURRETS[4] },
        5 => Bomber { w, fus_half: 7.0, fus_widest: l * 0.3, tail_y: l * 0.97, nose_glaze: l * 0.09, hull: false, floats: false,
            wing: wing(0.30, 0.48, 0.36, 0.43, 0.15), engines: &[0.2, 0.45], cowl: 5.2, nacelle_len: l * 0.16,
            tailplane: tailp(0.3, 0.85, 0.95), fins: &[0.85], turrets: BOSS_TURRETS[5] },
        _ => Bomber { w, fus_half: 9.0, fus_widest: l * 0.28, tail_y: l * 0.98, nose_glaze: l * 0.07, hull: false, floats: false,
            wing: wing(0.30, 0.46, 0.36, 0.41, 0.15), engines: &[0.14, 0.3, 0.46], cowl: 5.6, nacelle_len: l * 0.14,
            tailplane: tailp(0.24, 0.86, 0.96), fins: &[0.0], turrets: BOSS_TURRETS[6] },
    };
    let mut c = Canvas::new(w, h);
    b.paint(&mut c);
    c.finish(true)
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

/// Health pickup, from regular fighters: a white roundel with a red cross, a
/// different shape and fixed colour from the weapon capsule's diamond.
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

/// The carrier: an Essex-class fleet carrier as in 1944, bow up. Deck Blue
/// planked flight deck, centreline and hull number fore and aft; two
/// centreline elevators and the port deck-edge one; arresting wires aft,
/// catapults at the bow; the island to starboard with funnel, radar and twin
/// 5-inch mounts fore and aft; 40 mm quads in the sponsons; Hellcats spotted
/// aft, wings folded. Shortened to about 60% to fit.
fn carrier_ship() -> Vec<u8> {
    let (w, h) = (CARRIER_W, CARRIER_H);
    let mut c = Canvas::new(w, h);
    let (wf, hf) = (w as f32, h as f32);
    let cx = 88.0;                  // the deck centreline (the island is to starboard)
    let (dl, dr) = (cx - 75.0, cx + 75.0);
    let (bow, stern) = (42.0, hf - 18.0);
    let hull = [0.24, 0.27, 0.31, 1.0];
    let grey = [0.46, 0.49, 0.53, 1.0];
    let white = [0.92, 0.92, 0.88, 1.0];
    // the hull below the deck: a sharp bow and a rounded stern
    c.clip(0.0, 0.0, wf, 70.0);
    c.solid(|x, y| y > 2.0 && y < 70.0 && (x - cx).abs() < (y - 2.0) * 1.3, hull);
    c.clip(0.0, stern - 20.0, wf, hf);
    c.solid(|x, y| y > stern - 20.0 && ((x - cx) / 72.0).powi(2) + ((y - (stern - 20.0)) / 36.0).powi(2) <= 1.0, hull);
    // sponsons and catwalks along both sides
    c.clip(0.0, 0.0, wf, hf);
    c.solid(|x, y| y > bow + 30.0 && y < stern - 10.0 && ((x > dl - 9.0 && x < dl) || (x > dr && x < dr + 9.0)),
        [0.3, 0.33, 0.37, 1.0]);
    for k in 0..90 {
        let y0 = bow + 34.0 + k as f32 * 7.4;
        if y0 > stern - 14.0 { break; }
        c.clip(0.0, y0 - 1.0, wf, y0 + 1.0);
        c.solid(|x, y| (y - y0).abs() < 0.5 && ((x - (dl - 8.0)).abs() < 0.6 || (x - (dr + 8.0)).abs() < 0.6), [0.62, 0.64, 0.66, 1.0]);
    }
    // the deck: squared at the ends, narrower at the bow
    c.clip(0.0, bow - 2.0, wf, stern + 2.0);
    let deck_half = |y: f32| if y < bow + 60.0 { lerp(52.0, 75.0, ((y - bow) / 60.0).clamp(0.0, 1.0)) } else { 75.0 };
    let deck = |x: f32, y: f32| y > bow && y < stern && (x - cx).abs() <= deck_half(y);
    c.fill(|x, y| deck(x, y), |x, _y| {
        // planking: a slightly different tone every strip, seams between
        let strip = ((x - cx + 80.0) / 2.6).floor();
        let tone = 0.94 + 0.06 * ((strip * 12.9898).sin() * 43758.5).fract();
        let seam = if ((x - cx + 80.0) % 2.6) < 0.35 { 0.82 } else { 1.0 };
        tone * seam
    }, [0.29, 0.34, 0.42, 1.0]);
    // tie-down strips across the deck
    for k in 0..40 {
        let y0 = bow + 20.0 + k as f32 * 18.0;
        if y0 > stern - 4.0 { break; }
        c.clip(0.0, y0 - 1.0, wf, y0 + 1.0);
        c.solid(|x, y| (y - y0).abs() < 0.35 && deck(x, y), [0.55, 0.58, 0.62, 0.28]);
    }
    c.clip(0.0, bow - 2.0, wf, stern + 2.0);
    // deck edge lines
    c.solid(|x, y| deck(x, y) && (deck_half(y) - (x - cx).abs()) < 1.2, [0.8, 0.8, 0.78, 0.8]);
    // elevators: centreline forward and aft, deck-edge to port
    for (y0, y1) in [(bow + 110.0, bow + 158.0), (stern - 196.0, stern - 148.0)] {
        c.clip(cx - 30.0, y0 - 2.0, cx + 30.0, y1 + 2.0);
        c.solid(|x, y| y > y0 && y < y1 && (x - cx).abs() < 25.0 && ((y - y0).abs() < 0.8 || (y - y1).abs() < 0.8
            || ((x - cx).abs() - 25.0).abs() < 0.8 || (x - cx).abs() > 24.2), [0.1, 0.12, 0.15, 0.9]);
        c.solid(|x, y| y > y0 + 1.0 && y < y1 - 1.0 && (x - cx).abs() < 24.0, [0.0, 0.0, 0.0, 0.08]);
    }
    let (ey0, ey1) = (bow + 250.0, bow + 292.0);
    c.clip(0.0, ey0 - 2.0, dl + 2.0, ey1 + 2.0);
    c.solid(|x, y| y > ey0 && y < ey1 && x > dl - 20.0 && x < dl + 1.0, [0.29, 0.34, 0.42, 1.0]);
    c.solid(|x, y| y > ey0 && y < ey1 && x > dl - 20.0 && x < dl + 1.0
        && ((y - ey0).abs() < 0.8 || (y - ey1).abs() < 0.8 || (x - (dl - 20.0)).abs() < 0.8), [0.1, 0.12, 0.15, 0.9]);
    // centreline: dashed white
    c.clip(cx - 2.0, bow, cx + 2.0, stern);
    c.solid(|x, y| (x - cx).abs() < 0.9 && y > bow + 50.0 && y < stern - 40.0 && ((y - bow) % 22.0) < 13.0, white);
    // catapult tracks at the bow
    for off in [-34.0f32, 34.0] {
        c.clip(cx + off - 2.0, bow, cx + off + 2.0, bow + 90.0);
        c.solid(|x, y| (x - cx - off).abs() < 0.6 && y > bow + 10.0 && y < bow + 84.0, [0.08, 0.08, 0.1, 0.8]);
    }
    // arresting wires aft, with their sheaves at the deck edge
    for k in 0..9 {
        let y0 = stern - 130.0 + k as f32 * 10.0;
        c.clip(0.0, y0 - 2.0, wf, y0 + 2.0);
        c.solid(|x, y| (y - y0).abs() < 0.45 && deck(x, y) && (x - cx).abs() < 70.0, [0.75, 0.76, 0.74, 0.8]);
        for sgn in [-1.0f32, 1.0] {
            c.solid(|x, y| ((x - cx - sgn * 70.0).powi(2) + (y - y0).powi(2)) < 2.2, [0.12, 0.12, 0.12, 1.0]);
        }
    }
    // barriers forward of the wires
    for y0 in [stern - 150.0, stern - 143.0] {
        c.clip(0.0, y0 - 2.0, wf, y0 + 2.0);
        c.solid(|x, y| (y - y0).abs() < 0.6 && deck(x, y) && (x - cx).abs() < 66.0, [0.85, 0.75, 0.2, 0.8]);
    }
    // the hull number, "9", fore and aft
    let nine = |x: f32, y: f32, x0: f32, y0: f32| -> bool {
        let (u, v) = ((x - x0) / 14.0, (y - y0) / 24.0);
        if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) { return false; }
        let bar = 0.2;
        let top = v < bar; let mid = (v - 0.45).abs() < bar / 2.0; let bot = v > 1.0 - bar;
        let left = u < bar * 1.4 && v < 0.5; let right = u > 1.0 - bar * 1.4;
        top || mid || (bot && u > 0.2) || left || right
    };
    c.clip(cx - 10.0, bow + 20.0, cx + 10.0, bow + 50.0);
    c.solid(|x, y| nine(x, y, cx - 7.0, bow + 22.0), white);
    c.clip(cx - 10.0, stern - 34.0, cx + 10.0, stern - 4.0);
    // aft, turned round to read from astern
    c.solid(|x, y| nine(cx * 2.0 - x, (stern - 20.0) * 2.0 - y, cx - 7.0, stern - 32.0), white);
    // the island, to starboard, its shadow on the deck
    let (ix0, ix1, iy0, iy1) = (dr - 12.0, dr + 22.0, bow + 200.0, bow + 330.0);
    c.clip(ix0 - 8.0, iy0 - 4.0, ix1 + 2.0, iy1 + 8.0);
    c.solid(|x, y| x > ix0 - 6.0 && x < ix0 && y > iy0 + 6.0 && y < iy1 + 6.0, [0.0, 0.0, 0.0, 0.25]);
    c.fill(|x, y| x > ix0 && x < ix1 && y > iy0 && y < iy1
        && !(y < iy0 + 20.0 && x < ix0 + (iy0 + 20.0 - y) * 0.5), |x, _| 0.85 + 0.25 * ((x - ix0) / (ix1 - ix0)), grey);
    // bridge windows, the funnel with its soot, radar and mast
    c.solid(|x, y| y > iy0 + 22.0 && y < iy0 + 25.0 && x > ix0 + 3.0 && x < ix1 - 3.0 && ((x - ix0) % 3.5) < 2.0,
        [0.1, 0.14, 0.2, 1.0]);
    let (fx, fy) = ((ix0 + ix1) / 2.0 + 2.0, iy0 + 78.0);
    c.solid(|x, y| ((x - fx) / 10.0).powi(2) + ((y - fy) / 20.0).powi(2) <= 1.0, [0.36, 0.38, 0.42, 1.0]);
    c.solid(|x, y| ((x - fx) / 7.0).powi(2) + ((y - fy) / 16.0).powi(2) <= 1.0, [0.06, 0.06, 0.07, 1.0]);
    let (ry, rx) = (iy0 + 44.0, (ix0 + ix1) / 2.0);
    c.solid(|x, y| (x - rx).abs() < 9.0 && (y - ry).abs() < 3.0
        && (((x - rx + 9.0) % 2.0) < 0.6 || (y - ry).abs() > 2.4), [0.7, 0.72, 0.75, 1.0]);
    c.solid(|x, y| (x - rx).abs() < 0.6 && y > ry - 10.0 && y < ry + 30.0, [0.2, 0.2, 0.22, 1.0]);
    c.solid(|x, y| (y - (ry - 8.0)).abs() < 0.5 && (x - rx).abs() < 6.0, [0.2, 0.2, 0.22, 1.0]);
    // twin 5-inch mounts, two forward of the island, two aft
    for (my, fwd) in [(iy0 - 36.0, true), (iy0 - 16.0, true), (iy1 + 12.0, false), (iy1 + 32.0, false)] {
        let mx = dr + 4.0;
        c.clip(mx - 12.0, my - 20.0, mx + 12.0, my + 20.0);
        let dir = if fwd { -1.0 } else { 1.0 };
        for off in [-2.2f32, 2.2] {
            c.solid(|x, y| (x - mx - off).abs() < 0.8 && (y - my) * dir > 0.0 && (y - my).abs() < 15.0, [0.15, 0.15, 0.16, 1.0]);
        }
        c.fill(|x, y| ((x - mx) / 8.0).powi(2) + ((y - my) / 6.5).powi(2) <= 1.0, |_, y| 1.1 - 0.02 * (y - my), grey);
    }
    // 40 mm quad mounts in the sponsons and at bow and stern
    let quads = [(dl - 5.0, bow + 90.0), (dl - 5.0, bow + 210.0), (dl - 5.0, bow + 420.0), (dl - 5.0, stern - 70.0),
                 (dr + 5.0, bow + 140.0), (dr + 5.0, bow + 480.0), (dr + 5.0, stern - 60.0),
                 (cx - 18.0, bow + 4.0), (cx + 18.0, bow + 4.0), (cx - 30.0, stern + 6.0), (cx + 30.0, stern + 6.0)];
    for (qx, qy) in quads {
        c.clip(qx - 8.0, qy - 10.0, qx + 8.0, qy + 8.0);
        c.solid(|x, y| ((x - qx).powi(2) + (y - qy).powi(2)) < 20.0, [0.4, 0.42, 0.45, 1.0]);
        for k in 0..4 {
            let bx = qx - 2.4 + k as f32 * 1.6;
            c.solid(|x, y| (x - bx).abs() < 0.35 && y < qy - 2.0 && y > qy - 8.0, [0.12, 0.12, 0.13, 1.0]);
        }
    }
    // Hellcats spotted aft, wings folded back along their sides
    let hellcat = |c: &mut Canvas, px: f32, py: f32| {
        c.clip(px - 12.0, py - 16.0, px + 12.0, py + 16.0);
        let blue = [0.18, 0.25, 0.4, 1.0];
        c.solid(|x, y| (y - py).abs() < 13.0 && (x - px).abs() < 3.4 - ((y - py + 4.0) / 13.0).abs().powi(2) * 1.6, blue);
        c.solid(|x, y| y > py - 7.0 && y < py + 9.0 && ((x - px).abs() - 5.5).abs() < 1.6, [0.2, 0.28, 0.44, 1.0]);
        c.solid(|x, y| y > py + 9.0 && y < py + 12.5 && (x - px).abs() < 7.0, blue);
        c.solid(|x, y| ((x - px) / 1.6).powi(2) + ((y - py + 3.0) / 3.0).powi(2) <= 1.0, [0.5, 0.65, 0.8, 0.9]);
        c.solid(|x, y| ((x - px) / 3.2).powi(2) + ((y - py + 12.5) / 1.4).powi(2) <= 1.0, [0.12, 0.12, 0.12, 1.0]);
    };
    for (k, (dx, dy)) in [(-44.0f32, -104.0f32), (-22.0, -104.0), (0.0, -100.0), (22.0, -104.0), (44.0, -104.0),
                          (-33.0, -74.0), (-11.0, -72.0), (11.0, -72.0), (33.0, -74.0)].iter().enumerate() {
        let _ = k;
        hellcat(&mut c, cx + dx, stern + dy + 30.0);
    }
    c.unclip();
    c.finish(false)
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
// Clouds: value-noise fBm, not flat circles                                //
// ---------------------------------------------------------------------- //
// Cumulus: a flattish harder base and a broken, billowing top, bright where
// lit and greyer between puffs. A few octaves of value noise (fBm) masked by
// an envelope flattened underneath drive both the edge and the lit/shadowed
// tint. Baked per variant at build time.

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
// Islands: rare, turret-armed landmasses                                  //
// ---------------------------------------------------------------------- //
// The clouds' fBm, masked to an opaque island: a noise-perturbed coastline, a
// band of sand, mottled green and rock inland, and a turret emplacement.
// Three sizes baked; which and where is picked at runtime.
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

/// Hermite smoothstep, clamped to 0..1: ramps the engine's pitch and chop
/// through the start-up.
fn smooth01(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// The carrier launch start-up in one shot: the starter whirring, the engine
/// catching on a couple of uneven chugs, then the prop spinning up to idle.
/// ~1.9s, played as the launch begins over the engine loops.
fn engine_start_sfx() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let dur_s = 1.9_f32;
    let n = ms_to_samples(dur_s * 1000.0);
    let mut rng = Lcg(0x0E17_0011);
    let mut s = vec![0f32; n];
    for i in 0..n {
        let t = i as f32 / sr;
        let k = i as f32 / n as f32;

        // Starter motor: a low buzz with a fast amplitude wobble (the
        // electric crank), fading out as the engine takes over.
        let starter_env = (1.0 - (k / 0.62).min(1.0)).powf(1.4);
        let crank_f = 52.0 + 12.0 * k;
        let crank_wob = 0.6 + 0.4 * (2.0 * PI * 9.0 * t).sin();
        let crank_noise = ((rng.next() % 4096) as f32 / 4096.0 - 0.5) * 0.5;
        let starter = ((2.0 * PI * crank_f * t).sin() * crank_wob + crank_noise) * starter_env * 0.5;

        // The engine: pitch and blade-chop rate ramp up from a lumpy
        // catch to a steady idle over the second half of the clip.
        let eng_env = (k / 0.30).min(1.0);
        let spin = smooth01((k - 0.22) / 0.62);
        let f0 = 34.0 + 46.0 * spin;
        let chop_hz = 5.0 + 20.0 * spin;
        let chop = 0.4 + 0.6 * ((2.0 * PI * chop_hz * t).sin() * 0.5 + 0.5).powf(1.7);
        let tone = (2.0 * PI * f0 * t).sin()
            + 0.5 * (2.0 * PI * f0 * 2.0 * t).sin()
            + 0.25 * (2.0 * PI * f0 * 3.0 * t).sin();
        let engine = (tone * 0.4).tanh() * chop * eng_env * 0.9;

        // Two or three uneven "chug" thumps as the cylinders first fire.
        let mut chug = 0.0f32;
        for &(ct, cv) in &[(0.60_f32, 0.9_f32), (0.76, 0.7), (0.88, 0.5)] {
            let d = t - ct * dur_s;
            if (0.0..0.10).contains(&d) {
                let ce = (1.0 - d / 0.10).powf(1.5);
                chug += (2.0 * PI * 44.0 * d).sin() * ce * cv;
            }
        }

        s[i] = starter + engine + chug * 0.55;
    }
    let f = ms_to_samples(8.0);
    for i in 0..f {
        let g = i as f32 / f as f32;
        s[i] *= g;
        s[n - 1 - i] *= g;
    }
    let pcm: Vec<i16> = s.iter().map(|&v| (v.tanh() * 24_000.0) as i16).collect();
    half_rate(&pcm)
}


// ---- the engine: a V12, cylinder by cylinder ------------------------------
// Measured from Spitfire recordings (Wikimedia Commons, Duxford ground run
// and a Biggin Hill landing): a comb of harmonics spaced at the crankshaft
// rate (~26 Hz at a steady run, ~16 Hz throttled back), loudest at the
// firing rate (6 x crank for a V12), the loudness pulsing at the crank rate
// because no two cylinders are quite alike, all over a broad exhaust rasp.
// A few sines (the old engine) read as a generator; a train of exhaust pops
// reads as a piston engine.

/// Crankshaft turns per second at cruise.
const CRANK_HZ: f32 = 26.0;
/// How loud each of the twelve cylinders fires, in firing order: the six of
/// one turn differ most, so the loudness pulses at the crank rate as in the
/// recordings, and the two turns differ a little.
const CYLINDERS: [f32; 12] = [1.0, 0.72, 0.88, 0.66, 0.94, 0.78, 0.97, 0.70, 0.90, 0.64, 0.92, 0.80];
/// Each cylinder fires a touch early or late (fraction of a firing gap),
/// the same every cycle, so the harmonics stay sharp lines.
const CYL_TIMING: [f32; 12] = [0.0, 0.05, -0.03, 0.04, -0.05, 0.02, 0.01, 0.06, -0.04, 0.03, -0.02, 0.05];
/// Four blades geared down: two blade passes per crank turn.
const BLADE_PER_CRANK: f32 = 2.0;

/// One cylinder's exhaust pop into `buf` at sample `at`: a burst of dark
/// noise and a short low thump. Higher `rpm` is a harder, brighter bark.
fn exhaust_pop(buf: &mut [f32], at: usize, amp: f32, rpm: f32, rng: &mut Rng) {
    let len = (0.03 * SR_F) as usize;
    let body = 95.0 + 45.0 * rpm;
    let bright = 0.13 + 0.10 * rpm;
    let mut lp = 0.0f32;
    for i in 0..len {
        let j = at + i;
        if j >= buf.len() { break; }
        let t = i as f32 / SR_F;
        let w = rng.next_f32() * 2.0 - 1.0;
        lp += (w - lp) * bright;
        // the exhaust stack rings at two pipe modes
        let thump = (2.0 * PI * body * t).sin() * (-t / 0.011).exp()
            + 0.5 * (2.0 * PI * body * 2.1 * t).sin() * (-t / 0.006).exp();
        buf[j] += amp * (lp * 1.3 * (-t / 0.005).exp() + thump * 1.2);
    }
}

/// The propeller: a soft roar of low noise, swelling at the blade-pass rate.
fn prop_roar(buf: &mut [f32], blade_hz: f32, level: f32, rng: &mut Rng) {
    let (mut l1, mut l2) = (0.0f32, 0.0f32);
    for (i, v) in buf.iter_mut().enumerate() {
        let t = i as f32 / SR_F;
        let w = rng.next_f32() * 2.0 - 1.0;
        l1 += (w - l1) * 0.04;
        l2 += (l1 - l2) * 0.04;
        let swell = 0.55 + 0.45 * (2.0 * PI * blade_hz * t).sin();
        *v += l2 * 9.0 * swell * level;
    }
}

/// One engine loop at `rpm` (1.0 = cruise), one second long. The crank rate
/// is rounded to an even number so the twelve-cylinder cycle (two turns) and
/// the blade pass fit the second whole, and the loop has no seam.
/// `sputter` is the labouring engine at a stall: lumpy, missing firings.
fn engine_loop(rpm: f32, sputter: bool) -> Vec<u8> {
    let n = SAMPLE_RATE as usize;
    let crank = ((CRANK_HZ * rpm / 2.0).round() * 2.0).max(4.0);
    let firing = (crank * 6.0) as usize; // firings per second
    let mut rng = Rng(0x0E61_4E55 ^ firing as u32);
    let mut buf = vec![0.0f32; n + (0.05 * SR_F) as usize];
    for k in 0..firing {
        let at = (((k as f32 + CYL_TIMING[k % 12]) / firing as f32).max(0.0) * SR_F) as usize;
        let mut amp = CYLINDERS[k % 12] * (0.75 + 0.25 * rpm.min(1.2));
        if sputter {
            let r = rng.next_f32();
            if r < 0.35 { continue; }            // a misfire
            if r > 0.96 { amp *= 2.2; }          // a pop in the exhaust
        }
        exhaust_pop(&mut buf, at, amp, rpm, &mut rng);
    }
    prop_roar(&mut buf, BLADE_PER_CRANK * crank, 0.35 + 0.25 * rpm, &mut rng);
    let tail = buf.split_off(n);
    for (i, v) in tail.iter().enumerate() { buf[i] += v; }
    let scaled: Vec<f32> = buf.iter().map(|v| v * 11_000.0).collect();
    half_rate(&soft_limit_to_pcm16(&scaled, MIX_KNEE))
}

/// Seconds into `engine_splutter_sfx` the engine dies, and catches again.
/// The game ducks the engine loops between them (see sky_raider's main.rs).
pub const SPLUTTER_DIES: f32 = 0.15;
pub const SPLUTTER_CATCHES: f32 = 1.2;
pub const SPLUTTER_SECS: f32 = 1.6;

/// Bad fuel: the cruising engine misses more and more, nearly dies with the
/// prop windmilling and two coughs, then catches with a pop and runs again.
/// Rendered at the cruise loop's level, so it crossfades with it.
fn engine_splutter_sfx() -> Vec<u8> {
    let n = (SPLUTTER_SECS * SR_F) as usize;
    let crank = CRANK_HZ;
    let firing = crank * 6.0;
    let mut rng = Rng(0x5B1A_7731);
    let mut buf = vec![0.0f32; n + (0.05 * SR_F) as usize];
    let count = (SPLUTTER_SECS * firing) as usize;
    for k in 0..count {
        let t = k as f32 / firing;
        // the chance a firing is missed: rising, nearly dead, recovering
        let miss = if t < 0.35 { 0.1 + 0.6 * t / 0.35 }
            else if t < 1.05 { 0.93 }
            else if t < 1.4 { 0.93 * (1.0 - (t - 1.05) / 0.35) }
            else { 0.0 };
        if rng.next_f32() < miss { continue; }
        let mut amp = CYLINDERS[k % 12];
        if (0.4..1.1).contains(&t) && rng.next_f32() < 0.3 { amp *= 2.4; } // a cough
        // the engine slows as it starves, and the pops get duller
        let rpm = if t < 1.05 { 1.0 - 0.35 * (t / 1.05) } else { 0.65 + 0.35 * ((t - 1.05) / 0.35).min(1.0) };
        let at = ((t + CYL_TIMING[k % 12] / firing) * SR_F) as usize;
        exhaust_pop(&mut buf, at, amp, rpm, &mut rng);
    }
    // two coughs as it tries to catch, and the pop as it does
    for &(t, a) in &[(0.55f32, 3.2f32), (0.83, 2.6), (1.08, 3.6)] {
        for k in 0..3 { exhaust_pop(&mut buf, ((t + k as f32 * 0.012) * SR_F) as usize, a, 0.7, &mut rng); }
    }
    let mut roar = vec![0.0f32; buf.len()];
    prop_roar(&mut roar, BLADE_PER_CRANK * crank, 0.6, &mut rng);
    for (v, r) in buf.iter_mut().zip(&roar) { *v += r; }
    buf.truncate(n);
    // fade out under the returning loop
    let f0 = ((SPLUTTER_CATCHES + 0.05) * SR_F) as usize;
    for (i, v) in buf.iter_mut().enumerate().skip(f0) {
        *v *= 1.0 - (i - f0) as f32 / (n - f0) as f32;
    }
    let scaled: Vec<f32> = buf.iter().map(|v| v * 11_000.0).collect();
    half_rate(&soft_limit_to_pcm16(&scaled, MIX_KNEE))
}

/// A flak shell bursting at altitude: the hard crack of the charge, a dull
/// "crump" of a body, and a short rattle of fragments; a smaller cousin of
/// explosion_sfx, heard from a little way off.
fn flak_burst_sfx() -> Vec<u8> {
    let b = Blast { secs: 0.45, boom_hz: 140.0, secondaries: 0, debris: 3, amp: 0.7, seed: 0xF1A4 };
    half_rate(&explosion_sfx(b))
}

/// A laser turret charging, 1.3 s: a whine climbing two octaves, its
/// tremolo quickening, so the ear counts down with the eye.
fn laser_charge_sfx() -> Vec<u8> {
    let n = (1.3 * SR_F) as usize;
    let mut ph = 0.0f32;
    let mut buf = vec![0.0f32; n];
    for (i, v) in buf.iter_mut().enumerate() {
        let k = i as f32 / n as f32;
        let t = i as f32 / SR_F;
        ph += (320.0 * 4f32.powf(k)) / SR_F;
        let trem = 0.6 + 0.4 * (2.0 * PI * (6.0 + 30.0 * k) * t).sin();
        let tone = (2.0 * PI * ph).sin() + 0.3 * (2.0 * PI * ph * 2.0).sin();
        *v = tone * trem * (0.25 + 0.75 * k) * (t / 0.02).min(1.0) * 9_000.0;
    }
    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// A laser turret's beam, 0.7 s: a harsh electric buzz with a crackle,
/// cutting off.
fn laser_fire_sfx() -> Vec<u8> {
    let n = (0.7 * SR_F) as usize;
    let mut rng = Rng(0x1A5E_F12E);
    let mut buf = vec![0.0f32; n];
    let mut lp = 0.0f32;
    for (i, v) in buf.iter_mut().enumerate() {
        let t = i as f32 / SR_F;
        let k = i as f32 / n as f32;
        let saw = |f: f32| 2.0 * (t * f).fract() - 1.0;
        let w = rng.next_f32() * 2.0 - 1.0;
        lp += (w - lp) * 0.3;
        let buzz = saw(110.0) * 0.6 + saw(221.0) * 0.4 + saw(330.5) * 0.2;
        let env = (t / 0.01).min(1.0) * (1.0 - k).powf(0.3);
        *v = (buzz * 0.8 + lp * 0.7) * env * 14_000.0;
    }
    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// The laser barrier's drone, looped and ridden by distance in the game:
/// the 110 Hz fundamental, its harmonics and the tremolo all complete whole
/// cycles in the buffer, so it loops without a seam.
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

/// The barrier's close-range buzz, crossfaded in as the beam nears the
/// player: an octave up with a fifth, and a hard 12.5 Hz throb. Every part
/// fits whole cycles into 400 ms, so it loops without a click.
fn barrier_hum2_sfx() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let n = ms_to_samples(400.0);
    let tau = 2.0 * std::f32::consts::PI;
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let tone = (tau * 220.0 * t).sin() + 0.5 * (tau * 330.0 * t).sin() + 0.25 * (tau * 110.0 * t).sin();
        let throb = 0.55 + 0.45 * (tau * 12.5 * t).sin().max(0.0);
        s.push(((tone * 0.6).tanh() * throb * 15_000.0) as i16);
    }
    encode_pcm16_mono(&s)
}

// ---------------------------------------------------------------------- //
// Japanese boss name banners
// //
// ---------------------------------------------------------------------- //
// blip's bitmap font covers only A-Z/0-9, so the katakana needed for the
// seven names were rasterised once, offline, from Noto Sans CJK JP Bold and
// baked in below; the build and the game never touch the font.

/// Katakana glyphs needed for the seven boss names, 10 wide x 12 tall.
const KATAKANA_CHARS: [char; 37] = [
    'ス', 'カ', 'ウ', 'ト', 'ボ', 'マ', 'ー', 'イ', 'ン', 'タ', 'セ', 'プ',
    'ガ', 'シ', 'ッ', 'ド', 'レ', 'ノ', 'バ', 'ル', 'ク', 'ザ', 'ゥ', 'ム',
    'キ', 'ャ', 'リ', 'ア', 'ペ', 'デ', 'ロ', 'ヤ',
    'ュ', 'ヒ', 'テ', 'コ', 'フ',
];
const KATAKANA_GLYPHS: [[u16; 12]; 37] = [
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
    [0x000, 0x000, 0x000, 0x000, 0x000, 0x0F8, 0x018, 0x018, 0x018, 0x1FC, 0x000, 0x000], // ュ
    [0x000, 0x000, 0x080, 0x080, 0x08C, 0x0F0, 0x080, 0x080, 0x080, 0x0FE, 0x07E, 0x000], // ヒ
    [0x000, 0x000, 0x078, 0x000, 0x1FE, 0x030, 0x030, 0x030, 0x060, 0x0C0, 0x000, 0x000], // テ
    [0x000, 0x000, 0x000, 0x0FC, 0x00C, 0x00C, 0x00C, 0x00C, 0x0FC, 0x000, 0x000, 0x000], // コ
    [0x000, 0x000, 0x000, 0x1FC, 0x00C, 0x00C, 0x018, 0x030, 0x060, 0x0C0, 0x000, 0x000], // フ
];

/// The seven boss aircraft's Japanese names, in katakana, in the order of
/// BOSS_SPECS.
const BOSS_NAMES_JA: [&[char]; 7] = [
    &['ド', 'ン', 'リ', 'ュ', 'ウ'],          // Ki-49 Donryu
    &['ヒ', 'リ', 'ュ', 'ウ'],                // Ki-67 Hiryu
    &['リ', 'ッ', 'コ', 'ウ'],                // G4M, the "Rikko"
    &['レ', 'ン', 'ザ', 'ン'],                // G8N Renzan
    &['タ', 'イ', 'テ', 'イ'],                // H8K flying boat, "Taitei"
    &['シ', 'ン', 'ザ', 'ン'],                // G5N Shinzan
    &['フ', 'ガ', 'ク'],                      // Fugaku
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
// Music: rock                                                             //
// ---------------------------------------------------------------------- //
// Raider's theme is a rock instrumental with its own voices (`power_chord`,
// `lead_guitar`, a rock kit), not the shared techno.rs kit, so it sounds like
// its own band. Guitars run a rig's chain: detuned saws (rhythm) or a
// saw/square blend (lead) driven into a tanh clipper (the amp), then a
// one-pole low-pass (the cabinet). Voices mix into one f32 buffer
// soft-limited once at the end.

/// Rock kick: tight and dry with a hard beater click that cuts through
/// distorted guitar; much less boom than `techno::kick`.
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
    let mut freq = freq;
    while freq > 660.0 { freq *= 0.5; } // an octave down until it sings, not screams
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
    let (freq, vol) = (tame(freq), vol * 0.6);
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
pub fn music() -> Vec<u8> {
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

    // a rock band, not a lullaby: the guitars keep their bite
    low_pass(&mut buf, 6500.0);
    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// The second loop in Raider's rotation — same band, harder and faster: a
/// drop-D thrash riff in D minor at 176 BPM, tremolo-picked chugs with a
/// chromatic breakdown accent, straight 8th-note kicks underneath, and a
/// shred solo that ends on a screaming pinch harmonic and a dive bomb.
/// No chorus, no let-up: one relentless riff either side of the solo, so
/// over a long level it answers the main theme's developed form with a
/// pure adrenaline hit.
pub fn music2() -> Vec<u8> {
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

    // a rock band, not a lullaby: the guitars keep their bite
    low_pass(&mut buf, 6500.0);
    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// A rumble (an explosion, an engine) encoded at half rate: little of it
/// lives above 8 kHz, and these are most of the game's download.
fn half_rate(pcm: &[i16]) -> Vec<u8> {
    let mut f: Vec<f32> = pcm.iter().map(|&v| v as f32).collect();
    low_pass(&mut f, 8000.0);
    let s: Vec<i16> = f.iter().map(|&v| v.clamp(-32767.0, 32767.0) as i16).collect();
    encode_pcm16_music(&s)
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
        ("sounds/barrier_hum2.wav",   barrier_hum2_sfx()),
        ("sounds/engine_start.wav",   engine_start_sfx()),
        ("sounds/enemy_gun.wav",      enemy_gun_sfx()),
        ("sounds/backfire.wav",       backfire_sfx()),
        ("sounds/engine_splutter.wav", engine_splutter_sfx()),
        ("sounds/flak_burst.wav",     flak_burst_sfx()),
        ("sounds/laser_charge.wav",   laser_charge_sfx()),
        ("sounds/laser_fire.wav",     laser_fire_sfx()),
        ("sounds/engine0.wav",        engine_loop(0.4, true)),
        ("sounds/engine1.wav",        engine_loop(0.65, false)),
        ("sounds/engine2.wav",        engine_loop(1.0, false)),
        ("sounds/engine3.wav",        engine_loop(1.2, false)),
        ("sounds/engine4.wav",        engine_loop(1.42, false)),
        ("sounds/burst1_1.wav", encode_pcm16_mono(&gun_burst(2, 0x5a0f82d5))),
        ("sounds/burst1_2.wav", encode_pcm16_mono(&gun_burst(2, 0x5a0f8698))),
        ("sounds/burst2_1.wav", encode_pcm16_mono(&gun_burst(3, 0x5a0f83d6))),
        ("sounds/burst2_2.wav", encode_pcm16_mono(&gun_burst(3, 0x5a0f8799))),
        ("sounds/burst3_1.wav", encode_pcm16_mono(&gun_burst(5, 0x5a0f84d7))),
        ("sounds/burst3_2.wav", encode_pcm16_mono(&gun_burst(5, 0x5a0f889a))),
        ("sounds/burst4_1.wav", encode_pcm16_mono(&gun_burst(6, 0x5a0f85d8))),
        ("sounds/burst4_2.wav", encode_pcm16_mono(&gun_burst(6, 0x5a0f899b))),
        ("sounds/burst5_1.wav", encode_pcm16_mono(&gun_burst(8, 0x5a0f86d9))),
        ("sounds/burst5_2.wav", encode_pcm16_mono(&gun_burst(8, 0x5a0f8a9c))),
        ("sounds/enemy_explode0.wav", half_rate(&explosion_sfx(ENEMY_BLASTS[0]))),
        ("sounds/enemy_explode1.wav", half_rate(&explosion_sfx(ENEMY_BLASTS[1]))),
        ("sounds/enemy_explode2.wav", half_rate(&explosion_sfx(ENEMY_BLASTS[2]))),
        ("sounds/enemy_explode3.wav", half_rate(&explosion_sfx(ENEMY_BLASTS[3]))),
        ("sounds/player_explode0.wav", half_rate(&explosion_sfx(PLAYER_BLASTS[0]))),
        ("sounds/player_explode1.wav", half_rate(&explosion_sfx(PLAYER_BLASTS[1]))),
        // A short, quieter crack for a non-lethal hit — reads as "took a
        // glancing blow" rather than player_explode's full "you're down".
        ("sounds/ricochet1.wav",      encode_pcm16_mono(&ricochet_sfx(3300.0, 0x51C0_C4E7))),
        ("sounds/ricochet2.wav",      encode_pcm16_mono(&ricochet_sfx(2700.0, 0x2B1E_77A3))),
        ("sounds/player_hit.wav",     encode_pcm16_mono(&hit_sfx())),
        ("sounds/boss_explode0.wav",  half_rate(&explosion_sfx(BOSS_BLASTS[0]))),
        ("sounds/boss_explode1.wav",  half_rate(&explosion_sfx(BOSS_BLASTS[1]))),
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
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two ends of the sea/sky gradient from draw_sea() in the game,
    /// duplicated because the game does not depend on this crate at runtime;
    /// if it changes there, this test should fail.
    const SKY_HORIZON: (u8, u8, u8) = (25, 61, 117);
    const SKY_NEAR: (u8, u8, u8) = (7, 28, 76);

    /// Relative luminance, as WCAG defines it.
    fn luminance((r, g, b): (u8, u8, u8)) -> f64 {
        fn channel(v: u8) -> f64 {
            let c = v as f64 / 255.0;
            if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        }
        0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
    }

    fn contrast(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
        let (la, lb) = (luminance(a), luminance(b));
        let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
        (hi + 0.05) / (lo + 0.05)
    }

    /// An enemy you cannot see is not a hard enemy: contact costs HP, so
    /// every kind's body and wings must clear 3:1 against the sky, the WCAG
    /// floor for a graphical object whose shape carries meaning.
    #[test]
    fn enemy_planes_are_visible_against_the_sky() {
        for kind in 0..3 {
            let (body, wing) = enemy_colors(kind);
            for (what, color) in [("body", body), ("wing", wing)] {
                for (where_, sky) in [("horizon", SKY_HORIZON), ("near", SKY_NEAR)] {
                    let c = contrast(color, sky);
                    assert!(
                        c >= 3.0,
                        "enemy kind {kind}'s {what} is {c:.2}:1 against the {where_} sky \
                         — below the 3:1 floor, so the plane reads as a smudge rather than \
                         a shape a player can weave between"
                    );
                }
            }
        }
    }

    /// The wings, the widest surface, are no darker than the fuselage.
    #[test]
    fn a_planes_wings_are_no_darker_than_its_fuselage() {
        for kind in 0..3 {
            let (body, wing) = enemy_colors(kind);
            assert!(
                luminance(wing) >= luminance(body),
                "enemy kind {kind}'s wings are darker than its body, which hides the widest \
                 part of the silhouette against the sky"
            );
        }
    }

    /// Each kind still has to be tellable from the others at a glance —
    /// the weaver dodges, the ace always drops a power-up, and a player
    /// reacts to which is which before reading anything else.
    #[test]
    fn the_three_enemy_kinds_stay_distinguishable_from_each_other() {
        let colors: Vec<_> = (0..3).map(|k| enemy_colors(k).0).collect();
        for a in 0..colors.len() {
            for b in (a + 1)..colors.len() {
                let (x, y) = (colors[a], colors[b]);
                let far_apart = (x.0 as i32 - y.0 as i32).abs()
                    + (x.1 as i32 - y.1 as i32).abs()
                    + (x.2 as i32 - y.2 as i32).abs();
                assert!(
                    far_apart >= 120,
                    "enemy kinds {a} and {b} are within {far_apart} of each other — lifting them \
                     out of the sky must not flatten them into each other"
                );
            }
        }
    }
}
