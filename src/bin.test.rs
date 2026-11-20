use super::*;

#[test]
fn reads_little_endian() {
    let mut r = Reader::new(&[1, 0, 0, 0, 0xFE, 0xFF, 0xFF, 0xFF, 0, 0, 0x80, 0x3F, 7, 0]);
    assert_eq!(r.u32(), Ok(1));
    assert_eq!(r.i32(), Ok(-2));
    assert_eq!(r.f32(), Ok(1.0));
    assert_eq!(r.u16(), Ok(7));
    assert!(r.is_empty());
    assert_eq!(r.finish(), Ok(()));
}

#[test]
fn eof_reports_offset() {
    let mut r = Reader::new(&[1, 2, 3, 4, 5]);
    r.u32().unwrap();
    let err = r.u32().unwrap_err();
    assert_eq!(err.offset, 4);
    assert_eq!(
        err.to_string(),
        "unexpected end of file: need 4 bytes, 1 left (at byte 0x4)"
    );
}

#[test]
fn cstr() {
    let mut r = Reader::new(b"abc\0\0x");
    assert_eq!(r.cstr().as_deref(), Ok("abc"));
    assert_eq!(r.cstr().as_deref(), Ok(""));
    assert_eq!(r.cstr().unwrap_err().offset, 5);
}

#[test]
fn cstr_rejects_bad_utf8() {
    let mut r = Reader::new(b"ab\xFF\0");
    let err = r.cstr().unwrap_err();
    assert_eq!(err.offset, 2);
    assert_eq!(r.pos(), 0);
}

#[test]
fn count_checks_what_is_left() {
    let mut r = Reader::new(&[3, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(r.count(4).unwrap_err().offset, 0);
    let mut r = Reader::new(&[2, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0]);
    assert_eq!(r.list(4, Reader::u32), Ok(vec![1, 2]));
}

#[test]
fn finish_reports_trailing_bytes() {
    let r = Reader::new(&[0]);
    assert_eq!(
        r.finish().unwrap_err().message,
        "1 extra byte after the end of the data"
    );
}

#[test]
fn writer_mirrors_reader() {
    let mut w = Writer::new();
    w.u32(5);
    w.i32(-1);
    w.f32(0.5);
    w.rawcode(Rawcode(*b"hfoo"));
    w.cstr("héllo").unwrap();
    w.count(2).unwrap();
    let bytes = w.into_bytes();

    let mut r = Reader::new(&bytes);
    assert_eq!(r.u32(), Ok(5));
    assert_eq!(r.i32(), Ok(-1));
    assert_eq!(r.f32(), Ok(0.5));
    assert_eq!(r.rawcode(), Ok(Rawcode(*b"hfoo")));
    assert_eq!(r.cstr().as_deref(), Ok("héllo"));
    assert_eq!(r.u32(), Ok(2));
    assert_eq!(r.finish(), Ok(()));
}

#[test]
fn writer_rejects_nul_in_string() {
    let mut w = Writer::new();
    w.u32(0);
    assert_eq!(w.cstr("a\0b").unwrap_err().offset, 5);
}

#[test]
fn rawcode_display_and_parse() {
    let foo = Rawcode(*b"hfoo");
    assert_eq!(foo.to_string(), "hfoo");
    assert_eq!(foo.to_int(), 0x6866_6F6F);
    assert_eq!(Rawcode::from_int(0x6866_6F6F), foo);
    assert_eq!(Rawcode::parse("hfoo"), Some(foo));

    assert_eq!(Rawcode::NONE.to_string(), "0x00000000");
    assert_eq!(Rawcode::parse("0x00000000"), Some(Rawcode::NONE));
    assert_eq!(Rawcode(*b"a'bc").to_string(), "0x61276263");
    assert_eq!(Rawcode::parse("a'bc"), None);
    assert_eq!(Rawcode::parse("abc"), None);
    assert_eq!(Rawcode::parse("0x123"), None);
}
