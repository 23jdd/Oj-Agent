use oj_agent::{
    agent::{ModelConfig, Provider},
    chat::{self, Message},
    session::{SessionStore, SqliteStore},
};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::Arc,
    thread,
    time::Duration,
};

const SOURCE: &str = "function solve(input,emit){ const a=[...input]; emit({values:a,explanation:'Initial'}); a.sort((x,y)=>x-y);emit({values:a,explanation:'Sorted'});return a;}";
fn request() -> Value {
    json!({"source":SOURCE,"cases":[
        {"name":"mixed","category":"normal","input":[3,1,2],"expected":[1,2,3]},
        {"name":"empty","category":"boundary","input":[],"expected":[]},
        {"name":"duplicates and negatives","category":"adversarial","input":[2,-1,2,-1],"expected":[-1,-1,2,2]}
    ]})
}
fn mock(
    count: usize,
    response: impl Fn(usize, &Value) -> Value + Send + 'static,
) -> (ModelConfig, thread::JoinHandle<Vec<Value>>) {
    mock_metered(count, move |index, request| {
        (response(index, request), None)
    })
}
fn mock_metered(
    count: usize,
    response: impl Fn(usize, &Value) -> (Value, Option<Value>) + Send + 'static,
) -> (ModelConfig, thread::JoinHandle<Vec<Value>>) {
    mock_raw(count, move |index, request| {
        let (turn, usage) = response(index, request);
        let mut body = json!({"id":"chat","object":"chat.completion","created":0,"model":"mock","choices":[{"index":0,"message":{"role":"assistant","content":turn.to_string()},"finish_reason":"stop"}]});
        if let Some(usage) = usage {
            body["usage"] = usage;
        }
        body
    })
}
fn mock_raw(
    count: usize,
    response: impl Fn(usize, &Value) -> Value + Send + 'static,
) -> (ModelConfig, thread::JoinHandle<Vec<Value>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
    let thread = thread::spawn(move || {
        let mut requests = Vec::new();
        for index in 0..count {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 8192];
            let (header, size) = loop {
                let n = socket.read(&mut buffer).unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buffer[..n]);
                if let Some(position) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let header = String::from_utf8_lossy(&bytes[..position]);
                    let length = header
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    break (position + 4, length);
                }
            };
            while bytes.len() < header + size {
                let n = socket.read(&mut buffer).unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buffer[..n]);
            }
            let request: Value = serde_json::from_slice(&bytes[header..header + size]).unwrap();
            let body = response(index, &request).to_string();
            write!(socket,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
            requests.push(request);
        }
        requests
    });
    (
        ModelConfig {
            provider: Provider::Compatible,
            model: "mock".into(),
            endpoint,
            api_key: "test-only-key".into(),
            request: Default::default(),
        },
        thread,
    )
}

fn reported_usage() -> Value {
    json!({"prompt_tokens":10,"completion_tokens":20,"total_tokens":30,
        "prompt_tokens_details":{"cached_tokens":4},"completion_tokens_details":{"reasoning_tokens":7}})
}

#[tokio::test]
async fn configured_round_limit_stops_repairs_and_is_explained_to_the_model() {
    let mut store = SqliteStore::memory().unwrap();
    let session = store.create_chat("round limit").unwrap();
    store
        .append_message(&session, &Message::new("user", "question".into(), vec![]))
        .unwrap();
    let (mut config, server) = mock(1, |_, _| json!({"invalid":true}));
    config.request.max_rounds = 1;
    let error = chat::respond(&mut store, &session, &config, Arc::default(), |_| {})
        .await
        .unwrap_err();
    assert_eq!(error.code, "CHAT_TOOL_LIMIT");
    assert!(error.message.contains("1-round"));
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].to_string().contains("within 1 rounds"));
}

#[tokio::test]
async fn recovery_saves_only_the_final_answer_and_counts_usage_from_both_requests() {
    let mut store = SqliteStore::memory().unwrap();
    let session = store.create_chat("empty response recovery").unwrap();
    store
        .append_message(&session, &Message::new("user", "question".into(), vec![]))
        .unwrap();
    let (mut config, server) = mock_raw(2, |index, _| {
        let message = if index == 0 {
            json!({"role":"assistant","content":null,"reasoning_content":"private-reasoning-not-a-chat-message"})
        } else {
            json!({"role":"assistant","content":json!({"markdown":"完整正文。","tool_calls":[],"done":true}).to_string()})
        };
        json!({"id":"chat","object":"chat.completion","created":0,"model":"mock",
            "choices":[{"index":0,"message":message,"finish_reason":if index==0 {"length"} else {"stop"}}],
            "usage":reported_usage()})
    });
    config.provider = Provider::DeepSeek;
    let answer = chat::respond(&mut store, &session, &config, Arc::default(), |_| {})
        .await
        .unwrap();
    assert_eq!(answer.markdown, "完整正文。");
    let messages = store.messages(&session).unwrap();
    assert_eq!(messages.len(), 2);
    assert!(
        !serde_json::to_string(&messages)
            .unwrap()
            .contains("private-reasoning-not-a-chat-message")
    );
    let usage = store.token_usage(&session).unwrap();
    assert_eq!(usage.responses, 2);
    assert_eq!(usage.total.tokens, 60);
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1]["thinking"]["type"], "disabled");
}

