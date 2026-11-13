//! WARLOCK's renderer. The cellar is drawn in four passes:
//! 1. stone: flagstones, walls, sconces and floor pieces from the tile sheet;
//! 2. light: a light map (torches, staffs, spells, each blocked by walls)
//!    multiplied over the stone, so the cellar is dark between the lights;
//! 3. figures: wizards and monsters, tinted by the light where they stand
//!    but never darker than readable;
//! 4. magic: glows, bolts, sparks, waves and pillars added on top, so
//!    overlapping spells burn brighter.

use super::*;
use crate::magic::{Kind, Magic};
use blip::macroquad::color::Color;
use blip::macroquad::material::{gl_use_default_material, gl_use_material, load_material, Material, MaterialParams};
use blip::macroquad::math::{vec2, Rect};
use blip::macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation, PipelineParams, ShaderSource};
use blip::macroquad::shapes::{draw_circle, draw_circle_lines, draw_line, draw_rectangle};
use blip::macroquad::texture::{draw_texture_ex, DrawTextureParams, FilterMode, Texture2D};

const SPR: f32 = 32.0;
/// Light-map samples per cell, each way.
const SUB: i32 = 4;
const LW: usize = COLS * SUB as usize;
const LH: usize = ROWS * SUB as usize;
const SUB_PX: f32 = CELL as f32 / SUB as f32;

/// The colour of the dark: a cold blue-grey, so shadow reads as stone, not void.
const AMBIENT: [f32; 3] = [0.30, 0.32, 0.46];
pub const TORCH_LIGHT: [f32; 3] = [1.25, 0.72, 0.34];

const VERT: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
}
"#;
const FRAG: &str = r#"#version 100
precision mediump float;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform sampler2D Texture;
void main() {
    gl_FragColor = color * texture2D(Texture, uv);
}
"#;

