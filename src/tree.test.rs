use super::*;

fn sample() -> Node {
    Node::with_children(
        "Root",
        vec![
            Node::with_children("A", vec![Node::leaf("A1"), Node::leaf("A\"2")]),
            Node::leaf("B"),
        ],
    )
}

#[test]
fn renders_box_drawing_tree() {
    assert_eq!(
        render_tree(&sample()),
        "Root\n├─ A\n│  ├─ A1\n│  └─ A\"2\n└─ B\n"
    );
}

#[test]
fn renders_dot_with_escaped_labels() {
    let dot = render_dot(&sample());
    assert!(dot.starts_with("digraph AST {\n"));
    assert!(dot.contains("n3 [label=\"A\\\"2\"];"));
    assert!(dot.contains("n1 -> n3;"));
    assert!(dot.contains("n0 -> n4;"));
}

#[test]
fn tag_sets_kind_and_span() {
    let n = Node::leaf("x").tag("expr", Some(Span::new(1, 2, 1, 3)));
    assert_eq!(n.kind, "expr");
    assert_eq!(n.span, Some(Span::new(1, 2, 1, 3)));
}
