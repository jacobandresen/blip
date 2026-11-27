//! The hazards of the round: who and what they are, how each wakes as the bike comes near and moves on, steering round
//! the others, and whether it touches the bike.

use super::*;

/// Something on the street.
#[derive(Clone, Copy)]
pub struct Hazard {
    pub kind: Hz,
    pub side: Side,
    pub x: f32,
    pub d: f32,
    /// The x it came from (a gate, a driveway), for going home.
    pub home: f32,
    pub phase: u8,
    pub t: f32,
    pub gone: bool,
    /// Already did its harmless thing (a splash, a soak) to the bike this time round.
    pub touched: bool,
    /// An oncoming car of the day's traffic, not a parked one backing out.
    pub oncoming: bool,
    /// A dog's paper to fetch: where it is.
    pub fetch: Option<(f32, f32)>,
    /// One of the city's yellow buses, not a car.
    pub bus: bool,
    /// A cyclist waiting at a red light or behind another.
    pub held: bool,
    /// Where a tourist is in its own rhythm of weaving and staring; the guide of a flock has none.
    pub beat: f32,
    pub guide: bool,
    /// Which street it is on: 0 the old, 1 the new after the turn.
    pub seg: u8,
    /// A cyclist has rung its bell at the bike (once).
    pub rang: bool,
}

// Hazard phases.
pub const WAIT: u8 = 0;
pub const TELL: u8 = 1;
pub const GO: u8 = 2;
pub const PAUSE: u8 = 3;
pub const AWAY: u8 = 4;
pub const HOME: u8 = 5;

/// How long each tell runs before the thing moves, s.
pub const CAR_TELL: f32 = 1.2;
pub const DOG_TELL: f32 = 0.9;
pub const HISS: f32 = 0.9;
pub const SPRAY: f32 = 1.8;
pub const SPRAY_OFF: f32 = 1.6;
/// How far ahead each wakes, px.
pub const CAR_WAKE: f32 = 330.0;
pub const DOG_WAKE: f32 = 270.0;
pub const DOG_CHASE: f32 = 2.6;
pub const DOG_SPEED: f32 = 135.0;
/// How far into the street a dog goes after the bike: the middle of the road stays clear.
pub const DOG_REACH: [f32; 2] = [214.0, 266.0];
pub const SPRAY_REACH: f32 = 46.0;

impl Hazard {
    pub(super) fn new(kind: Hz, side: Side, row: usize) -> Self {
        let s = side.index();
        let d = door_d(row);
        let (x, d) = match kind {
            Hz::Car => ([108.0, 372.0][s], d + 30.0),
            Hz::Dog => (BOX_X[s] - side.sign() * 4.0, d - 30.0),
            Hz::Sprinkler => ([112.0, 368.0][s], d - 34.0),
            Hz::Blader | Hz::Cart | Hz::Cyclist | Hz::Tourist => (BIKE_LANE[s], d),
            Hz::Van => (if side == Side::Left { BIKE_LANE[0] } else { CAR_LANE[1] }, d),
            Hz::Puddle(x) | Hz::Pigeons(x) => (x as f32, d),
            Hz::Stall => ([150.0, 330.0][s], d),
            Hz::CrossCar => (if side == Side::Left { -40.0 } else { 520.0 }, row as f32 * LOT + CROSS_LANES[s]),
            // at the stop beside the cycle track, its doors on the kerb side
            Hz::Bus => (if side == Side::Left { CURB[0] + 28.0 + 14.0 } else { CURB[1] - 28.0 - 14.0 }, d),
            Hz::Walker => ([140.0, 340.0][s], d),
        };
        Self {
            kind, side, x, d, home: x, phase: WAIT, t: 0.0, gone: false, touched: false, oncoming: false, fetch: None, bus: false,
            held: false, beat: 0.0, guide: false, rang: false, seg: (d >= NEW_STREET_ROW as f32 * LOT) as u8,
        }
    }

    pub(super) fn traffic(row: usize) -> Self {
        // every other one is a yellow city bus
        Self { oncoming: true, bus: row.is_multiple_of(2), x: CAR_LANE[0], d: door_d(row), home: CAR_LANE[0], ..Self::new(Hz::Car, Side::Left, row) }
    }

