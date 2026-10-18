use std::collections::HashMap;
use std::sync::Arc;

use blip::{Blip, BlipSound, BlipTex, Jukebox, BLIP_BLACK, BLIP_BLUE, BLIP_CYAN, BLIP_DARKGRAY, BLIP_GRAY, BLIP_GREEN, BLIP_MAGENTA, BLIP_ORANGE, BLIP_RED, BLIP_WHITE, BLIP_YELLOW, NEON_CYAN};
use macroquad::{audio::load_sound_from_bytes, color::Color, texture::Texture2D};

#[cfg(target_arch = "wasm32")]
extern "C" {
    fn blip_basic_source_len() -> i32;
    fn blip_basic_source_copy(ptr: *mut u8, capacity: i32) -> i32;
    fn blip_basic_ready();
    fn blip_basic_program_loaded();
    fn blip_basic_runtime_error(ptr: *const u8, len: i32);
    fn blip_basic_sound(frequency: f32, seconds: f32, volume: f32);
    fn blip_basic_audio_event();
}

const WIDTH: i32 = 320;
const HEIGHT: i32 = 200;
include!(concat!(env!("OUT_DIR"), "/tape_gfx.rs"));
include!(concat!(env!("OUT_DIR"), "/rom_sfx.rs"));

fn conf() -> macroquad::window::Conf { blip::window_conf("BLIP BASIC", WIDTH, HEIGHT) }

#[derive(Clone, Debug, PartialEq)]
enum Value { Num(f64), Str(String), Color(Color) }
impl Value {
    fn number(&self) -> f64 { match self { Self::Num(n) => *n, Self::Str(s) => s.parse().unwrap_or(0.0), Self::Color(_) => 0.0 } }
    fn string(&self) -> String { match self { Self::Num(n) if n.fract() == 0.0 => format!("{n:.0}"), Self::Num(n) => n.to_string(), Self::Str(s) => s.clone(), Self::Color(_) => String::new() } }
}

#[derive(Clone, Debug, PartialEq)]
enum Token { Num(f64), Str(String), Name(String), Op(String), LParen, RParen, Comma }

fn tokenize(source: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = source.chars().collect();
    let mut out = Vec::new(); let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() { i += 1; continue; }
        if chars[i].is_ascii_digit() || chars[i] == '.' {
            let start = i; i += 1; while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') { i += 1; }
            out.push(Token::Num(chars[start..i].iter().collect::<String>().parse().map_err(|_| "Bad number".to_string())?)); continue;
        }
        if chars[i] == '"' {
            i += 1; let mut text = String::new(); let mut closed = false;
            while i < chars.len() { if chars[i] == '"' { if i + 1 < chars.len() && chars[i + 1] == '"' { text.push('"'); i += 2; } else { i += 1; closed = true; break; } } else { text.push(chars[i]); i += 1; } }
            if !closed { return Err("Unclosed string".into()); } out.push(Token::Str(text)); continue;
        }
        if chars[i].is_ascii_alphabetic() || chars[i] == '_' {
            let start = i; i += 1; while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '$') { i += 1; }
            out.push(Token::Name(chars[start..i].iter().collect::<String>().to_ascii_uppercase())); continue;
        }
        let pair = if i + 1 < chars.len() { format!("{}{}", chars[i], chars[i + 1]) } else { String::new() };
        if ["<=", ">=", "<>", "=="].contains(&pair.as_str()) { out.push(Token::Op(pair)); i += 2; continue; }
        match chars[i] { '(' => out.push(Token::LParen), ')' => out.push(Token::RParen), ',' => out.push(Token::Comma), '+' | '-' | '*' | '/' | '=' | '<' | '>' => out.push(Token::Op(chars[i].to_string())), c => return Err(format!("Unexpected character {c}")) }
        i += 1;
    }
    Ok(out)
}

fn split_statements(source: &str) -> Vec<String> {
    if source.trim_start().to_ascii_uppercase().starts_with("IF ") { return vec![source.trim().to_string()]; }
    let mut out = Vec::new(); let mut part = String::new(); let mut quoted = false; let mut depth = 0;
    for ch in source.chars() {
        if ch == '"' { quoted = !quoted; }
        if !quoted { if ch == '(' { depth += 1; } else if ch == ')' { depth -= 1; } }
        if ch == ':' && !quoted && depth == 0 { if !part.trim().is_empty() { out.push(part.trim().to_string()); } part.clear(); } else { part.push(ch); }
    }
    if !part.trim().is_empty() { out.push(part.trim().to_string()); } out
}

fn split_args(source: &str) -> Vec<String> {
    let mut out = Vec::new(); let mut part = String::new(); let mut quoted = false; let mut depth = 0;
    for ch in source.chars() {
        if ch == '"' { quoted = !quoted; }
        if !quoted { if ch == '(' { depth += 1; } else if ch == ')' { depth -= 1; } }
        if ch == ',' && !quoted && depth == 0 { out.push(part.trim().to_string()); part.clear(); } else { part.push(ch); }
    }
    out.push(part.trim().to_string()); out
}

#[derive(Clone)]
struct ForLoop { name: String, end: f64, step: f64, pc: usize, statement: usize }

#[derive(Clone, Copy, PartialEq)]
enum MusicRequest { Bouncer(usize), Defender(usize), Adder(usize), Serpent(usize), Rally(usize), Meteors(usize), SkyRaider(usize), Brawler(usize), Bubbler(usize), Stop }
impl MusicRequest {
    fn group(self) -> u8 { match self { Self::Bouncer(_) => 0, Self::Defender(_) => 1, Self::Adder(_) => 2, Self::Serpent(_) => 3, Self::Rally(_) => 4, Self::Meteors(_) => 5, Self::SkyRaider(_) => 6, Self::Brawler(_) => 7, Self::Bubbler(_) => 8, Self::Stop => 9 } }
}

