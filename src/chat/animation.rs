use super::{AnimationBundle, AnimationCase};
use crate::{
    agent::check_cancelled,
    error::{AppError, Result},
    ir::*,
    renderer::SkiaRenderer,
};
use std::sync::atomic::AtomicBool;

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
            .collect::<std::collections::BTreeSet<_>>()
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
    if title.is_empty()
        || title.len() > 160
        || !["array", "table", "stack", "queue"].contains(&layout)
    {
        return Err(AppError::validation(
            "AnimationVerification",
            "Provide a short title and an array/table/stack/queue layout.",
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
        let text = |id: &str, y, height, value: String| SceneObject {
            id: id.into(),
            kind: "text".into(),
            x: 32.,
            y,
            width: 896.,
            height,
            text: value,
            values: vec![],
            state: "normal".into(),
            edges: vec![],
            directed: false,
        };
        let scene = Scene {
            width: 960,
            height: 720,
            objects: vec![
                text("title", 32., 40., title.into()),
                text(
                    "subtitle",
                    80.,
                    20.,
                    format!("{} / {} / 已通过提供的断言", case.name, case.category),
                ),
                SceneObject {
                    id: "values_view".into(),
                    kind: layout.into(),
                    x: 32.,
                    y: 112.,
                    width: 896.,
                    height: 336.,
                    text: String::new(),
                    values: case.steps[0].values.clone(),
                    state: "normal".into(),
                    edges: vec![],
                    directed: false,
                },
                text("explanation", 472., 72., String::new()),
                text("state_view", 568., 120., String::new()),
            ],
        };
        let mut timeline = Vec::new();
        let mut step_times = Vec::new();
        for (index, step) in case.steps.iter().enumerate() {
            if step.values.len() > 64 || step.indices.iter().any(|i| *i >= step.values.len()) {
                return Err(AppError::validation(
                    "AnimationVerification",
                    "Trace indices are outside the current values.",
                ));
            }
            let at = index as f64 * 0.8;
            step_times.push(at + 0.32);
            for (action, target, text, values, indices) in [
                (
                    Action::UpdateValue,
                    "values_view",
                    String::new(),
                    step.values.clone(),
                    vec![],
                ),
                (
                    Action::Highlight,
                    "values_view",
                    String::new(),
                    vec![],
                    step.indices.clone(),
                ),
                (
                    Action::UpdateText,
                    "explanation",
                    format!(
                        "{:02} / {:02}  {}",
                        index + 1,
                        case.steps.len(),
                        step.explanation
                    ),
                    vec![],
                    vec![],
                ),
                (
                    Action::UpdateText,
                    "state_view",
                    if index + 1 == case.steps.len() {
                        format!("{}\n返回值：{}", step.state, case.actual)
                    } else {
                        step.state.clone()
                    },
                    vec![],
                    vec![],
                ),
            ] {
                timeline.push(AnimationAction {
                    at,
                    duration: 0.32,
                    action,
                    target: target.into(),
                    text,
                    values,
                    indices,
                });
            }
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
        // Check every completed state and transition midpoint; rasterize representative
        // frames for each case. These checks do not prove the chosen assertions complete.
        for (index, step) in case.steps.iter().enumerate() {
            let at = animation.step_times[index];
            let track = prepared
                .timeline
                .objects
                .iter()
                .find(|o| o.object.id == "values_view")
                .expect("value track");
            if track.sample(at).values != step.values {
                return Err(AppError::validation(
                    "AnimationVerification",
                    "Compiled values differ from the observed trace.",
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
        verification_id: id.into(),
        basis: basis.into(),
        cases,
    })
}
