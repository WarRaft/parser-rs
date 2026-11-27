use super::*;
use crate::span::Span;

use crate::kind::{Embedded, Kw, KwSet};

const SPEC: LexSpec = LexSpec {
    keywords: KwSet::of(&[Kw::Local]),
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
    ..LexSpec::BASE
};

fn kinds(src: &str) -> Vec<TokenKind> {
    tokenize(src, &SPEC)
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
    let err = tokenize("a + b", &SPEC).unwrap_err();
    assert_eq!(err.message, "unexpected character '+'");
    assert_eq!((err.line, err.col), (1, 3));
}

#[test]
fn rawcodes_can_be_disabled() {
    let spec = LexSpec {
        rawcodes: false,
        ..SPEC
    };
    let err = tokenize("'abcd'", &spec).unwrap_err();
    assert!(err.message.contains("unexpected character '''"));
}

#[test]
fn records_token_columns_and_skips_comments() {
    let toks = tokenize("/* c */ local // x\n  y", &SPEC).unwrap();
    assert_eq!((toks[0].line, toks[0].col, toks[0].end_col), (1, 9, 14));
    assert_eq!((toks[1].line, toks[1].col, toks[1].end_col), (2, 3, 4));
}

#[test]
fn lossy_lexing_recovers_and_collects_comments() {
    let lexed = tokenize_lossy("a + b // c\n\"open\nlocal /* x */", &SPEC);
    let kinds: Vec<_> = lexed.tokens.iter().map(|t| t.kind.clone()).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Ident("b".into()),
            TokenKind::Keyword(Kw::Local),
            TokenKind::Eof,
        ]
    );
    assert_eq!(lexed.errors.len(), 2, "{:?}", lexed.errors); // `+` (not in SPEC), the unterminated string
    assert_eq!(lexed.comments, vec![Span::new(1, 7, 1, 11), Span::new(3, 7, 3, 14)]);
}

const HOST: LexSpec = LexSpec {
    embeds: &[EmbedSpec {
        lang: "zinc",
        open: "//! zinc",
        close: "//! endzinc",
        prefix: false,
    }],
    ..SPEC
};

fn embedded(tok: &Token<TokenKind>) -> &Embedded {
    match &tok.kind {
        TokenKind::Embedded(e) => e,
        other => panic!("expected an embedded block, got {other:?}"),
    }
}

#[test]
fn lexes_an_embedded_block_as_one_token_and_resumes_after_it() {
    let src = "local\n  //!zinc\nfoo { + }\n\n  //! endzinc  \nlocal";
    let toks = tokenize(src, &HOST).unwrap();
    assert_eq!(toks.len(), 4); // local, block, local, eof
    assert_eq!(toks[0].kind, TokenKind::Keyword(Kw::Local));
    let e = embedded(&toks[1]);
    assert_eq!(e.lang, "zinc");
    assert_eq!(e.source, "foo { + }\n\n");
    assert_eq!(e.first_line, 3);
    assert_eq!(e.open, Span::new(2, 3, 2, 10));
    assert_eq!(e.close, Some(Span::new(5, 3, 5, 14)));
    assert_eq!((toks[1].line, toks[1].col), (2, 3));
    assert_eq!((toks[2].kind.clone(), toks[2].line), (TokenKind::Keyword(Kw::Local), 6));
}

#[test]
fn directives_only_count_alone_on_their_line() {
    // after code on the same line, and without an embed spec, it's a comment
    let toks = tokenize("local //! zinc\n", &HOST).unwrap();
    assert_eq!(toks.len(), 2);
    let toks = tokenize("//! zinc\n//! endzinc", &SPEC).unwrap();
    assert_eq!(toks.len(), 1);
}

