//! Input — wraps macroquad's keyboard state with the BLIP_KEY_* constants.
//! Touch input from the web shell synthesizes keyboard events via shell.js.

use macroquad::input::{is_key_down, is_key_pressed, KeyCode};

pub const BLIP_KEY_UP:    KeyCode = KeyCode::Up;
pub const BLIP_KEY_DOWN:  KeyCode = KeyCode::Down;
pub const BLIP_KEY_LEFT:  KeyCode = KeyCode::Left;
pub const BLIP_KEY_RIGHT: KeyCode = KeyCode::Right;
pub const BLIP_KEY_W:     KeyCode = KeyCode::W;
pub const BLIP_KEY_A:     KeyCode = KeyCode::A;
pub const BLIP_KEY_S:     KeyCode = KeyCode::S;
pub const BLIP_KEY_D:     KeyCode = KeyCode::D;
pub const BLIP_KEY_SPACE: KeyCode = KeyCode::Space;

#[inline]
pub fn key_held(key: KeyCode) -> bool {
    is_key_down(key)
}

#[inline]
pub fn key_pressed(key: KeyCode) -> bool {
    is_key_pressed(key)
}

/// Was *any* key first-pressed this frame?
pub fn any_key_pressed() -> bool {
    use macroquad::input::get_last_key_pressed;
    get_last_key_pressed().is_some()
}
