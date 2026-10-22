//! FastTracker 2 modules: four chip voices playing notes, as BLIP BASIC's TUNE
//! reads and plays them (square, pulse, triangle, noise; four steps a beat).
//!
//! A tune's notes are heard as it renders: each instrument reports what it
//! plays with `heard`, and `record` writes them out, a step to a sixteenth.

use std::cell::RefCell;

use crate::wav::SAMPLE_RATE;

/// One step of a channel: a MIDI note, or nothing.
pub type Step = Option<i32>;

/// The part an instrument plays, which is TUNE's voice for it.
#[derive(Clone, Copy, PartialEq)]
pub enum Part { Tune = 0, Harmony = 1, Bass = 2, Drums = 3 }

/// The drums' notes on the noise voice.
pub const KICK: i32 = 36;
pub const SNARE: i32 = 48;
pub const HAT: i32 = 72;

struct Recording { bpm: f32, notes: Vec<(usize, i32, Part)> }

thread_local! {
    static HEARD: RefCell<Option<Recording>> = const { RefCell::new(None) };
}

/// The tune being recorded beats at `bpm`.
pub fn tempo(bpm: f32) {
    HEARD.with(|h| if let Some(r) = h.borrow_mut().as_mut() { r.bpm = bpm });
}

/// An instrument played `midi` at sample `at`: kept only while a tune is recorded.
pub fn heard(at: usize, midi: i32, part: Part) {
    HEARD.with(|h| if let Some(r) = h.borrow_mut().as_mut() { r.notes.push((at, midi, part)) });
}

/// The same for an instrument tuned in hertz.
pub fn heard_hz(at: usize, hz: f32, part: Part) {
    heard(at, (69.0 + 12.0 * (hz / 440.0).log2()).round() as i32, part);
}

/// Render a tune with `make`; returns its WAV and, from the notes it played, its XM.
pub fn record(name: &str, make: impl FnOnce() -> Vec<u8>) -> (Vec<u8>, Vec<u8>) {
    HEARD.with(|h| *h.borrow_mut() = Some(Recording { bpm: 120.0, notes: Vec::new() }));
    let wav = make();
    let Recording { bpm, notes } = HEARD.with(|h| h.borrow_mut().take()).expect("recording");
    let step = SAMPLE_RATE as f32 * 15.0 / bpm;
    let steps = notes.iter().map(|&(at, ..)| (at as f32 / step).round() as usize + 1).max().unwrap_or(1);
    let mut voices = vec![vec![None; steps]; 4];
    for (at, midi, part) in notes {
        let s = (at as f32 / step).round() as usize;
        // An instrument that plays both the tune and the bass (a koto, a guitar) is the bass
        // below C3; a second note in the tune at once (a doubling, a chord) is the harmony.
        let part = if part == Part::Tune && midi < 48 { Part::Bass } else { part };
        let c = if part == Part::Tune && voices[0][s].is_some() { Part::Harmony } else { part } as usize;
        voices[c][s].get_or_insert(midi);
    }
    (wav, module(name, bpm, &voices))
}

/// A game's tunes as files: `music/NAME.wav` to play, `music/NAME.xm` for TUNE.
pub fn tunes(list: &[(&str, &dyn Fn() -> Vec<u8>)]) -> Vec<(String, Vec<u8>)> {
    list.iter()
        .flat_map(|(name, make)| {
            let (wav, xm) = record(name, make);
            [(format!("music/{name}.wav"), wav), (format!("music/{name}.xm"), xm)]
        })
        .collect()
}

const ROWS: usize = 16;
const VOICES: usize = 4;

