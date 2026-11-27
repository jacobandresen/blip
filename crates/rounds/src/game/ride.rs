//! The ride: steering and speed, the lights at each junction, turning onto the new street, and the camera that
//! follows the bike.

use super::*;

impl Game {
    /// Steering (-1..1), speed (-1 slower, 1 faster) and the two throws, from whoever holds them.
    pub fn ride(&mut self, steer: f32, pedal: f32, dt: f32) {
        if self.in_junction() { return self.ride_junction(steer, pedal, dt); }
        self.heading = 0.0;
        let target = steer * STEER_V;
        let step = STEER_ACC * dt;
        self.steer_v += (target - self.steer_v).clamp(-step, step);
        self.bike_x += self.steer_v * dt;
        let lim = self.ride_limits();
        if self.bike_x < lim[0] || self.bike_x > lim[1] {
            self.bike_x = self.bike_x.clamp(lim[0], lim[1]);
            self.steer_v = 0.0;
        }
        // the bike never stops, but for a red light (braking toward it)
        let stop = if pedal < 0.0 && self.state == State::Play { self.red_ahead() } else { None };
        let want = if stop.is_some() { 0.0 }
            else if pedal > 0.0 { self.top_speed() }
            else if pedal < 0.0 { SPEED_MIN }
            else { CRUISE * self.pace().min(1.2) };
        let want = want.min(self.top_speed());
        let rate = if stop.is_some() { 300.0 } else if want < self.speed { 160.0 } else { 90.0 };
        self.speed += (want - self.speed).clamp(-rate * dt, rate * dt);
        let slowed = if self.splash > 0.0 { 0.6 } else { 1.0 };
        let before = self.bike_d;
        self.bike_d += self.speed * slowed * dt;
        if let Some(d) = stop {
            if before <= d && self.bike_d > d { self.bike_d = d; self.speed = 0.0; }
        }
    }

    /// The light the bike's street shows at `row`'s junction, and the seconds into its cycle.
    pub fn light(&self, row: usize) -> (Light, f32) {
        let offset = if row == JUNCTIONS[0] { 0.0 } else { 4.0 };
        let t = (self.ride_t + offset).rem_euclid(CYCLE);
        (if t < GREEN_END { Light::Green } else if t < AMBER_END { Light::Amber } else { Light::Red }, t)
    }

    /// Where the bike's nose stops for the light at `row`, as a distance for its middle.
    pub fn stop_d(row: usize) -> f32 { row as f32 * LOT + 16.0 - BIKE_HL - 2.0 }

    /// Which street the bike is on.
    pub fn seg(&self) -> u8 { self.turned as u8 }

    /// Is the bike in the junction of the turn, steering by heading?
    pub fn in_junction(&self) -> bool {
        self.state == State::Play && !self.turned && self.bike_d > JD - MOUTH_HALF - BIKE_HL
    }

    /// Can the bike stand at (`x`, `d`) on the old street: on its road short of the roadworks, or in the new
    /// street's mouth?
    pub(super) fn open_at(x: f32, d: f32) -> bool {
        let road = x > CURB[0] + BIKE_R && x < CURB[1] - BIKE_R && d < WORKS_D - BIKE_R;
        let mouth = d > JD - MOUTH_HALF + BIKE_R && d < JD + MOUTH_HALF - BIKE_R && x < CURB[1] - BIKE_R;
        road || mouth
    }

    /// The camera: where it looks in the world (x, d), and how far it has turned. It follows the bike's heading.
    pub fn camera(&self) -> (f32, f32, f32) {
        let (x, d, a) = if !self.turned {
            let k = (self.heading / std::f32::consts::FRAC_PI_2).clamp(0.0, 1.0);
            (blip::lerp(240.0, self.bike_x, k), self.bike_d, self.heading)
        } else {
            let (x, d) = new_street_to_world(240.0, self.bike_d);
            (x, d, std::f32::consts::FRAC_PI_2)
        };
        (x + self.cam_off[0], d + self.cam_off[1], a + self.cam_off[2])
    }

    /// A car on the old street coming toward the bike, near enough that turning across it now is unsafe.
    pub fn oncoming_near(&self) -> bool {
        self.hazards.iter().any(|h| h.seg == 0 && !h.gone && h.velocity(self.bike_x, self.bike_d).1 < -20.0
            && h.x < 260.0 && h.d > self.bike_d - 30.0 && h.d < self.bike_d + 320.0)
    }

