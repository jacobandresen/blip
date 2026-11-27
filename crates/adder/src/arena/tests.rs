use super::*;

fn bare() -> World {
    World::empty(1)
}

fn wall() -> Snake {
    Snake::new(1, false, Pt::new(100.0, 270.0), 0.0)
}

#[test]
fn body_collisions_follow_the_thin_width() {
    let mut w = bare();
    w.snakes.push(wall());
    assert_eq!(w.first_hit(0, Pt::new(76.0, 273.0), Pt::new(88.0, 273.0)), Hit::None);
    assert!(matches!(w.first_hit(0, Pt::new(76.0, 271.0), Pt::new(88.0, 271.0)), Hit::Body(0)));
}

#[test]
fn a_body_is_passable_only_at_its_gap() {
    let mut w = bare();
    w.snakes.push(wall());
    let from = Pt::new(84.0, 250.0);
    let to = Pt::new(84.0, 280.0);
    assert_eq!(w.first_hit(0, from, to), Hit::Body(0));
    w.snakes[0].holes.push(Hole { at: 5, len: 7, spent: false });
    assert_eq!(w.first_hit(0, from, to), Hit::Gap(0, 0));
}

#[test]
fn a_solid_trail_is_not_masked_by_an_overlapping_gap() {
    let mut w = bare();
    w.snakes.push(wall());
    w.snakes[0].holes.push(Hole { at: 5, len: 7, spent: false });
    w.snakes.push(wall());
    assert_eq!(w.first_hit(0, Pt::new(84.0, 250.0), Pt::new(84.0, 280.0)), Hit::Body(1));
}

#[test]
fn a_gap_pays_once_but_stays_passable() {
    let mut w = bare();
    w.snakes.push(wall());
    w.snakes[0].holes.push(Hole { at: 5, len: 7, spent: false });
    w.snakes.push(Snake::new(0, true, Pt::new(84.0, 250.0), 0.0));
    w.go_through(1, 0, 0, Pt::new(84.0, 270.0));
    w.go_through(1, 0, 0, Pt::new(84.0, 270.0));
    assert_eq!(w.snakes[1].score, HOLE_POINTS);
    assert_eq!(w.events, vec![Event::Through]);
    assert_eq!(w.first_hit(1, Pt::new(84.0, 260.0), Pt::new(84.0, 280.0)), Hit::Gap(0, 0));
    w.tick_hole(0, 10.0);
    assert!(w.snakes[0].in_hole(8));
}

#[test]
fn moving_keeps_the_tail_and_grows_without_food() {
    let mut w = bare();
    w.snakes.push(Snake::new(0, true, Pt::new(200.0, 200.0), 0.0));
    let tail = w.snakes[0].path.back().copied();
    for _ in 0..60 {
        w.step(1.0 / 60.0, &[Steer::default(); SEATS]);
    }
    assert!(w.snakes[0].alive);
    assert!(w.snakes[0].path.len() >= START_S + 42);
    assert_eq!(w.snakes[0].len_s, w.snakes[0].path.len());
    assert_eq!(w.snakes[0].path.back().copied(), tail);
}

#[test]
fn growth_samples_the_whole_step_not_just_its_endpoint() {
    let mut s = wall();
    s.grow(Pt::new(100.0, 270.0), Pt::new(110.0, 270.0));
    let xs: Vec<_> = s.path.iter().take(6).map(|p| p.x).collect();
    assert_eq!(xs, vec![110.0, 108.0, 106.0, 104.0, 102.0, 100.0]);
}

#[test]
fn growth_keeps_gaps_at_their_original_position() {
    let mut s = wall();
    s.holes.push(Hole { at: 5, len: 7, spent: false });
    let centre = s.hole_centre();
    s.grow(Pt::new(100.0, 270.0), Pt::new(120.0, 270.0));
    assert_eq!(s.hole_centre(), centre);
    assert_eq!(s.holes[0].at, 15);
}

