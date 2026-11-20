use super::*;

#[test]
fn rule_ids_are_unique_kebab_case() {
    let mut seen = std::collections::HashSet::new();
    for r in RULES {
        assert!(seen.insert(r.id), "{} listed twice", r.id);
        assert!(r.id.chars().all(|c| c.is_ascii_lowercase() || c == '-'), "{}", r.id);
        assert!(!r.languages.is_empty() && !r.summary.is_empty(), "{}", r.id);
    }
}

#[test]
fn only_infinite_loop_is_off_by_default() {
    let off: Vec<_> = RULES.iter().filter(|r| !r.enabled).map(|r| r.id).collect();
    assert_eq!(off, vec![rule::INFINITE_LOOP]);
    assert_eq!(find_rule("unused-local").map(|r| r.enabled), Some(true));
    assert_eq!(find_rule("nope"), None);
}
