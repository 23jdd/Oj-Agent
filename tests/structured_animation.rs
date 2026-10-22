use oj_agent::{chat::animation::compile_verified, ir::AnimationIr, renderer::SkiaRenderer};
use oj_sandbox::{Cell, Edge, Node, Pointer, Report, Step};
use serde_json::json;
use std::sync::atomic::AtomicBool;

fn node(id: &str, label: &str, value: i64, state: &str) -> Node {
    Node {
        id: id.into(),
        label: label.into(),
        value: Some(value),
        state: state.into(),
        x: None,
        y: None,
        parent: None,
    }
}
fn edge(from: &str, to: &str, label: &str, directed: bool) -> Edge {
    Edge {
        from: from.into(),
        to: to.into(),
        label: label.into(),
        directed,
    }
}
fn step(
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    pointers: Vec<Pointer>,
    cells: Vec<Cell>,
    explanation: &str,
) -> Step {
    Step {
        values: vec![],
        indices: vec![],
        explanation: explanation.into(),
        state: String::new(),
        nodes,
        edges,
        pointers,
        cells,
    }
}
fn report(steps: Vec<Step>) -> Report {
    let cases = ["normal", "boundary", "adversarial"]
        .into_iter()
        .enumerate()
        .map(|(index, category)| oj_sandbox::CaseResult {
            name: category.into(),
            category: category.into(),
            input: json!(index),
            expected: json!({"ok": true}),
            actual: json!({"ok": true}),
            passed: true,
            steps: steps.clone(),
            error: None,
        })
        .collect();
    Report {
        language: "javascript".into(),
        cases,
        passed: true,
    }
}
fn compile(steps: Vec<Step>, layout: &str) -> AnimationIr {
    let bundle = compile_verified(
        &report(steps),
        "verification",
        "structured test",
        "Structured animation",
        layout,
        &AtomicBool::new(false),
    )
    .unwrap();
    bundle.cases[0].animation.clone()
}
fn assert_frame_is_painted(animation: &AnimationIr) {
    let prepared = SkiaRenderer.prepare(animation).unwrap();
    let frame = prepared.frame(animation.duration.max(0.32)).unwrap();
    assert_eq!(frame.rgba.len(), 960 * 720 * 4);
    assert!(
        frame
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| *pixel != [245, 245, 245, 255] && *pixel != [255, 255, 255, 255])
    );
}

#[test]
fn tree_layout_uses_explicit_nodes_edges_and_pointers() {
    let nodes = vec![
        node("root", "A", 1, "active"),
        node("left", "B", 2, "visited"),
        node("right", "C", 3, "normal"),
        node("leaf", "D", 4, "normal"),
    ];
    let edges = vec![
        edge("root", "left", "L", true),
        edge("root", "right", "R", false),
        edge("left", "leaf", "", false),
    ];
    let pointers = vec![Pointer {
        name: "cur".into(),
        target: "left".into(),
    }];
    let animation = compile(
        vec![
            step(
                nodes.clone(),
                edges.clone(),
                pointers.clone(),
                vec![],
                "visit root",
            ),
            step(
                vec![
                    node("root", "A", 1, "done"),
                    node("left", "B", 2, "active"),
                    node("right", "C", 3, "normal"),
                    node("leaf", "D", 4, "normal"),
                ],
                edges,
                vec![Pointer {
                    name: "cur".into(),
                    target: "leaf".into(),
                }],
                vec![],
                "visit leaf",
            ),
        ],
        "tree",
    );
    let object = animation
        .scene
        .objects
        .iter()
        .find(|object| object.id == "values_view")
        .unwrap();
    assert_eq!(object.kind, "tree");
    assert_eq!(object.values, [1, 2, 3, 4]);
    assert_eq!(object.labels, ["A", "B", "C", "D"]);
    assert_eq!(object.edges.len(), 3);
    assert_eq!(object.pointers[0].name, "cur");
    assert!(object.directed);
    let value = serde_json::to_value(&animation).unwrap();
    oj_agent::ir::parse_validated::<AnimationIr>(&value).unwrap();
    assert_frame_is_painted(&animation);
}

