//! Load sounds at startup and play them synchronously during the game loop.

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
static CURRENT_AMBIENT: Mutex<Option<Sound>> = Mutex::new(None);
static CURRENT_ALERT: Mutex<Option<Sound>> = Mutex::new(None);

fn play_loop(current: &Mutex<Option<Sound>>, sound: &BlipSound, volume: f32) {
    stop_loop(current);
    play_sound(sound, PlaySoundParams { looped: true, volume });
    if let Ok(mut guard) = current.lock() {
        *guard = Some(sound.clone());
    }
}

fn stop_loop(current: &Mutex<Option<Sound>>) {
    if let Ok(mut guard) = current.lock() {
        if let Some(sound) = guard.take() {
            stop_sound(&sound);
        }
    }
}

/// Start looping music at volume 0.7, replacing the current track.
pub fn play_music(s: &BlipSound) {
    play_loop(&CURRENT_MUSIC, s, 0.7);
}

pub fn stop_music() { stop_loop(&CURRENT_MUSIC); }

/// Start looping ambient sound at volume 0.18.
pub fn play_ambient(s: &BlipSound) {
    play_loop(&CURRENT_AMBIENT, s, 0.18);
}

pub fn stop_ambient() { stop_loop(&CURRENT_AMBIENT); }

/// Start a looping alert at volume 0.55, layered over music and ambient sound.
pub fn play_alert(s: &BlipSound) {
    play_loop(&CURRENT_ALERT, s, 0.55);
}

pub fn stop_alert() { stop_loop(&CURRENT_ALERT); }

/// Synthesize a beep at `freq` Hz for `duration_ms`; load it at startup.
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

/// Synthesizes tracks from WAV makers on demand; call `warm_up` on a screen where a short stall is harmless.
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
        if first >= self.tracks.len() { return; }
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

    /// Has track `i` been rendered?
    pub fn ready(&self, i: usize) -> bool {
        self.tracks.get(i).is_some_and(|t| t.is_some())
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
