use super::*;

#[test]
fn reads_a_setting_from_the_header() {
    assert_eq!(header_directive("// @dialect jass\nglobals", "//", "dialect"), Some("jass"));
    assert_eq!(
        header_directive("\n  //   @dialect   vjass  \n", "//", "dialect"),
        Some("vjass")
    );
    assert_eq!(header_directive("-- @dialect lua54\n", "--", "dialect"), Some("lua54"));
}

#[test]
fn skips_other_header_comments() {
    let src = "// My library\n//! some JassHelper directive\n\n// @dialect jass\nglobals";
    assert_eq!(header_directive(src, "//", "dialect"), Some("jass"));
}

#[test]
fn stops_at_the_first_line_of_code() {
    let src = "globals\n// @dialect jass\nendglobals";
    assert_eq!(header_directive(src, "//", "dialect"), None);
}

#[test]
fn needs_an_exact_key() {
    assert_eq!(header_directive("// @dialectx jass", "//", "dialect"), None);
    assert_eq!(header_directive("// dialect jass", "//", "dialect"), None);
    assert_eq!(header_directive("// @dialect", "//", "dialect"), Some(""));
}

#[test]
fn first_setting_wins() {
    let src = "// @dialect jass\n// @dialect vjass\n";
    assert_eq!(header_directive(src, "//", "dialect"), Some("jass"));
}
