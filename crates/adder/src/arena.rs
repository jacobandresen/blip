//! The pit: continuous snakes, the holes in them, and the rules that end a run.
//!
//! Movement is analogue — a heading that turns at a rate, the body the path the
//! head has taken — because the original's freedom of movement has no grid in
//! it. The one place a body lets a head through is a hole in it, and going
//! through a hole is what scores.

use std::collections::VecDeque;

use blip::{rand_int, rand_range_f32, segment_circle_overlap, BlipColor, Fx};

// ---- the pit -------------------------------------------------------------
pub const PIT_X: f32 = 10.0;
pub const PIT_Y: f32 = 38.0; // under the HUD
pub const PIT_W: f32 = 480.0 - PIT_X * 2.0;
pub const PIT_H: f32 = 540.0 - PIT_Y - PIT_X;
pub const BODY_R: f32 = 6.0; // a snake's half-thickness
pub const SAMPLE: f32 = 4.0; // the path is kept this finely
pub const START_S: usize = 9; // samples a fresh snake starts with
pub const GROW_S: usize = 2; // samples one egg adds
pub const EGG_R: f32 = 7.0;

// ---- tuning --------------------------------------------------------------
pub const GLIDE: f32 = 100.0; // px/s
pub const STRIKE: f32 = 1.75; // the throttle
pub const TURN: f32 = 3.2; // rad/s
pub const STIFF: f32 = 0.5; // the strike's neck
pub const LIVES: i32 = 3;
pub const MAX_SNAKES: usize = 6;
pub const SEATS: usize = 6;
pub const EGG_POINTS: i32 = 10;
pub const HOLE_POINTS: i32 = 50;
pub const KILL_POINTS: i32 = 100;
pub const ROUND_POINTS: i32 = 250;
pub const EGGS_IN_PIT: usize = 3;
pub const HOLE_S: usize = 5; // samples of body a hole takes out
pub const HOLE_TTL: f32 = 8.0;
pub const HOLE_WAIT: f32 = 2.5;
pub const HOLE_GRACE: f32 = 0.35; // a spent hole stays a way through this long
pub const SHED_EVERY: i32 = 4;
pub const SHED_S: usize = 5; // 20px: the stretch of tail a shed takes with it
/// A tail nearer the head than this is left alone. It has to be short enough
/// that a snake which has just eaten its fourth egg can shed at all: at that
/// point its tail is only 44px behind its nose.
pub const SHED_CLEAR: f32 = 32.0;
pub const RESPAWN: f32 = 1.6;
pub const SPEED_UP: f32 = 1.08; // a round on
pub const CLEAR_OF: f32 = 64.0; // the room a respawn looks for
const SELF_SKIP: usize = 3; // leading samples of its own a head ignores

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Pt { pub x: f32, pub y: f32 }

impl Pt {
    pub fn new(x: f32, y: f32) -> Self { Self { x, y } }
    pub fn dist(self, o: Pt) -> f32 { ((self.x - o.x).powi(2) + (self.y - o.y).powi(2)).sqrt() }
}

/// A passable span of a body, `at` samples in from the head. `spent` means it
/// has already paid: it is still a way through until it closes.
#[derive(Copy, Clone, Debug)]
pub struct Hole { pub at: usize, pub len: usize, pub spent: bool }

/// What is driving one snake this frame: -1 is full left lock, 1 full right.
#[derive(Copy, Clone, Debug, Default)]
pub struct Steer { pub turn: f32, pub boost: bool }

/// What the head's next step meets first.
#[derive(Copy, Clone, Debug, PartialEq)]
enum Hit { None, Wall, Skin, Body(usize, usize) }

/// Something worth a sound or a word. The pit keeps no audio of its own: the
/// app drains these each frame and plays them.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Event { Eat, Through, Shed, Kill, Die, RoundWin }

pub struct Snake {
    /// Head first, one sample every `SAMPLE` px travelled.
    pub path: VecDeque<Pt>,
    pub head: Pt,
    pub heading: f32,
    /// Samples of body the snake should be holding, growth included.
    pub len_s: usize,
    pub seat: usize,
    /// False for a seat a CPU is holding; player two turns it true by joining.
    pub human: bool,
    pub score: i32,
    pub lives: i32,
    pub alive: bool,
    pub respawn: f32,
    /// Eggs swallowed, which is the shed meter.
    pub eaten: i32,
    pub hole: Option<Hole>,
    pub hole_ttl: f32,
    pub hole_wait: f32,
    pub boost: bool,
    travelling: f32,
}

