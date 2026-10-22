//! Saved request policy, shared by every provider and snapshotted for each turn.
use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ReasoningMode {
    #[default]
    ProviderDefault,
    Off,
    Low,
    High,
    Max,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct RequestConfig {
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub max_output_tokens: u64,
    pub recovery_output_tokens: u64,
    pub empty_response_retries: u32,
    pub max_rounds: u32,
    pub reasoning: ReasoningMode,
    pub disable_thinking_on_recovery: bool,
}
impl Default for RequestConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 90,
            max_retries: 2,
            retry_delay_ms: 500,
            max_output_tokens: 8192,
            recovery_output_tokens: 16384,
            empty_response_retries: 1,
            max_rounds: 8,
            reasoning: ReasoningMode::ProviderDefault,
            disable_thinking_on_recovery: true,
        }
    }
}
impl RequestConfig {
    pub fn validate(&self) -> Result<()> {
        for (label, value, min, max) in [
            ("单次超时（秒）", self.timeout_secs, 1, 1800),
            ("额外重试次数", self.max_retries as u64, 0, 5),
            ("重试间隔（毫秒）", self.retry_delay_ms, 0, 30000),
            ("输出 Token 上限", self.max_output_tokens, 1, 262144),
            (
                "恢复输出 Token 上限",
                self.recovery_output_tokens,
                1,
                262144,
            ),
            ("无正文恢复次数", self.empty_response_retries as u64, 0, 5),
            ("每条消息最大轮次", self.max_rounds as u64, 1, 32),
        ] {
            if !(min..=max).contains(&value) {
                return Err(AppError::new(
                    "MODEL_CONFIG_INVALID",
                    "Configuration",
                    "ModelSettings",
                    format!("{label}必须在 {min}–{max} 之间。"),
                )
                .recoverable());
            }
        }
        if self.recovery_output_tokens < self.max_output_tokens {
            return Err(AppError::new(
                "MODEL_CONFIG_INVALID",
                "Configuration",
                "ModelSettings",
                "恢复输出 Token 上限不能小于普通输出上限。",
            )
            .recoverable());
        }
        Ok(())
    }
}
