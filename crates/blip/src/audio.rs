//! Audio — load sounds at startup, play them synchronously during the game loop.
//!
//! The basic pattern for every game:
//! 1. At startup (inside `async fn main`), load each sound with `load_sound()` or `beep()`.
//! 2. Store the returned `BlipSound` handles somewhere (usually a `Sounds` struct).
//! 3. Call `play_sfx()`, `play_music()`, or `play_ambient()` whenever you need them — these
//!    are synchronous and can be called freely inside the game loop.

use std::f32::consts::PI;
use std::sync::Mutex;

use macroquad::audio::{
    load_sound_from_bytes, play_sound, play_sound_once, stop_sound, PlaySoundParams, Sound,
};

/// An opaque handle to a decoded sound. Store these at startup; replaying them is free.
pub type BlipSound = Sound;

/// Decode a WAV byte slice into a `BlipSound`. Await it at startup, not in
/// the game loop. The bytes are copied into the audio engine, so they only
/// need to live for the call.
pub async fn load_sound(bytes: &[u8]) -> BlipSound {
    load_sound_from_bytes(bytes)
        .await
        .expect("blip::audio::load_sound: decode failed")
}

/// Play a sound effect once at full volume.
pub fn play_sfx(s: &BlipSound) {
    play_sound_once(s);
}

/// Play a sound effect once at a specific volume (0.0 = silent, 1.0 = full).
pub fn play_sfx_volume(s: &BlipSound, volume: f32) {
    play_sound(s, PlaySoundParams { looped: false, volume });
}

static CURRENT_MUSIC: Mutex<Option<Sound>> = Mutex::new(None);

/// Start looping background music. Replaces any currently playing music.
/// Volume is set to 0.7 — up front in the mix, not background wallpaper.
pub fn play_music(s: &BlipSound) {
    stop_music();
    play_sound(s, PlaySoundParams { looped: true, volume: 0.7 });
    if let Ok(mut guard) = CURRENT_MUSIC.lock() {
        *guard = Some(s.clone());
    }
}

pub fn stop_music() {
    if let Ok(mut guard) = CURRENT_MUSIC.lock() {
        if let Some(s) = guard.take() {
            stop_sound(&s);
        }
    }
}

static CURRENT_AMBIENT: Mutex<Option<Sound>> = Mutex::new(None);

/// Start looping ambient sound (e.g. wind, ocean). Quieter than music at volume 0.18.
pub fn play_ambient(s: &BlipSound) {
    stop_ambient();
    play_sound(s, PlaySoundParams { looped: true, volume: 0.18 });
    if let Ok(mut guard) = CURRENT_AMBIENT.lock() {
        *guard = Some(s.clone());
    }
}

pub fn stop_ambient() {
    if let Ok(mut guard) = CURRENT_AMBIENT.lock() {
        if let Some(s) = guard.take() {
            stop_sound(&s);
        }
    }
}

static CURRENT_ALERT: Mutex<Option<Sound>> = Mutex::new(None);

/// Start looping an attention-grabbing alert sound (e.g. a boss/UFO siren)
/// layered over the music and ambient channels — loud enough to stand out.
pub fn play_alert(s: &BlipSound) {
    stop_alert();
    play_sound(s, PlaySoundParams { looped: true, volume: 0.55 });
    if let Ok(mut guard) = CURRENT_ALERT.lock() {
        *guard = Some(s.clone());
    }
}

pub fn stop_alert() {
    if let Ok(mut guard) = CURRENT_ALERT.lock() {
        if let Some(s) = guard.take() {
            stop_sound(&s);
        }
    }
}

/// Synthesize a short beep as a `BlipSound` (no WAV file): `freq` in Hz,
/// `duration_ms` long. Await at startup; replay freely with `play_sfx`.
/// A fundamental with a soft third harmonic and a touch of tanh saturation, a
/// fast attack and a slight downward drift.
pub async fn beep(freq: f32, duration_ms: f32) -> BlipSound {
    load_sound(&synth_beep_wav(freq, duration_ms)).await
}

