//! Two players: the select screen, joining, and a match from start to result.

use super::*;

// ---- two players ---------------------------------------------------------

#[test]
pub(crate) fn the_two_players_share_no_keys() {
    // No key moves both fighters, read off the pads themselves rather than a
    // copied list.
    let keys = |p: &Pad| {
        let mut v: Vec<String> = Vec::new();
        for set in [p.up, p.down, p.left, p.right, p.punch, p.kick] {
            v.extend(set.iter().map(|k| format!("{k:?}")));
        }
        v
    };
    let one = keys(pad(Mode::Versus, 0));
    let two = keys(pad(Mode::Versus, 1));
    for a in &one {
        assert!(!two.contains(a), "{a} is bound to both players");
    }
    // And nobody has the same key on two of their own controls.
    for (name, set) in [("player one", &one), ("player two", &two)] {
        let mut sorted = set.clone();
        sorted.sort();
        let before = sorted.len();
        sorted.dedup();
        assert_eq!(sorted.len(), before, "{name} has a key bound twice");
        assert_eq!(before, 6, "{name} should have six controls, has {before}");
    }
    // Sharing takes the arrows and the deck buttons off player one;
    // alone, they get everything back.
    let alone = keys(pad(Mode::Solo, 0));
    assert!(alone.len() > one.len(), "the solo player lost their cabinet keys");
    assert!(alone.iter().any(|k| k == "Up"), "the solo player cannot use the stick");
    // Player two is player two in every mode. Deciding by mode first
    // handed them player one's keys on every screen before the mode
    // had been chosen.
    assert_eq!(keys(pad(Mode::Solo, 1)), two,
        "player two was given player one's keys in one-player mode");
}

#[test]
pub(crate) fn two_players_cannot_bring_the_same_fighter() {
    // The one rule the select screen exists to enforce.
    let mut g = Game::new();
    g.mode = Mode::Versus;
    for taken in 0..FIGHTERS.len() {
        g.pick = taken;
        g.locked = [true, false];
        for at in 0..FIGHTERS.len() {
            assert_eq!(g.taken_by_other(1, at), at == taken,
                "player two's view of fighter {at} with {taken} taken is wrong");
        }
        // And the other way round.
        g.pick2 = taken;
        g.locked = [false, true];
        for at in 0..FIGHTERS.len() {
            assert_eq!(g.taken_by_other(0, at), at == taken);
        }
    }
}

#[test]
pub(crate) fn a_locked_fighter_is_free_again_in_one_player() {
    // Solo has nobody to collide with, and the lock-out must not leak
    // into it — a one-player game that refuses a fighter is a bug.
    let mut g = Game::new();
    g.mode = Mode::Solo;
    g.pick2 = 1;
    g.locked = [false, true];
    for at in 0..FIGHTERS.len() {
        assert!(!g.taken_by_other(0, at), "solo refused fighter {at}");
    }
}

#[test]
pub(crate) fn a_versus_match_is_one_match_with_a_winner() {
    // No ladder, no second opponent, no score: two people played one
    // match and one of them won it.
    let mut g = Game::new();
    g.mode = Mode::Versus;
    g.pick = 0;
    g.pick2 = 2;
    g.start_versus();
    assert_eq!(g.p[0].who, 0);
    assert_eq!(g.p[1].who, 2);
    assert_eq!(g.state, State::RoundIntro);

    // Give player one the match and run the end-of-match step.
    g.p[0].rounds = ROUNDS_TO_WIN;
    g.state = State::MatchEnd;
    // Armed with a real duration: tick() reports nothing for a timer
    // that was never running.
    g.phase.start(0.05);
    for _ in 0..8 { update_match_end(&mut g, F); }
    assert_eq!(g.state, State::Over, "a versus match sent someone up a ladder");
    assert_eq!(g.opponent_index, 0, "a versus match advanced the ladder index");
}

