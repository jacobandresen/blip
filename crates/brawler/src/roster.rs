//! The roster and the stages: who the fighters are, what each is built
//! like and can do, and where each is at home.

/// What punch and kick together does.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum Special { ChiBolt, BullRush, TalonKick, LaserVision, DriveBy }

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
    /// Split blue tunic, bare thighs, tall boots and ribboned hair buns.
    Qipao,
    /// Peaked cap, armoured shoulders and a military tunic.
    Uniform,
    /// A fitted commando suit, beret, gauntlets and twin braids.
    Commando,
    /// A heavyweight in a bronze cuirass, bare arms and greaves.
    Gladiator,
    /// Streetwear, bandana and gold chain; built from the same anatomy rig.
    Ganger,
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
    pub(crate) power_desc: &'static str,
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
    /// Ladder-ending boss with deliberately extreme stats.
    pub(crate) boss: bool,
}

pub(crate) const FIGHTERS: [Archetype; 15] = [
    Archetype {
        name: "RYUKA", health: 100, walk: 132.0, back_walk: 108.0, jump_scale: 1.0,
        power: 1.0, reach: 1.0, special: Special::ChiBolt, special_name: "CHI BOLT", power_desc: "RANGED CHI SHOT",
        color: (0.92, 0.92, 0.96), trim: (0.85, 0.25, 0.25),
        skin: (0.85, 0.68, 0.52), hair: (0.24, 0.16, 0.12), build: Build::Gi, bulk: 1.0, size: 1.0, boss: false,
    },
    Archetype {
        name: "BRUTUS", health: 120, walk: 96.0, back_walk: 78.0, jump_scale: 0.88,
        power: 1.28, reach: 0.88, special: Special::BullRush, special_name: "BULL RUSH",
        power_desc: "HEAVY BULL CHARGE",
        color: (0.30, 0.20, 0.26), trim: (0.95, 0.75, 0.2),
        skin: (0.74, 0.50, 0.34), hair: (0.20, 0.14, 0.12), build: Build::Bare, bulk: 1.28, size: 1.0, boss: false,
    },
    Archetype {
        name: "KESTREL", health: 108, walk: 164.0, back_walk: 140.0, jump_scale: 1.12,
        power: 0.96, reach: 1.12, special: Special::TalonKick, special_name: "TALON KICK",
        power_desc: "LONG-RANGE KICK",
        color: (0.25, 0.65, 0.85), trim: (0.95, 0.95, 0.35),
        skin: (0.90, 0.74, 0.60), hair: (0.55, 0.42, 0.18), build: Build::Suit, bulk: 0.86, size: 1.0, boss: false,
    },
    // Four turtles, told apart by the mask: `color` is the shell.
    Archetype {
        name: "GIOTTO", health: 88, walk: 220.0, back_walk: 180.0, jump_scale: 1.30,
        power: 1.00, reach: 1.12, special: Special::ChiBolt, special_name: "SHURIKEN",
        power_desc: "CALLS THE TURTLE TEAM",
        color: TURTLE_SHELL, trim: (0.25, 0.50, 0.95),
        skin: TURTLE_SKIN, hair: TURTLE_SHELL, build: Build::Turtle, bulk: 1.14, size: 0.5, boss: false,
    },
    Archetype {
        name: "TITIAN", health: 96, walk: 200.0, back_walk: 164.0, jump_scale: 1.25,
        power: 1.10, reach: 1.06, special: Special::BullRush, special_name: "SHELL RAM",
        power_desc: "HEAVY SHELL CHARGE",
        color: TURTLE_SHELL, trim: (0.90, 0.22, 0.20),
        skin: TURTLE_SKIN, hair: TURTLE_SHELL, build: Build::Turtle, bulk: 1.22, size: 0.5, boss: false,
    },
    Archetype {
        name: "VERMEER", health: 94, walk: 235.0, back_walk: 192.0, jump_scale: 1.32,
        power: 1.05, reach: 1.15, special: Special::TalonKick, special_name: "RISING KICK",
        power_desc: "RISING STRIKE",
        color: TURTLE_SHELL, trim: (0.62, 0.36, 0.85),
        skin: TURTLE_SKIN, hair: TURTLE_SHELL, build: Build::Turtle, bulk: 1.08, size: 0.5, boss: false,
    },
    Archetype {
        name: "BOSCH", health: 86, walk: 250.0, back_walk: 205.0, jump_scale: 1.38,
        power: 0.92, reach: 1.12, special: Special::ChiBolt, special_name: "PIZZA TOSS",
        power_desc: "SPINNING PROJECTILE",
        color: TURTLE_SHELL, trim: (0.98, 0.58, 0.14),
        skin: TURTLE_SKIN, hair: TURTLE_SHELL, build: Build::Turtle, bulk: 1.12, size: 0.5, boss: false,
    },
    Archetype {
        name: "WEBBER", health: 90, walk: 300.0, back_walk: 250.0, jump_scale: 1.20,
        power: 0.92, reach: 1.08, special: Special::ChiBolt, special_name: "WEB SHOT",
        power_desc: "SNARES THE OPPONENT",
        color: (0.20, 0.32, 0.78), trim: (0.86, 0.14, 0.16),
        skin: (0.86, 0.14, 0.16), hair: (0.10, 0.10, 0.12), build: Build::Spider, bulk: 0.88, size: 1.0, boss: false,
    },
    Archetype {
        name: "GAMMA", health: 92, walk: 84.0, back_walk: 70.0, jump_scale: 0.90,
        power: 1.80, reach: 0.82, special: Special::BullRush, special_name: "RAMPAGE",
        power_desc: "GIANT-SCALE DAMAGE",
        color: (0.46, 0.28, 0.62), trim: (0.72, 0.52, 0.92),
        skin: (0.38, 0.64, 0.26), hair: (0.10, 0.10, 0.12), build: Build::Giant, bulk: 1.15, size: 1.5, boss: false,
    },
    // The OVERPOWERED ladder boss: 200 health, triple damage, and a laser.
    // Everyone can hurt him; his stats make him hard, not immune.
    Archetype {
        name: "ZENITH", health: 200, walk: 330.0, back_walk: 275.0, jump_scale: 1.0,
        power: 3.0, reach: 1.0, special: Special::LaserVision, special_name: "LASER VISION",
        power_desc: "OVERPOWERED",
        color: (0.16, 0.36, 0.84), trim: (0.86, 0.14, 0.16),
        skin: (0.90, 0.72, 0.58), hair: (0.08, 0.08, 0.10), build: Build::Caped, bulk: 1.12,
        size: 1.0, boss: true,
    },
    Archetype {
        name: "JADE", health: 96, walk: 174.0, back_walk: 146.0, jump_scale: 1.18,
        power: 0.94, reach: 1.14, special: Special::TalonKick, special_name: "LOTUS KICK",
        power_desc: "RISING LOTUS STRIKE",
        color: (0.10, 0.38, 0.88), trim: (0.98, 0.86, 0.42),
        skin: (0.88, 0.68, 0.52), hair: (0.08, 0.07, 0.10), build: Build::Qipao,
        bulk: 0.94, size: 1.0, boss: false,
    },
    Archetype {
        name: "REGENT", health: 116, walk: 112.0, back_walk: 92.0, jump_scale: 0.96,
        power: 1.16, reach: 1.02, special: Special::BullRush, special_name: "VIOLET CRUSH",
        power_desc: "ARMOURED CHARGE",
        color: (0.78, 0.12, 0.18), trim: (0.72, 0.74, 0.84),
        skin: (0.76, 0.58, 0.46), hair: (0.10, 0.08, 0.12), build: Build::Uniform,
        bulk: 1.20, size: 1.0, boss: false,
    },
    Archetype {
        name: "KITE", health: 98, walk: 202.0, back_walk: 168.0, jump_scale: 1.16,
        power: 0.98, reach: 1.12, special: Special::TalonKick, special_name: "SPIRAL DIVE",
        power_desc: "AIRBORNE DIVE KICK",
        color: (0.12, 0.52, 0.46), trim: (0.80, 0.20, 0.32),
        skin: (0.90, 0.72, 0.56), hair: (0.86, 0.66, 0.26), build: Build::Commando,
        bulk: 0.90, size: 1.0, boss: false,
    },
    Archetype {
        name: "AURELIA", health: 116, walk: 88.0, back_walk: 74.0, jump_scale: 0.86,
        power: 1.20, reach: 0.94, special: Special::BullRush, special_name: "BRONZE BREAKER",
        power_desc: "HEAVY ARMOUR BREAK",
        color: (0.64, 0.38, 0.16), trim: (0.98, 0.76, 0.34),
        skin: (0.76, 0.52, 0.38), hair: (0.68, 0.16, 0.12), build: Build::Gladiator,
        bulk: 1.24, size: 1.08, boss: false,
    },
    Archetype {
        name: "MACK", health: 108, walk: 142.0, back_walk: 116.0, jump_scale: 1.02,
        power: 1.05, reach: 1.0, special: Special::DriveBy, special_name: "YO! HOMIES!",
        power_desc: "LOWRIDER DRIVE-BY",
        color: (0.18, 0.18, 0.24), trim: (0.78, 0.12, 0.16),
        skin: (0.42, 0.27, 0.20), hair: (0.10, 0.08, 0.08), build: Build::Ganger,
        bulk: 1.04, size: 1.0, boss: false,
    },
];

/// The stages. Scenery only: the same floor line, height and walls in all.
pub(crate) const STAGES: usize = 8;
pub(crate) const STAGE_NAMES: [&str; STAGES] =
    ["THE DOCKS", "MOONLIT TEMPLE", "THE AIR BASE", "THE BATH HOUSE", "RIVER VILLAGE",
     "CRYSTAL FORTRESS", "QUEENS ROOFTOP", "THE HOOD"];

/// Where a fighter is at home. As in Street Fighter II, a fight is held at
/// the opponent's; a ladder never meets two of a kind in a row, so it never
/// visits one stage twice.
pub(crate) fn home_of(who: usize) -> usize {
    match FIGHTERS[who].build {
        Build::Bare | Build::Gladiator => 0,
        Build::Gi | Build::Qipao => 1,
        Build::Suit | Build::Uniform | Build::Commando => 2,
        Build::Turtle => 3,
        Build::Giant => 4,
        Build::Caped => 5,
        Build::Spider => 6,
        Build::Ganger => 7,
    }
}

pub(crate) const TURTLE_SKIN: (f32, f32, f32) = (0.44, 0.68, 0.30);
pub(crate) const TURTLE_SHELL: (f32, f32, f32) = (0.44, 0.31, 0.17);
