use crate::cosy::Voice;
use crate::song::{major, minor, Chord, Groove, Song, H, R};

// Two device-synthesized E Lydian tracks: DRIFT for early waves, STORM from wave 5.

const E: Chord = major(40);
const FS: Chord = major(42);
const GSM: Chord = minor(44);
const CSM: Chord = minor(37);
const B: Chord = major(47);

/// Slow and spacious: long glass notes that hang in the air.
pub fn drift_song() -> Song<'static> {
    Song {
        bpm: 96.0,
        melody: &[
            [76, H, H, 83, 82, H, 80, H],
            [78, H, H, H, R, R, 76, 78],
            [80, H, H, 82, 83, H, 87, H],
            [85, H, H, H, R, R, R, R],
            [76, H, H, 83, 82, H, 80, H],
            [78, H, 80, H, 82, H, 83, H],
            [85, H, 83, H, 82, H, 78, H],
            [76, H, H, H, R, R, R, R],
            [88, H, H, 87, 83, H, H, H],
            [85, H, 83, H, 82, H, R, R],
            [80, H, H, 82, 83, H, 85, H],
            [87, H, H, H, R, R, R, R],
            [88, H, 87, H, 83, H, 80, H],
            [82, H, H, 80, 78, H, H, H],
            [80, H, 78, H, 75, H, 71, H],
            [76, H, H, H, R, R, R, R],
        ],
        chords: &[
            [E, E],
            [FS, FS],
            [GSM, B],
            [CSM, CSM],
            [E, E],
            [FS, FS],
            [CSM, FS],
            [E, E],
            [E, B],
            [CSM, FS],
            [GSM, CSM],
            [B, B],
            [E, GSM],
            [FS, FS],
            [GSM, B],
            [E, E],
        ],
        lead: Voice::Glass,
        lead_vol: 0.40,
        harmony: Some((Voice::Bell, -12)),
        bass: [0, R, 7, R],
        groove: Groove::Brush,
        seed: 0x3E7E_0001,
        ..Song::DEFAULT
    }
}

pub fn drift_wav() -> Vec<u8> {
    drift_song().render()
}

/// Driving: the same scale in a rolling figure over a steady kick.
pub fn storm_song() -> Song<'static> {
    Song {
        bpm: 140.0,
        melody: &[
            [76, 83, 80, 83, 76, 83, 82, 83],
            [78, 85, 82, 85, 78, 85, 83, 85],
            [80, 87, 83, 87, 80, 87, 85, 87],
            [88, H, 87, H, 85, H, 83, H],
            [76, 83, 80, 83, 76, 83, 82, 83],
            [78, 85, 82, 85, 78, 85, 83, 85],
            [85, H, 83, H, 82, H, 78, H],
            [76, H, H, H, R, R, R, R],
        ],
        chords: &[
            [E, E],
            [FS, FS],
            [GSM, GSM],
            [CSM, B],
            [E, E],
            [FS, FS],
            [CSM, FS],
            [E, E],
        ],
        lead: Voice::Glass,
        lead_vol: 0.30,
        harmony: Some((Voice::Saw, -12)),
        arp: false,
        bass: [0, 12, 0, 12],
        bass_voice: Voice::Growl,
        groove: Groove::Drive,
        action: true,
        seed: 0x3E7E_0002,
        ..Song::DEFAULT
    }
}

pub fn storm_wav() -> Vec<u8> {
    storm_song().render()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_tracks_render_the_exposed_song_data() {
        assert_eq!(drift_wav(), drift_song().render());
        assert_eq!(storm_wav(), storm_song().render());
    }
}
