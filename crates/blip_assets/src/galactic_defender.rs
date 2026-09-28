//! Galactic Defender assets: pixel-art aliens and saucer, techno music, and
//! effects on a warbling theremin (see `cosy`).

use crate::image::Image;
use crate::techno::{warm, 
    bass_note, clap, hat, kick, lead_stab, open_hat, phrase_note, sidechain_duck, supersaw, Rng, MIX_KNEE,
};
use crate::wav::{encode_pcm16_mono, soft_limit_to_pcm16, SAMPLE_RATE};
use crate::cosy::{self, Tone, Voice, H};
use crate::Asset;

// Must match crates/galactic_defender/src/main.rs's ALIEN_W / ALIEN_H.
const ALIEN_W: i32 = 36;
const ALIEN_H: i32 = 28;
// Must match crates/galactic_defender/src/main.rs's UFO_W / UFO_H.
const UFO_W: i32 = 36;
const UFO_H: i32 = 20;

fn player_ship() -> Vec<u8> {
    let w: i32 = 32;
    let h: i32 = 28;
    let mut img = Image::new(w as u32, h as u32);
    let cx = w / 2;
    for y in 0..h {
        for x in 0..w {
            let top_half = y as f32 / h as f32;
            let half_w = (1.0 + top_half * (w as f32 / 2.0 - 1.0)) as i32;
            if (x - cx).abs() <= half_w && y > h / 4 {
                img.set(x, y, 0, 200, 200);
            }
            if (x - cx).abs() <= 2 && y <= h / 4 + 2 {
                img.set(x, y, 0, 220, 255);
            }
            if y == h - 1 && (x - cx).abs() <= 4 && (x - cx).abs() >= 2 {
                img.set(x, y, 255, 100, 0);
            }
        }
    }
    img.set(cx, h / 2 - 2, 180, 230, 255);
    img.set(cx - 1, h / 2 - 1, 100, 180, 255);
    img.set(cx + 1, h / 2 - 1, 100, 180, 255);
    img.encode_png()
}

/// Turn a 9-character '0'/'1' string into a 9-bit row mask (MSB = leftmost).
fn row(s: &str) -> u16 {
    let mut v = 0u16;
    for (i, c) in s.bytes().enumerate() {
        if c == b'1' {
            v |= 1 << (8 - i);
        }
    }
    v
}

/// An alien glyph: a 9x8 bitmap with a second frame per kind (mandibles,
/// claws, tentacles shift) for the two-frame march. `frame` is 0 or 1.
fn alien(kind: usize, frame: usize) -> Vec<u8> {
    let w: i32 = ALIEN_W;
    let h: i32 = ALIEN_H;
    let mut img = Image::new(w as u32, h as u32);
    let (r, g, b) = match kind {
        0 => (255u8, 80, 255),
        1 => (0,    230, 230),
        _ => (110,  255, 110),
    };
    // [frame A, frame B] — rows 0..4 (head/eyes/mandibles) stay put, rows
    // 5..7 (legs / claws / tentacles) swap between frames.
    let patterns: [[u16; 8]; 2] = match kind {
        0 => [
            // Squid: antenna nubs, jagged toothy mandible, legs together.
            [row("001000100"), row("001111100"), row("011111110"),
             row("111111111"), row("110111011"), row("011111110"),
             row("101010101"), row("000101000")],
            // ...antenna twitch inward, legs thrown wide apart.
            [row("000101000"), row("001111100"), row("011111110"),
             row("111111111"), row("110111011"), row("011111110"),
             row("100000001"), row("010000010")],
        ],
        1 => [
            // Crab: flat head, ridged shell, pincers tucked in close.
            [row("000000000"), row("000111000"), row("001111100"),
             row("011111110"), row("111111111"), row("110111011"),
             row("010000010"), row("000101000")],
            // ...pincers thrown wide open, gnarly.
            [row("000000000"), row("000111000"), row("001111100"),
             row("011111110"), row("111111111"), row("110111011"),
             row("100000001"), row("001000100")],
        ],
        _ => [
            // Octopus: round head, tentacle skirt waving one way...
            [row("000000000"), row("000111000"), row("001111100"),
             row("011111110"), row("111111111"), row("011111110"),
             row("101010101"), row("010101010")],
            // ...and the other, for a wavy crawl.
            [row("000000000"), row("000111000"), row("001111100"),
             row("011111110"), row("111111111"), row("011111110"),
             row("010101010"), row("101010101")],
        ],
    };
    let pattern = patterns[frame];
    let cell = 3;
    let cols = 9;
    let rows = 8;
    let ox = (w - cols * cell) / 2;
    let oy = (h - rows * cell) / 2;
    for ry in 0..rows {
        for cx in 0..cols {
            if pattern[ry as usize] & (1 << (cols - 1 - cx)) != 0 {
                let px_x = ox + cx * cell;
                let px_y = oy + ry * cell;
                for dy in 0..cell {
                    for dx in 0..cell {
                        img.set(px_x + dx, px_y + dy, r, g, b);
                    }
                }
            }
        }
    }
    // Eyes on the wide head row: a bright square in a dark socket with a dark
    // pupil, darting with the frame.
    let eye_y = oy + 3 * cell;
    let eye_dx = if frame == 1 { 1 } else { 0 };
    let eye_size = cell + 1;
    for ex in [ox + 3 * cell - eye_dx, ox + 5 * cell + eye_dx] {
        for dy in -1..=eye_size {
            for dx in -1..=eye_size {
                img.set(ex + dx, eye_y + dy, 10, 10, 10);
            }
        }
        for dy in 0..eye_size {
            for dx in 0..eye_size {
                img.set(ex + dx, eye_y + dy, 255, 255, 255);
            }
        }
        let pupil_ox = 1 + eye_dx;
        for dy in 1..=2 {
            for dx in 0..=1 {
                img.set(ex + pupil_ox + dx, eye_y + dy, 15, 15, 20);
            }
        }
    }
    img.encode_png()
}

