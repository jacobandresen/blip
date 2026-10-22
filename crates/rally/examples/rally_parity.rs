//! Deterministic ball traces on the BASIC tape's 640x400 court, for
//! BASIC/Rust comparisons (test/rally-parity.mjs).
#[path = "../src/physics.rs"]
#[allow(dead_code)]
mod physics;
use physics::{Ball, Court, Event};

/// RALLY.BLIP's SETUP lines.
const TAPE: Court = Court {
    top: 32.0,
    bottom: 400.0,
    width: 640.0,
    lpad_x: 32.0,
    rpad_x: 596.0,
    pad_w: 12.0,
    pad_h: 64.0,
    ball: 12.0,
    speed_up: 20.0,
    top_speed: 600.0,
};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let case = args.get(1).map(String::as_str).unwrap_or("flight");
    let steps: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    let mut ball = Ball { ball_x: 314.0, ball_y: 210.0, ball_vx: 250.0, ball_vy: 80.0, ball_spd: 360.0 };
    let (mut left_y, right_y, mut left_vy, mut right_vy) = (184.0, 184.0, 0.0, 0.0);
    let dt = if case == "fast" { 0.1 } else { 0.016 };
    match case {
        "top" => (ball.ball_y, ball.ball_vy) = (33.0, -200.0),
        "bottom" => (ball.ball_y, ball.ball_vy) = (387.0, 200.0),
        "left" | "edge" | "spin" | "cap" | "fast" | "miss" | "corner" => {
            (ball.ball_x, ball.ball_vx, ball.ball_vy) = (45.0, -360.0, 0.0);
            match case {
                "edge" => ball.ball_y = 181.5,
                "spin" => left_vy = 300.0,
                "cap" => (ball.ball_vx, ball.ball_spd) = (-590.0, 590.0),
                "fast" => (ball.ball_x, ball.ball_vx, ball.ball_spd) = (85.0, -600.0, 600.0),
                "miss" => ball.ball_y = 330.0,
                "corner" => (left_y, ball.ball_y, ball.ball_vy) = (34.0, 31.0, -80.0),
                _ => {}
            }
        }
        "right" | "right-spin" => {
            (ball.ball_x, ball.ball_vx, ball.ball_vy) = (583.0, 360.0, 0.0);
            if case == "right-spin" {
                right_vy = -300.0;
            }
        }
        "point-left" => (ball.ball_x, ball.ball_y, ball.ball_vx, ball.ball_vy) = (-11.0, 330.0, -360.0, 0.0),
        "point-right" => (ball.ball_x, ball.ball_y, ball.ball_vx, ball.ball_vy) = (639.0, 330.0, 360.0, 0.0),
        _ => {}
    }
    let (mut score_l, mut score_r) = (0, 0);
    for _ in 0..steps {
        for event in physics::step(&TAPE, &mut ball, dt, left_y, right_y, left_vy, right_vy) {
            if let Event::Point(left) = event {
                if left {
                    score_l += 1;
                } else {
                    score_r += 1;
                }
            }
        }
    }
    println!(
        "[{}, {}, {}, {}, {}, {}, {}, {}, {}]",
        ball.ball_x, ball.ball_y, ball.ball_vx, ball.ball_vy, ball.ball_spd, score_l, score_r, left_y, right_y
    );
}