impl Snake {
    fn new(seat: usize, human: bool, head: Pt, heading: f32) -> Self {
        let mut s = Self {
            path: VecDeque::new(),
            head,
            heading,
            len_s: START_S,
            seat,
            human,
            score: 0,
            lives: LIVES,
            alive: true,
            respawn: 0.0,
            eaten: 0,
            hole: None,
            hole_ttl: 0.0,
            hole_wait: 0.0,
            boost: false,
            travelling: 0.0,
        };
        s.lay_path();
        s
    }

    /// A straight body behind the head, so a fresh snake has a tail at all.
    fn lay_path(&mut self) {
        self.path.clear();
        for i in 0..self.len_s {
            let d = i as f32 * SAMPLE;
            self.path.push_back(Pt::new(
                self.head.x - self.heading.cos() * d,
                self.head.y - self.heading.sin() * d,
            ));
        }
    }

    pub fn name(&self) -> &'static str { seat_name(self.seat) }
    pub fn colour(&self) -> BlipColor { seat_colour(self.seat) }
    pub fn speed(&self, base: f32) -> f32 { if self.boost { base * STRIKE } else { base } }
    pub fn turn_rate(&self) -> f32 { if self.boost { TURN * STIFF } else { TURN } }

    /// Whether sample `i` sits in the hole: the only part of a body that a head
    /// may cross without dying.
    pub fn in_hole(&self, i: usize) -> bool {
        self.hole.map_or(false, |h| i >= h.at && i < h.at + h.len)
    }

    pub fn hole_centre(&self) -> Option<Pt> {
        let h = self.hole?;
        self.path.get(h.at + h.len / 2).copied()
    }
}

/// Seats are one family: player one is the olive viper the cabinet already
/// knows, and every later seat gets a colour of its own so a busy pit stays
/// readable.
pub fn seat_colour(seat: usize) -> BlipColor {
    match seat {
        0 => BlipColor { r: 0.62, g: 0.73, b: 0.28, a: 1.0 },
        1 => BlipColor { r: 0.45, g: 0.85, b: 1.0, a: 1.0 },
        2 => BlipColor { r: 0.78, g: 0.55, b: 1.0, a: 1.0 },
        3 => BlipColor { r: 0.45, g: 0.95, b: 0.55, a: 1.0 },
        4 => BlipColor { r: 0.95, g: 0.45, b: 0.35, a: 1.0 },
        _ => BlipColor { r: 0.78, g: 0.78, b: 0.84, a: 1.0 },
    }
}

pub fn seat_name(seat: usize) -> &'static str {
    match seat { 0 => "P1", 1 => "P2", 2 => "C1", 3 => "C2", 4 => "C3", _ => "C4" }
}

pub struct World {
    pub snakes: Vec<Snake>,
    pub eggs: Vec<Pt>,
    /// Frozen tail segments: wall for the rest of the round.
    pub skin: Vec<Pt>,
    pub round: i32,
    pub over: bool,
    /// Human seats the game was started with; player two can add to them.
    pub humans: usize,
    pub fx: Fx,
    pub banner_t: f32,
    pub events: Vec<Event>,
}

impl World {
    /// A fresh pit: the human seats, one CPU viper to race, and eggs down.
    pub fn new(humans: usize) -> Self {
        let mut w = Self {
            snakes: Vec::new(),
            eggs: Vec::new(),
            skin: Vec::new(),
            round: 1,
            over: false,
            humans,
            fx: Fx::new(),
            banner_t: 9.0,
            events: Vec::new(),
        };
        for seat in 0..humans.max(1) { w.join(seat, true); }
        w.join(w.snakes.len(), false);
        w.fill_eggs();
        w
    }

    /// A pit of CPUs only: the title screen plays itself with one of these.
    pub fn demo(count: usize) -> Self {
        let mut w = World::new(1);
        w.snakes.clear();
        for seat in 0..count { w.join(seat, false); }
        w.fill_eggs();
        w
    }

    fn join(&mut self, seat: usize, human: bool) -> usize {
        let (head, heading) = self.clear_spot();
        self.snakes.push(Snake::new(seat, human, head, heading));
        self.snakes.len() - 1
    }

