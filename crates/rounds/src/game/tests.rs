use super::*;

/// A game in play, its random numbers seeded and held for the test (`blip::testing::seeded`).
fn playing() -> (std::sync::MutexGuard<'static, ()>, Game) {
    let rng = blip::testing::seeded(1);
    let mut g = Game::new();
    g.start(false);
    g.state = State::Play;
    (rng, g)
}

/// The first subscriber's house on `side`.
fn first_sub(g: &Game, side: Side) -> usize {
    (0..ROWS).find(|&r| g.houses[side.index()][r].sub).unwrap()
}

#[test]
fn a_continue_rides_the_same_day_again_with_fresh_lives_and_keeps_the_runs_best() {
    let (_rng, mut g) = playing();
    g.day = 3;
    g.begin_day();
    g.sess.score = 500;
    g.sess.lives = 0;
    g.end_game();
    assert!(g.offering(), "homes remain, so a continue is offered");
    g.continue_day();
    assert!(g.state == State::Card, "the day starts again from its card");
    assert_eq!((g.day, g.sess.lives, g.sess.score, g.sess.run_best()), (3, LIVES, 0, 500));
}

#[test]
fn a_close_throw_reaches_the_wall_and_a_far_one_lands_short() {
    let near = flight(200.0, 0.0, CRUISE, Side::Left, 0.0, false);
    assert!(near.wall && near.x == WALL[0]);
    let far = flight(300.0, 0.0, CRUISE, Side::Left, 0.0, false);
    assert!(!far.wall && (far.x - 150.0).abs() < 0.01);
    let right = flight(280.0, 0.0, CRUISE, Side::Right, 0.0, false);
    assert!(right.wall && right.x == WALL[1]);
}

#[test]
fn a_paper_through_the_slot_scores_a_letterbox_and_one_beside_it_the_doorstep() {
    let lot = Some(Lot::House { drive: false });
    let at = |d| Flight { x: WALL[0], d, wall: true, slid: false, t: 0.3 };
    assert_eq!(outcome(Side::Left, at(door_d(3)), lot).0, Outcome::Letterbox);
    assert_eq!(outcome(Side::Left, at(door_d(3) + 15.0), lot).0, Outcome::Doorstep);
    assert_eq!(outcome(Side::Left, at(door_d(3) + 40.0), lot).0, Outcome::Garden);
    let slid = Flight { slid: true, ..at(door_d(3)) };
    assert_eq!(outcome(Side::Left, slid, lot).0, Outcome::Doorstep, "a paper that slid there cannot go through the slot");
    let sample = Flight { x: BOX_X[1], d: door_d(5) + BOX_D, wall: false, slid: false, t: FLIGHT };
    assert_eq!(outcome(Side::Right, sample, lot).0, Outcome::Sample);
}

#[test]
fn every_target_can_be_reached_from_the_street() {
    // From somewhere between the kerbs a throw reaches the slot, the doorstep and the sample box, both sides,
    // in every weather.
    for (wind, rain) in [(0.0, false), (-60.0, false), (0.0, true)] {
        for side in Side::BOTH {
            let mut seen = [false; 3];
            let mut x = RIDE[0];
            while x <= RIDE[1] {
                let f = flight(x, door_d(4) - 200.0, CRUISE, side, wind, rain);
                let f = Flight { d: door_d(4) + if f.wall { 0.0 } else { BOX_D }, ..f };
                match outcome(side, f, Some(Lot::House { drive: false })).0 {
                    Outcome::Letterbox => seen[0] = true,
                    Outcome::Sample => seen[2] = true,
                    _ => {}
                }
                let step = Flight { d: door_d(4), ..f };
                if outcome(side, step, Some(Lot::House { drive: false })).0 == Outcome::Doorstep { seen[1] = true; }
                x += 1.0;
            }
            assert_eq!(seen, [true, true, true], "{side:?} wind {wind} rain {rain}: {seen:?}");
        }
    }
}

#[test]
fn delivering_scores_once_and_a_letterbox_mends_a_heart() {
    let (_rng, mut g) = playing();
    let r = first_sub(&g, Side::Left);
    g.houses[0][r].hearts = 1;
    let f = Flight { x: WALL[0], d: door_d(r), wall: true, slid: false, t: 0.3 };
    for _ in 0..2 {
        g.papers.push(Paper { x: 200.0, d: 0.0, vx: -THROW_V, vd: 0.0, t: 0.0, side: Side::Left, by: 0, f, landed: None });
        let i = g.papers.len() - 1;
        g.land(i, f);
    }
    assert_eq!(g.sess.score, LETTERBOX_PTS);
    assert_eq!(g.houses[0][r].hearts, 2);
}

