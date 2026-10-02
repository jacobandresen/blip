//! The solo run: the ladder, winning and losing, continues and the bonus rounds.

use super::*;

// ---- the ladder ----------------------------------------------------------

#[test]
pub(crate) fn the_ladder_is_everybody_else_once() {
    let boss = FIGHTERS.iter().position(|a| a.invincible).unwrap();
    for pick in 0..FIGHTERS.len() {
        let ladder = Game { pick, ..Game::new() }.ladder();
        assert_eq!(ladder.len(), FIGHTERS.len() - 1);
        let mut met = ladder.to_vec();
        met.sort();
        met.dedup();
        assert_eq!(met.len(), ladder.len(), "{} meets somebody twice", FIGHTERS[pick].name);
        assert!(!ladder.contains(&pick), "{} fights themself", FIGHTERS[pick].name);
        // The one nothing hurts is the last fight, never an earlier one, and
        // the giant is the one before: nobody opens against either.
        let giant = FIGHTERS.iter().position(|a| a.build == Build::Giant).unwrap();
        let ends: Vec<usize> = [giant, boss].into_iter().filter(|&w| w != pick).collect();
        assert_eq!(&ladder[RUNGS - ends.len()..], &ends[..],
            "{}'s ladder does not end with the giant and the boss", FIGHTERS[pick].name);
    }
}

#[test]
pub(crate) fn every_fight_is_at_the_opponents_home_and_every_stage_is_somebodys() {
    // No ladder has two fights running in one place, whoever climbs it.
    for pick in 0..FIGHTERS.len() {
        let mut g = Game { pick, ..Game::new() };
        let ladder = g.ladder();
        let mut seen = vec![];
        for (rung, foe) in ladder.into_iter().enumerate() {
            g.start_match(rung);
            assert_eq!(g.stage, home_of(foe), "{} is not fought at home", FIGHTERS[foe].name);
            assert!(g.stage < STAGES);
            seen.push(g.stage);
        }
        assert!(seen.windows(2).all(|w| w[0] != w[1]),
            "{} fights twice running in one place: {seen:?}", FIGHTERS[pick].name);
    }
    for stage in 0..STAGES {
        assert!((0..FIGHTERS.len()).any(|who| home_of(who) == stage),
            "nobody lives at {}", STAGE_NAMES[stage]);
    }
    // Two players meet at player two's.
    let mut g = Game { pick: 0, pick2: 8, ..Game::new() };
    g.start_versus();
    assert_eq!(g.stage, home_of(8));
}

#[test]
pub(crate) fn each_opponent_is_fought_somewhere_else() {
    let mut g = Game::new();
    g.pick = 0;
    g.start_match(0);
    let first = g.stage;
    g.start_match(1);
    assert_ne!(first, g.stage, "both opponents were fought on the same stage");
}

#[test]
pub(crate) fn the_second_opponent_is_harder_than_the_first() {
    let mut g = Game::new();
    g.pick = 0;
    g.start_match(0);
    let first = g.difficulty;
    g.start_match(1);
    assert!(g.difficulty > first, "the ladder does not climb");
}

#[test]
pub(crate) fn a_round_starts_both_fighters_whole_and_apart() {
    let mut g = Game::new();
    g.pick = 2;
    g.start_match(0);
    assert_eq!(g.p[0].health, FIGHTERS[g.p[0].who].health);
    assert_eq!(g.p[1].health, FIGHTERS[g.p[1].who].health);
    assert!((g.p[1].x - g.p[0].x).abs() > 150.0, "the round starts inside each other");
    assert!(g.p[0].facing > 0.0 && g.p[1].facing < 0.0, "the fighters do not face each other");
}

#[test]
pub(crate) fn round_wins_survive_the_round_that_earned_them() {
    // start_round() rebuilds both fighters; the score of the match is
    // the one thing that must not be rebuilt with them.
    let mut g = Game::new();
    g.pick = 0;
    g.start_match(0);
    g.p[0].rounds = 1;
    g.start_round();
    assert_eq!(g.p[0].rounds, 1, "a round win was lost between rounds");
    assert_eq!(g.p[0].health, FIGHTERS[g.p[0].who].health, "health did not reset");
}

// ---- winning and losing --------------------------------------------------

/// Push a phase timer past its end the way the frame loop would.
pub(crate) fn run_phase(g: &mut Game, step: impl Fn(&mut Game, f32)) {
    for _ in 0..600 {
        let before = g.state;
        step(g, F);
        if g.state != before { return; }
    }
    panic!("a phase never ended: still in a state after 10 seconds");
}

