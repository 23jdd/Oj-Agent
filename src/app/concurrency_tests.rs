use super::*;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

fn state(db: PathBuf) -> Rc<RefCell<State>> {
    let (tx, rx) = mpsc::channel();
    Rc::new(RefCell::new(State {
        model: ModelSettings::default(),
        animation: None,
        bundle: None,
        animation_artifact: String::new(),
        conversations: HashMap::default(),
        preview: preview::PreviewWorker::new(tx.clone()),
        preview_epoch: 0,
        style: Arc::default(),
        preview_size: None,
        preview_view: None,
        pending_resize: None,
        player: Player::default(),
        jobs: jobs::Jobs::default(),
        last: Instant::now(),
        last_frame: -1.,
        rx,
        tx,
        db,
    }))
}

#[test]
fn clearing_an_error_survives_navigation_and_preserves_other_conversations_and_history() {
    i_slint_backend_testing::init_no_event_loop();
    let directory = tempfile::tempdir().unwrap();
    let db = directory.path().join("sessions.sqlite3");
    let mut store = SqliteStore::open(&db).unwrap();
    let first = store.create_chat("first").unwrap();
    let second = store.create_chat("second").unwrap();
    let error = AppError::new(
        "TEST_FAILURE",
        "ModelProvider",
        "Chat",
        "错误详情。".repeat(120),
    );
    store.record_error(&first, &error).unwrap();
    let state = state(db.clone());
    state
        .borrow_mut()
        .conversations
        .entry(first.clone())
        .or_default()
        .error = error.to_string();
    state
        .borrow_mut()
        .conversations
        .entry(second.clone())
        .or_default()
        .error = "second error".into();
    let job = state.borrow_mut().jobs.begin(&second).unwrap();
    let ui = AppWindow::new().unwrap();
    bind_conversation_navigation(&ui, state.clone());
    ui.invoke_select_session(first.clone().into());
    ui.set_composer("unsent draft".into());
    ui.show().unwrap();
    ui.window()
        .dispatch_event(slint::platform::WindowEvent::Resized {
            size: slint::LogicalSize::new(960., 640.),
        });
    let clear =
        i_slint_backend_testing::ElementHandle::find_by_accessible_label(&ui, "清除错误提示")
            .next()
            .unwrap();
    let position = clear.absolute_position();
    let size = clear.size();
    assert!(
        position.x >= 0.
            && position.y >= 0.
            && position.x + size.width <= 961.
            && position.y + size.height <= 641.
    );
    clear.invoke_accessible_default_action();
    assert!(ui.get_error_text().is_empty());
    assert!(state.borrow().conversations[&first].error.is_empty());
    assert_eq!(ui.get_composer(), "unsent draft");
    assert!(!job.cancel.load(std::sync::atomic::Ordering::Relaxed));
    assert!(state.borrow().jobs.busy(&second));
    ui.invoke_select_session(second.clone().into());
    assert_eq!(ui.get_error_text(), "second error");
    ui.invoke_select_session(first.clone().into());
    assert!(ui.get_error_text().is_empty());
    assert_eq!(ui.get_composer(), "unsent draft");
    let count: i64 = rusqlite::Connection::open(db)
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM errors WHERE session_id=?1",
            [&first],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
    show_error(&ui, AppError::validation("Chat", "new failure"));
    assert!(ui.get_error_text().contains("new failure"));
    ui.invoke_clear_error();
    assert!(ui.get_error_text().is_empty());
    ui.hide().unwrap();
}

