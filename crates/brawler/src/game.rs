//! The match: projectiles, the helpers a turtle calls, the barrels of the
//! bonus round, and `Game`, which holds a whole session: the ladder, the
//! round and everything on the stage.

use super::*;

/// A flash where something connected. `blocked` picks the colour, which
/// is how a player tells "that cost me nothing" from "that cost me".
#[derive(Copy, Clone)]
pub(crate) struct Spark { pub(crate) x: f32, pub(crate) y: f32, pub(crate) ttl: f32, pub(crate) life: f32, pub(crate) blocked: bool }

/// One of the other turtles, called in: a fighter run by a script instead of
/// a stick. He waits his turn, runs in, strikes once and runs off the far side.
#[derive(Copy, Clone)]
pub(crate) struct Helper { pub(crate) f: Fighter, pub(crate) side: usize, pub(crate) wait: f32, pub(crate) mv: MoveId, pub(crate) struck: bool }

/// A barrel in the bonus round. In the first round five stand in a row and
/// take three blows each; in the second ten come down one at a time, bounce,
/// and roll for the edge, and one blow does it.
#[derive(Copy, Clone)]
pub(crate) struct Barrel {
    /// Where its foot is.
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) vx: f32,
    pub(crate) vy: f32,
    /// Seconds until it appears.
    pub(crate) wait: f32,
    /// The blows it has left. Nought with `broke_t` negative is one that
    /// was never there, or rolled away.
    pub(crate) hp: i32,
    /// One that falls and rolls, rather than stands.
    pub(crate) rolls: bool,
    /// Clocks for the drawing: since it was last struck, and since it broke.
    pub(crate) hit_t: f32,
    pub(crate) broke_t: f32,
}
/// The most barrels a bonus round has.
pub(crate) const BARRELS: usize = 10;
pub(crate) const BARREL_HP: i32 = 3;
pub(crate) const BARREL_W: f32 = 40.0;
/// How fast a fallen barrel rolls for the edge.
pub(crate) const BARREL_ROLL: f32 = 90.0;
/// The bonus round: after which fights it comes, how long it lasts, and
/// how long its result stays up.
pub(crate) const BONUS_AFTER: [usize; 2] = [2, 5];
pub(crate) const BONUS_SECS: f32 = 20.0;
pub(crate) const BONUS_TALLY: f32 = 2.4;

