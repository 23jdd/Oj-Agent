use crate::{error::Result, ir::*};

pub fn plan(algorithm: &AlgorithmIr, trace: &TraceIr) -> Result<VisualizationIr> {
    trace.validate(algorithm)?;
    let ir = VisualizationIr {
        header: Header::new("visualization-ir", "visualization_main"),
        template: format!("{:?}", algorithm.category),
        objects: vec![
            VisualObject {
                id: "values_view".into(),
                kind: if trace.initial_state.contains_key("graph") {
                    if trace.initial_state["layout"] == "tree" {
                        "tree-view"
                    } else {
                        "graph-view"
                    }
                } else {
                    match algorithm.category {
                        Algorithm::Fibonacci => "table-view",
                        Algorithm::Stack => "stack-view",
                        Algorithm::Queue => "queue-view",
                        _ => "array-view",
                    }
                }
                .into(),
                bind: "values".into(),
                placement: "center".into(),
            },
            VisualObject {
                id: "explanation".into(),
                kind: "text-view".into(),
                bind: "state".into(),
                placement: "below".into(),
            },
            VisualObject {
                id: "state_view".into(),
                kind: "text-view".into(),
                bind: "state".into(),
                placement: "below".into(),
            },
        ],
    };
    ir.validate(algorithm)?;
    Ok(ir)
}
