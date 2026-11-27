//! Growing trails, permanent gaps, and one elimination per worm per round.

use std::collections::VecDeque;

use blip::{rand_range_f32, segment_circle_overlap, BlipColor, Fx};

pub const PIT_X: f32 = 10.0;
pub const PIT_Y: f32 = 38.0;
pub const PIT_W: f32 = 480.0 - PIT_X * 2.0;
pub const PIT_H: f32 = 540.0 - PIT_Y - PIT_X;
pub const BODY_R: f32 = 2.0;
pub const SAMPLE: f32 = 2.0;
pub const START_S: usize = 17;
pub const GLIDE: f32 = 85.0;
pub const TURN: f32 = 3.2;
pub const LIVES: i32 = 3;
pub const MAX_SNAKES: usize = 6;
pub const SEATS: usize = 6;
pub const HOLE_POINTS: i32 = 50;
pub const KILL_POINTS: i32 = 100;
pub const ROUND_POINTS: i32 = 250;
pub const HOLE_S: usize = 10;
pub const HOLE_WAIT: f32 = 2.5;
pub const ROUND_WAIT: f32 = 2.0;
const CLEAR_OF: f32 = 64.0;
const SELF_SKIP: usize = 4;

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Pt {
    pub x: f32,
    pub y: f32,
}

impl Pt {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub fn dist(self, o: Pt) -> f32 {
        blip::distance(self.x, self.y, o.x, o.y)
    }
}

/// Indices shift as the head grows; the gap stays at the same place in the pit.
#[derive(Copy, Clone, Debug)]
pub struct Hole {
    pub at: usize,
    pub len: usize,
    pub spent: bool,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Steer {
    pub turn: f32,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum Hit {
    None,
    Wall,
    Body(usize),
    Gap(usize, usize),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Event {
    Through,
    Kill,
    /// A viper is out; `human` when a person steered it.
    Die { human: bool },
    RoundWin,
}

pub struct Snake {
    /// Head first, one sample every `SAMPLE` pixels travelled; the tail never moves.
    pub path: VecDeque<Pt>,
    pub head: Pt,
    pub heading: f32,
    pub len_s: usize,
    pub seat: usize,
    pub human: bool,
    pub score: i32,
    pub lives: i32,
    pub alive: bool,
    pub holes: Vec<Hole>,
    hole_wait: f32,
    gap_left: usize,
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
            lives: if human { LIVES } else { 1 },
            alive: true,
            holes: Vec::new(),
            hole_wait: HOLE_WAIT,
            gap_left: 0,
            travelling: 0.0,
        };
        s.lay_path();
        s
    }

    fn lay_path(&mut self) {
        self.path.clear();
        for i in 0..self.len_s {
            let d = i as f32 * SAMPLE;
            self.path.push_back(Pt::new(self.head.x - self.heading.cos() * d, self.head.y - self.heading.sin() * d));
        }
    }

    pub fn name(&self) -> &'static str {
        seat_name(self.seat)
    }
    pub fn colour(&self) -> BlipColor {
        seat_colour(self.seat)
    }
    pub fn in_hole(&self, i: usize) -> bool {
        self.holes.iter().any(|h| i >= h.at && i < h.at + h.len)
    }

    pub fn hole_centre(&self) -> Option<Pt> {
        let h = self.holes.iter().find(|h| !h.spent)?;
        self.path.get(h.at + h.len / 2).copied()
    }

    fn grow(&mut self, from: Pt, to: Pt) {
        let distance = from.dist(to);
        if distance <= 0.0 {
            return;
        }
        let mut along = SAMPLE - self.travelling;
        while along <= distance {
            for h in &mut self.holes {
                h.at += 1;
            }
            self.path.push_front(Pt::new(
                from.x + (to.x - from.x) * along / distance,
                from.y + (to.y - from.y) * along / distance,
            ));
            if self.gap_left > 0 {
                if self.gap_left == HOLE_S {
                    self.holes.push(Hole { at: 0, len: 1, spent: false });
                } else if let Some(h) = self.holes.last_mut() {
                    h.at = 0;
                    h.len += 1;
                }
                self.gap_left -= 1;
            }
            along += SAMPLE;
        }
        self.travelling = (self.travelling + distance).rem_euclid(SAMPLE);
        self.len_s = self.path.len();
        self.head = to;
    }
}

