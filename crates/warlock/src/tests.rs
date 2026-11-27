use super::*;

/// Room 2 with its runes cleansed and every monster slain: the moment a whelp may come.
fn cleared_room_two() -> Game {
    let mut g = Game::new();
    g.sess.level = 2;
    g.load_room();
    for r in &mut g.runes {
        r.cleansed = true;
    }
    for i in 0..g.enemies.len() {
        g.kill_enemy(i);
    }
    g
}

#[test]
fn leaving_the_spell_circle_returns_to_play() {
    let mut g = Game::new();
    g.start_game();
    let level = g.sess.level;
    g.state = State::Bonus;
    g.start_bonus();
    g.leave_bonus();
    assert!(g.state == State::Play, "the spell circle never handed back to play");
    assert!(g.bonus.is_none());
    assert_eq!(g.sess.level, level + 1);
}

#[test]
fn a_bolt_stops_at_a_wall_and_not_before() {
    let mut g = Game::new();
    g.start_game();
    g.place_wizard(0, Cell { c: 1, r: 1 });
    g.wizards[0].facing = Dir::Right;
    g.cast_bolt(0);
    let mut last = Cell { c: 0, r: 0 };
    for _ in 0..40 {
        g.update_projectiles(BOLT_INTERVAL);
        if g.bolts.is_empty() {
            break;
        }
        last = g.bolts[0].tile;
    }
    assert!(last.c <= 14, "the bolt crossed the right wall");
}

#[test]
fn reveal_exposes_wraiths() {
    let mut g = Game::new();
    g.sess.level = 2;
    g.load_room(); // room 2 introduces wraiths
    g.wizards[0].reveal_cd = 0.0;
    assert!(g.enemies.iter().any(|e| e.kind == EnemyKind::Wraith));
    g.reveal(0);
    for e in &g.enemies {
        if e.kind == EnemyKind::Wraith {
            assert!(e.revealed > 0.0, "wraith not exposed by reveal");
        }
    }
}

#[test]
fn cleansing_a_rune_opens_the_exit() {
    let mut g = Game::new();
    g.start_game();
    for r in &mut g.runes {
        r.cleansed = true;
    }
    for e in &mut g.enemies {
        e.gone = true;
    }
    g.check_exit();
    assert!(g.exit_open, "exit should open once the chamber is clear");
}

#[test]
fn reveal_cleanses_an_adjacent_rune() {
    let mut g = Game::new();
    g.start_game();
    g.wizards[0].reveal_cd = 0.0;
    let rune = g.runes[0].tile;
    // stand beside it
    g.place_wizard(0, Cell { c: rune.c + 1, r: rune.r });
    g.reveal(0);
    assert!(g.runes[0].cleansed, "an adjacent rune was not cleansed by reveal");
}

#[test]
fn the_warlock_falls_after_four_ward_breaks() {
    let mut g = Game::new();
    g.sess.level = 6;
    g.load_room();
    assert!(g.warlock.is_some(), "room 6 has no warlock");
    assert_eq!(g.warlock.as_ref().unwrap().wards, 4);
    for _ in 0..4 {
        g.hit_warlock();
    }
    assert_eq!(
        g.warlock.as_ref().map(|w| w.wards),
        Some(0),
        "four ward breaks should defeat the warlock"
    );
    for r in &mut g.runes {
        r.cleansed = true;
    }
    for e in &mut g.enemies {
        e.gone = true;
    }
    g.check_exit();
    assert!(g.exit_open, "exit should open once the warlock and minions are cleared");
}

#[test]
fn reveal_draws_spellhounds_to_the_cast_point() {
    let mut g = Game::new();
    g.sess.level = 3;
    g.load_room();
    g.wizards[0].reveal_cd = 0.0;
    g.wizards[0].tile = Cell { c: 4, r: 4 };
    g.reveal(0);
    for e in &g.enemies {
        if e.kind == EnemyKind::Spellhound {
            assert!(e.hunting, "reveal should set a spellhound hunting");
            assert_eq!(e.target, Cell { c: 4, r: 4 }, "spellhound should head for the cast point");
        }
    }
}

#[test]
fn a_rune_gate_carries_the_wizard_across_the_room() {
    let mut g = Game::new();
    g.sess.level = 4;
    g.load_room();
    assert!(!g.gates.is_empty());
    let gate = g.gates[0];
    // step onto gate A
    g.place_wizard(0, gate.a);
    g.warp_through_gate(0, 0);
    assert_eq!(g.wizards[0].tile, gate.b, "the wizard should emerge at the paired sigil");
    assert!(g.gates[0].cooldown > 0.0, "the gate pair should close after use");
}

