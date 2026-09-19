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

use crate::techno::{bass_note, clap, hat, kick, Rng, MIX_KNEE};
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
fn music(bpm: f32, bass_hook: &[f32]) -> Vec<i16> {
    let step = (SAMPLE_RATE as f32 * 60.0 / bpm / 4.0) as usize; // 16ths
    let bars = 2;
    let steps = bars * 16;
    let n = step * steps + ms_to_samples(400.0);
    let mut buf = vec![0.0f32; n];
    let mut rng = Rng(0x9E3B);
    for s in 0..steps {
        let off = s * step;
        if s % 4 == 0 { kick(&mut buf, off, 1.0); }
        if s % 8 == 4 { clap(&mut buf, off, &mut rng, 0.7); }
        if s % 2 == 1 { hat(&mut buf, off, &mut rng, 0.35); }
        let note = bass_hook[s % bass_hook.len()];
        if note > 0.0 {
            bass_note(&mut buf, off, note, 60.0 * 1000.0 / bpm / 2.0, 0.55);
        }
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

pub fn generate() -> Vec<Asset> {
    // Two stages, two loops — the only thing that makes a location sound
    // like somewhere rather than a backdrop.
    let dock = [55.0, 0.0, 55.0, 0.0, 73.42, 0.0, 55.0, 0.0,
                65.41, 0.0, 65.41, 0.0, 49.0, 0.0, 49.0, 0.0];
    let temple = [43.65, 0.0, 0.0, 43.65, 58.27, 0.0, 43.65, 0.0,
                  65.41, 0.0, 58.27, 0.0, 49.0, 0.0, 0.0, 49.0];
    vec![
        ("sounds/hit_light.wav", encode_pcm16_mono(&impact(110.0, 0.25, 520.0, 0.8, 0x11))),
        ("sounds/hit_heavy.wav", encode_pcm16_mono(&impact(220.0, 0.6, 300.0, 1.0, 0x22))),
        ("sounds/whoosh.wav",    encode_pcm16_mono(&whoosh(150.0, 0.7, 0x33))),
        ("sounds/block.wav",     encode_pcm16_mono(&block_sfx())),
        ("sounds/bell.wav",      encode_pcm16_mono(&bell())),
        ("sounds/ko.wav",        encode_pcm16_mono(&ko())),
        ("sounds/projectile.wav", encode_pcm16_mono(&projectile())),
        ("sounds/music_dock.wav", encode_pcm16_mono(&music(146.0, &dock))),
        ("sounds/music_temple.wav", encode_pcm16_mono(&music(132.0, &temple))),
    ]
}
