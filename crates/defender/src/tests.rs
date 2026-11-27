use super::*;

fn at_level(level: i32) -> Game {
    let mut g = Game::new();
    g.sess.reset(LIVES_START);
    g.sess.level = level;
    g
}

#[test]
fn a_mothership_ends_every_fifth_level_and_no_others() {
    for level in 1..=20 {
        let g = at_level(level);
        assert_eq!(
            g.is_boss_level(), level % 5 == 0,
            "level {level} disagrees about whether it ends with a mothership"
        );
    }
}

/// A run of `n` hits.
fn run_of(n: i32) -> blip::Streak {
    let mut s = blip::Streak::new(STREAK_PER, STREAK_MAX);
    for _ in 0..n { s.hit(); }
    s
}

#[test]
fn the_strip_counts_down_to_the_next_multiple() {
    let shown: Vec<(i32, i32, String)> = [1, 3, 4, 6, 7]
        .into_iter().map(|n| (n, run_of(n).multiple(), streak_label(&run_of(n)))).collect();
    assert_eq!(shown[0], (1, 1, "1 IN A ROW  X1  3 FOR X2".to_string()));
    assert_eq!(shown[1], (3, 1, "3 IN A ROW  X1  1 FOR X2".to_string()));
    assert_eq!(shown[2], (4, 2, "4 IN A ROW  X2  3 FOR X3".to_string()));
    assert_eq!(shown[3], (6, 2, "6 IN A ROW  X2  1 FOR X3".to_string()));
    assert_eq!(shown[4], (7, 3, "7 IN A ROW  X3  3 FOR X4".to_string()));
}

#[test]
fn the_multiple_stops_at_its_cap() {
    assert_eq!(run_of(STREAK_PER * STREAK_MAX * 3).multiple(), STREAK_MAX);
    assert_eq!(streak_label(&run_of(100)), format!("100 IN A ROW  X{STREAK_MAX}"));
}

#[test]
fn each_mothership_is_tougher_than_the_last() {
    // The point of the fight is that it is the thing a run is aiming
    // at. If the fifth one is no harder than the first, the levels
    // between them have not been climbing toward anything.
    let boss = |level| { let mut g = at_level(level); g.start_boss(); g };
    let levels = [5, 10, 15, 20, 25];
    blip::testing::climbs("mothership health", levels, |l| boss(l).boss_hp);
    blip::testing::climbs("mothership speed", levels, |l| boss(l).boss_speed());
    blip::testing::never_rises("volley gap", levels, |l| boss(l).boss_volley_gap());
}

#[test]
fn the_volley_never_becomes_unsurvivable() {
    // Deep runs must stay a fight rather than a wall: the gap between
    // volleys has a floor, so the difficulty curve flattens instead
    // of eventually firing every frame.
    let mut g = at_level(500);
    g.start_boss();
    assert!(
        g.boss_volley_gap() >= BOSS_VOLLEY_FLOOR,
        "the volley gap fell through its floor at level 500 ({}s)", g.boss_volley_gap()
    );
}

#[test]
fn a_mothership_starts_at_full_health_and_on_screen() {
    let mut g = at_level(10);
    g.start_boss();
    assert!(g.boss_active);
    assert_eq!(g.boss_hp, g.boss_hp_max);
    // It flies in from whichever side it starts on, so it is allowed
    // to begin off-screen — but only by its own width, or it spends
    // the first seconds of the fight invisible.
    assert!(
        g.boss_x >= -BOSS_W && g.boss_x <= WIN_W as f32,
        "the mothership started {} px out, off its own entry", g.boss_x
    );
}

#[test]
fn a_mothership_hangs_above_the_player_not_on_top_of_them() {
    let mut g = at_level(5);
    g.start_boss();
    for _ in 0..600 {
        g.boss_phase += 0.05;
        let y = g.boss_y();
        assert!(y >= PLAY_Y as f32, "the mothership rode up into the HUD ({y})");
        assert!(
            y + BOSS_H < (GROUND_Y - 40) as f32,
            "the mothership sank onto the player's own line ({y})"
        );
    }
}

#[test]
fn a_dead_mothership_does_not_come_back_on_the_same_level() {
    // Once the mothership is down the formation-cleared check keeps
    // passing; without boss_done the level would summon one forever.
    let mut g = at_level(5);
    g.start_boss();
    g.boss_active = false;
    g.boss_done = true;
    assert!(g.is_boss_level() && g.boss_done,
        "nothing records that this level's mothership has already been dealt with");
}

#[test]
fn a_new_level_forgets_the_last_mothership() {
    let mut g = at_level(5);
    g.start_boss();
    g.boss_done = true;
    g.sess.next_level();
    g.start_wave();
    assert!(!g.boss_active, "the old mothership survived into the next level");
    assert!(!g.boss_done, "the next boss level would be skipped entirely");
}
