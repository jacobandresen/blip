use blip::macroquad::texture::Texture2D;
use blip::{Blip, BlipColor};

use super::{Kind, E_H, E_W};

pub(super) struct Art {
    title_letters: [Texture2D; 7],
    title_info: [Texture2D; 3],
    title_start: [Texture2D; 3],
    title_sky: Texture2D,
    players: [Texture2D; 2],
    enemies: [[Texture2D; 2]; 3],
    tiles: [[Texture2D; 2]; 8],
}

impl Art {
    pub(super) fn load() -> Self {
        let load = |bytes: &[u8]| {
            let texture = Texture2D::from_file_with_format(
                bytes,
                Some(blip::macroquad::prelude::ImageFormat::Png),
            );
            texture.set_filter(blip::macroquad::texture::FilterMode::Nearest);
            texture
        };
        Self {
            title_sky: load(blip::png!("TITLE_SKY")),
            title_info: [blip::png!("TITLE_INFO_KEYS"), blip::png!("TITLE_INFO_PAD"), blip::png!("TITLE_INFO_TOUCH")].map(load),
            title_start: [blip::png!("TITLE_START_KEYS"), blip::png!("TITLE_START_PAD"), blip::png!("TITLE_START_TOUCH")].map(load),
            title_letters: [
                blip::png!("TITLE_LETTER_1"), blip::png!("TITLE_LETTER_2"), blip::png!("TITLE_LETTER_3"),
                blip::png!("TITLE_LETTER_4"), blip::png!("TITLE_LETTER_5"), blip::png!("TITLE_LETTER_6"),
                blip::png!("TITLE_LETTER_7"),
            ].map(load),
            players: [load(blip::png!("PLAYER_GREEN")), load(blip::png!("PLAYER_BLUE"))],
            enemies: [
                [load(blip::png!("ROBOT")), load(blip::png!("ROBOT_ANGRY"))],
                [load(blip::png!("BUNNY")), load(blip::png!("BUNNY_ANGRY"))],
                [load(blip::png!("GHOST")), load(blip::png!("GHOST_ANGRY"))],
            ],
            tiles: std::array::from_fn(|i| {
                let normal = [
                    blip::png!("TILE_1"),
                    blip::png!("TILE_2"),
                    blip::png!("TILE_3"),
                    blip::png!("TILE_4"),
                    blip::png!("TILE_5"),
                    blip::png!("TILE_6"),
                    blip::png!("TILE_7"),
                    blip::png!("TILE_8"),
                ][i];
                let border = [
                    blip::png!("BORDER_1"),
                    blip::png!("BORDER_2"),
                    blip::png!("BORDER_3"),
                    blip::png!("BORDER_4"),
                    blip::png!("BORDER_5"),
                    blip::png!("BORDER_6"),
                    blip::png!("BORDER_7"),
                    blip::png!("BORDER_8"),
                ][i];
                [load(normal), load(border)]
            }),
        }
    }

    pub(super) fn title_letter(&self, blip: &Blip, letter: usize, x: f32, y: f32) {
        blip.draw_texture(&self.title_letters[letter], x - 16.0, y - 16.0, 68.0, 74.0);
    }

    pub(super) fn title_background(&self, blip: &Blip) {
        blip.draw_texture(&self.title_sky, 0.0, 0.0, 624.0, 600.0);
    }

    pub(super) fn title_panels(&self, blip: &Blip, controls: blip::web::Controls, glow: f32) {
        let mode = match controls {
            blip::web::Controls::Keys => 0,
            blip::web::Controls::Pad => 1,
            blip::web::Controls::Touch => 2,
        };
        blip.draw_texture(&self.title_info[mode], 0.0, 400.0, 624.0, 92.0);
        blip.draw_texture_tinted(&self.title_start[mode], 0.0, 359.0, 624.0, 40.0, BlipColor::new(1.0, 1.0, 1.0, glow));
    }

    pub(super) fn tile(&self, blip: &Blip, round: usize, border: bool, x: f32, y: f32, size: f32) {
        blip.draw_texture(&self.tiles[round][usize::from(border)], x, y, size, size);
    }

    pub(super) fn player(&self, blip: &Blip, x: f32, foot: f32, player: usize, scale: f32) {
        blip.draw_texture(
            &self.players[player],
            x - 16.0 * scale,
            foot - 36.0 * scale,
            32.0 * scale,
            36.0 * scale,
        );
    }

    pub(super) fn enemy(
        &self,
        blip: &Blip,
        kind: Kind,
        x: f32,
        y: f32,
        scale: f32,
        alpha: f32,
        angry: bool,
    ) {
        let index = match kind {
            Kind::Walker => 0,
            Kind::Hopper => 1,
            Kind::Ghost => 2,
        };
        let (w, h) = (32.0 * scale, 36.0 * scale);
        blip.draw_texture_tinted(
            &self.enemies[index][usize::from(angry)],
            x + (E_W - w) * 0.5,
            y + E_H - h,
            w,
            h,
            BlipColor {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: alpha,
            },
        );
    }
}