fn synth_beep_wav(freq: f32, duration_ms: f32) -> Vec<u8> {
    const SR: u32 = 44_100;
    let n = (SR as f32 * duration_ms / 1000.0) as usize;
    let attack = ((SR as usize) / 500).max(1).min(n / 4 + 1);
    let mut samples: Vec<i16> = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / SR as f32;
        let env = if i < attack {
            i as f32 / attack as f32
        } else {
            let d = (i - attack) as f32 / (n - attack).max(1) as f32;
            (1.0 - d).powf(1.6)
        };
        let drift = 1.0 - 0.05 * (i as f32 / n as f32); // slight downward pitch character
        let f = freq * drift;
        let fund = (2.0 * PI * f * t).sin();
        let third = (2.0 * PI * f * 3.0 * t).sin() / 3.0;
        let shaped = (fund * 0.8 + third * 0.35).tanh();
        let s = (env * 11_500.0 * shaped) as i16;
        samples.push(s);
    }
    encode_wav(SR, &samples)
}

fn encode_wav(sample_rate: u32, samples: &[i16]) -> Vec<u8> {
    let n = samples.len();
    let data_bytes = (n * 2) as u32;
    let mut out = Vec::with_capacity(44 + n * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_bytes.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// A game's music: tracks synthesised on the device from `fn() -> Vec<u8>`
/// WAV makers. `start` renders the first one wanted; the rest wait until
/// `warm_up` is called on a screen where a short stall is harmless (a title
/// or a pause), so loading never renders every track up front.
pub struct Jukebox {
    makers: Vec<fn() -> Vec<u8>>,
    tracks: Vec<Option<(BlipSound, f32)>>,
    playing: Option<usize>,
    /// Seconds left in the current loop, for `rotate`.
    left: f32,
}

impl Jukebox {
    pub fn new(makers: &[fn() -> Vec<u8>]) -> Self {
        Self { makers: makers.to_vec(), tracks: vec![None; makers.len()], playing: None, left: 0.0 }
    }

    /// Render track `first` and play it.
    pub async fn start(&mut self, first: usize) {
        self.render(first).await;
        self.switch(first);
    }

    /// Render the next track still missing, if any. Returns true while more remain.
    pub async fn warm_up(&mut self) -> bool {
        if let Some(i) = self.tracks.iter().position(|t| t.is_none()) {
            self.render(i).await;
        }
        self.tracks.iter().any(|t| t.is_none())
    }

    /// Play track `i` unless it is already playing or not rendered yet.
    pub fn play(&mut self, i: usize) {
        if self.playing != Some(i) && self.tracks.get(i).is_some_and(|t| t.is_some()) { self.switch(i); }
    }

    /// Silence; the next `play` starts again.
    pub fn stop(&mut self) {
        stop_music();
        self.playing = None;
    }

    /// Move on to the next ready track each time the current loop ends.
    pub fn rotate(&mut self, dt: f32) {
        let Some(cur) = self.playing else { return };
        self.left -= dt;
        if self.left > 0.0 { return; }
        let n = self.tracks.len();
        let next = (1..=n).map(|k| (cur + k) % n).find(|&i| self.tracks[i].is_some()).unwrap_or(cur);
        self.switch(next);
    }

    async fn render(&mut self, i: usize) {
        let wav = (self.makers[i])();
        let secs = wav_seconds(&wav);
        self.tracks[i] = Some((load_sound(&wav).await, secs));
    }

    fn switch(&mut self, i: usize) {
        if let Some((s, secs)) = &self.tracks[i] {
            play_music(s);
            self.playing = Some(i);
            self.left = *secs;
        }
    }
}

/// The length of a 16-bit mono PCM WAV, from its header.
fn wav_seconds(wav: &[u8]) -> f32 {
    if wav.len() < 44 { return 0.0; }
    let rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]) as f32;
    let data = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]) as f32;
    data / 2.0 / rate.max(1.0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_wav_header_gives_its_length() {
        let wav = super::encode_wav(22_050, &vec![0i16; 22_050 * 3]);
        assert!((super::wav_seconds(&wav) - 3.0).abs() < 1e-4);
    }
}