#[tokio::test]
async fn token_usage_counts_repairs_and_tool_rounds_without_double_counting_details() {
    let mut store = SqliteStore::memory().unwrap();
    let session = store.create_chat("usage").unwrap();
    store
        .append_message(&session, &Message::new("user", "verify".into(), vec![]))
        .unwrap();
    let (config, server) = mock_metered(3, |index, _| {
        let turn = match index {
            0 => json!({"invalid":"repair this response"}),
            1 => {
                json!({"markdown":"","tool_calls":[{"name":"sandbox.verify","arguments":request()}],"done":false})
            }
            _ => json!({"markdown":"已验证。","tool_calls":[],"done":true}),
        };
        (turn, Some(reported_usage()))
    });
    chat::respond(&mut store, &session, &config, Arc::default(), |_| {})
        .await
        .unwrap();
    let usage = store.token_usage(&session).unwrap();
    assert_eq!(usage.responses, 3);
    assert_eq!(usage.input.tokens, 30);
    assert_eq!(usage.output.tokens, 60);
    assert_eq!(usage.total.tokens, 90);
    assert_eq!(usage.cached_input.tokens, 12);
    assert_eq!(usage.reasoning.tokens, 21);
    assert!(!usage.label().contains('≥'));
    server.join().unwrap();
}

#[tokio::test]
async fn cancelled_or_failed_turns_keep_usage_already_returned_by_the_provider() {
    let mut store = SqliteStore::memory().unwrap();
    let session = store.create_chat("cancelled usage").unwrap();
    store
        .append_message(&session, &Message::new("user", "question".into(), vec![]))
        .unwrap();
    let (config, server) = mock_metered(1, |_, _| {
        (
            json!({"markdown":"reply","tool_calls":[],"done":true}),
            Some(reported_usage()),
        )
    });
    let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = cancel.clone();
    let error = chat::respond(&mut store, &session, &config, cancel, move |message| {
        if message == "AI 正在处理回复…" {
            flag.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    })
    .await
    .unwrap_err();
    assert_eq!(error.kind, "Cancelled");
    assert_eq!(store.messages(&session).unwrap().len(), 1);
    assert_eq!(store.token_usage(&session).unwrap().total.tokens, 30);
    server.join().unwrap();
    let failed = store.create_chat("failed usage").unwrap();
    store
        .append_message(&failed, &Message::new("user", "question".into(), vec![]))
        .unwrap();
    let (config, server) =
        mock_metered(8, |_, _| (json!({"invalid":true}), Some(reported_usage())));
    assert!(
        chat::respond(&mut store, &failed, &config, Arc::default(), |_| {})
            .await
            .is_err()
    );
    assert_eq!(store.token_usage(&failed).unwrap().total.tokens, 240);
    assert_eq!(store.token_usage(&session).unwrap().total.tokens, 30);
    server.join().unwrap();
}
fn prompt(request: &Value) -> Value {
    let messages = request["messages"].as_array().unwrap();
    let content = &messages.iter().rev().find(|m| m["role"] == "user").unwrap()["content"];
    let text = content.as_str().map(str::to_owned).unwrap_or_else(|| {
        content
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|p| p["text"].as_str())
            .collect::<String>()
    });
    serde_json::from_str(&text).unwrap()
}

