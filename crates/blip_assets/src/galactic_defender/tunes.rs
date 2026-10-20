use crate::cosy::Voice;
use crate::song::{major, minor, Chord, Groove, Song, H, R};

const EM: Chord = minor(40);
const AM: Chord = minor(45);
const DM: Chord = minor(50);
const B: Chord = major(47);
const E: Chord = major(40);
const C: Chord = major(48);
const D: Chord = major(50);
const F: Chord = major(41);
const G: Chord = major(43);

pub fn invasion_song() -> Song<'static> {
    Song {
        bpm: 126.0,
        melody: &[
            [76, H, H, 79, 78, H, 76, H],
            [71, H, H, H, R, R, 74, 76],
            [77, H, H, 76, 74, H, 72, H],
            [71, H, H, H, R, R, R, R],
            [76, H, H, 79, 83, H, 81, 79],
            [78, H, 76, H, 75, H, 71, H],
            [72, H, 74, H, 75, H, 78, H],
            [76, H, H, H, R, R, R, R],
            [83, H, H, 81, 79, H, 78, H],
            [76, H, H, H, 71, H, R, R],
            [81, H, H, 79, 77, H, 76, H],
            [74, H, H, H, R, R, R, R],
            [76, H, 79, H, 83, H, 88, H],
            [87, H, H, 83, 81, H, 78, H],
            [79, H, 78, H, 75, H, 71, H],
            [76, H, H, H, R, R, R, R],
        ],
        chords: &[
            [EM, EM],
            [B, B],
            [F, F],
            [B, B],
            [EM, EM],
            [B, B],
            [AM, B],
            [EM, EM],
            [G, D],
            [EM, EM],
            [F, F],
            [G, G],
            [EM, EM],
            [B, B],
            [C, B],
            [EM, EM],
        ],
        lead: Voice::Saw,
        lead_vol: 0.30,
        harmony: Some((Voice::Theremin, -12)),
        bass: [0, 0, 7, 0],
        bass_voice: Voice::Growl,
        groove: Groove::FourFloor,
        action: true,
        seed: 0xDEF0_1001,
        ..Song::DEFAULT
    }
}

pub fn invasion_wav() -> Vec<u8> {
    invasion_song().render()
}

pub fn mothership_song() -> Song<'static> {
    Song {
        bpm: 158.0,
        melody: &[
            [69, H, 72, 69, 76, H, 75, 76],
            [77, H, 76, 74, 72, H, 71, H],
            [69, H, 72, 69, 76, H, 79, H],
            [77, H, 76, H, 75, H, R, R],
            [81, H, 80, 81, 84, H, 81, H],
            [77, H, 76, 74, 76, H, 71, H],
            [72, H, 74, H, 75, H, 76, H],
            [69, H, H, H, R, R, R, R],
        ],
        chords: &[
            [AM, AM],
            [F, E],
            [AM, AM],
            [F, E],
            [AM, AM],
            [DM, E],
            [F, E],
            [AM, AM],
        ],
        lead: Voice::Saw,
        lead_vol: 0.30,
        harmony: Some((Voice::Saw, -12)),
        bass: [0, 0, 12, 0],
        bass_voice: Voice::Growl,
        groove: Groove::Drive,
        action: true,
        seed: 0xDEF0_2002,
        ..Song::DEFAULT
    }
}

pub fn mothership_wav() -> Vec<u8> {
    mothership_song().render()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_tracks_render_the_exposed_song_data() {
        assert_eq!(invasion_wav(), invasion_song().render());
        assert_eq!(mothership_wav(), mothership_song().render());
    }
}
