//! Bubbler's synthesized music and sound effects.

use std::f32::consts::PI;

use crate::cosy::{midi_hz, pulse, SR};
use crate::wav::{encode_pcm16_mono, soft_limit_to_pcm16, Rng, MIX_KNEE};
use crate::Asset;

#[path = "bubbler/gfx.rs"]
pub mod gfx;
#[path = "bubbler/tunes.rs"]
pub mod tunes;
pub use tunes::{hurry_wav, jingle_wav, theme_wav};

pub fn generate() -> Vec<Asset> {
    gfx::generate()
}

/// Effects. `which`: 0 blow a bubble, 1 pop, 2 trap an enemy, 3 an enemy
/// defeated (a bright chirp), 4 fruit, 5 jump, 6 player lost, 7 bounce on
/// a bubble, 8 extra points sparkle.
pub fn sfx_wav(which: usize) -> Vec<u8> {
    let mut rng = Rng(0x5EED_0001 + which as u32 * 977);
    let dur = match which {
        0 => 0.16,
        1 => 0.12,
        2 => 0.3,
        3 => 0.35,
        4 => 0.22,
        5 => 0.18,
        6 => 1.1,
        7 => 0.16,
        _ => 0.4,
    };
    let n = (dur * SR) as usize;
    let mut buf = vec![0.0f32; n];
    let mut ph = 0.0f32;
    for (i, out) in buf.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let k = t / dur;
        let w = rng.next_f32() * 2.0 - 1.0;
        *out = match which {
            // blow: a round wet "bloop" rising, with a little air
            0 => {
                ph += (260.0 + 520.0 * k * k) / SR;
                (2.0 * PI * ph).sin() * (1.0 - k).powf(1.5) * 0.8 + w * 0.08 * (1.0 - k)
            }
            // pop: a bright snap
            1 => {
                ph += (1800.0 - 1200.0 * k) / SR;
                (2.0 * PI * ph).sin() * (-t / 0.025).exp() + w * (-t / 0.008).exp() * 0.6
            }
            // trap: a wobbly rising "bwoing"
            2 => {
                ph += (330.0 + 330.0 * k + 40.0 * (2.0 * PI * 18.0 * t).sin()) / SR;
                pulse(ph, 0.3) * (1.0 - k) * 0.55
            }
            // defeated: a quick two-step chirp up
            3 => {
                let f = if k < 0.4 { 880.0 } else { 1320.0 };
                ph += f / SR;
                pulse(ph, 0.25) * (1.0 - k) * 0.5
            }
            // fruit: a bell ding
            4 => {
                ph += 1568.0 / SR;
                ((2.0 * PI * ph).sin() + 0.4 * (2.0 * PI * ph * 2.0).sin())
                    * (-t / 0.09).exp()
                    * 0.6
            }
            // jump: a springy upward sweep
            5 => {
                ph += (300.0 + 600.0 * k) / SR;
                pulse(ph, 0.5) * (1.0 - k) * 0.35
            }
            // lost a life: a sad falling wobble
            6 => {
                ph += (620.0 * (1.0 - 0.7 * k) + 30.0 * (2.0 * PI * 7.0 * t).sin()) / SR;
                pulse(ph, 0.4) * (1.0 - k).powf(0.8) * 0.45
            }
            // bounce on a bubble: a soft rubbery boing
            7 => {
                ph += (220.0 + 260.0 * (-t / 0.04).exp()) / SR;
                (2.0 * PI * ph).sin() * (1.0 - k) * 0.7
            }
            // sparkle: rising glassy pings
            _ => {
                let step = (k * 6.0) as i32;
                ph += midi_hz(84 + step * 3) / SR;
                (2.0 * PI * ph).sin() * (-(t % (dur / 6.0)) / 0.05).exp() * 0.5
            }
        };
    }
    let scaled: Vec<f32> = buf.iter().map(|v| v * 20_000.0).collect();
    encode_pcm16_mono(&soft_limit_to_pcm16(&scaled, MIX_KNEE))
}
