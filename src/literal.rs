//! Scanners for the literal forms of the supported languages: numbers
//! (whose exact syntax a [`NumberSpec`] picks), quoted strings with
//! backslash escapes, four-character codes ("rawcodes", e.g. `'hfoo'`),
//! Lua long brackets (`[[...]]`, `[==[...]==]`) and AngelScript heredocs
//! (`"""..."""`). Each scanner expects the cursor to sit on the literal's
//! first character and reports errors at `(line, col)`, the literal's
//! start, which the caller captured before calling in.

use crate::cursor::Cursor;
use crate::token::ParseError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Number {
    Int(i64),
    Real(f64),
}

/// Which number forms a language has, beyond the always-on decimal
/// integers, `0x` hex integers, and `1.5` reals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NumberSpec {
    /// `1e10`, `1.5E-3` (Lua, AngelScript).
    pub exponent: bool,
    /// `.5` (JASS, Lua, AngelScript). The lexer checks for it, since the
    /// number starts at the `.`.
    pub leading_dot: bool,
    /// `0b1010` binary, `0o17` octal, `0d42` decimal (AngelScript).
    pub radix_prefixes: bool,
    /// A trailing `f`/`F`/`d`/`D` on a real: `1.5f` (AngelScript).
    pub float_suffix: bool,
    /// `2144.` reals: a dot with no digits after it (JASS).
    pub trailing_dot: bool,
    /// `017` octal integers: a leading `0` (JASS, like C). Reals stay
    /// decimal: `017.5` is 17.5.
    pub leading_zero_octal: bool,
    /// `$1F` hex integers (JASS). The lexer checks for it, since the
    /// number starts at the `$`.
    pub dollar_hex: bool,
}

/// Scans a number literal per `spec`. Hex integers wrap to 64 bits, like
/// Lua's `0xFFFFFFFFFFFFFFFF == -1` and JASS's `0xFFFFFFFF`.
pub fn number(cur: &mut Cursor, spec: &NumberSpec, line: usize, col: usize) -> Result<Number, ParseError> {
    if spec.dollar_hex && cur.peek() == Some('$') {
        cur.advance();
        let digits = cur.take_while(|c| c.is_ascii_hexdigit());
        return u64::from_str_radix(&digits, 16)
            .map(|v| Number::Int(v as i64))
            .map_err(|_| ParseError::at(format!("invalid literal '${digits}'"), line, col));
    }
    let radix = match (cur.peek(), cur.peek_at(1)) {
        (Some('0'), Some('x' | 'X')) => Some(16),
        (Some('0'), Some('b' | 'B')) if spec.radix_prefixes => Some(2),
        (Some('0'), Some('o' | 'O')) if spec.radix_prefixes => Some(8),
        (Some('0'), Some('d' | 'D')) if spec.radix_prefixes => Some(10),
        _ => None,
    };
    if let Some(radix) = radix {
        let prefix: String = [cur.advance().unwrap_or('0'), cur.advance().unwrap_or('x')]
            .iter()
            .collect();
        let digits = cur.take_while(|c| c.is_digit(radix) || c == '_');
        let clean = digits.replace('_', "");
        return u64::from_str_radix(&clean, radix)
            .map(|v| Number::Int(v as i64))
            .map_err(|_| ParseError::at(format!("invalid literal '{prefix}{digits}'"), line, col));
    }

    let mut s = cur.take_while(|c| c.is_ascii_digit());
    let mut is_real = false;
    if cur.peek() == Some('.') && cur.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
        cur.advance();
        s.push('.');
        s.push_str(&cur.take_while(|c| c.is_ascii_digit()));
        is_real = true;
    } else if spec.trailing_dot && !s.is_empty() && cur.peek() == Some('.') {
        cur.advance();
        is_real = true;
    }
    if spec.exponent && matches!(cur.peek(), Some('e' | 'E')) {
        let sign = matches!(cur.peek_at(1), Some('+' | '-'));
        let digit_at = if sign { 2 } else { 1 };
        if cur.peek_at(digit_at).is_some_and(|c| c.is_ascii_digit()) {
            s.push('e');
            cur.advance();
            if sign {
                s.push(cur.advance().unwrap_or('+'));
            }
            s.push_str(&cur.take_while(|c| c.is_ascii_digit()));
            is_real = true;
        }
    }
    if is_real && spec.float_suffix && matches!(cur.peek(), Some('f' | 'F' | 'd' | 'D')) {
        cur.advance();
    }
    if is_real {
        // `.5` arrives here as "" + ".5"
        let text = if s.starts_with('.') { format!("0{s}") } else { s.clone() };
        return text
            .parse()
            .map(Number::Real)
            .map_err(|_| ParseError::at(format!("invalid real literal '{s}'"), line, col));
    }
    if spec.leading_zero_octal && s.len() > 1 && s.starts_with('0') {
        return u64::from_str_radix(&s, 8)
            .map(|v| Number::Int(v as i64))
            .map_err(|_| ParseError::at(format!("invalid octal literal '{s}' (a leading 0 means octal: digits 0-7)"), line, col));
    }
    s.parse()
        .map(Number::Int)
        .map_err(|_| ParseError::at(format!("invalid integer literal '{s}'"), line, col))
}

