//! The street's traffic: riders and cars giving way, cyclists queued and sent along the track, cars across the junction,
//! and every hazard's frame, with the bike meeting them.

use super::*;

impl Game {
    /// Nothing drives through anything else: whatever would run into something in its way within a moment waits for
    /// it (a car behind a cyclist, a tourist that would swerve into a car), as traffic does.
    pub(super) fn give_way(&mut self) {
        let (bx, bd) = (self.bike_x, self.bike_d);
        let movers: Vec<(usize, f32, f32, f32, f32, f32, f32)> = self.hazards.iter().enumerate()
            .filter(|(_, h)| !h.gone && h.solid() && h.kind != Hz::Dog)
            .map(|(i, h)| {
                // a car about to back out, or backing, needs room in its lane for its whole length
                // ... and across the bike lane on the way there: the whole sweep from the drive to the lane
                if h.kind == Hz::Car && !h.oncoming && (h.phase == TELL || h.phase == GO) {
                    let (hw, hl) = h.size_driving();
                    let lane = CAR_LANE[h.side.index()];
                    return (i, (lane + h.x) / 2.0, h.d, hw + (lane - h.x).abs() / 2.0, hl, (lane - h.x).signum() * 0.001, 0.0);
                }
                // a car waiting to pull away looks along the way it is about to drive, at its full length
                if h.phase == PAUSE {
                    let (hw, hl) = h.size_driving();
                    return (i, h.x, h.d, hw, hl, 0.0, h.side.sign() * if h.side == Side::Right { 160.0 } else { 120.0 });
                }
                let (hw, hl) = h.size();
                let (vx, vd) = h.velocity(bx, bd);
                (i, h.x, h.d, hw, hl, vx, vd)
            }).collect();
        let mut solid: Vec<(usize, f32, f32, f32, f32, u8)> = self.hazards.iter().enumerate()
            .filter(|(_, h)| !h.gone && h.solid())
            .map(|(i, h)| { let (hw, hl) = h.size(); (i, h.x, h.d, hw, hl, h.seg) }).collect();
        // a fallen rider, or a bike standing still, is something everyone stops for
        if self.state == State::Crash || self.speed < 20.0 {
            solid.push((usize::MAX, self.bike_x, self.bike_d, BIKE_HW + 6.0, BIKE_HL, self.seg()));
        }
        for &(i, x, d, hw, hl, vx, vd) in &movers {
            if self.hazards[i].kind == Hz::Dog || vx == 0.0 && vd == 0.0 { continue; }
            let _ = ();
            let seg = self.hazards[i].seg;
            // where it would be in a moment, and whether something else is there, ahead of it
            let backing = vx.abs() < 0.01 && vd == 0.0;
            let (nx, nd) = (x + vx * 0.35, d + vd * 0.35);
            let blocked = solid.iter().any(|&(j, ox, od, ow, ol, oseg)| {
                j != i && oseg == seg && (nx - ox).abs() < hw + ow + 2.0 && (nd - od).abs() < hl + ol + 2.0
                    && (backing || (ox - x) * vx + (od - d) * vd > 0.0)
            });
            self.hazards[i].held = blocked || self.hazards[i].held;
        }
    }

    /// Cyclists stop at a red light, and behind a cyclist stopped ahead of them on the same track.
    pub(super) fn queue_cyclists(&mut self) {
        let lights: Vec<(usize, Light)> = JUNCTIONS.iter().map(|&r| (r, self.light(r).0)).collect();
        let snapshot: Vec<(Side, f32, bool)> = self.hazards.iter()
            .map(|h| (h.side, h.d, h.kind == Hz::Cyclist && h.phase == GO && !h.gone)).collect();
        let (bx, bd) = (self.bike_x, self.bike_d);
        // side-street cars wait at their lines but in the part of the red that is theirs
        for i in 0..self.hazards.len() {
            let h = self.hazards[i];
            if h.kind != Hz::CrossCar || h.gone { continue; }
            let Some(&(r, _)) = lights.iter().find(|(r, _)| (h.d - *r as f32 * LOT).abs() < LOT) else { continue };
            let (_, t) = self.light(r);
            let go = (CROSS_GO[0]..CROSS_GO[1]).contains(&t);
            let (hw, _) = h.size();
            let east = h.side == Side::Left;
            let line = if east { CURB[0] - 6.0 - hw } else { CURB[1] + 6.0 + hw };
            let to = if east { line - h.x } else { h.x - line };
            if !go && (0.0..10.0).contains(&to) { self.hazards[i].held = true; }
        }
        for i in 0..self.hazards.len() {
            let h = self.hazards[i];
            let road_user = matches!(h.kind, Hz::Cyclist | Hz::Car | Hz::Bus | Hz::Van | Hz::Blader | Hz::Tourist);
            if !road_user || h.seg != 0 || h.kind == Hz::Cyclist && h.phase != GO { if h.kind == Hz::Cyclist { self.hazards[i].held = false; } continue; }
            let vd = { let mut free = h; free.held = false; free.velocity(bx, bd).1 };
            if vd == 0.0 { continue; }
            let up = vd > 0.0;
            let (_, hl) = h.size();
            // the stop line facing it: the near one going up the street, the far one coming down
            let at_light = lights.iter().any(|&(r, l)| {
                let line = if up { r as f32 * LOT + 16.0 - hl } else { (r + 1) as f32 * LOT - 16.0 + hl };
                let to = if up { line - h.d } else { h.d - line };
                l != Light::Green && (0.0..12.0).contains(&to)
            });
            let behind = snapshot.iter().enumerate().any(|(j, &(side, d, cyc))| {
                let gap = if up { d - h.d } else { h.d - d };
                j != i && cyc && side == h.side && self.hazards[j].held && (0.0..30.0).contains(&gap)
            });
            self.hazards[i].held = at_light || h.kind == Hz::Cyclist && behind;
        }
    }

