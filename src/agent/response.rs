//! Validate final answers without exposing provider reasoning or raw response bodies.
use crate::error::{AppError, Result};
use rig::{
    completion::{CompletionResponse, FinishReason},
    message::AssistantContent,
};

pub(super) fn final_text(response: &CompletionResponse, budget: u64) -> Result<String> {
    let text = response.text();
    let reason = response.finish_reason();
    let reasoning = response
        .choice
        .iter()
        .any(|part| matches!(part, AssistantContent::Reasoning(_)))
        || response
            .usage
            .reasoning_tokens
            .is_some_and(|tokens| tokens > 0);
    let failure = if matches!(reason, Some(FinishReason::ContentFilter)) {
        Some((
            "MODEL_CONTENT_FILTERED",
            "提供方过滤了本次回复，未能给出完整正文。请调整问题后重试。",
        ))
    } else if response.error.is_some() || response.aborted.is_some() {
        Some((
            "MODEL_RESPONSE_FAILED",
            "提供方未能完成本次回复。请重试或检查模型与接口是否兼容。",
        ))
    } else if matches!(reason, Some(FinishReason::Length)) {
        Some((
            "MODEL_OUTPUT_TRUNCATED",
            "模型已耗尽本次输出预算，未完成正文；思考内容也可能占用预算。请缩小问题或更换模型后重试。",
        ))
    } else if matches!(reason, Some(FinishReason::Other(_))) {
        Some((
            "MODEL_RESPONSE_INCOMPLETE",
            "提供方提前结束了本次回复。请重试或检查服务状态。",
        ))
    } else if response.tool_calls().next().is_some() {
        Some((
            "MODEL_TOOL_PROTOCOL",
            "接口返回了未请求的工具调用，没有可用正文。请检查模型与接口是否兼容。",
        ))
    } else if text.trim().is_empty() {
        Some(if reasoning {
            (
                "MODEL_REASONING_ONLY",
                "模型返回了思考内容，但没有最终正文。请重试或更换模型。",
            )
        } else {
            (
                "AGENT_RESPONSE_EMPTY",
                "接口返回成功，但回复正文为空。请重试，并检查所选模型与接口是否兼容。",
            )
        })
    } else {
        None
    };
    if let Some((code, message)) = failure {
        // Only normalized, non-content diagnostics may enter session error records.
        let mut error = AppError::new(code, "ModelProvider", "ModelResponse", message)
            .recoverable()
            .context("max_output_tokens", budget)
            .context("reasoning_present", reasoning)
            .context(
                "finish_reason",
                match reason {
                    Some(FinishReason::Stop) => "stop",
                    Some(FinishReason::Length) => "length",
                    Some(FinishReason::ContentFilter) => "content_filter",
                    Some(FinishReason::ToolCalls) => "tool_calls",
                    Some(_) => "other",
                    None => "unreported",
                },
            );
        if let Some(tokens) = response.usage.output_tokens {
            error = error.context("output_tokens", tokens);
        }
        return Err(error);
    }
    if text.len() > 1_000_000 {
        return Err(AppError::new(
            "AGENT_OUTPUT_LIMIT",
            "Agent",
            "ModelResponse",
            "Model response exceeds 1 MB.",
        ));
    }
    Ok(text)
}

pub(super) fn can_recover(error: &AppError) -> bool {
    matches!(
        error.code.as_str(),
        "AGENT_RESPONSE_EMPTY" | "MODEL_REASONING_ONLY" | "MODEL_OUTPUT_TRUNCATED"
    )
}
