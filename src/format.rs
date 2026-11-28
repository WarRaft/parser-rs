//! Formatting that only moves lines: every line keeps its place and its
//! text, and only the whitespace it starts with changes. A formatter never
//! adds, removes, joins or splits a line, so a diff of a formatted file is
//! its indentation alone, and every line number (a diagnostic's, a
//! breakpoint's, a map's script error) still means the same line.
//!
//! A language says how deep each line goes ([`Indent`], one per line); the
//! shared [`edits`] turns that into edits of the lines' leading
//! whitespace. [`braces`] works out the levels of a language whose blocks
//! are `{ }` (Zinc, AngelScript) from its tokens; a keyword language
//! (JASS) counts its own blocks and uses [`literal_lines`] for what it
//! must not move.
//!
//! Formatting reads the tolerant token stream, never the AST: it works on
//! code that doesn't parse, as it's being typed.

use std::ops::RangeInclusive;

use crate::diagnostic::Edit;
use crate::kind::{Kw, Punct, TokenKind};
use crate::lexer::{tokenize_lossy_at, LexSpec, Lexed};
use crate::span::Span;

/// Where one line goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Indent {
    /// Left as it is: a line inside a string (its whitespace is the
    /// string's), or one no rule speaks of.
    Keep,
    /// That many indentation units deep.
    Level(usize),
    /// Moved by as much as line `n` (1-based) is: the inside of a block
    /// comment, or of a block a language keeps as text (a textmacro),
    /// moves with its first line and keeps its own alignment. A line
    /// that starts left of line `n`'s old indentation stays.
    Follow(usize),
}

/// The edits that indent `src`'s lines `lines` (1-based, inclusive; the
/// rest stay) as `indents` says, `indents[0]` being line 1's, each level
/// `unit` (four spaces, a tab). A line of whitespace alone is emptied,
/// unless it's [`Indent::Keep`]. Only lines that change get an edit, and
/// an edit only ever replaces the spaces and tabs a line starts with (a
/// byte-order mark stays).
pub fn edits(src: &str, indents: &[Indent], unit: &str, lines: RangeInclusive<usize>) -> Vec<Edit> {
    let mut out = Vec::new();
    // Each line's indentation, old and new, for `Follow`.
    let mut old: Vec<String> = Vec::new();
    let mut new: Vec<String> = Vec::new();
    for (i, text) in src.split('\n').enumerate() {
        let line = i + 1;
        let text = text.strip_suffix('\r').unwrap_or(text);
        let bom = usize::from(line == 1 && text.starts_with('\u{feff}'));
        let body: &str = if bom == 1 { &text['\u{feff}'.len_utf8()..] } else { text };
        let lead: String = body.chars().take_while(|c| matches!(c, ' ' | '\t')).collect();
        let blank = lead.len() == body.len();
        let wanted = match indents.get(i).copied().unwrap_or(Indent::Keep) {
            Indent::Keep => lead.clone(),
            _ if blank => String::new(),
            Indent::Level(n) => unit.repeat(n),
            Indent::Follow(n) => match (old.get(n.wrapping_sub(1)), new.get(n.wrapping_sub(1))) {
                (Some(head_old), Some(head_new)) if n < line => match lead.strip_prefix(head_old.as_str()) {
                    Some(rest) => format!("{head_new}{rest}"),
                    None => lead.clone(),
                },
                _ => lead.clone(),
            },
        };
        if wanted != lead && lines.contains(&line) {
            let col = 1 + bom;
            out.push(Edit {
                span: Span::new(line, col, line, col + lead.chars().count()),
                text: wanted.clone(),
            });
        }
        // A line outside the range is where it is: what follows it moves
        // as it does.
        let kept = if lines.contains(&line) { wanted } else { lead.clone() };
        old.push(lead);
        new.push(kept);
    }
    out
}

/// The number of lines of `src`: one more than its line breaks.
pub fn line_count(src: &str) -> usize {
    src.matches('\n').count() + 1
}

/// Marks what a formatter must not move by itself in `indents` (the lines
/// of a text starting at line `first_line`, `indents[0]` being its): a
/// line a string or heredoc goes on into stays ([`Indent::Keep`]), its
/// whitespace being the string's; a line a block comment goes on into
/// moves with the comment's first line ([`Indent::Follow`]).
pub fn literal_lines(lexed: &Lexed, first_line: usize, indents: &mut [Indent]) {
    let mut set = |line: usize, indent: Indent| {
        if let Some(slot) = line.checked_sub(first_line).and_then(|i| indents.get_mut(i)) {
            *slot = indent;
        }
    };
    for c in &lexed.comments {
        for line in c.start_line + 1..=c.end_line {
            set(line, Indent::Follow(c.start_line));
        }
    }
    for t in &lexed.tokens {
        if t.end_line > t.line && !matches!(t.kind, TokenKind::Embedded(_)) {
            for line in t.line + 1..=t.end_line {
                set(line, Indent::Keep);
            }
        }
    }
}

