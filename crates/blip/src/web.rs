//! Wrappers for the browser bridge in `web/game.js`.
//! They call JavaScript on wasm and do nothing in native builds.

#[cfg(target_arch = "wasm32")]
extern "C" {
    fn blip_spend_coin();
    fn blip_set_mode(mode: i32);
    fn blip_paddles(left: f32, right: f32);
    fn blip_game_over(score: i32);
    fn blip_high_score() -> i32;
    fn blip_touch_down(slot: i32) -> i32;
    fn blip_touch_pos(slot: i32, axis: i32) -> f32;
    fn blip_haptic();
    fn blip_controls() -> i32;
    fn blip_picture(width: i32, height: i32);
    fn blip_crt() -> i32;
    fn blip_glitch() -> i32;
}

/// Whether the picture gets the CRT treatment: curved glass, bloom, scanlines
/// and noise. A game opened as an app (the home screen, its own icon)
/// asks for a clean, modern picture instead; natively `BLIP_CRT=0` does.
pub fn crt() -> bool {
    #[cfg(target_arch = "wasm32")]
    unsafe { blip_crt() != 0 }
    #[cfg(not(target_arch = "wasm32"))]
    { std::env::var_os("BLIP_CRT").is_none_or(|v| v != "0") }
}

/// Whether occasional picture faults should interrupt the CRT image.
pub fn glitches() -> bool {
    #[cfg(target_arch = "wasm32")]
    unsafe { blip_glitch() != 0 }
    #[cfg(not(target_arch = "wasm32"))]
    { std::env::var_os("BLIP_GLITCH").is_none_or(|v| v != "0") }
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

/// Set players: `0` one, `1` two, `2` title screen with the second station waiting to join.
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

/// Pointer position normalized to the canvas, pressed state and player slot (0 or 1).
/// Native slot 0 reports the mouse; use [`crate::Blip::touch`] for game pixels.
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

/// What the player is holding, so a title can name the right thing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Controls {
    /// A keyboard (always, natively; `BLIP_CONTROLS=pad|touch` overrides).
    Keys,
    /// The deck's pad or stick on a touch screen.
    Pad,
    /// The touch surface: swipes, drags and taps on the picture.
    Touch,
}

impl Controls {
    /// The wording for whichever it is.
    pub fn pick<'a>(self, keys: &'a str, pad: &'a str, touch: &'a str) -> &'a str {
        match self { Controls::Keys => keys, Controls::Pad => pad, Controls::Touch => touch }
    }
}

/// The prompt to press on, for what the player holds: `fire_prompt("TO START")`
/// is PRESS SPACE TO START on a keyboard, PRESS FIRE on a deck, TAP TO START on a touch screen.
pub fn fire_prompt(then: &str) -> String {
    let press = controls().pick("PRESS SPACE", "PRESS FIRE", "TAP");
    if then.is_empty() { press.to_string() } else { format!("{press} {then}") }
}

/// What the player holds: the page's deck or touch surface on the web, `BLIP_CONTROLS` natively.
pub fn controls() -> Controls {
    #[cfg(target_arch = "wasm32")]
    let code = unsafe { blip_controls() };
    #[cfg(not(target_arch = "wasm32"))]
    let code = match std::env::var("BLIP_CONTROLS").as_deref() { Ok("pad") => 1, Ok("touch") => 2, _ => 0 };
    match code { 1 => Controls::Pad, 2 => Controls::Touch, _ => Controls::Keys }
}

/// Tell the shell the game's virtual canvas size, so it can size things to
/// the letterboxed picture (the touch strip is never wider than it).
pub fn picture(width: i32, height: i32) {
    #[cfg(target_arch = "wasm32")]
    unsafe { blip_picture(width, height); }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (width, height);
}

/// Report the final score to the shell, which keeps this browser's best.
/// Call once on entering game over; natively it ends a bot run.
pub fn report_score(score: i32) {
    #[cfg(target_arch = "wasm32")]
    unsafe { blip_game_over(score); }
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::bot::set("score", score as f64);
        crate::bot::finish("over");
    }
}

/// This browser's best score for the current game, kept by the shell. 0 when
/// none is known, which is always the case natively unless `BLIP_HI=180940`
/// fakes one so the HUD's centre field can be checked.
pub fn high_score() -> i32 {
    #[cfg(target_arch = "wasm32")]
    unsafe { blip_high_score() }
    #[cfg(not(target_arch = "wasm32"))]
    std::env::var("BLIP_HI").ok().and_then(|v| v.trim().parse().ok()).unwrap_or(0)
}
