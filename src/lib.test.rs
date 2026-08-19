use super::*;

/// `line` = a(1cp), ю(1cp, 2-byte UTF-8, 1 UTF-16 unit), 😀(1cp, 4-byte
/// UTF-8, 2 UTF-16 units — outside the BMP, same as the icon-font PUA
/// codepoints mentioned in the design discussion), b(1cp).
const MIXED_LINE: &str = "aю😀b";

#[test]
fn utf16_recoding_only_widens_for_astral_codepoints() {
    let e = ColumnEncoding::Utf16;
    assert_eq!(e.from_codepoints(MIXED_LINE, 1), 1); // before 'a'
    assert_eq!(e.from_codepoints(MIXED_LINE, 2), 2); // before 'ю' — Cyrillic is BMP, no widening
    assert_eq!(e.from_codepoints(MIXED_LINE, 3), 3); // before '😀'
    assert_eq!(e.from_codepoints(MIXED_LINE, 4), 5); // before 'b' — 😀 ate 2 units
    assert_eq!(e.from_codepoints(MIXED_LINE, 5), 6); // end of line
}

#[test]
fn utf8_recoding_widens_for_every_non_ascii_codepoint() {
    let e = ColumnEncoding::Utf8;
    assert_eq!(e.from_codepoints(MIXED_LINE, 1), 1);
    assert_eq!(e.from_codepoints(MIXED_LINE, 2), 2); // before 'ю' — 'a' is 1 byte
    assert_eq!(e.from_codepoints(MIXED_LINE, 3), 4); // before '😀' — 'ю' is 2 bytes
    assert_eq!(e.from_codepoints(MIXED_LINE, 4), 8); // before 'b' — 😀 is 4 bytes
    assert_eq!(e.from_codepoints(MIXED_LINE, 5), 9); // end of line
}

#[test]
fn codepoints_encoding_is_a_no_op() {
    let e = ColumnEncoding::Codepoints;
    for col in 0..=5 {
        assert_eq!(e.from_codepoints(MIXED_LINE, col), col);
        assert_eq!(e.to_codepoints(MIXED_LINE, col), col);
    }
}

#[test]
fn to_codepoints_inverts_from_codepoints_at_codepoint_boundaries() {
    for encoding in [ColumnEncoding::Utf16, ColumnEncoding::Utf8] {
        for col in 1..=5 {
            let recoded = encoding.from_codepoints(MIXED_LINE, col);
            assert_eq!(
                encoding.to_codepoints(MIXED_LINE, recoded),
                col,
                "{encoding:?} round-trip failed for codepoint column {col}"
            );
        }
    }
}

#[test]
fn whole_line_sentinel_survives_recoding() {
    let span = Span::whole_line(3);
    assert_eq!(span.in_encoding(MIXED_LINE, ColumnEncoding::Utf16), span);
}

#[test]
fn span_in_encoding_widens_only_the_affected_column() {
    // A Var-like span for the 'b' in `aю😀b`, which sits after an astral
    // codepoint.
    let span = Span::new(1, 4, 1, 5); // codepoint columns
    let utf16 = span.in_encoding(MIXED_LINE, ColumnEncoding::Utf16);
    assert_eq!(utf16, Span::new(1, 5, 1, 6));
    let utf8 = span.in_encoding(MIXED_LINE, ColumnEncoding::Utf8);
    assert_eq!(utf8, Span::new(1, 8, 1, 9));
}

#[test]
fn join_spans_left_start_to_right_end() {
    let left = Span::new(2, 12, 2, 13);
    let right = Span::new(2, 16, 2, 17);
    assert_eq!(left.join(right), Span::new(2, 12, 2, 17));
}
