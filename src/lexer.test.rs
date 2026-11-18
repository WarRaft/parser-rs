use super::*;

crate::keywords! {
    enum Kw {
        Local = "local",
    }
}

const SPEC: LexSpec = LexSpec {
    line_comment: "//",
    block_comment: Some(("/*", "*/")),
    rawcodes: true,
    puncts: &[
        Punct::Lt,
        Punct::Shl,
        Punct::ShlAssign,
        Punct::Assign,
        Punct::Minus,
        Punct::Arrow,
    ],
};

fn kinds(src: &str) -> Vec<TokenKind<Kw>> {
    tokenize::<Kw>(src, &SPEC)
        .unwrap()
        .into_iter()
        .map(|t| t.kind)
        .collect()
}

#[test]
fn lexes_keywords_identifiers_and_literals() {
    assert_eq!(
        kinds("local x_1 42 1.5 0x10 \"s\" 'hfoo'"),
        vec![
            TokenKind::Keyword(Kw::Local),
            TokenKind::Ident("x_1".into()),
            TokenKind::Int(42),
            TokenKind::Real(1.5),
            TokenKind::Int(16),
            TokenKind::Str("s".into()),
            TokenKind::Int(0x68666f6f),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn matches_punctuation_longest_first() {
    assert_eq!(
        kinds("<<= << < -> - ="),
        vec![
            TokenKind::Punct(Punct::ShlAssign),
            TokenKind::Punct(Punct::Shl),
            TokenKind::Punct(Punct::Lt),
            TokenKind::Punct(Punct::Arrow),
            TokenKind::Punct(Punct::Minus),
            TokenKind::Punct(Punct::Assign),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn rejects_puncts_outside_the_spec() {
    // `=` is allowed, `==` isn't listed — so it lexes as two `=`s...
    let toks = kinds("a == b");
    assert_eq!(toks[1], TokenKind::Punct(Punct::Assign));
    assert_eq!(toks[2], TokenKind::Punct(Punct::Assign));
    // ...while a character that starts no listed punct is an error.
    let err = tokenize::<Kw>("a + b", &SPEC).unwrap_err();
    assert_eq!(err.message, "unexpected character '+'");
    assert_eq!((err.line, err.col), (1, 3));
}

#[test]
fn rawcodes_can_be_disabled() {
    let spec = LexSpec {
        rawcodes: false,
        ..SPEC
    };
    let err = tokenize::<Kw>("'abcd'", &spec).unwrap_err();
    assert!(err.message.contains("unexpected character '''"));
}

#[test]
fn records_token_columns_and_skips_comments() {
    let toks = tokenize::<Kw>("/* c */ local // x\n  y", &SPEC).unwrap();
    assert_eq!((toks[0].line, toks[0].col, toks[0].end_col), (1, 9, 14));
    assert_eq!((toks[1].line, toks[1].col, toks[1].end_col), (2, 3, 4));
}
