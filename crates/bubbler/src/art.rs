use blip::image::mix_rgb as mix;
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
    tiles: [[Texture2D; 5]; 8],
}

impl Art {
    pub(super) fn load() -> Self {
        let load = blip::load_png;
        // the toys are drawn at twice their size and shown smoothed, soft like felt
        let soft = |texture: Texture2D| {
            texture.set_filter(blip::macroquad::texture::FilterMode::Linear);
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
            players: [load(blip::png!("PLAYER_GREEN")), load(blip::png!("PLAYER_BLUE"))].map(soft),
            enemies: [
                [load(blip::png!("ROBOT")), load(blip::png!("ROBOT_ANGRY"))].map(soft),
                [load(blip::png!("BUNNY")), load(blip::png!("BUNNY_ANGRY"))].map(soft),
                [load(blip::png!("GHOST")), load(blip::png!("GHOST_ANGRY"))].map(soft),
            ],
            // per round: a lone cushion, a run's left end, middle and right end, and the wall
            tiles: [
                [blip::png!("TILE_1"), blip::png!("TILE_1_L"), blip::png!("TILE_1_M"), blip::png!("TILE_1_R"), blip::png!("BORDER_1")],
                [blip::png!("TILE_2"), blip::png!("TILE_2_L"), blip::png!("TILE_2_M"), blip::png!("TILE_2_R"), blip::png!("BORDER_2")],
                [blip::png!("TILE_3"), blip::png!("TILE_3_L"), blip::png!("TILE_3_M"), blip::png!("TILE_3_R"), blip::png!("BORDER_3")],
                [blip::png!("TILE_4"), blip::png!("TILE_4_L"), blip::png!("TILE_4_M"), blip::png!("TILE_4_R"), blip::png!("BORDER_4")],
                [blip::png!("TILE_5"), blip::png!("TILE_5_L"), blip::png!("TILE_5_M"), blip::png!("TILE_5_R"), blip::png!("BORDER_5")],
                [blip::png!("TILE_6"), blip::png!("TILE_6_L"), blip::png!("TILE_6_M"), blip::png!("TILE_6_R"), blip::png!("BORDER_6")],
                [blip::png!("TILE_7"), blip::png!("TILE_7_L"), blip::png!("TILE_7_M"), blip::png!("TILE_7_R"), blip::png!("BORDER_7")],
                [blip::png!("TILE_8"), blip::png!("TILE_8_L"), blip::png!("TILE_8_M"), blip::png!("TILE_8_R"), blip::png!("BORDER_8")],
            ]
            .map(|set| set.map(|bytes| soft(load(bytes)))),
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

    /// `piece`: 0 a lone cushion, 1-3 a run's left end, middle, right end, 4 the wall.
    pub(super) fn tile(&self, blip: &Blip, round: usize, piece: usize, x: f32, y: f32, size: f32) {
        blip.draw_texture(&self.tiles[round][piece], x, y, size, size);
    }

    /// A dragon, its feet at `foot`, turned the way `face` points. `squash` above 1 stretches it tall
    /// (a jump), below 1 flattens it (a landing), keeping its volume; `blink` shuts its eyes.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn player(
        &self, blip: &Blip, x: f32, foot: f32, player: usize, scale: f32, face: f32, squash: f32, blink: bool,
    ) {
        let sy = squash.powf(0.7);
        let (w, h) = (32.0 * scale / sy, 36.0 * scale * sy);
        let (left, top) = (x - w / 2.0, foot - h);
        blip.draw_texture_facing(&self.players[player], left, top, w, h, BlipColor::new(1.0, 1.0, 1.0, 1.0), face);
        if blink {
            // lids in the body colour over both eyes (sprite grid 13.5,15.5 and 20.5,15.5), a lash line across
            let body = [(120, 220, 150), (130, 190, 255)][player];
            let (kx, ky) = (w / 32.0, h / 36.0);
            for ex in [13.5, 20.5] {
                let u = if face < 0.0 { 32.0 - ex } else { ex };
                let (cx, cy) = (left + u * kx, top + 15.5 * ky);
                blip::macroquad::shapes::draw_ellipse(
                    cx, cy, 2.8 * kx, 3.6 * ky, 0.0,
                    BlipColor::from_rgba(body.0, body.1, body.2, 255),
                );
                blip::macroquad::shapes::draw_line(
                    cx - 2.2 * kx, cy + 0.8 * ky, cx + 2.2 * kx, cy + 0.8 * ky, 1.4 * ky,
                    BlipColor::from_rgba(40, 25, 45, 255),
                );
            }
        }
    }

    /// A trapped toy, its body's middle at `cx`,`cy`, stretched `sx` wide and `sy` tall (squashed into its
    /// bubble), tipped by `tilt` radians as it wriggles.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn toy_in_bubble(
        &self, kind: Kind, angry: bool, cx: f32, cy: f32, sx: f32, sy: f32, tilt: f32, live: Live,
    ) {
        let (w, h) = (32.0 * sx, 36.0 * sy);
        let (left, top) = (cx - 16.0 * sx, cy - 18.0 * sy);
        self.toy(kind, angry, (left, top, w, h), false, tilt, (cx, cy), 1.0, live);
    }

