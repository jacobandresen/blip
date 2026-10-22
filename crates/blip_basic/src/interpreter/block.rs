//! AmigaBASIC-style blocks, found at load: a line ending in `THEN` opens an IF
//! that runs to `ELSEIF … THEN`, `ELSE` or `END IF`; `WHILE … WEND` repeats.

use super::{procedure::leading_name, syntax::top_level_then, Program};
use blip::Blip;

/// A statement's place in the program: line index, statement index.
pub(super) type At = (usize, usize);

#[derive(Clone, Debug, Default)]
pub(super) struct Block {
    /// Each `ELSEIF` or `ELSE` in order, then `END IF`.
    branches: Vec<At>,
}

/// `IF X THEN` with nothing after `THEN`, the block's condition.
pub(super) fn block_condition(statement: &str) -> Option<&str> {
    let (word, rest) = leading_name(statement);
    if word != "IF" && word != "ELSEIF" {
        return None;
    }
    let then = top_level_then(rest)?;
    rest[then + 4..].trim().is_empty().then(|| rest[..then].trim())
}

fn is_end_if(statement: &str) -> bool {
    let mut words = statement.split_whitespace();
    matches!((words.next(), words.next(), words.next()), (Some(e), Some(i), None)
        if e.eq_ignore_ascii_case("END") && i.eq_ignore_ascii_case("IF"))
}

fn is_else(statement: &str) -> bool {
    statement.trim().eq_ignore_ascii_case("ELSE")
}

impl Program {
    pub(super) fn find_blocks(&mut self) -> Result<(), String> {
        let mut open: Vec<(At, Block, bool)> = Vec::new();
        let mut whiles: Vec<(At, u16)> = Vec::new();
        for (line, statements) in self.statements.iter().enumerate() {
            let number = self.lines[line].0;
            for (at, statement) in statements.iter().enumerate() {
                let here = (line, at);
                let (word, _) = leading_name(statement);
                if word == "WHILE" {
                    whiles.push((here, number));
                } else if word == "WEND" {
                    let (start, _) = whiles.pop().ok_or_else(|| format!("WEND without WHILE at line {number}"))?;
                    self.block_ends.insert(start, here);
                    self.block_ends.insert(here, start);
                } else if word == "IF" && block_condition(statement).is_some() {
                    open.push((here, Block::default(), false));
                } else if word == "ELSEIF" || is_else(statement) {
                    let Some((_, block, had_else)) = open.last_mut() else {
                        return Err(format!("{word} without IF at line {number}"));
                    };
                    if *had_else {
                        return Err(format!("{word} after ELSE at line {number}"));
                    }
                    if word == "ELSEIF" && block_condition(statement).is_none() {
                        return Err(format!("ELSEIF needs THEN at the end of line {number}"));
                    }
                    *had_else = word == "ELSE";
                    block.branches.push(here);
                } else if is_end_if(statement) {
                    let Some((start, mut block, _)) = open.pop() else {
                        return Err(format!("END IF without IF at line {number}"));
                    };
                    block.branches.push(here);
                    for &branch in &block.branches {
                        self.block_ends.insert(branch, here);
                    }
                    self.blocks.insert(start, block);
                }
            }
        }
        if let Some((_, number)) = whiles.last() {
            return Err(format!("WHILE needs WEND at line {number}"));
        }
        match open.last() {
            Some(((line, _), _, _)) => Err(format!("IF needs END IF at line {}", self.lines[*line].0)),
            None => Ok(()),
        }
    }

    fn jump_to(&mut self, (line, statement): At) {
        self.position.line = line;
        self.position.statement = statement + 1;
        self.position.pending.clear();
    }

    /// A false condition tries each `ELSEIF` in turn, then takes `ELSE` or leaves.
    pub(super) fn enter_block(&mut self, condition: &str, blip: &Blip) -> Result<(), String> {
        let here = self.here.ok_or("A block IF must start its own statement")?;
        if self.eval(condition, blip)?.number() != 0.0 {
            return Ok(());
        }
        let branches = self.blocks.get(&here).ok_or("IF needs END IF")?.branches.clone();
        for branch in branches {
            let source = self.statements[branch.0][branch.1].clone();
            match block_condition(&source) {
                Some(next) if self.eval(next, blip)?.number() == 0.0 => continue,
                _ => {
                    self.jump_to(branch);
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    /// Reaching `ELSEIF` or `ELSE` means a branch ran: leave the block.
    pub(super) fn leave_block(&mut self) -> Result<(), String> {
        let here = self.here.ok_or("ELSE must start its own statement")?;
        let end = *self.block_ends.get(&here).ok_or("ELSE without IF")?;
        self.jump_to(end);
        Ok(())
    }
}

impl Program {
    /// `WHILE X`: a false condition leaves past its `WEND`.
    pub(super) fn enter_while(&mut self, condition: &str, blip: &Blip) -> Result<(), String> {
        let here = self.here.ok_or("WHILE must start its own statement")?;
        if self.eval(condition, blip)?.number() == 0.0 {
            let end = *self.block_ends.get(&here).ok_or("WHILE needs WEND")?;
            self.jump_to(end);
        }
        Ok(())
    }

    /// `WEND` goes back to test its `WHILE` again.
    pub(super) fn repeat_while(&mut self) -> Result<(), String> {
        let here = self.here.ok_or("WEND must start its own statement")?;
        let (line, statement) = *self.block_ends.get(&here).ok_or("WEND without WHILE")?;
        self.position.line = line;
        self.position.statement = statement;
        self.position.pending.clear();
        Ok(())
    }
}
