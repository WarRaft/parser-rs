use super::*;

#[test]
fn declares_every_standard_type_and_modifier_in_documented_order() {
    let (types, mods) = legend();
    assert_eq!(
        types,
        [
            "namespace", "class", "enum", "interface", "struct", "typeParameter", "type",
            "parameter", "variable", "property", "enumMember", "decorator", "event", "function",
            "method", "macro", "label", "comment", "string", "keyword", "number", "regexp",
            "operator",
        ]
    );
    assert_eq!(
        mods,
        [
            "declaration", "definition", "readonly", "static", "deprecated", "abstract", "async",
            "modification", "documentation", "defaultLibrary",
        ]
    );
    assert_eq!(SemanticType::Operator.index(), 22);
    assert_eq!(SemanticModifier::DefaultLibrary.bit(), 1 << 9);
}

#[test]
fn modifiers_are_a_bitset() {
    let m = Modifiers::of(&[SemanticModifier::Declaration, SemanticModifier::Readonly]);
    assert_eq!(m.0, 0b101);
    assert!(m.contains(SemanticModifier::Readonly));
    assert!(!m.contains(SemanticModifier::Static));
    assert_eq!(m.with_if(false, SemanticModifier::Static), m);
    assert_eq!(m.with(SemanticModifier::Static).0, 0b1101);
}

fn tok(line: usize, s: usize, e: usize, ty: SemanticType) -> SemanticToken {
    SemanticToken::new(line, s, e, ty, Modifiers::NONE)
}

#[test]
fn encodes_relative_positions_in_utf16() {
    let src = "ab cd\n😀 x";
    let data = encode(
        vec![
            // given out of order on purpose
            tok(2, 3, 4, SemanticType::Variable),
            tok(1, 1, 3, SemanticType::Keyword),
            tok(1, 4, 6, SemanticType::Function),
        ],
        src,
    );
    assert_eq!(
        data,
        vec![
            0, 0, 2, SemanticType::Keyword.index(), 0,
            0, 3, 2, SemanticType::Function.index(), 0,
            // `x` sits after an astral emoji: UTF-16 column 3, not 2
            1, 3, 1, SemanticType::Variable.index(), 0,
        ]
    );
}

#[test]
fn drops_overlapping_and_empty_tokens() {
    let data = encode(
        vec![
            tok(1, 1, 5, SemanticType::Comment),
            tok(1, 3, 4, SemanticType::Keyword),
            tok(1, 6, 6, SemanticType::Keyword),
        ],
        "abcdefg",
    );
    assert_eq!(data.len(), 5);
}

#[test]
fn splits_multiline_comments_per_line() {
    let src = "x /* a\n\nbc */ y";
    let toks = comment_tokens(&[Span::new(1, 3, 3, 6)], src);
    assert_eq!(
        toks,
        vec![
            tok(1, 3, 7, SemanticType::Comment),
            tok(3, 1, 6, SemanticType::Comment),
        ]
    );
}

#[test]
fn classifies_lexical_tokens() {
    use crate::kind::Kw;
    let t = |k: TokenKind| lexical_type(&k);
    assert_eq!(t(TokenKind::Keyword(Kw::If)), Some(SemanticType::Keyword));
    assert_eq!(t(TokenKind::Int(1)), Some(SemanticType::Number));
    assert_eq!(t(TokenKind::Real(1.0)), Some(SemanticType::Number));
    assert_eq!(t(TokenKind::Str("s".into())), Some(SemanticType::String));
    assert_eq!(t(TokenKind::Punct(Punct::Plus)), Some(SemanticType::Operator));
    assert_eq!(t(TokenKind::Punct(Punct::LParen)), None);
    assert_eq!(t(TokenKind::Ident("x".into())), None);
}
