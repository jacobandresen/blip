//! Brawler assets: sounds only. The fighters are drawn by the game from the
//! same numbers as their hitboxes, so there is no sprite sheet to disagree
//! with the rules.

use crate::techno::{tame, warm, Rng, MIX_KNEE};
use crate::wav::{encode_pcm16_mono, encode_pcm16_music, env, ms_to_samples,
    soft_limit_to_pcm16, SAMPLE_RATE};
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

/// A crowd, all at once: a wide band of noise that swells and falls, with a
/// slow wobble so it reads as many voices, and no pitch (a pitched cheer is a
/// chord).
fn crowd(ms: f32, vol: f32, seed: u32) -> Vec<i16> {
    let n = ms_to_samples(ms);
    let mut buf = vec![0.0f32; n];
    let mut rng = Rng(seed);
    let mut lp = 0.0f32;
    let mut hp = 0.0f32;
    for (i, out) in buf.iter_mut().enumerate() {
        let t = i as f32 / n as f32;
        // Up fast, down slow: a room reacting, then settling.
        let e = if t < 0.16 { t / 0.16 } else { (1.0 - (t - 0.16) / 0.84).powf(1.7) };
        let noise = rng.next_f32() * 2.0 - 1.0;
        // Open enough to stay a cheer. Filtered down to a narrow band
        // it came out as a rumble — measurably lower in pitch than a
        // punch, which is not what a room full of people sounds like.
        lp += (noise - lp) * 0.78;
        hp += (lp - hp) * 0.020;
        let voices = lp - hp;
        // Several slow wobbles at once — individual voices coming and
        // going out of the mass.
        let w = 1.0
            + 0.22 * (t * 37.0).sin()
            + 0.15 * (t * 23.0 + 1.7).sin()
            + 0.10 * (t * 61.0 + 0.4).sin();
        *out = voices * w * e * vol * 5200.0;
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// Boots hitting boards. Shorter and woodier than a body landing on
/// them — mostly the box the floor is, with a click of heel on top.
fn land(seed: u32) -> Vec<i16> {
    let ms = 130.0;
    let n = ms_to_samples(ms);
    let mut buf = vec![0.0f32; n];
    let mut rng = Rng(seed);
    let mut phase = 0.0f32;
    for (i, out) in buf.iter_mut().enumerate() {
        let t = i as f32 / n as f32;
        let e = env(i, n, ms_to_samples(0.6), ms_to_samples(ms * 0.9));
        let f = 132.0 * (1.0 - 0.36 * t);
        phase += 2.0 * std::f32::consts::PI * f / SAMPLE_RATE as f32;
        let click = (rng.next_f32() * 2.0 - 1.0) * (1.0 - t).powf(9.0);
        *out = (phase.sin() * 0.82 + click * 0.5) * e * 8000.0;
    }
    soft_limit_to_pcm16(&buf, MIX_KNEE)
}

/// Cloth snapping taut, the sound of a gi when a leg goes out fast: the
/// whoosh is air moving, this is speed. Noise through a band opening upward
/// as the leg extends, a one-sample difference to strip the bottom, an
/// envelope almost all attack, then a quieter second crack as the leg comes
/// back.
fn gi_snap(seed: u32) -> Vec<i16> {
    let ms = 150.0;
    let n = ms_to_samples(ms);
    let mut buf = vec![0.0f32; n];
    let mut rng = Rng(seed);
    let per_ms = ms_to_samples(1.0) as f32;
    // Each crack gets its own filter sweep; sharing one made the second the
    // louder.
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

/// A plucked string: koto, or shamisen struck harder. Karplus-Strong: a
/// period-long delay line of noise read round and round, averaging
/// neighbours, so the highs die first and a bright pluck decays to a pure
/// tone, like a real string.
fn pluck(buf: &mut [f32], off: usize, freq: f32, ms: f32, gain: f32, rng: &mut Rng) {
    let freq = tame(freq);
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

/// A taiko: a low body tone falling in pitch as the head relaxes, a noise
/// crack for the stick, and a room-length tail.
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

/// The hirajoshi scale in semitones from the root: root, minor second,
/// fourth, fifth, minor sixth. The minor second is what makes it sound
/// Japanese.
const HIRAJOSHI: [f32; 5] = [0.0, 1.0, 5.0, 7.0, 8.0];

fn degree(root: f32, step: i32) -> f32 {
    let oct = step.div_euclid(5);
    let d = HIRAJOSHI[step.rem_euclid(5) as usize];
    root * 2.0f32.powf((d + 12.0 * oct as f32) / 12.0)
}

/// A theme built from four-bar phrases, sequenced by `order` rather than
/// looped: eight slots of four bars outlast the round, so no seam is heard.
/// Phrases are scale degrees, so a tune cannot leave the scale.
fn theme(bpm: f32, root: f32, phrases: &[&[i32]], bass: &[&[i32]], order: &[usize],
         drive: f32) -> Vec<i16> {
    let step = (SAMPLE_RATE as f32 * 60.0 / bpm / 4.0) as usize; // 16ths
    let bars = 4 * order.len();
    let steps = bars * 16;
    // Room past the end for the last notes to ring out in, which is
    // then folded back onto the top — see `wrap_tail`.
    let body = step * steps;
    let n = body + ms_to_samples(900.0);
    let mut buf = vec![0.0f32; n];
    let mut rng = Rng(0x9E3B);
    let beat_ms = 60.0 * 1000.0 / bpm;
    for s in 0..steps {
        let off = s * step;
        let slot = s / 64;                 // which four-bar phrase
        let melody = phrases[order[slot] % phrases.len()];
        let line = bass[order[slot] % bass.len()];
        let within = s % 64;               // step inside the phrase
        let bar = within / 16;
        let b = s % 16;
        // Each phrase has a quiet bar, and a whole phrase now and then
        // is quiet throughout — a theme with no let-up in it has
        // nothing to come back from.
        let hush = order[slot] == 3;
        let lull = bar == 2 || hush;
        let last_bar = bar == 3;
        let last_slot = slot + 1 == order.len();

        // The pulse: a heavy taiko on one, an answer past the middle.
        if b == 0 {
            taiko(&mut buf, off, 96.0, if bar == 0 && !hush { 1.0 } else { 0.8 }, &mut rng);
        }
        if b == 8 && !lull { taiko(&mut buf, off, 104.0, 0.7, &mut rng); }
        if b == 12 && bar % 2 == 1 && !hush { taiko(&mut buf, off, 122.0, 0.6, &mut rng); }
        // Every phrase fills into the next one; the last one fills
        // hardest, because that is the seam back to the top.
        if last_bar && matches!(b, 10 | 12 | 14) {
            let g = if last_slot { 0.7 } else { 0.5 } + 0.15 * drive;
            taiko(&mut buf, off, 92.0 + (b as f32 - 10.0) * 14.0, g, &mut rng);
        }
        // Rim on the off-beats, denser on a driving stage.
        if b % 4 == 2 && !(lull && b == 10) { rim(&mut buf, off, 0.5, &mut rng); }
        if b % 8 == 7 && !hush { rim(&mut buf, off, 0.32, &mut rng); }
        if drive > 0.5 && b % 4 == 3 && !lull { rim(&mut buf, off, 0.18, &mut rng); }

        // The koto line.
        let note = melody[within % melody.len()];
        if note > -90 {
            let gain = if lull { 0.62 } else { 0.85 };
            pluck(&mut buf, off, degree(root, note), beat_ms * 1.6, gain, &mut rng);
            // Doubled an octave up on the last bar of a phrase, so the
            // tune has a top to it once every four bars.
            if last_bar && b == 0 {
                pluck(&mut buf, off, degree(root, note + 5), beat_ms * 1.2, 0.4, &mut rng);
            }
        }

        // The bass walks a note a bar.
        if b == 0 {
            let d = line[bar % line.len()];
            pluck(&mut buf, off, degree(root, d) * 0.5, beat_ms * 3.6, 0.55, &mut rng);
        }
        if b == 10 && !lull {
            let d = line[bar % line.len()];
            pluck(&mut buf, off, degree(root, d + 2) * 0.5, beat_ms * 1.2, 0.3, &mut rng);
        }
    }
    wrap_tail(&mut buf, body);
    warm(&mut buf[..body]);
    soft_limit_to_pcm16(&buf[..body], MIX_KNEE)
}

/// Fold what rings out past the end of the loop onto its start, so tails
/// carry over the loop point instead of the music stopping dead.
fn wrap_tail(buf: &mut [f32], body: usize) {
    for i in body..buf.len() {
        buf[i - body] += buf[i];
    }
}


/// The three themes, synthesised at startup rather than baked in: a
/// round-length theme is a megabyte of PCM, times three, while the code is a
/// couple of hundred lines. 0 and 1 are the stages, 2 the select screen.
pub fn theme_wav(which: usize) -> Vec<u8> {
    // All on hirajoshi, differing in pace, register and density: the docks
    // fast and hard on the drum, the temple slower and higher with room
    // between notes, the select screen between them and out of the way.
    // Four-bar phrases, sequenced; -99 is a rest.
    const R: i32 = -99;

    // -- the docks ------------------------------------------------------
    let d_a = [0, R, R, 2, 1, R, 0, R, 3, R, 2, R, 0, R, R, 1,
               4, R, 3, R, 2, R, R, 0, 1, R, 0, R, R, 2, R, R,
               0, R, R, 2, 1, R, 0, R, 3, R, 4, R, 3, R, R, 2,
               4, R, 5, R, 4, R, 3, R, 2, R, R, 1, 0, R, R, R];
    let d_b = [2, R, 3, R, 4, R, R, 3, 2, R, R, 4, 5, R, R, R,
               4, R, 3, R, 2, R, 1, R, 0, R, R, 2, 1, R, R, R,
               3, R, R, 4, 5, R, 4, R, 3, R, R, 2, 1, R, 2, R,
               0, R, 1, R, 2, R, R, 3, 2, R, 1, R, 0, R, R, R];
    let d_c = [5, R, R, R, 4, R, R, R, 3, R, R, R, 2, R, R, R,
               4, R, R, 3, R, R, 2, R, R, R, 1, R, R, R, R, R,
               0, R, R, 1, 2, R, R, 3, 4, R, R, R, 3, R, R, R,
               2, R, R, R, 1, R, R, R, 0, R, R, R, R, R, R, R];
    let d_d = [0, R, R, R, R, R, 2, R, R, R, 1, R, R, R, R, R,
               0, R, R, R, R, R, R, R, 2, R, R, R, R, R, R, R,
               3, R, R, R, R, R, 2, R, R, R, R, R, 1, R, R, R,
               0, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R];

    // -- the temple -----------------------------------------------------
    let t_a = [3, R, R, R, 2, R, 4, R, 3, R, R, 1, 0, R, R, R,
               1, R, 2, R, R, 3, R, 2, 0, R, R, R, R, R, 1, R,
               3, R, R, R, 4, R, R, 3, 5, R, R, 4, 3, R, R, R,
               2, R, R, 1, R, R, 2, R, 0, R, R, R, R, R, R, R];
    let t_b = [5, R, R, 4, R, R, 3, R, R, R, 4, R, 5, R, R, R,
               4, R, R, R, 3, R, R, 2, R, R, 3, R, R, R, R, R,
               2, R, R, 3, 4, R, R, 5, R, R, 4, R, 3, R, R, R,
               1, R, R, R, 2, R, R, R, 0, R, R, R, R, R, R, R];
    let t_c = [0, R, R, R, 1, R, R, R, 2, R, R, R, 3, R, R, R,
               2, R, R, 1, R, R, 0, R, R, R, R, R, 1, R, R, R,
               4, R, R, R, 3, R, R, R, 2, R, R, 1, R, R, R, R,
               0, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R];
    let t_d = [3, R, R, R, R, R, R, R, 2, R, R, R, R, R, R, R,
               1, R, R, R, R, R, R, R, 0, R, R, R, R, R, R, R,
               2, R, R, R, R, R, R, R, 1, R, R, R, R, R, R, R,
               0, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R];

    // -- the select screen ----------------------------------------------
    let s_a = [0, R, R, R, 3, R, 2, R, R, R, 1, R, 0, R, R, R,
               2, R, R, R, 1, R, R, 0, R, R, R, R, 3, R, R, R,
               4, R, R, 3, R, R, 2, R, R, R, 3, R, 4, R, R, R,
               2, R, R, R, 0, R, R, 1, R, R, R, R, R, R, R, R];
    let s_b = [3, R, R, R, 4, R, R, R, 5, R, R, 4, 3, R, R, R,
               2, R, R, 3, R, R, 4, R, R, R, 3, R, R, R, R, R,
               1, R, R, R, 2, R, R, 1, 0, R, R, R, R, R, 2, R,
               1, R, R, R, R, R, 0, R, R, R, R, R, R, R, R, R];

    // A note a bar under each, so the ground moves.
    let b0 = [0, 0, 3, 2];
    let b1 = [2, 3, 4, 2];
    let b2 = [0, 4, 3, 0];
    let b3 = [0, 0, 0, 0];

    // Eight slots of four bars: a rondo, so the tune keeps coming back
    // without the piece repeating. Slot 3 is the hushed one.
    let order = [0usize, 1, 0, 3, 2, 1, 0, 1];
    match which {
        0 => encode_pcm16_music(&theme(132.0, 293.66,
            &[&d_a, &d_b, &d_c, &d_d], &[&b0, &b1, &b2, &b3], &order, 1.0)),
        1 => encode_pcm16_music(&theme(108.0, 220.0,
            &[&t_a, &t_b, &t_c, &t_d], &[&b0, &b1, &b2, &b3], &order, 0.2)),
        _ => encode_pcm16_music(&theme(118.0, 246.94,
            &[&s_a, &s_b], &[&b2, &b0], &[0, 1, 0, 1], 0.0)),
    }
}

/// The crowd, built at startup: 2.5 s of PCM is a quarter of a megabyte for a
/// few lines of filtered noise.
pub fn crowd_wav(long: bool) -> Vec<u8> {
    let s = if long { crowd(1600.0, 1.0, 0xFEED) } else { crowd(900.0, 0.85, 0xC0DE) };
    encode_pcm16_mono(&s)
}

pub fn generate() -> Vec<Asset> {
    vec![
        // Three light hits climbing in pitch, one per combo step, so a string
        // sounds like it is going somewhere. The pitched share climbs too: a
        // light hit is three quarters hiss, and pitch alone moved 400 Hz
        // unheard.
        ("sounds/hit_light.wav",  encode_pcm16_mono(&impact(110.0, 0.30, 520.0, 0.80, 0x11))),
        ("sounds/hit_light2.wav", encode_pcm16_mono(&impact(102.0, 0.46, 700.0, 0.85, 0x1b))),
        ("sounds/hit_light3.wav", encode_pcm16_mono(&impact(94.0, 0.62, 940.0, 0.90, 0x2f))),
        ("sounds/hit_heavy.wav",  encode_pcm16_mono(&impact(220.0, 0.6, 300.0, 1.0, 0x22))),
        // A knockdown is the biggest thing that happens in a round and
        // it shared its sound with an ordinary heavy hit. Lower, longer
        // and almost all thump: the sound of someone hitting boards.
        ("sounds/crunch.wav",    encode_pcm16_mono(&impact(340.0, 0.78, 190.0, 1.0, 0x5a))),
        ("sounds/land.wav",      encode_pcm16_mono(&land(0x71))),
        ("sounds/whoosh.wav",    encode_pcm16_mono(&whoosh(150.0, 0.7, 0x33))),
        ("sounds/gi.wav",        encode_pcm16_mono(&gi_snap(0x6C1D))),
        ("sounds/block.wav",     encode_pcm16_mono(&block_sfx())),
        ("sounds/bell.wav",      encode_pcm16_mono(&bell())),
        ("sounds/ko.wav",        encode_pcm16_mono(&ko())),
        ("sounds/projectile.wav", encode_pcm16_mono(&projectile())),
    ]
}
