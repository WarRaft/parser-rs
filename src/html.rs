//! The self-contained HTML AST viewer: a source panel with syntax
//! highlighting on the left, a collapsible, color-coded [`Node`] tree on
//! the right. Clicking (or selecting) a token in the source highlights the
//! smallest tree node covering it, and vice versa — both directions are
//! wired through exact (line, column) spans, not just whole-line matching.
//! Everything (CSS, JS, data) is inlined, so the file works offline from
//! `file://` — no server, no editor library required.

use crate::kind::TokenKind;
use crate::lexer::{tokenize_lossy_at, LexSpec};
use crate::semantic::{SemanticToken, SemanticType};
use crate::span::Span;
use crate::tree::Node;

/// The language-specific part of the viewer: its lexer spec. Source
/// highlighting runs the language's real lexer (tolerant mode), so it
/// always agrees with the parser about what a keyword, string or comment
/// is. Embedded blocks (`//! zinc` in vJASS) are highlighted with their
/// own language's spec from `embedded`, by [`EmbedSpec::lang`](crate::lexer::EmbedSpec::lang).
#[derive(Debug, Clone, Copy)]
pub struct HtmlLang {
    pub spec: &'static LexSpec,
    pub embedded: &'static [(&'static str, &'static LexSpec)],
}

/// Renders the viewer page for `tree` (the AST already mapped onto
/// [`Node`]s), built from `source`, titled `title`.
pub fn render_html(tree: &Node, source: &str, title: &str, lang: &HtmlLang) -> String {
    page(tree, &highlight_source(source, lang), title)
}

/// [`render_html`] for a language that doesn't use the shared token lexer
/// (a line-oriented data format such as BNI): its source is colored from
/// the semantic tokens its own scanner produces, the same ones the editor
/// gets, so the viewer and the editor still agree.
pub fn render_html_tokens(tree: &Node, source: &str, title: &str, tokens: &[SemanticToken]) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut marks: Vec<Vec<Mark>> = vec![Vec::new(); lines.len()];
    for t in tokens {
        let span = Span::new(t.line, t.start_col, t.end_line, t.end_col);
        add_mark(&mut marks, &lines, span, token_class(t.ty));
    }
    page(tree, &render_lines(&lines, &mut marks), title)
}

/// The viewer's CSS class for a semantic token type.
fn token_class(ty: SemanticType) -> &'static str {
    use SemanticType as T;
    match ty {
        T::Comment => "tok-com",
        T::String | T::Regexp => "tok-str",
        T::Number => "tok-num",
        T::Keyword | T::Macro | T::Namespace => "tok-kw",
        T::Operator => "tok-punct",
        _ => "tok-ident",
    }
}

fn page(tree: &Node, source_html: &str, title: &str) -> String {
    let tree_json = node_to_json(tree);
    TEMPLATE
        .replace("__TITLE__", &html_escape(title))
        .replace("__SOURCE_HTML__", source_html)
        .replace("__TREE_JSON__", &tree_json)
}

fn node_to_json(node: &Node) -> String {
    let mut s = String::new();
    write_node_json(node, &mut s);
    s
}

fn write_node_json(node: &Node, out: &mut String) {
    out.push('{');
    out.push_str("\"label\":\"");
    out.push_str(&json_escape(&node.label));
    out.push_str("\",\"kind\":\"");
    out.push_str(node.kind);
    out.push('"');
    match node.span {
        Some(span) => {
            out.push_str(&format!(",\"line\":{}", span.start_line));
            out.push_str(&format!(",\"eline\":{}", span.end_line));
            if span.start_col == 0 {
                // A "whole line" sentinel (see `Span::whole_line`).
                out.push_str(",\"scol\":null,\"ecol\":null");
            } else {
                out.push_str(&format!(",\"scol\":{}", span.start_col));
                out.push_str(&format!(",\"ecol\":{}", span.end_col));
            }
        }
        None => out.push_str(",\"line\":null,\"eline\":null,\"scol\":null,\"ecol\":null"),
    }
    out.push_str(",\"children\":[");
    for (i, child) in node.children.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_node_json(child, out);
    }
    out.push_str("]}");
}

