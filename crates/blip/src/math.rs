//! Math and collision helpers — small utilities used across all games. The random ones need the engine; the rest
//! serve build scripts too.

/// Clamp `v` so it stays within `[lo, hi]`.
#[inline]
pub fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

/// Linear interpolation between `a` and `b`. `t=0` returns `a`, `t=1` returns `b`.
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Euclidean distance between two points, without squaring large coordinates.
#[inline]
pub fn distance(x0: f32, y0: f32, x1: f32, y1: f32) -> f32 {
    (x1 - x0).hypot(y1 - y0)
}

/// Wrap `v` into `[lo, hi)`, including values multiple spans away.
/// Returns `lo` when `hi <= lo`.
#[inline]
pub fn wrap(v: f32, lo: f32, hi: f32) -> f32 {
    let span = hi - lo;
    if span <= 0.0 {
        return lo;
    }
    (v - lo).rem_euclid(span) + lo
}

/// Random `f32` in the half-open range `[lo, hi)`.
///
/// Returns `lo` when `hi <= lo`.
#[cfg(feature = "engine")]
#[inline]
pub fn rand_range_f32(lo: f32, hi: f32) -> f32 {
    if hi <= lo {
        return lo;
    }
    macroquad::rand::gen_range(lo, hi)
}

/// Shortest signed angle from `from` to `to`, in `[-π, π)`.
#[inline]
pub fn angle_diff(from: f32, to: f32) -> f32 {
    (to - from + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

/// A speed with mass: `v` moves toward `target` by up to `accel` per second, or `brake` when stopping or
/// turning back. A bot rolling a mover ahead calls the same function the game does.
#[inline]
pub fn spool(v: f32, target: f32, accel: f32, brake: f32, dt: f32) -> f32 {
    let rate = if target == 0.0 || target * v < 0.0 { brake } else { accel } * dt;
    v + clamp(target - v, -rate, rate)
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
#[cfg(feature = "engine")]
pub fn rand_seed(seed: u64) {
    macroquad::rand::srand(seed);
}

/// Random integer in the inclusive range `[lo, hi]`.
#[cfg(feature = "engine")]
pub fn rand_int(lo: i32, hi: i32) -> i32 {
    if hi <= lo {
        return lo;
    }
    macroquad::rand::gen_range(lo as i64, hi as i64 + 1) as i32
}

/// Axis-aligned bounding-box (AABB) overlap test.
/// Returns true if rectangle 1 and rectangle 2 share any area.
/// Both rectangles are specified as (x, y, width, height) with origin at top-left.
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn rects_overlap(x1: f32, y1: f32, w1: f32, h1: f32, x2: f32, y2: f32, w2: f32, h2: f32) -> bool {
    x1 < x2 + w2 && x1 + w1 > x2 && y1 < y2 + h2 && y1 + h1 > y2
}

/// Whether the segment from `(x0, y0)` to `(x1, y1)` touches a circle.
#[inline]
pub fn segment_circle_overlap(x0: f32, y0: f32, x1: f32, y1: f32, cx: f32, cy: f32, radius: f32) -> bool {
    let (dx, dy) = (x1 - x0, y1 - y0);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > f32::EPSILON { (((cx - x0) * dx + (cy - y0) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
    let (ex, ey) = (cx - (x0 + t * dx), cy - (y0 + t * dy));
    ex * ex + ey * ey <= radius * radius
}

/// Whether a moving point segment touches an axis-aligned rectangle.
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn segment_rect_overlap(x0: f32, y0: f32, x1: f32, y1: f32, rx: f32, ry: f32, rw: f32, rh: f32) -> bool {
    let (dx, dy) = (x1 - x0, y1 - y0);
    let (mut enter, mut leave) = (0.0_f32, 1.0_f32);
    for (p, d, lo, hi) in [(x0, dx, rx, rx + rw), (y0, dy, ry, ry + rh)] {
        if d.abs() <= f32::EPSILON {
            if p < lo || p > hi {
                return false;
            }
        } else {
            let a = (lo - p) / d;
            let b = (hi - p) / d;
            enter = enter.max(a.min(b));
            leave = leave.min(a.max(b));
            if enter > leave {
                return false;
            }
        }
    }
    true
}

/// Where each thing in a pool was as the frame began, for swept hit tests: `at` gives a live thing's position and
/// `None` for an empty slot. A thing spawned into an empty slot during the frame then sweeps from where it is, not
/// from where the slot's previous thing ended (which can be anywhere, often right where it last hit something).
pub fn frame_starts<T>(pool: &[T], at: impl Fn(&T) -> Option<(f32, f32)>) -> Vec<Option<(f32, f32)>> {
    pool.iter().map(at).collect()
}

/// Where a swept test begins: the frame's start, or where the thing is now when it arrived during the frame.
#[inline]
pub fn swept_from(start: Option<(f32, f32)>, now: (f32, f32)) -> (f32, f32) {
    start.unwrap_or(now)
}

/// Did a point moving from `from` to `to` touch a box carried by an anchor moving from `anchor_from` to `anchor_to`?
/// `rect` is the box (x, y, w, h) relative to its anchor. Tested in the box's own frame, so neither passes through
/// the other however fast both move.
#[inline]
pub fn sweep_hits(from: (f32, f32), to: (f32, f32), anchor_from: (f32, f32), anchor_to: (f32, f32), rect: (f32, f32, f32, f32)) -> bool {
    segment_rect_overlap(from.0 - anchor_from.0, from.1 - anchor_from.1, to.0 - anchor_to.0, to.1 - anchor_to.1, rect.0, rect.1, rect.2, rect.3)
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

/// Repeatable hash of a grid cell in `[0, 1)`, for per-cell look without
/// storing it: the same value for the same cell and seed on every frame.
pub fn hash(x: i32, y: i32, s: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263) ^ s.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0xffff) as f32 / 65535.0
}

/// Smooth value noise in `[0, 1]`: `hash` on the integer lattice, eased between its points. One unit is one lattice
/// cell: divide the coordinates by the size the features should have.
pub fn value_noise(x: f32, y: f32, seed: u32) -> f32 {
    let (xi, yi) = (x.floor() as i32, y.floor() as i32);
    let (xf, yf) = (x - x.floor(), y - y.floor());
    let (u, v) = (smoothstep01(xf), smoothstep01(yf));
    let a = hash(xi, yi, seed);
    let b = hash(xi + 1, yi, seed);
    let c = hash(xi, yi + 1, seed);
    let d = hash(xi + 1, yi + 1, seed);
    a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v
}

/// Whether (x, y) is inside the polygon `pts` (even-odd rule).
pub fn in_polygon(pts: &[(f32, f32)], x: f32, y: f32) -> bool {
    let mut inside = false;
    let mut j = pts.len() - 1;
    for i in 0..pts.len() {
        let ((xi, yi), (xj, yj)) = (pts[i], pts[j]);
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// The box round some points: (left, top, right, bottom).
pub fn bounds(pts: &[(f32, f32)]) -> (f32, f32, f32, f32) {
    pts.iter().fold((f32::MAX, f32::MAX, f32::MIN, f32::MIN), |(x0, y0, x1, y1), &(x, y)| {
        (x0.min(x), y0.min(y), x1.max(x), y1.max(y))
    })
}

#[cfg(test)]
mod tests {
    use super::{
        angle_diff, bounds, ease_out_cubic, hash, in_polygon, rand_range_f32, rand_seed, scatter, segment_circle_overlap,
        segment_rect_overlap, smoothstep01, value_noise, wrap,
    };

    #[test]
    fn value_noise_meets_the_lattice_and_stays_in_range() {
        assert_eq!(value_noise(3.0, -2.0, 7), hash(3, -2, 7), "on a lattice point it is the point's hash");
        let mut prev = value_noise(0.0, 0.5, 1);
        for k in 1..400 {
            let v = value_noise(k as f32 * 0.01, 0.5, 1);
            assert!((0.0..=1.0).contains(&v));
            assert!((v - prev).abs() < 0.05, "smooth: no jump between nearby points");
            prev = v;
        }
    }

    #[test]
    fn point_distance_handles_coincident_negative_and_large_coordinates() {
        assert_eq!(super::distance(2.0, -3.0, 2.0, -3.0), 0.0);
        assert_eq!(super::distance(-2.0, -3.0, 1.0, 1.0), 5.0);
        assert_eq!(super::distance(0.0, 0.0, 3e20, 4e20), 5e20);
    }

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
    fn a_thing_spawned_mid_frame_sweeps_from_where_it_is_and_a_moving_box_is_swept_in_its_own_frame() {
        let starts = super::frame_starts(&[(true, 5.0), (false, 99.0)], |&(live, x)| live.then_some((x, 0.0)));
        assert_eq!(super::swept_from(starts[0], (6.0, 0.0)), (5.0, 0.0));
        assert_eq!(super::swept_from(starts[1], (7.0, 0.0)), (7.0, 0.0));
        // A point and a 2x2 box move right together: they never meet, though the point's path crosses where the box was.
        assert!(!super::sweep_hits((0.0, 0.0), (10.0, 0.0), (5.0, -1.0), (15.0, -1.0), (0.0, 0.0, 2.0, 2.0)));
        assert!(super::sweep_hits((0.0, 0.0), (10.0, 0.0), (5.0, -1.0), (5.0, -1.0), (0.0, 0.0, 2.0, 2.0)));
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

    #[test]
    fn a_point_is_inside_a_square_and_outside_its_notch() {
        let sq = [(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)];
        assert!(in_polygon(&sq, 2.0, 2.0) && !in_polygon(&sq, 5.0, 2.0));
        let notch = [(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (2.0, 1.0), (0.0, 4.0)];
        assert!(!in_polygon(&notch, 2.0, 3.0) && in_polygon(&notch, 1.0, 1.0));
        assert_eq!(bounds(&notch), (0.0, 0.0, 4.0, 4.0));
    }
}
