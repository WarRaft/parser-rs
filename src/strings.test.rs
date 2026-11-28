use super::*;
use crate::lexer::{tokenize_lossy, LexSpec};

#[test]
fn quoted_strings_and_their_text() {
    let spec = LexSpec { single_quote_strings: true, long_brackets: true, ..LexSpec::BASE };
    let src = "a \"Menu\" 'B' [[long]]\n\"\" \"e\\\"\"";
    let got: Vec<(String, usize, usize, usize)> =
        literals(&tokenize_lossy(src, &spec).tokens, src).into_iter().map(|l| (l.text, l.span.start_line, l.span.start_col, l.span.end_col)).collect();
    assert_eq!(got, [("Menu".into(), 1, 4, 8), ("B".into(), 1, 11, 12), ("".into(), 2, 2, 2), ("e\"".into(), 2, 5, 8)]);
}
