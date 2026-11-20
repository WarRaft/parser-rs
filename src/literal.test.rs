use super::*;

const WC3: NumberSpec = NumberSpec {
    exponent: false,
    leading_dot: true,
    radix_prefixes: false,
    float_suffix: false,
    trailing_dot: true,
    leading_zero_octal: true,
    dollar_hex: true,
};
const ALL: NumberSpec = NumberSpec {
    exponent: true,
    leading_dot: true,
    radix_prefixes: true,
    float_suffix: true,
    trailing_dot: false,
    leading_zero_octal: false,
    dollar_hex: false,
};

fn num(src: &str, spec: &NumberSpec) -> Result<Number, ParseError> {
    number(&mut Cursor::new(src), spec, 1, 1)
}

#[test]
fn scans_numbers() {
    assert_eq!(num("123", &WC3), Ok(Number::Int(123)));
    assert_eq!(num("0x1F", &WC3), Ok(Number::Int(31)));
    assert_eq!(num("4.25", &WC3), Ok(Number::Real(4.25)));
    assert_eq!(num("0xFFFFFFFFFFFFFFFF", &WC3), Ok(Number::Int(-1))); // wraps
}

#[test]
fn trailing_dot_reals() {
    assert_eq!(num("2144.", &WC3), Ok(Number::Real(2144.0)));
    let mut cur = Cursor::new("225.)");
    assert_eq!(number(&mut cur, &WC3, 1, 1), Ok(Number::Real(225.0)));
    assert_eq!(cur.peek(), Some(')'));
    assert_eq!(num(".5", &WC3), Ok(Number::Real(0.5)));
    // Without the option the dot is left for the next token (Lua's `..`).
    let mut cur = Cursor::new("1.)");
    assert_eq!(number(&mut cur, &ALL, 1, 1), Ok(Number::Int(1)));
    assert_eq!(cur.peek(), Some('.'));
}

#[test]
fn leading_zero_octal() {
    assert_eq!(num("017", &WC3), Ok(Number::Int(15)));
    assert_eq!(num("0", &WC3), Ok(Number::Int(0)));
    assert_eq!(num("00", &WC3), Ok(Number::Int(0)));
    assert_eq!(num("0.5", &WC3), Ok(Number::Real(0.5)));
    assert_eq!(num("017.5", &WC3), Ok(Number::Real(17.5)));
    assert_eq!(num("017.", &WC3), Ok(Number::Real(17.0)));
    let err = num("08", &WC3).unwrap_err();
    assert!(err.message.contains("invalid octal literal '08'"), "{err:?}");
    // Without the option it's decimal.
    assert_eq!(num("017", &ALL), Ok(Number::Int(17)));
}

#[test]
fn dollar_hex() {
    assert_eq!(num("$A", &WC3), Ok(Number::Int(10)));
    assert_eq!(num("$ff", &WC3), Ok(Number::Int(255)));
    assert_eq!(num("$FFFFFFFF", &WC3), Ok(Number::Int(0xFFFF_FFFF)));
    let mut cur = Cursor::new("$984)");
    assert_eq!(number(&mut cur, &WC3, 1, 1), Ok(Number::Int(0x984)));
    assert_eq!(cur.peek(), Some(')'));
    let err = num("$g", &WC3).unwrap_err();
    assert!(err.message.contains("invalid literal '$'"), "{err:?}");
}

#[test]
fn optional_number_forms() {
    assert_eq!(num("1e3", &ALL), Ok(Number::Real(1000.0)));
    assert_eq!(num("2.5E-1", &ALL), Ok(Number::Real(0.25)));
    assert_eq!(num("0b101", &ALL), Ok(Number::Int(5)));
    assert_eq!(num("0o17", &ALL), Ok(Number::Int(15)));
    assert_eq!(num("1.5f", &ALL), Ok(Number::Real(1.5)));
    // without the options those stop early
    let mut cur = Cursor::new("1e3");
    assert_eq!(number(&mut cur, &WC3, 1, 1), Ok(Number::Int(1)));
    assert_eq!(cur.peek(), Some('e'));
    // `1e` with no digits is `1` followed by an identifier
    let mut cur = Cursor::new("1else");
    assert_eq!(number(&mut cur, &ALL, 1, 1), Ok(Number::Int(1)));
}

#[test]
fn a_dot_without_digits_is_not_part_of_the_number() {
    let mut cur = Cursor::new("1..2");
    assert_eq!(number(&mut cur, &ALL, 1, 1), Ok(Number::Int(1)));
    assert_eq!(cur.peek(), Some('.'));
}

#[test]
fn scans_strings_with_escapes() {
    let mut cur = Cursor::new(r#""a\n\"b\\" rest"#);
    assert_eq!(string(&mut cur, 1, 1), Ok("a\n\"b\\".to_string()));
    assert_eq!(cur.peek(), Some(' '));
    let mut cur = Cursor::new(r#"'it\'s \x41\65\u{263A}'"#);
    assert_eq!(quoted(&mut cur, '\'', 1, 1), Ok("it's AA☺".to_string()));
}

#[test]
fn reports_unterminated_string() {
    let err = string(&mut Cursor::new("\"abc\nx"), 3, 7).unwrap_err();
    assert!(err.message.contains("unterminated string"));
    assert_eq!((err.line, err.col), (3, 7));
}

#[test]
fn packs_rawcodes() {
    assert_eq!(rawcode(&mut Cursor::new("'hfoo'"), 1, 1), Ok(0x68666f6f));
    assert_eq!(rawcode(&mut Cursor::new("'d'"), 1, 1), Ok(100));
    for bad in ["'ab'", "'abc'", "''", "'abcde'"] {
        let err = rawcode(&mut Cursor::new(bad), 1, 1).unwrap_err();
        assert!(err.message.contains("must be 1 or 4 characters"), "{bad}: {err:?}");
    }
}

#[test]
fn scans_lua_long_brackets() {
    let cur = Cursor::new("[==[");
    assert_eq!(long_bracket_level(&cur, 0), Some(2));
    assert_eq!(long_bracket_level(&Cursor::new("[=x"), 0), None);
    let mut cur = Cursor::new("[[\nline1\n]] ]]");
    assert_eq!(long_bracket(&mut cur, "long string", 1, 1), Ok("line1\n".to_string()));
    assert_eq!((cur.line(), cur.col()), (3, 3));
    let mut cur = Cursor::new("[=[ a ]] b ]=]");
    assert_eq!(long_bracket(&mut cur, "long string", 1, 1), Ok(" a ]] b ".to_string()));
    let err = long_bracket(&mut Cursor::new("[[ open"), "long string", 1, 1).unwrap_err();
    assert_eq!(err.message, "unterminated long string");
}

#[test]
fn scans_heredocs() {
    let mut cur = Cursor::new("\"\"\"a\n\"b\"\n\"\"\"x");
    assert_eq!(heredoc(&mut cur, 1, 1), Ok("a\n\"b\"\n".to_string()));
    assert_eq!(cur.peek(), Some('x'));
}
