//! The shared token vocabulary: one [`TokenKind`] for every language.
//!
//! - Identifiers, literals and EOF look the same in every WC3-related
//!   language (JASS, vJASS, Zinc, Lua, AngelScript, ...), so they're plain
//!   variants.
//! - Punctuation/operators are one shared [`Punct`] enum and keywords one
//!   shared [`Kw`] enum, each the *union* of what all the languages use.
//!   A language or dialect doesn't define its own enums. It picks which
//!   puncts and keywords are *active* in its
//!   [`LexSpec`](crate::lexer::LexSpec).
//! - Activation happens in the lexer. A word whose keyword isn't active
//!   lexes as a plain identifier. So `library` is an identifier in plain
//!   JASS and a keyword in vJASS, and the same parser serves both: it never
//!   sees a keyword the dialect doesn't have.

use std::fmt::{self, Debug, Display};

use crate::span::Span;
use crate::token::TokenKindExt;

macro_rules! keywords {
    ($($(#[$vmeta:meta])* $variant:ident = $text:literal),* $(,)?) => {
        /// Every keyword of every supported language and dialect. Which ones
        /// a lexer recognizes is decided by a [`KwSet`].
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Kw {
            $($(#[$vmeta])* $variant),*
        }

        impl Kw {
            /// Every variant.
            pub const ALL: &'static [Kw] = &[$(Kw::$variant),*];

            /// The keyword spelled `word` in *any* language (case-sensitive).
            /// Whether it's active is a separate question — see [`KwSet`].
            pub fn from_word(word: &str) -> Option<Kw> {
                match word {
                    $($text => Some(Kw::$variant),)*
                    _ => None,
                }
            }

            /// The source spelling.
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Kw::$variant => $text),*
                }
            }
        }
    };
}

keywords! {
    // ----- JASS -----
    Function = "function",
    EndFunction = "endfunction",
    Takes = "takes",
    Returns = "returns",
    Nothing = "nothing",
    Native = "native",
    Globals = "globals",
    EndGlobals = "endglobals",
    Constant = "constant",
    Local = "local",
    Set = "set",
    Call = "call",
    If = "if",
    Then = "then",
    ElseIf = "elseif",
    Else = "else",
    EndIf = "endif",
    Loop = "loop",
    EndLoop = "endloop",
    ExitWhen = "exitwhen",
    Return = "return",
    Debug = "debug",
    Type = "type",
    Extends = "extends",
    Array = "array",
    And = "and",
    Or = "or",
    Not = "not",
    True = "true",
    False = "false",
    Null = "null",

    // ----- vJASS (also used by Zinc) -----
    Library = "library",
    LibraryOnce = "library_once",
    EndLibrary = "endlibrary",
    Scope = "scope",
    EndScope = "endscope",
    Requires = "requires",
    Needs = "needs",
    Uses = "uses",
    Initializer = "initializer",
    Optional = "optional",
    Private = "private",
    Public = "public",
    Static = "static",
    Struct = "struct",
    EndStruct = "endstruct",
    Interface = "interface",
    EndInterface = "endinterface",
    Method = "method",
    EndMethod = "endmethod",
    Module = "module",
    EndModule = "endmodule",
    Implement = "implement",
    Operator = "operator",
    Stub = "stub",
    Delegate = "delegate",
    Readonly = "readonly",
    This = "this",
    ThisType = "thistype",
    Super = "super",
    Hook = "hook",
    Defaults = "defaults",
    Keyword = "keyword",

    // ----- Zinc / Lua control flow -----
    While = "while",
    Do = "do",
    For = "for",
    Break = "break",

    // ----- Lua -----
    Nil = "nil",
    End = "end",
    Repeat = "repeat",
    Until = "until",
    In = "in",
    Goto = "goto",

    // ----- AngelScript (reserved words only; contextual words such as
    // `get`, `set`, `this`, `super`, `override`, `final`, `shared`,
    // `abstract`, `function`, `from`, `property`, `explicit`, `external`
    // stay identifiers and are recognized by the parser) -----
    Auto = "auto",
    Bool = "bool",
    Case = "case",
    Cast = "cast",
    Catch = "catch",
    Class = "class",
    Const = "const",
    Continue = "continue",
    Default = "default",
    Double = "double",
    Enum = "enum",
    Float = "float",
    Funcdef = "funcdef",
    Import = "import",
    InOut = "inout",
    Int = "int",
    Int8 = "int8",
    Int16 = "int16",
    Int32 = "int32",
    Int64 = "int64",
    Is = "is",
    Mixin = "mixin",
    Namespace = "namespace",
    Out = "out",
    Protected = "protected",
    Switch = "switch",
    Try = "try",
    Typedef = "typedef",
    UInt = "uint",
    UInt8 = "uint8",
    UInt16 = "uint16",
    UInt32 = "uint32",
    UInt64 = "uint64",
    Void = "void",
    Xor = "xor",
}

impl Display for Kw {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A set of active [`Kw`]s: a `const`-buildable bitset.
///
/// ```
/// use parser_rs::{Kw, KwSet};
/// const BASE: KwSet = KwSet::of(&[Kw::If, Kw::Then]);
/// const EXTENDED: KwSet = BASE.union(KwSet::of(&[Kw::Library]));
/// assert!(EXTENDED.contains(Kw::If) && EXTENDED.contains(Kw::Library));
/// assert!(!BASE.contains(Kw::Library));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KwSet(u128);

const _: () = assert!(Kw::ALL.len() <= 128, "KwSet is a u128 bitset");

impl KwSet {
    pub const EMPTY: KwSet = KwSet(0);

    pub const fn of(kws: &[Kw]) -> KwSet {
        let mut bits = 0u128;
        let mut i = 0;
        while i < kws.len() {
            bits |= 1 << kws[i] as u32;
            i += 1;
        }
        KwSet(bits)
    }

    pub const fn union(self, other: KwSet) -> KwSet {
        KwSet(self.0 | other.0)
    }

    pub const fn contains(self, kw: Kw) -> bool {
        self.0 & (1 << kw as u32) != 0
    }

    /// The active keyword spelled `word`, if any.
    pub fn lookup(self, word: &str) -> Option<Kw> {
        Kw::from_word(word).filter(|&k| self.contains(k))
    }
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
    // AngelScript extras
    StarStar = "**",
    StarStarAssign = "**=",
    UShr = ">>>",
    UShrAssign = ">>>=",
    CaretCaret = "^^",
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

/// A block of another language embedded in the host file between
/// directive lines (vJASS `//! zinc` ... `//! endzinc`), lexed as one
/// [`TokenKind::Embedded`] token. The host parser hands `source` to the
/// embedded language's parser and carries on after it. See
/// [`crate::lexer::EmbedSpec`].
#[derive(Debug, Clone, PartialEq)]
pub struct Embedded {
    /// Which embedded language: the [`EmbedSpec::lang`](crate::lexer::EmbedSpec::lang) that matched.
    pub lang: &'static str,
    /// The lines between the directives, verbatim (each `\n`-terminated).
    pub source: String,
    /// The file line `source` starts on. Lex it with
    /// [`tokenize_at`](crate::lexer::tokenize_at) and every position
    /// stays file-absolute.
    pub first_line: usize,
    /// The opening directive line.
    pub open: Span,
    /// The closing directive line; `None` if the block is unterminated.
    pub close: Option<Span>,
}

/// A token kind, shared by every language.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Keyword(Kw),
    Ident(String),
    Int(i64),
    Real(f64),
    /// A string literal's decoded value.
    Str(String),
    Punct(Punct),
    /// A whole embedded-language block; see [`Embedded`].
    Embedded(Box<Embedded>),
    Eof,
}

impl TokenKindExt for TokenKind {
    fn is_eof(&self) -> bool {
        matches!(self, TokenKind::Eof)
    }
}

/// Human-readable, for error messages: `'endfunction'`, `')'`,
/// `identifier 'x'`, `end of file`, ...
impl Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Keyword(k) => write!(f, "'{k}'"),
            TokenKind::Ident(s) => write!(f, "identifier '{s}'"),
            TokenKind::Int(v) => write!(f, "integer {v}"),
            TokenKind::Real(v) => write!(f, "real {v}"),
            TokenKind::Str(s) => write!(f, "string {s:?}"),
            TokenKind::Punct(p) => write!(f, "'{p}'"),
            TokenKind::Embedded(e) => write!(f, "embedded {} block", e.lang),
            TokenKind::Eof => f.write_str("end of file"),
        }
    }
}

#[cfg(test)]
#[path = "kind.test.rs"]
mod tests;
