//! Numbered BASIC programs, their execution state, and frame budget.

mod block;
mod color;
mod data;
mod expression;
mod graphics;
mod procedure;
mod statement;
mod syntax;
#[cfg(test)]
mod tests;
mod value;

use self::{
    block::{At, Block},
    procedure::{Frame, Function, Procedure},
    syntax::{split_statements, Token},
    value::Value,
};
use crate::{music::MusicRequest, shared_math};
use blip::{Blip, BlipTex};
use crate::sounds::Sounds;
use std::{collections::HashMap, sync::Arc};

// Calls and loops save the pending THEN actions as well as the source position.
#[derive(Clone, Default)]
struct Continuation {
    line: usize,
    statement: usize,
    pending: Vec<Arc<str>>,
}

#[derive(Clone)]
struct ForLoop {
    name: String,
    end: f64,
    step: f64,
    resume: Continuation,
}

#[derive(Default)]
pub(crate) struct Program {
    lines: Vec<(u16, String)>,
    statements: Vec<Vec<Arc<str>>>,
    expressions: HashMap<String, Arc<[Token]>>,
    position: Continuation,
    vars: HashMap<String, Value>,
    arrays: HashMap<String, Value>,
    effects: blip::Fx,
    loops: Vec<ForLoop>,
    calls: Vec<Continuation>,
    procedures: HashMap<String, Procedure>,
    functions: HashMap<String, Function>,
    function_depth: usize,
    data: Vec<(u16, Value)>,
    data_at: usize,
    blocks: HashMap<At, Block>,
    block_ends: HashMap<At, At>,
    /// The statement being run, when it came from the program rather than a THEN.
    here: Option<At>,
    frames: Vec<Frame>,
    output: Vec<String>,
    print_run: String,
    wait_until: f64,
    input: blip::input::PressBuffer,
    input_step: bool,
    pub(crate) yielded: bool,
    uninterrupted: u32,
    pub(crate) active: bool,
    pub(crate) error: Option<String>,
    rng: u64,
    pen: i32,
    paper: i32,
    pub(crate) music_request: Option<MusicRequest>,
    pub(crate) music_file_request: Option<String>,
    pub(crate) sound_request: Option<(f32, f32, f32)>,
}
impl Program {
    pub(crate) fn new() -> Self {
        Self {
            rng: 0x5eed,
            pen: 1,
            input_step: true,
            ..Self::default()
        }
    }

    fn random(&mut self, max: f64) -> f64 {
        self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.rng >> 32) as f64 / (u32::MAX as f64 + 1.0) * max
    }
    pub(crate) fn load(&mut self, source: &str) {
        *self = Self {
            rng: self.rng,
            active: true,
            ..Self::new()
        };
        self.input.clear();
        self.lines = source
            .lines()
            .filter_map(|line| {
                let (number, body) = line.trim().split_once(char::is_whitespace)?;
                Some((number.parse().ok()?, body.trim().to_owned()))
            })
            .collect();
        self.lines.sort_by_key(|line| line.0);
        self.statements = self
            .lines
            .iter()
            .map(|(_, source)| {
                split_statements(source)
                    .into_iter()
                    .map(Arc::<str>::from)
                    .collect()
            })
            .collect();
        if let Err(error) = self.find_procedures().and_then(|_| self.find_blocks()).and_then(|_| self.find_data()) {
            self.error = Some(format!("? {error}"));
            self.active = false;
        }
    }
    fn goto(&mut self, number: f64) -> Result<(), String> {
        let number = number as u16;
        self.position.line = self
            .lines
            .iter()
            .position(|(n, _)| *n == number)
            .ok_or_else(|| format!("Line {number} not found"))?;
        self.position.statement = 0;
        self.position.pending.clear();
        Ok(())
    }
    fn next_statement(&mut self) -> Option<(u16, Arc<str>)> {
        while self.position.line < self.lines.len() {
            let line = self.position.line;
            if let Some(source) = self.position.pending.pop() {
                self.here = None;
                return Some((self.lines[line].0, source));
            }
            if let Some(source) = self.statements[line].get(self.position.statement) {
                self.here = Some((line, self.position.statement));
                self.position.statement += 1;
                return Some((self.lines[line].0, Arc::clone(source)));
            }
            self.position.line += 1;
            self.position.statement = 0;
        }
        None
    }

    fn fail(&mut self, message: String, line: u16) {
        self.error = Some(format!("? {message} at line {line}"));
        self.active = false;
    }

    pub(crate) fn tick(
        &mut self,
        blip: &mut Blip,
        textures: &HashMap<String, BlipTex>,
        sounds: &mut Sounds,
    ) {
        const FRAME_BUDGET: usize = 50_000;
        const RUNAWAY_LIMIT: u32 = 100_000;

        // WAIT can skip render frames; keep their press edges for the next update.
        self.input.capture();
        if !self.active || macroquad::time::get_time() * 1000.0 < self.wait_until {
            return;
        }
        if self.input_step {
            self.input.begin_step();
            self.input_step = false;
        }
        self.yielded = false;
        let deadline = macroquad::time::get_time() + 0.008;
        for executed in 0..FRAME_BUDGET {
            if executed % 128 == 0 && macroquad::time::get_time() >= deadline {
                return;
            }
            if !self.active {
                return;
            }
            let Some((line, source)) = self.next_statement() else {
                self.active = false;
                return;
            };
            self.uninterrupted += 1;
            if self.uninterrupted > RUNAWAY_LIMIT {
                self.fail("Program took too long between WAIT commands".into(), line);
                return;
            }
            if let Err(error) = self.execute(&source, blip, textures, sounds) {
                self.fail(error, line);
                return;
            }
            if self.yielded {
                return;
            }
        }
        // Finite loops can span frames; WAIT resets the cumulative runaway limit.
    }
}
