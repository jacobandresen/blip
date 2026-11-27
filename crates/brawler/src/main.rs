//! One-on-one fighter with fourteen characters and CPU or local two-player modes.
//! Attack heights, frame data and poses share the same move definitions.

// The effects and music are made by build.rs; the tests check them here.
#[cfg(test)]
mod tunes;
mod moves;
mod roster;
mod fighter;
mod game;
mod rules;
mod cpu;
mod plan;
mod keys;
mod fight;
mod flow;
mod sound;
mod shots;
mod draw;
#[cfg(not(target_arch = "wasm32"))]
mod export;
mod bot;

use blip::macroquad::input::KeyCode;
use blip::input::{BLIP_KEY_A, BLIP_KEY_BUTTON2, BLIP_KEY_D,
    BLIP_KEY_DOWN, BLIP_KEY_F, BLIP_KEY_G, BLIP_KEY_J, BLIP_KEY_K, BLIP_KEY_LEFT,
    BLIP_KEY_RIGHT, BLIP_KEY_S, BLIP_KEY_SPACE, BLIP_KEY_UP, BLIP_KEY_W};
use blip::{clamp, Jukebox, rand_int, rects_overlap, segment_rect_overlap, web,
    window_conf, Blip,
    BlipColor, Session, Timer, BLIP_BLACK, BLIP_WHITE, BLIP_YELLOW};

use moves::*;
use roster::*;
use fighter::*;
use game::*;
use rules::*;
use cpu::*;
use plan::*;
use keys::*;
use fight::*;
use flow::*;
use sound::{throat_of, Sfx, Sounds};

// ---- stage ---------------------------------------------------------------
const WIN_W: i32 = 640;
const WIN_H: i32 = 400;
const FLOOR_Y: f32 = 330.0;
/// How close a fighter's centre may get to the edge. The wall matters:
/// cornering someone is half of what a round is about, so the box has to
/// be tight enough that a corner is a real place to be.
const WALL_MARGIN: f32 = 46.0;

// ---- timing --------------------------------------------------------------
/// One frame at 60 Hz; move timings use frames.
const F: f32 = 1.0 / 60.0;
/// 45 seconds; at 60 seconds, rounds too often timed out against mashers.
const ROUND_SECS: f32 = 45.0;
const ROUNDS_TO_WIN: i32 = 2;

// ---- bodies --------------------------------------------------------------
// The drawn silhouette fills this hit width; `Archetype::size` scales it.
const BODY_W: f32 = 30.0;
const STAND_H: f32 = 120.0;
const CROUCH_H: f32 = 74.0;
/// Height of a fighter on their back (the raised knee is the highest point),
/// so a flying kick sails over a prone body.
const PRONE_H: f32 = 34.0;

fn main_conf() -> blip::macroquad::window::Conf { window_conf("BRAWLER", WIN_W, WIN_H) }

