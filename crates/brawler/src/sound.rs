//! Effects are generated at build time; crowd and ambience are synthesized at startup.

use crate::roster::{Build, FIGHTERS};

/// A sound the rules ask for. They only name it and the main loop plays it,
/// so a round can be run, and tested, with no audio device.
#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) enum Sfx {
    /// One of the three light hits: the step of the combo it lands in.
    HitLight(usize),
    HitHeavy, Crunch, Land, Cheer, Roar, Whoosh, Gi, Block, Bell, Ko, Projectile, Laser, Quake,
    Parry, Counter, Fruit, Tick, Gong, Whistle, Thwip, Drums, Smash, Tweet, Bellow, Swing, Win, Lose,
    /// The Colosseum's kitten, at a knockout.
    Meow,
    /// A kick landing, light or heavy: shin and instep, not knuckles.
    HitKick(bool),
    /// A foot set down on the walk.
    Step,
    /// A fighter's own voice (`throat_of`): taking a blow (heavy or not), shouting behind one, going down.
    Hurt(usize, bool), Kiai(usize), Cry(usize),
}

/// Which voice a fighter has (an index into `tunes::THROATS`): a man, a deep voice, a woman, a turtle, DRAKE, LANCER.
pub(crate) fn throat_of(who: usize) -> usize {
    match FIGHTERS[who].name {
        "KESTREL" | "JADE" | "KITE" | "AURELIA" => 2,
        "BRUTUS" | "GAMMA" | "REGENT" => 1,
        "DRAKE" => 4,
        "LANCER" => 5,
        _ if FIGHTERS[who].build == Build::Turtle => 3,
        _ => 0,
    }
}

/// A different number each call, the same series each run: which take plays, and how loud, without drawing on
/// the game's random numbers (a bot run must repeat).
fn shuffle(n: &std::cell::Cell<u32>) -> u32 {
    let v = n.get().wrapping_add(0x9E37_79B9);
    n.set(v);
    let x = v ^ (v >> 15);
    x.wrapping_mul(0x2C1B_3C6D) >> 8
}

blip::sounds!(
    /// Every effect, loaded once at startup.
    pub(crate) Sounds {
        /// Three light hits at rising pitch. A combo picks the next one up,
        /// so a chain sounds like it is climbing rather than like the same
        /// hit played four times.
        hit_light: [blip::BlipSound; 3] = [blip::sound!("hit_light"), blip::sound!("hit_light2"), blip::sound!("hit_light3")],
        hit_heavy,
        /// A second take of each of those four, played turn about with the
        /// first.
        hit_again: [blip::BlipSound; 4] = [
            blip::sound!("hit_light_b"), blip::sound!("hit_light2_b"),
            blip::sound!("hit_light3_b"), blip::sound!("hit_heavy_b"),
        ],
        take: std::cell::Cell<bool> = std::cell::Cell::new(false),
        kick: [blip::BlipSound; 4] = [
            blip::sound!("kick_light"), blip::sound!("kick_light_b"),
            blip::sound!("kick_heavy"), blip::sound!("kick_heavy_b"),
        ],
        /// Per throat: two grunts and a cry; three shouts; a knockout.
        hurt: [[blip::BlipSound; 3]; 6] = [
            [blip::sound!("hurt0_0"), blip::sound!("hurt0_1"), blip::sound!("hurt0_2")],
            [blip::sound!("hurt1_0"), blip::sound!("hurt1_1"), blip::sound!("hurt1_2")],
            [blip::sound!("hurt2_0"), blip::sound!("hurt2_1"), blip::sound!("hurt2_2")],
            [blip::sound!("hurt3_0"), blip::sound!("hurt3_1"), blip::sound!("hurt3_2")],
            [blip::sound!("hurt4_0"), blip::sound!("hurt4_1"), blip::sound!("hurt4_2")],
            [blip::sound!("hurt5_0"), blip::sound!("hurt5_1"), blip::sound!("hurt5_2")],
        ],
        kiai: [[blip::BlipSound; 3]; 6] = [
            [blip::sound!("kiai0_0"), blip::sound!("kiai0_1"), blip::sound!("kiai0_2")],
            [blip::sound!("kiai1_0"), blip::sound!("kiai1_1"), blip::sound!("kiai1_2")],
            [blip::sound!("kiai2_0"), blip::sound!("kiai2_1"), blip::sound!("kiai2_2")],
            [blip::sound!("kiai3_0"), blip::sound!("kiai3_1"), blip::sound!("kiai3_2")],
            [blip::sound!("kiai4_0"), blip::sound!("kiai4_1"), blip::sound!("kiai4_2")],
            [blip::sound!("kiai5_0"), blip::sound!("kiai5_1"), blip::sound!("kiai5_2")],
        ],
        cry: [blip::BlipSound; 6] = [blip::sound!("cry0"), blip::sound!("cry1"), blip::sound!("cry2"), blip::sound!("cry3"),
            blip::sound!("cry4"), blip::sound!("cry5")],
        dice: std::cell::Cell<u32> = std::cell::Cell::new(0),
        block_again: blip::BlipSound = blip::sound!("block_b"),
        step: [blip::BlipSound; 2] = [blip::sound!("step"), blip::sound!("step_b")],
        /// A body hitting boards: the biggest thing in a round, with its own
        /// sound.
        crunch,
        land,
        /// The crowd that has been drawn watching every round in silence.
        cheer,
        roar,
        whoosh,
        /// Cloth snapping taut, played on the frame a leg unfolds.
        gi,
        block,
        bell,
        ko,
        projectile,
        laser,
        /// The giant coming down.
        quake,
        /// A guard impact: nothing else in the fight rings.
        parry,
        counter,
        fruit,
        /// The menus: a wood block for a step, a gong for a choice.
        tick,
        gong,
        whistle,
        thwip,
        /// The round called, a barrel broken, a head full of birds.
        drums,
        smash,
        tweet,
        /// Looped while a fighter is nearly out.
        heart,
        /// What the stage sounds like under the fight: a crowd, wind, the
        /// crowd with crickets, the crowd over a city (see `ambience_wav`).
        ambience: [blip::BlipSound; 4] = [
            blip::sound!("ambience_crowd"), blip::sound!("ambience_wind"),
            blip::sound!("ambience_crickets"), blip::sound!("ambience_city"),
        ],
        /// The giant's rage jump, and any blow with a whole body behind it.
        bellow,
        swing,
        /// The match decided: played alone, the stage music stopped under it.
        win,
        lose,
        meow,
    }
);