fn material(color: BlendState) -> Option<Material> {
    load_material(
        ShaderSource::Glsl { vertex: VERT, fragment: FRAG },
        MaterialParams {
            pipeline_params: PipelineParams {
                color_blend: Some(color),
                alpha_blend: Some(BlendState::new(Equation::Add, BlendFactor::Zero, BlendFactor::One)),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .ok()
}

/// A static light's reach: the light-map samples it touches and how strongly.
struct Baked {
    weights: Vec<(u32, f32)>,
}

pub struct Art {
    tiles: Texture2D,
    floor: Texture2D,
    sprites: Texture2D,
    title: Texture2D,
    glow: Texture2D,
    light_tex: Texture2D,
    light: Vec<[f32; 3]>,
    bytes: Vec<u8>,
    add: Option<Material>,
    mul: Option<Material>,
    baked_stamp: u32,
    torches: Vec<Baked>,
    candle_light: Vec<Baked>,
    banners: Vec<Cell>,
    candles: Vec<Cell>,
    /// A dragon skull's left cell, and whether it faces left.
    skull: Option<(Cell, bool)>,
    vignette: Vec<f32>,
}

fn c(r: f32, g: f32, b: f32, a: f32) -> Color {
    Color { r, g, b, a }
}
fn with_a(col: Color, a: f32) -> Color {
    Color { a, ..col }
}

/// A soft radial falloff, white, for every glow and streak.
fn glow_texture() -> Texture2D {
    let n = 64u16;
    let mut px = vec![0u8; n as usize * n as usize * 4];
    for y in 0..n as i32 {
        for x in 0..n as i32 {
            let dx = (x as f32 + 0.5) / n as f32 * 2.0 - 1.0;
            let dy = (y as f32 + 0.5) / n as f32 * 2.0 - 1.0;
            let d = (dx * dx + dy * dy).sqrt();
            let a = ((1.0 - d).max(0.0)).powf(2.2);
            let o = ((y * n as i32 + x) * 4) as usize;
            px[o] = 255;
            px[o + 1] = 255;
            px[o + 2] = 255;
            px[o + 3] = (a * 255.0) as u8;
        }
    }
    let t = Texture2D::from_rgba8(n, n, &px);
    t.set_filter(FilterMode::Linear);
    t
}

impl Art {
    pub fn new() -> Self {
        let tiles = blip::load_png(blip::png!("tiles"));
        let floor = blip::load_png(blip::png!("floor"));
        let sprites = blip::load_png(blip::png!("sprites"));
        let title = blip::load_png(blip::png!("title"));
        let bytes = vec![0u8; LW * LH * 4];
        let light_tex = Texture2D::from_rgba8(LW as u16, LH as u16, &bytes);
        light_tex.set_filter(FilterMode::Linear);
        // multiply by twice the light-map value: 0.5 leaves stone as painted,
        // 1.0 doubles it, so torchlight can lift stone above its base colour
        let mul = material(BlendState::new(
            Equation::Add,
            BlendFactor::Value(BlendValue::DestinationColor),
            BlendFactor::Value(BlendValue::SourceColor),
        ));
        let add = material(BlendState::new(Equation::Add, BlendFactor::Value(BlendValue::SourceAlpha), BlendFactor::One));
        let mut vignette = vec![1.0; LW * LH];
        for y in 0..LH {
            for x in 0..LW {
                let dx = (x as f32 + 0.5) / LW as f32 * 2.0 - 1.0;
                let dy = (y as f32 + 0.5) / LH as f32 * 2.0 - 1.0;
                vignette[y * LW + x] = 1.0 - 0.35 * (dx * dx + dy * dy).min(1.6) / 1.6;
            }
        }
        Art {
            tiles,
            floor,
            sprites,
            title,
            glow: glow_texture(),
            light_tex,
            light: vec![[0.0; 3]; LW * LH],
            bytes,
            add,
            mul,
            baked_stamp: u32::MAX,
            torches: Vec::new(),
            candle_light: Vec::new(),
            banners: Vec::new(),
            candles: Vec::new(),
            skull: None,
            vignette,
        }
    }

    fn additive(&self, on: bool) {
        match (&self.add, on) {
            (Some(m), true) => gl_use_material(m),
            _ => gl_use_default_material(),
        }
    }

    /// A soft glow of radius `r` (additive when the additive pass is on).
    fn glow(&self, x: f32, y: f32, r: f32, col: Color) {
        draw_texture_ex(&self.glow, x - r, y - r, col, DrawTextureParams { dest_size: Some(vec2(r * 2.0, r * 2.0)), ..Default::default() });
    }

    /// A glow stretched along an angle: streaks, trails, beams.
    fn streak(&self, x: f32, y: f32, len: f32, wid: f32, angle: f32, col: Color) {
        draw_texture_ex(
            &self.glow,
            x - len / 2.0,
            y - wid / 2.0,
            col,
            DrawTextureParams { dest_size: Some(vec2(len, wid)), rotation: angle, ..Default::default() },
        );
    }

    fn sprite(&self, col: u32, row: u32, x: f32, y: f32, flip: bool, tint: Color) {
        self.sprite_sized(col, row, x, y, SPR, flip, tint);
    }

    fn sprite_sized(&self, col: u32, row: u32, x: f32, y: f32, size: f32, flip: bool, tint: Color) {
        draw_texture_ex(
            &self.sprites,
            (x - size / 2.0).round(),
            (y - size / 2.0).round(),
            tint,
            DrawTextureParams {
                dest_size: Some(vec2(size, size)),
                source: Some(Rect::new(col as f32 * SPR, row as f32 * SPR, SPR, SPR)),
                flip_x: flip,
                ..Default::default()
            },
        );
    }

    fn sprite_rot(&self, col: u32, row: u32, x: f32, y: f32, size: f32, rot: f32, tint: Color) {
        draw_texture_ex(
            &self.sprites,
            x - size / 2.0,
            y - size / 2.0,
            tint,
            DrawTextureParams {
                dest_size: Some(vec2(size, size)),
                source: Some(Rect::new(col as f32 * SPR, row as f32 * SPR, SPR, SPR)),
                rotation: rot,
                ..Default::default()
            },
        );
    }

    fn tile(&self, col: u32, row: u32, x: f32, y: f32) {
        self.tile_part(col, row, 0.0, 32.0, x, y);
    }

    /// Rows `sy..sy+sh` of a tile, drawn at `x, y`.
    fn tile_part(&self, col: u32, row: u32, sy: f32, sh: f32, x: f32, y: f32) {
        draw_texture_ex(
            &self.tiles,
            x,
            y,
            blip::macroquad::color::WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(CELL as f32, sh)),
                source: Some(Rect::new(col as f32 * 32.0, row as f32 * 32.0 + sy, 32.0, sh)),
                ..Default::default()
            },
        );
    }

    // ---- light ------------------------------------------------------------------

    /// The light reaching a board pixel, from the last light map.
    pub fn light_at(&self, x: f32, y: f32) -> [f32; 3] {
        let lx = ((x / SUB_PX) as i32).clamp(0, LW as i32 - 1) as usize;
        let ly = (((y - HUD_H as f32) / SUB_PX) as i32).clamp(0, LH as i32 - 1) as usize;
        self.light[ly * LW + lx]
    }

    fn bake(&mut self, g: &Game) {
        if self.baked_stamp == g.room_stamp {
            return;
        }
        self.baked_stamp = g.room_stamp;
        self.torches = g.torches.iter().map(|&t| {
            let (x, y) = torch_flame(t);
            Baked { weights: reach(&g.walls, x, y + 6.0, 150.0) }
        }).collect();
        self.dress(g);
        self.candle_light = self.candles.iter().map(|&cl| {
            let (x, y) = center(cl);
            Baked { weights: reach(&g.walls, x, y - 4.0, 80.0) }
        }).collect();
    }

    /// Choose where this room's banners, candles and bones go: off the
    /// paths that matter (spawns, runes, stairs, gates, casting circles).
    fn dress(&mut self, g: &Game) {
        let mut busy: Vec<Cell> = vec![g.player_spawn, g.exit];
        busy.extend(g.runes.iter().map(|r| r.tile));
        busy.extend(g.gates.iter().flat_map(|gt| [gt.a, gt.b]));
        if let Some(wl) = &g.warlock {
            busy.extend(wl.circles.iter().copied());
        }
        if let Some(b) = &g.bonus {
            busy.extend(b.stones.iter().copied());
        }
        let free = |cl: Cell| g.walls.floor(cl.c, cl.r) && !busy.iter().any(|b| (b.c - cl.c).abs() + (b.r - cl.r).abs() <= 1);
        let salt = g.room_stamp as i32 % 7;
        self.banners.clear();
        for r in 0..ROWS as i32 - 1 {
            for cc in 1..COLS as i32 - 1 {
                let cl = Cell { c: cc, r };
                if !g.walls.get(cc, r) || !g.walls.floor(cc, r + 1) || g.torches.iter().any(|t| (t.c - cc).abs() <= 1 && t.r == r) {
                    continue;
                }
                let spaced = self.banners.iter().all(|b| (b.c - cc).abs() + (b.r - r).abs() >= 4);
                if spaced && hash2(cc + salt, r * 3) > 0.62 {
                    self.banners.push(cl);
                }
            }
        }
        self.candles.clear();
        for r in 1..ROWS as i32 - 1 {
            for cc in 1..COLS as i32 - 1 {
                let cl = Cell { c: cc, r };
                if !free(cl) {
                    continue;
                }
                let walls = [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().filter(|&&(dc, dr)| g.walls.get(cc + dc, r + dr)).count();
                let spaced = self.candles.iter().all(|b| (b.c - cc).abs() + (b.r - r).abs() >= 5);
                if walls >= 2 && spaced && self.candles.len() < 4 && hash2(cc * 5, r + salt) > 0.45 {
                    self.candles.push(cl);
                }
            }
        }
        self.skull = None;
        'find: for r in (1..ROWS as i32 - 1).rev() {
            for cc in 1..COLS as i32 - 2 {
                let (a, b) = (Cell { c: cc, r }, Cell { c: cc + 1, r });
                if free(a) && free(b) && g.walls.get(cc, r + 1) && g.walls.get(cc + 1, r + 1) && !self.candles.contains(&a) && !self.candles.contains(&b) && hash2(cc + salt, r) > 0.55 {
                    self.skull = Some((a, hash2(r, cc) > 0.5));
                    break 'find;
                }
            }
        }
    }

    /// Rebuild the light map for this frame.
    fn relight(&mut self, g: &Game, t: f32) {
        self.bake(g);
        for (i, l) in self.light.iter_mut().enumerate() {
            let v = self.vignette[i];
            *l = [AMBIENT[0] * v, AMBIENT[1] * v, AMBIENT[2] * v];
        }
        for (k, b) in self.torches.iter().enumerate() {
            let f = torch_flicker(t, k);
            let col = [TORCH_LIGHT[0] * f, TORCH_LIGHT[1] * f, TORCH_LIGHT[2] * f];
            for &(i, w) in &b.weights {
                let l = &mut self.light[i as usize];
                l[0] += col[0] * w;
                l[1] += col[1] * w;
                l[2] += col[2] * w;
            }
        }
        for (k, b) in self.candle_light.iter().enumerate() {
            let f = torch_flicker(t * 1.3, k + 11) * 0.8;
            for &(i, w) in &b.weights {
                let l = &mut self.light[i as usize];
                l[0] += 1.2 * f * w;
                l[1] += 0.8 * f * w;
                l[2] += 0.42 * f * w;
            }
        }
        let mut lights: Vec<(f32, f32, f32, [f32; 3])> = Vec::new();
        for idx in 0..2 {
            if g.active[idx] && g.state != State::Title {
                let (x, y) = g.wizard_px(idx);
                let col = if idx == 0 { [0.85, 0.92, 1.15] } else { [1.15, 0.85, 0.7] };
                let boost = 1.0 + g.wizards[idx].cast_t * 2.0;
                lights.push((x, y, 150.0, [col[0] * boost, col[1] * boost, col[2] * boost]));
            }
        }
        for b in &g.bolts {
            let (x, y) = proj_px(b);
            if b.proj == Proj::Lance {
                lights.push((x, y, 140.0, [1.3, 1.0, 1.5]));
            } else {
                lights.push((x, y, 90.0, [0.6, 0.85, 1.3]));
            }
        }
        if let Some((x, y)) = g.whelp_px() {
            lights.push((x, y, 80.0, [1.2, 0.6, 0.25]));
        }
        for f in &g.fires {
            let (x, y) = proj_px(f);
            let col = if f.proj == Proj::Warlock { [1.0, 0.45, 1.3] } else { [1.4, 0.6, 0.2] };
            lights.push((x, y, 85.0, col));
        }
        for r in &g.runes {
            let (x, y) = center(r.tile);
            if r.cleansed {
                lights.push((x, y, 90.0, [1.1, 0.85, 0.35]));
            } else {
                let p = 0.35 + 0.15 * (t * 2.5 + x).sin();
                lights.push((x, y, 55.0, [0.6 * p, 0.2 * p, 0.9 * p]));
            }
        }
        if g.exit_open {
            let (x, y) = center(g.exit);
            lights.push((x, y, 110.0, [1.0, 0.85, 0.5]));
        }
        for gate in &g.gates {
            if gate.cooldown <= 0.0 {
                for cl in [gate.a, gate.b] {
                    let (x, y) = center(cl);
                    lights.push((x, y, 60.0, [0.3, 0.7, 1.0]));
                }
            }
        }
        if let Some(wl) = &g.warlock {
            if wl.wards > 0 {
                let (x, y) = center(wl.circles[wl.circle]);
                let k = if wl.state == WState::Materializing { 1.0 - wl.timer / 0.7 } else { 1.0 };
                lights.push((x, y, 120.0, [0.9 * k, 0.35 * k, 1.3 * k]));
            }
        }
        if let Some(b) = &g.bonus {
            for (i, s) in b.stones.iter().enumerate() {
                let lit = b.showing && i == b.seq[b.show_i.min(b.seq.len() - 1)];
                let (x, y) = center(*s);
                lights.push((x, y, if lit { 120.0 } else { 50.0 }, if lit { [1.3, 1.0, 0.4] } else { [0.5, 0.2, 0.8] }));
            }
        }
        for f in &g.magic.flashes {
            let k = 1.0 - f.t / f.life;
            let k = k * k;
            let col = [f.c[0] * k * 1.6, f.c[1] * k * 1.6, f.c[2] * k * 1.6];
            if f.through {
                // a reveal's light: an expanding ring of it, seen through stone
                let e = 1.0 - (1.0 - f.t / f.life).powi(2);
                add_ring(&mut self.light, f.x, f.y, f.r * e, 70.0, col);
            } else {
                lights.push((f.x, f.y, f.r * (0.6 + 0.4 * k), col));
            }
        }
        for (x, y, r, col) in lights {
            add_light(&mut self.light, &g.walls, x, y, r, col);
        }
        for (i, l) in self.light.iter().enumerate() {
            let o = i * 4;
            self.bytes[o] = ((l[0] * 0.5).min(1.0) * 255.0) as u8;
            self.bytes[o + 1] = ((l[1] * 0.5).min(1.0) * 255.0) as u8;
            self.bytes[o + 2] = ((l[2] * 0.5).min(1.0) * 255.0) as u8;
            self.bytes[o + 3] = 255;
        }
        self.light_tex.update_from_bytes(LW as u32, LH as u32, &self.bytes);
    }

    /// The tint for a figure standing at `x, y`: the light there, lifted so
    /// that nothing that can hurt the wizard ever hides in the dark.
    fn figure_tint(&self, x: f32, y: f32, a: f32) -> Color {
        let l = self.light_at(x, y);
        let f = |v: f32| (0.62 + v * 0.45).min(1.15);
        c(f(l[0]).min(1.0), f(l[1]).min(1.0), f(l[2]).min(1.0), a)
    }

    // ---- the frame ----------------------------------------------------------------

    pub fn draw_frame(&mut self, blip: &Blip, g: &Game, t: f32) {
        match g.state {
            State::Title => self.draw_title(blip, g, t),
            State::Over => {
                self.draw_cellar(blip, g, t);
                draw_rectangle(0.0, 0.0, WIN_W as f32, WIN_H as f32, c(0.02, 0.02, 0.04, 0.72));
                draw_over(blip, g);
            }
            State::Play | State::Dead | State::Bonus => {
                self.draw_cellar(blip, g, t);
                self.draw_panel(blip, g, t);
                draw_taunt(blip, g);
                blip.draw_hud(g.sess.score, hud_lives(g));
                if g.intro > 0.0 && g.state == State::Play {
                    self.draw_title_card(blip, g);
                }
            }
        }
    }

    fn draw_cellar(&mut self, blip: &Blip, g: &Game, t: f32) {
        let shake = g.magic.shake;
        let (sx, sy) = if shake > 0.0 {
            (rand_int(-100, 100) as f32 / 100.0 * shake, rand_int(-100, 100) as f32 / 100.0 * shake)
        } else {
            (0.0, 0.0)
        };
        blip.set_offset(sx.round(), sy.round());
        self.relight(g, t);
        self.draw_stone(g);
        self.draw_floor_pieces(g, t);
        self.draw_haze(t);
        // light pass
        if let Some(m) = &self.mul {
            gl_use_material(m);
            draw_texture_ex(
                &self.light_tex,
                0.0,
                HUD_H as f32,
                blip::macroquad::color::WHITE,
                DrawTextureParams { dest_size: Some(vec2(WIN_W as f32, (ROWS as i32 * CELL) as f32)), ..Default::default() },
            );
            gl_use_default_material();
        }
        self.draw_smoke(&g.magic);
        if g.state != State::Title {
            self.draw_figures(g, t);
        }
        self.additive(true);
        self.draw_glows(g, t);
        self.draw_magic(&g.magic, t);
        self.additive(false);
        g.fx.draw(blip);
        blip.set_offset(0.0, 0.0);
        if let Some((col, tt, life)) = g.magic.tint {
            let k = 1.0 - tt / life;
            draw_rectangle(0.0, HUD_H as f32, WIN_W as f32, (ROWS as i32 * CELL) as f32, with_a(col, col.a * k));
        }
    }

    fn draw_stone(&self, g: &Game) {
        let wall = |c: i32, r: i32| !Walls::in_bounds(c, r) || g.walls.get(c, r);
        draw_texture_ex(&self.floor, 0.0, HUD_H as f32, blip::macroquad::color::WHITE, DrawTextureParams::default());
        for r in 0..ROWS as i32 {
            for cc in 0..COLS as i32 {
                let x = (cc * CELL) as f32;
                let y = (HUD_H + r * CELL) as f32;
                let h = hash2(cc, r);
                if wall(cc, r) {
                    let cap = [0u32, 1, 2, 6][(h * 4.0) as usize % 4];
                    self.tile(cap, 1, x, y);
                    if !wall(cc, r + 1) {
                        // the wall's front face, above the floor below it
                        let face = [3u32, 4, 5, 7][((h * 13.0) as usize) % 4];
                        self.tile_part(face, 1, 0.0, 15.0, x, y + 17.0);
                    }
                    // a worn lit rim wherever the wall top meets open floor
                    let rim = c(0.42, 0.43, 0.5, 1.0);
                    if !wall(cc, r + 1) {
                        draw_rectangle(x, y + 15.0, CELL as f32, 2.0, c(0.62, 0.62, 0.66, 1.0));
                    }
                    if !wall(cc, r - 1) {
                        draw_rectangle(x, y, CELL as f32, 2.0, rim);
                    }
                    if !wall(cc - 1, r) {
                        draw_rectangle(x, y, 2.0, if wall(cc, r + 1) { CELL as f32 } else { 17.0 }, rim);
                    }
                    if !wall(cc + 1, r) {
                        draw_rectangle(x + CELL as f32 - 2.0, y, 2.0, if wall(cc, r + 1) { CELL as f32 } else { 17.0 }, rim);
                    }
                } else {
                    if h > 0.88 {
                        self.tile(4 + ((h * 97.0) as u32 % 4), 0, x, y);
                    }
                    // contact shadow where floor meets wall
                    if wall(cc, r - 1) {
                        for k in 0..5 {
                            draw_rectangle(x, y + k as f32 * 1.5, CELL as f32, 1.5, c(0.0, 0.0, 0.02, 0.5 - k as f32 * 0.1));
                        }
                    }
                    if wall(cc - 1, r) {
                        for k in 0..3 {
                            draw_rectangle(x + k as f32 * 1.5, y, 1.5, CELL as f32, c(0.0, 0.0, 0.02, 0.4 - k as f32 * 0.12));
                        }
                    }
                    if wall(cc + 1, r) {
                        for k in 0..3 {
                            draw_rectangle(x + CELL as f32 - (k + 1) as f32 * 1.5, y, 1.5, CELL as f32, c(0.0, 0.0, 0.02, 0.4 - k as f32 * 0.12));
                        }
                    }
                }
            }
        }
        let white = blip::macroquad::color::WHITE;
        for &tc in &g.torches {
            let (x, y) = torch_flame(tc);
            self.sprite(0, 5, x, y + 8.0, false, white);
        }
        for &b in &self.banners {
            let (x, y) = center(b);
            self.sprite(4, 5, x, y + 12.0, false, white);
        }
        if let Some((cl, left)) = self.skull {
            let (x, y) = center(cl);
            let (col, row) = if left { (0.0, 7.0) } else { (5.0, 5.0) };
            draw_texture_ex(&self.sprites, x - 16.0, y - 16.0, white, DrawTextureParams {
                dest_size: Some(vec2(64.0, 32.0)),
                source: Some(Rect::new(col * SPR, row * SPR, 64.0, 32.0)),
                ..Default::default()
            });
        }
        for &cl in &self.candles {
            let (x, y) = center(cl);
            self.sprite(3, 5, x, y, false, white);
        }
    }

    /// Smoke haze drifting through the cellar, drawn before the light pass so
    /// torchlight catches it.
    fn draw_haze(&self, t: f32) {
        for k in 0..9 {
            let fk = k as f32;
            let x = (fk * 71.0 + t * (6.0 + fk)).rem_euclid(WIN_W as f32 + 200.0) - 100.0;
            let y = HUD_H as f32 + 40.0 + (fk * 53.0).rem_euclid(340.0) + (t * 0.3 + fk).sin() * 14.0;
            self.glow(x, y, 120.0 + (fk * 17.0).rem_euclid(60.0), c(0.55, 0.55, 0.6, 0.10));
        }
    }

    fn draw_floor_pieces(&self, g: &Game, t: f32) {
        let white = blip::macroquad::color::WHITE;
        // the stair well down
        if g.state != State::Bonus && g.state != State::Title {
            let (x, y) = center(g.exit);
            self.sprite(if g.exit_open { 6 } else { 5 }, 4, x, y, false, white);
        }
        for gate in &g.gates {
            for cl in [gate.a, gate.b] {
                let (x, y) = center(cl);
                let tint = if gate.cooldown > 0.0 { c(0.3, 0.3, 0.35, 1.0) } else { white };
                self.sprite_rot(7, 4, x, y, 32.0, t * 0.6, tint);
            }
        }
        if let Some(wl) = &g.warlock {
            for (i, cl) in wl.circles.iter().enumerate() {
                let (x, y) = center(*cl);
                let live = i == wl.circle && wl.wards > 0;
                let col = if wl.sealed[i] { c(0.35, 0.33, 0.4, 1.0) } else if live { c(0.85, 0.45, 1.0, 1.0) } else { c(0.45, 0.25, 0.55, 1.0) };
                draw_circle_lines(x, y, 13.0, 1.5, col);
                draw_circle_lines(x, y, 9.0, 1.0, with_a(col, 0.7));
                // a ring of runes turning between two circles
                for k in 0..8 {
                    let a = k as f32 / 8.0 * TAU - t * if live { 1.2 } else { 0.2 };
                    draw_glyph(x + a.cos() * 11.0, y + a.sin() * 11.0, k, 1.6, with_a(col, 0.85));
                }
                draw_circle_lines(x, y, 3.0, 1.0, with_a(col, 0.6));
            }
        }
        for r in &g.runes {
            let (x, y) = center(r.tile);
            self.sprite(if r.cleansed { 4 } else { 3 }, 4, x, y - 2.0, false, white);
        }
        if let Some(b) = &g.bonus {
            for (i, s) in b.stones.iter().enumerate() {
                let (x, y) = center(*s);
                let lit = b.showing && i == b.seq[b.show_i.min(b.seq.len() - 1)];
                self.sprite(if lit { 4 } else { 3 }, 4, x, y - 2.0, false, white);
            }
        }
    }

    fn draw_figures(&self, g: &Game, t: f32) {
        // soft shadows first, so no figure's shadow falls over another figure
        let shadow = c(0.0, 0.0, 0.0, 0.45);
        let mut feet: Vec<(f32, f32, f32)> = Vec::new();
        for e in g.enemies.iter().filter(|e| !e.gone && (e.kind != EnemyKind::Wraith || e.visible)) {
            let (x, y) = g.enemy_px(e);
            let w = if e.kind == EnemyKind::Imp { 7.0 } else { 10.0 };
            feet.push((x, y + if e.kind == EnemyKind::Imp { 15.0 } else { 13.0 }, w));
        }
        for idx in 0..2 {
            if g.active[idx] {
                let (x, y) = g.wizard_px(idx);
                feet.push((x, y + 14.0, 9.0));
            }
        }
        for (x, y, w) in feet {
            blip::macroquad::shapes::draw_ellipse(x, y, w, 3.0, 0.0, shadow);
        }

        // potions: a little glass flask, liquid glowing within, bobbing on the floor
        for p in &g.potions {
            let (x, y) = center(p.tile);
            if p.t > POTION_SECS - 2.0 && (p.t * 8.0) as i32 % 2 == 0 {
                continue;
            }
            let bob = (t * 3.0 + p.tile.c as f32).sin() * 1.5;
            let y = y + 2.0 + bob;
            let col = potion_color(p.kind);
            blip::macroquad::shapes::draw_ellipse(x, y + 9.0 - bob, 6.0, 2.0, 0.0, shadow);
            draw_circle(x, y + 2.0, 6.0, c(0.08, 0.08, 0.12, 0.9));
            draw_circle(x, y + 3.0, 4.6, with_a(col, 0.9));
            draw_rectangle(x - 2.0, y - 7.0, 4.0, 5.0, c(0.55, 0.6, 0.7, 0.9));
            draw_rectangle(x - 2.5, y - 9.0, 5.0, 2.5, c(0.45, 0.3, 0.18, 1.0));
            draw_circle(x - 2.0, y + 0.5, 1.4, c(1.0, 1.0, 1.0, 0.7));
        }

        for s in &g.sightings {
            let k = 1.0 - s.t / SIGHTING_FADE;
            let (x, y) = center(s.tile);
            self.sprite(2, 2, x, y, false, c(0.4, 0.9, 1.0, 0.35 * k));
        }
        self.draw_corpses(&g.magic);
        self.draw_warlock(g, t);
        for e in g.enemies.iter().filter(|e| !e.gone) {
            self.draw_enemy(g, e, t);
        }
        if let (Some(w), Some((x, y))) = (&g.whelp, g.whelp_px()) {
            let f = (t * 12.0) as u32 % 3;
            let swoop = (t * 7.0).sin() * 3.0 - 6.0;
            blip::macroquad::shapes::draw_ellipse(x, y + 14.0, 8.0, 2.5, 0.0, c(0.0, 0.0, 0.0, 0.3));
            // it flickers in its last second, about to fly off
            if w.left > 1.0 || (t * 12.0) as i32 % 2 == 0 {
                self.sprite(f, 4, x, y + swoop, w.face_left, blip::macroquad::color::WHITE);
            }
        }
        for idx in 0..2 {
            self.draw_wizard(g, idx, t);
        }
    }

    /// Struck figures: a white-hot flash, then the sprite unravels upward in
    /// thinning slices while it burns its colour away.
    fn draw_corpses(&self, m: &Magic) {
        for cp in &m.corpses {
            let k = cp.t / cp.life;
            let a = 1.0 - k;
            let slices = 8;
            for sl in 0..slices {
                let fs = sl as f32 / slices as f32;
                // upper slices lift away first
                let lift = (k * 30.0) * (1.2 - fs) + (k * k * 20.0);
                let sway = ((sl as f32 * 1.7 + k * 12.0).sin()) * k * 6.0;
                let sy = cp.row as f32 * SPR + fs * SPR;
                let sh = SPR / slices as f32;
                let w = SPR * (1.0 - k * 0.5);
                draw_texture_ex(
                    &self.sprites,
                    (cp.x - w / 2.0 + sway).round(),
                    (cp.y - SPR / 2.0 + fs * SPR - lift).round(),
                    c(1.0, 1.0, 1.0, a),
                    DrawTextureParams {
                        dest_size: Some(vec2(w, sh)),
                        source: Some(Rect::new(cp.col as f32 * SPR, sy, SPR, sh)),
                        flip_x: cp.flip,
                        ..Default::default()
                    },
                );
            }
            // burn: the colour and then white-hot, added over the sprite
            self.additive(true);
            let heat = (1.0 - k / 0.25).max(0.0) * 0.9;
            for _ in 0..1 {
                draw_texture_ex(
                    &self.sprites,
                    (cp.x - SPR / 2.0).round(),
                    (cp.y - SPR / 2.0 - k * 10.0).round(),
                    with_a(cp.c, heat),
                    DrawTextureParams {
                        dest_size: Some(vec2(SPR, SPR)),
                        source: Some(Rect::new(cp.col as f32 * SPR, cp.row as f32 * SPR, SPR, SPR)),
                        flip_x: cp.flip,
                        ..Default::default()
                    },
                );
            }
            self.glow(cp.x, cp.y - k * 10.0, 20.0, with_a(cp.c, 0.35 * (1.0 - k)));
            self.additive(false);
        }
    }

    fn draw_enemy(&self, g: &Game, e: &Enemy, t: f32) {
        let (x, y) = g.enemy_px(e);
        let ph = (e.tile.c * 3 + e.tile.r * 5) as f32;
        match e.kind {
            EnemyKind::Imp => {
                let f = ((t * 9.0 + ph) as i32 & 1) as u32;
                let bob = (t * 9.0 + ph).sin() * 1.5;
                self.sprite(f, 2, x, y + bob - 1.0, e.face_left, self.figure_tint(x, y, 1.0));
            }
            EnemyKind::Wraith => {
                if !e.visible {
                    return;
                }
                let a = wraith_alpha(e, t);
                let f = 2 + ((t * 4.0 + ph) as i32 & 1) as u32;
                let drift = (t * 3.0 + ph).sin() * 2.0;
                let tint = self.figure_tint(x, y, a);
                self.sprite(f, 2, x, y + drift - 2.0, e.face_left, tint);
            }
            EnemyKind::Spellhound => {
                let moving = e.lerp < 1.0;
                let f = if moving { 4 + ((t * (if e.hunting { 14.0 } else { 9.0 })) as i32 % 3) as u32 } else { 4 };
                self.sprite(f, 2, x, y, e.face_left, self.figure_tint(x, y, 1.0));
            }
            EnemyKind::Gargoyle => {
                let spread = e.windup > 0.0;
                let f = if spread { 2 + ((t * 10.0) as i32 & 1) as u32 } else { ((t * 2.0 + ph) as i32 & 1) as u32 };
                let shiver = if spread { (t * 60.0).sin() * 0.8 } else { 0.0 };
                self.sprite(f, 3, x + shiver, y, false, self.figure_tint(x, y, 1.0));
            }
        }
    }

    fn draw_warlock(&self, g: &Game, t: f32) {
        let Some(wl) = &g.warlock else { return };
        if wl.wards <= 0 {
            return;
        }
        let (x, y) = center(wl.circles[wl.circle]);
        let fade = match wl.state {
            WState::Materializing => (1.0 - wl.timer / 0.7).clamp(0.0, 1.0),
            _ => 1.0,
        };
        let cast = matches!(wl.state, WState::Aiming | WState::Casting);
        let f = if cast { 6 } else { 4 + ((t * 3.0) as i32 & 1) as u32 };
        let hover = (t * 2.5).sin() * 2.0 - 3.0;
        // the figure rises out of the floor as it materialises
        let size = 42.0 * (0.6 + 0.4 * fade);
        // a cloak of smoke around him
        for k in 0..6 {
            let a = t * 0.8 + k as f32 * TAU / 6.0;
            let r = 14.0 + (t * 2.0 + k as f32).sin() * 3.0;
            self.glow(x + a.cos() * r, y + 6.0 + a.sin() * r * 0.4, 14.0, c(0.08, 0.04, 0.12, 0.5 * fade));
        }
        self.sprite_sized(f, 3, x, y + hover + (1.0 - fade) * 8.0, size, wl.aim_dir == Dir::Left, c(1.0, 1.0, 1.0, fade));
    }

    fn draw_wizard(&self, g: &Game, idx: usize, t: f32) {
        if !g.active[idx] {
            return;
        }
        let w = &g.wizards[idx];
        if g.state == State::Dead && g.dead_index == Some(idx) && (g.dead_timer.remaining() / 0.12) as i32 % 2 == 0 {
            return;
        }
        let (x, y) = g.wizard_px(idx);
        let row = idx as u32;
        let stepping = w.moving && w.lerp > 0.1 && w.lerp < 0.85;
        let alt = (w.tile.c + w.tile.r) & 1 == 1;
        let casting = w.cast_t > 0.0;
        let (col, flip) = match w.facing {
            Dir::Down => {
                if casting { (6, false) } else if stepping { (1, alt) } else { (0, false) }
            }
            Dir::Up => {
                if stepping { (3, !alt) } else { (2, true) }
            }
            Dir::Right => {
                if casting { (7, false) } else if stepping { (5, false) } else { (4, false) }
            }
            Dir::Left => {
                if casting { (7, true) } else if stepping { (5, true) } else { (4, true) }
            }
        };
        let bob = if stepping { -1.0 } else { (t * 2.0 + idx as f32).sin() * 0.4 };
        let tint = self.figure_tint(x, y, 1.0);
        let tint = c(tint.r.max(0.85), tint.g.max(0.85), tint.b.max(0.85), 1.0);
        self.sprite(col, row, x, y - 1.0 + bob, flip, tint);
    }

    /// Everything that gives off light, drawn additively over the figures.
    fn draw_glows(&self, g: &Game, t: f32) {
        for p in &g.potions {
            let (x, y) = center(p.tile);
            let k = 0.7 + 0.3 * (t * 4.0 + p.tile.r as f32).sin();
            self.glow(x, y + 4.0, 22.0 * k, with_a(potion_color(p.kind), 0.35));
        }
        // a hasted wizard runs in a wash of blue light
        for idx in 0..2 {
            if g.active[idx] && g.wizards[idx].haste > 0.0 {
                let (x, y) = g.wizard_px(idx);
                let fade = (g.wizards[idx].haste / 1.0).min(1.0);
                self.glow(x, y + 4.0, 30.0, c(0.3, 0.8, 1.0, 0.3 * fade));
                for k in 0..4 {
                    let a = t * 6.0 + k as f32 * TAU / 4.0;
                    self.glow(x + a.cos() * 12.0, y + 6.0 + a.sin() * 5.0, 2.5, c(0.6, 0.95, 1.0, 0.8 * fade));
                }
            }
        }
        // torch flames
        for (k, &tc) in g.torches.iter().enumerate() {
            let (x, y) = torch_flame(tc);
            let f = torch_flicker(t, k);
            self.glow(x, y, 44.0 * f, c(1.0, 0.5, 0.18, 0.3));
            self.glow(x, y - 2.0, 11.0 * f, c(1.0, 0.6, 0.22, 0.9));
            let sway = (t * 7.0 + k as f32).sin() * 1.2;
            self.streak(x + sway * 0.5, y - 5.0, 9.0, 20.0 * f, 0.0, c(1.0, 0.72, 0.32, 0.95));
            self.streak(x + sway, y - 4.0, 4.0, 10.0 * f, 0.0, c(1.0, 0.95, 0.75, 1.0));
        }
        for (k, &cl) in self.candles.iter().enumerate() {
            let (x, y) = center(cl);
            self.glow(x, y - 4.0, 22.0, c(1.0, 0.6, 0.25, 0.25));
            for &(dx, top) in &[(-7.0f32, 17.0f32), (-2.0, 11.0), (3.0, 15.0), (7.0, 20.0), (-4.0, 21.0)] {
                let f = torch_flicker(t * 1.5, k * 5 + top as usize);
                let (fx, fy) = (x + dx, y - 16.0 + top - 3.0);
                self.streak(fx, fy, 3.0, 6.0 * f, 0.0, c(1.0, 0.75, 0.35, 0.9));
                self.glow(fx, fy + 0.5, 1.6, c(1.0, 0.95, 0.8, 1.0));
            }
        }
        if g.state == State::Title {
            return;
        }
        // the staffs' crystals
        for idx in 0..2 {
            if !g.active[idx] || (g.state == State::Dead && g.dead_index == Some(idx)) {
                continue;
            }
            let w = &g.wizards[idx];
            let (x, y) = g.wizard_px(idx);
            let (tx, ty) = staff_tip(w);
            let col = if idx == 0 { c(0.55, 0.85, 1.0, 1.0) } else { c(1.0, 0.7, 0.4, 1.0) };
            let pulse = 0.8 + 0.2 * (t * 5.0 + idx as f32).sin() + w.cast_t * 4.0;
            self.glow(x + tx, y + ty, 14.0 * pulse, with_a(col, 0.5));
            self.glow(x + tx, y + ty, 4.0 * pulse, c(1.0, 1.0, 1.0, 0.9));
            if w.protect > 0.0 {
                let k = (w.protect / RESPAWN_PROTECT).min(1.0);
                let r = 20.0 + (t * 6.0).sin() * 1.5;
                draw_circle_lines(x, y, r, 1.5, with_a(col, 0.6 * k));
                self.glow(x, y, r + 6.0, with_a(col, 0.18 * k));
                for j in 0..6 {
                    let a = t * 2.0 + j as f32 / 6.0 * TAU;
                    self.glow(x + a.cos() * r, y + a.sin() * r, 3.0, with_a(col, 0.8 * k));
                }
            }
        }
        // monster eyes and auras
        for e in g.enemies.iter().filter(|e| !e.gone) {
            let (x, y) = g.enemy_px(e);
            let fl = if e.face_left { -1.0 } else { 1.0 };
            match e.kind {
                EnemyKind::Imp => {
                    let bob = (t * 9.0 + (e.tile.c * 3 + e.tile.r * 5) as f32).sin() * 1.5;
                    self.glow(x, y - 3.0 + bob, 9.0, c(1.0, 0.85, 0.3, 0.35));
                }
                EnemyKind::Wraith => {
                    if !e.visible {
                        continue;
                    }
                    let a = wraith_alpha(e, t);
                    let drift = (t * 3.0 + (e.tile.c * 3 + e.tile.r * 5) as f32).sin() * 2.0;
                    self.glow(x, y, 22.0, c(0.4, 0.6, 1.0, 0.22 * a));
                    self.glow(x - 2.0, y - 7.0 + drift, 3.0, c(0.6, 1.0, 1.0, a));
                    self.glow(x + 2.0, y - 7.0 + drift, 3.0, c(0.6, 1.0, 1.0, a));
                    if e.revealed > 0.0 || (e.lunging && e.charge > 0.0) {
                        draw_circle_lines(x, y, 15.0 + (t * 12.0).sin(), 1.5, c(0.6, 0.95, 1.0, 0.7 * a));
                    }
                }
                EnemyKind::Spellhound => {
                    let k = if e.hunting { 1.0 } else { 0.65 };
                    self.glow(x + fl * 10.0, y - 5.0, 4.0 + 3.0 * k, c(1.0, 0.5, 0.15, 0.9));
                    self.glow(x, y - 3.0, 14.0 + 8.0 * k, c(1.0, 0.45, 0.1, 0.3 * k));
                    if e.hunting {
                        self.streak(x - fl * 14.0, y - 4.0, 26.0, 6.0, 0.0, c(1.0, 0.4, 0.1, 0.35));
                    }
                }
                EnemyKind::Gargoyle => {
                    let charge = if e.windup > 0.0 { 1.0 - e.windup / GARGOYLE_TELL } else { 0.0 };
                    self.glow(x, y - 2.0, 6.0 + charge * 16.0, c(1.0, 0.55, 0.15, 0.4 + charge * 0.6));
                    if charge > 0.0 {
                        // embers converge on the maw: the tell
                        for j in 0..8 {
                            let a = j as f32 / 8.0 * TAU + t * 3.0;
                            let r = 26.0 * (1.0 - charge);
                            self.glow(x + a.cos() * r, y - 2.0 + a.sin() * r, 2.5, c(1.0, 0.7, 0.3, charge));
                        }
                    }
                }
            }
        }
        if let (Some(w), Some((x, y))) = (&g.whelp, g.whelp_px()) {
            let swoop = (t * 7.0).sin() * 3.0 - 6.0;
            let fl = if w.face_left { -1.0 } else { 1.0 };
            self.glow(x, y + swoop, 24.0, c(1.0, 0.6, 0.2, 0.3));
            self.glow(x + fl * 11.0, y + swoop - 6.0, 4.0, c(1.0, 0.8, 0.4, 0.9));
            // a ring that shrinks as its time runs out
            let k = (w.left / WHELP_SECS).clamp(0.0, 1.0);
            draw_circle_lines(x, y + swoop, 10.0 + 10.0 * k, 1.0, c(1.0, 0.75, 0.3, 0.5));
        }
        // the Warlock's aura and aim
        if let Some(wl) = &g.warlock {
            if wl.wards > 0 {
                let (x, y) = center(wl.circles[wl.circle]);
                let fade = if wl.state == WState::Materializing { (1.0 - wl.timer / 0.7).clamp(0.0, 1.0) } else { 1.0 };
                self.glow(x, y, 34.0, c(0.7, 0.3, 1.0, 0.35 * fade));
                if wl.state == WState::Materializing {
                    for j in 0..10 {
                        let a = j as f32 / 10.0 * TAU + t * 6.0;
                        let r = 30.0 * (1.0 - fade) + 6.0;
                        self.glow(x + a.cos() * r, y + a.sin() * r * 0.6, 3.0, c(0.85, 0.5, 1.0, 0.9));
                    }
                }
                let hover = (t * 2.5).sin() * 2.0 - 3.0;
                self.glow(x - 1.5, y - 8.0 + hover, 2.5, c(1.0, 0.7, 1.0, fade));
                self.glow(x + 1.5, y - 8.0 + hover, 2.5, c(1.0, 0.7, 1.0, fade));
                if matches!(wl.state, WState::Aiming | WState::Casting) {
                    let k = if wl.state == WState::Casting { 1.0 } else { 1.0 - wl.timer / 0.95 };
                    let mut p = wl.aim_from;
                    let (dc, dr) = wl.aim_dir.delta();
                    let ang = if dc != 0 { 0.0 } else { std::f32::consts::FRAC_PI_2 };
                    for n in 0..GARGOYLE_RANGE {
                        p = Cell { c: p.c + dc, r: p.r + dr };
                        if g.walls.get(p.c, p.r) || !Walls::in_bounds(p.c, p.r) {
                            break;
                        }
                        let (lx, ly) = center(p);
                        let pulse = 0.5 + 0.5 * (t * 10.0 - n as f32 * 0.8).sin();
                        self.streak(lx, ly, 36.0, 8.0 + 6.0 * k, ang, c(0.8, 0.35, 1.0, 0.35 + 0.5 * k * pulse));
                        self.glow(lx, ly, 4.0, c(1.0, 0.7, 1.0, 0.8 * k));
                        draw_glyph(lx, ly, n as usize, 4.0, c(1.0, 0.8, 1.0, 0.7 * k));
                    }
                    for j in 0..3 {
                        let a = t * 9.0 + j as f32 * TAU / 3.0;
                        self.glow(x + a.cos() * 14.0, y - 8.0 + a.sin() * 6.0, 3.0, c(0.9, 0.5, 1.0, k));
                    }
                }
            }
        }
        // runes, gates and the open stairs
        for r in &g.runes {
            let (x, y) = center(r.tile);
            if r.cleansed {
                self.glow(x, y - 2.0, 18.0, c(1.0, 0.8, 0.3, 0.45 + 0.1 * (t * 3.0).sin()));
            } else {
                let p = 0.5 + 0.5 * (t * 2.5 + x).sin();
                self.glow(x, y - 2.0, 10.0 + p * 4.0, c(0.7, 0.3, 1.0, 0.25 + 0.3 * p));
            }
        }
        for gate in &g.gates {
            if gate.cooldown <= 0.0 {
                for cl in [gate.a, gate.b] {
                    let (x, y) = center(cl);
                    self.glow(x, y, 22.0, c(0.3, 0.75, 1.0, 0.35 + 0.15 * (t * 4.0).sin()));
                }
            }
        }
        if g.exit_open && g.state != State::Bonus {
            let (x, y) = center(g.exit);
            self.glow(x, y, 30.0, c(1.0, 0.8, 0.4, 0.4 + 0.15 * (t * 4.0).sin()));
            self.streak(x, y - 18.0, 16.0, 44.0, 0.0, c(1.0, 0.85, 0.5, 0.3));
        }
        if let Some(b) = &g.bonus {
            for (i, s) in b.stones.iter().enumerate() {
                let lit = b.showing && i == b.seq[b.show_i.min(b.seq.len() - 1)];
                let (x, y) = center(*s);
                if lit {
                    self.glow(x, y, 34.0, c(1.0, 0.8, 0.3, 0.8));
                    self.streak(x, y - 30.0, 18.0, 70.0, 0.0, c(1.0, 0.85, 0.4, 0.5));
                }
            }
        }
        // spells in flight
        for b in &g.bolts {
            let (x, y) = proj_px(b);
            let (dx, dy) = b.dir.delta();
            let (fx, fy) = (dx as f32, dy as f32);
            let ang = fy.atan2(fx);
            if b.proj == Proj::Lance {
                // the lance: a spear of white-gold light with a violet sheath, runes turning on it
                self.streak(x - fx * 30.0, y - fy * 30.0, 90.0, 30.0, ang, c(0.6, 0.3, 1.0, 0.7));
                self.streak(x - fx * 22.0, y - fy * 22.0, 70.0, 10.0, ang, c(1.0, 0.85, 0.5, 1.0));
                self.streak(x - fx * 16.0, y - fy * 16.0, 50.0, 4.0, ang, c(1.0, 1.0, 1.0, 1.0));
                self.glow(x, y, 40.0, c(0.8, 0.6, 1.0, 0.8));
                self.glow(x, y, 14.0, c(1.0, 0.95, 0.8, 1.0));
                self.glow(x, y, 6.0, c(1.0, 1.0, 1.0, 1.0));
                for k in 0..3 {
                    let back = 12.0 + k as f32 * 14.0;
                    draw_glyph(x - fx * back - fy * 6.0, y - fy * back - fx * 6.0, k + (t * 10.0) as usize, 3.0, c(1.0, 0.85, 1.0, 0.9 - k as f32 * 0.25));
                }
                continue;
            }
            // a comet: a long cold tail, a hot white head, arcs crackling about it
            self.streak(x - fx * 26.0, y - fy * 26.0, 80.0, 22.0, ang, c(0.25, 0.5, 1.0, 0.45));
            self.streak(x - fx * 14.0, y - fy * 14.0, 46.0, 9.0, ang, c(0.6, 0.85, 1.0, 0.9));
            self.glow(x, y, 26.0, c(0.35, 0.6, 1.0, 0.55));
            self.glow(x, y, 9.0, c(0.85, 0.95, 1.0, 1.0));
            self.glow(x, y, 4.0, c(1.0, 1.0, 1.0, 1.0));
            let seed = (t * 40.0) as i32;
            for k in 0..3 {
                let mut px = x;
                let mut py = y;
                for s in 0..4 {
                    let nx = px + (hash2(seed + k * 7, s) - 0.5) * 16.0 - fx * 6.0;
                    let ny = py + (hash2(s, seed + k * 13) - 0.5) * 16.0 - fy * 6.0;
                    draw_line(px, py, nx, ny, 1.2, c(0.75, 0.9, 1.0, 0.9 - s as f32 * 0.2));
                    px = nx;
                    py = ny;
                }
            }
        }
        for f in &g.fires {
            let (x, y) = proj_px(f);
            let (dx, dy) = f.dir.delta();
            let ang = (dy as f32).atan2(dx as f32);
            if f.proj == Proj::Warlock {
                self.glow(x, y, 22.0, c(0.75, 0.3, 1.0, 0.6));
                self.glow(x, y, 7.0, c(1.0, 0.85, 1.0, 1.0));
                for j in 0..3 {
                    let a = t * 14.0 + j as f32 * TAU / 3.0;
                    self.glow(x + a.cos() * 9.0, y + a.sin() * 9.0, 2.5, c(0.9, 0.6, 1.0, 1.0));
                }
            } else {
                let fl = 1.0 + 0.15 * (t * 40.0).sin();
                self.streak(x - dx as f32 * 10.0, y - dy as f32 * 10.0, 30.0, 12.0 * fl, ang, c(1.0, 0.4, 0.1, 0.6));
                self.glow(x, y, 16.0 * fl, c(1.0, 0.5, 0.15, 0.6));
                self.glow(x, y, 6.0, c(1.0, 0.9, 0.5, 1.0));
            }
        }
    }

    fn draw_magic(&self, m: &Magic, t: f32) {
        for w in &m.waves {
            let k = w.t / w.life;
            let e = 1.0 - (1.0 - k) * (1.0 - k);
            let r = w.r * e;
            let a = (1.0 - k) * w.c.a;
            draw_circle_lines(w.x, w.y, r, w.width, with_a(w.c, a));
            // a soft luminous band behind the bright edge
            let n = (r * 0.35).clamp(12.0, 90.0) as usize;
            for j in 0..n {
                let ang = j as f32 / n as f32 * TAU;
                self.glow(w.x + ang.cos() * r * 0.97, w.y + ang.sin() * r * 0.97, w.width * 4.0 + 4.0, with_a(w.c, a * 0.22));
            }
            if w.glyphs {
                let n = 12;
                for j in 0..n {
                    let ang = j as f32 / n as f32 * TAU + t * 0.8;
                    let gx = w.x + ang.cos() * r;
                    let gy = w.y + ang.sin() * r;
                    draw_glyph(gx, gy, j, 4.0, with_a(w.c, a));
                    self.glow(gx, gy, 6.0, with_a(w.c, a * 0.5));
                }
            }
        }
        for p in &m.pillars {
            let k = p.t / p.life;
            let a = if k < 0.15 { k / 0.15 } else { 1.0 - (k - 0.15) / 0.85 };
            let h = 260.0;
            let wob = 1.0 + 0.08 * (t * 30.0).sin();
            self.streak(p.x, p.y - h / 2.0 + 8.0, p.w * 3.5 * wob, h * 1.2, 0.0, with_a(p.c, 0.35 * a));
            self.streak(p.x, p.y - h / 2.0 + 8.0, p.w * 1.6, h, 0.0, with_a(p.c, 0.7 * a));
            self.streak(p.x, p.y - h / 2.0 + 8.0, p.w * 0.6, h, 0.0, c(1.0, 1.0, 0.95, 0.9 * a));
            self.glow(p.x, p.y, p.w * 2.6, with_a(p.c, 0.8 * a));
            self.glow(p.x, p.y, p.w * 0.9, c(1.0, 1.0, 1.0, a));
        }
        for p in &m.parts {
            let k = p.t / p.life;
            let a = (1.0 - k) * p.c.a;
            match p.kind {
                Kind::Spark => {
                    let len = 0.035;
                    draw_line(p.x, p.y, p.x - p.vx * len, p.y - p.vy * len, p.size, with_a(p.c, a));
                    self.glow(p.x, p.y, p.size * 2.5, with_a(p.c, a * 0.6));
                }
                Kind::Mote => {
                    let tw = 0.65 + 0.35 * (p.t * 18.0 + p.spin * 3.0).sin();
                    self.glow(p.x, p.y, p.size * 3.0, with_a(p.c, a * 0.6 * tw));
                    self.glow(p.x, p.y, p.size * 0.9, c(1.0, 1.0, 1.0, a * tw));
                }
                Kind::Shard => {
                    let ang = p.spin * p.t;
                    let (ca, sa) = (ang.cos() * p.size, ang.sin() * p.size);
                    draw_line(p.x - ca, p.y - sa, p.x + ca, p.y + sa, 1.6, with_a(p.c, a));
                    self.glow(p.x, p.y, p.size * 2.0, with_a(p.c, a * 0.5));
                }
                Kind::Smoke => {}
            }
        }
    }

    fn draw_smoke(&self, m: &Magic) {
        for p in m.parts.iter().filter(|p| p.kind == Kind::Smoke) {
            let k = p.t / p.life;
            let r = p.size * (1.0 + k * 1.5);
            self.glow(p.x, p.y, r * 2.0, with_a(p.c, p.c.a * (1.0 - k)));
        }
    }

    /// The chamber's name on black, the way a chapter opens: it holds, then
    /// the cellar fades up behind it.
    fn draw_title_card(&self, blip: &Blip, g: &Game) {
        let left = g.intro / INTRO_SECS; // 1 → 0
        let dark = if left > 0.55 { 1.0 } else { left / 0.55 };
        let ink = (if left > 0.25 { 1.0 } else { left / 0.25 }).min(((1.0 - left) * 6.0).min(1.0));
        let top = HUD_H as f32;
        let h = (ROWS as i32 * CELL) as f32;
        draw_rectangle(0.0, top, WIN_W as f32, h, c(0.01, 0.01, 0.02, dark));
        let cy = top + h * 0.42;
        let gold = c(0.85, 0.66, 0.3, ink);
        let name = chamber_name(g);
        let sz = if name.chars().count() * 24 <= WIN_W as usize - 24 { 4.0 } else { 3.0 };
        let w = (WIN_W as f32 * 0.7) * ink;
        draw_rectangle(WIN_W as f32 / 2.0 - w / 2.0, cy - 16.0, w, 1.0, with_a(gold, ink * 0.8));
        draw_rectangle(WIN_W as f32 / 2.0 - w / 2.0, cy + sz * 7.0 + 12.0, w, 1.0, with_a(gold, ink * 0.8));
        blip.draw_centered_outlined(name, WIN_W as f32 / 2.0, cy, sz, gold, c(0.0, 0.0, 0.0, ink));
        let sub = if g.double { "THIS CHAMBER PAYS DOUBLE".to_string() } else { format!("CHAMBER {}", g.sess.level) };
        blip.draw_centered_outlined(&sub, WIN_W as f32 / 2.0, cy + sz * 7.0 + 24.0, 2.0, c(0.7, 0.68, 0.65, ink), c(0.0, 0.0, 0.0, ink));
        self.additive(true);
        self.streak(WIN_W as f32 / 2.0, cy + sz * 3.5, 360.0 * ink, 40.0, 0.0, c(1.0, 0.55, 0.2, 0.18 * ink));
        self.additive(false);
    }

    // ---- the panel under the cellar -------------------------------------------------

    fn draw_panel(&self, blip: &Blip, g: &Game, t: f32) {
        let y0 = (HUD_H + ROWS as i32 * CELL) as f32;
        let h = PANEL_H as f32;
        // a carved stone slab with a gilt edge
        for k in 0..(WIN_W / CELL) {
            for j in 0..3 {
                self.tile_part(3 + (k as u32 % 3), 1, 0.0, 32.0, (k * CELL) as f32, y0 + j as f32 * 32.0);
            }
        }
        draw_rectangle(0.0, y0, WIN_W as f32, h, c(0.02, 0.02, 0.04, 0.72));
        draw_rectangle(0.0, y0, WIN_W as f32, 2.0, c(0.79, 0.63, 0.29, 1.0));
        draw_rectangle(0.0, y0 + 2.0, WIN_W as f32, 1.0, c(0.3, 0.22, 0.1, 1.0));

        // the scrying glass: every monster, even the unseen, as a blip
        let cell = 5.0;
        let (rw, rh) = (COLS as f32 * cell, ROWS as f32 * cell);
        let rx = ((WIN_W as f32 - rw) / 2.0).round();
        let ry = y0 + 26.0;
        draw_rectangle(rx - 4.0, ry - 4.0, rw + 8.0, rh + 8.0, c(0.47, 0.37, 0.17, 1.0));
        draw_rectangle(rx - 3.0, ry - 3.0, rw + 6.0, rh + 6.0, c(0.79, 0.63, 0.29, 1.0));
        draw_rectangle(rx - 1.0, ry - 1.0, rw + 2.0, rh + 2.0, c(0.02, 0.03, 0.06, 1.0));
        for r in 0..ROWS as i32 {
            for cc in 0..COLS as i32 {
                if g.walls.get(cc, r) {
                    draw_rectangle(rx + cc as f32 * cell, ry + r as f32 * cell, cell, cell, c(0.1, 0.13, 0.2, 1.0));
                }
            }
        }
        let dot = |cl: Cell, col: Color| {
            draw_rectangle(rx + cl.c as f32 * cell + 1.0, ry + cl.r as f32 * cell + 1.0, cell - 2.0, cell - 2.0, col);
        };
        for r in &g.runes {
            dot(r.tile, if r.cleansed { RUNE_CLEAN } else { c(0.55, 0.25, 0.8, 1.0) });
        }
        if g.exit_open {
            dot(g.exit, EXIT_C);
        }
        for e in g.enemies.iter().filter(|e| !e.gone) {
            let col = match e.kind {
                EnemyKind::Imp => IMP,
                EnemyKind::Wraith => {
                    if e.visible { WRAITH } else { c(0.4, 0.5, 0.8, 0.35 + 0.25 * (t * 6.0).sin().abs()) }
                }
                EnemyKind::Spellhound => HOUND,
                EnemyKind::Gargoyle => GARGOYLE,
            };
            dot(e.tile, col);
        }
        if let Some(wl) = &g.warlock {
            if wl.wards > 0 {
                dot(wl.circles[wl.circle], WARLOCK_C);
            }
        }
        if let Some(w) = &g.whelp {
            dot(w.tile, if (t * 8.0) as i32 % 2 == 0 { FIREBALL } else { RUNE_CLEAN });
        }
        for idx in 0..2 {
            if g.active[idx] && (t * 4.0) as i32 % 2 == 0 {
                dot(g.wizards[idx].tile, if idx == 0 { WIZ_ROBE } else { WIZ_ROBE2 });
            }
        }
        // a sheen across the glass
        self.additive(true);
        self.streak(rx + rw * 0.3, ry + rh * 0.3, rw * 0.9, 10.0, -0.6, c(0.6, 0.7, 1.0, 0.12));
        self.additive(false);

        // the chamber's name above the glass
        let name = chamber_name(g);
        blip.draw_centered_outlined(name, WIN_W as f32 / 2.0, y0 + 5.0, 2.0, c(0.85, 0.7, 0.4, 1.0), BLIP_BLACK);

        // left: each wizard's hats (lives) and reveal charge
        for idx in 0..2 {
            if idx == 1 && !g.two_player {
                continue;
            }
            let x0 = if idx == 0 { 14.0 } else { WIN_W as f32 - 162.0 };
            let yy = y0 + 30.0;
            let col = if idx == 0 { WIZ_ROBE } else { WIZ_ROBE2 };
            blip.draw_text(if idx == 0 { "P1" } else { "P2" }, x0, yy - 4.0, 2.0, col);
            for k in 0..g.lives[idx].clamp(0, 6) {
                self.sprite_sized(1 + idx as u32, 5, x0 + 40.0 + k as f32 * 18.0, yy + 3.0, 22.0, false, blip::macroquad::color::WHITE);
            }
            // reveal: a crystal that fills as it recharges
            let cd = g.wizards[idx].reveal_cd;
            let frac = if cd <= 0.0 { 1.0 } else { 1.0 - cd / REVEAL_CD };
            let (ox, oy) = (x0 + 8.0, yy + 34.0);
            blip.draw_text("REVEAL", x0 + 24.0, oy - 6.0, 2.0, if frac >= 1.0 { REVEAL_C } else { BLIP_GRAY });
            draw_circle_lines(ox, oy, 8.0, 1.5, c(0.5, 0.45, 0.35, 1.0));
            self.additive(true);
            self.glow(ox, oy, 7.0 * frac, with_a(REVEAL_C, 0.9));
            if frac >= 1.0 {
                self.glow(ox, oy, 14.0, with_a(REVEAL_C, 0.3 + 0.2 * (t * 4.0).sin()));
            }
            self.additive(false);
        }
        // right: what is left to do here
        if !g.two_player {
            let x0 = WIN_W as f32 - 150.0;
            let yy = y0 + 30.0;
            let monsters = g.enemies.iter().filter(|e| !e.gone).count() as i32
                + if g.warlock.as_ref().is_some_and(|w| w.wards > 0) { 1 } else { 0 };
            let runes = g.runes.iter().filter(|r| !r.cleansed).count() as i32;
            self.sprite_sized(0, 2, x0 + 10.0, yy + 3.0, 26.0, false, blip::macroquad::color::WHITE);
            blip.draw_number(monsters, x0 + 28.0, yy - 4.0, 2.0, BLIP_WHITE);
            self.sprite_sized(3, 4, x0 + 76.0, yy + 3.0, 26.0, false, blip::macroquad::color::WHITE);
            blip.draw_number(runes, x0 + 94.0, yy - 4.0, 2.0, BLIP_WHITE);
            let (msg, col) = if g.whelp.is_some() {
                ("WHELP!", FIREBALL)
            } else if g.double {
                ("DOUBLE", RUNE_CLEAN)
            } else if g.exit_open {
                ("STAIRS OPEN", EXIT_C)
            } else if runes > 0 {
                ("CLEANSE", RUNE_CLEAN)
            } else {
                ("HUNT", IMP)
            };
            blip.draw_text(msg, x0 + 2.0, yy + 28.0, 2.0, col);
        }
    }

    // ---- title -----------------------------------------------------------------------

    fn draw_title(&self, blip: &Blip, _g: &Game, t: f32) {
        let white = blip::macroquad::color::WHITE;
        draw_texture_ex(&self.title, 0.0, 0.0, white, DrawTextureParams { dest_size: Some(vec2(WIN_W as f32, WIN_H as f32)), ..Default::default() });

        // the storm: every few seconds a forked bolt, and the sky flashes
        let period = 5.3;
        let cycle = (t / period).floor();
        let lt = t - cycle * period;
        let strike = lt < 0.32;
        let flash = if lt < 0.08 { 1.0 } else if lt < 0.14 { 0.25 } else if lt < 0.22 { 0.8 } else if lt < 0.32 { 0.3 } else { 0.0 };
        self.additive(true);
        if flash > 0.0 {
            for k in 0..5 {
                self.glow(k as f32 * 70.0, 120.0, 150.0, c(0.45, 0.5, 0.8, 0.16 * flash));
            }
            self.glow(60.0 + hash2(cycle as i32, 1) * 220.0, 120.0, 200.0, c(0.6, 0.65, 0.95, 0.3 * flash));
            let mut x = 30.0 + hash2(cycle as i32, 1) * 240.0;
            let mut y = 80.0;
            let mut k = 0;
            while y < 196.0 {
                let nx = x + (hash2(cycle as i32 * 31 + k, 7) - 0.5) * 34.0;
                let ny = y + 10.0 + hash2(k, cycle as i32 + 3) * 16.0;
                draw_line(x, y, nx, ny, 2.0, c(0.85, 0.9, 1.0, flash));
                self.streak((x + nx) / 2.0, (y + ny) / 2.0, 30.0, 10.0, (ny - y).atan2(nx - x), c(0.5, 0.6, 1.0, 0.4 * flash));
                if k % 3 == 1 {
                    // a fork that dies away
                    let (mut fx, mut fy) = (nx, ny);
                    for j in 0..3 {
                        let gx = fx + (hash2(k * 7 + j, cycle as i32) - 0.3) * 26.0;
                        let gy = fy + 9.0;
                        draw_line(fx, fy, gx, gy, 1.0, c(0.7, 0.8, 1.0, flash * (0.7 - j as f32 * 0.2)));
                        fx = gx;
                        fy = gy;
                    }
                }
                x = nx;
                y = ny;
                k += 1;
            }
        }
        self.additive(false);
        // the lit arch and the figure in it flash with the sky
        if strike {
            self.additive(true);
            self.glow(400.0, 300.0, 200.0, c(0.4, 0.45, 0.7, 0.12 * flash));
            self.additive(false);
        }

        // mist rolling over the maze
        for k in 0..8 {
            let fk = k as f32;
            let x = (fk * 97.0 + t * (8.0 + fk * 2.0)).rem_euclid(WIN_W as f32 + 300.0) - 150.0;
            let y = 250.0 + (fk * 37.0).rem_euclid(260.0) + (t * 0.4 + fk).sin() * 10.0;
            self.glow(x, y, 150.0, c(0.45, 0.5, 0.62, 0.10));
        }

        self.additive(true);
        // the torch on the pillar
        let f = torch_flicker(t, 3);
        let (tx, ty) = (308.0, 244.0);
        self.glow(tx, ty, 90.0 * f, c(1.0, 0.5, 0.18, 0.25));
        self.glow(tx, ty, 20.0 * f, c(1.0, 0.6, 0.25, 0.8));
        self.streak(tx, ty - 6.0, 12.0, 28.0 * f, 0.0, c(1.0, 0.7, 0.3, 0.95));
        self.streak(tx, ty - 4.0, 6.0, 14.0 * f, 0.0, c(1.0, 0.95, 0.75, 1.0));
        for k in 0..10 {
            let life = 2.2;
            let p = ((t + k as f32 * 0.37) / life).fract();
            let ex = tx + (p * 40.0 + k as f32).sin() * 8.0 + p * 10.0;
            let ey = ty - 10.0 - p * 120.0;
            self.glow(ex, ey, 2.2, c(1.0, 0.65, 0.3, 1.0 - p));
        }
        // the Warlock's eyes, burning in the dark of the hood
        let burn = 0.75 + 0.25 * (t * 3.0).sin() + flash * 0.4;
        for (ex, ey) in [(388.0f32, 194.0f32), (406.0, 194.0)] {
            self.glow(ex, ey, 12.0 * burn, c(0.75, 0.3, 1.0, 0.6));
            self.glow(ex, ey, 3.5, c(1.0, 0.85, 1.0, burn.min(1.0)));
        }
        // the inscription round the arch wakes letter by letter, the light running along it
        let strokes = crate::runes::strokes();
        let n = crate::runes::letters() as f32;
        let head = (t * 0.35).fract() * (n + 6.0) - 3.0;
        for &(i, a, b) in &strokes {
            let lag = head - i as f32;
            let lit = 0.3 + 0.7 * (if lag >= 0.0 { (-lag * 0.35).exp() } else { (1.0 + lag).max(0.0) });
            draw_line(a.0, a.1, b.0, b.1, 1.2, c(0.8, 0.5, 1.0, lit));
            self.glow((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0, 8.0 * lit, c(0.6, 0.2, 1.0, 0.3 * lit));
        }

        // his circle on the flagstones: a ring of turning script, breathing
        let (cx, cy, rx, ry) = (250.0f32, 452.0f32, 62.0f32, 15.0f32);
        let breath = 0.65 + 0.35 * (t * 1.7).sin();
        let ring = |r: f32, col: Color, w: f32| {
            let n = 48;
            for k in 0..n {
                let a0 = k as f32 / n as f32 * TAU;
                let a1 = (k + 1) as f32 / n as f32 * TAU;
                draw_line(cx + a0.cos() * rx * r, cy + a0.sin() * ry * r, cx + a1.cos() * rx * r, cy + a1.sin() * ry * r, w, col);
            }
        };
        self.glow(cx, cy, 90.0, c(0.45, 0.1, 0.8, 0.22 * breath));
        for k in 0..10 {
            let a = k as f32 / 10.0 * TAU;
            self.glow(cx + a.cos() * rx * 0.92, cy + a.sin() * ry * 0.92, 18.0, c(0.6, 0.2, 1.0, 0.25 * breath));
        }
        ring(1.0, c(0.75, 0.35, 1.0, 0.85 * breath), 1.5);
        ring(0.84, c(0.6, 0.25, 1.0, 0.6 * breath), 1.0);
        ring(0.63, c(0.8, 0.5, 1.0, 0.7 * breath), 1.0);
        for k in 0..16 {
            let a = k as f32 / 16.0 * TAU + t * 0.35;
            let (gx, gy) = (cx + a.cos() * rx * 0.92, cy + a.sin() * ry * 0.92);
            draw_glyph(gx, gy, k + 2, 2.0, c(0.95, 0.75, 1.0, 0.9 * breath));
        }
        // the spokes of a three-armed sigil inside, turning the other way
        for k in 0..3 {
            let a = k as f32 / 3.0 * TAU - t * 0.5;
            let (x0, y0) = (cx + a.cos() * rx * 0.63, cy + a.sin() * ry * 0.63);
            let (x1, y1) = (cx + (a + 2.1).cos() * rx * 0.2, cy + (a + 2.1).sin() * ry * 0.2);
            draw_line(x0, y0, x1, y1, 1.0, c(0.8, 0.5, 1.0, 0.6 * breath));
        }
        // and the dark it calls up, rising out of it as violet sparks
        for k in 0..18 {
            let life = 3.0;
            let p = ((t + k as f32 * 0.61) / life).fract();
            let a = hash2(k, (t / life + k as f32 * 0.61 / life).floor() as i32) * TAU;
            let x = cx + a.cos() * rx * 0.9 + (p * 9.0 + k as f32).sin() * 4.0;
            let y = cy + a.sin() * ry * 0.9 - p * 90.0;
            self.glow(x, y, 2.5 + (1.0 - p) * 2.0, c(0.75, 0.4, 1.0, (1.0 - p) * 0.9));
        }

        // black candles round the circle, and the stone on his breast
        for (k, &(x, foot, h)) in [(197.0f32, 445.0f32, 9.0f32), (303.0, 445.0, 8.0), (192.0, 460.0, 14.0), (250.0, 468.0, 11.0), (308.0, 460.0, 16.0)].iter().enumerate() {
            let f = torch_flicker(t, k + 5);
            let top = foot - h - 3.0;
            self.glow(x, foot, 26.0 * f, c(1.0, 0.55, 0.25, 0.18));
            self.glow(x, top, 9.0 * f, c(1.0, 0.6, 0.25, 0.7));
            self.streak(x, top - 2.0, 3.0, 8.0 * f, 0.0, c(1.0, 0.8, 0.45, 0.95));
            self.streak(x, top - 1.0, 1.6, 4.0 * f, 0.0, c(1.0, 1.0, 0.85, 1.0));
        }
        let gem = 0.6 + 0.4 * (t * 2.1).sin().abs();
        self.glow(396.0, 304.0, 22.0 * gem, c(0.6, 0.15, 1.0, 0.45));
        self.glow(396.0, 304.0, 4.5, c(0.95, 0.75, 1.0, gem));

        // in his claws, an orb of the void: black at the heart, violet at the rim,
        // the dark drawn into it in spirals
        let (hx, hy) = (192.0f32, 300.0f32);
        let swell = 1.0 + 0.1 * (t * 3.1).sin();
        self.glow(hx, hy, 60.0 * swell, c(0.55, 0.12, 0.9, 0.35));
        self.glow(hx, hy, 24.0 * swell, c(0.8, 0.35, 1.0, 0.7));
        for k in 0..14 {
            let p = (t * 0.7 + k as f32 / 14.0) % 1.0;
            let a = k as f32 * 2.4 + p * 5.0;
            let r = 34.0 * (1.0 - p) + 8.0;
            self.glow(hx + a.cos() * r, hy + a.sin() * r * 0.8, 2.6, c(0.85, 0.55, 1.0, p.min(1.0 - p) * 2.0));
        }
        self.additive(false);
        draw_circle(hx, hy, 8.5 * swell, c(0.02, 0.0, 0.04, 1.0));
        self.additive(true);
        draw_circle_lines(hx, hy, 9.0 * swell, 1.5, c(0.9, 0.6, 1.0, 0.9));
        for k in 0..3 {
            let a = t * 4.0 + k as f32 * TAU / 3.0;
            self.streak(hx + a.cos() * 9.0, hy + a.sin() * 9.0, 10.0, 2.5, a + 1.57, c(1.0, 0.7, 1.0, 0.8));
        }

        // the wizard's staff answers: a crystal of cold blue fire, the gem in
        // his hat burning with it
        let (sx, sy) = (164.0f32, 296.0f32);
        let pulse = 1.0 + 0.15 * (t * 5.0).sin();
        self.glow(sx, sy, 70.0 * pulse, c(0.25, 0.5, 1.0, 0.35));
        self.glow(sx, sy, 18.0 * pulse, c(0.6, 0.85, 1.0, 0.9));
        self.glow(sx, sy, 6.0, c(1.0, 1.0, 1.0, 1.0));
        for k in 0..4 {
            let a = t * 2.2 + k as f32 * TAU / 4.0;
            self.glow(sx + a.cos() * 16.0, sy + a.sin() * 6.0 - 2.0, 2.4, c(0.7, 0.9, 1.0, 0.9));
        }
        self.glow(124.0, 347.0, 7.0 * pulse, c(0.5, 0.8, 1.0, 0.8));

        // the two spells meet: beams pushing at one point that sways between them
        let clash = ((t * 0.45).fract() < 0.55) as i32 as f32;
        if clash > 0.0 {
            let m = 0.5 + 0.18 * (t * 2.3).sin() + 0.06 * (t * 7.0).sin();
            let (mx, my) = (sx + (hx - sx) * m, sy + (hy - sy) * m - 2.0);
            let seed = (t * 30.0) as i32;
            for (fx, fy, ex, ey, col) in [(sx, sy, mx, my, c(0.55, 0.85, 1.0, 0.95)), (hx, hy, mx, my, c(0.9, 0.55, 1.0, 0.95))] {
                self.streak((fx + ex) / 2.0, (fy + ey) / 2.0, ((ex - fx).powi(2) + (ey - fy).powi(2)).sqrt() + 6.0, 7.0, (ey - fy).atan2(ex - fx), with_a(col, 0.75));
                self.streak((fx + ex) / 2.0, (fy + ey) / 2.0, ((ex - fx).powi(2) + (ey - fy).powi(2)).sqrt(), 2.5, (ey - fy).atan2(ex - fx), c(1.0, 1.0, 1.0, 0.9));
                for strand in 0..2 {
                    let (mut x, mut y) = (fx, fy);
                    for k in 1..=4 {
                        let q = k as f32 / 4.0;
                        let nx = fx + (ex - fx) * q + (hash2(seed + strand * 9, k) - 0.5) * 8.0 * (1.0 - q * 0.5);
                        let ny = fy + (ey - fy) * q + (hash2(k, seed + strand * 5) - 0.5) * 10.0;
                        draw_line(x, y, nx, ny, 1.2, col);
                        x = nx;
                        y = ny;
                    }
                }
            }
            self.glow(mx, my, 30.0, c(0.9, 0.8, 1.0, 0.55));
            self.glow(mx, my, 8.0, c(1.0, 1.0, 1.0, 1.0));
            for k in 0..6 {
                let a = hash2(seed, k) * TAU;
                let r = 6.0 + hash2(k, seed) * 14.0;
                self.glow(mx + a.cos() * r, my + a.sin() * r, 1.8, c(1.0, 0.9, 1.0, 0.9));
            }
        }
        self.additive(false);

        // bats out of the tower, across the moon
        for k in 0..4 {
            let period = 9.0 + k as f32 * 1.7;
            let p = ((t + k as f32 * 2.3) / period).fract();
            if p > 0.6 {
                continue;
            }
            let q = p / 0.6;
            let x = 290.0 - q * 330.0 + k as f32 * 18.0;
            let y = 60.0 + k as f32 * 14.0 + (q * 9.0 + k as f32).sin() * 10.0 - q * 20.0;
            let flap = (t * 14.0 + k as f32 * 1.3).sin();
            let s = 5.0 - k as f32 * 0.6;
            let ink = c(0.02, 0.02, 0.04, 1.0);
            draw_line(x, y, x - s, y - s * 0.6 * flap, 1.5, ink);
            draw_line(x - s, y - s * 0.6 * flap, x - s * 1.8, y + s * 0.2, 1.2, ink);
            draw_line(x, y, x + s, y - s * 0.6 * flap, 1.5, ink);
            draw_line(x + s, y - s * 0.6 * flap, x + s * 1.8, y + s * 0.2, 1.2, ink);
            draw_circle(x, y, 1.5, ink);
        }

        // the name, gilded, over the storm
        let ty0 = 18.0;
        self.additive(true);
        let aura = 0.7 + 0.3 * (t * 1.3).sin();
        for k in 0..7 {
            let x = WIN_W as f32 / 2.0 + (k as f32 - 3.0) * 44.0;
            self.glow(x, ty0 + 26.0 + (t * 2.0 + k as f32).sin() * 3.0, 46.0, c(0.5, 0.12, 0.85, 0.32 * aura));
        }
        // witchfire licking up off the letters
        for k in 0..16 {
            let life = 1.4;
            let p = ((t + k as f32 * 0.29) / life).fract();
            let x = WIN_W as f32 / 2.0 - 140.0 + hash2(k, (t / life + k as f32 * 0.29 / life).floor() as i32) * 280.0;
            let y = ty0 + 8.0 - p * 26.0;
            self.glow(x + (p * 7.0 + k as f32).sin() * 3.0, y, 4.0 * (1.0 - p) + 1.0, c(0.7, 0.35, 1.0, (1.0 - p) * 0.7));
        }
        self.additive(false);
        blip.draw_centered_outlined("WARLOCK", WIN_W as f32 / 2.0, ty0 + 3.0, 7.0, c(0.08, 0.02, 0.01, 0.9), c(0.0, 0.0, 0.0, 0.9));
        blip.draw_centered_outlined("WARLOCK", WIN_W as f32 / 2.0, ty0, 7.0, c(0.86, 0.66, 0.3, 1.0), c(0.12, 0.05, 0.02, 1.0));
        draw_rectangle(WIN_W as f32 / 2.0 - 150.0, ty0 + 56.0, 300.0, 2.0, c(1.0, 0.86, 0.5, 0.55));
        self.additive(true);
        self.streak(WIN_W as f32 / 2.0, ty0 + 57.0, 380.0, 14.0, 0.0, c(1.0, 0.6, 0.25, 0.35 + 0.1 * (t * 2.0).sin()));
        self.additive(false);
        blip.draw_hi(web::high_score(), ty0 + 66.0, BLIP_YELLOW);

        // his greeting, on a scrap of parchment
        const SAYS: [(&str, &str); 4] = [
            ("WELCOME TO", "MY CELLARS"),
            ("COME CLOSER,", "LITTLE WIZARD"),
            ("I AM NOT", "A VAMPIRE"),
            ("MIND YOUR", "ROBES"),
        ];
        let say = SAYS[((t / 6.0) as usize) % SAYS.len()];
        let show = ((t / 6.0).fract() * 6.0).clamp(0.0, 6.0);
        let a = (show * 3.0).min(1.0).min((5.4 - show).max(0.0) * 3.0);
        if a > 0.0 {
            let (bx, by, bw, bh) = (118.0, 150.0, 164.0, 44.0);
            draw_rectangle(bx, by, bw, bh, c(0.86, 0.8, 0.66, 0.92 * a));
            draw_rectangle(bx, by, bw, 2.0, c(0.62, 0.5, 0.32, a));
            draw_rectangle(bx, by + bh - 2.0, bw, 2.0, c(0.62, 0.5, 0.32, a));
            blip::macroquad::shapes::draw_triangle(vec2(bx + bw, by + 12.0), vec2(bx + bw, by + 30.0), vec2(bx + bw + 44.0, by + 30.0), c(0.86, 0.8, 0.66, 0.92 * a));
            let ink = c(0.22, 0.08, 0.06, a);
            blip.draw_centered_outlined(say.0, bx + bw / 2.0, by + 6.0, 2.0, ink, c(0.0, 0.0, 0.0, 0.0));
            blip.draw_centered_outlined(say.1, bx + bw / 2.0, by + 24.0, 2.0, ink, c(0.0, 0.0, 0.0, 0.0));
        }

        // the prompts, on a band of shadow along the bottom
        for k in 0..12 {
            draw_rectangle(0.0, 452.0 + k as f32 * 8.0, WIN_W as f32, 8.0, c(0.0, 0.0, 0.01, 0.06 * k as f32));
        }
        let by = web::controls();
        if (t * 2.0) as i32 % 2 == 0 {
            blip.draw_start_prompt(470.0);
        }
        blip.draw_centered_outlined(
            by.pick("ARROWS MOVE  SPACE BOLT  Z REVEAL", "STICK MOVE  A BOLT  B REVEAL", "DRAG TO MOVE  TAP BOLT OR REVEAL"),
            WIN_W as f32 / 2.0,
            500.0,
            2.0,
            c(0.78, 0.76, 0.72, 1.0),
            BLIP_BLACK,
        );
        const TIPS: [&str; 8] = [
            "J JOINS A SECOND WIZARD",
            "REVEAL BESIDE A RUNE TO CLEANSE IT",
            "THE SCRYING GLASS SEES WRAITHS",
            "STRIKE THE WHELP FOR DOUBLE",
            "GARGOYLES GLOW BEFORE THEY BREATHE",
            "REVEAL CALLS THE HOUNDS",
            "REVEAL THEN BOLT: A LANCE",
            "MONSTERS MAY DROP POTIONS",
        ];
        // A new tip every four seconds; the first says how the second player joins.
        let tip = match (t / 4.0) as usize % TIPS.len() {
            0 => by.pick(TIPS[0], "2ND STATION JOINS WITH A", "2ND PLAYER TAPS 2P JOIN"),
            n => TIPS[n],
        };
        let fade = ((t / 4.0).fract() * 8.0).min(1.0).min((1.0 - (t / 4.0).fract()) * 8.0);
        blip.draw_centered_outlined(tip, WIN_W as f32 / 2.0, 522.0, 2.0, c(0.86, 0.66, 0.3, fade), with_a(BLIP_BLACK, fade));
    }
}

fn potion_color(k: PotionKind) -> Color {
    match k {
        PotionKind::Haste => POTION_HASTE,
        PotionKind::Gold => POTION_GOLD,
        PotionKind::Life => POTION_LIFE,
    }
}

// ---- helpers shared with the game -------------------------------------------------

use std::f32::consts::TAU;

pub fn hash2(x: i32, y: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263) ^ 0x5bd1_e995;
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0xffff) as f32 / 65535.0
}