/// JSON-escapes a string, additionally escaping `/` as `\/` so that a
/// string literal containing `</script>` can't prematurely close the
/// `<script>` tag this JSON is embedded in.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '/' => out.push_str("\\/"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// One highlighted piece of a line: `[scol, ecol)` with a CSS class.
type Mark = (usize, usize, &'static str);

fn highlight_source(src: &str, lang: &HtmlLang) -> String {
    let lines: Vec<&str> = src.lines().collect();
    let mut marks: Vec<Vec<Mark>> = vec![Vec::new(); lines.len()];
    collect_marks(src, lang.spec, 1, lang, &lines, &mut marks);
    render_lines(&lines, &mut marks)
}

fn render_lines(lines: &[&str], marks: &mut [Vec<Mark>]) -> String {
    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        let n = i + 1;
        out.push_str(&format!("<div class=\"line\" id=\"L{n}\">"));
        out.push_str(&format!("<span class=\"ln\">{n}</span>"));
        out.push_str("<span class=\"code\">");
        out.push_str(&render_line(line, &mut marks[i]));
        out.push_str("</span></div>\n");
    }
    out
}

/// Lexes `src` (a whole file, or an embedded block starting on
/// `first_line`) and records a mark per token and comment, split per line.
fn collect_marks(
    src: &str,
    spec: &LexSpec,
    first_line: usize,
    lang: &HtmlLang,
    lines: &[&str],
    marks: &mut [Vec<Mark>],
) {
    let lexed = tokenize_lossy_at(src, spec, first_line);
    for span in &lexed.comments {
        add_mark(marks, lines, *span, "tok-com");
    }
    for tok in &lexed.tokens {
        let class = match &tok.kind {
            TokenKind::Keyword(_) => "tok-kw",
            TokenKind::Ident(_) => "tok-ident",
            TokenKind::Int(_) | TokenKind::Real(_) => "tok-num",
            TokenKind::Str(_) => "tok-str",
            TokenKind::Punct(_) => "tok-punct",
            TokenKind::Embedded(block) => {
                for directive in std::iter::once(block.open).chain(block.close) {
                    add_mark(marks, lines, directive, "tok-kw");
                }
                if let Some((_, spec)) = lang.embedded.iter().find(|(l, _)| *l == block.lang) {
                    collect_marks(&block.source, spec, block.first_line, lang, lines, marks);
                }
                continue;
            }
            TokenKind::Eof => continue,
        };
        add_mark(marks, lines, tok.span(), class);
    }
}

fn add_mark(marks: &mut [Vec<Mark>], lines: &[&str], span: Span, class: &'static str) {
    for line in span.start_line..=span.end_line {
        let Some(text) = lines.get(line.wrapping_sub(1)) else {
            continue;
        };
        let start = if line == span.start_line { span.start_col } else { 1 };
        let end = if line == span.end_line {
            span.end_col
        } else {
            text.chars().count() + 1
        };
        if end > start {
            marks[line - 1].push((start, end, class));
        }
    }
}

/// Renders one source line: every mark becomes a `<span class="tok ...">`
/// tagged with `data-scol`/`data-ecol`, in the same 1-based codepoint
/// columns the parsers record. That alignment is what lets the viewer's
/// JS map a clicked source token straight to the tree node whose span
/// contains it, and back again. Unmarked text (whitespace, characters the
/// lexer rejected) is emitted as-is.
fn render_line(line: &str, marks: &mut [Mark]) -> String {
    marks.sort_by_key(|m| m.0);
    let chars: Vec<char> = line.chars().collect();
    let text = |from: usize, to: usize| -> String {
        chars[(from - 1).min(chars.len())..(to - 1).min(chars.len())]
            .iter()
            .collect()
    };
    let mut out = String::new();
    let mut col = 1;
    for &(start, end, class) in marks.iter() {
        if start < col {
            continue; // overlaps the previous mark
        }
        out.push_str(&html_escape(&text(col, start)));
        emit_tok(&mut out, class, start, end, &text(start, end));
        col = end;
    }
    out.push_str(&html_escape(&text(col, chars.len() + 1)));
    out
}

fn emit_tok(out: &mut String, class: &str, start_col: usize, end_col: usize, text: &str) {
    out.push_str(&format!(
        "<span class=\"tok {class}\" data-scol=\"{start_col}\" data-ecol=\"{end_col}\">"
    ));
    out.push_str(&html_escape(text));
    out.push_str("</span>");
}

const TEMPLATE: &str = include_str!("html_template.html");

#[cfg(test)]
#[path = "html.test.rs"]
mod tests;
