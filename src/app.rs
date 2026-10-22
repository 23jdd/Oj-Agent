use crate::{
    agent::{ModelConfig, Provider},
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
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
slint::include_modules!();
mod appearance;
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
    Progress(String),
    ChatFinished(Result<()>),
    Attached(Result<Vec<Attachment>>),
    Exported(Result<PathBuf>),
}
struct State {
    animation: Option<Arc<AnimationIr>>,
    bundle: Option<AnimationBundle>,
    animation_artifact: String,
    attachments: Vec<Attachment>,
    file_pending: bool,
    preview: preview::PreviewWorker,
    preview_epoch: u64,
    style: Arc<StyleConfig>,
    preview_size: Option<RasterSize>,
    pending_resize: Option<(RasterSize, Instant)>,
    player: Player,
    cancel: Arc<AtomicBool>,
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
fn refresh_sessions(ui: &AppWindow, path: &Path) -> Result<()> {
    let items = SqliteStore::open(path)?
        .list()?
        .into_iter()
        .map(|s| SessionItem {
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
    Ok(())
}
fn refresh_attachments(ui: &AppWindow, state: &State) {
    ui.set_attachments(ModelRc::new(VecModel::from(
        state
            .attachments
            .iter()
            .map(|a| AttachmentItem {
                id: a.id.clone().into(),
                name: a.name.clone().into(),
            })
            .collect::<Vec<_>>(),
    )));
}
fn clear_chat(ui: &AppWindow, state: &mut State) {
    state.animation = None;
    state.bundle = None;
    state.animation_artifact.clear();
    state.attachments.clear();
    state.preview_epoch = state.preview_epoch.wrapping_add(1);
    state.player = Player::default();
    ui.set_selected_session("".into());
    ui.set_composer("".into());
    ui.set_animation_open(false);
    ui.set_messages(Default::default());
    ui.set_error_text("".into());
    refresh_attachments(ui, state);
}
fn pick_files(ui: &AppWindow, state: &mut State) {
    if state.file_pending || ui.get_busy() {
        return;
    }
    state.file_pending = true;
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
        let _ = tx.send(Event::Attached(result));
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
        Ok(style) => (style, None),
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
    ui.set_provider_index(provider_index(&model.provider));
    ui.set_model_name(model.model.clone().into());
    ui.set_endpoint(model.endpoint.clone().into());
    ui.set_api_key(model.api_key.clone().into());
    ui.set_model_config_path(model_path.display().to_string().into());
    let db = database_path()?;
    SqliteStore::open(&db)?.recover_interrupted()?;
    refresh_sessions(&ui, &db)?;
    let (tx, rx) = mpsc::channel();
    let state = Rc::new(RefCell::new(State {
        animation: None,
        bundle: None,
        animation_artifact: String::new(),
        attachments: vec![],
        file_pending: false,
        preview: preview::PreviewWorker::new(tx.clone()),
        preview_epoch: 0,
        style: Arc::new(style),
        preview_size: None,
        pending_resize: None,
        player: Player::default(),
        cancel: Arc::new(AtomicBool::new(false)),
        last: Instant::now(),
        last_frame: -1.,
        rx,
        tx,
        db,
    }));
    appearance::bind(&ui, state.clone(), style_path);
    if let Some(error) = style_error.or(model_error) {
        show_error(&ui, error);
    }
    {
        let weak = ui.as_weak();
        let path = model_path.clone();
        ui.on_save_model(move || {
            if let Some(ui) = weak.upgrade() {
                let settings = ModelSettings {
                    version: crate::settings::VERSION,
                    provider: provider_from_index(ui.get_provider_index()),
                    model: ui.get_model_name().into(),
                    endpoint: ui.get_endpoint().into(),
                    api_key: ui.get_api_key().into(),
                };
                match settings.save(&path) {
                    Ok(()) => {
                        ui.set_settings_open(false);
                        ui.set_settings_error("".into());
                        ui.set_error_text("".into());
                        ui.set_status_text("模型配置已保存".into());
                    }
                    Err(error) => ui.set_settings_error(error.to_string().into()),
                }
            }
        });
    }
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
            let config = ModelConfig {
                provider: provider_from_index(ui.get_provider_index()),
                model: ui.get_model_name().into(),
                endpoint: ui.get_endpoint().into(),
                api_key: ui.get_api_key().into(),
            };
            if let Err(error) = config.validate() {
                ui.set_style_open(false);
                ui.set_settings_open(true);
                show_error(&ui, error);
                return;
            }
            let mut s = state.borrow_mut();
            let text = ui.get_composer().trim().to_string();
            if text.is_empty() && s.attachments.is_empty() {
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
                    s.attachments.clone(),
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
                Ok(session)
            })();
            let session = match result {
                Ok(session) => session,
                Err(error) => {
                    show_error(&ui, error);
                    return;
                }
            };
            s.attachments.clear();
            refresh_attachments(&ui, &s);
            ui.set_composer("".into());
            ui.set_error_text("".into());
            ui.set_busy(true);
            ui.set_status_text("AI 正在回复…".into());
            let _ = refresh_chat(&ui, &s);
            let _ = refresh_sessions(&ui, &s.db);
            s.cancel = Arc::new(AtomicBool::new(false));
            let cancel = s.cancel.clone();
            let tx = s.tx.clone();
            let db = s.db.clone();
            std::thread::spawn(move || {
                let result = (|| {
                    let mut store = SqliteStore::open(&db)?;
                    let runtime =
                        tokio::runtime::Runtime::new().map_err(|e| AppError::io("Chat", e))?;
                    let result = runtime.block_on(chat::respond(
                        &mut store,
                        &session,
                        &config,
                        cancel,
                        |p| {
                            let _ = tx.send(Event::Progress(p.into()));
                        },
                    ));
                    if let Err(error) = &result {
                        let _ = store.record_error(&session, error);
                    }
                    result.map(|_| ())
                })();
                let _ = tx.send(Event::ChatFinished(result));
            });
        });
    }
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
                s.attachments.retain(|a| a.id != id.as_str());
                refresh_attachments(&ui, &s);
            }
        });
    }
    {
        let state = state.clone();
        ui.on_cancel(move || {
            state.borrow().cancel.store(true, Ordering::Relaxed);
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_new_session(move || {
            if let Some(ui) = weak.upgrade()
                && !ui.get_busy()
            {
                clear_chat(&ui, &mut state.borrow_mut());
                ui.set_status_text("新对话".into());
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_select_session(move |id| {
            if let Some(ui) = weak.upgrade()
                && !ui.get_busy()
            {
                let mut s = state.borrow_mut();
                clear_chat(&ui, &mut s);
                ui.set_selected_session(id);
                if let Err(error) = refresh_chat(&ui, &s) {
                    show_error(&ui, error);
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_delete_session(move || {
            if let Some(ui) = weak.upgrade()
                && !ui.get_busy()
            {
                let mut s = state.borrow_mut();
                let result = SqliteStore::open(&s.db)
                    .and_then(|mut store| store.delete(&ui.get_selected_session()));
                match result {
                    Ok(()) => {
                        clear_chat(&ui, &mut s);
                        let _ = refresh_sessions(&ui, &s.db);
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
                    s.cancel = Arc::new(AtomicBool::new(false));
                    let cancel = s.cancel.clone();
                    let path = PathBuf::from(ui.get_export_path().as_str());
                    let style = s.style.clone();
                    let tx = s.tx.clone();
                    let db = s.db.clone();
                    let session = ui.get_selected_session().to_string();
                    let parent = s.animation_artifact.clone();
                    let case = ui.get_case_index();
                    ui.set_busy(true);
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
                        let _ = tx.send(Event::Exported(result));
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
                                    show_error(&ui, error);
                                }
                            }
                        }
                    }
                    Event::Progress(message) => {
                        ui.set_status_text(message.into());
                        let _ = refresh_chat(&ui, &s);
                    }
                    Event::ChatFinished(result) => {
                        ui.set_busy(false);
                        match result {
                            Ok(()) => ui.set_status_text("回复完成".into()),
                            Err(error) => show_error(&ui, error),
                        }
                        let _ = refresh_chat(&ui, &s);
                        let _ = refresh_sessions(&ui, &s.db);
                    }
                    Event::Attached(result) => {
                        s.file_pending = false;
                        match result {
                            Ok(files) => {
                                if s.attachments.len() + files.len() > 8 {
                                    show_error(
                                        &ui,
                                        AppError::validation(
                                            "Attachment",
                                            "At most eight files per message.",
                                        ),
                                    );
                                } else {
                                    let mut composer = ui.get_composer().to_string();
                                    if composer.ends_with('@') {
                                        composer.pop();
                                    }
                                    for file in files {
                                        if !s
                                            .attachments
                                            .iter()
                                            .any(|a| a.sha256 == file.sha256 && a.name == file.name)
                                        {
                                            composer.push_str(&format!("@{} ", file.name));
                                            s.attachments.push(file);
                                        }
                                    }
                                    ui.set_composer(composer.into());
                                    refresh_attachments(&ui, &s);
                                }
                            }
                            Err(error) => show_error(&ui, error),
                        }
                    }
                    Event::Exported(result) => {
                        ui.set_busy(false);
                        match result {
                            Ok(path) => {
                                ui.set_status_text(format!("已导出 {}", path.display()).into())
                            }
                            Err(error) => show_error(&ui, error),
                        }
                    }
                }
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
    state.borrow().cancel.store(true, Ordering::Relaxed);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::session_label;

    #[test]
    fn session_labels_stay_on_one_line() {
        assert_eq!(
            session_label("第一行标题\n第二行标题"),
            "第一行标题 第二行标题"
        );
        assert_eq!(session_label("  spaced\t title  "), "spaced title");
        assert_eq!(session_label("\n"), "");
    }
}