/// A word that pops up where something happened: COUNTER!, a fruit's points.
#[derive(Copy, Clone)]
pub(crate) struct Pop { pub(crate) text: &'static str, pub(crate) x: f32, pub(crate) y: f32, pub(crate) ttl: f32 }
pub(crate) const POP_SECS: f32 = 0.9;

/// The bonus fruit on the boards: where, how long it has left, and which of
/// the eight it is.
#[derive(Copy, Clone)]
pub(crate) struct Fruit { pub(crate) x: f32, pub(crate) ttl: f32, pub(crate) kind: usize }

/// A projectile in flight.
#[derive(Copy, Clone)]
pub(crate) struct Bolt { pub(crate) x: f32, pub(crate) y: f32, pub(crate) vx: f32, pub(crate) vy: f32, pub(crate) owner: usize, pub(crate) active: bool, pub(crate) damage: i32 }


/// Which screen the game is on.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum State { Title, Select, Vs, RoundIntro, Fight, RoundEnd, MatchEnd, Bonus, Continue, Challenger, Over, Won }

/// Who took a round.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum RoundResult { P1, P2, Draw }

/// One player against the ladder (score, difficulty curve) or two in a single
/// match with a winner; they share only the round.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum Mode { Solo, Versus }

/// The whole session: who is playing whom, how far up the ladder, and
/// everything on the stage.
pub(crate) struct Game {
    pub(crate) state: State,
    pub(crate) sess: Session,
    pub(crate) p: [Fighter; 2],
    pub(crate) bolts: [Bolt; 4],
    /// The attract mode: two CPU fighters playing a round behind the title's
    /// invitation. `idle` is how long the title has been left alone, and
    /// `demo_cpu` the first fighter's plan and its delay.
    pub(crate) demo: bool,
    pub(crate) idle: f32,
    pub(crate) demo_cpu: (CpuPlan, f32),
    /// Sounds asked for this frame, for the main loop to play.
    pub(crate) sounds: Vec<(Sfx, f32)>,
    pub(crate) helpers: [Option<Helper>; 6],
    pub(crate) barrels: [Barrel; BARRELS],
    /// How many barrels this bonus round has.
    pub(crate) bonus_total: usize,
    /// Counting down once the bonus round is decided, while its tally shows.
    pub(crate) bonus_done: f32,
    /// Screenshot scene 15: draw the kiosk card's poster instead of the game.
    pub(crate) poster: bool,
    pub(crate) mode: Mode,
    pub(crate) pick: usize,
    /// Player two's fighter, and whether each player has committed to
    /// their choice. Nobody fights until both have.
    pub(crate) pick2: usize,
    pub(crate) locked: [bool; 2],
    /// Which line of the title menu is highlighted.
    pub(crate) menu: usize,
    /// Which of the other two fighters this match is against, 0 then 1.
    pub(crate) opponent_index: usize,
    pub(crate) stage: usize,
    pub(crate) round: i32,
    pub(crate) clock: f32,
    pub(crate) phase: Timer,
    pub(crate) banner: &'static str,
    /// How many times this run has been continued, for the ending.
    pub(crate) continues: u32,
    /// How much easier the CPU has been made by continues: a player who
    /// keeps losing the same fight meets a slightly slower opponent each time.
    pub(crate) mercy: f32,
    /// Seconds of fade-in left: each new screen comes up out of black.
    pub(crate) fade: f32,
    /// Nobody has landed a blow yet this round.
    pub(crate) untouched: bool,
    /// The time and vitality bonus of the round just won, for the tally.
    pub(crate) bonus: [i32; 2],
    pub(crate) result: RoundResult,
    pub(crate) hitspark: [Spark; 4],
    pub(crate) shake: f32,
    /// Frames where everything stops on contact: it gives a hit weight, and
    /// it is when both players need to see who got hit with what.
    pub(crate) hitstop: f32,
    /// This round's opening bow has begun.
    pub(crate) bowed: bool,
    /// The last combo worth shouting about, and how long to shout it —
    /// a reward the player cannot see is not a reward.
    pub(crate) combo_shown: i32,
    pub(crate) combo_t: f32,
    /// Who landed it, so the number appears over the right shoulder.
    pub(crate) combo_side: usize,
    /// Rising as the ladder goes on: reaction time shortens and the
    /// reads get better. See cpu_think().
    pub(crate) difficulty: f32,
    pub(crate) cpu_delay: f32,
    pub(crate) cpu_plan: CpuPlan,
    /// Held so a punch and a kick pressed together read as one input
    /// rather than two — see read_special(). One per player: shared,
    /// each player's punch armed the other player's special.
    pub(crate) punch_at: [f32; 2],
    pub(crate) kick_at: [f32; 2],
    pub(crate) now: f32,
    /// Menu edge detection, per player and per direction. A menu step
    /// and a punch are not the same event, and sharing the slots made
    /// each one eat the other's.
    pub(crate) sel_held: [[bool; 2]; 2],
    pub(crate) sel_fire: [bool; 2],
    pub(crate) pops: [Pop; 4],
    pub(crate) fruit: Fruit,
    /// This round's fruit has not appeared yet.
    pub(crate) fruit_due: bool,
    /// Each health bar as it was a moment ago: the bar drains to the new
    /// value, so a player sees what a blow cost.
    pub(crate) ghost: [f32; 2],
    /// Where the floor was last shaken by a rage jump, and for how much longer
    /// the dust of it hangs.
    pub(crate) quake_x: f32,
    pub(crate) quake_t: f32,
    /// Real seconds of slow motion left on the finishing blow.
    pub(crate) slow: f32,
    /// The announcer's verdict on how the round was won: PERFECT, GREAT.
    pub(crate) call: &'static str,
}

/// How many fights a solo run is: everybody else, once.
pub(crate) const RUNGS: usize = FIGHTERS.len() - 1;
/// The difficulty dial for the first and the last opponent on the ladder.
pub(crate) const FIRST_RUNG: f32 = -0.45;
pub(crate) const LAST_RUNG: f32 = 0.8;

/// What the CPU has decided to do until it next thinks.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum CpuPlan { Wait, Approach, Retreat, Attack(MoveId), Jump, Block }

impl Game {
    /// A fresh game at the title screen.
    pub(crate) fn new() -> Self {
        Game {
            state: State::Title,
            sess: Session::new(1),
            p: [Fighter::new(0, 200.0, 1.0), Fighter::new(1, 440.0, -1.0)],
            bolts: [Bolt { x: 0.0, y: 0.0, vx: 0.0, vy: 0.0, owner: 0, active: false, damage: 0 }; 4],
            demo: false,
            idle: 0.0,
            demo_cpu: (CpuPlan::Wait, 0.0),
            sounds: Vec::new(),
            helpers: [None; 6],
            barrels: [Barrel { x: 0.0, y: FLOOR_Y, vx: 0.0, vy: 0.0, wait: 0.0, hp: 0, rolls: false,
                hit_t: 9.0, broke_t: -1.0 }; BARRELS],
            bonus_total: 0,
            bonus_done: 0.0,
            poster: false,
            mode: Mode::Solo,
            pick: 0,
            pick2: 1,
            locked: [false; 2],
            menu: 0,
            opponent_index: 0,
            stage: 0,
            round: 1,
            clock: ROUND_SECS,
            phase: Timer::default(),
            banner: "",
            bonus: [0, 0],
            untouched: true,
            fade: 0.0,
            mercy: 0.0,
            continues: 0,
            result: RoundResult::Draw,
            hitspark: [Spark { x: 0.0, y: 0.0, ttl: 0.0, life: 1.0, blocked: false }; 4],
            shake: 0.0,
            hitstop: 0.0,
            bowed: false,
            combo_shown: 0,
            combo_t: 0.0,
            combo_side: 0,
            difficulty: 0.0,
            cpu_delay: 0.0,
            cpu_plan: CpuPlan::Wait,
            punch_at: [-1.0; 2],
            kick_at: [-1.0; 2],
            now: 0.0,
            sel_held: [[false; 2]; 2],
            sel_fire: [false; 2],
            pops: [Pop { text: "", x: 0.0, y: 0.0, ttl: 0.0 }; 4],
            fruit: Fruit { x: 0.0, ttl: 0.0, kind: 0 },
            fruit_due: false,
            ghost: [0.0; 2],
            quake_x: 0.0,
            quake_t: 0.0,
            slow: 0.0,
            call: "",
        }
    }