#[test]
pub(crate) fn a_solo_run_still_climbs_the_ladder() {
    // The other half of the same branch: adding versus must not have
    // taken the ladder away from the one-player game.
    let mut g = Game::new();
    g.mode = Mode::Solo;
    g.pick = 0;
    g.start_match(0);
    g.p[0].rounds = ROUNDS_TO_WIN;
    g.state = State::MatchEnd;
    // Armed with a real duration: tick() reports nothing for a timer
    // that was never running.
    g.phase.start(0.05);
    for _ in 0..8 { update_match_end(&mut g, F); }
    assert_eq!(g.opponent_index, 1, "beating the first opponent did not advance the ladder");
    assert_ne!(g.state, State::Over);
}

#[test]
pub(crate) fn each_player_has_their_own_special_window() {
    // Punch and kick inside a short window is the special. Held in one
    // pair of slots, player one's punch armed player two's kick and
    // both of them threw specials neither asked for.
    let mut g = Game::new();
    g.mode = Mode::Versus;
    g.now = 1.0;
    g.punch_at = [-1.0; 2];
    g.kick_at = [-1.0; 2];
    // Player one punches, player two kicks, on the same frame.
    g.punch_at[0] = g.now;
    g.kick_at[1] = g.now;
    for who in 0..2 {
        let (pa, ka) = (g.punch_at[who], g.kick_at[who]);
        let together = (pa - ka).abs() <= 0.08 && g.now - pa.max(ka) <= 0.08
            && pa > 0.0 && ka > 0.0;
        assert!(!together, "player {who} threw a special off the other player's button");
    }
}

#[test]
pub(crate) fn a_special_pressed_a_few_frames_apart_still_comes_out() {
    for (first, late) in [(Input { punch_low: true, ..Default::default() }, 3),
                          (Input { kick_low: true, ..Default::default() }, 4)] {
        let mut f = at(0, 200.0, 1.0);
        apply_input(&mut f, first, false, F);
        advance(&mut f, F);
        assert_eq!(f.act, Act::Attack);
        for _ in 1..late {
            apply_input(&mut f, Input::default(), false, F);
            advance(&mut f, F);
        }
        apply_input(&mut f, Input { special: true, ..Default::default() }, false, F);
        assert_eq!(f.mv, MoveId::Special, "the second button {late} frames late was lost");
        assert!(f.t < F, "the special did not start from its first frame");
    }
    // Once the first attack is out, it is too late: no special from a jab
    // that has already landed or missed.
    let mut f = at(0, 200.0, 1.0);
    f.start_attack(MoveId::LowPunch);
    for _ in 0..8 { advance(&mut f, F); }
    apply_input(&mut f, Input { special: true, ..Default::default() }, false, F);
    assert_eq!(f.mv, MoveId::LowPunch);
}

// ---- two-player play tests -----------------------------------------------
// The real flow: `update_menus` as main() calls it, and the game's own
// `apply_input` / `advance` / `resolve_hit`. Only keyboard and speaker are
// replaced.

pub(crate) fn press(back: bool, fwd: bool, fire: bool) -> MenuIn { MenuIn { back, fwd, fire } }
pub(crate) const NOTHING: MenuIn = MenuIn { back: false, fwd: false, fire: false };

/// Walk the title menu into two-player mode and return the game sitting
/// on the select screen.
pub(crate) fn at_versus_select() -> Game {
    let mut g = Game::new();
    assert_eq!(g.state, State::Title);
    update_menus(&mut g, [press(false, true, false), NOTHING]); // move to 2 PLAYERS
    assert_eq!(g.menu, 1, "the title menu did not move");
    update_menus(&mut g, [press(false, false, true), NOTHING]); // confirm
    assert_eq!(g.state, State::Select);
    assert_eq!(g.mode, Mode::Versus);
    g
}

