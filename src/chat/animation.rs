use super::{AnimationBundle, AnimationCase};
use crate::{
    agent::check_cancelled,
    error::{AppError, Result},
    ir::*,
    renderer::SkiaRenderer,
};
use oj_sandbox::Step;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::atomic::AtomicBool,
};

const LAYOUTS: &[&str] = &[
    "array",
    "table",
    "stack",
    "queue",
    "tree",
    "graph",
    "linked-list",
    "grid",
];

struct Frame {
    values: Vec<i64>,
    structure: SceneStructure,
}

struct Frames {
    kind: &'static str,
    rows: u32,
    cols: u32,
    frames: Vec<Frame>,
}

pub fn compile_verified(
    report: &oj_sandbox::Report,
    id: &str,
    basis: &str,
    title: &str,
    layout: &str,
    cancel: &AtomicBool,
) -> Result<AnimationBundle> {
    if !report.passed
        || report
            .cases
            .iter()
            .any(|c| !c.passed || c.error.is_some() || c.actual != c.expected)
        || report
            .cases
            .iter()
            .map(|c| c.input.to_string())
            .collect::<BTreeSet<_>>()
            .len()
            < 3
        || report.cases.len() < 3
        || !["normal", "boundary", "adversarial"]
            .iter()
            .all(|category| report.cases.iter().any(|c| c.category == *category))
    {
        return Err(AppError::validation(
            "AnimationVerification",
            "Animation needs passed normal, boundary and adversarial cases (at least three); fix failed checks first.",
        ));
    }
    if title.is_empty() || title.len() > 160 || !LAYOUTS.contains(&layout) {
        return Err(AppError::validation(
            "AnimationVerification",
            format!(
                "Provide a short title and one layout: {}.",
                LAYOUTS.join("/")
            ),
        ));
    }
    let mut cases = Vec::new();
    for case in &report.cases {
        check_cancelled(cancel)?;
        if case.steps.is_empty() || case.steps.len() > 256 {
            return Err(AppError::validation(
                "AnimationVerification",
                format!(
                    "{}: emit 1–256 execution steps, including for empty input.",
                    case.name
                ),
            ));
        }
        let frames = build_frames(&case.steps, layout)?;
        let first = frames
            .frames
            .first()
            .ok_or_else(|| AppError::validation("AnimationVerification", "No execution frame."))?;
        let kind = frames.kind;
        let image_object = SceneObject {
            id: "values_view".into(),
            kind: kind.into(),
            x: 32.,
            y: 112.,
            width: 896.,
            height: 336.,
            text: String::new(),
            values: first.values.clone(),
            state: "normal".into(),
            edges: first.structure.edges.clone(),
            directed: any_directed(&case.steps),
            edge_labels: first.structure.edge_labels.clone(),
            labels: first.structure.labels.clone(),
            states: first.structure.states.clone(),
            positions: first.structure.positions.clone(),
            pointers: first.structure.pointers.clone(),
            parents: first.structure.parents.clone(),
            rows: frames.rows,
            cols: frames.cols,
        };
        let scene = Scene {
            width: 960,
            height: 720,
            objects: vec![
                text_object("title", 32., 40., title.into()),
                text_object(
                    "subtitle",
                    80.,
                    20.,
                    format!(
                        "{} / {} / {} / 已通过提供的断言",
                        case.name,
                        case.category,
                        kind.to_uppercase()
                    ),
                ),
                image_object,
                text_object("explanation", 472., 72., String::new()),
                text_object("state_view", 568., 120., String::new()),
            ],
        };
        let mut timeline = Vec::new();
        let mut step_times = Vec::new();
        for (index, (step, frame)) in case.steps.iter().zip(&frames.frames).enumerate() {
            if step.values.len() > oj_sandbox::MAX_VALUES {
                return Err(AppError::validation(
                    "AnimationVerification",
                    "Trace contains too many values.",
                ));
            }
            let at = index as f64 * 0.8;
            step_times.push(at + 0.32);
            timeline.push(AnimationAction {
                at,
                duration: 0.32,
                action: Action::UpdateValue,
                target: "values_view".into(),
                text: String::new(),
                values: frame.values.clone(),
                indices: vec![],
                structure: Some(frame.structure.clone()),
            });
            let active =
                if let Some(indices) = step_indices(step, &frame.structure, kind, frames.cols) {
                    indices
                } else {
                    frame
                        .structure
                        .states
                        .iter()
                        .enumerate()
                        .filter(|(_, state)| {
                            matches!(state.as_str(), "active" | "current" | "visiting")
                        })
                        .map(|(index, _)| index)
                        .collect()
                };
            if !active.is_empty() {
                timeline.push(AnimationAction {
                    at,
                    duration: 0.32,
                    action: Action::Highlight,
                    target: "values_view".into(),
                    text: String::new(),
                    values: vec![],
                    indices: active,
                    structure: None,
                });
            }
            timeline.push(text_action(
                at,
                "explanation",
                format!(
                    "{:02} / {:02}  {}",
                    index + 1,
                    case.steps.len(),
                    step.explanation
                ),
            ));
            timeline.push(text_action(
                at,
                "state_view",
                if index + 1 == case.steps.len() {
                    format!("{}\n返回值：{}", step.state, case.actual)
                } else {
                    step.state.clone()
                },
            ));
        }
        let animation = AnimationIr {
            header: Header {
                version: 2,
                ..Header::new("animation-ir", &uuid::Uuid::new_v4().to_string())
            },
            scene,
            timeline,
            duration: case.steps.len() as f64 * 0.8,
            step_times,
        };
        animation.validate()?;
        let prepared = SkiaRenderer.prepare(&animation)?;
        for (index, frame) in frames.frames.iter().enumerate() {
            let at = animation.step_times[index];
            let track = prepared
                .timeline
                .objects
                .iter()
                .find(|o| o.object.id == "values_view")
                .expect("value track");
            let sample = track.sample(at);
            if sample.values != frame.values {
                return Err(AppError::validation(
                    "AnimationVerification",
                    "Compiled values differ from the observed trace.",
                ));
            }
            if sample.structure.edges.len() != frame.structure.edges.len()
                || sample.structure.pointers.len() != frame.structure.pointers.len()
            {
                return Err(AppError::validation(
                    "AnimationVerification",
                    "Compiled structure differs from the observed trace.",
                ));
            }
            for time in [at - 0.16, at] {
                prepared.timeline.clamp_time(time)?;
            }
        }
        for time in [0., 0.16, animation.duration / 2., animation.duration] {
            check_cancelled(cancel)?;
            let frame = prepared.frame(time)?;
            if frame.rgba.len() != 960 * 720 * 4
                || frame
                    .rgba
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .all(|pixel| *pixel == [255, 255, 255, 255])
            {
                return Err(AppError::validation(
                    "AnimationVerification",
                    "Raster frame is incomplete.",
                ));
            }
        }
        cases.push(AnimationCase {
            name: case.name.clone(),
            category: case.category.clone(),
            animation,
        });
    }
    Ok(AnimationBundle {
        title: title.into(),
        basis: basis.into(),
        verification_id: id.into(),
        cases,
    })
}

