use oj_agent::{
    agent::parse_analysis,
    animation::{self, Player},
    ir::*,
    renderer::{Renderer, SkiaRenderer},
    session::{SessionStore, SqliteStore},
    tools, trace, visualization,
};
use serde_json::json;
use std::sync::atomic::AtomicBool;

fn input(algorithm: Algorithm, values: Vec<i64>, target: i64) -> ProblemInput {
    ProblemInput {
        algorithm,
        values,
        target,
        ..Default::default()
    }
}
fn pipeline(input: &ProblemInput) -> AnimationIr {
    let (a, t) = trace::simulate(input).unwrap();
    let v = visualization::plan(&a, &t).unwrap();
    for raw in [
        serde_json::to_value(&a).unwrap(),
        serde_json::to_value(&t).unwrap(),
        serde_json::to_value(&v).unwrap(),
    ] {
        assert_eq!(raw["version"], 1);
    }
    animation::compile(&a, &t, &v).unwrap()
}
#[test]
fn canonical_algorithm_results() {
    let cases = [
        (Algorithm::TwoSum, vec![2, 7, 11, 15], 9, json!([0, 1])),
        (Algorithm::TwoSum, vec![3, 3], 6, json!([0, 1])),
        (Algorithm::TwoSum, vec![3], 6, json!(null)),
        (Algorithm::BinarySearch, vec![1, 3, 5, 7], 5, json!(2)),
        (Algorithm::BinarySearch, vec![1], 0, json!(null)),
        (Algorithm::BinarySearch, vec![1], 2, json!(null)),
        (
            Algorithm::BubbleSort,
            vec![4, -2, 4, 1],
            0,
            json!([-2, 1, 4, 4]),
        ),
        (Algorithm::ArrayTraversal, vec![-2, 3, 5], 0, json!(6)),
        (Algorithm::TwoPointers, vec![1, 2, 4, 7], 9, json!([1, 3])),
        (Algorithm::SlidingWindow, vec![-3, -2, -4], 2, json!(-5)),
        (Algorithm::Stack, vec![1, 2, 3], 0, json!([3, 2, 1])),
        (Algorithm::Queue, vec![1, 2, 3], 0, json!([1, 2, 3])),
        (
            Algorithm::Bfs,
            vec![1, 2, 3, 4, 5],
            0,
            json!([1, 2, 3, 4, 5]),
        ),
        (
            Algorithm::Dfs,
            vec![1, 2, 3, 4, 5],
            0,
            json!([1, 2, 4, 5, 3]),
        ),
        (Algorithm::Fibonacci, vec![0], 10, json!(55)),
    ];
    for (algorithm, values, target, expected) in cases {
        let p = input(algorithm, values, target);
        let (a, t) = trace::simulate(&p).unwrap();
        assert_eq!(t.final_state["result"], expected, "{}", a.name);
        assert_eq!(t.trace_source, "simulation");
        assert_eq!(t.confidence, "derived");
        let animation = pipeline(&p);
        animation.validate().unwrap();
        let value = serde_json::to_value(&animation).unwrap();
        parse_validated::<AnimationIr>(&value).unwrap();
    }
}
#[test]
fn invalid_inputs_fail_before_execution() {
    for p in [
        input(Algorithm::BinarySearch, vec![2, 1], 0),
        input(Algorithm::SlidingWindow, vec![1], 0),
        input(Algorithm::Fibonacci, vec![1], 41),
        input(Algorithm::TwoSum, vec![], 0),
        input(Algorithm::TwoSum, vec![1; 65], 0),
        input(Algorithm::TwoSum, vec![i64::MAX], 0),
    ] {
        assert!(trace::simulate(&p).is_err());
    }
}
#[test]
fn validates_references_versions_and_timing() {
    let (a, mut t) = trace::simulate(&Default::default()).unwrap();
    t.events[0].entity = "missing".into();
    assert!(t.validate(&a).is_err());
    t.events[0].entity = "values".into();
    t.events[0].seq = 42;
    assert!(t.validate(&a).is_err());
    let mut animation = pipeline(&Default::default());
    animation.header.version = 99;
    assert!(animation.validate().is_err());
    animation.header.version = 1;
    animation.timeline[0].target = "missing".into();
    assert!(animation.validate().is_err());
    animation.timeline[0].target = "values_view".into();
    animation.timeline[0].duration = f64::NAN;
    assert!(animation.validate().is_err());
    animation.timeline[0].duration = 0.;
    animation.timeline[0].at = -1.;
    assert!(animation.validate().is_err());
}
#[test]
fn model_schema_rejects_missing_fields_and_unsupported_algorithms() {
    assert!(parse_analysis("not json").is_err());
    assert!(parse_analysis(r#"{"explanation":"ok","problem":{}}"#).is_err());
    let mut value = json!({"explanation":"Uses a map.","problem":ProblemInput::default()});
    let text = serde_json::to_string(&value).unwrap();
    assert!(parse_analysis(&format!("```json\n{text}\n``` ")).is_ok());
    value["problem"]["algorithm"] = json!("arbitrary-shell");
    assert!(parse_analysis(&value.to_string()).is_err());
}
#[test]
fn immutable_revisions_and_stale_dependencies() {
    let mut store = SqliteStore::memory().unwrap();
    let id = store.create(&Default::default()).unwrap();
    let v1 = store
        .add_artifact(&id, "algorithm", &json!({"v":1}), &[], true)
        .unwrap();
    store
        .add_artifact(&id, "trace", &json!({}), std::slice::from_ref(&v1), true)
        .unwrap();
    let invalid = store
        .add_artifact(&id, "algorithm", &json!({"v":2}), &[], false)
        .unwrap();
    assert_eq!(store.active(&id, "algorithm").unwrap().unwrap().id, v1);
    assert!(
        store
            .add_artifact(&id, "trace", &json!({}), &[invalid], true)
            .is_err()
    );
    store
        .add_artifact(&id, "algorithm", &json!({"v":3}), &[], true)
        .unwrap();
    assert!(store.active(&id, "trace").unwrap().is_none());
    assert_eq!(store.artifacts(&id).unwrap().len(), 4);
    let other = store.create(&Default::default()).unwrap();
    assert!(
        store
            .add_artifact(&other, "trace", &json!({}), &[v1], true)
            .is_err()
    );
    store.delete(&id).unwrap();
    assert!(store.artifacts(&id).unwrap().is_empty());
    assert_eq!(store.list().unwrap().len(), 1);
}
#[tokio::test]
async fn pipeline_persists_checkpoints_and_cancellation() {
    let mut store = SqliteStore::memory().unwrap();
    let input = ProblemInput::default();
    let id = store.create(&input).unwrap();
    let generation = tools::generate(
        &mut store,
        &id,
        &input,
        None,
        &AtomicBool::new(false),
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(generation.trace_source, "simulation / derived");
    assert!(store.active(&id, "animation").unwrap().is_some());
    assert_eq!(store.list().unwrap()[0].status, "ready");
    let id = store.create(&input).unwrap();
    let err = tools::generate(
        &mut store,
        &id,
        &input,
        None,
        &AtomicBool::new(true),
        |_| {},
    )
    .await
    .unwrap_err();
    assert_eq!(err.code, "CANCELLED");
    assert_eq!(
        store
            .list()
            .unwrap()
            .iter()
            .find(|s| s.id == id)
            .unwrap()
            .status,
        "cancelled"
    );
}
#[test]
fn reopen_database_preserves_revisions_and_input() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("test.sqlite3");
    let id;
    {
        let mut store = SqliteStore::open(&path).unwrap();
        id = store.create(&Default::default()).unwrap();
        store
            .checkpoint(&id, "TraceGeneration", "processing")
            .unwrap();
    }
    let mut store = SqliteStore::open(&path).unwrap();
    assert_eq!(store.list().unwrap()[0].status, "processing");
    store.recover_interrupted().unwrap();
    assert_eq!(store.list().unwrap()[0].status, "interrupted");
    assert_eq!(store.input(&id).unwrap().values, vec![2, 7, 11, 15]);
}
#[test]
fn seek_is_deterministic_and_player_stops_at_end() {
    let animation = pipeline(&Default::default());
    let mut player = Player {
        playing: true,
        ..Default::default()
    };
    player.tick(1000., &animation);
    assert!(!player.playing);
    assert_eq!(player.time, animation.duration);
    player.step(false, &animation);
    let previous = player.time;
    player.step(false, &animation);
    assert!(player.time < previous);
    let renderer = SkiaRenderer;
    let before = renderer.frame(&animation, 0.).unwrap();
    let middle = renderer.frame(&animation, 1.6).unwrap();
    let again = renderer.frame(&animation, 0.).unwrap();
    assert_eq!(before.rgba, again.rgba);
    assert_ne!(before.rgba, middle.rgba);
    assert_eq!(
        before.rgba.len(),
        before.width as usize * before.height as usize * 4
    );
}
#[test]
fn export_roundtrip_and_failure_leave_ir_usable() {
    let animation = pipeline(&input(Algorithm::ArrayTraversal, vec![1], 0));
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("sample.gif");
    oj_agent::export::gif(&animation, &path, 2).unwrap();
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    let mut reader = options
        .read_info(std::fs::File::open(&path).unwrap())
        .unwrap();
    assert_eq!(reader.width(), 960);
    assert!(reader.read_next_frame().unwrap().is_some());
    assert!(oj_agent::export::gif(&animation, &temp.path().join("missing/out.gif"), 2).is_err());
    animation.validate().unwrap();
}
#[test]
fn tool_errors_are_structured() {
    let value = tools::invoke("ir.validate", &json!({"ir_type":"animation","document":{}}));
    assert_eq!(value["ok"], false);
    assert!(value["error"]["stage"].is_string());
    assert_eq!(
        tools::invoke("unknown", &json!({}))["error"]["code"],
        "TOOL_UNKNOWN"
    );
}
#[test]
fn cancelled_export_preserves_an_existing_destination() {
    let animation = pipeline(&Default::default());
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("existing.gif");
    std::fs::write(&path, b"existing data").unwrap();
    let error = oj_agent::export::gif_cancellable(&animation, &path, 10, &AtomicBool::new(true))
        .unwrap_err();
    assert_eq!(error.code, "CANCELLED");
    assert_eq!(std::fs::read(path).unwrap(), b"existing data");
}
#[test]
fn highlight_indices_follow_updated_array_lengths() {
    let mut animation = pipeline(&Default::default());
    animation.timeline[0].values = vec![2];
    animation.timeline[1].indices = vec![3];
    assert!(animation.validate().is_err());
}
#[tokio::test]
async fn export_references_the_exact_animation_revision() {
    let mut store = SqliteStore::memory().unwrap();
    let id = store.create(&Default::default()).unwrap();
    tools::generate(
        &mut store,
        &id,
        &Default::default(),
        None,
        &AtomicBool::new(false),
        |_| {},
    )
    .await
    .unwrap();
    let animation = store.active(&id, "animation").unwrap().unwrap();
    let temp = tempfile::NamedTempFile::new().unwrap();
    store.record_export(&id, temp.path(), "png").unwrap();
    assert_eq!(
        store.active(&id, "export").unwrap().unwrap().derived_from,
        vec![animation.id]
    );
}
#[cfg(not(target_os = "linux"))]
#[test]
fn unsupported_host_never_runs_submitted_code() {
    let error =
        oj_agent::runtime::SandboxRuntime::run_python("raise Exception('must not execute')", "")
            .unwrap_err();
    assert_eq!(error.code, "SANDBOX_UNAVAILABLE");
}
