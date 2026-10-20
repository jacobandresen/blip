//! Shared keyboard input helpers for game code.
//! Web touch controls feed these same key states through `shell.js`.

use macroquad::input::{is_key_down, is_key_pressed, KeyCode};
use std::collections::HashSet;

/// Preserve press edges while a logical update is waiting or spans several frames.
#[derive(Default)]
pub struct PressBuffer {
    pending: HashSet<KeyCode>,
    current: HashSet<KeyCode>,
}

impl PressBuffer {
    pub fn clear(&mut self) {
        self.pending.clear();
        self.current.clear();
        crate::web::clear_buffered_keys();
    }

    pub fn capture(&mut self) {
        self.pending.extend(macroquad::input::get_keys_pressed());
        use KeyCode::*;
        // Audio decoding yields inside macroquad and can clear its frame-local edges.
        let keys = [Up, Down, Left, Right, Space, Enter, Escape,
            A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z];
        self.pending.extend(keys.into_iter().filter(|key| crate::web::buffered_key_pressed(*key)));
        crate::web::clear_buffered_keys();
    }

    pub fn begin_step(&mut self) {
        self.current = std::mem::take(&mut self.pending);
    }

    pub fn pressed(&self, key: KeyCode) -> bool {
        self.current.contains(&key) || crate::bot::pressed(key)
    }

    pub fn active(&self, key: KeyCode) -> bool {
        key_held(key) || self.pressed(key)
    }
}

pub const BLIP_KEY_UP:      KeyCode = KeyCode::Up;
pub const BLIP_KEY_DOWN:    KeyCode = KeyCode::Down;
pub const BLIP_KEY_LEFT:    KeyCode = KeyCode::Left;
pub const BLIP_KEY_RIGHT:   KeyCode = KeyCode::Right;
pub const BLIP_KEY_W:       KeyCode = KeyCode::W;
pub const BLIP_KEY_A:       KeyCode = KeyCode::A;
pub const BLIP_KEY_S:       KeyCode = KeyCode::S;
pub const BLIP_KEY_D:       KeyCode = KeyCode::D;
pub const BLIP_KEY_SPACE:   KeyCode = KeyCode::Space;  // primary fire / jump / confirm
pub const BLIP_KEY_BUTTON2: KeyCode = KeyCode::Z;       // secondary action
pub const BLIP_KEY_F:       KeyCode = KeyCode::F;
pub const BLIP_KEY_G:       KeyCode = KeyCode::G;
pub const BLIP_KEY_J:       KeyCode = KeyCode::J;
pub const BLIP_KEY_K:       KeyCode = KeyCode::K;

/// Primary fire / jump / confirm — true only on the frame the key goes down.
#[inline]
pub fn btn1_pressed() -> bool { key_pressed(BLIP_KEY_SPACE) }

/// True every frame the key is held down — good for movement.
#[inline]
pub fn key_held(key: KeyCode) -> bool {
    is_key_down(key) || crate::bot::held(key)
}

/// Held, or pressed this frame: a touch tap shorter than a frame (down and
/// up between two frames) is never "held", and would otherwise be lost.
#[inline]
pub fn key_active(key: KeyCode) -> bool {
    key_held(key) || is_key_pressed(key)
}

/// True only on the single frame the key first goes down — good for jumping or firing.
#[inline]
pub fn key_pressed(key: KeyCode) -> bool {
    is_key_pressed(key) || crate::bot::pressed(key)
}

/// True if any key was first pressed this frame — handy for "press any key to continue" screens.
pub fn any_key_pressed() -> bool {
    use macroquad::input::get_last_key_pressed;
    get_last_key_pressed().is_some() || crate::bot::any_pressed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn press_edges_wait_for_an_update_and_stay_stable_until_the_next_one() {
        let mut input = PressBuffer::default();
        input.pending.extend([KeyCode::Left, KeyCode::Space]);
        assert!(!input.pressed(KeyCode::Left));
        input.begin_step();
        assert!(input.pressed(KeyCode::Left));
        assert!(input.pressed(KeyCode::Space));
        input.pending.insert(KeyCode::Right);
        assert!(!input.pressed(KeyCode::Right));
        assert!(input.pressed(KeyCode::Left));
        input.begin_step();
        assert!(!input.pressed(KeyCode::Left));
        assert!(!input.pressed(KeyCode::Space));
        assert!(input.pressed(KeyCode::Right));
        input.begin_step();
        assert!(!input.pressed(KeyCode::Right));
    }
}
