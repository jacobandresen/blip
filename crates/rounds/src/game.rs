//! The rules of the round: the bike, the papers in flight, the houses' goodwill, the hazards, the week and the
//! Canal Run. Drawing is in draw.rs; nothing here draws or plays a sound, it only queues them (`Game::cues`).

use crate::levels::*;
use blip::{rand_range_f32, Session, Timer};

pub const WIN_W: i32 = 480;
pub const WIN_H: i32 = 540;
pub const HUD_H: f32 = 28.0;
/// The strip under the street: day, papers and goodwill.
pub const BAR_H: f32 = 30.0;
/// Where the bike rides on screen: most of the view is the street ahead.
pub const BIKE_Y: f32 = 392.0;
/// One lot along the street, px.
pub const LOT: f32 = 110.0;

// The street across, px: houses, front gardens, fences, pavements, bike lanes and the road, mirrored.
pub const WALL: [f32; 2] = [84.0, 396.0];
pub const FENCE: [f32; 2] = [126.0, 354.0];
pub const CURB: [f32; 2] = [150.0, 330.0];
pub const BIKE_LANE: [f32; 2] = [163.0, 317.0];
pub const CAR_LANE: [f32; 2] = [208.0, 272.0];
/// A bridge's railings.
pub const RAIL: [f32; 2] = [178.0, 302.0];
/// How deep the doorstep is, from the wall.
pub const STEP: f32 = 16.0;
/// Where a non-subscriber's sample box stands: x, and along the street from the door.
pub const BOX_X: [f32; 2] = [113.0, 367.0];
pub const BOX_D: f32 = 24.0;
pub const BOX_HALF: f32 = 9.0;
/// Half the doorstep along the street, and half the letterbox slot.
pub const DOOR_HALF: f32 = 22.0;
pub const SLOT_HALF: f32 = 6.0;

/// A throw: sideways speed and time in the air, and how much of the bike's speed the paper keeps.
pub const THROW_V: f32 = 300.0;
pub const FLIGHT: f32 = 0.5;
pub const CARRY: f32 = 0.5;
/// How far a paper slides on a wet street.
pub const SKID: f32 = 34.0;
pub const SKID_SECS: f32 = 0.25;
pub const THROW_GAP: f32 = 0.22;

pub const SPEED_MIN: f32 = 70.0;
pub const CRUISE: f32 = 115.0;
pub const SPEED_MAX: f32 = 185.0;
/// Market day's top speed.
pub const MARKET_MAX: f32 = 135.0;
/// The handlebars: steering speed and how fast it builds and settles (a damped feel, not a twitch).
pub const STEER_V: f32 = 175.0;
pub const STEER_ACC: f32 = 900.0;
pub const BIKE_HW: f32 = 8.0;
pub const BIKE_HL: f32 = 18.0;
/// A solid thing and the bike crash only once their boxes overlap this far (px across, along): a brush is forgiven.
pub const CRASH_INSET: (f32, f32) = (2.0, 3.0);
/// Where the bike can ride: kerb to kerb.
pub const RIDE: [f32; 2] = [CURB[0] + BIKE_HW + 2.0, CURB[1] - BIKE_HW - 2.0];

pub const LIVES: i32 = 3;
/// How long a continue is offered at game over.
pub const CONTINUE_SECS: f32 = 9.0;
pub const MAX_PAPERS: i32 = 40;
pub const CRASH_SECS: f32 = 1.6;
pub const SAFE_SECS: f32 = 2.0;
pub const CARD_SECS: f32 = 3.0;
pub const TALLY_SECS: f32 = 4.5;
pub const CANAL_SECS: f32 = 18.0;
/// The chance a sampled neighbour subscribes the next day.
pub const WIN_OVER: f32 = 0.6;

pub const LETTERBOX_PTS: i32 = 250;
pub const DOORSTEP_PTS: i32 = 100;
pub const SAMPLE_PTS: i32 = 150;
pub const PERFECT_PTS: i32 = 1000;
pub const KEPT_PTS: i32 = 200;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State { Title, Card, Play, Crash, Tally, Canal, Week, Over }

/// A traffic light, as the bike sees it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Light { Green, Amber, Red }

/// One turn of the lights, s: green for the bike's street, amber, then red while the side street drives.
pub const CYCLE: f32 = 11.0;
pub const GREEN_END: f32 = 5.5;
pub const AMBER_END: f32 = 6.7;
/// Side-street cars may cross their line only in this part of the bike's red, so every one is over the junction
/// before the bike's street gets green again; the rest of the time they queue at their lines.
pub const CROSS_GO: [f32; 2] = [6.9, 8.6];
/// How often a car comes down each side-street lane, s.
pub const CROSS_EVERY: f32 = 2.4;
pub const CROSS_SPEED: f32 = 170.0;
/// The side street's two lanes, from the junction row's start: eastbound nearer, westbound farther (they keep right).
pub const CROSS_LANES: [f32; 2] = [35.0, 75.0];
/// A bus at its stop: how long it waits once the bike is near, and where its passengers cross the cycle track.
pub const BUS_DWELL: f32 = 3.2;
pub const BUS_WAKE: f32 = 420.0;
/// The left turn: the middle of Frederiksgade's mouth along Store Kongensgade, its half width (road), and the
/// roadworks closing Store Kongensgade beyond it.
pub const JD: f32 = 29.0 * LOT;
pub const MOUTH_HALF: f32 = 90.0;
pub const WORKS_D: f32 = NEW_STREET_ROW as f32 * LOT + 26.0;
/// How fast the handlebars turn the bike in the junction, rad/s, and how round the bike is there.
pub const TURN_RATE: f32 = 1.9;
/// How long before meeting the bike a cyclist in its line rings its bell, seconds (a person's reaction and then some).
pub const BELL_SECS: f32 = 1.4;
/// Seconds stood unsteered at the junction's barrier before the bike turns itself.
pub const JUNCTION_ASSIST: f32 = 3.0;
pub const BIKE_R: f32 = 10.0;

/// Store Kongensgade's point (`x`, `d`) in the world, where it is the frame; Frederiksgade's runs off to the left of
/// the junction, turned a quarter.
pub fn new_street_to_world(x: f32, d: f32) -> (f32, f32) { (240.0 - (d - JD), JD + (x - 240.0)) }
pub fn world_to_new_street(x: f32, d: f32) -> (f32, f32) { (240.0 + (d - JD), JD + (240.0 - x)) }

/// What became of a paper.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome { Letterbox, Doorstep, Sample, Garden, Street, Splash }

/// One house (or shop, park, bridge) along one side.
#[derive(Clone, Copy)]
pub struct House {
    pub lot: Lot,
    pub sub: bool,
    pub hearts: u8,
    /// Delivered today; a sample landed today.
    pub done: bool,
    pub sampled: bool,
    /// Judged once the bike is past it.
    pub judged: bool,
    /// Seconds since something happened here, for the drawing: the flap, a wave, a broken heart.
    pub flap: f32,
    pub wave: f32,
    pub sad: f32,
    pub cancelled: f32,
    /// Seconds since this house joined the round (from the day card on).
    pub joined: f32,
}

impl House {
    fn new(lot: Lot, sub: bool) -> Self {
        Self { lot, sub, hearts: 2, done: false, sampled: false, judged: false,
            flap: 9.0, wave: 9.0, sad: 9.0, cancelled: 9.0, joined: 9.0 }
    }
}

#[derive(Clone, Copy)]
pub struct Paper {
    pub x: f32,
    pub d: f32,
    pub vx: f32,
    pub vd: f32,
    pub t: f32,
    pub side: Side,
    pub by: usize,
    /// Where it will end, worked out when thrown.
    pub f: Flight,
    /// Landed: how long it has lain there, and how.
    pub landed: Option<(f32, Outcome)>,
}

