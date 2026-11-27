pub const SPEED_START: f32 = 180.0;
pub const SPEED_MIN: f32 = 70.0;
pub const SPEED_STEP: f32 = 10.0;
pub const FOODS_PER_LVL: i32 = 5;
/// The level each tune after the title's begins at (main.rs Game::track): MORNING, MOUNTAIN, ODE, MARCH, CANCAN.
pub const TUNE_FROM: [i32; 5] = [1, 3, 5, 7, 9];
/// Where the snake is quick enough that surviving takes skill (a bot playthrough starts there).
pub const FAST_LEVEL: i32 = 5;

pub const BONUS_EVERY: i32 = 3;
pub const BONUS_TTL: f32 = 6.0;
pub const BONUS_WARN: f32 = 2.0;
pub const BONUS_VALUE: i32 = 50;
pub const OBS_FADE_SECS: f32 = 1.2;

pub const SHEARS_FROM: usize = 25;
pub const SHEARS_CUT: usize = 4;
pub const SHEARS_TTL: f32 = 7.0;
