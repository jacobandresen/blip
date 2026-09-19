//! Brawler assets — sounds only.
//!
//! The fighters are not sprites. They are drawn as posed figures out of
//! rectangles by the game itself, one pose per state, because a fighting
//! game lives or dies on whether you can read *what your opponent is
//! doing right now* — and a pose assembled from the same parts every
//! frame is guaranteed to agree with the hitboxes it is being judged
//! against. A sprite sheet would have been a second source of truth for
//! the same question, drawn by hand, for a game whose whole point is
//! that the picture and the rules match.
//!
//! So what is left here is what cannot be drawn: the noises a fight
//! makes.

use crate::techno::{Rng, MIX_KNEE};
use crate::wav::{encode_pcm16_mono, env, ms_to_samples, soft_limit_to_pcm16, SAMPLE_RATE};
use crate::Asset;

/// A short noise burst through a falling band — the body of every impact
/// sound here. `tone` sets how much pitched thump is mixed under the
/// noise: a jab is almost all hiss, a knockdown is mostly thump.
fn impact(ms: f32, tone: f32, freq: f32, vol: f32, seed: u32) -> Vec<i16> {
    let n = ms_to_samples(ms);
    let mut buf = vec![0.0f32; n];
    let mut rng = Rng(seed);
    let mut phase = 0.0f32;
    for (i, out) in buf.iter_mut().enumerate() {
        let t = i as f32 / n as f32;
        let e = env(i, n, ms_to_samples(1.0), ms_to_samples(ms * 0.8));
        // The pitch drops through the hit; that fall is most of what
        // makes it read as a strike rather than a click.
        let f = freq * (1.0 - 0.55 * t);
        phase += 2.0 * std::f32::consts::PI * f / SAMPLE_RATE as f32;
        let noise = rng.next_f32() * 2.0 - 1.0;
        let thump = phase.sin();
        *out = (noise * (1.0 - tone) + thump * tone) * e * vol * 9000.0;
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// The swing itself, heard before anything connects. Filtered noise
/// rising and falling — the tell that an attack has started, which is
/// the sound a defender is actually listening for.
fn whoosh(ms: f32, vol: f32, seed: u32) -> Vec<i16> {
    let n = ms_to_samples(ms);
    let mut buf = vec![0.0f32; n];
    let mut rng = Rng(seed);
    let mut lp = 0.0f32;
    for (i, out) in buf.iter_mut().enumerate() {
        let t = i as f32 / n as f32;
        let e = (t * std::f32::consts::PI).sin();
        let noise = rng.next_f32() * 2.0 - 1.0;
        // A one-pole low pass that opens as the swing passes: the sweep
        // is what gives it direction.
        let k = 0.04 + 0.5 * t;
        lp += (noise - lp) * k;
        *out = lp * e * vol * 7000.0;
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// Cloth snapping taut — the sound a gi makes when a leg goes out fast.
///
/// The `whoosh` above is air moving: low, soft-edged, and it says "an
/// attack has started". This says something narrower and more useful —
/// *that was quick*. A kick is the fastest thing a body does, and the
/// only audible evidence of the speed is the fabric, which does not
/// swish; it cracks, because the trouser leg runs out of slack all at
/// once and stops.
///
/// So: noise through a band that opens upward as the leg extends, run
/// through a one-sample difference to strip the bottom out of it, under
/// an envelope that is almost all attack. Then a second, quieter crack
/// a beat later — cloth snaps twice, once on the way out and once when
/// the leg is pulled back.
fn gi_snap(seed: u32) -> Vec<i16> {
    let ms = 150.0;
    let n = ms_to_samples(ms);
    let mut buf = vec![0.0f32; n];
    let mut rng = Rng(seed);
    let per_ms = ms_to_samples(1.0) as f32;
    // Two cracks: the leg going out, and the smaller one as it is
    // pulled back. Each gets its own filter, because the brightness
    // belongs to the crack and not to the clock — sharing one sweep
    // across both made the *second* snap the loud one, which is the
    // opposite of what cloth does.
    for (at_ms, gain) in [(0.0f32, 1.0f32), (62.0, 0.38)] {
        let from = ms_to_samples(at_ms);
        let (mut lp, mut prev) = (0.0f32, 0.0f32);
        for i in from..n {
            let d = (i - from) as f32 / per_ms;
            let noise = rng.next_f32() * 2.0 - 1.0;
            // The band opens as the slack runs out.
            let k = (0.20 + 0.030 * d).min(0.95);
            lp += (noise - lp) * k;
            let bright = lp - prev;
            prev = lp;
            // Almost all attack: up in two milliseconds, gone in fifty.
            let e = (d / 2.5).min(1.0) * (-d * 0.062).exp();
            buf[i] += bright * e * gain * 19000.0;
        }
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// Blocked: a hard, short, bright clack with no low end. Deliberately
/// unlike a hit — the difference between "I blocked that" and "I ate
/// that" has to be audible without looking at the health bar.
fn block_sfx() -> Vec<i16> {
    let n = ms_to_samples(90.0);
    let mut buf = vec![0.0f32; n];
    let mut rng = Rng(0x51A7);
    let mut hp = 0.0f32;
    let mut prev = 0.0f32;
    for (i, out) in buf.iter_mut().enumerate() {
        let e = env(i, n, ms_to_samples(0.5), ms_to_samples(70.0));
        let noise = rng.next_f32() * 2.0 - 1.0;
        hp = 0.85 * (hp + noise - prev); // one-pole high pass
        prev = noise;
        *out = hp * e * 8000.0;
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// The bell that opens a round.
fn bell() -> Vec<i16> {
    let n = ms_to_samples(900.0);
    let mut buf = vec![0.0f32; n];
    // Three inharmonic partials: a struck bell, not a note.
    for (freq, amp) in [(880.0f32, 1.0f32), (1320.0, 0.55), (2093.0, 0.3)] {
        for (i, out) in buf.iter_mut().enumerate() {
            let t = i as f32 / SAMPLE_RATE as f32;
            let decay = (-3.2 * t).exp();
            let phase = 2.0 * std::f32::consts::PI * freq * t;
            *out += phase.sin() * decay * amp * 5000.0;
        }
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// Knockout: the impact, plus a long falling tone under it.
fn ko() -> Vec<i16> {
    let n = ms_to_samples(1100.0);
    let mut buf = vec![0.0f32; n];
    let mut rng = Rng(0xF157);
    let mut phase = 0.0f32;
    for (i, out) in buf.iter_mut().enumerate() {
        let t = i as f32 / n as f32;
        let e = (-2.6 * t).exp();
        let f = 220.0 * (1.0 - 0.75 * t);
        phase += 2.0 * std::f32::consts::PI * f / SAMPLE_RATE as f32;
        let noise = (rng.next_f32() * 2.0 - 1.0) * (1.0 - t).max(0.0) * 0.45;
        *out = (phase.sin() * 0.8 + noise) * e * 11000.0;
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// A fireball leaving the hand: noise and a rising tone.
fn projectile() -> Vec<i16> {
    let n = ms_to_samples(420.0);
    let mut buf = vec![0.0f32; n];
    let mut rng = Rng(0x2B01);
    let mut phase = 0.0f32;
    for (i, out) in buf.iter_mut().enumerate() {
        let t = i as f32 / n as f32;
        let e = env(i, n, ms_to_samples(6.0), ms_to_samples(300.0));
        let f = 180.0 + 520.0 * t;
        phase += 2.0 * std::f32::consts::PI * f / SAMPLE_RATE as f32;
        let noise = (rng.next_f32() * 2.0 - 1.0) * 0.5;
        *out = (phase.sin() * 0.7 + noise) * e * 8000.0;
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// Two bars of driving loop to fight over. Deliberately plain: the
/// sounds that matter in a fight are the ones the other player is
/// making, and music that competes with them costs the player
/// information.
/// A plucked string: koto, or shamisen if you hit it harder.
///
/// Karplus-Strong — fill a delay line one period long with noise, then
/// read it round and round, averaging each sample with its neighbour as
/// it goes. The averaging is a low-pass, so the high partials die first
/// and the note decays from a bright pluck into a pure tone, which is
/// what a struck string actually does. Two dozen lines of arithmetic
/// and it sounds like catgut rather than like a synthesiser, which no
/// amount of enveloping a sine wave ever will.
fn pluck(buf: &mut [f32], off: usize, freq: f32, ms: f32, gain: f32, rng: &mut Rng) {
    let period = (SAMPLE_RATE as f32 / freq).max(2.0) as usize;
    let n = ms_to_samples(ms).min(buf.len().saturating_sub(off));
    let mut line: Vec<f32> = (0..period).map(|_| rng.next_f32() * 2.0 - 1.0).collect();
    // A little brightness taken off the initial burst: a koto is
    // plucked with a plectrum, not struck with a hammer.
    for i in 1..period {
        line[i] = line[i] * 0.6 + line[i - 1] * 0.4;
    }
    for i in 0..n {
        let idx = i % period;
        let nxt = (i + 1) % period;
        let out = line[idx];
        line[idx] = (line[idx] + line[nxt]) * 0.5 * 0.996;
        // A slow overall fade so notes do not pile up into a drone.
        let e = (-(i as f32) / n as f32 * 3.2).exp();
        buf[off + i] += out * e * gain * 7600.0;
    }
}

/// A taiko: a big skin under tension.
///
/// Low body tone whose pitch falls as the head relaxes, a crack of
/// noise for the stick, and a long enough tail to sound like a drum in
/// a room rather than a click.
fn taiko(buf: &mut [f32], off: usize, freq: f32, gain: f32, rng: &mut Rng) {
    let n = ms_to_samples(460.0).min(buf.len().saturating_sub(off));
    let mut phase = 0.0f32;
    for i in 0..n {
        let t = i as f32 / n as f32;
        let e = (-t * 5.5).exp();
        let f = freq * (1.0 - 0.42 * t);
        phase += 2.0 * std::f32::consts::PI * f / SAMPLE_RATE as f32;
        let skin = (rng.next_f32() * 2.0 - 1.0) * (-t * 46.0).exp() * 0.55;
        buf[off + i] += (phase.sin() * 0.95 + skin) * e * gain * 5400.0;
    }
}

/// The rim — a dry wooden click, the sound of the stick on the hoop.
fn rim(buf: &mut [f32], off: usize, gain: f32, rng: &mut Rng) {
    let n = ms_to_samples(45.0).min(buf.len().saturating_sub(off));
    let mut prev = 0.0f32;
    for i in 0..n {
        let t = i as f32 / n as f32;
        let x = rng.next_f32() * 2.0 - 1.0;
        // A one-sample difference: all click, no body.
        let hp = x - prev;
        prev = x;
        buf[off + i] += hp * (-t * 14.0).exp() * gain * 3400.0;
    }
}

/// The *hirajōshi* scale, in semitones from the root: root, minor
/// second, fourth, fifth, minor sixth.
///
/// The minor second is the whole thing. It is the interval that makes a
/// pentatonic run sound Japanese rather than merely folk — take it out
/// and what is left could be Scottish. Everything here is built on it.
const HIRAJOSHI: [f32; 5] = [0.0, 1.0, 5.0, 7.0, 8.0];

fn degree(root: f32, step: i32) -> f32 {
    let oct = step.div_euclid(5);
    let d = HIRAJOSHI[step.rem_euclid(5) as usize];
    root * 2.0f32.powf((d + 12.0 * oct as f32) / 12.0)
}

/// Two bars of a stage's loop.
///
/// Taiko on the pulse, rim on the off-beats, a koto line over the top
/// and a low string holding the root underneath. The melody is written
/// as scale degrees rather than frequencies, so it cannot leave the
/// scale by accident — which is how a tune stops sounding like the
/// place it is supposed to be.
fn music(bpm: f32, root: f32, melody: &[i32]) -> Vec<i16> {
    let step = (SAMPLE_RATE as f32 * 60.0 / bpm / 4.0) as usize; // 16ths
    let steps = 2 * 16;
    let n = step * steps + ms_to_samples(600.0);
    let mut buf = vec![0.0f32; n];
    let mut rng = Rng(0x9E3B);
    let beat_ms = 60.0 * 1000.0 / bpm;
    for s in 0..steps {
        let off = s * step;
        // The pulse: a heavy taiko on one and three, a lighter one
        // answering off the middle of the bar.
        if s % 8 == 0 { taiko(&mut buf, off, 96.0, 1.0, &mut rng); }
        if s % 16 == 12 { taiko(&mut buf, off, 122.0, 0.6, &mut rng); }
        if s % 4 == 2 { rim(&mut buf, off, 0.5, &mut rng); }
        if s % 8 == 7 { rim(&mut buf, off, 0.32, &mut rng); }
        // The koto line.
        let note = melody[s % melody.len()];
        if note > -90 {
            pluck(&mut buf, off, degree(root, note), beat_ms * 1.6, 0.85, &mut rng);
        }
        // A low string holding the root under the bar.
        if s % 16 == 0 {
            pluck(&mut buf, off, degree(root, 0) * 0.5, beat_ms * 4.0, 0.5, &mut rng);
        }
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}


pub fn generate() -> Vec<Asset> {
    // Two stages, two loops — the only thing that makes a location
    // sound like somewhere rather than a backdrop. Both sit on
    // hirajōshi; the docks take it fast and low on the drum, the temple
    // slower and higher up the scale, which is most of the difference
    // between a working waterfront and somewhere people are quiet.
    //
    // -99 is a rest.
    const R: i32 = -99;
    let dock = [0, R, R, 2, 1, R, 0, R, 3, R, 2, R, 0, R, R, 1,
                4, R, 3, R, 2, R, R, 0, 1, R, 0, R, R, 2, R, R];
    let temple = [3, R, R, R, 2, R, 4, R, 3, R, R, 1, 0, R, R, R,
                  1, R, 2, R, R, 3, R, 2, 0, R, R, R, R, R, 1, R];
    vec![
        ("sounds/hit_light.wav", encode_pcm16_mono(&impact(110.0, 0.25, 520.0, 0.8, 0x11))),
        ("sounds/hit_heavy.wav", encode_pcm16_mono(&impact(220.0, 0.6, 300.0, 1.0, 0x22))),
        ("sounds/whoosh.wav",    encode_pcm16_mono(&whoosh(150.0, 0.7, 0x33))),
        ("sounds/gi.wav",        encode_pcm16_mono(&gi_snap(0x6C1D))),
        ("sounds/block.wav",     encode_pcm16_mono(&block_sfx())),
        ("sounds/bell.wav",      encode_pcm16_mono(&bell())),
        ("sounds/ko.wav",        encode_pcm16_mono(&ko())),
        ("sounds/projectile.wav", encode_pcm16_mono(&projectile())),
        ("sounds/music_dock.wav", encode_pcm16_mono(&music(132.0, 293.66, &dock))),
        ("sounds/music_temple.wav", encode_pcm16_mono(&music(108.0, 220.0, &temple))),
    ]
}
