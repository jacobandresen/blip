//! Cached tokens, operator precedence, and bindings to Rust functions.

use super::{
    shared_math,
    syntax::{tokenize, Token},
    Program, Value,
};
use blip::Blip;
use macroquad::color::Color;
use std::sync::Arc;

impl Program {
    pub(super) fn eval(&mut self, source: &str, blip: &Blip) -> Result<Value, String> {
        let tokens = if let Some(tokens) = self.expressions.get(source) {
            Arc::clone(tokens)
        } else {
            let tokens: Arc<[Token]> = tokenize(source)?.into();
            self.expressions
                .insert(source.to_string(), Arc::clone(&tokens));
            tokens
        };
        let mut parser = Expr {
            tokens,
            at: 0,
            program: self,
            blip,
        };
        let value = parser.expr(0)?;
        if parser.at != parser.tokens.len() {
            return Err("Unexpected expression token".into());
        }
        Ok(value)
    }
}

struct Expr<'p, 'b> {
    tokens: Arc<[Token]>,
    at: usize,
    program: &'p mut Program,
    blip: &'b Blip,
}
impl Expr<'_, '_> {
    fn expr(&mut self, min: u8) -> Result<Value, String> {
        let mut left = self.atom()?;
        loop {
            let op = match self.tokens.get(self.at) {
                Some(Token::Op(op)) => op.clone(),
                Some(Token::Name(n)) if n == "AND" || n == "OR" || n == "MOD" => n.clone(),
                _ => break,
            };
            let prec = match op.as_str() {
                "OR" => 1,
                "AND" => 2,
                "=" | "==" | "<>" | "<" | ">" | "<=" | ">=" => 3,
                "+" | "-" => 4,
                "*" | "/" | "MOD" => 5,
                _ => 0,
            };
            if prec < min {
                break;
            }
            self.at += 1;
            let right = self.expr(prec + 1)?;
            left = match op.as_str() {
                "+" if matches!(left, Value::Str(_)) || matches!(right, Value::Str(_)) => {
                    Value::Str(left.string() + &right.string())
                }
                "+" => Value::Num(left.number() + right.number()),
                "-" => Value::Num(left.number() - right.number()),
                "*" => Value::Num(left.number() * right.number()),
                "/" => Value::Num(left.number() / right.number()),
                // Floored: -1 MOD 8 is 7, so a wrapped position never goes negative.
                "MOD" => {
                    let (a, b) = (left.number(), right.number());
                    Value::Num(a - b * (a / b).floor())
                }
                "=" | "==" => Value::Num((left == right) as u8 as f64),
                "<>" => Value::Num((left != right) as u8 as f64),
                "<" => Value::Num((left.number() < right.number()) as u8 as f64),
                ">" => Value::Num((left.number() > right.number()) as u8 as f64),
                "<=" => Value::Num((left.number() <= right.number()) as u8 as f64),
                ">=" => Value::Num((left.number() >= right.number()) as u8 as f64),
                "AND" => Value::Num((left.number() != 0.0 && right.number() != 0.0) as u8 as f64),
                "OR" => Value::Num((left.number() != 0.0 || right.number() != 0.0) as u8 as f64),
                _ => unreachable!(),
            };
        }
        Ok(left)
    }
    fn atom(&mut self) -> Result<Value, String> {
        let token = self
            .tokens
            .get(self.at)
            .cloned()
            .ok_or("Expression expected")?;
        self.at += 1;
        match token {
            Token::Num(n) => Ok(Value::Num(n)),
            Token::Str(s) => Ok(Value::Str(s)),
            Token::Op(op) if op == "-" => Ok(Value::Num(-self.atom()?.number())),
            Token::Op(op) if op == "+" => Ok(Value::Num(self.atom()?.number())),
            Token::LParen => {
                let v = self.expr(0)?;
                self.need_rparen()?;
                Ok(v)
            }
            Token::Name(name) => self.name(&name),
            _ => Err("Expression expected".into()),
        }
    }
    fn name(&mut self, name: &str) -> Result<Value, String> {
        match name {
            "NOT" => return Ok(Value::Num((self.atom()?.number() == 0.0) as u8 as f64)),
            "INKEY$" => {
                return Ok(Value::Str(
                    if self.program.input.pressed(blip::input::BLIP_KEY_SPACE) { " " } else { "" }.into(),
                ))
            }
            _ => {}
        }
        if !matches!(self.tokens.get(self.at), Some(Token::LParen)) {
            return Ok(self
                .program
                .vars
                .get(name)
                .cloned()
                .unwrap_or_else(|| Value::empty(name)));
        }
        self.at += 1;
        let args = self.arguments()?;
        self.call(name, &args)
    }

