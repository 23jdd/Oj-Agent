//! Portable, capability-free JavaScript verification. No host modules are registered.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub name: String,
    /// normal, boundary or adversarial; supplied cases are tests, not a proof.
    pub category: String,
    pub input: Value,
    pub expected: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub source: String,
    pub cases: Vec<Case>,
}
pub const MAX_STEPS: usize = 256;
pub const MAX_VALUES: usize = 64;
pub const MAX_NODES: usize = 128;
pub const MAX_EDGES: usize = 256;
pub const MAX_POINTERS: usize = 32;
pub const MAX_CELLS: usize = 256;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub value: Option<i64>,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub x: Option<f32>,
    #[serde(default)]
    pub y: Option<f32>,
    #[serde(default)]
    pub parent: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub directed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Pointer {
    pub name: String,
    /// A node ID, a value index, or "row,col" for a grid cell.
    pub target: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Cell {
    pub row: usize,
    pub col: usize,
    #[serde(default)]
    pub value: i64,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub state: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Step {
    #[serde(default)]
    pub values: Vec<i64>,
    #[serde(default)]
    pub indices: Vec<usize>,
    pub explanation: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub edges: Vec<Edge>,
    #[serde(default)]
    pub pointers: Vec<Pointer>,
    #[serde(default)]
    pub cells: Vec<Cell>,
}

fn bounded_text(value: &str, limit: usize) -> bool {
    value.chars().count() <= limit && !value.chars().any(char::is_control)
}
/// Validates one emitted step independently of rendering. The animation compiler
/// repeats these checks because stored reports may predate a schema change.
pub fn validate_step(step: &Step) -> Result<(), String> {
    if step.values.len() > MAX_VALUES
        || step.values.iter().any(|v| v.unsigned_abs() > 1_000_000_000)
        || step.indices.iter().any(|i| *i >= step.values.len())
        || step.explanation.len() > 2000
        || step.state.len() > 2000
    {
        return Err("Trace contains invalid indices, too many values, or oversized text.".into());
    }
    if step.nodes.len() > MAX_NODES
        || step.edges.len() > MAX_EDGES
        || step.pointers.len() > MAX_POINTERS
        || step.cells.len() > MAX_CELLS
    {
        return Err("Trace exceeds node, edge, pointer or cell limits.".into());
    }
    let mut node_ids = std::collections::BTreeSet::new();
    for node in &step.nodes {
        if !node_ids.insert(node.id.as_str())
            || !bounded_text(&node.id, 128)
            || !bounded_text(&node.label, 128)
            || !bounded_text(&node.state, 32)
            || node.value.is_some_and(|v| v.unsigned_abs() > 1_000_000_000)
            || node
                .x
                .is_some_and(|v| !v.is_finite() || !(0. ..=1.).contains(&v))
            || node
                .y
                .is_some_and(|v| !v.is_finite() || !(0. ..=1.).contains(&v))
            || node
                .parent
                .as_deref()
                .is_some_and(|parent| !bounded_text(parent, 128))
        {
            return Err(format!("Invalid node: {}", node.id));
        }
    }
    for edge in &step.edges {
        let endpoint = |target: &str| node_ids.contains(target) || cell_target_exists(step, target);
        if !endpoint(edge.from.as_str())
            || !endpoint(edge.to.as_str())
            || !bounded_text(&edge.label, 64)
        {
            return Err(format!("Invalid edge: {} -> {}", edge.from, edge.to));
        }
    }
    for node in &step.nodes {
        if let Some(parent) = &node.parent
            && !node_ids.contains(parent.as_str())
        {
            return Err(format!("Unknown parent for node: {}", node.id));
        }
    }
    let mut cell_keys = std::collections::BTreeSet::new();
    for cell in &step.cells {
        if cell.row > 63
            || cell.col > 63
            || !cell_keys.insert((cell.row, cell.col))
            || !bounded_text(&cell.label, 64)
            || !bounded_text(&cell.state, 32)
            || cell.value.unsigned_abs() > 1_000_000_000
        {
            return Err(format!("Invalid cell: {},{}", cell.row, cell.col));
        }
    }
    let mut pointer_names = std::collections::BTreeSet::new();
    for pointer in &step.pointers {
        if !bounded_text(&pointer.name, 64)
            || !pointer_names.insert(pointer.name.as_str())
            || !pointer_target_exists(step, &pointer.target)
        {
            return Err(format!("Invalid pointer: {}", pointer.name));
        }
    }
    Ok(())
}
fn cell_target_exists(step: &Step, target: &str) -> bool {
    let Some((row, col)) = target.split_once(',') else {
        return false;
    };
    let (Ok(row), Ok(col)) = (row.parse::<usize>(), col.parse::<usize>()) else {
        return false;
    };
    step.cells
        .iter()
        .any(|cell| cell.row == row && cell.col == col)
}
fn pointer_target_exists(step: &Step, target: &str) -> bool {
    if step.nodes.iter().any(|node| node.id == target) || cell_target_exists(step, target) {
        return true;
    }
    target
        .parse::<usize>()
        .is_ok_and(|index| index < step.values.len() || index < step.cells.len())
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CaseResult {
    pub name: String,
    pub category: String,
    pub input: Value,
    pub expected: Value,
    pub actual: Value,
    pub passed: bool,
    pub steps: Vec<Step>,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub language: String,
    pub cases: Vec<CaseResult>,
    pub passed: bool,
}

pub fn verify(request: &Request, cancel: Arc<AtomicBool>) -> Result<Report, String> {
    if request.source.is_empty()
        || request.source.len() > 100_000
        || request.cases.is_empty()
        || request.cases.len() > 16
    {
        return Err(
            "Provide solve(input, emit), at most 100 KB of JavaScript and 1–16 test cases.".into(),
        );
    }
    let mut names = std::collections::BTreeSet::new();
    for case in &request.cases {
        if case.name.is_empty()
            || case.name.len() > 128
            || !names.insert(&case.name)
            || !["normal", "boundary", "adversarial"].contains(&case.category.as_str())
            || serde_json::to_vec(&case.input)
                .map_err(|e| e.to_string())?
                .len()
                > 65536
            || serde_json::to_vec(&case.expected)
                .map_err(|e| e.to_string())?
                .len()
                > 65536
        {
            return Err("Cases need unique names, normal/boundary/adversarial categories and JSON input/expected values under 64 KiB.".into());
        }
    }
    let mut cases = Vec::new();
    for case in &request.cases {
        if cancel.load(Ordering::Relaxed) {
            return Err("Verification cancelled.".into());
        }
        let result = execute(&request.source, &case.input, cancel.clone());
        let (actual, steps, error) = match result {
            Ok((actual, steps)) => (actual, steps, None),
            Err(error) => (Value::Null, vec![], Some(error)),
        };
        let passed = error.is_none() && actual == case.expected;
        cases.push(CaseResult {
            name: case.name.clone(),
            category: case.category.clone(),
            input: case.input.clone(),
            expected: case.expected.clone(),
            actual,
            passed,
            steps,
            error,
        });
    }
    Ok(Report {
        language: "javascript".into(),
        passed: cases.iter().all(|c| c.passed),
        cases,
    })
}
fn execute(
    source: &str,
    input: &Value,
    cancel: Arc<AtomicBool>,
) -> Result<(Value, Vec<Step>), String> {
    let runtime = rquickjs::Runtime::new().map_err(|e| e.to_string())?;
    runtime.set_memory_limit(32 * 1024 * 1024);
    runtime.set_max_stack_size(256 * 1024);
    let started = Instant::now();
    runtime.set_interrupt_handler(Some(Box::new(move || {
        cancel.load(Ordering::Relaxed) || started.elapsed() > Duration::from_secs(2)
    })));
    let context = rquickjs::Context::full(&runtime).map_err(|e| e.to_string())?;
    // Capture intrinsics before untrusted source runs. Its global Function scope cannot
    // access this wrapper's steps, expected output, or serializer.
    let factory = serde_json::to_string(&format!("\"use strict\";\n{source}\n; return solve;"))
        .map_err(|e| e.to_string())?;
    let input = serde_json::to_string(&input.to_string()).map_err(|e| e.to_string())?;
    let script = format!(
        r#"(() => {{
        const stringify = JSON.stringify.bind(JSON), parse = JSON.parse.bind(JSON);
        const push = Function.prototype.call.bind(Array.prototype.push);
        const steps = [];
        const emit = value => {{
            if (steps.length >= 256) throw new Error('Trace exceeds 256 steps');
            const encoded = stringify(value);
            if (!encoded || encoded.length > 8192) throw new Error('Trace step exceeds 8 KiB');
            push(steps, parse(encoded));
        }};
        const solve = Function({factory})();
        if (typeof solve !== 'function') throw new Error('Define solve(input, emit)');
        const result = solve(parse({input}), emit);
        if (result === undefined || (result && typeof result.then === 'function')) throw new Error('Return a synchronous JSON value');
        const encoded = stringify({{ result, steps }});
        if (encoded.length > 1048576) throw new Error('Output exceeds 1 MiB');
        return encoded;
    }})()"#
    );
    let json: String = context.with(|ctx| {
        ctx.eval::<String, _>(script).map_err(|e| {
            if e.is_exception() {
                let exception = ctx.catch();
                format!("JavaScript exception: {exception:?} (time, memory and stack limits apply)")
            } else {
                e.to_string()
            }
        })
    })?;
    let result: Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let steps: Vec<Step> = serde_json::from_value(result["steps"].clone())
        .map_err(|e| format!("Invalid trace step: {e}"))?;
    if steps.len() > MAX_STEPS {
        return Err("Trace exceeds 256 steps.".into());
    }
    for step in &steps {
        validate_step(step)?;
    }
    Ok((result["result"].clone(), steps))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(source: &str) -> Request {
        Request {
            source: source.into(),
            cases: vec![Case {
                name: "test".into(),
                category: "normal".into(),
                input: serde_json::json!([2, 3]),
                expected: serde_json::json!(5),
            }],
        }
    }
    #[test]
    fn verifies_observed_trace_and_compares_outside_the_vm() {
        let report = verify(&request("function solve(a, emit) { emit({values:a, explanation:'sum'}); return a[0]+a[1]; }"), Arc::default()).unwrap();
        assert!(report.passed);
        assert_eq!(report.cases[0].steps[0].values, [2, 3]);
        assert!(
            !verify(&request("function solve() {return 0;}"), Arc::default())
                .unwrap()
                .passed
        );
    }
    #[test]
    fn accepts_structured_nodes_edges_pointers_and_cells() {
        let source = r#"function solve(a, emit) {
            emit({nodes:[{id:"a",label:"A",value:1,state:"active"},{id:"b",label:"B",value:2}],
                  edges:[{from:"a",to:"b",label:"next",directed:true}],
                  pointers:[{name:"cur",target:"a"}],explanation:"linked"});
            emit({cells:[{row:0,col:0,value:1},{row:1,col:1,value:2}],
                  pointers:[{name:"dp",target:"1,1"}],explanation:"grid"});
            return a[0]+a[1];
        }"#;
        let report = verify(&request(source), Arc::default()).unwrap();
        assert!(report.passed);
        assert_eq!(report.cases[0].steps.len(), 2);
        assert_eq!(report.cases[0].steps[0].nodes.len(), 2);
        assert_eq!(report.cases[0].steps[1].cells.len(), 2);
    }
    #[test]
    fn rejects_dangling_structured_references() {
        let source = r#"function solve(a, emit) {
            emit({nodes:[{id:"a"}],edges:[{from:"a",to:"missing"}],explanation:"bad"});
            return a[0]+a[1];
        }"#;
        let report = verify(&request(source), Arc::default()).unwrap();
        assert!(!report.passed);
        assert!(report.cases[0].error.is_some());
    }
    #[test]
    fn has_no_host_files_network_process_or_module_capabilities() {
        let source = "function solve() { return [typeof process, typeof require, typeof fetch, typeof std, typeof os, typeof Deno].every(v=>v==='undefined') ? 5 : 0; }";
        assert!(verify(&request(source), Arc::default()).unwrap().passed);
    }
    #[test]
    fn bounds_loops_recursion_memory_and_cancellation() {
        for source in [
            "function solve(){while(true){}}",
            "function solve(){return solve();}",
            "function solve(){return new Array(50000000).fill(7);}",
        ] {
            assert!(!verify(&request(source), Arc::default()).unwrap().passed);
        }
        assert!(
            verify(
                &request("function solve(){return 5;}"),
                Arc::new(AtomicBool::new(true))
            )
            .is_err()
        );
    }
}
