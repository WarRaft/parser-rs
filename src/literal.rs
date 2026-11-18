//! Scanners for the C-like literal forms shared by the WC3 script
//! languages (JASS, Zinc, ...): decimal/hex integers, reals, double-quoted
//! strings with backslash escapes, and single-quoted four-character codes
//! ("rawcodes", e.g. `'hfoo'`). Each scanner expects the cursor to sit on
//! the literal's first character and reports errors at `(line, col)` —
//! the literal's start, which the caller captured before calling in.

use crate::cursor::Cursor;
use crate::token::ParseError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Number {
    Int(i64),
    Real(f64),
}

/// Scans `0x1F`/`0X1F` hex, `123` decimal, or `1.5` real literals.
pub fn number(cur: &mut Cursor, line: usize, col: usize) -> Result<Number, ParseError> {
    if cur.peek() == Some('0') && matches!(cur.peek_at(1), Some('x') | Some('X')) {
        cur.advance();
        cur.advance();
        let s = cur.take_while(|c| c.is_ascii_hexdigit());
        return i64::from_str_radix(&s, 16)
            .map(Number::Int)
            .map_err(|_| ParseError::at(format!("invalid hex literal '0x{s}'"), line, col));
    }

    let mut s = cur.take_while(|c| c.is_ascii_digit());
    if cur.peek() == Some('.') && cur.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
        cur.advance();
        s.push('.');
        s.push_str(&cur.take_while(|c| c.is_ascii_digit()));
        return s
            .parse()
            .map(Number::Real)
            .map_err(|_| ParseError::at(format!("invalid real literal '{s}'"), line, col));
    }
    s.parse()
        .map(Number::Int)
        .map_err(|_| ParseError::at(format!("invalid integer literal '{s}'"), line, col))
}

/// Scans a `"..."` string, decoding `\n`, `\t`, `\"`, `\\` (any other
/// escaped character stands for itself). Strings can't span lines.
pub fn string(cur: &mut Cursor, line: usize, col: usize) -> Result<String, ParseError> {
    let unterminated = || ParseError::at("unterminated string literal", line, col);
    cur.advance(); // opening quote
    let mut s = String::new();
    loop {
        match cur.peek() {
            None | Some('\n') => return Err(unterminated()),
            Some('"') => {
                cur.advance();
                return Ok(s);
            }
            Some('\\') => {
                cur.advance();
                match cur.advance() {
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some(other) => s.push(other),
                    None => return Err(unterminated()),
                }
            }
            Some(c) => {
                s.push(c);
                cur.advance();
            }
        }
    }
}

/// Scans a `'abcd'` rawcode and packs its four bytes big-endian into an
/// integer, the way the game does.
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
    if s.len() != 4 {
        return Err(ParseError::at(
            format!("rawcode literal '{s}' must be exactly 4 characters"),
            line,
            col,
        ));
    }
    Ok(s.bytes().fold(0i64, |acc, b| (acc << 8) | b as i64))
}

#[cfg(test)]
#[path = "literal.test.rs"]
mod tests;
