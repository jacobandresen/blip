// Compiled by build.rs, and by the game only for its tests: much goes unused in each.
#![allow(dead_code)]

use blip::{image::Image, Asset};

// One texture per letter preserves the title's individual bobbing motion.
fn title_letter(ch: char, index: usize) -> Vec<u8> {
    let mut image = Image::new(68, 74);
    let hue = ((0.9 + index as f32 * 0.06) % 1.0) * 6.0;
    let x = 0.45 * (1.0 - ((hue % 2.0) - 1.0).abs());
    let (r, g, b) = match hue as i32 {
        0 => (0.45, x, 0.0), 1 => (x, 0.45, 0.0), 2 => (0.0, 0.45, x),
        3 => (0.0, x, 0.45), 4 => (x, 0.0, 0.45), _ => (0.45, 0.0, x),
    };
    let color = [((r + 0.55) * 255.0) as u8, ((g + 0.55) * 255.0) as u8, ((b + 0.55) * 255.0) as u8];
    let mut shape = |rgb: [u8; 3], alpha: f32, inside: &dyn Fn(f32, f32) -> bool| {
        for y in 0..74 {
            for x in 0..68 {
                let mut covered = 0;
                for sy in 0..3 { for sx in 0..3 {
                    if inside(x as f32 + (sx as f32 + 0.5) / 3.0, y as f32 + (sy as f32 + 0.5) / 3.0) { covered += 1; }
                } }
                if covered > 0 { image.blend(x, y, [rgb[0], rgb[1], rgb[2], (alpha * covered as f32 / 9.0 * 255.0).round() as u8]); }
            }
        }
    };
    shape(color, 0.16, &|x, y| (x - 31.0).hypot(y - 37.0) <= 27.6);
    shape(color, 0.7, &|x, y| ((x - 31.0).hypot(y - 37.0) - 27.6).abs() <= 1.0);
    shape([255, 255, 255], 0.7, &|x, y| {
        let (dx, dy) = (x - 20.2, y - 23.8);
        let angle = -35.0_f32.to_radians();
        let ex = (dx * angle.cos() + dy * angle.sin()) / 7.8;
        let ey = (-dx * angle.sin() + dy * angle.cos()) / 4.2;
        ex * ex + ey * ey <= 1.0
    });
    let text = ch.to_string();
    for (dx, dy) in [(-2, 0), (2, 0), (0, -2), (0, 2)] {
        image.text(&text, 16 + dx, 16 + dy, 6, [38, 5, 63, 230]);
    }
    image.text(&text, 16, 16, 6, [color[0], color[1], color[2], 255]);
    image.encode_png()
}

fn title_panel(image: &mut Image, y: f32, width: f32, height: f32, edge: [u8; 4]) {
    let radius = (height / 2.0).min(14.0);
    for (grow, color) in [(1.6, edge), (0.0, [34, 14, 44, 188])] {
        let (half_w, half_h, r) = (width / 2.0 + grow, height / 2.0 + grow, radius + grow);
        for py in (y as i32 - 2)..(y as i32 + height as i32 + 2) {
            for px in (312.0 - half_w).floor() as i32..(312.0 + half_w).ceil() as i32 {
                let dx = (px as f32 + 0.5 - 312.0).abs() - half_w + r;
                let dy = (py as f32 + 0.5 - y - height / 2.0).abs() - half_h + r;
                let distance = dx.max(0.0).hypot(dy.max(0.0)) + dx.max(dy).min(0.0) - r;
                let coverage = (0.5 - distance).clamp(0.0, 1.0);
                image.blend(px, py, [color[0], color[1], color[2], (color[3] as f32 * coverage) as u8]);
            }
        }
    }
}

fn title_text(image: &mut Image, text: &str, y: i32, color: [u8; 4]) {
    let x = (624 - (text.len() as i32 * 12 - 2)) / 2;
    image.text(text, x + 1, y + 1, 2, [60, 20, 70, 230]);
    image.text(text, x, y, 2, color);
}