/// Where a torch's flame burns: on the face of its wall, above the floor below.
pub fn torch_flame(tc: Cell) -> (f32, f32) {
    let (x, y) = center(tc);
    (x, y + 4.0)
}

pub fn torch_flicker(t: f32, k: usize) -> f32 {
    let p = k as f32 * 1.7;
    0.86 + 0.08 * (t * 9.0 + p).sin() + 0.06 * (t * 23.0 + p * 2.3).sin()
}

pub fn proj_px(p: &Projectile) -> (f32, f32) {
    let a = center(p.from);
    let b = center(p.tile);
    (lerp(a.0, b.0, p.lerp.min(1.0)), lerp(a.1, b.1, p.lerp.min(1.0)))
}

/// The staff crystal's offset from a wizard's centre, for the pose he holds.
pub fn staff_tip(w: &Wizard) -> (f32, f32) {
    let cast = w.cast_t > 0.0;
    match w.facing {
        Dir::Down => if cast { (9.0, -15.0) } else { (8.0, -12.0) },
        Dir::Up => (-8.0, -12.0),
        Dir::Right => if cast { (13.0, -10.0) } else { (8.0, -12.0) },
        Dir::Left => if cast { (-13.0, -10.0) } else { (-8.0, -12.0) },
    }
}

pub fn wraith_alpha(e: &Enemy, t: f32) -> f32 {
    if e.lunging {
        if e.charge > 0.0 {
            0.35 + 0.65 * (1.0 - e.charge / WRAITH_CHARGE)
        } else {
            1.0
        }
    } else if e.revealed > 0.0 {
        0.9
    } else {
        0.55 + 0.25 * (t * 8.0).sin()
    }
}

