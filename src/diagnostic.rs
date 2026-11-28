//! Lint diagnostics, shared so that every consumer (CLI, editor backend)
//! handles every language's lint output the same way.
//!
//! Every diagnostic names the **rule** that produced it ([`rule`]). A rule
//! has one id across languages when it checks the same thing (an unused
//! local is `unused-local` in JASS, Zinc, Lua and AngelScript), so one
//! setting covers it everywhere. [`RULES`] lists them all, with whether
//! each is on by default; a project can turn rules on or off, or change
//! their level (`[lint]` in `wc3.bni`).
//!
//! A rule may belong to **groups** ([`group`], [`GROUPS`]): rules that are
//! switched together. A project setting for a group applies to each of its
//! rules, unless the project sets that rule by name.

use crate::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Error,
    Warning,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Error => "error",
            Level::Warning => "warning",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub level: Level,
    pub message: String,
    /// 1-indexed line the diagnostic applies to (as a whole), unless
    /// `span` is more exact.
    pub line: usize,
    /// The id of the rule that produced it: one of [`rule`].
    pub rule: &'static str,
    /// Exactly what it's about, when the rule knows: the editor marks this
    /// instead of the whole `line`.
    pub span: Option<Span>,
    /// An edit that resolves it, which the editor offers as a quick fix.
    pub fix: Option<Fix>,
}

/// A quick fix: a title for the menu and the edits that make it.
#[derive(Debug, Clone, PartialEq)]
pub struct Fix {
    pub title: String,
    pub edits: Vec<Edit>,
}

/// Replace `span` (1-indexed codepoint columns, end exclusive; column 1
/// of the next line to take a line's end with it) with `text`.
#[derive(Debug, Clone, PartialEq)]
pub struct Edit {
    pub span: Span,
    pub text: String,
}

/// `src` with `edits` applied together: their spans (1-based lines,
/// character columns, end exclusive) are all on `src` as it is, and don't
/// overlap. For tools that change a file through edits, as an editor
/// would, and for their tests.
pub fn apply_edits(src: &str, edits: &[Edit]) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut line_starts = vec![0];
    for (i, &c) in chars.iter().enumerate() {
        if c == '\n' {
            line_starts.push(i + 1);
        }
    }
    let offset = |line: usize, col: usize| {
        line_starts
            .get(line.saturating_sub(1))
            .map_or(chars.len(), |s| (s + col.saturating_sub(1)).min(chars.len()))
    };
    let mut ranges: Vec<(usize, usize, &str)> = edits
        .iter()
        .map(|e| (offset(e.span.start_line, e.span.start_col), offset(e.span.end_line, e.span.end_col), e.text.as_str()))
        .collect();
    ranges.sort_by_key(|r| (r.0, r.1));
    let mut out = String::new();
    let mut pos = 0;
    for (start, end, text) in ranges {
        out.extend(&chars[pos..start.max(pos)]);
        out += text;
        pos = end.max(pos);
    }
    out.extend(&chars[pos..]);
    out
}

impl Fix {
    /// A fix that deletes `span`.
    pub fn delete(title: impl Into<String>, span: Span) -> Self {
        Fix { title: title.into(), edits: vec![Edit { span, text: String::new() }] }
    }
}

impl Diagnostic {
    /// Marks exactly `span` (its start line becomes `line`).
    pub fn at(mut self, span: Span) -> Self {
        self.line = span.start_line;
        self.span = Some(span);
        self
    }

    pub fn with_fix(mut self, fix: Fix) -> Self {
        self.fix = Some(fix);
        self
    }

    pub fn error(rule: &'static str, message: impl Into<String>, line: usize) -> Self {
        Diagnostic {
            level: Level::Error,
            message: message.into(),
            line,
            rule,
            span: None,
            fix: None,
        }
    }

    pub fn warning(rule: &'static str, message: impl Into<String>, line: usize) -> Self {
        Diagnostic {
            level: Level::Warning,
            message: message.into(),
            line,
            rule,
            span: None,
            fix: None,
        }
    }
}

