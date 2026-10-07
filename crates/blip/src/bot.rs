//! Native playtest autopilot. `BLIP_BOT=1` enables fixed 60 Hz input;
//! `BLIP_BOT_MAXT` sets the time limit and `BLIP_BOT_SHOTS` enables screenshots.
//! Disabled on wasm.

use macroquad::input::KeyCode;

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::KeyCode;
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    pub struct St {
        pub on: bool,
        pub held: Vec<KeyCode>,
        pub prev: Vec<KeyCode>,
        pub clock: f32,
        pub maxt: f32,
        pub shots: Option<String>,
        pub shot_every: f32,
        pub next_shot: f32,
        pub shot_n: u32,
        pub stats: BTreeMap<String, f64>,
    }

    pub static ST: Mutex<Option<St>> = Mutex::new(None);

    pub fn with<R>(f: impl FnOnce(&mut St) -> R) -> R {
        let mut g = ST.lock().unwrap();
        let st = g.get_or_insert_with(|| {
            let num = |k: &str, d: f32| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
            St {
                on: std::env::var_os("BLIP_BOT").is_some(),
                held: Vec::new(),
                prev: Vec::new(),
                clock: 0.0,
                maxt: num("BLIP_BOT_MAXT", 600.0),
                shots: std::env::var("BLIP_BOT_SHOTS").ok(),
                shot_every: num("BLIP_BOT_SHOT_EVERY", 4.0),
                next_shot: 1.0,
                shot_n: 0,
                stats: BTreeMap::new(),
            }
        });
        f(st)
    }
}

/// Is the autopilot on for this run?
#[inline]
pub fn active() -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    { imp::with(|s| s.on) }
    #[cfg(target_arch = "wasm32")]
    { false }
}

/// The keys the bot holds from now until the next call.
pub fn hold(keys: &[KeyCode]) {
    #[cfg(not(target_arch = "wasm32"))]
    imp::with(|s| { s.held.clear(); s.held.extend_from_slice(keys); });
    #[cfg(target_arch = "wasm32")]
    let _ = keys;
}

pub(crate) fn held(k: KeyCode) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    { imp::with(|s| s.on && s.held.contains(&k)) }
    #[cfg(target_arch = "wasm32")]
    { let _ = k; false }
}

pub(crate) fn pressed(k: KeyCode) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    { imp::with(|s| s.on && s.held.contains(&k) && !s.prev.contains(&k)) }
    #[cfg(target_arch = "wasm32")]
    { let _ = k; false }
}

pub(crate) fn any_pressed() -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    { imp::with(|s| s.on && s.held.iter().any(|k| !s.prev.contains(k))) }
    #[cfg(target_arch = "wasm32")]
    { false }
}

/// Game seconds since the run began.
pub fn clock() -> f32 {
    #[cfg(not(target_arch = "wasm32"))]
    { imp::with(|s| s.clock) }
    #[cfg(target_arch = "wasm32")]
    { 0.0 }
}

/// Add to a named stat.
pub fn add(key: &str, n: f64) {
    #[cfg(not(target_arch = "wasm32"))]
    imp::with(|s| if s.on { *s.stats.entry(key.to_string()).or_insert(0.0) += n; });
    #[cfg(target_arch = "wasm32")]
    let _ = (key, n);
}

/// Set a named stat (last value wins).
pub fn set(key: &str, v: f64) {
    #[cfg(not(target_arch = "wasm32"))]
    imp::with(|s| if s.on { s.stats.insert(key.to_string(), v); });
    #[cfg(target_arch = "wasm32")]
    let _ = (key, v);
}

/// Print the RESULT line and end the process.
pub fn finish(outcome: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let line = imp::with(|s| {
            if !s.on { return None; }
            let mut l = format!("RESULT outcome={outcome} t={:.0}", s.clock);
            for (k, v) in &s.stats {
                if v.fract() == 0.0 { l += &format!(" {k}={v}"); } else { l += &format!(" {k}={v:.2}"); }
            }
            Some(l)
        });
        if let Some(l) = line {
            println!("{l}");
            std::process::exit(0);
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = outcome;
}

/// Called by `Blip::next_frame` just before the frame is shown: saves a
/// frame when one is due and ends a run that has hit its time limit.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn before_present() {
    let (shot, over) = imp::with(|s| {
        if !s.on { return (None, false); }
        let mut shot = None;
        if let Some(dir) = &s.shots {
            if s.clock >= s.next_shot {
                s.next_shot += s.shot_every;
                s.shot_n += 1;
                shot = Some(format!("{dir}/{:04}.png", s.shot_n));
            }
        }
        (shot, s.clock >= s.maxt)
    });
    if let Some(p) = shot { macroquad::texture::get_screen_data().export_png(&p); }
    if over { finish("timeout"); }
}

/// Called after the frame: the fixed step, and this frame's keys become last frame's.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn after_present() -> Option<f32> {
    imp::with(|s| {
        if !s.on { return None; }
        s.prev = s.held.clone();
        s.clock += 1.0 / 60.0;
        Some(1.0 / 60.0)
    })
}
