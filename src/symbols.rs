//! Symbols: what a file declares and what its names refer to, as a flat
//! list of occurrences. A language crate produces it from the same
//! token-based classifier that produces its semantic tokens, so the index
//! and the highlighting always agree, and half-typed code is indexed too.
//!
//! An occurrence doesn't point at its declaration. It carries the name
//! and the scope the name is resolved in; a consumer that sees the whole
//! project (the editor backend) matches names within that scope:
//!
//! - [`Scope::Project`]: visible from every file of the project, like
//!   JASS functions, natives, globals and types. Project-scope names live
//!   in **namespaces** (AngelScript's; every other language has only the
//!   global one): two project-scope occurrences are the same symbol when
//!   they have the same name and resolve to the same namespace (see
//!   [`Occurrence::resolve`]).
//! - [`Scope::Local`]`(n)`: visible only inside the `n`-th local scope of
//!   this file (a JASS function body: its parameters and locals; a Zinc
//!   library: its private members). Two functions' `i` are different
//!   symbols.
//! - [`Scope::Member`]: a member of a struct-like type (Zinc and
//!   AngelScript fields and methods, Lua table fields), used after a `.`.
//!   Telling which type `obj.value` belongs to takes type inference, so members are matched by name across the
//!   project, in their own namespace: every `value` field and every
//!   `.value` are one symbol, apart from any global `value`.
//! - [`Scope::Data`]: a name a data format declares and its strings refer
//!   to (FDF's frames and string keys: `INHERITS "EscMenuButtonTemplate"`),
//!   matched by name across the files that see each other, apart from
//!   code's names. Its `namespace` is the name of the block it's declared
//!   in (a frame's parent), for the outline; it doesn't take part in
//!   matching.

/// What a name is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Function,
    /// A function implemented by the game (`native` in JASS).
    Native,
    Global,
    Constant,
    Local,
    Parameter,
    Type,
    /// A Zinc library.
    Library,
    Struct,
    Interface,
    /// A Zinc module: members a struct can implement.
    Module,
    Method,
    /// A struct or class field, or a table field (Lua), or a virtual
    /// property (AngelScript).
    Field,
    /// An AngelScript namespace.
    Namespace,
    Class,
    Enum,
    EnumMember,
    /// An FDF frame (`Frame "TEXT" "Title"`, `Texture "Icon"`).
    Frame,
    /// A key of a data format's string table (FDF's `StringList`).
    Key,
}

impl SymbolKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Native => "native",
            SymbolKind::Global => "global",
            SymbolKind::Constant => "constant",
            SymbolKind::Local => "local",
            SymbolKind::Parameter => "parameter",
            SymbolKind::Type => "type",
            SymbolKind::Library => "library",
            SymbolKind::Struct => "struct",
            SymbolKind::Interface => "interface",
            SymbolKind::Module => "module",
            SymbolKind::Method => "method",
            SymbolKind::Field => "field",
            SymbolKind::Namespace => "namespace",
            SymbolKind::Class => "class",
            SymbolKind::Enum => "enum",
            SymbolKind::EnumMember => "enum member",
            SymbolKind::Frame => "frame",
            SymbolKind::Key => "key",
        }
    }

    /// Is this a symbol of a function body (a parameter or local)? Those
    /// aren't listed in outlines or symbol searches.
    pub fn is_local(self) -> bool {
        matches!(self, SymbolKind::Local | SymbolKind::Parameter)
    }
}

/// Where a name is resolved; see the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    Project,
    Local(u32),
    Member,
    Data,
}

/// One name in the source: a declaration or a reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    pub name: String,
    /// 1-based line; 1-based codepoint columns, end-exclusive (like
    /// [`crate::Span`]). A name never spans lines.
    pub line: usize,
    pub col: usize,
    pub end_col: usize,
    /// For a reference, what the name was classified as where it's used
    /// (a call is a `Function` even if it resolves to a `Native`).
    pub kind: SymbolKind,
    pub declaration: bool,
    pub scope: Scope,
    /// For a project-scope name: the namespace a declaration lives in, or
    /// a reference is made from, as `A::B`; empty for the global one.
    pub namespace: String,
    /// For a project-scope reference: the namespace path written before
    /// the name, `B` in `B::C`, `::B` in `::B::C` (from the global
    /// namespace), `::` in `::C`. `None` for a plain name.
    pub qualifier: Option<String>,
}

impl Occurrence {
    /// Is the 1-based codepoint column `col` of `line` on this name? Its
    /// end counts, so a cursor right after a name is on it.
    pub fn covers(&self, line: usize, col: usize) -> bool {
        self.line == line && self.col <= col && col <= self.end_col
    }

    /// Do `self` and `other` name the same symbol (within one file, for
    /// local scopes)? Only by name and scope: project-scope names must
    /// also [`resolve`](Occurrence::resolve) to the same namespace.
    pub fn same_symbol(&self, other: &Occurrence) -> bool {
        self.name == other.name && self.scope == other.scope
    }

    /// The namespaces a project-scope reference may mean, most specific
    /// first, as AngelScript looks names up: the written qualifier from
    /// the current namespace, then from each enclosing one, out to the
    /// global namespace. An absolute qualifier (`::B`) means only itself.
    pub fn lookup_chain(&self) -> Vec<String> {
        let join = |a: &str, b: &str| match (a.is_empty(), b.is_empty()) {
            (true, _) => b.to_string(),
            (_, true) => a.to_string(),
            _ => format!("{a}::{b}"),
        };
        let qualifier = self.qualifier.as_deref().unwrap_or("");
        if let Some(absolute) = qualifier.strip_prefix("::") {
            return vec![absolute.to_string()];
        }
        let parts: Vec<&str> = self.namespace.split("::").filter(|p| !p.is_empty()).collect();
        (0..=parts.len()).rev().map(|n| join(&parts[..n].join("::"), qualifier)).collect()
    }

    /// The namespace a project-scope name means: a declaration's own; for
    /// a reference, the first namespace of its [lookup
    /// chain](Occurrence::lookup_chain) that `declares` the name, else (a
    /// name declared nowhere in the project, like a native) the last one,
    /// relative to the global namespace.
    pub fn resolve(&self, declares: impl Fn(&str) -> bool) -> String {
        if self.declaration {
            return self.namespace.clone();
        }
        let chain = self.lookup_chain();
        chain.iter().find(|ns| declares(ns)).unwrap_or_else(|| chain.last().expect("never empty")).clone()
    }
}

#[cfg(test)]
#[path = "symbols.test.rs"]
mod tests;
