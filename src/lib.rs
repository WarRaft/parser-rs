//! Foundational, language-agnostic building blocks shared across this
//! author's hand-written parsers (JASS, Zinc, ...):
//!
//! - [`span`]: a source [`Span`] and a [`ColumnEncoding`] for translating it
//!   into whatever coordinate convention a consumer expects;
//! - [`cursor`]: a character [`Cursor`] that tracks line/column for lexers;
//! - [`literal`]: scanners for the C-like literal forms (numbers, quoted
//!   strings, four-character codes) both WC3 languages share;
//! - [`kind`]: the shared token vocabulary: [`TokenKind`], plus one [`Kw`]
//!   and one [`Punct`] enum covering every language's keywords/operators,
//!   with a [`KwSet`] choosing which keywords a language activates;
//! - [`lexer`]: a table-driven tokenizer configured by a [`LexSpec`];
//! - [`token`]: [`Token`], [`ParseError`], [`TokenStream`] and the
//!   [`TokenParser`] trait that gives a recursive-descent parser its helpers;
//! - [`semantic`]: VS Code's standard semantic token types/modifiers, the
//!   shared lexical highlighting and the editor wire encoding;
//! - [`diagnostic`]: lint [`Diagnostic`]s and their [`Level`];
//! - [`tree`]: a labelled [`Node`] tree plus text/Graphviz renderers;
//! - [`html`]: the self-contained, offline AST viewer page.
//!
//! Each language crate keeps only what's actually language-specific: its
//! keywords and [`LexSpec`], AST, grammar, lint rules, and AST → [`Node`]
//! mapping.

pub mod cursor;
pub mod diagnostic;
pub mod html;
pub mod kind;
pub mod lexer;
pub mod literal;
pub mod semantic;
pub mod span;
pub mod token;
pub mod tree;

pub use cursor::Cursor;
pub use diagnostic::{Diagnostic, Level};
pub use html::{render_html, HtmlLang};
pub use kind::{Embedded, Kw, KwSet, Punct, TokenKind};
pub use lexer::{tokenize, tokenize_at, tokenize_lossy, tokenize_lossy_at, EmbedSpec, LexSpec, Lexed};
pub use semantic::{Modifiers, SemanticModifier, SemanticToken, SemanticType};
pub use span::{ColumnEncoding, Span};
pub use token::{ParseError, Token, TokenKindExt, TokenParser, TokenStream};
pub use tree::{render_dot, render_tree, Node};
