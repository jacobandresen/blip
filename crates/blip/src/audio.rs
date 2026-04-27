//! Audio: WAV one-shots and looping background music via macroquad's
//! quad-snd backend.

use std::f32::consts::PI;
use std::sync::Mutex;

use macroquad::audio::{
    load_sound_from_bytes, play_sound, play_sound_once, stop_sound, PlaySoundParams, Sound,
};

/// Opaque sound handle. Re-export for game crates.
pub type BlipSound = Sound;

/// Decode a WAV (or other supported format) byte slice into a `BlipSound`.
/// Call from `async fn main()` at startup so playback later is sync.
pub async fn load_sound(bytes: &'static [u8]) -> BlipSound {
    load_sound_from_bytes(bytes)
        .await
        .expect("blip::audio::load_sound: decode failed")
}

/// One-shot sound effect at full volume.
pub fn play_sfx(s: &BlipSound) {
    play_sound_once(s);
}

/// Fire-and-forget SFX with explicit volume (0.0 – 1.0).
pub fn play_sfx_volume(s: &BlipSound, volume: f32) {
    play_sound(s, PlaySoundParams { looped: false, volume });
}

static CURRENT_MUSIC: Mutex<Option<Sound>> = Mutex::new(None);

/// Start (or replace) looping background music at the C version's gain (0.45).
pub fn play_music(s: &BlipSound) {
    stop_music();
    play_sound(s, PlaySoundParams { looped: true, volume: 0.45 });
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

/// Synthesize a short sine beep and decode it into a `BlipSound`.
/// Awaited at game startup; callers stash the returned handle and replay
/// it via `play_sfx`. Mirrors the C `blip_play_beep(freq, duration_ms)`
/// envelope (10 ms attack/release, amplitude ≈ 10000/32767).
pub async fn beep(freq: f32, duration_ms: f32) -> BlipSound {
    let bytes = synth_beep_wav(freq, duration_ms);
    // Macroquad needs a 'static slice; the bytes Vec lives long enough for the
    // decode call, so leak it onto the heap. There are at most a handful of
    // beep tones per game and they last for the program lifetime anyway.
    let leaked: &'static [u8] = Box::leak(bytes.into_boxed_slice());
    load_sound(leaked).await
}

fn synth_beep_wav(freq: f32, duration_ms: f32) -> Vec<u8> {
    const SR: u32 = 44_100;
    let n = (SR as f32 * duration_ms / 1000.0) as usize;
    let fade = (SR as usize) / 100;
    let mut samples: Vec<i16> = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / SR as f32;
        let env = if i < fade {
            i as f32 / fade as f32
        } else if i + fade > n {
            (n - i) as f32 / fade as f32
        } else {
            1.0
        };
        let s = (env * 10_000.0 * (2.0 * PI * freq * t).sin()) as i16;
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