    /// Player two drops in: the seat stops being a CPU. False if it is already
    /// held by a person.
    pub fn join_human(&mut self, seat: usize) -> bool {
        match self.snakes.iter_mut().find(|s| s.seat == seat) {
            Some(s) if !s.human => { s.human = true; true }
            _ => false,
        }
    }

    pub fn score_of(&self, seat: usize) -> i32 {
        self.snakes.iter().find(|s| s.seat == seat).map_or(0, |s| s.score)
    }

    pub fn lives_of(&self, seat: usize) -> i32 {
        self.snakes.iter().find(|s| s.seat == seat).map_or(0, |s| s.lives)
    }

    /// A step quickens a round at a time, because the pit is one snake busier.
    pub fn speed_base(&self) -> f32 { (GLIDE * SPEED_UP.powi(self.round - 1)).min(220.0) }

    /// No human left with a life: the run is over even if CPUs are still going.
    /// A pit with no people in it never ends, which is what the title screen's
    /// demo is.
    pub fn humans_out(&self) -> bool {
        let mut humans = 0;
        for s in self.snakes.iter().filter(|s| s.human) {
            humans += 1;
            if s.lives > 0 { return false; }
        }
        humans > 0
    }

    /// How far the nearest wall, body or skin is from a point.
    pub fn room(&self, p: Pt) -> f32 {
        let mut d = (p.x - PIT_X).min(PIT_X + PIT_W - p.x);
        d = d.min(p.y - PIT_Y).min(PIT_Y + PIT_H - p.y);
        for s in &self.snakes {
            if !s.alive { continue; }
            for q in s.path.iter().step_by(2) { d = d.min(p.dist(*q) - BODY_R); }
        }
        for q in &self.skin { d = d.min(p.dist(*q) - BODY_R); }
        d
    }

    /// The roomiest of a handful of spots, and a heading that points away from
    /// the middle of the pit: somewhere a snake can be put down alive.
    fn clear_spot(&self) -> (Pt, f32) {
        let mid = Pt::new(PIT_X + PIT_W / 2.0, PIT_Y + PIT_H / 2.0);
        let mut best = (mid, -1.0);
        for _ in 0..60 {
            let p = Pt::new(
                rand_range_f32(PIT_X + 30.0, PIT_X + PIT_W - 30.0),
                rand_range_f32(PIT_Y + 40.0, PIT_Y + PIT_H - 40.0),
            );
            let d = self.room(p);
            if d > best.1 { best = (p, d); }
            if d > CLEAR_OF { break; }
        }
        let heading = if best.0.dist(mid) < 1.0 {
            rand_range_f32(0.0, 6.28)
        } else {
            (best.0.y - mid.y).atan2(best.0.x - mid.x)
        };
        (best.0, heading)
    }

    fn fill_eggs(&mut self) {
        while self.eggs.len() < EGGS_IN_PIT { self.spawn_egg(); }
    }

    /// An egg where a snake can reach it, and not on top of another egg.
    fn spawn_egg(&mut self) {
        let mut best: Option<Pt> = None;
        let mut best_room = -1.0;
        for _ in 0..50 {
            let p = Pt::new(
                rand_range_f32(PIT_X + 20.0, PIT_X + PIT_W - 20.0),
                rand_range_f32(PIT_Y + 20.0, PIT_Y + PIT_H - 20.0),
            );
            if self.eggs.iter().any(|e| p.dist(*e) < EGG_R * 4.0) { continue; }
            let d = self.room(p);
            if d > best_room { best_room = d; best = Some(p); }
            if d > 44.0 { break; }
        }
        if let Some(p) = best { self.eggs.push(p); }
    }

    /// A hole opens in the middle of a body, never at the head end: it is a
    /// doorway, and a doorway behind your own neck is no use to anyone.
    fn open_hole(&mut self, i: usize) {
        let s = &mut self.snakes[i];
        let span = HOLE_S.min(s.len_s.saturating_sub(SELF_SKIP + 3));
        if span < 2 { return; }
        let lo = SELF_SKIP + 1;
        let hi = s.len_s.saturating_sub(span + 1);
        let at = if hi <= lo { lo } else { rand_int(lo as i32, hi as i32) as usize };
        s.hole = Some(Hole { at, len: span, spent: false });
        s.hole_ttl = HOLE_TTL;
    }

