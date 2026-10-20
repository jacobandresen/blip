//! Deterministic ball traces for BASIC/Rust comparisons.
#[path = "../src/physics.rs"]
#[allow(dead_code)]
mod physics;
use physics::{Ball, Event};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let case = args.get(1).map(String::as_str).unwrap_or("flight");
    let steps: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    let mut ball = Ball {
        ball_x: 234.0,
        ball_y: 278.0,
        ball_vx: 250.0,
        ball_vy: 80.0,
        ball_spd: 275.0,
    };
    let (mut left_y, right_y, mut left_vy, mut right_vy) = (248.0, 248.0, 0.0, 0.0);
    let dt = if case == "fast" { 0.1 } else { 0.016 };
    match case {
        "top" => {
            ball.ball_y = 29.0;
            ball.ball_vy = -200.0;
        }
        "bottom" => {
            ball.ball_y = 527.0;
            ball.ball_vy = 200.0;
        }
        "left" | "edge" | "spin" | "cap" | "fast" | "miss" | "corner" => {
            ball.ball_x = 41.0;
            ball.ball_vx = -275.0;
            ball.ball_vy = 0.0;
            if case == "edge" {
                ball.ball_y = 245.5;
            }
            if case == "spin" {
                left_vy = 300.0;
            }
            if case == "cap" {
                ball.ball_vx = -445.0;
                ball.ball_spd = 445.0;
            }
            if case == "fast" {
                ball.ball_x = 65.0;
                ball.ball_vx = -450.0;
                ball.ball_spd = 450.0;
            }
            if case == "miss" {
                ball.ball_y = 400.0;
            }
            if case == "corner" {
                left_y = 30.0;
                ball.ball_y = 27.0;
                ball.ball_vy = -80.0;
            }
        }
        "right" | "right-spin" => {
            ball.ball_x = 427.0;
            ball.ball_vx = 275.0;
            ball.ball_vy = 0.0;
            if case == "right-spin" {
                right_vy = -300.0;
            }
        }
        "point-left" => {
            ball.ball_x = -11.0;
            ball.ball_y = 400.0;
            ball.ball_vx = -275.0;
            ball.ball_vy = 0.0;
        }
        "point-right" => {
            ball.ball_x = 479.0;
            ball.ball_y = 400.0;
            ball.ball_vx = 275.0;
            ball.ball_vy = 0.0;
        }
        _ => {}
    }
    let (mut score_l, mut score_r) = (0, 0);
    for _ in 0..steps {
        for event in physics::step(&mut ball, dt, left_y, right_y, left_vy, right_vy) {
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
        ball.ball_x,
        ball.ball_y,
        ball.ball_vx,
        ball.ball_vy,
        ball.ball_spd,
        score_l,
        score_r,
        left_y,
        right_y
    );
}
