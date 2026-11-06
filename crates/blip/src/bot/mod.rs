//! The playtest autopilot every game shares. A game's own `bot.rs` says how
//! to play it; this module does the rest.
//!
//! - [`Pad`]: the controls a bot works this frame, said as a player would
//!   ("steer toward the ball", "tap fire"); presses get their release frames.
//! - [`Playthrough`]: a named run that shows one theme of the game ("the snake
//!   eats and grows into level 2") and the stats that prove it.
//! - [`grid`] and [`predict`]: path finding on tile maps, and where a moving
//!   thing will be.
//! - Stats: [`add`], [`set`], [`reached_level`]; the run ends with one
//!   `RESULT` line ([`finish`]).
//!
//! Switches (native only; the web build never runs a bot):
//! `BLIP_BOT=1` plays at a fixed 60 Hz. `BLIP_BOT_MAXT` sets the time limit,
//! `BLIP_BOT_LEVEL=<n>` starts at level n, `BLIP_BOT_TITLE=<secs>` waits on the
//! title, `BLIP_BOT_SHOTS=<dir>` saves frames every `BLIP_BOT_SHOT_EVERY`
//! seconds, `BLIP_BOT_FUZZ=<seed>` mashes random keys instead of playing.
//! `BLIP_BOT_PLAYTHROUGH=<name>` runs one of the game's playthroughs and
//! `BLIP_BOT_LIST=1` lists them. `BLIP_BOT_SEED=<n>` fixes the random numbers
//! (a playthrough has its own), so a run repeats exactly. A game's own switches are `BLIP_BOT_<NAME>`:
//! read them with [`num`], [`flag`] or [`is`].

use macroquad::input::KeyCode;
use std::collections::BTreeMap;
use std::sync::Mutex;

pub mod grid;
mod pad;
mod playthrough;
pub mod predict;

pub use pad::Pad;
pub use playthrough::{playthroughs, Goal, Playthrough};

/// One bot run: what it holds, its clock and its tallies.
struct Run {
    on: bool,
    held: Vec<KeyCode>,
    prev: Vec<KeyCode>,
    clock: f32,
    maxt: f32,
    shots: Option<String>,
    shot_every: f32,
    next_shot: f32,
    shot_n: u32,
    stats: BTreeMap<String, f64>,
    /// Random key mashing instead of the autopilot: the generator state, or 0 when off.
    fuzz: u64,
    fuzz_left: u32,
    fuzz_keys: Vec<KeyCode>,
    /// No keys before this clock: the title stays up (BLIP_BOT_TITLE).
    title: f32,
    /// The playthrough being run, if any.
    playthrough: Option<&'static Playthrough>,
}

static RUN: Mutex<Option<Run>> = Mutex::new(None);

fn env_num(key: &str, default: f32) -> f32 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn with<R>(f: impl FnOnce(&mut Run) -> R) -> R {
    let mut run = RUN.lock().unwrap_or_else(|e| e.into_inner());
    let run = run.get_or_insert_with(|| Run {
        on: std::env::var_os("BLIP_BOT").is_some(),
        held: Vec::new(),
        prev: Vec::new(),
        clock: 0.0,
        maxt: env_num("BLIP_BOT_MAXT", 600.0),
        shots: std::env::var("BLIP_BOT_SHOTS").ok(),
        shot_every: env_num("BLIP_BOT_SHOT_EVERY", 4.0),
        next_shot: 1.0,
        shot_n: 0,
        stats: BTreeMap::new(),
        fuzz: std::env::var("BLIP_BOT_FUZZ").ok()
            .map_or(0, |v| v.parse::<u64>().unwrap_or(1).wrapping_mul(2654435761).max(1)),
        fuzz_left: 0,
        fuzz_keys: Vec::new(),
        title: env_num("BLIP_BOT_TITLE", 0.0),
        playthrough: None,
    });
    f(run)
}

/// Is the autopilot on for this run?
pub fn active() -> bool {
    with(|r| r.on)
}

// ---- settings ----------------------------------------------------------------

/// A playtest setting, `BLIP_BOT_<name>`; a playthrough's own settings come first.
pub fn var(name: &str) -> Option<String> {
    if !active() { return None; }
    if let Some(value) = with(|r| r.playthrough).and_then(|p| p.setting(name)) {
        return Some(value);
    }
    std::env::var(format!("BLIP_BOT_{name}")).ok()
}

/// A playtest setting, `BLIP_BOT_<name>`, as a number.
pub fn num(name: &str) -> Option<f64> {
    var(name)?.parse().ok()
}

/// Is the playtest setting `BLIP_BOT_<name>` this word?
pub fn is(name: &str, word: &str) -> bool {
    var(name).is_some_and(|v| v == word)
}

/// Is the playtest switch `BLIP_BOT_<name>` set (to anything but 0)?
pub fn flag(name: &str) -> bool {
    var(name).is_some_and(|v| v != "0")
}

/// The random seed of a bot run: `BLIP_BOT_SEED`, else the playthrough's.
/// Playing at a fixed 60 Hz from a fixed seed, a run repeats exactly.
pub fn seed() -> Option<u64> {
    num("SEED").map(|s| s as u64).or_else(|| with(|r| r.playthrough).map(|p| p.seed))
}

