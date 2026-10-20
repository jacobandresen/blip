use blip::macroquad::texture::Texture2D;
use blip::{Blip, BlipColor};

use super::{Kind, E_H, E_W};

mod assets {
    include!(concat!(env!("OUT_DIR"), "/gfx.rs"));
}

pub(super) struct Art {
    title_letters: [Texture2D; 7],
    title_info: [Texture2D; 3],
    title_start: [Texture2D; 3],
    title_bubbles: Texture2D,
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
            title_sky: load(assets::TITLE_SKY),
            title_bubbles: load(assets::TITLE_BUBBLES),
            title_info: [assets::TITLE_INFO_KEYS, assets::TITLE_INFO_PAD, assets::TITLE_INFO_TOUCH].map(load),
            title_start: [assets::TITLE_START_KEYS, assets::TITLE_START_PAD, assets::TITLE_START_TOUCH].map(load),
            title_letters: [
                assets::TITLE_LETTER_1, assets::TITLE_LETTER_2, assets::TITLE_LETTER_3,
                assets::TITLE_LETTER_4, assets::TITLE_LETTER_5, assets::TITLE_LETTER_6,
                assets::TITLE_LETTER_7,
            ].map(load),
            players: [load(assets::PLAYER_GREEN), load(assets::PLAYER_BLUE)],
            enemies: [
                [load(assets::ROBOT), load(assets::ROBOT_ANGRY)],
                [load(assets::BUNNY), load(assets::BUNNY_ANGRY)],
                [load(assets::GHOST), load(assets::GHOST_ANGRY)],
            ],
            tiles: std::array::from_fn(|i| {
                let normal = [
                    assets::TILE_1,
                    assets::TILE_2,
                    assets::TILE_3,
                    assets::TILE_4,
                    assets::TILE_5,
                    assets::TILE_6,
                    assets::TILE_7,
                    assets::TILE_8,
                ][i];
                let border = [
                    assets::BORDER_1,
                    assets::BORDER_2,
                    assets::BORDER_3,
                    assets::BORDER_4,
                    assets::BORDER_5,
                    assets::BORDER_6,
                    assets::BORDER_7,
                    assets::BORDER_8,
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

    pub(super) fn title_bubble(&self, blip: &Blip, index: usize, x: f32, y: f32, wobble: f32) {
        let frame = ((wobble * 0.15 + x * 0.002).rem_euclid(1.0) * 24.0) as u32;
        let size = (12.0 + (wobble * 3.0).sin() * 0.6) * (32.0 / 12.0);
        blip.draw_texture_cell(&self.title_bubbles, frame, 24, (index % 2) as u32, 2,
            x - size / 2.0, y - size / 2.0, size, size, 0.0);
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
