//! Where moving things will be: a bot plays ahead of the action.

/// Where a point moving freely to `x` ends up between walls at `lo` and
/// `hi` that it bounces off (a ball's path, the walls folded in).
pub fn bounce(x: f32, lo: f32, hi: f32) -> f32 {
    let span = hi - lo;
    if span <= 0.0 { return lo; }
    let folded = (x - lo).rem_euclid(2.0 * span);
    lo + if folded > span { 2.0 * span - folded } else { folded }
}

/// The shortest way from `a` to `b` on a wrapping axis `span` long
/// (a screen whose edges join): negative is backward.
pub fn wrapped(a: f32, b: f32, span: f32) -> f32 {
    let d = (b - a).rem_euclid(span);
    if d > span / 2.0 { d - span } else { d }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ball_folds_back_off_both_walls() {
        assert_eq!(bounce(5.0, 0.0, 10.0), 5.0);
        assert_eq!(bounce(13.0, 0.0, 10.0), 7.0);
        assert_eq!(bounce(-3.0, 0.0, 10.0), 3.0);
        assert_eq!(bounce(23.0, 0.0, 10.0), 3.0);
    }

    #[test]
    fn the_short_way_round_a_wrapping_screen() {
        assert_eq!(wrapped(10.0, 90.0, 100.0), -20.0);
        assert_eq!(wrapped(90.0, 10.0, 100.0), 20.0);
    }
}