#[test]
fn two_missed_mornings_lose_a_subscriber() {
    let (_rng, mut g) = playing();
    let r = first_sub(&g, Side::Right);
    let before = g.subscribers();
    for _ in 0..2 {
        g.houses[1][r].judged = false;
        g.houses[1][r].done = false;
        g.bike_d = door_d(r) + 201.0;
        g.judge_houses();
    }
    assert!(!g.houses[1][r].sub);
    assert_eq!(g.subscribers(), before - 1);
}

#[test]
fn a_sample_can_win_a_new_subscriber_by_the_next_day() {
    let (_rng, mut g) = playing();
    let r = (0..ROWS).find(|&r| g.houses[0][r].lot.is_house() && !g.houses[0][r].sub).unwrap();
    let mut won = 0;
    for seed in 0..20 {
        blip::rand_seed(seed);
        g.houses[0][r].sub = false;
        g.houses[0][r].sampled = true;
        g.tally = Tally::default();
        g.end_day();
        won += g.tally.won;
        g.state = State::Play;
    }
    assert!(won > 5 && won < 20, "{won}/20 samples won a subscriber");
}

#[test]
fn the_canal_pays_more_for_a_longer_throw() {
    assert_eq!(Game::target_points(QUAY_MID + 95.0), 50);
    assert_eq!(Game::target_points(QUAY_MID - 190.0), 500);
    assert!(Game::target_points(40.0) > Game::target_points(90.0));
}

#[test]
fn every_nyhavn_target_can_be_reached_from_the_quay() {
    let (_rng, mut g) = playing();
    g.start_canal();
    let lim = g.ride_limits();
    for t in &g.targets {
        let x = t.x - t.side().sign() * THROW_V * FLIGHT;
        assert!(x >= lim[0] - 0.5 && x <= lim[1] + 0.5, "{:?} at {} needs the bike at {x}", t.mark, t.x);
    }
}

#[test]
fn steering_left_in_the_junction_turns_the_bike_into_the_new_street() {
    let (_rng, mut g) = playing();
    g.bike_d = JD - MOUTH_HALF - 60.0;
    g.bike_x = 200.0;
    for _ in 0..(60 * 20) {
        if g.state != State::Play { g.state = State::Play; }
        let steer = if g.in_junction() && g.bike_d > JD - 30.0 && g.heading < 1.5 { -1.0 } else { 0.0 };
        g.ride(steer, 0.0, 1.0 / 60.0);
        g.update_play(1.0 / 60.0);
        g.safe = 1.0;
        if g.turned { break; }
    }
    assert!(g.turned, "never round the corner: x {} d {} heading {}", g.bike_x, g.bike_d, g.heading);
    assert!(g.bike_d >= NEW_STREET_ROW as f32 * LOT - 30.0 && (RIDE[0]..=RIDE[1]).contains(&g.bike_x), "on the new street at {} {}", g.bike_x, g.bike_d);
}

#[test]
fn a_crash_at_the_statue_does_not_put_the_bike_back_on_it() {
    let (_rng, mut g) = playing();
    g.bike_d = statue_d() - STATUE_HALF;
    g.bike_x = 240.0;
    g.update_play(1.0 / 60.0);
    assert_eq!(g.state, State::Crash, "rode into the pedestal");
    while g.state == State::Crash { g.update_crash(1.0 / 60.0); }
    for _ in 0..60 {
        g.ride(0.0, 0.0, 1.0 / 60.0);
        g.update_play(1.0 / 60.0);
        assert_eq!(g.state, State::Play, "crashed again straight after getting back on, at {} {}", g.bike_x, g.bike_d);
    }
}

#[test]
fn a_rider_who_never_steers_still_gets_round_the_corner() {
    for pedal in [0.0, 1.0] {
        let (_rng, mut g) = playing();
        g.bike_d = JD - MOUTH_HALF - 60.0;
        g.bike_x = 200.0;
        for _ in 0..(60 * 30) {
            if g.state != State::Play { g.state = State::Play; }
            g.ride(0.0, pedal, 1.0 / 60.0);
            g.update_play(1.0 / 60.0);
            g.safe = 1.0;
            if g.turned { break; }
        }
        assert!(g.turned, "pedal {pedal}: stalled at x {} d {} heading {}", g.bike_x, g.bike_d, g.heading);
    }
}

/// Every day of the week ridden for `secs`, the bike kept safe and steered round the turn; `look` sees each frame.
fn ride_every_day(secs: usize, mut look: impl FnMut(&Game, usize, usize)) {
    for day in 0..7 {
        let (_rng, mut g) = playing();
        g.day = day;
        g.begin_day();
        g.state = State::Play;
        for frame in 0..(60 * secs) {
            g.state = State::Play;
            let steer = if g.in_junction() && g.bike_d > JD - 30.0 && g.heading < 1.5 { -1.0 } else { 0.0 };
            g.ride(steer, 0.0, 1.0 / 60.0);
            g.safe = 5.0;
            g.update_play(1.0 / 60.0);
            if g.state != State::Play { break; }
            look(&g, day, frame);
        }
    }
}

