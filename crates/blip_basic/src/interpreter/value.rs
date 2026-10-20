//! BASIC values, numeric and string conversion, and array indexing.

use macroquad::color::Color;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Value {
    Num(f64),
    Str(String),
    Color(Color),
}
impl Value {
    pub(super) fn empty(name: &str) -> Self {
        if name.ends_with('$') {
            Self::Str(String::new())
        } else {
            Self::Num(0.0)
        }
    }

    pub(super) fn array_key(name: &str, indices: &[Self]) -> String {
        let indices = indices
            .iter()
            .map(Self::string)
            .collect::<Vec<_>>()
            .join(",");
        format!("{name}({indices})")
    }

    pub(super) fn number(&self) -> f64 {
        match self {
            Self::Num(n) => *n,
            Self::Str(s) => s.parse().unwrap_or(0.0),
            Self::Color(_) => 0.0,
        }
    }
    pub(super) fn string(&self) -> String {
        match self {
            Self::Num(n) if n.fract() == 0.0 => format!("{n:.0}"),
            Self::Num(n) => n.to_string(),
            Self::Str(s) => s.clone(),
            Self::Color(_) => String::new(),
        }
    }
}
