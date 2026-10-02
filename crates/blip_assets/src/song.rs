//! A song written as data: a melody of eighth notes over two chords a bar,
//! played by a game's own instrument (see `cosy::Voice`) over a bass line,
//! a quiet arpeggio and soft drums. Games synthesise their songs on the
//! device at load, so music adds code, not megabytes, to the download.
//!
//! ```ignore
//! let tune = Song { bpm: 120.0, melody: &[[72, H, 76, H, 79, H, R, R]], chords: &[[C, G]], ..Song::DEFAULT };
//! let wav = tune.render();
//! ```

use std::f32::consts::PI;

use crate::cosy::{note, Voice, SR};
use crate::wav::{bright, warm, Rng, MIX_KNEE};
use crate::wav::{encode_pcm16_music, soft_limit_to_pcm16};

/// Hold the previous note for another eighth.
pub const H: i32 = -2;
/// Rest for an eighth.
pub const R: i32 = -1;

/// A chord: its root as a MIDI note, and the intervals above it.
pub type Chord = (i32, [i32; 3]);

pub const fn major(root: i32) -> Chord { (root, [0, 4, 7]) }
pub const fn minor(root: i32) -> Chord { (root, [0, 3, 7]) }

/// How the drums play under each bar.
#[derive(Clone, Copy, PartialEq)]
pub enum Groove {
    /// No drums at all.
    None,
    /// Kick on one and three, snare on two and four, a light hat between.
    Backbeat,
    /// A kick on every beat and a snare on two and four: for the tense songs.
    FourFloor,
    /// Only a soft kick on one and a rim on three: for calm songs.
    Brush,
    /// A hard kick on every beat, a cracking snare on two and four and
    /// sixteenth hats: for the action songs.
    Drive,
}

pub struct Song<'a> {
    pub bpm: f32,
    /// One row per bar, eight eighth notes each: MIDI notes, `H` or `R`.
    pub melody: &'a [[i32; 8]],
    /// Two chords per bar, one per half. Must be as long as `melody`.
    pub chords: &'a [[Chord; 2]],
    /// The instrument that carries the tune, and how loud.
    pub lead: Voice,
    pub lead_vol: f32,
    /// Semitones added to every melody note (the harmony follows).
    pub transpose: i32,
    /// From halfway through, a second voice doubles the tune this many
    /// semitones away (a third or an octave below reads as harmony).
    pub harmony: Option<(Voice, i32)>,
    /// The bass per half bar: four eighths, as semitones above the chord's
    /// root (`R` rests).
    pub bass: [i32; 4],
    pub bass_voice: Voice,
    /// Sixteenth-note chord arpeggio under the tune.
    pub arp: bool,
    pub groove: Groove,
    /// Snare fill into every fourth bar.
    pub fills: bool,
    /// Dotted-eighth echo level, 0 for none.
    pub echo: f32,
    /// An action song: the bass is struck on every sixteenth and the mix
    /// keeps its edge instead of being warmed.
    pub action: bool,
    pub seed: u32,
}

