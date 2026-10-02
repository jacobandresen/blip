//! Drawing. Fighters are posed from the same numbers the rules use: the reach
//! and height a punch is judged by are the reach and height it is drawn at.

use super::*;

mod stage;
mod parts;
mod pose;
mod body;
mod hud;
mod screens;
#[cfg(feature = "gallery")]
mod gallery;

pub(crate) use stage::*;
pub(crate) use parts::*;
pub(crate) use pose::*;
pub(crate) use body::*;
pub(crate) use hud::*;
pub(crate) use screens::*;
#[cfg(feature = "gallery")]
pub(crate) use gallery::*;
use blip::{blend, scatter, shade};

/// An opaque colour from a roster triple.
pub(crate) fn rgb(c: (f32, f32, f32)) -> BlipColor { BlipColor { r: c.0, g: c.1, b: c.2, a: 1.0 } }

pub(crate) use blip::font::text_width as text_w;
/// A roster triple at alpha `a`.
pub(crate) fn rgba(c: (f32, f32, f32), a: f32) -> BlipColor { BlipColor { r: c.0, g: c.1, b: c.2, a } }

/// Draw the frame: whichever screen the game is on, and the fade over it.
pub(crate) fn draw(blip: &Blip, g: &Game) {
    // BLIP_SELECT=n shows the select screen, cursor on fighter n, instead of
    // the pose sheet.
    #[cfg(feature = "gallery")]
    {
        match std::env::var("BLIP_SELECT").ok().and_then(|v| v.parse::<usize>().ok()) {
            Some(pick) => draw_select(blip, &Game { pick: pick % FIGHTERS.len(), now: g.now, ..Game::new() }),
            None => draw_gallery(blip, g.now),
        }
        return;
    }
    #[allow(unreachable_code)]
    if g.poster { return draw_poster(blip, g); }
    draw_screen(blip, g);
    if g.demo {
        stamp(blip, "DEMO", 64.0, 2.0, PALE);
        if (g.now * 1.6) as i32 % 2 == 0 { stamp(blip, "PRESS PUNCH TO PLAY", 360.0, 3.0, BLIP_YELLOW); }
    }
    if g.fade > 0.0 {
        blip.fill_rect(0.0, 0.0, WIN_W as f32, WIN_H as f32, BlipColor { a: (g.fade / FADE).min(1.0), ..BLIP_BLACK });
    }
}

/// The screen for the current state.
pub(crate) fn draw_screen(blip: &Blip, g: &Game) {
    match g.state {
        State::Title => draw_title(blip, g),
        State::Select => draw_select(blip, g),
        State::Vs => draw_vs(blip, g),
        State::Won => draw_ending(blip, g),
        State::Bonus => draw_bonus(blip, g),
        _ => {
            draw_stage(blip, g);
            draw_fight(blip, g);
            draw_hud(blip, g);
            draw_banner(blip, g);
        }
    }
}
