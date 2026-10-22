use crate::error::{AppError, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_EVENTS: usize = 10_000;
pub const MAX_VALUES: usize = 64;
pub const MAX_OBJECT_VALUES: usize = 128;
pub const MAX_EDGES: usize = 256;
pub const MAX_POINTERS: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Algorithm {
    ArrayTraversal,
    TwoSum,
    BinarySearch,
    BubbleSort,
    TwoPointers,
    SlidingWindow,
    Stack,
    Queue,
    Bfs,
    Dfs,
    Fibonacci,
}
impl Algorithm {
    pub fn name(&self) -> &'static str {
        match self {
            Self::ArrayTraversal => "Array traversal",
            Self::TwoSum => "Two Sum",
            Self::BinarySearch => "Binary search",
            Self::BubbleSort => "Bubble sort",
            Self::TwoPointers => "Two pointers",
            Self::SlidingWindow => "Sliding window",
            Self::Stack => "Stack",
            Self::Queue => "Queue",
            Self::Bfs => "Breadth-first search",
            Self::Dfs => "Depth-first search",
            Self::Fibonacci => "Fibonacci DP",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProblemInput {
    pub title: String,
    pub description: String,
    pub algorithm: Algorithm,
    pub values: Vec<i64>,
    pub target: i64,
    #[serde(default)]
    pub source_code: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub stdin: String,
    #[serde(default)]
    pub execute_source: bool,
    #[serde(default)]
    pub graph: Option<GraphInput>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GraphInput {
    pub edges: Vec<[usize; 2]>,
    #[serde(default)]
    pub directed: bool,
    #[serde(default)]
    pub start: usize,
}
fn default_language() -> String {
    "python".into()
}
impl Default for ProblemInput {
    fn default() -> Self {
        Self {
            title: "Two Sum".into(),
            description: "Find two indices whose values sum to the target.".into(),
            algorithm: Algorithm::TwoSum,
            values: vec![2, 7, 11, 15],
            target: 9,
            source_code: String::new(),
            language: default_language(),
            stdin: String::new(),
            execute_source: false,
            graph: None,
        }
    }
}
impl ProblemInput {
    pub fn validate(&self) -> Result<()> {
        if self.description.trim().is_empty() {
            return Err(AppError::new(
                "INPUT_EMPTY",
                "Input",
                "InputNormalization",
                "Enter a problem statement.",
            ));
        }
        if self.description.len() > 100_000 || self.source_code.len() > 100_000 {
            return Err(AppError::validation(
                "InputNormalization",
                "Problem or source exceeds 100 KB.",
            ));
        }
        if self.stdin.len() > 100_000 || (self.execute_source && self.source_code.trim().is_empty())
        {
            return Err(AppError::validation(
                "InputNormalization",
                "Execution requires source and stdin within 100 KB.",
            ));
        }
        if let Some(graph) = &self.graph
            && (graph.start >= self.values.len()
                || graph.edges.len() > 512
                || graph
                    .edges
                    .iter()
                    .flatten()
                    .any(|i| *i >= self.values.len()))
        {
            return Err(AppError::validation(
                "InputNormalization",
                "Graph endpoints/start must reference existing nodes; at most 512 edges.",
            ));
        }
        if self.values.is_empty() || self.values.len() > MAX_VALUES {
            return Err(AppError::validation(
                "InputNormalization",
                "Provide 1–64 values.",
            ));
        }
        if self.values.iter().any(|v| v.unsigned_abs() > 1_000_000_000)
            || self.target.unsigned_abs() > 1_000_000_000
        {
            return Err(AppError::validation(
                "InputNormalization",
                "Values and target must be within ±1,000,000,000.",
            ));
        }
        if matches!(
            self.algorithm,
            Algorithm::BinarySearch | Algorithm::TwoPointers
        ) && !self.values.windows(2).all(|w| w[0] <= w[1])
        {
            return Err(AppError::validation(
                "InputNormalization",
                "This algorithm requires values sorted in ascending order.",
            ));
        }
        if self.algorithm == Algorithm::SlidingWindow
            && (self.target < 1 || self.target as usize > self.values.len())
        {
            return Err(AppError::validation(
                "InputNormalization",
                "For sliding window, target is the window size (1–array length).",
            ));
        }
        if self.algorithm == Algorithm::Fibonacci && !(1..=40).contains(&self.target) {
            return Err(AppError::validation(
                "InputNormalization",
                "For Fibonacci, target is n (1–40).",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Header {
    pub schema: String,
    pub version: u32,
    pub id: String,
}
impl Header {
    pub fn new(schema: &str, id: &str) -> Self {
        Self {
            schema: schema.into(),
            version: 1,
            id: id.into(),
        }
    }
    pub fn validate(&self, schema: &str) -> Result<()> {
        if self.schema != schema || !(1..=2).contains(&self.version) || self.id.is_empty() {
            return Err(AppError::validation(
                "IRValidation",
                format!("Expected {schema} version 1 with a nonempty id."),
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Entity {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub role: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AlgorithmIr {
    #[serde(flatten)]
    pub header: Header,
    pub name: String,
    pub category: Algorithm,
    pub entities: Vec<Entity>,
    pub phases: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EventKind {
    Read,
    Write,
    Compare,
    Insert,
    Remove,
    Swap,
    Push,
    Pop,
    Enqueue,
    Dequeue,
    Visit,
    Branch,
    Return,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TraceEvent {
    pub id: String,
    pub seq: usize,
    pub kind: EventKind,
    pub entity: String,
    pub indices: Vec<usize>,
    pub values: Vec<i64>,
    pub explanation: String,
    pub state: BTreeMap<String, Value>,
    #[serde(default)]
    pub source: Option<SourceLocation>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    #[serde(default)]
    pub function: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TraceIr {
    #[serde(flatten)]
    pub header: Header,
    pub algorithm: String,
    pub trace_source: String,
    pub confidence: String,
    pub initial_state: BTreeMap<String, Value>,
    pub events: Vec<TraceEvent>,
    pub final_state: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VisualObject {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub bind: String,
    pub placement: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VisualizationIr {
    #[serde(flatten)]
    pub header: Header,
    pub template: String,
    pub objects: Vec<VisualObject>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Scene {
    pub width: u32,
    pub height: u32,
    pub objects: Vec<SceneObject>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ScenePointer {
    pub name: String,
    pub target: usize,
}
/// Structural state that can change independently of numeric cell values.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct SceneStructure {
    #[serde(default)]
    pub edges: Vec<[usize; 2]>,
    #[serde(default)]
    pub edge_labels: Vec<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub states: Vec<String>,
    #[serde(default)]
    pub positions: Vec<[f32; 2]>,
    #[serde(default)]
    pub pointers: Vec<ScenePointer>,
    #[serde(default)]
    pub parents: Vec<Option<usize>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SceneObject {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub text: String,
    pub values: Vec<i64>,
    pub state: String,
    #[serde(default)]
    pub edges: Vec<[usize; 2]>,
    #[serde(default)]
    pub directed: bool,
    #[serde(default)]
    pub edge_labels: Vec<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub states: Vec<String>,
    #[serde(default)]
    pub positions: Vec<[f32; 2]>,
    #[serde(default)]
    pub pointers: Vec<ScenePointer>,
    #[serde(default)]
    pub parents: Vec<Option<usize>>,
    #[serde(default)]
    pub rows: u32,
    #[serde(default)]
    pub cols: u32,
}
impl SceneObject {
    pub fn structure(&self) -> SceneStructure {
        SceneStructure {
            edges: self.edges.clone(),
            edge_labels: self.edge_labels.clone(),
            labels: self.labels.clone(),
            states: self.states.clone(),
            positions: self.positions.clone(),
            pointers: self.pointers.clone(),
            parents: self.parents.clone(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    Highlight,
    UpdateText,
    UpdateValue,
    Show,
    Hide,
    Wait,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnimationAction {
    pub at: f64,
    pub duration: f64,
    pub action: Action,
    pub target: String,
    pub indices: Vec<usize>,
    pub text: String,
    pub values: Vec<i64>,
    #[serde(default)]
    pub structure: Option<SceneStructure>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnimationIr {
    #[serde(flatten)]
    pub header: Header,
    pub scene: Scene,
    pub timeline: Vec<AnimationAction>,
    pub duration: f64,
    pub step_times: Vec<f64>,
}

pub fn schema<T: JsonSchema>() -> Value {
    schemars::schema_for!(T).to_value()
}
pub fn parse_validated<T: JsonSchema + DeserializeOwned>(value: &Value) -> Result<T> {
    let schema = schema::<T>();
    let validator = jsonschema::validator_for(&schema)
        .map_err(|e| AppError::validation("SchemaValidation", e.to_string()))?;
    if let Some(e) = validator.iter_errors(value).next() {
        return Err(AppError::validation(
            "SchemaValidation",
            format!("{}: {}", e.instance_path(), e),
        ));
    }
    serde_json::from_value(value.clone()).map_err(|e| AppError::validation("Parse", e.to_string()))
}
pub fn unique_ids<'a>(ids: impl Iterator<Item = &'a str>, stage: &str) -> Result<()> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if id.is_empty() || !seen.insert(id) {
            return Err(AppError::validation(
                stage,
                format!("Empty or duplicate ID: {id}"),
            ));
        }
    }
    Ok(())
}
fn valid_label(value: &str, limit: usize) -> bool {
    value.chars().count() <= limit && !value.chars().any(char::is_control)
}
fn validate_structure(structure: &SceneStructure, count: usize) -> Result<()> {
    if structure.edges.len() > MAX_EDGES
        || structure.pointers.len() > MAX_POINTERS
        || structure
            .edges
            .iter()
            .flatten()
            .any(|index| *index >= count)
        || (!structure.edge_labels.is_empty()
            && structure.edge_labels.len() != structure.edges.len())
        || structure
            .edge_labels
            .iter()
            .any(|label| !valid_label(label, 64))
        || structure.labels.len() > count
        || structure.states.len() > count
        || structure
            .labels
            .iter()
            .any(|label| !valid_label(label, 128))
        || structure.states.iter().any(|state| !valid_label(state, 32))
        || (!structure.positions.is_empty() && structure.positions.len() != count)
        || structure
            .positions
            .iter()
            .flatten()
            .any(|value| !value.is_finite() || !(0. ..=1.).contains(value))
        || (!structure.parents.is_empty() && structure.parents.len() != count)
        || structure
            .parents
            .iter()
            .flatten()
            .any(|parent| *parent >= count)
    {
        return Err(AppError::validation(
            "AnimationValidation",
            "Invalid scene structure limits or references.",
        ));
    }
    unique_ids(
        structure
            .pointers
            .iter()
            .map(|pointer| pointer.name.as_str()),
        "AnimationValidation",
    )?;
    for pointer in &structure.pointers {
        if pointer.target >= count {
            return Err(AppError::validation(
                "AnimationValidation",
                format!("Pointer target is outside the object: {}", pointer.name),
            ));
        }
    }
    Ok(())
}
impl AlgorithmIr {
    pub fn validate(&self) -> Result<()> {
        self.header.validate("algorithm-ir")?;
        unique_ids(
            self.entities.iter().map(|e| e.id.as_str()),
            "AlgorithmValidation",
        )?;
        if self.entities.is_empty() || self.phases.is_empty() {
            return Err(AppError::validation(
                "AlgorithmValidation",
                "Entities and phases are required.",
            ));
        }
        for e in &self.entities {
            if !["array", "scalar", "map", "stack", "queue", "graph", "table"]
                .contains(&e.kind.as_str())
            {
                return Err(AppError::validation(
                    "AlgorithmValidation",
                    "Unknown entity type.",
                ));
            }
        }
        Ok(())
    }
}
impl TraceIr {
    pub fn validate(&self, algorithm: &AlgorithmIr) -> Result<()> {
        self.header.validate("trace-ir")?;
        algorithm.validate()?;
        if self.algorithm != algorithm.header.id
            || self.events.is_empty()
            || self.events.len() > MAX_EVENTS
        {
            return Err(AppError::validation(
                "TraceValidation",
                "Invalid algorithm reference or event count.",
            ));
        }
        if ![
            "simulation",
            "runtime",
            "instrumentation",
            "model",
            "manual",
        ]
        .contains(&self.trace_source.as_str())
            || !["derived", "verified", "inferred", "unknown"].contains(&self.confidence.as_str())
            || (self.trace_source == "model" && self.confidence == "verified")
        {
            return Err(AppError::validation(
                "TraceValidation",
                "Invalid trace provenance.",
            ));
        }
        unique_ids(self.events.iter().map(|e| e.id.as_str()), "TraceValidation")?;
        for (i, event) in self.events.iter().enumerate() {
            if event.seq != i
                || !algorithm.entities.iter().any(|e| e.id == event.entity)
                || event.values.len() > MAX_VALUES
                || event
                    .indices
                    .iter()
                    .any(|index| *index >= event.values.len())
            {
                return Err(AppError::validation(
                    "TraceValidation",
                    format!("Invalid event at events[{i}]."),
                ));
            }
        }
        Ok(())
    }
}
impl VisualizationIr {
    pub fn validate(&self, algorithm: &AlgorithmIr) -> Result<()> {
        self.header.validate("visualization-ir")?;
        unique_ids(
            self.objects.iter().map(|o| o.id.as_str()),
            "VisualizationValidation",
        )?;
        if self.objects.is_empty() {
            return Err(AppError::validation(
                "VisualizationValidation",
                "No visual objects.",
            ));
        }
        for obj in &self.objects {
            if !algorithm.entities.iter().any(|e| e.id == obj.bind)
                || ![
                    "array-view",
                    "text-view",
                    "graph-view",
                    "tree-view",
                    "table-view",
                    "stack-view",
                    "queue-view",
                ]
                .contains(&obj.kind.as_str())
                || !["center", "below"].contains(&obj.placement.as_str())
            {
                return Err(AppError::validation(
                    "VisualizationValidation",
                    "Unknown binding, component or placement.",
                ));
            }
        }
        Ok(())
    }
}
impl AnimationIr {
    pub fn validate(&self) -> Result<()> {
        self.header.validate("animation-ir")?;
        if self.scene.width < 320
            || self.scene.width > 1920
            || self.scene.height < 240
            || self.scene.height > 1080
            || !self.duration.is_finite()
            || self.duration <= 0.
            || self.duration > 3600.
            || self.timeline.len() > MAX_EVENTS * 4
        {
            return Err(AppError::validation(
                "AnimationValidation",
                "Invalid canvas, duration or action count.",
            ));
        }
        unique_ids(
            self.scene.objects.iter().map(|o| o.id.as_str()),
            "AnimationValidation",
        )?;
        for o in &self.scene.objects {
            let grid = o.kind == "grid";
            if ![
                "array",
                "text",
                "graph",
                "tree",
                "table",
                "stack",
                "queue",
                "linked-list",
                "grid",
            ]
            .contains(&o.kind.as_str())
                || ![o.x, o.y, o.width, o.height]
                    .iter()
                    .all(|v| v.is_finite() && *v >= 0.)
                || o.x + o.width > self.scene.width as f32
                || o.y + o.height > self.scene.height as f32
                || o.values.len() > MAX_OBJECT_VALUES
                || (grid
                    && (o.rows > 64
                        || o.cols > 64
                        || u64::from(o.rows) * u64::from(o.cols) != o.values.len() as u64))
            {
                return Err(AppError::validation(
                    "AnimationValidation",
                    "Invalid object geometry, type, grid dimensions or values.",
                ));
            }
            validate_structure(&o.structure(), o.values.len())?;
        }
        let mut last = 0.;
        let mut lengths: BTreeMap<&str, usize> = self
            .scene
            .objects
            .iter()
            .map(|o| (o.id.as_str(), o.values.len()))
            .collect();
        for a in &self.timeline {
            let object = self
                .scene
                .objects
                .iter()
                .find(|o| o.id == a.target)
                .ok_or_else(|| {
                    AppError::validation(
                        "AnimationValidation",
                        format!("Unknown target: {}", a.target),
                    )
                })?;
            if !a.at.is_finite()
                || !a.duration.is_finite()
                || a.at < last
                || a.duration < 0.
                || a.at + a.duration > self.duration + 0.000001
                || a.values.len() > MAX_OBJECT_VALUES
                || (a.action == Action::Highlight
                    && (object.kind == "text"
                        || a.indices.iter().any(|i| *i >= lengths[a.target.as_str()])))
                || (a.action == Action::UpdateValue && object.kind == "text")
                || (a.action == Action::UpdateText && object.kind != "text")
                || (a.action != Action::UpdateValue && a.structure.is_some())
            {
                return Err(AppError::validation(
                    "AnimationValidation",
                    "Invalid timeline order, duration or array index.",
                ));
            }
            if let Some(structure) = &a.structure {
                validate_structure(structure, a.values.len())?;
            }
            if a.action == Action::UpdateValue {
                lengths.insert(a.target.as_str(), a.values.len());
            }
            last = a.at;
        }
        if self.step_times.is_empty()
            || self
                .step_times
                .iter()
                .any(|t| !t.is_finite() || *t < 0. || *t > self.duration)
            || !self.step_times.windows(2).all(|w| w[0] < w[1])
        {
            return Err(AppError::validation(
                "AnimationValidation",
                "Invalid step timestamps.",
            ));
        }
        Ok(())
    }
}