fn title_info(mode: usize) -> Vec<u8> {
    let mut image = Image::new(624, 92);
    let join = if mode == 0 { "P2 PRESS J TO JOIN" } else { "P2 TOUCH TO JOIN" };
    title_text(&mut image, join, 6, [165, 215, 255, 255]);
    let hints = [
        ["MOVE A D   JUMP W   BUBBLE F", "HOLD JUMP TO RIDE BUBBLES"],
        ["PAD MOVES   BUTTONS BUBBLE AND JUMP", "HOLD JUMP TO RIDE BUBBLES"],
        ["TOUCH TO BUBBLE   SLIDE TO RUN", "SWIPE UP TO JUMP"],
    ][mode];
    title_panel(&mut image, 30.0, hints[0].len() as f32 * 12.0 + 34.0, 54.0, [255, 196, 150, 165]);
    title_text(&mut image, hints[0], 40, [255, 244, 222, 242]);
    title_text(&mut image, hints[1], 60, [255, 244, 222, 242]);
    image.encode_png()
}

fn title_start(mode: usize) -> Vec<u8> {
    let mut image = Image::new(624, 40);
    let text = if mode == 2 { "P1 TOUCH TO START" } else { "P1 PRESS BUBBLE" };
    title_panel(&mut image, 4.0, text.len() as f32 * 12.0 + 24.0, 32.0, [170, 240, 190, 230]);
    title_text(&mut image, text, 13, [170, 240, 190, 255]);
    image.encode_png()
}

fn title_sky() -> Vec<u8> {
    let mut image = Image::new(624, 600);
    for y in 0..600 {
        let t = (((y as f32 + 0.5) * 18.0 / 600.0).floor() / 17.0).min(1.0);
        let color = ((18.0 + 30.0 * t) as u8, (12.0 + 8.0 * t) as u8, (52.0 + 32.0 * t) as u8);
        for x in 0..624 { image.set(x, y, color.0, color.1, color.2); }
    }
    image.encode_png()
}

fn disc(img: &mut Image, cx: i32, cy: i32, rx: i32, ry: i32, color: (u8, u8, u8)) {
    for y in -ry..=ry {
        for x in -rx..=rx {
            if x * x * ry * ry + y * y * rx * rx <= rx * rx * ry * ry {
                img.set(cx + x, cy + y, color.0, color.1, color.2);
            }
        }
    }
}

fn line(img: &mut Image, x0: i32, y0: i32, x1: i32, y1: i32, color: (u8, u8, u8)) {
    let steps = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
    for i in 0..=steps {
        let x = x0 + (x1 - x0) * i / steps;
        let y = y0 + (y1 - y0) * i / steps;
        img.set(x, y, color.0, color.1, color.2);
    }
}

/// A plush toy turned a little to the right, its face off-centre that way, so
/// mirrored it plainly faces left.
fn plush(body: (u8, u8, u8), angry_body: (u8, u8, u8), kind: usize, angry: bool) -> Vec<u8> {
    let body = if angry { angry_body } else { body };
    let mut img = Image::new(32, 36);
    let dark = (
        (body.0 as f32 * 0.68) as u8,
        (body.1 as f32 * 0.68) as u8,
        (body.2 as f32 * 0.68) as u8,
    );
    if kind == 1 {
        disc(&mut img, 11, 7, 3, 8, dark);
        disc(&mut img, 21, 7, 3, 8, dark);
        disc(&mut img, 11, 7, 1, 6, (255, 160, 190));
        disc(&mut img, 21, 7, 1, 6, (255, 160, 190));
    }
    if kind == 2 {
        for x in [7, 13, 19, 25] {
            disc(&mut img, x, 24, 4, 4, dark);
        }
    }
    disc(&mut img, 16, 20, 12, 11, dark);
    disc(&mut img, 16, 18, 11, 10, body);
    disc(&mut img, 19, 22, 7, 6, (255, 240, 205));
    if kind == 0 {
        disc(&mut img, 16, 5, 3, 3, (230, 180, 70));
        line(&mut img, 16, 8, 16, 12, (230, 180, 70));
        disc(&mut img, 7, 28, 5, 3, dark);
        disc(&mut img, 25, 28, 5, 3, dark);
        disc(&mut img, 18, 18, 8, 6, (250, 245, 255));
    } else if kind == 1 {
        // the tail, behind it
        disc(&mut img, 5, 24, 3, 3, (255, 250, 250));
    }
    for x in [15, 22] {
        disc(&mut img, x, 16, 2, 3, (40, 25, 45));
        disc(&mut img, x, 15, 1, 1, (255, 255, 255));
    }
    if angry {
        line(&mut img, 11, 13, 16, 15, (90, 40, 60));
        line(&mut img, 24, 13, 19, 15, (90, 40, 60));
    }
    disc(&mut img, 12, 20, 2, 1, (255, 150, 165));
    disc(&mut img, 19, 22, 1, 1, (120, 40, 60));
    img.encode_png()
}

