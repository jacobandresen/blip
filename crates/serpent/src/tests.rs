use super::*;

#[test]
fn the_golden_mouse_runs_from_a_head_that_comes_near() {
    let mut g = running();
    let head = g.snake_at(0);
    g.bonus = Cell { c: head.c + 3, r: head.r };
    g.bonus_ttl = BONUS_TTL;
    let dist = |g: &Game| (g.bonus.c - head.c).abs() + (g.bonus.r - head.r).abs();
    assert!(!scurry(&mut g, MOUSE_HOP * 0.5), "not before its hop is due");
    assert!(scurry(&mut g, MOUSE_HOP));
    assert_eq!(dist(&g), 4, "one cell farther away");
    g.bonus = Cell { c: head.c + MOUSE_SEES + 2, r: head.r };
    let before = g.bonus;
    assert!(!scurry(&mut g, MOUSE_HOP));
    assert_eq!(g.bonus, before, "a far head is not seen");
}

/// A snake set up mid-board, travelling right, with nothing queued.
fn running() -> Game {
    let mut g = Game::new();
    g.reset_snake();
    g
}

/// One move step as update_play() takes it: the turn comes off the queue
/// and becomes the direction.
fn step(g: &mut Game) -> Dir {
    g.cur_dir = g.take_turn();
    g.cur_dir
}

#[test]
fn a_corner_taken_faster_than_one_step_keeps_both_turns() {
    // Down then left inside one step must both happen.
    let mut g = running();
    g.queue_turn(Dir::Down);
    g.queue_turn(Dir::Left);

    assert_eq!(step(&mut g), Dir::Down, "the first turn of the gesture was dropped");
    assert_eq!(step(&mut g), Dir::Left, "the second turn of the gesture was dropped");
    assert_eq!(step(&mut g), Dir::Left, "with nothing queued the snake carries on");
}

#[test]
fn a_queued_turn_cannot_double_back_into_the_neck() {
    // Travelling right, up then down: the second is a reversal of the
    // first and must be refused.
    let mut g = running();
    g.queue_turn(Dir::Up);
    g.queue_turn(Dir::Down);

    assert_eq!(step(&mut g), Dir::Up);
    assert_eq!(step(&mut g), Dir::Up, "a reversal of the queued turn was accepted");
}

#[test]
fn a_reversal_is_still_refused_with_nothing_queued() {
    let mut g = running(); // travelling right
    g.queue_turn(Dir::Left);
    assert_eq!(step(&mut g), Dir::Right, "the snake reversed into itself");
}

#[test]
fn holding_a_direction_does_not_fill_the_queue() {
    // Repeats of the direction already being travelled are not
    // turns, and must not push a real turn out of the queue.
    let mut g = running();
    g.queue_turn(Dir::Right);
    g.queue_turn(Dir::Right);
    g.queue_turn(Dir::Down);
    assert_eq!(step(&mut g), Dir::Down, "a real turn was crowded out by repeats");
}

#[test]
fn the_queue_is_bounded() {
    // Three turns inside one step is no longer a gesture, it is a
    // path the snake has not committed to. The third is dropped
    // rather than remembered.
    let mut g = running();
    g.queue_turn(Dir::Down);
    g.queue_turn(Dir::Left);
    g.queue_turn(Dir::Up);
    assert_eq!(g.turns_len, TURN_QUEUE);
    assert_eq!(step(&mut g), Dir::Down);
    assert_eq!(step(&mut g), Dir::Left);
    assert_eq!(step(&mut g), Dir::Left, "a third turn inside one step was remembered anyway");
}

#[test]
fn a_bonus_never_lands_on_the_snake_the_food_or_an_obstacle() {
    // Any of those is a bonus that cannot be taken: inside the snake
    // it is unreachable, on the food it is taken by accident, in an
    // obstacle it is a reward for dying.
    let mut g = running();
    g.sess.level = 3;          // levels past 1 have an obstacle field
    g.build_obstacles();
    for _ in 0..200 {
        g.bonus_ttl = 0.0;
        g.spawn_bonus();
        assert!(g.bonus_active(), "no bonus was placed on an almost-empty board");
        assert!(g.cell_is_free(g.bonus), "the bonus landed on the snake or an obstacle");
        assert_ne!(g.bonus, g.food, "the bonus landed on the ordinary food");
    }
}

#[test]
fn a_bonus_is_placed_away_from_the_snakes_head() {
    // A bonus that appears under the nose is five foods for no
    // decision, which is the one outcome that makes the rest of the
    // board pointless.
    let mut g = running();
    let head = g.snake_at(0);
    let far_enough = (COLS + ROWS) / 3;
    let mut close = 0;
    for _ in 0..200 {
        g.bonus_ttl = 0.0;
        g.spawn_bonus();
        if (g.bonus.c - head.c).abs() + (g.bonus.r - head.r).abs() < far_enough { close += 1; }
    }
    assert_eq!(close, 0, "{close}/200 bonuses appeared within reach of the head");
}

#[test]
fn ordinary_food_never_replaces_a_bonus_that_is_still_up() {
    // spawn_food() runs on the same step a bonus can be live, and two
    // pieces on one cell is one of them silently eating the other.
    let mut g = running();
    g.spawn_bonus();
    let bonus = g.bonus;
    for _ in 0..200 {
        g.spawn_food();
        assert_ne!(g.food, bonus, "food was placed on top of the live bonus");
    }
}

#[test]
fn a_bonus_does_not_survive_the_life_that_earned_it() {
    let mut g = running();
    g.spawn_bonus();
    assert!(g.bonus_active());
    g.reset_snake();
    assert!(!g.bonus_active(), "the bonus carried over into the next life");
}

#[test]
fn a_new_levels_obstacles_never_land_on_the_snake_or_just_ahead_of_it() {
    // They arrive under a moving snake; one on its body or in its next
    // steps is a death the player never had a chance to see coming.
    for level in 2..=6 {
        let mut g = running();
        g.sess.level = level;
        g.raise_obstacles();
        let head = g.snake_at(0);
        for o in &g.obstacles[..g.obstacle_count] {
            assert!((0..g.snake_len).all(|i| g.snake_at(i) != *o), "level {level}: an obstacle on the snake");
            assert!(!(o.r == head.r && o.c > head.c && o.c <= head.c + 3), "level {level}: an obstacle just ahead");
        }
        assert!(g.obs_fade < 1.0, "the new layout is solid at once");
    }
}

#[test]
fn a_new_snake_starts_with_an_empty_queue() {
    // Turns left over from the life just lost would steer the new
    // snake on its first step, which reads as the game moving on its
    // own before the player has touched anything.
    let mut g = running();
    g.queue_turn(Dir::Down);
    g.reset_snake();
    assert_eq!(g.turns_len, 0);
    assert_eq!(step(&mut g), Dir::Right);
}
