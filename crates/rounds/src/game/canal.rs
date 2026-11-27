//! The Canal Run that ends each day: papers onto Nyhavn's tables, ships and tour boats.

use super::*;

impl Game {
    pub fn start_canal(&mut self) {
        self.state = State::Canal;
        self.clock = 0.0;
        self.timer.start(CANAL_SECS);
        self.papers.clear();
        self.pops.clear();
        self.bits.clear();
        self.bike_x = QUAY_MID;
        self.bike_d = 0.0;
        self.speed = CRUISE;
        self.steer_v = 0.0;
        self.targets.clear();
        // café tables all along the terrace, old ships moored nose to tail, tour boats now and then
        let mut d = 360.0;
        while d < HARBOUR_D - 200.0 {
            self.targets.push(Target { x: rand_range_f32(32.0, 88.0), d, vd: 0.0, hit: -1.0, mark: Mark::Table });
            d += rand_range_f32(90.0, 150.0);
        }
        let mut d = 300.0;
        while d < HARBOUR_D - 250.0 {
            self.targets.push(Target { x: rand_range_f32(311.0, 320.0), d, vd: 0.0, hit: -1.0, mark: Mark::Ship });
            d += rand_range_f32(150.0, 200.0);
        }
        let mut d = 700.0;
        while d < HARBOUR_D + 600.0 {
            self.targets.push(Target { x: rand_range_f32(352.0, 388.0), d, vd: -rand_range_f32(22.0, 34.0), hit: -1.0, mark: Mark::TourBoat });
            d += rand_range_f32(380.0, 520.0);
        }
    }

    /// A Canal Run target's worth, by how far it is from the middle of the quay: 50 to 500.
    pub fn target_points(x: f32) -> i32 {
        let far = ((x - QUAY_MID).abs() - 95.0) / 95.0;
        (((50.0 + far.clamp(0.0, 1.0) * 450.0) / 50.0).round() * 50.0) as i32
    }

    pub fn update_canal(&mut self, dt: f32) {
        self.clock += dt;
        for cd in &mut self.throw_cd { *cd -= dt; }
        for t in &mut self.targets {
            t.d += t.vd * dt;
            if t.hit >= 0.0 { t.hit += dt; }
        }
        // Papers here fly free: no wall, only water and targets.
        let mut done = Vec::new();
        for (i, p) in self.papers.iter_mut().enumerate() {
            if let Some((t, _)) = &mut p.landed { *t += dt; continue; }
            p.t += dt;
            if p.t >= FLIGHT { done.push(i); }
        }
        for i in done {
            let p = self.papers[i];
            let (x, d) = (p.f.x, p.f.d);
            self.papers[i].landed = Some((0.0, Outcome::Splash));
            let hit = self.targets.iter().position(|t| t.hit < 0.0 && (t.x - x).abs() < 15.0 && (t.d - d).abs() < 15.0);
            if let Some(k) = hit {
                let pts = Self::target_points(self.targets[k].x);
                self.targets[k].hit = 0.0;
                self.papers[i].landed = Some((9.0, Outcome::Sample));
                self.score(p.by, pts);
                self.tally.canal += pts;
                self.tally.canal_hits += 1;
                self.cues.push(Cue::Target);
                self.pop(x, d + 16.0, &pts.to_string(), [255, 214, 90]);
                self.burst(x, d, 10, 70.0, [255, 230, 140]);
                blip::bot::add("canal_hits", 1.0);
                blip::bot::add("canal_points", pts as f64);
            } else {
                self.cues.push(Cue::Splash);
                self.burst(x, d, 6, 40.0, [200, 230, 255]);
            }
        }
        self.papers.retain(|p| p.landed.is_none_or(|(t, _)| t < 1.2));
        self.update_effects(dt);
        if self.timer.tick(dt) { self.next_day(); }
    }

    /// A Canal Run throw: no papers counted, no misses.
    pub fn canal_throw(&mut self, side: Side, by: usize) {
        if self.throw_cd[side.index()] > 0.0 { return; }
        self.throw_cd[side.index()] = THROW_GAP;
        let (x, d) = (self.bike_x, self.bike_d + 10.0);
        let f = Flight { x: x + side.sign() * THROW_V * FLIGHT, d: d + self.speed * CARRY * FLIGHT, wall: false, slid: false, t: FLIGHT };
        self.papers.push(Paper { x, d, vx: side.sign() * THROW_V, vd: self.speed * CARRY, t: 0.0, side, by, f, landed: None });
        self.cues.push(Cue::Throw);
    }

    /// Where a Canal Run throw to `side` would land now.
    pub fn canal_landing(&self, side: Side) -> (f32, f32) {
        (self.bike_x + side.sign() * THROW_V * FLIGHT, self.bike_d + 10.0 + self.speed * CARRY * FLIGHT)
    }
}
