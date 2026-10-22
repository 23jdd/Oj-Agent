use crate::{
    error::{AppError, Result},
    ir::*,
};
pub mod easing;
pub mod timeline;

pub fn compile(
    algorithm: &AlgorithmIr,
    trace: &TraceIr,
    visual: &VisualizationIr,
) -> Result<AnimationIr> {
    trace.validate(algorithm)?;
    visual.validate(algorithm)?;
    let count = trace
        .events
        .iter()
        .map(|e| e.values.len())
        .max()
        .unwrap_or(1);
    let mut initial = trace.events[0].values.clone();
    initial.resize(count, 0);
    let mut objects = vec![SceneObject {
        id: "title".into(),
        kind: "text".into(),
        x: 32.,
        y: 32.,
        width: 896.,
        height: 40.,
        text: algorithm.name.clone(),
        values: vec![],
        state: "normal".into(),
        edges: vec![],
        directed: false,
    }];
    let kind = visual
        .objects
        .iter()
        .find(|o| o.id == "values_view")
        .map(|o| o.kind.strip_suffix("-view").unwrap_or(&o.kind))
        .unwrap_or("array");
    objects.push(SceneObject {
        id: "subtitle".into(),
        kind: "text".into(),
        x: 32.,
        y: 80.,
        width: 896.,
        height: 20.,
        text: format!("{} / {count} VALUES / STEP BY STEP", kind.to_uppercase()),
        values: vec![],
        state: "normal".into(),
        edges: vec![],
        directed: false,
    });
    for obj in &visual.objects {
        let (y, height) = match obj.id.as_str() {
            "values_view" => (112., 336.),
            "explanation" => (472., 72.),
            _ => (568., 120.),
        };
        objects.push(SceneObject {
            id: obj.id.clone(),
            kind: obj.kind.strip_suffix("-view").unwrap_or("text").into(),
            x: 32.,
            y,
            width: 896.,
            height,
            text: String::new(),
            values: if obj.kind != "text-view" {
                initial.clone()
            } else {
                vec![]
            },
            state: "normal".into(),
            edges: if obj.id == "values_view" {
                trace
                    .initial_state
                    .get("graph")
                    .and_then(|g| serde_json::from_value::<GraphInput>(g.clone()).ok())
                    .map(|g| g.edges)
                    .unwrap_or_default()
            } else {
                vec![]
            },
            directed: trace
                .initial_state
                .get("graph")
                .and_then(|g| g["directed"].as_bool())
                .unwrap_or(false),
        });
    }
    if !objects.iter().any(|o| o.id == "values_view")
        || !objects.iter().any(|o| o.id == "explanation")
        || !objects.iter().any(|o| o.id == "state_view")
    {
        return Err(AppError::validation(
            "AnimationCompilation",
            "Template requires values_view, explanation and state_view.",
        ));
    }
    let mut timeline = vec![];
    let mut steps = vec![];
    for event in &trace.events {
        let at = event.seq as f64 * 0.8;
        // Stepping pauses on the completed state rather than the beginning of its transition.
        steps.push(at + 0.32);
        let primary = visual
            .objects
            .iter()
            .find(|o| o.id == "values_view")
            .map(|o| o.kind.as_str())
            .unwrap_or("array-view");
        let values = if ["stack-view", "queue-view"].contains(&primary) {
            event
                .state
                .get("container")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default()
        } else {
            event.values.clone()
        };
        for (action, target, text, values, indices) in [
            (
                Action::UpdateValue,
                "values_view",
                String::new(),
                values,
                vec![],
            ),
            (
                Action::Highlight,
                "values_view",
                String::new(),
                vec![],
                event.indices.clone(),
            ),
            (
                Action::UpdateText,
                "explanation",
                format!(
                    "{:02} / {:02}   {}{}",
                    event.seq + 1,
                    trace.events.len(),
                    event
                        .source
                        .as_ref()
                        .map(|s| format!("{}:{} · ", s.file, s.line))
                        .unwrap_or_default(),
                    event.explanation
                ),
                vec![],
                vec![],
            ),
            (
                Action::UpdateText,
                "state_view",
                event
                    .state
                    .iter()
                    .map(|(k, v)| format!("{k}: {v}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
                vec![],
                vec![],
            ),
        ] {
            timeline.push(AnimationAction {
                at,
                duration: 0.32,
                action,
                target: target.into(),
                indices,
                text,
                values,
            });
        }
    }
    let animation = AnimationIr {
        header: Header {
            schema: "animation-ir".into(),
            version: 2,
            id: "animation_main".into(),
        },
        scene: Scene {
            width: 960,
            height: 720,
            objects,
        },
        timeline,
        duration: steps.len() as f64 * 0.8,
        step_times: steps,
    };
    animation.validate()?;
    Ok(animation)
}

#[derive(Debug, Clone)]
pub struct Player {
    pub time: f64,
    pub speed: f64,
    pub playing: bool,
}
impl Default for Player {
    fn default() -> Self {
        Self {
            time: 0.,
            speed: 1.,
            playing: false,
        }
    }
}
impl Player {
    pub fn seek(&mut self, time: f64, animation: &AnimationIr) {
        if time.is_finite() {
            self.time = time.clamp(0., animation.duration);
        }
    }
    pub fn tick(&mut self, elapsed: f64, animation: &AnimationIr) {
        if self.playing && elapsed.is_finite() && elapsed >= 0. {
            self.seek(self.time + elapsed * self.speed, animation);
            if self.time >= animation.duration {
                self.playing = false;
            }
        }
    }
    pub fn step(&mut self, forward: bool, animation: &AnimationIr) {
        self.playing = false;
        let next = if forward {
            animation
                .step_times
                .iter()
                .copied()
                .find(|t| *t > self.time + 0.001)
                .unwrap_or(animation.duration)
        } else {
            animation
                .step_times
                .iter()
                .copied()
                .rev()
                .find(|t| *t < self.time - 0.001)
                .unwrap_or(0.)
        };
        self.seek(next, animation);
    }
}