#[test]
fn growth_opens_multiple_gaps_without_closing_old_ones() {
    let mut w = bare();
    w.snakes.push(wall());
    w.tick_hole(0, HOLE_WAIT);
    w.snakes[0].grow(Pt::new(100.0, 270.0), Pt::new(150.0, 270.0));
    let centre = w.snakes[0].hole_centre();
    w.tick_hole(0, HOLE_WAIT);
    w.snakes[0].grow(Pt::new(150.0, 270.0), Pt::new(200.0, 270.0));
    assert_eq!(w.snakes[0].holes.len(), 2);
    assert_eq!(w.snakes[0].hole_centre(), centre);
}

#[test]
fn gaps_are_laid_down_gradually_without_changing_existing_trail() {
    let mut w = bare();
    w.snakes.push(wall());
    let old_path = w.snakes[0].path.clone();
    w.tick_hole(0, HOLE_WAIT);
    assert!(w.snakes[0].holes.is_empty());
    assert_eq!(w.snakes[0].path, old_path);

    w.snakes[0].grow(Pt::new(100.0, 270.0), Pt::new(104.0, 270.0));
    assert_eq!(w.snakes[0].holes[0].at, 0);
    assert_eq!(w.snakes[0].holes[0].len, 2);
    assert!(w.snakes[0].in_hole(0) && w.snakes[0].in_hole(1));
    assert!(!w.snakes[0].in_hole(2));

    w.snakes[0].grow(Pt::new(104.0, 270.0), Pt::new(124.0, 270.0));
    assert_eq!(w.snakes[0].holes[0].at, 2);
    assert_eq!(w.snakes[0].holes[0].len, HOLE_S);
    assert!(!w.snakes[0].in_hole(0));
    assert!(w.snakes[0].in_hole(2));
    assert!(!w.snakes[0].in_hole(12));
    assert_eq!(w.snakes[0].path.iter().skip(12).copied().collect::<VecDeque<_>>(), old_path);
    for i in 12..w.snakes[0].path.len() {
        assert!(!w.snakes[0].in_hole(i));
    }
}

#[test]
fn all_seats_and_rounds_move_at_the_same_speed() {
    let mut w = bare();
    w.snakes.push(Snake::new(0, true, Pt::new(200.0, 200.0), 0.0));
    w.snakes.push(Snake::new(1, false, Pt::new(200.0, 400.0), 0.0));
    w.round = 12;
    w.step(0.1, &[Steer::default(); SEATS]);
    assert_eq!(w.speed_base(), GLIDE);
    assert!((w.snakes[0].head.x - 208.5).abs() < 0.001);
    assert!((w.snakes[1].head.x - 208.5).abs() < 0.001);
}

#[test]
fn an_eliminated_worm_keeps_its_trail_and_never_revives_in_round() {
    let mut w = bare();
    w.snakes.push(Snake::new(0, true, Pt::new(200.0, 200.0), 0.0));
    w.snakes.push(wall());
    w.snakes.push(Snake::new(2, false, Pt::new(300.0, 400.0), 0.0));
    let path = w.snakes[0].path.clone();
    w.eliminate(0, None);
    w.eliminate(0, None);
    w.step(0.1, &[Steer::default(); SEATS]);
    assert!(!w.snakes[0].alive);
    assert_eq!(w.snakes[0].lives, LIVES - 1);
    assert_eq!(w.snakes[0].path, path);
    assert_eq!(w.round, 1);
    assert_eq!(w.first_hit(1, Pt::new(184.0, 190.0), Pt::new(184.0, 210.0)), Hit::Body(0));
}

#[test]
fn last_alive_wins_then_the_next_round_clears_trails_and_preserves_lives() {
    let mut w = World::new(1);
    let score = w.snakes[1].score;
    w.eliminate(0, None);
    w.step(0.0, &[Steer::default(); SEATS]);
    assert_eq!(w.round, 1);
    assert_eq!(w.round_wait, ROUND_WAIT);
    assert_eq!(w.snakes[1].score, score + ROUND_POINTS);
    assert!(!w.snakes[0].alive);
    let head = w.snakes[1].head;
    w.step(ROUND_WAIT / 2.0, &[Steer::default(); SEATS]);
    assert_eq!(w.snakes[1].head, head);
    w.step(ROUND_WAIT / 2.0, &[Steer::default(); SEATS]);
    assert_eq!(w.round, 2);
    assert_eq!(w.snakes.len(), 3);
    assert!(w.snakes[0].alive);
    assert_eq!(w.snakes[0].lives, LIVES - 1);
    assert!(w.snakes.iter().all(|s| s.path.len() == START_S && s.holes.is_empty()));
}