#[test]
pub(crate) fn beating_the_first_opponent_moves_you_to_the_second_somewhere_else() {
    // The ladder is the whole single-player structure, and it is the one
    // part a player only reaches by being good — so it is the part least
    // likely to be exercised by hand before shipping.
    let mut g = Game::new();
    g.pick = 0;
    g.start_match(0);
    let first_foe = g.p[1].who;
    let first_stage = g.stage;

    g.p[0].rounds = ROUNDS_TO_WIN;
    g.state = State::MatchEnd;
    g.phase.start(0.1);
    run_phase(&mut g, update_match_end);

    assert_eq!(g.state, State::Vs, "the next match was not billed");
    assert_ne!(g.p[1].who, first_foe, "the same opponent came back for a second match");
    assert_ne!(g.stage, first_stage, "the second match is in the same place as the first");
    assert_eq!(g.p[0].rounds, 0, "round wins carried over into the next opponent");
    assert_eq!(g.p[1].rounds, 0);
    assert_eq!(g.p[0].health, FIGHTERS[g.p[0].who].health, "the next match started hurt");
}

#[test]
pub(crate) fn beating_every_opponent_wins_the_game() {
    let mut g = Game::new();
    g.pick = 1;
    g.start_match(RUNGS - 1); // the last rung
    g.p[0].rounds = ROUNDS_TO_WIN;
    g.state = State::MatchEnd;
    g.phase.start(0.1);
    run_phase(&mut g, update_match_end);
    assert_eq!(g.state, State::Won, "clearing the ladder did not end the game");
}

#[test]
pub(crate) fn losing_a_match_ends_the_run() {
    let mut g = Game::new();
    g.pick = 2;
    g.start_match(0);
    g.p[1].rounds = ROUNDS_TO_WIN;
    g.state = State::MatchEnd;
    g.phase.start(0.1);
    run_phase(&mut g, update_match_end);
    assert_eq!(g.state, State::Continue, "a beaten player was not offered the fight again");
    // Nobody presses: the count runs out and the run is over.
    for _ in 0..(CONTINUE_SECS / F) as usize + 2 { update_continue(&mut g, F, false); }
    assert_eq!(g.state, State::Over, "losing did not end the run");
}

#[test]
pub(crate) fn a_continue_is_the_same_fight_again_from_nothing() {
    let mut g = Game::new();
    g.pick = 2;
    g.start_match(4);
    let foe = g.p[1].who;
    g.sess.add_score(7000);
    g.p[1].rounds = ROUNDS_TO_WIN;
    g.state = State::MatchEnd;
    g.phase.start(0.1);
    run_phase(&mut g, update_match_end);
    // The press that was still down from the fight is not an answer.
    update_continue(&mut g, F, true);
    assert_eq!(g.state, State::Continue);
    for _ in 0..60 { update_continue(&mut g, F, false); }
    update_continue(&mut g, F, true);
    assert_eq!(g.state, State::Vs);
    assert_eq!((g.opponent_index, g.p[1].who), (4, foe), "the ladder moved");
    assert_eq!(g.sess.score, 0, "the score was carried through a continue");
    assert_eq!((g.p[0].rounds, g.p[1].rounds), (0, 0));
    // And a little easier than it was, though only so far.
    let fair = FIRST_RUNG + (LAST_RUNG - FIRST_RUNG) * 4.0 / (RUNGS - 1) as f32;
    assert!((g.difficulty - (fair - MERCY)).abs() < 1e-4);
    g.mercy = MERCY * 3.0;
    g.state = State::Continue;
    g.phase.start(CONTINUE_SECS);
    for _ in 0..60 { update_continue(&mut g, F, false); }
    update_continue(&mut g, F, true);
    assert!((g.mercy - MERCY * 3.0).abs() < 1e-4, "mercy without end");
}

#[test]
pub(crate) fn a_match_cannot_run_forever() {
    // Every round ends with somebody getting a point, including a double
    // KO — which gives both fighters one, so even a match of nothing but
    // draws terminates at two rounds rather than looping.
    let mut g = Game::new();
    g.pick = 0;
    g.start_match(0);
    for round in 0..8 {
        g.p[0].rounds += 1;
        g.p[1].rounds += 1; // the worst case: every round a draw
        g.state = State::RoundEnd;
        g.phase.start(0.05);
        run_phase(&mut g, update_round_end);
        if g.state == State::MatchEnd { return; }
        assert!(round < 4, "a drawn match kept starting new rounds");
    }
    panic!("the match never reached its end");
}

