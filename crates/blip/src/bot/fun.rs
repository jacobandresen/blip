//! What a bot run says about how much fun the game is, added to its RESULT line. Three measures, in order of
//! importance (FUN.md beside this file has the reasons and the sources):
//!
//! 1. Skill pays: the bot's score against random play's. Scored by `tools/fun_report.mjs` from two runs; nothing here.
//! 2. Close calls: a hazard whose hit box comes within [`NEAR`] of the player's starts a pass, which ends as a near
//!    miss when the boxes part (`fun_near_pm`, a minute) or as a hit when they touch or the game reports one (a rise
//!    in `deaths` or a `hit_*` stat; `fun_hits_pm`). Only games whose bot reports hit boxes ([`super::near`]).
//! 3. Busy: `fun_dead`, the share of the run in stretches of more than [`DEAD_SECS`] with the controls untouched.
//!
//! With `BLIP_BOT_NEAR=<dir>`, each pass's closest frame is saved there with the boxes drawn, and each idle
//! stretch is printed.

use super::rollout::Mover;
use macroquad::input::KeyCode;
use macroquad::texture::Image;

/// Untouched controls for longer than this is time with nothing to do.
pub const DEAD_SECS: f32 = 2.0;
/// Hit boxes closer than this, edge to edge in canvas pixels, are a pass.
pub const NEAR: f32 = 16.0;
/// Frames saved per kind (near, hit) and run.
const SHOTS: u32 = 8;
/// A life lost this soon (seconds) after a hit is that hit, not another.
const LIFE_AFTER_HIT: f32 = 3.0;

/// How a box is drawn on a saved frame.
#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) enum Paint { Player, Nearest, Other }

/// A hazard within reach, from when its box came near until the boxes part or touch. One begun while the hazards
/// of a hit are still near is no near miss when it ends.
struct Pass { closest: f32, at: f32, shot: Option<Image>, after_hit: bool }

#[derive(Default)]
pub(crate) struct Meter {
    frames: u64,
    still: u32,
    dead: u64,
    pass: Option<Pass>,
    /// Boxes touching now (a touch is one hit, however long), the clock of the last hit, and whether the hazards
    /// of that hit are still near.
    touching: bool,
    last_hit: f32,
    after_hit: bool,
    near_misses: u32,
    hits: u32,
    /// The game's bot reports hit boxes: only then are close calls measured.
    boxes_seen: bool,
    /// This frame's boxes, and those the overlay draws on a frame being saved.
    recent: Vec<(Mover, Paint)>,
    pub(crate) boxes: Vec<(Mover, Paint)>,
    pub(crate) capture: bool,
    /// Write the next captured frame as this file.
    export: Option<String>,
    dir: Option<String>,
    saved: [u32; 2],
}

impl Meter {
    pub(crate) fn saving_to(dir: Option<String>) -> Self { Self { dir, ..Self::default() } }

    /// Frames are being saved: every one must be drawn.
    pub(crate) fn saving(&self) -> bool { self.dir.is_some() }

    /// One frame of controls.
    pub(crate) fn frame(&mut self, held: &[KeyCode], prev: &[KeyCode]) {
        self.frames += 1;
        if held.len() == prev.len() && held.iter().all(|k| prev.contains(k)) { self.still += 1; } else { self.end_still(); }
    }

    fn end_still(&mut self) {
        let secs = self.still as f32 / 60.0;
        if secs > DEAD_SECS {
            self.dead += self.still as u64;
            if self.dir.is_some() { println!("IDLE t={:.1} secs={secs:.1}", self.frames as f32 / 60.0 - secs); }
        }
        self.still = 0;
    }

    /// This frame's hit boxes at `clock`.
    pub(crate) fn near(&mut self, me: &[Mover], hazards: &[Mover], clock: f32) {
        self.boxes_seen = true;
        let gaps: Vec<f32> = hazards.iter().map(|h| if h.weight > 0.0 { me.iter().map(|m| gap(m, h)).fold(f32::INFINITY, f32::min) } else { f32::INFINITY }).collect();
        let nearest = gaps.iter().enumerate().min_by(|a, b| a.1.total_cmp(b.1)).map(|(i, g)| (*g, i));
        let closest = nearest.map_or(f32::INFINITY, |(g, _)| g);
        if self.dir.is_some() {
            self.recent = me.iter().map(|m| (*m, Paint::Player)).collect();
            for (i, h) in hazards.iter().enumerate() {
                if nearest.is_some_and(|(_, ni)| ni == i) { self.recent.push((*h, Paint::Nearest)); }
                else if gaps[i] < NEAR * 4.0 { self.recent.push((*h, Paint::Other)); }
            }
        }
        if closest > 0.0 { self.touching = false; }
        if closest >= NEAR {
            self.after_hit = false;
            if let Some(p) = self.pass.take().filter(|p| !p.after_hit) {
                self.near_misses += 1;
                self.save(0, p);
            }
            return;
        }
        let after_hit = self.after_hit;
        let pass = self.pass.get_or_insert(Pass { closest: f32::INFINITY, at: clock, shot: None, after_hit });
        if closest < pass.closest {
            pass.closest = closest;
            pass.at = clock;
            if self.dir.is_some() && self.saved[0] < SHOTS { self.capture = true; self.boxes = self.recent.clone(); }
        }
        if closest <= 0.0 && !self.touching {
            self.touching = true;
            self.hit(clock, false);
        }
    }