/// Rule ids. Kebab-case, stable: projects refer to them by name.
pub mod rule {
    pub const STATEMENT_OUTSIDE_FUNCTION: &str = "statement-outside-function";
    pub const MODIFIER_OUTSIDE_LIBRARY: &str = "modifier-outside-library";
    pub const DUPLICATE_DECLARATION: &str = "duplicate-declaration";
    pub const UNUSED_LOCAL: &str = "unused-local";
    pub const SHADOWED_VARIABLE: &str = "shadowed-variable";
    pub const MISSING_RETURN: &str = "missing-return";
    pub const INFINITE_LOOP: &str = "infinite-loop";
    pub const EMPTY_BRANCH: &str = "empty-branch";
    pub const BREAK_OUTSIDE_LOOP: &str = "break-outside-loop";
    pub const CONTINUE_OUTSIDE_LOOP: &str = "continue-outside-loop";
    pub const DUPLICATE_LABEL: &str = "duplicate-label";
    pub const UNDEFINED_LABEL: &str = "undefined-label";
    pub const VARARG_OUTSIDE_VARARG_FUNCTION: &str = "vararg-outside-vararg-function";
    pub const ITEM_OUTSIDE_SECTION: &str = "item-outside-section";
    pub const EMPTY_SECTION_NAME: &str = "empty-section-name";
    pub const DUPLICATE_SECTION: &str = "duplicate-section";
    pub const UNCLOSED_SECTION_HEADER: &str = "unclosed-section-header";
    pub const UNTERMINATED_STRING: &str = "unterminated-string";
    pub const IMPLICIT_SIGNATURE: &str = "implicit-signature";
    pub const IMPLICIT_KEYWORD: &str = "implicit-keyword";
    pub const UNKNOWN_NAME: &str = "unknown-name";
    pub const ARGUMENT_COUNT: &str = "argument-count";
    pub const TYPE_MISMATCH: &str = "type-mismatch";
    pub const ARRAY_ACCESS: &str = "array-access";
    pub const CONSTANT_ASSIGNMENT: &str = "constant-assignment";
    pub const ARRAY_STRUCT: &str = "array-struct";
    pub const MERGED_FUNCTION: &str = "merged-function";
    pub const DECLARATION_AFTER_FUNCTION: &str = "declaration-after-function";
    pub const GLOBAL_OUTSIDE_GLOBALS: &str = "global-outside-globals";
    pub const CALL_BEFORE_DECLARATION: &str = "call-before-declaration";
    pub const LOCAL_AFTER_STATEMENT: &str = "local-after-statement";
    pub const SHADOWED_NAME: &str = "shadowed-name";
    pub const HIDDEN_TYPE: &str = "hidden-type";
    pub const RESERVED_NAME: &str = "reserved-name";
    pub const UNINITIALIZED_VARIABLE: &str = "uninitialized-variable";
    pub const RETURN_CONVERSION: &str = "return-conversion";
    pub const CONSTANT_FUNCTION: &str = "constant-function";
    pub const RECURSIVE_LOCAL: &str = "recursive-local";
    pub const NATIVE_AS_CODE: &str = "native-as-code";
    pub const TOO_MANY_ARGUMENTS: &str = "too-many-arguments";
    pub const UNSAFE_IN_GLOBALS: &str = "unsafe-in-globals";
    pub const CODE_ARRAY: &str = "code-array";
    pub const ARRAY_INITIALIZER: &str = "array-initializer";
    pub const CONSTANT_WITHOUT_VALUE: &str = "constant-without-value";
    pub const LONG_STRING: &str = "long-string";
    pub const UNSAFE_RAWCODE: &str = "unsafe-rawcode";
    pub const NUMBER_OVERFLOW: &str = "number-overflow";
    pub const LONG_NAME: &str = "long-name";
    pub const FILTER_RETURNS_BOOLEAN: &str = "filter-returns-boolean";
    pub const SHADOWED_GLOBAL: &str = "shadowed-global";
    pub const MODULO_OPERATOR: &str = "modulo-operator";
    pub const GLOBAL_MAYBE_UNINITIALIZED: &str = "global-maybe-uninitialized";
    pub const STRING_HASH_COLLISION: &str = "string-hash-collision";
    pub const OBJECT_VALUE_TYPE: &str = "object-value-type";
    pub const OBJECT_VALUE_RANGE: &str = "object-value-range";
    pub const OBJECT_UNKNOWN_FIELD: &str = "object-unknown-field";
    pub const OBJECT_FIELD_LEVEL: &str = "object-field-level";
    pub const OBJECT_FIELD_OWNER: &str = "object-field-owner";
    pub const DUPLICATE_CELL: &str = "duplicate-cell";
    pub const CELL_OUTSIDE_BOUNDS: &str = "cell-outside-bounds";
    pub const DUPLICATE_STRING: &str = "duplicate-string";
    pub const MISSING_FILE: &str = "missing-file";
    pub const TOC_LAST_LINE: &str = "toc-last-line";
}

