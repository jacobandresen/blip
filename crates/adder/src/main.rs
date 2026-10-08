//! Adder (Viper), a pit snake: 360 degrees of movement, holes in the bodies
//! that pay to pass through, and up to six vipers in one pit — two of them
//! people, the rest CPU. The rules are in `arena`, the brains in `cpu`.

mod arena;
mod cpu;

use blip::input::{
    btn1_pressed, key_active, key_pressed, BLIP_KEY_A, BLIP_KEY_D,
    BLIP_KEY_J, BLIP_KEY_LEFT, BLIP_KEY_RIGHT, BLIP_KEY_SPACE,
};
use blip::macroquad::input::KeyCode;
use blip::{
    play_sfx, web, window_conf, Blip, BlipColor, Jukebox, Timer,
    GAME_OVER_MIN_WAIT, BLIP_BLACK, BLIP_RED, BLIP_WHITE,
};

use arena::{Event, Steer, World, SEATS};

const WIN_W: i32 = 480;
const WIN_H: i32 = 540;
const FAST_ROUND: i32 = 4; // the tune changes up from here
const DEMO_SNAKES: usize = 3;

#[derive(Copy, Clone, PartialEq, Eq)]
enum State { Title, Play, Over }

struct Sounds {
    through: blip::BlipSound,
    kill: blip::BlipSound,
    round: blip::BlipSound,
    game_over: blip::BlipSound,
}

struct Game {
    world: World,
    /// The pit the title screen plays itself with.
    demo: World,
    state: State,
    timer: Timer,
    humans: usize,
}

impl Game {
    fn new() -> Self {
        Self {
            world: World::new(1),
            demo: World::demo(DEMO_SNAKES),
            state: State::Title,
            timer: Timer::default(),
            humans: 1,
        }
    }

    fn start(&mut self) {
        self.world = World::new(self.humans);
        self.state = State::Play;
    }

    /// The music for where the game is: the slow coil, then the hunt.
    fn track(&self) -> usize {
        if self.state == State::Play && self.world.round >= FAST_ROUND { 1 } else { 0 }
    }
}

// ---- input ---------------------------------------------------------------
/// Player one uses A/D (or the arrows, alone) and Space to start; player two uses arrows and J to join.
fn seat_keys(seat: usize) -> [KeyCode; 3] {
    if seat == 0 {
        [BLIP_KEY_A, BLIP_KEY_D, BLIP_KEY_SPACE]
    } else {
        [BLIP_KEY_LEFT, BLIP_KEY_RIGHT, BLIP_KEY_J]
    }
}

/// Left and right curve the viper; it keeps its heading when neither is held.
fn human_steer(seat: usize, solo: bool) -> Steer {
    let [left, right, _] = seat_keys(seat);
    // Alone, the arrows turn too; once player two joins they are theirs.
    let arrows = solo && seat == 0;
    let mut turn = match (key_active(left) || (arrows && key_active(BLIP_KEY_LEFT)),
                          key_active(right) || (arrows && key_active(BLIP_KEY_RIGHT))) {
        (true, false) => -1.0,
        (false, true) => 1.0,
        _ => 0.0,
    };
    if seat == 0 && web::controls() == web::Controls::Touch {
        if let Some((x, _, _)) = web::touch_fraction(0) {
            turn = if x < 0.45 { -1.0 } else if x > 0.55 { 1.0 } else { 0.0 };
        }
    }
    Steer { turn }
}

/// Intent for every seat this frame: the human seats answer to their keys and
/// everything else to a brain.
fn gather(g: &Game) -> [Steer; SEATS] {
    autopilot(&g.world);
    let mut steers = [Steer::default(); SEATS];
    for (i, s) in g.world.snakes.iter().enumerate() {
        if !s.alive { continue; }
        let seat = s.seat.min(SEATS - 1);
        steers[seat] = if s.human { human_steer(s.seat, g.humans < 2) } else { cpu::steer(&g.world, i) };
    }
    steers
}

