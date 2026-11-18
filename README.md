# parser-rs

Language-agnostic building blocks shared by this author's hand-written
parsers: [`jass-rs`](../jass-rs) (JASS, vJASS planned), [`zinc-rs`](../zinc-rs),
and Lua and AngelScript later. Their consumer is the
[`wc3-rs`](../wc3-rs) VS Code extension.

This README is the **design reference** for all of them: how a parser
crate is structured and why. Language crates document their *language*
(syntax reference and parser status) in their own READMEs and follow
everything here.

---

## Principles

### 1. Language crates contain only what's language-specific

A language crate has its active keyword set and `LexSpec`, its AST,
grammar, lint rules, AST → `Node` mapping, and semantic-token
classification. Everything else lives here and is reused: spans, the
lexer, literals, token stream, errors, diagnostics, tree renderers, the
HTML viewer and semantic-token encoding. If a second language would
need a copy of something, it belongs in `parser-rs`.

### 2. One vocabulary for all languages; dialects are sets

There is exactly one `TokenKind`, one `Kw` (every keyword of every
language) and one `Punct` (every operator of every language). A language,
or a dialect of one, doesn't define token types. It **selects** a subset:

```rust
const JASS: KwSet  = KwSet::of(&[Kw::Function, Kw::EndFunction, /* ... */]);
const VJASS: KwSet = JASS.union(KwSet::of(&[Kw::Library, Kw::Struct, /* ... */]));

const JASS_SPEC: LexSpec  = LexSpec { keywords: JASS, puncts: &[/* ... */], /* ... */ };
const VJASS_SPEC: LexSpec = LexSpec { keywords: VJASS, block_comment: Some(("/*", "*/")), ..JASS_SPEC };
```

So an optional dialect (vJASS on top of JASS) is **not a second parser**.
It's a bigger keyword set, maybe extra puncts and comment forms, plus
grammar flags for the constructs it adds. Shared spellings share a
variant: JASS `and` and Lua `and` are both `Kw::And`, and Zinc and Lua
`while` are both `Kw::While`. The meaning is up to each parser.

### 3. Keyword activation happens in the lexer, not the parser

A word whose keyword is inactive in the current `LexSpec` lexes as a
plain `Ident`. This matters because a dialect's keywords are valid names
in the base language: `local integer library = 1` is fine in plain JASS.
If the lexer always produced `Keyword(Library)`, every parser rule
expecting a name would have to turn it back into one. With activation in
the lexer, a parser never sees a keyword its dialect doesn't have.
Grammar flags then only decide which *constructs* are allowed.

### 4. Embedded languages: one token, then hand over control

A host language can contain blocks of another language between directive
lines. vJASS has `//! zinc` ... `//! endzinc`, for example. This is
declared, not hand-coded:

```rust
const VJASS_SPEC: LexSpec = LexSpec {
    embeds: &[EmbedSpec { lang: "zinc", open: "//! zinc", close: "//! endzinc" }],
    /* ... */
};
```

1. **Lexer.** A directive must stand alone on its line (indentation
   allowed) and is matched ignoring whitespace, so `//!zinc` counts. The
   lexer swallows the whole block into **one**
   `TokenKind::Embedded(Embedded { lang, source, first_line, open, close })`
   token. The host lexer never tokenizes the embedded code. Without an
   `EmbedSpec`, the same line is just a comment.
2. **Host parser.** Where the block is allowed (vJASS: top level), it
   passes control: `zinc_rs::parse_at(&block.source, block.first_line)`.
   It wraps the result in an AST node (`TopLevel::Zinc`) and continues
   with the next host token. Elsewhere the token is a normal "unexpected
   token" error (`embedded zinc block`).
3. **Positions stay file-absolute.** The embedded crate lexes with
   `tokenize_at` / `tokenize_lossy_at(src, spec, first_line)`. Blocks
   start on a line boundary, so only the line needs offsetting. Its
   errors, AST spans, diagnostics and semantic tokens need no
   translation.
4. **Every feature delegates the same way.** Lint, tree, query and
   semantic highlighting all hand the block to the embedded crate. Its
   directive lines are highlighted as `macro`. An unterminated block is a
   lex error at its opening directive, and the tolerant lexer still
   returns the block to EOF.
5. **The embedded crate exposes `_at` entry points**, such as `parse_at`
   and `semantic_tokens_at`. Dependencies point from host to embedded
   (`jass-rs → zinc-rs`), never back.

### 5. Positions: one internal convention, convert at the boundary

`Span` is 1-based line and 1-based **codepoint** column, end-exclusive.
Statements and declarations may carry `Span::whole_line(line)`, while
expressions carry exact spans. Consumers convert at the edge:
`ColumnEncoding::Utf16` for VS Code/LSP, `Utf8` for byte offsets. Never
store a second encoding on AST nodes.

### 6. Two pipelines: strict for analysis, tolerant for the editor

- **Strict:** `tokenize` (first error wins) → parser → AST → lint, tree
  rendering, queries. A file that doesn't parse gets one `ParseError` and
  no lint.
- **Tolerant:** `tokenize_lossy` (skips bad characters and unterminated
  literals, collects comments) → token-level classification → semantic
  tokens. Editor features like highlighting **must not depend on a
  successful parse**, because the code being typed is usually broken.

