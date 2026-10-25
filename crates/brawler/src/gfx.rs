use blip::image::Image;
use blip::Asset;

fn pose_cell(name: &str, row: usize, frame: usize, flip: bool) -> Image {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("art").join(format!("{name}.png"));
    let sheet = Image::from_png(&std::fs::read(path).expect("poster fighter art"));
    let (w, h) = (sheet.w / 6, sheet.h / 19);
    let mut cell = Image::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let sx = frame as u32 * w + if flip { w - 1 - x } else { x };
            let sy = row as u32 * h + y;
            let at = ((sy * sheet.w + sx) * 4) as usize;
            cell.set_rgba(x as i32, y as i32, sheet.px[at], sheet.px[at + 1],
                sheet.px[at + 2], sheet.px[at + 3]);
        }
    }
    cell
}

fn film_poster() -> Vec<u8> {
    let (w, h) = (424, 238);
    let mut image = Image::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let dx = (x as f32 - 214.0) / 245.0;
            let dy = (y as f32 - 119.0) / 170.0;
            let glow = (1.0 - dx * dx - dy * dy).clamp(0.0, 1.0);
            image.set(x as i32, y as i32,
                (20.0 + glow * 76.0) as u8,
                (8.0 + glow * 16.0) as u8,
                (21.0 + glow * 10.0) as u8);
        }
    }
    for i in 0..13 {
        let x = i as f32 * 39.0 - 22.0;
        image.line((x, 0.0), (214.0, 64.0), 1.4, [255, 174, 63, 60]);
        image.line((x, h as f32), (214.0, 119.0), 1.0, [255, 75, 51, 55]);
    }
    for y in (36..174).step_by(8) {
        for x in (4..420).step_by(8) {
            if (x / 8 + y / 8) % 2 == 0 {
                image.circle(x as f32, y as f32, 1.0, [255, 205, 118, 42]);
            }
        }
    }
    let lineup = [
        ("aurelia", 10, 3, false, -10.0, 66.0, 108.0, 155.0),
        ("jade", 10, 3, false, 52.0, 45.0, 135.0, 180.0),
        ("regent", 8, 3, true, 328.0, 64.0, 104.0, 155.0),
        ("mack", 11, 3, true, 258.0, 42.0, 139.0, 183.0),
        ("ryuka", 15, 3, false, 116.0, 18.0, 195.0, 198.0),
    ];
    for (name, row, frame, flip, x, y, width, height) in lineup {
        let pose = pose_cell(name, row, frame, flip);
        image.blit(&pose, x, y, width, height);
    }
    for y in 176..h {
        for x in 0..w {
            image.blend(x as i32, y as i32, [12, 7, 18, 235]);
        }
    }
    image.text("BRAWLER", 107, 178, 5, [255, 199, 69, 255]);
    image.text("BLIP, 2026", 152, 218, 2, [255, 205, 118, 255]);
    for x in 0..w as i32 {
        image.blend(x, 0, [255, 193, 80, 230]);
        image.blend(x, h as i32 - 1, [255, 193, 80, 230]);
    }
    for y in 0..h as i32 {
        image.blend(0, y, [255, 193, 80, 180]);
        image.blend(w as i32 - 1, y, [255, 193, 80, 180]);
    }
    image.encode_png()
}

fn impact_glint() -> Vec<u8> {
    let mut image = Image::new(32, 32);
    for y in 0..32i32 {
        for x in 0..32i32 {
            let dx = (x - 16) as f32;
            let dy = (y - 16) as f32;
            let d = (dx * dx + dy * dy).sqrt();
            if d < 13.0 && ((x + y) % 5 != 0 || d < 8.0) {
                let edge = (1.0 - d / 13.0).clamp(0.0, 1.0);
                image.set_rgba(
                    x,
                    y,
                    255,
                    (210.0 + 45.0 * edge) as u8,
                    90,
                    (edge * 210.0) as u8,
                );
            }
        }
    }
    image.encode_png()
}

pub fn generate() -> Vec<Asset> {
    let mut assets = vec![
        ("images/impact_glint.png", impact_glint()),
        ("images/film_poster.png", film_poster()),
    ];
    // Captures are refreshed by tools/make_brawler_art.mjs, then packaged like other art.
    for path in [
        "images/ryuka.png", "images/brutus.png", "images/kestrel.png", "images/giotto.png",
        "images/titian.png", "images/vermeer.png", "images/bosch.png", "images/webber.png",
        "images/gamma.png", "images/zenith.png", "images/jade.png", "images/regent.png",
        "images/kite.png", "images/aurelia.png", "images/stage_1.png", "images/stage_2.png",
        "images/stage_3.png", "images/stage_4.png", "images/stage_5.png", "images/stage_6.png",
        "images/stage_7.png", "images/mack.png", "images/stage_8.png", "images/portraits.png",
    ] {
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("art").join(path.rsplit('/').next().unwrap());
        if let Ok(bytes) = std::fs::read(file) { assets.push((path, bytes)); }
    }
    assets
}