    /// The fighter player `side` has the cursor on.
    pub(crate) fn picked(&self, who: usize) -> usize {
        if who == 0 { self.pick } else { self.pick2 }
    }
    /// Move player `who`'s cursor to fighter `v`.
    pub(crate) fn set_pick(&mut self, who: usize, v: usize) {
        if who == 0 { self.pick = v; } else { self.pick2 = v; }
    }
    /// Has the *other* player already locked this fighter? Two players
    /// cannot bring the same fighter to the same fight.
    pub(crate) fn taken_by_other(&self, who: usize, at: usize) -> bool {
        self.mode == Mode::Versus && self.locked[1 - who] && self.picked(1 - who) == at
    }

    /// The ladder: every other fighter, once. Round the roster from the pick,
    /// reordered so no two fights running are at the same home stage, with
    /// the giant and then the fighter nothing hurts kept for the end.
    pub(crate) fn ladder(&self) -> [usize; RUNGS] {
        // The fights that close the ladder, in order, and everyone before.
        fn place(ring: &[usize], ends: &[usize], out: &mut Vec<usize>) -> bool {
            if out.len() == ring.len() { return true; }
            let early = ring.len() - ends.len();
            for &who in ring {
                let fits = if out.len() < early { !ends.contains(&who) } else { ends[out.len() - early] == who };
                if !fits || out.contains(&who) { continue; }
                if out.last().is_some_and(|&prev| home_of(prev) == home_of(who)) { continue; }
                out.push(who);
                if place(ring, ends, out) { return true; }
                out.pop();
            }
            false
        }
        let n = FIGHTERS.len();
        let ring: Vec<usize> = (1..n).map(|k| (self.pick + k) % n).collect();
        let mut ends: Vec<usize> = ring.iter().copied()
            .filter(|&w| FIGHTERS[w].invincible || FIGHTERS[w].build == Build::Giant).collect();
        ends.sort_by_key(|&w| FIGHTERS[w].invincible);
        let mut out = Vec::with_capacity(RUNGS);
        if !place(&ring, &ends, &mut out) { out = ring; }
        std::array::from_fn(|i| out[i])
    }

    /// Two players, one match, no ladder and no difficulty dial.
    pub(crate) fn start_versus(&mut self) {
        self.mode = Mode::Versus;
        self.opponent_index = 0;
        // Player two is the challenger here, so the fight is at their home.
        self.stage = home_of(self.pick2);
        self.difficulty = 0.0;
        self.p[0] = Fighter::new(self.pick, 200.0, 1.0);
        self.p[1] = Fighter::new(self.pick2, 440.0, -1.0);
        self.round = 1;
        self.start_round();
    }