### 7. Errors and messages

- A `ParseError` carries `line`, `col` and `end_col` of the offending
  token, so the editor can underline it exactly.
- Messages use `Display`, never `Debug`: `expected ')', found '{'`,
  `unexpected token in expression: 'endfunction'`. Never print Rust
  variant names.
- Lint returns shared `Diagnostic`s (`Level::Error` / `Warning`, a whole
  line), sorted by line.

### 8. Parsing technique

- Hand-written recursive descent over a `TokenStream`
  (`peek`/`check`/`eat`/`expect`/`nth_kind`/`prev`).
- Binary operators are one `TokenParser::left_assoc` call per precedence
  level. Precedence is language-specific and documented in the language
  README. For example, JASS and Zinc bind `and`/`&&` *looser* than
  `or`/`||`.
- Literals use the shared scanners (`literal`): decimal and hex ints,
  reals, `"strings"` with escapes, `'rawcodes'`.

### 9. Editor integration (VS Code)

- Semantic highlighting uses only VS Code's **standard** token types and
  modifiers (`semantic::SemanticType` / `SemanticModifier`, all 23 + 10, in
  documented order). The legend is served from Rust (`semantic::legend`),
  so indices are defined in one place.
- Identifier classification is token-based (see principle 6). It should
  read declarations by their fixed token shapes and gather file-wide names
  (constants, types, natives) in a first pass.
- The server dispatches on the document's `languageId`. Everything past
  that dispatch uses the shared types and is language-agnostic.

### 10. Tests and layout

- Unit tests sit next to the code as `foo.test.rs`, included from
  `foo.rs` via `#[cfg(test)] #[path = "foo.test.rs"] mod tests;`.
- `tests/`: integration tests (`integration.rs`), CLI tests
  (`cli_launch.rs`), and golden HTML snapshots of every fixture
  (`html_snapshot.rs`). Regenerate those with
  `UPDATE_SNAPSHOTS=1 cargo test --test html_snapshot` and review the diff.
- A refactor of shared code must leave every language's snapshots
  byte-identical, unless the change is intended.
- `cargo test` and `cargo clippy --all-targets` stay clean in every crate.

### 11. Documentation

- `parser-rs/README.md` (this file) holds the design and shared modules.
- `<lang>-rs/README.md` holds a syntax reference for the language,
  detailed enough to build on, plus **parser status**: what's supported,
  what isn't yet, and the lint rules.
- Code comments and docs are in English.

---

## Modules

| module       | what                                                                                    |
|--------------|-----------------------------------------------------------------------------------------|
| `span`       | `Span` (1-based line, codepoint columns) + `ColumnEncoding` (UTF-16/UTF-8)              |
| `cursor`     | `Cursor`: char-by-char lexer cursor with line/col, trivia/comment skipping              |
| `literal`    | literal scanners: decimal/hex ints, reals, `"strings"`, `'rawcodes'`                    |
| `kind`       | `TokenKind`, `Kw` + `KwSet`, `Punct`, `Embedded` (an embedded-language block)           |
| `lexer`      | `LexSpec`, `EmbedSpec`; `tokenize`/`tokenize_lossy` (+ comments) and their `_at` forms  |
| `token`      | `Token<K>`, `ParseError`, `TokenStream<K>`, `TokenParser::left_assoc`                   |
| `semantic`   | VS Code's 23 standard token types + 10 modifiers, legend, lexical layer, wire `encode`  |
| `diagnostic` | lint `Diagnostic` + `Level`                                                             |
| `tree`       | labelled `Node` tree, `render_tree` (box drawing), `render_dot` (Graphviz)              |
| `html`       | `render_html`: the self-contained offline AST viewer page                               |

## Adding a language or dialect

1. **Keywords.** Add any missing words to `Kw` in `kind.rs`, one line
   each and grouped by language, then build the language's `KwSet`. For a
   dialect, `union` its extras onto the base set.
2. **Operators.** Add missing operators to `Punct` and list the
   language's operators in its `LexSpec`. Matching is longest-first.
3. **Lexical forms.** If the generic lexer lacks one, add it as a
   `LexSpec` option rather than writing a separate lexer. Lua, for
   example, will need single-quoted strings, `[[long strings]]` and
   `--[[ ]]` comments.
4. **The crate.** Give it the same layout as `jass-rs`/`zinc-rs`: `token`,
   `lexer`, `ast`, `parser`, `lint`, `printer`, `query`, `html`,
   `semantic`, `main`, plus a README with the syntax reference and parser
   status.
5. **Editor.** Add the language to the server's `languageId` dispatch and
   to the extension's `LANGUAGES` / `package.json`.

```rust
use parser_rs::{Kw, KwSet, LexSpec, Punct};

const LUA: LexSpec = LexSpec {
    keywords: KwSet::of(&[Kw::Function, Kw::End, Kw::Local, Kw::Nil /* ... */]),
    line_comment: "--",
    block_comment: None,
    rawcodes: false,
    puncts: &[Punct::Plus, Punct::DotDot, Punct::TildeEq, Punct::Hash /* ... */],
};
let tokens = parser_rs::tokenize(src, &LUA)?;
```
