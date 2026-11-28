use super::*;
use crate::diagnostic::apply_edits;
use crate::kind::KwSet;

const SPEC: LexSpec = LexSpec {
    keywords: KwSet::of(&[Kw::Case, Kw::Default]),
    block_comment: Some(("/*", "*/")),
    heredocs: true,
    puncts: &[
        Punct::LBrace,
        Punct::RBrace,
        Punct::LParen,
        Punct::RParen,
        Punct::LBracket,
        Punct::RBracket,
        Punct::Semicolon,
        Punct::Comma,
        Punct::Colon,
        Punct::Assign,
        Punct::Plus,
    ],
    ..LexSpec::BASE
};

fn format(src: &str, style: BraceStyle) -> String {
    let indents = braces(src, &SPEC, 1, 0, style);
    let out = apply_edits(src, &edits(src, &indents, "    ", 1..=usize::MAX));
    assert_eq!(line_count(&out), line_count(src), "a formatter keeps every line");
    out
}

#[test]
fn indents_blocks_and_dedents_their_closing_brace() {
    let src = "f() {\nx = 1;\n  if (x) {\ny();\n      }\n}\n";
    assert_eq!(format(src, BraceStyle::default()), "f() {\n    x = 1;\n    if (x) {\n        y();\n    }\n}\n");
}

#[test]
fn a_brace_on_its_own_line_stands_with_its_statement() {
    let src = "void f()\n  {\n x();\n   }\n";
    assert_eq!(format(src, BraceStyle::default()), "void f()\n{\n    x();\n}\n");
}

#[test]
fn brackets_opened_on_one_line_count_once() {
    let src = "f(g(\na,\nb));\nc();\n";
    assert_eq!(format(src, BraceStyle::default()), "f(g(\n    a,\n    b));\nc();\n");
}

#[test]
fn a_statement_going_on_is_one_deeper() {
    let src = "if (x)\ny();\nz = 1 +\n2 +\n3;\nw();\n";
    assert_eq!(format(src, BraceStyle::default()), "if (x)\n    y();\nz = 1 +\n    2 +\n    3;\nw();\n");
}

#[test]
fn list_items_are_not_statements_going_on() {
    let src = "enum E {\nA,\nB\n}\n";
    assert_eq!(format(src, BraceStyle::default()), "enum E {\n    A,\n    B\n}\n");
}

#[test]
fn case_labels_stand_at_their_switch_and_their_code_deeper() {
    let src = "switch (x) {\n  case 1:\na();\nbreak;\n  default:\n{\nb();\n}\n}\nc();\n";
    assert_eq!(
        format(src, BraceStyle { case_labels: true }),
        "switch (x) {\ncase 1:\n    a();\n    break;\ndefault:\n{\n    b();\n}\n}\nc();\n"
    );
}

#[test]
fn a_block_comment_moves_with_its_first_line() {
    let src = "f() {\n/* a\n     b\n   */\n}\n";
    assert_eq!(format(src, BraceStyle::default()), "f() {\n    /* a\n         b\n       */\n}\n");
}

#[test]
fn a_string_going_on_stays() {
    let src = "f() {\nx = \"\"\"a\n  b\"\"\";\n}\n";
    assert_eq!(format(src, BraceStyle::default()), "f() {\n    x = \"\"\"a\n  b\"\"\";\n}\n");
}

#[test]
fn blank_lines_are_emptied_and_crlf_kept() {
    let src = "f() {\r\n   \r\nx();\r\n}\r\n";
    assert_eq!(format(src, BraceStyle::default()), "f() {\r\n\r\n    x();\r\n}\r\n");
}

#[test]
fn only_changed_lines_in_the_range_get_edits() {
    let src = "f() {\nx();\n    y();\nz();\n}\n";
    let indents = braces(src, &SPEC, 1, 0, BraceStyle::default());
    let e = edits(src, &indents, "\t", 3..=4);
    assert_eq!(apply_edits(src, &e), "f() {\nx();\n\ty();\n\tz();\n}\n");
    assert_eq!(e.len(), 2);
}

#[test]
fn a_closing_brace_too_many_stays_at_the_top() {
    let src = "}\nx();\n";
    assert_eq!(format(src, BraceStyle::default()), "}\nx();\n");
}

#[test]
fn a_byte_order_mark_stays_first() {
    let src = "\u{feff}  x();\n";
    assert_eq!(format(src, BraceStyle::default()), "\u{feff}x();\n");
}
