//! Effects are generated at build time; crowd and ambience are synthesized at startup.


/// A sound the rules ask for. They only name it and the main loop plays it,
/// so a round can be run, and tested, with no audio device.
#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) enum Sfx {
    /// One of the three light hits: the step of the combo it lands in.
    HitLight(usize),
    HitHeavy, Crunch, Land, Cheer, Roar, Whoosh, Gi, Block, Bell, Ko, Projectile, Laser, Quake,
    Parry, Counter, Fruit, Tick, Gong, Whistle, Thwip, Drums, Smash, Tweet, Bellow, Swing, Win, Lose,
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
    }
);

impl Sounds {
    /// Play one effect at `volume` (1.0 is as recorded).
    pub(crate) fn play(&self, s: Sfx, volume: f32) {
        let sound = match s {
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
        };
        blip::audio::play_sfx_volume(sound, volume);
    }
}
