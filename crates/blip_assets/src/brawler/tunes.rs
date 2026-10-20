use crate::wav::{
    encode_pcm16_mono, encode_pcm16_music, low_pass, ms_to_samples, soft_limit_to_pcm16, tame, Rng,
    MIX_KNEE, SAMPLE_RATE,
};

const SRF: f32 = SAMPLE_RATE as f32;

/// Plucked string using Karplus–Strong averaging; upper harmonics decay first.
pub(super) fn pluck(buf: &mut [f32], off: usize, freq: f32, ms: f32, gain: f32, rng: &mut Rng) {
    let freq = tame(freq);
    let period = (SAMPLE_RATE as f32 / freq).max(2.0) as usize;
    let n = ms_to_samples(ms).min(buf.len().saturating_sub(off));
    let mut line: Vec<f32> = (0..period).map(|_| rng.next_f32() * 2.0 - 1.0).collect();
    // A little brightness taken off the initial burst: a koto is
    // plucked with a plectrum, not struck with a hammer.
    for i in 1..period {
        line[i] = line[i] * 0.6 + line[i - 1] * 0.4;
    }
    // A long (low) string has few trips round the line to lose its hiss in,
    // so it starts rounder: a bass note should be a note, not a thwack.
    for _ in 0..period / 150 {
        let first = line[0];
        for i in 0..period - 1 {
            line[i] = (line[i] + line[i + 1]) * 0.5;
        }
        line[period - 1] = (line[period - 1] + first) * 0.5;
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
pub(super) fn taiko(buf: &mut [f32], off: usize, freq: f32, gain: f32, rng: &mut Rng) {
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
pub(super) fn rim(buf: &mut [f32], off: usize, gain: f32, rng: &mut Rng) {
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

/// Five-note scales in semitones from the root. Hirajoshi's minor second is
/// what makes it sound Japanese; yo is the bright folk scale without one;
/// in is the dark one; the minor pentatonic is the one a rock band plays.
pub(super) const HIRAJOSHI: [f32; 5] = [0.0, 1.0, 5.0, 7.0, 8.0];
pub(super) const YO: [f32; 5] = [0.0, 2.0, 5.0, 7.0, 9.0];
const IN: [f32; 5] = [0.0, 1.0, 5.0, 7.0, 10.0];
const MINOR: [f32; 5] = [0.0, 3.0, 5.0, 7.0, 10.0];
/// The major pentatonic, open and heroic; and the minor with its fifth
/// flattened, which is the blues.
const MAJOR: [f32; 5] = [0.0, 2.0, 4.0, 7.0, 9.0];
const BLUES: [f32; 5] = [0.0, 3.0, 5.0, 6.0, 10.0];

pub(super) fn degree(scale: &[f32; 5], root: f32, step: i32) -> f32 {
    let oct = step.div_euclid(5);
    let d = scale[step.rem_euclid(5) as usize];
    root * 2.0f32.powf((d + 12.0 * oct as f32) / 12.0)
}

/// A shakuhachi: a breathy tone that swells in, wavers once it is held and
/// dies away, with the air of the player's breath round it.
pub(super) fn flute(buf: &mut [f32], off: usize, freq: f32, ms: f32, gain: f32, rng: &mut Rng) {
    let n = ms_to_samples(ms).min(buf.len().saturating_sub(off));
    let (mut ph, mut air) = (0.0f32, 0.0f32);
    for i in 0..n {
        let t = i as f32 / SRF;
        let k = i as f32 / n as f32;
        // Blown from just under the note, as the player's lip finds it.
        let scoop = 1.0 - 0.03 * (-t / 0.05).exp();
        let vib = 1.0 + 0.007 * (t / 0.35).min(1.0) * (2.0 * std::f32::consts::PI * 5.2 * t).sin();
        ph += 2.0 * std::f32::consts::PI * freq * scoop * vib / SRF;
        air += ((rng.next_f32() * 2.0 - 1.0) - air) * 0.25;
        let e = (t / 0.09).min(1.0) * (1.0 - k).powf(0.7);
        let tone = ph.sin() + 0.28 * (2.0 * ph).sin() + 0.10 * (3.0 * ph).sin();
        buf[off + i] += (tone + air * 0.5) * e * gain * 2600.0;
    }
}

/// A struck glass: one clear partial and its octave, gone in half a second.
/// Not tamed like the koto, so it can sit above the tune.
fn chime(buf: &mut [f32], off: usize, freq: f32, gain: f32) {
    let n = ms_to_samples(520.0).min(buf.len().saturating_sub(off));
    for i in 0..n {
        let t = i as f32 / SRF;
        let ph = 2.0 * std::f32::consts::PI * freq * t;
        buf[off + i] += (ph.sin() + 0.3 * (2.0 * ph).sin()) * (-t * 7.0).exp() * gain * 2400.0;
    }
}

/// What a theme is made of. Phrases are four bars of sixteenths in scale
/// degrees, so a tune cannot leave the scale; `bass` is a degree a bar.
struct Theme<'a> {
    bpm: f32,
    /// The lowest note of the tune. Degrees 0..=5 must stay under A4, where
    /// `tame` starts folding notes down an octave and bends the melody.
    root: f32,
    scale: &'a [f32; 5],
    phrases: &'a [&'a [i32]],
    bass: &'a [&'a [i32]],
    /// Which phrase each four-bar slot plays; phrase 3 is the hushed one.
    order: &'a [usize],
    /// 0..1: how hard the drums push.
    drive: f32,
    /// What the sticks do between the drum beats.
    feel: Feel,
}

/// A theme's percussion besides the big drum.
#[derive(Copy, Clone)]
enum Feel {
    Rim,
    March,
    Push,
    Chime,
}

/// A theme sequenced by `order` rather than looped: eight slots of four bars
/// outlast the round, so no seam is heard.
impl Theme<'_> {
    fn pcm(&self) -> Vec<i16> {
        let Theme {
            bpm,
            root,
            scale,
            phrases,
            bass,
            order,
            drive,
            feel,
        } = *self;
        let step = (SAMPLE_RATE as f32 * 60.0 / bpm / 4.0) as usize; // 16ths
        let bars = 4 * order.len();
        let steps = bars * 16;
        // Room past the end for the last notes to ring out in, which is
        // then folded back onto the top — see `wrap_tail`.
        let body = step * steps;
        let n = body + ms_to_samples(1400.0);
        let mut buf = vec![0.0f32; n];
        let mut rng = Rng(0x9E3B);
        let beat_ms = 60.0 * 1000.0 / bpm;
        for s in 0..steps {
            let off = s * step;
            let slot = s / 64; // which four-bar phrase
            let melody = phrases[order[slot] % phrases.len()];
            let line = bass[order[slot] % bass.len()];
            let within = s % 64; // step inside the phrase
            let bar = within / 16;
            let b = s % 16;
            // Each phrase has a quiet bar, and a whole phrase now and then
            // is quiet throughout — a theme with no let-up in it has
            // nothing to come back from.
            let hush = order[slot] == 3;
            let lull = bar == 2 || hush;
            let last_bar = bar == 3;
            let last_slot = slot + 1 == order.len();

            // The fight under the tune, as hard as the stage drives: a low drum
            // on every beat, a bass string plucked on every eighth (the octave
            // on the off-beats), and sticks on the sixteenths at full drive.
            let ground = line[bar % line.len()];
            if drive >= 0.4 && !hush {
                if b % 4 == 0 && b != 0 {
                    taiko(&mut buf, off, 84.0, 0.42 * drive, &mut rng);
                }
                if b % 2 == 0 && !(lull && b >= 8) {
                    let f = degree(scale, root, ground) * if b % 4 == 2 { 0.5 } else { 0.25 };
                    pluck(&mut buf, off, f, beat_ms * 0.45, 0.5 * drive, &mut rng);
                }
                if drive > 0.8 && b % 2 == 1 && !lull {
                    rim(&mut buf, off, 0.14, &mut rng);
                }
            }

            // The pulse: a heavy taiko on one, an answer past the middle.
            if b == 0 {
                taiko(
                    &mut buf,
                    off,
                    96.0,
                    if bar == 0 && !hush { 1.0 } else { 0.8 },
                    &mut rng,
                );
            }
            if b == 8 && !lull {
                taiko(&mut buf, off, 104.0, 0.7, &mut rng);
            }
            if b == 12 && bar % 2 == 1 && !hush {
                taiko(&mut buf, off, 122.0, 0.6, &mut rng);
            }
            // Every phrase fills into the next one; the last one fills
            // hardest, because that is the seam back to the top.
            if last_bar && matches!(b, 10 | 12 | 14) {
                let g = if last_slot { 0.7 } else { 0.5 } + 0.15 * drive;
                taiko(&mut buf, off, 92.0 + (b as f32 - 10.0) * 14.0, g, &mut rng);
            }
            match feel {
                // A parade ground: the rim on every beat, and a roll into each
                // new phrase.
                Feel::March => {
                    if b % 4 == 0 && !hush {
                        rim(&mut buf, off, 0.36, &mut rng);
                    }
                    if b % 4 == 2 {
                        rim(&mut buf, off, 0.5, &mut rng);
                    }
                    if last_bar && b >= 12 {
                        rim(&mut buf, off, 0.22 + 0.06 * (b - 12) as f32, &mut rng);
                    }
                }
                // The city: every rim pushed a sixteenth ahead of where it is
                // expected, and a soft drum in the gap.
                Feel::Push => {
                    if b % 4 == 3 && !(lull && b == 11) {
                        rim(&mut buf, off, 0.46, &mut rng);
                    }
                    if b == 6 && !lull {
                        taiko(&mut buf, off, 132.0, 0.4, &mut rng);
                    }
                }
                // Ice: no sticks at all, and a chime answering the drum from
                // high above the tune.
                Feel::Chime => {
                    if b % 8 == 4 {
                        let d = line[bar % line.len()];
                        chime(
                            &mut buf,
                            off,
                            degree(scale, root, d + 4) * 2.0,
                            if lull { 0.35 } else { 0.5 },
                        );
                    }
                }
                // Rim on the off-beats, denser on a driving stage.
                Feel::Rim => {
                    if b % 4 == 2 && !(lull && b == 10) {
                        rim(&mut buf, off, 0.5, &mut rng);
                    }
                    if b % 8 == 7 && !hush {
                        rim(&mut buf, off, 0.32, &mut rng);
                    }
                    if drive > 0.5 && b % 4 == 3 && !lull {
                        rim(&mut buf, off, 0.18, &mut rng);
                    }
                }
            }

            // The koto line.
            let note = melody[within % melody.len()];
            if note > -90 {
                let gain = if lull { 0.62 } else { 0.85 };
                pluck(
                    &mut buf,
                    off,
                    degree(scale, root, note),
                    beat_ms * 1.6,
                    gain,
                    &mut rng,
                );
                // Doubled an octave up on the last bar of a phrase, so the
                // tune has a top to it once every four bars.
                if last_bar && b == 0 {
                    pluck(
                        &mut buf,
                        off,
                        degree(scale, root, note + 5),
                        beat_ms * 1.2,
                        0.4,
                        &mut rng,
                    );
                }
                // A second koto answers the tune three steps behind it, on the
                // phrases after the first: the tune comes back with company.
                if order[slot] != 0 && !lull && s + 3 < steps {
                    pluck(
                        &mut buf,
                        off + 3 * step,
                        degree(scale, root, note),
                        beat_ms * 0.9,
                        0.22,
                        &mut rng,
                    );
                }
            }

            // Each phrase opens on a strummed chord: three strings a moment apart.
            if within == 0 && !hush {
                let d = line[0];
                for (j, up) in [0, 2, 4].into_iter().enumerate() {
                    pluck(
                        &mut buf,
                        off + j * ms_to_samples(14.0),
                        degree(scale, root, d + up),
                        beat_ms * 2.4,
                        0.3,
                        &mut rng,
                    );
                }
            }

            // The flute answers where the koto leaves room: over the quiet bar
            // and the cadence, and all through a hushed phrase. It holds a note
            // of the bar's chord an octave above the tune.
            let d = line[bar % line.len()];
            if b == 0 && (lull || last_bar) {
                let up = if last_bar { 0 } else { 2 };
                flute(
                    &mut buf,
                    off,
                    degree(scale, root, d + up) * 2.0,
                    beat_ms * 3.4,
                    if hush { 0.8 } else { 0.6 },
                    &mut rng,
                );
            }

            // The bass walks a note a bar.
            if b == 0 {
                pluck(
                    &mut buf,
                    off,
                    degree(scale, root, d) * 0.5,
                    beat_ms * 3.6,
                    0.55,
                    &mut rng,
                );
            }
            if b == 10 && !lull {
                pluck(
                    &mut buf,
                    off,
                    degree(scale, root, d + 2) * 0.5,
                    beat_ms * 1.2,
                    0.3,
                    &mut rng,
                );
            }
        }
        wrap_tail(&mut buf, body);
        // Fight music: brighter than the house warmth, and as loud as the other
        // games' (the koto alone sat 12 dB under them).
        low_pass(&mut buf[..body], 6500.0);
        for v in buf[..body].iter_mut() {
            *v *= 2.6;
        }
        soft_limit_to_pcm16(&buf[..body], MIX_KNEE)
    }
}