    fn arguments(&mut self) -> Result<Vec<Value>, String> {
        let mut args = Vec::new();
        if !matches!(self.tokens.get(self.at), Some(Token::RParen)) {
            loop {
                args.push(self.expr(0)?);
                if !matches!(self.tokens.get(self.at), Some(Token::Comma)) {
                    break;
                }
                self.at += 1;
            }
        }
        self.need_rparen()?;
        Ok(args)
    }

    fn call(&mut self, name: &str, args: &[Value]) -> Result<Value, String> {
        if shared_math::contains(name) {
            if args.iter().any(|arg| !matches!(arg, Value::Num(_))) {
                return Err(format!("{name} needs numeric arguments"));
            }
            let numbers = args.iter().map(Value::number).collect::<Vec<_>>();
            return shared_math::call(name, &numbers).unwrap().map(Value::Num);
        }
        let first = args.first().cloned().unwrap_or(Value::Num(0.0));
        Ok(match name {
            "TIME" if args.is_empty() => Value::Num(macroquad::time::get_time()),
            "CONTROL_TEXT$" if args.len() == 3 => Value::Str(
                blip::web::controls().pick(&args[0].string(), &args[1].string(), &args[2].string()).to_string(),
            ),
            "INT" => Value::Num(first.number().floor()),
            "ABS" => Value::Num(first.number().abs()),
            "SQR" => Value::Num(first.number().sqrt()),
            "SIN" => Value::Num(first.number().sin()),
            "COS" => Value::Num(first.number().cos()),
            "RND" => Value::Num(
                self.program
                    .random(args.first().map(Value::number).unwrap_or(1.0)),
            ),
            "STR$" => Value::Str(first.string()),
            "LEN" => Value::Num(first.string().chars().count() as f64),
            "COLOR" => {
                if !(3..=4).contains(&args.len()) {
                    return Err("COLOR needs red,green,blue[,alpha]".into());
                }
                Value::Color(Color::new(
                    args[0].number() as f32,
                    args[1].number() as f32,
                    args[2].number() as f32,
                    args.get(3).map(Value::number).unwrap_or(1.0) as f32,
                ))
            }
            "KEY_ACTIVE" | "KEY_HELD" | "KEY_PRESSED" => {
                let key = key_code(&first.string().to_ascii_uppercase())?;
                let down = match name {
                    "KEY_ACTIVE" => self.program.input.active(key),
                    "KEY_HELD" => blip::input::key_held(key),
                    _ => self.program.input.pressed(key),
                };
                Value::Num(down as u8 as f64)
            }
            "TEXT_CX" => {
                if args.len() != 2 {
                    return Err("TEXT_CX needs text,size".into());
                }
                Value::Num(
                    self.blip
                        .text_cx(&args[0].string(), args[1].number() as i32)
                        as f64,
                )
            }
            _ => self
                .program
                .arrays
                .get(&Value::array_key(name, args))
                .cloned()
                .unwrap_or_else(|| Value::empty(name)),
        })
    }

    fn need_rparen(&mut self) -> Result<(), String> {
        if matches!(self.tokens.get(self.at), Some(Token::RParen)) {
            self.at += 1;
            Ok(())
        } else {
            Err("Expected )".into())
        }
    }
}

fn key_code(name: &str) -> Result<macroquad::input::KeyCode, String> {
    use blip::input::*;
    Ok(match name {
        "LEFT" => BLIP_KEY_LEFT,
        "RIGHT" => BLIP_KEY_RIGHT,
        "UP" => BLIP_KEY_UP,
        "DOWN" => BLIP_KEY_DOWN,
        "SPACE" | "FIRE" => BLIP_KEY_SPACE,
        "ENTER" | "RETURN" => macroquad::input::KeyCode::Enter,
        "A" => BLIP_KEY_A,
        "D" => BLIP_KEY_D,
        "W" => BLIP_KEY_W,
        "S" => BLIP_KEY_S,
        "F" => BLIP_KEY_F,
        "G" => BLIP_KEY_G,
        "I" => macroquad::input::KeyCode::I,
        "J" => BLIP_KEY_J,
        "K" => BLIP_KEY_K,
        "BUTTON2" | "Z" => BLIP_KEY_BUTTON2,
        _ => return Err(format!("Unknown key {name}")),
    })
}