#[test]
fn cancelled_request_cannot_mark_a_new_turn_failed_or_recreate_a_deleted_session() {
    let mut store = SqliteStore::memory().unwrap();
    let session = store.create_chat("first").unwrap();
    let first = Message::new("user", "first request".into(), vec![]);
    store.append_message(&session, &first).unwrap();
    store
        .record_chat_error(&session, &first.id, &oj_agent::agent::cancelled())
        .unwrap();
    assert_eq!(store.list().unwrap()[0].status, "cancelled");
    let next = Message::new("user", "next request".into(), vec![]);
    store.append_message(&session, &next).unwrap();
    store.checkpoint(&session, "Chat", "processing").unwrap();
    store
        .record_chat_error(&session, &first.id, &oj_agent::agent::cancelled())
        .unwrap();
    assert_eq!(store.list().unwrap()[0].status, "processing");
    store.delete(&session).unwrap();
    store
        .record_chat_error(&session, &next.id, &oj_agent::agent::cancelled())
        .unwrap();
    assert!(store.list().unwrap().is_empty());
}

#[tokio::test]
async fn cancelled_conversation_does_not_cancel_a_different_conversations_request() {
    let mut store = SqliteStore::memory().unwrap();
    let stopped = store.create_chat("stopped").unwrap();
    let stopped_message = Message::new("user", "stop this".into(), vec![]);
    store.append_message(&stopped, &stopped_message).unwrap();
    let cancel = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let (config, server) = mock(
        1,
        |_, _| json!({"markdown":"第二个会话正常回复。","tool_calls":[],"done":true}),
    );
    let error = chat::respond(&mut store, &stopped, &config, cancel, |_| {})
        .await
        .unwrap_err();
    assert_eq!(error.kind, "Cancelled");
    store
        .record_chat_error(&stopped, &stopped_message.id, &error)
        .unwrap();
    let next = store.create_chat("next").unwrap();
    store
        .append_message(&next, &Message::new("user", "answer this".into(), vec![]))
        .unwrap();
    let response = chat::respond(&mut store, &next, &config, Arc::default(), |_| {})
        .await
        .unwrap();
    assert!(response.markdown.contains("正常回复"));
    assert_eq!(store.messages(&stopped).unwrap().len(), 1);
    assert_eq!(store.messages(&next).unwrap().len(), 2);
    server.join().unwrap();
}
#[tokio::test]
async fn conversation_preserves_context_and_can_answer_without_animation() {
    let mut store = SqliteStore::memory().unwrap();
    let session = store.create_chat("chat").unwrap();
    store
        .append_message(&session, &Message::new("user", "前一个问题".into(), vec![]))
        .unwrap();
    store
        .append_message(
            &session,
            &Message::new("assistant", "前一个回答".into(), vec![]),
        )
        .unwrap();
    store
        .append_message(&session, &Message::new("user", "继续解释".into(), vec![]))
        .unwrap();
    let (config, server) = mock(1, |_, req| {
        assert_eq!(prompt(req)["messages"].as_array().unwrap().len(), 3);
        json!({"markdown":"## 解释\n\n**普通回答**，无需动画。","tool_calls":[],"done":true})
    });
    let result = chat::respond(&mut store, &session, &config, Arc::default(), |_| {})
        .await
        .unwrap();
    assert!(result.markdown.contains("普通回答"));
    assert!(!result.markdown.contains("oj-animation:"));
    assert!(store.artifacts(&session).unwrap().is_empty());
    assert_eq!(store.messages(&session).unwrap().len(), 4);
    let usage = store.token_usage(&session).unwrap();
    assert_eq!(usage.responses, 1);
    assert_eq!(usage.total.reported, 0);
    assert!(usage.label().contains("未上报"));
    server.join().unwrap();
}
#[tokio::test]
async fn ai_tools_verify_cases_publish_local_animation_and_restore_it() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("chat.db");
    let file = dir.path().join("sort.js");
    std::fs::write(&file, SOURCE).unwrap();
    let attachment = chat::files::attach(&file).unwrap();
    std::fs::write(&file, "changed after attachment").unwrap();
    assert_eq!(attachment.text, SOURCE);
    let mut store = SqliteStore::open(&db).unwrap();
    let session = store.create_chat("排序讨论").unwrap();
    store
        .append_message(
            &session,
            &Message::new("user", "@sort.js 检查并解释过程".into(), vec![attachment]),
        )
        .unwrap();
    let (config, server) = mock(3, |index, req| {
        let p = prompt(req);
        match index {
            0 => {
                assert_eq!(p["messages"][0]["attachments"][0]["text"], SOURCE);
                json!({"markdown":"","done":false,"tool_calls":[{"name":"sandbox.verify","arguments":request()}]})
            }
            1 => {
                let id = &p["tool_results"][0]["result"]["data"]["verification_id"];
                assert!(id.is_string());
                json!({"markdown":"","done":false,"tool_calls":[{"name":"animation.create","arguments":{"verification_id":id,"title":"排序过程","layout":"array"}}]})
            }
            _ => {
                let link = &p["tool_results"][1]["result"]["data"]["link"];
                assert!(link.as_str().unwrap().starts_with("oj-animation:"));
                json!({"markdown":format!("三个情况已检查。\n\n[打开动画]({})",link.as_str().unwrap()),"tool_calls":[],"done":true})
            }
        }
    });
    chat::respond(&mut store, &session, &config, Arc::default(), |_| {})
        .await
        .unwrap();
    server.join().unwrap();
    let artifact = store
        .artifacts(&session)
        .unwrap()
        .into_iter()
        .find(|a| a.kind == "chat-animation")
        .unwrap();
    let bundle = store.chat_animation(&session, &artifact.id).unwrap();
    assert_eq!(bundle.cases.len(), 3);
    assert!(bundle.basis.contains("原始 JavaScript"));
    for case in &bundle.cases {
        case.animation.validate().unwrap();
        assert_eq!(case.animation.header.version, 2);
    }
    let other = store.create_chat("other").unwrap();
    assert!(store.chat_animation(&other, &artifact.id).is_err());
    drop(store);
    let reopened = SqliteStore::open(&db).unwrap();
    assert!(
        reopened
            .messages(&session)
            .unwrap()
            .last()
            .unwrap()
            .markdown
            .contains(&artifact.id)
    );
    assert_eq!(
        reopened
            .chat_animation(&session, &artifact.id)
            .unwrap()
            .cases
            .len(),
        3
    );
}

