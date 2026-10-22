//! Every WAV a program can play, by its path: effects once, music looped, one
//! tune at a time. Each is decoded on first use; a tape's tunes are only
//! fetched then too.

use crate::host::audio_event;
use macroquad::audio::{load_sound_from_bytes, play_sound, stop_sound, PlaySoundParams, Sound};
use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct Sounds {
    bytes: HashMap<String, Vec<u8>>,
    /// Known, but fetched (from this URL) only when first played.
    later: HashMap<String, String>,
    decoded: HashMap<String, Sound>,
    pending: Vec<(String, f32, bool)>,
    music: Option<String>,
}

impl Sounds {
    pub(crate) fn add(&mut self, path: String, bytes: Vec<u8>) {
        self.bytes.insert(path, bytes);
    }

    pub(crate) fn add_later(&mut self, path: String, url: String) {
        self.later.insert(path, url);
    }

    /// Forget every file: a new program brings its own.
    pub(crate) fn clear(&mut self) {
        self.stop_music();
        *self = Self::default();
    }

    pub(crate) fn play(&mut self, path: String, volume: f32) -> Result<(), String> {
        self.queue(path, volume, false)
    }

    pub(crate) fn play_music(&mut self, path: String) -> Result<(), String> {
        self.stop_music();
        self.music = Some(path.clone());
        self.queue(path, 0.7, true)
    }

    pub(crate) fn stop_music(&mut self) {
        if let Some(sound) = self.music.take().and_then(|path| self.decoded.get(&path)) {
            stop_sound(sound);
        }
    }

    fn queue(&mut self, path: String, volume: f32, looped: bool) -> Result<(), String> {
        if !self.decoded.contains_key(&path) && !self.bytes.contains_key(&path) && !self.later.contains_key(&path) {
            return Err(format!("Unknown sound {path}"));
        }
        self.pending.push((path, volume, looped));
        audio_event();
        Ok(())
    }

    pub(crate) async fn flush(&mut self) -> Result<(), String> {
        for (path, volume, looped) in std::mem::take(&mut self.pending) {
            if !self.decoded.contains_key(&path) {
                let bytes = match self.bytes.remove(&path) {
                    Some(bytes) => bytes,
                    None => macroquad::file::load_file(&self.later[&path])
                        .await
                        .map_err(|error| format!("Could not load {path}: {error}"))?,
                };
                let sound = load_sound_from_bytes(&bytes)
                    .await
                    .map_err(|error| format!("Could not load {path}: {error}"))?;
                self.decoded.insert(path.clone(), sound);
            }
            // A tune stopped before it had loaded stays stopped.
            if looped && self.music.as_ref() != Some(&path) {
                continue;
            }
            play_sound(&self.decoded[&path], PlaySoundParams { looped, volume });
        }
        Ok(())
    }
}