/// A flying saucer: metal disc, glass dome, a ring of rim lights. Always
/// drawn upright (rotated pixel art looks jagged); `frame` (0..N_LIGHTS)
/// steps the light ring, so cycling the frames spins the lights.
const UFO_N_LIGHTS: usize = 8;

fn ufo_saucer(frame: usize) -> Vec<u8> {
    let w: i32 = UFO_W;
    let h: i32 = UFO_H;
    let mut img = Image::new(w as u32, h as u32);
    let cx = w as f32 / 2.0;
    let cy = h as f32 * 0.62;
    let disc_rx = w as f32 / 2.0 - 1.0;
    let disc_ry = h as f32 * 0.30;

    // Disc body — a squashed metallic-red ellipse with a darker underside band.
    for y in 0..h {
        for x in 0..w {
            let dx = (x as f32 + 0.5 - cx) / disc_rx;
            let dy = (y as f32 + 0.5 - cy) / disc_ry;
            let d2 = dx * dx + dy * dy;
            if d2 <= 1.0 {
                if dy > 0.25 {
                    img.set(x, y, 150, 30, 40); // shadowed underside
                } else {
                    img.set(x, y, 220, 60, 70);
                }
            }
        }
    }

    // Dome on top.
    let dome_cy = cy - disc_ry * 0.9;
    let dome_r = w as f32 * 0.22;
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - dome_cy;
            if dx * dx + dy * dy <= dome_r * dome_r && (y as f32) < cy - disc_ry * 0.35 {
                img.set(x, y, 120, 230, 255);
            }
        }
    }
    img.set(cx as i32 - 1, (dome_cy - dome_r * 0.4) as i32, 220, 250, 255);

    // Rim lights, evenly spaced and alternating colour, offset by `frame`
    // so the ring appears to rotate as frames advance.
    for i in 0..UFO_N_LIGHTS {
        let idx = i + frame;
        let a = (idx as f32 / UFO_N_LIGHTS as f32) * std::f32::consts::PI * 2.0;
        let lx = (cx + a.cos() * disc_rx * 0.88).round() as i32;
        let ly = (cy + a.sin() * disc_ry * 0.88).round() as i32;
        let (r, g, b) = if idx % 2 == 0 { (255u8, 230, 60) } else { (255, 255, 255) };
        img.set(lx, ly, r, g, b);
        img.set(lx, ly - 1, r, g, b);
    }
    img.encode_png()
}

/// The saucer's call: a theremin warbling round one note, 1.2 s.
fn ufo_siren() -> Vec<u8> {
    cosy::sfx(&cosy::glide(1.2, 560.0, 560.0, Tone::Sine, 0.25, 70.0))
}

/// The saucer charging: a theremin sliding up two octaves, trembling
/// harder as it nears the top, 1.6 s.
fn laser_charge_sfx() -> Vec<u8> {
    let rise = cosy::glide(1.6, 180.0, 720.0, Tone::Sine, 0.1, 18.0);
    let under = cosy::glide(1.6, 90.0, 360.0, Tone::Tri, 0.1, 0.0);
    cosy::sfx(&cosy::mix(rise, &under, 0.0, 0.4))
}

