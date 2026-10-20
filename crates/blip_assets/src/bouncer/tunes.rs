use crate::cosy::Voice;
use crate::song::{major, minor, Chord, Groove, Song, H, R};

// ---- music: two tunes on the glockenspiel -------------------------------
// Synthesised on the device (see `song`). Octave leaps for the ball's
// bounce; the game alternates the two by level.

const C: Chord = major(48);
const D: Chord = major(50);
const F: Chord = major(41);
const G: Chord = major(43);
const AM: Chord = minor(45);
const EM: Chord = minor(40);

/// Odd levels: C major, bouncing up the octave and skipping back down.
pub fn bounce_song() -> Song<'static> {
    Song {
        bpm: 124.0,
        melody: &[
            [72, 84, 79, H, 76, H, 79, H],
            [77, H, 76, H, 74, H, R, R],
            [71, 83, 79, H, 74, H, 77, H],
            [76, H, 74, H, 72, H, R, R],
            [72, 84, 79, H, 76, H, 79, 81],
            [84, H, 81, H, 77, H, 81, H],
            [79, H, 77, 76, 74, H, 71, H],
            [72, H, H, H, R, R, R, R],
            [81, H, 79, H, 77, H, 76, H],
            [74, H, 76, 77, 79, H, R, R],
            [79, H, 77, H, 76, H, 74, H],
            [72, H, 74, 76, 77, H, R, R],
            [76, 88, 84, H, 79, H, 84, H],
            [86, H, 84, H, 83, H, 79, H],
            [77, H, 76, H, 74, H, 79, H],
            [72, H, H, H, R, R, R, R],
        ],
        chords: &[
            [C, C],
            [F, G],
            [G, G],
            [C, C],
            [C, C],
            [F, F],
            [G, G],
            [C, C],
            [F, F],
            [G, G],
            [C, G],
            [AM, F],
            [C, C],
            [G, G],
            [F, G],
            [C, C],
        ],
        lead: Voice::Glock,
        lead_vol: 0.36,
        harmony: Some((Voice::Pluck, -12)),
        seed: 0xB0DE_1234,
        ..Song::DEFAULT
    }
}

pub fn bounce_wav() -> Vec<u8> {
    bounce_song().render()
}

/// Even levels: G major, quicker, climbing in thirds.
pub fn rebound_song() -> Song<'static> {
    Song {
        bpm: 138.0,
        melody: &[
            [79, H, 83, 86, H, 83, 79, H],
            [81, H, 84, H, 83, H, 81, H],
            [79, H, 83, 86, H, 88, 86, H],
            [84, H, 83, H, 81, H, R, R],
            [76, H, 79, 83, H, 79, 76, H],
            [78, H, 81, H, 84, H, 83, 81],
            [79, H, 78, H, 76, H, 74, H],
            [79, H, H, H, R, R, R, R],
            [86, H, 84, 83, 81, H, 79, H],
            [84, H, 83, 81, 79, H, 78, H],
            [79, 81, 83, 84, 86, H, 83, H],
            [81, H, H, H, R, R, R, R],
            [86, H, 91, H, 88, H, 86, H],
            [84, H, 88, H, 86, H, 84, H],
            [83, H, 81, H, 78, H, 74, H],
            [79, H, H, H, R, R, R, R],
        ],
        chords: &[
            [G, G],
            [D, D],
            [G, G],
            [C, D],
            [EM, EM],
            [D, D],
            [C, D],
            [G, G],
            [G, D],
            [C, D],
            [G, G],
            [D, D],
            [G, C],
            [C, C],
            [D, D],
            [G, G],
        ],
        lead: Voice::Glock,
        lead_vol: 0.34,
        harmony: Some((Voice::Pluck, -12)),
        groove: Groove::FourFloor,
        seed: 0xB0DE_5678,
        ..Song::DEFAULT
    }
}

pub fn rebound_wav() -> Vec<u8> {
    rebound_song().render()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_tracks_render_the_exposed_song_data() {
        assert_eq!(bounce_wav(), bounce_song().render());
        assert_eq!(rebound_wav(), rebound_song().render());
    }
}