impl Paper {
    /// Where it is now, and how high (0 on the ground, 1 at the top of its arc).
    pub fn at(&self) -> (f32, f32, f32) {
        if self.landed.is_some() { return (self.f.x, self.f.d, 0.0); }
        let t = self.t.min(self.f.t);
        let air = t.min(FLIGHT);
        let h = (air / FLIGHT * std::f32::consts::PI).sin();
        if self.f.wall && !self.f.slid {
            let k = (t / self.f.t.max(1e-3)).min(1.0);
            return (blip::lerp(self.x, self.f.x, k), blip::lerp(self.d, self.f.d, k), h);
        }
        if t <= FLIGHT { return (self.x + self.vx * t, self.d + self.vd * t, h); }
        // sliding on the wet street
        let k = (t - FLIGHT) / (self.f.t - FLIGHT).max(1e-3);
        let (ax, ad) = (self.x + self.vx * FLIGHT, self.d + self.vd * FLIGHT);
        (blip::lerp(ax, self.f.x, k), ad, 0.0)
    }
}

/// Where a paper thrown now will end: its last x and d, and whether it hit the house wall (and if it only slid there).
#[derive(Clone, Copy, Debug)]
pub struct Flight { pub x: f32, pub d: f32, pub wall: bool, pub slid: bool, pub t: f32 }

/// The whole flight of a paper thrown from `(x, d)` at bike speed `v`, worked out at once: a throw is fair
/// because it is exact. Papers fly to the wall or land short, and on a wet street slide on.
pub fn flight(x: f32, d: f32, v: f32, side: Side, wind: f32, rain: bool) -> Flight {
    let vd = v * CARRY + wind;
    let wall = WALL[side.index()];
    let reach = (wall - x).abs();
    let fly = THROW_V * FLIGHT;
    if reach <= fly {
        let t = reach / THROW_V;
        return Flight { x: wall, d: d + vd * t, wall: true, slid: false, t };
    }
    let (mut lx, ld) = (x + side.sign() * fly, d + vd * FLIGHT);
    if rain {
        let slide = SKID.min(reach - fly);
        lx += side.sign() * slide;
        let t = FLIGHT + SKID_SECS * slide / SKID;
        if slide < SKID { return Flight { x: wall, d: ld, wall: true, slid: true, t }; }
        return Flight { x: lx, d: ld, wall: false, slid: true, t };
    }
    Flight { x: lx, d: ld, wall: false, slid: false, t: FLIGHT }
}

/// Half the footprint of Frederik V's pedestal, in the middle of the road on the palace square.
pub const STATUE_HALF: f32 = 16.0;

/// Where along the street the statue stands.
pub fn statue_d() -> f32 { STATUE_ROW as f32 * LOT }

/// The row a distance along the street is on.
pub fn row_at(d: f32) -> Option<usize> {
    (d >= 0.0).then(|| (d / LOT) as usize).filter(|r| *r < ROWS)
}

/// The middle of row `row`, where its door is.
pub fn door_d(row: usize) -> f32 { row as f32 * LOT + LOT * 0.5 }