/// The beam: a warbling theremin falling away over a low triangle, 0.55 s.
fn laser_blast_sfx() -> Vec<u8> {
    let fall = cosy::glide(0.55, 900.0, 220.0, Tone::Sine, 1.0, 45.0);
    let low = cosy::glide(0.55, 110.0, 80.0, Tone::Tri, 0.8, 0.0);
    cosy::sfx(&cosy::mix(fall, &low, 0.0, 0.6))
}

/// One march step: a soft triangle thud falling onto `base_freq`. Four
/// descending ones cycle as the aliens advance.
fn march_thump(base_freq: f32) -> Vec<u8> {
    cosy::finish_warm(cosy::glide(0.16, base_freq * 1.4, base_freq, Tone::Tri, 1.4, 0.0), 24_000.0)
}

fn bullet() -> Vec<u8> {
    let w: i32 = 8;
    let h: i32 = 16;
    let mut img = Image::new(w as u32, h as u32);
    let cx = w / 2;
    for y in 0..h {
        img.set(cx,     y, 255, 255, 255);
        img.set(cx - 1, y, 200, 200, 200);
        img.set(cx + 1, y, 200, 200, 200);
    }
    img.encode_png()
}

fn explosion() -> Vec<u8> {
    let w: i32 = 32;
    let h: i32 = 32;
    let mut img = Image::new(w as u32, h as u32);
    let cx = w / 2;
    let cy = h / 2;
    let angles = [
        0.0_f32, 0.523, 1.047, 1.571, 2.094, 2.618,
        3.142, 3.665, 4.189, 4.712, 5.236, 5.760,
    ];
    for angle in angles {
        for r in 0..(w / 2 - 1) {
            let x = cx + (r as f32 * angle.cos()) as i32;
            let y = cy + (r as f32 * angle.sin()) as i32;
            let t = r as f32 / (w as f32 / 2.0);
            let red = (255.0 * (1.0 - t)) as u8;
            let green = (150.0 * (1.0 - t)) as u8;
            img.set(x, y, red, green, 0);
        }
    }
    for dy in -2..=2 {
        for dx in -2..=2 {
            img.set(cx + dx, cy + dy, 255, 255, 200);
        }
    }
    img.encode_png()
}

fn shield_block() -> Vec<u8> {
    let w: i32 = 12;
    let h: i32 = 12;
    let mut img = Image::new(w as u32, h as u32);
    for y in 0..h {
        for x in 0..w {
            img.set(x, y, 0, 180, 0);
        }
    }
    for x in 0..w {
        img.set(x, 0, 100, 255, 100);
    }
    img.encode_png()
}

/// Title loop: a catchy 138 BPM trance groove, one hook riff over an Am-F
/// vamp (answered on each fourth bar, phrase_note), four-on-the-floor with
/// the sidechain pump, no breakdown so the hook is heard at once. The back
/// half adds an octave-up harmony and busier percussion.
fn music() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let bpm = 138.0_f32;
    let bars = 16;
    let lift_bar = bars / 2;
    let steps_per_bar = 16;
    let total_steps = bars * steps_per_bar;
    let step_ms = 60_000.0 / bpm / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * total_steps + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0xD0D0_1000);
    let mut kick_offsets = Vec::new();

    // Am - F, a two-chord vamp, one root every 2 bars.
    let bass_roots = [110.00_f32, 87.31]; // A2, F2
    const BASS_HIT: [bool; 16] = [
        true, false, false, true, false, false, true, false,
        true, false, false, true, false, true, false, false,
    ];
    // The hook: one 4-note riff on the beat, identical every single bar —
    // repetition is what makes a hook catchy.
    const HOOK: [f32; 4] = [440.00, 523.25, 659.25, 523.25]; // A4 C5 E5 C5

    for step in 0..total_steps {
        let bar = step / steps_per_bar;
        let pos = step % steps_per_bar;
        let off = step * step_samples;
        let lifted = bar >= lift_bar;

        if pos % 4 == 0 {
            kick_offsets.push(off);
        }
        if pos == 4 || pos == 12 {
            clap(&mut buf, off, &mut rng, 0.42);
        }
        if lifted && pos == 8 {
            clap(&mut buf, off, &mut rng, 0.30);
        }
        if pos % 2 == 1 {
            hat(&mut buf, off, &mut rng, 0.20);
        }
        if pos == 14 || (lifted && pos == 6) {
            open_hat(&mut buf, off, &mut rng, 0.14);
        }
        if BASS_HIT[pos] {
            let root = bass_roots[(bar / 2) % bass_roots.len()];
            bass_note(&mut buf, off, root, step_ms * 0.7, 0.58);
        }
        if pos % 4 == 0 {
            supersaw(&mut buf, off, phrase_note(&HOOK, bar, pos / 4), step_ms * 3.5, 0.22, 8.0, 0.008);
            if lifted {
                supersaw(&mut buf, off, phrase_note(&HOOK, bar, pos / 4) * 2.0, step_ms * 3.5, 0.11, 8.0, 0.008);
            }
        }
    }

    // Sidechain-duck the bass/hook under each kick, then lay the kicks in on
    // top — the pumping four-on-the-floor feel of a real trance mix.
    sidechain_duck(&mut buf, &kick_offsets, 0.55, step_ms * 0.85);
    for &off in &kick_offsets {
        kick(&mut buf, off, 0.9);
    }

    warm(&mut buf);
    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// Fast pursuit loop — the same idea, harder and faster: one driving 8th-note
