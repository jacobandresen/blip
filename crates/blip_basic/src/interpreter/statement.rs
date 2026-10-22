//! Statements change control flow or state; drawing calls go through `graphics`.

use super::{
    block::block_condition,
    color::paper_color,
    procedure::leading_name,
    syntax::{assignment_eq, method_call, split_args, matching_else, print_items, split_statements, top_level_then},
    ForLoop, MusicRequest, Program, Value,
};
use crate::host::audio_event;
use crate::sounds::Sounds;
use blip::{Blip, BlipTex};
use std::{collections::HashMap, sync::Arc};

impl Program {
    pub(super) fn execute(
        &mut self,
        source: &str,
        blip: &mut Blip,
        textures: &HashMap<String, BlipTex>,
        sounds: &mut Sounds,
    ) -> Result<(), String> {
        let source = source.trim();
        if source.is_empty() || source.starts_with('\'') {
            return Ok(());
        }
        let command = method_call(source).map(|(name, args)| format!("{name} {args}"));
        let command = command.as_deref().unwrap_or(source);
        let (keyword, rest) = command
            .split_once(char::is_whitespace)
            .unwrap_or((command, ""));
        let rest = rest.trim();

        match keyword.to_ascii_uppercase().as_str() {
            "REM" => Ok(()),
            "IF" => match block_condition(source) {
                Some(condition) => self.enter_block(condition, blip),
                None => self.inline_if(rest, blip),
            },
            "ELSEIF" | "ELSE" => self.leave_block(),
            "WHILE" => self.enter_while(rest, blip),
            "WEND" => self.repeat_while(),
            "EXIT" if rest.eq_ignore_ascii_case("FOR") => {
                self.loops.pop().ok_or("EXIT FOR outside a FOR")?;
                self.skip_loop()
            }
            "GOTO" | "GOSUB" => {
                let target = self.eval(rest, blip)?.number();
                if keyword.eq_ignore_ascii_case("GOSUB") {
                    self.calls.push(self.position.clone());
                }
                self.goto(target)
            }
            "RETURN" => {
                self.position = self.calls.pop().ok_or("RETURN without GOSUB")?;
                Ok(())
            }
            "FOR" => self.begin_loop(rest, blip),
            "NEXT" => self.next_loop(rest),
            "DEF" | "DATA" => Ok(()),
            "READ" => self.read_data(rest, blip),
            "RESTORE" => self.restore_data(rest, blip),
            "SUB" => self.skip_procedure(rest),
            "CALL" => self.call_procedure(rest, blip),
            "LOCAL" => self.declare_locals(rest),
            "END" | "EXIT" if rest.eq_ignore_ascii_case("SUB") => self.return_from_procedure(),
            "END" if rest.eq_ignore_ascii_case("IF") => Ok(()),
            "END" | "STOP" => {
                self.active = false;
                Ok(())
            }
            "CLS" => {
                self.output.clear();
                blip.clear(paper_color(self.paper));
                Ok(())
            }
            "PAPER" => {
                self.paper = self.eval(rest, blip)?.number() as i32;
                blip.clear(paper_color(self.paper));
                Ok(())
            }
            "PEN" | "INK" => {
                self.pen = self.eval(rest, blip)?.number() as i32;
                Ok(())
            }
            "WAIT" => {
                let milliseconds = self.eval(rest, blip)?.number().clamp(0.0, 10_000.0);
                self.wait(milliseconds);
                Ok(())
            }
            "REST" => {
                let seconds = self.eval(rest, blip)?.number();
                if !(0.0..=10.0).contains(&seconds) {
                    return Err("REST duration must be 0–10 seconds".into());
                }
                self.wait(seconds * 1000.0);
                Ok(())
            }
            "SOUND" => self.sound(rest, blip),
            "PRINT" => self.print(rest, blip),
            "PLAY_SFX" => self.play_sfx(rest, blip, sounds),
            "PLAY_MUSIC" => self.play_music(rest, blip),
            "STOP_MUSIC" => {
                self.music_file_request = None;
                self.music_request = Some(MusicRequest::Stop);
                Ok(())
            }
            _ if self.is_bare_call(command) => self.call_procedure(command, blip),
            _ => match assignment_eq(command) {
                Some(eq) => self.assign(command, eq, blip),
                None => self.draw_statement(command, blip, textures),
            },
        }
    }