/// Intent for a pit nobody is holding: every snake answers to its brain.
fn cpu_steers(w: &World) -> [Steer; SEATS] {
    let mut steers = [Steer::default(); SEATS];
    for (i, s) in w.snakes.iter().enumerate() {
        if s.alive { steers[s.seat.min(SEATS - 1)] = cpu::steer(w, i); }
    }
    steers
}

/// The native autopilot holds only turn keys, through the normal input path.
#[cfg(not(target_arch = "wasm32"))]
fn autopilot(w: &World) {
    if !blip::bot::active() { return; }
    let mut held = Vec::new();
    for (i, s) in w.snakes.iter().enumerate() {
        if !s.human || !s.alive { continue; }
        let st = cpu::steer(w, i);
        let [left, right, _] = seat_keys(s.seat);
        if st.turn < -0.15 { held.push(left); }
        if st.turn > 0.15 { held.push(right); }
    }
    blip::bot::hold(&held);
}

#[cfg(target_arch = "wasm32")]
fn autopilot(_w: &World) {}

// ---- the loop ------------------------------------------------------------
fn update_title(g: &mut Game, dt: f32) {
    g.demo.step(dt, &cpu_steers(&g.demo));
    g.demo.events.clear();
    #[cfg(not(target_arch = "wasm32"))]
    if blip::bot::active() { blip::bot::hold(&[BLIP_KEY_SPACE]); }
    if btn1_pressed() {
        web::spend_coin();
        g.start();
    }
}

fn update_play(g: &mut Game, dt: f32, sfx: &Sounds) {
    // J joins player two and lights the kiosk's second station.
    if key_pressed(seat_keys(1)[2]) && g.world.join_human(1) {
        g.humans = 2;
        web::set_mode(true);
    }

    let steers = gather(g);
    g.world.step(dt, &steers);

    for e in g.world.events.drain(..) {
        match e {
            Event::Through => play_sfx(&sfx.through),
            Event::Kill => play_sfx(&sfx.kill),
            Event::RoundWin => play_sfx(&sfx.round),
            Event::Die => {
                play_sfx(&sfx.game_over);
                web::haptic();
            }
        }
    }

    if g.world.over {
        g.timer.start(GAME_OVER_MIN_WAIT);
        g.state = State::Over;
        web::report_score(g.world.score_of(0));
    }
}

fn update_over(g: &mut Game, dt: f32) {
    g.timer.tick(dt);
    if g.timer.active() || !btn1_pressed() { return; }
    web::spend_coin();
    g.start();
}

fn conf() -> blip::macroquad::window::Conf {
    window_conf("ADDER", WIN_W, WIN_H)
}

const THROUGH_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/through.wav"));
const STRIKE_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/strike.wav"));
const ROUND_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/pit.wav"));
const GAME_OVER_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/game_over.wav"));

#[blip::macroquad::main(conf)]
async fn main() {
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g = Game::new();

    let sfx = Sounds {
        through: blip::audio::load_sound(THROUGH_WAV).await,
        kill: blip::audio::load_sound(STRIKE_WAV).await,
        round: blip::audio::load_sound(ROUND_WAV).await,
        game_over: blip::audio::load_sound(GAME_OVER_WAV).await,
    };
    let mut music = Jukebox::new(&[blip_assets::adder::coil_wav, blip_assets::adder::hunt_wav]);
    music.start(0).await;

    let mut shot_frame: u32 = 0;

    loop {
        let dt = blip.delta_time;

        if blip.screenshot_mode {
            shot_frame += 1;
            if shot_frame == 1 { g.start(); }
        }

        match g.state {
            State::Title => update_title(&mut g, dt),
            State::Play => update_play(&mut g, dt, &sfx),
            State::Over => update_over(&mut g, dt),
        }

        music.play(g.track());
        if g.state != State::Play { music.warm_up().await; }

        blip.clear(BLIP_BLACK);
        match g.state {
            State::Title => draw_title(&blip, &g),
            State::Play => draw_play(&blip, &g),
            State::Over => draw_over(&blip, &g),
        }

        blip.next_frame(60).await;
    }
}