    /// Half its size across and along the street, as the bike would hit it; zero for what cannot be hit.
    pub fn size(&self) -> (f32, f32) {
        match self.kind {
            Hz::Car if self.bus => (13.0, 40.0),
            Hz::Car if self.oncoming || self.phase >= AWAY => (11.0, 20.0),
            // turning into its lane
            Hz::Car if self.phase == PAUSE => (11.0, 20.0),
            // backing out across the pavement, nose to the house
            Hz::Car => (19.0, 10.0),
            Hz::Dog if self.phase >= GO => (7.0, 9.0),
            Hz::Blader | Hz::Walker => (6.0, 7.0),
            Hz::Cyclist | Hz::Tourist => (6.0, 13.0),
            Hz::CrossCar => (20.0, 10.0),
            Hz::Bus => (13.0, 44.0),
            Hz::Van => (13.0, 30.0),
            Hz::Stall => (25.0, 32.0),
            _ => (0.0, 0.0),
        }
    }

    /// Its size once it is out driving (a parked car backing out is turned across the street before that).
    pub fn size_driving(&self) -> (f32, f32) {
        match self.kind { Hz::Car if !self.bus => (11.0, 20.0), Hz::Car | Hz::Bus => (13.0, 44.0), Hz::Van => (13.0, 30.0), _ => self.size() }
    }

    /// Is it out where it can be hit?
    /// Another rider on the cycle track: a cyclist, a tourist on a hire bike, a skater.
    pub fn rider(&self) -> bool { matches!(self.kind, Hz::Cyclist | Hz::Tourist | Hz::Blader) }
    /// Something with an engine: a car (reversing out or oncoming), a bus, a van, a car crossing the junction.
    pub fn vehicle(&self) -> bool { matches!(self.kind, Hz::Car | Hz::Bus | Hz::Van | Hz::CrossCar) }
    pub fn solid(&self) -> bool {
        let (hw, _) = self.size();
        if hw == 0.0 || self.gone { return false; }
        match self.kind {
            Hz::Car => self.oncoming || self.phase >= GO,
            Hz::Dog => self.phase == GO || self.phase == HOME || self.phase == AWAY,
            _ => true,
        }
    }

    /// How it is moving now, px/s.
    pub fn velocity(&self, bike_x: f32, bike_d: f32) -> (f32, f32) {
        // waiting for something in its way (or a red light): it stands still
        if self.held && self.kind != Hz::Dog { return (0.0, 0.0); }
        let s = self.side.sign();
        match (self.kind, self.phase) {
            (Hz::Car, _) if self.oncoming => {
                let blocked = (bike_x - self.x).abs() < 26.0 && (0.0..70.0).contains(&(self.d - bike_d));
                (0.0, if self.phase >= GO && !blocked { -110.0 } else { 0.0 })
            }
            (Hz::Car, GO) => (-s * 65.0, 0.0),
            // pulling away, but never into the bike: a driver waits for one just ahead or alongside
            (Hz::Car, AWAY) => {
                let gap = (bike_d - self.d) * s;
                let alongside = (bike_x - self.x).abs() < 26.0 && gap > -10.0 && gap < 100.0;
                (0.0, if alongside { 0.0 } else { s * if self.side == Side::Right { 160.0 } else { 120.0 } })
            }
            (Hz::Dog, GO) => {
                let tx = self.chase_x(bike_x);
                ((tx - self.x).clamp(-1.0, 1.0) * 120.0, ((bike_d - self.d) * 3.0).clamp(-DOG_SPEED, DOG_SPEED))
            }
            (Hz::Blader, GO) => (0.0, -70.0),
            (Hz::Cyclist, GO) => (0.0, if self.side == Side::Right { 58.0 } else { -85.0 }),
            (Hz::CrossCar, _) => (if self.side == Side::Left { CROSS_SPEED } else { -CROSS_SPEED }, 0.0),
            (Hz::Tourist, GO) => {
                let (_, staring, _) = tourist_phase(self.t);
                let pace = if staring { 0.3 } else { 1.0 };
                let vd = if self.side == Side::Right { 80.0 } else { -95.0 } * pace;
                if self.guide { return (0.0, vd); }
                (tourist_x(self.home, self.side, self.t + self.beat + 0.05) - tourist_x(self.home, self.side, self.t + self.beat), vd).into_rate(0.05)
            }
            (Hz::Bus, AWAY) => (0.0, s * if self.side == Side::Right { 120.0 } else { 100.0 }),
            (Hz::Cart, GO) => (0.0, -24.0),
            (Hz::Van, GO) => (0.0, 62.0),
            (Hz::Walker, GO) => (-s * 34.0, 0.0),
            _ => (0.0, 0.0),
        }
    }