fn dragon(body: (u8, u8, u8)) -> Vec<u8> {
    let mut img = Image::new(32, 36);
    let dark = (
        (body.0 as f32 * 0.68) as u8,
        (body.1 as f32 * 0.68) as u8,
        (body.2 as f32 * 0.68) as u8,
    );
    disc(&mut img, 9, 30, 5, 3, dark);
    disc(&mut img, 23, 30, 5, 3, dark);
    for (x, y) in [(7, 13), (10, 9), (14, 7)] {
        disc(&mut img, x, y, 3, 3, (255, 240, 205));
    }
    disc(&mut img, 16, 20, 12, 11, dark);
    disc(&mut img, 16, 18, 11, 10, body);
    disc(&mut img, 20, 22, 7, 6, (255, 240, 205));
    disc(&mut img, 13, 15, 2, 3, (40, 25, 45));
    disc(&mut img, 20, 15, 2, 3, (40, 25, 45));
    disc(&mut img, 13, 14, 1, 1, (255, 255, 255));
    disc(&mut img, 20, 14, 1, 1, (255, 255, 255));
    disc(&mut img, 24, 20, 2, 2, (255, 150, 165));
    line(&mut img, 25, 21, 29, 21, (120, 40, 60));
    disc(&mut img, 16, 7, 3, 3, body);
    img.encode_png()
}

const TILES: [((u8, u8, u8), (u8, u8, u8), (u8, u8, u8)); 8] = [
    ((240, 120, 180), (255, 190, 225), (160, 60, 120)),
    ((80, 200, 230), (170, 240, 255), (30, 110, 150)),
    ((250, 180, 60), (255, 230, 150), (170, 100, 20)),
    ((110, 220, 110), (190, 255, 180), (40, 130, 60)),
    ((190, 110, 250), (230, 190, 255), (110, 50, 170)),
    ((255, 120, 120), (255, 196, 186), (170, 56, 70)),
    ((226, 222, 84), (252, 250, 176), (136, 130, 34)),
    ((150, 240, 210), (222, 255, 240), (60, 150, 130)),
];

fn tile(round: usize, border: bool) -> Vec<u8> {
    let (block, light, dark) = TILES[round];
    let shade = |c: (u8, u8, u8), k: f32| {
        (
            (c.0 as f32 * k) as u8,
            (c.1 as f32 * k) as u8,
            (c.2 as f32 * k) as u8,
        )
    };
    let (block, light, dark) = if border {
        (shade(block, 0.62), shade(light, 0.7), dark)
    } else {
        (block, light, dark)
    };
    let mut img = Image::new(32, 32);
    for y in 1..31 {
        for x in 1..31 {
            let c = if y < 5 {
                light
            } else if y >= 28 || x >= 28 {
                dark
            } else {
                block
            };
            img.set(x, y, c.0, c.1, c.2);
        }
    }
    let dot = shade(light, if border { 0.25 } else { 0.45 });
    for y in 14..18 {
        for x in 14..18 {
            img.set(x, y, dot.0, dot.1, dot.2);
        }
    }
    img.set(13, 14, 255, 255, 255);
    img.encode_png()
}

