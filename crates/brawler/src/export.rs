//! Capture the fighters' pose sheets, which the film poster is cut from, from the
//! same bodies and poses the game draws.

use super::*;
use blip::macroquad::prelude::{
    clear_background, render_target, set_camera, set_default_camera, Camera2D, Rect,
};
use std::path::Path;

const CELL_W: f32 = 320.0;
const CELL_H: f32 = 256.0;
const FEET: f32 = 244.0;
const FRAMES: usize = 6;

/// The rows of a pose sheet; move names and timing come from `moves.rs`.
const POSES: [(Act, MoveId); 19] = [
    (Act::Idle, MoveId::LowPunch),
    (Act::Walk, MoveId::LowPunch),
    (Act::Crouch, MoveId::LowPunch),
    (Act::Block, MoveId::LowPunch),
    (Act::Hitstun, MoveId::LowPunch),
    (Act::Knockdown, MoveId::LowPunch),
    (Act::Victory, MoveId::LowPunch),
    (Act::Attack, MoveId::LowPunch),
    (Act::Attack, MoveId::HighPunch),
    (Act::Attack, MoveId::LowKick),
    (Act::Attack, MoveId::HighKick),
    (Act::Attack, MoveId::Special),
    (Act::Air, MoveId::LowPunch),
    (Act::Attack, MoveId::JumpKick),
    (Act::Attack, MoveId::Sweep),
    (Act::Attack, MoveId::Uppercut),
    (Act::Knockdown, MoveId::LowPunch),
    (Act::Walk, MoveId::LowPunch),
    (Act::Attack, MoveId::HighKick),
];

pub(crate) fn pose_art(blip: &Blip, folder: &Path) {
    std::fs::create_dir_all(folder).expect("art folder");
    for (who, arch) in FIGHTERS.iter().enumerate() {
        let target = render_target((CELL_W as usize * FRAMES) as u32, CELL_H as u32 * POSES.len() as u32);
        let mut camera = Camera2D::from_display_rect(Rect::new(0.0, 0.0, target.texture.width(), target.texture.height()));
        camera.render_target = Some(target.clone());
        set_camera(&camera);
        clear_background(BlipColor { r: 0.0, g: 0.0, b: 0.0, a: 0.0 });
        for row in 0..POSES.len() {
            for frame in 0..FRAMES {
                let f = sample(who, row, frame);
                // Phase comes from distance walked; the picture stays on its cell's anchor.
                let shift = CELL_W * (frame as f32 + 0.5) - f.x;
                camera.target.x -= shift;
                set_camera(&camera);
                draw::draw_body(blip, &f, f.t, CELL_H * row as f32 + FEET - f.y, 0.0, 0, false);
                camera.target.x += shift;
            }
        }
        set_default_camera();
        let raw = target.texture.get_texture_data();
        let (w, h, png) = pack(&raw);
        std::fs::write(folder.join(format!("{}.png", arch.name.to_ascii_lowercase())), png).expect("fighter PNG");
        println!("[art] {}: {w}x{h} cells, {FRAMES} frames", arch.name);
    }
}

/// One tight, symmetric crop for every cell preserves the feet and mirroring anchors.
fn pack(raw: &blip::macroquad::texture::Image) -> (u32, u32, Vec<u8>) {
    let (cw, ch) = (CELL_W as usize, CELL_H as usize);
    let mut half = 0usize;
    let (mut top, mut bottom) = (ch, 0usize);
    let offset = |x: usize, y: usize| ((raw.height as usize - 1 - y) * raw.width as usize + x) * 4;
    for y in 0..raw.height as usize {
        for x in 0..raw.width as usize {
            if raw.bytes[offset(x, y) + 3] == 0 { continue; }
            half = half.max((x % cw).abs_diff(cw / 2) + 2);
            top = top.min(y % ch);
            bottom = bottom.max(y % ch);
        }
    }
    half = half.min(cw / 2);
    top = top.saturating_sub(2);
    bottom = (bottom + 2).min(ch - 1);
    let (w, h) = (half * 2, bottom - top + 1);
    let left = cw / 2 - half;
    let mut packed = blip::image::Image::new((w * FRAMES) as u32, (h * POSES.len()) as u32);
    for row in 0..POSES.len() {
        for frame in 0..FRAMES {
            for y in 0..h {
                for x in 0..w {
                    let source = offset(frame * cw + left + x, row * ch + top + y);
                    let dest = ((row * h + y) * w * FRAMES + frame * w + x) * 4;
                    packed.px[dest..dest + 4].copy_from_slice(&raw.bytes[source..source + 4]);
                }
            }
        }
    }
    (w as u32, h as u32, packed.encode_png())
}

/// One cell of a pose sheet: fighter `who` in pose `row` at `frame`.
fn sample(who: usize, row: usize, frame: usize) -> Fighter {
    let (act, mv) = POSES[row];
    let arch = FIGHTERS[who];
    let mut f = Fighter::new(who, 0.0, 1.0);
    f.y = FLOOR_Y;
    f.act = act;
    f.shown = act;
    f.mv = mv;
    f.shown_mv = mv;
    if row == 18 { f.alt_kick = true; }
    let m = f.scaled(move_data(mv));
    f.t = match act {
        Act::Attack => [m.startup * 0.15, m.startup * 0.55, m.startup * 0.9,
            m.startup + m.active * 0.5, m.startup + m.active + m.recovery * 0.3,
            m.startup + m.active + m.recovery * 0.7][frame] * F,
        Act::Knockdown if row == 16 => 0.86 + frame as f32 * 0.05,
        Act::Knockdown => [0.0, 0.05, 0.10, 0.16, 0.23, 0.32][frame],
        Act::Victory => frame as f32 * 0.24,
        Act::Idle => 0.2 + frame as f32 / FRAMES as f32 * std::f32::consts::TAU / draw::STYLES[who].rate,
        _ => frame as f32 * 0.07,
    };
    if act == Act::Air || mv == MoveId::JumpKick {
        f.y -= 1.0;
        f.vy = -500.0 + frame as f32 * 200.0;
    }
    if act == Act::Walk {
        let phase = frame + if row == 17 { FRAMES } else { 0 };
        f.x = phase as f32 / (FRAMES * 2) as f32 * draw::STEP * 4.0 * arch.size;
    }
    f
}
