//! Semantic highlighting, following VS Code's standard token types and
//! modifiers:
//! <https://code.visualstudio.com/api/language-extensions/semantic-highlight-guide#standard-token-types-and-modifiers>
//!
//! - [`SemanticType`] / [`SemanticModifier`] declare *every* standard type
//!   and modifier, in the documented order. Their indices are what the wire
//!   format carries, and [`legend`] is what the editor registers.
//! - [`lexical_type`] / [`comment_tokens`] cover what every language
//!   highlights the same way: keywords, numbers, strings, operators,
//!   comments.
//! - Each language adds identifier classification (function, parameter,
//!   variable, ...) on top, then [`encode`] turns the tokens into the
//!   `deltaLine, deltaStartChar, length, tokenType, tokenModifiers` integer
//!   array the editor expects. Columns go out as 0-based UTF-16, tokens are
//!   single-line and non-overlapping.

use crate::kind::{Punct, TokenKind};
use crate::span::{ColumnEncoding, Span};
use crate::token::Token;

macro_rules! standard_enum {
    ($(#[$meta:meta])* $name:ident { $($(#[$vmeta:meta])* $variant:ident = $text:literal),* $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $($(#[$vmeta])* $variant),*
        }

        impl $name {
            /// Every variant, in the documented order (= legend order).
            pub const ALL: &'static [$name] = &[$($name::$variant),*];

            /// The standard name, as the legend spells it.
            pub fn as_str(self) -> &'static str {
                match self {
                    $($name::$variant => $text),*
                }
            }

            /// The position in [`ALL`](Self::ALL) / the legend.
            pub fn index(self) -> u32 {
                self as u32
            }
        }
    };
}

standard_enum! {
    /// VS Code's standard semantic token types.
    SemanticType {
        /// Identifiers that declare or reference a namespace, module, or package.
        Namespace = "namespace",
        /// Identifiers that declare or reference a class type.
        Class = "class",
        /// Identifiers that declare or reference an enumeration type.
        Enum = "enum",
        /// Identifiers that declare or reference an interface type.
        Interface = "interface",
        /// Identifiers that declare or reference a struct type.
        Struct = "struct",
        /// Identifiers that declare or reference a type parameter.
        TypeParameter = "typeParameter",
        /// Identifiers that declare or reference a type that is not covered above.
        Type = "type",
        /// Identifiers that declare or reference function or method parameters.
        Parameter = "parameter",
        /// Identifiers that declare or reference a local or global variable.
        Variable = "variable",
        /// Identifiers that declare or reference a member property, field, or variable.
        Property = "property",
        /// Identifiers that declare or reference an enumeration property, constant, or member.
        EnumMember = "enumMember",
        /// Identifiers that declare or reference decorators and annotations.
        Decorator = "decorator",
        /// Identifiers that declare an event property.
        Event = "event",
        /// Identifiers that declare a function.
        Function = "function",
        /// Identifiers that declare a member function or method.
        Method = "method",
        /// Identifiers that declare a macro.
        Macro = "macro",
        /// Identifiers that declare a label.
        Label = "label",
        /// Tokens that represent a comment.
        Comment = "comment",
        /// Tokens that represent a string literal.
        String = "string",
        /// Tokens that represent a language keyword.
        Keyword = "keyword",
        /// Tokens that represent a number literal.
        Number = "number",
        /// Tokens that represent a regular expression literal.
        Regexp = "regexp",
        /// Tokens that represent an operator.
        Operator = "operator",
    }
}

standard_enum! {
    /// VS Code's standard semantic token modifiers.
    SemanticModifier {
        /// Declarations of symbols.
        Declaration = "declaration",
        /// Definitions of symbols, for example, in header files.
        Definition = "definition",
        /// Readonly variables and member fields (constants).
        Readonly = "readonly",
        /// Class members (static members).
        Static = "static",
        /// Symbols that should no longer be used.
        Deprecated = "deprecated",
        /// Types and member functions that are abstract.
        Abstract = "abstract",
        /// Functions that are marked async.
        Async = "async",
        /// Variable references where the variable is assigned to.
        Modification = "modification",
        /// Occurrences of symbols in documentation.
        Documentation = "documentation",
        /// Symbols that are part of the standard library.
        DefaultLibrary = "defaultLibrary",
    }
}

impl SemanticModifier {
    /// This modifier's bit in a [`Modifiers`] set.
    pub fn bit(self) -> u32 {
        1 << self.index()
    }
}

/// A set of [`SemanticModifier`]s — the bitset the wire format carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers(pub u32);

impl Modifiers {
    pub const NONE: Modifiers = Modifiers(0);

    pub fn of(mods: &[SemanticModifier]) -> Self {
        Modifiers(mods.iter().fold(0, |acc, m| acc | m.bit()))
    }

    pub fn with(self, m: SemanticModifier) -> Self {
        Modifiers(self.0 | m.bit())
    }

    /// Adds `m` only `if on`.
    pub fn with_if(self, on: bool, m: SemanticModifier) -> Self {
        if on { self.with(m) } else { self }
    }

    pub fn contains(self, m: SemanticModifier) -> bool {
        self.0 & m.bit() != 0
    }
}

/// The names to register as the editor's `SemanticTokensLegend`, in index
/// order: `(token types, token modifiers)`.
pub fn legend() -> (Vec<&'static str>, Vec<&'static str>) {
    (
        SemanticType::ALL.iter().map(|t| t.as_str()).collect(),
        SemanticModifier::ALL.iter().map(|m| m.as_str()).collect(),
    )
}