/// Scans a `"..."` string.
pub fn string(cur: &mut Cursor, line: usize, col: usize) -> Result<String, ParseError> {
    quoted(cur, '"', line, col)
}

/// Scans a `"..."` string without escapes: everything up to the next `"`
/// on the line, backslashes included. A line break ends it with an error.
pub fn raw(cur: &mut Cursor, line: usize, col: usize) -> Result<String, ParseError> {
    cur.advance(); // opening quote
    let mut s = String::new();
    loop {
        match cur.peek() {
            None | Some('\n') => return Err(ParseError::at("unterminated string literal", line, col)),
            Some('"') => {
                cur.advance();
                return Ok(s);
            }
            Some(c) => {
                s.push(c);
                cur.advance();
            }
        }
    }
}

/// Scans a string delimited by `quote` (`"` or `'`), decoding escapes:
/// `\n \t \r \a \b \f \v \\ \" \'`, `\xHH`, `\ddd` (decimal, up to 3
/// digits), `\u{H..}` and `\uHHHH`. `\z` skips following whitespace (Lua),
/// and an escaped line break continues the string on the next line. Any
/// other escaped character stands for itself. An unescaped line break
/// ends the string with an error.
pub fn quoted(cur: &mut Cursor, quote: char, line: usize, col: usize) -> Result<String, ParseError> {
    let unterminated = || ParseError::at("unterminated string literal", line, col);
    cur.advance(); // opening quote
    let mut s = String::new();
    loop {
        match cur.peek() {
            None | Some('\n') => return Err(unterminated()),
            Some(c) if c == quote => {
                cur.advance();
                return Ok(s);
            }
            Some('\\') => {
                cur.advance();
                let Some(e) = cur.advance() else {
                    return Err(unterminated());
                };
                match e {
                    'n' => s.push('\n'),
                    't' => s.push('\t'),
                    'r' => s.push('\r'),
                    'a' => s.push('\u{7}'),
                    'b' => s.push('\u{8}'),
                    'f' => s.push('\u{c}'),
                    'v' => s.push('\u{b}'),
                    'z' => {
                        cur.take_while(char::is_whitespace);
                    }
                    'x' => {
                        let hex = take_up_to(cur, 2, |c| c.is_ascii_hexdigit());
                        push_code(&mut s, u32::from_str_radix(&hex, 16).ok());
                    }
                    'u' => {
                        let hex = if cur.eat('{') {
                            let h = cur.take_while(|c| c.is_ascii_hexdigit());
                            cur.eat('}');
                            h
                        } else {
                            take_up_to(cur, 4, |c| c.is_ascii_hexdigit())
                        };
                        push_code(&mut s, u32::from_str_radix(&hex, 16).ok());
                    }
                    d if d.is_ascii_digit() => {
                        let mut digits = d.to_string();
                        digits.push_str(&take_up_to(cur, 2, |c| c.is_ascii_digit()));
                        push_code(&mut s, digits.parse().ok());
                    }
                    other => s.push(other),
                }
            }
            Some(c) => {
                s.push(c);
                cur.advance();
            }
        }
    }
}