/// Sprites used by the Rust game and BASIC tape.
pub fn generate() -> Vec<Asset> {
    let mut assets = vec![
        ("images/TITLE_SKY.png", title_sky()),
        ("images/TITLE_INFO_KEYS.png", title_info(0)),
        ("images/TITLE_INFO_PAD.png", title_info(1)),
        ("images/TITLE_INFO_TOUCH.png", title_info(2)),
        ("images/TITLE_START_KEYS.png", title_start(0)),
        ("images/TITLE_START_PAD.png", title_start(1)),
        ("images/TITLE_START_TOUCH.png", title_start(2)),
        ("images/TITLE_LETTER_1.png", title_letter('B', 0)),
        ("images/TITLE_LETTER_2.png", title_letter('U', 1)),
        ("images/TITLE_LETTER_3.png", title_letter('B', 2)),
        ("images/TITLE_LETTER_4.png", title_letter('B', 3)),
        ("images/TITLE_LETTER_5.png", title_letter('L', 4)),
        ("images/TITLE_LETTER_6.png", title_letter('E', 5)),
        ("images/TITLE_LETTER_7.png", title_letter('R', 6)),
        ("images/PLAYER_GREEN.png", dragon((90, 210, 110))),
        ("images/PLAYER_BLUE.png", dragon((90, 170, 255))),
        (
            "images/ROBOT.png",
            plush((200, 190, 245), (255, 150, 140), 0, false),
        ),
        (
            "images/ROBOT_ANGRY.png",
            plush((200, 190, 245), (255, 150, 140), 0, true),
        ),
        (
            "images/BUNNY.png",
            plush((255, 195, 220), (255, 140, 160), 1, false),
        ),
        (
            "images/BUNNY_ANGRY.png",
            plush((255, 195, 220), (255, 140, 160), 1, true),
        ),
        (
            "images/GHOST.png",
            plush((215, 245, 255), (255, 190, 235), 2, false),
        ),
        (
            "images/GHOST_ANGRY.png",
            plush((215, 245, 255), (255, 190, 235), 2, true),
        ),
    ];
    for round in 0..TILES.len() {
        let tile_name = [
            "images/TILE_1.png",
            "images/TILE_2.png",
            "images/TILE_3.png",
            "images/TILE_4.png",
            "images/TILE_5.png",
            "images/TILE_6.png",
            "images/TILE_7.png",
            "images/TILE_8.png",
        ][round];
        let border_name = [
            "images/BORDER_1.png",
            "images/BORDER_2.png",
            "images/BORDER_3.png",
            "images/BORDER_4.png",
            "images/BORDER_5.png",
            "images/BORDER_6.png",
            "images/BORDER_7.png",
            "images/BORDER_8.png",
        ][round];
        assets.push((tile_name, tile(round, false)));
        assets.push((border_name, tile(round, true)));
    }
    assets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprites_and_round_tiles_have_their_intended_png_sizes() {
        let assets = generate();
        assert_eq!(assets.len(), 38);
        for (path, bytes) in assets {
            assert!(path.starts_with("images/"));
            let mut reader = png::Decoder::new(bytes.as_slice()).read_info().unwrap();
            let mut pixels = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut pixels).unwrap();
            let is_tile = path.contains("TILE_") || path.contains("BORDER_");
            let expected_size = if path.contains("TITLE_LETTER_") {
                (68, 74)
            } else if path.contains("TITLE_INFO_") {
                (624, 92)
            } else if path.contains("TITLE_START_") {
                (624, 40)
            } else if path.contains("TITLE_SKY") {
                (624, 600)
            } else if is_tile {
                (32, 32)
            } else {
                (32, 36)
            };
            assert_eq!((info.width, info.height), expected_size, "{path}");
            let mut alpha = pixels[..info.buffer_size()].chunks_exact(4).map(|px| px[3]);
            assert!(alpha.clone().any(|a| a > 0), "{path}");
            if path.contains("TITLE_SKY") {
                assert!(alpha.all(|a| a == 255), "{path}");
            } else {
                assert!(alpha.any(|a| a == 0), "{path}");
            }
        }
    }
}