/// One highlighted range. Columns are 1-based codepoints (like [`Span`]),
/// `end_col` exclusive, on `end_line`. Most tokens are single-line
/// (`end_line == line`); a multi-line one (a Lua long string, an
/// AngelScript heredoc) is split per line by [`encode`], since editor
/// tokens can't span lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticToken {
    pub line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub ty: SemanticType,
    pub mods: Modifiers,
}

impl SemanticToken {
    /// A single-line token.
    pub fn new(line: usize, start_col: usize, end_col: usize, ty: SemanticType, mods: Modifiers) -> Self {
        SemanticToken {
            line,
            start_col,
            end_line: line,
            end_col,
            ty,
            mods,
        }
    }

    /// A token covering `tok` (possibly several lines).
    pub fn at<K>(tok: &Token<K>, ty: SemanticType, mods: Modifiers) -> Self {
        SemanticToken {
            line: tok.line,
            start_col: tok.col,
            end_line: tok.end_line,
            end_col: tok.end_col,
            ty,
            mods,
        }
    }

    /// This token cut into single-line pieces, given the text's lines
    /// (`lines[0]` is line 1).
    pub fn split_lines(self, lines: &[&str]) -> Vec<SemanticToken> {
        if self.end_line <= self.line {
            return vec![self];
        }
        (self.line..=self.end_line)
            .filter_map(|line| {
                let start = if line == self.line { self.start_col } else { 1 };
                let end = if line == self.end_line {
                    self.end_col
                } else {
                    lines.get(line - 1).map_or(1, |l| l.chars().count() + 1)
                };
                (end > start).then(|| SemanticToken::new(line, start, end, self.ty, self.mods))
            })
            .collect()
    }
}

/// How every language highlights a non-identifier token: keywords,
/// numbers, strings, and operators. Brackets, commas, semicolons and dots
/// aren't operators and get `None`, as do identifiers (the language's job)
/// and EOF.
pub fn lexical_type(kind: &TokenKind) -> Option<SemanticType> {
    match kind {
        TokenKind::Keyword(_) => Some(SemanticType::Keyword),
        TokenKind::Int(_) | TokenKind::Real(_) => Some(SemanticType::Number),
        TokenKind::Str(_) => Some(SemanticType::String),
        TokenKind::Punct(p) => match p {
            Punct::LParen
            | Punct::RParen
            | Punct::LBracket
            | Punct::RBracket
            | Punct::LBrace
            | Punct::RBrace
            | Punct::Comma
            | Punct::Semicolon
            | Punct::Dot
            | Punct::Colon => None,
            _ => Some(SemanticType::Operator),
        },
        // An embedded block is highlighted by its own language.
        TokenKind::Ident(_) | TokenKind::Embedded(_) | TokenKind::Eof => None,
    }
}

/// Comment tokens for `comments` (from [`crate::lexer::Lexed::comments`]),
/// split per line since editor tokens can't span lines.
pub fn comment_tokens(comments: &[Span], source: &str) -> Vec<SemanticToken> {
    comment_tokens_at(comments, source, 1)
}

/// [`comment_tokens`] for a `source` region starting on file line
/// `first_line` (an embedded block).
pub fn comment_tokens_at(comments: &[Span], source: &str, first_line: usize) -> Vec<SemanticToken> {
    let lines: Vec<&str> = source.lines().collect();
    let mut out = Vec::new();
    for span in comments {
        for line in span.start_line..=span.end_line {
            let start = if line == span.start_line { span.start_col } else { 1 };
            let end = if line == span.end_line {
                span.end_col
            } else {
                lines
                    .get(line - first_line)
                    .map_or(1, |l| l.chars().count() + 1)
            };
            if end > start {
                out.push(SemanticToken::new(
                    line,
                    start,
                    end,
                    SemanticType::Comment,
                    Modifiers::NONE,
                ));
            }
        }
    }
    out
}

/// Encodes tokens into the editor's integer array: per token,
/// `deltaLine, deltaStartChar, length, tokenType, tokenModifiers`, where
/// `deltaStartChar` is relative to the previous token's start when on the
/// same line. Tokens are sorted first. A token overlapping the one before
/// it is dropped, and so is an empty one.
pub fn encode(tokens: Vec<SemanticToken>, source: &str) -> Vec<u32> {
    let lines: Vec<&str> = source.lines().collect();
    let mut tokens: Vec<SemanticToken> =
        tokens.into_iter().flat_map(|t| t.split_lines(&lines)).collect();
    tokens.sort_by_key(|t| (t.line, t.start_col));
    let utf16 = |line: usize, col: usize| {
        let text = lines.get(line.wrapping_sub(1)).copied().unwrap_or("");
        ColumnEncoding::Utf16.from_codepoints(text, col) - 1
    };

    let mut data = Vec::with_capacity(tokens.len() * 5);
    let (mut prev_line, mut prev_start) = (1usize, 0usize);
    let mut prev_end: Option<(usize, usize)> = None;
    for t in tokens {
        if t.end_col <= t.start_col || t.line == 0 || t.start_col == 0 {
            continue;
        }
        if prev_end.is_some_and(|(line, end)| line == t.line && t.start_col < end) {
            continue;
        }
        let start = utf16(t.line, t.start_col);
        let length = utf16(t.line, t.end_col) - start;
        let delta_line = t.line - prev_line;
        let delta_start = if delta_line == 0 { start - prev_start } else { start };
        data.extend([
            delta_line as u32,
            delta_start as u32,
            length as u32,
            t.ty.index(),
            t.mods.0,
        ]);
        prev_line = t.line;
        prev_start = start;
        prev_end = Some((t.line, t.end_col));
    }
    data
}

#[cfg(test)]
#[path = "semantic.test.rs"]
mod tests;
