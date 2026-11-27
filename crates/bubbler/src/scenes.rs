//! Posed screens: the cabinet card, and BUBBLER_SCENE's frozen screens for checking the text.

use super::*;

/// The cabinet card: two dragons mid-round, bubbles up, a chain ready.
pub(super) fn stage_cabinet_card(g: &mut Game) {
    g.start_game(true);
    g.state = State::Play;
    g.p[0].x = 150.0;
    g.p[0].y = HUD + 19.0 * TILE - P_H;
    g.p[0].mouth = 10.0;
    g.p[0].safe = 0.0;
    g.p[1].x = 360.0;
    g.p[1].y = HUD + 15.0 * TILE - P_H;
    g.p[1].face = -1.0;
    g.p[1].safe = 0.0;
    for (k, e) in g.enemies.iter_mut().enumerate() {
        if k < 3 {
            e.active = false;
        }
        e.pop_in = 1.0;
    }
    for (k, (x, y)) in
        [(282.0, 70.0), (306.0, 72.0), (330.0, 70.0), (200.0, 260.0), (230.0, 390.0)].iter().enumerate()
    {
        g.bubbles.push(Bubble {
            x: *x,
            y: *y,
            vx: 0.0,
            age: 1.0,
            phase: if k < 3 { Phase::Top } else { Phase::Float },
            owner: k % 2,
            trapped: if k < 3 { Some((Kind::Walker, false)) } else { None },
            active: true,
            wob: k as f32,
            squish: 0.0,
        });
    }
    // one of each treat on the floor
    for (kind, x) in [(0usize, 150.0f32), (1, 196.0), (2, 410.0), (3, 456.0)] {
        g.fruits.push(Fruit {
            x,
            y: HUD + 23.0 * TILE - 16.0,
            vx: 0.0,
            vy: 0.0,
            kind,
            t: 1.0,
            on_ground: true,
            active: true,
        });
    }
}

/// Native-only: BUBBLER_SCENE=intro|play|hurry|chain|solo|clear|over|won
/// jumps to that screen and freezes it, for checking the text.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn stage_scene(g: &mut Game, scene: &str) {
    if scene != "title" {
        g.start_game(true);
        g.state = State::Play;
    }
    for e in g.enemies.iter_mut() {
        e.pop_in = 1.0;
    }
    for p in g.p.iter_mut() {
        p.safe = 0.0;
        p.score = 128_450;
    }
    g.p[1].score = 97_320;
    match scene {
        "intro" => {
            g.state = State::Intro;
            g.state_t = 1.2;
        }
        "hurry" => {
            g.round_t = hurry_at(g.round) + 0.1;
            g.hurry = true;
        }
        "chain" => {
            g.chain = (3, 1.0);
        }
        "solo" => {
            g.p[1].joined = false;
            g.two_up = false;
        }
        "clear" => {
            g.state = State::Clear;
            g.state_t = 0.8;
        }
        "over" => {
            g.state = State::Over;
            g.state_t = 3.0;
            g.cont.start(CONTINUE_SECS);
        }
        "won" => {
            g.state = State::Won;
            g.state_t = 3.0;
        }
        _ => {}
    }
}