    /// Distance from a head along `dir` to the first thing that would stop it,
    /// up to `reach`. A hole counts as clear: it is a way through.
    pub fn clearance(&self, who: usize, dir: f32, reach: f32) -> f32 {
        let from = self.snakes[who].head;
        let to = Pt::new(from.x + dir.cos() * reach, from.y + dir.sin() * reach);
        let mut hit = self.wall_hit(from, to);
        for (j, s) in self.snakes.iter().enumerate() {
            if !s.alive { continue; }
            let skip = if j == who { SELF_SKIP } else { 0 };
            for i in (skip..s.path.len()).step_by(2) {
                if s.in_hole(i) { continue; }
                let q = match s.path.get(i) { Some(q) => *q, None => continue };
                if segment_circle_overlap(from.x, from.y, to.x, to.y, q.x, q.y, BODY_R) {
                    hit = hit.min(from.dist(q) - BODY_R);
                }
            }
        }
        for q in &self.skin {
            if segment_circle_overlap(from.x, from.y, to.x, to.y, q.x, q.y, BODY_R) {
                hit = hit.min(from.dist(*q) - BODY_R);
            }
        }
        hit.max(0.0)
    }

    /// Distance along a step to the packed earth, which is what a head dies on.
    fn wall_hit(&self, from: Pt, to: Pt) -> f32 {
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        let mut t = 1.0f32;
        if dx > 0.0001 { t = t.min((PIT_X + PIT_W - BODY_R - from.x) / dx); }
        if dx < -0.0001 { t = t.min((PIT_X + BODY_R - from.x) / dx); }
        if dy > 0.0001 { t = t.min((PIT_Y + PIT_H - BODY_R - from.y) / dy); }
        if dy < -0.0001 { t = t.min((PIT_Y + BODY_R - from.y) / dy); }
        t.clamp(0.0, 1.0) * from.dist(to)
    }

    /// What a head's step meets first: a wall, a body and which sample of it,
    /// skin, or nothing. A sample inside a hole is reported as a body too — it
    /// is the one pass that is legal, and the caller has to tell them apart.
    fn first_hit(&self, who: usize, from: Pt, to: Pt) -> Hit {
        let r = BODY_R;
        if to.x < PIT_X + r || to.x > PIT_X + PIT_W - r || to.y < PIT_Y + r || to.y > PIT_Y + PIT_H - r {
            return Hit::Wall;
        }
        let mut best: Option<(f32, Hit)> = None;
        for (j, s) in self.snakes.iter().enumerate() {
            if !s.alive { continue; }
            let skip = if j == who { SELF_SKIP } else { 0 };
            for i in skip..s.path.len() {
                let q = match s.path.get(i) { Some(q) => *q, None => continue };
                if segment_circle_overlap(from.x, from.y, to.x, to.y, q.x, q.y, r) {
                    nearer(&mut best, from.dist(q), Hit::Body(j, i));
                }
            }
        }
        for q in &self.skin {
            if segment_circle_overlap(from.x, from.y, to.x, to.y, q.x, q.y, r) {
                nearer(&mut best, from.dist(*q), Hit::Skin);
            }
        }
        best.map_or(Hit::None, |(_, h)| h)
    }
}

/// Keep the hit nearest the head: the head meets it first.
fn nearer(best: &mut Option<(f32, Hit)>, d: f32, h: Hit) {
    if best.map_or(true, |(bd, _)| d < bd) { *best = Some((d, h)); }
}

