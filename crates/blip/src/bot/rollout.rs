//! Looking ahead: try each move a bot could make, roll the bot and everything moving forward a second or so, and
//! keep the move whose path stays clearest while it still makes progress. This is the sampled-velocity way of
//! avoiding things (velocity obstacles, scored rather than pruned): cheap, and fair to what the bot can see.
//!
//! ```ignore
//! let movers: Vec<Mover> = hazards.iter().map(|b| Mover::moving(b.x, b.y, b.vx, b.vy).sized(3.0, 3.0)).collect();
//! let look = Rollout { me: (10.0, 10.0), ..Rollout::DEFAULT };
//! let best = blip::bot::best([-1, 0, 1], |dx| look.risk(&path_for(dx), &movers) + distance_to_goal(dx));
//! ```

/// Something moving in a straight line: a box `hw` x `hh` either side of (`x`, `y`), from `from` seconds on.
#[derive(Copy, Clone, Debug)]
pub struct Mover {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub hw: f32,
    pub hh: f32,
    /// Seconds from now until it is there (something not launched yet).
    pub from: f32,
    /// How much hitting it matters: 1 a crash, less for a nuisance, negative for something worth touching.
    pub weight: f32,
    /// Seconds from now when it stops where it has got to (something that moves into place and waits).
    pub until: f32,
}

impl Mover {
    pub fn still(x: f32, y: f32) -> Self { Self::moving(x, y, 0.0, 0.0) }
    pub fn moving(x: f32, y: f32, vx: f32, vy: f32) -> Self {
        Self { x, y, vx, vy, hw: 0.0, hh: 0.0, from: 0.0, weight: 1.0, until: f32::INFINITY }
    }
    pub fn sized(self, hw: f32, hh: f32) -> Self { Self { hw, hh, ..self } }
    pub fn weighted(self, weight: f32) -> Self { Self { weight, ..self } }
    pub fn from(self, from: f32) -> Self { Self { from, ..self } }
    /// It stops moving `until` seconds from now.
    pub fn stops(self, until: f32) -> Self { Self { until, ..self } }

    /// Where it is `t` seconds from now, if it is there yet.
    pub fn at(&self, t: f32) -> Option<(f32, f32)> {
        if t < self.from { return None; }
        let t = t.min(self.until) - self.from.min(self.until);
        Some((self.x + self.vx * t, self.y + self.vy * t))
    }
}

/// How a bot weighs a path: its own size, the margin it keeps, and what a hit and a near miss cost.
#[derive(Copy, Clone, Debug)]
pub struct Rollout {
    /// The bot's half width and half height.
    pub me: (f32, f32),
    /// Clearance it wants on every side.
    pub margin: f32,
    /// The time between two points of a path.
    pub dt: f32,
    /// What touching a mover (with the margin) costs, and the soft cost of passing near one, which falls off over
    /// about `halo_reach` beyond the touch.
    pub hit: f32,
    pub halo: f32,
    pub halo_reach: f32,
    /// How much less the end of the path counts than its start: soonest is worst, the far future is uncertain.
    pub fade: f32,
}

impl Rollout {
    pub const DEFAULT: Self = Self { me: (8.0, 8.0), margin: 6.0, dt: 0.1, hit: 10.0, halo: 1.0, halo_reach: 20.0, fade: 0.6 };

    /// The cost of following `path` (the bot's centre every `dt`, starting one step from now) among `movers`.
    pub fn risk(&self, path: &[(f32, f32)], movers: &[Mover]) -> f32 {
        let n = path.len().max(1) as f32;
        let mut cost = 0.0;
        for (k, &(px, py)) in path.iter().enumerate() {
            let t = (k + 1) as f32 * self.dt;
            let soon = 1.0 - self.fade * k as f32 / n;
            for m in movers {
                let Some((x, y)) = m.at(t) else { continue };
                cost += m.weight * soon * self.contact(x - px, y - py, m);
            }
        }
        cost
    }

    /// How much one mover at offset (`ex`, `ey`) from the bot touches it: `hit` inside, a halo outside.
    fn contact(&self, ex: f32, ey: f32, m: &Mover) -> f32 {
        let (rx, ry) = (m.hw + self.me.0 + self.margin, m.hh + self.me.1 + self.margin);
        let (ox, oy) = ((ex.abs() - rx).max(0.0), (ey.abs() - ry).max(0.0));
        if ox == 0.0 && oy == 0.0 { return self.hit; }
        let r = self.halo_reach.max(1.0);
        self.halo * (-(ox * ox + oy * oy) / (r * r)).exp()
    }
}

/// A path along one axis: from `at` toward `to` at up to `speed` per second, `steps` points `dt` apart, while
/// the other axis runs on at `along` per second from `y`. Good enough for a bot that steers left and right while it
/// keeps moving along the other.
pub fn glide(at: f32, to: f32, speed: f32, y: f32, along: f32, dt: f32, steps: usize) -> Vec<(f32, f32)> {
    let mut x = at;
    (1..=steps).map(|k| {
        x += (to - x).clamp(-speed * dt, speed * dt);
        (x, y + along * dt * k as f32)
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bot::best;

    #[test]
    fn a_path_through_a_mover_costs_a_hit_and_one_beside_it_only_a_little() {
        let look = Rollout { me: (5.0, 5.0), margin: 2.0, ..Rollout::DEFAULT };
        let wall = [Mover::still(0.0, 10.0).sized(5.0, 5.0)];
        let into = glide(0.0, 0.0, 100.0, 0.0, 100.0, 0.1, 3);
        let past = glide(40.0, 40.0, 100.0, 0.0, 100.0, 0.1, 3);
        assert!(look.risk(&into, &wall) >= look.hit);
        assert!(look.risk(&past, &wall) < 0.5);
    }

    #[test]
    fn a_mover_not_yet_launched_is_not_there_and_one_that_stops_stays_put() {
        let late = Mover::moving(0.0, 0.0, 10.0, 0.0).from(0.5);
        assert_eq!(late.at(0.2), None);
        assert_eq!(late.at(1.5), Some((10.0, 0.0)));
        let stopping = Mover::moving(0.0, 0.0, 10.0, 0.0).stops(2.0);
        assert_eq!(stopping.at(5.0), Some((20.0, 0.0)));
    }

    #[test]
    fn the_best_move_steps_out_of_the_way() {
        let look = Rollout { me: (5.0, 5.0), ..Rollout::DEFAULT };
        // a hazard coming straight at the middle
        let hazard = [Mover::moving(0.0, 60.0, 0.0, -100.0).sized(8.0, 12.0)];
        let pick = best([-30.0, 0.0, 30.0], |x| look.risk(&glide(0.0, x, 150.0, 0.0, 0.0, 0.1, 10), &hazard) + x.abs() * 0.001);
        assert_ne!(pick, Some(0.0));
    }
}
