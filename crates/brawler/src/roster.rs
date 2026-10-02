//! The roster and the stages: who the ten fighters are, what each is built
//! like and can do, and where each is at home.

/// What punch and kick together does.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum Special { ChiBolt, BullRush, TalonKick, LaserVision }

/// What a fighter wears and how they are built, kept beside the numbers so
/// looks and moves stay one roster.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum Build {
    /// Karate gi: jacket to the elbows, trousers to mid-shin, bare feet.
    Gi,
    /// Stripped to the waist, heavy boots.
    Bare,
    /// A one-piece flight suit, wrapped forearms and shins.
    Suit,
    /// Shell on the back, plastron in front, a mask and pads in `trim`.
    Turtle,
    /// Masked head to toe: `cloth` limbs, `trim` body, head, gloves and boots.
    Spider,
    /// Stripped to the waist and barefoot, trousers torn off at the knee.
    Giant,
    /// A `cloth` suit with a cape and boots in `trim`.
    Caped,
}

/// A fighter's identity, in the only terms that change how they play.
#[derive(Copy, Clone)]
pub(crate) struct Archetype {
    pub(crate) name: &'static str,
    pub(crate) health: i32,
    pub(crate) walk: f32,
    pub(crate) back_walk: f32,
    pub(crate) jump_scale: f32,
    /// Scales every attack's damage and reach: the heavies hit hardest from
    /// closest, the light fighters poke from outside.
    pub(crate) power: f32,
    pub(crate) reach: f32,
    pub(crate) special: Special,
    pub(crate) special_name: &'static str,
    pub(crate) color: (f32, f32, f32),
    pub(crate) trim: (f32, f32, f32),
    pub(crate) skin: (f32, f32, f32),
    pub(crate) hair: (f32, f32, f32),
    pub(crate) build: Build,
    /// Limb and torso thickness, around 1.0: where a heavyweight looks like
    /// one without being any bigger.
    pub(crate) bulk: f32,
    /// The whole fighter's scale: body, hurtbox and reach together, so a
    /// small fighter is hard to hit and has to get close.
    pub(crate) size: f32,
    /// Blows land on them and move them, and cost them nothing, unless
    /// thrown by someone bigger (see `Fighter::hurt`).
    pub(crate) invincible: bool,
}

