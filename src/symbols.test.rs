use super::*;

fn reference(namespace: &str, qualifier: Option<&str>) -> Occurrence {
    Occurrence {
        name: "C".into(),
        line: 1,
        col: 1,
        end_col: 2,
        kind: SymbolKind::Class,
        declaration: false,
        scope: Scope::Project,
        namespace: namespace.into(),
        qualifier: qualifier.map(String::from),
    }
}

#[test]
fn lookup_goes_from_the_current_namespace_outwards() {
    assert_eq!(reference("A::B", None).lookup_chain(), vec!["A::B", "A", ""]);
    assert_eq!(reference("A::B", Some("X")).lookup_chain(), vec!["A::B::X", "A::X", "X"]);
    assert_eq!(reference("", None).lookup_chain(), vec![""]);
    assert_eq!(reference("A", Some("::X")).lookup_chain(), vec!["X"]);
    assert_eq!(reference("A", Some("::")).lookup_chain(), vec![""]);
}

#[test]
fn resolution_takes_the_first_namespace_that_declares_the_name() {
    let r = reference("A::B", None);
    assert_eq!(r.resolve(|ns| ns == "A" || ns.is_empty()), "A");
    assert_eq!(r.resolve(|ns| ns == "A::B"), "A::B");
    // Declared nowhere: the global namespace.
    assert_eq!(r.resolve(|_| false), "");
    assert_eq!(reference("A", Some("X")).resolve(|_| false), "X");
    // A declaration is where it is.
    let mut d = reference("A", None);
    d.declaration = true;
    assert_eq!(d.resolve(|_| false), "A");
}
