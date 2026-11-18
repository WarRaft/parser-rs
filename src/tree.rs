//! A generic labelled tree used as an intermediate representation between
//! a language's AST and its rendered forms (indented text, Graphviz DOT,
//! the HTML viewer). Each language only maps its AST onto [`Node`]s; the
//! renderers are shared.

use crate::span::Span;

/// `kind` is a coarse category used for coloring in the HTML viewer (see
/// the `.kind-*` classes in its template: `root`, `container`, `decl`,
/// `sig`, `stmt`, `expr`, `lit`, `node`); `span` (when known) lets that
/// viewer highlight the matching source range on click.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub label: String,
    pub kind: &'static str,
    pub span: Option<Span>,
    pub children: Vec<Node>,
}

impl Node {
    pub fn leaf(label: impl Into<String>) -> Self {
        Node::with_children(label, Vec::new())
    }

    pub fn with_children(label: impl Into<String>, children: Vec<Node>) -> Self {
        Node {
            label: label.into(),
            kind: "node",
            span: None,
            children,
        }
    }

    pub fn tag(mut self, kind: &'static str, span: Option<Span>) -> Self {
        self.kind = kind;
        self.span = span;
        self
    }
}

/// Renders a tree with box-drawing connectors, e.g.:
///
/// ```text
/// Program
/// └─ Function Add(...)
///    ├─ Local integer sum
///    └─ Return
/// ```
pub fn render_tree(root: &Node) -> String {
    let mut out = String::new();
    out.push_str(&root.label);
    out.push('\n');
    render_children(&root.children, "", &mut out);
    out
}

fn render_children(children: &[Node], prefix: &str, out: &mut String) {
    for (i, child) in children.iter().enumerate() {
        let is_last = i == children.len() - 1;
        let connector = if is_last { "└─ " } else { "├─ " };
        out.push_str(prefix);
        out.push_str(connector);
        out.push_str(&child.label);
        out.push('\n');
        let child_prefix = format!("{prefix}{}", if is_last { "   " } else { "│  " });
        render_children(&child.children, &child_prefix, out);
    }
}

/// Renders a tree as a Graphviz DOT graph. Feed the output to `dot -Tpng`
/// (or an online viewer) to get an actual picture of it.
pub fn render_dot(root: &Node) -> String {
    let mut out = String::new();
    out.push_str("digraph AST {\n");
    out.push_str("    node [shape=box, fontname=\"monospace\"];\n");
    let mut counter = 0usize;
    write_dot_node(root, &mut counter, None, &mut out);
    out.push_str("}\n");
    out
}

fn write_dot_node(node: &Node, counter: &mut usize, parent: Option<usize>, out: &mut String) {
    let id = *counter;
    *counter += 1;
    let escaped = node.label.replace('\\', "\\\\").replace('"', "\\\"");
    out.push_str(&format!("    n{id} [label=\"{escaped}\"];\n"));
    if let Some(p) = parent {
        out.push_str(&format!("    n{p} -> n{id};\n"));
    }
    for child in &node.children {
        write_dot_node(child, counter, Some(id), out);
    }
}

#[cfg(test)]
#[path = "tree.test.rs"]
mod tests;
