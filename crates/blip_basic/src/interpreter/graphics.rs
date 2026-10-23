//! BASIC drawing arguments translated into the same `Blip` calls as native games.

use super::{
    color::{paper_color, value_color},
    syntax::split_args,
    Program, Value,
};
use blip::{Blip, BlipTex, BLIP_RED, BLIP_WHITE};
use macroquad::color::Color;
use std::collections::HashMap;

impl Program {
    pub(super) fn draw_statement(
        &mut self,
        command: &str,
        blip: &mut Blip,
        textures: &HashMap<String, BlipTex>,
    ) -> Result<(), String> {
        let upper = command.to_ascii_uppercase();
        let rest = command.split_once(char::is_whitespace).map(|x| x.1.trim()).unwrap_or("");
        let args = if rest.is_empty() { Vec::new() } else { split_args(rest) };
        let vals = args.iter().map(|a| self.eval(a, blip)).collect::<Result<Vec<_>, _>>()?;
        let n = |i: usize| -> f32 { vals.get(i).map(Value::number).unwrap_or(0.0) as f32 };
        let c = |i: usize| -> Color { vals.get(i).map(value_color).unwrap_or(BLIP_WHITE) };
        let texture = |i: usize| -> Result<&BlipTex, String> {
            let requested = vals.get(i).map(Value::string).unwrap_or_default();
            // A picture's file: `.PNG` when no other is named.
            let mut name = requested.to_ascii_uppercase();
            if !name.contains('.') {
                name.push_str(".PNG");
            }
            if !crate::host::file_available(&name) {
                return Err(format!("File not available {requested}"));
            }
            textures.get(&name).ok_or_else(|| format!("Unknown texture {requested}"))
        };
        match upper.split_whitespace().next().unwrap_or("") {
            "CANVAS" if vals.len() == 2 => {
                let (width, height) = (n(0), n(1));
                if ![width, height].iter().all(|n| n.is_finite() && (1.0..=4096.0).contains(n) && n.fract() == 0.0) {
                    return Err("CANVAS needs whole-number dimensions from 1 to 4096".into());
                }
                blip.resize_canvas(width as i32, height as i32);
            }
            "CLEAR" if vals.len() == 1 => blip.clear(c(0)),
            "FX_CLEAR" if vals.is_empty() => self.effects.clear(),
            "FX_UPDATE" if vals.len() == 1 => self.effects.update(n(0)),
            "FX_DRAW" if vals.is_empty() => self.effects.draw(blip),
            "FX_BURST" if vals.len() == 5 => {
                self.effects.burst(n(0), n(1), n(2).clamp(0.0, 1024.0) as usize, n(3), c(4))
            }
            "FX_POPUP" if vals.len() == 4 => self.effects.popup(n(0), n(1), &vals[2].string(), c(3)),
            "FX_RING" if vals.len() == 5 => self.effects.ring(n(0), n(1), n(2), n(3), c(4)),
            "DRAW_CIRCLE" if (4..=5).contains(&vals.len()) => {
                let width = if vals.len() == 5 { n(4) } else { 2.0 };
                macroquad::shapes::draw_circle_lines(n(0), n(1), n(2).max(0.0), width, c(3));
            }
            "FILL_RECT" if vals.len() == 5 => blip.fill_rect(n(0), n(1), n(2), n(3), c(4)),
            "DRAW_RECT" if vals.len() == 5 => blip.draw_rect(n(0), n(1), n(2), n(3), c(4)),
            "DRAW_LINE" if vals.len() == 5 => blip.draw_line(n(0), n(1), n(2), n(3), c(4)),
            "CIRCLE" if vals.len() == 3 => {
                let (cx, cy, r) = (n(0), n(1), n(2).max(0.0));
                let color = paper_color(self.pen);
                let mut last = (cx + r, cy);
                for step in 1..=32 {
                    let angle = std::f32::consts::TAU * step as f32 / 32.0;
                    let point = (cx + angle.cos() * r, cy + angle.sin() * r);
                    blip.draw_line(last.0, last.1, point.0, point.1, color);
                    last = point;
                }
            }
            "DRAW_LINE_EX" if vals.len() == 6 => blip.draw_line_ex(n(0), n(1), n(2), n(3), n(4), c(5)),
            "DRAW_POLYLINE" if vals.len() == 6 => {
                let name = vals[0].string().to_ascii_uppercase();
                let row = vals[1].string();
                let count = n(2);
                if !count.is_finite() || !(0.0..=65536.0).contains(&count) || count.fract() != 0.0 {
                    return Err("DRAW_POLYLINE needs a whole-number count from 0 to 65536".into());
                }
                let value = |index, component| {
                    self.arrays.get(&format!("{name}({row},{index},{component})")).map(Value::number).unwrap_or(0.0)
                };
                blip.draw_polyline(
                    (1..=count as u32).map(|i| (value(i, 0) as f32, value(i, 1) as f32, value(i, 2) != 0.0)),
                    n(3),
                    c(4),
                    c(5),
                );
            }
            "DRAW_GLOW_LINE" if vals.len() == 5 => blip.draw_glow_line(n(0), n(1), n(2), n(3), c(4)),
            "FILL_CIRCLE" if vals.len() == 4 => blip.fill_circle(n(0), n(1), n(2), c(3)),
            "FILL_CAPSULE" if vals.len() == 7 => blip.fill_capsule(n(0), n(1), n(2), n(3), n(4), n(5), c(6)),
            "FILL_GLOW_CIRCLE" if vals.len() == 4 => blip.fill_glow_circle(n(0), n(1), n(2), c(3)),
            "FILL_SUNBURST" if vals.len() == 7 => {
                blip.fill_sunburst(n(0), n(1), n(2).max(1.0) as usize, n(3), n(4), [c(5), c(6)])
            }
            "DRAW_HUD" if vals.len() == 2 => blip.draw_hud(n(0) as i32, n(1) as i32),
            // A negative width mirrors the picture, in the box a positive one would fill.
            "DRAW_TEXTURE" if (5..=6).contains(&vals.len()) => {
                let tint = if vals.len() == 6 { c(5) } else { BLIP_WHITE };
                blip.draw_texture_facing(texture(0)?, n(1), n(2), n(3).abs(), n(4), tint, n(3).signum())
            }
            "DRAW_TEXTURE" if vals.len() == 9 => {
                blip.draw_texture_region(texture(0)?, n(5), n(6), n(7), n(8), n(1), n(2), n(3), n(4))
            }
            "DRAW_TEXTURE" if vals.len() == 10 => blip.draw_texture_cell(
                texture(0)?,
                n(5).max(0.0) as u32,
                n(6).max(1.0) as u32,
                n(7).max(0.0) as u32,
                n(8).max(1.0) as u32,
                n(1),
                n(2),
                n(3),
                n(4),
                n(9),
            ),
            "DRAW_CHAR" if vals.len() == 5 => blip.draw_char(
                vals[0].string().to_ascii_uppercase().chars().next().unwrap_or(' '),
                n(1),
                n(2),
                n(3),
                c(4),
            ),
            "DRAW_TEXT" if (5..=7).contains(&vals.len()) => {
                let text = vals[0].string().to_ascii_uppercase();
                let align = vals.get(5).map(Value::string).unwrap_or_else(|| "LEFT".into()).to_ascii_uppercase();
                match (align.as_str(), vals.get(6)) {
                    ("LEFT", None) => blip.draw_text(&text, n(1), n(2), n(3), c(4)),
                    ("LEFT", Some(_)) => blip.draw_text_outlined(&text, n(1), n(2), n(3), c(4), c(6)),
                    ("CENTER", None) => {
                        blip.draw_text(&text, n(1) - text.chars().count() as f32 * 3.0 * n(3), n(2), n(3), c(4))
                    }
                    ("CENTER", Some(_)) => blip.draw_centered_outlined(&text, n(1), n(2), n(3), c(4), c(6)),
                    _ => return Err("DRAW_TEXT alignment must be LEFT or CENTER".into()),
                }
            }
            "DRAW_NUMBER" if vals.len() == 5 => blip.draw_number(n(0) as i32, n(1), n(2), n(3), c(4)),
            op => return Err(format!("Unknown command or bad arguments: {op}")),
        }
        Ok(())
    }
    pub(crate) fn draw_output(&self, blip: &Blip) {
        for (i, line) in self.output.iter().enumerate() {
            blip.draw_text(&line.to_ascii_uppercase(), 8.0, 8.0 + i as f32 * 26.0, 3.0, BLIP_WHITE);
        }
        if let Some(error) = &self.error {
            blip.draw_text(&error.to_ascii_uppercase(), 8.0, 376.0, 2.0, BLIP_RED);
        }
    }
}
