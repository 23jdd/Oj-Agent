use super::easing::{lerp, progress};
use crate::{
    error::{AppError, Result},
    ir::*,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameKey {
    segment: usize,
    phase: u64,
}

struct Keyframe<T> {
    at: f64,
    duration: f64,
    from: T,
    to: T,
}
impl<T> Keyframe<T> {
    fn progress(&self, time: f64) -> f32 {
        progress(time, self.at, self.duration)
    }
}
fn current<T>(track: &[Keyframe<T>], time: f64) -> Option<&Keyframe<T>> {
    let index = track.partition_point(|key| key.at <= time);
    index.checked_sub(1).map(|i| &track[i])
}

pub struct ValueFrame {
    pub at: f64,
    pub duration: f64,
    pub from: Vec<i64>,
    pub to: Vec<i64>,
    /// Each destination owns at most one source, including duplicate values.
    pub origins: Vec<Option<usize>>,
}

pub struct ObjectTrack {
    pub object: SceneObject,
    initial_structure: SceneStructure,
    values: Vec<ValueFrame>,
    structure: Vec<Keyframe<SceneStructure>>,
    text: Vec<Keyframe<String>>,
    highlights: Vec<Keyframe<[f32; MAX_OBJECT_VALUES]>>,
    visibility: Vec<Keyframe<f32>>,
}

pub struct ObjectSample<'a> {
    pub object: &'a SceneObject,
    pub previous_values: &'a [i64],
    pub values: &'a [i64],
    pub origins: Option<&'a [Option<usize>]>,
    pub value_progress: f32,
    pub previous_structure: &'a SceneStructure,
    pub structure: &'a SceneStructure,
    pub structure_progress: f32,
    pub previous_text: &'a str,
    pub text: &'a str,
    pub text_progress: f32,
    pub highlights: [f32; MAX_OBJECT_VALUES],
    pub opacity: f32,
}

impl ObjectTrack {
    fn highlights_at(&self, time: f64) -> [f32; MAX_OBJECT_VALUES] {
        current(&self.highlights, time)
            .map(|key| {
                let p = key.progress(time);
                std::array::from_fn(|i| lerp(key.from[i], key.to[i], p))
            })
            .unwrap_or([0.; MAX_OBJECT_VALUES])
    }
    fn opacity_at(&self, time: f64) -> f32 {
        current(&self.visibility, time)
            .map(|key| lerp(key.from, key.to, key.progress(time)))
            .unwrap_or(1.)
    }
    fn structure_at(&self, time: f64) -> SceneStructure {
        current(&self.structure, time)
            .map(|key| key.to.clone())
            .unwrap_or_else(|| self.initial_structure.clone())
    }
    pub fn sample(&self, time: f64) -> ObjectSample<'_> {
        let value_index = self.values.partition_point(|key| key.at <= time);
        let values = value_index.checked_sub(1).map(|i| &self.values[i]);
        let structure = current(&self.structure, time);
        let text = current(&self.text, time);
        ObjectSample {
            object: &self.object,
            previous_values: values
                .map(|key| key.from.as_slice())
                .unwrap_or(&self.object.values),
            values: values
                .map(|key| key.to.as_slice())
                .unwrap_or(&self.object.values),
            origins: values.map(|key| key.origins.as_slice()),
            value_progress: values
                .map(|key| progress(time, key.at, key.duration))
                .unwrap_or(1.),
            previous_structure: structure
                .map(|key| &key.from)
                .unwrap_or(&self.initial_structure),
            structure: structure
                .map(|key| &key.to)
                .unwrap_or(&self.initial_structure),
            structure_progress: structure.map(|key| key.progress(time)).unwrap_or(1.),
            previous_text: text
                .map(|key| key.from.as_str())
                .unwrap_or(&self.object.text),
            text: text.map(|key| key.to.as_str()).unwrap_or(&self.object.text),
            text_progress: text.map(|key| key.progress(time)).unwrap_or(1.),
            highlights: self.highlights_at(time),
            opacity: self.opacity_at(time),
        }
    }
}

/// Compiled property tracks: sampling never depends on the previous frame or seek direction.
pub struct Timeline {
    pub width: u32,
    pub height: u32,
    pub duration: f64,
    pub objects: Vec<ObjectTrack>,
    boundaries: Vec<f64>,
    active_until: Vec<f64>,
}