/// Group ids. Kebab-case, stable, never the same as a rule id.
pub mod group {
    pub const STRICT: &str = "strict";
    pub const WARNINGS: &str = "warnings";
}

/// A group of rules switched together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Group {
    pub id: &'static str,
    pub summary: &'static str,
}

/// Every group. Keep in step with the `wc3-rs` README.
pub const GROUPS: &[Group] = &[Group {
    id: group::STRICT,
    summary: "strict JASS, as the game reads it: what extended JASS and this toolchain accept beyond it, and what breaks the game's compiling or running the script; always errors in the strict dialect",
}, Group {
    id: group::WARNINGS,
    summary: "JASS the game reads and runs, but that's likely a mistake or a limit of a patch: warnings, each set on its own",
}];

/// The group with id `id`.
pub fn find_group(id: &str) -> Option<&'static Group> {
    GROUPS.iter().find(|g| g.id == id)
}

/// A lint rule: its id, the languages that check it, whether it's on
/// unless a project says otherwise, and what it finds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rule {
    pub id: &'static str,
    pub languages: &'static [&'static str],
    pub enabled: bool,
    pub summary: &'static str,
    /// The groups it belongs to ([`group`]).
    pub groups: &'static [&'static str],
}

const ALL: &[&str] = &["JASS", "Zinc", "Lua", "AngelScript"];

