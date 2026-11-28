//! Every quoted string literal of a text, from its tokens: its value and
//! where its text is (inside the quotes). For a consumer that links
//! strings to what they name by their text alone (the editor: a string
//! that's a frame's name is a reference to it, in any language).

use crate::kind::TokenKind;
use crate::span::Span;
use crate::token::Token;

/// A string literal: its value, and its text's span, inside the quotes.
#[derive(Debug, Clone, PartialEq)]
pub struct Literal {
    pub text: String,
    pub span: Span,
}

/// Every string in `tokens` (of `src`) quoted on one line (`"..."`,
/// `'...'`; not a Lua long string or a heredoc, whose text isn't between
/// two quotes), in source order.
pub fn literals(tokens: &[Token<TokenKind>], src: &str) -> Vec<Literal> {
    let lines: Vec<&str> = src.lines().collect();
    tokens
        .iter()
        .filter_map(|t| {
            let TokenKind::Str(text) = &t.kind else { return None };
            let line = lines.get(t.line.checked_sub(1)?)?;
            let mut chars = line.chars().skip(t.col - 1);
            let quote = chars.next().filter(|c| *c == '"' || *c == '\'')?;
            let quoted = t.end_line == t.line && t.end_col >= t.col + 2 && line.chars().nth(t.end_col - 2) == Some(quote);
            // `"""heredoc"""` starts with two more.
            let heredoc = chars.next() == Some(quote) && chars.next() == Some(quote);
            (quoted && !heredoc).then(|| Literal { text: text.clone(), span: Span::new(t.line, t.col + 1, t.line, t.end_col - 1) })
        })
        .collect()
}

#[cfg(test)]
#[path = "strings.test.rs"]
mod tests;