#[blip::macroquad::main(main_conf)]
async fn main() {
    blip::bot::playthroughs(bot::PLAYTHROUGHS);
    let mut blip = Blip::new(WIN_W, WIN_H);
    // Whole lit scenes, not lines on black: at full bloom every fighter
    // wears a halo and the names smear.
    blip.set_bloom(0.15);
    // And the interlace's rows swap half as far, or the lit scenes twitter.
    blip.set_interlace(0.5);
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(folder) = std::env::var_os("BLIP_EXPORT_ART") {
        export::pose_art(&blip, std::path::Path::new(&folder));
        return;
    }
    let mut g = Game::new();

    // Say so before the slow part (the themes take a second or two), or the
    // loading screen shows a dead second station.
    web::set_players(TITLE_OPEN);

    let sfx = Sounds::load().await;
    let mut music = Jukebox::new("brawler", &[
        "DOCK", "TEMPLE", "SELECT", "AIRBASE", "BATHHOUSE", "VILLAGE", "FORTRESS", "ROOFTOP", "HOOD", "WALKOUT", "PAGODA", "COLOSSEUM",
    ]);
    music.play(SELECT_TRACK);
    let mut room = Room::default();
    let mut shot_frame: u32 = 0;
    // Player two's punch in a solo game away from the fight (choosing, the intro, a round's end): the challenge
    // comes as the next fight starts.
    let mut p2_waiting = false;

    loop {
        let dt = blip.delta_time;
        g.now += dt;
        let solo = g.mode == Mode::Solo && !g.demo && !blip.screenshot_mode && !blip::bot::active();
        if !solo || g.state == State::Title { p2_waiting = false; }
        else if matches!(g.state, State::Select | State::Vs | State::RoundIntro | State::RoundEnd) && any_pressed(P2.punch) { p2_waiting = true; }

        if blip.screenshot_mode { shots::pose(&mut g, &mut shot_frame); }
        // The demo lasts one round, or until somebody wants the machine.
        if g.demo {
            let m = read_menu(&mut g);
            let over = !matches!(g.state, State::RoundIntro | State::Fight | State::RoundEnd);
            if over || m[0].fire || m[1].fire { g = fresh_game(); }
        }

        if blip::bot::active() { bot::menus(&mut g); }
        match g.state {
            State::Title | State::Select => update_title_menus(&mut g, dt),
            State::Vs => {
                let m = read_menu(&mut g);
                update_vs(&mut g, dt, m[0].fire || m[1].fire);
            }
            State::RoundIntro => update_round_intro(&mut g, dt),
            // Here comes a new challenger: player two's punch button, in a game they are not in.
            State::Fight if solo && (any_pressed(P2.punch) || p2_waiting) => { p2_waiting = false; challenge(&mut g); }
            State::Fight => update_fight(&mut g, dt),
            State::Challenger => update_challenger(&mut g, dt),
            State::Bonus => update_bonus_stage(&mut g, dt),
            State::RoundEnd => {
                update_round_end(&mut g, dt);
                if g.state == State::MatchEnd {
                    music.stop();
                    let lost = g.mode == Mode::Solo && g.p[0].rounds < g.p[1].rounds;
                    g.snd(if lost { Sfx::Lose } else { Sfx::Win });
                }
            }
            State::MatchEnd => result_screen(&mut g, dt),
            State::Continue => continue_screen(&mut g, dt),
            State::Over | State::Won => last_screen(&mut g, dt),
        }

        // The rules only name the sounds they want; they are played here.
        // (The demo is silent: it runs behind the title music, and behind
        // the kiosk's coin screen, for as long as nobody is playing.)
        let quiet = g.demo;
        for (snd, volume) in g.sounds.drain(..) { if !quiet { sfx.play(snd, volume); } }
        room.update(&g, &sfx);
        if let Some(track) = track(&g) { music.play(track); }

        if g.fade > 0.0 { g.fade -= dt; }
        blip.clear(BLIP_BLACK);
        draw::draw(&blip, &g);
        blip.next_frame().await;
    }
}

/// The theme of each stage, in stage order.
const STAGE_TRACK: [usize; STAGES] = [0, 1, 3, 4, 5, 6, 7, 8, 10, 0, 11];
/// The title and select screens have a loop of their own: silence in
/// front of a noisy game reads as not loaded.
const SELECT_TRACK: usize = 9;

/// A machine back at its title, the second station open again.
fn fresh_game() -> Game {
    let g = Game::new();
    web::set_players(TITLE_OPEN);
    g
}

/// The tune for where the game is: the select loop on the title and in the demo, the stage's theme around a fight,
/// and whatever is playing on (or after) the result screens.
fn track(g: &Game) -> Option<usize> {
    if g.demo || matches!(g.state, State::Title | State::Select) { Some(SELECT_TRACK) }
    else if matches!(g.state, State::Vs | State::RoundIntro | State::Fight | State::RoundEnd | State::Bonus) { Some(STAGE_TRACK[g.stage % STAGES]) }
    else { None }
}

/// The title and the fighter select: a title nobody touches starts showing the game off.
fn update_title_menus(g: &mut Game, dt: f32) {
    let m = read_menu(g);
    let touched = m.iter().any(|m| m.back || m.fwd || m.fire);
    g.idle = if touched || g.state != State::Title { 0.0 } else { g.idle + dt };
    if let Some(cue) = update_menus(g, m) {
        g.snd(if cue == Cue::Step { Sfx::Tick } else { Sfx::Gong });
    }
    if g.idle > ATTRACT_AFTER { g.start_demo(); }
}

