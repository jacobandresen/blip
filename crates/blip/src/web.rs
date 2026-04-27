//! Web FFI: spend a coin via the JS-side `window.blipSpendCoin` hook.
//!
//! On wasm, this calls a JS import provided by `web/blip_bridge.js`
//! (registered as a miniquad plugin before the wasm module loads).
//! On native, it is a no-op.

#[cfg(target_arch = "wasm32")]
extern "C" {
    fn blip_spend_coin();
}

/// Notify the kiosk shell that the player should be charged a coin.
/// Typically called once on game-over before a fresh credit is consumed.
pub fn spend_coin() {
    #[cfg(target_arch = "wasm32")]
    unsafe { blip_spend_coin(); }
}
