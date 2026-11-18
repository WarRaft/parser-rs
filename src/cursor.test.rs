use super::*;

#[test]
fn tracks_lines_and_codepoint_columns() {
    let mut c = Cursor::new("aю\nb");
    c.advance();
    c.advance();
    assert_eq!((c.line(), c.col()), (1, 3));
    c.advance();
    assert_eq!((c.line(), c.col()), (2, 1));
}

#[test]
fn skips_line_and_block_comments() {
    let mut c = Cursor::new("  // x\n /* a\n b */ z");
    c.skip_trivia("//", Some(("/*", "*/"))).unwrap();
    assert_eq!(c.peek(), Some('z'));
    assert_eq!((c.line(), c.col()), (3, 7));
}

#[test]
fn block_comments_are_ignored_when_not_configured() {
    let mut c = Cursor::new("/* x */");
    c.skip_trivia("//", None).unwrap();
    assert_eq!(c.peek(), Some('/'));
}

#[test]
fn reports_unterminated_block_comment_at_its_start() {
    let mut c = Cursor::new("\n  /* never closed");
    let err = c.skip_trivia("//", Some(("/*", "*/"))).unwrap_err();
    assert_eq!((err.line, err.col), (2, 3));
}

#[test]
fn take_while_and_eat_str() {
    let mut c = Cursor::new("abc123->");
    assert_eq!(c.take_while(char::is_alphabetic), "abc");
    assert_eq!(c.take_while(|c| c.is_ascii_digit()), "123");
    assert!(!c.eat_str("=>"));
    assert!(c.eat_str("->"));
    assert!(c.is_eof());
}
