//! Brawler: a one-on-one fighting game in tribute to Street Fighter II. Ten
//! fighters, seven stages, a nine-fight ladder against the CPU or one match
//! between two players.
//!
//! What it takes seriously, because they are the game:
//!
//! 1. **Attack height.** Every attack is low, mid or overhead, and a block
//!    works only at the right height: crouch-blocking eats sweeps and loses
//!    to jump-ins, standing is the reverse.
//! 2. **Frame data.** Startup, active and recovery in frames; punishable in
//!    proportion to reach and damage. A whiffed sweep hurts, a jab does not.
//! 3. **What you can see.** Poses are built from the same numbers as the
//!    hitboxes, so an arm that looks extended is what hits you.
//!
//! Where things are:
//!
//! | module | what is in it |
//! |---|---|
//! | [`moves`] | physics constants and the move table |
//! | [`roster`] | the ten fighters and the seven stages |
//! | [`fighter`] | one fighter's state in a round |
//! | [`game`] | the session: ladder, round, everything on the stage |
//! | [`rules`] | input to action, movement, and what a blow does |
//! | [`cpu`] | the CPU opponent |
//! | [`keys`] | each player's keys, read as an `Input` |
//! | [`fight`] | one frame of a round |
//! | [`flow`] | menus, billing, bonus round, results, continue |
//! | [`draw`] | all drawing: stages, fighter parts, poses, HUD, screens |
//! | [`sound`] | the effects, and the `Sfx` the rules ask for by name |
//! | [`shots`] | screenshot scenes for the card and for checking drawings |
//! | `bot` | the native playtest autopilot (`BLIP_BOT=1`) |
//!
//! This file is the frame loop: load, read the state, update it, play the
//! sounds it asked for, keep the music and ambience in step, draw.

mod moves;
mod roster;
mod fighter;
mod game;
mod rules;
mod cpu;
mod keys;
mod fight;
mod flow;
mod sound;
mod shots;
mod draw;
#[cfg(not(target_arch = "wasm32"))]
mod bot;

use blip::macroquad::input::KeyCode;
use blip::input::{key_held, key_pressed, BLIP_KEY_A, BLIP_KEY_BUTTON2, BLIP_KEY_D,
    BLIP_KEY_DOWN, BLIP_KEY_F, BLIP_KEY_G, BLIP_KEY_J, BLIP_KEY_K, BLIP_KEY_LEFT,
    BLIP_KEY_RIGHT, BLIP_KEY_S, BLIP_KEY_SPACE, BLIP_KEY_UP, BLIP_KEY_W};
use blip::{clamp, Jukebox, rand_int, rects_overlap, web,
    window_conf, Blip,
    BlipColor, Session, Timer, BLIP_BLACK, BLIP_WHITE, BLIP_YELLOW};

use moves::*;
use roster::*;
use fighter::*;
use game::*;
use rules::*;
use cpu::*;
use keys::*;
use fight::*;
use flow::*;
use sound::{Sfx, Sounds};

// ---- stage ---------------------------------------------------------------
const WIN_W: i32 = 640;
const WIN_H: i32 = 400;
const FLOOR_Y: f32 = 330.0;
/// How close a fighter's centre may get to the edge. The wall matters:
/// cornering someone is half of what a round is about, so the box has to
/// be tight enough that a corner is a real place to be.
const WALL_MARGIN: f32 = 46.0;

// ---- timing --------------------------------------------------------------
/// One frame at 60fps. Frame data is written in frames because that is
/// the unit fighting games are argued about in, and converted here once.
const F: f32 = 1.0 / 60.0;
/// Forty-five seconds, not the arcade's sixty: at sixty, rounds against a
/// masher ran the full count and the clock decided them. Good play finishes
/// in twenty to thirty.
const ROUND_SECS: f32 = 45.0;
const ROUNDS_TO_WIN: i32 = 2;

// ---- bodies --------------------------------------------------------------
// The width of a fighter, for being hit and for being drawn: the same number,
// or attacks land on air beside what the player sees. The silhouette is built
// to fill it. These are a full-size fighter's; `Archetype::size` scales them.
const BODY_W: f32 = 30.0;
const STAND_H: f32 = 120.0;
const CROUCH_H: f32 = 74.0;
/// Height of a fighter on their back (the raised knee is the highest point),
/// so a flying kick sails over a prone body.
const PRONE_H: f32 = 34.0;

fn main_conf() -> blip::macroquad::window::Conf { window_conf("BRAWLER", WIN_W, WIN_H) }