/// Fold what rings out past the end of the loop onto its start, so tails
/// carry over the loop point instead of the music stopping dead.
fn wrap_tail(buf: &mut [f32], body: usize) {
    for i in body..buf.len() {
        buf[i - body] += buf[i];
    }
}

/// Device-synthesized themes by stage: docks, temple, select, air base, bath house, river, fortress, rooftop.
pub fn theme_wav(which: usize) -> Vec<u8> {
    // The docks, the temple and the select screen are on hirajoshi and differ
    // in pace and density; each later stage has a scale of its own.
    // Four-bar phrases, sequenced; -99 is a rest.
    const R: i32 = -99;

    // -- the docks ------------------------------------------------------
    let d_a = [
        0, R, R, 2, 1, R, 0, R, 3, R, 2, R, 0, R, R, 1, 4, R, 3, R, 2, R, R, 0, 1, R, 0, R, R, 2,
        R, R, 0, R, R, 2, 1, R, 0, R, 3, R, 4, R, 3, R, R, 2, 4, R, 5, R, 4, R, 3, R, 2, R, R, 1,
        0, R, R, R,
    ];
    let d_b = [
        2, R, 3, R, 4, R, R, 3, 2, R, R, 4, 5, R, R, R, 4, R, 3, R, 2, R, 1, R, 0, R, R, 2, 1, R,
        R, R, 3, R, R, 4, 5, R, 4, R, 3, R, R, 2, 1, R, 2, R, 0, R, 1, R, 2, R, R, 3, 2, R, 1, R,
        0, R, R, R,
    ];
    let d_c = [
        5, R, R, R, 4, R, R, R, 3, R, R, R, 2, R, R, R, 4, R, R, 3, R, R, 2, R, R, R, 1, R, R, R,
        R, R, 0, R, R, 1, 2, R, R, 3, 4, R, R, R, 3, R, R, R, 2, R, R, R, 1, R, R, R, 0, R, R, R,
        R, R, R, R,
    ];
    let d_d = [
        0, R, R, R, R, R, 2, R, R, R, 1, R, R, R, R, R, 0, R, R, R, R, R, R, R, 2, R, R, R, R, R,
        R, R, 3, R, R, R, R, R, 2, R, R, R, R, R, 1, R, R, R, 0, R, R, R, R, R, R, R, R, R, R, R,
        R, R, R, R,
    ];

    // -- the temple -----------------------------------------------------
    let t_a = [
        3, R, R, R, 2, R, 4, R, 3, R, R, 1, 0, R, R, R, 1, R, 2, R, R, 3, R, 2, 0, R, R, R, R, R,
        1, R, 3, R, R, R, 4, R, R, 3, 5, R, R, 4, 3, R, R, R, 2, R, R, 1, R, R, 2, R, 0, R, R, R,
        R, R, R, R,
    ];
    let t_b = [
        5, R, R, 4, R, R, 3, R, R, R, 4, R, 5, R, R, R, 4, R, R, R, 3, R, R, 2, R, R, 3, R, R, R,
        R, R, 2, R, R, 3, 4, R, R, 5, R, R, 4, R, 3, R, R, R, 1, R, R, R, 2, R, R, R, 0, R, R, R,
        R, R, R, R,
    ];
    let t_c = [
        0, R, R, R, 1, R, R, R, 2, R, R, R, 3, R, R, R, 2, R, R, 1, R, R, 0, R, R, R, R, R, 1, R,
        R, R, 4, R, R, R, 3, R, R, R, 2, R, R, 1, R, R, R, R, 0, R, R, R, R, R, R, R, R, R, R, R,
        R, R, R, R,
    ];
    let t_d = [
        3, R, R, R, R, R, R, R, 2, R, R, R, R, R, R, R, 1, R, R, R, R, R, R, R, 0, R, R, R, R, R,
        R, R, 2, R, R, R, R, R, R, R, 1, R, R, R, R, R, R, R, 0, R, R, R, R, R, R, R, R, R, R, R,
        R, R, R, R,
    ];

    // -- the select screen ----------------------------------------------
    let s_a = [
        0, R, R, R, 3, R, 2, R, R, R, 1, R, 0, R, R, R, 2, R, R, R, 1, R, R, 0, R, R, R, R, 3, R,
        R, R, 4, R, R, 3, R, R, 2, R, R, R, 3, R, 4, R, R, R, 2, R, R, R, 0, R, R, 1, R, R, R, R,
        R, R, R, R,
    ];
    let s_b = [
        3, R, R, R, 4, R, R, R, 5, R, R, 4, 3, R, R, R, 2, R, R, 3, R, R, 4, R, R, R, 3, R, R, R,
        R, R, 1, R, R, R, 2, R, R, 1, 0, R, R, R, R, R, 2, R, 1, R, R, R, R, R, 0, R, R, R, R, R,
        R, R, R, R,
    ];

    // -- the air base: a march on the minor pentatonic -------------------
    let a_a = [
        0, R, 0, R, 2, R, 3, R, 4, R, R, 3, R, R, 2, R, 0, R, 0, R, 2, R, 3, R, 5, R, R, 4, R, R,
        3, R, 4, R, 4, R, 5, R, 6, R, 5, R, R, 4, R, R, 3, R, 2, R, 3, R, 4, R, R, 2, 0, R, R, R,
        R, R, R, R,
    ];
    let a_b = [
        5, R, R, R, 4, R, 5, R, 6, R, R, R, 5, R, 4, R, 3, R, R, R, 2, R, 3, R, 4, R, R, R, R, R,
        2, R, 5, R, R, R, 4, R, 5, R, 6, R, R, 5, R, R, 4, R, 3, R, 2, R, 3, R, 4, R, 5, R, R, R,
        R, R, R, R,
    ];
    let a_c = [
        0, R, R, 2, R, R, 3, R, 4, R, R, R, 3, R, 2, R, 0, R, R, 2, R, R, 3, R, 2, R, R, R, R, R,
        R, R, 3, R, R, 4, R, R, 5, R, 6, R, R, R, 5, R, 4, R, 3, R, R, 2, R, R, 0, R, 0, R, R, R,
        R, R, R, R,
    ];
    let a_d = [
        0, R, R, R, R, R, R, R, 3, R, R, R, R, R, R, R, 2, R, R, R, R, R, R, R, 0, R, R, R, R, R,
        R, R, 4, R, R, R, R, R, R, R, 3, R, R, R, R, R, R, R, 2, R, R, R, R, R, 3, R, 0, R, R, R,
        R, R, R, R,
    ];

    // -- the bath house: bright and easy, on yo --------------------------
    let h_a = [
        2, R, R, 3, 4, R, R, R, 3, R, 2, R, 0, R, R, R, 2, R, R, 3, 4, R, 5, R, 4, R, R, R, R, R,
        3, R, 2, R, R, 3, 4, R, R, R, 5, R, 4, R, 3, R, R, 2, 3, R, 2, R, 1, R, R, 2, 0, R, R, R,
        R, R, R, R,
    ];
    let h_b = [
        5, R, R, R, 4, R, 3, R, 4, R, R, R, 2, R, R, R, 3, R, R, R, 2, R, 1, R, 2, R, R, R, R, R,
        R, R, 5, R, R, R, 4, R, 3, R, 4, R, R, 5, 6, R, R, R, 5, R, 4, R, 3, R, 2, R, 0, R, R, R,
        R, R, R, R,
    ];
    let h_c = [
        0, R, 2, R, 3, R, 2, R, 0, R, R, R, R, R, 1, R, 0, R, 2, R, 3, R, 4, R, 3, R, R, R, R, R,
        R, R, 4, R, 3, R, 2, R, 3, R, 4, R, R, R, 5, R, R, R, 4, R, R, 3, R, R, 2, R, 0, R, R, R,
        R, R, R, R,
    ];
    let h_d = [
        2, R, R, R, R, R, R, R, 3, R, R, R, R, R, R, R, 4, R, R, R, R, R, R, R, 2, R, R, R, R, R,
        R, R, 3, R, R, R, R, R, R, R, 1, R, R, R, R, R, R, R, 0, R, R, R, R, R, R, R, R, R, R, R,
        R, R, R, R,
    ];

    // -- the river village: slow and heavy, on in ------------------------
    let v_a = [
        0, R, R, R, 0, R, 1, R, 2, R, R, R, R, R, R, R, 3, R, R, R, 2, R, 1, R, 0, R, R, R, R, R,
        R, R, 0, R, R, R, 0, R, 1, R, 2, R, R, R, 3, R, R, R, 4, R, R, 3, R, R, 2, R, 0, R, R, R,
        R, R, R, R,
    ];
    let v_b = [
        4, R, R, R, 3, R, R, R, 4, R, R, R, 5, R, R, R, 4, R, R, 3, R, R, 2, R, 3, R, R, R, R, R,
        R, R, 2, R, R, R, 3, R, R, R, 4, R, R, R, 6, R, R, R, 5, R, R, 4, R, R, 3, R, 2, R, R, R,
        R, R, R, R,
    ];
    let v_c = [
        0, R, 2, R, R, R, 3, R, R, R, 2, R, 0, R, R, R, 0, R, 2, R, R, R, 3, R, R, R, 4, R, 3, R,
        R, R, 5, R, R, R, 4, R, R, R, 3, R, R, R, 2, R, R, R, 1, R, R, R, 0, R, R, R, 0, R, R, R,
        R, R, R, R,
    ];
    let v_d = [
        0, R, R, R, R, R, R, R, R, R, R, R, 1, R, R, R, 2, R, R, R, R, R, R, R, R, R, R, R, R, R,
        R, R, 3, R, R, R, R, R, R, R, 2, R, R, R, 1, R, R, R, 0, R, R, R, R, R, R, R, R, R, R, R,
        R, R, R, R,
    ];

    // -- the crystal fortress: slow and open, on the major pentatonic ------
    let f_a = [
        0, R, R, R, 3, R, R, R, R, R, R, R, 4, R, 3, R, 5, R, R, R, R, R, R, R, 4, R, R, R, 3, R,
        R, R, 0, R, R, R, 3, R, R, R, R, R, R, R, 4, R, 5, R, 6, R, R, R, R, R, 5, R, 5, R, R, R,
        R, R, R, R,
    ];
    let f_b = [
        5, R, R, R, 4, R, R, R, 3, R, R, R, R, R, 2, R, 3, R, R, R, R, R, R, R, 2, R, R, R, 0, R,
        R, R, 5, R, R, R, 4, R, R, R, 3, R, R, R, 4, R, 5, R, 6, R, R, R, R, R, R, R, 5, R, R, R,
        R, R, R, R,
    ];
    let f_c = [
        2, R, R, R, 3, R, R, R, 4, R, R, R, R, R, R, R, 3, R, R, R, 4, R, R, R, 5, R, R, R, R, R,
        R, R, 4, R, R, R, 5, R, R, R, 6, R, R, R, R, R, 5, R, 4, R, R, R, 3, R, R, R, 0, R, R, R,
        R, R, R, R,
    ];
    let f_d = [
        0, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R, 3, R, R, R, R, R, R, R, R, R, R, R, R, R,
        R, R, 2, R, R, R, R, R, R, R, 4, R, R, R, R, R, R, R, 0, R, R, R, R, R, R, R, R, R, R, R,
        R, R, R, R,
    ];

    // -- the rooftop: pushed off the beat, on the blues -------------------
    let q_a = [
        0, R, R, 0, R, R, 1, R, 2, R, 3, R, 2, R, 1, R, 0, R, R, R, R, R, 4, R, R, R, 3, R, 2, R,
        R, R, 0, R, R, 0, R, R, 1, R, 2, R, 3, R, 4, R, 5, R, 4, R, R, 3, R, R, 2, R, 0, R, R, R,
        R, R, R, R,
    ];
    let q_b = [
        5, R, R, R, 4, R, R, 5, R, R, 4, R, 3, R, 2, R, 4, R, R, R, 3, R, R, 4, R, R, 3, R, 2, R,
        1, R, 5, R, R, R, 4, R, R, 5, R, R, 6, R, 5, R, 4, R, 3, R, 2, R, 1, R, 2, R, 0, R, R, R,
        R, R, R, R,
    ];
    let q_c = [
        2, R, 2, R, R, R, 3, R, 2, R, R, R, 1, R, 0, R, 2, R, 2, R, R, R, 3, R, 4, R, R, R, R, R,
        R, R, 5, R, 5, R, R, R, 4, R, 3, R, R, R, 2, R, 1, R, 0, R, R, 1, R, R, 2, R, 0, R, R, R,
        R, R, R, R,
    ];
    let q_d = [
        0, R, R, R, R, R, R, R, 2, R, R, R, R, R, 1, R, 0, R, R, R, R, R, R, R, R, R, R, R, R, R,
        R, R, 3, R, R, R, R, R, R, R, 2, R, R, R, R, R, 1, R, 0, R, R, R, R, R, R, R, R, R, R, R,
        R, R, R, R,
    ];

    // A note a bar under each, so the ground moves.
    let b0 = [0, 0, 3, 2];
    let b1 = [2, 3, 4, 2];
    let b2 = [0, 4, 3, 0];
    let b3 = [0, 0, 0, 0];

    // Eight slots of four bars: a rondo, so the tune keeps coming back
    // without the piece repeating. Slot 3 is the hushed one.
    let order = [0usize, 1, 0, 3, 2, 1, 0, 1];
    let bass: [&[i32]; 4] = [&b0, &b1, &b2, &b3];
    let stage = |bpm, root, scale, phrases: [&[i32]; 4], drive, feel| {
        Theme {
            bpm,
            root,
            scale,
            phrases: &phrases,
            bass: &bass,
            order: &order,
            drive,
            feel,
        }
        .pcm()
    };
    encode_pcm16_music(&match which {
        0 => stage(
            148.0,
            196.00,
            &HIRAJOSHI,
            [&d_a, &d_b, &d_c, &d_d],
            1.0,
            Feel::Rim,
        ),
        1 => stage(
            128.0,
            220.00,
            &HIRAJOSHI,
            [&t_a, &t_b, &t_c, &t_d],
            0.6,
            Feel::Rim,
        ),
        3 => stage(
            156.0,
            174.61,
            &MINOR,
            [&a_a, &a_b, &a_c, &a_d],
            1.0,
            Feel::March,
        ),
        4 => stage(136.0, 196.00, &YO, [&h_a, &h_b, &h_c, &h_d], 0.7, Feel::Rim),
        5 => stage(124.0, 164.81, &IN, [&v_a, &v_b, &v_c, &v_d], 0.8, Feel::Rim),
        6 => stage(
            120.0,
            196.00,
            &MAJOR,
            [&f_a, &f_b, &f_c, &f_d],
            0.5,
            Feel::Chime,
        ),
        7 => stage(
            142.0,
            185.00,
            &BLUES,
            [&q_a, &q_b, &q_c, &q_d],
            0.9,
            Feel::Push,
        ),
        _ => Theme {
            bpm: 118.0,
            root: 207.65,
            scale: &HIRAJOSHI,
            phrases: &[&s_a, &s_b],
            bass: &[&b2, &b0],
            order: &[0, 1, 0, 1],
            drive: 0.0,
            feel: Feel::Rim,
        }
        .pcm(),
    })
}

