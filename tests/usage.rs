use oj_agent::{
    agent::Provider,
    session::{SessionStore, SqliteStore},
    usage::{TokenUsage, UsageRecord, UsageTotals},
};

#[test]
fn usage_is_persistent_idempotent_and_scoped_to_the_conversation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("sessions.sqlite3");
    let mut store = SqliteStore::open(&path).unwrap();
    let first = store.create_chat("first").unwrap();
    let second = store.create_chat("second").unwrap();
    let record = UsageRecord::new(
        "turn",
        Provider::DeepSeek,
        "model".into(),
        TokenUsage {
            input: Some(100),
            output: Some(20),
            total: Some(120),
            ..Default::default()
        },
    );
    store.record_token_usage(&first, &record).unwrap();
    store.record_token_usage(&first, &record).unwrap();
    let unknown = UsageRecord::new(
        "turn",
        Provider::Compatible,
        "unknown".into(),
        TokenUsage::default(),
    );
    store.record_token_usage(&second, &unknown).unwrap();
    drop(store);
    let mut store = SqliteStore::open(&path).unwrap();
    assert_eq!(store.token_usage(&first).unwrap().responses, 1);
    assert_eq!(store.token_usage(&first).unwrap().total.tokens, 120);
    let second_usage = store.token_usage(&second).unwrap();
    assert_eq!(second_usage.responses, 1);
    assert_eq!(second_usage.total.reported, 0);
    assert!(second_usage.label().contains("未上报"));
    let mut another_unknown = unknown.clone();
    another_unknown.id = uuid::Uuid::new_v4().to_string();
    store.record_token_usage(&first, &another_unknown).unwrap();
    let partial = store.token_usage(&first).unwrap();
    assert_eq!(partial.responses, 2);
    assert!(partial.label().contains("≥120"));
    store.delete(&first).unwrap();
    store.record_token_usage(&first, &record).unwrap();
    assert_eq!(store.token_usage(&first).unwrap().responses, 0);
    assert_eq!(store.list().unwrap().len(), 1);
    assert_eq!(store.token_usage(&second).unwrap().responses, 1);
}

#[test]
fn old_databases_gain_usage_storage_without_changing_existing_messages() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("sessions.sqlite3");
    let mut store = SqliteStore::open(&path).unwrap();
    let session = store.create_chat("old").unwrap();
    store
        .append_message(
            &session,
            &oj_agent::chat::Message::new("user", "original".into(), vec![]),
        )
        .unwrap();
    drop(store);
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute("DROP TABLE token_usage", [])
        .unwrap();
    let store = SqliteStore::open(&path).unwrap();
    assert_eq!(store.messages(&session).unwrap()[0].markdown, "original");
    assert_eq!(store.token_usage(&session).unwrap().responses, 0);
    assert!(
        store
            .token_usage(&session)
            .unwrap()
            .label()
            .contains("暂无")
    );
}

#[test]
fn zero_missing_and_partial_counters_remain_distinct_and_overflow_is_a_lower_bound() {
    let mut raw = rig::completion::Usage::default();
    raw.input_tokens = Some(0);
    raw.output_tokens = Some(0);
    let normalized = TokenUsage::from(&raw);
    assert_eq!(normalized.total, Some(0));
    let mut totals = UsageTotals::default();
    totals.add(&normalized);
    assert!(totals.label().contains("合计 0"));
    assert!(!totals.label().contains("未上报"));
    totals.add(&TokenUsage::default());
    assert!(totals.label().contains("≥0"));
    assert!(totals.label().contains("未上报"));
    let mut large = UsageTotals::default();
    let usage = TokenUsage {
        input: Some(u64::MAX),
        output: Some(1),
        total: Some(u64::MAX),
        ..Default::default()
    };
    large.add(&usage);
    large.add(&usage);
    assert!(large.label().contains(&format!("≥{}", u64::MAX)));
}