fn take_up_to(cur: &mut Cursor, n: usize, pred: impl Fn(char) -> bool) -> String {
    let mut s = String::new();
    while s.len() < n {
        match cur.peek() {
            Some(c) if pred(c) => {
                s.push(c);
                cur.advance();
            }
            _ => break,
        }
    }
    s
}

fn push_code(s: &mut String, code: Option<u32>) {
    s.push(code.and_then(char::from_u32).unwrap_or('\u{fffd}'));
}

/// Scans a `'abcd'` rawcode and packs its four bytes big-endian into an
/// integer, the way the game does. A one-character `'d'` is that
/// character's byte (100); any other length is an error.
pub fn rawcode(cur: &mut Cursor, line: usize, col: usize) -> Result<i64, ParseError> {
    cur.advance(); // opening quote
    let mut s = String::new();
    loop {
        match cur.peek() {
            None | Some('\n') => {
                return Err(ParseError::at("unterminated rawcode literal", line, col));
            }
            Some('\'') => {
                cur.advance();
                break;
            }
            Some(c) => {
                s.push(c);
                cur.advance();
            }
        }
    }
    if s.len() != 4 && s.len() != 1 {
        return Err(ParseError::at(
            format!("rawcode literal '{s}' must be 1 or 4 characters"),
            line,
            col,
        ));
    }
    Ok(s.bytes().fold(0i64, |acc, b| (acc << 8) | b as i64))
}

/// The level of a Lua long bracket opening `offset` characters ahead
/// (`[[` is level 0, `[==[` level 2), if one starts there.
pub fn long_bracket_level(cur: &Cursor, offset: usize) -> Option<usize> {
    if cur.peek_at(offset) != Some('[') {
        return None;
    }
    let mut level = 0;
    while cur.peek_at(offset + 1 + level) == Some('=') {
        level += 1;
    }
    (cur.peek_at(offset + 1 + level) == Some('[')).then_some(level)
}

/// Scans a Lua long bracket `[==[ ... ]==]` (the cursor on its first `[`)
/// and returns its contents verbatim, minus a newline right after the
/// opening bracket, as Lua does. May span lines.
pub fn long_bracket(cur: &mut Cursor, what: &str, line: usize, col: usize) -> Result<String, ParseError> {
    let level = long_bracket_level(cur, 0).unwrap_or(0);
    for _ in 0..level + 2 {
        cur.advance();
    }
    cur.eat('\r');
    cur.eat('\n');
    let close = format!("]{}]", "=".repeat(level));
    let mut s = String::new();
    while !cur.eat_str(&close) {
        match cur.advance() {
            Some(c) => s.push(c),
            None => return Err(ParseError::at(format!("unterminated {what}"), line, col)),
        }
    }
    Ok(s)
}

/// Scans an AngelScript heredoc `"""..."""` (the cursor on its first
/// quote). May span lines. Its contents are verbatim, with no escapes.
pub fn heredoc(cur: &mut Cursor, line: usize, col: usize) -> Result<String, ParseError> {
    cur.eat_str("\"\"\"");
    let mut s = String::new();
    while !cur.eat_str("\"\"\"") {
        match cur.advance() {
            Some(c) => s.push(c),
            None => return Err(ParseError::at("unterminated heredoc string", line, col)),
        }
    }
    Ok(s)
}

#[cfg(test)]
#[path = "literal.test.rs"]
mod tests;
