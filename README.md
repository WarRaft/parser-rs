# parser-rs

Language-agnostic building blocks shared by this author's hand-written
parsers: [`jass-rs`](../jass-rs) (JASS and vJASS), [`zinc-rs`](../zinc-rs),
[`lua-rs`](../lua-rs) (Lua 5.3), [`as-rs`](../as-rs) (AngelScript),
[`bni-rs`](../bni-rs) (BNI, the Warcraft III data-file format) and
[`fdf-rs`](../fdf-rs) (FDF, the game's frame definitions), and the
readers of the game's binary files: [`w3abdhqtu-rs`](../w3abdhqtu-rs)
(object data). Their consumer is the
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

**A dialect is chosen per file**, so a project can mix them: plain JASS,
vJASS with embedded Zinc, and later other dialects such as the Chinese
JASS variant, whose options will build on the same mechanism. A file
declares its dialect in a header comment, read by
`directive::header_directive`:

```jass
// @dialect vjass
```

- **Default.** No declaration means the **base language** (plain JASS).
  A file opts *into* extensions, never out of them, and an unknown name
  also falls back to the base. In the editor, a file that belongs to a
  project (a `wc3.bni` in its folder or above) falls back to the
  project's default dialect first; see the
  [`wc3-rs` README](../wc3-rs/README.md#projects). The library API itself
  never looks at the file system.
- **Header only.** Only the header is read: the leading blank and comment
  lines.
- **Safe for other tools.** The setting is an ordinary comment, so
  JassHelper, the game and other tools ignore it.

**A format whose words are data has no keywords.** In FDF, `Frame`,
`Width`, `INHERITS` and `TOPLEFT` are names the game gives a meaning to,
not grammar: its `LexSpec` activates no keyword, and one rule parses
every statement (name, values, an optional block), whatever its name.
Never hardcode the names such a format happens to use today; the
program that reads the document decides what a name means. Its names
(FDF's frames and keys, declared by a statement's shape) are symbols
in `Scope::Data`: matched by name across the files, apart from code's.
Code refers to them softly: `strings::literals` gives every quoted
string of a text (its value, its text's span inside the quotes), and a
consumer links the ones whose text is such a name, whatever they're
passed to.

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
- **One lexer for everything that colors source.** The HTML viewer's
  source panel runs the language's own `LexSpec` through
  `tokenize_lossy` too, never a separate hand-written highlighter. So the
  viewer, the editor and the parser always agree on what a keyword,
  string or comment is. Embedded blocks are colored by their own
  language's spec (`HtmlLang::embedded`).
- **Multi-line tokens.** Lua long strings and AngelScript heredocs keep a
  real `end_line` (`Token::spanning`). Every consumer splits them per line
  where it needs to: `semantic::encode` and the HTML viewer do.
- **Line-oriented data formats** (BNI) are the one exception to the
  shared lexer: when a character's meaning depends on its place in the
  line and values are free text, the crate has its own line scanner
  with its own token kinds. That scanner is then *the* lexer: the
  parser, the semantic highlighter and the viewer all read it, and the
  viewer gets its colors as semantic tokens via `render_html_tokens`.
  Such a format also parses line by line and never fails as a whole: a
  malformed line becomes an error in the document and parsing goes on.
- **Never stall.** A tolerant parser or classifier must always make
  progress. When a construct is cut off (an unclosed parameter list, for
  example), it ends it at the first token that can't belong to it,
  instead of swallowing the rest of the file.

### 7. Errors and messages

- A `ParseError` carries `line`, `col` and `end_col` of the offending
  token, so the editor can underline it exactly.
- Messages use `Display`, never `Debug`: `expected ')', found '{'`,
  `unexpected token in expression: 'endfunction'`. Never print Rust
  variant names.
- **Recognize misplaced code, then report it.** A construct that is
  well-formed but in the wrong place, such as a JASS `if` at the top
  level, is parsed in full and kept in the AST. Lint then reports it
  with the fix ("move it into a function"). The parse doesn't fail, so
  the rest of the file still gets its tree, lint and highlighting, and
  the message can say what to do instead of "unexpected token".
- Lint returns shared `Diagnostic`s (`Level::Error` / `Warning`, a whole
  line), sorted by line. A rule that knows exactly what it's about marks
  that (`Diagnostic::at(span)`): an empty `else` is marked on `else`, not
  on its `if`. A rule may offer a **fix** (`with_fix`, `Fix`: a title and
  edits on spans), which the editor shows as a quick fix; the AST keeps
  whatever spans the fix needs. Each names its **rule** (`diagnostic::rule`), one
  id per check across languages; `RULES` lists them with whether each is
  on by default. A new check gets an id there, and a row in the rule
  table of the `wc3-rs` README.
- **Groups** (`diagnostic::group`, `GROUPS`) switch several rules at once;
  a rule lists the groups it belongs to (`Rule::groups`). A group id is
  never also a rule id. `pure` holds the checks for **pure JASS as the
  game reads it**: what this toolchain accepts beyond the game's grammar
  (such as left-out `takes` / `returns`) is fine by default and reported
  once a project turns `pure` on. A relaxation of the game's grammar gets
  a rule in `pure`, so a project that must stay game-exact can say so.

### 8. Parsing technique

- Hand-written recursive descent over a `TokenStream`
  (`peek`/`check`/`eat`/`expect`/`nth_kind`/`prev`).
- Binary operators are one `TokenParser::left_assoc` call per precedence
  level. Precedence is language-specific and documented in the language
  README. For example, JASS and Zinc bind `and`/`&&` *looser* than
  `or`/`||`.
- Literals use the shared scanners (`literal`): decimal and hex ints,
  reals, `"strings"` with escapes, `'rawcodes'`. A language whose
  strings are paths (FDF) sets `raw_strings`: no escapes, a backslash
  is itself.

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

### 12. Binary formats

The game's binary files (object data, `war3map.w3i`, `.doo`, ...) get a
crate each, like a language, built on the `bin` module:

- **Read field by field** with `bin::Reader`, the way the format is laid
  out. No derive macros or generic schema: the layouts branch on version
  and file type (`.w3d` has levels only since version 2), and plain code
  says so most clearly.
- **Errors carry the byte offset** (`BinError`) and say what was wrong in
  words: `field uhpm has unknown value type 7 (at byte 0x18)`. A count
  is checked against the bytes left (`Reader::count`), so a corrupt file
  fails instead of allocating gigabytes. Strings are UTF-8; anything else
  is an error, not a lossy guess.
- **Reading and writing are exact.** Each crate writes with
  `bin::Writer`, and `write(parse(bytes)) == bytes` for every real file.
  Whatever the format stores, the data model keeps, even values the game
  ignores, so the editor can show a file as text and save it back
  without changing a byte it didn't mean to. Bytes after the end of the
  data are an error.
- **`Rawcode`** is the shared four-character id. It displays as its
  characters (`hfoo`) or, when one isn't printable, as `0x` and the
  big-endian value, the way JASS writes `'hfoo'`; `Rawcode::parse`
  reads both back.
- **Tested on real files.** Unit tests build files byte by byte; a
  `corpus` test (ignored by default) checks the round trip over a folder
  of files taken from real maps.

---

## Modules

| module       | what                                                                                    |
|--------------|-----------------------------------------------------------------------------------------|
| `span`       | `Span` (1-based line, codepoint columns) + `ColumnEncoding` (UTF-16/UTF-8)              |
| `cursor`     | `Cursor`: char-by-char lexer cursor with line/col, trivia/comment skipping              |
| `literal`    | scanners: numbers per `NumberSpec`, quoted strings, rawcodes, Lua long brackets, heredocs |
| `kind`       | `TokenKind`, `Kw` + `KwSet`, `Punct`, `Embedded` (an embedded-language block)           |
| `lexer`      | `LexSpec` (`LexSpec::BASE` + options), `EmbedSpec`; `tokenize`/`tokenize_lossy` (+ `_at`) |
| `token`      | `Token<K>`, `ParseError`, `TokenStream<K>`, `TokenParser::left_assoc`                   |
| `semantic`   | VS Code's 23 standard token types + 10 modifiers, legend, lexical layer, wire `encode`  |
| `directive`  | `header_directive`: per-file settings in header comments (`// @dialect vjass`)          |
| `symbols`    | `Occurrence` (a declared or referenced name, its `SymbolKind`, `Scope` and namespace), and namespace lookup (`lookup_chain`, `resolve`): the input of the editor's symbol index |
| `diagnostic` | lint `Diagnostic` + `Level`; rule ids (`rule`) and the rule list (`RULES`, `find_rule`); groups (`group`, `GROUPS`, `find_group`); `Edit` / `Fix`, and `apply_edits` to apply edits to a text |
| `tree`       | labelled `Node` tree, `render_tree` (box drawing), `render_dot` (Graphviz)              |
| `bin`        | binary files: little-endian `Reader` / `Writer`, `BinError` (message + byte offset), `Rawcode` |
| `html`       | `render_html`: the offline AST viewer, source colored by the language's own lexer; `render_html_tokens` colors it from semantic tokens instead (BNI) |

## Adding a language or dialect

1. **Keywords.** Add any missing words to `Kw` in `kind.rs`, one line
   each and grouped by language, then build the language's `KwSet`. For a
   dialect, `union` its extras onto the base set.
2. **Operators.** Add missing operators to `Punct` and list the
   language's operators in its `LexSpec`. Matching is longest-first.
3. **Lexical forms.** If the generic lexer lacks one, add it as a
   `LexSpec` option rather than writing a separate lexer. The options so
   far:

   | option                 | effect                                   | used by       |
   |------------------------|------------------------------------------|---------------|
   | `line_comment`         | line comment opener (`//`, `--`)         | all           |
   | `block_comment`        | `/* */`-style block comments             | vJASS, Zinc, AS |
   | `rawcodes`             | `'abcd'` packs into an integer, `'a'` is its byte | JASS, Zinc |
   | `single_quote_strings` | `'...'` is a string                      | Lua, AS       |
   | `long_brackets`        | `[[...]]` strings, `--[[ ]]` comments    | Lua           |
   | `heredocs`             | `"""..."""` strings                | AS            |
   | `numbers`              | exponents, `.5`, `0b`/`0o`/`0d`, `1.5f`  | per language  |
   | `embeds`               | embedded-language blocks                 | vJASS         |
4. **The crate.** Give it the same layout as the existing crates: `token`,
   `lexer`, `ast`, `parser`, `lint`, `printer`, `query`, `html`,
   `semantic`, `main`, plus a README with the syntax reference and parser
   status.
5. **Editor.** Add the language to the server's `languageId` dispatch and
   to the extension's `LANGUAGES` / `package.json`.

This is how `lua-rs` describes Lua (abridged):

```rust
use parser_rs::{Kw, KwSet, LexSpec, NumberSpec, Punct};

pub const SPEC: LexSpec = LexSpec {
    keywords: KwSet::of(&[Kw::Function, Kw::End, Kw::Local, Kw::Nil /* ... */]),
    line_comment: "--",
    single_quote_strings: true,
    long_brackets: true,
    numbers: NumberSpec { exponent: true, leading_dot: true, ..LexSpec::BASE.numbers },
    puncts: &[Punct::Plus, Punct::DotDot, Punct::TildeEq, Punct::Hash /* ... */],
    ..LexSpec::BASE
};
let tokens = parser_rs::tokenize(src, &SPEC)?;
```