#[test]
fn graph_layout_supports_directed_edges_positions_and_edge_labels() {
    let nodes = vec![
        Node {
            x: Some(0.1),
            y: Some(0.5),
            ..node("s", "S", 0, "current")
        },
        Node {
            x: Some(0.5),
            y: Some(0.2),
            ..node("a", "A", 1, "normal")
        },
        Node {
            x: Some(0.8),
            y: Some(0.8),
            ..node("b", "B", 2, "normal")
        },
    ];
    let edges = vec![edge("s", "a", "4", true), edge("s", "b", "7", true)];
    let animation = compile(
        vec![step(
            nodes,
            edges,
            vec![Pointer {
                name: "path".into(),
                target: "a".into(),
            }],
            vec![],
            "relax edges",
        )],
        "graph",
    );
    let object = &animation.scene.objects[2];
    assert_eq!(object.kind, "graph");
    assert_eq!(object.edge_labels, ["4", "7"]);
    assert_eq!(object.positions.len(), 3);
    assert!(object.directed);
    assert_frame_is_painted(&animation);
}

#[test]
fn grid_layout_uses_true_rows_columns_dependencies_and_cell_pointers() {
    let cells = vec![
        Cell {
            row: 0,
            col: 0,
            value: 1,
            label: "dp".into(),
            state: "done".into(),
        },
        Cell {
            row: 0,
            col: 1,
            value: 2,
            label: String::new(),
            state: "normal".into(),
        },
        Cell {
            row: 1,
            col: 0,
            value: 3,
            label: String::new(),
            state: "normal".into(),
        },
        Cell {
            row: 1,
            col: 1,
            value: 6,
            label: String::new(),
            state: "active".into(),
        },
    ];
    let edges = vec![edge("0,0", "1,1", "depends", true)];
    let pointers = vec![Pointer {
        name: "cell".into(),
        target: "1,1".into(),
    }];
    let animation = compile(
        vec![step(vec![], edges, pointers, cells, "fill table")],
        "grid",
    );
    let object = animation
        .scene
        .objects
        .iter()
        .find(|object| object.id == "values_view")
        .unwrap();
    assert_eq!(object.kind, "grid");
    assert_eq!((object.rows, object.cols), (2, 2));
    assert_eq!(object.values, [1, 2, 3, 6]);
    assert_eq!(object.labels[0], "dp");
    assert_eq!(object.states[3], "active");
    assert_eq!(object.edges, [[0, 3]]);
    assert_eq!(object.pointers[0].target, 3);
    assert_frame_is_painted(&animation);
}

#[test]
fn linked_list_layout_tracks_identity_and_next_edges() {
    let nodes = vec![
        node("a", "A", 10, "done"),
        node("b", "B", 20, "active"),
        node("c", "C", 30, "normal"),
    ];
    let edges = vec![edge("a", "b", "", true), edge("b", "c", "", true)];
    let animation = compile(
        vec![step(
            nodes,
            edges,
            vec![Pointer {
                name: "cur".into(),
                target: "b".into(),
            }],
            vec![],
            "advance",
        )],
        "linked-list",
    );
    let object = &animation.scene.objects[2];
    assert_eq!(object.kind, "linked-list");
    assert_eq!(object.values, [10, 20, 30]);
    assert_eq!(object.edges, [[0, 1], [1, 2]]);
    assert_frame_is_painted(&animation);
}

