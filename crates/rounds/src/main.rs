//! ROUNDS: the morning paper round through a Danish town, on a cargo bike. Houses line both sides of the street:
//! A throws left, B throws right. Through the letterbox is best, the doorstep is good, the garden is a miss, and
//! two misses lose a subscriber; a sample in a neighbour's box may win a new one. Each day of the week has its own
//! weather, and each ends with the Canal Run. Two players ride a tandem: the front steers and throws left, the back
//! pedals and throws right.

use blip::input::{
    coin_start, key_held, key_pressed, play_again, BLIP_KEY_A, BLIP_KEY_BUTTON2, BLIP_KEY_D, BLIP_KEY_DOWN, BLIP_KEY_F,
    BLIP_KEY_G, BLIP_KEY_J, BLIP_KEY_K, BLIP_KEY_LEFT, BLIP_KEY_RIGHT, BLIP_KEY_S, BLIP_KEY_SPACE, BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::{play_sfx_volume, web, window_conf, Blip, Jukebox};

mod bot;
mod draw;
mod game;
mod levels;
// The effects and music are made by build.rs; the tests check them here.
#[cfg(test)]
mod tunes;

use game::*;
use levels::*;

blip::sounds!(Sounds {
    throw, flap, thump, plop, sample, splash, crash, refill, heart_lost, cancelled, bell, reverse, rattle, bark, hiss,
    coo, soggy, target, empty, day_start, day_end, perfect, join, game_over, church, ting,
});

fn play(sfx: &Sounds, cue: Cue) {
    let (s, v) = match cue {
        Cue::Throw => (&sfx.throw, 0.45),
        Cue::Flap => (&sfx.flap, 0.9),
        Cue::Thump => (&sfx.thump, 0.8),
        Cue::Plop => (&sfx.plop, 0.5),
        Cue::Sample => (&sfx.sample, 0.7),
        Cue::Splash => (&sfx.splash, 0.6),
        Cue::Crash => (&sfx.crash, 0.9),
        Cue::Refill => (&sfx.refill, 0.8),
        Cue::HeartLost => (&sfx.heart_lost, 0.6),
        Cue::Cancelled => (&sfx.cancelled, 0.7),
        Cue::Bell => (&sfx.bell, 0.6),
        Cue::Reverse => (&sfx.reverse, 0.5),
        Cue::Rattle => (&sfx.rattle, 0.6),
        Cue::Bark => (&sfx.bark, 0.6),
        Cue::Hiss => (&sfx.hiss, 0.4),
        Cue::Coo => (&sfx.coo, 0.5),
        Cue::Soggy => (&sfx.soggy, 0.6),
        Cue::Target => (&sfx.target, 0.8),
        Cue::Empty => (&sfx.empty, 0.5),
        Cue::DayStart => (&sfx.day_start, 0.8),
        Cue::DayEnd => (&sfx.day_end, 0.8),
        Cue::Perfect => (&sfx.perfect, 0.8),
        Cue::Join => (&sfx.join, 0.8),
        Cue::Church => (&sfx.church, 0.5),
        Cue::Ting => (&sfx.ting, 0.5),
    };
    play_sfx_volume(s, v);
}

/// What the riders hold this frame: steering, pedalling and the throws (to which side, by whom).
/// Alone, the arrows steer too and both buttons are yours; on the tandem the front seat (WASD, F) steers and throws
/// left, the back seat (the arrows, J or K) pedals and throws right.
fn hands(two: bool) -> (f32, f32, Vec<(Side, usize)>) {
    let held = |k| key_held(k);
    let left = held(BLIP_KEY_A) || (!two && held(BLIP_KEY_LEFT));
    let right = held(BLIP_KEY_D) || (!two && held(BLIP_KEY_RIGHT));
    let fast = if two { held(BLIP_KEY_UP) } else { held(BLIP_KEY_W) || held(BLIP_KEY_UP) };
    let slow = if two { held(BLIP_KEY_DOWN) } else { held(BLIP_KEY_S) || held(BLIP_KEY_DOWN) };
    let mut throws = Vec::new();
    if key_pressed(BLIP_KEY_F) || key_pressed(BLIP_KEY_SPACE) { throws.push((Side::Left, 0)); }
    if two {
        if key_pressed(BLIP_KEY_J) || key_pressed(BLIP_KEY_K) { throws.push((Side::Right, 1)); }
    } else if key_pressed(BLIP_KEY_G) || key_pressed(BLIP_KEY_BUTTON2) {
        throws.push((Side::Right, 0));
    }
    let axis = |a: bool, b: bool| (b as i32 - a as i32) as f32;
    (axis(left, right), axis(slow, fast), throws)
}

fn skip() -> bool { key_pressed(BLIP_KEY_SPACE) || key_pressed(BLIP_KEY_F) }

/// The tune for now: the round (hurried on hard streets), the canal, or none.
fn track(g: &Game) -> Option<usize> {
    match g.state {
        State::Over => None,
        State::Canal => Some(2),
        State::Title => Some(0),
        _ => Some(if g.hard() { 1 } else { 0 }),
    }
}

fn conf() -> blip::macroquad::window::Conf {
    window_conf("ROUNDS", WIN_W, WIN_H)
}

#[blip::macroquad::main(conf)]
async fn main() {
    blip::bot::playthroughs(bot::PLAYTHROUGHS);
    let mut blip = Blip::new(WIN_W, WIN_H);
    blip.set_bloom(0.35);
    let mut g = Game::new();
    // The back seat of the tandem stays open for a second rider.
    web::set_players(2);
    let sfx = Sounds::load().await;
    let mut music = Jukebox::new("rounds", &["MORNING", "RUSH", "CANAL"]);
    // A screenshot (BLIP_SCREENSHOT_OUT) of a ride: ROUNDS_ROW puts the bike on that row, ROUNDS_DAY on that day.
    if blip.screenshot_mode {
        #[cfg(not(target_arch = "wasm32"))]
        let env = |k: &str, d: f32| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
        #[cfg(target_arch = "wasm32")]
        let env = |_: &str, d: f32| d;
        g.start(false);
        g.day = env("ROUNDS_DAY", 1.0) as usize - 1;
        g.begin_day();
        g.state = State::Play;
        g.clock = 5.0;
        g.bike_d = LOT * env("ROUNDS_ROW", 5.6);
        g.turned = g.bike_d >= NEW_STREET_ROW as f32 * LOT;
        g.bike_x = 214.0;
        // ROUNDS_CANAL=<distance>: Nyhavn, that far along the quay
        if env("ROUNDS_TITLE", 0.0) > 0.0 { g.state = State::Title; }
        let canal = env("ROUNDS_CANAL", -1.0);
        if canal >= 0.0 {
            g.start_canal();
            g.bike_d = canal;
            g.clock = 5.0;
        }
    }

    // Player two's J on a card or a tally between rides: they climb on as the next ride starts.
    let mut p2_waiting = false;
    loop {
        let dt = blip.delta_time;
        let t = blip::now();
        if blip::bot::active() { bot::drive(&g); }
        let was = g.state;
        match g.state {
            State::Title | State::Over => p2_waiting = false,
            _ => if !g.two_player && key_pressed(BLIP_KEY_J) { p2_waiting = true; },
        }
        match g.state {
            State::Title => {
                // the street rolls by under the title
                g.bike_d += 50.0 * dt;
                if g.bike_d > g.end_d() { g.bike_d = LOT; }
                g.bike_x = 240.0 + (t * 0.7).sin() * 40.0;
                if key_pressed(BLIP_KEY_J) {
                    web::spend_coin();
                    web::spend_coin();
                    web::set_mode(true);
                    g.start(false);
                    g.join();
                } else if coin_start() {
                    g.start(false);
                }
            }
            State::Card => g.update_card(dt, skip()),
            State::Play => {
                if !g.two_player && p2_waiting {
                    p2_waiting = false;
                    web::spend_coin();
                    web::set_mode(true);
                    g.join();
                }
                let (steer, pedal, throws) = hands(g.two_player);
                g.ride(steer, pedal, dt);
                for (side, by) in throws { g.throw(side, by); }
                g.update_play(dt);
            }
            State::Crash => g.update_crash(dt),
            State::Tally => g.update_tally(dt, skip()),
            State::Canal => {
                let (steer, pedal, throws) = hands(g.two_player);
                g.ride(steer, pedal, dt);
                for (side, by) in throws { g.canal_throw(side, by); }
                g.update_canal(dt);
            }
            State::Week => g.update_week(dt, skip()),
            State::Over if g.offering() => match g.cont.update(dt, blip::input::btn1_pressed()) {
                Some(true) => g.continue_day(),
                Some(false) => {
                    g.post_score();
                    web::set_players(2);
                    g.state = State::Title;
                }
                None => if g.cont.ticked() { g.cues.push(Cue::Ting); },
            },
            State::Over => {
                if play_again(&mut g.wait, dt) {
                    web::set_players(2);
                    g.start(false);
                }
            }
        }
        if g.state != was {
            if g.state == State::Crash { web::haptic(); }
            // A new best (over the run's continues) ends on the perfect-day fanfare instead.
            if g.state == State::Over {
                if g.sess.beats_best() { play_sfx_volume(&sfx.perfect, 1.0); } else { play_sfx_volume(&sfx.game_over, 0.8); }
            }
        }
        for cue in std::mem::take(&mut g.cues) { play(&sfx, cue); }
        match track(&g) { Some(i) => music.play(i), None => music.stop() }

        match g.state {
            State::Title => draw::draw_title(&blip, &g, web::high_score(), t),
            State::Canal => {
                draw::plain_view(&blip, &g);
                draw::draw_canal(&blip, &g, t);
                draw::draw_bike(&blip, &g, t, g.bike_x, BIKE_Y);
                draw::draw_bits(&blip, &g);
                draw::draw_pops(&blip, &g);
                blip.draw_hud(g.sess.score, g.sess.lives);
                draw::draw_bar(&blip, &g, t);
                draw::draw_canal_hud(&blip, &g);
            }
            _ => {
                draw::draw_world(&blip, &g, t);
                draw::draw_weather(&blip, &g, t);
                draw::draw_pops(&blip, &g);
                blip.draw_hud(g.sess.score, g.sess.lives);
                draw::draw_bar(&blip, &g, t);
                draw::draw_banner(&blip, &g);
                match g.state {
                    State::Card => draw::draw_card(&blip, &g, t),
                    State::Tally => draw::draw_tally(&blip, &g),
                    State::Week => draw::draw_week(&blip, &g),
                    State::Over => draw::draw_over(&blip, &g, web::high_score()),
                    _ => {}
                }
            }
        }
        blip.next_frame().await;
    }
}