    /// A free toy standing at (`x`,`y`) in its hit box, drawn `scale` big and squashed `sx` x `sy`, rocked `tilt`
    /// radians about its feet (so it waddles), turned the way `face` points.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn enemy(
        &self,
        kind: Kind,
        x: f32,
        y: f32,
        scale: f32,
        alpha: f32,
        angry: bool,
        face: f32,
        (sx, sy, tilt): (f32, f32, f32),
        live: Live,
    ) {
        let (w, h) = (32.0 * scale * sx, 36.0 * scale * sy);
        let (left, top) = (x + (E_W - w) * 0.5, y + E_H - h);
        self.toy(kind, angry, (left, top, w, h), face < 0.0, tilt, (left + w * 0.5, top + h * 0.92), alpha, live);
    }

    /// A toy's sprite, then its live arms and, mid-blink, its eyelids, all turned `tilt` about `pivot`.
    #[allow(clippy::too_many_arguments)]
    fn toy(
        &self, kind: Kind, angry: bool, (left, top, w, h): (f32, f32, f32, f32), flip: bool, tilt: f32,
        pivot: (f32, f32), alpha: f32, live: Live,
    ) {
        use blip::macroquad::shapes::{draw_circle, draw_ellipse, draw_line};
        blip::macroquad::texture::draw_texture_ex(
            &self.enemies[kind as usize][usize::from(angry)],
            left,
            top,
            BlipColor::new(1.0, 1.0, 1.0, alpha),
            blip::macroquad::texture::DrawTextureParams {
                dest_size: Some(blip::macroquad::math::vec2(w, h)),
                flip_x: flip,
                rotation: tilt,
                pivot: Some(blip::macroquad::math::vec2(pivot.0, pivot.1)),
                ..Default::default()
            },
        );
        // sprite pixels (64 x 72) to the screen, mirrored and turned like the sprite
        let (c, s) = (tilt.cos(), tilt.sin());
        let at = |u: f32, v: f32| {
            let u = if flip { 64.0 - u } else { u };
            let (dx, dy) = (left + u * w / 64.0 - pivot.0, top + v * h / 72.0 - pivot.1);
            (pivot.0 + dx * c - dy * s, pivot.1 + dx * s + dy * c)
        };
        let k = (w / 64.0 + h / 72.0) / 2.0;
        let rgb = |c: (u8, u8, u8), m: f32| {
            BlipColor::new(c.0 as f32 / 255.0 * m, c.1 as f32 / 255.0 * m, c.2 as f32 / 255.0 * m, alpha)
        };
        let (body, limb) = TOY_COLOURS[kind as usize][usize::from(angry)];
        // arms: a little capsule from each shoulder; 0 hangs down, 1.6 points out, 3 straight up
        for (side, angle) in [(-1.0f32, live.arms[0]), (1.0, live.arms[1])] {
            let shoulder = (32.0 + side * 21.5, 40.0);
            let dir = (side * angle.sin(), angle.cos());
            let pts = [2.0, 7.0].map(|d| at(shoulder.0 + dir.0 * d, shoulder.1 + dir.1 * d));
            for p in pts {
                draw_circle(p.0, p.1, 5.8 * k, rgb(limb, 0.62));
            }
            for p in pts {
                draw_circle(p.0, p.1, 4.6 * k, rgb(limb, 1.0));
            }
            let hand = pts[1];
            draw_circle(hand.0 - 1.2 * k, hand.1 - 1.4 * k, 1.5 * k, BlipColor::new(1.0, 1.0, 1.0, 0.3 * alpha));
        }
        if live.blink {
            // closed, smiling eyes: lids in the face's colour and a soft curved lash
            let (lid, lash, eyes) = match kind {
                Kind::Walker => (mix(body, (255, 255, 255), 0.55), (60, 40, 70), [(25.0, 32.0), (43.0, 32.0)]),
                Kind::Hopper => (body, (60, 40, 70), [(26.0, 32.0), (44.0, 32.0)]),
                Kind::Ghost => ((70, 62, 80), (235, 230, 245), [(25.0, 33.0), (43.0, 33.0)]),
            };
            for (u, v) in eyes {
                let p = at(u, v);
                draw_ellipse(p.0, p.1, 4.8 * k, 5.8 * k, tilt.to_degrees(), rgb(lid, 1.0));
                let arc = [at(u - 3.4, v - 0.5), at(u, v + 1.8), at(u + 3.4, v - 0.5)];
                draw_line(arc[0].0, arc[0].1, arc[1].0, arc[1].1, 1.3 * k, rgb(lash, 1.0));
                draw_line(arc[1].0, arc[1].1, arc[2].0, arc[2].1, 1.3 * k, rgb(lash, 1.0));
            }
        }
    }
}

/// What makes a toy look alive this frame: its arms' angles (left, right; 0 hanging, 3 up) and a blink.
#[derive(Clone, Copy)]
pub(super) struct Live {
    pub arms: [f32; 2],
    pub blink: bool,
}

/// Each toy's (body, limb) colours, calm and cross: the same as gfx.rs paints them.
const TOY_COLOURS: [[((u8, u8, u8), (u8, u8, u8)); 2]; 3] = [
    [((150, 190, 215), (138, 175, 198)), ((245, 110, 175), (225, 101, 161))],
    [((245, 165, 85), (225, 152, 78)), ((240, 105, 85), (221, 97, 78))],
    [((246, 244, 250), (70, 62, 80)), ((255, 200, 222), (70, 62, 80))],
];