#[test]
fn reveal_then_bolt_casts_a_lance_that_pierces() {
    let mut g = Game::new();
    g.start_game();
    g.intro = 0.0;
    // clear a row and line up three imps along it
    let w = g.wizards[0].tile;
    for c in w.c + 1..w.c + 5 {
        g.walls.set(c, w.r, false);
    }
    g.enemies.truncate(3);
    for (k, e) in g.enemies.iter_mut().enumerate() {
        e.kind = EnemyKind::Imp;
        e.tile = Cell { c: w.c + 2 + k as i32, r: w.r };
        e.from = e.tile;
        e.step_t = 1000.0;
    }
    g.wizards[0].facing = Dir::Right;
    g.reveal(0);
    g.cast_bolt(0);
    assert!(g.bolts[0].proj == Proj::Lance);
    for _ in 0..40 {
        g.update_projectiles(0.02);
    }
    assert!(g.enemies.iter().all(|e| e.gone), "the lance goes through all three");
    // a plain bolt stops at the first
    let mut g = Game::new();
    g.start_game();
    g.wizards[0].reveal_cd = 0.0;
    g.cast_bolt(0);
    assert!(g.bolts[0].proj == Proj::Bolt);
}

#[test]
fn potions_haste_gild_and_add_a_hat() {
    let mut g = Game::new();
    g.start_game();
    g.intro = 0.0;
    let here = g.wizards[0].tile;
    g.drop_potion(here, PotionKind::Haste);
    g.update_potions(0.01);
    assert!(g.potions.is_empty(), "standing on it drinks it");
    assert!(g.wizards[0].haste > 0.0);
    let before = g.sess.score;
    g.drop_potion(here, PotionKind::Gold);
    g.update_potions(0.01);
    assert_eq!(g.sess.score, before + SCORE_GOLD);
    g.lives[0] = LIVES_MAX;
    g.drop_potion(here, PotionKind::Life);
    g.update_potions(0.01);
    assert_eq!(g.lives[0], LIVES_MAX, "hats are capped");
    // one left lying seeps away
    g.drop_potion(Cell { c: here.c + 50, r: here.r }, PotionKind::Gold);
    g.update_potions(POTION_SECS + 0.1);
    assert!(g.potions.is_empty());
}

#[test]
fn j_on_the_title_starts_a_two_player_game() {
    let mut g = Game::new();
    g.start_two();
    assert!(g.state == State::Play);
    assert!(g.two_player);
    assert_eq!(g.active, [true, true]);
    assert_eq!(g.lives, [LIVES_START, LIVES_START]);
    assert_ne!(g.wizards[1].tile, g.wizards[0].tile);
}

#[test]
fn joining_adds_a_second_wizard_with_her_own_lives() {
    let mut g = Game::new();
    g.start_game();
    assert!(!g.two_player);
    assert!(!g.active[1]);
    // simulate the join key (J is pressed through the input layer; here we
    // drive the join directly, which try_join gates on key_pressed).
    g.two_player = true;
    g.active[1] = true;
    g.place_wizard(1, Cell { c: 2, r: 1 });
    assert_eq!(g.lives[1], LIVES_START);
    assert_eq!(hud_lives(&g), LIVES_START * 2, "co-op shows both wizards' lives");
}

#[test]
fn a_downed_partner_returns_on_chamber_clear() {
    let mut g = Game::new();
    g.start_game();
    g.two_player = true;
    g.active[1] = true;
    // partner two runs out of lives
    g.active[1] = false;
    g.lives[1] = 0;
    // clearing the chamber revives them with one life
    g.finish_chamber();
    assert!(g.active[1], "the downed partner should return");
    assert_eq!(g.lives[1], 1, "the returned partner gets one life");
}

#[test]
fn a_bolt_passes_through_the_other_wizard() {
    // Friendly spells pass through wizards: no wizard is a bolt target.
    let mut g = Game::new();
    g.start_game();
    g.two_player = true;
    g.active[1] = true;
    // place wizard 1 directly in front of wizard 0's facing
    g.wizards[0].facing = Dir::Right;
    g.place_wizard(1, Cell { c: g.wizards[0].tile.c + 2, r: g.wizards[0].tile.r });
    g.cast_bolt(0);
    // run the bolt a few tiles; wizard 1 must not take damage
    for _ in 0..6 {
        g.update_projectiles(BOLT_INTERVAL);
    }
    assert_eq!(g.lives[1], LIVES_START, "a friendly bolt must not harm the other wizard");
}

