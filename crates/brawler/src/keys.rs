//! The keys: which ones each player answers to, and one frame of them as
//! the `Input` the rules read.

use super::*;

/// The keys one player answers to: a stick and two buttons. Two at one
/// keyboard get sets that never overlap (W A S D with F G; the arrows with
/// J K); alone, player one also answers to the arrows and the fire keys.
pub(crate) struct Pad {
    pub(crate) up: &'static [KeyCode],
    pub(crate) down: &'static [KeyCode],
    pub(crate) left: &'static [KeyCode],
    pub(crate) right: &'static [KeyCode],
    pub(crate) punch: &'static [KeyCode],
    pub(crate) kick: &'static [KeyCode],
}

pub(crate) static P1_ALONE: Pad = Pad {
    up: &[BLIP_KEY_W, BLIP_KEY_UP],
    down: &[BLIP_KEY_S, BLIP_KEY_DOWN],
    left: &[BLIP_KEY_A, BLIP_KEY_LEFT],
    right: &[BLIP_KEY_D, BLIP_KEY_RIGHT],
    // The generic fire keys (Space, button 2) are the low attacks, the
    // pokes you open with; C and X are the high ones.
    punch: &[BLIP_KEY_F, BLIP_KEY_SPACE],
    kick: &[BLIP_KEY_G, BLIP_KEY_BUTTON2],
};

pub(crate) static P1_SHARING: Pad = Pad {
    up: &[BLIP_KEY_W],
    down: &[BLIP_KEY_S],
    left: &[BLIP_KEY_A],
    right: &[BLIP_KEY_D],
    punch: &[BLIP_KEY_F],
    kick: &[BLIP_KEY_G],
};

pub(crate) static P2: Pad = Pad {
    up: &[BLIP_KEY_UP],
    down: &[BLIP_KEY_DOWN],
    left: &[BLIP_KEY_LEFT],
    right: &[BLIP_KEY_RIGHT],
    punch: &[BLIP_KEY_J],
    kick: &[BLIP_KEY_K],
};

/// The keys player `who` uses in this mode.
pub(crate) fn pad(mode: Mode, who: usize) -> &'static Pad {
    // Who comes first. Matching on the mode first handed player two
    // player one's keys whenever the game was in one-player mode,
    // which is every screen before the mode has been chosen.
    match (who, mode) {
        (1, _) => &P2,
        (_, Mode::Solo) => &P1_ALONE,
        _ => &P1_SHARING,
    }
}

/// Is any of these keys down?
pub(crate) fn any_held(keys: &[KeyCode]) -> bool { keys.iter().any(|k| key_held(*k)) }
/// Was any of these keys pressed this frame?
pub(crate) fn any_pressed(keys: &[KeyCode]) -> bool { keys.iter().any(|k| key_pressed(*k)) }

/// Player `who`'s keys as an `Input`, with the late second button of a
/// special allowed for.
pub(crate) fn human_input(g: &mut Game, who: usize) -> Input {
    let k = pad(g.mode, who);
    let mut inp = Input::default();
    inp.left = any_held(k.left);
    inp.right = any_held(k.right);
    inp.up = any_held(k.up);
    inp.down = any_held(k.down);
    // Two buttons; the stick picks the height. Held toward the opponent a
    // punch or kick is the overhead one, otherwise the low poke (down and
    // up turn them into the sweep and the flying kick in grounded_move()).
    let toward = if g.p[who].facing > 0.0 { inp.right } else { inp.left };
    let (punch, kick) = (any_pressed(k.punch), any_pressed(k.kick));
    let (plo, phi) = (punch && !toward, punch && toward);
    let (low, high) = (kick && !toward, kick && toward);
    if punch { g.punch_at[who] = g.now; }
    if kick { g.kick_at[who] = g.now; }
    // Punch and kick together inside a short window is the special: easy on a
    // stick, and in the way of no other move.
    let (pa, ka) = (g.punch_at[who], g.kick_at[who]);
    let together = (pa - ka).abs() <= 0.08 && g.now - pa.max(ka) <= 0.08
        && pa > 0.0 && ka > 0.0;
    if together {
        inp.special = true;
        g.punch_at[who] = -1.0;
        g.kick_at[who] = -1.0;
    } else {
        inp.punch_low = plo;
        inp.punch_high = phi;
        inp.kick_low = low;
        inp.kick_high = high;
    }
    inp
}
