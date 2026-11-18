//! The self-contained HTML AST viewer: a source panel with syntax
//! highlighting on the left, a collapsible, color-coded [`Node`] tree on
//! the right. Clicking (or selecting) a token in the source highlights the
//! smallest tree node covering it, and vice versa — both directions are
//! wired through exact (line, column) spans, not just whole-line matching.
//! Everything (CSS, JS, data) is inlined, so the file works offline from
//! `file://` — no server, no editor library required.

use crate::tree::Node;

/// The (small) language-specific part of the viewer: how to tell keywords
/// apart from identifiers, and which comment forms exist. Strings
/// (`"..."`), rawcodes (`'...'`), numbers and `//` line comments are
/// highlighted for every language.
#[derive(Debug, Clone, Copy)]
pub struct HtmlLang {
    pub is_keyword: fn(&str) -> bool,
    /// Whether `/* ... */` block comments (possibly multi-line) exist.
    pub block_comments: bool,
}

/// Renders the viewer page for `tree` (the AST already mapped onto
/// [`Node`]s), built from `source`, titled `title`.
pub fn render_html(tree: &Node, source: &str, title: &str, lang: &HtmlLang) -> String {
    let tree_json = node_to_json(tree);
    let source_html = highlight_source(source, lang);

    TEMPLATE
        .replace("__TITLE__", &html_escape(title))
        .replace("__SOURCE_HTML__", &source_html)
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

fn highlight_source(src: &str, lang: &HtmlLang) -> String {
    let mut out = String::new();
    let mut in_block_comment = false;
    for (i, line) in src.lines().enumerate() {
        let n = i + 1;
        out.push_str(&format!("<div class=\"line\" id=\"L{n}\">"));
        out.push_str(&format!("<span class=\"ln\">{n}</span>"));
        out.push_str("<span class=\"code\">");
        out.push_str(&highlight_line(line, lang, &mut in_block_comment));
        out.push_str("</span></div>\n");
    }
    out
}

/// Tokenizes a single source line for display, wrapping every non-space
/// lexeme (identifiers, keywords, literals, comments, and individual
/// punctuation/operator characters) in a `<span class="tok ...">` tagged
/// with `data-scol`/`data-ecol`. Column bookkeeping deliberately mirrors
/// [`crate::Cursor`]'s (1-indexed, one column per character) so that these
/// spans line up exactly with the spans the parsers record — that
/// alignment is what lets the viewer's JS map a clicked source token
/// straight to the tree node whose span contains it, and back again.
///
/// `in_block_comment` carries an open `/* ...` across lines.
fn highlight_line(line: &str, lang: &HtmlLang, in_block_comment: &mut bool) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0usize;
    let mut out = String::new();

    if *in_block_comment {
        i = block_comment_end(&chars, 0, in_block_comment);
        if i > 0 {
            emit_tok(&mut out, "tok-com", 1, i + 1, &chars[..i].iter().collect::<String>());
        }
    }

    while i < chars.len() {
        let c = chars[i];
        let col = i + 1;

        if c == '/' && chars.get(i + 1) == Some(&'/') {
            let text: String = chars[i..].iter().collect();
            emit_tok(&mut out, "tok-com", col, chars.len() + 1, &text);
            break;
        }

        if lang.block_comments && c == '/' && chars.get(i + 1) == Some(&'*') {
            *in_block_comment = true;
            let end = block_comment_end(&chars, i + 2, in_block_comment);
            let text: String = chars[i..end].iter().collect();
            emit_tok(&mut out, "tok-com", col, end + 1, &text);
            i = end;
            continue;
        }

        if c == '"' || c == '\'' {
            let start = i;
            i += 1;
            while i < chars.len() && chars[i] != c {
                if c == '"' && chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1;
                }
                i += 1;
            }
            if i < chars.len() {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            emit_tok(&mut out, "tok-str", col, i + 1, &text);
            continue;
        }

        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '.') {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            emit_tok(&mut out, "tok-num", col, i + 1, &text);
            continue;
        }

        if c == '_' || c.is_alphabetic() {
            let start = i;
            while i < chars.len() && (chars[i] == '_' || chars[i].is_alphanumeric()) {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            let class = if (lang.is_keyword)(&text) {
                "tok-kw"
            } else {
                "tok-ident"
            };
            emit_tok(&mut out, class, col, i + 1, &text);
            continue;
        }

        if c.is_whitespace() {
            out.push(c);
            i += 1;
            continue;
        }

        emit_tok(&mut out, "tok-punct", col, col + 1, &c.to_string());
        i += 1;
    }

    out
}

/// Scans from `from` for a closing `*/`; returns the index just past it
/// (clearing `in_block_comment`), or the line's end if it isn't there.
fn block_comment_end(chars: &[char], from: usize, in_block_comment: &mut bool) -> usize {
    let mut j = from;
    while j + 1 < chars.len() {
        if chars[j] == '*' && chars[j + 1] == '/' {
            *in_block_comment = false;
            return j + 2;
        }
        j += 1;
    }
    chars.len()
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