impl World {
    /// One frame for the whole pit. `steers` is what each seat's driver wants;
    /// seats with no snake in the pit are ignored.
    pub fn step(&mut self, dt: f32, steers: &[Steer; SEATS]) {
        if self.over { return; }
        self.fx.update(dt);
        self.banner_t += dt;
        self.fill_eggs();
        let base = self.speed_base();

        for i in 0..self.snakes.len() {
            if !self.snakes[i].alive {
                if self.snakes[i].lives <= 0 { continue; } // out of this round
                self.snakes[i].respawn -= dt;
                if self.snakes[i].respawn <= 0.0 { self.respawn(i); }
                continue;
            }
            self.tick_hole(i, dt);

            let seat = self.snakes[i].seat.min(SEATS - 1);
            let steer = steers[seat];
            let from = self.snakes[i].head;
            let s = &mut self.snakes[i];
            s.boost = steer.boost;
            s.heading += steer.turn.clamp(-1.0, 1.0) * s.turn_rate() * dt;
            let speed = s.speed(base);
            let to = Pt::new(
                from.x + s.heading.cos() * speed * dt,
                from.y + s.heading.sin() * speed * dt,
            );

            match self.first_hit(i, from, to) {
                Hit::Wall | Hit::Skin => { self.die(i, None); continue; }
                Hit::Body(j, k) => {
                    if self.snakes[j].in_hole(k) { self.through(i, j, to); }
                    else { self.die(i, Some(j)); continue; }
                }
                Hit::None => {}
            }

            let s = &mut self.snakes[i];
            s.head = to;
            s.travelling += speed * dt;
            while s.travelling >= SAMPLE {
                s.travelling -= SAMPLE;
                s.path.push_front(to);
            }
            while s.path.len() > s.len_s { s.path.pop_back(); }

            if let Some(k) = self.eggs.iter().position(|e| e.dist(to) < BODY_R + EGG_R) {
                self.eat(i, k, to);
            }
        }

        if self.round_done() {
            if self.humans_out() { self.over = true; } else { self.next_round(); }
        }
    }

    /// The hole's own clock: it lives a while, and once someone has gone through
    /// it, it closes a moment later and a fresh one opens somewhere else.
    fn tick_hole(&mut self, i: usize, dt: f32) {
        if self.snakes[i].hole.is_some() {
            self.snakes[i].hole_ttl -= dt;
            if self.snakes[i].hole_ttl <= 0.0 {
                self.snakes[i].hole = None;
                self.snakes[i].hole_wait = HOLE_WAIT;
            }
        } else if self.snakes[i].hole_wait > 0.0 {
            self.snakes[i].hole_wait -= dt;
        } else {
            self.open_hole(i);
        }
    }

    /// A head went through a hole: the pass pays once, and the hole closes
    /// behind the one who went through it.
    fn through(&mut self, i: usize, j: usize, at: Pt) {
        let spent = self.snakes[j].hole.map_or(true, |h| h.spent);
        if spent { return; }
        if let Some(h) = self.snakes[j].hole {
            self.snakes[j].hole = Some(Hole { spent: true, ..h });
            self.snakes[j].hole_ttl = HOLE_GRACE;
        }
        let pts = HOLE_POINTS * self.round;
        self.snakes[i].score += pts;
        let (mine, theirs) = (self.snakes[i].colour(), self.snakes[j].colour());
        self.fx.ring(at.x, at.y, BODY_R * 3.0, 0.35, theirs);
        self.fx.popup(at.x, at.y - 14.0, &format!("+{pts}"), mine);
        self.events.push(Event::Through);
        blip::bot::add("through", 1.0);
    }

    /// An egg is swallowed: it pays, it lengthens the snake, and every fourth
    /// one leaves the tail behind as skin.
    fn eat(&mut self, i: usize, k: usize, at: Pt) {
        self.eggs.remove(k);
        let pts = EGG_POINTS * self.round * if self.snakes[i].boost { 2 } else { 1 };
        self.snakes[i].score += pts;
        self.snakes[i].len_s += GROW_S;
        self.snakes[i].eaten += 1;
        let c = self.snakes[i].colour();
        self.fx.ring(at.x, at.y, BODY_R * 2.4, 0.28, c);
        self.fx.popup(at.x, at.y - 14.0, &format!("+{pts}"), c);
        self.events.push(Event::Eat);
        blip::bot::add("eggs", 1.0);
        if self.snakes[i].boost { blip::bot::add("strike_eggs", 1.0); }
        if self.snakes[i].eaten % SHED_EVERY == 0 { self.shed(i); }
    }

    /// Freeze the tail as skin — all of it or none of it, and never a tail
    /// lying near the head, where the wall would be the owner's own trap.
    fn shed(&mut self, i: usize) -> bool {
        // The body that exists, not the length it is growing into: a snake that
        // has just swallowed an egg is `len_s` samples long on paper and shorter
        // than that on the floor until it has travelled the difference.
        let (have, head) = (self.snakes[i].path.len(), self.snakes[i].head);
        if have <= SHED_S + SELF_SKIP { return false; }
        let mut frozen = Vec::new();
        for k in 0..SHED_S {
            let at = have - 1 - k;
            match self.snakes[i].path.get(at) {
                Some(q) if !self.snakes[i].in_hole(at) && q.dist(head) >= SHED_CLEAR => frozen.push(*q),
                _ => return false,
            }
        }
        self.snakes[i].len_s -= frozen.len();
        self.skin.extend(frozen);
        self.events.push(Event::Shed);
        blip::bot::add("sheds", 1.0);
        true
    }

