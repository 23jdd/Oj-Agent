use super::*;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

fn state(db: PathBuf) -> Rc<RefCell<State>> {
    let (tx, rx) = mpsc::channel();
    Rc::new(RefCell::new(State {
        animation: None,
        bundle: None,
        animation_artifact: String::new(),
        conversations: HashMap::default(),
        preview: preview::PreviewWorker::new(tx.clone()),
        preview_epoch: 0,
        style: Arc::default(),
        preview_size: None,
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
    let body = serde_json::json!({"id":"local","object":"chat.completion","created":0,"model":"mock",
        "choices":[{"index":0,"message":{"role":"assistant","content":content},"finish_reason":"stop"}]}).to_string();
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
    assert_eq!(store.messages(&second).unwrap()[1].markdown, "reply B");
    ui.invoke_select_session(second.clone().into());
    assert!(!ui.get_busy());
    ui.set_composer("B unsent draft".into());
    release_tx.send(()).unwrap();
    finish_pending(&ui, &state, &first);
    assert_eq!(ui.get_composer(), "B unsent draft");
    assert_eq!(ui.get_status_text(), "回复完成");
    assert_eq!(ui.get_messages().row_count(), 2);
    ui.invoke_select_session(first.clone().into());
    assert!(!ui.get_busy());
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
