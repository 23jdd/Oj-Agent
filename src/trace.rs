use crate::{error::Result, ir::*};
use serde_json::{Value, json};
use std::collections::{BTreeMap, VecDeque};

pub fn algorithm_model(input: &ProblemInput) -> AlgorithmIr {
    AlgorithmIr {
        header: Header::new("algorithm-ir", "algorithm_main"),
        name: input.algorithm.name().into(),
        category: input.algorithm.clone(),
        entities: vec![
            Entity {
                id: "values".into(),
                kind: "array".into(),
                role: "input".into(),
            },
            Entity {
                id: "state".into(),
                kind: "map".into(),
                role: "state".into(),
            },
        ],
        phases: vec!["initialize".into(), "iterate".into(), "finish".into()],
    }
}
/// Runs trusted built-in algorithms only. Submitted source belongs to runtime::SandboxRuntime.
pub fn simulate(input: &ProblemInput) -> Result<(AlgorithmIr, TraceIr)> {
    input.validate()?;
    let algorithm = algorithm_model(input);
    let mut trace = TraceIr {
        header: Header::new("trace-ir", "trace_main"),
        algorithm: algorithm.header.id.clone(),
        trace_source: "simulation".into(),
        confidence: "derived".into(),
        initial_state: BTreeMap::from([
            ("values".into(), json!(input.values)),
            ("target".into(), json!(input.target)),
        ]),
        events: vec![],
        final_state: BTreeMap::new(),
    };
    if matches!(input.algorithm, Algorithm::Bfs | Algorithm::Dfs) {
        let graph = input.graph.clone().unwrap_or(GraphInput {
            edges: (1..input.values.len()).map(|i| [(i - 1) / 2, i]).collect(),
            directed: true,
            start: 0,
        });
        trace.initial_state.insert("graph".into(), json!(graph));
        trace.initial_state.insert(
            "layout".into(),
            json!(if input.graph.is_some() {
                "graph"
            } else {
                "tree"
            }),
        );
    }
    let mut values = input.values.clone();
    let mut result = Value::Null;
    let mut state = BTreeMap::new();
    let mut emit = |kind,
                    indices: Vec<usize>,
                    vals: &[i64],
                    explanation: String,
                    s: &BTreeMap<String, Value>| {
        let seq = trace.events.len();
        trace.events.push(TraceEvent {
            id: format!("event_{seq}"),
            seq,
            kind,
            entity: "values".into(),
            indices,
            values: vals.to_vec(),
            explanation,
            state: s.clone(),
            source: None,
        });
    };
    emit(
        EventKind::Read,
        vec![],
        &values,
        format!("{} · target {}", input.algorithm.name(), input.target),
        &state,
    );
    match input.algorithm {
        Algorithm::TwoSum => {
            let mut seen = BTreeMap::new();
            for (i, value) in values.iter().copied().enumerate() {
                let complement = input.target - value;
                state.insert("seen".into(), json!(seen));
                emit(
                    EventKind::Compare,
                    vec![i],
                    &values,
                    format!("Index {i}: {value}; look for {complement} in seen."),
                    &state,
                );
                if let Some(j) = seen.get(&complement).copied() {
                    result = json!([j, i]);
                    emit(
                        EventKind::Return,
                        vec![j, i],
                        &values,
                        format!("Found: indices {j} and {i} sum to {}.", input.target),
                        &state,
                    );
                    break;
                }
                seen.insert(value, i);
                state.insert("seen".into(), json!(seen));
                emit(
                    EventKind::Insert,
                    vec![i],
                    &values,
                    format!("Remember {value} → {i}."),
                    &state,
                );
            }
        }
        Algorithm::BinarySearch => {
            let (mut left, mut right) = (0, values.len());
            while left < right {
                let mid = left + (right - left) / 2;
                state.insert("range".into(), json!([left, right - 1]));
                emit(
                    EventKind::Compare,
                    vec![left, mid, right - 1],
                    &values,
                    format!(
                        "Range [{left}, {}], mid {mid}: compare {} with {}.",
                        right - 1,
                        values[mid],
                        input.target
                    ),
                    &state,
                );
                match values[mid].cmp(&input.target) {
                    std::cmp::Ordering::Equal => {
                        result = json!(mid);
                        emit(
                            EventKind::Return,
                            vec![mid],
                            &values,
                            format!("Found target at index {mid}."),
                            &state,
                        );
                        break;
                    }
                    std::cmp::Ordering::Less => left = mid + 1,
                    std::cmp::Ordering::Greater => right = mid,
                }
            }
        }
        Algorithm::BubbleSort => {
            for end in (1..values.len()).rev() {
                let mut swapped = false;
                for i in 0..end {
                    emit(
                        EventKind::Compare,
                        vec![i, i + 1],
                        &values,
                        format!("Compare {} and {}.", values[i], values[i + 1]),
                        &state,
                    );
                    if values[i] > values[i + 1] {
                        values.swap(i, i + 1);
                        swapped = true;
                        emit(
                            EventKind::Swap,
                            vec![i, i + 1],
                            &values,
                            format!("Swap positions {i} and {}.", i + 1),
                            &state,
                        );
                    }
                }
                if !swapped {
                    break;
                }
            }
            result = json!(values);
        }
        Algorithm::TwoPointers => {
            let (mut left, mut right) = (0, values.len() - 1);
            while left < right {
                let sum = values[left] + values[right];
                emit(
                    EventKind::Compare,
                    vec![left, right],
                    &values,
                    format!(
                        "{} + {} = {sum}; target {}.",
                        values[left], values[right], input.target
                    ),
                    &state,
                );
                match sum.cmp(&input.target) {
                    std::cmp::Ordering::Equal => {
                        result = json!([left, right]);
                        break;
                    }
                    std::cmp::Ordering::Less => left += 1,
                    std::cmp::Ordering::Greater => right -= 1,
                }
            }
        }
        Algorithm::SlidingWindow => {
            let k = input.target as usize;
            let mut best = i64::MIN;
            for start in 0..=values.len() - k {
                let sum: i64 = values[start..start + k].iter().sum();
                best = best.max(sum);
                state.insert("max_sum".into(), json!(best));
                emit(
                    EventKind::Read,
                    (start..start + k).collect(),
                    &values,
                    format!(
                        "Window [{start}, {}]: sum {sum}, best {best}.",
                        start + k - 1
                    ),
                    &state,
                );
            }
            result = json!(best);
        }
        Algorithm::Stack | Algorithm::Queue => {
            let mut container = VecDeque::new();
            for (i, v) in values.iter().enumerate() {
                container.push_back(*v);
                state.insert("container".into(), json!(container));
                emit(
                    if input.algorithm == Algorithm::Stack {
                        EventKind::Push
                    } else {
                        EventKind::Enqueue
                    },
                    vec![i],
                    &values,
                    format!("Insert {v}."),
                    &state,
                );
            }
            let mut output = vec![];
            while let Some(v) = if input.algorithm == Algorithm::Stack {
                container.pop_back()
            } else {
                container.pop_front()
            } {
                output.push(v);
                state.insert("container".into(), json!(container));
                emit(
                    if input.algorithm == Algorithm::Stack {
                        EventKind::Pop
                    } else {
                        EventKind::Dequeue
                    },
                    vec![],
                    &values,
                    format!("Remove {v}."),
                    &state,
                );
            }
            result = json!(output);
        }
        Algorithm::Bfs | Algorithm::Dfs => {
            let graph = input.graph.clone().unwrap_or(GraphInput {
                edges: (1..values.len()).map(|i| [(i - 1) / 2, i]).collect(),
                directed: true,
                start: 0,
            });
            let mut adjacency = vec![Vec::new(); values.len()];
            for [a, b] in &graph.edges {
                adjacency[*a].push(*b);
                if !graph.directed {
                    adjacency[*b].push(*a);
                }
            }
            let mut frontier = VecDeque::from([graph.start]);
            let mut discovered = vec![false; values.len()];
            let mut visited = vec![];
            while let Some(i) = if input.algorithm == Algorithm::Bfs {
                frontier.pop_front()
            } else {
                frontier.pop_back()
            } {
                if discovered[i] {
                    continue;
                }
                discovered[i] = true;
                visited.push(values[i]);
                state.insert("visited".into(), json!(visited));
                state.insert(
                    "visited_indices".into(),
                    json!(
                        discovered
                            .iter()
                            .enumerate()
                            .filter_map(|(i, v)| v.then_some(i))
                            .collect::<Vec<_>>()
                    ),
                );
                emit(
                    EventKind::Visit,
                    vec![i],
                    &values,
                    format!("Visit node {} (index {i}).", values[i]),
                    &state,
                );
                let mut children = adjacency[i].clone();
                if input.algorithm == Algorithm::Dfs {
                    children.reverse();
                }
                for child in children {
                    if !discovered[child] {
                        frontier.push_back(child);
                    }
                }
            }
            result = json!(visited);
        }
        Algorithm::Fibonacci => {
            values = vec![0; input.target as usize + 1];
            values[1] = 1;
            for i in 2..values.len() {
                values[i] = values[i - 1] + values[i - 2];
                emit(
                    EventKind::Write,
                    vec![i - 2, i - 1, i],
                    &values,
                    format!(
                        "dp[{i}] = {} + {} = {}.",
                        values[i - 2],
                        values[i - 1],
                        values[i]
                    ),
                    &state,
                );
            }
            result = json!(values.last());
        }
        Algorithm::ArrayTraversal => {
            let mut sum = 0i64;
            for (i, v) in values.iter().enumerate() {
                sum += v;
                state.insert("sum".into(), json!(sum));
                emit(
                    EventKind::Read,
                    vec![i],
                    &values,
                    format!("Read values[{i}] = {v}; running sum {sum}."),
                    &state,
                );
            }
            result = json!(sum);
        }
    }
    state.insert("result".into(), result.clone());
    emit(
        EventKind::Return,
        vec![],
        &values,
        format!("Result: {result}"),
        &state,
    );
    trace.final_state = state;
    trace.validate(&algorithm)?;
    Ok((algorithm, trace))
}