struct Program {
    lines: Vec<(u16, String)>, statements: Vec<Vec<Arc<str>>>, expressions: HashMap<String, Arc<[Token]>>, pc: usize, statement: usize, vars: HashMap<String, Value>, arrays: HashMap<String, Value>, loops: Vec<ForLoop>, calls: Vec<(usize, usize)>, output: Vec<String>, wait_until: f64, yielded: bool, active: bool, error: Option<String>, rng: u64, pen: i32, paper: i32, music_request: Option<MusicRequest>, sound_request: Option<(f32, f32, f32)>,
}
impl Program {
    fn new() -> Self { Self { lines: vec![], statements: vec![], expressions: HashMap::new(), pc: 0, statement: 0, vars: HashMap::new(), arrays: HashMap::new(), loops: vec![], calls: vec![], output: vec![], wait_until: 0.0, yielded: false, active: false, error: None, rng: 0x5eed, pen:1, paper:0, music_request: None, sound_request: None } }
    fn load(&mut self, source: &str) {
        self.lines.clear(); self.statements.clear(); self.expressions.clear(); self.vars.clear(); self.arrays.clear(); self.loops.clear(); self.calls.clear(); self.output.clear(); self.error = None; self.active = true; self.pc = 0; self.statement = 0; self.wait_until = 0.0; self.yielded = false; self.paper=0; self.pen=1; self.music_request=None; self.sound_request=None;
        for line in source.lines() { let line=line.trim(); if line.is_empty() { continue; } if let Some((number, body))=line.split_once(char::is_whitespace) { if let Ok(n)=number.parse::<u16>() { self.lines.push((n,body.trim().to_string())); } } }
        self.lines.sort_by_key(|line| line.0);
        self.statements=self.lines.iter().map(|(_,source)|split_statements(source).into_iter().map(Arc::<str>::from).collect()).collect();
    }
    fn eval(&mut self, source: &str, blip: &Blip) -> Result<Value, String> {
        let tokens=if let Some(tokens)=self.expressions.get(source){Arc::clone(tokens)}else{let tokens:Arc<[Token]>=tokenize(source)?.into();self.expressions.insert(source.to_string(),Arc::clone(&tokens));tokens};let mut parser = Expr { tokens, at: 0, program: self, blip };
        let value = parser.expr(0)?; if parser.at != parser.tokens.len() { return Err("Unexpected expression token".into()); } Ok(value)
    }
    fn goto(&mut self, number: f64) -> Result<(), String> {
        let number = number as u16;
        self.pc = self.lines.iter().position(|(n,_)| *n == number).ok_or_else(|| format!("Line {number} not found"))?; self.statement=0; Ok(())
    }
    fn tick(&mut self, blip: &Blip, textures: &HashMap<String, BlipTex>, sounds: &HashMap<String, BlipSound>) {
        self.yielded = false;
        if !self.active || macroquad::time::get_time()*1000.0 < self.wait_until { return; }
        for _ in 0..5000 {
            if !self.active || self.pc >= self.statements.len() { self.active=false; return; }
            let line=self.lines[self.pc].0;
            if self.statement >= self.statements[self.pc].len() { self.pc+=1; self.statement=0; continue; }
            let current=self.statement; self.statement+=1;
            let pc=self.pc;let statement=self.statement;
            let source=Arc::clone(&self.statements[pc][current]);
            if let Err(error)=self.execute(&source, blip, textures, sounds) { self.error=Some(format!("? {error} at line {line}")); self.active=false; return; }
            if self.yielded { return; }
            if macroquad::time::get_time()*1000.0 < self.wait_until { return; }
            if self.pc!=pc||self.statement!=statement { continue; }
        }
        let line=self.lines.get(self.pc).map(|entry|entry.0).unwrap_or_default();
        let loop_state=self.loops.last().map(|frame|format!("{}={}",frame.name,self.vars.get(&frame.name).map(Value::string).unwrap_or_default())).unwrap_or_default();
        self.error=Some(format!("? Program took too long between WAIT commands at line {line} ({} GOSUB, {} FOR, {loop_state})",self.calls.len(),self.loops.len())); self.active=false;
    }
    fn execute(&mut self, source: &str, blip: &Blip, textures: &HashMap<String, BlipTex>, sounds: &HashMap<String, BlipSound>) -> Result<(), String> {
        let mut command=source.trim().to_string(); let upper=command.to_ascii_uppercase(); if command.is_empty() || upper=="REM" || upper.starts_with("REM ") || command.starts_with('\'') { return Ok(()); }
        if let Some((method,args))=method_call(&command) { command=format!("{} {}",method.to_ascii_uppercase(),args); }
        let upper=command.to_ascii_uppercase();
        if upper.starts_with("IF ") {
            let rest=&command[3..]; let then=rest.to_ascii_uppercase().find(" THEN ").ok_or("IF needs THEN")?;
            let condition=rest[..then].trim(); let body=rest[then+6..].trim();
            if self.eval(condition,blip)?.number()!=0.0 { for part in split_statements(body) { self.execute(&part,blip,textures,sounds)?; if !self.active { break; } } }
            return Ok(());
        }
        if let Some(rest)=command.strip_prefix("GOTO ").or_else(||command.strip_prefix("goto ")) { let n=self.eval(rest,blip)?.number(); return self.goto(n); }
        if let Some(rest)=command.strip_prefix("GOSUB ").or_else(||command.strip_prefix("gosub ")) { let n=self.eval(rest,blip)?.number();self.calls.push((self.pc,self.statement));return self.goto(n); }
        if upper=="RETURN" { let (pc,statement)=self.calls.pop().ok_or("RETURN without GOSUB")?;self.pc=pc;self.statement=statement;return Ok(()); }
        if upper.starts_with("FOR ") {
            let rest=&command[4..]; let eq=rest.find('=').ok_or("FOR needs =")?; let name=rest[..eq].trim().to_ascii_uppercase(); let tail=rest[eq+1..].trim();
            let to=tail.to_ascii_uppercase().find(" TO ").ok_or("FOR needs TO")?; let start=self.eval(tail[..to].trim(),blip)?.number(); let tail=&tail[to+4..]; let step_at=tail.to_ascii_uppercase().find(" STEP "); let end=self.eval(step_at.map_or(tail,|i|&tail[..i]).trim(),blip)?.number(); let step=if let Some(i)=step_at { self.eval(tail[i+6..].trim(),blip)?.number() } else { 1.0 };
            self.vars.insert(name.clone(),Value::Num(start)); self.loops.push(ForLoop{name,end,step,pc:self.pc,statement:self.statement}); return Ok(());
        }
        if upper.starts_with("NEXT") {
            let frame=self.loops.last().cloned().ok_or_else(||format!("NEXT {} without FOR",command[4..].trim()))?;
            if let Some(name)=command[4..].trim().split_whitespace().next() { if !name.is_empty() && name.to_ascii_uppercase()!=frame.name { return Err("NEXT variable mismatch".into()); } }
            let current=self.vars.get(&frame.name).map(Value::number).unwrap_or(0.0)+frame.step; self.vars.insert(frame.name.clone(),Value::Num(current));
            if frame.step!=0.0 && if frame.step>0.0 { current<=frame.end } else { current>=frame.end } { self.pc=frame.pc; self.statement=frame.statement; } else { self.loops.pop(); } return Ok(());
        }
        if upper=="END" || upper=="STOP" { self.active=false; return Ok(()); }
        if upper=="CLS" { self.output.clear(); blip.clear(paper_color(self.paper)); return Ok(()); }
        if upper.starts_with("PAPER ") { self.paper=self.eval(command[6..].trim(),blip)?.number() as i32;blip.clear(paper_color(self.paper));return Ok(()); }
        if upper.starts_with("PEN ")||upper.starts_with("INK ") { self.pen=self.eval(command[4..].trim(),blip)?.number() as i32;return Ok(()); }
        if let Some(rest)=command.strip_prefix("WAIT ").or_else(||command.strip_prefix("wait ")) { self.wait_until=macroquad::time::get_time()*1000.0+self.eval(rest,blip)?.number().clamp(0.0,10000.0); self.yielded=true; return Ok(()); }
        if upper.starts_with("SOUND ") { let args=split_args(command[6..].trim());if args.len()<2||args.len()>3{return Err("SOUND needs frequency,duration[,volume]".into())}let frequency=self.eval(&args[0],blip)?.number();let seconds=self.eval(&args[1],blip)?.number();let volume=args.get(2).map(|value|self.eval(value,blip).map(|v|v.number())).transpose()?.unwrap_or(0.35);if !(0.0..=12000.0).contains(&frequency)||!(0.0..=10.0).contains(&seconds)||!(0.0..=1.0).contains(&volume){return Err("SOUND values are out of range".into())}self.sound_request=Some((frequency as f32,seconds as f32,volume as f32));return Ok(()); }
        if upper.starts_with("REST ") { let seconds=self.eval(command[5..].trim(),blip)?.number();if !(0.0..=10.0).contains(&seconds){return Err("REST duration must be 0–10 seconds".into())}self.wait_until=macroquad::time::get_time()*1000.0+seconds*1000.0;self.yielded=true;return Ok(()); }
        if upper.starts_with("PRINT") { let text=command.get(5..).unwrap_or("").trim(); let value=if text.is_empty(){String::new()}else{split_args(text).iter().map(|p|self.eval(p,blip).map(|v|v.string())).collect::<Result<Vec<_>,_>>()?.join(" ")}; self.output.push(value); if self.output.len()>10 { self.output.remove(0); } return Ok(()); }
        if upper.starts_with("PLAY_SFX ") { let args=split_args(command[8..].trim()); if args.is_empty()||args.len()>2{return Err("PLAY_SFX needs name[,volume]".into())} let name=self.eval(&args[0],blip)?.string().to_ascii_lowercase(); let volume=args.get(1).map(|value|self.eval(value,blip).map(|v|v.number() as f32)).transpose()?.unwrap_or(1.0).clamp(0.0,1.0); let sound=sounds.get(&name).ok_or_else(||format!("Unknown sound {name}"))?;blip::play_sfx_volume(sound,volume);#[cfg(target_arch="wasm32")]unsafe{blip_basic_audio_event();}return Ok(()); }
        if upper.starts_with("PLAY_MUSIC ") { let name=self.eval(command[11..].trim(),blip)?.string().to_ascii_uppercase(); let level=self.vars.get("LV").or_else(||self.vars.get("ROUND")).map(Value::number).unwrap_or(1.0) as i32; self.music_request=Some(match name.as_str(){"BOUNCER_TITLE"=>MusicRequest::Bouncer(0),"BOUNCER_GAME"=>MusicRequest::Bouncer((level+1).rem_euclid(2) as usize),"BOUNCER_ALT"=>MusicRequest::Bouncer(1),"DEFENDER_MAIN"=>MusicRequest::Defender(0),"DEFENDER_BOSS"=>MusicRequest::Defender(1),"ADDER_COIL"=>MusicRequest::Adder(0),"ADDER_HUNT"=>MusicRequest::Adder(1),"SERPENT_MAIN"=>MusicRequest::Serpent(0),"SERPENT_FRENZY"=>MusicRequest::Serpent(1),"RALLY_MAIN"=>MusicRequest::Rally(0),"RALLY_ALT"=>MusicRequest::Rally(1),"METEORS_DRIFT"=>MusicRequest::Meteors(0),"METEORS_STORM"=>MusicRequest::Meteors(1),"SKY_RAIDER_MAIN"=>MusicRequest::SkyRaider(0),"SKY_RAIDER_ALT"=>MusicRequest::SkyRaider(1),"BRAWLER_ARENA"=>MusicRequest::Brawler(0),"BUBBLER_MAIN"=>MusicRequest::Bubbler(0),"BUBBLER_HURRY"=>MusicRequest::Bubbler(1),_=>return Err(format!("Unknown music track {name}"))}); return Ok(()); }
        if upper=="STOP_MUSIC" { self.music_request=Some(MusicRequest::Stop); return Ok(()); }
        if let Some(eq)=assignment_eq(&command) {
            let lhs=command[..eq].trim().strip_prefix("LET ").or_else(||command[..eq].trim().strip_prefix("let ")).unwrap_or(command[..eq].trim()).trim(); let value=self.eval(command[eq+1..].trim(),blip)?;
            if let Some(open)=lhs.find('(') { let name=lhs[..open].trim().to_ascii_uppercase(); let inds=lhs[open+1..].trim_end_matches(')').trim(); let key=split_args(inds).iter().map(|s|self.eval(s,blip).map(|v|v.string())).collect::<Result<Vec<_>,_>>()?.join(","); self.arrays.insert(format!("{name}({key})"),value); } else { self.vars.insert(lhs.to_ascii_uppercase(),value); } return Ok(());
        }
        self.draw_statement(&command,blip,textures)
    }
    fn draw_statement(&mut self, command: &str, blip: &Blip, textures: &HashMap<String, BlipTex>) -> Result<(), String> {
        let upper=command.to_ascii_uppercase();
        let rest=command.split_once(char::is_whitespace).map(|x|x.1.trim()).unwrap_or("");
        let args=split_args(rest);
        let vals=args.iter().map(|a|self.eval(a,blip)).collect::<Result<Vec<_>,_>>()?;
        let n=|i:usize| -> f32 { vals.get(i).map(Value::number).unwrap_or(0.0) as f32 };
        let c=|i:usize| -> Color { vals.get(i).map(value_color).unwrap_or(BLIP_WHITE) };
        let texture=|i:usize| -> Result<&BlipTex,String> { let requested=vals.get(i).map(Value::string).unwrap_or_default();let name=if requested.starts_with('/') {requested.to_ascii_uppercase()} else {requested.to_ascii_lowercase()};textures.get(&name).ok_or_else(||format!("Unknown texture {requested}")) };
        match upper.split_whitespace().next().unwrap_or("") {
            "CLEAR" => blip.clear(c(0)),
            "FILL_RECT" if vals.len()==5 => blip.fill_rect(n(0),n(1),n(2),n(3),c(4)),
            "DRAW_RECT" if vals.len()==5 => blip.draw_rect(n(0),n(1),n(2),n(3),c(4)),
            "DRAW_LINE" if vals.len()==5 => blip.draw_line(n(0),n(1),n(2),n(3),c(4)),
            "CIRCLE" if vals.len()==3 => { let (cx,cy,r)=(n(0),n(1),n(2).max(0.0));let color=paper_color(self.pen);let mut last=(cx+r,cy);for step in 1..=32 { let angle=std::f32::consts::TAU*step as f32/32.0;let point=(cx+angle.cos()*r,cy+angle.sin()*r);blip.draw_line(last.0,last.1,point.0,point.1,color);last=point; } },
            "DRAW_LINE_EX" if vals.len()==6 => blip.draw_line_ex(n(0),n(1),n(2),n(3),n(4),c(5)),
            "DRAW_GLOW_LINE" if vals.len()==5 => blip.draw_glow_line(n(0),n(1),n(2),n(3),c(4)),
            "FILL_CIRCLE" if vals.len()==4 => blip.fill_circle(n(0),n(1),n(2),c(3)),
            "FILL_CAPSULE" if vals.len()==7 => blip.fill_capsule(n(0),n(1),n(2),n(3),n(4),n(5),c(6)),
            "FILL_GLOW_CIRCLE" if vals.len()==4 => blip.fill_glow_circle(n(0),n(1),n(2),c(3)),
            "FILL_SUNBURST" if vals.len()==7 => blip.fill_sunburst(n(0),n(1),n(2).max(1.0) as usize,n(3),n(4),[c(5),c(6)]),
            "DRAW_HUD" if vals.len()==2 => blip.draw_hud(n(0) as i32,n(1) as i32),
            "DRAW_TEXTURE" if vals.len()==5 => blip.draw_texture(texture(0)?,n(1),n(2),n(3),n(4)),
            "DRAW_TEXTURE" if vals.len()==6 => blip.draw_texture_tinted(texture(0)?,n(1),n(2),n(3),n(4),c(5)),
            "DRAW_TEXTURE" if vals.len()==9 => blip.draw_texture_region(texture(0)?,n(5),n(6),n(7),n(8),n(1),n(2),n(3),n(4)),
            "DRAW_TEXTURE" if vals.len()==10 => blip.draw_texture_cell(texture(0)?,n(5).max(0.0) as u32,n(6).max(1.0) as u32,n(7).max(0.0) as u32,n(8).max(1.0) as u32,n(1),n(2),n(3),n(4),n(9)),
            "DRAW_CHAR" if vals.len()==5 => blip.draw_char(vals[0].string().to_ascii_uppercase().chars().next().unwrap_or(' '),n(1),n(2),n(3),c(4)),
            "DRAW_TEXT" if (5..=7).contains(&vals.len()) => {
                let text=vals[0].string().to_ascii_uppercase();
                let align=vals.get(5).map(Value::string).unwrap_or_else(||"LEFT".into()).to_ascii_uppercase();
                match (align.as_str(),vals.get(6)) {
                    ("LEFT",None)=>blip.draw_text(&text,n(1),n(2),n(3),c(4)),
                    ("LEFT",Some(_))=>blip.draw_text_outlined(&text,n(1),n(2),n(3),c(4),c(6)),
                    ("CENTER",None)=>blip.draw_text(&text,n(1)-text.chars().count() as f32*3.0*n(3),n(2),n(3),c(4)),
                    ("CENTER",Some(_))=>blip.draw_centered_outlined(&text,n(1),n(2),n(3),c(4),c(6)),
                    _=>return Err("DRAW_TEXT alignment must be LEFT or CENTER".into()),
                }
            }
            "DRAW_NUMBER" if vals.len()==5 => blip.draw_number(n(0) as i32,n(1),n(2),n(3),c(4)),
            op => return Err(format!("Unknown command or bad arguments: {op}")),
        }
        Ok(())
    }
    fn draw_output(&self, blip: &Blip) {
        for (i,line) in self.output.iter().enumerate() { blip.draw_text(&line.to_ascii_uppercase(),4.0,4.0+i as f32*16.0,2.0,BLIP_WHITE); }
        if let Some(error)=&self.error { blip.draw_text(&error.to_ascii_uppercase(),4.0,180.0,1.0,BLIP_RED); }
    }
}

struct Expr<'p, 'b> { tokens: Arc<[Token]>, at: usize, program: &'p mut Program, blip: &'b Blip }
impl Expr<'_, '_> {
    fn expr(&mut self,min:u8)->Result<Value,String>{
        let mut left=self.atom()?;
        loop { let op=match self.tokens.get(self.at){Some(Token::Op(op))=>op.clone(),Some(Token::Name(n)) if n=="AND"||n=="OR"=>n.clone(),_=>break}; let prec=match op.as_str(){"OR"=>1,"AND"=>2,"="|"=="|"<>"|"<"|">"|"<="|">="=>3,"+"|"-"=>4,"*"|"/"=>5,_=>0};if prec<min{break}self.at+=1;let right=self.expr(prec+1)?;left=match op.as_str(){"+" if matches!(left,Value::Str(_))||matches!(right,Value::Str(_))=>Value::Str(left.string()+&right.string()),"+"=>Value::Num(left.number()+right.number()),"-"=>Value::Num(left.number()-right.number()),"*"=>Value::Num(left.number()*right.number()),"/"=>Value::Num(left.number()/right.number()),"="|"=="=>Value::Num((left==right) as u8 as f64),"<>"=>Value::Num((left!=right) as u8 as f64),"<"=>Value::Num((left.number()<right.number()) as u8 as f64),">"=>Value::Num((left.number()>right.number()) as u8 as f64),"<="=>Value::Num((left.number()<=right.number()) as u8 as f64),">="=>Value::Num((left.number()>=right.number()) as u8 as f64),"AND"=>Value::Num((left.number()!=0.0&&right.number()!=0.0) as u8 as f64),"OR"=>Value::Num((left.number()!=0.0||right.number()!=0.0) as u8 as f64),_=>unreachable!()};}
        Ok(left)
    }
    fn atom(&mut self)->Result<Value,String>{
        let token=self.tokens.get(self.at).cloned().ok_or("Expression expected")?;self.at+=1;
        match token {
            Token::Num(n)=>Ok(Value::Num(n)),Token::Str(s)=>Ok(Value::Str(s)),Token::Op(op) if op=="-"=>Ok(Value::Num(-self.atom()?.number())),Token::Op(op) if op=="+"=>Ok(Value::Num(self.atom()?.number())),Token::LParen=>{let v=self.expr(0)?;self.need_rparen()?;Ok(v)},
            Token::Name(name)=>{
                if name=="INKEY$" { return Ok(Value::Str(if blip::input::btn1_pressed(){" ".into()}else{String::new()})); }
                if matches!(self.tokens.get(self.at),Some(Token::LParen)) { self.at+=1;let mut args=Vec::new();if !matches!(self.tokens.get(self.at),Some(Token::RParen)){loop{args.push(self.expr(0)?);if !matches!(self.tokens.get(self.at),Some(Token::Comma)){break}self.at+=1;}}self.need_rparen()?;
                    if let Some(first)=name.chars().next(){if first.is_ascii_alphabetic()||first=='_' { if !matches!(name.as_str(),"INT"|"ABS"|"SQR"|"SIN"|"COS"|"RND"|"STR$"|"LEN"|"KEY_ACTIVE"|"KEY_HELD"|"KEY_PRESSED"|"TEXT_CX"|"COLOR") {let key=args.iter().map(Value::string).collect::<Vec<_>>().join(",");return Ok(self.program.arrays.get(&format!("{name}({key})")).cloned().unwrap_or_else(||if name.ends_with('$'){Value::Str(String::new())}else{Value::Num(0.0)}));} } }
                    let a=args.first().cloned().unwrap_or(Value::Num(0.0));return match name.as_str(){"INT"=>Ok(Value::Num(a.number().floor())),"ABS"=>Ok(Value::Num(a.number().abs())) ,"SQR"=>Ok(Value::Num(a.number().sqrt())),"SIN"=>Ok(Value::Num(a.number().sin())),"COS"=>Ok(Value::Num(a.number().cos())),"RND"=>{let max=args.first().map(Value::number).unwrap_or(1.0);self.program.rng=self.program.rng.wrapping_mul(6364136223846793005).wrapping_add(1);Ok(Value::Num(((self.program.rng>>33) as f64/(u32::MAX as f64))*max))},"STR$"=>Ok(Value::Str(a.string())),"LEN"=>Ok(Value::Num(a.string().chars().count() as f64)),"COLOR"=>{if args.len()<3||args.len()>4{return Err("COLOR needs red,green,blue[,alpha]".into())}Ok(Value::Color(Color::new(args[0].number() as f32,args[1].number() as f32,args[2].number() as f32,args.get(3).map(Value::number).unwrap_or(1.0) as f32)))},"KEY_ACTIVE"|"KEY_HELD"|"KEY_PRESSED"=>{let code=a.string().to_ascii_uppercase();let key=match code.as_str(){"LEFT"=>blip::input::BLIP_KEY_LEFT,"RIGHT"=>blip::input::BLIP_KEY_RIGHT,"UP"=>blip::input::BLIP_KEY_UP,"DOWN"=>blip::input::BLIP_KEY_DOWN,"SPACE"|"FIRE"=>blip::input::BLIP_KEY_SPACE,"A"=>blip::input::BLIP_KEY_A,"D"=>blip::input::BLIP_KEY_D,"W"=>blip::input::BLIP_KEY_W,"S"=>blip::input::BLIP_KEY_S,"F"=>blip::input::BLIP_KEY_F,"G"=>blip::input::BLIP_KEY_G,"I"=>macroquad::input::KeyCode::I,"J"=>blip::input::BLIP_KEY_J,"K"=>blip::input::BLIP_KEY_K,"BUTTON2"|"Z"=>blip::input::BLIP_KEY_BUTTON2,_=>return Err(format!("Unknown key {code}"))};let down=match name.as_str(){"KEY_ACTIVE"=>blip::input::key_active(key),"KEY_HELD"=>blip::input::key_held(key),_=>blip::input::key_pressed(key)};Ok(Value::Num(down as u8 as f64))},"TEXT_CX"=>{if args.len()!=2{return Err("TEXT_CX needs text,size".into())}Ok(Value::Num(self.blip.text_cx(&args[0].string(),args[1].number() as i32) as f64))},_=>Err(format!("Unknown function {name}"))};
                }
                Ok(self.program.vars.get(&name).cloned().unwrap_or_else(||if name.ends_with('$'){Value::Str(String::new())}else{Value::Num(0.0)}))
            }
            _=>Err("Expression expected".into()),
        }
    }
    fn need_rparen(&mut self)->Result<(),String>{if matches!(self.tokens.get(self.at),Some(Token::RParen)){self.at+=1;Ok(())}else{Err("Expected )".into())}}
}

fn method_call(command:&str)->Option<(String,String)>{
    let command=command.trim();let start=command.find('(')?;if !command.ends_with(')'){return None;}let raw=command[..start].trim();let name=raw.to_ascii_lowercase();
    let methods=["clear","fill_rect","draw_rect","draw_line","draw_line_ex","draw_glow_line","fill_circle","fill_capsule","fill_sunburst","fill_glow_circle","draw_texture","draw_texture_region","draw_texture_cell","draw_texture_tinted","draw_char","draw_text","draw_text_outlined","draw_centered_outlined","draw_number","draw_centered","draw_hud"];
    methods.contains(&name.as_str()).then(||(name,command[start+1..command.len()-1].to_string()))
}
fn assignment_eq(source:&str)->Option<usize>{let mut quoted=false;let mut depth=0;let cs:Vec<char>=source.chars().collect();for i in 0..cs.len(){if cs[i]=='"'{quoted=!quoted;}if !quoted{if cs[i]=='(' {depth+=1} else if cs[i]==')'{depth-=1} else if cs[i]=='='&&depth==0&&cs.get(i+1)!=Some(&'=')&&cs.get(i.wrapping_sub(1))!=Some(&'<')&&cs.get(i.wrapping_sub(1))!=Some(&'>'){return Some(i)}}}None}
fn value_color(value:&Value)->Color{
    match value {
        Value::Color(color)=>*color,
        Value::Num(number)=>paper_color(*number as i32),
        Value::Str(text)=>{
            let name=text.trim().to_ascii_uppercase();
            let hex=name.strip_prefix('#');
            if let Some(hex)=hex {
                if hex.len()==6 || hex.len()==8 {
                    if let Ok(bits)=u32::from_str_radix(hex,16) {
                        let (red,green,blue,alpha)=if hex.len()==8 {
                            ((bits>>24)&255,(bits>>16)&255,(bits>>8)&255,bits&255)
                        } else {(bits>>16,(bits>>8)&255,bits&255,255)};
                        return Color::new(red as f32/255.0,green as f32/255.0,blue as f32/255.0,alpha as f32/255.0);
                    }
                }
            }
            match name.as_str(){"BLIP_BLACK"=>BLIP_BLACK,"BLIP_WHITE"=>BLIP_WHITE,"BLIP_RED"=>BLIP_RED,"BLIP_GREEN"=>BLIP_GREEN,"BLIP_BLUE"=>BLIP_BLUE,"BLIP_CYAN"=>BLIP_CYAN,"BLIP_MAGENTA"=>BLIP_MAGENTA,"BLIP_YELLOW"=>BLIP_YELLOW,"BLIP_ORANGE"=>BLIP_ORANGE,"BLIP_GRAY"=>BLIP_GRAY,"BLIP_DARKGRAY"=>BLIP_DARKGRAY,"NEON_CYAN"=>NEON_CYAN,"NEON_MAGENTA"=>blip::NEON_MAGENTA,"NEON_PINK"=>blip::NEON_PINK,"NEON_PURPLE"=>blip::NEON_PURPLE,"NEON_GREEN"=>blip::NEON_GREEN,"NEON_ORANGE"=>blip::NEON_ORANGE,"NEON_YELLOW"=>blip::NEON_YELLOW,_=>BLIP_WHITE}
        }
    }
}
fn paper_color(index:i32)->Color{[Color::new(0.06,0.11,0.64,1.),Color::new(1.,0.96,0.64,1.),BLIP_CYAN,Color::new(1.,0.34,0.59,1.),Color::new(0.65,0.92,0.42,1.),BLIP_ORANGE,BLIP_GRAY,Color::new(0.07,0.07,0.07,1.)][index.rem_euclid(8) as usize]}

fn host_source()->Option<String>{
    #[cfg(target_arch="wasm32")] unsafe { let len=blip_basic_source_len();if len<=0{return None}let mut bytes=vec![0;len as usize];let copied=blip_basic_source_copy(bytes.as_mut_ptr(),len);bytes.truncate(copied.max(0) as usize);String::from_utf8(bytes).ok() }
    #[cfg(not(target_arch="wasm32"))] { Some(include_str!("../../../web/TAPE/BOUNCER.BLIP").to_string()) }
}

fn brawler_theme_0()->Vec<u8>{blip_assets::brawler::theme_wav(0)}
fn brawler_theme_1()->Vec<u8>{blip_assets::brawler::theme_wav(1)}
fn brawler_theme_2()->Vec<u8>{blip_assets::brawler::theme_wav(2)}
fn brawler_theme_3()->Vec<u8>{blip_assets::brawler::theme_wav(3)}
fn brawler_theme_4()->Vec<u8>{blip_assets::brawler::theme_wav(4)}
fn brawler_theme_5()->Vec<u8>{blip_assets::brawler::theme_wav(5)}
fn brawler_theme_6()->Vec<u8>{blip_assets::brawler::theme_wav(6)}
fn brawler_theme_7()->Vec<u8>{blip_assets::brawler::theme_wav(7)}

#[blip::macroquad::main(conf)]
async fn main(){
    let mut blip=Blip::new(WIDTH,HEIGHT);
    blip.set_frame_retention(true);
    #[cfg(target_arch="wasm32")]
    unsafe { blip_basic_ready(); }
    let mut textures=HashMap::new();
    for (path,bytes) in TAPE_GFX {
        let tex=Texture2D::from_file_with_format(bytes,Some(macroquad::prelude::ImageFormat::Png));
        tex.set_filter(macroquad::texture::FilterMode::Nearest);
        textures.insert(path.to_string(),tex.clone());
        let alias=path.rsplit('/').next().unwrap_or(path).trim_end_matches(".PNG").to_ascii_lowercase();
        textures.entry(alias).or_insert(tex);
        let parts: Vec<_> = path.split('/').collect();
        if parts.len() >= 5 {
            let prefix = if parts[3] == "GALACTIC_DEFENDER" { "GALACTIC" } else { parts[3] };
            let qualified = format!("{}_{}", prefix, parts[4].trim_end_matches(".PNG")).to_ascii_lowercase();
            if let Some(texture) = textures.get(*path).cloned() { textures.entry(qualified).or_insert(texture); }
        }
    }
    let mut sounds=HashMap::new();for (name,bytes) in SFX_NAMES.into_iter().zip(SFX_BYTES){if let Ok(sound)=load_sound_from_bytes(bytes).await{sounds.insert(name.to_string(),sound);}}
    let mut bouncer_music=Jukebox::new(&[blip_assets::bouncer::bounce_wav,blip_assets::bouncer::rebound_wav]);
    let mut defender_music=Jukebox::new(&[blip_assets::galactic_defender::invasion_wav,blip_assets::galactic_defender::mothership_wav]);
    let mut adder_music=Jukebox::new(&[blip_assets::adder::coil_wav,blip_assets::adder::hunt_wav]);
    let mut serpent_music=Jukebox::new(&[blip_assets::serpent::slither_wav,blip_assets::serpent::frenzy_wav]);
    let mut rally_music=Jukebox::new(&[blip_assets::rally::music,blip_assets::rally::music2,blip_assets::rally::music3,blip_assets::rally::music4,blip_assets::rally::music5]);
    let mut meteors_music=Jukebox::new(&[blip_assets::meteors::drift_wav,blip_assets::meteors::storm_wav]);
    let mut sky_raider_music=Jukebox::new(&[blip_assets::sky_raider::music,blip_assets::sky_raider::music2]);
    let mut brawler_music=Jukebox::new(&[brawler_theme_0,brawler_theme_1,brawler_theme_2,brawler_theme_3,brawler_theme_4,brawler_theme_5,brawler_theme_6,brawler_theme_7]);
    let mut bubbler_music=Jukebox::new(&[blip_assets::bubbler::theme_wav,blip_assets::bubbler::hurry_wav]);
    let mut active_music:Option<MusicRequest>=None;
    let mut program=Program::new();let mut old_source=String::new();let mut reported_error=String::new();
    loop {
        if let Some(source)=host_source(){if source!=old_source{old_source=source.clone();bouncer_music.stop();defender_music.stop();adder_music.stop();serpent_music.stop();rally_music.stop();meteors_music.stop();sky_raider_music.stop();brawler_music.stop();bubbler_music.stop();active_music=None;program.load(&source);reported_error.clear();#[cfg(target_arch="wasm32")]unsafe{blip_basic_program_loaded();}}}
        else if !old_source.is_empty(){old_source.clear();bouncer_music.stop();defender_music.stop();adder_music.stop();serpent_music.stop();rally_music.stop();meteors_music.stop();sky_raider_music.stop();brawler_music.stop();bubbler_music.stop();active_music=None;program=Program::new();reported_error.clear();}
        program.tick(&blip,&textures,&sounds);
        #[cfg(target_arch="wasm32")]
        if let Some((frequency,seconds,volume))=program.sound_request.take(){unsafe{blip_basic_sound(frequency,seconds,volume);}}
        if let Some(request)=program.music_request.take(){
            if request==MusicRequest::Stop { bouncer_music.stop();defender_music.stop();adder_music.stop();serpent_music.stop();rally_music.stop();meteors_music.stop();sky_raider_music.stop();brawler_music.stop();bubbler_music.stop();active_music=None; }
            else {
                if active_music!=Some(request) { #[cfg(target_arch="wasm32")]unsafe{blip_basic_audio_event();} }
                if active_music.map(MusicRequest::group)!=Some(request.group()){bouncer_music.stop();defender_music.stop();adder_music.stop();serpent_music.stop();rally_music.stop();meteors_music.stop();sky_raider_music.stop();brawler_music.stop();bubbler_music.stop();}
                match request {
                    MusicRequest::Bouncer(i)=>if bouncer_music.ready(i){bouncer_music.play(i)}else{bouncer_music.start(i).await},
                    MusicRequest::Defender(i)=>if defender_music.ready(i){defender_music.play(i)}else{defender_music.start(i).await},
                    MusicRequest::Adder(i)=>if adder_music.ready(i){adder_music.play(i)}else{adder_music.start(i).await},
                    MusicRequest::Serpent(i)=>if serpent_music.ready(i){serpent_music.play(i)}else{serpent_music.start(i).await},
                    MusicRequest::Rally(i)=>if rally_music.ready(i){rally_music.play(i)}else{rally_music.start(i).await},
                    MusicRequest::Meteors(i)=>if meteors_music.ready(i){meteors_music.play(i)}else{meteors_music.start(i).await},
                    MusicRequest::SkyRaider(i)=>if sky_raider_music.ready(i){sky_raider_music.play(i)}else{sky_raider_music.start(i).await},
                    MusicRequest::Brawler(i)=>if brawler_music.ready(i){brawler_music.play(i)}else{brawler_music.start(i).await},
                    MusicRequest::Bubbler(i)=>if bubbler_music.ready(i){bubbler_music.play(i)}else{bubbler_music.start(i).await},
                    MusicRequest::Stop=>unreachable!(),
                }
                active_music=Some(request);
            }
        }
        match active_music.map(MusicRequest::group){Some(0)=>{bouncer_music.warm_up().await;},Some(1)=>{defender_music.warm_up().await;},Some(2)=>{adder_music.warm_up().await;},Some(3)=>{serpent_music.warm_up().await;},Some(4)=>{rally_music.warm_up().await;},Some(5)=>{meteors_music.warm_up().await;},Some(6)=>{sky_raider_music.warm_up().await;},Some(7)=>{brawler_music.warm_up().await;},Some(8)=>{bubbler_music.warm_up().await;},_=>{}}
        #[cfg(target_arch="wasm32")]
        if let Some(error)=&program.error { if *error!=reported_error {reported_error=error.clone();unsafe { blip_basic_runtime_error(error.as_ptr(),error.len() as i32); }} }
        if !program.active { blip.clear(BLIP_BLACK); }
        program.draw_output(&blip);
        blip.next_frame(60).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn bouncer_source_loads_as_numbered_program(){let mut p=Program::new();p.load(include_str!("../../../web/TAPE/BOUNCER.BLIP"));assert!(p.lines.len()>80);assert_eq!(p.lines[0].0,10);assert_eq!(p.lines.last().unwrap().0,2510);assert!(p.lines.windows(2).all(|pair|pair[0].0<pair[1].0));let source=p.lines.iter().map(|(_,line)|line.as_str()).collect::<Vec<_>>().join(" ");assert!(source.contains("QUIET>5")&&source.contains("TARGETX=6+C*30+14.5")&&source.contains("GOSUB 2400")&&source.contains("ANGLE=0.024"));}
    #[test] fn defender_source_loads_as_numbered_program(){let mut p=Program::new();p.load(include_str!("../../../web/TAPE/DEFENDER.BLIP"));assert!(p.lines.len()>80);assert_eq!(p.lines[0].0,10);assert_eq!(p.lines.last().unwrap().0,1660);}
    #[test] fn serpent_source_loads_as_numbered_program(){let mut p=Program::new();p.load(include_str!("../../../web/TAPE/SERPENT.BLIP"));assert!(p.lines.len()>60);assert_eq!(p.lines[0].0,10);assert_eq!(p.lines.last().unwrap().0,1070);assert!(p.lines.windows(2).all(|pair|pair[0].0<pair[1].0));let source=p.lines.iter().map(|(_,line)|line.as_str()).collect::<Vec<_>>().join(" ");assert!(source.contains("NX>20")&&source.contains("NY>20")&&source.contains("GX=0 TO 160 STEP 8")&&source.contains("HEAD_X=10:HEAD_Y=10"));assert!(source.contains("FILL_CIRCLE(SX,SY")&&source.contains("LIVES=LIVES-1")&&source.contains("LEVEL=LEVEL+1"));assert!(source.contains("SCORE=SCORE+10*LEVEL*STREAK")&&source.contains("BONUS_TIME=6")&&source.contains("SERPENT/BONUS_EAT")&&source.contains("EVENLEVEL=INT(LEVEL/2)*2")&&source.contains("SHEARS_TIME=7")&&source.contains("BODY_LENGTH=BODY_LENGTH-4"));}
    #[test] fn rally_source_loads_as_numbered_program(){let mut p=Program::new();p.load(include_str!("../../../web/TAPE/RALLY.BLIP"));assert!(p.lines.len()>35);assert_eq!(p.lines[0].0,10);assert_eq!(p.lines.last().unwrap().0,820);assert!(p.lines.windows(2).all(|pair|pair[0].0<pair[1].0));let source=p.lines.iter().map(|(_,line)|line.as_str()).collect::<Vec<_>>().join(" ");assert!(source.contains("BALL_Y=139+SIN")&&source.contains("MODE=2 AND KEY_HELD(\"I\")")&&source.contains("MODE=2 AND KEY_HELD(\"K\")")&&source.contains("FILL_RECT(85,PY,4,24")&&source.contains("HITDIR=1")&&source.contains("BALL_SPEED=BALL_SPEED+0.0833")&&source.contains("REL*1.1")&&source.contains("WAIT 1200:GOTO 600"));}
    #[test] fn adder_source_loads_as_numbered_program(){let mut p=Program::new();p.load(include_str!("../../../web/TAPE/ADDER.BLIP"));assert!(p.lines.len()>50);assert_eq!(p.lines[0].0,10);assert_eq!(p.lines.last().unwrap().0,999);assert!(p.lines.windows(2).all(|pair|pair[0].0<pair[1].0));assert!(p.lines.iter().any(|(_,source)|source.contains("FILL_CIRCLE(SX,SY")));let source=p.lines.iter().map(|(_,line)|line.as_str()).collect::<Vec<_>>().join(" ");assert!(source.contains("LENGTH=LENGTH+1")&&source.contains("HOLE_LEFT=2")&&source.contains("H1_SPENT(SEGMENT)=H1_SPENT(SEGMENT-1)")&&source.contains("CANDIDATE=EDIR+CHOICE")&&source.contains("CLEARANCE=LOOK-1")&&source.contains("SCORE=SCORE+50*ROUND")&&source.contains("SCORE=SCORE+100")&&source.contains("WAIT 2000")&&source.contains("IF ROUND=4 THEN PLAY_MUSIC \"ADDER_HUNT\""));}
    #[test] fn meteors_source_loads_as_numbered_program(){let mut p=Program::new();p.load(include_str!("../../../web/TAPE/METEORS.BLIP"));assert!(p.lines.len()>80);assert_eq!(p.lines[0].0,10);assert_eq!(p.lines.last().unwrap().0,1980);assert!(p.lines.windows(2).all(|pair|pair[0].0<pair[1].0));let source=p.lines.iter().map(|(_,line)|line.as_str()).collect::<Vec<_>>().join(" ");assert!(source.contains("FOR I=1 TO 48")&&source.contains("DRAW_LINE(FX,FY")&&source.contains("SPLIT_R*0.62"));assert!(source.contains("FOR VERT=1 TO 9")&&source.contains("JAG(I,VERT)=0.75+RND(0.5)")&&source.contains("ROT+VERT*0.628319"));assert!(source.contains("SCORE=SCORE+20")&&source.contains("SCORE=SCORE+100"));assert!(source.contains("METEORS/THRUST")&&source.contains("METEORS/HYPERSPACE")&&source.contains("METEORS/SAUCER_BIG"));}
    #[test] fn sky_raider_source_loads_as_numbered_program(){
        let mut p=Program::new();
        p.load(include_str!("../../../web/TAPE/SKY_RAIDER.BLIP"));
        assert!(p.lines.len()>100);
        assert_eq!(p.lines[0].0,10);
        assert_eq!(p.lines.last().unwrap().0,1970);
        assert!(p.lines.windows(2).all(|pair|pair[0].0<pair[1].0));
        let source=p.lines.iter().map(|(_,line)|line.as_str()).collect::<Vec<_>>().join(" ");
        assert!(source.contains("ENEMY_WEAVER.PNG")&&source.contains("ENEMY_ACE.PNG"));
        assert!(source.contains("FORMATION_TYPE=INT(RND(3))+1")&&source.contains("FORMATION_X+WING*34")&&source.contains("FORMATION_DIR"));
        assert!(source.contains("SPAWN>80")&&source.contains("HIT_WAIT=70")&&source.contains("SCROLL=SCROLL+1")&&source.contains("KILLS>=60+15*(LEVEL-1)")&&source.contains("BOSS_WARNING")&&source.contains("BOSS_IMAGE$=")&&source.contains("LEVEL=LEVEL+1")&&source.contains("WEAPON=WEAPON+1")&&source.contains("SCORE=SCORE+30*WEAPON")&&source.contains("MAX_POWER=120")&&source.contains("SKY_RAIDER/POWERUP3")&&source.contains("SKY_RAIDER/POWERUP4")&&source.contains("SKY_RAIDER/MAX_POWER")&&source.contains("GUNS=8")&&source.contains("/TAPE/GFX/SKY_RAIDER/POWERUP.PNG"));
        assert!(source.contains("DRAW_TEXTURE(\"/TAPE/GFX/SKY_RAIDER/PLAYER_PLANE.PNG\",PX,PY,33,29)")&&source.contains("DRAW_HUD(SCORE,LIVES)"));
        assert!(source.contains("COLOR(0.10-0.07*DEPTH,0.24-0.13*DEPTH,0.46-0.16*DEPTH)")&&source.contains("EBVX(Q)=DX/DIST*1.8")&&source.contains("SKY_RAIDER/ENEMY_GUN"));
    }
    #[test] fn brawler_source_loads_as_numbered_program(){let mut p=Program::new();p.load(include_str!("../../../web/TAPE/BRAWLER.BLIP"));assert!(p.lines.len()>35);assert_eq!(p.lines[0].0,10);assert_eq!(p.lines.last().unwrap().0,1530);assert!(p.lines.windows(2).all(|pair|pair[0].0<pair[1].0));assert!(p.lines.iter().any(|(_,source)|source.contains("DRAW_LINE_EX(160+COS")));let source=p.lines.iter().map(|(_,line)|line.as_str()).collect::<Vec<_>>().join(" ");assert!(source.contains("KEY_PRESSED(\"F\")")&&source.contains("KEY_PRESSED(\"G\")")&&source.contains("KEY_PRESSED(\"Z\")")&&source.contains("PUNCH_BUFFER>0")&&source.contains("KICK_BUFFER>0"));assert!(source.contains("ROUND_TIME=45")&&source.contains("ROUND_TIME<=0")&&source.contains("PLAYER_HP>CPU_HP")&&source.contains("DRAW_TEXT(NAME$(PICK),25,27,1,\"BLIP_CYAN\",\"LEFT\",\"BLIP_BLACK\")"));}
    #[test] fn bubbler_source_loads_as_numbered_program(){
        let mut p=Program::new();
        p.load(include_str!("../../../web/TAPE/BUBBLER.BLIP"));
        assert!(p.lines.len()>75);
        assert_eq!(p.lines[0].0,10);
        assert_eq!(p.lines.last().unwrap().0,1980);
        assert!(p.lines.windows(2).all(|pair|pair[0].0<pair[1].0));
        let source=p.lines.iter().map(|(_,line)|line.as_str()).collect::<Vec<_>>().join(" ");
        assert!(source.contains("FILL_GLOW_CIRCLE(82,166")&&source.contains("EVY(E)=EVY(E)+0.09")&&source.contains("EVY(E)=0-2.6"));
        assert!(source.contains("PYLINE(P)=8+PROW*8")&&source.contains("PXLINE(P)=PCOL*12")&&source.contains("ENEMY_COUNT=15"));
        assert!(source.contains("EY_OLD(E)+16<=PYLINE(P)")&&source.contains("ETYPE(E)=EREST-ECOL*4")&&source.contains("EHOME(E)+SIN(T*3+E)*4"));
        assert!(source.contains("ROUND>8 THEN GOTO 690")&&source.contains("ROUND_TIME>HURRY_AT")&&source.contains("BT(B)=630-30*ROUND")&&source.contains("EANGRY(TRAPPED)=1")&&source.contains("#FFC3DC"));
        assert!(source.contains("SHOT_KIND(BSLOT)=2")&&source.contains("ROUND>=4 OR EANGRY(E)=1")&&source.contains("ETYPE(E)=3 AND FIRE_CD(E)<=0")&&source.contains("BUBBLER/SPARKLE"));
    }
    #[test] fn primitive_calls_use_the_crate_method_name(){let expected=Some(("fill_rect".into(),"1,2,3,4,color".into()));assert_eq!(method_call("fill_rect(1,2,3,4,color)"),expected);assert_eq!(method_call("FILL_RECT(1,2,3,4,color)"),expected);assert_eq!(method_call("BLIP.fill_rect(1,2,3,4,color)"),None);}
    #[test] fn hex_colors_support_rgb_and_rgba(){assert_eq!(value_color(&Value::Str("#336699".into())),Color::new(0x33 as f32/255.0,0x66 as f32/255.0,0x99 as f32/255.0,1.0));assert_eq!(value_color(&Value::Str("#33669980".into())).a,128.0/255.0);}
}