    /// A death costs a life and the length that had been earned; the snake
    /// comes back in the roomiest spot going a moment later.
    fn die(&mut self, i: usize, killer: Option<usize>) {
        let (p, c) = (self.snakes[i].head, self.snakes[i].colour());
        let s = &mut self.snakes[i];
        s.alive = false;
        s.lives -= 1;
        s.respawn = RESPAWN;
        s.len_s = START_S;
        s.travelling = 0.0;
        s.hole = None;
        s.hole_wait = HOLE_WAIT;
        self.fx.burst(p.x, p.y, 16, 140.0, c);
        self.events.push(Event::Die);
        if let Some(k) = killer {
            if k != i && self.snakes[k].alive {
                self.snakes[k].score += KILL_POINTS;
                self.events.push(Event::Kill);
            }
        }
        blip::bot::add("deaths", 1.0);
        if self.humans_out() { self.over = true; }
    }

    fn respawn(&mut self, i: usize) {
        let (head, heading) = self.clear_spot();
        let s = &mut self.snakes[i];
        s.alive = true;
        s.respawn = 0.0;
        s.head = head;
        s.heading = heading;
        s.len_s = START_S;
        s.travelling = 0.0;
        s.hole = None;
        s.hole_wait = HOLE_WAIT;
        s.lay_path();
    }

    fn round_done(&self) -> bool {
        // A pit with one snake in it has nothing to win: only a pit that has
        // been whittled down from more than one decides a round.
        self.snakes.iter().filter(|s| s.lives > 0).count() <= 1 && self.snakes.len() > 1
    }