/// Each round opens with the fighters bowing to each other, then the bell.
fn update_round_intro(g: &mut Game, dt: f32) {
    if !g.bowed {
        g.bowed = true;
        g.snd(Sfx::Drums);
        for f in g.p.iter_mut() { f.act = Act::Bow; f.shown = Act::Bow; f.t = 0.0; }
    }
    let mut cues = vec![];
    // Who faces whom is known before the bell: DRAKE warms up against LANCER.
    face_off(&mut g.p);
    for f in g.p.iter_mut() {
        f.t += dt;
        // DRAKE's opening flourish of the nunchaku hums as it turns.
        let warm = draw::intro_now(f) == draw::Intro::WarmUp;
        if formless(f.who) && f.act == Act::Bow && !warm && f.t > 0.15 && f.t < 0.6
            && (f.t / (4.0 * F)).floor() != ((f.t - dt) / (4.0 * F)).floor() { cues.push((Sfx::Swing, 0.35)); }
        // DRAKE's cry as he beckons, the round's first sound from him.
        if formless(f.who) && f.act == Act::Bow && !warm && f.reached(0.7, dt) {
            cues.push((Sfx::Kiai(throat_of(f.who)), 0.9));
        }
        // Knuckles crack in the warm-up, twice.
        if f.act == Act::Bow && warm && [1.02f32, 1.24].iter().any(|&at| f.reached(at, dt)) {
            cues.push((Sfx::Tick, 0.35));
        }
        if f.act == Act::Bow && f.t >= BOW_TIME { f.act = Act::Idle; f.t = 0.0; }
        note_handover(f, dt);
    }
    for (s, v) in cues { g.snd_at(s, v); }
    if g.phase.tick(dt) {
        g.state = State::Fight;
        g.snd(Sfx::Bell);
    }
}

/// The board-breaking bonus stage between fights.
fn update_bonus_stage(g: &mut Game, dt: f32) {
    let inp = if blip::bot::active() { bot::fight(g, dt) } else { human_input(g, 0) };
    let was_done = g.bonus_done > 0.0;
    match update_bonus(g, dt, inp) {
        // (DRAKE's boards crack as well as smash.)
        Some(true) => { g.snd(Sfx::Smash); g.snd_at(Sfx::Cheer, 0.5); if formless(g.p[0].who) { g.snd(Sfx::Tick); } }
        Some(false) => g.snd(Sfx::HitHeavy),
        None => {}
    }
    if g.bonus_done > 0.0 && !was_done { g.snd(Sfx::Bell); }
    if g.state == State::Vs { g.snd(Sfx::Gong); }
}

/// The room under the fight: a crowd, with crickets behind it by the river and the city under it on the roof (in the
/// fortress only wind), and a heart when the player is nearly out (in versus, when either of them is).
#[derive(Default)]
struct Room { ambient: Option<usize>, alarmed: bool }

impl Room {
    fn update(&mut self, g: &Game, sfx: &Sounds) {
        let ambient = (!g.demo && matches!(g.state, State::Vs | State::RoundIntro | State::Fight | State::RoundEnd | State::Bonus))
            .then_some(match g.stage { 5 | 8 => 1, 4 => 2, 6 => 3, _ => 0 });
        if ambient != self.ambient {
            match ambient {
                Some(k) => blip::audio::play_ambient(&sfx.ambience[k]),
                None => blip::audio::stop_ambient(),
            }
            self.ambient = ambient;
        }
        let low = |f: &Fighter| f.health > 0 && f.health * 5 <= f.arch().health;
        let danger = g.state == State::Fight && !g.demo && (low(&g.p[0]) || (g.mode == Mode::Versus && low(&g.p[1])));
        if danger != self.alarmed {
            if danger { blip::audio::play_alert(&sfx.heart); } else { blip::audio::stop_alert(); }
            self.alarmed = danger;
        }
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

/// Whether a player's kick is down: confirming with it picks the alternate colours.
fn menu_kick(g: &Game, who: usize) -> bool { any_held(pad(g.mode, who).kick) }

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
mod tests;/// The match's result: on to the next opponent (the gong), or the tournament won (the gong and the crowd's roar).
fn result_screen(g: &mut Game, dt: f32) {
    let m = read_menu(g);
    update_result(g, dt, m[0].fire || m[1].fire);
    match g.state {
        State::Vs => g.snd(Sfx::Gong),
        State::Won => { g.snd(Sfx::Gong); g.snd(Sfx::Roar); }
        _ => {}
    }
}

/// The continue count: each second ticks, and fire takes the fight on (the gong).
fn continue_screen(g: &mut Game, dt: f32) {
    let m = read_menu(g);
    update_continue(g, dt, m[0].fire);
    match g.state {
        State::Vs => g.snd(Sfx::Gong),
        State::Continue if g.cont.ticked() => g.snd(Sfx::Tick),
        _ => {}
    }
}

/// Game over or the tournament won: once the screen has had its moment, either player takes it back to the title.
fn last_screen(g: &mut Game, dt: f32) {
    g.phase.tick(dt);
    let m = read_menu(g);
    if !g.phase.active() && (m[0].fire || m[1].fire) { *g = fresh_game(); }
}

