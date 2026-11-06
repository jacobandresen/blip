//! The controls a bot works this frame, said the way a player would say it.

use crate::input::{BLIP_KEY_BUTTON2, BLIP_KEY_DOWN, BLIP_KEY_LEFT, BLIP_KEY_RIGHT, BLIP_KEY_SPACE, BLIP_KEY_UP};
use macroquad::input::KeyCode;

/// This frame's controls. Build one, say what to do, then [`Pad::send`] it:
///
/// ```ignore
/// let mut pad = Pad::new();
/// pad.steer(paddle_x, landing_x, 5.0);
/// pad.tap_fire();
/// pad.send();
/// ```
#[derive(Default)]
pub struct Pad {
    keys: Vec<KeyCode>,
}

impl Pad {
    pub fn new() -> Self {
        Self::default()
    }

    /// Hold `key` down this frame.
    pub fn hold(&mut self, key: KeyCode) -> &mut Self {
        if !self.keys.contains(&key) { self.keys.push(key); }
        self
    }

    /// Press `key`: down only if it was up last frame, so every tap is a
    /// fresh press (holding it would be a single press).
    pub fn tap(&mut self, key: KeyCode) -> &mut Self {
        if !super::held_before(key) { self.hold(key); }
        self
    }

    pub fn left(&mut self) -> &mut Self { self.hold(BLIP_KEY_LEFT) }
    pub fn right(&mut self) -> &mut Self { self.hold(BLIP_KEY_RIGHT) }
    pub fn up(&mut self) -> &mut Self { self.hold(BLIP_KEY_UP) }
    pub fn down(&mut self) -> &mut Self { self.hold(BLIP_KEY_DOWN) }

    /// Keep the fire button down (autofire, charging).
    pub fn fire(&mut self) -> &mut Self { self.hold(BLIP_KEY_SPACE) }
    /// One press of fire.
    pub fn tap_fire(&mut self) -> &mut Self { self.tap(BLIP_KEY_SPACE) }
    /// One press of the second button.
    pub fn tap_button2(&mut self) -> &mut Self { self.tap(BLIP_KEY_BUTTON2) }

    /// On a title, game-over or serve screen: press fire to go on.
    pub fn press_start(&mut self) -> &mut Self { self.tap_fire() }

    /// Move left or right from `at` toward `target`; within `slack` of it, stay.
    pub fn steer(&mut self, at: f32, target: f32, slack: f32) -> &mut Self {
        self.steer_with(at, target, slack, BLIP_KEY_LEFT, BLIP_KEY_RIGHT)
    }

    /// Move up or down from `at` toward `target` (screen y grows downward).
    pub fn climb(&mut self, at: f32, target: f32, slack: f32) -> &mut Self {
        self.steer_with(at, target, slack, BLIP_KEY_UP, BLIP_KEY_DOWN)
    }

    /// Rotate from `angle` toward `aim` (radians, clockwise is right);
    /// within `slack` of it, hold still.
    pub fn turn_to(&mut self, angle: f32, aim: f32, slack: f32) -> &mut Self {
        let d = crate::angle_diff(angle, aim);
        self.steer_with(0.0, d, slack, BLIP_KEY_LEFT, BLIP_KEY_RIGHT)
    }

    /// Toward `target` along one axis, with the keys for less and more.
    pub fn steer_with(&mut self, at: f32, target: f32, slack: f32, less: KeyCode, more: KeyCode) -> &mut Self {
        if target < at - slack { self.hold(less); } else if target > at + slack { self.hold(more); }
        self
    }

    /// Is `key` in this frame's controls?
    pub fn has(&self, key: KeyCode) -> bool {
        self.keys.contains(&key)
    }

    /// Hand this frame's controls to the game.
    pub fn send(&self) {
        super::hold(&self.keys);
    }
}
