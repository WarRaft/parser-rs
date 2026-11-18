use super::*;
use crate::span::Span;

const LANG: HtmlLang = HtmlLang {
    is_keyword: |w| matches!(w, "local" | "set" | "if"),
    block_comments: true,
};

const NO_BLOCKS: HtmlLang = HtmlLang {
    is_keyword: LANG.is_keyword,
    block_comments: false,
};

fn line(src: &str, lang: &HtmlLang) -> String {
    highlight_line(src, lang, &mut false)
}

#[test]
fn embeds_title_source_and_tree_data() {
    let tree = Node::with_children("Program", vec![Node::leaf("x")]).tag("root", None);
    let html = render_html(&tree, "set x = 1", "demo<1>.j", &LANG);
    assert!(html.contains("demo&lt;1&gt;.j"));
    assert!(html.contains("\"label\":\"Program\""));
    assert!(html.contains("id=\"ast-data\""));
    assert!(html.contains("id=\"L1\""));
    assert!(html.contains("tok-kw"));
}

#[test]
fn escapes_closing_script_tags_in_labels() {
    let tree = Node::leaf("String \"</script>\"");
    let html = render_html(&tree, "", "t", &LANG);
    assert!(html.contains("<\\/script>"));
}

#[test]
fn serializes_spans_and_whole_line_sentinels() {
    let exact = Node::leaf("Var a").tag("expr", Some(Span::new(2, 12, 2, 13)));
    assert!(node_to_json(&exact)
        .contains("\"kind\":\"expr\",\"line\":2,\"eline\":2,\"scol\":12,\"ecol\":13"));
    let whole = Node::leaf("Stmt").tag("stmt", Some(Span::whole_line(3)));
    assert!(node_to_json(&whole).contains("\"line\":3,\"eline\":3,\"scol\":null,\"ecol\":null"));
}

#[test]
fn highlights_keywords_strings_numbers_and_comments() {
    let l = line("local integer x = 1 // hi \"there\"", &LANG);
    assert!(l.contains(r#"<span class="tok tok-kw" data-scol="1" data-ecol="6">local</span>"#));
    assert!(l.contains(r#"<span class="tok tok-ident" data-scol="7" data-ecol="14">integer</span>"#));
    assert!(l.contains(r#"<span class="tok tok-num" data-scol="19" data-ecol="20">1</span>"#));
    assert!(l.contains(
        r#"<span class="tok tok-com" data-scol="21" data-ecol="34">// hi "there"</span>"#
    ));
}

#[test]
fn block_comments_span_lines_only_when_enabled() {
    let mut open = false;
    let first = highlight_line("x /* a", &LANG, &mut open);
    assert!(open);
    assert!(first.contains(r#"data-scol="3" data-ecol="7">/* a</span>"#));
    let second = highlight_line("b */ y", &LANG, &mut open);
    assert!(!open);
    assert!(second.contains(r#"<span class="tok tok-com" data-scol="1" data-ecol="5">b */</span>"#));
    assert!(second.contains(r#"<span class="tok tok-ident" data-scol="6" data-ecol="7">y</span>"#));

    let plain = line("/* a */", &NO_BLOCKS);
    assert!(plain.contains(r#"<span class="tok tok-punct" data-scol="1" data-ecol="2">/</span>"#));
}

#[test]
fn columns_count_codepoints_not_bytes() {
    let l = line("\"ю😀\" x", &LANG);
    assert!(l.contains(r#"<span class="tok tok-ident" data-scol="6" data-ecol="7">x</span>"#));
}
