//! Web FFI: JS callbacks via `web/blip_bridge.js` plugin.
//! On native all functions are no-ops.

pub const GAME_BOUNCER:          i32 = 0;
pub const GAME_SERPENT:          i32 = 1;
pub const GAME_GALACTIC_DEFENDER: i32 = 2;
pub const GAME_CANARIS:          i32 = 3;

#[cfg(target_arch = "wasm32")]
extern "C" {
    fn blip_spend_coin();
    fn blip_set_mode(mode: i32);
    fn blip_load_hi_score(game_id: i32) -> i32;
    fn blip_save_hi_score(game_id: i32, score: i32);
    fn blip_game_over(game_id: i32, score: i32);
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
}

/// Return the cached global hi_score for this game (loaded from Supabase on page load).
pub fn load_hi_score(game_id: i32) -> i32 {
    #[cfg(target_arch = "wasm32")]
    { unsafe { blip_load_hi_score(game_id) } }
    #[cfg(not(target_arch = "wasm32"))]
    { let _ = game_id; 0 }
}

/// Persist a new hi_score for this game to Supabase (fire-and-forget).
pub fn save_hi_score(game_id: i32, score: i32) {
    #[cfg(target_arch = "wasm32")]
    unsafe { blip_save_hi_score(game_id, score); }
    #[cfg(not(target_arch = "wasm32"))]
    { let _ = (game_id, score); }
}

/// Notify the shell that a game has ended with the given score.
/// The shell checks whether the score qualifies for the top-10 leaderboard.
pub fn game_over(game_id: i32, score: i32) {
    #[cfg(target_arch = "wasm32")]
    unsafe { blip_game_over(game_id, score); }
    #[cfg(not(target_arch = "wasm32"))]
    { let _ = (game_id, score); }
}
