//! Deterministic rule traces for the BASIC game's browser comparisons.

#[path = "../src/arena.rs"]
#[allow(dead_code)]
mod arena;

#[path = "../src/cpu.rs"]
#[allow(dead_code)]
mod cpu;

use arena::{Pt, Steer, World, SEATS};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let scenario = args.get(1).map(String::as_str).unwrap_or("straight");
    let steps: usize = args
        .get(2)
        .and_then(|value| value.parse().ok())
        .unwrap_or(60);
    let mut world = World::new(1);
    for (i, snake) in world.snakes.iter_mut().enumerate() {
        snake.head = if i == 0 {
            Pt::new(100.0, 180.0)
        } else {
            Pt::new(360.0, 380.0)
        };
        snake.heading = if i == 0 { 0.0 } else { std::f32::consts::PI };
        snake.path = (0..17)
            .map(|j| {
                Pt::new(
                    snake.head.x - snake.heading.cos() * j as f32 * 2.0,
                    snake.head.y - snake.heading.sin() * j as f32 * 2.0,
                )
            })
            .collect();
    }
    if scenario == "cpuwall" {
        let snake = &mut world.snakes[1];
        snake.head = Pt::new(440.0, 380.0);
        snake.heading = 0.0;
        snake.path = (0..17)
            .map(|i| Pt::new(440.0 - i as f32 * 2.0, 380.0))
            .collect();
    }
    if scenario == "wall" || scenario == "over" {
        world.snakes[0].head = Pt::new(467.0, 180.0);
    }
    if scenario == "over" {
        world.snakes[0].lives = 1;
    }
    if ["body", "gap", "spent", "overlap"].contains(&scenario) {
        world.snakes[1].path.push_back(Pt::new(102.0, 180.0));
        if scenario != "body" {
            world.snakes[1].holes.push(arena::Hole {
                at: 17,
                len: 1,
                spent: scenario == "spent",
            });
        }
        if scenario == "overlap" {
            world.snakes[1].path.push_back(Pt::new(102.0, 180.0));
        }
    }
    if scenario == "join" {
        world.join_human(1);
    }
    let mut steers = [Steer::default(); SEATS];
    if scenario == "left" {
        steers[0].turn = -1.0;
    }
    if scenario == "right" {
        steers[0].turn = 1.0;
    }
    if scenario == "join" {
        steers[1].turn = -1.0;
    }
    for _ in 0..steps {
        if scenario == "cpuwall" || scenario == "cpu-gaps" {
            steers[1] = cpu::steer(&world, 1);
        }
        world.step(0.016, &steers);
    }
    println!("{{\"round\":{},\"over\":{},\"round_wait\":{},\"snakes\":[{}]}}", world.round, world.over, world.round_wait,
        world.snakes.iter().map(|s| format!(
            "{{\"x\":{},\"y\":{},\"angle\":{},\"length\":{},\"alive\":{},\"lives\":{},\"score\":{},\"human\":{},\"gaps\":{}}}",
            s.head.x, s.head.y, s.heading, s.path.len(), s.alive, s.lives, s.score, s.human, s.holes.len()
        )).collect::<Vec<_>>().join(","));
}