/// Stage ambience loops: 0 crowd, 1 wind, 2 crowd and crickets, 3 crowd and city hum.
pub fn ambience_wav(kind: usize) -> Vec<u8> {
    let body = ms_to_samples(4000.0);
    let fade = ms_to_samples(400.0);
    let mut buf = vec![0.0f32; body + fade];
    let mut rng = Rng(0xA3B1 + kind as u32);
    let (mut lp, mut hp, mut bp, mut low) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    for (i, out) in buf.iter_mut().enumerate() {
        let t = i as f32 / body as f32;
        let turn = |cycles: f32, at: f32| (std::f32::consts::TAU * (cycles * t + at)).sin();
        let noise = rng.next_f32() * 2.0 - 1.0;
        *out = if kind != 1 {
            // Voices: the middle of the band, swelling here and there.
            lp += (noise - lp) * 0.30;
            hp += (lp - hp) * 0.030;
            let crowd = (lp - hp)
                * (1.0 + 0.25 * turn(3.0, 0.0) + 0.18 * turn(7.0, 0.3) + 0.12 * turn(11.0, 0.7))
                * 5200.0;
            let secs = t * 4.0;
            crowd
                + match kind {
                    // Crickets: a high note chopped into chirps, three to a burst.
                    2 => {
                        let burst = (secs * 2.5).fract() < 0.45;
                        let chop = (secs * 31.0).fract() < 0.5;
                        if burst && chop {
                            (std::f32::consts::TAU * 3900.0 * secs).sin() * 420.0
                        } else {
                            0.0
                        }
                    }
                    // The city: a low drone that beats slowly against itself.
                    3 => {
                        ((std::f32::consts::TAU * 58.0 * secs).sin()
                            + (std::f32::consts::TAU * 61.25 * secs).sin())
                            * 800.0
                    }
                    _ => 0.0,
                }
        } else {
            // Wind: a narrow band whose pitch rises and falls with the gusts.
            let gust = 0.5 + 0.5 * turn(1.0, 0.0) * turn(3.0, 0.2);
            let f = 2.0 * (std::f32::consts::PI * (260.0 + 420.0 * gust) / SRF).sin();
            let high = noise - low - 0.35 * bp;
            bp += f * high;
            low += f * bp;
            bp * (0.35 + 0.65 * gust) * 9000.0
        };
    }
    // The tail is laid over the head, each fading as the other comes in, so
    // the loop has no seam.
    for i in 0..fade {
        let k = i as f32 / fade as f32;
        buf[i] = buf[i] * k + buf[body + i] * (1.0 - k);
    }
    buf.truncate(body);
    encode_pcm16_music(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// The crowd, built at startup: 2.5 s of PCM is a quarter of a megabyte for a
/// few lines of filtered noise.
pub fn crowd_wav(long: bool) -> Vec<u8> {
    let s = if long {
        super::crowd(1600.0, 1.0, 0xFEED)
    } else {
        super::crowd(900.0, 0.85, 0xC0DE)
    };
    encode_pcm16_mono(&s)
}
