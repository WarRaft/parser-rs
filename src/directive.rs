//! Per-file settings written as comments at the top of a file:
//!
//! ```text
//! // @dialect jass
//! globals ...
//! ```
//!
//! A setting is an ordinary line comment, so every tool that doesn't know
//! about it (JassHelper, the game, other editors) ignores it. That lets a
//! project mix files of different dialects, each declaring its own.
//!
//! Only the file's **header** is read: the leading run of blank lines and
//! line comments. The first line of anything else ends it, so a setting
//! can't hide in the middle of the code.

/// The value of the header setting `@<key>` in `src`, if present: for
/// `// @dialect jass` and `key = "dialect"`, `Some("jass")`. `line_comment`
/// is the language's comment opener (`"//"`, `"--"`, ...). Whitespace
/// around the parts is ignored. If the key is set more than once, the first
/// one wins.
pub fn header_directive<'a>(src: &'a str, line_comment: &str, key: &str) -> Option<&'a str> {
    for line in src.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let comment = line.strip_prefix(line_comment)?.trim();
        let Some(rest) = comment.strip_prefix('@').and_then(|c| c.strip_prefix(key)) else {
            continue;
        };
        // `@dialectx` is a different key, not `@dialect` + "x"
        if rest.is_empty() || rest.starts_with(char::is_whitespace) {
            return Some(rest.trim());
        }
    }
    None
}

#[cfg(test)]
#[path = "directive.test.rs"]
mod tests;
