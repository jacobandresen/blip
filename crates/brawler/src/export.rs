//! Capture BASIC's art from the same bodies, poses and stages the game draws.

use super::*;
use blip::macroquad::prelude::{
    clear_background, render_target, set_camera, set_default_camera, Camera2D, Rect,
};
use std::path::Path;

const CELL_W: f32 = 320.0;
const CELL_H: f32 = 256.0;
const FEET: f32 = 244.0;
pub(crate) const FRAMES: usize = 6;

/// Row order is BASIC's pose table; move names and timing come from `moves.rs`.
pub(crate) const POSES: [(Act, MoveId); 19] = [
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

pub(crate) fn tape_art(blip: &Blip, folder: &Path) {
    std::fs::create_dir_all(folder).expect("BASIC art folder");
    let mut metadata = Vec::new();
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
        let (w, h, feet, png) = pack(&raw);
        std::fs::write(folder.join(format!("{}.png", arch.name.to_ascii_lowercase())), png).expect("fighter PNG");
        let trim = format!("#{:02X}{:02X}{:02X}",
            (arch.trim.0 * 255.0).round() as u8,
            (arch.trim.1 * 255.0).round() as u8,
            (arch.trim.2 * 255.0).round() as u8);
        metadata.push(format!("{{\"name\":\"{}\",\"width\":{w},\"height\":{h},\"feet\":{feet},\"frames\":{FRAMES},\"rate\":{},\"bulk\":{},\"size\":{},\"trim\":\"{trim}\",\"boss\":{}}}",
            arch.name, draw::STYLES[who].rate, arch.bulk, arch.size, arch.boss));
        println!("[art] {}: {w}x{h} cells, {FRAMES} frames", arch.name);
    }
    std::fs::write(folder.join("atlas.json"), format!("[{}]\n", metadata.join(","))).expect("atlas metadata");
    portraits(blip, folder);
    for stage in 0..STAGES {
        let target = render_target(WIN_W as u32, WIN_H as u32);
        let mut camera = Camera2D::from_display_rect(Rect::new(0.0, 0.0, WIN_W as f32, WIN_H as f32));
        camera.render_target = Some(target.clone());
        set_camera(&camera);
        clear_background(BLIP_BLACK);
        let g = Game { stage, now: 2.0, ..Game::new() };
        draw::draw_stage(blip, &g);
        set_default_camera();
        target.texture.get_texture_data().export_png(folder.join(format!("stage_{}.png", stage + 1)).to_str().expect("art path"));
        println!("[art] {}", STAGE_NAMES[stage]);
    }
}

fn portraits(blip: &Blip, folder: &Path) {
    let target = render_target(30, 30);
    let mut camera = Camera2D::from_display_rect(Rect::new(0.0, 0.0, 30.0, 30.0));
    camera.render_target = Some(target.clone());
    let mut sheet = blip::image::Image::new(FIGHTERS.len() as u32 * 30, 90);
    for (who, arch) in FIGHTERS.iter().enumerate() {
        for (row, mood) in [draw::Mood::Calm, draw::Mood::Hurt, draw::Mood::Happy].into_iter().enumerate() {
            set_camera(&camera);
            clear_background(blip::shade(draw::rgb(arch.color), 0.45));
            let look = draw::Look { bulk: arch.bulk * 0.82, size: 0.82, wind: (0.0, 0.5), ..draw::look_of(arch, 0.0) };
            let head = draw::V(17.0, 13.0);
            draw::draw_head(blip, &look, 1.0, head, draw::V(15.0, 26.0), mood);
            set_default_camera();
            let raw = target.texture.get_texture_data();
            for y in 0..30usize {
                for x in 0..30usize {
                    let offset = ((29 - y) * 30 + x) * 4;
                    sheet.set_rgba((who * 30 + x) as i32, (row * 30 + y) as i32,
                        raw.bytes[offset], raw.bytes[offset + 1], raw.bytes[offset + 2], raw.bytes[offset + 3]);
                }
            }
        }
    }
    std::fs::write(folder.join("portraits.png"), sheet.encode_png()).expect("portrait sheet");
}

/// One tight, symmetric crop for every cell preserves the feet and mirroring anchors.
fn pack(raw: &blip::macroquad::texture::Image) -> (u32, u32, u32, Vec<u8>) {
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
    (w as u32, h as u32, FEET as u32 - top as u32, packed.encode_png())
}

/// A pose sample shared by the tape exporter and screenshot comparisons.
pub(crate) fn sample(who: usize, row: usize, frame: usize) -> Fighter {
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
