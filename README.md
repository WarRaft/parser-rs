# parser-rs

Language-agnostic building blocks shared by this author's hand-written
parsers ([`jass-rs`](../jass-rs) and [`zinc-rs`](../zinc-rs), with Lua and
AngelScript planned). Each language crate only contains what's actually
language-specific: its keywords and `LexSpec`, AST, grammar, lint rules,
and AST → tree mapping.

| module       | what                                                                                  |
|--------------|---------------------------------------------------------------------------------------|
| `span`       | `Span` (1-indexed, codepoint columns) + `ColumnEncoding` (UTF-16/UTF-8)               |
| `cursor`     | `Cursor`: char-by-char lexer cursor with line/col, trivia/comment skipping            |
| `literal`    | WC3 literal scanners: decimal/hex ints, reals, `"strings"`, `'rawcodes'`              |
| `kind`       | shared `TokenKind<K>`, the `Punct` enum (every language's operators), `keywords!`     |
| `lexer`      | `tokenize::<K>(src, &LexSpec)`: table-driven lexer (comments, rawcodes, punct set)    |
| `token`      | `Token<K>`, `ParseError`, `TokenStream<K>`, `TokenParser::left_assoc`                 |
| `diagnostic` | lint `Diagnostic` + `Level`                                                           |
| `tree`       | labelled `Node` tree, `render_tree` (box drawing), `render_dot` (Graphviz)            |
| `html`       | `render_html`: the self-contained offline AST viewer page                             |

## Adding a language

```rust
use parser_rs::{LexSpec, Punct};

parser_rs::keywords! {
    pub enum Kw { Function = "function", End = "end", Local = "local" }
}

pub type TokenKind = parser_rs::TokenKind<Kw>; // Keyword(Kw) | Ident | Int | Real | Str | Punct | Eof
pub type Token = parser_rs::Token<TokenKind>;

pub const SPEC: LexSpec = LexSpec {
    line_comment: "--",
    block_comment: None,
    rawcodes: false,
    puncts: &[Punct::Plus, Punct::DotDot, Punct::TildeEq, Punct::Hash /* ... */],
};

let tokens = parser_rs::tokenize::<Kw>(src, &SPEC)?;
```

`Punct` already covers JASS, Zinc, Lua (`.. ... # ~= // ^ ::`) and
AngelScript (`++ -- :: ? @ <<= >>=` ...). A new language normally just
lists the operators it needs. A missing operator is one line in `kind.rs`.

Lexical forms the generic lexer doesn't handle yet need new `LexSpec`
options when their language arrives. For Lua that means single-quoted
strings, `[[long strings]]` and `--[[ ]]` comments.
