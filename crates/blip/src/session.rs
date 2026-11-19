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

/// A run of successes in a row (hits without a miss, quick meals) and the score multiple it earns: one more every
/// `per` successes, up to `max`. Rewarding a run is what makes accuracy pay more than luck.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Streak {
    pub count: i32,
    per: i32,
    max: i32,
    heard: i32,
    lost: i32,
}

impl Streak {
    pub const fn new(per: i32, max: i32) -> Self { Self { count: 0, per, max, heard: 1, lost: 0 } }

    /// One more success; the multiple it now earns.
    pub fn hit(&mut self) -> i32 {
        self.count += 1;
        self.multiple()
    }

    /// The run is broken.
    pub fn miss(&mut self) {
        if self.count > 0 { self.lost = self.count; }
        self.count = 0;
    }

    /// The length of a run that broke since the last call, to show it lost; call once a frame.
    pub fn broken(&mut self) -> Option<i32> {
        let n = std::mem::take(&mut self.lost);
        (n > 0).then_some(n)
    }

    pub fn multiple(&self) -> i32 { (1 + (self.count - 1).max(0) / self.per).clamp(1, self.max) }

    /// The new multiple when it has risen since the last call, for a chime; call once a frame.
    pub fn risen(&mut self) -> Option<i32> {
        let m = self.multiple();
        let rose = m > self.heard;
        self.heard = m;
        rose.then_some(m)
    }

    /// Successes still needed for the next multiple; none at the top.
    pub fn to_next(&self) -> Option<i32> {
        let m = self.multiple();
        (m < self.max).then(|| m * self.per + 1 - self.count)
    }
}

/// The arcade continue: a countdown in which a press plays on (spending a coin), or the game is over. A press counts
/// only once the offer has been up `CONTINUE_GRACE`, so a button still held from play does not take it.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Continue {
    left: f32,
    shown: f32,
    ticked: bool,
}

/// Seconds a continue is offered before a press counts.
pub const CONTINUE_GRACE: f32 = 0.8;

impl Continue {
    /// Offer a continue for `secs`. A bot run not set to take one (`bot::may_continue`) ends here, at game over, as
    /// a run did before continues: random play would take every offer, and a run would wait out the count.
    pub fn start(&mut self, secs: f32) {
        *self = Self { left: secs, shown: 0.0, ticked: false };
        if crate::bot::active() && !crate::bot::may_continue() { crate::bot::finish("over"); }
    }

    /// One frame of the offer: `Some(true)` to play on (the coin is spent), `Some(false)` once the time is up,
    /// `None` while it counts down.
    pub fn update(&mut self, dt: f32, pressed: bool) -> Option<bool> {
        self.shown += dt;
        self.ticked = false;
        if pressed && self.shown >= CONTINUE_GRACE {
            crate::web::spend_coin();
            return Some(true);
        }
        let before = self.seconds();
        self.left -= dt;
        self.ticked = self.showing() && self.seconds() < before;
        (self.left <= 0.0).then_some(false)
    }

    /// The offer is on screen: past the grace in which a held button does not count, and not yet run out.
    pub fn showing(&self) -> bool { self.left > 0.0 && self.shown >= CONTINUE_GRACE }

    /// The count on screen went down a second in the last `update`: a tick to hear.
    pub fn ticked(&self) -> bool { self.ticked }

    /// Whole seconds left, for the count on screen.
    pub fn seconds(&self) -> i32 { self.left.ceil().max(0.0) as i32 }

    /// Seconds left, for a count that animates within each second.
    pub fn remaining(&self) -> f32 { self.left.max(0.0) }
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
    fn a_continue_ignores_a_held_button_then_plays_on_or_runs_out() {
        let mut c = Continue::default();
        c.start(2.0);
        assert_eq!(c.update(0.5, true), None, "a press in the first moments is a button still held");
        assert_eq!(c.seconds(), 2);
        assert_eq!(c.update(0.5, true), Some(true));
        c.start(1.0);
        assert_eq!(c.update(0.6, false), None);
        assert_eq!(c.update(0.6, false), Some(false));
    }

    #[test]
    fn a_continue_ticks_once_a_second_while_showing() {
        let mut c = Continue::default();
        c.start(3.0);
        assert!(!c.showing());
        let mut ticks = 0;
        while c.update(0.1, false).is_none() { ticks += c.ticked() as i32; }
        // 3 -> 2 at 1.0 s and 2 -> 1 at 2.0 s; 1 -> 0 is the end, not a tick.
        assert_eq!(ticks, 2);
        assert!(!c.showing());
    }

    #[test]
    fn a_streak_earns_one_more_multiple_every_per_hits_up_to_its_cap() {
        let mut s = Streak::new(3, 3);
        assert_eq!(s.multiple(), 1);
        assert_eq!([s.hit(), s.hit(), s.hit(), s.hit()], [1, 1, 1, 2]);
        assert_eq!(s.to_next(), Some(3));
        for _ in 0..10 { s.hit(); }
        assert_eq!((s.multiple(), s.to_next()), (3, None));
        s.miss();
        assert_eq!(s.multiple(), 1);
    }

    #[test]
    fn a_risen_multiple_is_heard_once() {
        let mut s = Streak::new(2, 3);
        s.hit(); s.hit();
        assert_eq!(s.risen(), None);
        s.hit();
        assert_eq!((s.risen(), s.risen()), (Some(2), None));
        s.miss();
        assert_eq!(s.risen(), None);
        for _ in 0..3 { s.hit(); }
        assert_eq!(s.risen(), Some(2), "a new run's rise is heard again");
        s.miss();
        s.miss();
        assert_eq!((s.broken(), s.broken()), (Some(3), None), "a broken run is told once, however often missed");
    }

    #[test]
    fn a_lost_life_waits_to_respawn_and_the_last_one_waits_on_game_over() {
        let (mut sess, mut timer) = (Session::new(2), crate::Timer::default());
        assert!(sess.lose_life_after(&mut timer, 1.5) == LifeResult::StillAlive);
        assert_eq!(timer.remaining(), 1.5);
        assert!(sess.lose_life_after(&mut timer, 1.5) == LifeResult::GameOver);
        assert_eq!(timer.remaining(), GAME_OVER_MIN_WAIT);
    }
}