impl Song<'_> {
    /// Sensible defaults: override the fields a song cares about.
    pub const DEFAULT: Song<'static> = Song {
        bpm: 120.0,
        melody: &[],
        chords: &[],
        lead: Voice::Lead,
        lead_vol: 0.30,
        transpose: 0,
        harmony: None,
        bass: [0, 12, 7, 12],
        bass_voice: Voice::Bass,
        arp: true,
        groove: Groove::Backbeat,
        fills: true,
        echo: 0.2,
        action: false,
        seed: 0x5eed,
    };

    /// Seconds per loop.
    pub fn duration(&self) -> f32 { self.melody.len() as f32 * 4.0 * 60.0 / self.bpm }

    /// The whole loop as a WAV, seamless when played looped.
    pub fn render(&self) -> Vec<u8> {
        assert_eq!(self.melody.len(), self.chords.len(), "one chord pair per melody bar");
        let eighth = (60.0 / self.bpm / 2.0 * SR) as usize;
        let bar = eighth * 8;
        let bars = self.melody.len();
        let total = bar * bars;
        let mut buf = vec![0.0f32; total + SR as usize];
        let mut rng = Rng(self.seed | 1);
        for (b, line) in self.melody.iter().enumerate() {
            let b0 = b * bar;
            self.play_line(&mut buf, b0, eighth, line, b >= bars / 2);
            for half in 0..2 {
                let (root, iv) = self.chords[b][half];
                let h0 = b0 + half * 4 * eighth;
                for (s, &off) in self.bass.iter().enumerate() {
                    if off == R { continue; }
                    if self.action {
                        // struck twice, the second softer: a gallop
                        note(&mut buf, h0 + s * eighth, eighth * 2 / 5, root + off, 0.24, self.bass_voice);
                        note(&mut buf, h0 + s * eighth + eighth / 2, eighth * 2 / 5, root + off, 0.17, self.bass_voice);
                    } else {
                        note(&mut buf, h0 + s * eighth, eighth - eighth / 4, root + off, 0.34, self.bass_voice);
                    }
                }
                if self.arp {
                    for s in 0..8 {
                        let n = root + 12 + iv[s % 3] + if s % 6 >= 3 { 12 } else { 0 };
                        note(&mut buf, h0 + s * eighth / 2, eighth / 2, n, 0.05, Voice::Arp);
                    }
                }
            }
            self.play_drums(&mut buf, b, b0, eighth, &mut rng);
        }
        if self.echo > 0.0 {
            let d = eighth * 3 / 2;
            for i in (d..buf.len()).rev() { buf[i] += buf[i - d] * self.echo; }
        }
        // Fold the tail back onto the start so the loop has no seam.
        let tail = buf.split_off(total);
        for (i, v) in tail.iter().enumerate() { buf[i % total] += v; }
        if self.action { bright(&mut buf); } else { warm(&mut buf); }
        let scaled: Vec<f32> = buf.iter().map(|v| v * 21_000.0).collect();
        encode_pcm16_music(&soft_limit_to_pcm16(&scaled, MIX_KNEE))
    }

    fn play_line(&self, buf: &mut [f32], b0: usize, eighth: usize, line: &[i32; 8], second_half: bool) {
        let mut k = 0;
        while k < 8 {
            if line[k] < 0 { k += 1; continue; }
            let n = line[k] + self.transpose;
            let mut len = 1;
            while k + len < 8 && line[k + len] == H { len += 1; }
            let at = b0 + k * eighth;
            note(buf, at, len * eighth - eighth / 6, n, self.lead_vol, self.lead);
            if let (true, Some((v, shift))) = (second_half, self.harmony) {
                note(buf, at, len * eighth, n + shift, self.lead_vol * 0.35, v);
            }
            k += len;
        }
    }

    fn play_drums(&self, buf: &mut [f32], b: usize, b0: usize, eighth: usize, rng: &mut Rng) {
        for beat in 0..4 {
            let t0 = b0 + beat * 2 * eighth;
            match self.groove {
                Groove::None => {}
                Groove::Backbeat => {
                    if beat % 2 == 0 { kick(buf, t0, 0.5); } else { snare(buf, t0, rng, 0.28); }
                    hat(buf, t0 + eighth, rng, 0.07);
                }
                Groove::FourFloor => {
                    kick(buf, t0, 0.5);
                    if beat % 2 == 1 { snare(buf, t0, rng, 0.26); }
                    hat(buf, t0 + eighth, rng, 0.09);
                }
                Groove::Brush => {
                    if beat == 0 { kick(buf, t0, 0.4); }
                    if beat == 2 { snare(buf, t0, rng, 0.12); }
                    hat(buf, t0 + eighth, rng, 0.04);
                }
                Groove::Drive => {
                    punch(buf, t0, 0.62);
                    if beat % 2 == 1 { crack(buf, t0, rng, 0.44); }
                    // a pickup kick into three
                    if beat == 1 { punch(buf, t0 + eighth + eighth / 2, 0.4); }
                    for s in 0..4 {
                        hat(buf, t0 + s * eighth / 2, rng, if s == 2 { 0.24 } else { 0.12 });
                    }
                }
            }
        }
        if self.fills && self.groove != Groove::None && b % 4 == 3 {
            for s in 0..4 { snare(buf, b0 + 6 * eighth + s * eighth / 2, rng, 0.12 + 0.04 * s as f32); }
        }
    }
}

/// A soft round kick: a sine sweeping down from 160 Hz, lightly saturated.
pub fn kick(buf: &mut [f32], start: usize, vol: f32) {
    let n = (0.16 * SR) as usize;
    let mut ph = 0.0f32;
    for i in 0..n {
        let j = start + i;
        if j >= buf.len() { break; }
        let t = i as f32 / SR;
        ph += (48.0 + 110.0 * (-t / 0.03).exp()) / SR;
        buf[j] += ((2.0 * PI * ph).sin() * 2.0).tanh() * (-t / 0.07).exp() * vol;
    }
}