    /// A round is won: the survivor takes the points, the pit is swept, and one
    /// more viper joins — up to the six the original allowed.
    fn next_round(&mut self) {
        if let Some(w) = self.snakes.iter_mut().find(|s| s.lives > 0) { w.score += ROUND_POINTS; }
        self.events.push(Event::RoundWin);
        self.round += 1;
        self.banner_t = 0.0;
        self.skin.clear();
        self.eggs.clear();
        for i in 0..self.snakes.len() {
            let (head, heading) = self.clear_spot();
            let s = &mut self.snakes[i];
            s.alive = true;
            s.respawn = 0.0;
            s.lives = LIVES;
            s.len_s = START_S;
            s.eaten = 0;
            s.head = head;
            s.heading = heading;
            s.travelling = 0.0;
            s.hole = None;
            s.hole_wait = 0.0;
            s.lay_path();
        }
        if self.snakes.len() < MAX_SNAKES {
            let seat = self.snakes.len();
            let human = seat < self.humans;
            self.join(seat, human);
        }
        self.fill_eggs();
        blip::bot::set("round", self.round as f64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pit with one snake on a short leash: no eggs, no CPU waking up, so a
    /// test can put bodies exactly where it wants them.
    fn bare() -> World {
        let mut w = World::new(1);
        w.snakes.clear();
        w.eggs.clear();
        w.skin.clear();
        w
    }

    /// A snake lying along y = 270 from x = 68 to x = 100, head at the east end.
    fn wall(p: f32) -> Snake {
        let mut s = Snake::new(1, false, Pt::new(100.0, p), 0.0);
        s.hole = None;
        s
    }

    #[test]
    fn a_body_is_passable_only_at_its_hole() {
        let mut w = bare();
        w.snakes.push(wall(270.0));
        // A head coming down onto the middle of that body, at x = 84.
        let from = Pt::new(84.0, 250.0);
        let to = Pt::new(84.0, 270.0);
        assert!(matches!(w.first_hit(0, from, to), Hit::Body(_, _)), "the body was not solid");

        // The sample the head meets is 4 in from the head end: 100 - 4*4.
        w.snakes[0].hole = Some(Hole { at: 3, len: 3, spent: false });
        assert_eq!(w.first_hit(0, from, to), Hit::Body(0, 4), "the hole is not where the head arrives");
        assert!(w.snakes[0].in_hole(4), "the sample the head meets is not in the hole");
        assert!(!w.snakes[0].in_hole(2), "the hole covers more body than it was given");
    }

    #[test]
    fn going_through_a_hole_pays_once_and_closes_it() {
        let mut w = bare();
        let mut lying = wall(270.0);
        lying.hole = Some(Hole { at: 3, len: 3, spent: false });
        w.snakes.push(lying);
        w.snakes.push(Snake::new(1, false, Pt::new(84.0, 250.0), std::f32::consts::FRAC_PI_2));

        let at = Pt::new(84.0, 268.0);
        w.through(1, 0, at);
        assert_eq!(w.snakes[1].score, HOLE_POINTS * w.round, "a hole pass did not pay");
        assert!(w.snakes[0].hole.unwrap().spent, "the hole is still paying");
        assert_eq!(w.events, vec![Event::Through]);

        w.through(1, 0, at);
        assert_eq!(w.snakes[1].score, HOLE_POINTS * w.round, "the same hole paid twice");

        // The grace is short: the hole closes behind whoever went through it.
        for _ in 0..40 { w.tick_hole(0, 0.02); }
        assert!(w.snakes[0].hole.is_none(), "a spent hole stayed open for a second pass");
        for _ in 0..200 { w.tick_hole(0, 0.02); }
        assert!(w.snakes[0].hole.is_some(), "no fresh hole opened after the wait");
    }

    #[test]
    fn the_strike_is_quicker_and_stiffer_and_the_egg_is_worth_double() {
        let mut w = bare();
        w.snakes.push(Snake::new(0, true, Pt::new(200.0, 200.0), 0.0));
        let base = w.speed_base();
        let s = &mut w.snakes[0];
        s.boost = false;
        let (glide, turn) = (s.speed(base), s.turn_rate());
        s.boost = true;
        assert!(s.speed(base) > glide * 1.5, "the strike is no quicker than a glide");
        assert!(s.turn_rate() < turn * 0.6, "the strike does not stiffen the neck");

        w.eggs.push(Pt::new(10.0, 10.0));
        w.snakes[0].boost = false;
        w.eat(0, 0, Pt::new(10.0, 10.0));
        let plain = w.snakes[0].score;
        w.eggs.push(Pt::new(20.0, 20.0));
        w.snakes[0].boost = true;
        w.eat(0, 0, Pt::new(20.0, 20.0));
        assert_eq!(w.snakes[0].score - plain, plain * 2, "an egg taken on a strike is not worth double");
    }

    #[test]
    fn a_shed_leaves_the_tail_it_took_on_the_floor() {
        let mut w = bare();
        w.snakes.push(Snake::new(0, true, Pt::new(240.0, 270.0), 0.0));
        w.snakes[0].len_s = 30;
        w.snakes[0].lay_path();
        assert!(w.shed(0), "a shed in the open was refused");
        assert_eq!(w.skin.len(), SHED_S, "the whole tail did not come off");
        assert_eq!(w.snakes[0].len_s, 30 - SHED_S);
        assert_eq!(w.events, vec![Event::Shed]);
    }

    #[test]
    fn a_tail_lying_near_its_own_head_is_not_shed() {
        // A short snake is all tail near its head, and skin there is a wall it
        // would have to get out of at once.
        let mut w = bare();
        let mut s = Snake::new(0, true, Pt::new(240.0, 270.0), 0.0);
        s.len_s = SHED_S + SELF_SKIP + 1;
        s.lay_path();
        w.snakes.push(s);
        assert!(!w.shed(0), "the adder shed a tail lying under its own chin");
        assert!(w.skin.is_empty());
    }

    #[test]
    fn a_death_costs_a_life_and_the_length_and_the_killer_scores() {
        let mut w = bare();
        w.snakes.push(Snake::new(0, true, Pt::new(240.0, 270.0), 0.0));
        w.snakes.push(Snake::new(1, false, Pt::new(120.0, 120.0), 0.0));
        w.snakes[0].len_s = 24;
        w.snakes[0].lives = 2;
        w.die(0, Some(1));
        assert_eq!(w.snakes[0].lives, 1);
        assert_eq!(w.snakes[0].len_s, START_S, "the dead snake kept the length it had earned");
        assert!(!w.snakes[0].alive);
        assert_eq!(w.snakes[1].score, KILL_POINTS, "the body that did it scored nothing");
        assert!(w.events.contains(&Event::Die) && w.events.contains(&Event::Kill));
    }

    #[test]
    fn a_round_is_won_by_the_last_snake_standing_and_another_viper_joins() {
        let mut w = bare();
        w.snakes.push(Snake::new(0, true, Pt::new(240.0, 270.0), 0.0));
        w.snakes.push(Snake::new(1, false, Pt::new(120.0, 120.0), 0.0));
        w.snakes[1].lives = 0;
        assert!(w.round_done());
        let had = w.snakes.len();
        w.skin.push(Pt::new(50.0, 50.0));
        w.next_round();
        assert_eq!(w.round, 2);
        assert_eq!(w.snakes.len(), had + 1, "no viper joined the next round");
        assert!(w.snakes.iter().all(|s| s.lives == LIVES && s.alive), "a snake came back short of lives");
        assert_eq!(w.snakes[0].score, ROUND_POINTS, "winning the round paid nothing");
        assert!(w.skin.is_empty(), "the new round kept the old floor");
        assert_eq!(w.eggs.len(), EGGS_IN_PIT, "the new round has no eggs in it");
    }

    #[test]
    fn no_human_left_with_a_life_ends_the_run() {
        let mut w = bare();
        w.snakes.push(Snake::new(0, true, Pt::new(240.0, 270.0), 0.0));
        w.snakes.push(Snake::new(1, false, Pt::new(120.0, 120.0), 0.0));
        w.snakes[0].lives = 1;
        assert!(!w.humans_out());
        w.die(0, Some(1));
        assert!(w.humans_out());
        assert!(w.over, "the pit plays on with nobody in it");
    }

    #[test]
    fn a_brain_does_not_drive_straight_into_a_body_in_front_of_it() {
        let mut w = bare();
        w.snakes.push(Snake::new(0, false, Pt::new(120.0, 270.0), 0.0));
        // A body lying square across the way on, from just above it to well below.
        let mut square = Snake::new(1, false, Pt::new(150.0, 260.0), -std::f32::consts::FRAC_PI_2);
        square.len_s = 46;
        square.lay_path();
        w.snakes.push(square);
        w.eggs.push(Pt::new(400.0, 270.0));
        let st = crate::cpu::steer(&w, 0);
        assert!(st.turn.abs() > 0.0, "the brain drove straight into the body in its way");
    }

    /// A snake that has eaten its fourth egg has to be able to shed: if the
    /// guard that keeps a shed tail off its own nose is set wider than a
    /// fourth-egg snake is long, the shed meter never fires in a real pit.
    #[test]
    fn a_snake_that_eats_four_eggs_sheds() {
        let mut w = bare();
        w.snakes.push(Snake::new(0, true, Pt::new(60.0, 270.0), 0.0));
        for k in 0..SHED_EVERY { w.eggs.push(Pt::new(120.0 + k as f32 * 40.0, 270.0)); }
        let steers = [Steer::default(); SEATS];
        for _ in 0..900 {
            w.step(1.0 / 60.0, &steers);
            w.events.clear();
            if !w.skin.is_empty() { break; }
        }
        assert_eq!(w.snakes[0].eaten, SHED_EVERY, "the snake did not eat four eggs");
        assert!(!w.skin.is_empty(), "a snake that ate four eggs never shed its tail");
    }

    /// A pit of CPUs plays itself for ten seconds: eggs get eaten, the holes get
    /// used, and the pit is still a game at the end of it.
    #[test]
    fn a_pit_of_cpus_plays_itself() {
        let mut w = bare();
        for seat in 0..3 { w.join(seat, false); }
        w.fill_eggs();
        let mut steers = [Steer::default(); SEATS];
        let mut deaths = 0;
        for _ in 0..600 {
            for (i, s) in w.snakes.iter().enumerate() {
                steers[s.seat.min(SEATS - 1)] = crate::cpu::steer(&w, i);
            }
            w.step(1.0 / 60.0, &steers);
            for e in w.events.drain(..) { if e == Event::Die { deaths += 1; } }
        }
        let eaten: i32 = w.snakes.iter().map(|s| s.eaten).sum();
        assert!(eaten > 0, "three vipers ate nothing in ten seconds");
        assert!(deaths <= 4, "three vipers killed themselves {deaths} times in ten seconds");
        assert!(w.snakes.iter().all(|s| s.lives > 0), "a viper was put out of an empty pit");
    }
}