impl Timeline {
    pub fn new(animation: &AnimationIr) -> Result<Self> {
        animation.validate()?;
        let mut objects: Vec<_> = animation
            .scene
            .objects
            .iter()
            .cloned()
            .map(|object| {
                let initial_structure = object.structure();
                ObjectTrack {
                    object,
                    initial_structure,
                    values: vec![],
                    structure: vec![],
                    text: vec![],
                    highlights: vec![],
                    visibility: vec![],
                }
            })
            .collect();
        let indices: BTreeMap<_, _> = objects
            .iter()
            .enumerate()
            .map(|(i, object)| (object.object.id.clone(), i))
            .collect();
        let mut boundaries = Vec::new();
        let mut active_until = Vec::new();
        let mut latest_end: f64 = 0.;
        for action in &animation.timeline {
            let track = &mut objects[indices[&action.target]];
            let key = |from, to| Keyframe {
                at: action.at,
                duration: action.duration,
                from,
                to,
            };
            match action.action {
                Action::UpdateValue => {
                    let from = track
                        .values
                        .last()
                        .map(|key| &key.to)
                        .unwrap_or(&track.object.values)
                        .clone();
                    let preserve_values =
                        matches!(track.object.kind.as_str(), "array" | "stack" | "queue");
                    let origins = origins(&from, &action.values, preserve_values);
                    track.values.push(ValueFrame {
                        at: action.at,
                        duration: action.duration,
                        from,
                        to: action.values.clone(),
                        origins,
                    });
                    if let Some(structure) = &action.structure {
                        let from = track.structure_at(action.at);
                        track.structure.push(Keyframe {
                            at: action.at,
                            duration: action.duration,
                            from,
                            to: structure.clone(),
                        });
                    }
                }
                Action::UpdateText => {
                    let from = track
                        .text
                        .last()
                        .map(|key| &key.to)
                        .unwrap_or(&track.object.text)
                        .clone();
                    track.text.push(Keyframe {
                        at: action.at,
                        duration: action.duration,
                        from,
                        to: action.text.clone(),
                    });
                }
                Action::Highlight => {
                    let mut to = [0.; MAX_OBJECT_VALUES];
                    for index in &action.indices {
                        to[*index] = 1.;
                    }
                    let from = track.highlights_at(action.at);
                    track.highlights.push(key(from, to));
                }
                Action::Show | Action::Hide => {
                    let from = track.opacity_at(action.at);
                    track.visibility.push(Keyframe {
                        at: action.at,
                        duration: action.duration,
                        from,
                        to: if action.action == Action::Show {
                            1.
                        } else {
                            0.
                        },
                    });
                }
                Action::Wait => continue,
            }
            latest_end = latest_end.max(action.at + action.duration);
            boundaries.push(action.at);
            active_until.push(latest_end);
        }
        Ok(Self {
            width: animation.scene.width,
            height: animation.scene.height,
            duration: animation.duration,
            objects,
            boundaries,
            active_until,
        })
    }

    pub fn clamp_time(&self, time: f64) -> Result<f64> {
        if !time.is_finite() {
            return Err(AppError::validation("Rendering", "Time must be finite."));
        }
        Ok(time.clamp(0., self.duration))
    }

    pub fn frame_key(&self, time: f64) -> FrameKey {
        let segment = self.boundaries.partition_point(|at| *at <= time);
        let active = segment
            .checked_sub(1)
            .is_some_and(|i| self.active_until[i] > time);
        FrameKey {
            segment,
            phase: if active { time.to_bits() } else { 0 },
        }
    }
}

fn origins(from: &[i64], to: &[i64], by_value: bool) -> Vec<Option<usize>> {
    if !by_value {
        return (0..to.len())
            .map(|i| (i < from.len()).then_some(i))
            .collect();
    }
    let mut used = vec![false; from.len()];
    to.iter()
        .map(|value| {
            let source = from
                .iter()
                .enumerate()
                .find(|(i, candidate)| !used[*i] && *candidate == value)
                .map(|(i, _)| i);
            if let Some(index) = source {
                used[index] = true;
            }
            source
        })
        .collect()
}