/// hook repeated every bar over a 140 BPM Dm-Bb groove. The back half
/// (~every 30s) adds a 16th-note hat roll and an octave-up hook harmony.
fn music2() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let bpm = 140.0_f32;
    let bars = 18;
    let lift_bar = bars / 2;
    let steps_per_bar = 16;
    let total_steps = bars * steps_per_bar;
    let step_ms = 60_000.0 / bpm / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * total_steps + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0xD0D0_2000);
    let mut kick_offsets = Vec::new();

    // Dm - Bb, one root every 2 bars.
    let bass_roots = [146.83_f32, 116.54]; // D3, Bb2
    const BASS_HIT: [bool; 16] = [
        true, true, false, true, true, false, true, true,
        false, true, true, false, true, true, false, true,
    ];
    // The hook: one 8th-note riff, identical every bar.
    const HOOK: [f32; 8] = [
        587.33, 698.46, 880.00, 698.46, 587.33, 698.46, 880.00, 698.46, // D5 F5 A5 F5 ...
    ];

    for step in 0..total_steps {
        let bar = step / steps_per_bar;
        let pos = step % steps_per_bar;
        let off = step * step_samples;
        let lifted = bar >= lift_bar;

        if pos % 4 == 0 {
            kick_offsets.push(off);
        }
        if pos == 4 || pos == 12 {
            clap(&mut buf, off, &mut rng, 0.5);
        }
        if pos % 2 == 1 || (lifted && pos % 2 == 0) {
            hat(&mut buf, off, &mut rng, if pos % 2 == 1 { 0.26 } else { 0.14 });
        }
        if pos == 6 || pos == 14 {
            open_hat(&mut buf, off, &mut rng, 0.18);
        }
        if BASS_HIT[pos] {
            let root = bass_roots[(bar / 2) % bass_roots.len()];
            bass_note(&mut buf, off, root, step_ms * 0.6, 0.62);
        }
        if pos % 2 == 0 {
            supersaw(&mut buf, off, phrase_note(&HOOK, bar, pos / 2), step_ms * 1.7, 0.20, 5.0, 0.009);
            if lifted {
                supersaw(&mut buf, off, phrase_note(&HOOK, bar, pos / 2) * 2.0, step_ms * 1.7, 0.10, 5.0, 0.009);
            }
        }
    }

    sidechain_duck(&mut buf, &kick_offsets, 0.55, step_ms * 0.85);
    for &off in &kick_offsets {
        kick(&mut buf, off, 0.95);
    }

    warm(&mut buf);
    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// Slow, dark dread loop: sparse kick, a deep sub-bass drone and an ominous
