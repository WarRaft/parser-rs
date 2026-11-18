//! A character cursor for hand-written lexers: one-`char`-at-a-time access
//! with 1-indexed, codepoint-counted line/column tracking that lines up
//! with [`crate::Span`]'s native representation.

use crate::span::Span;
use crate::token::ParseError;

pub struct Cursor {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Cursor {
    pub fn new(src: &str) -> Self {
        Cursor::new_at(src, 1)
    }

    /// A cursor whose first line is numbered `first_line`: for lexing a
    /// region cut out of a larger file (an embedded block) while keeping
    /// that file's line numbers. The region must start at a line boundary,
    /// so columns need no offset.
    pub fn new_at(src: &str, first_line: usize) -> Self {
        Cursor {
            chars: src.chars().collect(),
            pos: 0,
            line: first_line,
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

    /// Whether only whitespace precedes the cursor on its line.
    pub fn at_line_start(&self) -> bool {
        self.chars[..self.pos]
            .iter()
            .rev()
            .take_while(|&&c| c != '\n')
            .all(|c| c.is_whitespace())
    }

    /// The text from the cursor to the end of its line (not consumed, `\n`
    /// excluded).
    pub fn rest_of_line(&self) -> String {
        self.chars[self.pos..]
            .iter()
            .take_while(|&&c| c != '\n')
            .collect()
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
        self.skip_trivia_collecting(line_comment, block_comment, None, &|_| false)
    }

    /// [`skip_trivia`](Self::skip_trivia) that also records each skipped
    /// comment's span into `comments` (block comments may span lines). An
    /// unterminated block comment is still recorded, up to end of input.
    /// Stops early, before consuming anything, wherever `stop` holds (e.g.
    /// on an embedded-language directive that looks like a comment).
    pub fn skip_trivia_collecting(
        &mut self,
        line_comment: &str,
        block_comment: Option<(&str, &str)>,
        mut comments: Option<&mut Vec<Span>>,
        stop: &dyn Fn(&Cursor) -> bool,
    ) -> Result<(), ParseError> {
        loop {
            if stop(self) {
                return Ok(());
            }
            let (line, col) = (self.line, self.col);
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.advance();
                    continue;
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
                        self.eat_str(open);
                        while !self.eat_str(close) {
                            if self.advance().is_none() {
                                if let Some(out) = comments.as_deref_mut() {
                                    out.push(Span::new(line, col, self.line, self.col));
                                }
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
            if let Some(out) = comments.as_deref_mut() {
                out.push(Span::new(line, col, self.line, self.col));
            }
        }
    }

}

#[cfg(test)]
#[path = "cursor.test.rs"]
mod tests;