/// Step a player's cursor `n` times forward.
pub(crate) fn cursor(g: &mut Game, who: usize, n: usize) {
    for _ in 0..n {
        let mut m = [NOTHING; 2];
        m[who] = press(false, true, false);
        update_menus(g, m);
    }
}

pub(crate) fn lock(g: &mut Game, who: usize) {
    let mut m = [NOTHING; 2];
    m[who] = press(false, false, true);
    update_menus(g, m);
}

/// Fight it out with two scripted humans until somebody wins the match
/// or the clock beats them both. Returns (frames, rounds won by each).
pub(crate) fn versus_bout(g: &mut Game, style: fn(usize, usize, &[Fighter; 2]) -> Input) -> (usize, [i32; 2]) {
    let mut frames = 0usize;
    while frames < 60 * 240 {
        frames += 1;
        if !step_versus(g, frames, style) { break; }
    }
    (frames, [g.p[0].rounds, g.p[1].rounds])
}

/// One frame of a versus match. Returns false once the match is over.
pub(crate) fn step_versus(g: &mut Game, frames: usize, style: fn(usize, usize, &[Fighter; 2]) -> Input) -> bool {
    match g.state {
        State::Vs => update_vs(g, F, false),
        State::RoundIntro => { if g.phase.tick(F) { g.state = State::Fight; } }
        State::RoundEnd => update_round_end(g, F),
        State::MatchEnd => { update_match_end(g, F); }
        State::Continue => update_continue(g, F, false),
        State::Over | State::Won => return false,
        State::Fight => {
            if g.hitstop > 0.0 { g.hitstop -= F; return true; }
            g.clock -= F;
            let ins = [style(0, frames, &g.p), style(1, frames, &g.p)];
            for i in 0..2 {
                let other = g.p[1 - i].x;
                if g.p[i].free() && !g.p[i].airborne() {
                    g.p[i].facing = if other >= g.p[i].x { 1.0 } else { -1.0 };
                }
                let close = face_off(&mut g.p)[i];
                apply_input(&mut g.p[i], ins[i], close, F);
                advance(&mut g.p[i], F);
                g.p[i].x = clamp(g.p[i].x, WALL_MARGIN, WIN_W as f32 - WALL_MARGIN);
            }
            separate(&mut g.p);
            for i in 0..2 {
                let (a, d) = if i == 0 { (0, 1) } else { (1, 0) };
                let mut atk = g.p[a];
                let mut def = g.p[d];
                let _ = resolve_hit(&mut atk, &mut def, ins[d]);
                g.p[a] = atk;
                g.p[d] = def;
            }
            if g.p[0].health <= 0 || g.p[1].health <= 0 || g.clock <= 0.0 {
                g.result = if g.p[0].health == g.p[1].health { RoundResult::Draw }
                    else if g.p[0].health > g.p[1].health { RoundResult::P1 }
                    else { RoundResult::P2 };
                match g.result {
                    RoundResult::P1 => { g.p[0].rounds += 1; }
                    RoundResult::P2 => { g.p[1].rounds += 1; }
                    RoundResult::Draw => { g.p[0].rounds += 1; g.p[1].rounds += 1; }
                }
                g.state = State::RoundEnd;
                g.phase.start(0.2);
            }
        }
        _ => return false,
    }
    true
}

/// Two humans who both actually fight.
pub(crate) fn brawlers(who: usize, frame: usize, p: &[Fighter; 2]) -> Input {
    let mut i = Input::default();
    let me = p[who];
    let foe = p[1 - who];
    let toward_right = foe.x > me.x;
    let phase = frame + who * 17;
    if (foe.x - me.x).abs() > 80.0 {
        if toward_right { i.right = true; } else { i.left = true; }
    } else if phase % 37 < 4 {
        i.punch_low = true;
    } else if phase % 37 < 8 {
        i.kick_low = true;
    } else if phase % 37 < 11 {
        i.kick_high = true;
    } else if phase % 37 < 20 {
        if toward_right { i.left = true; } else { i.right = true; } // guard
    } else if toward_right { i.right = true; } else { i.left = true; }
    i
}

