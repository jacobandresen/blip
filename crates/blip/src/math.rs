//! Math and collision helpers — small utilities used across all games.

use macroquad::rand;

/// Clamp `v` so it stays within `[lo, hi]`.
#[inline]
pub fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
    if v < lo { lo } else if v > hi { hi } else { v }
}

/// Linear interpolation between `a` and `b`. `t=0` returns `a`, `t=1` returns `b`.
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Wrap `v` into the half-open interval `[lo, hi)`.
///
/// Values may be more than one span outside the interval. If `hi <= lo`,
/// returns `lo` because the interval has no positive span.
#[inline]
pub fn wrap(v: f32, lo: f32, hi: f32) -> f32 {
    let span = hi - lo;
    if span <= 0.0 { return lo; }
    (v - lo).rem_euclid(span) + lo
}

/// Random `f32` in the half-open range `[lo, hi)`.
///
/// Returns `lo` when `hi <= lo`.
#[inline]
pub fn rand_range_f32(lo: f32, hi: f32) -> f32 {
    if hi <= lo { return lo; }
    rand::gen_range(lo, hi)
}

/// Shortest signed angle from `from` to `to`, in `[-π, π)`.
#[inline]
pub fn angle_diff(from: f32, to: f32) -> f32 {
    (to - from + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI
}

/// Cubic ease-out for normalized input; clamps `t` to `[0, 1]`.
#[inline]
pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Hermite smoothstep for normalized input; clamps `t` to `[0, 1]`.
#[inline]
pub fn smoothstep01(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Seed the random number generator. For tests: the generator is global, so
/// an unseeded simulation depends on what the tests beside it drew first.
pub fn rand_seed(seed: u64) {
    rand::srand(seed);
}

/// Random integer in the inclusive range `[lo, hi]`.
pub fn rand_int(lo: i32, hi: i32) -> i32 {
    if hi <= lo {
        return lo;
    }
    rand::gen_range(lo as i64, hi as i64 + 1) as i32
}

/// Axis-aligned bounding-box (AABB) overlap test.
/// Returns true if rectangle 1 and rectangle 2 share any area.
/// Both rectangles are specified as (x, y, width, height) with origin at top-left.
#[inline]
pub fn rects_overlap(
    x1: f32, y1: f32, w1: f32, h1: f32,
    x2: f32, y2: f32, w2: f32, h2: f32,
) -> bool {
    x1 < x2 + w2 && x1 + w1 > x2 && y1 < y2 + h2 && y1 + h1 > y2
}

/// Whether the segment from `(x0, y0)` to `(x1, y1)` touches a circle.
#[inline]
pub fn segment_circle_overlap(
    x0: f32, y0: f32, x1: f32, y1: f32, cx: f32, cy: f32, radius: f32,
) -> bool {
    let (dx, dy) = (x1 - x0, y1 - y0);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > f32::EPSILON {
        (((cx - x0) * dx + (cy - y0) * dy) / len2).clamp(0.0, 1.0)
    } else { 0.0 };
    let (ex, ey) = (cx - (x0 + t * dx), cy - (y0 + t * dy));
    ex * ex + ey * ey <= radius * radius
}

/// Whether a moving point segment touches an axis-aligned rectangle.
#[inline]
pub fn segment_rect_overlap(
    x0: f32, y0: f32, x1: f32, y1: f32, rx: f32, ry: f32, rw: f32, rh: f32,
) -> bool {
    let (dx, dy) = (x1 - x0, y1 - y0);
    let (mut enter, mut leave) = (0.0_f32, 1.0_f32);
    for (p, d, lo, hi) in [(x0, dx, rx, rx + rw), (y0, dy, ry, ry + rh)] {
        if d.abs() <= f32::EPSILON {
            if p < lo || p > hi { return false; }
        } else {
            let a = (lo - p) / d;
            let b = (hi - p) / d;
            enter = enter.max(a.min(b));
            leave = leave.min(a.max(b));
            if enter > leave { return false; }
        }
    }
    true
}

/// Deterministic scatter in `[0, 1)`: the same value for the same `k` and
/// `salt` on every frame, so scenery (a crowd, stars, windows) can be placed
/// without storing it. `k` indexes the thing, `salt` separates two uses.
pub fn scatter(k: usize, salt: u32) -> f32 {
    let mut h = (k as u32).wrapping_mul(2654435761).wrapping_add(salt);
    h ^= h >> 15;
    h = h.wrapping_mul(2246822519);
    h ^= h >> 13;
    (h % 1024) as f32 / 1024.0
}

#[cfg(test)]
mod tests {
    use super::{angle_diff, ease_out_cubic, rand_range_f32, rand_seed, scatter,
        segment_circle_overlap, segment_rect_overlap, smoothstep01, wrap};

    #[test]
    fn angle_and_easing_helpers_cover_wraps_and_endpoints() {
        let pi = std::f32::consts::PI;
        assert!((angle_diff(3.0, -3.0) - (2.0 * pi - 6.0)).abs() < 1e-5);
        assert!((angle_diff(-3.0, 3.0) + (2.0 * pi - 6.0)).abs() < 1e-5);
        assert_eq!(ease_out_cubic(-1.0), 0.0);
        assert_eq!(ease_out_cubic(0.0), 0.0);
        assert_eq!(ease_out_cubic(1.0), 1.0);
        assert_eq!(ease_out_cubic(2.0), 1.0);
        assert_eq!(smoothstep01(-1.0), 0.0);
        assert_eq!(smoothstep01(0.0), 0.0);
        assert_eq!(smoothstep01(1.0), 1.0);
        assert_eq!(smoothstep01(2.0), 1.0);
    }

    #[test]
    fn swept_segment_tests_cover_hits_misses_and_stationary_points() {
        assert!(segment_circle_overlap(0.0, 0.0, 10.0, 0.0, 5.0, 1.0, 1.0));
        assert!(!segment_circle_overlap(0.0, 0.0, 10.0, 0.0, 5.0, 2.0, 1.0));
        assert!(segment_circle_overlap(5.0, 0.0, 5.0, 0.0, 5.0, 0.5, 1.0));
        assert!(segment_rect_overlap(0.0, 5.0, 20.0, 5.0, 8.0, 2.0, 2.0, 6.0));
        assert!(!segment_rect_overlap(0.0, 0.0, 5.0, 0.0, 8.0, 2.0, 2.0, 6.0));
        assert!(segment_rect_overlap(9.0, 5.0, 9.0, 5.0, 8.0, 2.0, 2.0, 6.0));
    }

    #[test]
    fn random_float_range_stays_in_bounds_and_collapses_empty_ranges() {
        rand_seed(0xB11F);
        for _ in 0..1000 {
            let v = rand_range_f32(-3.5, 7.25);
            assert!((-3.5..7.25).contains(&v), "value {v}");
        }
        assert_eq!(rand_range_f32(4.0, 4.0), 4.0);
        assert_eq!(rand_range_f32(8.0, 4.0), 8.0);
    }

    #[test]
    fn wrap_uses_half_open_interval_and_handles_multiple_spans() {
        assert_eq!(wrap(0.0, 0.0, 10.0), 0.0);
        assert_eq!(wrap(9.0, 0.0, 10.0), 9.0);
        assert_eq!(wrap(10.0, 0.0, 10.0), 0.0);
        assert_eq!(wrap(-1.0, 0.0, 10.0), 9.0);
        assert_eq!(wrap(31.0, 0.0, 10.0), 1.0);
        assert_eq!(wrap(2.0, 5.0, 15.0), 12.0);
        assert_eq!(wrap(3.0, 5.0, 5.0), 5.0);
        assert_eq!(wrap(3.0, 8.0, 5.0), 8.0);
    }

    #[test]
    fn scatter_is_the_same_every_time_and_stays_in_range() {
        for k in 0..500 {
            let v = scatter(k, 0x51);
            assert!((0.0..1.0).contains(&v));
            assert_eq!(v, scatter(k, 0x51));
        }
        // A different salt is a different scatter, and it does spread.
        let differs = (0..100).filter(|&k| scatter(k, 1) != scatter(k, 2)).count();
        assert!(differs > 90);
        let mean = (0..1000).map(|k| scatter(k, 7)).sum::<f32>() / 1000.0;
        assert!((0.4..0.6).contains(&mean), "mean {mean}");
    }
}