pub fn seat_colour(seat: usize) -> BlipColor {
    match seat {
        0 => BlipColor { r: 0.41, g: 0.75, b: 0.30, a: 1.0 }, // the blip look's leaf
        1 => BlipColor { r: 0.45, g: 0.85, b: 1.0, a: 1.0 },
        2 => BlipColor { r: 0.78, g: 0.55, b: 1.0, a: 1.0 },
        3 => BlipColor { r: 0.45, g: 0.95, b: 0.55, a: 1.0 },
        4 => BlipColor { r: 0.95, g: 0.45, b: 0.35, a: 1.0 },
        _ => BlipColor { r: 0.78, g: 0.78, b: 0.84, a: 1.0 },
    }
}

pub fn seat_name(seat: usize) -> &'static str {
    match seat {
        0 => "P1",
        1 => "P2",
        2 => "C1",
        3 => "C2",
        4 => "C3",
        _ => "C4",
    }
}

pub struct World {
    pub snakes: Vec<Snake>,
    pub round: i32,
    pub over: bool,
    pub humans: usize,
    pub fx: Fx,
    pub banner_t: f32,
    pub events: Vec<Event>,
    pub round_wait: f32,
}

impl World {
    pub fn new(humans: usize) -> Self {
        let humans = humans.clamp(1, 2);
        let mut w = Self::empty(humans);
        for seat in 0..humans {
            w.join(seat, true);
        }
        w.join(humans, false);
        w
    }

    /// A new pit at `round`, as full as playing up to it would have made it: a CPU more each round.
    pub fn at_round(humans: usize, round: i32) -> Self {
        let mut w = Self::new(humans);
        w.round = round.max(1);
        for _ in 1..w.round {
            if w.snakes.len() < MAX_SNAKES { w.join(w.snakes.len(), false); }
        }
        w
    }

    fn empty(humans: usize) -> Self {
        Self {
            snakes: Vec::new(),
            round: 1,
            over: false,
            humans,
            fx: Fx::new(),
            banner_t: 0.0,
            events: Vec::new(),
            round_wait: 0.0,
        }
    }

    pub fn demo(count: usize) -> Self {
        let mut w = Self::empty(0);
        for seat in 0..count.clamp(2, MAX_SNAKES) {
            w.join(seat, false);
        }
        w
    }

    fn join(&mut self, seat: usize, human: bool) {
        let (head, heading) = self.find_spawn();
        self.snakes.push(Snake::new(seat, human, head, heading));
    }

    /// Joining a dead CPU reserves its seat for the next round, not a revival.
    pub fn join_human(&mut self, seat: usize) -> bool {
        if self.over || self.round_wait > 0.0 || seat != 1 {
            return false;
        }
        match self.snakes.iter_mut().find(|s| s.seat == seat) {
            Some(s) if !s.human => {
                s.human = true;
                s.lives = LIVES;
                self.humans = 2;
                true
            }
            _ => false,
        }
    }

    pub fn score_of(&self, seat: usize) -> i32 {
        self.snakes.iter().find(|s| s.seat == seat).map_or(0, |s| s.score)
    }

    pub fn lives_of(&self, seat: usize) -> i32 {
        self.snakes.iter().find(|s| s.seat == seat).map_or(0, |s| s.lives)
    }

    pub fn speed_base(&self) -> f32 {
        GLIDE
    }

    pub fn humans_out(&self) -> bool {
        self.snakes.iter().any(|s| s.human) && !self.snakes.iter().any(|s| s.human && s.lives > 0)
    }

