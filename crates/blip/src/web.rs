//! Web integration — thin wrappers around JavaScript calls made via `web/blip_bridge.js`.
//!
//! When compiled for `wasm32`, these functions call into the browser's JS environment
//! to communicate with the kiosk shell.
//! On native builds they all do nothing, so your game logic works identically on the desktop.

#[cfg(target_arch = "wasm32")]
extern "C" {
    fn blip_spend_coin();
    fn blip_set_mode(mode: i32);
    fn blip_paddles(left: f32, right: f32);
    fn blip_game_over(score: i32);
    fn blip_high_score() -> i32;
    fn blip_high_name(ptr: *mut u8, cap: i32) -> i32;
    fn blip_touch_down(slot: i32) -> i32;
    fn blip_touch_pos(slot: i32, axis: i32) -> f32;
    fn blip_haptic();
    fn blip_picture(width: i32, height: i32);
}

/// Notify the kiosk shell that the player should be charged a coin.
pub fn spend_coin() {
    #[cfg(target_arch = "wasm32")]
    unsafe { blip_spend_coin(); }
}

/// Notify the kiosk shell which game mode was selected (0 = 1P/CPU, 1 = 2P).
pub fn set_mode(two_player: bool) {
    set_players(i32::from(two_player));
}

/// Tell the shell how many players are in: `0` one, `1` two, `2` the title
/// screen of a two-seat machine, with station two lit and waiting to be
/// touched. In a one-player game station two is dead to the touch (its keys
/// may be player one's), which would make touch-to-join impossible without
/// the third state.
pub fn set_players(code: i32) {
    #[cfg(target_arch = "wasm32")]
    unsafe { blip_set_mode(code); }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = code;
}

/// Report the two paddle positions (each 0.0 = top … 1.0 = bottom of travel)
/// so the shell can spin the on-screen paddle dials to match — including the
/// CPU's paddle in 1-player mode. Rally-only; a no-op everywhere else.
pub fn paddles(left: f32, right: f32) {
    #[cfg(target_arch = "wasm32")]
    unsafe { blip_paddles(left, right); }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (left, right);
}

/// A finger (or a PC's pointer) on the shell's touch surface, as a fraction
/// of the canvas (0..1 across and down; beyond when it is on the strip below
/// the picture), and whether it is pressed: a finger always is, a mouse
/// hovering over the trackpad is not. Slot 0 is player one, slot 1 player
/// two. Natively slot 0 is the mouse while its left button is held.
/// Games want [`crate::Blip::touch`], which maps this into game pixels.
pub fn touch_fraction(slot: usize) -> Option<(f32, f32, bool)> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        // 0 = nothing, 1 = hovering, 2 = pressed
        let state = blip_touch_down(slot as i32);
        if state == 0 { return None; }
        Some((blip_touch_pos(slot as i32, 0), blip_touch_pos(slot as i32, 1), state == 2))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use macroquad::input::{is_mouse_button_down, mouse_position, MouseButton};
        use macroquad::window::{screen_height, screen_width};
        if slot != 0 || !is_mouse_button_down(MouseButton::Left) { return None; }
        let (x, y) = mouse_position();
        Some((x / screen_width(), y / screen_height(), true))
    }
}

/// A short buzz under the player's thumb (a paddle hit, a lost life). Only
/// while the touch controls are in use, and only where the Vibration API is.
pub fn haptic() {
    #[cfg(target_arch = "wasm32")]
    unsafe { blip_haptic(); }
}

/// Tell the shell the game's virtual canvas size, so it can size things to
/// the letterboxed picture (the touch strip is never wider than it).
pub fn picture(width: i32, height: i32) {
    #[cfg(target_arch = "wasm32")]
    unsafe { blip_picture(width, height); }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (width, height);
}

/// Report the final score to the shell for the high-score board
/// (`web/blip_scores.js`). Call once on entering game over; a no-op natively
/// and without a backend.
pub fn report_score(score: i32) {
    #[cfg(target_arch = "wasm32")]
    unsafe { blip_game_over(score); }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = score;
}

/// The leading score for the current game and who holds it, as tracked by
/// `web/blip_scores.js` (the cached leaderboard #1, falling back to this
/// browser's local best + claimed handle).
#[derive(Clone, Default)]
pub struct HighScore {
    /// The top score. 0 when nothing is known — no backend configured, no
    /// local best yet, and always on native builds. Gate any "HI" display
    /// on `score > 0`.
    pub score: i32,
    /// The record holder's handle, uppercased for the bitmap font. Empty
    /// when unknown (e.g. a local best set before a handle was claimed).
    pub name: String,
}

impl HighScore {
    /// A one-line label for a title / game-over screen: `"HI 180940 JACOB"`,
    /// or `"HI 180940"` when the holder is unknown. `prefix` is typically
    /// `"HI"` or `"BEST"`.
    pub fn label(&self, prefix: &str) -> String {
        if self.name.is_empty() {
            format!("{prefix} {}", self.score)
        } else {
            format!("{prefix} {} {}", self.score, self.name)
        }
    }
}

/// Fetch the current [`HighScore`] from the kiosk shell. Cheap enough to
/// call each frame on a title / game-over screen; returns an all-zero
/// value on native builds.
pub fn high_score() -> HighScore {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let mut buf = [0u8; 48];
        let n = blip_high_name(buf.as_mut_ptr(), buf.len() as i32);
        let name = if n > 0 {
            core::str::from_utf8(&buf[..n as usize]).unwrap_or("").to_string()
        } else {
            String::new()
        };
        HighScore { score: blip_high_score(), name }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Dev / screenshot hook: `BLIP_HI=180940` or `BLIP_HI=180940:JACOB`
        // fakes a record so the HUD's centre field can be checked natively.
        match std::env::var("BLIP_HI") {
            Ok(v) if !v.is_empty() => {
                let (num, name) = v.split_once(':').unwrap_or((&v, ""));
                HighScore {
                    score: num.trim().parse().unwrap_or(0),
                    name: name.trim().to_uppercase(),
                }
            }
            _ => HighScore::default(),
        }
    }
}