    /// Steering in the junction turns the bike; it rides where it points, and stops against a kerb or the barrier.
    /// A rider who never steers left still gets round: stood unsteered at the barrier (not braking to wait in the
    /// bike box) for JUNCTION_ASSIST seconds, the handlebars ease left once nothing is coming (the day stalled there).
    pub(super) fn ride_junction(&mut self, steer: f32, pedal: f32, dt: f32) {
        let blocked = self.speed < 5.0;
        // Steering or braking is the rider's own: it ends the assist at once.
        self.stuck = if steer != 0.0 || pedal < 0.0 { 0.0 } else if blocked { self.stuck + dt } else { self.stuck };
        let assist = self.stuck > JUNCTION_ASSIST && !self.oncoming_near();
        let steer = if assist {
            if !self.pops.iter().any(|p| p.text == "TURN LEFT") {
                let (x, d) = (self.bike_x, self.bike_d);
                self.pop(x, d + 36.0, "TURN LEFT", [255, 214, 90]);
            }
            // Past the mouth, at the barrier, the rider walks the bike back (straight, 40 px/s) into the new street's
            // mouth; straight on short of it, a quarter turn left inside it.
            let past = self.bike_d > JD + MOUTH_HALF - BIKE_R - 8.0;
            if past {
                self.bike_d -= 40.0 * dt;
                self.speed = 0.0;
            }
            let target = if past || self.bike_d <= JD - 30.0 { 0.0 } else { std::f32::consts::FRAC_PI_2 };
            if self.heading < target - 0.05 { -0.6 } else if self.heading > target + 0.05 { 0.6 } else { 0.0 }
        } else { steer };
        if self.heading == 0.0 && self.steer_v != 0.0 {
            self.heading = (-self.steer_v / self.speed.max(40.0)).atan();
            self.steer_v = 0.0;
        }
        self.heading = (self.heading - steer * TURN_RATE * dt).clamp(-0.35, 1.8);
        // the brakes work here: a cyclist waits to turn across the traffic
        let want = if pedal > 0.0 { CRUISE } else if pedal < 0.0 { 0.0 } else { CRUISE * 0.75 };
        self.speed += (want - self.speed).clamp(-240.0 * dt, 90.0 * dt);
        let (s, c) = self.heading.sin_cos();
        let (nx, nd) = (self.bike_x - s * self.speed * dt, self.bike_d + c * self.speed * dt);
        if Self::open_at(nx, nd) {
            self.bike_x = nx;
            self.bike_d = nd;
        } else {
            if self.speed > 60.0 { self.cues.push(Cue::Thump); }
            self.speed = 0.0;
        }
        if self.bike_x < CURB[0] - 20.0 { self.onto_new_street(); }
    }

    /// Round the corner: the bike's place, speed and drift carried into the new street's own frame, and the camera
    /// eased from where it was.
    pub(super) fn onto_new_street(&mut self) {
        let before = self.camera();
        let (h, v) = (self.heading, self.speed);
        let (x, d) = world_to_new_street(self.bike_x, self.bike_d);
        self.turned = true;
        self.stuck = 0.0;
        self.bike_x = x;
        self.bike_d = d;
        self.speed = (v * h.sin()).max(SPEED_MIN * 0.5);
        self.steer_v = v * h.cos();
        self.heading = 0.0;
        self.cam_off = [0.0; 3];
        let after = self.camera();
        self.cam_off = [before.0 - after.0, before.1 - after.1, before.2 - after.2];
        self.papers.clear();
        blip::bot::add("left_turns", 1.0);
        if blip::bot::active() { eprintln!("TURN t={:.2}", blip::bot::clock()); }
    }

    /// The next junction ahead whose light is not green, within braking distance.
    pub(super) fn red_ahead(&self) -> Option<f32> {
        JUNCTIONS.iter().map(|&r| (r, Self::stop_d(r)))
            .find(|&(r, d)| d >= self.bike_d - 0.5 && d - self.bike_d < 160.0 && self.light(r).0 != Light::Green)
            .map(|(_, d)| d)
    }

    /// Where the bike can ride now: kerb to kerb, or along Nyhavn's quay.
    pub fn ride_limits(&self) -> [f32; 2] {
        if self.state == State::Canal { [QUAY[0] + BIKE_HW + 2.0, QUAY[1] - BIKE_HW - 4.0] } else { RIDE }
    }
}
