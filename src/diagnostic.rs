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
    /// 1-indexed line the diagnostic applies to (as a whole).
    pub line: usize,
    /// The id of the rule that produced it: one of [`rule`].
    pub rule: &'static str,
}

impl Diagnostic {
    pub fn error(rule: &'static str, message: impl Into<String>, line: usize) -> Self {
        Diagnostic {
            level: Level::Error,
            message: message.into(),
            line,
            rule,
        }
    }

    pub fn warning(rule: &'static str, message: impl Into<String>, line: usize) -> Self {
        Diagnostic {
            level: Level::Warning,
            message: message.into(),
            line,
            rule,
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
    pub const DECLARATION_AFTER_FUNCTION: &str = "declaration-after-function";
    pub const GLOBAL_OUTSIDE_GLOBALS: &str = "global-outside-globals";
}

/// Group ids. Kebab-case, stable, never the same as a rule id.
pub mod group {
    pub const PURE: &str = "pure";
}

/// A group of rules switched together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Group {
    pub id: &'static str,
    pub summary: &'static str,
}

/// Every group. Keep in step with the `wc3-rs` README.
pub const GROUPS: &[Group] = &[Group {
    id: group::PURE,
    summary: "pure JASS as the game reads it: what this toolchain accepts beyond it",
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
        groups: &[group::PURE],
    },
    Rule {
        id: rule::DECLARATION_AFTER_FUNCTION,
        languages: &["JASS"],
        enabled: false,
        summary: "a global, `globals` block, `native` or `type` after a function; the game reads them only before the first one",
        groups: &[group::PURE],
    },
    Rule {
        id: rule::GLOBAL_OUTSIDE_GLOBALS,
        languages: &["JASS"],
        enabled: false,
        summary: "a variable declared at the top level outside `globals ... endglobals`, where the game doesn't read it",
        groups: &[group::PURE],
    },
];

/// The rule with id `id`.
pub fn find_rule(id: &str) -> Option<&'static Rule> {
    RULES.iter().find(|r| r.id == id)
}

#[cfg(test)]
#[path = "diagnostic.test.rs"]
mod tests;
