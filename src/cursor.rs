//! A character cursor for hand-written lexers: one-`char`-at-a-time access
//! with 1-indexed, codepoint-counted line/column tracking that lines up
//! with [`crate::Span`]'s native representation.

use crate::token::ParseError;

pub struct Cursor {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Cursor {
    pub fn new(src: &str) -> Self {
        Cursor {
            chars: src.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn col(&self) -> usize {
        self.col
    }

    pub fn is_eof(&self) -> bool {
        self.pos >= self.chars.len()
    }

    pub fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    pub fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    /// Whether the upcoming characters spell `s`.
    pub fn starts_with(&self, s: &str) -> bool {
        s.chars().enumerate().all(|(i, c)| self.peek_at(i) == Some(c))
    }

    pub fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    /// Consumes `c` if it's next.
    pub fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Consumes `s` if the upcoming characters spell it.
    pub fn eat_str(&mut self, s: &str) -> bool {
        if self.starts_with(s) {
            for _ in s.chars() {
                self.advance();
            }
            true
        } else {
            false
        }
    }

    /// Consumes characters while `pred` holds and returns them.
    pub fn take_while(&mut self, pred: impl Fn(char) -> bool) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if !pred(c) {
                break;
            }
            s.push(c);
            self.advance();
        }
        s
    }

    /// Skips whitespace, `line_comment`-to-end-of-line comments, and (if
    /// given) `(open, close)` block comments. An unterminated block comment
    /// is an error located at its opening delimiter.
    pub fn skip_trivia(
        &mut self,
        line_comment: &str,
        block_comment: Option<(&str, &str)>,
    ) -> Result<(), ParseError> {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.advance();
                }
                _ if self.starts_with(line_comment) => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                _ => match block_comment {
                    Some((open, close)) if self.starts_with(open) => {
                        let (line, col) = (self.line, self.col);
                        self.eat_str(open);
                        while !self.eat_str(close) {
                            if self.advance().is_none() {
                                return Err(ParseError::at(
                                    "unterminated block comment",
                                    line,
                                    col,
                                ));
                            }
                        }
                    }
                    _ => return Ok(()),
                },
            }
        }
    }
}

#[cfg(test)]
#[path = "cursor.test.rs"]
mod tests;
