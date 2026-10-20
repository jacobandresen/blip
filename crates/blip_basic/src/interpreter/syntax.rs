//! Split source outside strings and parentheses before parsing individual expressions.

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Token {
    Num(f64),
    Str(String),
    Name(String),
    Op(String),
    LParen,
    RParen,
    Comma,
}

pub(super) fn tokenize(source: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = source.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        if chars[i].is_ascii_digit() || chars[i] == '.' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            out.push(Token::Num(
                chars[start..i]
                    .iter()
                    .collect::<String>()
                    .parse()
                    .map_err(|_| "Bad number".to_string())?,
            ));
            continue;
        }
        if chars[i] == '"' {
            i += 1;
            let mut text = String::new();
            let mut closed = false;
            while i < chars.len() {
                if chars[i] == '"' {
                    if i + 1 < chars.len() && chars[i + 1] == '"' {
                        text.push('"');
                        i += 2;
                    } else {
                        i += 1;
                        closed = true;
                        break;
                    }
                } else {
                    text.push(chars[i]);
                    i += 1;
                }
            }
            if !closed {
                return Err("Unclosed string".into());
            }
            out.push(Token::Str(text));
            continue;
        }
        if chars[i].is_ascii_alphabetic() || chars[i] == '_' {
            let start = i;
            i += 1;
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '$')
            {
                i += 1;
            }
            out.push(Token::Name(
                chars[start..i]
                    .iter()
                    .collect::<String>()
                    .to_ascii_uppercase(),
            ));
            continue;
        }
        let pair = if i + 1 < chars.len() {
            format!("{}{}", chars[i], chars[i + 1])
        } else {
            String::new()
        };
        if ["<=", ">=", "<>", "=="].contains(&pair.as_str()) {
            out.push(Token::Op(pair));
            i += 2;
            continue;
        }
        match chars[i] {
            '(' => out.push(Token::LParen),
            ')' => out.push(Token::RParen),
            ',' => out.push(Token::Comma),
            '+' | '-' | '*' | '/' | '=' | '<' | '>' => out.push(Token::Op(chars[i].to_string())),
            c => return Err(format!("Unexpected character {c}")),
        }
        i += 1;
    }
    Ok(out)
}

fn top_level(source: &str) -> impl Iterator<Item = (usize, char)> + '_ {
    let mut quoted = false;
    let mut depth = 0;
    source.char_indices().filter(move |&(_, ch)| {
        match ch {
            '"' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => depth -= 1,
            _ => return !quoted && depth == 0,
        }
        false
    })
}

fn split_top_level(source: &str, separator: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut start = 0;
    for (at, ch) in top_level(source) {
        if ch != separator {
            continue;
        }
        if separator == ':' {
            // THEN actions and comments both own the remainder of the line.
            let rest = source[start..].trim_start();
            let keyword = rest
                .split(|ch: char| ch.is_whitespace() || ch == ':')
                .next();
            if rest.starts_with('\'')
                || keyword.is_some_and(|word| {
                    word.eq_ignore_ascii_case("REM") || word.eq_ignore_ascii_case("IF")
                })
            {
                break;
            }
        }
        parts.push(source[start..at].trim().to_owned());
        start = at + ch.len_utf8();
    }
    parts.push(source[start..].trim().to_owned());
    parts
}

pub(super) fn split_statements(source: &str) -> Vec<String> {
    split_top_level(source, ':')
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect()
}

pub(super) fn split_args(source: &str) -> Vec<String> {
    split_top_level(source, ',')
}

pub(super) fn method_call(command: &str) -> Option<(String, String)> {
    let command = command.trim();
    let start = command.find('(')?;
    if !command.ends_with(')') {
        return None;
    }
    let raw = command[..start].trim();
    let name = raw.to_ascii_lowercase();
    let methods = [
        "canvas",
        "clear",
        "fill_rect",
        "draw_rect",
        "draw_line",
        "draw_line_ex",
        "draw_polyline",
        "fx_clear",
        "fx_update",
        "fx_draw",
        "fx_burst",
        "fx_popup",
        "fx_ring",
        "draw_glow_line",
        "fill_circle",
        "fill_capsule",
        "fill_sunburst",
        "fill_glow_circle",
        "draw_texture",
        "draw_texture_region",
        "draw_texture_cell",
        "draw_texture_tinted",
        "draw_char",
        "draw_text",
        "draw_text_outlined",
        "draw_centered_outlined",
        "draw_number",
        "draw_centered",
        "draw_hud",
    ];
    methods
        .contains(&name.as_str())
        .then(|| (name, command[start + 1..command.len() - 1].to_string()))
}
pub(super) fn assignment_eq(source: &str) -> Option<usize> {
    let bytes = source.as_bytes();
    top_level(source).find_map(|(at, ch)| {
        let comparison = bytes
            .get(at.wrapping_sub(1))
            .is_some_and(|ch| b"<>=".contains(ch))
            || bytes.get(at + 1) == Some(&b'=');
        (ch == '=' && !comparison).then_some(at)
    })
}