/// How a `{ }` language lays out what [`braces`] can't see from brackets
/// alone.
#[derive(Debug, Clone, Copy, Default)]
pub struct BraceStyle {
    /// `case x:` and `default:` labels (AngelScript's `switch`): a label
    /// stands at its `switch`'s level, as AngelScript's manual has it, and
    /// so does a `{` right after one; the label's code is one deeper.
    pub case_labels: bool,
}

/// An open bracket: which, and the level of the line it opened on.
struct Open {
    punct: Punct,
    level: usize,
}

/// The closing bracket of `p`, if it opens one.
fn closing(p: Punct) -> Option<Punct> {
    match p {
        Punct::LBrace => Some(Punct::RBrace),
        Punct::LParen => Some(Punct::RParen),
        Punct::LBracket => Some(Punct::RBracket),
        _ => None,
    }
}

fn is_closer(p: Punct) -> bool {
    matches!(p, Punct::RBrace | Punct::RParen | Punct::RBracket)
}

/// Closes the innermost bracket `p` closes, and any opened in it that
/// were left open; nothing if none is open. The level of the line it
/// opened on.
fn close(stack: &mut Vec<Open>, p: Punct) -> Option<usize> {
    let i = stack.iter().rposition(|o| closing(o.punct) == Some(p))?;
    let level = stack[i].level;
    stack.truncate(i);
    Some(level)
}

/// The levels of a `{ }` language's lines (`src`, lexed by `spec`, the
/// first being line `first_line` of its file), `base` levels deep:
///
/// - A line inside a bracket (`{ ( [`) is a level deeper than the line
///   the bracket opened on, so brackets opened on one line count once
///   (`f(g(` is one level). A line starting with closing brackets stands
///   where the line of the first of them opened: `});` under
///   `function() {`.
/// - A statement going on to the next line (the line before doesn't end
///   in `;`, `{`, `}`, `,` or `:`, inside braces or at the top) puts that
///   line one deeper, so `if (x)` without braces indents its statement;
///   a line starting with `{` doesn't, the block's own.
/// - What [`literal_lines`] marks moves as it says.
pub fn braces(src: &str, spec: &LexSpec, first_line: usize, base: usize, style: BraceStyle) -> Vec<Indent> {
    let lexed = tokenize_lossy_at(src, spec, first_line);
    let count = line_count(src);
    let mut indents = vec![Indent::Level(base); count];
    literal_lines(&lexed, first_line, &mut indents);
    let mut stack: Vec<Open> = Vec::new();
    // The last token before the line: does a statement go on?
    let mut prev: Option<&TokenKind> = None;
    let tokens: Vec<_> = lexed.tokens.iter().filter(|t| t.kind != TokenKind::Eof).collect();
    let mut next = 0;
    // The level of the line before, for brackets opened on a line that
    // isn't its own (one a string goes on into).
    let mut last = base;
    for line in first_line..first_line + count {
        let start = next;
        while next < tokens.len() && tokens[next].line == line {
            next += 1;
        }
        let here = &tokens[start..next];
        let slot = line - first_line;
        let mut rest = here;
        if matches!(indents[slot], Indent::Level(_)) {
            // Closing brackets first: the line stands where the first
            // of them opened.
            let mut closed = None;
            while let Some(TokenKind::Punct(p)) = rest.first().map(|t| &t.kind) {
                if !is_closer(*p) {
                    break;
                }
                closed = closed.or(close(&mut stack, *p));
                rest = &rest[1..];
            }
            let first = here.first().map(|t| &t.kind);
            let in_braces = stack.last().is_none_or(|o| o.punct == Punct::LBrace);
            let label = style.case_labels
                && in_braces
                && !stack.is_empty()
                && matches!(first, Some(TokenKind::Keyword(Kw::Case | Kw::Default)));
            let brace = matches!(first, Some(TokenKind::Punct(Punct::LBrace)));
            let after_label = style.case_labels && matches!(prev, Some(TokenKind::Punct(Punct::Colon)));
            let level = match (closed, stack.last()) {
                (Some(level), _) => level,
                // A label stands at its switch's level, and so does a
                // block right after one: `case 1:` `{`.
                (None, Some(o)) if label || (brace && after_label) => o.level,
                (None, top) => {
                    let ended = match prev {
                        None => true,
                        Some(TokenKind::Punct(p)) => matches!(
                            p,
                            Punct::Semicolon | Punct::LBrace | Punct::RBrace | Punct::Comma | Punct::Colon
                        ),
                        Some(_) => false,
                    };
                    let going_on = in_braces && !ended && !brace && !here.is_empty();
                    top.map_or(base, |o| o.level + 1) + usize::from(going_on)
                }
            };
            indents[slot] = Indent::Level(level);
            last = level;
        }
        for t in rest {
            if let TokenKind::Punct(p) = t.kind {
                if closing(p).is_some() {
                    stack.push(Open { punct: p, level: last });
                } else if is_closer(p) {
                    close(&mut stack, p);
                }
            }
        }
        if let Some(t) = here.last() {
            prev = Some(&t.kind);
        }
    }
    indents
}

#[cfg(test)]
#[path = "format.test.rs"]
mod tests;