#[test]
fn model_settings_save_validate_and_cancel_without_mutating_active_request_snapshots() {
    i_slint_backend_testing::init_no_event_loop();
    let directory = tempfile::tempdir().unwrap();
    let state = state(directory.path().join("sessions.sqlite3"));
    let ui = AppWindow::new().unwrap();
    let path = directory.path().join("model.json");
    model_settings::bind(&ui, state.clone(), path.clone());
    let snapshot = state.borrow().model.model_config();
    ui.invoke_open_model();
    ui.set_api_key("test-key-only".into());
    ui.set_timeout_secs("300".into());
    ui.set_max_output_tokens("12000".into());
    ui.set_recovery_output_tokens("24000".into());
    ui.set_max_retries("not a number".into());
    ui.invoke_save_model();
    assert!(ui.get_settings_open());
    assert!(ui.get_settings_error().contains("额外重试次数"));
    i_slint_backend_testing::ElementHandle::find_by_accessible_label(&ui, "清除模型设置错误提示")
        .next()
        .unwrap()
        .invoke_accessible_default_action();
    assert!(ui.get_settings_error().is_empty());
    assert_eq!(ui.get_max_retries(), "not a number");
    assert!(!path.exists());
    assert_eq!(state.borrow().model.request.timeout_secs, 90);
    ui.set_max_retries("4".into());
    ui.set_reasoning_index(2);
    ui.invoke_save_model();
    assert!(!ui.get_settings_open());
    let saved = ModelSettings::load(&path).unwrap();
    assert_eq!(saved.request.timeout_secs, 300);
    assert_eq!(saved.request.max_output_tokens, 12000);
    assert_eq!(saved.request, state.borrow().model.request);
    assert_eq!(snapshot.request.timeout_secs, 90);
    ui.invoke_open_model();
    ui.set_timeout_secs("1".into());
    ui.set_settings_open(false); // The cancel action only closes the draft.
    ui.invoke_open_model();
    assert_eq!(ui.get_timeout_secs(), "300");
    ui.invoke_reset_request();
    assert_eq!(ui.get_timeout_secs(), "90");
    assert_eq!(ui.get_api_key(), "test-key-only");
    assert_eq!(state.borrow().model.request.timeout_secs, 300);
    ui.set_busy(true); // Editing settings must remain possible during another reply.
    ui.show().unwrap();
    ui.window()
        .dispatch_event(slint::platform::WindowEvent::Resized {
            size: slint::LogicalSize::new(960., 640.),
        });
    for label in ["保存模型设置", "取消模型设置", "关闭窗口", "最小化"] {
        let element = i_slint_backend_testing::ElementHandle::find_by_accessible_label(&ui, label)
            .next()
            .unwrap();
        let position = element.absolute_position();
        let size = element.size();
        assert_eq!(element.accessible_enabled(), Some(true));
        assert!(
            position.x >= 0.
                && position.y >= 0.
                && position.x + size.width <= 961.
                && position.y + size.height <= 641.,
            "{label}: {position:?} {size:?}"
        );
    }
    ui.hide().unwrap();
}
fn read_request(socket: &mut TcpStream) -> String {
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut bytes = vec![];
    let mut buffer = [0; 4096];
    let (header, length) = loop {
        let count = socket.read(&mut buffer).unwrap();
        assert!(count > 0);
        bytes.extend_from_slice(&buffer[..count]);
        if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
            let length = String::from_utf8_lossy(&bytes[..end])
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|value| value.trim().parse::<usize>().unwrap())
                })
                .unwrap();
            break (end + 4, length);
        }
    };
    while bytes.len() < header + length {
        let count = socket.read(&mut buffer).unwrap();
        assert!(count > 0);
        bytes.extend_from_slice(&buffer[..count]);
    }
    String::from_utf8(bytes[header..header + length].to_vec()).unwrap()
}
fn reply(socket: &mut TcpStream, markdown: &str) {
    let content = serde_json::json!({"markdown":markdown,"tool_calls":[],"done":true}).to_string();
    let input = if markdown == "reply A" { 100 } else { 200 };
    let body = serde_json::json!({"id":"local","object":"chat.completion","created":0,"model":"mock",
        "choices":[{"index":0,"message":{"role":"assistant","content":content},"finish_reason":"stop"}],
        "usage":{"prompt_tokens":input,"completion_tokens":input/10,"total_tokens":input+input/10}}).to_string();
    write!(socket,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
}
fn finish_pending(ui: &AppWindow, state: &Rc<RefCell<State>>, session: &str) {
    let deadline = Instant::now() + Duration::from_secs(8);
    while state.borrow().jobs.busy(session) {
        assert!(
            Instant::now() < deadline,
            "worker did not finish: {session}"
        );
        let event = state.borrow().rx.recv_timeout(Duration::from_millis(100));
        if let Ok(event) = event {
            handle_event(ui, &mut state.borrow_mut(), event);
        }
    }
}

#[test]
fn second_conversation_receives_a_reply_while_first_network_request_is_still_pending() {
    i_slint_backend_testing::init_no_event_loop();
    let directory = tempfile::tempdir().unwrap();
    let db = directory.path().join("sessions.sqlite3");
    let mut store = SqliteStore::open(&db).unwrap();
    let first = store.create_chat("A").unwrap();
    let second = store.create_chat("B").unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let mut release = Some(release_rx);
        let mut handlers = vec![];
        for index in 0..2 {
            let (mut socket, _) = listener.accept().unwrap();
            let body = read_request(&mut socket);
            assert!(body.contains(if index == 0 { "request A" } else { "request B" }));
            started_tx.send(index).unwrap();
            if index == 0 {
                let release = release.take().unwrap();
                handlers.push(std::thread::spawn(move || {
                    let _ = release.recv_timeout(Duration::from_secs(8));
                    reply(&mut socket, "reply A");
                }));
            } else {
                reply(&mut socket, "reply B");
            }
        }
        for handler in handlers {
            handler.join().unwrap();
        }
    });
    let state = state(db);
    let ui = AppWindow::new().unwrap();
    ui.set_provider_index(2);
    ui.set_model_name("mock".into());
    ui.set_endpoint(endpoint.into());
    ui.set_api_key("local-test-key".into());
    state.borrow_mut().model = model_settings::read(&ui).unwrap();
    bind_conversation_navigation(&ui, state.clone());
    bind_chat_send(&ui, state.clone());
    ui.show().unwrap();
    ui.invoke_select_session(first.clone().into());
    ui.set_composer("request A".into());
    ui.invoke_send_message();
    assert_eq!(started_rx.recv_timeout(Duration::from_secs(5)).unwrap(), 0);
    assert!(ui.get_busy());
    for label in ["+ 新对话", "B", "删除对话"] {
        let element = i_slint_backend_testing::ElementHandle::find_by_accessible_label(&ui, label)
            .next()
            .unwrap();
        assert_eq!(
            element.accessible_enabled(),
            Some(true),
            "{label} must remain usable while A responds"
        );
    }
    ui.invoke_select_session(second.clone().into());
    assert!(!ui.get_busy());
    ui.set_composer("request B".into());
    ui.invoke_send_message();
    assert_eq!(started_rx.recv_timeout(Duration::from_secs(5)).unwrap(), 1);
    assert!(state.borrow().jobs.busy(&first) && state.borrow().jobs.busy(&second));
    ui.invoke_select_session(first.clone().into());
    finish_pending(&ui, &state, &second);
    assert!(ui.get_busy(), "B finishing must not unlock A");
    assert_eq!(ui.get_selected_session(), first);
    assert_eq!(store.messages(&first).unwrap().len(), 1);
    assert_eq!(store.token_usage(&first).unwrap().responses, 0);
    assert_eq!(store.token_usage(&second).unwrap().total.tokens, 220);
    assert_eq!(store.messages(&second).unwrap()[1].markdown, "reply B");
    ui.invoke_select_session(second.clone().into());
    assert!(!ui.get_busy());
    assert!(ui.get_token_usage().contains("合计 220"));
    ui.set_composer("B unsent draft".into());
    release_tx.send(()).unwrap();
    finish_pending(&ui, &state, &first);
    assert_eq!(ui.get_composer(), "B unsent draft");
    assert_eq!(ui.get_status_text(), "回复完成");
    assert_eq!(ui.get_messages().row_count(), 2);
    assert!(ui.get_token_usage().contains("合计 220"));
    ui.invoke_select_session(first.clone().into());
    assert!(!ui.get_busy());
    assert!(ui.get_token_usage().contains("合计 110"));
    assert_eq!(store.messages(&first).unwrap()[1].markdown, "reply A");
    server.join().unwrap();
}

