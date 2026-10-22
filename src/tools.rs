use crate::{
    agent::{self, ModelConfig, RigModel},
    animation,
    error::{AppError, Result},
    ir::*,
    session::{SessionStore, SqliteStore},
    trace, visualization,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Generation {
    pub session_id: String,
    pub animation: AnimationIr,
    pub explanation: String,
    pub trace_source: String,
}

/// Each boundary writes an immutable validated artifact before advancing its checkpoint.
pub async fn generate(
    store: &mut SqliteStore,
    id: &str,
    input: &ProblemInput,
    config: Option<&ModelConfig>,
    cancel: &AtomicBool,
    progress: impl Fn(&str),
) -> Result<Generation> {
    let result = generate_inner(store, id, input, config, cancel, progress).await;
    if let Err(error) = &result {
        store.record_error(id, error)?;
    }
    result
}
async fn generate_inner(
    store: &mut SqliteStore,
    id: &str,
    input: &ProblemInput,
    config: Option<&ModelConfig>,
    cancel: &AtomicBool,
    progress: impl Fn(&str),
) -> Result<Generation> {
    input.validate()?;
    store.update_input(id, input)?;
    save(store, id, "input", input, &[])?;
    store.clear_active(id, "analysis")?;
    let mut effective = input.clone();
    let mut explanation = format!(
        "{}\n\nDeterministic built-in simulation for the selected sample. Submitted source is not executed.\n\n{}",
        input.algorithm.name(),
        algorithm_description(&input.algorithm)
    );
    if let Some(config) = config {
        store.checkpoint(id, "ProblemAnalysis", "processing")?;
        progress("Analyzing problem…");
        let output = RigModel::new(config)?.analyze(input, cancel).await?;
        for attempt in &output.attempts {
            store.add_artifact(
                id,
                "model-response",
                &serde_json::to_value(attempt)
                    .map_err(|e| AppError::validation("ProblemAnalysis", e.to_string()))?,
                &[],
                attempt.validation_error.is_none(),
            )?;
        }
        explanation = output.analysis.explanation;
        effective = output.analysis.problem;
        store.add_artifact(id,"analysis",&json!({"explanation":explanation,"problem":effective,"provider":config.provider,"model":config.model,"prompt_version":1}),&[],true)?;
        if !output.analysis.supported {
            return Err(AppError::new(
                "ALGORITHM_UNSUPPORTED",
                "Agent",
                "ProblemAnalysis",
                &explanation,
            )
            .recoverable());
        }
        store.update_input(id, &effective)?;
    }
    agent::check_cancelled(cancel)?;
    store.checkpoint(id, "TraceGeneration", "processing")?;
    progress("Generating deterministic trace…");
    let (algorithm, trace) = trace::simulate(&effective)?;
    let algorithm_id = save(store, id, "algorithm", &algorithm, &[])?;
    let trace_id = save(
        store,
        id,
        "trace",
        &trace,
        std::slice::from_ref(&algorithm_id),
    )?;
    agent::check_cancelled(cancel)?;
    store.checkpoint(id, "VisualizationPlanning", "processing")?;
    progress("Planning visualization…");
    let visual = visualization::plan(&algorithm, &trace)?;
    let visual_id = save(
        store,
        id,
        "visualization",
        &visual,
        &[algorithm_id, trace_id.clone()],
    )?;
    store.checkpoint(id, "AnimationCompilation", "processing")?;
    progress("Compiling animation…");
    let animation = animation::compile(&algorithm, &trace, &visual)?;
    agent::check_cancelled(cancel)?;
    save(store, id, "animation", &animation, &[trace_id, visual_id])?;
    store.checkpoint(id, "Preview", "ready")?;
    progress("Preview ready");
    Ok(Generation {
        session_id: id.into(),
        animation,
        explanation,
        trace_source: format!("{} / {}", trace.trace_source, trace.confidence),
    })
}
fn save<T: Serialize>(
    store: &mut SqliteStore,
    id: &str,
    kind: &str,
    value: &T,
    parents: &[String],
) -> Result<String> {
    store.add_artifact(
        id,
        kind,
        &serde_json::to_value(value)
            .map_err(|e| AppError::validation("Serialization", e.to_string()))?,
        parents,
        true,
    )
}
pub fn algorithm_description(algorithm: &Algorithm) -> &'static str {
    match algorithm {
        Algorithm::TwoSum => {
            "Hash map lookup before insertion prevents using one element twice. Expected O(n) time, O(n) space."
        }
        Algorithm::BinarySearch => {
            "Search a sorted array using a half-open interval. O(log n) time, O(1) space."
        }
        Algorithm::BubbleSort => {
            "Compare adjacent elements and swap inversions. O(n²) time, O(1) auxiliary space."
        }
        Algorithm::TwoPointers => {
            "Move the left or right pointer in a sorted array to find the target sum. O(n) time."
        }
        Algorithm::SlidingWindow => {
            "Target is the fixed window width. Compare sums of all windows. This teaching implementation recomputes each sum: O(nk) time."
        }
        Algorithm::Stack => {
            "Push every value, then pop in last-in-first-out order. O(n) time and space."
        }
        Algorithm::Queue => {
            "Enqueue every value, then dequeue in first-in-first-out order. O(n) time and space."
        }
        Algorithm::Bfs => {
            "Array values label a complete binary tree, with children at 2i+1 and 2i+2. Visit nodes level by level. O(n) time and space."
        }
        Algorithm::Dfs => {
            "Array values label a complete binary tree, with children at 2i+1 and 2i+2. Visit in preorder using a stack. O(n) time."
        }
        Algorithm::Fibonacci => {
            "Target is n (1–40). Compute dp[i] = dp[i−1] + dp[i−2], starting with 0 and 1. O(n) time and space."
        }
        Algorithm::ArrayTraversal => {
            "Read each value once and maintain a running sum. O(n) time, O(1) auxiliary space."
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ToolDefinition {
    pub name: &'static str,
    pub description: &'static str,
    pub side_effects: bool,
    pub input_schema: Value,
    pub output_schema: Value,
}
pub fn definitions() -> Vec<ToolDefinition> {
    [
        ("problem.parse","Validate and normalize a problem",false,schema::<ProblemInput>()),
        ("trace.simulate","Run a trusted algorithm template",false,schema::<ProblemInput>()),
        ("ir.validate","Validate versioned IR and semantic references",false,json!({"type":"object","required":["ir_type","document"],"properties":{"ir_type":{"enum":["algorithm","trace","visualization","animation"]},"document":{"type":"object"},"algorithm":{"type":"object"}}})),
        ("code.run","Execute Python in an isolated Linux rootfs",true,json!({"type":"object","required":["source"],"properties":{"source":{"type":"string"},"stdin":{"type":"string"}}})),
        ("code.trace","Execute Python that emits Trace IR on stdout",true,json!({"type":"object","required":["source","algorithm"],"properties":{"source":{"type":"string"},"stdin":{"type":"string"},"algorithm":schema::<AlgorithmIr>()}})),
        ("visualization.apply_template","Compile semantic layout",false,json!({"type":"object","required":["algorithm","trace"]})),
        ("animation.compile","Compile deterministic animation",false,json!({"type":"object","required":["algorithm","trace","visualization"]})),
        ("animation.validate","Validate an animation",false,schema::<AnimationIr>()),
        ("sandbox.verify","Run JavaScript in a bounded capability-free VM and compare JSON test results",false,json!({"type":"object","required":["source","cases"],"additionalProperties":false,"properties":{"source":{"type":"string","maxLength":100000},"cases":{"type":"array","minItems":1,"maxItems":16,"items":{"type":"object","required":["name","category","input","expected"],"properties":{"name":{"type":"string"},"category":{"enum":["normal","boundary","adversarial"]},"input":{},"expected":{}}}}}})),
    ].into_iter().map(|(name,description,side_effects,input_schema)|ToolDefinition{name,description,side_effects,input_schema,output_schema:json!({"type":"object","required":["ok"],"properties":{"ok":{"type":"boolean"},"data":{},"error":{"type":"object"},"warnings":{"type":"array"}}})}).collect()
}
pub fn invoke(name: &str, input: &Value) -> Value {
    match dispatch(name, input) {
        Ok(data) => json!({"ok":true,"data":data,"warnings":[]}),
        Err(error) => json!({"ok":false,"error":error}),
    }
}
fn dispatch(name: &str, input: &Value) -> Result<Value> {
    match name {
        "sandbox.verify" => {
            let request: oj_sandbox::Request = serde_json::from_value(input.clone())
                .map_err(|e| AppError::validation("Sandbox", e.to_string()))?;
            let report = oj_sandbox::verify(&request, std::sync::Arc::new(AtomicBool::new(false)))
                .map_err(|e| AppError::new("SANDBOX_FAILED", "Execution", "Sandbox", e))?;
            Ok(json!(report))
        }
        "problem.parse" => {
            let p: ProblemInput = parse_validated(input)?;
            p.validate()?;
            Ok(json!(p))
        }
        "trace.simulate" => {
            let p: ProblemInput = parse_validated(input)?;
            let (a, t) = trace::simulate(&p)?;
            Ok(json!({"algorithm_ir":a,"trace_ir":t}))
        }
        "ir.validate" => {
            let document = &input["document"];
            match input["ir_type"].as_str() {
                Some("algorithm") => parse_validated::<AlgorithmIr>(document)?.validate()?,
                Some("animation") => parse_validated::<AnimationIr>(document)?.validate()?,
                Some("trace") => parse_validated::<TraceIr>(document)?
                    .validate(&parse_validated::<AlgorithmIr>(&input["algorithm"])?)?,
                Some("visualization") => parse_validated::<VisualizationIr>(document)?
                    .validate(&parse_validated::<AlgorithmIr>(&input["algorithm"])?)?,
                _ => return Err(AppError::validation("IRValidation", "Unknown IR type.")),
            };
            Ok(json!({"valid":true,"errors":[]}))
        }
        "animation.validate" => {
            parse_validated::<AnimationIr>(input)?.validate()?;
            Ok(json!({"valid":true}))
        }
        "visualization.apply_template" => Ok(json!(visualization::plan(
            &parse_validated(&input["algorithm"])?,
            &parse_validated(&input["trace"])?
        )?)),
        "animation.compile" => Ok(json!(animation::compile(
            &parse_validated(&input["algorithm"])?,
            &parse_validated(&input["trace"])?,
            &parse_validated(&input["visualization"])?
        )?)),
        "code.run" | "code.trace" => {
            let source = input["source"]
                .as_str()
                .ok_or_else(|| AppError::validation("Execution", "source is required."))?;
            let execution = crate::runtime::SandboxRuntime::run_python(
                source,
                input["stdin"].as_str().unwrap_or(""),
            )?;
            if name == "code.trace" {
                let raw: Value = serde_json::from_str(&execution.stdout)
                    .map_err(|e| AppError::validation("TraceValidation", e.to_string()))?;
                let mut trace: TraceIr = parse_validated(&raw)?;
                trace.trace_source = "instrumentation".into();
                trace.confidence = "derived".into();
                trace.validate(&parse_validated::<AlgorithmIr>(&input["algorithm"])?)?;
                Ok(json!({"trace_ir":trace,"execution":execution}))
            } else {
                Ok(json!(execution))
            }
        }
        _ => Err(AppError::new(
            "TOOL_UNKNOWN",
            "Input",
            "ToolDispatch",
            format!("Unknown tool: {name}"),
        )),
    }
}
