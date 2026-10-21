//! AmigaBASIC-style procedures: `SUB NAME(A, B$) ... END SUB`, entered with
//! `CALL NAME(1, "X")` or a bare `NAME 1, "X"`, and one-line functions,
//! `DEF FNNAME(A)=expression`. Parameters and `LOCAL` names belong to the
//! call; every other name is shared with the program.

use super::{syntax::{assignment_eq, split_args}, value::Value, Continuation, Program};
use blip::Blip;

/// Deep enough for any recursive drawing, shallow enough to stop a runaway.
const MAX_DEPTH: usize = 200;

#[derive(Clone, Debug)]
pub(super) struct Procedure {
    pub(super) params: Vec<String>,
    /// Where the body starts: the statement after `SUB`.
    pub(super) body: (usize, usize),
    /// The `END SUB` statement; normal flow resumes after it.
    pub(super) end: (usize, usize),
}

/// `DEF FNNAME(A, B)=expression`.
#[derive(Clone, Debug)]
pub(super) struct Function {
    pub(super) params: Vec<String>,
    pub(super) body: String,
}

pub(super) struct Frame {
    resume: Continuation,
    /// Each parameter or local's value before the call, restored on return.
    saved: Vec<(String, Option<Value>)>,
    loops: usize,
    calls: usize,
}