    /// Copenhagen rides a bike: riders keep coming down the left track toward the bike, and the right track ahead
    /// fills with commuters slower than the paper round. Fewer in the rain, fewer on market day.
    pub(super) fn cycle_traffic(&mut self) {
        let busy = match self.weather() { Weather::Rain => 0.55, Weather::Market => 0.6, Weather::Bundles => 0.8, _ => 1.0 };
        for (s, side) in Side::BOTH.into_iter().enumerate() {
            if self.ride_t < self.cycle_next[s] { continue; }
            let gap = if side == Side::Left { 0.8 } else { 1.5 } / busy;
            self.cycle_next[s] = self.ride_t + gap * rand_range_f32(0.6, 1.4);
            let d = self.bike_d + if side == Side::Left { 560.0 } else { 470.0 };
            let Some(row) = row_at(d) else { continue };
            if !self.turned && d > JD - MOUTH_HALF { continue; }
            if self.houses[0][row].lot == Lot::Bridge { continue; }
            // not on top of anyone in the lane: riders 30 px, a bus or a van parked on the track its length and more
            let room = |h: &Hazard| match h.kind { Hz::Cyclist | Hz::Tourist | Hz::Blader => 30.0, Hz::Bus | Hz::Van => 60.0, _ => -1.0 };
            if self.hazards.iter().any(|h| !h.gone && h.side == side && (h.d - d).abs() < room(h)) { continue; }
            let mut c = Hazard::new(Hz::Cyclist, side, row);
            c.d = d;
            c.phase = GO;
            c.seg = self.seg();
            self.hazards.push(c);
        }
    }

    /// While the bike's light is red the side street drives: each lane sends a car now and then.
    pub(super) fn cross_traffic(&mut self) {
        for (j, &row) in JUNCTIONS.iter().enumerate() {
            let near = (door_d(row) - self.bike_d).abs() < 700.0;
            for lane in 0..2 {
                if near && self.ride_t >= self.cross_next[j][lane] {
                    let side = if lane == 0 { Side::Left } else { Side::Right };
                    let mut car = Hazard::new(Hz::CrossCar, side, row);
                    // only into a lane with room at its end
                    let clear = !self.hazards.iter().any(|h| h.kind == Hz::CrossCar && !h.gone && h.side == side && (h.x - car.x).abs() < 56.0);
                    if clear {
                        car.phase = GO;
                        car.bus = (self.ride_t * 7.0) as i32 % 5 == 0;
                        self.hazards.push(car);
                    }
                    self.cross_next[j][lane] = self.ride_t + CROSS_EVERY * rand_range_f32(0.6, 1.4);
                }
            }
        }
        self.hazards.retain(|h| h.kind != Hz::CrossCar || (-60.0..540.0).contains(&h.x));
        // traffic turns off before the roadworks on the old street, and before the palace square on the new
        let square = STATUE_ROW as f32 * LOT - 2.0 * LOT;
        for h in &mut self.hazards {
            let drives = matches!(h.kind, Hz::Van) && h.phase == GO || matches!(h.kind, Hz::Bus | Hz::Car) && h.phase == AWAY && !h.oncoming;
            if drives && (h.seg == 0 && h.d > JD - MOUTH_HALF || h.seg == 1 && h.d > square) { h.gone = true; }
        }
    }

