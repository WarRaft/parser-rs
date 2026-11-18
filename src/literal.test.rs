use super::*;

fn num(src: &str) -> Result<Number, ParseError> {
    number(&mut Cursor::new(src), 1, 1)
}

#[test]
fn scans_numbers() {
    assert_eq!(num("123"), Ok(Number::Int(123)));
    assert_eq!(num("0x1F"), Ok(Number::Int(31)));
    assert_eq!(num("4.25"), Ok(Number::Real(4.25)));
}

#[test]
fn a_dot_without_digits_is_not_part_of_the_number() {
    let mut cur = Cursor::new("1.x");
    assert_eq!(number(&mut cur, 1, 1), Ok(Number::Int(1)));
    assert_eq!(cur.peek(), Some('.'));
}

#[test]
fn scans_strings_with_escapes() {
    let mut cur = Cursor::new(r#""a\n\"b\\" rest"#);
    assert_eq!(string(&mut cur, 1, 1), Ok("a\n\"b\\".to_string()));
    assert_eq!(cur.peek(), Some(' '));
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
    let err = rawcode(&mut Cursor::new("'ab'"), 1, 1).unwrap_err();
    assert!(err.message.contains("4 characters"));
}