    pub(super) fn chase_x(&self, bike_x: f32) -> f32 {
        if self.side == Side::Left { bike_x.min(DOG_REACH[0]) } else { bike_x.max(DOG_REACH[1]) }
    }
}

/// A tourist's ride, by the seconds since it set off: (weaving, staring at the screen, how far out it has swerved 0..1).
pub fn tourist_phase(t: f32) -> (f32, bool, f32) {
    let c = t.rem_euclid(4.6);
    let out = if c < 3.4 { 0.0 } else if c < 4.0 { (c - 3.4) / 0.6 } else { 1.0 - (c - 4.0) / 0.6 };
    ((t * 2.5).sin() * 7.0, (2.6..3.4).contains(&c), out)
}

/// Where a tourist is across the street: its lane, a weave, and the swerve out into the road.
pub fn tourist_x(home: f32, side: Side, t: f32) -> f32 {
    let (weave, _, out) = tourist_phase(t);
    home + weave - side.sign() * 36.0 * blip::smoothstep01(out)
}

trait Rate { fn into_rate(self, dt: f32) -> (f32, f32); }
impl Rate for (f32, f32) { fn into_rate(self, dt: f32) -> (f32, f32) { (self.0 / dt, self.1) } }

/// Is this row Dronning Louises Bro?
pub fn bridge_rows(row: usize) -> bool { lot(Side::Left, row).0 == Lot::Bridge }

/// Where the other road users are this frame, for the hazards that steer round them.
pub struct Traffic {
    riders: Vec<(f32, f32, u8)>,
    /// vans parked on the cycle track, which riders swing out around
    vans: Vec<(f32, f32, u8)>,
    /// every vehicle's box, so a tourist's swerve never takes it into one
    vehicles: Vec<(f32, f32, f32, f32, u8)>,
}

impl Traffic {
    pub(super) fn of(hazards: &[Hazard]) -> Traffic {
        let live = || hazards.iter().filter(|h| !h.gone);
        Traffic {
            riders: live().filter(|h| h.rider()).map(|h| (h.x, h.d, h.seg)).collect(),
            vans: live().filter(|h| h.kind == Hz::Van && h.solid()).map(|h| (h.x, h.d, h.seg)).collect(),
            vehicles: live().filter(|h| h.solid() && h.vehicle())
                .map(|h| { let (w, l) = h.size(); (h.x, h.d, w, l, h.seg) }).collect(),
        }
    }
    pub(super) fn rider_at(&self, x: f32, d: f32, s: u8) -> bool {
        self.riders.iter().any(|&(rx, rd, rs)| rs == s && (rx - x).abs() < 18.0 && (rd - d).abs() < 24.0)
    }
    /// How close the nearest rider is, in rider boxes.
    pub(super) fn rider_gap(&self, x: f32, d: f32, s: u8) -> f32 {
        self.riders.iter().filter(|r| r.2 == s).map(|&(rx, rd, _)| ((rx - x) / 18.0).abs().max(((rd - d) / 24.0).abs())).fold(f32::MAX, f32::min)
    }
    /// A dog waits rather than run through a rider; one already near a rider may only move away from it.
    pub(super) fn dog_may(&self, x: f32, d: f32, nx: f32, nd: f32, s: u8) -> bool {
        !self.rider_at(nx, nd, s) || self.rider_at(x, d, s) && self.rider_gap(nx, nd, s) > self.rider_gap(x, d, s)
    }
    pub(super) fn vehicle_at(&self, x: f32, d: f32, s: u8) -> bool {
        self.vehicles.iter().any(|&(vx, vd, w, l, vs)| vs == s && (vx - x).abs() < w + 8.0 && (vd - d).abs() < l + 15.0)
    }
}

