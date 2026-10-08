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
pub struct Pt { pub x: f32, pub y: f32 }

impl Pt {
    pub fn new(x: f32, y: f32) -> Self { Self { x, y } }
    pub fn dist(self, o: Pt) -> f32 { ((self.x - o.x).powi(2) + (self.y - o.y).powi(2)).sqrt() }
}

/// Indices shift as the head grows; the gap stays at the same place in the pit.
#[derive(Copy, Clone, Debug)]
pub struct Hole { pub at: usize, pub len: usize, pub spent: bool }

#[derive(Copy, Clone, Debug, Default)]
pub struct Steer { pub turn: f32 }

#[derive(Copy, Clone, Debug, PartialEq)]
enum Hit { None, Wall, Body(usize), Gap(usize, usize) }

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Event { Through, Kill, Die, RoundWin }

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
            path: VecDeque::new(), head, heading, len_s: START_S, seat, human,
            score: 0, lives: if human { LIVES } else { 1 }, alive: true,
            holes: Vec::new(), hole_wait: HOLE_WAIT, gap_left: 0, travelling: 0.0,
        };
        s.lay_path();
        s
    }

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
    pub fn in_hole(&self, i: usize) -> bool {
        self.holes.iter().any(|h| i >= h.at && i < h.at + h.len)
    }

    pub fn hole_centre(&self) -> Option<Pt> {
        let h = self.holes.iter().find(|h| !h.spent)?;
        self.path.get(h.at + h.len / 2).copied()
    }

    fn grow(&mut self, from: Pt, to: Pt) {
        let distance = from.dist(to);
        if distance <= 0.0 { return; }
        let mut along = SAMPLE - self.travelling;
        while along <= distance {
            for h in &mut self.holes { h.at += 1; }
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
        for seat in 0..humans { w.join(seat, true); }
        w.join(humans, false);
        w
    }

    fn empty(humans: usize) -> Self {
        Self {
            snakes: Vec::new(), round: 1, over: false, humans,
            fx: Fx::new(), banner_t: 0.0, events: Vec::new(), round_wait: 0.0,
        }
    }

    pub fn demo(count: usize) -> Self {
        let mut w = Self::empty(0);
        for seat in 0..count.clamp(2, MAX_SNAKES) { w.join(seat, false); }
        w
    }

    fn join(&mut self, seat: usize, human: bool) {
        let (head, heading) = self.clear_spot();
        self.snakes.push(Snake::new(seat, human, head, heading));
    }

    /// Joining a dead CPU reserves its seat for the next round, not a revival.
    pub fn join_human(&mut self, seat: usize) -> bool {
        if self.over || self.round_wait > 0.0 || seat != 1 { return false; }
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

    pub fn speed_base(&self) -> f32 { GLIDE }

    pub fn humans_out(&self) -> bool {
        self.snakes.iter().any(|s| s.human)
            && !self.snakes.iter().any(|s| s.human && s.lives > 0)
    }

    fn room(&self, p: Pt) -> f32 {
        let mut d = (p.x - PIT_X).min(PIT_X + PIT_W - p.x);
        d = d.min(p.y - PIT_Y).min(PIT_Y + PIT_H - p.y);
        for s in &self.snakes {
            d = d.min(p.dist(s.head) - BODY_R);
            for q in &s.path { d = d.min(p.dist(*q) - BODY_R); }
        }
        d
    }

    fn clear_spot(&self) -> (Pt, f32) {
        let mid = Pt::new(PIT_X + PIT_W / 2.0, PIT_Y + PIT_H / 2.0);
        let mut best = (mid, -1.0);
        for _ in 0..60 {
            let p = Pt::new(
                rand_range_f32(PIT_X + 50.0, PIT_X + PIT_W - 50.0),
                rand_range_f32(PIT_Y + 50.0, PIT_Y + PIT_H - 50.0),
            );
            let d = self.room(p);
            if d > best.1 { best = (p, d); }
            if d > CLEAR_OF { break; }
        }
        let heading = (mid.y - best.0.y).atan2(mid.x - best.0.x);
        (best.0, heading)
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
                if s.in_hole(i) { continue; }
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
        if dx > 0.0001 { t = t.min((PIT_X + PIT_W - BODY_R - from.x) / dx); }
        if dx < -0.0001 { t = t.min((PIT_X + BODY_R - from.x) / dx); }
        if dy > 0.0001 { t = t.min((PIT_Y + PIT_H - BODY_R - from.y) / dy); }
        if dy < -0.0001 { t = t.min((PIT_Y + BODY_R - from.y) / dy); }
        t.clamp(0.0, 1.0) * from.dist(to)
    }

    fn first_hit(&self, who: usize, from: Pt, to: Pt) -> Hit {
        let r = BODY_R;
        if to.x < PIT_X + r || to.x > PIT_X + PIT_W - r || to.y < PIT_Y + r || to.y > PIT_Y + PIT_H - r {
            return Hit::Wall;
        }
        let mut gap = Hit::None;
        for (j, s) in self.snakes.iter().enumerate() {
            let skip = if j == who { SELF_SKIP } else { 0 };
            for (i, q) in s.path.iter().enumerate().skip(skip) {
                if !segment_circle_overlap(from.x, from.y, to.x, to.y, q.x, q.y, r) { continue; }
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
        let speed = self.speed_base();
        for i in 0..self.snakes.len() {
            if !self.snakes[i].alive { continue; }
            self.tick_hole(i, dt);
            let seat = self.snakes[i].seat.min(SEATS - 1);
            let from = self.snakes[i].head;
            let s = &mut self.snakes[i];
            s.heading += steers[seat].turn.clamp(-1.0, 1.0) * TURN * dt;
            let to = Pt::new(from.x + s.heading.cos() * speed * dt, from.y + s.heading.sin() * speed * dt);
            match self.first_hit(i, from, to) {
                Hit::Wall => { self.die(i, None); continue; }
                Hit::Body(j) => { self.die(i, Some(j)); continue; }
                Hit::Gap(j, k) => self.through(i, j, k, to),
                Hit::None => {}
            }
            self.snakes[i].grow(from, to);
        }
        if self.humans_out() {
            self.over = true;
        } else if self.round_done() {
            if let Some(w) = self.snakes.iter_mut().find(|s| s.alive) {
                w.score += ROUND_POINTS;
                self.events.push(Event::RoundWin);
            }
            self.round_wait = ROUND_WAIT;
        }
    }

    fn through(&mut self, i: usize, j: usize, k: usize, at: Pt) {
        let hole = &mut self.snakes[j].holes[k];
        if hole.spent { return; }
        hole.spent = true;
        let pts = HOLE_POINTS * self.round;
        self.snakes[i].score += pts;
        self.fx.popup(at.x, at.y - 14.0, &format!("+{pts}"), self.snakes[i].colour());
        self.events.push(Event::Through);
        blip::bot::add("through", 1.0);
    }

    fn die(&mut self, i: usize, killer: Option<usize>) {
        if !self.snakes[i].alive { return; }
        let s = &mut self.snakes[i];
        s.alive = false;
        s.lives -= 1;
        self.fx.burst(s.head.x, s.head.y, 10, 100.0, s.colour());
        self.events.push(Event::Die);
        if let Some(k) = killer {
            if k != i {
                self.snakes[k].score += KILL_POINTS;
                self.events.push(Event::Kill);
            }
        }
        blip::bot::add("deaths", 1.0);
    }

    fn round_done(&self) -> bool {
        self.snakes.len() > 1 && self.snakes.iter().filter(|s| s.alive).count() <= 1
    }

    fn next_round(&mut self) {
        self.round += 1;
        self.banner_t = 0.0;
        let old = std::mem::take(&mut self.snakes);
        for s in old {
            let (head, heading) = self.clear_spot();
            let mut fresh = Snake::new(s.seat, s.human, head, heading);
            fresh.score = s.score;
            if s.human {
                fresh.lives = s.lives;
                fresh.alive = s.lives > 0;
                if !fresh.alive { fresh.path.clear(); fresh.len_s = 0; }
            }
            self.snakes.push(fresh);
        }
        if self.snakes.len() < MAX_SNAKES { self.join(self.snakes.len(), false); }
        blip::bot::set("round", self.round as f64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bare() -> World { World::empty(1) }

    fn wall() -> Snake {
        Snake::new(1, false, Pt::new(100.0, 270.0), 0.0)
    }

    #[test]
    fn body_collisions_follow_the_thin_width() {
        let mut w = bare();
        w.snakes.push(wall());
        assert_eq!(w.first_hit(0, Pt::new(76.0, 273.0), Pt::new(88.0, 273.0)), Hit::None);
        assert!(matches!(w.first_hit(0, Pt::new(76.0, 271.0), Pt::new(88.0, 271.0)), Hit::Body(0)));
    }

    #[test]
    fn a_body_is_passable_only_at_its_gap() {
        let mut w = bare();
        w.snakes.push(wall());
        let from = Pt::new(84.0, 250.0);
        let to = Pt::new(84.0, 280.0);
        assert_eq!(w.first_hit(0, from, to), Hit::Body(0));
        w.snakes[0].holes.push(Hole { at: 5, len: 7, spent: false });
        assert_eq!(w.first_hit(0, from, to), Hit::Gap(0, 0));
    }

    #[test]
    fn a_solid_trail_is_not_masked_by_an_overlapping_gap() {
        let mut w = bare();
        w.snakes.push(wall());
        w.snakes[0].holes.push(Hole { at: 5, len: 7, spent: false });
        w.snakes.push(wall());
        assert_eq!(w.first_hit(0, Pt::new(84.0, 250.0), Pt::new(84.0, 280.0)), Hit::Body(1));
    }

    #[test]
    fn a_gap_pays_once_but_stays_passable() {
        let mut w = bare();
        w.snakes.push(wall());
        w.snakes[0].holes.push(Hole { at: 5, len: 7, spent: false });
        w.snakes.push(Snake::new(0, true, Pt::new(84.0, 250.0), 0.0));
        w.through(1, 0, 0, Pt::new(84.0, 270.0));
        w.through(1, 0, 0, Pt::new(84.0, 270.0));
        assert_eq!(w.snakes[1].score, HOLE_POINTS);
        assert_eq!(w.events, vec![Event::Through]);
        assert_eq!(w.first_hit(1, Pt::new(84.0, 260.0), Pt::new(84.0, 280.0)), Hit::Gap(0, 0));
        w.tick_hole(0, 10.0);
        assert!(w.snakes[0].in_hole(8));
    }

    #[test]
    fn moving_keeps_the_tail_and_grows_without_food() {
        let mut w = bare();
        w.snakes.push(Snake::new(0, true, Pt::new(200.0, 200.0), 0.0));
        let tail = w.snakes[0].path.back().copied();
        for _ in 0..60 { w.step(1.0 / 60.0, &[Steer::default(); SEATS]); }
        assert!(w.snakes[0].alive);
        assert!(w.snakes[0].path.len() >= START_S + 42);
        assert_eq!(w.snakes[0].len_s, w.snakes[0].path.len());
        assert_eq!(w.snakes[0].path.back().copied(), tail);
    }

    #[test]
    fn growth_samples_the_whole_step_not_just_its_endpoint() {
        let mut s = wall();
        s.grow(Pt::new(100.0, 270.0), Pt::new(110.0, 270.0));
        let xs: Vec<_> = s.path.iter().take(6).map(|p| p.x).collect();
        assert_eq!(xs, vec![110.0, 108.0, 106.0, 104.0, 102.0, 100.0]);
    }

    #[test]
    fn growth_keeps_gaps_at_their_original_position() {
        let mut s = wall();
        s.holes.push(Hole { at: 5, len: 7, spent: false });
        let centre = s.hole_centre();
        s.grow(Pt::new(100.0, 270.0), Pt::new(120.0, 270.0));
        assert_eq!(s.hole_centre(), centre);
        assert_eq!(s.holes[0].at, 15);
    }

    #[test]
    fn growth_opens_multiple_gaps_without_closing_old_ones() {
        let mut w = bare();
        w.snakes.push(wall());
        w.tick_hole(0, HOLE_WAIT);
        w.snakes[0].grow(Pt::new(100.0, 270.0), Pt::new(150.0, 270.0));
        let centre = w.snakes[0].hole_centre();
        w.tick_hole(0, HOLE_WAIT);
        w.snakes[0].grow(Pt::new(150.0, 270.0), Pt::new(200.0, 270.0));
        assert_eq!(w.snakes[0].holes.len(), 2);
        assert_eq!(w.snakes[0].hole_centre(), centre);
    }

    #[test]
    fn gaps_are_laid_down_gradually_without_changing_existing_trail() {
        let mut w = bare();
        w.snakes.push(wall());
        let old_path = w.snakes[0].path.clone();
        w.tick_hole(0, HOLE_WAIT);
        assert!(w.snakes[0].holes.is_empty());
        assert_eq!(w.snakes[0].path, old_path);

        w.snakes[0].grow(Pt::new(100.0, 270.0), Pt::new(104.0, 270.0));
        assert_eq!(w.snakes[0].holes[0].at, 0);
        assert_eq!(w.snakes[0].holes[0].len, 2);
        assert!(w.snakes[0].in_hole(0) && w.snakes[0].in_hole(1));
        assert!(!w.snakes[0].in_hole(2));

        w.snakes[0].grow(Pt::new(104.0, 270.0), Pt::new(124.0, 270.0));
        assert_eq!(w.snakes[0].holes[0].at, 2);
        assert_eq!(w.snakes[0].holes[0].len, HOLE_S);
        assert!(!w.snakes[0].in_hole(0));
        assert!(w.snakes[0].in_hole(2));
        assert!(!w.snakes[0].in_hole(12));
        assert_eq!(w.snakes[0].path.iter().skip(12).copied().collect::<VecDeque<_>>(), old_path);
        for i in 12..w.snakes[0].path.len() { assert!(!w.snakes[0].in_hole(i)); }
    }

    #[test]
    fn all_seats_and_rounds_move_at_the_same_speed() {
        let mut w = bare();
        w.snakes.push(Snake::new(0, true, Pt::new(200.0, 200.0), 0.0));
        w.snakes.push(Snake::new(1, false, Pt::new(200.0, 400.0), 0.0));
        w.round = 12;
        w.step(0.1, &[Steer::default(); SEATS]);
        assert_eq!(w.speed_base(), GLIDE);
        assert!((w.snakes[0].head.x - 208.5).abs() < 0.001);
        assert!((w.snakes[1].head.x - 208.5).abs() < 0.001);
    }

    #[test]
    fn an_eliminated_worm_keeps_its_trail_and_never_revives_in_round() {
        let mut w = bare();
        w.snakes.push(Snake::new(0, true, Pt::new(200.0, 200.0), 0.0));
        w.snakes.push(wall());
        w.snakes.push(Snake::new(2, false, Pt::new(300.0, 400.0), 0.0));
        let path = w.snakes[0].path.clone();
        w.die(0, None);
        w.die(0, None);
        w.step(0.1, &[Steer::default(); SEATS]);
        assert!(!w.snakes[0].alive);
        assert_eq!(w.snakes[0].lives, LIVES - 1);
        assert_eq!(w.snakes[0].path, path);
        assert_eq!(w.round, 1);
        assert_eq!(w.first_hit(1, Pt::new(184.0, 190.0), Pt::new(184.0, 210.0)), Hit::Body(0));
    }

    #[test]
    fn last_alive_wins_then_the_next_round_clears_trails_and_preserves_lives() {
        let mut w = World::new(1);
        let score = w.snakes[1].score;
        w.die(0, None);
        w.step(0.0, &[Steer::default(); SEATS]);
        assert_eq!(w.round, 1);
        assert_eq!(w.round_wait, ROUND_WAIT);
        assert_eq!(w.snakes[1].score, score + ROUND_POINTS);
        assert!(!w.snakes[0].alive);
        let head = w.snakes[1].head;
        w.step(ROUND_WAIT / 2.0, &[Steer::default(); SEATS]);
        assert_eq!(w.snakes[1].head, head);
        w.step(ROUND_WAIT / 2.0, &[Steer::default(); SEATS]);
        assert_eq!(w.round, 2);
        assert_eq!(w.snakes.len(), 3);
        assert!(w.snakes[0].alive);
        assert_eq!(w.snakes[0].lives, LIVES - 1);
        assert!(w.snakes.iter().all(|s| s.path.len() == START_S && s.holes.is_empty()));
    }

    #[test]
    fn no_human_lives_left_ends_the_run() {
        let mut w = World::new(1);
        w.snakes[0].lives = 1;
        w.die(0, None);
        w.step(0.0, &[Steer::default(); SEATS]);
        assert!(w.over);
        assert_eq!(w.round, 1);
    }

    #[test]
    fn player_two_can_join_without_reviving_an_eliminated_seat() {
        let mut w = World::new(1);
        w.join(2, false);
        w.die(1, None);
        assert!(w.join_human(1));
        assert_eq!(w.humans, 2);
        assert_eq!(w.snakes[1].lives, LIVES);
        assert!(!w.snakes[1].alive);
        assert!(!w.join_human(1));
    }

    #[test]
    fn a_brain_avoids_a_dead_body_in_front_of_it() {
        let mut w = bare();
        w.snakes.push(Snake::new(0, false, Pt::new(120.0, 270.0), 0.0));
        let mut square = Snake::new(1, false, Pt::new(150.0, 230.0), -std::f32::consts::FRAC_PI_2);
        square.len_s = 46;
        square.lay_path();
        square.alive = false;
        w.snakes.push(square);
        assert!(crate::cpu::steer(&w, 0).turn.abs() > 0.0);
    }

    #[test]
    fn a_pit_of_cpus_can_play_multiple_rounds() {
        let mut w = World::demo(3);
        for _ in 0..3600 {
            let mut steers = [Steer::default(); SEATS];
            for (i, s) in w.snakes.iter().enumerate() {
                if s.alive { steers[s.seat] = crate::cpu::steer(&w, i); }
            }
            w.step(1.0 / 60.0, &steers);
            w.events.clear();
        }
        assert!(!w.over);
        assert!(w.round > 1);
        assert!(w.snakes.len() <= MAX_SNAKES);
        assert!(w.snakes.iter().all(|s| s.len_s == s.path.len()));
    }
}
