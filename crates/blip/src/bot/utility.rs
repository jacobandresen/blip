//! Utility AI: score every option a bot has, every frame, and take the best. Each score is a few considerations
//! (how near, how urgent, how much it pays), each a number from 0 to 1 shaped by a response curve, multiplied
//! together. A bot written this way weighs everything at once instead of following a fixed order of ifs.
//!
//! ```ignore
//! let target = utility::choose(targets, |r| utility::score(&[
//!     Curve::Inverse.of(norm(r.distance, 0.0, 400.0)),     // near is better
//!     Curve::Logistic { mid: 0.5, steep: 10.0 }.of(norm(r.closing, -50.0, 150.0)), // closing in matters
//! ]));
//! ```

/// How a consideration's input (0 to 1) becomes its weight (0 to 1).
#[derive(Copy, Clone, Debug)]
pub enum Curve {
    /// The input as it is.
    Linear,
    /// One minus the input: less is better.
    Inverse,
    /// The input raised to a power: above 1 cares only near the top, below 1 rises fast and levels off.
    Power(f32),
    /// An S: little below `mid`, much above it, `steep` saying how sharp the turn is.
    Logistic { mid: f32, steep: f32 },
    /// Nothing below the threshold, all of it at and above.
    Step(f32),
}

impl Curve {
    pub fn of(self, x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);
        match self {
            Curve::Linear => x,
            Curve::Inverse => 1.0 - x,
            Curve::Power(k) => x.powf(k),
            Curve::Logistic { mid, steep } => 1.0 / (1.0 + (-(x - mid) * steep).exp()),
            Curve::Step(at) => if x >= at { 1.0 } else { 0.0 },
        }
    }
}

/// `v` placed between `lo` (0) and `hi` (1), clamped.
pub fn norm(v: f32, lo: f32, hi: f32) -> f32 {
    if hi == lo { return 0.0; }
    ((v - lo) / (hi - lo)).clamp(0.0, 1.0)
}

/// The considerations multiplied, made up for their number: a product of many numbers under 1 sinks, so each is
/// raised toward 1 a little the more there are (Dave Mark's compensation). Any zero vetoes the option.
pub fn score(considerations: &[f32]) -> f32 {
    let n = considerations.len() as f32;
    if n == 0.0 { return 0.0; }
    let fix = 1.0 - 1.0 / n;
    considerations.iter().map(|&c| {
        let c = c.clamp(0.0, 1.0);
        c + (1.0 - c) * fix * c
    }).product()
}

/// The option with the highest score; none when every score is zero.
pub fn choose<T>(options: impl IntoIterator<Item = T>, score: impl Fn(&T) -> f32) -> Option<T> {
    options.into_iter().map(|o| { let s = score(&o); (o, s) }).filter(|(_, s)| *s > 0.0)
        .max_by(|a, b| a.1.total_cmp(&b.1)).map(|(o, _)| o)
}

/// A choice kept until another clearly beats it: a bot that changes its mind every frame dithers between two
/// goals and reaches neither. Keep one in [`super::remember`].
#[derive(Clone, Debug)]
pub struct Commit<K> {
    pub current: Option<K>,
}

impl<K> Default for Commit<K> {
    fn default() -> Self { Self { current: None } }
}

impl<K: PartialEq + Copy> Commit<K> {
    /// Of `options` (each a key and its score now), keep the current one while it is still on offer and the best
    /// beats it by no more than `margin` (as a fraction: 0.2 is 20 % better); otherwise take the best.
    pub fn pick(&mut self, options: impl IntoIterator<Item = (K, f32)>, margin: f32) -> Option<K> {
        let options: Vec<(K, f32)> = options.into_iter().filter(|(_, s)| *s > 0.0).collect();
        let best = options.iter().copied().max_by(|a, b| a.1.total_cmp(&b.1));
        let held = self.current.and_then(|k| options.iter().find(|(o, _)| *o == k).copied());
        self.current = match (held, best) {
            (Some((k, s)), Some((_, b))) if b <= s * (1.0 + margin) => Some(k),
            (_, best) => best.map(|(k, _)| k),
        };
        self.current
    }

    pub fn clear(&mut self) { self.current = None; }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curves_shape_a_consideration() {
        assert_eq!(Curve::Linear.of(0.25), 0.25);
        assert_eq!(Curve::Inverse.of(0.25), 0.75);
        assert!(Curve::Power(2.0).of(0.5) < 0.5);
        let s = Curve::Logistic { mid: 0.5, steep: 12.0 };
        assert!(s.of(0.2) < 0.1 && s.of(0.8) > 0.9);
        assert_eq!(Curve::Step(0.5).of(0.49), 0.0);
        assert_eq!(norm(150.0, 100.0, 200.0), 0.5);
    }

    #[test]
    fn a_zero_vetoes_and_many_considerations_do_not_sink_the_score() {
        assert_eq!(score(&[0.9, 0.0, 1.0]), 0.0);
        let few = score(&[0.8, 0.8]);
        let many = score(&[0.8, 0.8, 0.8, 0.8, 0.8, 0.8]);
        assert!(many > 0.8f32.powi(6) * 1.5, "{many} sank");
        assert!((few - many).abs() < 0.35);
    }

    #[test]
    fn the_best_option_wins_and_a_choice_holds_against_a_close_rival() {
        assert_eq!(choose([1, 5, 3], |&x| x as f32), Some(5));
        assert_eq!(choose([1, 2], |_| 0.0), None);
        let mut c = Commit::default();
        assert_eq!(c.pick([("a", 1.0), ("b", 0.9)], 0.2), Some("a"));
        assert_eq!(c.pick([("a", 1.0), ("b", 1.1)], 0.2), Some("a"), "a 10% better rival should not win");
        assert_eq!(c.pick([("a", 1.0), ("b", 1.5)], 0.2), Some("b"));
        assert_eq!(c.pick([("c", 0.1)], 0.2), Some("c"), "a choice no longer on offer is dropped");
    }
}