pub(crate) const FIGHTERS: [Archetype; 10] = [
    Archetype {
        name: "RYUKA", health: 100, walk: 132.0, back_walk: 108.0, jump_scale: 1.0,
        power: 1.0, reach: 1.0, special: Special::ChiBolt, special_name: "CHI BOLT",
        color: (0.92, 0.92, 0.96), trim: (0.85, 0.25, 0.25),
        skin: (0.85, 0.68, 0.52), hair: (0.24, 0.16, 0.12), build: Build::Gi, bulk: 1.0, size: 1.0, invincible: false,
    },
    Archetype {
        name: "BRUTUS", health: 120, walk: 96.0, back_walk: 78.0, jump_scale: 0.88,
        power: 1.28, reach: 0.88, special: Special::BullRush, special_name: "BULL RUSH",
        color: (0.30, 0.20, 0.26), trim: (0.95, 0.75, 0.2),
        skin: (0.74, 0.50, 0.34), hair: (0.20, 0.14, 0.12), build: Build::Bare, bulk: 1.28, size: 1.0, invincible: false,
    },
    Archetype {
        name: "KESTREL", health: 108, walk: 164.0, back_walk: 140.0, jump_scale: 1.12,
        power: 0.96, reach: 1.12, special: Special::TalonKick, special_name: "TALON KICK",
        color: (0.25, 0.65, 0.85), trim: (0.95, 0.95, 0.35),
        skin: (0.90, 0.74, 0.60), hair: (0.55, 0.42, 0.18), build: Build::Suit, bulk: 0.86, size: 1.0, invincible: false,
    },
    // Four turtles, told apart by the mask: `color` is the shell.
    Archetype {
        name: "GIOTTO", health: 88, walk: 220.0, back_walk: 180.0, jump_scale: 1.30,
        power: 1.00, reach: 1.12, special: Special::ChiBolt, special_name: "SHURIKEN",
        color: TURTLE_SHELL, trim: (0.25, 0.50, 0.95),
        skin: TURTLE_SKIN, hair: TURTLE_SHELL, build: Build::Turtle, bulk: 1.14, size: 0.5, invincible: false,
    },
    Archetype {
        name: "TITIAN", health: 96, walk: 200.0, back_walk: 164.0, jump_scale: 1.25,
        power: 1.10, reach: 1.06, special: Special::BullRush, special_name: "SHELL RAM",
        color: TURTLE_SHELL, trim: (0.90, 0.22, 0.20),
        skin: TURTLE_SKIN, hair: TURTLE_SHELL, build: Build::Turtle, bulk: 1.22, size: 0.5, invincible: false,
    },
    Archetype {
        name: "VERMEER", health: 94, walk: 235.0, back_walk: 192.0, jump_scale: 1.32,
        power: 1.05, reach: 1.15, special: Special::TalonKick, special_name: "RISING KICK",
        color: TURTLE_SHELL, trim: (0.62, 0.36, 0.85),
        skin: TURTLE_SKIN, hair: TURTLE_SHELL, build: Build::Turtle, bulk: 1.08, size: 0.5, invincible: false,
    },
    Archetype {
        name: "BOSCH", health: 86, walk: 250.0, back_walk: 205.0, jump_scale: 1.38,
        power: 0.92, reach: 1.12, special: Special::ChiBolt, special_name: "PIZZA TOSS",
        color: TURTLE_SHELL, trim: (0.98, 0.58, 0.14),
        skin: TURTLE_SKIN, hair: TURTLE_SHELL, build: Build::Turtle, bulk: 1.12, size: 0.5, invincible: false,
    },
    Archetype {
        name: "WEBBER", health: 90, walk: 300.0, back_walk: 250.0, jump_scale: 1.20,
        power: 0.92, reach: 1.08, special: Special::ChiBolt, special_name: "WEB SHOT",
        color: (0.20, 0.32, 0.78), trim: (0.86, 0.14, 0.16),
        skin: (0.86, 0.14, 0.16), hair: (0.10, 0.10, 0.12), build: Build::Spider, bulk: 0.88, size: 1.0, invincible: false,
    },
    Archetype {
        name: "GAMMA", health: 92, walk: 84.0, back_walk: 70.0, jump_scale: 0.90,
        power: 1.80, reach: 0.82, special: Special::BullRush, special_name: "RAMPAGE",
        color: (0.46, 0.28, 0.62), trim: (0.72, 0.52, 0.92),
        skin: (0.38, 0.64, 0.26), hair: (0.10, 0.10, 0.12), build: Build::Giant, bulk: 1.15, size: 1.5, invincible: false,
    },
    // The man of steel: only the giant can hurt him, he flies instead of
    // jumping, his punch hits three times as hard and his kick is the laser,
    // which kills. He is the last fight of every ladder (see `round_result`).
    Archetype {
        name: "ZENITH", health: 200, walk: 330.0, back_walk: 275.0, jump_scale: 1.0,
        power: 3.0, reach: 1.0, special: Special::LaserVision, special_name: "LASER VISION",
        color: (0.16, 0.36, 0.84), trim: (0.86, 0.14, 0.16),
        skin: (0.90, 0.72, 0.58), hair: (0.08, 0.08, 0.10), build: Build::Caped, bulk: 1.12,
        size: 1.0, invincible: true,
    },
];

/// The stages. Scenery only: the same floor line, height and walls in all.
pub(crate) const STAGES: usize = 7;
pub(crate) const STAGE_NAMES: [&str; STAGES] =
    ["THE DOCKS", "MOONLIT TEMPLE", "THE AIR BASE", "THE BATH HOUSE", "RIVER VILLAGE",
     "CRYSTAL FORTRESS", "QUEENS ROOFTOP"];

/// Where a fighter is at home. As in Street Fighter II, a fight is held at
/// the opponent's; a ladder never meets two of a kind in a row, so it never
/// visits one stage twice.
pub(crate) fn home_of(who: usize) -> usize {
    match FIGHTERS[who].build {
        Build::Bare => 0,
        Build::Gi => 1,
        Build::Suit => 2,
        Build::Turtle => 3,
        Build::Giant => 4,
        Build::Caped => 5,
        Build::Spider => 6,
    }
}

pub(crate) const TURTLE_SKIN: (f32, f32, f32) = (0.44, 0.68, 0.30);
pub(crate) const TURTLE_SHELL: (f32, f32, f32) = (0.44, 0.31, 0.17);