/// two-note motif each bar (~28.8 s at 100 BPM, 12 bars). The back half adds
/// a wide supersaw wash.
fn music3() -> Vec<u8> {
    let sr = SAMPLE_RATE as f32;
    let bpm = 100.0_f32;
    let bars = 12;
    let lift_bar = bars / 2;
    let steps_per_bar = 16;
    let total_steps = bars * steps_per_bar;
    let step_ms = 60_000.0 / bpm / 4.0;
    let step_samples = (sr * step_ms / 1000.0) as usize;
    let total = step_samples * total_steps + SAMPLE_RATE as usize / 4;
    let mut buf = vec![0f32; total];
    let mut rng = Rng(0xD0D0_3000);
    let mut kick_offsets = Vec::new();

    // A1 - Bb1 sub-bass drone, one long note every 2 bars.
    let drone_roots = [55.00_f32, 58.27];
    const KICK_HIT: [bool; 16] = [
        true, false, false, false, false, false, true, false,
        false, false, true, false, false, false, false, false,
    ];
    // The motif: a flat minor-second dyad, same every bar — dread's version
    // of a hook, memorable because it never changes.
    const MOTIF: [f32; 2] = [220.00, 233.08]; // A3, Bb3

    for step in 0..total_steps {
        let bar = step / steps_per_bar;
        let pos = step % steps_per_bar;
        let off = step * step_samples;
        let lifted = bar >= lift_bar;

        if KICK_HIT[pos] {
            kick_offsets.push(off);
        }
        if pos == 8 {
            open_hat(&mut buf, off, &mut rng, 0.10);
        }
        if pos == 0 {
            let root = drone_roots[(bar / 2) % drone_roots.len()];
            bass_note(&mut buf, off, root, step_ms * 8.0, 0.54);
        }
        if pos == 8 {
            lead_stab(&mut buf, off, MOTIF[bar % 2], step_ms * 6.0, 0.14);
            if lifted {
                supersaw(&mut buf, off, MOTIF[bar % 2] * 2.0, step_ms * 6.0, 0.10, 400.0, 0.02);
            }
        }
    }

    sidechain_duck(&mut buf, &kick_offsets, 0.5, step_ms * 3.0);
    for &off in &kick_offsets {
        kick(&mut buf, off, 0.8);
    }

    warm(&mut buf);
    encode_pcm16_mono(&soft_limit_to_pcm16(&buf, MIX_KNEE))
}

/// Game over: the theremin walking down A minor, slowly.
fn game_over_sfx() -> Vec<u8> {
    cosy::jingle_with(&[81, H, 77, H, 76, H, 72, H, 69, H, H, H], 0.14, Voice::Theremin, -1, Some((Voice::Bell, -2)))
}

/// Wave cleared: the theremin climbing A minor to a held note.
fn level_clear_sfx() -> Vec<u8> {
    cosy::jingle_with(&[69, 72, 76, 81, H, 79, 81, H, H], 0.1, Voice::Theremin, -1, Some((Voice::Bell, -2)))
}

/// The player's shot: a quick theremin "pew" falling an octave.
fn shoot_sfx() -> Vec<u8> {
    cosy::finish_warm(cosy::glide(0.11, 1300.0, 620.0, Tone::Sine, 1.2, 25.0), 15_000.0)
}

/// An alien hit: a soft pop and a bloop tumbling down, no hiss.
fn explosion_sfx() -> Vec<u8> {
    let pop = cosy::snap(0.06, 900.0, 0xDEF1);
    cosy::sfx(&cosy::mix(pop, &cosy::bloop(0.3, 360.0, 80.0, 0xDEF2), 0.02, 0.9))
}

pub fn generate() -> Vec<Asset> {
    vec![
        ("images/player_ship.png",     player_ship()),
        ("images/alien_squid_a.png",   alien(0, 0)),
        ("images/alien_squid_b.png",   alien(0, 1)),
        ("images/alien_crab_a.png",    alien(1, 0)),
        ("images/alien_crab_b.png",    alien(1, 1)),
        ("images/alien_octopus_a.png", alien(2, 0)),
        ("images/alien_octopus_b.png", alien(2, 1)),
        ("images/bullet.png",        bullet()),
        ("images/explosion.png",     explosion()),
        ("images/shield_block.png",  shield_block()),
        ("images/ufo_saucer_0.png",  ufo_saucer(0)),
        ("images/ufo_saucer_1.png",  ufo_saucer(1)),
        ("images/ufo_saucer_2.png",  ufo_saucer(2)),
        ("images/ufo_saucer_3.png",  ufo_saucer(3)),
        ("images/ufo_saucer_4.png",  ufo_saucer(4)),
        ("images/ufo_saucer_5.png",  ufo_saucer(5)),
        ("images/ufo_saucer_6.png",  ufo_saucer(6)),
        ("images/ufo_saucer_7.png",  ufo_saucer(7)),
        ("sounds/shoot.wav",       shoot_sfx()),
        ("sounds/explosion.wav",   explosion_sfx()),
        ("sounds/game_over.wav",   game_over_sfx()),
        ("sounds/march1.wav",      march_thump(98.0)),
        ("sounds/march2.wav",      march_thump(87.0)),
        ("sounds/march3.wav",      march_thump(78.0)),
        ("sounds/march4.wav",      march_thump(70.0)),
        ("sounds/level_clear.wav", level_clear_sfx()),
        ("sounds/ufo_siren.wav",   ufo_siren()),
        ("sounds/laser_charge.wav", laser_charge_sfx()),
        ("sounds/laser_blast.wav",  laser_blast_sfx()),
        ("sounds/music.wav",       music()),
        ("sounds/music2.wav",      music2()),
        ("sounds/music3.wav",      music3()),
    ]
}
