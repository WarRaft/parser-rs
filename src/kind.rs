//! The shared token vocabulary: one [`TokenKind`] for every language,
//! generic only over the language's keyword set.
//!
//! - Identifiers, literals and EOF look the same in every WC3-related
//!   language (JASS, Zinc, Lua, AngelScript, ...), so they're plain
//!   variants here.
//! - Punctuation/operators are one shared [`Punct`] enum — the *union* of
//!   what the languages use. Each language picks which of them its lexer
//!   accepts (see [`crate::lexer::LexSpec::puncts`]); a new language
//!   usually needs no new variants at all.
//! - Keywords differ per language, so they're the type parameter `K`,
//!   declared with the [`keywords!`](crate::keywords) macro.

use std::fmt::{self, Debug, Display};

use crate::token::TokenKindExt;

/// A language's keyword set. Implement it with [`keywords!`](crate::keywords).
pub trait Keyword: Copy + PartialEq + Debug + 'static {
    /// The keyword spelled `word`, if `word` is one (case-sensitive).
    fn from_word(word: &str) -> Option<Self>;
    /// The keyword's source spelling.
    fn as_str(self) -> &'static str;
}

/// Declares a keyword enum and its [`Keyword`] impl in one go:
///
/// ```
/// parser_rs::keywords! {
///     pub enum Kw {
///         Function = "function",
///         EndFunction = "endfunction",
///     }
/// }
/// use parser_rs::Keyword;
/// assert_eq!(Kw::from_word("endfunction"), Some(Kw::EndFunction));
/// assert_eq!(Kw::Function.as_str(), "function");
/// ```
#[macro_export]
macro_rules! keywords {
    ($(#[$meta:meta])* $vis:vis enum $name:ident { $($variant:ident = $text:literal),* $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        $vis enum $name {
            $($variant),*
        }

        impl $crate::Keyword for $name {
            fn from_word(word: &str) -> Option<Self> {
                match word {
                    $($text => Some($name::$variant),)*
                    _ => None,
                }
            }

            fn as_str(self) -> &'static str {
                match self {
                    $($name::$variant => $text),*
                }
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str($crate::Keyword::as_str(*self))
            }
        }
    };
}

macro_rules! puncts {
    ($($variant:ident = $text:literal),* $(,)?) => {
        /// Every punctuation/operator token any supported language uses.
        /// A language's lexer only produces the subset listed in its
        /// [`LexSpec`](crate::lexer::LexSpec).
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Punct {
            $($variant),*
        }

        impl Punct {
            /// Every variant, for building a language's punct list.
            pub const ALL: &'static [Punct] = &[$(Punct::$variant),*];

            /// The source spelling.
            pub fn text(self) -> &'static str {
                match self {
                    $(Punct::$variant => $text),*
                }
            }
        }
    };
}

puncts! {
    // arithmetic
    Plus = "+",
    Minus = "-",
    Star = "*",
    Slash = "/",
    SlashSlash = "//",
    Percent = "%",
    Caret = "^",
    PlusPlus = "++",
    MinusMinus = "--",
    // bitwise / misc unary
    Amp = "&",
    Pipe = "|",
    Tilde = "~",
    Shl = "<<",
    Shr = ">>",
    Hash = "#",
    At = "@",
    // logical
    Bang = "!",
    AndAnd = "&&",
    OrOr = "||",
    // comparison
    EqEq = "==",
    NotEq = "!=",
    TildeEq = "~=",
    Lt = "<",
    Gt = ">",
    LtEq = "<=",
    GtEq = ">=",
    // assignment
    Assign = "=",
    PlusAssign = "+=",
    MinusAssign = "-=",
    StarAssign = "*=",
    SlashAssign = "/=",
    PercentAssign = "%=",
    AmpAssign = "&=",
    PipeAssign = "|=",
    CaretAssign = "^=",
    ShlAssign = "<<=",
    ShrAssign = ">>=",
    // structure
    Arrow = "->",
    FatArrow = "=>",
    Dot = ".",
    DotDot = "..",
    Ellipsis = "...",
    Comma = ",",
    Semicolon = ";",
    Colon = ":",
    ColonColon = "::",
    Question = "?",
    LParen = "(",
    RParen = ")",
    LBracket = "[",
    RBracket = "]",
    LBrace = "{",
    RBrace = "}",
}

impl Display for Punct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text())
    }
}

/// A token kind, shared by every language; `K` is the language's
/// [`Keyword`] set.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind<K> {
    Keyword(K),
    Ident(String),
    Int(i64),
    Real(f64),
    /// A string literal's decoded value.
    Str(String),
    Punct(Punct),
    Eof,
}

impl<K: Keyword> TokenKindExt for TokenKind<K> {
    fn is_eof(&self) -> bool {
        matches!(self, TokenKind::Eof)
    }
}

/// Human-readable, for error messages: `'endfunction'`, `')'`,
/// `identifier 'x'`, `end of file`, ...
impl<K: Keyword> Display for TokenKind<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Keyword(k) => write!(f, "'{}'", k.as_str()),
            TokenKind::Ident(s) => write!(f, "identifier '{s}'"),
            TokenKind::Int(v) => write!(f, "integer {v}"),
            TokenKind::Real(v) => write!(f, "real {v}"),
            TokenKind::Str(s) => write!(f, "string {s:?}"),
            TokenKind::Punct(p) => write!(f, "'{}'", p.text()),
            TokenKind::Eof => f.write_str("end of file"),
        }
    }
}

#[cfg(test)]
#[path = "kind.test.rs"]
mod tests;