impl Hazard {
    /// Wakes as the bike comes near: a car reverses out, a dog rattles its gate and gives chase, a sprinkler hisses
    /// and sprays, riders and walkers set off, a bus stops and leaves, pigeons scatter. A cyclist in the bike's line
    /// rings its bell about BELL_SECS before they would meet: a tell you hear.
    pub(super) fn wake(&mut self, bx: f32, bd: f32, speed: f32, seg: u8, cues: &mut Vec<Cue>) {
        let h = self;
        let ahead = h.d - bd;
        if h.kind == Hz::Cyclist && h.phase == GO && !h.rang && h.seg == seg && (h.x - bx).abs() < 20.0 + BIKE_HW {
            let closing = speed - h.velocity(bx, bd).1;
            if ahead > 0.0 && closing > 20.0 && ahead / closing < BELL_SECS {
                h.rang = true;
                cues.push(Cue::Bell);
                blip::bot::add("bells", 1.0);
            }
        }
        match h.kind {
            Hz::Car if h.oncoming && h.phase == WAIT && ahead < 620.0 => h.phase = GO,
            Hz::Car if h.oncoming => {}
            Hz::Car => match h.phase {
                WAIT if ahead < CAR_WAKE => { h.phase = TELL; h.t = 0.0; cues.push(Cue::Reverse); }
                TELL if h.t > CAR_TELL && !h.held => { h.phase = GO; h.t = 0.0; }
                GO if (h.x - CAR_LANE[h.side.index()]).abs() < 2.0 => { h.phase = PAUSE; h.t = 0.0; }
                // it turns into the lane only with the bike clear of it
                PAUSE if h.t > 0.5 && !h.held && ((bx - h.x).abs() > 30.0 || (bd - h.d).abs() > 70.0) => { h.phase = AWAY; h.t = 0.0; }
                _ => {}
            },
            Hz::Dog => match h.phase {
                WAIT if ahead < DOG_WAKE => { h.phase = TELL; h.t = 0.0; cues.push(Cue::Rattle); }
                TELL if h.t > DOG_TELL => { h.phase = GO; h.t = 0.0; cues.push(Cue::Bark); }
                GO if h.t > DOG_CHASE => { h.phase = HOME; h.t = 0.0; }
                _ => {}
            },
            Hz::Sprinkler => if ahead < 520.0 && ahead > -200.0 {
                let cycle = h.t % (SPRAY_OFF + HISS + SPRAY);
                let phase = if cycle < SPRAY_OFF { WAIT } else if cycle < SPRAY_OFF + HISS { TELL } else { GO };
                if phase == TELL && h.phase != TELL && ahead < 400.0 { cues.push(Cue::Hiss); }
                if phase != GO { h.touched = false; }
                h.phase = phase;
            } else { h.t = 0.0; },
            Hz::Blader | Hz::Cart | Hz::Walker | Hz::Cyclist | Hz::Tourist if h.phase == WAIT && ahead < match h.kind {
                Hz::Tourist => 480.0,
                Hz::Blader => 560.0, Hz::Cart => 520.0, Hz::Cyclist if h.side == Side::Left => 600.0, Hz::Cyclist => 420.0,
                _ => 380.0 } => { h.phase = GO; h.t = 0.0; }
            Hz::Van if h.side == Side::Right && h.phase == WAIT && ahead < 420.0 => h.phase = GO,
            Hz::Bus => match h.phase {
                WAIT if ahead < BUS_WAKE => { h.phase = TELL; h.t = 0.0; }
                TELL if h.t > BUS_DWELL => { h.phase = AWAY; h.t = 0.0; }
                _ => {}
            },
            Hz::Pigeons(_) if h.phase == WAIT && ahead < 130.0 && (h.x - bx).abs() < 90.0 => {
                h.phase = GO; h.t = 0.0; cues.push(Cue::Coo);
            }
            _ => {}
        }
    }