/// What a paper ending at `f` on `side` hits, by where it is.
pub fn outcome(side: Side, f: Flight, lot: Option<Lot>) -> (Outcome, Option<usize>) {
    let row = row_at(f.d);
    let s = side.index();
    let Some((row, lot)) = row.zip(lot) else { return (Outcome::Street, None) };
    if lot == Lot::Bridge { return (Outcome::Splash, Some(row)); }
    let off = (f.d - door_d(row)).abs();
    let beyond_fence = if side == Side::Left { f.x > FENCE[0] } else { f.x < FENCE[1] };
    if beyond_fence { return (Outcome::Street, Some(row)); }
    if !lot.is_house() { return (Outcome::Garden, Some(row)); }
    if f.wall {
        if !f.slid && off <= SLOT_HALF { return (Outcome::Letterbox, Some(row)); }
        return (if off <= DOOR_HALF { Outcome::Doorstep } else { Outcome::Garden }, Some(row));
    }
    if (f.x - WALL[s]).abs() <= STEP && off <= DOOR_HALF { return (Outcome::Doorstep, Some(row)); }
    if (f.x - BOX_X[s]).abs() <= BOX_HALF && (f.d - door_d(row) - BOX_D).abs() <= BOX_HALF {
        return (Outcome::Sample, Some(row));
    }
    (Outcome::Garden, Some(row))
}

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
    fn new(kind: Hz, side: Side, row: usize) -> Self {
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
        Self { kind, side, x, d, home: x, phase: WAIT, t: 0.0, gone: false, touched: false, oncoming: false, fetch: None, bus: false, held: false, beat: 0.0, guide: false, rang: false,
            seg: (d >= NEW_STREET_ROW as f32 * LOT) as u8 }
    }

    fn traffic(row: usize) -> Self {
        // every other one is a yellow city bus
        Self { oncoming: true, bus: row % 2 == 0, x: CAR_LANE[0], d: door_d(row), home: CAR_LANE[0], ..Self::new(Hz::Car, Side::Left, row) }
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
                let (_, staring, out) = tourist_phase(self.t);
                let pace = if staring { 0.3 } else { 1.0 };
                let vd = if self.side == Side::Right { 80.0 } else { -95.0 } * pace;
                let _ = out;
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

    fn chase_x(&self, bike_x: f32) -> f32 {
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

/// Words and sparks in the street, carried along with it.
pub struct Pop { pub x: f32, pub d: f32, pub text: String, pub color: [u8; 3], pub t: f32 }
pub struct Bit { pub x: f32, pub d: f32, pub vx: f32, pub vd: f32, pub t: f32, pub life: f32, pub color: [u8; 3], pub size: f32 }

/// Something to hear, queued by the rules for main.rs to play.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cue {
    Throw, Flap, Thump, Plop, Sample, Splash, Crash, Refill, HeartLost, Cancelled, Bell, Reverse, Rattle, Bark,
    Hiss, Coo, Soggy, Target, DayStart, DayEnd, Perfect, Join, Empty, Church, Ting,
}

/// The day's tally, for the card after it.
#[derive(Clone, Copy, Default)]
pub struct Tally {
    pub letterbox: i32,
    pub doorstep: i32,
    pub samples: i32,
    pub misses: i32,
    pub lost: i32,
    pub won: i32,
    pub perfect: i32,
    pub canal: i32,
    pub canal_hits: i32,
}

/// What a paper can land on in Nyhavn.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mark { Table, Ship, TourBoat }

/// A Canal Run target: a café table on the terrace (left), the deck of an old ship moored at the quay or a canal
/// tour boat passing (right).
#[derive(Clone, Copy)]
pub struct Target { pub x: f32, pub d: f32, pub vd: f32, pub hit: f32, pub mark: Mark }

impl Target {
    pub fn side(&self) -> Side { if self.x < QUAY_MID { Side::Left } else { Side::Right } }
}

/// Nyhavn's north quay, where the bike rides the Canal Run; the café terraces are left of it, the canal right.
pub const QUAY: [f32; 2] = [150.0, 252.0];
pub const QUAY_MID: f32 = 201.0;
/// Where the canal meets the harbour, from the start of the run.
pub const HARBOUR_D: f32 = CANAL_SECS * CRUISE + 260.0;

pub struct Game {
    pub state: State,
    /// The continue offered at game over while homes remain, and the best score of the run before one was taken (a
    /// continue starts the score afresh; the run's best is what is posted).
    pub cont: blip::Continue,
    pub run_best: i32,
    pub sess: Session,
    /// Player two's score; the session's is player one's.
    pub score2: i32,
    pub two_player: bool,
    pub week: i32,
    /// The day of the week, 0 Monday.
    pub day: usize,
    pub houses: [Vec<House>; 2],
    pub hazards: Vec<Hazard>,
    pub papers: Vec<Paper>,
    pub stock: i32,
    pub bike_x: f32,
    pub bike_d: f32,
    pub steer_v: f32,
    pub speed: f32,
    pub safe: f32,
    pub throw_cd: [f32; 2],
    pub timer: Timer,
    pub wait: Timer,
    pub tally: Tally,
    pub pops: Vec<Pop>,
    pub bits: Vec<Bit>,
    pub cues: Vec<Cue>,
    /// Seconds into the current state, for the drawing.
    pub clock: f32,
    pub targets: Vec<Target>,
    /// The crash: which way the bike went over.
    pub crash_dir: f32,
    /// The best this machine has seen, for the screens.
    pub banner: Option<(String, f32)>,
    /// Puddle slow-down left, s.
    pub splash: f32,
    /// The part of town the bike is in (an index into `PLACES`), and how long since it rode in.
    pub place: usize,
    pub place_t: f32,
    /// Seconds of riding today: the traffic lights run on it.
    pub ride_t: f32,
    /// Through today's left turn.
    pub turned: bool,
    /// When each junction's lane may send its next car.
    pub cross_next: [[f32; 2]; 2],
    /// When each cycle track sends its next rider.
    pub cycle_next: [f32; 2],
    /// The bike's heading in the junction (0 straight on, a quarter turn to the left onto the new street).
    pub heading: f32,
    /// Seconds the bike has stood against the junction's barrier unsteered (JUNCTION_ASSIST then turns it).
    pub stuck: f32,
    /// What is left of a jump in the camera (world x, d and its turn), easing away.
    pub cam_off: [f32; 3],
}

impl Game {
    pub fn new() -> Self {
        let mut g = Self {
            state: State::Title, sess: Session::new(LIVES), cont: blip::Continue::default(), run_best: 0, score2: 0, two_player: false, week: 1, day: 0,
            houses: [Vec::new(), Vec::new()], hazards: Vec::new(), papers: Vec::new(), stock: 0,
            bike_x: 240.0, bike_d: LOT * 1.2, steer_v: 0.0, speed: CRUISE, safe: 0.0, throw_cd: [0.0; 2],
            timer: Timer::default(), wait: Timer::default(), tally: Tally::default(), pops: Vec::new(), bits: Vec::new(),
            cues: Vec::new(), clock: 0.0, targets: Vec::new(), crash_dir: 1.0, banner: None, splash: 0.0,
            place: 0, place_t: 0.0, ride_t: 0.0, turned: false, cross_next: [[0.0; 2]; 2], cycle_next: [0.0; 2], heading: 0.0, stuck: 0.0, cam_off: [0.0; 3],
        };
        g.reset_route();
        g.load_day();
        g
    }

    pub fn reset_route(&mut self) {
        for side in Side::BOTH {
            self.houses[side.index()] = (0..ROWS).map(|r| { let (l, s) = lot(side, r); House::new(l, s) }).collect();
        }
    }

    pub fn today(&self) -> &'static Day { &DAYS[self.day] }
    pub fn weather(&self) -> Weather { self.today().weather }
    pub fn wind(&self) -> f32 { if let Weather::Wind(w) = self.weather() { w * self.week_sign() } else { 0.0 } }
    /// The wind turns round every other week.
    fn week_sign(&self) -> f32 { if self.week % 2 == 0 { -1.0 } else { 1.0 } }
    pub fn rain(&self) -> bool { self.weather() == Weather::Rain }
    /// Each week rides a little faster.
    pub fn pace(&self) -> f32 { 1.0 + 0.1 * (self.week - 1).min(4) as f32 }
    /// A hard street: the music hurries for it.
    pub fn hard(&self) -> bool { self.week > 1 || self.day >= 5 }
    pub fn top_speed(&self) -> f32 {
        if self.weather() == Weather::Market { MARKET_MAX } else { SPEED_MAX * self.pace().min(1.2) }
    }
    pub fn subscribers(&self) -> usize { self.houses.iter().flatten().filter(|h| h.sub).count() }
    /// Points for a delivery today.
    fn worth(&self, pts: i32) -> i32 { if self.weather() == Weather::Bundles { pts * 2 } else { pts } }
    /// How far along today's route the bike is, 0..1: the morning brightens with it.
    pub fn progress(&self) -> f32 { (self.bike_d / (ROWS as f32 * LOT)).clamp(0.0, 1.0) }
    pub fn end_d(&self) -> f32 { (ROWS - 2) as f32 * LOT }

    /// A new game: the week from Monday (a bot run may start on another day).
    pub fn start(&mut self, two: bool) {
        self.sess.reset(LIVES);
        self.run_best = 0;
        self.score2 = 0;
        self.two_player = two;
        self.week = 1 + (self.sess.level - 1) / 7;
        self.day = ((self.sess.level - 1) % 7) as usize;
        self.reset_route();
        self.begin_day();
    }

    /// A second rider climbs on the back of the tandem.
    pub fn join(&mut self) {
        if self.two_player { return; }
        self.two_player = true;
        self.cues.push(Cue::Join);
        self.banner = Some(("TANDEM! 2P THROWS RIGHT".into(), 2.5));
        blip::bot::add("join", 1.0);
    }

    fn load_day(&mut self) {
        let day = self.today();
        self.hazards = day.hazards.iter().map(|&(row, side, kind)| Hazard::new(kind, side, row as usize))
            .chain(day.traffic.iter().map(|&row| Hazard::traffic(row as usize)))
            .chain(STOPS.iter().map(|&(row, side)| Hazard::new(Hz::Bus, side, row)))
            .collect();
        // no two vehicles start on the same spot: a later one waits further up the street
        for i in 0..self.hazards.len() {
            for _ in 0..8 {
                let a = self.hazards[i];
                if !matches!(a.kind, Hz::Car | Hz::Bus | Hz::Van) { break; }
                let (aw, al) = a.size_driving();
                let clash = self.hazards[..i].iter().any(|b| matches!(b.kind, Hz::Car | Hz::Bus | Hz::Van) && b.seg == a.seg && {
                    let (bw, bl) = b.size_driving();
                    (a.x - b.x).abs() < aw + bw + 4.0 && (a.d - b.d).abs() < al + bl + 4.0
                });
                if !clash { break; }
                self.hazards[i].d += 70.0;
            }
        }
        // tourists come in flocks: a guide with a flag, the rest strung out behind on their rented e-bikes
        let flocks: Vec<Hazard> = self.hazards.iter().filter(|h| h.kind == Hz::Tourist).copied().collect();
        self.hazards.retain(|h| h.kind != Hz::Tourist);
        for (f, lead) in flocks.into_iter().enumerate() {
            let back = if lead.side == Side::Right { -1.0 } else { 1.0 };
            self.hazards.push(Hazard { guide: true, ..lead });
            for i in 1..(4 + (f % 3)) {
                let lane = BIKE_LANE[lead.side.index()] + if i % 2 == 0 { 5.0 } else { -5.0 };
                self.hazards.push(Hazard { d: lead.d + back * 30.0 * i as f32, home: lane, x: lane, beat: i as f32 * 1.37, ..lead });
            }
        }
        self.papers.clear();
        self.pops.clear();
        self.bits.clear();
        self.bike_x = 240.0;
        self.bike_d = LOT * 1.2;
        self.steer_v = 0.0;
        self.speed = CRUISE;
        self.splash = 0.0;
        self.place = 0;
        self.place_t = 0.0;
        self.ride_t = 0.0;
        self.turned = false;
        self.cross_next = [[0.0; 2]; 2];
        self.cycle_next = [0.0; 2];
        self.heading = 0.0;
        self.cam_off = [0.0; 3];
        for h in self.houses.iter_mut().flatten() {
            h.done = false;
            h.sampled = false;
            h.judged = false;
        }
    }

    /// The day card: the day, its weather and the round.
    pub fn begin_day(&mut self) {
        self.load_day();
        let subs = self.subscribers() as i32;
        self.stock = if self.weather() == Weather::Bundles { subs + 3 } else { subs + 8 }.min(MAX_PAPERS);
        self.tally = Tally::default();
        self.state = State::Card;
        self.clock = 0.0;
        self.timer.start(CARD_SECS);
        self.cues.push(Cue::DayStart);
        blip::bot::set("day", (self.day + 1) as f64);
    }

    /// Player `p`'s points.
    pub fn score(&mut self, p: usize, pts: i32) {
        if p == 1 { self.score2 += pts; } else { self.sess.add_score(pts); }
    }

    pub fn pop(&mut self, x: f32, d: f32, text: &str, color: [u8; 3]) {
        self.pops.push(Pop { x, d, text: text.to_string(), color, t: 0.0 });
    }

    fn burst(&mut self, x: f32, d: f32, n: usize, speed: f32, color: [u8; 3]) {
        for i in 0..n {
            let a = (i as f32 + rand_range_f32(0.0, 0.7)) / n as f32 * std::f32::consts::TAU;
            let v = speed * rand_range_f32(0.4, 1.0);
            self.bits.push(Bit { x, d, vx: a.cos() * v, vd: a.sin() * v, t: 0.0, life: rand_range_f32(0.35, 0.7), color,
                size: rand_range_f32(1.5, 3.0) });
        }
    }

    // ---- play ------------------------------------------------------------

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
        let want = if stop.is_some() { 0.0 } else if pedal > 0.0 { self.top_speed() } else if pedal < 0.0 { SPEED_MIN } else { CRUISE * self.pace().min(1.2) };
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
    fn open_at(x: f32, d: f32) -> bool {
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
    fn ride_junction(&mut self, steer: f32, pedal: f32, dt: f32) {
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
    fn onto_new_street(&mut self) {
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
    fn red_ahead(&self) -> Option<f32> {
        JUNCTIONS.iter().map(|&r| (r, Self::stop_d(r)))
            .find(|&(r, d)| d >= self.bike_d - 0.5 && d - self.bike_d < 160.0 && self.light(r).0 != Light::Green)
            .map(|(_, d)| d)
    }

    /// Where the bike can ride now: kerb to kerb, or along Nyhavn's quay.
    pub fn ride_limits(&self) -> [f32; 2] {
        if self.state == State::Canal { [QUAY[0] + BIKE_HW + 2.0, QUAY[1] - BIKE_HW - 4.0] } else { RIDE }
    }

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

    pub fn update_play(&mut self, dt: f32) {
        self.clock += dt;
        self.ride_t += dt;
        self.cross_traffic();
        self.cycle_traffic();
        let ease = (-5.0 * dt).exp();
        for o in &mut self.cam_off { *o *= ease; }
        for cd in &mut self.throw_cd { *cd -= dt; }
        self.safe -= dt;
        self.splash -= dt;
        self.update_papers(dt);
        self.update_hazards(dt);
        self.judge_houses();
        self.update_effects(dt);
        let row = row_at(self.bike_d).unwrap_or(0);
        let here = PLACES.iter().rposition(|(r, _)| *r <= row).unwrap_or(0);
        if here != self.place {
            self.place = here;
            self.place_t = 0.0;
            if matches!(PLACES[here].1, "MARMORKIRKEN" | "RUNDETAARN") { self.cues.push(Cue::Church); }
        }
        // an oncoming cyclist passing close rings
        let (bx, bd) = (self.bike_x, self.bike_d);
        if self.hazards.iter().any(|h| h.kind == Hz::Cyclist && h.side == Side::Left && h.phase == GO
            && (h.x - bx).abs() < 26.0 && (h.d - bd - 60.0).abs() < 2.0) { self.cues.push(Cue::Ting); }
        self.place_t += dt;
        if self.state != State::Play { return; }
        if self.on_bridge() && (self.bike_x - BIKE_HW < RAIL[0] || self.bike_x + BIKE_HW > RAIL[1]) {
            self.crash("THE RAILING");
            return;
        }
        if (self.bike_x - 240.0).abs() < STATUE_HALF + BIKE_HW - CRASH_INSET.0 && (self.bike_d - statue_d()).abs() < STATUE_HALF + BIKE_HL - CRASH_INSET.1 {
            self.crash("FREDERIK V");
            return;
        }
        if self.subscribers() == 0 {
            self.end_game();
            return;
        }
        if self.bike_d >= self.end_d() { self.end_day(); }
    }

    /// Nothing drives through anything else: whatever would run into something in its way within a moment waits for
    /// it (a car behind a cyclist, a tourist that would swerve into a car), as traffic does.
    fn give_way(&mut self) {
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
    fn queue_cyclists(&mut self) {
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
    fn cycle_traffic(&mut self) {
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
    fn cross_traffic(&mut self) {
        for (j, &row) in JUNCTIONS.iter().enumerate() {
            let near = (door_d(row) - self.bike_d).abs() < 700.0;
            let (_, t) = self.light(row);
            let _ = t;
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

    /// Is the bike on a bridge, nose or tail?
    pub fn on_bridge(&self) -> bool {
        [self.bike_d - BIKE_HL, self.bike_d + BIKE_HL].iter()
            .any(|&d| row_at(d).is_some_and(|r| self.houses[0][r].lot == Lot::Bridge))
    }

    fn update_papers(&mut self, dt: f32) {
        let mut ended = Vec::new();
        for (i, p) in self.papers.iter_mut().enumerate() {
            if let Some((t, _)) = &mut p.landed { *t += dt; continue; }
            p.t += dt;
            if p.t >= p.f.t { ended.push(i); }
        }
        for i in ended { let f = self.papers[i].f; self.land(i, f); }
        self.papers.retain(|p| p.landed.is_none_or(|(t, _)| t < 2.5));
    }

    fn land(&mut self, i: usize, f: Flight) {
        let p = self.papers[i];
        let s = p.side.index();
        let lot = row_at(f.d).map(|r| self.houses[s][r].lot);
        let (what, row) = outcome(p.side, f, lot);
        self.papers[i].landed = Some((0.0, what));
        let gold = [255, 214, 90];
        match (what, row) {
            (Outcome::Letterbox | Outcome::Doorstep, Some(r)) if self.houses[s][r].sub && !self.houses[s][r].done => {
                let letterbox = what == Outcome::Letterbox;
                let pts = self.worth(if letterbox { LETTERBOX_PTS } else { DOORSTEP_PTS });
                self.score(p.by, pts);
                let h = &mut self.houses[s][r];
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
                if letterbox { self.papers[i].landed = Some((9.0, what)); }
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
        // A dog after the bike would rather have the paper.
        let (px, pd) = (f.x, f.d);
        if let Some(dog) = self.hazards.iter_mut().find(|h| h.kind == Hz::Dog && (h.phase == TELL || h.phase == GO)
            && (h.x - px).abs() < 90.0 && (h.d - pd).abs() < 90.0)
        {
            dog.phase = AWAY;
            dog.t = 0.0;
            dog.fetch = Some((px, pd));
            self.cues.push(Cue::Bark);
            blip::bot::add("fetches", 1.0);
        }
    }

    /// Houses the bike has left behind: a subscriber with no paper loses a heart, and with none left, the round.
    fn judge_houses(&mut self) {
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

    fn update_hazards(&mut self, dt: f32) {
        let (bx, bd, speed) = (self.bike_x, self.bike_d, self.speed);
        let mut cues = Vec::new();
        let mut hits = Vec::new();
        for h in &mut self.hazards { h.held = false; }
        self.queue_cyclists();
        self.give_way();
        let seg = self.seg();
        // where the riders are, so a dog waits rather than run through one
        let riders: Vec<(f32, f32, u8)> = self.hazards.iter().filter(|h| !h.gone && matches!(h.kind, Hz::Cyclist | Hz::Tourist | Hz::Blader))
            .map(|h| (h.x, h.d, h.seg)).collect();
        let rider_at = |x: f32, d: f32, s: u8| riders.iter().any(|&(rx, rd, rs)| rs == s && (rx - x).abs() < 18.0 && (rd - d).abs() < 24.0);
        // how close the nearest rider is (a dog already near one may only move away from it)
        let rider_gap = |x: f32, d: f32, s: u8| riders.iter().filter(|r| r.2 == s).map(|&(rx, rd, _)| ((rx - x) / 18.0).abs().max(((rd - d) / 24.0).abs())).fold(f32::MAX, f32::min);
        let dog_may = |x: f32, d: f32, nx: f32, nd: f32, s: u8| !rider_at(nx, nd, s) || rider_at(x, d, s) && rider_gap(nx, nd, s) > rider_gap(x, d, s);
        // vans parked on the cycle track, which riders swing out around
        let vans: Vec<(f32, f32, u8)> = self.hazards.iter().filter(|h| !h.gone && h.kind == Hz::Van && h.solid())
            .map(|h| (h.x, h.d, h.seg)).collect();
        // every vehicle's box, so a tourist's swerve never takes it into one
        let vehicles: Vec<(f32, f32, f32, f32, u8)> = self.hazards.iter()
            .filter(|h| !h.gone && h.solid() && matches!(h.kind, Hz::Car | Hz::Bus | Hz::Van | Hz::CrossCar))
            .map(|h| { let (w, l) = h.size(); (h.x, h.d, w, l, h.seg) }).collect();
        let vehicle_at = |x: f32, d: f32, s: u8| vehicles.iter().any(|&(vx, vd, w, l, vs)| vs == s && (vx - x).abs() < w + 8.0 && (vd - d).abs() < l + 15.0);
        for (i, h) in self.hazards.iter_mut().enumerate() {
            if h.gone || h.seg > seg { continue; }
            h.t += dt;
            let ahead = h.d - bd;
            // A cyclist in the bike's line rings its bell about BELL_SECS before they would meet: a tell you hear.
            if h.kind == Hz::Cyclist && h.phase == GO && !h.rang && h.seg == seg && (h.x - bx).abs() < 20.0 + BIKE_HW {
                let closing = speed - h.velocity(bx, bd).1;
                if ahead > 0.0 && closing > 20.0 && ahead / closing < BELL_SECS {
                    h.rang = true;
                    cues.push(Cue::Bell);
                    blip::bot::add("bells", 1.0);
                }
            }
            match h.kind {
                Hz::Car if h.oncoming => if h.phase == WAIT && ahead < 620.0 { h.phase = GO; },
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
                Hz::Blader | Hz::Cart | Hz::Walker | Hz::Cyclist | Hz::Tourist => if h.phase == WAIT && ahead < match h.kind {
                    Hz::Tourist => 480.0,
                    Hz::Blader => 560.0, Hz::Cart => 520.0, Hz::Cyclist if h.side == Side::Left => 600.0, Hz::Cyclist => 420.0,
                    _ => 380.0 } { h.phase = GO; h.t = 0.0; },
                Hz::Van => if h.side == Side::Right && h.phase == WAIT && ahead < 420.0 { h.phase = GO; },
                Hz::Bus => match h.phase {
                    WAIT if ahead < BUS_WAKE => { h.phase = TELL; h.t = 0.0; }
                    TELL if h.t > BUS_DWELL => { h.phase = AWAY; h.t = 0.0; }
                    _ => {}
                },
                Hz::Pigeons(_) => if h.phase == WAIT && ahead < 130.0 && (h.x - bx).abs() < 90.0 {
                    h.phase = GO; h.t = 0.0; cues.push(Cue::Coo);
                },
                _ => {}
            }
            // move
            let (vx, vd) = h.velocity(bx, bd);
            let dog_waits = h.kind == Hz::Dog && !dog_may(h.x, h.d, h.x + vx * 0.15, h.d + vd * 0.15, h.seg);
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
                h.x = if vehicle_at(x, h.d, h.seg) { h.x + (h.home - h.x).clamp(-60.0 * dt, 60.0 * dt) } else { x };
            }
            if h.kind == Hz::Tourist && h.held { h.t -= dt; }
            // Back to the lane (the bridge's is narrower), but not while held: a rider waiting beside a bus drifted into it.
            if matches!(h.kind, Hz::Cyclist | Hz::Tourist | Hz::Blader) && !h.held {
                let on_bridge = row_at(h.d).is_some_and(bridge_rows);
                let lane = if on_bridge { [RAIL[0] + 9.0, RAIL[1] - 9.0][h.side.index()] } else { h.home };
                // a van on the track ahead (or alongside): out into the road past it, 13 + 6 px clear and a margin
                let (_, vd) = h.velocity(bx, bd);
                let lane = vans.iter().find(|&&(vx, vd0, vs)| vs == h.seg && (vx - h.home).abs() < 10.0
                    && (-50.0..90.0).contains(&((vd0 - h.d) * vd.signum())))
                    .map_or(lane, |&(vx, _, _)| vx - h.side.sign() * 24.0);
                if h.kind != Hz::Tourist || h.guide { h.x += (lane - h.x).clamp(-60.0 * dt, 60.0 * dt); }
            }
            if h.kind == Hz::Dog && (h.phase == HOME || h.phase == AWAY) {
                // a fetch first, then home through the gate
                let (tx, td) = h.fetch.filter(|_| h.phase == AWAY).unwrap_or((h.home, h.d));
                let k = ((160.0 * dt) / ((tx - h.x).hypot(td - h.d)).max(1.0)).min(1.0);
                let (nx, nd) = (h.x + (tx - h.x) * k * 9.0, h.d + (td - h.d) * k * 9.0);
                if dog_may(h.x, h.d, nx, nd, h.seg) {
                    h.x += (tx - h.x) * k;
                    h.d += (td - h.d) * k;
                }
                if h.phase == AWAY && (tx - h.x).abs() < 3.0 && (td - h.d).abs() < 3.0 && h.t > 0.8 { h.phase = HOME; h.fetch = None; }
                if h.phase == HOME && (h.x - h.home).abs() < 3.0 { h.phase = PAUSE; }
            }
            if h.d < bd - 600.0 { h.gone = true; }
            // the bike, on the same street
            if h.seg != seg { continue; }
            let (hw, hl) = h.size();
            let near_x = (h.x - bx).abs();
            let near_d = (h.d - bd).abs();
            match h.kind {
                Hz::Puddle(_) if near_x < 20.0 + BIKE_HW && near_d < 14.0 + BIKE_HL && !h.touched => hits.push((i, false)),
                Hz::Sprinkler if h.phase == GO && !h.touched && near_d < SPRAY_REACH + BIKE_HL
                    && (bx - h.x).abs() < (CURB[0] - FENCE[0]) + SPRAY_REACH => {
                    let reach = if h.side == Side::Left { bx - BIKE_HW < CURB[0] + SPRAY_REACH } else { bx + BIKE_HW > CURB[1] - SPRAY_REACH };
                    if reach { hits.push((i, false)); }
                }
                Hz::Cart if near_x < 14.0 + BIKE_HW && near_d < 14.0 + BIKE_HL => hits.push((i, false)),
                // passengers crossing the cycle track to and from the bus
                Hz::Bus if h.phase == TELL && (bx - BIKE_LANE[h.side.index()]).abs() < 13.0 + BIKE_HW && (bd - h.d).abs() < 12.0 + BIKE_HL
                    && self.safe <= 0.0 => {
                    hits.push((i, true));
                }
                _ if h.solid() && near_x < hw + BIKE_HW - CRASH_INSET.0 && near_d < hl + BIKE_HL - CRASH_INSET.1 => hits.push((i, true)),
                _ => {}
            }
        }
        self.cues.extend(cues);
        if self.state != State::Play { return; }
        for (i, crash) in hits {
            let h = &mut self.hazards[i];
            // riding into another rider gently is a bump, not a crash: the bike is knocked aside and held to their pace
            let rider = matches!(h.kind, Hz::Cyclist | Hz::Tourist | Hz::Blader);
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
                continue;
            }
            if crash {
                if self.safe > 0.0 { continue; }
                if blip::bot::active() {
                    eprintln!("CRASH {:?} oncoming={} phase={} h=({:.0},{:.0}) bike=({:.0},{:.0}) v={:.0}",
                        h.kind, h.oncoming, h.phase, h.x, h.d, self.bike_x, self.bike_d, self.speed);
                }
                let what = match h.kind {
                    Hz::Car if h.bus => "THE BUS", Hz::Bus if h.phase == TELL && (h.x - self.bike_x).abs() > 20.0 => "A PASSENGER", Hz::Car => "A CAR", Hz::Dog => "A DOG", Hz::Blader => "A SKATER", Hz::Van => "THE VAN",
                    Hz::Cyclist => "A CYCLIST", Hz::CrossCar => "A CROSSING CAR", Hz::Bus => "THE BUS", Hz::Tourist => "A TOURIST",
                    Hz::Stall => "A STALL", _ => "SOMEONE",
                };
                self.crash(what);
                return;
            }
            h.touched = true;
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

    fn update_effects(&mut self, dt: f32) {
        for p in &mut self.pops { p.t += dt; p.d += 30.0 * dt; }
        self.pops.retain(|p| p.t < 1.0);
        for b in &mut self.bits {
            b.t += dt;
            b.x += b.vx * dt;
            b.d += b.vd * dt;
            b.vx *= 1.0 - 3.0 * dt;
            b.vd *= 1.0 - 3.0 * dt;
        }
        self.bits.retain(|b| b.t < b.life);
        for h in self.houses.iter_mut().flatten() {
            h.flap += dt; h.wave += dt; h.sad += dt; h.cancelled += dt; h.joined += dt;
        }
        if let Some((_, t)) = &mut self.banner { *t -= dt; if *t <= 0.0 { self.banner = None; } }
    }

    fn crash(&mut self, what: &str) {
        self.state = State::Crash;
        self.clock = 0.0;
        self.crash_dir = if self.steer_v < 0.0 { -1.0 } else { 1.0 };
        self.cues.push(Cue::Crash);
        let (x, d) = (self.bike_x, self.bike_d);
        self.burst(x, d, 14, 90.0, [245, 240, 225]);
        self.pop(x, d + 36.0, &format!("OOF! {what}"), [255, 150, 120]);
        blip::bot::add("crashes", 1.0);
        blip::bot::add("hit_crash", 1.0);
        blip::bot::add(&format!("crash_{}", what.split(' ').last().unwrap_or("x").to_lowercase()), 1.0);
        self.timer.start(CRASH_SECS);
    }

    pub fn update_crash(&mut self, dt: f32) {
        self.clock += dt;
        self.ride_t += dt;
        self.update_hazards(dt);
        self.update_papers(dt);
        self.update_effects(dt);
        if !self.timer.tick(dt) { return; }
        if self.sess.lose_life() == blip::LifeResult::GameOver { return self.end_game(); }
        // Back on the bike: the street just ahead is cleared, and a moment's grace.
        let bd = self.bike_d;
        for h in &mut self.hazards {
            if h.solid() && h.d > bd - 160.0 && h.d < bd + 260.0 { h.gone = true; }
        }
        // Mid-road, unless that is Frederik V's pedestal: back there the bike met it again at once, and three
        // lives went in 3.3 s (Tuesday, seed 3).
        let at_statue = (bd - statue_d()).abs() < STATUE_HALF + BIKE_HL + 120.0;
        self.bike_x = if at_statue { 240.0 + STATUE_HALF + BIKE_HW + 12.0 } else { 240.0 };
        self.steer_v = 0.0;
        self.heading = 0.0;
        self.stuck = 0.0;
        self.speed = CRUISE * 0.8;
        self.safe = SAFE_SECS;
        self.state = State::Play;
    }

    pub fn end_game(&mut self) {
        self.state = State::Over;
        self.clock = 0.0;
        self.wait.start(blip::GAME_OVER_MIN_WAIT);
        blip::bot::set("subscribers", self.subscribers() as f64);
        // While homes remain the day can be ridden again; with none left the round is over for good.
        if self.subscribers() > 0 { self.cont.start(CONTINUE_SECS); } else { self.post_score(); }
    }

    /// Is a continue on offer?
    pub fn offering(&self) -> bool { self.subscribers() > 0 && self.cont.remaining() > 0.0 }

    /// The run is over: post its best (two riders share the tandem, and their game posts no best).
    pub fn post_score(&mut self) {
        if self.two_player { blip::bot::finish("over"); } else { blip::web::report_score(self.sess.score.max(self.run_best)); }
    }

    /// A continue taken: the same day again from its card, with fresh lives and the score afresh.
    pub fn continue_day(&mut self) {
        blip::bot::add("continues", 1.0);
        self.run_best = self.run_best.max(self.sess.score);
        self.sess.score = 0;
        self.score2 = 0;
        self.sess.lives = LIVES;
        self.begin_day();
    }

    fn end_day(&mut self) {
        // Anyone still waiting is judged now.
        self.bike_d += 400.0;
        self.judge_houses();
        self.bike_d -= 400.0;
        let delivered = self.tally.letterbox + self.tally.doorstep;
        if self.tally.misses == 0 && delivered > 0 {
            self.tally.perfect = PERFECT_PTS * (self.day as i32 + 1);
            self.sess.add_score(self.tally.perfect);
            blip::bot::add("perfect_days", 1.0);
        }
        // Samples win neighbours over for tomorrow.
        for h in self.houses.iter_mut().flatten() {
            if h.sampled && !h.sub && rand_range_f32(0.0, 1.0) < WIN_OVER {
                h.sub = true;
                h.hearts = 2;
                h.joined = -CARD_SECS - TALLY_SECS - CANAL_SECS;
                self.tally.won += 1;
            }
        }
        blip::bot::add("new_subscribers", self.tally.won as f64);
        blip::bot::add("days", 1.0);
        blip::bot::set("subscribers", self.subscribers() as f64);
        self.state = State::Tally;
        self.clock = 0.0;
        self.timer.start(TALLY_SECS);
        self.cues.push(if self.tally.perfect > 0 { Cue::Perfect } else { Cue::DayEnd });
        if self.subscribers() == 0 { self.end_game(); }
    }

    pub fn update_tally(&mut self, dt: f32, skip: bool) {
        self.clock += dt;
        self.update_effects(dt);
        if self.timer.tick(dt) || (skip && self.clock > 1.0) { self.start_canal(); }
    }

    // ---- the Canal Run ---------------------------------------------------

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

    fn next_day(&mut self) {
        self.sess.next_level();
        if self.day == 6 {
            // The week is done: every subscriber kept pays, and a fresh life for the next.
            let kept = self.subscribers() as i32;
            self.sess.add_score(kept * KEPT_PTS);
            self.sess.lives += 1;
            self.tally = Tally { won: kept, ..Tally::default() };
            self.state = State::Week;
            self.clock = 0.0;
            self.timer.start(TALLY_SECS);
            self.cues.push(Cue::Perfect);
            blip::bot::add("weeks", 1.0);
            return;
        }
        self.day += 1;
        self.begin_day();
    }

    pub fn update_week(&mut self, dt: f32, skip: bool) {
        self.clock += dt;
        if self.timer.tick(dt) || (skip && self.clock > 1.0) {
            self.week += 1;
            self.day = 0;
            self.begin_day();
        }
    }

    pub fn update_card(&mut self, dt: f32, skip: bool) {
        self.clock += dt;
        self.update_effects(dt);
        if self.timer.tick(dt) || (skip && self.clock > 0.8) {
            self.state = State::Play;
            self.clock = 0.0;
            self.cues.push(Cue::Bell);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playing() -> Game {
        blip::rand_seed(1);
        let mut g = Game::new();
        g.start(false);
        g.state = State::Play;
        g
    }

    /// The first subscriber's house on `side`.
    fn first_sub(g: &Game, side: Side) -> usize {
        (0..ROWS).find(|&r| g.houses[side.index()][r].sub).unwrap()
    }

    #[test]
    fn a_continue_rides_the_same_day_again_with_fresh_lives_and_keeps_the_runs_best() {
        let mut g = playing();
        g.day = 3;
        g.begin_day();
        g.sess.score = 500;
        g.sess.lives = 0;
        g.end_game();
        assert!(g.offering(), "homes remain, so a continue is offered");
        g.continue_day();
        assert!(g.state == State::Card, "the day starts again from its card");
        assert_eq!((g.day, g.sess.lives, g.sess.score, g.run_best), (3, LIVES, 0, 500));
    }

    #[test]
    fn a_close_throw_reaches_the_wall_and_a_far_one_lands_short() {
        let near = flight(200.0, 0.0, CRUISE, Side::Left, 0.0, false);
        assert!(near.wall && near.x == WALL[0]);
        let far = flight(300.0, 0.0, CRUISE, Side::Left, 0.0, false);
        assert!(!far.wall && (far.x - 150.0).abs() < 0.01);
        let right = flight(280.0, 0.0, CRUISE, Side::Right, 0.0, false);
        assert!(right.wall && right.x == WALL[1]);
    }

    #[test]
    fn a_paper_through_the_slot_scores_a_letterbox_and_one_beside_it_the_doorstep() {
        let lot = Some(Lot::House { drive: false });
        let at = |d| Flight { x: WALL[0], d, wall: true, slid: false, t: 0.3 };
        assert_eq!(outcome(Side::Left, at(door_d(3)), lot).0, Outcome::Letterbox);
        assert_eq!(outcome(Side::Left, at(door_d(3) + 15.0), lot).0, Outcome::Doorstep);
        assert_eq!(outcome(Side::Left, at(door_d(3) + 40.0), lot).0, Outcome::Garden);
        let slid = Flight { slid: true, ..at(door_d(3)) };
        assert_eq!(outcome(Side::Left, slid, lot).0, Outcome::Doorstep, "a paper that slid there cannot go through the slot");
        let sample = Flight { x: BOX_X[1], d: door_d(5) + BOX_D, wall: false, slid: false, t: FLIGHT };
        assert_eq!(outcome(Side::Right, sample, lot).0, Outcome::Sample);
    }

    #[test]
    fn every_target_can_be_reached_from_the_street() {
        // From somewhere between the kerbs a throw reaches the slot, the doorstep and the sample box, both sides,
        // in every weather.
        for (wind, rain) in [(0.0, false), (-60.0, false), (0.0, true)] {
            for side in Side::BOTH {
                let mut seen = [false; 3];
                let mut x = RIDE[0];
                while x <= RIDE[1] {
                    let f = flight(x, door_d(4) - 200.0, CRUISE, side, wind, rain);
                    let f = Flight { d: door_d(4) + if f.wall { 0.0 } else { BOX_D }, ..f };
                    match outcome(side, f, Some(Lot::House { drive: false })).0 {
                        Outcome::Letterbox => seen[0] = true,
                        Outcome::Sample => seen[2] = true,
                        _ => {}
                    }
                    let step = Flight { d: door_d(4), ..f };
                    if outcome(side, step, Some(Lot::House { drive: false })).0 == Outcome::Doorstep { seen[1] = true; }
                    x += 1.0;
                }
                assert_eq!(seen, [true, true, true], "{side:?} wind {wind} rain {rain}: {seen:?}");
            }
        }
    }

    #[test]
    fn delivering_scores_once_and_a_letterbox_mends_a_heart() {
        let mut g = playing();
        let r = first_sub(&g, Side::Left);
        g.houses[0][r].hearts = 1;
        let f = Flight { x: WALL[0], d: door_d(r), wall: true, slid: false, t: 0.3 };
        for _ in 0..2 {
            g.papers.push(Paper { x: 200.0, d: 0.0, vx: -THROW_V, vd: 0.0, t: 0.0, side: Side::Left, by: 0, f, landed: None });
            let i = g.papers.len() - 1;
            g.land(i, f);
        }
        assert_eq!(g.sess.score, LETTERBOX_PTS);
        assert_eq!(g.houses[0][r].hearts, 2);
    }

    #[test]
    fn two_missed_mornings_lose_a_subscriber() {
        let mut g = playing();
        let r = first_sub(&g, Side::Right);
        let before = g.subscribers();
        for _ in 0..2 {
            g.houses[1][r].judged = false;
            g.houses[1][r].done = false;
            g.bike_d = door_d(r) + 201.0;
            g.judge_houses();
        }
        assert!(!g.houses[1][r].sub);
        assert_eq!(g.subscribers(), before - 1);
    }

    #[test]
    fn a_sample_can_win_a_new_subscriber_by_the_next_day() {
        let mut g = playing();
        let r = (0..ROWS).find(|&r| g.houses[0][r].lot.is_house() && !g.houses[0][r].sub).unwrap();
        let mut won = 0;
        for seed in 0..20 {
            blip::rand_seed(seed);
            g.houses[0][r].sub = false;
            g.houses[0][r].sampled = true;
            g.tally = Tally::default();
            g.end_day();
            won += g.tally.won;
            g.state = State::Play;
        }
        assert!(won > 5 && won < 20, "{won}/20 samples won a subscriber");
    }

    #[test]
    fn the_canal_pays_more_for_a_longer_throw() {
        assert_eq!(Game::target_points(QUAY_MID + 95.0), 50);
        assert_eq!(Game::target_points(QUAY_MID - 190.0), 500);
        assert!(Game::target_points(40.0) > Game::target_points(90.0));
    }

    #[test]
    fn every_nyhavn_target_can_be_reached_from_the_quay() {
        let mut g = playing();
        g.start_canal();
        let lim = g.ride_limits();
        for t in &g.targets {
            let x = t.x - t.side().sign() * THROW_V * FLIGHT;
            assert!(x >= lim[0] - 0.5 && x <= lim[1] + 0.5, "{:?} at {} needs the bike at {x}", t.mark, t.x);
        }
    }

    #[test]
    fn steering_left_in_the_junction_turns_the_bike_into_the_new_street() {
        let mut g = playing();
        g.bike_d = JD - MOUTH_HALF - 60.0;
        g.bike_x = 200.0;
        for _ in 0..(60 * 20) {
            if g.state != State::Play { g.state = State::Play; }
            let steer = if g.in_junction() && g.bike_d > JD - 30.0 && g.heading < 1.5 { -1.0 } else { 0.0 };
            g.ride(steer, 0.0, 1.0 / 60.0);
            g.update_play(1.0 / 60.0);
            g.safe = 1.0;
            if g.turned { break; }
        }
        assert!(g.turned, "never round the corner: x {} d {} heading {}", g.bike_x, g.bike_d, g.heading);
        assert!(g.bike_d >= NEW_STREET_ROW as f32 * LOT - 30.0 && (RIDE[0]..=RIDE[1]).contains(&g.bike_x), "on the new street at {} {}", g.bike_x, g.bike_d);
    }

    #[test]
    fn a_crash_at_the_statue_does_not_put_the_bike_back_on_it() {
        let mut g = playing();
        g.bike_d = statue_d() - STATUE_HALF;
        g.bike_x = 240.0;
        g.update_play(1.0 / 60.0);
        assert_eq!(g.state, State::Crash, "rode into the pedestal");
        while g.state == State::Crash { g.update_crash(1.0 / 60.0); }
        for _ in 0..60 {
            g.ride(0.0, 0.0, 1.0 / 60.0);
            g.update_play(1.0 / 60.0);
            assert_eq!(g.state, State::Play, "crashed again straight after getting back on, at {} {}", g.bike_x, g.bike_d);
        }
    }

    #[test]
    fn a_rider_who_never_steers_still_gets_round_the_corner() {
        for pedal in [0.0, 1.0] {
            let mut g = playing();
            g.bike_d = JD - MOUTH_HALF - 60.0;
            g.bike_x = 200.0;
            for _ in 0..(60 * 30) {
                if g.state != State::Play { g.state = State::Play; }
                g.ride(0.0, pedal, 1.0 / 60.0);
                g.update_play(1.0 / 60.0);
                g.safe = 1.0;
                if g.turned { break; }
            }
            assert!(g.turned, "pedal {pedal}: stalled at x {} d {} heading {}", g.bike_x, g.bike_d, g.heading);
        }
    }

    #[test]
    fn buses_and_cars_never_drive_into_each_other() {
        for day in 0..7 {
            let mut g = playing();
            g.day = day;
            g.begin_day();
            g.state = State::Play;
            for frame in 0..(60 * 50) {
                g.state = State::Play;
                let steer = if g.in_junction() && g.bike_d > JD - 30.0 && g.heading < 1.5 { -1.0 } else { 0.0 };
                g.ride(steer, 0.0, 1.0 / 60.0);
                g.safe = 5.0;
                g.update_play(1.0 / 60.0);
                if g.state != State::Play { break; }
                let cars: Vec<&Hazard> = g.hazards.iter().filter(|h| !h.gone && h.solid() && matches!(h.kind, Hz::Car | Hz::Bus | Hz::Van | Hz::CrossCar)).collect();
                for (i, a) in cars.iter().enumerate() {
                    for b in &cars[i + 1..] {
                        if a.seg != b.seg { continue; }
                        let ((aw, al), (bw, bl)) = (a.size(), b.size());
                        let over = (a.x - b.x).abs() < aw + bw - 3.0 && (a.d - b.d).abs() < al + bl - 3.0;
                        assert!(!over, "day {day} frame {frame}: {:?} p{} held {} at ({:.0},{:.0}) runs into {:?} p{} held {} at ({:.0},{:.0})",
                            a.kind, a.phase, a.held, a.x, a.d, b.kind, b.phase, b.held, b.x, b.d);
                    }
                }
            }
        }
    }

    #[test]
    fn riders_never_ride_through_cars_dogs_or_buses() {
        let mut found = Vec::new();
        for day in 0..7 {
            let mut g = playing();
            g.day = day;
            g.begin_day();
            g.state = State::Play;
            for frame in 0..(60 * 100) {
                g.state = State::Play;
                let steer = if g.in_junction() && g.bike_d > JD - 30.0 && g.heading < 1.5 { -1.0 } else { 0.0 };
                g.ride(steer, 0.0, 1.0 / 60.0);
                g.safe = 5.0;
                g.update_play(1.0 / 60.0);
                if g.state != State::Play { break; }
                let riders: Vec<&Hazard> = g.hazards.iter().filter(|h| !h.gone && matches!(h.kind, Hz::Cyclist | Hz::Tourist | Hz::Blader)).collect();
                let others: Vec<&Hazard> = g.hazards.iter().filter(|h| !h.gone && h.solid() && matches!(h.kind, Hz::Car | Hz::Bus | Hz::Van | Hz::CrossCar | Hz::Dog)).collect();
                for a in &riders {
                    for b in &others {
                        if a.seg != b.seg { continue; }
                        let ((aw, al), (bw, bl)) = (a.size(), b.size());
                        if (a.x - b.x).abs() < aw + bw - 3.0 && (a.d - b.d).abs() < al + bl - 3.0 && found.len() < 12 {
                            found.push(format!("day {day} frame {frame}: {:?} p{} held {} at ({:.0},{:.0}) through {:?} p{} at ({:.0},{:.0})",
                                a.kind, a.phase, a.held, a.x, a.d, b.kind, b.phase, b.x, b.d));
                        }
                    }
                }
            }
        }
        assert!(found.is_empty(), "{}", found.join("\n"));
    }

    #[test]
    fn at_the_crossing_nothing_drives_into_anything() {
        // the bike waits short of the junction while the lights go round several times
        let mut g = playing();
        g.bike_d = Game::stop_d(JUNCTIONS[0]) - 60.0;
        for frame in 0..(60 * 60) {
            g.state = State::Play;
            g.speed = 0.0;
            g.safe = 5.0;
            g.update_play(1.0 / 60.0);
            let cars: Vec<&Hazard> = g.hazards.iter().filter(|h| !h.gone && h.solid() && matches!(h.kind, Hz::Car | Hz::Bus | Hz::Van | Hz::CrossCar)).collect();
            for (i, a) in cars.iter().enumerate() {
                for b in &cars[i + 1..] {
                    let ((aw, al), (bw, bl)) = (a.size(), b.size());
                    let over = (a.x - b.x).abs() < aw + bw - 2.0 && (a.d - b.d).abs() < al + bl - 2.0;
                    assert!(!over, "frame {frame}: {:?} at ({:.0},{:.0}) in {:?} at ({:.0},{:.0})", a.kind, a.x, a.d, b.kind, b.x, b.d);
                }
            }
        }
        assert!(g.hazards.iter().any(|h| h.kind == Hz::CrossCar), "no side-street traffic came");
    }

    #[test]
    fn going_straight_on_stops_at_the_roadworks() {
        let mut g = playing();
        g.bike_d = JD - MOUTH_HALF - 60.0;
        g.bike_x = 280.0;
        for _ in 0..(60 * 10) {
            g.state = State::Play;
            g.ride(0.0, 1.0, 1.0 / 60.0);
            g.safe = 1.0;
        }
        assert!(!g.turned && g.bike_d < WORKS_D, "rode through the barrier to {}", g.bike_d);
    }

    #[test]
    fn the_two_streets_meet_where_they_should() {
        let (x, d) = new_street_to_world(240.0, JD + 110.0);
        assert!((x - 130.0).abs() < 0.01 && (d - JD).abs() < 0.01);
        let (bx, bd) = world_to_new_street(x, d);
        assert!((bx - 240.0).abs() < 0.01 && (bd - JD - 110.0).abs() < 0.01);
    }

    #[test]
    fn a_week_of_days_comes_round_to_monday() {
        let mut g = playing();
        for day in 0..7 {
            assert_eq!(g.day, day);
            g.end_day();
            g.start_canal();
            g.timer.start(0.0001);
            g.update_canal(0.01);
        }
        assert_eq!(g.state, State::Week);
        g.update_week(5.0, false);
        assert_eq!((g.week, g.day, g.state), (2, 0, State::Card));
    }
}
