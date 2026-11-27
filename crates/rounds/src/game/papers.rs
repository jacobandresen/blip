//! The papers: thrown to either side, flying and landing, a delivery or a sample, a dog that would rather have one,
//! and each house judged once the bike has passed it.

use super::*;

impl Game {
    /// Throw to `side` for player `by`, if there is a paper and the arm is ready.
    pub fn throw(&mut self, side: Side, by: usize) {
        if self.throw_cd[side.index()] > 0.0 || self.in_junction() && self.heading.abs() > 0.2 { return; }
        if self.stock <= 0 {
            self.throw_cd[side.index()] = 0.5;
            self.cues.push(Cue::Empty);
            self.pop(self.bike_x, self.bike_d + 30.0, "NO PAPERS", [240, 120, 110]);
            return;
        }
        self.stock -= 1;
        self.throw_cd[side.index()] = THROW_GAP;
        let (x, d) = (self.bike_x, self.bike_d + 10.0);
        let f = flight(x, d, self.speed, side, self.wind(), self.rain());
        self.papers.push(Paper { x, d, vx: side.sign() * THROW_V, vd: self.speed * CARRY + self.wind(), t: 0.0, side, by, f,
            landed: None });
        self.cues.push(Cue::Throw);
        blip::bot::add("throws", 1.0);
    }

    /// Where a paper thrown to `side` right now would end, and what it would hit (the bot aims with this).
    pub fn would_hit(&self, side: Side) -> (Outcome, Option<usize>) {
        let f = flight(self.bike_x, self.bike_d + 10.0, self.speed, side, self.wind(), self.rain());
        let lot = row_at(f.d).map(|r| self.houses[side.index()][r].lot);
        outcome(side, f, lot)
    }

    pub(super) fn update_papers(&mut self, dt: f32) {
        let mut ended = Vec::new();
        for (i, p) in self.papers.iter_mut().enumerate() {
            if let Some((t, _)) = &mut p.landed { *t += dt; continue; }
            p.t += dt;
            if p.t >= p.f.t { ended.push(i); }
        }
        for i in ended { let f = self.papers[i].f; self.land(i, f); }
        self.papers.retain(|p| p.landed.is_none_or(|(t, _)| t < 2.5));
    }

    pub(super) fn land(&mut self, i: usize, f: Flight) {
        let p = self.papers[i];
        let s = p.side.index();
        let lot = row_at(f.d).map(|r| self.houses[s][r].lot);
        let (what, row) = outcome(p.side, f, lot);
        self.papers[i].landed = Some((0.0, what));
        match (what, row) {
            (Outcome::Letterbox | Outcome::Doorstep, Some(r)) if self.houses[s][r].sub && !self.houses[s][r].done => {
                self.deliver(i, f, r, what == Outcome::Letterbox);
            }
            (Outcome::Sample, Some(r)) if !self.houses[s][r].sub && !self.houses[s][r].sampled => {
                let pts = self.worth(SAMPLE_PTS);
                self.score(p.by, pts);
                self.houses[s][r].sampled = true;
                self.houses[s][r].wave = 0.0;
                self.tally.samples += 1;
                self.cues.push(Cue::Sample);
                self.burst(f.x, f.d, 8, 60.0, [140, 220, 255]);
                self.pop(f.x, f.d + 14.0, &format!("SAMPLE {pts}"), [140, 220, 255]);
                blip::bot::add("samples", 1.0);
            }
            (Outcome::Splash, _) => {
                self.cues.push(Cue::Splash);
                self.burst(f.x, f.d, 6, 40.0, [200, 230, 255]);
                self.papers[i].landed = Some((9.0, what));
            }
            (Outcome::Letterbox | Outcome::Doorstep, _) => self.cues.push(Cue::Thump),
            _ => self.cues.push(Cue::Plop),
        }
        self.dog_fetches(f.x, f.d);
    }

    /// Paper `i` reached a subscriber in row `r`: through the letterbox (the flap, two hearts) or on the doorstep.
    pub(super) fn deliver(&mut self, i: usize, f: Flight, r: usize, letterbox: bool) {
        let p = self.papers[i];
        let gold = [255, 214, 90];
        let pts = self.worth(if letterbox { LETTERBOX_PTS } else { DOORSTEP_PTS });
        self.score(p.by, pts);
        let h = &mut self.houses[p.side.index()][r];
        h.done = true;
        h.wave = 0.0;
        if letterbox {
            h.flap = 0.0;
            h.hearts = 2;
            self.tally.letterbox += 1;
            self.cues.push(Cue::Flap);
            self.burst(f.x, f.d, 10, 70.0, gold);
            blip::bot::add("letterbox", 1.0);
        } else {
            self.tally.doorstep += 1;
            self.cues.push(Cue::Thump);
            blip::bot::add("doorstep", 1.0);
        }
        blip::bot::add("deliveries", 1.0);
        self.pop(f.x - p.side.sign() * 20.0, f.d + 14.0, &pts.to_string(), if letterbox { gold } else { [255, 255, 255] });
        if letterbox { self.papers[i].landed = Some((9.0, Outcome::Letterbox)); }
    }

    /// A dog after the bike would rather have a paper that lands near it.
    pub(super) fn dog_fetches(&mut self, px: f32, pd: f32) {
        let near = |h: &Hazard| h.kind == Hz::Dog && (h.phase == TELL || h.phase == GO) && (h.x - px).abs() < 90.0 && (h.d - pd).abs() < 90.0;
        if let Some(dog) = self.hazards.iter_mut().find(|h| near(h)) {
            dog.phase = AWAY;
            dog.t = 0.0;
            dog.fetch = Some((px, pd));
            self.cues.push(Cue::Bark);
            blip::bot::add("fetches", 1.0);
        }
    }

    /// Houses the bike has left behind: a subscriber with no paper loses a heart, and with none left, the round.
    pub(super) fn judge_houses(&mut self) {
        let behind = self.bike_d - 200.0;
        for s in 0..2 {
            for r in 0..ROWS {
                if door_d(r) > behind { break; }
                let h = self.houses[s][r];
                if h.judged { continue; }
                self.houses[s][r].judged = true;
                if !h.sub || h.done { continue; }
                self.tally.misses += 1;
                blip::bot::add("misses", 1.0);
                if blip::bot::active() { eprintln!("MISS side={s} row={r} day={}", self.day + 1); }
                let h = &mut self.houses[s][r];
                h.hearts = h.hearts.saturating_sub(1);
                h.sad = 0.0;
                if h.hearts == 0 {
                    h.sub = false;
                    h.cancelled = 0.0;
                    self.tally.lost += 1;
                    self.cues.push(Cue::Cancelled);
                    blip::bot::add("cancelled", 1.0);
                } else {
                    self.cues.push(Cue::HeartLost);
                }
            }
        }
    }
}
