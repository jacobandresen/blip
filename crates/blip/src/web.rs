//! Web integration — thin wrappers around JavaScript calls made via `web/blip_bridge.js`.
//!
//! When compiled for `wasm32`, these functions call into the browser's JS environment
//! to communicate with the kiosk shell.
//! On native builds they all do nothing, so your game logic works identically on the desktop.

#[cfg(target_arch = "wasm32")]
extern "C" {
    fn blip_spend_coin();
    fn blip_set_mode(mode: i32);
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
