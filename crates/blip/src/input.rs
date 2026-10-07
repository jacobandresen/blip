//! Shared keyboard input helpers for game code.
//! Web touch controls feed these same key states through `shell.js`.

use macroquad::input::{is_key_down, is_key_pressed, KeyCode};

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