fn is_name(word: &str) -> bool {
    let mut chars = word.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// The leading identifier of a statement, upper-cased, and what follows it.
pub(super) fn leading_name(source: &str) -> (String, &str) {
    let end = source
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
        .unwrap_or(source.len());
    (source[..end].to_ascii_uppercase(), &source[end..])
}

/// `(A, B)` around a whole argument list is the call's own; `(A+1)*2` is not.
fn call_arguments(rest: &str) -> &str {
    let rest = rest.trim();
    if !rest.starts_with('(') {
        return rest;
    }
    let mut depth = 0;
    let mut quoted = false;
    for (at, ch) in rest.char_indices() {
        match ch {
            '"' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => {
                depth -= 1;
                if depth == 0 {
                    return if at + 1 == rest.len() { rest[1..at].trim() } else { rest };
                }
            }
            _ => {}
        }
    }
    rest
}

fn is_keyword(word: &str, keyword: &str) -> bool {
    word.split_whitespace().collect::<Vec<_>>() == keyword.split(' ').collect::<Vec<_>>()
}

/// Names a procedure may not take: statements the interpreter already knows.
const RESERVED: [&str; 34] = [
    "REM", "IF", "GOTO", "GOSUB", "RETURN", "FOR", "NEXT", "END", "STOP", "CLS", "PAPER", "PEN",
    "INK", "WAIT", "REST", "SOUND", "PRINT", "PLAY_SFX", "PLAY_MUSIC", "STOP_MUSIC", "SUB", "CALL",
    "EXIT", "LOCAL", "LET", "THEN", "ELSE", "ELSEIF", "DEF", "DATA", "READ", "RESTORE", "WHILE", "WEND",
];

impl Program {
    /// Finds every `SUB ... END SUB` once, when the program loads.
    pub(super) fn find_procedures(&mut self) -> Result<(), String> {
        let mut open: Option<(String, Procedure, u16)> = None;
        for (line, statements) in self.statements.iter().enumerate() {
            let number = self.lines[line].0;
            for (at, statement) in statements.iter().enumerate() {
                let (word, rest) = leading_name(statement);
                if word == "DEF" {
                    let (name, function) = Self::def_fn(rest.trim())
                        .map_err(|error| format!("{error} at line {number}"))?;
                    if self.functions.insert(name.clone(), function).is_some() {
                        return Err(format!("{name} is defined twice"));
                    }
                } else if word == "SUB" {
                    if let Some((name, _, _)) = &open {
                        return Err(format!("SUB inside SUB {name} at line {number}"));
                    }
                    let (name, params) = Self::sub_header(rest.trim())
                        .map_err(|error| format!("{error} at line {number}"))?;
                    let proc = Procedure { params, body: (line, at + 1), end: (0, 0) };
                    open = Some((name, proc, number));
                } else if is_keyword(statement, "END SUB") {
                    let Some((name, mut proc, _)) = open.take() else {
                        return Err(format!("END SUB without SUB at line {number}"));
                    };
                    proc.end = (line, at);
                    if self.procedures.insert(name.clone(), proc).is_some() {
                        return Err(format!("SUB {name} is defined twice"));
                    }
                }
            }
        }
        match open {
            Some((name, _, number)) => Err(format!("SUB {name} needs END SUB at line {number}")),
            None => Ok(()),
        }
    }

    /// `NAME(A, B$) STATIC` → the name and its parameters.
    fn sub_header(header: &str) -> Result<(String, Vec<String>), String> {
        let header = header.trim();
        let header = header
            .strip_suffix("STATIC")
            .or_else(|| header.strip_suffix("static"))
            .unwrap_or(header)
            .trim();
        let (name, rest) = leading_name(header);
        if !is_name(&name) || RESERVED.contains(&name.as_str()) {
            return Err(format!("SUB needs a name, not {}", if name.is_empty() { header } else { &name }));
        }
        let rest = rest.trim();
        let params = if rest.is_empty() {
            Vec::new()
        } else {
            let inner = rest
                .strip_prefix('(')
                .and_then(|r| r.strip_suffix(')'))
                .ok_or_else(|| format!("SUB {name} needs (parameters)"))?;
            let params: Vec<String> = split_args(inner)
                .into_iter()
                .map(|p| p.trim().to_ascii_uppercase())
                .filter(|p| !p.is_empty())
                .collect();
            if let Some(bad) = params.iter().find(|p| !is_name(p)) {
                return Err(format!("SUB {name} has a bad parameter {bad}"));
            }
            params
        };
        Ok((name, params))
    }

    /// `FNNAME(A, B)=expression` → the name and its function.
    fn def_fn(definition: &str) -> Result<(String, Function), String> {
        let eq = assignment_eq(definition).ok_or("DEF needs =")?;
        let (name, params) = Self::sub_header(&definition[..eq])
            .map_err(|_| format!("DEF needs FNNAME(...), not {}", definition[..eq].trim()))?;
        if !name.starts_with("FN") {
            return Err(format!("DEF needs a name starting FN, not {name}"));
        }
        let body = definition[eq + 1..].trim().to_string();
        Ok((name, Function { params, body }))
    }

    /// Runs `FNNAME(args)`: the parameters shadow the program's names meanwhile.
    pub(super) fn call_function(&mut self, name: &str, args: &[Value], blip: &Blip) -> Result<Value, String> {
        let function = self.functions.get(name).cloned().ok_or_else(|| format!("No DEF {name}"))?;
        if args.len() != function.params.len() {
            return Err(format!("{name} needs {} values, got {}", function.params.len(), args.len()));
        }
        // Each nested FN is a Rust call too: this keeps a runaway off the stack.
        if self.function_depth >= MAX_DEPTH {
            return Err(format!("Too many nested calls ({MAX_DEPTH})"));
        }
        let saved: Vec<_> = function
            .params
            .iter()
            .zip(args)
            .map(|(param, value)| (param.clone(), self.vars.insert(param.clone(), value.clone())))
            .collect();
        self.function_depth += 1;
        let result = self.eval(&function.body, blip);
        self.function_depth -= 1;
        for (name, old) in saved.into_iter().rev() {
            match old {
                Some(value) => self.vars.insert(name, value),
                None => self.vars.remove(&name),
            };
        }
        result
    }

    pub(super) fn is_procedure(&self, name: &str) -> bool {
        self.procedures.contains_key(name)
    }

    /// Normal flow steps over a procedure's body.
    pub(super) fn skip_procedure(&mut self, header: &str) -> Result<(), String> {
        let (name, _) = leading_name(header.trim());
        let proc = self.procedures.get(&name).ok_or("SUB not found")?;
        self.position.line = proc.end.0;
        self.position.statement = proc.end.1 + 1;
        self.position.pending.clear();
        Ok(())
    }

    /// `NAME args` or `NAME(args)`, after `CALL` or on its own.
    pub(super) fn call_procedure(&mut self, call: &str, blip: &Blip) -> Result<(), String> {
        let (name, rest) = leading_name(call.trim());
        let proc = self
            .procedures
            .get(&name)
            .cloned()
            .ok_or_else(|| format!("No SUB called {name}"))?;
        let rest = call_arguments(rest);
        let args = if rest.is_empty() { Vec::new() } else { split_args(rest) };
        if args.len() != proc.params.len() {
            return Err(format!("{name} needs {} values, got {}", proc.params.len(), args.len()));
        }
        if self.frames.len() >= MAX_DEPTH {
            return Err(format!("Too many nested CALLs ({MAX_DEPTH})"));
        }
        let values = args
            .iter()
            .map(|arg| self.eval(arg, blip))
            .collect::<Result<Vec<_>, _>>()?;
        let mut saved = Vec::with_capacity(values.len());
        for (param, value) in proc.params.iter().zip(values) {
            if param.ends_with('$') != matches!(value, Value::Str(_)) && !matches!(value, Value::Color(_)) {
                return Err(format!("{name}: {param} needs {}", if param.ends_with('$') { "text" } else { "a number" }));
            }
            saved.push((param.clone(), self.vars.insert(param.clone(), value)));
        }
        self.frames.push(Frame {
            resume: self.position.clone(),
            saved,
            loops: self.loops.len(),
            calls: self.calls.len(),
        });
        self.position = Continuation { line: proc.body.0, statement: proc.body.1, pending: Vec::new() };
        Ok(())
    }

    /// `LOCAL A, B$`: names that start empty and are put back on return.
    pub(super) fn declare_locals(&mut self, names: &str) -> Result<(), String> {
        let frame = self.frames.last_mut().ok_or("LOCAL outside a SUB")?;
        for name in split_args(names) {
            let name = name.trim().to_ascii_uppercase();
            if !is_name(&name) {
                return Err(format!("LOCAL needs names, not {name}"));
            }
            let old = self.vars.insert(name.clone(), Value::empty(&name));
            frame.saved.push((name, old));
        }
        Ok(())
    }

    /// `END SUB` and `EXIT SUB`: put the caller's names back and resume it.
    pub(super) fn return_from_procedure(&mut self) -> Result<(), String> {
        let frame = self.frames.pop().ok_or("END SUB without CALL")?;
        for (name, old) in frame.saved.into_iter().rev() {
            match old {
                Some(value) => self.vars.insert(name, value),
                None => self.vars.remove(&name),
            };
        }
        self.loops.truncate(frame.loops);
        self.calls.truncate(frame.calls);
        self.position = frame.resume;
        Ok(())
    }
}