#[test]
fn buses_and_cars_never_drive_into_each_other() {
    ride_every_day(50, |g, day, frame| {
        let cars: Vec<&Hazard> = g.hazards.iter().filter(|h| !h.gone && h.solid() && h.vehicle()).collect();
        for (i, a) in cars.iter().enumerate() {
            for b in &cars[i + 1..] {
                if a.seg != b.seg { continue; }
                let ((aw, al), (bw, bl)) = (a.size(), b.size());
                let over = (a.x - b.x).abs() < aw + bw - 3.0 && (a.d - b.d).abs() < al + bl - 3.0;
                assert!(!over, "day {day} frame {frame}: {:?} p{} held {} at ({:.0},{:.0}) runs into {:?} p{} held {} at ({:.0},{:.0})",
                    a.kind, a.phase, a.held, a.x, a.d, b.kind, b.phase, b.held, b.x, b.d);
            }
        }
    });
}

#[test]
fn riders_never_ride_through_cars_dogs_or_buses() {
    let mut found = Vec::new();
    ride_every_day(100, |g, day, frame| {
        let riders: Vec<&Hazard> = g.hazards.iter().filter(|h| !h.gone && h.rider()).collect();
        let others: Vec<&Hazard> = g.hazards.iter().filter(|h| !h.gone && h.solid() && (h.vehicle() || h.kind == Hz::Dog)).collect();
        for a in &riders {
            for b in &others {
                if a.seg != b.seg { continue; }
                let ((aw, al), (bw, bl)) = (a.size(), b.size());
                if (a.x - b.x).abs() < aw + bw - 3.0 && (a.d - b.d).abs() < al + bl - 3.0 && found.len() < 12 {
                    found.push(format!("day {day} frame {frame}: {:?} p{} held {} at ({:.0},{:.0}) through {:?} p{} at ({:.0},{:.0})",
                        a.kind, a.phase, a.held, a.x, a.d, b.kind, b.phase, b.x, b.d));
                }
            }
        }
    });
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn at_the_crossing_nothing_drives_into_anything() {
    // the bike waits short of the junction while the lights go round several times
    let (_rng, mut g) = playing();
    g.bike_d = Game::stop_d(JUNCTIONS[0]) - 60.0;
    for frame in 0..(60 * 60) {
        g.state = State::Play;
        g.speed = 0.0;
        g.safe = 5.0;
        g.update_play(1.0 / 60.0);
        let cars: Vec<&Hazard> = g.hazards.iter().filter(|h| !h.gone && h.solid() && h.vehicle()).collect();
        for (i, a) in cars.iter().enumerate() {
            for b in &cars[i + 1..] {
                let ((aw, al), (bw, bl)) = (a.size(), b.size());
                let over = (a.x - b.x).abs() < aw + bw - 2.0 && (a.d - b.d).abs() < al + bl - 2.0;
                assert!(!over, "frame {frame}: {:?} at ({:.0},{:.0}) in {:?} at ({:.0},{:.0})", a.kind, a.x, a.d, b.kind, b.x, b.d);
            }
        }
    }
    assert!(g.hazards.iter().any(|h| h.kind == Hz::CrossCar), "no side-street traffic came");
}

#[test]
fn going_straight_on_stops_at_the_roadworks() {
    let (_rng, mut g) = playing();
    g.bike_d = JD - MOUTH_HALF - 60.0;
    g.bike_x = 280.0;
    for _ in 0..(60 * 10) {
        g.state = State::Play;
        g.ride(0.0, 1.0, 1.0 / 60.0);
        g.safe = 1.0;
    }
    assert!(!g.turned && g.bike_d < WORKS_D, "rode through the barrier to {}", g.bike_d);
}

#[test]
fn the_two_streets_meet_where_they_should() {
    let (x, d) = new_street_to_world(240.0, JD + 110.0);
    assert!((x - 130.0).abs() < 0.01 && (d - JD).abs() < 0.01);
    let (bx, bd) = world_to_new_street(x, d);
    assert!((bx - 240.0).abs() < 0.01 && (bd - JD - 110.0).abs() < 0.01);
}

#[test]
fn a_week_of_days_comes_round_to_monday() {
    let (_rng, mut g) = playing();
    for day in 0..7 {
        assert_eq!(g.day, day);
        g.end_day();
        g.start_canal();
        g.timer.start(0.0001);
        g.update_canal(0.01);
    }
    assert_eq!(g.state, State::Week);
    g.update_week(5.0, false);
    assert_eq!((g.week, g.day, g.state), (2, 0, State::Card));
}
