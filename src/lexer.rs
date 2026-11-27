//! A table-driven lexer producing the shared [`TokenKind`]. A language (or
//! dialect) describes itself with a [`LexSpec`] (comment syntax, which
//! literal forms exist, which [`Kw`]s and [`Punct`]s are active) and gets
//! a complete tokenizer:
//!
//! ```
//! use parser_rs::lexer::{tokenize, LexSpec};
//! use parser_rs::{Kw, KwSet, Punct, TokenKind};
//!
//! const SPEC: LexSpec = LexSpec {
//!     keywords: KwSet::of(&[Kw::Set]),
//!     puncts: &[Punct::Assign, Punct::EqEq],
//!     ..LexSpec::BASE
//! };
//! let kinds: Vec<_> = tokenize("set x == 1", &SPEC)
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
use crate::kind::{Embedded, KwSet, Punct, TokenKind};
use crate::literal::{self, Number, NumberSpec};
use crate::span::Span;
use crate::token::{ParseError, Token};

/// A language's lexical rules.
#[derive(Debug, Clone, Copy)]
pub struct LexSpec {
    /// The active keywords. A word spelling an inactive keyword (say,
    /// `library` in plain JASS) lexes as a plain identifier.
    pub keywords: KwSet,
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
    /// Other languages that can be embedded in this one between directive
    /// lines; see [`EmbedSpec`]. Empty for most languages.
    pub embeds: &'static [EmbedSpec],
    /// Which number forms exist beyond `42`, `0x2A`, `4.2`.
    pub numbers: NumberSpec,
    /// Whether `'...'` is a string (Lua, AngelScript). Takes effect only
    /// without `rawcodes`.
    pub single_quote_strings: bool,
    /// Lua long brackets: `[[long strings]]`, `[==[...]==]`, and long
    /// comments `--[[ ... ]]` (the line comment opener plus a long bracket).
    pub long_brackets: bool,
    /// AngelScript `"""heredoc"""` strings.
    pub heredocs: bool,
    /// Block comments nest (vJASS: `/* a /* b */ c */` is one comment).
    pub nested_block_comments: bool,
    /// `"..."` holds no escapes: a backslash is itself (FDF, whose strings
    /// are file paths: `"UI\FrameDef\UI\EscMenuTemplates.fdf"`).
    pub raw_strings: bool,
}

impl LexSpec {
    /// Everything off: `//` comments, no keywords, no puncts. Specs
    /// start from this and switch on what their language has
    /// (`LexSpec { keywords: ..., puncts: ..., ..LexSpec::BASE }`).
    pub const BASE: LexSpec = LexSpec {
        keywords: KwSet::EMPTY,
        line_comment: "//",
        block_comment: None,
        rawcodes: false,
        puncts: &[],
        embeds: &[],
        numbers: NumberSpec {
            exponent: false,
            leading_dot: false,
            radix_prefixes: false,
            float_suffix: false,
            trailing_dot: false,
            leading_zero_octal: false,
            dollar_hex: false,
        },
        single_quote_strings: false,
        long_brackets: false,
        heredocs: false,
        nested_block_comments: false,
        raw_strings: false,
    };
}

/// Whether a Lua long comment (`--[[` / `--[==[`) starts at the cursor.
fn long_comment_at(cur: &Cursor, spec: &LexSpec) -> bool {
    spec.long_brackets
        && cur.starts_with(spec.line_comment)
        && literal::long_bracket_level(cur, spec.line_comment.chars().count()).is_some()
}

/// An embedded language: its blocks run from an `open` directive line to a
/// `close` directive line, e.g. vJASS's `//! zinc` ... `//! endzinc`.
///
/// Each directive must be alone on its line (indentation allowed) and is
/// matched ignoring whitespace, so `//!zinc` and `//!  zinc` both count.
/// The lexer turns a whole block into one [`TokenKind::Embedded`] token.
/// The host parser hands its [`Embedded::source`] to the embedded
/// language's parser (lexed with [`tokenize_at`] so line numbers stay
/// file-absolute), then carries on with the host's next token. That's
/// how control passes to the embedded parser and back.
#[derive(Debug, Clone, Copy)]
pub struct EmbedSpec {
    /// An id for the embedded language, e.g. `"zinc"`.
    pub lang: &'static str,
    pub open: &'static str,
    pub close: &'static str,
    /// The opening directive is only the start of its line, more after it
    /// (`//! textmacro NAME takes a, b`); else the whole line.
    pub prefix: bool,
}

