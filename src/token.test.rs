use super::*;

#[derive(Debug, Clone, PartialEq)]
enum K {
    Num(i64),
    Minus,
    Eof,
}

impl std::fmt::Display for K {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl TokenKindExt for K {
    fn is_eof(&self) -> bool {
        *self == K::Eof
    }
}

fn stream(kinds: Vec<K>) -> TokenStream<K> {
    TokenStream::new(
        kinds
            .into_iter()
            .enumerate()
            .map(|(i, k)| Token::new(k, 1, i + 1, i + 2))
            .collect(),
    )
}

struct Calc {
    ts: TokenStream<K>,
}

impl TokenParser for Calc {
    type Kind = K;
    fn tokens(&mut self) -> &mut TokenStream<K> {
        &mut self.ts
    }
}

impl Calc {
    fn num(&mut self) -> Result<i64, ParseError> {
        match self.ts.peek_kind().clone() {
            K::Num(n) => {
                self.ts.advance();
                Ok(n)
            }
            other => Err(self.ts.error(format!("expected number, found {other:?}"))),
        }
    }
}

#[test]
fn left_assoc_folds_left() {
    let mut p = Calc {
        ts: stream(vec![K::Num(10), K::Minus, K::Num(3), K::Minus, K::Num(2), K::Eof]),
    };
    let v = p
        .left_assoc(Calc::num, |k| (*k == K::Minus).then_some(()), |a, _, b| a - b)
        .unwrap();
    assert_eq!(v, 5);
    assert!(p.ts.at_eof());
}

#[test]
fn expect_reports_the_offending_token_span() {
    let mut ts = stream(vec![K::Num(1), K::Eof]);
    let err = ts.expect(K::Minus).unwrap_err();
    assert_eq!(err.message, "expected Minus, found Num(1)");
    assert_eq!((err.line, err.col, err.end_col), (1, 1, 2));
}

#[test]
fn advancing_past_the_end_sticks_at_eof() {
    let mut ts = stream(vec![K::Eof]);
    ts.advance();
    ts.advance();
    assert!(ts.at_eof());
    assert_eq!(ts.nth_kind(5), &K::Eof);
}

#[test]
fn errors_are_at_least_one_column_wide() {
    let eof = Token::new(K::Eof, 3, 7, 7);
    let err = ParseError::at_token("x", &eof);
    assert_eq!((err.col, err.end_col), (7, 8));
}