    /// The bonus round: the player alone at home with a row of barrels and
    /// a clock.
    pub(crate) fn start_bonus(&mut self) {
        self.stage = home_of(self.pick);
        self.p[0] = Fighter::new(self.pick, 90.0, 1.0);
        // Nobody to fight: the second fighter is only where the first one
        // looks, and is never drawn.
        self.p[1] = Fighter::new(self.pick, 0.0, -1.0);
        // The first bonus round is the row; the second is the fall.
        let falling = self.opponent_index != BONUS_AFTER[0];
        self.bonus_total = if falling { BARRELS } else { 5 };
        let none = Barrel { x: 0.0, y: FLOOR_Y, vx: 0.0, vy: 0.0, wait: 0.0, hp: 0, rolls: false,
            hit_t: 9.0, broke_t: -1.0 };
        for (i, b) in self.barrels.iter_mut().enumerate() {
            *b = if i >= self.bonus_total { none }
                else if falling {
                    Barrel { x: 90.0 + blip::scatter(i, 0xb0) * 460.0, y: -30.0, wait: 0.8 + i as f32 * 1.7,
                        hp: 1, rolls: true, ..none }
                } else {
                    Barrel { x: 200.0 + i as f32 * 85.0, hp: BARREL_HP, ..none }
                };
        }
        for b in self.bolts.iter_mut() { b.active = false; }
        for s in self.hitspark.iter_mut() { s.ttl = 0.0; }
        self.helpers = [None; 6];
        self.fruit.ttl = 0.0;
        self.hitstop = 0.0;
        self.slow = 0.0;
        self.clock = BONUS_SECS;
        self.bonus_done = 0.0;
        self.bonus = [0, 0];
        self.state = State::Bonus;
        self.fade = FADE;
    }

    /// The attract mode: a round between two fighters drawn by lot, on the
    /// second one's stage. Nothing is scored and nothing is spent.
    pub(crate) fn start_demo(&mut self) {
        let fair: Vec<usize> = (0..FIGHTERS.len()).filter(|&w| !FIGHTERS[w].invincible).collect();
        let a = fair[rand_int(0, fair.len() as i32 - 1) as usize];
        let b = fair[(fair.iter().position(|&w| w == a).unwrap()
            + rand_int(1, fair.len() as i32 - 1) as usize) % fair.len()];
        self.mode = Mode::Solo;
        self.demo = true;
        self.pick = a;
        self.stage = home_of(b);
        self.difficulty = 0.4;
        self.p = [Fighter::new(a, 200.0, 1.0), Fighter::new(b, 440.0, -1.0)];
        self.round = 1;
        self.start_round();
        self.fade = FADE;
    }

    /// Put the match just set up behind its billing: who, where, and how
    /// far up the ladder.
    pub(crate) fn announce(&mut self) {
        self.state = State::Vs;
        self.phase.start(VS_SECS);
        self.fade = FADE;
    }

    /// Set up fight `opponent_index` of the ladder: the opponent, their
    /// stage, the difficulty, round one.
    pub(crate) fn start_match(&mut self, opponent_index: usize) {
        self.opponent_index = opponent_index;
        let foe = self.ladder()[opponent_index];
        self.stage = home_of(foe);
        // Below zero the CPU also hesitates (see cpu_think): the first fights
        // are ones a newcomer can win. The dial climbs evenly to the last,
        // less a little for each continue.
        self.difficulty = FIRST_RUNG
            + (LAST_RUNG - FIRST_RUNG) * opponent_index as f32 / (RUNGS - 1) as f32
            - self.mercy;
        self.p[0] = Fighter::new(self.pick, 200.0, 1.0);
        self.p[1] = Fighter::new(foe, 440.0, -1.0);
        self.round = 1;
        self.start_round();
    }

    /// Both fighters back to their marks at full health, the stage cleared,
    /// the bow to come.
    pub(crate) fn start_round(&mut self) {
        let (a, b) = (self.p[0].who, self.p[1].who);
        let (r0, r1) = (self.p[0].rounds, self.p[1].rounds);
        self.p[0] = Fighter::new(a, 200.0, 1.0);
        self.p[1] = Fighter::new(b, 440.0, -1.0);
        self.p[0].rounds = r0;
        self.p[1].rounds = r1;
        for b in self.bolts.iter_mut() { b.active = false; }
        self.helpers = [None; 6];
        for s in self.hitspark.iter_mut() { s.ttl = 0.0; }
        for w in self.pops.iter_mut() { w.ttl = 0.0; }
        self.fruit.ttl = 0.0;
        self.fruit_due = true;
        self.untouched = true;
        self.ghost = [self.p[0].health as f32, self.p[1].health as f32];
        self.slow = 0.0;
        self.quake_t = 0.0;
        self.call = "";
        self.hitstop = 0.0;
        self.clock = ROUND_SECS;
        self.banner = "ROUND";
        self.state = State::RoundIntro;
        self.phase.start(ROUND_INTRO);
        self.bowed = false;
        self.cpu_plan = CpuPlan::Wait;
        self.cpu_delay = 0.4;
    }

