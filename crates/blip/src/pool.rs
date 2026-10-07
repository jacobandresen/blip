//! Helpers for fixed-size arrays of active and inactive entities.

/// Reports whether an entity occupies an active pool slot.
pub trait Pooled {
    fn is_active(&self) -> bool;
}

/// Copies `item` into the first inactive slot, or returns `false` if full.
pub fn pool_spawn<T: Pooled + Copy>(pool: &mut [T], item: T) -> bool {
    if let Some(slot) = pool.iter_mut().find(|slot| !slot.is_active()) {
        *slot = item;
        true
    } else {
        false
    }
}

/// Iterate over only the active entries in `pool`.
pub fn pool_iter<T: Pooled>(pool: &[T]) -> impl Iterator<Item = &T> {
    pool.iter().filter(|e| e.is_active())
}

/// Mutably iterate over only the active entries in `pool`.
pub fn pool_iter_mut<T: Pooled>(pool: &mut [T]) -> impl Iterator<Item = &mut T> {
    pool.iter_mut().filter(|e| e.is_active())
}
