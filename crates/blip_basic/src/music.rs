//! Native game music, selected by BASIC track names.

use crate::host::audio_event;
use blip::Jukebox;
use macroquad::audio::{play_sound, stop_sound, PlaySoundParams, Sound};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum MusicRequest {
    Bouncer(usize),
    Defender(usize),
    Adder(usize),
    Serpent(usize),
    Rally(usize),
    Meteors(usize),
    SkyRaider(usize),
    Brawler(usize),
    Bubbler(usize),
    Stop,
}
impl MusicRequest {
    fn selection(self) -> Option<(usize, usize)> {
        Some(match self {
            Self::Bouncer(track) => (0, track),
            Self::Defender(track) => (1, track),
            Self::Adder(track) => (2, track),
            Self::Serpent(track) => (3, track),
            Self::Rally(track) => (4, track),
            Self::Meteors(track) => (5, track),
            Self::SkyRaider(track) => (6, track),
            Self::Brawler(track) => (7, track),
            Self::Bubbler(track) => (8, track),
            Self::Stop => return None,
        })
    }
}

pub(crate) struct Music {
    jukeboxes: [Jukebox; 9],
    active: Option<MusicRequest>,
    files: HashMap<String, Sound>,
    active_file: Option<String>,
}

impl Music {
    pub(crate) fn new() -> Self {
        Self {
            jukeboxes: [
                Jukebox::new(&[
                    || crate::bouncer_tunes::bounce_song().render(),
                    || crate::bouncer_tunes::rebound_song().render(),
                ]),
                Jukebox::new(&[
                    || crate::defender_tunes::invasion_song().render(),
                    || crate::defender_tunes::mothership_song().render(),
                ]),
                Jukebox::new(&[
                    || crate::adder_tunes::coil_song().render(),
                    || crate::adder_tunes::hunt_song().render(),
                ]),
                Jukebox::new(&[
                    || crate::serpent_tunes::slither_song().render(),
                    || crate::serpent_tunes::frenzy_song().render(),
                ]),
                Jukebox::new(&[
                    crate::rally_tunes::music,
                    crate::rally_tunes::music2,
                    crate::rally_tunes::music3,
                    crate::rally_tunes::music4,
                    crate::rally_tunes::music5,
                ]),
                Jukebox::new(&[
                    || crate::meteors_tunes::drift_song().render(),
                    || crate::meteors_tunes::storm_song().render(),
                ]),
                Jukebox::new(&[
                    crate::raider_tunes::music,
                    crate::raider_tunes::music2,
                ]),
                Jukebox::new(&[
                    || crate::brawler_tunes::theme_wav(0),
                    || crate::brawler_tunes::theme_wav(1),
                    || crate::brawler_tunes::theme_wav(2),
                    || crate::brawler_tunes::theme_wav(3),
                    || crate::brawler_tunes::theme_wav(4),
                    || crate::brawler_tunes::theme_wav(5),
                    || crate::brawler_tunes::theme_wav(6),
                    || crate::brawler_tunes::theme_wav(7),
                ]),
                Jukebox::new(&[
                    crate::bubbler_tunes::theme_wav,
                    crate::bubbler_tunes::hurry_wav,
                ]),
            ],
            active: None,
            files: HashMap::new(),
            active_file: None,
        }
    }

    pub(crate) fn stop(&mut self) {
        for jukebox in &mut self.jukeboxes {
            jukebox.stop();
        }
        for sound in self.files.values() {
            stop_sound(sound);
        }
        self.active = None;
        self.active_file = None;
    }

    pub(crate) fn clear_files(&mut self) {
        self.files.clear();
    }

    pub(crate) fn insert_file(&mut self, path: String, sound: Sound) {
        self.files.insert(path, sound);
    }

    pub(crate) fn play_file(&mut self, path: String) -> Result<(), String> {
        self.stop();
        let sound = self
            .files
            .get(&path)
            .ok_or_else(|| format!("? MUSIC FILE NOT FOUND: {path}"))?;
        play_sound(
            sound,
            PlaySoundParams {
                looped: true,
                volume: 0.7,
            },
        );
        self.active_file = Some(path);
        audio_event();
        Ok(())
    }

    pub(crate) async fn play(&mut self, request: MusicRequest) {
        let Some((game, track)) = request.selection() else {
            self.stop();
            return;
        };
        if self.active_file.take().is_some() {
            for sound in self.files.values() {
                stop_sound(sound);
            }
        }
        if self.active != Some(request) {
            audio_event();
        }
        if self
            .active
            .and_then(MusicRequest::selection)
            .map(|(game, _)| game)
            != Some(game)
        {
            self.stop();
        }
        let jukebox = &mut self.jukeboxes[game];
        if jukebox.ready(track) {
            jukebox.play(track);
        } else {
            jukebox.start(track).await;
        }
        self.active = Some(request);
    }

    pub(crate) async fn warm_up(&mut self) {
        if let Some((game, _)) = self.active.and_then(MusicRequest::selection) {
            self.jukeboxes[game].warm_up().await;
        }
    }
}
