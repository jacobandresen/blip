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
}

/// Notify the kiosk shell that the player should be charged a coin.
pub fn spend_coin() {
    #[cfg(target_arch = "wasm32")]
    unsafe { blip_spend_coin(); }
}

/// Notify the kiosk shell which game mode was selected (0 = 1P/CPU, 1 = 2P).
pub fn set_mode(two_player: bool) {
    #[cfg(target_arch = "wasm32")]
    unsafe { blip_set_mode(if two_player { 1 } else { 0 }); }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = two_player;
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
