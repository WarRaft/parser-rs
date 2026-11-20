use super::*;
use crate::kind::{Kw, KwSet, Punct};
use crate::lexer::EmbedSpec;
use crate::span::Span;

static INNER: LexSpec = LexSpec {
    keywords: KwSet::of(&[Kw::Library]),
    block_comment: Some(("/*", "*/")),
    puncts: &[Punct::LBrace, Punct::RBrace],
    ..LexSpec::BASE
};

static HOST: LexSpec = LexSpec {
    keywords: KwSet::of(&[Kw::Local, Kw::Set, Kw::If]),
    rawcodes: true,
    puncts: &[Punct::Assign, Punct::LtEq, Punct::Lt],
    embeds: &[EmbedSpec {
        lang: "inner",
        open: "//! inner",
        close: "//! endinner",
    }],
    ..LexSpec::BASE
};

static LANG: HtmlLang = HtmlLang {
    spec: &HOST,
    embedded: &[("inner", &INNER)],
};

fn source_html(src: &str) -> String {
    highlight_source(src, &LANG)
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
fn highlights_with_the_languages_own_lexer() {
    let h = source_html("local x = 'hfoo' <= \"s\" // hi \"there\"");
    assert!(h.contains(r#"<span class="tok tok-kw" data-scol="1" data-ecol="6">local</span> "#));
    assert!(h.contains(r#"<span class="tok tok-ident" data-scol="7" data-ecol="8">x</span>"#));
    // a rawcode is a number; `<=` is one operator, not two characters
    assert!(h.contains(r#"<span class="tok tok-num" data-scol="11" data-ecol="17">'hfoo'</span>"#));
    assert!(h.contains(r#"<span class="tok tok-punct" data-scol="18" data-ecol="20">&lt;=</span>"#));
    assert!(h.contains(r#"<span class="tok tok-str" data-scol="21" data-ecol="24">"s"</span>"#));
    assert!(h.contains(
        r#"<span class="tok tok-com" data-scol="25" data-ecol="38">// hi "there"</span>"#
    ));
}

#[test]
fn highlights_embedded_blocks_with_their_own_lexer() {
    let h = source_html("set\n//! inner\nlibrary { /* a\nb */ }\n//! endinner\n");
    assert!(h.contains(r#"<span class="tok tok-kw" data-scol="1" data-ecol="10">//! inner</span>"#));
    // `library` is only a keyword of the inner language
    assert!(h.contains(r#"<span class="tok tok-kw" data-scol="1" data-ecol="8">library</span>"#));
    // a block comment spanning two lines is marked on both
    assert!(h.contains(r#"<span class="tok tok-com" data-scol="11" data-ecol="15">/* a</span>"#));
    assert!(h.contains(r#"<span class="tok tok-com" data-scol="1" data-ecol="5">b */</span>"#));
}

#[test]
fn unlexable_characters_are_kept_as_plain_text() {
    let h = source_html("x # y");
    assert!(h.contains(r#"x</span> # <span"#));
}

#[test]
fn columns_count_codepoints_not_bytes() {
    let h = source_html("\"ю😀\" x");
    assert!(h.contains(r#"<span class="tok tok-ident" data-scol="6" data-ecol="7">x</span>"#));
}

#[test]
fn colors_source_from_semantic_tokens() {
    use crate::semantic::{Modifiers, SemanticToken, SemanticType};
    let tokens = [
        SemanticToken::new(1, 2, 5, SemanticType::Namespace, Modifiers::NONE),
        SemanticToken::new(2, 1, 4, SemanticType::Property, Modifiers::NONE),
        SemanticToken::new(2, 5, 7, SemanticType::Number, Modifiers::NONE),
    ];
    let html = render_html_tokens(&Node::leaf("Doc"), "[Sec]\nKey=42", "t", &tokens);
    assert!(html.contains("<span class=\"tok tok-kw\" data-scol=\"2\" data-ecol=\"5\">Sec</span>]"));
    assert!(html.contains("<span class=\"tok tok-ident\" data-scol=\"1\" data-ecol=\"4\">Key</span>="));
    assert!(html.contains("<span class=\"tok tok-num\" data-scol=\"5\" data-ecol=\"7\">42</span>"));
}