/// A tiny rune: one of a few stave shapes, so a ring of them reads as script.
fn draw_glyph(x: f32, y: f32, n: usize, s: f32, col: Color) {
    draw_line(x, y - s, x, y + s, 1.0, col);
    match n % 4 {
        0 => draw_line(x, y - s * 0.4, x + s * 0.7, y - s, 1.0, col),
        1 => {
            draw_line(x, y - s * 0.2, x - s * 0.7, y - s * 0.8, 1.0, col);
            draw_line(x, y - s * 0.2, x + s * 0.7, y - s * 0.8, 1.0, col);
        }
        2 => draw_line(x - s * 0.6, y, x + s * 0.6, y - s * 0.6, 1.0, col),
        _ => {
            draw_line(x, y - s, x + s * 0.6, y - s * 0.3, 1.0, col);
            draw_line(x + s * 0.6, y - s * 0.3, x, y + s * 0.3, 1.0, col);
        }
    }
}

/// Is there a clear line from a light at `(x, y)` to the sample at `(px, py)`?
/// A wall stops the light, except the wall the sample sits in (so wall faces light up).
fn clear_line(walls: &Walls, x: f32, y: f32, px: f32, py: f32) -> bool {
    let (dx, dy) = (px - x, py - y);
    let dist = (dx * dx + dy * dy).sqrt();
    let steps = (dist / 5.0) as i32;
    let lc = Cell { c: (x / CELL as f32) as i32, r: ((y - HUD_H as f32) / CELL as f32) as i32 };
    let tc = Cell { c: (px / CELL as f32) as i32, r: ((py - HUD_H as f32) / CELL as f32) as i32 };
    for s in 1..steps {
        let k = s as f32 / steps as f32;
        let qx = x + dx * k;
        let qy = y + dy * k;
        let cl = Cell { c: (qx / CELL as f32).floor() as i32, r: ((qy - HUD_H as f32) / CELL as f32).floor() as i32 };
        if cl != lc && cl != tc && walls.get(cl.c, cl.r) {
            return false;
        }
    }
    true
}

