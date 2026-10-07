//! Effects are generated at build time; crowd and ambience are synthesized at startup.

use blip::audio::load_sound as load;

/// One of the build-time effects, by file name.
macro_rules! wav {
    ($name:literal) => {
        include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/", $name, ".wav"))
    };
}

/// A sound the rules ask for. They only name it and the main loop plays it,
/// so a round can be run, and tested, with no audio device.
#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) enum Sfx {
    /// One of the three light hits: the step of the combo it lands in.
    HitLight(usize),
    HitHeavy, Crunch, Land, Cheer, Roar, Whoosh, Gi, Block, Bell, Ko, Projectile, Laser, Quake,
    Parry, Counter, Fruit, Tick, Gong, Whistle, Thwip, Drums, Smash, Tweet, Bellow, Swing, Win, Lose,
}

/// Every effect, loaded once at startup.
pub(crate) struct Sounds {
    /// Three light hits at rising pitch. A combo picks the next one up,
    /// so a chain sounds like it is climbing rather than like the same
    /// hit played four times.
    pub(crate) hit_light: [blip::BlipSound; 3],
    pub(crate) hit_heavy: blip::BlipSound,
    /// A second take of each of those four, played turn about with the
    /// first.
    pub(crate) hit_again: [blip::BlipSound; 4],
    pub(crate) take: std::cell::Cell<bool>,
    /// A body hitting boards: the biggest thing in a round, with its own
    /// sound.
    pub(crate) crunch: blip::BlipSound,
    pub(crate) land: blip::BlipSound,
    /// The crowd that has been drawn watching every round in silence.
    pub(crate) cheer: blip::BlipSound,
    pub(crate) roar: blip::BlipSound,
    pub(crate) whoosh: blip::BlipSound,
    /// Cloth snapping taut, played on the frame a leg unfolds.
    pub(crate) gi: blip::BlipSound,
    pub(crate) block: blip::BlipSound,
    pub(crate) bell: blip::BlipSound,
    pub(crate) ko: blip::BlipSound,
    pub(crate) projectile: blip::BlipSound,
    pub(crate) laser: blip::BlipSound,
    /// The giant coming down.
    pub(crate) quake: blip::BlipSound,
    /// A guard impact: nothing else in the fight rings.
    pub(crate) parry: blip::BlipSound,
    pub(crate) counter: blip::BlipSound,
    pub(crate) fruit: blip::BlipSound,
    /// The menus: a wood block for a step, a gong for a choice.
    pub(crate) tick: blip::BlipSound,
    pub(crate) gong: blip::BlipSound,
    pub(crate) whistle: blip::BlipSound,
    pub(crate) thwip: blip::BlipSound,
    /// The round called, a barrel broken, a head full of birds.
    pub(crate) drums: blip::BlipSound,
    pub(crate) smash: blip::BlipSound,
    pub(crate) tweet: blip::BlipSound,
    /// Looped while a fighter is nearly out.
    pub(crate) heart: blip::BlipSound,
    /// What the stage sounds like under the fight: a crowd, wind, the
    /// crowd with crickets, the crowd over a city (see `ambience_wav`).
    pub(crate) ambience: [blip::BlipSound; 4],
    /// The giant's rage jump, and any blow with a whole body behind it.
    pub(crate) bellow: blip::BlipSound,
    pub(crate) swing: blip::BlipSound,
    /// The match decided: played alone, the stage music stopped under it.
    pub(crate) win: blip::BlipSound,
    pub(crate) lose: blip::BlipSound,
}

impl Sounds {
    pub(crate) async fn load() -> Sounds {
        Sounds {
            hit_light: [
                load(wav!("hit_light")).await,
                load(wav!("hit_light2")).await,
                load(wav!("hit_light3")).await,
            ],
            hit_heavy: load(wav!("hit_heavy")).await,
            hit_again: [
                load(wav!("hit_light_b")).await,
                load(wav!("hit_light2_b")).await,
                load(wav!("hit_light3_b")).await,
                load(wav!("hit_heavy_b")).await,
            ],
            take: std::cell::Cell::new(false),
            crunch: load(wav!("crunch")).await,
            land: load(wav!("land")).await,
            // Built here rather than shipped, like the music: see
            // blip_assets::brawler::crowd_wav.
            cheer: load(&blip_assets::brawler::crowd_wav(false)).await,
            roar: load(&blip_assets::brawler::crowd_wav(true)).await,
            whoosh: load(wav!("whoosh")).await,
            gi: load(wav!("gi")).await,
            block: load(wav!("block")).await,
            bell: load(wav!("bell")).await,
            ko: load(wav!("ko")).await,
            projectile: load(wav!("projectile")).await,
            laser: load(wav!("laser")).await,
            quake: load(wav!("quake")).await,
            parry: load(wav!("parry")).await,
            counter: load(wav!("counter")).await,
            fruit: load(wav!("fruit")).await,
            tick: load(wav!("tick")).await,
            gong: load(wav!("gong")).await,
            whistle: load(wav!("whistle")).await,
            drums: load(wav!("drums")).await,
            smash: load(wav!("smash")).await,
            tweet: load(wav!("tweet")).await,
            heart: load(wav!("heart")).await,
            ambience: [
                load(&blip_assets::brawler::ambience_wav(0)).await,
                load(&blip_assets::brawler::ambience_wav(1)).await,
            load(&blip_assets::brawler::ambience_wav(2)).await,
            load(&blip_assets::brawler::ambience_wav(3)).await,
            ],
            bellow: load(wav!("bellow")).await,
            swing: load(wav!("swing")).await,
            thwip: load(wav!("thwip")).await,
            win: load(wav!("win")).await,
            lose: load(wav!("lose")).await,
        }
    }

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