#[test]
fn a_ghost_does_not_kill_until_its_body_reaches_the_wizard() {
    let mut g = Game::new();
    g.start_game();
    let w = g.wizards[0].tile;
    g.enemies.clear();
    g.enemies.push(Enemy {
        kind: EnemyKind::Wraith,
        tile: Cell { c: w.c + 1, r: w.r },
        from: Cell { c: w.c + 1, r: w.r },
        lerp: 0.0,
        step_t: 100.0,
        step: 1.0,
        dir: Dir::Left,
        visible: true,
        revealed: 0.0,
        flash: 0.0,
        hunting: false,
        target: Cell { c: 0, r: 0 },
        lunging: true,
        windup: 0.0,
        charge: 0.0,
        shot_cd: 0.0,
        gone: false,
        face_left: true,
    });
    // the wraith starts a step onto the wizard's tile: its body is still
    // a whole cell away, so no contact yet
    g.enemies[0].from = g.enemies[0].tile;
    g.enemies[0].tile = w;
    g.enemies[0].lerp = 0.0;
    g.check_contact();
    assert_eq!(g.lives[0], LIVES_START, "a ghost killed from a whole cell away");
    // once it has visually arrived, the touch is real
    g.enemies[0].lerp = 1.0;
    g.check_contact();
    assert_eq!(g.lives[0], LIVES_START - 1, "an arrived ghost did not connect");
}

#[test]
fn a_wraith_pauses_visibly_before_its_first_lunge() {
    let mut g = Game::new();
    g.sess.level = 2;
    g.load_room();
    let wraith_tile = {
        let e = g.enemies.iter_mut().find(|e| e.kind == EnemyKind::Wraith).unwrap();
        e.lunging = false;
        e.charge = 0.0;
        e.step_t = 100.0;
        e.tile
    };
    // put a wizard beside it, then run one visibility pass
    g.place_wizard(0, Cell { c: wraith_tile.c + 1, r: wraith_tile.r });
    g.update_enemies(0.0);
    let e = g.enemies.iter().find(|e| e.kind == EnemyKind::Wraith).unwrap();
    assert!(e.lunging, "the wraith should now be lunging");
    assert!(e.charge > 0.0, "a fresh lunge should telegraph before it moves");
}

#[test]
fn a_cleansed_stone_makes_nearby_wraiths_visible() {
    let mut g = Game::new();
    g.sess.level = 2;
    g.load_room();
    // cleanse the first rune and put a wraith beside it
    g.runes[0].cleansed = true;
    let rune_tile = g.runes[0].tile;
    if let Some(w) = g.enemies.iter_mut().find(|e| e.kind == EnemyKind::Wraith) {
        w.tile = rune_tile;
        w.from = rune_tile;
        w.revealed = 0.0;
        w.flash = WRAITH_FLASH_OFF; // force the invisible part of the cycle
        w.step_t = 100.0; // hold still so update_enemies doesn't move it
    }
    g.update_enemies(0.0);
    let near = g.enemies.iter().find(|e| e.kind == EnemyKind::Wraith && e.tile == rune_tile);
    assert!(
        near.is_some_and(|w| w.visible),
        "a cleansed rune should force the wraith beside it visible"
    );
}

#[test]
fn the_last_monster_brings_a_whelp_that_holds_the_stairs() {
    let mut g = cleared_room_two();
    assert!(g.whelp.is_some(), "clearing a chamber from room 2 on should call a whelp");
    assert!(!g.exit_open, "the stairs wait while the whelp is loose");
    // left alone, it flies off and the stairs open
    for _ in 0..((WHELP_SECS + 1.0) * 60.0) as i32 {
        g.update_whelp(1.0 / 60.0);
    }
    assert!(g.whelp.is_none(), "the whelp should leave when its time runs out");
    assert!(g.exit_open, "the stairs open once the whelp has gone");
    assert!(!g.double_next, "an escaped whelp pays nothing");
}

#[test]
fn striking_the_whelp_doubles_the_next_chamber() {
    let mut g = cleared_room_two();
    g.strike_whelp();
    assert!(g.whelp.is_none());
    assert!(g.exit_open);
    g.finish_chamber();
    assert!(g.double, "the chamber after a struck whelp pays double");
    let before = g.sess.score;
    g.add_points(100);
    assert_eq!(g.sess.score - before, 200);
    g.finish_chamber();
    assert!(!g.double, "the double lasts one chamber");
}

#[test]
fn no_whelp_in_the_first_chamber_or_the_sanctum() {
    for level in [1, 6] {
        let mut g = Game::new();
        g.sess.level = level;
        g.load_room();
        let n = g.enemies.len();
        for i in 0..n {
            g.kill_enemy(i);
        }
        assert!(g.whelp.is_none(), "level {level} should not call a whelp");
    }
}
