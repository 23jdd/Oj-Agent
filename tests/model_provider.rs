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
    }
}
fn valid() -> String {
    json!({"explanation":"Hash map, O(n).","problem":ProblemInput::default(),"supported":true})
        .to_string()
}

#[tokio::test]
async fn compatible_and_deepseek_use_a_common_validated_interface() {
    for provider in [Provider::Compatible, Provider::DeepSeek] {
        let (endpoint, thread) = server(vec![(200, answer(&valid()))]);
        let output = RigModel::new(&config(endpoint, provider))
            .unwrap()
            .analyze(&Default::default(), &AtomicBool::new(false))
            .await
            .unwrap();
        assert_eq!(output.attempts.len(), 1);
        assert_eq!(output.analysis.problem.values, vec![2, 7, 11, 15]);
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
    let (endpoint, thread) = server(vec![(200, body)]);
    let result = RigModel::new(&config(endpoint, Provider::OpenAi))
        .unwrap()
        .analyze(&Default::default(), &AtomicBool::new(false))
        .await;
    assert!(
        result.is_ok(),
        "{}",
        result.err().map(|e| e.to_string()).unwrap_or_default()
    );
    assert!(thread.join().unwrap()[0].starts_with("POST /v1/responses"));
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