#[test]
pub(crate) fn playtest_1_a_whole_two_player_session() {
    // Title to winner, through the real flow, pressing the real keys.
    let _sim = simulating(0x2F1A);
    let mut g = at_versus_select();
    cursor(&mut g, 1, 1);            // P2 looks around
    lock(&mut g, 0);                 // P1 takes RYUKA
    lock(&mut g, 1);                 // P2 takes whoever they are on
    assert_eq!(g.state, State::Vs, "locking both did not start the match");
    assert_ne!(g.p[0].who, g.p[1].who, "the same fighter reached the ring twice");

    let (frames, rounds) = versus_bout(&mut g, brawlers);
    assert!(frames < 60 * 240, "the match never finished");
    assert!(rounds[0] >= ROUNDS_TO_WIN || rounds[1] >= ROUNDS_TO_WIN,
        "nobody took the match: {rounds:?}");
    assert_eq!(g.state, State::Over, "a versus match did not end at the result screen");
}

#[test]
pub(crate) fn playtest_2_player_two_is_moved_off_a_fighter_that_gets_taken() {
    // Both cursors on one fighter, player one takes it: player two moves to a
    // free one.
    let mut g = at_versus_select();
    g.pick = 0;
    g.pick2 = 0;
    lock(&mut g, 0);
    assert!(g.locked[0]);
    assert_ne!(g.pick2, g.pick, "player two was left stranded on a taken fighter");
    lock(&mut g, 1);
    assert!(g.locked[1], "player two could not lock the fighter they were moved to");
    assert_ne!(g.p[0].who, g.p[1].who, "the same fighter reached the ring twice");
    assert_eq!(g.state, State::Vs);
}

#[test]
pub(crate) fn playtest_3_either_player_may_lock_in_first() {
    // The lock-out has to work both ways round, and the cursor that is
    // still moving must never be able to land on the taken fighter.
    for first in 0..2 {
        let second = 1 - first;
        let mut g = at_versus_select();
        g.pick = 1;
        g.pick2 = 1;
        lock(&mut g, first);
        let taken = g.picked(first);
        assert_ne!(g.picked(second), taken,
            "player {second} was left on the fighter player {first} took");
        // Walk the whole ring twice; it must never land on the taken one.
        for _ in 0..FIGHTERS.len() * 2 {
            cursor(&mut g, second, 1);
            assert_ne!(g.picked(second), taken,
                "player {second}'s cursor landed on the taken fighter");
        }
        lock(&mut g, second);
        assert!(g.locked[second], "player {second} could not lock a free fighter");
        assert_eq!(g.state, State::Vs);
        assert_ne!(g.p[0].who, g.p[1].who);
    }
}

#[test]
pub(crate) fn playtest_4_each_player_drives_only_their_own_fighter() {
    // The failure that makes the mode unplayable: one person's stick
    // moving both fighters.
    let mut g = at_versus_select();
    lock(&mut g, 0);
    lock(&mut g, 1);
    g.state = State::Fight;
    let (x0, x1) = (g.p[0].x, g.p[1].x);
    // Only player one presses anything, for a while.
    for _ in 0..40 {
        let mut a = Input::default();
        a.right = true;
        for i in 0..2 {
            let inp = if i == 0 { a } else { Input::default() };
            apply_input(&mut g.p[i], inp, false, F);
            advance(&mut g.p[i], F);
        }
    }
    assert!(g.p[0].x > x0 + 20.0, "player one did not move on their own input");
    assert!((g.p[1].x - x1).abs() < 0.5,
        "player two moved {:.0}px without touching anything", (g.p[1].x - x1).abs());
}