/// Whether `line` is the directive `directive`, ignoring all whitespace.
fn is_directive(line: &str, directive: &str) -> bool {
    let squeeze = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    squeeze(line) == squeeze(directive)
}

/// Whether `line` starts with the directive `directive` (`//! textmacro`):
/// the same marker, then its word or the word with `_once`, then anything
/// (`//! textmacro X takes a`, `//!textmacro_once X`; not `//! textmacros`).
fn starts_with_directive(line: &str, directive: &str) -> bool {
    let Some((marker, word)) = directive.split_once(char::is_whitespace) else { return is_directive(line, directive) };
    let Some(rest) = line.trim_start().strip_prefix(marker) else { return false };
    let first = rest.split_whitespace().next().unwrap_or("");
    let word = word.trim();
    first == word || first.strip_prefix(word) == Some("_once")
}

/// The embed whose opening directive the cursor sits on, if any.
fn embed_at(cur: &Cursor, spec: &LexSpec) -> Option<EmbedSpec> {
    let on_text = cur.peek().is_some_and(|c| !c.is_whitespace());
    if spec.embeds.is_empty() || !on_text || !cur.at_line_start() {
        return None;
    }
    let line = cur.rest_of_line();
    spec.embeds
        .iter()
        .copied()
        .find(|e| if e.prefix { starts_with_directive(&line, e.open) } else { is_directive(&line, e.open) })
}

/// Consumes an embedded block, from its opening directive (under the
/// cursor) through its closing one. An unterminated block runs to end of
/// input and is reported in `errors`, but still returned.
fn lex_embedded(cur: &mut Cursor, embed: EmbedSpec, errors: &mut Vec<ParseError>) -> Token<TokenKind> {
    let (line, col) = (cur.line(), cur.col());
    let open_len = cur.rest_of_line().trim_end().chars().count();
    let open = Span::new(line, col, line, col + open_len);
    cur.take_while(|c| c != '\n');
    cur.advance();

    let first_line = cur.line();
    let mut source = String::new();
    let mut close = None;
    while !cur.is_eof() {
        let text = cur.rest_of_line();
        if is_directive(&text, embed.close) {
            let start = text.chars().take_while(|c| c.is_whitespace()).count() + 1;
            let end = text.trim_end().chars().count() + 1;
            close = Some(Span::new(cur.line(), start, cur.line(), end));
            cur.take_while(|c| c != '\n');
            break;
        }
        source.push_str(&text);
        source.push('\n');
        cur.take_while(|c| c != '\n');
        cur.advance();
    }
    if close.is_none() {
        errors.push(ParseError {
            message: format!("unterminated '{}' block: missing '{}'", embed.open, embed.close),
            line,
            col,
            end_col: open.end_col,
        });
    }
    let embedded = Embedded {
        lang: embed.lang,
        source,
        first_line,
        open,
        close,
    };
    Token::new(TokenKind::Embedded(Box::new(embedded)), line, col, open.end_col)
}

/// Lexes `src` into tokens ending with [`TokenKind::Eof`], failing on the
/// first lexical error (what a parser wants).
pub fn tokenize(src: &str, spec: &LexSpec) -> Result<Vec<Token<TokenKind>>, ParseError> {
    tokenize_at(src, spec, 1)
}

/// [`tokenize`] for a region of a larger file starting on line
/// `first_line` (an [`Embedded::source`]): positions come out
/// file-absolute.
pub fn tokenize_at(
    src: &str,
    spec: &LexSpec,
    first_line: usize,
) -> Result<Vec<Token<TokenKind>>, ParseError> {
    let lexed = tokenize_lossy_at(src, spec, first_line);
    match lexed.errors.into_iter().next() {
        Some(err) => Err(err),
        None => Ok(lexed.tokens),
    }
}

/// The result of [`tokenize_lossy`].
pub struct Lexed {
    /// Every token that lexed cleanly, ending with [`TokenKind::Eof`].
    pub tokens: Vec<Token<TokenKind>>,
    /// Every comment's span (block comments may span lines).
    pub comments: Vec<Span>,
    /// Every lexical error, in source order.
    pub errors: Vec<ParseError>,
}

