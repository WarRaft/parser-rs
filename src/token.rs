//! Generic token plumbing for recursive-descent parsers: a [`Token`] over
//! a language's own kind enum, the [`ParseError`] every stage reports, a
//! [`TokenStream`] cursor with the usual `peek`/`check`/`expect` helpers,
//! and the [`TokenParser`] trait that adds grammar-level helpers (e.g.
//! left-associative binary operator levels) to any parser holding one.

use std::fmt::{Debug, Display};

use crate::span::Span;

/// What a [`TokenStream`] needs from a token kind. Implemented for the
/// shared [`crate::TokenKind`]; `Display` is used in error messages.
pub trait TokenKindExt: Clone + PartialEq + Debug + Display {
    /// Whether this is the end-of-input marker every token list ends with.
    fn is_eof(&self) -> bool;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token<K> {
    pub kind: K,
    pub line: usize,
    /// 1-indexed column of the token's first character.
    pub col: usize,
    /// The line the token ends on. Usually `line`; only multi-line
    /// literals (Lua `[[long strings]]`, AngelScript `"""heredocs"""`)
    /// differ.
    pub end_line: usize,
    /// 1-indexed column just past the token's last character (exclusive),
    /// on `end_line`.
    pub end_col: usize,
}

impl<K> Token<K> {
    /// A single-line token.
    pub fn new(kind: K, line: usize, col: usize, end_col: usize) -> Self {
        Token {
            kind,
            line,
            col,
            end_line: line,
            end_col,
        }
    }

    /// A token covering `span` (possibly several lines).
    pub fn spanning(kind: K, span: Span) -> Self {
        Token {
            kind,
            line: span.start_line,
            col: span.start_col,
            end_line: span.end_line,
            end_col: span.end_col,
        }
    }

    pub fn span(&self) -> Span {
        Span::new(self.line, self.col, self.end_line, self.end_col)
    }
}

/// A lexing or parsing failure at a source location.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub col: usize,
    /// Exclusive end column of the offending token, for consumers (e.g. an
    /// editor) that want to underline the exact span rather than just a
    /// single point. Lexer-level errors (which don't track a token's own
    /// end) use `col + 1`.
    pub end_col: usize,
}

impl ParseError {
    /// An error at a single character position.
    pub fn at(message: impl Into<String>, line: usize, col: usize) -> Self {
        ParseError {
            message: message.into(),
            line,
            col,
            end_col: col + 1,
        }
    }

    /// An error covering `token`'s first line. It's at least one column
    /// wide, so an error at end of file still shows up when underlined.
    pub fn at_token<K>(message: impl Into<String>, token: &Token<K>) -> Self {
        let end_col = if token.end_line == token.line {
            token.end_col.max(token.col + 1)
        } else {
            token.col + 1
        };
        ParseError {
            message: message.into(),
            line: token.line,
            col: token.col,
            end_col,
        }
    }
}

/// A cursor over a lexed token list, which must end with an EOF token
/// (see [`TokenKindExt::is_eof`]). Reading past the end keeps returning
/// that EOF token.
pub struct TokenStream<K> {
    tokens: Vec<Token<K>>,
    pos: usize,
}

impl<K: TokenKindExt> TokenStream<K> {
    pub fn new(tokens: Vec<Token<K>>) -> Self {
        assert!(
            tokens.last().is_some_and(|t| t.kind.is_eof()),
            "a token stream must end with an EOF token"
        );
        TokenStream { tokens, pos: 0 }
    }

    pub fn peek(&self) -> &Token<K> {
        &self.tokens[self.pos]
    }

    /// The current position, to [`reset`](Self::reset) to later: for
    /// speculative parsing ("is this a declaration?") in languages whose
    /// grammar needs more than fixed lookahead (AngelScript's C-like
    /// `Type name` vs. expression statements).
    pub fn mark(&self) -> usize {
        self.pos
    }

    /// Rewinds to a position from [`mark`](Self::mark).
    pub fn reset(&mut self, mark: usize) {
        self.pos = mark;
    }

    /// The kind `n` tokens *back* (`1` = the token just consumed).
    pub fn prev_kind(&self, n: usize) -> Option<&K> {
        self.pos.checked_sub(n).map(|i| &self.tokens[i].kind)
    }

    pub fn peek_kind(&self) -> &K {
        &self.peek().kind
    }

    /// The kind `n` tokens ahead (`0` = [`peek_kind`](Self::peek_kind)).
    pub fn nth_kind(&self, n: usize) -> &K {
        let i = (self.pos + n).min(self.tokens.len() - 1);
        &self.tokens[i].kind
    }

    /// The most recently consumed token (the first token if none was).
    pub fn prev(&self) -> &Token<K> {
        &self.tokens[self.pos.saturating_sub(1)]
    }

    pub fn advance(&mut self) -> Token<K> {
        let t = self.tokens[self.pos].clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    pub fn check(&self, kind: &K) -> bool {
        self.peek_kind() == kind
    }

    pub fn at_eof(&self) -> bool {
        self.peek_kind().is_eof()
    }

    /// Consumes the next token if it's `kind`.
    pub fn eat(&mut self, kind: &K) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// An error located at the next token.
    pub fn error(&self, message: impl Into<String>) -> ParseError {
        ParseError::at_token(message, self.peek())
    }

    pub fn expect(&mut self, kind: K) -> Result<Token<K>, ParseError> {
        if self.check(&kind) {
            Ok(self.advance())
        } else {
            Err(self.error(format!("expected {kind}, found {}", self.peek_kind())))
        }
    }
}

/// Grammar-level helpers for a parser built around a [`TokenStream`].
pub trait TokenParser: Sized {
    type Kind: TokenKindExt;

    fn tokens(&mut self) -> &mut TokenStream<Self::Kind>;

    /// Parses one left-associative binary precedence level:
    /// `operand (op operand)*`, folding left with `build`. `op` maps the
    /// next token kind to this level's operator, or `None` to stop.
    fn left_assoc<E, Op>(
        &mut self,
        operand: fn(&mut Self) -> Result<E, ParseError>,
        op: impl Fn(&Self::Kind) -> Option<Op>,
        build: impl Fn(E, Op, E) -> E,
    ) -> Result<E, ParseError> {
        let mut left = operand(self)?;
        while let Some(o) = op(self.tokens().peek_kind()) {
            self.tokens().advance();
            let right = operand(self)?;
            left = build(left, o, right);
        }
        Ok(left)
    }
}

#[cfg(test)]
#[path = "token.test.rs"]
mod tests;