fn text_object(id: &str, y: f32, height: f32, text: String) -> SceneObject {
    SceneObject {
        id: id.into(),
        kind: "text".into(),
        x: 32.,
        y,
        width: 896.,
        height,
        text,
        values: vec![],
        state: "normal".into(),
        edges: vec![],
        directed: false,
        edge_labels: vec![],
        labels: vec![],
        states: vec![],
        positions: vec![],
        pointers: vec![],
        parents: vec![],
        rows: 0,
        cols: 0,
    }
}
fn text_action(at: f64, target: &str, text: String) -> AnimationAction {
    AnimationAction {
        at,
        duration: 0.32,
        action: Action::UpdateText,
        target: target.into(),
        text,
        values: vec![],
        indices: vec![],
        structure: None,
    }
}
fn any_directed(steps: &[Step]) -> bool {
    steps
        .iter()
        .flat_map(|step| &step.edges)
        .any(|edge| edge.directed)
}
fn build_frames(steps: &[Step], layout: &str) -> Result<Frames> {
    let has_nodes = steps.iter().any(|step| !step.nodes.is_empty());
    let has_cells = steps.iter().any(|step| !step.cells.is_empty());
    let has_values = steps.iter().any(|step| !step.values.is_empty());
    if has_nodes && has_cells {
        return Err(AppError::validation(
            "AnimationVerification",
            "Emit nodes or cells for one animation, not both.",
        ));
    }
    let kind = if layout == "table" && has_cells {
        "grid"
    } else {
        layout
    };
    match kind {
        "tree" | "graph" | "linked-list" => {
            if has_cells || (has_values && !has_nodes) {
                return Err(AppError::validation(
                    "AnimationVerification",
                    "Tree, graph and linked-list layouts need nodes and edges, not a values-only trace.",
                ));
            }
        }
        "grid" => {
            if has_nodes || (has_values && !has_cells) {
                return Err(AppError::validation(
                    "AnimationVerification",
                    "Grid layouts need row/col cells, not a values-only trace.",
                ));
            }
        }
        "array" | "table" | "stack" | "queue" => {
            if has_nodes || has_cells {
                return Err(AppError::validation(
                    "AnimationVerification",
                    "Array, table, stack and queue layouts need values, not nodes or cells.",
                ));
            }
        }
        _ => unreachable!("layout was validated"),
    }
    let (rows, cols) = if kind == "grid" {
        let rows = steps
            .iter()
            .flat_map(|step| &step.cells)
            .map(|cell| cell.row + 1)
            .max()
            .unwrap_or(0);
        let cols = steps
            .iter()
            .flat_map(|step| &step.cells)
            .map(|cell| cell.col + 1)
            .max()
            .unwrap_or(0);
        (rows as u32, cols as u32)
    } else {
        (0, 0)
    };
    let node_ids = if matches!(kind, "tree" | "graph" | "linked-list") {
        let mut seen = BTreeSet::new();
        let mut ids = Vec::new();
        for step in steps {
            for node in &step.nodes {
                if seen.insert(node.id.clone()) {
                    ids.push(node.id.clone());
                }
            }
        }
        ids
    } else {
        vec![]
    };
    let node_index: BTreeMap<&str, usize> = node_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect();
    let mut frames = Vec::with_capacity(steps.len());
    for step in steps {
        if matches!(kind, "tree" | "graph" | "linked-list")
            && !step.values.is_empty()
            && step.nodes.is_empty()
        {
            return Err(AppError::validation(
                "AnimationVerification",
                "A node/graph step cannot contain only values.",
            ));
        }
        if kind == "grid" && !step.values.is_empty() && step.cells.is_empty() {
            return Err(AppError::validation(
                "AnimationVerification",
                "A grid step cannot contain only values.",
            ));
        }
        oj_sandbox::validate_step(step).map_err(|error| {
            AppError::validation(
                "AnimationVerification",
                format!("Invalid execution step: {error}"),
            )
        })?;
        let frame = match kind {
            "tree" | "graph" | "linked-list" => graph_frame(step, &node_ids, &node_index)?,
            "grid" => grid_frame(step, rows as usize, cols as usize)?,
            _ => Frame {
                values: step.values.clone(),
                structure: SceneStructure {
                    pointers: map_index_pointers(&step.pointers, step.values.len())?,
                    ..SceneStructure::default()
                },
            },
        };
        frames.push(frame);
    }
    Ok(Frames {
        kind: kind_of(kind),
        rows,
        cols,
        frames,
    })
}
fn kind_of(kind: &str) -> &'static str {
    match kind {
        "tree" => "tree",
        "graph" => "graph",
        "linked-list" => "linked-list",
        "grid" => "grid",
        "table" => "table",
        "stack" => "stack",
        "queue" => "queue",
        _ => "array",
    }
}
fn graph_frame(
    step: &Step,
    node_ids: &[String],
    node_index: &BTreeMap<&str, usize>,
) -> Result<Frame> {
    let mut values = vec![0; node_ids.len()];
    let mut labels = vec![String::new(); node_ids.len()];
    let mut states = vec!["hidden".to_string(); node_ids.len()];
    let mut positions = vec![[0.5f32, 0.5f32]; node_ids.len()];
    let has_positions = step
        .nodes
        .iter()
        .any(|node| node.x.is_some() || node.y.is_some());
    for node in &step.nodes {
        let index = *node_index
            .get(node.id.as_str())
            .ok_or_else(|| AppError::validation("AnimationVerification", "Unknown node."))?;
        values[index] = node.value.unwrap_or(0);
        labels[index] = if node.label.is_empty() {
            node.value
                .map(|value| value.to_string())
                .unwrap_or_else(|| node.id.clone())
        } else {
            node.label.clone()
        };
        states[index] = if node.state.is_empty() {
            "normal".into()
        } else {
            node.state.clone()
        };
        if let (Some(x), Some(y)) = (node.x, node.y) {
            positions[index] = [x, y];
        }
    }
    let mut edges = Vec::new();
    let mut edge_labels = Vec::new();
    let mut seen = BTreeSet::new();
    for edge in &step.edges {
        let from = *node_index
            .get(edge.from.as_str())
            .ok_or_else(|| AppError::validation("AnimationVerification", "Unknown edge source."))?;
        let to = *node_index
            .get(edge.to.as_str())
            .ok_or_else(|| AppError::validation("AnimationVerification", "Unknown edge target."))?;
        if seen.insert((from, to)) {
            edges.push([from, to]);
            edge_labels.push(edge.label.clone());
        }
    }
    let mut parents = Vec::new();
    for node in &step.nodes {
        if let Some(parent) = &node.parent {
            let from = *node_index.get(parent.as_str()).ok_or_else(|| {
                AppError::validation("AnimationVerification", "Unknown parent node.")
            })?;
            let to = *node_index.get(node.id.as_str()).ok_or_else(|| {
                AppError::validation("AnimationVerification", "Unknown child node.")
            })?;
            if parents.is_empty() {
                parents = vec![None; node_ids.len()];
            }
            parents[to] = Some(from);
            if seen.insert((from, to)) {
                edges.push([from, to]);
                edge_labels.push(String::new());
            }
        }
    }
    let pointers = step
        .pointers
        .iter()
        .map(|pointer| {
            let target = node_index
                .get(pointer.target.as_str())
                .copied()
                .or_else(|| {
                    pointer
                        .target
                        .parse::<usize>()
                        .ok()
                        .filter(|index| *index < node_ids.len())
                })
                .ok_or_else(|| {
                    AppError::validation(
                        "AnimationVerification",
                        format!("Pointer target is not a node: {}", pointer.target),
                    )
                })?;
            Ok(ScenePointer {
                name: pointer.name.clone(),
                target,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Frame {
        values,
        structure: SceneStructure {
            edges,
            edge_labels,
            labels,
            states,
            positions: if has_positions { positions } else { vec![] },
            pointers,
            parents,
        },
    })
}
fn grid_frame(step: &Step, rows: usize, cols: usize) -> Result<Frame> {
    let count = rows.saturating_mul(cols);
    let mut values = vec![0; count];
    let mut labels = vec![String::new(); count];
    let mut states = vec!["empty".to_string(); count];
    for cell in &step.cells {
        if cell.row >= rows || cell.col >= cols {
            return Err(AppError::validation(
                "AnimationVerification",
                "Grid cell is outside the declared dimensions.",
            ));
        }
        let index = cell.row * cols + cell.col;
        values[index] = cell.value;
        labels[index] = cell.label.clone();
        states[index] = if cell.state.is_empty() {
            "normal".into()
        } else {
            cell.state.clone()
        };
    }
    let mut edges = Vec::new();
    let mut edge_labels = Vec::new();
    for edge in &step.edges {
        let from = grid_target(&edge.from, rows, cols).ok_or_else(|| {
            AppError::validation("AnimationVerification", "Invalid grid edge source.")
        })?;
        let to = grid_target(&edge.to, rows, cols).ok_or_else(|| {
            AppError::validation("AnimationVerification", "Invalid grid edge target.")
        })?;
        edges.push([from, to]);
        edge_labels.push(edge.label.clone());
    }
    let pointers = step
        .pointers
        .iter()
        .map(|pointer| {
            let target = grid_target(&pointer.target, rows, cols).ok_or_else(|| {
                AppError::validation(
                    "AnimationVerification",
                    format!("Pointer target is not a grid cell: {}", pointer.target),
                )
            })?;
            Ok(ScenePointer {
                name: pointer.name.clone(),
                target,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Frame {
        values,
        structure: SceneStructure {
            edges,
            edge_labels,
            labels,
            states,
            pointers,
            ..SceneStructure::default()
        },
    })
}
fn grid_target(target: &str, rows: usize, cols: usize) -> Option<usize> {
    if let Some((row, col)) = target.split_once(',')
        && let (Ok(row), Ok(col)) = (row.parse::<usize>(), col.parse::<usize>())
        && row < rows
        && col < cols
    {
        return Some(row * cols + col);
    }
    target
        .parse::<usize>()
        .ok()
        .filter(|index| *index < rows * cols)
}
fn map_index_pointers(pointers: &[oj_sandbox::Pointer], len: usize) -> Result<Vec<ScenePointer>> {
    pointers
        .iter()
        .map(|pointer| {
            let target = pointer
                .target
                .parse::<usize>()
                .ok()
                .filter(|index| *index < len)
                .ok_or_else(|| {
                    AppError::validation(
                        "AnimationVerification",
                        format!("Pointer target is not a value index: {}", pointer.target),
                    )
                })?;
            Ok(ScenePointer {
                name: pointer.name.clone(),
                target,
            })
        })
        .collect()
}
fn step_indices(
    step: &Step,
    structure: &SceneStructure,
    kind: &str,
    cols: u32,
) -> Option<Vec<usize>> {
    if !step.indices.is_empty() {
        return Some(step.indices.clone());
    }
    if kind == "grid" {
        return Some(
            step.cells
                .iter()
                .filter(|cell| matches!(cell.state.as_str(), "active" | "current" | "visiting"))
                .map(|cell| cell.row * cols as usize + cell.col)
                .collect(),
        );
    }
    if matches!(kind, "tree" | "graph" | "linked-list") {
        return Some(
            structure
                .states
                .iter()
                .enumerate()
                .filter(|(_, state)| matches!(state.as_str(), "active" | "current" | "visiting"))
                .map(|(index, _)| index)
                .collect(),
        );
    }
    None
}
