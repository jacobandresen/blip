//! What the games' unit tests share.

use std::fmt::Debug;
use std::sync::{Mutex, MutexGuard};

static RANDOM: Mutex<()> = Mutex::new(());

/// Seed the random numbers and hold them until the guard drops. They are process-global and `cargo test` runs in
/// parallel, so a test that depends on them takes this first, or it passes or fails by scheduling.
pub fn seeded(seed: u64) -> MutexGuard<'static, ()> {
    // A poisoned lock only means another seeded test failed; this one can still run.
    let guard = RANDOM.lock().unwrap_or_else(|e| e.into_inner());
    crate::rand_seed(seed);
    guard
}

/// Assert `f` never goes down from one of `steps` to the next: a ramp that only climbs (or holds).
pub fn never_falls<I: Copy + Debug, T: PartialOrd + Debug>(what: &str, steps: impl IntoIterator<Item = I>, f: impl Fn(I) -> T) {
    pairs(what, steps, f, |a, b| b >= a, "falls");
}

/// Assert `f` goes up from each of `steps` to the next: a ramp that climbs every step.
pub fn climbs<I: Copy + Debug, T: PartialOrd + Debug>(what: &str, steps: impl IntoIterator<Item = I>, f: impl Fn(I) -> T) {
    pairs(what, steps, f, |a, b| b > a, "does not climb");
}

/// Assert `f` never goes up from one of `steps` to the next: a ramp that only eases (or holds).
pub fn never_rises<I: Copy + Debug, T: PartialOrd + Debug>(what: &str, steps: impl IntoIterator<Item = I>, f: impl Fn(I) -> T) {
    pairs(what, steps, f, |a, b| b <= a, "rises");
}

/// Assert an authored map is `rows` rows of `cols` characters each.
pub fn assert_grid(what: &str, map: &[&str], cols: usize, rows: usize) {
    assert_eq!(map.len(), rows, "{what}: {} rows, not {rows}", map.len());
    for (r, row) in map.iter().enumerate() {
        assert_eq!(row.chars().count(), cols, "{what}: row {r} is not {cols} wide");
    }
}

fn pairs<I: Copy + Debug, T: PartialOrd + Debug>(what: &str, steps: impl IntoIterator<Item = I>, f: impl Fn(I) -> T,
    ok: impl Fn(&T, &T) -> bool, wrong: &str) {
    let values: Vec<(I, T)> = steps.into_iter().map(|i| (i, f(i))).collect();
    for w in values.windows(2) {
        let ((i, a), (j, b)) = (&w[0], &w[1]);
        assert!(ok(a, b), "{what} {wrong} from {i:?} to {j:?}: {a:?} then {b:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ramp_that_holds_passes_and_one_that_dips_does_not() {
        never_falls("level", 1..=5, |l: i32| l.min(3));
        never_rises("gap", 1..=5, |l: i32| 10 - l);
        climbs("speed", 1..=5, |l: i32| l * 2);
        assert!(std::panic::catch_unwind(|| climbs("hold", [1, 2], |_: i32| 1)).is_err());
        assert!(std::panic::catch_unwind(|| never_falls("dip", [1, 2, 3], |l: i32| if l == 2 { 0 } else { l })).is_err());
    }
}