/// Lexes `src` without stopping at errors: a bad character or literal is
/// skipped and lexing resumes after it. That's what an editor wants for
/// highlighting half-typed code. It also collects comment spans, which a
/// parser has no use for but a highlighter does.
pub fn tokenize_lossy(src: &str, spec: &LexSpec) -> Lexed {
    tokenize_lossy_at(src, spec, 1)
}

/// [`tokenize_lossy`] for a region starting on line `first_line`; see
/// [`tokenize_at`].
pub fn tokenize_lossy_at(src: &str, spec: &LexSpec, first_line: usize) -> Lexed {
    let mut cur = Cursor::new_at(src, first_line);
    let mut lexed = Lexed {
        tokens: Vec::new(),
        comments: Vec::new(),
        errors: Vec::new(),
    };
    loop {
        match next_token(&mut cur, spec, &mut lexed.comments, &mut lexed.errors) {
            Ok(tok) => {
                let is_eof = tok.kind == TokenKind::Eof;
                lexed.tokens.push(tok);
                if is_eof {
                    return lexed;
                }
            }
            Err(err) => {
                // A bad character leaves the cursor on it; step past so we
                // never report (or stall on) the same character twice.
                if (cur.line(), cur.col()) == (err.line, err.col) {
                    cur.advance();
                }
                lexed.errors.push(err);
            }
        }
    }
}

fn next_token(
    cur: &mut Cursor,
    spec: &LexSpec,
    comments: &mut Vec<Span>,
    errors: &mut Vec<ParseError>,
) -> Result<Token<TokenKind>, ParseError> {
    loop {
        cur.skip_trivia_collecting(
            spec.line_comment,
            spec.block_comment,
            spec.nested_block_comments,
            Some(comments),
            &|c| embed_at(c, spec).is_some() || long_comment_at(c, spec),
        )?;
        if !long_comment_at(cur, spec) {
            break;
        }
        let (line, col) = (cur.line(), cur.col());
        cur.eat_str(spec.line_comment);
        let result = literal::long_bracket(cur, "long comment", line, col);
        comments.push(Span::new(line, col, cur.line(), cur.col()));
        result?;
    }
    let (line, col) = (cur.line(), cur.col());

    if let Some(embed) = embed_at(cur, spec) {
        return Ok(lex_embedded(cur, embed, errors));
    }

    let Some(c) = cur.peek() else {
        return Ok(Token::new(TokenKind::Eof, line, col, col));
    };

    let starts_number = c.is_ascii_digit()
        || (spec.numbers.leading_dot && c == '.' && cur.peek_at(1).is_some_and(|d| d.is_ascii_digit()))
        || (spec.numbers.dollar_hex && c == '$');
    let kind = if starts_number {
        match literal::number(cur, &spec.numbers, line, col)? {
            Number::Int(v) => TokenKind::Int(v),
            Number::Real(v) => TokenKind::Real(v),
        }
    } else if c == '_' || c.is_alphabetic() {
        let word = cur.take_while(|c| c == '_' || c.is_alphanumeric());
        match spec.keywords.lookup(&word) {
            Some(k) => TokenKind::Keyword(k),
            None => TokenKind::Ident(word),
        }
    } else if c == '"' && spec.heredocs && cur.starts_with("\"\"\"") {
        TokenKind::Str(literal::heredoc(cur, line, col)?)
    } else if c == '"' {
        TokenKind::Str(if spec.raw_strings { literal::raw(cur, line, col)? } else { literal::quoted(cur, '"', line, col)? })
    } else if c == '\'' && spec.rawcodes {
        TokenKind::Int(literal::rawcode(cur, line, col)?)
    } else if c == '\'' && spec.single_quote_strings {
        TokenKind::Str(literal::quoted(cur, '\'', line, col)?)
    } else if c == '[' && spec.long_brackets && literal::long_bracket_level(cur, 0).is_some() {
        TokenKind::Str(literal::long_bracket(cur, "long string", line, col)?)
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
    Ok(Token::spanning(kind, Span::new(line, col, cur.line(), cur.col())))
}

#[cfg(test)]
#[path = "lexer.test.rs"]
mod tests;
