use super::*;
use std::collections::HashSet;

#[test]
fn keyword_spellings_round_trip_and_are_unique() {
    let mut seen = HashSet::new();
    for &k in Kw::ALL {
        assert!(seen.insert(k.as_str()), "duplicate spelling {:?}", k.as_str());
        assert_eq!(Kw::from_word(k.as_str()), Some(k));
    }
    assert_eq!(Kw::from_word("Then"), None); // case-sensitive
    assert_eq!(Kw::Then.to_string(), "then");
}

#[test]
fn keyword_sets_activate_keywords() {
    const JASS: KwSet = KwSet::of(&[Kw::If, Kw::Local]);
    const VJASS: KwSet = JASS.union(KwSet::of(&[Kw::Library]));
    assert_eq!(JASS.lookup("library"), None); // plain identifier in JASS
    assert_eq!(VJASS.lookup("library"), Some(Kw::Library));
    assert_eq!(VJASS.lookup("local"), Some(Kw::Local));
    assert_eq!(VJASS.lookup("nil"), None); // a Lua keyword, inactive here
    assert!(KwSet::of(Kw::ALL).contains(Kw::Goto));
    assert!(!KwSet::EMPTY.contains(Kw::If));
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
    let k = |t: TokenKind| t.to_string();
    assert_eq!(k(TokenKind::Keyword(Kw::Then)), "'then'");
    assert_eq!(k(TokenKind::Ident("x".into())), "identifier 'x'");
    assert_eq!(k(TokenKind::Punct(Punct::RParen)), "')'");
    assert_eq!(k(TokenKind::Int(3)), "integer 3");
    assert_eq!(k(TokenKind::Str("a".into())), "string \"a\"");
    assert_eq!(k(TokenKind::Eof), "end of file");
}
