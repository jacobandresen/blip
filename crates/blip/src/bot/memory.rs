//! What a bot keeps from one frame to the next. A game's `drive(&g)` is called afresh every frame; a bot that
//! commits to a target, counts down a wait or remembers where a threat was keeps it here, one value per type.

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static MINDS: RefCell<HashMap<TypeId, Box<dyn Any>>> = RefCell::new(HashMap::new());
}

/// Work on the bot's remembered `T` (made with `Default` the first time): `remember(|m: &mut Mind| m.target)`.
pub fn remember<T: Default + 'static, R>(f: impl FnOnce(&mut T) -> R) -> R {
    // Taken out while `f` runs, so `f` may remember something else.
    let mut value: Box<dyn Any> = MINDS.with(|m| m.borrow_mut().remove(&TypeId::of::<T>()))
        .unwrap_or_else(|| Box::new(T::default()));
    let out = f(value.downcast_mut::<T>().expect("one value per type"));
    MINDS.with(|m| m.borrow_mut().insert(TypeId::of::<T>(), value));
    out
}

/// Forget everything (a new game).
pub fn forget() {
    MINDS.with(|m| m.borrow_mut().clear());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Count(u32);

    #[test]
    fn a_value_is_kept_between_calls_and_can_be_nested() {
        forget();
        remember(|c: &mut Count| c.0 += 1);
        let n = remember(|c: &mut Count| { c.0 += 1; remember(|s: &mut String| s.push('x')); c.0 });
        assert_eq!(n, 2);
        assert_eq!(remember(|s: &mut String| s.clone()), "x");
    }
}
