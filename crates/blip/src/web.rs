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

/// Report the final score to the kiosk shell when a game ends, so it can
/// post to the shared high-score board (`web/blip_scores.js`). Call once on
/// the transition into the game-over state. A no-op on native builds and
/// when no leaderboard backend is configured.
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
