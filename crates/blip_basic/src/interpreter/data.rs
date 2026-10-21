//! `DATA` lists, read in program order by `READ`; `RESTORE [line]` rewinds.

use super::{procedure::leading_name, syntax::split_args, value::Value, Program};
use blip::Blip;

impl Program {
    /// Every `DATA` item, in order, with the line it is on.
    pub(super) fn find_data(&mut self) -> Result<(), String> {
        for (line, statements) in self.statements.iter().enumerate() {
            for statement in statements {
                let (word, rest) = leading_name(statement);
                if word != "DATA" {
                    continue;
                }
                for item in split_args(rest.trim()) {
                    let item = item.trim();
                    let value = if let Some(text) = item.strip_prefix('"').and_then(|t| t.strip_suffix('"')) {
                        Value::Str(text.replace("\"\"", "\""))
                    } else {
                        Value::Num(item.parse().map_err(|_| {
                            format!("DATA needs numbers or \"text\", not {item} at line {}", self.lines[line].0)
                        })?)
                    };
                    self.data.push((self.lines[line].0, value));
                }
            }
        }
        Ok(())
    }

    /// `READ A, B$, C(I)`: each name takes the next item.
    pub(super) fn read_data(&mut self, names: &str, blip: &Blip) -> Result<(), String> {
        for target in split_args(names) {
            let (_, value) = self.data.get(self.data_at).cloned().ok_or("Out of DATA")?;
            self.data_at += 1;
            let name = target.split('(').next().unwrap_or_default().trim();
            if name.ends_with('$') != matches!(value, Value::Str(_)) {
                return Err(format!("READ {} met the wrong kind of DATA", target.trim()));
            }
            self.store(target.trim(), value, blip)?;
        }
        Ok(())
    }

    /// `RESTORE` goes back to the first item; `RESTORE 870` to the first at or after line 870.
    pub(super) fn restore_data(&mut self, line: &str, blip: &Blip) -> Result<(), String> {
        let from = if line.is_empty() { 0.0 } else { self.eval(line, blip)?.number() };
        self.data_at = self.data.iter().position(|(n, _)| f64::from(*n) >= from).unwrap_or(self.data.len());
        Ok(())
    }
}