    /// The other turtles come in from behind whoever called, one after
    /// another: three of them, or two when the fourth is the opponent. Each
    /// side has three slots of its own, and the sweep always comes last.
    pub(crate) fn call_turtles(&mut self, side: usize) {
        let (caller, foe) = (self.p[side], self.p[1 - side].who);
        let from = if caller.facing > 0.0 { -30.0 } else { WIN_W as f32 + 30.0 };
        let others: Vec<usize> = (0..FIGHTERS.len())
            .filter(|&w| FIGHTERS[w].build == Build::Turtle && w != caller.who && w != foe)
            .take(3).collect();
        let first = HELPER_MOVES.len() - others.len();
        for (k, &who) in others.iter().enumerate() {
            self.helpers[side * 3 + k] = Some(Helper {
                f: Fighter::new(who, from, caller.facing),
                side, wait: k as f32 * HELPER_GAP, mv: HELPER_MOVES[first + k], struck: false,
            });
        }
    }

    /// Launch `owner`'s projectile from where their special puts it.
    pub(crate) fn spawn_bolt(&mut self, owner: usize, damage: i32) {
        let foe = self.p[1 - owner];
        let laser = self.p[owner].arch().special == Special::LaserVision;
        // From the air the laser is aimed, so he turns to look first: at the
        // spot the stare fixed on, or failing that at the opponent.
        let (ax, ay) = self.p[owner].aim.unwrap_or((foe.x, foe.y - foe.height() * 0.5));
        if laser && self.p[owner].airborne() {
            self.p[owner].facing = if ax >= self.p[owner].x { 1.0 } else { -1.0 };
        }
        let f = self.p[owner];
        let x = f.x + f.facing * (f.width() / 2.0 + 10.0);
        // A bolt leaves the hands at chest height. The laser leaves the eyes,
        // twice as fast: level at the head in front of it from the ground (a
        // crouch goes under either), straight at its aim from the air.
        let (y, vx, vy) = if !laser {
            (f.y - 54.0 * f.size(), f.facing * 300.0, 0.0)
        } else if !f.airborne() {
            (f.y - 98.0 * f.size().min(f.foe_size), f.facing * LASER_SPEED, 0.0)
        } else {
            let y = f.y - 100.0 * f.size();
            let (dx, dy) = (ax - x, ay - y);
            let d = dx.hypot(dy).max(1.0);
            (y, dx / d * LASER_SPEED, dy / d * LASER_SPEED)
        };
        if let Some(b) = self.bolts.iter_mut().find(|b| !b.active) {
            *b = Bolt { x, y, vx, vy, owner, active: true, damage };
        }
    }

    /// How far the picture is thrown down this frame by the last heavy blow:
    /// a few pixels, dying away as it rings.
    pub(crate) fn shake_px(&self) -> f32 {
        if self.shake > 0.0 { (self.shake * 60.0).sin() * self.shake * 30.0 } else { 0.0 }
    }

    /// Ask for a sound.
    pub(crate) fn snd(&mut self, s: Sfx) { self.sounds.push((s, 1.0)); }
    /// Ask for a sound at a volume.
    pub(crate) fn snd_at(&mut self, s: Sfx, volume: f32) { self.sounds.push((s, volume)); }

    /// A word over (`x`, `y`) for a moment.
    pub(crate) fn pop(&mut self, text: &'static str, x: f32, y: f32) {
        let fresh = Pop { text, x, y, ttl: POP_SECS };
        let slot = self.pops.iter().position(|w| w.ttl <= 0.0).unwrap_or(0);
        self.pops[slot] = fresh;
    }

    /// A hit splash at (`x`, `y`): `big` for a knockdown, `blocked` for a
    /// guard.
    pub(crate) fn spark(&mut self, x: f32, y: f32, big: bool, blocked: bool) {
        let life = if big { 0.26 } else { 0.17 };
        let fresh = Spark { x, y, ttl: life, life, blocked };
        for s in self.hitspark.iter_mut() {
            if s.ttl <= 0.0 { *s = fresh; return; }
        }
        self.hitspark[0] = fresh;
    }
}