#[test]
fn delayed_attachments_errors_and_deletion_stay_with_their_own_conversation() {
    i_slint_backend_testing::init_no_event_loop();
    let directory = tempfile::tempdir().unwrap();
    let db = directory.path().join("sessions.sqlite3");
    let mut store = SqliteStore::open(&db).unwrap();
    let first = store.create_chat("A").unwrap();
    let second = store.create_chat("B").unwrap();
    let state = state(db);
    let ui = AppWindow::new().unwrap();
    bind_conversation_navigation(&ui, state.clone());
    ui.invoke_select_session(first.clone().into());
    ui.set_composer("A @".into());
    let request = uuid::Uuid::new_v4();
    state
        .borrow_mut()
        .conversations
        .entry(first.clone())
        .or_default()
        .file_request = Some(request);
    ui.invoke_select_session(second.clone().into());
    ui.set_composer("B draft".into());
    let attachment = Attachment {
        id: "a".into(),
        name: "a.rs".into(),
        text: "source A".into(),
        sha256: "hash A".into(),
    };
    handle_event(
        &ui,
        &mut state.borrow_mut(),
        Event::Attached {
            session: first.clone(),
            request,
            result: Ok(vec![attachment]),
        },
    );
    assert_eq!(ui.get_composer(), "B draft");
    assert_eq!(ui.get_attachments().row_count(), 0);
    ui.invoke_select_session(first.clone().into());
    assert_eq!(ui.get_composer(), "A @a.rs ");
    assert_eq!(ui.get_attachments().row_count(), 1);
    let first_job = state.borrow_mut().jobs.begin(&first).unwrap();
    let second_job = state.borrow_mut().jobs.begin(&second).unwrap();
    ui.invoke_select_session(second.clone().into());
    handle_event(
        &ui,
        &mut state.borrow_mut(),
        Event::ChatFinished(first_job.id, Err(AppError::validation("Chat", "A failed"))),
    );
    assert!(ui.get_error_text().is_empty());
    assert!(ui.get_busy());
    ui.invoke_select_session(first.clone().into());
    assert!(ui.get_error_text().contains("A failed"));
    let deleted_job = state.borrow_mut().jobs.begin(&first).unwrap();
    ui.invoke_delete_session();
    assert!(
        deleted_job
            .cancel
            .load(std::sync::atomic::Ordering::Relaxed)
    );
    assert!(!second_job.cancel.load(std::sync::atomic::Ordering::Relaxed));
    handle_event(
        &ui,
        &mut state.borrow_mut(),
        Event::Attached {
            session: first.clone(),
            request,
            result: Ok(vec![]),
        },
    );
    assert!(!state.borrow().conversations.contains_key(&first));
    assert!(state.borrow().jobs.busy(&second));
    state.borrow_mut().jobs.stop_all();
}
