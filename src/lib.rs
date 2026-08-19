//! Foundational, language-agnostic types shared across this author's
//! hand-written parsers (JASS today, more grammars later) — a source
//! [`Span`] and a [`ColumnEncoding`] for translating it into whatever
//! coordinate convention a given consumer expects (an editor extension's
//! own API, a byte-oriented tool, ...), without every parser reinventing
//! the same position bookkeeping, and without paying to carry every
//! encoding on every AST node up front.

/// A source location: a line/column range. Columns are 1-indexed and
/// codepoint-counted by convention — see [`ColumnEncoding`] for recoding
/// into UTF-16 units, UTF-8 bytes, or back. `end_line`/`end_col` are
/// exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

impl Span {
    pub fn new(start_line: usize, start_col: usize, end_line: usize, end_col: usize) -> Self {
        Span {
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }

    /// A span with no column information, meaning "somewhere on this
    /// line" — for constructs (e.g. a statement) that only track a
    /// starting line, not exact columns.
    pub fn whole_line(line: usize) -> Self {
        Span::new(line, 0, line, 0)
    }

    /// Combines two spans into the range from `self`'s start to `other`'s
    /// end — e.g. for a binary expression spanning from its left operand's
    /// start to its right operand's end.
    pub fn join(self, other: Span) -> Span {
        Span::new(
            self.start_line,
            self.start_col,
            other.end_line,
            other.end_col,
        )
    }

    /// Recodes this span's columns into `encoding`, given the source text
    /// it was parsed from. Only columns change — `Span` always stores plain
    /// Unicode codepoint counts internally (the cheapest, simplest
    /// representation, and the one every consumer gets for free), and
    /// recoding into a consumer's own convention (UTF-16 for VS Code/LSP,
    /// UTF-8 bytes, ...) happens on demand for just the span(s) actually
    /// needed. That keeps `Span` itself at a fixed 4 `usize`s regardless of
    /// how many encodings exist, instead of multiplying its size across
    /// every AST node in a large file for encodings most callers never use.
    ///
    /// A "whole line" span (see [`Span::whole_line`], `start_col`/`end_col`
    /// both `0`) is returned unchanged: there's no column to recode.
    /// Lines outside `source` are also left unchanged.
    pub fn in_encoding(self, source: &str, encoding: ColumnEncoding) -> Span {
        let recode = |line: usize, col: usize| {
            line.checked_sub(1)
                .and_then(|i| source.lines().nth(i))
                .map(|text| encoding.from_codepoints(text, col))
                .unwrap_or(col)
        };
        Span::new(
            self.start_line,
            recode(self.start_line, self.start_col),
            self.end_line,
            recode(self.end_line, self.end_col),
        )
    }
}

/// How a character offset within a line is counted. `Span` always stores
/// plain Unicode codepoint counts (see [`Span::in_encoding`] for why); this
/// picks the unit a *consumer* wants their columns translated into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnEncoding {
    /// One unit per Unicode codepoint (`char`). `Span`'s native counting,
    /// so converting to/from it is a no-op.
    Codepoints,
    /// One unit per UTF-16 code unit — two for codepoints outside the
    /// Basic Multilingual Plane (emoji, some CJK extension ideographs, and
    /// the supplementary-plane Private Use Area some icon fonts use, e.g.
    /// for in-game glyphs). This is what VS Code's and LSP's default
    /// `Position` expects, because both are ultimately backed by
    /// UTF-16-native JavaScript strings.
    Utf16,
    /// One unit per UTF-8 byte. Matches LSP's `positionEncoding: "utf-8"`
    /// and any tool that works on raw byte offsets.
    Utf8,
}

impl ColumnEncoding {
    /// Converts `col`, measured in `self`'s units on `line_text`, to a
    /// codepoint-counted column (`Span`'s native representation). `0` (the
    /// "whole line" sentinel) always maps to `0` unchanged. A `col` that
    /// falls strictly inside a multi-unit codepoint (e.g. mid-surrogate-pair
    /// for `Utf16`) snaps forward to the codepoint boundary after it,
    /// rather than panicking or guessing.
    pub fn to_codepoints(self, line_text: &str, col: usize) -> usize {
        if col == 0 || self == ColumnEncoding::Codepoints {
            return col;
        }
        let mut units = 0usize;
        for (i, c) in line_text.chars().enumerate() {
            if units >= col - 1 {
                return i + 1;
            }
            units += self.unit_len(c);
        }
        line_text.chars().count() + 1
    }

    /// The inverse of [`to_codepoints`](Self::to_codepoints): converts a
    /// codepoint-counted column into one measured in `self`'s units.
    pub fn from_codepoints(self, line_text: &str, codepoint_col: usize) -> usize {
        if codepoint_col == 0 || self == ColumnEncoding::Codepoints {
            return codepoint_col;
        }
        line_text
            .chars()
            .take(codepoint_col - 1)
            .map(|c| self.unit_len(c))
            .sum::<usize>()
            + 1
    }

    fn unit_len(self, c: char) -> usize {
        match self {
            ColumnEncoding::Codepoints => 1,
            ColumnEncoding::Utf16 => c.len_utf16(),
            ColumnEncoding::Utf8 => c.len_utf8(),
        }
    }
}

#[cfg(test)]
#[path = "lib.test.rs"]
mod tests;
