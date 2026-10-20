use crate::cosy::Voice;
use crate::song::{major, minor, Chord, Groove, Song, H, R};

// ---- music: two tunes on the pluck ---------------------------------------
// Synthesised on the device (see `song`): A minor pentatonic, the same notes
// Serpent winds through on a marimba, struck dry and earthy instead. COIL
// plays the early pits, HUNT from pit 4.

const AM: Chord = minor(45);
const C: Chord = major(48);
const DM: Chord = minor(50);
const EM: Chord = minor(40);
const F: Chord = major(41);
const G: Chord = major(43);

/// The slow tune: a coiled figure, hypnotic, the way a charmer's line hangs.
pub fn coil_song() -> Song<'static> {
    Song {
        bpm: 92.0,
        melody: &[
            [69, H, 72, H, 74, H, 72, H],
            [69, H, H, H, 67, H, R, R],
            [72, H, 74, H, 76, H, 74, H],
            [72, H, H, H, R, R, 69, H],
            [76, H, 74, H, 72, H, 74, H],
            [76, H, H, H, 79, H, R, R],
            [76, H, 74, H, 72, H, 69, H],
            [67, H, H, H, R, R, R, R],
            [81, H, 79, H, 76, H, 79, H],
            [81, H, H, H, 84, H, R, R],
            [84, H, 81, H, 79, H, 76, H],
            [79, H, H, H, R, R, 76, H],
            [74, H, 76, H, 79, H, 81, H],
            [79, H, H, H, 76, H, R, R],
            [74, H, 72, H, 69, H, 72, H],
            [69, H, H, H, R, R, R, R],
        ],
        chords: &[
            [AM, AM],
            [AM, G],
            [C, C],
            [G, G],
            [AM, AM],
            [F, F],
            [DM, EM],
            [AM, AM],
            [F, F],
            [C, C],
            [AM, AM],
            [EM, EM],
            [F, F],
            [G, G],
            [DM, EM],
            [AM, AM],
        ],
        lead: Voice::Pluck,
        lead_vol: 0.5,
        harmony: Some((Voice::Bell, -12)),
        bass: [0, R, 7, R],
        groove: Groove::Brush,
        seed: 0xADD0_0001,
        ..Song::DEFAULT
    }
}

pub fn coil_wav() -> Vec<u8> {
    coil_song().render()
}

/// The fast tune, from pit 4: the same five notes run in eighths.
pub fn hunt_song() -> Song<'static> {
    Song {
        bpm: 138.0,
        melody: &[
            [69, 72, 74, 72, 69, 72, 74, 76],
            [79, H, 74, H, 72, H, 74, H],
            [76, 74, 72, 74, 76, 79, 81, H],
            [79, H, H, H, R, 76, 74, 72],
            [69, 72, 74, 76, 79, 76, 74, 72],
            [69, H, 67, H, 69, H, 72, H],
            [74, 76, 79, 81, 84, H, 81, H],
            [79, H, H, H, R, R, R, R],
            [81, 79, 76, 79, 81, 84, 81, H],
            [79, H, 76, H, 74, H, 72, H],
            [74, 76, 74, 72, 69, 72, 74, H],
            [76, H, H, H, R, 79, 81, H],
            [84, H, 81, H, 79, H, 81, H],
            [76, H, 74, H, 72, H, R, R],
            [74, 72, 69, 72, 74, 76, 79, H],
            [69, H, H, H, R, R, R, R],
        ],
        chords: &[
            [AM, AM],
            [AM, G],
            [C, C],
            [G, G],
            [AM, AM],
            [F, F],
            [DM, EM],
            [AM, AM],
            [F, F],
            [C, C],
            [AM, AM],
            [EM, EM],
            [F, F],
            [G, G],
            [DM, EM],
            [AM, AM],
        ],
        lead: Voice::Pluck,
        lead_vol: 0.36,
        harmony: Some((Voice::Bell, -12)),
        bass: [0, 12, 0, 12],
        groove: Groove::FourFloor,
        seed: 0xADD0_0002,
        ..Song::DEFAULT
    }
}

pub fn hunt_wav() -> Vec<u8> {
    hunt_song().render()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_tracks_render_the_exposed_song_data() {
        assert_eq!(coil_wav(), coil_song().render());
        assert_eq!(hunt_wav(), hunt_song().render());
    }
}
