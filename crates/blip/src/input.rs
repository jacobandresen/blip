//! Input — wraps macroquad's keyboard state with named BLIP_KEY_* constants.
//! Games should use these constants rather than raw `KeyCode` values so the
//! same bindings work whether the player uses arrow keys or WASD.
//! Touch input from the web shell is translated into these same key events by shell.js,
//! so you get mobile support for free.

use macroquad::input::{is_key_down, is_key_pressed, KeyCode};

// Direction keys — available as both arrow keys and WASD.
pub const BLIP_KEY_UP:      KeyCode = KeyCode::Up;
pub const BLIP_KEY_DOWN:    KeyCode = KeyCode::Down;
pub const BLIP_KEY_LEFT:    KeyCode = KeyCode::Left;
pub const BLIP_KEY_RIGHT:   KeyCode = KeyCode::Right;
pub const BLIP_KEY_W:       KeyCode = KeyCode::W;
pub const BLIP_KEY_A:       KeyCode = KeyCode::A;
pub const BLIP_KEY_S:       KeyCode = KeyCode::S;
pub const BLIP_KEY_D:       KeyCode = KeyCode::D;
// Action buttons
pub const BLIP_KEY_SPACE:   KeyCode = KeyCode::Space;  // primary fire / jump / confirm
pub const BLIP_KEY_BUTTON2: KeyCode = KeyCode::Z;       // secondary action
// Two more action keys, for the rare game that needs a row of them —
// brawler puts a kick height on each. Z, X, C sit in a row on the
// keyboard the way a cabinet's kick buttons sit in a row on the deck.
pub const BLIP_KEY_X:       KeyCode = KeyCode::X;
pub const BLIP_KEY_C:       KeyCode = KeyCode::C;
// Two home-row clusters, for a game two people play at one keyboard.
// F-G-H and J-K-L sit either side of the split a touch typist's hands
// already make, so the player on the left and the player on the right
// each get three buttons without reaching across each other.
pub const BLIP_KEY_F:       KeyCode = KeyCode::F;
pub const BLIP_KEY_G:       KeyCode = KeyCode::G;
pub const BLIP_KEY_H:       KeyCode = KeyCode::H;
pub const BLIP_KEY_J:       KeyCode = KeyCode::J;
pub const BLIP_KEY_K:       KeyCode = KeyCode::K;
pub const BLIP_KEY_L:       KeyCode = KeyCode::L;

/// Primary fire / jump / confirm — true only on the frame the key goes down.
#[inline]
pub fn btn1_pressed() -> bool { is_key_pressed(BLIP_KEY_SPACE) }

/// Secondary action button — true only on the frame the key goes down.
#[inline]
pub fn btn2_pressed() -> bool { is_key_pressed(BLIP_KEY_BUTTON2) }

/// True every frame the key is held down — good for movement.
#[inline]
pub fn key_held(key: KeyCode) -> bool {
    is_key_down(key)
}

/// True only on the single frame the key first goes down — good for jumping or firing.
#[inline]
pub fn key_pressed(key: KeyCode) -> bool {
    is_key_pressed(key)
}

/// True if any key was first pressed this frame — handy for "press any key to continue" screens.
pub fn any_key_pressed() -> bool {
    use macroquad::input::get_last_key_pressed;
    get_last_key_pressed().is_some()
}