/// The samples a light of radius `r` reaches, with their falloff weights.
fn reach(walls: &Walls, x: f32, y: f32, r: f32) -> Vec<(u32, f32)> {
    let mut out = Vec::new();
    let lx0 = (((x - r) / SUB_PX).floor() as i32).max(0);
    let lx1 = (((x + r) / SUB_PX).ceil() as i32).min(LW as i32 - 1);
    let ly0 = (((y - HUD_H as f32 - r) / SUB_PX).floor() as i32).max(0);
    let ly1 = (((y - HUD_H as f32 + r) / SUB_PX).ceil() as i32).min(LH as i32 - 1);
    for ly in ly0..=ly1 {
        for lx in lx0..=lx1 {
            let px = (lx as f32 + 0.5) * SUB_PX;
            let py = (ly as f32 + 0.5) * SUB_PX + HUD_H as f32;
            let d = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
            if d >= r {
                continue;
            }
            if !clear_line(walls, x, y, px, py) {
                continue;
            }
            let f = 1.0 - d / r;
            out.push(((ly as usize * LW + lx as usize) as u32, f * f));
        }
    }
    out
}

/// A band of light `w` wide around radius `r`, ignoring walls.
fn add_ring(light: &mut [[f32; 3]], x: f32, y: f32, r: f32, w: f32, col: [f32; 3]) {
    for ly in 0..LH {
        for lx in 0..LW {
            let px = (lx as f32 + 0.5) * SUB_PX;
            let py = (ly as f32 + 0.5) * SUB_PX + HUD_H as f32;
            let d = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
            let off = (d - r).abs();
            if off < w && d <= r + w {
                let k = 1.0 - off / w;
                // inside the band the light lingers a little
                let k = if d < r { k.max(0.25) } else { k };
                let l = &mut light[ly * LW + lx];
                l[0] += col[0] * k;
                l[1] += col[1] * k;
                l[2] += col[2] * k;
            }
        }
    }
}

fn add_light(light: &mut [[f32; 3]], walls: &Walls, x: f32, y: f32, r: f32, col: [f32; 3]) {
    for (i, w) in reach(walls, x, y, r) {
        let l = &mut light[i as usize];
        l[0] += col[0] * w;
        l[1] += col[1] * w;
        l[2] += col[2] * w;
    }
}

pub fn chamber_name(g: &Game) -> &'static str {
    if g.state == State::Bonus {
        return "THE SPELL CIRCLE";
    }
    room_name(g.sess.level)
}