/// The level a bot run starts at, `BLIP_BOT_LEVEL` counted from 1, as an index
/// clamped to `0..count`; 0 when unset.
pub fn level(count: usize) -> usize {
    num("LEVEL").map_or(0, |n| (n.max(1.0) as usize).min(count.max(1)) - 1)
}

// ---- stats -------------------------------------------------------------------

/// Game seconds since the run began.
pub fn clock() -> f32 {
    with(|r| r.clock)
}

/// Add to a named stat.
pub fn add(key: &str, n: f64) {
    with(|r| if r.on { *r.stats.entry(key.to_string()).or_insert(0.0) += n; });
}

/// Set a named stat (last value wins).
pub fn set(key: &str, v: f64) {
    with(|r| if r.on { r.stats.insert(key.to_string(), v); });
}

/// A stat's value so far (0 when never set).
pub fn stat(key: &str) -> f64 {
    with(|r| r.stats.get(key).copied().unwrap_or(0.0))
}

/// Record that play is now on level `n`: the `level` stat and `t_level<n>`,
/// the clock when it began. `Session` calls this; games without one call it themselves.
pub fn reached_level(n: i32) {
    set("level", n as f64);
    set(&format!("t_level{n}"), clock() as f64);
}

/// Record how many authored levels the game has (the `levels` stat), so a
/// playtest can visit each one; endless games leave it out.
pub fn levels(n: usize) {
    set("levels", n as f64);
}

/// Print the RESULT line and end the process. `outcome` is how the game
/// ended (`over`, `won`, `timeout`); a playthrough adds its verdict.
pub fn finish(outcome: &str) {
    let line = with(|r| {
        if !r.on { return None; }
        let mut line = format!("RESULT outcome={outcome} t={:.0}", r.clock);
        for (k, v) in &r.stats {
            if v.fract() == 0.0 { line += &format!(" {k}={v}"); } else { line += &format!(" {k}={v:.2}"); }
        }
        if let Some(p) = r.playthrough {
            line += &p.verdict(outcome, &r.stats);
        }
        Some(line)
    });
    if let Some(line) = line {
        println!("{line}");
        std::process::exit(0);
    }
}

// ---- keys --------------------------------------------------------------------

/// Keys a fuzzing run presses, each held for a few frames at random.
const FUZZ_KEYS: [KeyCode; 11] = [
    KeyCode::Up, KeyCode::Down, KeyCode::Left, KeyCode::Right, KeyCode::W, KeyCode::A,
    KeyCode::S, KeyCode::D, KeyCode::Space, KeyCode::Z, KeyCode::F,
];

/// The keys the bot holds from now until the next call. Bots usually say
/// this through a [`Pad`].
pub fn hold(keys: &[KeyCode]) {
    with(|r| {
        r.held.clear();
        if r.clock < r.title { return; }
        if r.fuzz == 0 { r.held.extend_from_slice(keys); return; }
        if r.fuzz_left == 0 {
            r.fuzz_left = 4;
            r.fuzz = r.fuzz.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let bits = r.fuzz >> 20;
            r.fuzz_keys = FUZZ_KEYS.iter().enumerate()
                .filter(|(i, _)| (bits >> (i * 3)) & 3 == 0).map(|(_, k)| *k).collect();
        }
        r.fuzz_left -= 1;
        let keys = r.fuzz_keys.clone();
        r.held.extend(keys);
    });
}

pub(crate) fn held(k: KeyCode) -> bool {
    with(|r| r.on && r.held.contains(&k))
}

/// Was `k` down last frame? A fresh press needs it up first.
pub(crate) fn held_before(k: KeyCode) -> bool {
    with(|r| r.prev.contains(&k))
}

pub(crate) fn pressed(k: KeyCode) -> bool {
    with(|r| r.on && r.held.contains(&k) && !r.prev.contains(&k))
}

pub(crate) fn any_pressed() -> bool {
    with(|r| r.on && r.held.iter().any(|k| !r.prev.contains(k)))
}

// ---- the frame ---------------------------------------------------------------

/// Called by `Blip::next_frame` just before the frame is shown: saves a frame
/// when one is due, and ends a run that has met its goals or its time limit.
pub(crate) fn before_present() {
    let (shot, met, over) = with(|r| {
        if !r.on { return (None, false, false); }
        let mut shot = None;
        if let Some(dir) = &r.shots {
            if r.clock >= r.next_shot {
                r.next_shot += r.shot_every;
                r.shot_n += 1;
                shot = Some(format!("{dir}/{:04}.png", r.shot_n));
            }
        }
        let met = r.playthrough.is_some_and(|p| p.met(&r.stats));
        (shot, met, r.clock >= r.maxt)
    });
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(p) = shot { macroquad::texture::get_screen_data().export_png(&p); }
    #[cfg(target_arch = "wasm32")]
    let _ = shot;
    if met { finish("goals"); }
    if over { finish("timeout"); }
}

/// Called after the frame: the fixed step, and this frame's keys become last frame's.
pub(crate) fn after_present() -> Option<f32> {
    with(|r| {
        if !r.on { return None; }
        r.prev = r.held.clone();
        r.clock += 1.0 / 60.0;
        Some(1.0 / 60.0)
    })
}
