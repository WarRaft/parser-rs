//! A table-driven lexer producing the shared [`TokenKind`]. A language
//! describes itself with a [`LexSpec`] (comment syntax, which literal forms
//! exist, which [`Punct`]s are valid) plus its [`Keyword`] type, and gets a
//! complete tokenizer:
//!
//! ```
//! use parser_rs::lexer::{tokenize, LexSpec};
//! use parser_rs::{Punct, TokenKind};
//!
//! parser_rs::keywords! { pub enum Kw { Set = "set" } }
//!
//! const SPEC: LexSpec = LexSpec {
//!     line_comment: "//",
//!     block_comment: None,
//!     rawcodes: false,
//!     puncts: &[Punct::Assign, Punct::EqEq],
//! };
//! let kinds: Vec<_> = tokenize::<Kw>("set x == 1", &SPEC)
//!     .unwrap()
//!     .into_iter()
//!     .map(|t| t.kind)
//!     .collect();
//! assert_eq!(kinds, vec![
//!     TokenKind::Keyword(Kw::Set),
//!     TokenKind::Ident("x".into()),
//!     TokenKind::Punct(Punct::EqEq),
//!     TokenKind::Int(1),
//!     TokenKind::Eof,
//! ]);
//! ```

use crate::cursor::Cursor;
use crate::kind::{Keyword, Punct, TokenKind};
use crate::literal::{self, Number};
use crate::token::{ParseError, Token};

/// A language's lexical rules.
#[derive(Debug, Clone, Copy)]
pub struct LexSpec {
    /// Line comment opener, e.g. `"//"` (or `"--"` for Lua).
    pub line_comment: &'static str,
    /// Block comment delimiters, e.g. `Some(("/*", "*/"))`.
    pub block_comment: Option<(&'static str, &'static str)>,
    /// Whether `'abcd'` is a four-character-code integer literal.
    pub rawcodes: bool,
    /// The punctuation/operators this language has. Matching is
    /// longest-first, so listing both `=` and `==` just works; a character
    /// that starts none of them is an "unexpected character" error.
    pub puncts: &'static [Punct],
}

/// Lexes `src` into tokens ending with [`TokenKind::Eof`].
pub fn tokenize<K: Keyword>(
    src: &str,
    spec: &LexSpec,
) -> Result<Vec<Token<TokenKind<K>>>, ParseError> {
    let mut cur = Cursor::new(src);
    let mut tokens = Vec::new();
    loop {
        let tok = next_token(&mut cur, spec)?;
        let is_eof = tok.kind == TokenKind::Eof;
        tokens.push(tok);
        if is_eof {
            return Ok(tokens);
        }
    }
}

fn next_token<K: Keyword>(
    cur: &mut Cursor,
    spec: &LexSpec,
) -> Result<Token<TokenKind<K>>, ParseError> {
    cur.skip_trivia(spec.line_comment, spec.block_comment)?;
    let (line, col) = (cur.line(), cur.col());

    let Some(c) = cur.peek() else {
        return Ok(Token::new(TokenKind::Eof, line, col, col));
    };

    let kind = if c.is_ascii_digit() {
        match literal::number(cur, line, col)? {
            Number::Int(v) => TokenKind::Int(v),
            Number::Real(v) => TokenKind::Real(v),
        }
    } else if c == '_' || c.is_alphabetic() {
        let word = cur.take_while(|c| c == '_' || c.is_alphanumeric());
        match K::from_word(&word) {
            Some(k) => TokenKind::Keyword(k),
            None => TokenKind::Ident(word),
        }
    } else if c == '"' {
        TokenKind::Str(literal::string(cur, line, col)?)
    } else if c == '\'' && spec.rawcodes {
        TokenKind::Int(literal::rawcode(cur, line, col)?)
    } else {
        let punct = spec
            .puncts
            .iter()
            .copied()
            .filter(|p| cur.starts_with(p.text()))
            .max_by_key(|p| p.text().len())
            .ok_or_else(|| ParseError::at(format!("unexpected character '{c}'"), line, col))?;
        cur.eat_str(punct.text());
        TokenKind::Punct(punct)
    };
    Ok(Token::new(kind, line, col, cur.col()))
}

#[cfg(test)]
#[path = "lexer.test.rs"]
mod tests;