/// An XM of up to four `channels` of steps at `bpm`, a bar of 16 steps a pattern.
pub fn module(name: &str, bpm: f32, channels: &[Vec<Step>]) -> Vec<u8> {
    let steps = channels.iter().map(Vec::len).max().unwrap_or(0).max(1);
    let patterns = steps.div_ceil(ROWS).min(256);
    let mut out = Vec::new();
    out.extend_from_slice(b"Extended Module: ");
    out.extend_from_slice(&text(name, 20));
    out.push(0x1a);
    out.extend_from_slice(&text("BLIP", 20));
    put16(&mut out, 0x0104);
    put32(&mut out, 276);
    put16(&mut out, patterns as u16); // song length
    put16(&mut out, 0); // restart
    put16(&mut out, VOICES as u16);
    put16(&mut out, patterns as u16);
    put16(&mut out, VOICES as u16); // instruments
    put16(&mut out, 1); // linear frequencies
    put16(&mut out, 6); // ticks a row: four rows a beat at `bpm`
    put16(&mut out, bpm.round().clamp(32.0, 255.0) as u16);
    let mut order = [0u8; 256];
    for (i, o) in order.iter_mut().take(patterns).enumerate() {
        *o = i as u8;
    }
    out.extend_from_slice(&order);
    for p in 0..patterns {
        let mut data = Vec::new();
        for row in p * ROWS..(p + 1) * ROWS {
            for c in 0..VOICES {
                match channels.get(c).and_then(|ch| ch.get(row)).copied().flatten() {
                    // TUNE reads an XM note as MIDI + 1.
                    Some(midi) => data.extend_from_slice(&[(midi + 1).clamp(1, 96) as u8, c as u8 + 1, 0, 0, 0]),
                    None => data.push(0x80),
                }
            }
        }
        put32(&mut out, 9);
        out.push(0);
        put16(&mut out, ROWS as u16);
        put16(&mut out, data.len() as u16);
        out.extend_from_slice(&data);
    }
    for (name, wave) in ["SQUARE", "PULSE", "TRIANGLE", "NOISE"].iter().zip(waves()) {
        instrument(&mut out, name, &wave);
    }
    out
}

/// One cycle of each chip voice at 8363 Hz, so a 32-sample cycle is middle C;
/// the noise is a short burst.
fn waves() -> [Vec<i8>; VOICES] {
    let square = (0..32).map(|i| if i < 16 { 64 } else { -64 }).collect();
    let pulse = (0..32).map(|i| if i < 8 { 64 } else { -64 }).collect();
    let triangle = (0..32).map(|i: i32| (if i < 16 { i * 8 - 64 } else { 192 - i * 8 }) as i8).collect();
    let mut seed = 0x1234_5678u32;
    let noise = (0..1500)
        .map(|i| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            ((seed >> 24) as i8 as i32 * (1500 - i) / 1500 / 2) as i8
        })
        .collect();
    [square, pulse, triangle, noise]
}

fn instrument(out: &mut Vec<u8>, name: &str, wave: &[i8]) {
    let looped = wave.len() == 32;
    put32(out, 263);
    out.extend_from_slice(&text(name, 22));
    out.push(0);
    put16(out, 1); // one sample
    put32(out, 40); // sample header size
    out.extend_from_slice(&[0; 96]); // every note plays sample 0
    out.extend_from_slice(&[0; 263 - 4 - 22 - 1 - 2 - 4 - 96]); // no envelopes, no vibrato
    put32(out, wave.len() as u32);
    put32(out, 0);
    put32(out, if looped { wave.len() as u32 } else { 0 });
    out.push(40); // volume
    out.push(0); // finetune
    out.push(if looped { 1 } else { 0 }); // forward loop, 8-bit
    out.push(128); // centre
    out.push((-12i8) as u8); // TUNE's note numbering is an octave above FastTracker's
    out.push(0);
    out.extend_from_slice(&text(name, 22));
    let mut last = 0i8;
    for &s in wave {
        out.push(s.wrapping_sub(last) as u8);
        last = s;
    }
}

fn text(s: &str, len: usize) -> Vec<u8> {
    let mut v: Vec<u8> = s.bytes().take(len).collect();
    v.resize(len, b' ');
    v
}

fn put16(out: &mut Vec<u8>, v: u16) { out.extend_from_slice(&v.to_le_bytes()); }
fn put32(out: &mut Vec<u8>, v: u32) { out.extend_from_slice(&v.to_le_bytes()); }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bar_of_notes_is_one_pattern_of_four_channels() {
        let melody: Vec<Step> = (0..16).map(|i| if i % 4 == 0 { Some(60 + i) } else { None }).collect();
        let xm = module("TEST", 120.0, &[melody]);
        assert_eq!(&xm[..17], b"Extended Module: ");
        let u16_at = |i: usize| u16::from_le_bytes([xm[i], xm[i + 1]]);
        assert_eq!((u16_at(64), u16_at(68), u16_at(70), u16_at(72)), (1, 4, 1, 4));
        assert_eq!((u16_at(76), u16_at(78)), (6, 120));
        // the first cell of the pattern: middle C as TUNE reads it, on the square
        let cell = 60 + 276 + 9;
        assert_eq!(&xm[cell..cell + 2], &[61, 1]);
    }
}
