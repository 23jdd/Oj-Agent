use oj_agent::{
    agent::{ModelConfig, Provider, RigModel},
    ir::ProblemInput,
};
use serde_json::json;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

fn server(replies: Vec<(u16, String)>) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let thread = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut requests = vec![];
        for (status, body) in replies {
            let mut stream = loop {
                match listener.accept() {
                    Ok((s, _)) => break s,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        if Instant::now() > deadline {
                            return requests;
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(e) => panic!("{e}"),
                }
            };
            // Winsock may inherit the listener's nonblocking flag on accepted sockets.
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut data = Vec::new();
            let mut buffer = [0u8; 8192];
            loop {
                let n = stream.read(&mut buffer).unwrap();
                if n == 0 {
                    break;
                }
                data.extend_from_slice(&buffer[..n]);
                let text = String::from_utf8_lossy(&data);
                if let Some(end) = text.find("\r\n\r\n") {
                    let length = text[..end]
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|n| n.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if data.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            requests.push(String::from_utf8(data).unwrap());
            let response = format!(
                "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        }
        requests
    });
    (endpoint, thread)
}
fn answer(content: &str) -> String {
    json!({"id":"test","object":"chat.completion","created":1,"model":"test-model","choices":[{"index":0,"message":{"role":"assistant","content":content},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":20,"total_tokens":30}}).to_string()
}
fn config(endpoint: String, provider: Provider) -> ModelConfig {
    ModelConfig {
        endpoint,
        provider,
        model: "test-model".into(),
        api_key: "test-key-only".into(),
        request: Default::default(),
    }
}
fn valid() -> String {
    json!({"explanation":"Hash map, O(n).","problem":ProblemInput::default(),"supported":true})
        .to_string()
}

fn unfinished(content: Option<&str>, reasoning: Option<&str>, reason: &str) -> String {
    json!({"id":"unfinished","object":"chat.completion","created":1,"model":"test-model",
        "choices":[{"index":0,"message":{"role":"assistant","content":content,"reasoning_content":reasoning},"finish_reason":reason}],
        "usage":{"prompt_tokens":10,"completion_tokens":8192,"total_tokens":8202}}).to_string()
}

fn request_body(request: &str) -> serde_json::Value {
    serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap()
}

#[tokio::test]
async fn reasoning_without_an_answer_recovers_once_and_observes_both_responses() {
    let (endpoint, thread) = server(vec![
        (
            200,
            unfinished(None, Some("private-reasoning-never-replay"), "length"),
        ),
        (
            200,
            unfinished(
                Some("final answer"),
                Some("private-reasoning-never-replay"),
                "stop",
            ),
        ),
    ]);
    let model = RigModel::new(&config(endpoint, Provider::DeepSeek)).unwrap();
    let mut usage = vec![];
    let mut status = vec![];
    let text = model
        .chat_completion_with_progress(
            "question",
            &AtomicBool::new(false),
            |value| {
                usage.push(*value);
                Ok(())
            },
            |value| status.push(value.to_owned()),
        )
        .await
        .unwrap();
    assert_eq!(text, "final answer");
    assert_eq!(usage.len(), 2);
    assert_eq!(usage[0].output_tokens, Some(8192));
    assert!(status.iter().any(|value| value.contains("关闭思考模式")));
    let requests = thread.join().unwrap();
    assert_eq!(requests.len(), 2);
    let first = request_body(&requests[0]);
    let second = request_body(&requests[1]);
    assert!(first.get("thinking").is_none());
    assert_eq!(first["max_tokens"], 8192);
    assert_eq!(second["thinking"]["type"], "disabled");
    assert_eq!(second["max_tokens"], 16384);
    assert!(
        requests
            .iter()
            .all(|value| !value.contains("private-reasoning-never-replay"))
    );
}

#[tokio::test]
async fn repeated_answerless_responses_stop_after_one_recovery_with_safe_diagnostics() {
    for (content, reasoning, reason, code) in [
        (Some("  "), None, "stop", "AGENT_RESPONSE_EMPTY"),
        (None, None, "stop", "AGENT_RESPONSE_EMPTY"),
        (
            None,
            Some("private-reasoning-never-log"),
            "stop",
            "MODEL_REASONING_ONLY",
        ),
        (
            Some("{\"unfinished"),
            None,
            "length",
            "MODEL_OUTPUT_TRUNCATED",
        ),
    ] {
        let body = unfinished(content, reasoning, reason);
        let (endpoint, thread) = server(vec![(200, body.clone()), (200, body)]);
        let mut responses = 0;
        let error = RigModel::new(&config(endpoint, Provider::DeepSeek))
            .unwrap()
            .chat_completion_observed("question", &AtomicBool::new(false), |_| {
                responses += 1;
                Ok(())
            })
            .await
            .unwrap_err();
        assert_eq!(error.code, code);
        assert_eq!(error.context["attempts"], "2");
        assert_eq!(error.context["finish_reason"], reason);
        assert_eq!(responses, 2);
        assert!(error.recoverable);
        let serialized = serde_json::to_string(&error).unwrap();
        assert!(!serialized.contains("private-reasoning-never-log"));
        assert!(!serialized.contains("test-key-only"));
        assert!(!serialized.contains("unfinished"));
        assert_eq!(thread.join().unwrap().len(), 2);
    }
}

#[tokio::test]
async fn filtered_or_interrupted_responses_are_never_published_or_automatically_retried() {
    for (reason, code) in [
        ("content_filter", "MODEL_CONTENT_FILTERED"),
        ("insufficient_system_resource", "MODEL_RESPONSE_INCOMPLETE"),
    ] {
        let (endpoint, thread) = server(vec![(
            200,
            unfinished(Some("partial output"), None, reason),
        )]);
        let error = RigModel::new(&config(endpoint, Provider::DeepSeek))
            .unwrap()
            .chat_completion("question", &AtomicBool::new(false))
            .await
            .unwrap_err();
        assert_eq!(error.code, code);
        assert_eq!(thread.join().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn cancelling_during_empty_response_recovery_prevents_another_request() {
    let (endpoint, thread) = server(vec![(200, unfinished(None, Some("private"), "length"))]);
    let cancel = AtomicBool::new(false);
    let error = RigModel::new(&config(endpoint, Provider::DeepSeek))
        .unwrap()
        .chat_completion_with_progress(
            "question",
            &cancel,
            |_| Ok(()),
            |_| cancel.store(true, Ordering::Relaxed),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, "CANCELLED");
    assert_eq!(thread.join().unwrap().len(), 1);
}

#[tokio::test]
async fn compatible_provider_recovery_does_not_send_deepseek_options() {
    let (endpoint, thread) = server(vec![
        (200, unfinished(None, None, "length")),
        (200, answer("recovered")),
    ]);
    let text = RigModel::new(&config(endpoint, Provider::Compatible))
        .unwrap()
        .chat_completion("question", &AtomicBool::new(false))
        .await
        .unwrap();
    assert_eq!(text, "recovered");
    let requests = thread.join().unwrap();
    assert!(
        requests
            .iter()
            .all(|request| request_body(request).get("thinking").is_none())
    );
    assert_eq!(request_body(&requests[1])["max_tokens"], 16384);
}

#[tokio::test]
async fn configured_retry_count_can_disable_or_extend_network_retries() {
    for retries in [0, 3] {
        let body = json!({"error":{"message":"Unavailable"}}).to_string();
        let (endpoint, thread) = server(vec![(503, body); retries as usize + 1]);
        let mut settings = config(endpoint, Provider::Compatible);
        settings.request.max_retries = retries;
        settings.request.retry_delay_ms = 0;
        assert!(
            RigModel::new(&settings)
                .unwrap()
                .chat_completion("question", &AtomicBool::new(false))
                .await
                .is_err()
        );
        assert_eq!(thread.join().unwrap().len(), retries as usize + 1);
    }
    let (endpoint, thread) = server(vec![(200, unfinished(None, None, "stop"))]);
    let mut settings = config(endpoint, Provider::Compatible);
    settings.request.max_retries = 0;
    let error = RigModel::new(&settings)
        .unwrap()
        .chat_completion("question", &AtomicBool::new(false))
        .await
        .unwrap_err();
    assert_eq!(error.code, "AGENT_RESPONSE_EMPTY");
    assert_eq!(thread.join().unwrap().len(), 1);
}

#[tokio::test]
async fn configured_recovery_budgets_and_reasoning_reach_the_provider() {
    use oj_agent::agent::config::ReasoningMode;
    let (endpoint, thread) = server(vec![
        (200, unfinished(None, Some("private"), "length")),
        (200, unfinished(None, Some("private"), "length")),
        (200, answer("final")),
    ]);
    let mut settings = config(endpoint, Provider::DeepSeek);
    settings.request.max_output_tokens = 96;
    settings.request.recovery_output_tokens = 128;
    settings.request.empty_response_retries = 2;
    settings.request.reasoning = ReasoningMode::Low;
    settings.request.disable_thinking_on_recovery = false;
    assert_eq!(
        RigModel::new(&settings)
            .unwrap()
            .chat_completion("question", &AtomicBool::new(false))
            .await
            .unwrap(),
        "final"
    );
    let requests = thread.join().unwrap();
    assert_eq!(requests.len(), 3);
    for (request, limit) in requests.iter().zip([96, 128, 128]) {
        let body = request_body(request);
        assert_eq!(body["max_tokens"], limit);
        assert_eq!(body["thinking"]["type"], "enabled");
        assert_eq!(body["reasoning_effort"], "low");
    }
}

#[tokio::test]
async fn configured_timeout_is_enforced_without_unrequested_retries() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
    let thread = std::thread::spawn(move || {
        let (_stream, _) = listener.accept().unwrap();
        std::thread::sleep(Duration::from_millis(1200));
    });
    let mut settings = config(endpoint, Provider::Compatible);
    settings.request.timeout_secs = 1;
    settings.request.max_retries = 0;
    let started = Instant::now();
    let error = RigModel::new(&settings)
        .unwrap()
        .chat_completion("question", &AtomicBool::new(false))
        .await
        .unwrap_err();
    assert_eq!(error.code, "MODEL_TIMEOUT");
    assert!(error.message.contains("1 秒"));
    assert!(error.message.contains("1 次"));
    assert!(started.elapsed() < Duration::from_secs(2));
    thread.join().unwrap();
}

#[tokio::test]
async fn configured_long_backoff_is_still_cancellable() {
    let (endpoint, thread) = server(vec![(
        503,
        json!({"error":{"message":"Unavailable"}}).to_string(),
    )]);
    let mut settings = config(endpoint, Provider::Compatible);
    settings.request.retry_delay_ms = 30000;
    let cancel = AtomicBool::new(false);
    let started = Instant::now();
    let error = RigModel::new(&settings)
        .unwrap()
        .chat_completion_with_progress(
            "question",
            &cancel,
            |_| Ok(()),
            |_| cancel.store(true, Ordering::Relaxed),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, "CANCELLED");
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(thread.join().unwrap().len(), 1);
}

#[tokio::test]
async fn compatible_and_deepseek_use_a_common_validated_interface() {
    for provider in [Provider::Compatible, Provider::DeepSeek] {
        let (endpoint, thread) = server(vec![(200, answer(&valid())), (200, answer(&valid()))]);
        let model = RigModel::new(&config(endpoint, provider)).unwrap();
        let output = model
            .analyze(&Default::default(), &AtomicBool::new(false))
            .await
            .unwrap();
        assert_eq!(output.attempts.len(), 1);
        assert_eq!(output.analysis.problem.values, vec![2, 7, 11, 15]);
        let mut usage = None;
        model
            .chat_completion_observed("usage", &AtomicBool::new(false), |value| {
                usage = Some(oj_agent::usage::TokenUsage::from(value));
                Ok(())
            })
            .await
            .unwrap();
        assert_eq!(usage.unwrap().total, Some(30));
        let requests = thread.join().unwrap();
        assert!(requests[0].starts_with("POST /v1/chat/completions"));
    }
}
#[tokio::test]
async fn authentication_failure_is_not_retried_or_logged_with_secret() {
    let (endpoint, thread) = server(vec![(
        401,
        json!({"error":{"message":"test-key-only","type":"authentication_error"}}).to_string(),
    )]);
    let error = RigModel::new(&config(endpoint, Provider::Compatible))
        .unwrap()
        .analyze(&Default::default(), &AtomicBool::new(false))
        .await
        .err()
        .unwrap();
    assert!(!error.to_string().contains("test-key-only"));
    assert_eq!(thread.join().unwrap().len(), 1);
}
#[tokio::test]
async fn transient_failures_are_bounded_to_three_network_attempts() {
    let body = json!({"error":{"message":"Unavailable"}}).to_string();
    let (endpoint, thread) = server(vec![(503, body.clone()), (503, body.clone()), (503, body)]);
    assert!(
        RigModel::new(&config(endpoint, Provider::Compatible))
            .unwrap()
            .analyze(&Default::default(), &AtomicBool::new(false))
            .await
            .is_err()
    );
    assert_eq!(thread.join().unwrap().len(), 3);
}
#[tokio::test]
async fn repairs_are_bounded_and_preserve_previous_responses() {
    let (endpoint, thread) = server(vec![
        (200, answer("broken")),
        (200, answer("{}")),
        (200, answer(&valid())),
    ]);
    let output = RigModel::new(&config(endpoint, Provider::Compatible))
        .unwrap()
        .analyze(&Default::default(), &AtomicBool::new(false))
        .await
        .unwrap();
    assert_eq!(output.attempts.len(), 3);
    assert!(output.attempts[0].validation_error.is_some());
    assert!(output.attempts[2].validation_error.is_none());
    assert_eq!(thread.join().unwrap().len(), 3);
    let (endpoint, thread) = server(vec![(200, answer("broken")); 3]);
    let error = RigModel::new(&config(endpoint, Provider::Compatible))
        .unwrap()
        .analyze(&Default::default(), &AtomicBool::new(false))
        .await
        .err()
        .unwrap();
    assert_eq!(error.code, "AGENT_REPAIR_FAILED");
    assert!(error.context.contains_key("attempts"));
    assert_eq!(thread.join().unwrap().len(), 3);
}
#[tokio::test]
async fn cancellation_interrupts_pending_network_io() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
    let cancel = Arc::new(AtomicBool::new(false));
    let signal = cancel.clone();
    let thread = std::thread::spawn(move || {
        let (_stream, _) = listener.accept().unwrap();
        signal.store(true, Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(150));
    });
    let started = Instant::now();
    let error = RigModel::new(&config(endpoint, Provider::Compatible))
        .unwrap()
        .analyze(&Default::default(), &cancel)
        .await
        .err()
        .unwrap();
    assert_eq!(error.code, "CANCELLED");
    assert!(started.elapsed() < Duration::from_secs(2));
    thread.join().unwrap();
}
#[tokio::test]
async fn openai_uses_the_responses_protocol() {
    let body=json!({"id":"resp_test","object":"response","created_at":1,"status":"completed","model":"test-model","output":[{"id":"msg_test","type":"message","status":"completed","role":"assistant","content":[{"type":"output_text","text":valid(),"annotations":[]}]}],"usage":{"input_tokens":10,"output_tokens":20,"total_tokens":30}}).to_string();
    let (endpoint, thread) = server(vec![(200, body.clone()), (200, body)]);
    let mut settings = config(endpoint, Provider::OpenAi);
    settings.request.max_output_tokens = 2048;
    settings.request.reasoning = oj_agent::agent::config::ReasoningMode::Low;
    let model = RigModel::new(&settings).unwrap();
    let result = model
        .analyze(&Default::default(), &AtomicBool::new(false))
        .await;
    assert!(
        result.is_ok(),
        "{}",
        result.err().map(|e| e.to_string()).unwrap_or_default()
    );
    let mut usage = None;
    model
        .chat_completion_observed("usage", &AtomicBool::new(false), |value| {
            usage = Some(oj_agent::usage::TokenUsage::from(value));
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(usage.unwrap().input, Some(10));
    for request in thread.join().unwrap() {
        assert!(request.starts_with("POST /v1/responses"));
        let body = request_body(&request);
        assert_eq!(body["max_output_tokens"], 2048);
        assert_eq!(body["reasoning"]["effort"], "low");
    }
}
#[tokio::test]
async fn unsupported_model_analysis_does_not_fabricate_an_animation() {
    use oj_agent::{
        session::{SessionStore, SqliteStore},
        tools,
    };
    let body=json!({"explanation":"No supported template matches this flow network.","problem":ProblemInput::default(),"supported":false}).to_string();
    let (endpoint, thread) = server(vec![(200, answer(&body))]);
    let mut store = SqliteStore::memory().unwrap();
    let id = store.create(&Default::default()).unwrap();
    let result = tools::generate(
        &mut store,
        &id,
        &Default::default(),
        Some(&config(endpoint, Provider::Compatible)),
        &AtomicBool::new(false),
        |_| {},
    )
    .await;
    assert_eq!(result.unwrap_err().code, "ALGORITHM_UNSUPPORTED");
    assert!(store.active(&id, "animation").unwrap().is_none());
    assert!(store.active(&id, "analysis").unwrap().is_some());
    assert_eq!(thread.join().unwrap().len(), 1);
}