    /// `IF X THEN A:B ELSE C`; a bare line number after THEN or ELSE is a GOTO.
    fn inline_if(&mut self, rest: &str, blip: &Blip) -> Result<(), String> {
        let then = top_level_then(rest).ok_or("IF needs THEN")?;
        let actions = &rest[then + 4..];
        let (yes, no) = match matching_else(actions) {
            Some(at) => (&actions[..at], Some(&actions[at + 4..])),
            None => (actions, None),
        };
        let taken = if self.eval(rest[..then].trim(), blip)?.number() != 0.0 { Some(yes) } else { no };
        let Some(branch) = taken.map(str::trim) else { return Ok(()) };
        if let Ok(line) = branch.parse::<u16>() {
            return self.goto(line as f64);
        }
        self.position.pending.extend(split_statements(branch).into_iter().rev().map(Arc::<str>::from));
        Ok(())
    }

    /// `NAME 1, 2` or `NAME(1, 2)` for a SUB; `NAME = 1` stays an assignment.
    fn is_bare_call(&self, command: &str) -> bool {
        let (name, rest) = leading_name(command);
        self.is_procedure(&name) && !rest.trim_start().starts_with('=')
    }

    fn wait(&mut self, milliseconds: f64) {
        self.wait_until = macroquad::time::get_time() * 1000.0 + milliseconds;
        self.uninterrupted = 0;
        self.yielded = true;
        self.input_step = true;
    }

    fn begin_loop(&mut self, rest: &str, blip: &Blip) -> Result<(), String> {
        let eq = rest.find('=').ok_or("FOR needs =")?;
        let name = rest[..eq].trim().to_ascii_uppercase();
        let tail = rest[eq + 1..].trim();
        let to = tail
            .to_ascii_uppercase()
            .find(" TO ")
            .ok_or("FOR needs TO")?;
        let start = self.eval(tail[..to].trim(), blip)?.number();
        let tail = &tail[to + 4..];
        let step_at = tail.to_ascii_uppercase().find(" STEP ");
        let end = self
            .eval(step_at.map_or(tail, |i| &tail[..i]).trim(), blip)?
            .number();
        let step = if let Some(i) = step_at {
            self.eval(tail[i + 6..].trim(), blip)?.number()
        } else {
            1.0
        };
        self.vars.insert(name.clone(), Value::Num(start));
        // FOR I=1 TO 0 does nothing, as in AmigaBASIC: no pass with stale values.
        if step > 0.0 && start > end || step < 0.0 && start < end {
            return self.skip_loop();
        }
        self.loops.push(ForLoop {
            name,
            end,
            step,
            resume: self.position.clone(),
        });
        Ok(())
    }

    /// Step past the `NEXT` that closes this `FOR`, over any loops nested inside.
    fn skip_loop(&mut self) -> Result<(), String> {
        let mut nested = 0;
        while let Some((_, statement)) = self.next_statement() {
            match leading_name(&statement).0.as_str() {
                "FOR" => nested += 1,
                "NEXT" if nested == 0 => return Ok(()),
                "NEXT" => nested -= 1,
                _ => {}
            }
        }
        Err("FOR without NEXT".into())
    }

    fn next_loop(&mut self, rest: &str) -> Result<(), String> {
        let frame = self
            .loops
            .last()
            .cloned()
            .ok_or_else(|| format!("NEXT {} without FOR", rest))?;
        if let Some(name) = rest.split_whitespace().next() {
            if !name.is_empty() && name.to_ascii_uppercase() != frame.name {
                return Err("NEXT variable mismatch".into());
            }
        }
        let current = self.vars.get(&frame.name).map(Value::number).unwrap_or(0.0) + frame.step;
        self.vars.insert(frame.name.clone(), Value::Num(current));
        if frame.step != 0.0
            && if frame.step > 0.0 {
                current <= frame.end
            } else {
                current >= frame.end
            }
        {
            self.position = frame.resume;
        } else {
            self.loops.pop();
        }
        Ok(())
    }

    fn sound(&mut self, rest: &str, blip: &Blip) -> Result<(), String> {
        let args = split_args(rest);
        if args.len() < 2 || args.len() > 3 {
            return Err("SOUND needs frequency,duration[,volume]".into());
        }
        let frequency = self.eval(&args[0], blip)?.number();
        let seconds = self.eval(&args[1], blip)?.number();
        let volume = args
            .get(2)
            .map(|value| self.eval(value, blip).map(|v| v.number()))
            .transpose()?
            .unwrap_or(0.35);
        if !(0.0..=12000.0).contains(&frequency)
            || !(0.0..=10.0).contains(&seconds)
            || !(0.0..=1.0).contains(&volume)
        {
            return Err("SOUND values are out of range".into());
        }
        self.sound_request = Some((frequency as f32, seconds as f32, volume as f32));
        Ok(())
    }

