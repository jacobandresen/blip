//! Timer — a simple countdown for delays and cooldowns.
//!
//! The typical pattern is:
//! ```ignore
//! // Arm it when something happens:
//! g.dead_timer.start(1.5);
//! g.state = State::Dead;
//!
//! // In the update function, tick it each frame:
//! if g.dead_timer.tick(dt) {
//!     g.state = State::Play;   // runs exactly once, the frame the timer expires
//! }
//! ```

/// A countdown timer. Starts inactive (zero). Call [`Timer::start`] to arm it,
/// [`Timer::tick`] each frame to advance it.
#[derive(Copy, Clone, Default)]
pub struct Timer(f32);

impl Timer {
    /// Arm the timer to count down from `secs` seconds.
    pub fn start(&mut self, secs: f32) {
        self.0 = secs;
    }

    /// Advance the timer by `dt` seconds. Returns `true` the single frame the
    /// timer crosses zero — useful as a one-shot transition trigger. Returns
    /// `false` if the timer was already inactive.
    pub fn tick(&mut self, dt: f32) -> bool {
        if self.0 <= 0.0 { return false; }
        self.0 -= dt;
        self.0 <= 0.0
    }

    /// `true` while the timer has time remaining (i.e. it has been started and not yet expired).
    pub fn active(&self) -> bool {
        self.0 > 0.0
    }

    /// Remaining seconds, clamped to zero so it never goes negative.
    pub fn remaining(&self) -> f32 {
        self.0.max(0.0)
    }
}
