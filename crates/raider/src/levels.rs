#[derive(Copy, Clone, PartialEq)]
pub enum BossPattern {
    Fan,
    Aimed,
    Sweep,
    Curtain,
}

pub struct BossSpec {
    pub hp: i32,
    pub speed: f32,
    pub fire_min: f32,
    pub fire_max: f32,
    pub dips: bool,
    pub escorts: bool,
    pub patterns: &'static [BossPattern],
    pub name: &'static str,
}

pub const BOSS_SPECS: [BossSpec; 7] = [
    BossSpec {
        hp: 80,
        speed: 76.0,
        fire_min: 0.48,
        fire_max: 1.00,
        dips: false,
        escorts: false,
        patterns: &[BossPattern::Fan],
        name: "KI-49 DONRYU",
    },
    BossSpec {
        hp: 130,
        speed: 84.0,
        fire_min: 0.44,
        fire_max: 0.92,
        dips: false,
        escorts: false,
        patterns: &[BossPattern::Fan],
        name: "KI-67 HIRYU",
    },
    BossSpec {
        hp: 190,
        speed: 92.0,
        fire_min: 0.38,
        fire_max: 0.82,
        dips: true,
        escorts: false,
        patterns: &[BossPattern::Fan, BossPattern::Aimed],
        name: "G4M BETTY",
    },
    BossSpec {
        hp: 260,
        speed: 100.0,
        fire_min: 0.34,
        fire_max: 0.72,
        dips: true,
        escorts: false,
        patterns: &[BossPattern::Fan, BossPattern::Aimed, BossPattern::Sweep],
        name: "G8N RENZAN",
    },
    BossSpec {
        hp: 340,
        speed: 109.0,
        fire_min: 0.30,
        fire_max: 0.64,
        dips: true,
        escorts: true,
        patterns: &[BossPattern::Aimed, BossPattern::Sweep],
        name: "H8K EMILY",
    },
    BossSpec {
        hp: 430,
        speed: 118.0,
        fire_min: 0.27,
        fire_max: 0.58,
        dips: true,
        escorts: true,
        patterns: &[BossPattern::Sweep, BossPattern::Aimed, BossPattern::Curtain],
        name: "G5N SHINZAN",
    },
    BossSpec {
        hp: 560,
        speed: 132.0,
        fire_min: 0.22,
        fire_max: 0.47,
        dips: true,
        escorts: true,
        patterns: &[BossPattern::Fan, BossPattern::Aimed, BossPattern::Sweep, BossPattern::Curtain],
        name: "FUGAKU",
    },
];

include!("boss_shape.rs");

pub const MAX_LEVEL: i32 = BOSS_SPECS.len() as i32;
pub const WAVE_KILL_BASE: i32 = 60;
pub const FLIGHT_BONUS: i32 = 50;
pub const BARRIER_MIN_LEVEL: i32 = 3;
pub const MAX_LASERS: usize = 2;

pub const BOSS_WING_GUNS: [&[f32]; 7] =
    [&[], &[0.62], &[0.62], &[0.36, 0.72], &[0.31, 0.62], &[0.32, 0.6, 0.8], &[0.62]];

pub const BOSS_LASERS: [&[(f32, f32)]; 7] = [&[], &[], &[], &[], &[], &[(0.0, 0.42)], &[(-0.36, 0.38), (0.36, 0.38)]];

pub const BOSS_WINGS: [(f32, f32, f32, f32); 7] = [
    (0.30, 0.52, 0.37, 0.46),
    (0.30, 0.50, 0.37, 0.45),
    (0.32, 0.54, 0.40, 0.48),
    (0.30, 0.52, 0.38, 0.46),
    (0.30, 0.46, 0.34, 0.41),
    (0.30, 0.48, 0.36, 0.43),
    (0.30, 0.46, 0.36, 0.41),
];