    /// Moves on: at its own pace, steering round the others (a dog waits for riders, a tourist's swerve for
    /// vehicles, riders swing out past a parked van), and keeping to its lane.
    pub(super) fn travel(&mut self, dt: f32, bx: f32, bd: f32, traffic: &Traffic) {
        let h = self;
        let (vx, vd) = h.velocity(bx, bd);
        let dog_waits = h.kind == Hz::Dog && !traffic.dog_may(h.x, h.d, h.x + vx * 0.15, h.d + vd * 0.15, h.seg);
        if !dog_waits {
            h.x += vx * dt;
            h.d += vd * dt;
        }
        // Leaving its stop a bus pulls out into its lane (30 px/s), clear of the riders beside it (on the narrow
        // bridge they rode into a bus still at the stop's line).
        if h.kind == Hz::Bus && h.phase == AWAY && !h.held {
            let lane = CAR_LANE[h.side.index()];
            h.x += (lane - h.x).clamp(-30.0 * dt, 30.0 * dt);
        }
        if h.kind == Hz::Car && h.phase == GO && !h.oncoming {
            let lane = CAR_LANE[h.side.index()];
            h.x = if h.side == Side::Left { h.x.min(lane) } else { h.x.max(lane) };
        }
        if h.kind == Hz::Tourist && h.phase == GO && !h.guide && !h.held {
            let x = tourist_x(h.home, h.side, h.t + h.beat);
            // the swerve waits while a vehicle stands beside the lane (it took tourists into buses at their stops)
            h.x = if traffic.vehicle_at(x, h.d, h.seg) { h.x + (h.home - h.x).clamp(-60.0 * dt, 60.0 * dt) } else { x };
        }
        if h.kind == Hz::Tourist && h.held { h.t -= dt; }
        // Back to the lane (the bridge's is narrower), but not while held: a rider waiting beside a bus drifted into it.
        if h.rider() && !h.held {
            let on_bridge = row_at(h.d).is_some_and(bridge_rows);
            let lane = if on_bridge { [RAIL[0] + 9.0, RAIL[1] - 9.0][h.side.index()] } else { h.home };
            // a van on the track ahead (or alongside): out into the road past it, 13 + 6 px clear and a margin
            let (_, vd) = h.velocity(bx, bd);
            let lane = traffic.vans.iter().find(|&&(vx, vd0, vs)| vs == h.seg && (vx - h.home).abs() < 10.0
                && (-50.0..90.0).contains(&((vd0 - h.d) * vd.signum())))
                .map_or(lane, |&(vx, _, _)| vx - h.side.sign() * 24.0);
            if h.kind != Hz::Tourist || h.guide { h.x += (lane - h.x).clamp(-60.0 * dt, 60.0 * dt); }
        }
        if h.kind == Hz::Dog && (h.phase == HOME || h.phase == AWAY) {
            // a fetch first, then home through the gate
            let (tx, td) = h.fetch.filter(|_| h.phase == AWAY).unwrap_or((h.home, h.d));
            let k = ((160.0 * dt) / ((tx - h.x).hypot(td - h.d)).max(1.0)).min(1.0);
            let (nx, nd) = (h.x + (tx - h.x) * k * 9.0, h.d + (td - h.d) * k * 9.0);
            if traffic.dog_may(h.x, h.d, nx, nd, h.seg) {
                h.x += (tx - h.x) * k;
                h.d += (td - h.d) * k;
            }
            if h.phase == AWAY && (tx - h.x).abs() < 3.0 && (td - h.d).abs() < 3.0 && h.t > 0.8 { h.phase = HOME; h.fetch = None; }
            if h.phase == HOME && (h.x - h.home).abs() < 3.0 { h.phase = PAUSE; }
        }
    }

    /// Whether it touches the bike: Some(true) a crash, Some(false) a soak, a splash or the paper cart.
    pub(super) fn touching(&self, bx: f32, bd: f32, safe: f32) -> Option<bool> {
        let h = self;
        let (hw, hl) = h.size();
        let near_x = (h.x - bx).abs();
        let near_d = (h.d - bd).abs();
        match h.kind {
            Hz::Puddle(_) if near_x < 20.0 + BIKE_HW && near_d < 14.0 + BIKE_HL && !h.touched => Some(false),
            Hz::Sprinkler if h.phase == GO && !h.touched && near_d < SPRAY_REACH + BIKE_HL
                && (bx - h.x).abs() < (CURB[0] - FENCE[0]) + SPRAY_REACH => {
                let reach = if h.side == Side::Left { bx - BIKE_HW < CURB[0] + SPRAY_REACH } else { bx + BIKE_HW > CURB[1] - SPRAY_REACH };
                reach.then_some(false)
            }
            Hz::Cart if near_x < 14.0 + BIKE_HW && near_d < 14.0 + BIKE_HL => Some(false),
            // passengers crossing the cycle track to and from the bus
            Hz::Bus if h.phase == TELL && (bx - BIKE_LANE[h.side.index()]).abs() < 13.0 + BIKE_HW && (bd - h.d).abs() < 12.0 + BIKE_HL
                && safe <= 0.0 => Some(true),
            _ if h.solid() && near_x < hw + BIKE_HW - CRASH_INSET.0 && near_d < hl + BIKE_HL - CRASH_INSET.1 => Some(true),
            _ => None,
        }
    }
}
