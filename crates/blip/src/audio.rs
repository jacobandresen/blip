//! Load sounds at startup and play them synchronously during the game loop.

use std::sync::{Mutex, OnceLock};

use macroquad::experimental::coroutines::{start_coroutine, Coroutine};

use macroquad::audio::{
    load_sound_from_bytes, play_sound, play_sound_once, stop_sound, PlaySoundParams, Sound,
};

/// An opaque handle to a decoded sound. Store these at startup; replaying them is free.
pub type BlipSound = Sound;

fn silent() -> bool {
    static SILENT: OnceLock<bool> = OnceLock::new();
    *SILENT.get_or_init(|| std::env::var("BLIP_SILENT").is_ok_and(|value| value == "1"))
}

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
    if !silent() { play_sound_once(s); }
}

/// Play a sound effect once at a specific volume (0.0 = silent, 1.0 = full).
pub fn play_sfx_volume(s: &BlipSound, volume: f32) {
    if !silent() { play_sound(s, PlaySoundParams { looped: false, volume }); }
}

static CURRENT_MUSIC: Mutex<Option<Sound>> = Mutex::new(None);
static CURRENT_AMBIENT: Mutex<Option<Sound>> = Mutex::new(None);
static CURRENT_ALERT: Mutex<Option<Sound>> = Mutex::new(None);

// The rendered loops span 8 dB RMS; bound matching so sparse intros stay restrained.
const MUSIC_RMS_TARGET_DB: f64 = -17.5;
const MUSIC_GAIN_MIN: f32 = 0.5;
const MUSIC_GAIN_MAX: f32 = 1.3;

fn play_loop(current: &Mutex<Option<Sound>>, sound: &BlipSound, volume: f32) {
    stop_loop(current);
    if silent() { return; }
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

/// Start looping music at a specific volume, replacing the current track.
fn play_music_volume(s: &BlipSound, volume: f32) {
    play_loop(&CURRENT_MUSIC, s, volume.clamp(0.0, 1.0));
}

fn stop_music() { stop_loop(&CURRENT_MUSIC); }

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

/// Plays a game's tunes from its music folder. Each loads in the background the
/// first time it is wanted, and starts once it has: a WAV is megabytes, and the
/// game must not wait on it.
pub struct Jukebox {
    files: Vec<String>,
    tracks: Vec<Option<(BlipSound, f32, f32)>>,
    loading: Vec<Option<Coroutine<(BlipSound, f32, f32)>>>,
    playing: Option<usize>,
    wanted: Option<usize>,
    /// Seconds left in the current loop, for `rotate`.
    left: f32,
}

impl Jukebox {
    /// `game`'s tunes `names`, as `web/<game>/music/<NAME>.wav`.
    pub fn new(game: &str, names: &[&str]) -> Self {
        let files = names.iter().map(|name| music_file(game, name)).collect();
        Self { files, tracks: vec![None; names.len()], loading: names.iter().map(|_| None).collect(),
            playing: None, wanted: None, left: 0.0 }
    }

    /// Play track `i`: at once if it has loaded, else as soon as it has.
    pub fn play(&mut self, i: usize) {
        if i >= self.tracks.len() { return; }
        self.wanted = Some(i);
        self.fetch(i);
        self.settle();
    }

    /// Silence; the next `play` starts again.
    pub fn stop(&mut self) {
        stop_music();
        self.playing = None;
        self.wanted = None;
    }

    /// Play the tracks in turn, moving on each time a loop ends; the next one
    /// loads while this one plays.
    pub fn rotate(&mut self, dt: f32) {
        let n = self.tracks.len();
        let Some(cur) = self.playing else { return self.play(self.wanted.unwrap_or(0)) };
        self.fetch((cur + 1) % n);
        self.left -= dt;
        if self.left > 0.0 { return self.settle(); }
        let next = (1..=n).map(|k| (cur + k) % n).find(|&i| self.tracks[i].is_some()).unwrap_or(cur);
        self.wanted = Some(next);
        self.switch(next);
    }

    /// Start loading track `i` unless it is loaded or on its way.
    fn fetch(&mut self, i: usize) {
        if self.tracks[i].is_some() || self.loading[i].is_some() { return; }
        let file = self.files[i].clone();
        self.loading[i] = Some(start_coroutine(async move {
            // A missing file is a quiet game, not a stopped one.
            let wav = macroquad::file::load_file(&file).await.unwrap_or_default();
            (load_sound(&wav).await, wav_seconds(&wav), music_gain(&wav))
        }));
    }

    /// Take in whatever has loaded, and play the wanted track if it now can.
    fn settle(&mut self) {
        for (track, loading) in self.tracks.iter_mut().zip(&mut self.loading) {
            if let Some(done) = loading.as_ref().and_then(|c| c.retrieve()) {
                *track = Some(done);
                *loading = None;
            }
        }
        if let Some(i) = self.wanted {
            if self.playing != Some(i) { self.switch(i); }
        }
    }

    fn switch(&mut self, i: usize) {
        if let Some((s, secs, gain)) = &self.tracks[i] {
            play_music_volume(s, 0.7 * gain);
            self.playing = Some(i);
            self.left = *secs;
        }
    }
}

/// Where a game's tune is: its music folder, beside the game's own page.
fn music_file(game: &str, name: &str) -> String {
    if cfg!(target_arch = "wasm32") {
        format!("music/{name}.wav")
    } else {
        format!("{}/../../web/{game}/music/{name}.wav", env!("CARGO_MANIFEST_DIR"))
    }
}

/// The length of a 16-bit mono PCM WAV, from its header.
fn wav_seconds(wav: &[u8]) -> f32 {
    if wav.len() < 44 { return 0.0; }
    let rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]) as f32;
    let data = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]) as f32;
    data / 2.0 / rate.max(1.0)
}

fn music_gain(wav: &[u8]) -> f32 {
    if wav.len() < 46 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" || &wav[36..40] != b"data" {
        return 1.0;
    }
    let bytes = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]) as usize;
    let end = wav.len().min(44 + bytes) & !1;
    let samples = wav[44..end].chunks_exact(2).map(|s| i16::from_le_bytes([s[0], s[1]]) as f64);
    let (sum, count) = samples.fold((0.0, 0usize), |(sum, count), s| (sum + s * s, count + 1));
    if count == 0 { return 1.0; }
    let rms = (sum / count as f64).sqrt() / i16::MAX as f64;
    if rms < 1e-5 { return 1.0; }
    let db = 20.0 * rms.log10();
    (10.0f64.powf((MUSIC_RMS_TARGET_DB - db) / 20.0) as f32)
        .clamp(MUSIC_GAIN_MIN, MUSIC_GAIN_MAX)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_wav_header_gives_its_length() {
        let wav = crate::wav::encode_at(&vec![0i16; 22_050 * 3], 22_050);
        assert!((super::wav_seconds(&wav) - 3.0).abs() < 1e-4);
    }

    #[test]
    fn jukebox_gain_balances_track_loudness_without_large_boosts() {
        let quiet = crate::wav::encode_at(&[4_000i16; 100], 22_050);
        let loud = crate::wav::encode_at(&[8_000i16; 100], 22_050);
        let q = super::music_gain(&quiet);
        let l = super::music_gain(&loud);
        assert!((q / l - 2.0).abs() < 0.02, "the louder track should receive half the gain: {q} vs {l}");
        assert!((0.5..=1.3).contains(&q) && (0.5..=1.3).contains(&l));
        assert_eq!(super::music_gain(b"not a wav"), 1.0);
    }
}