#[blip::macroquad::main(main_conf)]
async fn main() {
    let mut blip = Blip::new(WIN_W, WIN_H);
    // Whole lit scenes, not lines on black: at full bloom every fighter
    // wears a halo and the names smear.
    blip.set_bloom(0.15);
    // And the interlace's rows swap half as far, or the lit scenes twitter.
    blip.set_interlace(0.5);
    let mut g = Game::new();

    // Say so before the slow part (the themes take a second or two), or the
    // loading screen shows a dead second station.
    web::set_players(TITLE_OPEN);

    let sfx = Sounds::load().await;
    // The themes are synthesised here rather than shipped: as PCM they would
    // be most of the download.
    use blip_assets::brawler::theme_wav;
    let mut music = Jukebox::new(&[|| theme_wav(0), || theme_wav(1), || theme_wav(2),
        || theme_wav(3), || theme_wav(4), || theme_wav(5), || theme_wav(6), || theme_wav(7)]);
    /// The theme of each stage, in stage order.
    const STAGE_TRACK: [usize; STAGES] = [0, 1, 3, 4, 5, 6, 7];
    /// The title and select screens have a loop of their own: silence in
    /// front of a noisy game reads as not loaded.
    const SELECT_TRACK: usize = 2;
    let (mut ambient, mut alarmed): (Option<usize>, bool) = (None, false);
    music.start(SELECT_TRACK).await;
    let mut shot_frame: u32 = 0;

    loop {
        let dt = blip.delta_time;
        g.now += dt;

        if blip.screenshot_mode { shots::pose(&mut g, &mut shot_frame); }
        // The demo lasts one round, or until somebody wants the machine.
        if g.demo {
            let m = read_menu(&mut g);
            let over = !matches!(g.state, State::RoundIntro | State::Fight | State::RoundEnd);
            if over || m[0].fire || m[1].fire {
                g = Game::new();
                web::set_players(TITLE_OPEN);
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        if blip::bot::active() { bot::menus(&mut g, blip::bot::clock()); }
        match g.state {
            State::Title | State::Select => {
                let m = read_menu(&mut g);
                // A title nobody touches starts showing the game off.
                let touched = m.iter().any(|m| m.back || m.fwd || m.fire);
                g.idle = if touched || g.state != State::Title { 0.0 } else { g.idle + dt };
                if let Some(cue) = update_menus(&mut g, m) {
                    g.snd(if cue == Cue::Step { Sfx::Tick } else { Sfx::Gong });
                }
                if g.idle > ATTRACT_AFTER { g.start_demo(); }
            }
            State::Vs => {
                let m = read_menu(&mut g);
                update_vs(&mut g, dt, m[0].fire || m[1].fire);
            }
            State::RoundIntro => {
                // Each round opens with the fighters bowing to each other.
                if !g.bowed {
                    g.bowed = true;
                    g.snd(Sfx::Drums);
                    for f in g.p.iter_mut() { f.act = Act::Bow; f.shown = Act::Bow; f.t = 0.0; }
                }
                for f in g.p.iter_mut() {
                    f.t += dt;
                    if f.act == Act::Bow && f.t >= BOW_TIME { f.act = Act::Idle; f.t = 0.0; }
                    note_handover(f, dt);
                }
                if g.phase.tick(dt) {
                    g.state = State::Fight;
                    g.snd(Sfx::Bell);
                }
            }
            State::Fight => {
                // Here comes a new challenger: player two's punch button, in
                // a game they are not in.
                let solo = g.mode == Mode::Solo && !g.demo && !blip.screenshot_mode;
                #[cfg(not(target_arch = "wasm32"))]
                let solo = solo && !blip::bot::active();
                if solo && any_pressed(P2.punch) { challenge(&mut g); } else { update_fight(&mut g, dt); }
            }
            State::Challenger => update_challenger(&mut g, dt),
            State::Bonus => {
                #[cfg(not(target_arch = "wasm32"))]
                let inp = if blip::bot::active() { bot::fight(&g, dt) } else { human_input(&mut g, 0) };
                #[cfg(target_arch = "wasm32")]
                let inp = human_input(&mut g, 0);
                let was_done = g.bonus_done > 0.0;
                match update_bonus(&mut g, dt, inp) {
                    Some(true) => { g.snd(Sfx::Smash); g.snd_at(Sfx::Cheer, 0.5); }
                    Some(false) => g.snd(Sfx::HitHeavy),
                    None => {}
                }
                if g.bonus_done > 0.0 && !was_done { g.snd(Sfx::Bell); }
                if g.state == State::Vs { g.snd(Sfx::Gong); }
            }
            State::RoundEnd => {
                update_round_end(&mut g, dt);
                if g.state == State::MatchEnd {
                    music.stop();
                    let lost = g.mode == Mode::Solo && g.p[0].rounds < g.p[1].rounds;
                    g.snd(if lost { Sfx::Lose } else { Sfx::Win });
                }
            }
            State::MatchEnd => {
                let m = read_menu(&mut g);
                update_result(&mut g, dt, m[0].fire || m[1].fire);
                match g.state {
                    State::Vs => g.snd(Sfx::Gong),
                    State::Won => { g.snd(Sfx::Gong); g.snd(Sfx::Roar); }
                    _ => {}
                }
            }
            State::Continue => {
                let m = read_menu(&mut g);
                let before = g.phase.remaining().ceil();
                update_continue(&mut g, dt, m[0].fire);
                match g.state {
                    State::Vs => g.snd(Sfx::Gong),
                    State::Continue if g.phase.remaining().ceil() < before => g.snd(Sfx::Tick),
                    _ => {}
                }
            }
            State::Over | State::Won => {
                g.phase.tick(dt);
                // Either player can take it back to the title.
                let m = read_menu(&mut g);
                if !g.phase.active() && (m[0].fire || m[1].fire) {
                    g = Game::new();
                    web::set_players(TITLE_OPEN);
                }
            }
        }

        // The rules only name the sounds they want; they are played here.
        // (The demo is silent: it runs behind the title music, and behind
        // the kiosk's coin screen, for as long as nobody is playing.)
        let quiet = g.demo;
        for (snd, volume) in g.sounds.drain(..) { if !quiet { sfx.play(snd, volume); } }

        // The room under the fight: a crowd, with crickets behind it by the
        // river and the city under it on the roof; in the fortress only wind.
        let room = (!g.demo && matches!(g.state, State::Vs | State::RoundIntro | State::Fight | State::RoundEnd | State::Bonus))
            .then(|| match g.stage { 5 => 1, 4 => 2, 6 => 3, _ => 0 });
        if room != ambient {
            match room {
                Some(k) => blip::audio::play_ambient(&sfx.ambience[k]),
                None => blip::audio::stop_ambient(),
            }
            ambient = room;
        }
        // And a heart under it when the player is nearly out (in versus,
        // when either of them is).
        let low = |f: &Fighter| f.health > 0 && f.health * 5 <= f.arch().health;
        let danger = g.state == State::Fight && !g.demo
            && (low(&g.p[0]) || (g.mode == Mode::Versus && low(&g.p[1])));
        if danger != alarmed {
            if danger { blip::audio::play_alert(&sfx.heart); } else { blip::audio::stop_alert(); }
            alarmed = danger;
        }

        let want = match g.state {
            _ if g.demo => Some(SELECT_TRACK),
            State::Title | State::Select => Some(SELECT_TRACK),
            State::Vs | State::RoundIntro | State::Fight | State::RoundEnd | State::Bonus => {
                Some(STAGE_TRACK[g.stage % STAGES])
            }
            _ => None,
        };
        // A stage's theme is rendered when it is first wanted, which is at a
        // billing, a round's intro or the start of a bonus round: the half
        // second it takes hides behind a still picture, never inside a fight.
        // Warming all seven up on the title froze it seven times.
        match want {
            Some(track) if g.state != State::Fight && !music.ready(track) => music.start(track).await,
            Some(track) => music.play(track),
            None => {}
        }

        if g.fade > 0.0 { g.fade -= dt; }
        blip.clear(BLIP_BLACK);
        draw::draw(&blip, &g);
        blip.next_frame(60).await;
    }
}

/// A menu step for one player, debounced by hand: a held direction moves the
/// cursor once. `who` picks whose keys are read, so two cursors share a
/// screen.
fn menu_step(g: &mut Game, who: usize, back: bool) -> bool {
    let k = pad(g.mode, who);
    let held = if back { any_held(k.left) || any_held(k.up) }
               else { any_held(k.right) || any_held(k.down) };
    let slot = &mut g.sel_held[who][usize::from(!back)];
    let stepped = held && !*slot;
    *slot = held;
    stepped
}

/// The same, for a player's confirm — any of their attack buttons.
fn menu_fire(g: &mut Game, who: usize) -> bool {
    let k = pad(g.mode, who);
    let held = any_held(k.punch) || any_held(k.kick);
    let slot = &mut g.sel_fire[who];
    let fired = held && !*slot;
    *slot = held;
    fired
}

#[cfg(test)]
mod tests;