    pub(super) fn update_hazards(&mut self, dt: f32) {
        let (bx, bd, speed, safe) = (self.bike_x, self.bike_d, self.speed, self.safe);
        let mut cues = Vec::new();
        let mut hits = Vec::new();
        for h in &mut self.hazards { h.held = false; }
        self.queue_cyclists();
        self.give_way();
        let seg = self.seg();
        let traffic = Traffic::of(&self.hazards);
        for (i, h) in self.hazards.iter_mut().enumerate() {
            if h.gone || h.seg > seg { continue; }
            h.t += dt;
            h.wake(bx, bd, speed, seg, &mut cues);
            h.travel(dt, bx, bd, &traffic);
            if h.d < bd - 600.0 { h.gone = true; }
            // the bike, on the same street
            if h.seg == seg { if let Some(crash) = h.touching(bx, bd, safe) { hits.push((i, crash)); } }
        }
        self.cues.extend(cues);
        if self.state != State::Play { return; }
        for (i, crash) in hits {
            if self.meet(i, crash) { return; }
        }
    }

    /// The bike meets hazard `i`: a gentle bump into another rider, a crash (true: the ride is over), or a puddle, a
    /// sprinkler or the paper cart.
    pub(super) fn meet(&mut self, i: usize, crash: bool) -> bool {
        let h = &mut self.hazards[i];
        // riding into another rider gently is a bump, not a crash: the bike is knocked aside and held to their pace
        let rider = h.rider();
        let (_, hvd) = h.velocity(self.bike_x, self.bike_d);
        if crash && rider && (self.speed - hvd).abs() < 70.0 && self.safe <= 0.0 {
            let away = if self.bike_x < h.x { -1.0 } else { 1.0 };
            self.steer_v = away * 140.0;
            self.bike_x += away * 3.0;
            self.speed = hvd.max(0.0);
            self.bike_d -= if h.d > self.bike_d { 2.0 } else { -2.0 };
            self.safe = 0.35;
            self.cues.push(Cue::Ting);
            blip::bot::add("bumps", 1.0);
            return false;
        }
        if crash {
            if self.safe > 0.0 { return false; }
            if blip::bot::active() {
                eprintln!("CRASH {:?} oncoming={} phase={} h=({:.0},{:.0}) bike=({:.0},{:.0}) v={:.0}",
                    h.kind, h.oncoming, h.phase, h.x, h.d, self.bike_x, self.bike_d, self.speed);
            }
            let what = crash_name(h, self.bike_x);
            self.crash(what);
            return true;
        }
        h.touched = true;
        self.touch(i);
        false
    }

    /// The bike rolls through something harmless: a puddle splashes, a sprinkler soaks a paper, a cart refills the bag.
    fn touch(&mut self, i: usize) {
        let h = &mut self.hazards[i];
        match h.kind {
            Hz::Puddle(_) => {
                self.splash = 0.5;
                let (x, d) = (h.x, h.d);
                self.cues.push(Cue::Splash);
                self.burst(x, d, 10, 60.0, [170, 210, 240]);
                blip::bot::add("puddles", 1.0);
            }
            Hz::Sprinkler => {
                let (x, d) = (self.bike_x, self.bike_d);
                if self.stock > 0 { self.stock -= 1; }
                self.cues.push(Cue::Soggy);
                self.pop(x, d + 30.0, "SOGGY PAPER", [140, 200, 255]);
                blip::bot::add("soaked", 1.0);
            }
            Hz::Cart => {
                h.gone = true;
                let (x, d) = (h.x, h.d);
                self.stock = (self.stock + 10).min(MAX_PAPERS);
                self.cues.push(Cue::Refill);
                self.pop(x, d + 20.0, "+10 PAPERS", [255, 214, 90]);
                self.burst(x, d, 12, 80.0, [255, 240, 200]);
                blip::bot::add("refills", 1.0);
            }
            _ => {}
        }
    }
}

/// What the bike crashed into, for the crash banner: a bus's passenger while it waits at the stop, and the rest by kind.
fn crash_name(h: &Hazard, bike_x: f32) -> &'static str {
    match h.kind {
        Hz::Car if h.bus => "THE BUS",
        Hz::Bus if h.phase == TELL && (h.x - bike_x).abs() > 20.0 => "A PASSENGER",
        Hz::Car => "A CAR", Hz::Dog => "A DOG", Hz::Blader => "A SKATER", Hz::Van => "THE VAN",
        Hz::Cyclist => "A CYCLIST", Hz::CrossCar => "A CROSSING CAR", Hz::Bus => "THE BUS", Hz::Tourist => "A TOURIST",
        Hz::Stall => "A STALL", _ => "SOMEONE",
    }
}
