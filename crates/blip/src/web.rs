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
    fn blip_net_role() -> i32;
    fn blip_net_send(ptr: *const u8, len: i32);
    fn blip_net_poll(ptr: *mut u8, cap: i32) -> i32;
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

/// Generic two-device-multiplayer transport (see `docs/multiplayer.md`).
/// Deliberately game-agnostic here — a game defines its own wire format
/// (Rally's is `crates/rally/src/net.rs`) and just hands this raw bytes.
/// The signaling, `RTCPeerConnection` and `RTCDataChannel` all live in
/// `web/blip_net.js`; Rust never sees any of that, only a byte pipe.
pub mod net {
    /// Which role, if any, the pairing UI wants this instance to start as
    /// once the title screen sees it: `0` = no request yet (keep waiting /
    /// stay on the normal local menu), `1` = host, `2` = guest. A game
    /// polls this once per Title-state frame; there's no "take"/clear
    /// step needed because a game only polls while still on its title
    /// screen, and never returns to it mid-match (see `docs/multiplayer.md`
    /// Phase 5 for the rematch case this doesn't yet cover).
    pub fn role() -> i32 {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            super::blip_net_role()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Dev hook for native testing: BLIP_NET_ROLE=1 (host) or 2 (guest).
            std::env::var("BLIP_NET_ROLE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0)
        }
    }

    /// Send a blob of bytes out over the active connection (host -> guest
    /// state, in Rally's case). A no-op if nothing is connected — the JS
    /// side is responsible for dropping bytes it has nowhere to send.
    pub fn send(bytes: &[u8]) {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            super::blip_net_send(bytes.as_ptr(), bytes.len() as i32);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = bytes;
        }
    }

    /// Copy the most recent inbound packet (if any arrived since the last
    /// poll) into `buf`, returning the byte count — `0` if nothing new.
    /// Call once per frame; a game that misses a poll just sees the next
    /// one, there's no queue to fall behind on (only the latest packet is
    /// ever kept — see `web/blip_net.js`).
    pub fn poll(buf: &mut [u8]) -> usize {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            super::blip_net_poll(buf.as_mut_ptr(), buf.len() as i32).max(0) as usize
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = buf;
            0
        }
    }
}
