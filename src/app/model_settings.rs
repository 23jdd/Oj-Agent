//! Model settings are a draft until validation and atomic persistence succeed.
use super::*;
use crate::agent::config::{ReasoningMode, RequestConfig};

fn number<T: std::str::FromStr>(value: slint::SharedString, label: &str) -> Result<T> {
    value.trim().parse().map_err(|_| {
        AppError::new(
            "MODEL_CONFIG_INVALID",
            "Configuration",
            "ModelSettings",
            format!("{label}必须是有效的非负整数。"),
        )
        .recoverable()
    })
}
pub(super) fn read(ui: &AppWindow) -> Result<ModelSettings> {
    let reasoning = match ui.get_reasoning_index() {
        0 => ReasoningMode::ProviderDefault,
        1 => ReasoningMode::Off,
        2 => ReasoningMode::Low,
        3 => ReasoningMode::High,
        4 => ReasoningMode::Max,
        _ => {
            return Err(AppError::validation(
                "ModelSettings",
                "请选择有效的思考强度。",
            ));
        }
    };
    let settings = ModelSettings {
        version: crate::settings::VERSION,
        provider: provider_from_index(ui.get_provider_index()),
        model: ui.get_model_name().trim().into(),
        endpoint: ui.get_endpoint().trim().into(),
        api_key: ui.get_api_key().trim().into(),
        request: RequestConfig {
            timeout_secs: number(ui.get_timeout_secs(), "单次超时")?,
            max_retries: number(ui.get_max_retries(), "额外重试次数")?,
            retry_delay_ms: number(ui.get_retry_delay_ms(), "重试间隔")?,
            max_output_tokens: number(ui.get_max_output_tokens(), "输出 Token 上限")?,
            recovery_output_tokens: number(ui.get_recovery_output_tokens(), "恢复输出 Token 上限")?,
            empty_response_retries: number(ui.get_empty_response_retries(), "无正文恢复次数")?,
            max_rounds: number(ui.get_max_rounds(), "每条消息最大轮次")?,
            reasoning,
            disable_thinking_on_recovery: ui.get_recovery_thinking_off(),
        },
    };
    settings.validate()?;
    Ok(settings)
}
fn apply_request(ui: &AppWindow, request: &RequestConfig) {
    ui.set_timeout_secs(request.timeout_secs.to_string().into());
    ui.set_max_retries(request.max_retries.to_string().into());
    ui.set_retry_delay_ms(request.retry_delay_ms.to_string().into());
    ui.set_max_output_tokens(request.max_output_tokens.to_string().into());
    ui.set_recovery_output_tokens(request.recovery_output_tokens.to_string().into());
    ui.set_empty_response_retries(request.empty_response_retries.to_string().into());
    ui.set_max_rounds(request.max_rounds.to_string().into());
    ui.set_reasoning_index(match request.reasoning {
        ReasoningMode::ProviderDefault => 0,
        ReasoningMode::Off => 1,
        ReasoningMode::Low => 2,
        ReasoningMode::High => 3,
        ReasoningMode::Max => 4,
    });
    ui.set_recovery_thinking_off(request.disable_thinking_on_recovery);
}
pub(super) fn apply(ui: &AppWindow, settings: &ModelSettings) {
    ui.set_provider_index(provider_index(&settings.provider));
    ui.set_model_name(settings.model.clone().into());
    ui.set_endpoint(settings.endpoint.clone().into());
    ui.set_api_key(settings.api_key.clone().into());
    apply_request(ui, &settings.request);
}
pub(super) fn bind(ui: &AppWindow, state: Rc<RefCell<State>>, path: PathBuf) {
    let weak = ui.as_weak();
    let saved = state.clone();
    ui.on_open_model(move || {
        if let Some(ui) = weak.upgrade() {
            apply(&ui, &saved.borrow().model);
            ui.set_style_open(false);
            ui.set_settings_error("".into());
            ui.set_settings_open(true);
        }
    });
    let weak = ui.as_weak();
    ui.on_reset_request(move || {
        if let Some(ui) = weak.upgrade() {
            apply_request(&ui, &RequestConfig::default());
            ui.set_settings_error("".into());
        }
    });
    let weak = ui.as_weak();
    ui.on_save_model(move || {
        if let Some(ui) = weak.upgrade() {
            match read(&ui).and_then(|settings| {
                settings.save(&path)?;
                Ok(settings)
            }) {
                Ok(settings) => {
                    apply(&ui, &settings);
                    state.borrow_mut().model = settings;
                    ui.set_settings_open(false);
                    ui.set_settings_error("".into());
                    ui.set_error_text("".into());
                    ui.set_status_text("模型配置已保存，后续请求使用新配置".into());
                }
                Err(error) => ui.set_settings_error(error.to_string().into()),
            }
        }
    });
}
