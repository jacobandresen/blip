//! The rules of the round: the bike, the papers in flight, the houses' goodwill, the hazards, the week and the
//! Canal Run. Drawing is in draw.rs; nothing here draws or plays a sound, it only queues them (`Game::cues`).

use crate::levels::*;
use blip::{rand_range_f32, Session, Timer};

mod hazard;
mod traffic;
mod canal;
mod ride;
mod papers;
pub use hazard::*;

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
            state: State::Title, sess: Session::new(LIVES), cont: blip::Continue::default(), score2: 0, two_player: false, week: 1, day: 0,
            houses: [Vec::new(), Vec::new()], hazards: Vec::new(), papers: Vec::new(), stock: 0,
            bike_x: 240.0, bike_d: LOT * 1.2, steer_v: 0.0, speed: CRUISE, safe: 0.0, throw_cd: [0.0; 2],
            timer: Timer::default(), wait: Timer::default(), tally: Tally::default(), pops: Vec::new(), bits: Vec::new(),
            cues: Vec::new(), clock: 0.0, targets: Vec::new(), crash_dir: 1.0, banner: None, splash: 0.0,
            place: 0, place_t: 0.0, ride_t: 0.0, turned: false, cross_next: [[0.0; 2]; 2], cycle_next: [0.0; 2], heading: 0.0,
            stuck: 0.0, cam_off: [0.0; 3],
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
        if (self.bike_x - 240.0).abs() < STATUE_HALF + BIKE_HW - CRASH_INSET.0
            && (self.bike_d - statue_d()).abs() < STATUE_HALF + BIKE_HL - CRASH_INSET.1 {
            self.crash("FREDERIK V");
            return;
        }
        if self.subscribers() == 0 {
            self.end_game();
            return;
        }
        if self.bike_d >= self.end_d() { self.end_day(); }
    }

    /// Is the bike on a bridge, nose or tail?
    pub fn on_bridge(&self) -> bool {
        [self.bike_d - BIKE_HL, self.bike_d + BIKE_HL].iter()
            .any(|&d| row_at(d).is_some_and(|r| self.houses[0][r].lot == Lot::Bridge))
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
        blip::bot::add(&format!("crash_{}", what.split(' ').next_back().unwrap_or("x").to_lowercase()), 1.0);
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
        if self.two_player { blip::bot::finish("over"); } else { self.sess.report(); }
    }

    /// A continue taken: the same day again from its card, with fresh lives and the score afresh.
    pub fn continue_day(&mut self) {
        self.sess.play_on(LIVES);
        self.score2 = 0;
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
mod tests;
