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
    summary: "strict JASS, as the game reads it: what extended JASS and this toolchain accept beyond it; always errors in the strict dialect",
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
        groups: &[],
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
