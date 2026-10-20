use crate::cosy::Voice;
use crate::song::{major, minor, Chord, Groove, Song, H, R};

// ---- music: two tunes on the marimba -------------------------------------
// Synthesised on the device (see `song`): A minor pentatonic, the notes a
// snake winds through. SLITHER plays the early levels, FRENZY from level 5.

const AM: Chord = minor(45);
const C: Chord = major(48);
const DM: Chord = minor(50);
const EM: Chord = minor(40);
const F: Chord = major(41);
const G: Chord = major(43);

/// The calm tune: stepwise, winding, a little sly.
pub fn slither_song() -> Song<'static> {
    Song {
        bpm: 108.0,
        melody: &[
            [69, H, 72, 74, 76, H, 74, 72],
            [69, H, H, 67, 69, H, R, R],
            [72, H, 74, 76, 79, H, 76, 74],
            [76, H, H, H, R, R, 74, 72],
            [69, H, 72, 74, 76, H, 79, 81],
            [79, H, 76, H, 74, H, 72, H],
            [74, 76, 74, 72, 69, H, 67, H],
            [69, H, H, H, R, R, R, R],
            [81, H, 79, 76, 79, H, 76, 74],
            [76, H, 74, 72, 74, H, R, R],
            [72, H, 74, 76, 74, 72, 69, H],
            [67, H, 69, H, 72, H, R, R],
            [81, H, 79, H, 76, H, 79, 81],
            [84, H, 81, H, 79, H, 76, H],
            [74, 76, 79, 76, 74, 72, 67, H],
            [69, H, H, H, R, R, R, R],
        ],
        chords: &[
            [AM, AM],
            [AM, G],
            [C, C],
            [G, G],
            [AM, AM],
            [C, G],
            [DM, EM],
            [AM, AM],
            [F, F],
            [C, C],
            [AM, AM],
            [G, G],
            [F, F],
            [G, G],
            [DM, EM],
            [AM, AM],
        ],
        lead: Voice::Marimba,
        lead_vol: 0.6,
        harmony: Some((Voice::Marimba, -12)),
        bass: [0, R, 7, R],
        groove: Groove::Brush,
        seed: 0x5111_7000,
        ..Song::DEFAULT
    }
}

pub fn slither_wav() -> Vec<u8> {
    slither_song().render()
}

/// The fast tune: the same scale, running and leaping.
pub fn frenzy_song() -> Song<'static> {
    Song {
        bpm: 136.0,
        melody: &[
            [81, 79, 76, H, 74, 76, 79, H],
            [81, H, 84, H, 81, 79, 76, H],
            [74, 72, 69, H, 72, 74, 76, H],
            [74, H, 72, H, 69, H, R, R],
            [81, 79, 76, H, 74, 76, 79, H],
            [84, H, 86, H, 84, 81, 79, H],
            [76, 79, 81, 79, 76, 74, 72, 74],
            [69, H, H, H, R, 76, 79, 81],
            [84, H, 81, H, 79, H, 81, H],
            [76, H, 79, H, 74, H, R, R],
            [72, 74, 76, 79, 81, H, 79, 76],
            [79, H, H, H, R, R, R, R],
            [84, H, 81, H, 79, H, 81, 84],
            [86, H, 84, H, 81, H, 79, H],
            [76, 79, 81, 79, 76, 74, 72, 74],
            [69, H, H, H, R, R, R, R],
        ],
        chords: &[
            [AM, AM],
            [F, G],
            [DM, DM],
            [EM, AM],
            [AM, AM],
            [F, G],
            [C, G],
            [AM, AM],
            [F, F],
            [C, C],
            [AM, AM],
            [G, G],
            [F, F],
            [G, G],
            [C, EM],
            [AM, AM],
        ],
        lead: Voice::Marimba,
        lead_vol: 0.42,
        harmony: Some((Voice::Bell, -12)),
        bass: [0, 12, 0, 12],
        groove: Groove::FourFloor,
        seed: 0xF6E2_9000,
        ..Song::DEFAULT
    }
}

pub fn frenzy_wav() -> Vec<u8> {
    frenzy_song().render()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_tracks_render_the_exposed_song_data() {
        assert_eq!(slither_wav(), slither_song().render());
        assert_eq!(frenzy_wav(), frenzy_song().render());
    }
}