#[tokio::test]
async fn fabricated_animation_links_are_repaired_before_display() {
    let mut store = SqliteStore::memory().unwrap();
    let session = store.create_chat("links").unwrap();
    store
        .append_message(&session, &Message::new("user", "解释一下".into(), vec![]))
        .unwrap();
    let (config, server) = mock(2, |index, request| {
        if index == 0 {
            json!({"markdown":"[假的动画](oj-animation:00000000-0000-0000-0000-000000000000)","tool_calls":[],"done":true})
        } else {
            assert!(
                prompt(request)["tool_results"][0]["error"]
                    .as_str()
                    .unwrap()
                    .contains("unregistered")
            );
            json!({"markdown":"这次只提供文字解释。","tool_calls":[],"done":true})
        }
    });
    let result = chat::respond(&mut store, &session, &config, Arc::default(), |_| {})
        .await
        .unwrap();
    assert!(!result.markdown.contains("oj-animation:"));
    assert_eq!(store.messages(&session).unwrap().len(), 2);
    server.join().unwrap();
}
#[test]
fn failed_or_insufficient_checks_cannot_be_published_and_attachments_are_bounded() {
    let request: oj_sandbox::Request = serde_json::from_value(request()).unwrap();
    let mut report = oj_sandbox::verify(&request, Arc::default()).unwrap();
    report.passed = false;
    assert!(
        chat::animation::compile_verified(
            &report,
            "v",
            "generated JS",
            "test",
            "array",
            &Default::default()
        )
        .is_err()
    );
    report.passed = true;
    report.cases.truncate(1);
    assert!(
        chat::animation::compile_verified(
            &report,
            "v",
            "generated JS",
            "test",
            "array",
            &Default::default()
        )
        .is_err()
    );
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("binary");
    std::fs::write(&file, [0, 1, 2]).unwrap();
    assert!(chat::files::attach(&file).is_err());
    std::fs::write(&file, vec![b'a'; 100_001]).unwrap();
    assert!(chat::files::attach(&file).is_err());
    assert!(
        chat::parse_turn(
            r#"{"markdown":"x","tool_calls":[{"name":"x","arguments":{}}],"done":true}"#
        )
        .is_err()
    );
    assert_eq!(
        chat::parse_turn(
            "Here is the answer:\n```json\n{\"markdown\":\"ok\",\"done\":true}\n```\nThanks"
        )
        .unwrap()
        .markdown,
        "ok"
    );
    assert_eq!(
        chat::parse_turn("prefix {\"markdown\":\"embedded\",\"done\":true} suffix")
            .unwrap()
            .markdown,
        "embedded"
    );
    assert!(
        !chat::parse_turn(r#"{"markdown":"intermediate"}"#)
            .unwrap()
            .done
    );
    assert!(chat::parse_turn(r#"{"tool_calls":[{"name":"sandbox.verify"}]}"#).is_ok());
    assert!(
        oj_agent::tools::invoke(
            "sandbox.verify",
            &json!({"source":"function solve(){return 1;}","cases":[{"name":"one","category":"normal","input":null,"expected":1}]})
        )["data"]["passed"]
            == true
    );
}