    fn room_at(&self, p: Pt) -> f32 {
        let mut d = (p.x - PIT_X).min(PIT_X + PIT_W - p.x);
        d = d.min(p.y - PIT_Y).min(PIT_Y + PIT_H - p.y);
        for s in &self.snakes {
            d = d.min(p.dist(s.head) - BODY_R);
            for q in &s.path {
                d = d.min(p.dist(*q) - BODY_R);
            }
        }
        d
    }

    fn find_spawn(&self) -> (Pt, f32) {
        let mid = Pt::new(PIT_X + PIT_W / 2.0, PIT_Y + PIT_H / 2.0);
        let mut best = (mid, -1.0);
        for _ in 0..60 {
            let p = Pt::new(
                rand_range_f32(PIT_X + 50.0, PIT_X + PIT_W - 50.0),
                rand_range_f32(PIT_Y + 50.0, PIT_Y + PIT_H - 50.0),
            );
            let d = self.room_at(p);
            if d > best.1 {
                best = (p, d);
            }
            if d > CLEAR_OF {
                break;
            }
        }
        (best.0, self.open_heading(best.0))
    }

    /// The way out of `p` with the longest run to the wall whose path, a second
    /// on, keeps furthest from where the others will be: aimed at the middle,
    /// three snakes met there before anyone could steer.
    fn open_heading(&self, p: Pt) -> f32 {
        let ahead = |q: Pt, a: f32| Pt::new(q.x + a.cos() * 120.0, q.y + a.sin() * 120.0);
        let score = |a: f32| {
            let wall = self.wall_hit(p, ahead(p, a)).min(120.0);
            let mine = ahead(p, a);
            let apart = self
                .snakes
                .iter()
                .filter(|s| s.alive)
                .map(|s| mine.dist(ahead(s.head, s.heading)).min(mine.dist(s.head)))
                .fold(240.0f32, f32::min);
            wall * 2.0 + apart
        };
        (0..16)
            .map(|k| k as f32 * std::f32::consts::TAU / 16.0)
            .max_by(|a, b| score(*a).total_cmp(&score(*b)))
            .unwrap_or(0.0)
    }

    fn tick_hole(&mut self, i: usize, dt: f32) {
        let s = &mut self.snakes[i];
        s.hole_wait -= dt;
        if s.hole_wait <= 0.0 && s.gap_left == 0 {
            s.gap_left = HOLE_S;
            s.hole_wait += HOLE_WAIT;
        }
    }

    pub fn clearance(&self, who: usize, dir: f32, reach: f32) -> f32 {
        let from = self.snakes[who].head;
        let to = Pt::new(from.x + dir.cos() * reach, from.y + dir.sin() * reach);
        let mut hit = self.wall_hit(from, to);
        for (j, s) in self.snakes.iter().enumerate() {
            let skip = if j == who { SELF_SKIP } else { 0 };
            for (i, q) in s.path.iter().enumerate().skip(skip) {
                if s.in_hole(i) {
                    continue;
                }
                if segment_circle_overlap(from.x, from.y, to.x, to.y, q.x, q.y, BODY_R) {
                    hit = hit.min(from.dist(*q) - BODY_R);
                }
            }
        }
        hit.max(0.0)
    }

    fn wall_hit(&self, from: Pt, to: Pt) -> f32 {
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        let mut t = 1.0f32;
        if dx > 0.0001 {
            t = t.min((PIT_X + PIT_W - BODY_R - from.x) / dx);
        }
        if dx < -0.0001 {
            t = t.min((PIT_X + BODY_R - from.x) / dx);
        }
        if dy > 0.0001 {
            t = t.min((PIT_Y + PIT_H - BODY_R - from.y) / dy);
        }
        if dy < -0.0001 {
            t = t.min((PIT_Y + BODY_R - from.y) / dy);
        }
        t.clamp(0.0, 1.0) * from.dist(to)
    }