/// Every rule. Keep in step with the table in the `wc3-rs` README.
pub const RULES: &[Rule] = &[
    Rule {
        id: rule::STATEMENT_OUTSIDE_FUNCTION,
        languages: &["JASS", "Zinc"],
        enabled: true,
        summary: "a statement at the top level, where only declarations belong",
        groups: &[],
    },
    Rule {
        id: rule::MODIFIER_OUTSIDE_LIBRARY,
        languages: &["Zinc"],
        enabled: true,
        summary: "`private` / `public` outside a library, where it has no effect",
        groups: &[],
    },
    Rule {
        id: rule::DUPLICATE_DECLARATION,
        languages: &["JASS", "Zinc", "AngelScript"],
        enabled: true,
        summary: "a name declared twice in the same scope",
        groups: &[],
    },
    Rule {
        id: rule::UNUSED_LOCAL,
        languages: ALL,
        enabled: true,
        summary: "a local variable (or Lua local function) that's never used",
        groups: &[],
    },
    Rule {
        id: rule::SHADOWED_VARIABLE,
        languages: &["AngelScript"],
        enabled: true,
        summary: "a local that hides a variable of an outer scope",
        groups: &[],
    },
    Rule {
        id: rule::MISSING_RETURN,
        languages: &["JASS", "Zinc", "AngelScript"],
        enabled: true,
        summary: "a function (or `get` accessor) with a return type that may not return a value",
        groups: &[group::STRICT],
    },
    Rule {
        id: rule::INFINITE_LOOP,
        languages: ALL,
        enabled: false,
        summary: "a loop with no way out: a JASS `loop` without `exitwhen`, `while (true)` without `break` or `return`. \
                  Off by default: a thread that loops forever and sleeps (as the game's AI scripts do) is normal",
        groups: &[],
    },
    Rule {
        id: rule::EMPTY_BRANCH,
        languages: ALL,
        enabled: true,
        summary: "an empty `then` / `else` / `if` body",
        groups: &[],
    },
    Rule {
        id: rule::BREAK_OUTSIDE_LOOP,
        languages: &["Zinc", "Lua", "AngelScript"],
        enabled: true,
        summary: "`break` outside a loop (or switch)",
        groups: &[],
    },
    Rule {
        id: rule::CONTINUE_OUTSIDE_LOOP,
        languages: &["AngelScript"],
        enabled: true,
        summary: "`continue` outside a loop",
        groups: &[],
    },
    Rule {
        id: rule::DUPLICATE_LABEL,
        languages: &["Lua"],
        enabled: true,
        summary: "a label defined twice in the same block",
        groups: &[],
    },
    Rule {
        id: rule::UNDEFINED_LABEL,
        languages: &["Lua"],
        enabled: true,
        summary: "a `goto` with no visible label",
        groups: &[],
    },
    Rule {
        id: rule::VARARG_OUTSIDE_VARARG_FUNCTION,
        languages: &["Lua"],
        enabled: true,
        summary: "`...` in a function that doesn't take it",
        groups: &[],
    },
    Rule {
        id: rule::ITEM_OUTSIDE_SECTION,
        languages: &["BNI"],
        enabled: true,
        summary: "a `key=value` before the first `[section]`",
        groups: &[],
    },
    Rule {
        id: rule::EMPTY_SECTION_NAME,
        languages: &["BNI"],
        enabled: true,
        summary: "a `[]` section header",
        groups: &[],
    },
    Rule {
        id: rule::DUPLICATE_SECTION,
        languages: &["BNI"],
        enabled: true,
        summary: "a section header repeated in the file",
        groups: &[],
    },
    Rule {
        id: rule::UNCLOSED_SECTION_HEADER,
        languages: &["BNI"],
        enabled: true,
        summary: "a section header missing its `]`",
        groups: &[],
    },
    Rule {
        id: rule::UNTERMINATED_STRING,
        languages: &["BNI"],
        enabled: true,
        summary: "a quoted string with no closing `\"`",
        groups: &[],
    },
    Rule {
        id: rule::IMPLICIT_SIGNATURE,
        languages: &["JASS"],
        enabled: false,
        summary: "a function or native without `takes` or `returns`, which the game requires",
        groups: &[group::STRICT],
    },
    Rule {
        id: rule::UNKNOWN_NAME,
        languages: &["JASS", "FDF"],
        enabled: true,
        summary: "a function, variable or type declared nowhere: in the file, the project or the game's scripts; a frame an FDF block inherits (`INHERITS \"Template\"`) that no frame file declares",
        groups: &[],
    },
    Rule {
        id: rule::ARGUMENT_COUNT,
        languages: &["JASS"],
        enabled: true,
        summary: "a call with more or fewer arguments than its function takes, or a code value of a function that takes some",
        groups: &[],
    },
    Rule {
        id: rule::TYPE_MISMATCH,
        languages: &["JASS"],
        enabled: true,
        summary: "a value of the wrong type: an argument, a value set or returned, a condition, an operator's side",
        groups: &[],
    },
    Rule {
        id: rule::ARRAY_ACCESS,
        languages: &["JASS"],
        enabled: true,
        summary: "an array used without its index, or an index of what isn't an array",
        groups: &[],
    },
    Rule {
        id: rule::CONSTANT_ASSIGNMENT,
        languages: &["JASS"],
        enabled: true,
        summary: "a constant set",
        groups: &[],
    },
    Rule {
        id: rule::ARRAY_STRUCT,
        languages: &["JASS"],
        enabled: true,
        summary: "what a struct `extends array` can't have or do, its instances only numbers that nothing creates or destroys: a member's default value, an array member, `onDestroy` (its own or a module's), `allocate`, `create`, `destroy`, `deallocate` not its own",
        groups: &[],
    },
    Rule {
        id: rule::IMPLICIT_KEYWORD,
        languages: &["JASS"],
        enabled: false,
        summary: "an assignment without `set`, a call without `call` or a local without `local`, which the game requires",
        groups: &[group::STRICT],
    },
    Rule {
        id: rule::MERGED_FUNCTION,
        languages: &["JASS"],
        enabled: false,
        summary: "more than one `main` or `config`, which the build merges into one; the game reads one",
        groups: &[group::STRICT],
    },
    Rule {
        id: rule::CALL_BEFORE_DECLARATION,
        languages: &["JASS"],
        enabled: false,
        summary: "a function called (or named by `function X`) above its declaration; the game knows a function only from its declaration down",
        groups: &[group::STRICT],
    },
    Rule {
        id: rule::LOCAL_AFTER_STATEMENT,
        languages: &["JASS"],
        enabled: false,
        summary: "a `local` after a function's first other statement; the game reads locals only at its top (a build moves them there)",
        groups: &[group::STRICT],
    },
    Rule {
        id: rule::SHADOWED_NAME,
        languages: &["JASS"],
        enabled: true,
        summary: "a parameter or a local named as a declared type or a function (`local effect effect`): legal, the context tells them apart, but confusing (a build renames it)",
        groups: &[group::WARNINGS],
    },
    Rule {
        id: rule::HIDDEN_TYPE,
        languages: &["JASS"],
        enabled: true,
        summary: "a name the game keeps for a type: a parameter or a local named as a primitive (`local integer integer`), a global named as a type, or a type named after a parameter or a local of its name hid it in a function",
        groups: &[group::STRICT],
    },
    Rule {
        id: rule::RESERVED_NAME,
        languages: &["JASS"],
        enabled: false,
        summary: "a name the game keeps for itself: `alias` (a build renames it)",
        groups: &[group::STRICT],
    },
    Rule {
        id: rule::UNINITIALIZED_VARIABLE,
        languages: &["JASS"],
        enabled: true,
        summary: "a local read before anything sets it, or a global read in `globals` before its value: the game stops the thread (crashes, in `globals`)",
        groups: &[group::STRICT],
    },
    Rule {
        id: rule::RETURN_CONVERSION,
        languages: &["JASS"],
        enabled: true,
        summary: "a value returned that the game won't convert: an integer from a function returning real, `null` from one returning integer, real, boolean or code",
        groups: &[group::STRICT],
    },
    Rule {
        id: rule::CONSTANT_FUNCTION,
        languages: &["JASS"],
        enabled: true,
        summary: "a `constant function` that sets a global or calls a function that isn't constant",
        groups: &[group::STRICT],
    },
    Rule {
        id: rule::RECURSIVE_LOCAL,
        languages: &["JASS"],
        enabled: true,
        summary: "a function calling itself in a local's value",
        groups: &[group::STRICT],
    },
    Rule {
        id: rule::NATIVE_AS_CODE,
        languages: &["JASS"],
        enabled: true,
        summary: "`function N` of a native: the game can't run it as code",
        groups: &[group::STRICT],
    },
    Rule {
        id: rule::TOO_MANY_ARGUMENTS,
        languages: &["JASS"],
        enabled: true,
        summary: "a call with more than 31 arguments, more than the game passes",
        groups: &[group::STRICT],
    },
    Rule {
        id: rule::UNSAFE_IN_GLOBALS,
        languages: &["JASS"],
        enabled: true,
        summary: "a call in `globals` the game can't make yet: one that crashes it (`GetObjectName`, `CreateQuest`, `CreateMultiboard`, `CreateLeaderboard`, `CreateRegion` on saving) or always gives null (`OrderId`, `OrderId2String`, `UnitId2String`)",
        groups: &[group::STRICT],
    },
    Rule {
        id: rule::CODE_ARRAY,
        languages: &["JASS"],
        enabled: true,
        summary: "an array of `code`, which the game doesn't have",
        groups: &[group::STRICT],
    },
    Rule {
        id: rule::ARRAY_INITIALIZER,
        languages: &["JASS"],
        enabled: true,
        summary: "an array given a value where it's declared, which the game doesn't read",
        groups: &[group::STRICT],
    },
    Rule {
        id: rule::CONSTANT_WITHOUT_VALUE,
        languages: &["JASS"],
        enabled: true,
        summary: "a constant declared without its value",
        groups: &[group::STRICT],
    },
    Rule {
        id: rule::UNSAFE_RAWCODE,
        languages: &["JASS"],
        enabled: true,
        summary: "a rawcode of bytes an editor can't show or keep (a control character, a line break, bytes that aren't UTF-8): the game reads its bytes as they are, an editor saving the file changes them; written as `0x...`, the same number, it's safe",
        groups: &[group::WARNINGS],
    },
    Rule {
        id: rule::LONG_STRING,
        languages: &["JASS"],
        enabled: true,
        summary: "a string literal longer, in bytes, than the game keeps when a saved game is loaded (1023; a concatenation of literals, 4092), which crashes loading it; `[jass] string_limit` and `concat_limit` set the limits (0: none), for UjAPI or Reforged",
        groups: &[group::WARNINGS],
    },
    Rule {
        id: rule::NUMBER_OVERFLOW,
        languages: &["JASS"],
        enabled: true,
        summary: "a number literal too big for its type: the game cuts it, as its parser does",
        groups: &[group::WARNINGS],
    },
    Rule {
        id: rule::LONG_NAME,
        languages: &["JASS"],
        enabled: true,
        summary: "a name longer than 3958 characters, more than the game reads (a generator's slip)",
        groups: &[group::WARNINGS],
    },
    Rule {
        id: rule::FILTER_RETURNS_BOOLEAN,
        languages: &["JASS"],
        enabled: true,
        summary: "a function given to `Filter` or `Condition` that doesn't return boolean",
        groups: &[group::WARNINGS],
    },
    Rule {
        id: rule::SHADOWED_GLOBAL,
        languages: &["JASS"],
        enabled: false,
        summary: "a local or a parameter named as a global variable, which it hides there (legal; a build renames it)",
        groups: &[group::WARNINGS],
    },
    Rule {
        id: rule::MODULO_OPERATOR,
        languages: &["JASS"],
        enabled: false,
        summary: "the `%` operator, which patches before 1.29 don't have",
        groups: &[group::WARNINGS],
    },
    Rule {
        id: rule::GLOBAL_MAYBE_UNINITIALIZED,
        languages: &["JASS"],
        enabled: false,
        summary: "a global without a value read in a function: it may not be set yet (often set lazily, so off)",
        groups: &[],
    },
    Rule {
        id: rule::STRING_HASH_COLLISION,
        languages: &["JASS"],
        enabled: false,
        summary: "two different strings with the same `StringHash` (a hash's way: off)",
        groups: &[],
    },
    Rule {
        id: rule::DECLARATION_AFTER_FUNCTION,
        languages: &["JASS"],
        enabled: false,
        summary: "a global, `globals` block, `native` or `type` after a function; the game reads them only before the first one",
        groups: &[group::STRICT],
    },
    Rule {
        id: rule::GLOBAL_OUTSIDE_GLOBALS,
        languages: &["JASS"],
        enabled: false,
        summary: "a variable declared at the top level outside `globals ... endglobals`, where the game doesn't read it",
        groups: &[group::STRICT],
    },
    Rule {
        id: rule::OBJECT_VALUE_TYPE,
        languages: &["Object data"],
        enabled: true,
        summary: "a value stored as another type than the game's metadata gives the field (a number for a string, `int 6` for an unreal): the game misreads it",
        groups: &[],
    },
    Rule {
        id: rule::OBJECT_VALUE_RANGE,
        languages: &["Object data"],
        enabled: true,
        summary: "a number outside World Editor's limits for the field, or a flag other than 0 or 1",
        groups: &[],
    },
    Rule {
        id: rule::OBJECT_UNKNOWN_FIELD,
        languages: &["Object data"],
        enabled: true,
        summary: "a field the game's metadata doesn't have, like one of a newer game version",
        groups: &[],
    },
    Rule {
        id: rule::OBJECT_FIELD_LEVEL,
        languages: &["Object data"],
        enabled: true,
        summary: "a level or data column that doesn't fit the field: a level on a field without levels, none on one with, another column than its own",
        groups: &[],
    },
    Rule {
        id: rule::OBJECT_FIELD_OWNER,
        languages: &["Object data"],
        enabled: true,
        summary: "a field of other objects: an ability's data of another ability, a units-only field of an item",
        groups: &[],
    },
    Rule {
        id: rule::DUPLICATE_CELL,
        languages: &["SLK"],
        enabled: true,
        summary: "a cell set by two records; the later one wins",
        groups: &[],
    },
    Rule {
        id: rule::CELL_OUTSIDE_BOUNDS,
        languages: &["SLK"],
        enabled: true,
        summary: "a cell past the size the B record gives, which readers that trust B lose",
        groups: &[],
    },
    Rule {
        id: rule::DUPLICATE_STRING,
        languages: &["WTS"],
        enabled: true,
        summary: "a string number defined twice in a string table",
        groups: &[],
    },
    Rule {
        id: rule::MISSING_FILE,
        languages: &["FDF", "TOC"],
        enabled: true,
        summary: "a frame file a `.toc` lists, or an FDF string names (`IncludeFile \"....fdf\"`), that's neither the project's nor the game's",
        groups: &[],
    },
    Rule {
        id: rule::TOC_LAST_LINE,
        languages: &["TOC"],
        enabled: true,
        summary: "a `.toc` file's last line without a line break after it: the game doesn't read it",
        groups: &[],
    },
];

/// The rule with id `id`.
pub fn find_rule(id: &str) -> Option<&'static Rule> {
    RULES.iter().find(|r| r.id == id)
}

#[cfg(test)]
#[path = "diagnostic.test.rs"]
mod tests;
