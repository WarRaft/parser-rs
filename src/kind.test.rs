use super::*;
use std::collections::HashSet;

crate::keywords! {
    enum Kw {
        If = "if",
        Then = "then",
    }
}

#[test]
fn keywords_macro_round_trips() {
    assert_eq!(Kw::from_word("then"), Some(Kw::Then));
    assert_eq!(Kw::from_word("Then"), None);
    assert_eq!(Kw::If.as_str(), "if");
    assert_eq!(Kw::If.to_string(), "if");
}

#[test]
fn punct_spellings_are_unique() {
    let mut seen = HashSet::new();
    for p in Punct::ALL {
        assert!(seen.insert(p.text()), "duplicate spelling {:?}", p.text());
    }
}

#[test]
fn token_kinds_display_for_error_messages() {
    let k = |t: TokenKind<Kw>| t.to_string();
    assert_eq!(k(TokenKind::Keyword(Kw::Then)), "'then'");
    assert_eq!(k(TokenKind::Ident("x".into())), "identifier 'x'");
    assert_eq!(k(TokenKind::Punct(Punct::RParen)), "')'");
    assert_eq!(k(TokenKind::Int(3)), "integer 3");
    assert_eq!(k(TokenKind::Str("a".into())), "string \"a\"");
    assert_eq!(k(TokenKind::Eof), "end of file");
}