#[test]
fn a_person_who_outlasts_a_cpu_keeps_the_life() {
    let mut w = World::new(1);
    w.join(2, false);
    w.eliminate(1, None);
    w.eliminate(0, None);
    assert_eq!(w.snakes[0].lives, LIVES, "a CPU went first");
    let mut w = World::new(1);
    w.join(2, false);
    w.eliminate(0, None);
    assert_eq!(w.snakes[0].lives, LIVES - 1, "first out");
}

#[test]
fn the_round_ends_when_the_people_are_down() {
    let mut w = World::new(1);
    w.join(2, false);
    w.eliminate(0, None);
    w.step(0.0, &[Steer::default(); SEATS]);
    assert_eq!(w.round_wait, ROUND_WAIT);
    assert!(w.snakes[1].alive && w.snakes[2].alive);
    assert!(w.snakes.iter().all(|s| s.score == 0), "no winner while two are still alive");
}

#[test]
fn no_human_lives_left_ends_the_run() {
    let mut w = World::new(1);
    w.snakes[0].lives = 1;
    w.eliminate(0, None);
    w.step(0.0, &[Steer::default(); SEATS]);
    assert!(w.over);
    assert_eq!(w.round, 1);
}

#[test]
fn player_two_can_join_without_reviving_an_eliminated_seat() {
    let mut w = World::new(1);
    w.join(2, false);
    w.eliminate(1, None);
    assert!(w.join_human(1));
    assert_eq!(w.humans, 2);
    assert_eq!(w.snakes[1].lives, LIVES);
    assert!(!w.snakes[1].alive);
    assert!(!w.join_human(1));
}

#[test]
fn two_humans_steer_their_own_vipers() {
    let mut w = World::new(2);
    let before = [w.snakes[0].heading, w.snakes[1].heading];
    let mut steers = [Steer::default(); SEATS];
    steers[0].turn = -1.0;
    steers[1].turn = 1.0;

    w.step(0.1, &steers);

    assert!((w.snakes[0].heading - (before[0] - TURN * 0.1)).abs() < 1e-5);
    assert!((w.snakes[1].heading - (before[1] + TURN * 0.1)).abs() < 1e-5);
    assert!(w.snakes[0].human && w.snakes[1].human);
    assert!(!w.snakes[2].human);
}

#[test]
fn a_brain_avoids_a_dead_body_in_front_of_it() {
    let mut w = bare();
    w.snakes.push(Snake::new(0, false, Pt::new(120.0, 270.0), 0.0));
    let mut square = Snake::new(1, false, Pt::new(150.0, 230.0), -std::f32::consts::FRAC_PI_2);
    square.len_s = 46;
    square.lay_path();
    square.alive = false;
    w.snakes.push(square);
    assert!(crate::cpu::steer_cpu(&w, 0).turn.abs() > 0.0);
}

#[test]
fn a_pit_of_cpus_can_play_multiple_rounds() {
    let mut w = World::demo(3);
    for _ in 0..3600 {
        let mut steers = [Steer::default(); SEATS];
        for (i, s) in w.snakes.iter().enumerate() {
            if s.alive {
                steers[s.seat] = crate::cpu::steer_cpu(&w, i);
            }
        }
        w.step(1.0 / 60.0, &steers);
        w.events.clear();
    }
    assert!(!w.over);
    assert!(w.round > 1);
    assert!(w.snakes.len() <= MAX_SNAKES);
    assert!(w.snakes.iter().all(|s| s.len_s == s.path.len()));
}