    #[cfg(test)]
    fn first_hit(&self, who: usize, from: Pt, to: Pt) -> Hit {
        self.first_hit_with_heads(who, from, to, None)
    }

    fn first_hit_with_heads(&self, who: usize, from: Pt, to: Pt, moving_heads: Option<&[bool]>) -> Hit {
        let r = BODY_R;
        if to.x < PIT_X + r
            || to.x > PIT_X + PIT_W - r
            || to.y < PIT_Y + r
            || to.y > PIT_Y + PIT_H - r
        {
            return Hit::Wall;
        }
        let mut gap = Hit::None;
        for (j, s) in self.snakes.iter().enumerate() {
            let skip = if j == who {
                SELF_SKIP
            } else if moving_heads.is_some_and(|heads| heads[j]) {
                1
            } else {
                0
            };
            for (i, q) in s.path.iter().enumerate().skip(skip) {
                if !segment_circle_overlap(from.x, from.y, to.x, to.y, q.x, q.y, r) {
                    continue;
                }
                if let Some(k) = s.holes.iter().position(|h| i >= h.at && i < h.at + h.len) {
                    gap = Hit::Gap(j, k);
                } else {
                    return Hit::Body(j);
                }
            }
        }
        gap
    }

    pub fn step(&mut self, dt: f32, steers: &[Steer; SEATS]) {
        if self.over { return; }
        self.fx.update(dt);
        self.banner_t += dt;
        if self.round_wait > 0.0 {
            self.round_wait = (self.round_wait - dt).max(0.0);
            if self.round_wait == 0.0 { self.next_round(); }
            return;
        }
        let mut moves = self.plan_moves(dt, steers);
        heads_meet(&mut moves);
        // `i` pairs each move with its slot: the eliminations are indexed by it too.
        for (i, mv) in moves.iter().copied().enumerate() {
            let Some((_, to, hit)) = mv else { continue };
            match hit {
                Hit::Wall => self.eliminate(i, None),
                Hit::Body(j) => self.eliminate(i, Some(j)),
                Hit::Gap(j, k) => self.go_through(i, j, k, to),
                Hit::None => {}
            }
        }
        for (i, movement) in moves.into_iter().enumerate() {
            if let Some((from, to, _)) = movement {
                if self.snakes[i].alive { self.snakes[i].grow(from, to); }
            }
        }
        if self.humans_out() {
            self.over = true;
        } else if self.round_done() {
            let mut alive = self.snakes.iter_mut().filter(|s| s.alive);
            if let (Some(w), None) = (alive.next(), alive.next()) {
                w.score += ROUND_POINTS;
                if w.human { blip::bot::add("rounds_won", 1.0); }
                self.events.push(Event::RoundWin);
            }
            self.round_wait = ROUND_WAIT;
        }
    }

    /// Each live viper turns as steered and moves on at the round's speed; where it would go, and what it would meet
    /// on the way (tested against where the others' heads are now).
    fn plan_moves(&mut self, dt: f32, steers: &[Steer; SEATS]) -> [Option<Move>; MAX_SNAKES] {
        let speed = self.speed_base();
        let mut moving_heads = [false; MAX_SNAKES];
        for (i, snake) in self.snakes.iter().enumerate() { moving_heads[i] = snake.alive; }
        let mut moves = [None; MAX_SNAKES];
        for i in 0..self.snakes.len() {
            if !moving_heads[i] { continue; }
            self.tick_hole(i, dt);
            let seat = self.snakes[i].seat.min(SEATS - 1);
            let from = self.snakes[i].head;
            let s = &mut self.snakes[i];
            s.heading += steers[seat].turn.clamp(-1.0, 1.0) * TURN * dt;
            let to = Pt::new(from.x + s.heading.cos() * speed * dt, from.y + s.heading.sin() * speed * dt);
            let hit = self.first_hit_with_heads(i, from, to, Some(&moving_heads));
            moves[i] = Some((from, to, hit));
        }
        moves
    }