    /// The game says the player was hit (its own test, which may be swept or remove the hazard): the pass under
    /// way ends as a hit.
    pub(crate) fn hit(&mut self, clock: f32, life_lost: bool) {
        // A life lost with the hit that took it is one hit, also when the game takes the life once its crash or
        // explosion has played (Rounds: 1.6 s later, which counted every crash twice).
        let same = if life_lost { LIFE_AFTER_HIT } else { 0.05 };
        if !self.boxes_seen || (self.hits > 0 && clock - self.last_hit < same) { return; }
        self.last_hit = clock;
        self.after_hit = true;
        self.hits += 1;
        let p = self.pass.take().unwrap_or(Pass { closest: 0.0, at: clock, shot: None, after_hit: false });
        if p.shot.is_none() && self.dir.is_some() { self.capture = true; self.boxes = self.recent.clone(); }
        self.save(1, p);
    }

    /// Name the pass's frame: the closest one already taken, else the next one drawn.
    fn save(&mut self, kind: usize, p: Pass) {
        let Some(dir) = &self.dir else { return };
        if self.saved[kind] >= SHOTS { return; }
        self.saved[kind] += 1;
        let what = ["near", "hit"][kind];
        let name = format!("{dir}/{what}-{:02}-t{:.0}-gap{:.0}.png", self.saved[kind], p.at, p.closest.max(0.0));
        println!("PASS {what} t={:.2} gap={:.1} file={name}", p.at, p.closest.max(0.0));
        match p.shot {
            Some(img) => img.export_png(&name),
            None => self.export = Some(name),
        }
    }

    /// The frame just drawn, when one was asked for.
    pub(crate) fn captured(&mut self, img: Image) {
        self.capture = false;
        self.boxes.clear();
        if let Some(name) = self.export.take() { img.export_png(&name); return; }
        if let Some(p) = &mut self.pass { p.shot = Some(img); }
    }

    /// The fun stats for a run that ends at `clock`.
    pub(crate) fn stats(&mut self, clock: f32) -> Vec<(&'static str, f64)> {
        self.end_still();
        let mins = (clock as f64 / 60.0).max(1.0 / 60.0);
        let round = |v: f64| (v * 100.0).round() / 100.0;
        let mut out = vec![("fun_dead", round(self.dead as f64 / self.frames.max(1) as f64))];
        if self.boxes_seen {
            out.push(("fun_near_pm", round(self.near_misses as f64 / mins)));
            out.push(("fun_hits_pm", round(self.hits as f64 / mins)));
        }
        out
    }
}

/// The edge-to-edge distance between two boxes now (0 when they touch or overlap).
pub(crate) fn gap(a: &Mover, b: &Mover) -> f32 {
    let ox = ((a.x - b.x).abs() - a.hw - b.hw).max(0.0);
    let oy = ((a.y - b.y).abs() - a.hh - b.hh).max(0.0);
    ox.hypot(oy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn controls_left_alone_for_long_count_as_dead_time() {
        let mut m = Meter::default();
        for _ in 0..300 { m.frame(&[], &[]); }
        for k in 0..300 {
            let (now, before) = if k % 2 == 0 { ([KeyCode::Left], [KeyCode::Right]) } else { ([KeyCode::Right], [KeyCode::Left]) };
            m.frame(&now, &before);
        }
        let s: BTreeMap<_, _> = m.stats(10.0).into_iter().collect();
        assert_eq!(s["fun_dead"], 0.5);
        assert!(!s.contains_key("fun_near_pm"), "no hit boxes, no close calls");
    }

    #[test]
    fn a_hazard_that_passes_is_a_near_miss_and_one_that_touches_a_hit() {
        let mut m = Meter::default();
        let me = [Mover::still(0.0, 0.0).sized(5.0, 5.0)];
        // A hazard 5 wide passes 10 then 3 from the player's edge and goes; another comes in and touches (one hit,
        // however long the boxes overlap).
        for x in [60.0, 20.0, 13.0, 40.0, 30.0, 20.0, 12.0, 10.0, 10.0, 40.0] {
            m.near(&me, &[Mover::still(x, 0.0).sized(5.0, 5.0)], 0.0);
        }
        let s: BTreeMap<_, _> = m.stats(60.0).into_iter().collect();
        assert_eq!(s["fun_near_pm"], 1.0);
        assert_eq!(s["fun_hits_pm"], 1.0);
    }

    #[test]
    fn a_life_lost_after_its_crash_has_played_is_the_same_hit() {
        let mut m = Meter::default();
        m.near(&[Mover::still(0.0, 0.0).sized(5.0, 5.0)], &[Mover::still(40.0, 0.0).sized(5.0, 5.0)], 0.0);
        m.hit(1.0, false);
        m.hit(2.6, true);
        m.hit(3.0, false);
        m.hit(9.0, true);
        let s: BTreeMap<_, _> = m.stats(60.0).into_iter().collect();
        assert_eq!(s["fun_hits_pm"], 3.0, "crash and its lost life one hit; a later hit and a lone death two more");
    }
}
