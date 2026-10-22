// Motherships recur every fifth level and scale with each appearance.
pub const BOSS_EVERY: i32 = 5;
pub const BOSS_HP_BASE: i32 = 16;
pub const BOSS_HP_STEP: i32 = 8;
pub const BOSS_SPEED_BASE: f32 = 78.0;
pub const BOSS_SPEED_STEP: f32 = 14.0;
pub const BOSS_VOLLEY_MIN: f32 = 1.05;
pub const BOSS_VOLLEY_FLOOR: f32 = 0.42;
pub const BOSS_VOLLEY_STEP: f32 = 0.09;
pub const BOSS_HIT_SCORE: i32 = 10;
pub const BOSS_KILL_SCORE: i32 = 500;

// The formation descends farther per edge after the opening levels.
pub const MARCH_START: i32 = 520;
pub const MARCH_MIN: i32 = 65;
pub const MARCH_DROP_FIRST: f32 = 10.0;
pub const MARCH_DROP: f32 = 16.0;

pub const DIVE_LEVEL: i32 = 3;
