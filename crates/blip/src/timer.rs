//! Countdown timer for delays and cooldowns.

/// Starts inactive; `tick` reports once when an armed timer expires.
#[derive(Copy, Clone, Default)]
pub struct Timer(f32);

impl Timer {
    /// Arm the timer to count down from `secs` seconds.
    pub fn start(&mut self, secs: f32) {
        self.0 = secs;
    }

    /// Advances by `dt` seconds and returns `true` when the timer expires.
    pub fn tick(&mut self, dt: f32) -> bool {
        if self.0 <= 0.0 { return false; }
        self.0 -= dt;
        self.0 <= 0.0
    }

    /// Whether the timer has time remaining.
    pub fn active(&self) -> bool {
        self.0 > 0.0
    }

    /// Remaining seconds, clamped to zero.
    pub fn remaining(&self) -> f32 {
        self.0.max(0.0)
    }
}
