//! Decode generated effects on first use, then retain them for later frames.

use blip::BlipSound;
use macroquad::audio::load_sound_from_bytes;
use std::collections::HashMap;

pub(crate) struct Sounds {
    bytes: HashMap<&'static str, &'static [u8]>,
    decoded: HashMap<String, BlipSound>,
    pending: Vec<(String, f32)>,
}

impl Sounds {
    pub(crate) fn new(assets: impl IntoIterator<Item = (&'static str, &'static [u8])>) -> Self {
        Self {
            bytes: assets.into_iter().collect(),
            decoded: HashMap::new(),
            pending: Vec::new(),
        }
    }

    pub(crate) fn play(&mut self, name: String, volume: f32) -> Result<(), String> {
        if let Some(sound) = self.decoded.get(&name) {
            blip::play_sfx_volume(sound, volume);
        } else if self.bytes.contains_key(name.as_str()) {
            self.pending.push((name, volume));
        } else {
            return Err(format!("Unknown sound {name}"));
        }
        Ok(())
    }

    pub(crate) async fn flush(&mut self) -> Result<(), String> {
        for (name, volume) in std::mem::take(&mut self.pending) {
            if !self.decoded.contains_key(&name) {
                let sound = load_sound_from_bytes(self.bytes[name.as_str()])
                    .await
                    .map_err(|error| format!("Could not load sound {name}: {error}"))?;
                self.decoded.insert(name.clone(), sound);
            }
            blip::play_sfx_volume(&self.decoded[&name], volume);
        }
        Ok(())
    }
}
