use crate::{
    agent::Provider,
    animation::Player,
    chat::{self, AnimationBundle, Attachment},
    error::{AppError, Result},
    ir::*,
    renderer::RasterSize,
    session::{SessionStore, SqliteStore},
    settings::ModelSettings,
    style::StyleConfig,
};
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::{
    cell::RefCell,
    collections::HashMap,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};
slint::include_modules!();
mod appearance;
#[cfg(test)]
mod concurrency_tests;
mod conversations;
mod jobs;
mod model_settings;
mod preview;

pub fn data_dir() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("OJ_DATA_DIR").filter(|p| !p.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    directories::ProjectDirs::from("org", "oj-agent", "OjAgent")
        .map(|d| d.data_local_dir().to_path_buf())
        .ok_or_else(|| {
            AppError::new(
                "CONFIG_DATA_DIR",
                "Configuration",
                "Startup",
                "Cannot locate application data directory.",
            )
        })
}
pub fn database_path() -> Result<PathBuf> {
    let path = data_dir()?;
    std::fs::create_dir_all(&path).map_err(|e| AppError::io("Startup", e))?;
    Ok(path.join("sessions.sqlite3"))
}
enum Event {
    Preview {
        animation: Arc<AnimationIr>,
        epoch: u64,
        frame: Result<preview::Pixels>,
    },
    Progress(uuid::Uuid, String),
    ChatFinished(uuid::Uuid, Result<()>),
    UsageChanged(String),
    Attached {
        session: String,
        request: uuid::Uuid,
        result: Result<Vec<Attachment>>,
    },
    Exported(uuid::Uuid, Result<PathBuf>),
}
struct State {
    model: ModelSettings,
    animation: Option<Arc<AnimationIr>>,
    bundle: Option<AnimationBundle>,
    animation_artifact: String,
    conversations: HashMap<String, conversations::Conversation>,
    preview: preview::PreviewWorker,
    preview_epoch: u64,
    style: Arc<StyleConfig>,
    preview_size: Option<RasterSize>,
    pending_resize: Option<(RasterSize, Instant)>,
    player: Player,
    jobs: jobs::Jobs,
    last: Instant,
    last_frame: f64,
    rx: mpsc::Receiver<Event>,
    tx: mpsc::Sender<Event>,
    db: PathBuf,
}
fn show_error(ui: &AppWindow, error: AppError) {
    ui.set_error_text(error.to_string().into());
    ui.set_status_text(
        if error.kind == "Cancelled" {
            "已停止"
        } else {
            "操作未完成"
        }
        .into(),
    );
}
fn provider_index(provider: &Provider) -> i32 {
    match provider {
        Provider::OpenAi => 0,
        Provider::DeepSeek => 1,
        Provider::Compatible => 2,
    }
}
fn provider_from_index(index: i32) -> Provider {
    match index {
        1 => Provider::DeepSeek,
        2 => Provider::Compatible,
        _ => Provider::OpenAi,
    }
}
fn session_label(title: &str) -> String {
    title.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn refresh_sessions(ui: &AppWindow, state: &State) -> Result<()> {
    let items = SqliteStore::open(&state.db)?
        .list()?
        .into_iter()
        .map(|s| SessionItem {
            busy: state.jobs.busy(&s.id),
            id: s.id.into(),
            title: session_label(&s.title).into(),
            status: s.status.into(),
        })
        .collect::<Vec<_>>();
    ui.set_sessions(ModelRc::new(VecModel::from(items)));
    Ok(())
}
fn refresh_chat(ui: &AppWindow, state: &State) -> Result<()> {
    let messages = if ui.get_selected_session().is_empty() {
        vec![]
    } else {
        SqliteStore::open(&state.db)?.messages(&ui.get_selected_session())?
    };
    let rows = messages
        .into_iter()
        .map(|m| {
            let blocks = chat::markdown::parse(&m.markdown)
                .into_iter()
                .map(|b| MarkdownBlock {
                    kind: b.kind.into(),
                    lines: b.text.lines().count().max(1) as i32,
                    raw: b.text.into(),
                    body: b.styled,
                })
                .collect::<Vec<_>>();
            let files = m
                .attachments
                .into_iter()
                .map(|a| AttachmentItem {
                    id: a.id.into(),
                    name: a.name.into(),
                })
                .collect::<Vec<_>>();
            ChatMessage {
                role: m.role.into(),
                blocks: ModelRc::new(VecModel::from(blocks)),
                files: ModelRc::new(VecModel::from(files)),
            }
        })
        .collect::<Vec<_>>();
    let changed = rows.len() != ui.get_messages().row_count();
    ui.set_messages(ModelRc::new(VecModel::from(rows)));
    if changed {
        ui.invoke_reveal_latest();
    }
    refresh_token_usage(ui, state)
}
fn refresh_token_usage(ui: &AppWindow, state: &State) -> Result<()> {
    let session = ui.get_selected_session();
    let text = if session.is_empty() {
        String::new()
    } else {
        SqliteStore::open(&state.db)?.token_usage(&session)?.label()
    };
    ui.set_token_usage(text.into());
    Ok(())
}
fn refresh_attachments(ui: &AppWindow, state: &State) {
    ui.set_attachments(ModelRc::new(VecModel::from(
        state
            .conversations
            .get(ui.get_selected_session().as_str())
            .into_iter()
            .flat_map(|view| view.attachments.iter())
            .map(|a| AttachmentItem {
                id: a.id.clone().into(),
                name: a.name.clone().into(),
            })
            .collect::<Vec<_>>(),
    )));
}
fn save_draft(ui: &AppWindow, state: &mut State) {
    state
        .conversations
        .entry(ui.get_selected_session().to_string())
        .or_default()
        .draft = ui.get_composer().to_string();
}
fn sync_current(ui: &AppWindow, state: &State) {
    let session = ui.get_selected_session();
    let view = state.conversations.get(session.as_str());
    ui.set_busy(state.jobs.busy(&session));
    ui.set_file_pending(view.is_some_and(|view| view.file_request.is_some()));
    ui.set_status_text(
        view.map(|view| view.status.as_str())
            .unwrap_or("就绪")
            .into(),
    );
    ui.set_error_text(view.map(|view| view.error.as_str()).unwrap_or("").into());
}
fn restore_draft(ui: &AppWindow, state: &State) {
    ui.set_composer(
        state
            .conversations
            .get(ui.get_selected_session().as_str())
            .map(|view| view.draft.as_str())
            .unwrap_or("")
            .into(),
    );
    refresh_attachments(ui, state);
    sync_current(ui, state);
}
fn switch_chat(ui: &AppWindow, state: &mut State, session: &str) -> Result<()> {
    save_draft(ui, state);
    state.animation = None;
    state.bundle = None;
    state.animation_artifact.clear();
    state.preview_epoch = state.preview_epoch.wrapping_add(1);
    state.player = Player::default();
    ui.set_selected_session(session.into());
    ui.set_animation_open(false);
    ui.set_playing(false);
    restore_draft(ui, state);
    refresh_chat(ui, state)?;
    ui.invoke_reveal_latest();
    Ok(())
}
fn pick_files(ui: &AppWindow, state: &mut State) {
    let session = ui.get_selected_session().to_string();
    let view = state.conversations.entry(session.clone()).or_default();
    if view.file_request.is_some() || state.jobs.busy(&session) {
        return;
    }
    let request = uuid::Uuid::new_v4();
    view.file_request = Some(request);
    sync_current(ui, state);
    let tx = state.tx.clone();
    std::thread::spawn(move || {
        let files = rfd::FileDialog::new()
            .set_title("添加到对话：UTF-8 文本或源文件")
            .pick_files()
            .unwrap_or_default();
        let result = if files.len() > 8 {
            Err(AppError::new(
                "ATTACHMENT_LIMIT",
                "Input",
                "Chat",
                "At most eight attachments per message.",
            ))
        } else {
            files.iter().map(|p| chat::files::attach(p)).collect()
        };
        let _ = tx.send(Event::Attached {
            session,
            request,
            result,
        });
    });
}
fn select_case(ui: &AppWindow, state: &mut State, index: usize) -> Result<()> {
    let bundle = state
        .bundle
        .as_ref()
        .ok_or_else(|| AppError::validation("Animation", "No animation is open."))?;
    let case = bundle
        .cases
        .get(index)
        .ok_or_else(|| AppError::validation("Animation", "Unknown case."))?;
    case.animation.validate()?;
    state.animation = Some(Arc::new(case.animation.clone()));
    state.player = Player::default();
    state.preview_epoch = state.preview_epoch.wrapping_add(1);
    state.preview_size = None;
    ui.set_case_index(index as i32);
    ui.set_animation_open(true);
    show_frame(ui, state)
}
fn show_frame(ui: &AppWindow, state: &mut State) -> Result<()> {
    update_preview_size(ui, state);
    if let Some(animation) = &state.animation {
        let size = state.preview_size.unwrap_or(RasterSize::new(
            animation.scene.width,
            animation.scene.height,
        )?);
        state.preview.request(
            animation.clone(),
            state.player.time,
            state.preview_epoch,
            size,
            state.style.clone(),
        )?;
        ui.set_position((state.player.time / animation.duration) as f32);
        ui.set_time_label(format!("{:.1} / {:.1} s", state.player.time, animation.duration).into());
        ui.set_playing(state.player.playing);
        state.last_frame = state.player.time;
    }
    Ok(())
}
fn update_preview_size(ui: &AppWindow, state: &mut State) {
    if !ui.get_animation_open() {
        return;
    }
    let dpi = ui.window().scale_factor().max(0.1);
    ui.set_preview_scale_factor(dpi);
    let Some(animation) = &state.animation else {
        return;
    };
    let Some(size) = RasterSize::fit(
        (animation.scene.width, animation.scene.height),
        (ui.get_preview_width(), ui.get_preview_height()),
        dpi,
    ) else {
        return;
    };
    if state.preview_size == Some(size) {
        state.pending_resize = None;
        return;
    }
    let now = Instant::now();
    if state.preview_size.is_some() {
        match state.pending_resize {
            Some((pending, since)) if pending == size => {
                if now.duration_since(since) < Duration::from_millis(120) {
                    return;
                }
            }
            _ => {
                state.pending_resize = Some((size, now));
                return;
            }
        }
    }
    state.preview_size = Some(size);
    state.pending_resize = None;
    state.preview_epoch = state.preview_epoch.wrapping_add(1);
    state.last_frame = -1.;
}

fn bind_conversation_navigation(ui: &AppWindow, state: Rc<RefCell<State>>) {
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_clear_error(move || {
            if let Some(ui) = weak.upgrade() {
                if let Some(view) = state
                    .borrow_mut()
                    .conversations
                    .get_mut(ui.get_selected_session().as_str())
                {
                    view.error.clear();
                }
                ui.set_error_text("".into());
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_cancel(move || {
            if let Some(ui) = weak.upgrade() {
                let mut state = state.borrow_mut();
                let session = ui.get_selected_session().to_string();
                if state.jobs.stop(&session) {
                    let view = state.conversations.entry(session).or_default();
                    view.error.clear();
                    view.status = "已停止，可以继续对话或切换会话".into();
                }
                sync_current(&ui, &state);
                let _ = refresh_sessions(&ui, &state);
                let _ = refresh_token_usage(&ui, &state);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_new_session(move || {
            if let Some(ui) = weak.upgrade()
                && let Err(error) = switch_chat(&ui, &mut state.borrow_mut(), "")
            {
                show_error(&ui, error);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_select_session(move |id| {
            if let Some(ui) = weak.upgrade() {
                let mut s = state.borrow_mut();
                if let Err(error) = switch_chat(&ui, &mut s, &id) {
                    show_error(&ui, error);
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_delete_session(move || {
            if let Some(ui) = weak.upgrade() {
                let mut s = state.borrow_mut();
                let session = ui.get_selected_session().to_string();
                let result = SqliteStore::open(&s.db).and_then(|mut store| store.delete(&session));
                match result {
                    Ok(()) => {
                        s.jobs.stop(&session);
                        if let Err(error) = switch_chat(&ui, &mut s, "") {
                            show_error(&ui, error);
                        }
                        s.conversations.remove(&session);
                        let _ = refresh_sessions(&ui, &s);
                    }
                    Err(error) => show_error(&ui, error),
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_open_link(move |link| {
            if let Some(ui) = weak.upgrade() {
                let mut s = state.borrow_mut();
                let result = (|| {
                    let id = link
                        .strip_prefix("oj-animation:")
                        .filter(|id| uuid::Uuid::parse_str(id).is_ok())
                        .ok_or_else(|| {
                            AppError::new(
                                "LINK_NOT_LOCAL_ANIMATION",
                                "Input",
                                "Chat",
                                "Only registered local animation links open here.",
                            )
                        })?;
                    let bundle =
                        SqliteStore::open(&s.db)?.chat_animation(&ui.get_selected_session(), id)?;
                    if bundle.cases.is_empty() {
                        return Err(AppError::validation("Animation", "Animation has no cases."));
                    }
                    ui.set_animation_title(bundle.title.clone().into());
                    ui.set_animation_basis(
                        format!("验证对象：{} · 通过提供的测试；并非穷尽证明", bundle.basis).into(),
                    );
                    ui.set_case_names(ModelRc::new(VecModel::from(
                        bundle
                            .cases
                            .iter()
                            .map(|c| format!("{} · {}", c.name, c.category).into())
                            .collect::<Vec<slint::SharedString>>(),
                    )));
                    s.animation_artifact = id.into();
                    s.bundle = Some(bundle);
                    select_case(&ui, &mut s, 0)
                })();
                if let Err(error) = result {
                    show_error(&ui, error);
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_select_case(move |index| {
            if let Some(ui) = weak.upgrade()
                && let Err(error) = select_case(&ui, &mut state.borrow_mut(), index as usize)
            {
                show_error(&ui, error);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_back_to_chat(move || {
            if let Some(ui) = weak.upgrade() {
                let mut s = state.borrow_mut();
                s.player.playing = false;
                ui.set_playing(false);
                ui.set_animation_open(false);
            }
        });
    }
}

fn finish_job(ui: &AppWindow, state: &mut State, job: uuid::Uuid, result: Result<String>) {
    let Some(session) = state.jobs.finish(job) else {
        let _ = refresh_sessions(ui, state);
        return;
    };
    let view = state.conversations.entry(session.clone()).or_default();
    match result {
        Ok(status) => {
            view.status = status;
            view.error.clear();
        }
        Err(error) => {
            view.status = if error.kind == "Cancelled" {
                "已停止"
            } else {
                "操作未完成"
            }
            .into();
            view.error = error.to_string();
        }
    }
    if ui.get_selected_session() == session {
        sync_current(ui, state);
        let _ = refresh_chat(ui, state);
    }
    let _ = refresh_sessions(ui, state);
}
fn handle_event(ui: &AppWindow, s: &mut State, event: Event) {
    match event {
        Event::Preview {
            animation,
            epoch,
            frame,
        } => {
            if s.preview_epoch == epoch
                && s.animation
                    .as_ref()
                    .is_some_and(|a| Arc::ptr_eq(a, &animation))
            {
                match frame {
                    Ok(pixels) => {
                        ui.set_frame_pixel_width(pixels.width() as i32);
                        ui.set_frame_pixel_height(pixels.height() as i32);
                        ui.set_frame(slint::Image::from_rgba8_premultiplied(pixels));
                    }
                    Err(error) => {
                        s.player.playing = false;
                        s.preview_epoch = s.preview_epoch.wrapping_add(1);
                        show_error(ui, error);
                    }
                }
            }
        }
        Event::Progress(job, message) => {
            let Some(session) = s.jobs.owner(job).map(str::to_owned) else {
                return;
            };
            s.conversations.entry(session.clone()).or_default().status = message;
            if ui.get_selected_session() == session {
                sync_current(ui, s);
                let _ = refresh_chat(ui, s);
            }
        }
        Event::ChatFinished(job, result) => {
            finish_job(ui, s, job, result.map(|()| "回复完成".into()));
        }
        Event::UsageChanged(session) => {
            if ui.get_selected_session() == session
                && let Err(error) = refresh_token_usage(ui, s)
            {
                show_error(ui, error);
            }
        }
        Event::Attached {
            session,
            request,
            result,
        } => {
            if ui.get_selected_session() == session {
                save_draft(ui, s);
            }
            let Some(view) = s
                .conversations
                .get_mut(&session)
                .filter(|view| view.file_request == Some(request))
            else {
                return;
            };
            view.file_request = None;
            match result {
                Ok(files) => {
                    if view.attachments.len() + files.len() > 8 {
                        view.error =
                            AppError::validation("Attachment", "At most eight files per message.")
                                .to_string();
                    } else {
                        if !files.is_empty() && view.draft.ends_with('@') {
                            view.draft.pop();
                        }
                        for file in files {
                            if !view
                                .attachments
                                .iter()
                                .any(|a| a.sha256 == file.sha256 && a.name == file.name)
                            {
                                view.draft.push_str(&format!("@{} ", file.name));
                                view.attachments.push(file);
                            }
                        }
                    }
                }
                Err(error) => {
                    view.error = error.to_string();
                }
            }
            if ui.get_selected_session() == session {
                restore_draft(ui, s);
            }
        }
        Event::Exported(job, result) => {
            finish_job(
                ui,
                s,
                job,
                result.map(|path| format!("已导出 {}", path.display())),
            );
        }
    }
}

fn bind_chat_send(ui: &AppWindow, state: Rc<RefCell<State>>) {
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_send_message(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            if ui.get_busy() {
                return;
            }
            let config = state.borrow().model.model_config();
            if let Err(error) = config.validate() {
                ui.set_style_open(false);
                ui.invoke_open_model();
                ui.set_settings_error(error.to_string().into());
                show_error(&ui, error);
                return;
            }
            let mut s = state.borrow_mut();
            let previous_session = ui.get_selected_session().to_string();
            if s.jobs.busy(&previous_session) || s.conversations.get(&previous_session).is_some_and(|view| view.file_request.is_some()) { return; }
            let attachments = s.conversations.entry(previous_session.clone()).or_default().attachments.clone();
            let text = ui.get_composer().trim().to_string();
            if text.is_empty() && attachments.is_empty() {
                return;
            }
            if text.len() > 100_000 {
                show_error(&ui, AppError::validation("Chat", "Message exceeds 100 KB."));
                return;
            }
            let result = (|| {
                let title = session_label(&text).chars().take(32).collect::<String>();
                let message = chat::Message::new(
                    "user",
                    if text.is_empty() {
                        "请查看附件。".into()
                    } else {
                        text
                    },
                    attachments,
                );
                if serde_json::to_vec(&message).map_err(|e|AppError::io("Chat",e))?.len() > 900_000 {
                    return Err(AppError::new("CHAT_CONTEXT_LIMIT","Input","Chat","This message and its attachments are too large; remove or shorten some files."));
                }
                let mut store = SqliteStore::open(&s.db)?;
                let session = if ui.get_selected_session().is_empty() {
                    store.create_chat(if title.is_empty() { "附件讨论" } else { &title })?
                } else { ui.get_selected_session().to_string() };
                store.append_message(&session, &message)?;
                store.checkpoint(&session, "Chat", "processing")?;
                ui.set_selected_session(session.clone().into());
                Ok((session, message.id))
            })();
            let (session, request) = match result {
                Ok(session) => session,
                Err(error) => {
                    show_error(&ui, error);
                    return;
                }
            };
            if previous_session != session && let Some(view) = s.conversations.remove(&previous_session) {
                s.conversations.insert(session.clone(), view);
            }
            let view = s.conversations.entry(session.clone()).or_default();
            view.attachments.clear();
            view.draft.clear();
            view.error.clear();
            view.status = "AI 正在回复…".into();
            refresh_attachments(&ui, &s);
            ui.set_composer("".into());
            let job = s.jobs.begin(&session).expect("the current conversation has no active job");
            sync_current(&ui, &s);
            let _ = refresh_chat(&ui, &s);
            let _ = refresh_sessions(&ui, &s);
            let cancel = job.cancel;
            let job_id = job.id;
            let tx = s.tx.clone();
            let db = s.db.clone();
            std::thread::spawn(move || {
                let result = (|| {
                    let mut store = SqliteStore::open(&db)?;
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all().build().map_err(|e| AppError::io("Chat", e))?;
                    let result = runtime.block_on(chat::respond(
                        &mut store,
                        &session,
                        &config,
                        cancel,
                        |p| {
                            let _ = tx.send(Event::Progress(job_id, p.into()));
                        },
                    ));
                    // Dropping a runtime can otherwise wait for an unrelated blocking HTTP task.
                    runtime.shutdown_background();
                    if let Err(error) = &result {
                        let _ = store.record_chat_error(&session, &request, error);
                    }
                    result.map(|_| ())
                })();
                let _ = tx.send(Event::UsageChanged(session));
                let _ = tx.send(Event::ChatFinished(job_id, result));
            });
        });
    }
}

pub fn run() -> anyhow::Result<()> {
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("skia".into())
        .select()?;
    let ui = AppWindow::new()?;
    let style_path = std::env::var_os("OJ_STYLE_CONFIG")
        .map(PathBuf::from)
        .unwrap_or(data_dir()?.join("style.json"));
    let (style, style_error) = match StyleConfig::load(&style_path) {
        Ok(mut style) => match appearance::load_fonts(&mut style) {
            Ok(()) => (style, None),
            Err(error) => (StyleConfig::default(), Some(error)),
        },
        Err(error) => (StyleConfig::default(), Some(error)),
    };
    appearance::apply(&ui, &style);
    let model_path = std::env::var_os("OJ_MODEL_CONFIG")
        .map(PathBuf::from)
        .unwrap_or(data_dir()?.join("model.json"));
    let (model, model_error) = match ModelSettings::load(&model_path) {
        Ok(model) => (model, None),
        Err(error) => (ModelSettings::default(), Some(error)),
    };
    model_settings::apply(&ui, &model);
    ui.set_model_config_path(model_path.display().to_string().into());
    let db = database_path()?;
    SqliteStore::open(&db)?.recover_interrupted()?;
    let (tx, rx) = mpsc::channel();
    let state = Rc::new(RefCell::new(State {
        model,
        animation: None,
        bundle: None,
        animation_artifact: String::new(),
        conversations: HashMap::default(),
        preview: preview::PreviewWorker::new(tx.clone()),
        preview_epoch: 0,
        style: Arc::new(style),
        preview_size: None,
        pending_resize: None,
        player: Player::default(),
        jobs: jobs::Jobs::default(),
        last: Instant::now(),
        last_frame: -1.,
        rx,
        tx,
        db,
    }));
    refresh_sessions(&ui, &state.borrow())?;
    appearance::bind(&ui, state.clone(), style_path);
    if let Some(error) = style_error.or(model_error) {
        show_error(&ui, error);
    }
    model_settings::bind(&ui, state.clone(), model_path);
    bind_chat_send(&ui, state.clone());
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_attach_files(move || {
            if let Some(ui) = weak.upgrade() {
                pick_files(&ui, &mut state.borrow_mut());
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_composer_edited(move || {
            if let Some(ui) = weak.upgrade() {
                let value = ui.get_composer();
                if let Some(prefix) = value.strip_suffix('@')
                    && prefix.chars().last().is_none_or(char::is_whitespace)
                {
                    pick_files(&ui, &mut state.borrow_mut());
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_remove_attachment(move |id| {
            if let Some(ui) = weak.upgrade() {
                let mut s = state.borrow_mut();
                s.conversations
                    .entry(ui.get_selected_session().to_string())
                    .or_default()
                    .attachments
                    .retain(|a| a.id != id.as_str());
                refresh_attachments(&ui, &s);
            }
        });
    }
    bind_conversation_navigation(&ui, state.clone());
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_play_pause(move || {
            if let Some(ui) = weak.upgrade() {
                let mut s = state.borrow_mut();
                s.player.playing = !s.player.playing;
                s.last = Instant::now();
                ui.set_playing(s.player.playing);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_step(move |forward| {
            if let Some(ui) = weak.upgrade() {
                let mut s = state.borrow_mut();
                if let Some(a) = s.animation.clone() {
                    s.preview_epoch = s.preview_epoch.wrapping_add(1);
                    s.player.step(forward, &a);
                    if let Err(error) = show_frame(&ui, &mut s) {
                        show_error(&ui, error);
                    }
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_restart(move || {
            if let Some(ui) = weak.upgrade() {
                let mut s = state.borrow_mut();
                s.player.time = 0.;
                s.player.playing = false;
                s.preview_epoch = s.preview_epoch.wrapping_add(1);
                if let Err(error) = show_frame(&ui, &mut s) {
                    show_error(&ui, error);
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_seek(move |pos| {
            if let Some(ui) = weak.upgrade() {
                let mut s = state.borrow_mut();
                if let Some(a) = s.animation.clone() {
                    s.preview_epoch = s.preview_epoch.wrapping_add(1);
                    s.player.seek(pos as f64 * a.duration, &a);
                    if let Err(error) = show_frame(&ui, &mut s) {
                        show_error(&ui, error);
                    }
                }
            }
        });
    }
    {
        let state = state.clone();
        ui.on_change_speed(move |i| {
            state.borrow_mut().player.speed = match i {
                0 => 0.5,
                2 => 1.5,
                3 => 2.,
                _ => 1.,
            };
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_export_gif(move || {
            if let Some(ui) = weak.upgrade()
                && !ui.get_busy()
            {
                let mut s = state.borrow_mut();
                if let Some(a) = s.animation.clone() {
                    let session = ui.get_selected_session().to_string();
                    let Some(job) = s.jobs.begin(&session) else {
                        return;
                    };
                    let cancel = job.cancel;
                    let job_id = job.id;
                    let path = PathBuf::from(ui.get_export_path().as_str());
                    let style = s.style.clone();
                    let tx = s.tx.clone();
                    let db = s.db.clone();
                    let parent = s.animation_artifact.clone();
                    let case = ui.get_case_index();
                    let view = s.conversations.entry(session.clone()).or_default();
                    view.status = "正在导出 GIF…".into();
                    view.error.clear();
                    sync_current(&ui, &s);
                    let _ = refresh_sessions(&ui, &s);
                    std::thread::spawn(move || {
                        let result =
                            crate::export::gif_cancellable_styled(&a, &path, 30, &cancel, &style)
                                .and_then(|_| {
                                    SqliteStore::open(&db)?.add_artifact(
                                &session,
                                "export",
                                &serde_json::json!({"path":path,"format":"gif","case_index":case}),
                                &[parent],
                                true,
                            )?;
                                    Ok(path)
                                });
                        let _ = tx.send(Event::Exported(job_id, result));
                    });
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_export_json(move || {
            if let Some(ui) = weak.upgrade() {
                let s = state.borrow();
                if let Some(a) = &s.animation {
                    let path = PathBuf::from(ui.get_export_path().as_str()).with_extension("json");
                    let result = serde_json::to_vec_pretty(a)
                        .map_err(|e| AppError::io("Export", e))
                        .and_then(|bytes| {
                            std::fs::write(&path, bytes).map_err(|e| AppError::io("Export", e))
                        });
                    match result {
                        Ok(()) => ui.set_status_text(format!("已保存 {}", path.display()).into()),
                        Err(error) => show_error(&ui, error),
                    }
                }
            }
        });
    }
    let timer = slint::Timer::default();
    let weak = ui.as_weak();
    let tick_state = state.clone();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_nanos(16_666_667),
        move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let mut s = tick_state.borrow_mut();
            update_preview_size(&ui, &mut s);
            while let Ok(event) = s.rx.try_recv() {
                handle_event(&ui, &mut s, event);
            }
            let now = Instant::now();
            let elapsed = now.duration_since(s.last).as_secs_f64();
            s.last = now;
            if ui.get_animation_open()
                && !ui.get_busy()
                && let Some(a) = s.animation.clone()
            {
                s.player.tick(elapsed, &a);
                if s.player.time != s.last_frame
                    && let Err(error) = show_frame(&ui, &mut s)
                {
                    s.player.playing = false;
                    show_error(&ui, error);
                }
            }
        },
    );
    ui.run()?;
    state.borrow_mut().jobs.stop_all();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_labels_stay_on_one_line() {
        assert_eq!(
            session_label("第一行标题\n第二行标题"),
            "第一行标题 第二行标题"
        );
        assert_eq!(session_label("  spaced\t title  "), "spaced title");
        assert_eq!(session_label("\n"), "");
    }

    #[test]
    fn animation_round_trips_keep_composer_and_delete_action_inside_the_window() {
        use i_slint_backend_testing::ElementHandle;
        i_slint_backend_testing::init_no_event_loop();
        let ui = AppWindow::new().unwrap();
        ui.set_selected_session("one".into());
        ui.set_composer("尚未发送的草稿".into());
        let usage = "本会话 Token：输入 ≥123456 · 输出 ≥654321 · 合计 ≥777777 · 缓存命中 ≥12345 · 缓存写入 ≥23456 · 推理 ≥34567（含未上报用量）";
        ui.set_token_usage(usage.into());
        ui.set_sessions(ModelRc::new(VecModel::from(
            (0..40)
                .map(|i| SessionItem {
                    id: i.to_string().into(),
                    title: format!("会话 {i}").into(),
                    status: "ready".into(),
                    busy: false,
                })
                .collect::<Vec<_>>(),
        )));
        let blocks = crate::chat::markdown::parse(&"段落与较长的回复内容。".repeat(120))
            .into_iter()
            .map(|block| MarkdownBlock {
                kind: block.kind.into(),
                raw: block.text.into(),
                body: block.styled,
                lines: 1,
            })
            .collect::<Vec<_>>();
        ui.set_messages(ModelRc::new(VecModel::from(vec![ChatMessage {
            role: "assistant".into(),
            blocks: ModelRc::new(VecModel::from(blocks)),
            files: Default::default(),
        }])));
        ui.show().unwrap();
        for (width, height) in [(1320., 860.), (960., 640.), (1080., 720.), (1320., 860.)] {
            ui.set_animation_open(true);
            ui.window()
                .dispatch_event(slint::platform::WindowEvent::Resized {
                    size: slint::LogicalSize::new(width, height),
                });
            // Measure the animation before returning, so both page layouts are exercised.
            assert!(ui.get_preview_width() > 1. && ui.get_preview_height() > 1.);
            i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(200));
            slint::platform::update_timers_and_animations();
            ui.set_animation_open(false);
            for label in ["消息输入", "删除对话", "发送 ↑", usage] {
                let element = ElementHandle::find_by_accessible_label(&ui, label)
                    .next()
                    .unwrap_or_else(|| panic!("Missing {label}"));
                let position = element.absolute_position();
                let size = element.size();
                assert!(
                    size.width > 1. && size.height > 1.,
                    "Collapsed {label}: {size:?}"
                );
                assert!(
                    position.x >= 0.
                        && position.y >= 0.
                        && position.x + size.width <= width + 1.
                        && position.y + size.height <= height + 1.,
                    "Offscreen {label}: {position:?} {size:?} in {width}×{height}"
                );
            }
            assert_eq!(ui.get_composer(), "尚未发送的草稿");
        }
        ui.hide().unwrap();
    }

    #[test]
    fn navigation_and_stop_callbacks_preserve_chat_and_unlock_other_conversations() {
        i_slint_backend_testing::init_no_event_loop();
        let directory = tempfile::tempdir().unwrap();
        let db = directory.path().join("sessions.sqlite3");
        let mut store = SqliteStore::open(&db).unwrap();
        let first = store.create_chat("first").unwrap();
        let second = store.create_chat("second").unwrap();
        let (algorithm, trace) = crate::trace::simulate(&ProblemInput::default()).unwrap();
        let plan = crate::visualization::plan(&algorithm, &trace).unwrap();
        let animation = crate::animation::compile(&algorithm, &trace, &plan).unwrap();
        let bundle = AnimationBundle {
            title: "fixture".into(),
            verification_id: "fixture".into(),
            basis: "test fixture".into(),
            cases: vec![chat::AnimationCase {
                name: "normal".into(),
                category: "normal".into(),
                animation,
            }],
        };
        let artifact = store
            .add_artifact(
                &first,
                "chat-animation",
                &serde_json::json!(bundle),
                &[],
                true,
            )
            .unwrap();
        store
            .append_message(
                &first,
                &chat::Message::new(
                    "assistant",
                    format!("[打开动画](oj-animation:{artifact})"),
                    vec![],
                ),
            )
            .unwrap();
        let (tx, rx) = mpsc::channel();
        let state = Rc::new(RefCell::new(State {
            model: ModelSettings::default(),
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
        }));
        let ui = AppWindow::new().unwrap();
        bind_conversation_navigation(&ui, state.clone());
        ui.invoke_select_session(first.clone().into());
        ui.set_composer("保留的草稿".into());
        for _ in 0..3 {
            ui.invoke_open_link(format!("oj-animation:{artifact}").into());
            assert!(ui.get_animation_open());
            ui.invoke_back_to_chat();
            assert!(!ui.get_animation_open());
            assert_eq!(ui.get_selected_session(), first);
            assert_eq!(ui.get_composer(), "保留的草稿");
            assert_eq!(ui.get_messages().row_count(), 1);
        }
        let stopped = state.borrow_mut().jobs.begin(&first).unwrap();
        state
            .borrow_mut()
            .conversations
            .entry(first.clone())
            .or_default()
            .status = "first pending".into();
        sync_current(&ui, &state.borrow());
        ui.invoke_select_session(second.clone().into());
        assert_eq!(ui.get_selected_session(), second);
        assert!(!ui.get_busy());
        assert!(!stopped.cancel.load(std::sync::atomic::Ordering::Relaxed));
        ui.set_composer("第二个会话的草稿".into());
        let next = state.borrow_mut().jobs.begin(&second).unwrap();
        state
            .borrow_mut()
            .conversations
            .entry(second.clone())
            .or_default()
            .status = "second pending".into();
        sync_current(&ui, &state.borrow());
        assert!(ui.get_busy());
        ui.invoke_new_session();
        assert!(!ui.get_busy());
        ui.set_composer("新对话的草稿".into());
        assert!(state.borrow().jobs.busy(&first) && state.borrow().jobs.busy(&second));
        ui.invoke_select_session(first.clone().into());
        assert!(ui.get_busy());
        assert_eq!(ui.get_composer(), "保留的草稿");
        assert_eq!(ui.get_status_text(), "first pending");
        ui.invoke_cancel();
        assert!(!ui.get_busy());
        assert!(stopped.cancel.load(std::sync::atomic::Ordering::Relaxed));
        assert!(!next.cancel.load(std::sync::atomic::Ordering::Relaxed));
        ui.invoke_select_session(second.clone().into());
        assert_eq!(ui.get_selected_session(), second);
        assert_eq!(ui.get_composer(), "第二个会话的草稿");
        handle_event(
            &ui,
            &mut state.borrow_mut(),
            Event::Progress(stopped.id, "stale".into()),
        );
        handle_event(
            &ui,
            &mut state.borrow_mut(),
            Event::ChatFinished(stopped.id, Err(crate::agent::cancelled())),
        );
        assert!(ui.get_busy());
        assert_eq!(ui.get_status_text(), "second pending");
        // Finish B while A is selected: B's response must not replace A's messages or draft.
        ui.invoke_select_session(first.clone().into());
        store
            .append_message(
                &second,
                &chat::Message::new("assistant", "second reply".into(), vec![]),
            )
            .unwrap();
        handle_event(
            &ui,
            &mut state.borrow_mut(),
            Event::ChatFinished(next.id, Ok(())),
        );
        assert_eq!(ui.get_selected_session(), first);
        assert_eq!(ui.get_composer(), "保留的草稿");
        assert_eq!(ui.get_messages().row_count(), 1);
        ui.invoke_select_session(second.clone().into());
        assert!(!ui.get_busy());
        assert_eq!(ui.get_status_text(), "回复完成");
        assert_eq!(ui.get_messages().row_count(), 1);
        ui.invoke_delete_session();
        assert_eq!(ui.get_selected_session(), "");
        assert_eq!(store.list().unwrap().len(), 1);
        assert!(store.chat_animation(&first, &artifact).is_ok());
        assert_eq!(ui.get_composer(), "新对话的草稿");
    }
}