#[test]
pub(crate) fn a_round_win_is_worth_more_when_you_are_barely_scratched() {
    // Scoring has to reward playing well rather than merely surviving,
    // or the leaderboard measures patience.
    let mut clean = Game::new();
    clean.pick = 0;
    clean.start_match(0);
    clean.sess.add_score(0);
    let before = clean.sess.score;
    clean.p[0].health = FIGHTERS[clean.p[0].who].health;
    clean.sess.add_score(1000 + clean.p[0].health * 20);
    let clean_score = clean.sess.score - before;

    let mut scraped = Game::new();
    scraped.pick = 0;
    scraped.start_match(0);
    scraped.p[0].health = 1;
    let before2 = scraped.sess.score;
    scraped.sess.add_score(1000 + scraped.p[0].health * 20);
    let scraped_score = scraped.sess.score - before2;

    assert!(clean_score > scraped_score * 2,
        "winning untouched scores about the same as scraping through");
}

// ---- the bonus round --------------------------------------------------------

/// A player who has just won fight `rung`.
pub(crate) fn after_winning(rung: usize) -> Game {
    let mut g = Game { pick: 0, ..Game::new() };
    g.start_match(rung);
    g.p[0].rounds = ROUNDS_TO_WIN;
    g.state = State::MatchEnd;
    g.phase.start(0.1);
    run_phase(&mut g, update_match_end);
    g
}

#[test]
pub(crate) fn the_bonus_round_comes_after_the_third_and_sixth_fights_and_nowhere_else() {
    for rung in 0..RUNGS - 1 {
        let g = after_winning(rung);
        let want = if BONUS_AFTER.contains(&rung) { State::Bonus } else { State::Vs };
        assert_eq!(g.state, want, "after fight {}", rung + 1);
    }
}

#[test]
pub(crate) fn barrels_break_under_blows_and_the_ladder_goes_on_after() {
    let mut g = after_winning(BONUS_AFTER[0]);
    assert_eq!(g.bonus_total, 5);
    assert!(g.barrels[..5].iter().all(|b| b.hp == BARREL_HP && !b.rolls));
    let before = g.sess.score;
    // Walk up to each barrel in turn and kick it until it goes.
    let mut frames = 0;
    while g.state == State::Bonus && frames < 60 * 40 {
        frames += 1;
        let mut inp = Input::default();
        let ahead = (g.p[1].x - g.p[0].x) * g.p[0].facing;
        if ahead > attack_range(&g.p[0], MoveId::LowKick) - BODY_W / 2.0 {
            if g.p[0].facing > 0.0 { inp.right = true; } else { inp.left = true; }
        } else {
            inp.kick_low = frames % 4 < 2;
        }
        update_bonus(&mut g, F, inp);
    }
    assert!(g.barrels.iter().all(|b| b.hp <= 0), "barrels left standing: {:?}",
        g.barrels.iter().map(|b| b.hp).collect::<Vec<_>>());
    assert!(g.sess.score >= before + 5000 + 3000, "the barrels and the perfect were not paid");
    assert_eq!(g.state, State::Vs, "the ladder did not go on");
    assert_eq!(g.opponent_index, BONUS_AFTER[0] + 1);
}

#[test]
pub(crate) fn the_bonus_round_ends_on_the_clock_if_nothing_is_broken() {
    let mut g = after_winning(BONUS_AFTER[1]);
    for _ in 0..((BONUS_SECS + BONUS_TALLY) / F) as usize + 10 { update_bonus(&mut g, F, Input::default()); }
    assert_eq!(g.state, State::Vs);
    assert_eq!(g.bonus[0], 0, "a perfect was paid for nothing");
}

#[test]
fn pressing_a_button_on_the_result_screen_moves_on_and_never_strands_the_game() {
    // Whenever the press comes, and however often, the result gives way.
    for press_from in [0usize, 30, 90, 150, 215] {
        for won in [true, false] {
            let mut g = Game { pick: 0, ..Game::new() };
            g.start_match(1);
            g.p[if won { 0 } else { 1 }].rounds = ROUNDS_TO_WIN;
            g.state = State::MatchEnd;
            g.phase.start(MATCH_END);
            let mut frames = 0;
            while g.state == State::MatchEnd && frames < 600 {
                update_result(&mut g, F, frames >= press_from);
                frames += 1;
            }
            assert_ne!(g.state, State::MatchEnd,
                "stuck on the result (won {won}, pressing from frame {press_from})");
            assert_eq!(g.state, if won { State::Vs } else { State::Continue });
            // The press that was still down from the fight does not skip it unread.
            assert!(frames as f32 * F >= 1.1, "the result was gone in {frames} frames");
        }
    }
}
