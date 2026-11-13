//! Score, lives and level state for one play session.
//! Call `reset` to start a new game.

/// Returned by [`Session::lose_life`] so callers can branch cleanly without
/// inspecting the lives count themselves.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum LifeResult {
    /// The player still has lives remaining — show the death animation and respawn.
    StillAlive,
    /// No lives left — transition to the game-over screen.
    GameOver,
}

/// Per-session score state shared by every game.
pub struct Session {
    pub score: i32,
    pub lives: i32,
    pub level: i32,
}

impl Session {
    /// Create a new session with zero score, `lives` lives, and level 1.
    pub fn new(lives: i32) -> Self {
        Self { score: 0, lives, level: 1 }
    }

    /// Add `pts` to the score.
    pub fn add_score(&mut self, pts: i32) {
        self.score = self.score.saturating_add(pts);
        crate::bot::set("score", self.score as f64);
    }

    /// Decrement lives by one and report whether the game should end.
    pub fn lose_life(&mut self) -> LifeResult {
        self.lives -= 1;
        crate::bot::add("deaths", 1.0);
        if self.lives <= 0 { LifeResult::GameOver } else { LifeResult::StillAlive }
    }

    /// Lose a life and start `timer`: `respawn` seconds to the next life, or [`GAME_OVER_MIN_WAIT`] on the
    /// game-over screen, whose score is then reported.
    pub fn lose_life_after(&mut self, timer: &mut crate::Timer, respawn: f32) -> LifeResult {
        let result = self.lose_life();
        if result == LifeResult::GameOver {
            timer.start(GAME_OVER_MIN_WAIT);
            crate::web::report_score(self.score);
        } else {
            timer.start(respawn);
        }
        result
    }

    /// Advance to the next level (increments the level counter only).
    pub fn next_level(&mut self) {
        self.level += 1;
        crate::bot::reached_level(self.level);
    }

    /// Reset for a fresh game: zero the score, and restore lives and level.
    /// A bot run with `BLIP_BOT_LEVEL=n` starts at level n.
    pub fn reset(&mut self, lives: i32) {
        self.score = 0;
        self.level = 1 + crate::bot::level(i32::MAX as usize) as i32;
        self.lives = lives;
        crate::bot::reached_level(self.level);
    }
}

/// How long GAME OVER stays up before a key can dismiss it: the fire button
/// still held from the play that ended would otherwise skip it unread.
pub const GAME_OVER_MIN_WAIT: f32 = 2.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lost_life_waits_to_respawn_and_the_last_one_waits_on_game_over() {
        let (mut sess, mut timer) = (Session::new(2), crate::Timer::default());
        assert!(sess.lose_life_after(&mut timer, 1.5) == LifeResult::StillAlive);
        assert_eq!(timer.remaining(), 1.5);
        assert!(sess.lose_life_after(&mut timer, 1.5) == LifeResult::GameOver);
        assert_eq!(timer.remaining(), GAME_OVER_MIN_WAIT);
    }
}
