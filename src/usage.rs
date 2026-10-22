//! Provider-reported token counters; absent values are unknown, never zero estimates.
use crate::agent::Provider;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct TokenUsage {
    pub input: Option<u64>,
    pub output: Option<u64>,
    pub total: Option<u64>,
    pub cached_input: Option<u64>,
    pub cache_creation_input: Option<u64>,
    pub reasoning: Option<u64>,
}
impl From<&rig::completion::Usage> for TokenUsage {
    fn from(usage: &rig::completion::Usage) -> Self {
        Self {
            input: usage.input_tokens,
            output: usage.output_tokens,
            total: usage
                .total_tokens
                .or_else(|| usage.input_tokens?.checked_add(usage.output_tokens?)),
            cached_input: usage.cached_input_tokens,
            cache_creation_input: usage.cache_creation_input_tokens,
            reasoning: usage.reasoning_tokens,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UsageRecord {
    pub id: String,
    pub turn_id: String,
    pub provider: Provider,
    pub model: String,
    pub usage: TokenUsage,
}
impl UsageRecord {
    pub fn new(turn_id: &str, provider: Provider, model: String, usage: TokenUsage) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            turn_id: turn_id.into(),
            provider,
            model,
            usage,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Counter {
    pub tokens: u64,
    pub reported: u64,
    overflow: bool,
}
impl Counter {
    fn add(&mut self, value: Option<u64>) {
        if let Some(value) = value {
            self.tokens = self.tokens.checked_add(value).unwrap_or_else(|| {
                self.overflow = true;
                u64::MAX
            });
            self.reported += 1;
        }
    }
    fn label(&self, responses: u64) -> String {
        if self.reported == 0 {
            return "未上报".into();
        }
        format!(
            "{}{}",
            if self.reported < responses || self.overflow {
                "≥"
            } else {
                ""
            },
            self.tokens
        )
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UsageTotals {
    pub responses: u64,
    pub input: Counter,
    pub output: Counter,
    pub total: Counter,
    pub cached_input: Counter,
    pub cache_creation_input: Counter,
    pub reasoning: Counter,
}
impl UsageTotals {
    pub fn add(&mut self, usage: &TokenUsage) {
        self.responses += 1;
        self.input.add(usage.input);
        self.output.add(usage.output);
        self.total.add(usage.total);
        self.cached_input.add(usage.cached_input);
        self.cache_creation_input.add(usage.cache_creation_input);
        self.reasoning.add(usage.reasoning);
    }
    pub fn label(&self) -> String {
        if self.responses == 0 {
            return "本会话 Token：暂无用量记录".into();
        }
        let mut label = format!(
            "本会话 Token：输入 {} · 输出 {} · 合计 {}",
            self.input.label(self.responses),
            self.output.label(self.responses),
            self.total.label(self.responses)
        );
        if self.cached_input.reported > 0 {
            label.push_str(&format!(
                " · 缓存命中 {}",
                self.cached_input.label(self.responses)
            ));
        }
        if self.cache_creation_input.reported > 0 {
            label.push_str(&format!(
                " · 缓存写入 {}",
                self.cache_creation_input.label(self.responses)
            ));
        }
        if self.reasoning.reported > 0 {
            label.push_str(&format!(" · 推理 {}", self.reasoning.label(self.responses)));
        }
        if self.input.reported < self.responses
            || self.output.reported < self.responses
            || self.total.reported < self.responses
        {
            label.push_str("（含未上报用量）");
        }
        label
    }
}