// ---- drawing -------------------------------------------------------------
const BANNER: f32 = 1.6; // a round's name stays up this long

const EARTH: BlipColor = BlipColor { r: 0.085, g: 0.07, b: 0.05, a: 1.0 };
const DUST: BlipColor = BlipColor { r: 0.145, g: 0.12, b: 0.085, a: 1.0 };
const TOPSOIL: BlipColor = BlipColor { r: 0.40, g: 0.29, b: 0.15, a: 1.0 };
const HOLE: BlipColor = BlipColor { r: 1.0, g: 0.86, b: 0.34, a: 1.0 };
const SAND: BlipColor = BlipColor { r: 0.60, g: 0.52, b: 0.36, a: 1.0 };

/// The pit: dry earth inside a wall of packed earth, flecked so the floor is not
/// a flat field to read a heading against.
fn draw_pit(blip: &Blip) {
    blip.fill_rect(0.0, 0.0, WIN_W as f32, WIN_H as f32, EARTH);
    for i in 0..110u32 {
        let h = i.wrapping_mul(2654435761).wrapping_add(12345);
        let x = arena::PIT_X + (h % 456) as f32;
        let y = arena::PIT_Y + ((h >> 9) % 488) as f32;
        blip.fill_rect(x, y, 2.0, 2.0, DUST);
    }
    blip.fill_rect(0.0, 28.0, WIN_W as f32, arena::PIT_Y - 28.0, TOPSOIL);
    blip.fill_rect(0.0, arena::PIT_Y + arena::PIT_H, WIN_W as f32, WIN_H as f32 - arena::PIT_Y - arena::PIT_H, TOPSOIL);
    blip.fill_rect(0.0, 28.0, arena::PIT_X, WIN_H as f32 - 28.0, TOPSOIL);
    blip.fill_rect(arena::PIT_X + arena::PIT_W, 28.0, arena::PIT_X, WIN_H as f32 - 28.0, TOPSOIL);
}

/// Dead trails stay solid; passable gaps keep the seat's colour at 23% brightness.
fn draw_snake(blip: &Blip, s: &arena::Snake) {
    let c = s.colour();
    let gap = BlipColor { r: c.r * 0.23, g: c.g * 0.23, b: c.b * 0.23, ..c };
    let thick = arena::BODY_R * 2.0;
    for i in (1..s.path.len()).rev() {
        let (a, b) = (s.path[i], s.path[i - 1]);
        let col = if s.in_hole(i) || s.in_hole(i - 1) { gap } else { c };
        blip.draw_line_ex(a.x, a.y, b.x, b.y, thick, col);
    }
    if !s.alive { return; }
    if let Some(p) = s.path.front() {
        blip.draw_line_ex(s.head.x, s.head.y, p.x, p.y, thick, c);
    }
    let head_len = arena::BODY_R * 3.0;
    blip.draw_line_ex(s.head.x - s.heading.cos() * head_len,
        s.head.y - s.heading.sin() * head_len, s.head.x, s.head.y, thick + 1.0, c);
    if s.human {
        let label = if s.seat == 0 { "YOU" } else { "P2" };
        let x = (s.head.x - 10.0).clamp(arena::PIT_X + 2.0, arena::PIT_X + arena::PIT_W - 24.0);
        let y = (s.head.y - 16.0).clamp(arena::PIT_Y + 2.0, arena::PIT_Y + arena::PIT_H - 4.0);
        blip.draw_text_outlined(label, x, y, 2.0, BLIP_WHITE, BLIP_BLACK);
    }
}