/// The action kick: a faster, deeper sweep with a click on the front.
pub fn punch(buf: &mut [f32], start: usize, vol: f32) {
    let n = (0.14 * SR) as usize;
    let mut ph = 0.0f32;
    for i in 0..n {
        let j = start + i;
        if j >= buf.len() { break; }
        let t = i as f32 / SR;
        ph += (46.0 + 190.0 * (-t / 0.018).exp()) / SR;
        let click = (2.0 * PI * 1800.0 * t).sin() * (-t / 0.002).exp() * 0.35;
        buf[j] += (((2.0 * PI * ph).sin() * 2.6).tanh() * (-t / 0.055).exp() + click) * vol;
    }
}

/// The action snare: bright noise that cracks, over a 220 Hz body.
pub fn crack(buf: &mut [f32], start: usize, rng: &mut Rng, vol: f32) {
    let n = (0.16 * SR) as usize;
    let mut prev = 0.0f32;
    for i in 0..n {
        let j = start + i;
        if j >= buf.len() { break; }
        let t = i as f32 / SR;
        let w = rng.next_f32() * 2.0 - 1.0;
        let hp = w - 0.6 * prev;
        prev = w;
        let body = (2.0 * PI * 220.0 * t).sin() * (-t / 0.025).exp();
        buf[j] += (hp * 0.7 * (-t / 0.045).exp() + body * 0.6) * vol;
    }
}

/// A snare: softened noise over a short 190 Hz body.
pub fn snare(buf: &mut [f32], start: usize, rng: &mut Rng, vol: f32) {
    let n = (0.14 * SR) as usize;
    let mut lp = 0.0f32;
    for i in 0..n {
        let j = start + i;
        if j >= buf.len() { break; }
        let t = i as f32 / SR;
        let w = rng.next_f32() * 2.0 - 1.0;
        lp += (w - lp) * 0.5;
        let body = (2.0 * PI * 190.0 * t).sin() * (-t / 0.03).exp();
        buf[j] += (lp * (-t / 0.05).exp() * 0.9 + body * 0.5) * vol;
    }
}

/// A closed hat: a 35 ms tick of high-passed noise.
pub fn hat(buf: &mut [f32], start: usize, rng: &mut Rng, vol: f32) {
    let n = (0.035 * SR) as usize;
    let mut prev = 0.0f32;
    for i in 0..n {
        let j = start + i;
        if j >= buf.len() { break; }
        let t = i as f32 / SR;
        let w = rng.next_f32() * 2.0 - 1.0;
        let hp = w - prev;
        prev = w;
        buf[j] += hp * (-t / 0.012).exp() * vol;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rendered_song_is_exactly_its_bars_long() {
        // The tail folds back onto the start, so the loop point is the bar line.
        let song = Song { bpm: 120.0, melody: &[[60, H, 64, H, 67, H, R, R]; 4], chords: &[[major(48); 2]; 4], ..Song::DEFAULT };
        let wav = song.render();
        let samples = (wav.len() - 44) / 2;
        let secs = samples as f32 / (crate::wav::SAMPLE_RATE / 2) as f32;
        assert!((secs - song.duration()).abs() < 0.01, "{secs} s rendered for a {} s song", song.duration());
    }

    fn pcm(wav: &[u8]) -> Vec<i16> {
        wav[44..].chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect()
    }

    #[test]
    fn an_action_song_is_brighter_than_a_cosy_one_and_does_not_clip() {
        let tune = |action| Song {
            bpm: 140.0, melody: &[[72, H, 76, H, 79, H, 76, H]; 4], chords: &[[minor(45); 2]; 4],
            lead: Voice::Saw, bass_voice: Voice::Growl, groove: Groove::Drive, action, ..Song::DEFAULT
        }.render();
        // Sample-to-sample movement is a cheap measure of what is left above 3 kHz.
        let edge = |wav: &[u8]| {
            let s = pcm(wav);
            s.windows(2).map(|w| (w[1] as f64 - w[0] as f64).abs()).sum::<f64>() / s.len() as f64
        };
        let (cosy, action) = (tune(false), tune(true));
        assert!(edge(&action) > edge(&cosy) * 1.25, "the action mix kept no more edge: {} against {}", edge(&action), edge(&cosy));
        let peak = pcm(&action).iter().map(|v| v.unsigned_abs()).max().unwrap();
        assert!(peak < 31_500, "the action mix clips at {peak}");
    }
}