    fn print(&mut self, rest: &str, blip: &Blip) -> Result<(), String> {
        let mut line = std::mem::take(&mut self.print_run);
        let mut runs_on = false;
        for (item, after) in print_items(rest) {
            if !item.is_empty() {
                line += &self.eval(&item, blip)?.string();
            }
            if after == Some(',') {
                line.push(' ');
            }
            runs_on = after.is_some();
        }
        // A PRINT that ends in ; or , carries on the same line next time.
        if runs_on {
            self.print_run = line;
            return Ok(());
        }
        self.output.push(line);
        if self.output.len() > 10 {
            self.output.remove(0);
        }
        Ok(())
    }

    fn play_sfx(&mut self, rest: &str, blip: &Blip, sounds: &mut Sounds) -> Result<(), String> {
        let args = split_args(rest);
        if args.is_empty() || args.len() > 2 {
            return Err("PLAY_SFX needs name[,volume]".into());
        }
        let name = self.eval(&args[0], blip)?.string().to_ascii_lowercase();
        let volume = args
            .get(1)
            .map(|value| self.eval(value, blip).map(|v| v.number() as f32))
            .transpose()?
            .unwrap_or(1.0)
            .clamp(0.0, 1.0);
        sounds.play(name, volume)?;
        audio_event();
        Ok(())
    }

    fn play_music(&mut self, rest: &str, blip: &Blip) -> Result<(), String> {
        let name = self.eval(rest, blip)?.string();
        let builtin = name.to_ascii_uppercase();
        let level = self
            .vars
            .get("LV")
            .or_else(|| self.vars.get("ROUND"))
            .map(Value::number)
            .unwrap_or(1.0) as i32;
        self.music_file_request = None;
        self.music_request = Some(match builtin.as_str() {
            "BOUNCER_TITLE" => MusicRequest::Bouncer(0),
            "BOUNCER_GAME" => MusicRequest::Bouncer((level + 1).rem_euclid(2) as usize),
            "BOUNCER_ALT" => MusicRequest::Bouncer(1),
            "DEFENDER_MAIN" => MusicRequest::Defender(0),
            "DEFENDER_BOSS" => MusicRequest::Defender(1),
            "ADDER_COIL" => MusicRequest::Adder(0),
            "ADDER_HUNT" => MusicRequest::Adder(1),
            "SERPENT_MAIN" => MusicRequest::Serpent(0),
            "SERPENT_FRENZY" => MusicRequest::Serpent(1),
            "RALLY_MAIN" => MusicRequest::Rally(0),
            "RALLY_ALT" => MusicRequest::Rally(1),
            "METEORS_DRIFT" => MusicRequest::Meteors(0),
            "METEORS_STORM" => MusicRequest::Meteors(1),
            "RAIDER_MAIN" => MusicRequest::SkyRaider(0),
            "RAIDER_ALT" => MusicRequest::SkyRaider(1),
            "BRAWLER_ARENA" => MusicRequest::Brawler(0),
            "BUBBLER_MAIN" => MusicRequest::Bubbler(0),
            "BUBBLER_HURRY" => MusicRequest::Bubbler(1),
            _ => {
                let mut path = builtin;
                if !path.starts_with("/USER/") {
                    return Err(format!("Unknown music track {name}"));
                }
                if path.ends_with(".TUNE") {
                    path.truncate(path.len() - 5);
                    path.push_str(".WAV");
                } else if !path.ends_with(".WAV") {
                    path.push_str(".WAV");
                }
                self.music_request = None;
                self.music_file_request = Some(path);
                return Ok(());
            }
        });
        Ok(())
    }

    fn assign(&mut self, command: &str, eq: usize, blip: &Blip) -> Result<(), String> {
        let lhs = command[..eq].trim();
        let lhs = if lhs
            .get(..4)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("LET "))
        {
            lhs[4..].trim()
        } else {
            lhs
        };
        let value = self.eval(command[eq + 1..].trim(), blip)?;
        self.store(lhs, value, blip)
    }

    /// Put a value in a variable or an array element: `X`, `NAME$`, `A(I,J)`.
    pub(super) fn store(&mut self, lhs: &str, value: Value, blip: &Blip) -> Result<(), String> {
        if let Some(open) = lhs.find('(') {
            let name = lhs[..open].trim().to_ascii_uppercase();
            let arguments = lhs[open + 1..]
                .strip_suffix(')')
                .ok_or("Array assignment needs )")?;
            let indices = split_args(arguments.trim())
                .iter()
                .map(|source| self.eval(source, blip))
                .collect::<Result<Vec<_>, _>>()?;
            self.arrays.insert(Value::array_key(&name, &indices), value);
        } else {
            self.vars.insert(lhs.to_ascii_uppercase(), value);
        }
        Ok(())
    }
}
