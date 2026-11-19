//! Where moving things will be: a bot plays ahead of the action.

/// Where a point moving freely to `x` ends up between walls at `lo` and
/// `hi` that it bounces off (its path with the walls folded in).
pub fn bounce(x: f32, lo: f32, hi: f32) -> f32 {
    let span = hi - lo;
    if span <= 0.0 { return lo; }
    let folded = (x - lo).rem_euclid(2.0 * span);
    lo + if folded > span { 2.0 * span - folded } else { folded }
}

/// Where a point at `across` (moving `v_across`, bouncing between `walls`) is when it reaches `line` on the other
/// axis, from `along` moving `v_along`; and in how many seconds (0 once past it).
pub fn landing(across: f32, v_across: f32, along: f32, v_along: f32, line: f32, walls: (f32, f32)) -> (f32, f32) {
    let t = ((line - along) / v_along).max(0.0);
    (bounce(across + v_across * t, walls.0, walls.1), t)
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
    fn a_path_folds_back_off_both_walls() {
        assert_eq!(bounce(5.0, 0.0, 10.0), 5.0);
        assert_eq!(bounce(13.0, 0.0, 10.0), 7.0);
        assert_eq!(bounce(-3.0, 0.0, 10.0), 3.0);
        assert_eq!(bounce(23.0, 0.0, 10.0), 3.0);
        // 2 s to fall 20 to the line, drifting 8 right: off the wall at 10, back to 7.
        assert_eq!(landing(5.0, 4.0, 0.0, 10.0, 20.0, (0.0, 10.0)), (7.0, 2.0));
    }

    #[test]
    fn the_short_way_round_a_wrapping_screen() {
        assert_eq!(wrapped(10.0, 90.0, 100.0), -20.0);
        assert_eq!(wrapped(90.0, 10.0, 100.0), 20.0);
    }
}