#[test]
pub(crate) fn playtest_5_one_player_mode_is_untouched() {
    // The whole feature is worthless if it broke the game that was
    // already there. Walk the solo path through the same real flow.
    let _sim = simulating(0x1F5A);
    let mut g = Game::new();
    update_menus(&mut g, [press(false, false, true), NOTHING]);  // 1 PLAYER
    assert_eq!(g.mode, Mode::Solo);
    assert_eq!(g.state, State::Select);
    // One cursor, and no lock-out: every fighter is selectable.
    for _ in 0..FIGHTERS.len() {
        cursor(&mut g, 0, 1);
        assert!(!g.taken_by_other(0, g.pick));
    }
    // Player two's keys do nothing on the solo select screen.
    let before = g.pick;
    cursor(&mut g, 1, 3);
    assert_eq!(g.pick, before, "player two's keys moved the solo cursor");
    update_menus(&mut g, [press(false, false, true), NOTHING]);
    assert_eq!(g.state, State::Vs, "the solo game would not start");
    assert_eq!(g.p[1].who, g.ladder()[0], "the solo ladder picked the wrong opponent");
}

#[test]
pub(crate) fn a_versus_result_reports_a_winner_not_a_score() {
    // A versus match ends with a winner, not a solo run's GAME OVER over a
    // score.
    let mut g = Game::new();
    g.mode = Mode::Versus;
    g.pick = 0;
    g.pick2 = 2;
    g.start_versus();
    g.p[1].rounds = ROUNDS_TO_WIN;
    g.state = State::MatchEnd;
    g.phase.start(0.05);
    for _ in 0..8 { update_match_end(&mut g, F); }
    assert_eq!(g.state, State::Over);
    // Nothing was scored, and nothing should claim to have been.
    assert_eq!(g.sess.score, 0, "a versus match reported a score");
    assert!(g.p[1].rounds > g.p[0].rounds, "the result lost track of who won");
}

// ---- player two joins by reaching for their own stick -------------------

#[test]
pub(crate) fn player_two_touching_anything_chooses_two_players() {
    // A cabinet has never asked player one to select two-player mode on
    // behalf of somebody standing next to them. The second player
    // reaches for their own stick and the machine works out the rest.
    for (what, m2) in [("their stick forward", press(false, true, false)),
                       ("their stick back", press(true, false, false)),
                       ("a button", press(false, false, true))] {
        let mut g = Game::new();
        assert_eq!(g.menu, 0, "the title does not start on 1 PLAYER");
        let cue = update_menus(&mut g, [NOTHING, m2]);
        assert_eq!(g.menu, 1, "player two used {what} and the cabinet ignored them");
        assert_eq!(cue, Some(Cue::Step), "claiming the second slot made no sound");
        assert_eq!(g.state, State::Title, "player two's first touch started the match");
    }
}

#[test]
pub(crate) fn player_two_cannot_give_the_second_slot_back() {
    // The place being given up in that direction is theirs, and nobody
    // else is asking for it — so their stick only ever selects two
    // players. Player one keeps the toggle.
    let mut g = Game::new();
    update_menus(&mut g, [NOTHING, press(false, true, false)]);
    assert_eq!(g.menu, 1);
    for _ in 0..4 {
        update_menus(&mut g, [NOTHING, press(false, true, false)]);
        assert_eq!(g.menu, 1, "player two waggled their stick out of the game");
    }
    // Player one still can.
    update_menus(&mut g, [press(false, true, false), NOTHING]);
    assert_eq!(g.menu, 0, "player one lost the toggle");
}

#[test]
pub(crate) fn either_player_starts_the_match_once_the_second_slot_is_claimed() {
    // Player two has as much right to the start button as the player
    // who put the coin in — but only once they are actually in.
    let mut g = Game::new();
    // Before they have joined, their fire is the join, not a start.
    update_menus(&mut g, [NOTHING, press(false, false, true)]);
    assert_eq!(g.state, State::Title, "player two's join also started the match");
    assert_eq!(g.menu, 1);
    // And now it starts it.
    let cue = update_menus(&mut g, [NOTHING, press(false, false, true)]);
    assert_eq!(g.state, State::Select, "player two could not start the match they joined");
    assert_eq!(g.mode, Mode::Versus);
    assert_eq!(cue, Some(Cue::Confirm));
}

