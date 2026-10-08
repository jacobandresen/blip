//! Adder (Viper), a pit snake: 360 degrees of movement, holes in the bodies
//! that pay to pass through, and up to six vipers in one pit — two of them
//! people, the rest CPU. The rules are in `arena`, the brains in `cpu`.

mod arena;
mod cpu;

use blip::input::{
    btn1_pressed, key_active, key_pressed, BLIP_KEY_A, BLIP_KEY_BUTTON2, BLIP_KEY_D,
    BLIP_KEY_DOWN, BLIP_KEY_J, BLIP_KEY_LEFT, BLIP_KEY_RIGHT, BLIP_KEY_S, BLIP_KEY_SPACE,
    BLIP_KEY_UP, BLIP_KEY_W,
};
use blip::macroquad::input::KeyCode;
use blip::macroquad::texture::Texture2D;
use blip::{
    angle_diff, load_png, play_sfx, web, window_conf, Blip, BlipColor, Jukebox, Timer,
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
    eat: blip::BlipSound,
    through: blip::BlipSound,
    shed: blip::BlipSound,
    strike: blip::BlipSound,
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
/// The keys a seat holds: up, down, left, right, and the fire cap, which is the
/// throttle. Seat one is WASD so the arrows belong to player two, whose cap is
/// the deck's second one.
fn seat_keys(seat: usize) -> [KeyCode; 5] {
    if seat == 0 {
        [BLIP_KEY_W, BLIP_KEY_S, BLIP_KEY_A, BLIP_KEY_D, BLIP_KEY_SPACE]
    } else {
        [BLIP_KEY_UP, BLIP_KEY_DOWN, BLIP_KEY_LEFT, BLIP_KEY_RIGHT, BLIP_KEY_J]
    }
}

/// What a person is asking of a snake: the held directions become a heading and
/// the cap is the throttle. A heading is a vector, so two keys held together are
/// a diagonal — which is why this is a direction to point the head at rather
/// than a left and a right.
fn human_steer(w: &World, seat: usize) -> Steer {
    let [up, down, left, right, fire] = seat_keys(seat);
    let (mut vx, mut vy) = (0.0f32, 0.0f32);
    if key_active(left) { vx -= 1.0; }
    if key_active(right) { vx += 1.0; }
    if key_active(up) { vy -= 1.0; }
    if key_active(down) { vy += 1.0; }
    let boost = key_active(fire) || (seat == 0 && key_active(BLIP_KEY_BUTTON2));
    if vx == 0.0 && vy == 0.0 { return Steer { turn: 0.0, boost }; }
    let want = vy.atan2(vx);
    let heading = w.snakes.iter().find(|s| s.seat == seat).map_or(0.0, |s| s.heading);
    Steer { turn: (angle_diff(heading, want) / 0.25).clamp(-1.0, 1.0), boost }
}

/// Intent for every seat this frame: the human seats answer to their keys and
/// everything else to a brain.
fn gather(g: &Game) -> [Steer; SEATS] {
    autopilot(&g.world);
    let mut steers = [Steer::default(); SEATS];
    for (i, s) in g.world.snakes.iter().enumerate() {
        let seat = s.seat.min(SEATS - 1);
        steers[seat] = if s.human { human_steer(&g.world, s.seat) } else { cpu::steer(&g.world, i) };
    }
    steers
}

/// Intent for a pit nobody is holding: every snake answers to its brain.
fn cpu_steers(w: &World) -> [Steer; SEATS] {
    let mut steers = [Steer::default(); SEATS];
    for (i, s) in w.snakes.iter().enumerate() { steers[s.seat.min(SEATS - 1)] = cpu::steer(w, i); }
    steers
}

/// Hold the keys pointing nearest to `dir`. This is how a pad turns a
/// continuous heading: hold the cardinal a quarter turn off and let it go when
/// the nose arrives, because holding the cardinal you are already on asks for no
/// turn at all.
fn hold_toward(dir: f32, keys: [KeyCode; 5], held: &mut Vec<KeyCode>) {
    if dir.cos() > 0.5 { held.push(keys[3]); } else if dir.cos() < -0.5 { held.push(keys[2]); }
    if dir.sin() > 0.5 { held.push(keys[1]); } else if dir.sin() < -0.5 { held.push(keys[0]); }
}

/// The native autopilot (BLIP_BOT=1) drives the human seats the way a person
/// does: it holds the keys a thumb would, and the cap when it wants the
/// throttle. BLIP_BOT_FUZZ replaces the lot.
#[cfg(not(target_arch = "wasm32"))]
fn autopilot(w: &World) {
    if !blip::bot::active() { return; }
    for (i, s) in w.snakes.iter().enumerate() {
        if !s.human || !s.alive { continue; }
        let st = cpu::steer(w, i);
        let keys = seat_keys(s.seat);
        let mut held = Vec::new();
        if st.turn < -0.15 {
            hold_toward(s.heading - std::f32::consts::FRAC_PI_2, keys, &mut held);
        } else if st.turn > 0.15 {
            hold_toward(s.heading + std::f32::consts::FRAC_PI_2, keys, &mut held);
        }
        if st.boost { held.push(keys[4]); }
        blip::bot::hold(&held);
    }
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
    // Player two drops in on their own keys; the kiosk is told, so its deck can
    // light the second station.
    let p2 = seat_keys(1);
    if p2.iter().any(|k| key_pressed(*k)) && g.world.join_human(1) {
        g.humans = 2;
        web::set_mode(true);
    }

    let steers = gather(g);
    g.world.step(dt, &steers);

    for e in g.world.events.drain(..) {
        match e {
            Event::Eat => play_sfx(&sfx.eat),
            Event::Through => play_sfx(&sfx.through),
            Event::Shed => play_sfx(&sfx.shed),
            Event::Kill => play_sfx(&sfx.strike),
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

const EGG_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/egg.png"));
const EAT_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/eat.wav"));
const THROUGH_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/through.wav"));
const SHED_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/shed.wav"));
const STRIKE_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/strike.wav"));
const ROUND_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/pit.wav"));
const GAME_OVER_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/game_over.wav"));

#[blip::macroquad::main(conf)]
async fn main() {
    let mut blip = Blip::new(WIN_W, WIN_H);
    let mut g = Game::new();

    let egg = load_png(EGG_PNG);

    let sfx = Sounds {
        eat: blip::audio::load_sound(EAT_WAV).await,
        through: blip::audio::load_sound(THROUGH_WAV).await,
        shed: blip::audio::load_sound(SHED_WAV).await,
        strike: blip::audio::load_sound(STRIKE_WAV).await,
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
            State::Title => draw_title(&blip, &g, &egg),
            State::Play => draw_play(&blip, &g, &egg),
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
const SKIN: BlipColor = BlipColor { r: 0.86, g: 0.81, b: 0.65, a: 1.0 };
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

fn band_of(c: BlipColor) -> BlipColor {
    BlipColor { r: (c.r * 0.62).min(1.0), g: (c.g * 0.62).min(1.0), b: (c.b * 0.62).min(1.0), a: 1.0 }
}

/// One snake: a tube along the path its head has taken, banded so two vipers in
/// the same quarter of the pit stay tellable apart, its hole drawn as a lit gap
/// with a ring on it — a hole you cannot see is a hole you cannot aim at.
fn draw_snake(blip: &Blip, s: &arena::Snake) {
    if !s.alive { return; }
    let (c, band) = (s.colour(), band_of(s.colour()));
    let thick = arena::BODY_R * 2.0;
    let pts: Vec<arena::Pt> = s.path.iter().copied().collect();
    for i in (1..pts.len()).rev() {
        if s.in_hole(i) || s.in_hole(i - 1) { continue; }
        let col = if (i / 3) % 2 == 0 { c } else { band };
        blip.draw_line_ex(pts[i].x, pts[i].y, pts[i - 1].x, pts[i - 1].y, thick, col);
    }
    if let Some(p) = pts.first() {
        blip.draw_line_ex(s.head.x, s.head.y, p.x, p.y, thick, c);
    }
    if let Some(h) = s.hole {
        let mid = s.hole_centre();
        if !h.spent {
            if let Some(m) = mid {
                let r = arena::BODY_R * 1.1 + (blip::macroquad::time::get_time() as f32 * 6.0).sin().abs() * 2.0;
                blip.fill_circle(m.x, m.y, r, BlipColor { a: 0.35, ..HOLE });
            }
        }
        for k in [h.at, h.at + h.len] {
            if let Some(p) = s.path.get(k) {
                blip.fill_circle(p.x, p.y, arena::BODY_R * 0.8, if h.spent { band } else { HOLE });
            }
        }
    }
    // The head, with eyes either side of the way it is going.
    let head = BlipColor { r: (c.r + 0.2).min(1.0), g: (c.g + 0.2).min(1.0), b: (c.b + 0.2).min(1.0), a: 1.0 };
    blip.fill_circle(s.head.x, s.head.y, arena::BODY_R + 1.0, if s.boost { HOLE } else { head });
    let (fx, fy) = (s.heading.cos(), s.heading.sin());
    for side in [-1.0f32, 1.0] {
        let (ex, ey) = (s.head.x + fx * 3.0 - fy * side * 3.6, s.head.y + fy * 3.0 + fx * side * 3.6);
        blip.fill_circle(ex, ey, 1.9, BLIP_BLACK);
    }
}

/// Skin: what a snake left behind, pale enough never to be mistaken for a body.
fn draw_skin(blip: &Blip, w: &World) {
    for q in &w.skin { blip.fill_rect(q.x - 4.0, q.y - 4.0, 8.0, 8.0, SKIN); }
}

fn draw_eggs(blip: &Blip, w: &World, egg: &Texture2D) {
    let s = arena::EGG_R * 2.4;
    for e in &w.eggs { blip.draw_texture(egg, e.x - s / 2.0, e.y - s / 2.0, s, s); }
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

fn draw_world(blip: &Blip, w: &World, egg: &Texture2D) {
    draw_pit(blip);
    draw_skin(blip, w);
    draw_eggs(blip, w, egg);
    for s in &w.snakes { draw_snake(blip, s); }
    w.fx.draw(blip);
}

fn draw_play(blip: &Blip, g: &Game, egg: &Texture2D) {
    draw_world(blip, &g.world, egg);
    draw_standings(blip, &g.world);
    if g.world.banner_t < BANNER {
        let a = (1.0 - g.world.banner_t / BANNER).min(1.0);
        blip.draw_centered(&format!("ROUND {}", g.world.round), (arena::PIT_Y + arena::PIT_H / 2.0) as f32, 4.0,
            BlipColor { a, ..HOLE });
    }
    blip.draw_hud(g.world.score_of(0), g.world.lives_of(0));
}

/// The title screen is an attract mode: a real pit, three CPUs, and the game
/// showing its own movement and its own holes while it waits for a coin.
fn draw_title(blip: &Blip, g: &Game, egg: &Texture2D) {
    draw_world(blip, &g.demo, egg);
    draw_standings(blip, &g.demo);
    let veil = BlipColor { r: 0.0, g: 0.0, b: 0.0, a: 0.55 };
    blip.fill_rect(0.0, 46.0, WIN_W as f32, 92.0, veil);
    blip.fill_rect(0.0, 402.0, WIN_W as f32, 116.0, veil);
    blip.draw_text_outlined("ADDER", 96.0, 62.0, 6.0, arena::seat_colour(0), BLIP_BLACK);
    blip.draw_hi(&web::high_score(), 116.0, HOLE);
    let by = web::controls();
    blip.draw_centered(by.pick("PRESS FIRE", "PRESS FIRE", "TAP TO START"), 424.0, 3.0, BLIP_WHITE);
    blip.draw_centered(by.pick("WASD STEER   HOLD FIRE TO STRIKE", "STICK STEER   HOLD TO STRIKE", "DRAG STEER   TAP TO STRIKE"),
        454.0, 2.0, SAND);
    blip.draw_centered("PASS THROUGH THE HOLES", 474.0, 2.0, HOLE);
    blip.draw_centered("PLAYER 2 PRESSES ANY KEY", 494.0, 2.0, arena::seat_colour(1));
}

fn draw_over(blip: &Blip, g: &Game) {
    blip.clear(BLIP_BLACK);
    blip.draw_game_over(g.world.score_of(0), &web::high_score(), BLIP_RED, arena::seat_colour(0), HOLE,
        !g.timer.active());
    blip.draw_centered(&format!("ROUND {}   LENGTH {}", g.world.round, g.world.snakes.first().map_or(0, |s| s.len_s)),
        (WIN_H / 2 + 54) as f32, 2.0, SAND);
}
