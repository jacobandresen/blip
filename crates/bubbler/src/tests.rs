use super::*;

fn cell(x: f32, y: f32) -> (usize, usize) {
    (((y - HUD) / TILE) as usize, (x / TILE) as usize)
}

#[test]
fn every_round_is_a_full_grid_with_both_players_and_something_to_trap() {
    for (round, rows) in LEVELS.iter().enumerate() {
        blip::testing::assert_grid(&format!("round {}", round + 1), rows, COLS, ROWS);
        let text = rows.concat();
        assert_eq!(text.matches('A').count(), 1, "round {}: player one's start", round + 1);
        assert_eq!(text.matches('B').count(), 1, "round {}: player two's start", round + 1);
        assert!(!level_spawns(round).2.is_empty(), "round {}: no monsters", round + 1);
    }
}

#[test]
fn nobody_starts_inside_a_wall() {
    for (round, rows) in LEVELS.iter().enumerate() {
        let solid = |(r, c): (usize, usize)| rows[r].as_bytes()[c] == b'#';
        let (a, b, monsters) = level_spawns(round);
        for (x, y) in [a, b].into_iter().chain(monsters.iter().map(|&(_, x, y)| (x, y))) {
            assert!(!solid(cell(x + 1.0, y + 1.0)), "round {}: a start at ({x}, {y}) is in a wall", round + 1);
        }
    }
}

#[test]
fn no_monster_starts_on_top_of_a_player() {
    for round in 0..ROUNDS {
        let (a, b, monsters) = level_spawns(round);
        for &(_, x, y) in &monsters {
            for p in [a, b] {
                assert!(
                    (x - p.0).hypot(y - p.1) > 3.0 * TILE,
                    "round {}: a monster starts within three tiles of a player",
                    round + 1
                );
            }
        }
    }
}

#[test]
fn the_pressure_rises_then_holds_from_the_ramps_top() {
    blip::testing::never_falls("monster pace", 0..=RAMP_TOP, monster_pace);
    blip::testing::never_rises("trap life", 0..=RAMP_TOP, trap_life);
    for round in RAMP_TOP..ROUNDS {
        assert_eq!(monster_pace(round), monster_pace(RAMP_TOP));
        assert_eq!(trap_life(round), trap_life(RAMP_TOP));
    }
    assert!(trap_life(ROUNDS) >= 7.0, "a trapped monster stays caught long enough to pop");
}

#[test]
fn hurry_up_always_comes_before_the_skull() {
    for round in 0..ROUNDS {
        assert!(hurry_at(round) < skull_at(round), "round {}", round + 1);
    }
}

#[test]
fn the_game_goes_on_while_a_joined_player_has_lives_or_is_falling() {
    let mut g = Game::new();
    g.p[0].joined = true;
    g.p[1].joined = false;
    g.p[0].lives = 1;
    assert!(players_left(&g));
    g.p[0].lives = 0;
    g.p[0].dead_t = 0.5;
    assert!(players_left(&g), "the last life is still tumbling");
    g.p[0].dead_t = 0.0;
    assert!(!players_left(&g));
}