#[test]
fn unterminated_embedded_blocks_run_to_eof() {
    let err = tokenize("//! zinc\nfoo", &HOST).unwrap_err();
    assert_eq!(err.message, "unterminated '//! zinc' block: missing '//! endzinc'");
    assert_eq!((err.line, err.col, err.end_col), (1, 1, 9));

    let lexed = tokenize_lossy("//! zinc\nfoo", &HOST);
    let e = embedded(&lexed.tokens[0]);
    assert_eq!((e.source.as_str(), e.close), ("foo\n", None));
}

#[test]
fn tokenize_at_keeps_file_line_numbers() {
    let toks = tokenize_at("\nlocal", &SPEC, 10).unwrap();
    assert_eq!((toks[0].line, toks[0].col), (11, 1));
}

const LUA_LIKE: LexSpec = LexSpec {
    keywords: KwSet::of(&[Kw::Local, Kw::Nil]),
    line_comment: "--",
    puncts: &[Punct::Minus, Punct::DotDot, Punct::LBracket, Punct::RBracket, Punct::Dot],
    numbers: crate::literal::NumberSpec {
        exponent: true,
        leading_dot: true,
        radix_prefixes: false,
        float_suffix: false,
        trailing_dot: false,
        leading_zero_octal: false,
        dollar_hex: false,
    },
    single_quote_strings: true,
    long_brackets: true,
    ..LexSpec::BASE
};

#[test]
fn lua_like_lexical_forms() {
    let src = "--[==[ long\ncomment ]==] local s = 'a' .. [[x\ny]] -- tail\n.5 1e2 t[ [=[z]=] ]";
    let lexed = tokenize_lossy(src, &LexSpec { puncts: &[Punct::Assign, Punct::DotDot, Punct::LBracket, Punct::RBracket], ..LUA_LIKE });
    assert!(lexed.errors.is_empty(), "{:?}", lexed.errors);
    let kinds: Vec<_> = lexed.tokens.iter().map(|t| t.kind.clone()).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Keyword(Kw::Local),
            TokenKind::Ident("s".into()),
            TokenKind::Punct(Punct::Assign),
            TokenKind::Str("a".into()),
            TokenKind::Punct(Punct::DotDot),
            TokenKind::Str("x\ny".into()),
            TokenKind::Real(0.5),
            TokenKind::Real(100.0),
            TokenKind::Ident("t".into()),
            TokenKind::Punct(Punct::LBracket),
            TokenKind::Str("z".into()),
            TokenKind::Punct(Punct::RBracket),
            TokenKind::Eof,
        ]
    );
    // the long string token spans two lines
    let long = &lexed.tokens[5];
    assert_eq!((long.line, long.col, long.end_line, long.end_col), (2, 31, 3, 4));
    assert_eq!(lexed.comments[0], Span::new(1, 1, 2, 13));
    assert_eq!(lexed.comments[1], Span::new(3, 5, 3, 12));
}

#[test]
fn heredocs_and_unterminated_long_comments() {
    let spec = LexSpec { heredocs: true, ..LexSpec::BASE };
    let toks = tokenize("\"\"\"a\n\"b\" \"\"\"", &spec).unwrap();
    assert_eq!(toks[0].kind, TokenKind::Str("a\n\"b\" ".into()));
    assert_eq!((toks[0].end_line, toks[0].end_col), (2, 8));

    let lexed = tokenize_lossy("--[[ never", &LUA_LIKE);
    assert_eq!(lexed.errors[0].message, "unterminated long comment");
    assert_eq!(lexed.comments, vec![Span::new(1, 1, 1, 11)]);
}

#[test]
fn raw_strings_keep_backslashes() {
    let spec = LexSpec { raw_strings: true, ..LexSpec::BASE };
    let toks = tokenize("\"UI\\FrameDef\\new\"", &spec).unwrap();
    assert_eq!(toks[0].kind, TokenKind::Str("UI\\FrameDef\\new".into()));
    assert_eq!(tokenize_lossy("\"open\n", &spec).errors[0].message, "unterminated string literal");
}
