//! Countdown timer for delays and cooldowns, and the clock that animations read.

/// Seconds since the game started (on a bot run's own clock): what idle animations and pulses read.
pub fn now() -> f32 {
    seconds() as f32
}

/// On for the first half of each of `rate` cycles a second: a blinking lamp or warning.
pub fn blink(rate: f64) -> bool {
    (seconds() * rate) as i64 % 2 == 0
}

/// A bot run animates by its fixed 60 Hz clock, so its frames repeat exactly.
fn seconds() -> f64 {
    if crate::bot::active() { crate::bot::clock() as f64 } else { macroquad::time::get_time() }
}

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
