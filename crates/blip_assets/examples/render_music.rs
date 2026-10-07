//! Render every game's music to WAV and report synthesis time.
//! Run with `cargo run --release -p blip_assets --example render_music -- /tmp/music`.

use std::time::Instant;

use blip_assets::{bouncer, brawler, bubbler, galactic_defender, meteors, rally, serpent, sky_raider};

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "music".to_string());
    std::fs::create_dir_all(&dir).expect("create the output directory");
    let tunes: &[(&str, fn() -> Vec<u8>)] = &[
        ("serpent_slither", serpent::slither_wav),
        ("serpent_frenzy", serpent::frenzy_wav),
        ("bouncer_bounce", bouncer::bounce_wav),
        ("bouncer_rebound", bouncer::rebound_wav),
        ("defender_invasion", galactic_defender::invasion_wav),
        ("defender_mothership", galactic_defender::mothership_wav),
        ("meteors_drift", meteors::drift_wav),
        ("meteors_storm", meteors::storm_wav),
        ("rally_1", rally::music),
        ("rally_2", rally::music2),
        ("rally_3", rally::music3),
        ("rally_4", rally::music4),
        ("rally_5", rally::music5),
        ("raider_1", sky_raider::music),
        ("raider_2", sky_raider::music2),
        ("brawler_dock", || brawler::theme_wav(0)),
        ("brawler_temple", || brawler::theme_wav(1)),
        ("brawler_select", || brawler::theme_wav(2)),
        ("brawler_airbase", || brawler::theme_wav(3)),
        ("brawler_bathhouse", || brawler::theme_wav(4)),
        ("brawler_village", || brawler::theme_wav(5)),
        ("brawler_fortress", || brawler::theme_wav(6)),
        ("brawler_rooftop", || brawler::theme_wav(7)),
        ("brawler_crowd", || brawler::ambience_wav(0)),
        ("brawler_wind", || brawler::ambience_wav(1)),
        ("brawler_crickets", || brawler::ambience_wav(2)),
        ("brawler_city", || brawler::ambience_wav(3)),
        ("bubbler_theme", bubbler::theme_wav),
        ("bubbler_hurry", bubbler::hurry_wav),
    ];
    for (name, make) in tunes {
        let t = Instant::now();
        let wav = make();
        let took = t.elapsed();
        let path = format!("{dir}/{name}.wav");
        std::fs::write(&path, &wav).expect("write the WAV");
        println!("{path:40} {:>6} KB  rendered in {took:.0?}", wav.len() / 1024);
    }
}
