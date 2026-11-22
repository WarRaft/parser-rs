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
fn off_by_default() {
    let off: Vec<_> = RULES.iter().filter(|r| !r.enabled).map(|r| r.id).collect();
    assert_eq!(
        off,
        vec![
            rule::INFINITE_LOOP,
            rule::IMPLICIT_SIGNATURE,
            rule::IMPLICIT_KEYWORD,
            rule::MERGED_FUNCTION,
            rule::DECLARATION_AFTER_FUNCTION,
            rule::GLOBAL_OUTSIDE_GLOBALS,
        ]
    );
    assert_eq!(find_rule("unused-local").map(|r| r.enabled), Some(true));
    assert_eq!(find_rule("nope"), None);
}

#[test]
fn groups_are_known_and_distinct_from_rules() {
    let mut seen = std::collections::HashSet::new();
    for g in GROUPS {
        assert!(seen.insert(g.id), "{} listed twice", g.id);
        assert!(g.id.chars().all(|c| c.is_ascii_lowercase() || c == '-'), "{}", g.id);
        assert!(find_rule(g.id).is_none(), "{} is both a group and a rule", g.id);
        assert!(RULES.iter().any(|r| r.groups.contains(&g.id)), "{} has no rules", g.id);
    }
    for r in RULES {
        for g in r.groups {
            assert!(find_group(g).is_some(), "{} names unknown group {g}", r.id);
        }
    }
    assert_eq!(find_rule(rule::IMPLICIT_SIGNATURE).unwrap().groups, &[group::PURE]);
}
