//! Pool — helpers for fixed-size entity arrays with an active/inactive flag.
//!
//! All games store entities in a static array where each slot has an `active: bool`
//! (or `alive: bool`) field. Implement [`Pooled`] with a one-liner, then use the
//! free functions to replace manual `if !e.active { continue; }` loops.
//!
//! ```ignore
//! impl Pooled for Barrel {
//!     fn is_active(&self) -> bool { self.active }
//! }
//!
//! // Spawn into the first free slot:
//! pool_spawn(&mut g.barrels, Barrel { active: true, .. });
//!
//! // Iterate only active entries:
//! for b in pool_iter(&g.barrels) { draw_barrel(blip, b); }
//! for b in pool_iter_mut(&mut g.barrels) { update_barrel(b, dt); }
//! ```

/// Implement on any entity struct that can be active or inactive.
/// One-liner: `fn is_active(&self) -> bool { self.active }`.
pub trait Pooled {
    fn is_active(&self) -> bool;
}

/// Copy `item` into the first inactive slot in `pool`.
/// Returns `false` if the pool is full — a sign to increase the pool size or tune spawn rates.
pub fn pool_spawn<T: Pooled + Copy>(pool: &mut [T], item: T) -> bool {
    for slot in pool.iter_mut() {
        if !slot.is_active() {
            *slot = item;
            return true;
        }
    }
    false
}

/// Iterate over only the active entries in `pool`.
pub fn pool_iter<T: Pooled>(pool: &[T]) -> impl Iterator<Item = &T> {
    pool.iter().filter(|e| e.is_active())
}

/// Mutably iterate over only the active entries in `pool`.
pub fn pool_iter_mut<T: Pooled>(pool: &mut [T]) -> impl Iterator<Item = &mut T> {
    pool.iter_mut().filter(|e| e.is_active())
}