impl Sounds {
    /// Play one effect at `volume` (1.0 is as recorded).
    pub(crate) fn play(&self, s: Sfx, volume: f32) {
        let roll = shuffle(&self.dice);
        let sound = match s {
            Sfx::Step => &self.step[(roll & 1) as usize],
            Sfx::Block if roll & 2 != 0 => &self.block_again,
            Sfx::HitKick(heavy) => &self.kick[heavy as usize * 2 + (roll & 1) as usize],
            Sfx::Hurt(who, heavy) => &self.hurt[who.min(5)][if heavy { 2 } else { (roll & 1) as usize }],
            Sfx::Kiai(who) => &self.kiai[who.min(5)][(roll % 3) as usize],
            Sfx::Cry(who) => &self.cry[who.min(5)],
            Sfx::HitLight(_) | Sfx::HitHeavy => {
                let second = self.take.replace(!self.take.get());
                match (s, second) {
                    (Sfx::HitLight(step), false) => &self.hit_light[step.min(2)],
                    (Sfx::HitLight(step), true) => &self.hit_again[step.min(2)],
                    (_, false) => &self.hit_heavy,
                    (_, true) => &self.hit_again[3],
                }
            }
            Sfx::Crunch => &self.crunch,
            Sfx::Land => &self.land,
            Sfx::Cheer => &self.cheer,
            Sfx::Roar => &self.roar,
            Sfx::Whoosh => &self.whoosh,
            Sfx::Gi => &self.gi,
            Sfx::Block => &self.block,
            Sfx::Bell => &self.bell,
            Sfx::Ko => &self.ko,
            Sfx::Projectile => &self.projectile,
            Sfx::Laser => &self.laser,
            Sfx::Quake => &self.quake,
            Sfx::Parry => &self.parry,
            Sfx::Counter => &self.counter,
            Sfx::Fruit => &self.fruit,
            Sfx::Tick => &self.tick,
            Sfx::Gong => &self.gong,
            Sfx::Whistle => &self.whistle,
            Sfx::Thwip => &self.thwip,
            Sfx::Drums => &self.drums,
            Sfx::Smash => &self.smash,
            Sfx::Tweet => &self.tweet,
            Sfx::Bellow => &self.bellow,
            Sfx::Swing => &self.swing,
            Sfx::Win => &self.win,
            Sfx::Lose => &self.lose,
            Sfx::Meow => &self.meow,
        };
        // No two blows land alike: up to an eighth either way.
        let swing = matches!(s, Sfx::HitLight(_) | Sfx::HitHeavy | Sfx::HitKick(_) | Sfx::Hurt(..) | Sfx::Kiai(_)
            | Sfx::Block | Sfx::Land | Sfx::Step);
        let loud = if swing { 0.88 + 0.24 * ((roll >> 4) % 256) as f32 / 255.0 } else { 1.0 };
        blip::audio::play_sfx_volume(sound, volume * loud);
    }
}