    fn go_through(&mut self, i: usize, j: usize, k: usize, at: Pt) {
        let hole = &mut self.snakes[j].holes[k];
        if hole.spent {
            return;
        }
        hole.spent = true;
        let pts = HOLE_POINTS * self.round;
        self.snakes[i].score += pts;
        self.fx.popup(at.x, at.y - 14.0, &format!("+{pts}"), self.snakes[i].colour());
        self.events.push(Event::Through);
        if self.snakes[i].human { blip::bot::add("through", 1.0); }
    }

    fn eliminate(&mut self, i: usize, killer: Option<usize>) {
        if !self.snakes[i].alive {
            return;
        }
        // A person who outlasted a CPU this round keeps the life: at a person's reaction time (BLIP_BOT_REACT=250)
        // every lost round cost one, and three were gone in about 22 s (fun report difficulty 0.58, calm 0.63).
        let outlasted = self.snakes[i].human && self.snakes.iter().any(|o| !o.human && !o.alive);
        let s = &mut self.snakes[i];
        s.alive = false;
        if outlasted {
            self.fx.popup(s.head.x, s.head.y - 14.0, "LIFE KEPT", s.colour());
            if s.human { blip::bot::add("lives_kept", 1.0); }
        } else {
            s.lives -= 1;
        }
        let s = &self.snakes[i];
        self.fx.burst(s.head.x, s.head.y, 10, 100.0, s.colour());
        self.events.push(Event::Die { human: s.human });
        if let Some(k) = killer {
            if k != i {
                self.snakes[k].score += KILL_POINTS;
                if self.snakes[k].human { blip::bot::add("kills", 1.0); }
                self.events.push(Event::Kill);
            }
        }
        if self.snakes[i].human && !outlasted { blip::bot::add("deaths", 1.0); }
    }

    /// Once every person in the pit is down, the CPUs' finish is only waiting.
    fn round_done(&self) -> bool {
        let people_down = self.snakes.iter().any(|s| s.human) && !self.snakes.iter().any(|s| s.human && s.alive);
        self.snakes.len() > 1 && (people_down || self.snakes.iter().filter(|s| s.alive).count() <= 1)
    }

    fn next_round(&mut self) {
        self.round += 1;
        self.banner_t = 0.0;
        let old = std::mem::take(&mut self.snakes);
        for s in old {
            let (head, heading) = self.find_spawn();
            let mut fresh = Snake::new(s.seat, s.human, head, heading);
            fresh.score = s.score;
            if s.human {
                fresh.lives = s.lives;
                fresh.alive = s.lives > 0;
                if !fresh.alive {
                    fresh.path.clear();
                    fresh.len_s = 0;
                }
            }
            self.snakes.push(fresh);
        }
        if self.snakes.len() < MAX_SNAKES {
            self.join(self.snakes.len(), false);
        }
        blip::bot::reached_level(self.round);
    }
}

/// A viper's move this frame: from, to, and what it meets on the way.
type Move = (Pt, Pt, Hit);

/// Two heads that meet head on both go out, each on the other's body.
fn heads_meet(moves: &mut [Option<Move>; MAX_SNAKES]) {
    for i in 0..moves.len() {
        let Some((from_i, to_i, _)) = moves[i] else { continue };
        for j in i + 1..moves.len() {
            let Some((from_j, to_j, _)) = moves[j] else { continue };
            let (x0, y0, x1, y1) = (from_i.x - from_j.x, from_i.y - from_j.y, to_i.x - to_j.x, to_i.y - to_j.y);
            if segment_circle_overlap(x0, y0, x1, y1, 0.0, 0.0, BODY_R * 2.0) {
                moves[i].as_mut().unwrap().2 = Hit::Body(j);
                moves[j].as_mut().unwrap().2 = Hit::Body(i);
            }
        }
    }
}

#[cfg(test)]
mod tests;
