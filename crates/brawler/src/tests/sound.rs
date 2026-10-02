//! What a fight sounds like: the generated effects and themes.

use super::*;

// ---- what a fight sounds like -------------------------------------------

/// Every sample of a mono 16-bit WAV, as floats.
pub(crate) fn wav_samples(wav: &[u8]) -> Vec<f32> {
    wav[44..].chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
        .collect()
}

/// How high a sound sits, as zero crossings per second. Crude against a
/// real spectrum and exactly right for the question asked of it: is
/// this one brighter than that one.
pub(crate) fn brightness(wav: &[u8]) -> f32 {
    let s = wav_samples(wav);
    let crossings = s.windows(2).filter(|w| (w[0] < 0.0) != (w[1] < 0.0)).count();
    crossings as f32 / (s.len() as f32 / 44_100.0).max(0.001)
}

/// How much energy a sound carries at one frequency — one bin of a
/// discrete Fourier transform, done the direct way. Forty of these is a
/// coarse spectrum, which is all any question here needs.
pub(crate) fn bin(s: &[f32], hz: f32) -> f32 {
    let w = 2.0 * std::f32::consts::PI * hz / 44_100.0;
    let (mut re, mut im) = (0.0f32, 0.0f32);
    for (i, x) in s.iter().enumerate() {
        re += x * (w * i as f32).cos();
        im += x * (w * i as f32).sin();
    }
    (re * re + im * im).sqrt() / s.len() as f32
}

/// The frequency a sound sits on and how much it stands out. Not zero
/// crossings: a hit is two thirds hiss, and noise swamps the thump's pitch.
pub(crate) fn dominant(wav: &[u8], lo: f32, hi: f32) -> (f32, f32) {
    let s = wav_samples(wav);
    let n = 40;
    let bins: Vec<(f32, f32)> = (0..n)
        .map(|i| {
            let hz = lo * (hi / lo).powf(i as f32 / (n - 1) as f32);
            (hz, bin(&s, hz))
        })
        .collect();
    let mean = bins.iter().map(|b| b.1).sum::<f32>() / n as f32;
    let peak = bins.iter().cloned().fold((0.0, 0.0), |a, b| if b.1 > a.1 { b } else { a });
    (peak.0, peak.1 / mean.max(1e-9))
}

pub(crate) fn asset(name: &str) -> Vec<u8> {
    blip_assets::brawler::generate().into_iter()
        .find(|(n, _)| n.ends_with(name))
        .unwrap_or_else(|| panic!("no asset {name}")).1
}

#[test]
pub(crate) fn a_combo_climbs_as_it_lands() {
    // Three light hits, each brighter, picked by the combo count, so a
    // four-hit string sounds like one.
    let b: Vec<f32> = ["hit_light.wav", "hit_light2.wav", "hit_light3.wav"]
        .iter().map(|n| dominant(&asset(n), 150.0, 1600.0).0).collect();
    assert!(b[1] > b[0] * 1.15 && b[2] > b[1] * 1.15,
        "the three light hits do not climb: {b:?}");
}

#[test]
pub(crate) fn a_body_hitting_the_floor_does_not_sound_like_a_jab() {
    // A knockdown is the biggest thing in a round: its own sound, not a
    // heavy hit's.
    let jab = asset("hit_light.wav");
    let heavy = asset("hit_heavy.wav");
    let crunch = asset("crunch.wav");
    assert_ne!(crunch, heavy, "a knockdown still plays the heavy hit");
    // Lower than both, and longer than both: that is what makes it read
    // as weight rather than as speed.
    let (c_hz, _) = dominant(&crunch, 80.0, 1600.0);
    let (h_hz, _) = dominant(&heavy, 80.0, 1600.0);
    assert!(c_hz < h_hz,
        "the crunch ({c_hz:.0}Hz) is not lower than a heavy hit ({h_hz:.0}Hz)");
    assert!(brightness(&crunch) < brightness(&jab) * 0.9,
        "the crunch is nearly as bright as a jab");
    assert!(wav_seconds(&crunch) > wav_seconds(&heavy) * 1.2,
        "the crunch is no longer than a heavy hit");
}

#[test]
pub(crate) fn the_crowd_reacts_for_longer_than_it_takes_to_notice() {
    // Thirty-odd onlookers have been drawn watching every round in
    // silence. A cheer that is over before the eye reaches them is the
    // same as no cheer.
    let cheer = blip_assets::brawler::crowd_wav(false);
    let roar = blip_assets::brawler::crowd_wav(true);
    assert!(wav_seconds(&cheer) > 0.6, "the knockdown cheer is {:.2}s", wav_seconds(&cheer));
    assert!(wav_seconds(&roar) > wav_seconds(&cheer) * 1.5,
        "a KO gets no more of a reaction than a knockdown");
    // It has to be a room, not a chord: noise is flat, so no one
    // frequency in it should stand far above its neighbours the way a
    // hit's thump does.
    let (_, crowd_peak) = dominant(&roar, 200.0, 4000.0);
    let (_, hit_peak) = dominant(&asset("hit_heavy.wav"), 200.0, 4000.0);
    assert!(crowd_peak < hit_peak,
        "the crowd has a pitch in it ({crowd_peak:.1}x over flat, against a hit's \
         {hit_peak:.1}x) — that is a chord, not a room");
    // And it must not clip: it plays under a hit and a bell.
    let peak = wav_samples(&roar).iter().fold(0.0f32, |m, s| m.max(s.abs()));
    assert!(peak < 0.99, "the crowd clips at {peak:.3}");
}

#[test]
pub(crate) fn building_the_crowd_is_quick_enough_to_do_at_startup() {
    // Synthesised on the device rather than shipped — a quarter of a
    // megabyte that never crosses the wire — which is only a good trade
    // if the player does not wait for it.
    let t = std::time::Instant::now();
    std::hint::black_box(blip_assets::brawler::crowd_wav(false));
    std::hint::black_box(blip_assets::brawler::crowd_wav(true));
    let ms = t.elapsed().as_millis();
    assert!(ms < 400, "the crowd took {ms}ms to build");
}