/// Who else is in the pit: a chip in each snake's colour and its score down the
/// right-hand side. Player one is on the top bar, with everyone else's work.
fn draw_standings(blip: &Blip, w: &World) {
    let mut y = arena::PIT_Y + 10.0;
    for s in &w.snakes {
        if s.seat == 0 { continue; }
        let c = if s.alive { s.colour() } else { BlipColor { r: 0.36, g: 0.36, b: 0.36, a: 1.0 } };
        blip.fill_rect(arena::PIT_X + arena::PIT_W - 8.0, y + 2.0, 5.0, 10.0, c);
        blip.draw_text_outlined(&format!("{} {}", s.name(), s.score),
            arena::PIT_X + arena::PIT_W - 92.0, y, 2.0, c, BLIP_BLACK);
        y += 16.0;
    }
}

fn draw_world(blip: &Blip, w: &World) {
    draw_pit(blip);
    for s in &w.snakes { draw_snake(blip, s); }
    w.fx.draw(blip);
}

fn draw_play(blip: &Blip, g: &Game) {
    draw_world(blip, &g.world);
    draw_standings(blip, &g.world);
    if g.world.round_wait > 0.0 {
        let won = g.world.snakes.iter().any(|s| s.alive && s.human);
        blip.draw_centered(if won { "YOU WIN" } else { "ROUND LOST" },
            arena::PIT_Y + arena::PIT_H / 2.0, 4.0, if won { HOLE } else { BLIP_RED });
    } else if g.world.banner_t < BANNER {
        let a = (1.0 - g.world.banner_t / BANNER).min(1.0);
        blip.draw_centered(&format!("ROUND {}", g.world.round), (arena::PIT_Y + arena::PIT_H / 2.0) as f32, 4.0,
            BlipColor { a, ..HOLE });
    }
    blip.draw_hud(g.world.score_of(0), g.world.lives_of(0));
}

/// The title screen is an attract mode: a real pit, three CPUs, and the game
/// showing its own movement and its own holes while it waits for a coin.
fn draw_title(blip: &Blip, g: &Game) {
    draw_world(blip, &g.demo);
    draw_standings(blip, &g.demo);
    let veil = BlipColor { r: 0.0, g: 0.0, b: 0.0, a: 0.55 };
    blip.fill_rect(0.0, 46.0, WIN_W as f32, 92.0, veil);
    blip.fill_rect(0.0, 384.0, WIN_W as f32, 136.0, veil);
    blip.draw_text_outlined("ADDER", 96.0, 62.0, 6.0, arena::seat_colour(0), BLIP_BLACK);
    blip.draw_hi(&web::high_score(), 116.0, HOLE);
    let by = web::controls();
    blip.draw_centered(by.pick("SPACE TO START", "PRESS FIRE TO START", "TAP TO START"), 404.0, 3.0, BLIP_WHITE);
    blip.draw_centered(by.pick("GREEN VIPER: A/D TURN", "GREEN VIPER: STICK LEFT/RIGHT", "GREEN VIPER: DRAG LEFT/RIGHT"),
        430.0, 2.0, SAND);
    blip.draw_centered("CONSTANT MOVEMENT", 450.0, 2.0, SAND);
    blip.draw_centered("GROWING TRAILS STAY ALL ROUND", 468.0, 2.0, SAND);
    blip.draw_centered("DIM GAPS ARE PASSABLE", 486.0, 2.0, HOLE);
    blip.draw_centered("P2: LEFT/RIGHT TURN, J TO JOIN", 504.0, 2.0, arena::seat_colour(1));
}

fn draw_over(blip: &Blip, g: &Game) {
    blip.clear(BLIP_BLACK);
    blip.draw_game_over(g.world.score_of(0), &web::high_score(), BLIP_RED, arena::seat_colour(0), HOLE,
        !g.timer.active());
    blip.draw_centered(&format!("ROUND {}   LENGTH {}", g.world.round, g.world.snakes.first().map_or(0, |s| s.len_s)),
        (WIN_H / 2 + 54) as f32, 2.0, SAND);
}