#[test]
fn array_layout_tracks_two_pointers_and_highlights() {
    let steps = vec![
        Step {
            values: vec![1, 3, 5, 7],
            indices: vec![0, 3],
            explanation: "left and right".into(),
            state: "lo=0 hi=3".into(),
            nodes: vec![],
            edges: vec![],
            pointers: vec![
                Pointer {
                    name: "left".into(),
                    target: "0".into(),
                },
                Pointer {
                    name: "right".into(),
                    target: "3".into(),
                },
            ],
            cells: vec![],
        },
        Step {
            values: vec![1, 3, 5, 7],
            indices: vec![1, 2],
            explanation: "narrow range".into(),
            state: "lo=1 hi=2".into(),
            nodes: vec![],
            edges: vec![],
            pointers: vec![
                Pointer {
                    name: "left".into(),
                    target: "1".into(),
                },
                Pointer {
                    name: "right".into(),
                    target: "2".into(),
                },
            ],
            cells: vec![],
        },
    ];
    let animation = compile(steps, "array");
    let object = &animation.scene.objects[2];
    assert_eq!(object.kind, "array");
    assert_eq!(object.values, [1, 3, 5, 7]);
    assert_eq!(object.pointers.len(), 2);
    assert_eq!(object.pointers[0].target, 0);
    assert_eq!(object.pointers[1].target, 3);
    assert_frame_is_painted(&animation);
}

#[test]
fn sandbox_observations_compile_end_to_end_without_hand_built_ir() {
    let source = r#"function solve(input, emit) {
        const nodes = input.map((value,index)=>({id:"n"+index,label:"n"+index,value,state:index===0?"active":"normal"}));
        const edges = nodes.slice(1).map((_,index)=>({from:"n"+index,to:"n"+(index+1),directed:true,label:"next"}));
        emit({nodes, edges, pointers: nodes.length ? [{name:"cur",target:"n0"}] : [], explanation:"observed list"});
        return nodes.length;
    }"#;
    let cases = [(vec![1, 2, 3], 3), (vec![7], 1), (vec![], 0)]
        .into_iter()
        .map(|(input, expected)| oj_sandbox::Case {
            name: format!("case-{expected}"),
            category: if expected == 3 {
                "normal".into()
            } else if expected == 1 {
                "boundary".into()
            } else {
                "adversarial".into()
            },
            input: json!(input),
            expected: json!(expected),
        })
        .collect();
    let report = oj_sandbox::verify(
        &oj_sandbox::Request {
            source: source.into(),
            cases,
        },
        std::sync::Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    assert!(report.passed);
    let bundle = compile_verified(
        &report,
        "verification",
        "sandbox observation",
        "Observed list",
        "linked-list",
        &AtomicBool::new(false),
    )
    .unwrap();
    let object = &bundle.cases[0].animation.scene.objects[2];
    assert_eq!(object.kind, "linked-list");
    assert_eq!(object.values, [1, 2, 3]);
    assert_eq!(object.edges, [[0, 1], [1, 2]]);
    assert_eq!(object.edge_labels, ["next", "next"]);
    assert_eq!(object.pointers[0].target, 0);
    assert_frame_is_painted(&bundle.cases[0].animation);
}

#[test]
fn layouts_reject_traces_with_the_wrong_structure() {
    let values_only = Step {
        values: vec![1, 2, 3],
        indices: vec![0],
        explanation: "array".into(),
        state: String::new(),
        nodes: vec![],
        edges: vec![],
        pointers: vec![],
        cells: vec![],
    };
    let result = compile_verified(
        &report(vec![values_only]),
        "verification",
        "test",
        "Wrong layout",
        "tree",
        &AtomicBool::new(false),
    );
    assert!(result.is_err());

    let dangling = step(
        vec![node("a", "A", 1, "normal")],
        vec![],
        vec![Pointer {
            name: "bad".into(),
            target: "missing".into(),
        }],
        vec![],
        "bad pointer",
    );
    let result = compile_verified(
        &report(vec![dangling]),
        "verification",
        "test",
        "Bad pointer",
        "tree",
        &AtomicBool::new(false),
    );
    assert!(result.is_err());
}