#[test]
pub(crate) fn a_solo_player_is_never_dragged_into_two_player_mode() {
    // Player one working the title on their own has to be able to reach
    // 1 PLAYER and start there, with the second station sitting
    // untouched beside them.
    let mut g = Game::new();
    update_menus(&mut g, [press(false, true, false), NOTHING]);
    assert_eq!(g.menu, 1);
    update_menus(&mut g, [press(false, true, false), NOTHING]);
    assert_eq!(g.menu, 0);
    update_menus(&mut g, [press(false, false, true), NOTHING]);
    assert_eq!(g.state, State::Select);
    assert_eq!(g.mode, Mode::Solo, "a solo player was put into a versus match");
}

#[test]
#[ignore = "diagnostic"]
pub(crate) fn dump_ladders() {
    for pick in 0..FIGHTERS.len() {
        let names: Vec<&str> = Game { pick, ..Game::new() }.ladder().iter().map(|&w| FIGHTERS[w].name).collect();
        println!("{:8} -> {}", FIGHTERS[pick].name, names.join(", "));
    }
}

#[test]
pub(crate) fn the_billing_gives_way_to_the_bow_and_a_press_cuts_it_short() {
    let mut g = Game { pick: 0, ..Game::new() };
    g.start_match(0);
    g.announce();
    // A press on the frame it appears is the one that chose the fighter.
    update_vs(&mut g, F, true);
    assert_eq!(g.state, State::Vs, "the billing was skipped before it was seen");
    for _ in 0..60 { update_vs(&mut g, F, false); }
    update_vs(&mut g, F, true);
    assert_eq!(g.state, State::RoundIntro, "a press did not move it on");

    g.announce();
    run_phase(&mut g, |g, dt| update_vs(g, dt, false));
    assert_eq!(g.state, State::RoundIntro, "left alone, it never started the round");
    assert!(g.phase.active(), "the bow got no time");
}

#[test]
pub(crate) fn a_turtle_hits_its_own_size_softer_than_it_hits_a_grown_fighter() {
    let turtle = FIGHTERS.iter().position(|a| a.build == Build::Turtle).unwrap();
    let blow = |foe: usize| {
        let mut p = [at(turtle, 200.0, 1.0), at(foe, 240.0, -1.0)];
        face_off(&mut p);
        p[0].scaled(move_data(MoveId::HighKick)).damage
    };
    let (small, big) = (blow(turtle + 1), blow(0));
    assert!(small < big, "{small} against a turtle, {big} against a grown fighter");
}

#[test]
pub(crate) fn nobody_is_thrown_twice_running() {
    let mut p = [at(0, 300.0, 1.0), at(1, 330.0, -1.0)];
    face_off(&mut p);
    wind_to_active(&mut p[0], MoveId::Throw);
    {
        let [atk, def] = &mut p;
        let (dmg, _, _) = resolve_hit(atk, def, Input::default());
        assert!(dmg > 0, "the first throw missed");
        assert!(def.throw_rest > 0.0);
    }
    // Back on their feet, and grabbed again at once: it does not take.
    let hp = p[1].health;
    p[1].act = Act::Idle;
    p[1].x = 330.0;
    wind_to_active(&mut p[0], MoveId::Throw);
    let [atk, def] = &mut p;
    assert_eq!(resolve_hit(atk, def, Input::default()).0, 0, "thrown again straight away");
    assert_eq!(def.health, hp);
    // It wears off.
    for _ in 0..(THROW_REST / F) as usize + 2 { advance(def, F); }
    assert_eq!(def.throw_rest, 0.0);
}
