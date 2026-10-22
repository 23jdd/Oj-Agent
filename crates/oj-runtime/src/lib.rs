pub mod instrument;
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(not(windows))]
use serde_json::json;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Failure {
    pub code: String,
    pub kind: String,
    pub stage: String,
    pub message: String,
    pub recoverable: bool,
    pub context: Box<BTreeMap<String, String>>,
}
impl Failure {
    pub fn new(code: &str, stage: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            kind: "Execution".into(),
            stage: stage.into(),
            message: message.into(),
            recoverable: false,
            context: Box::default(),
        }
    }
}
pub type Result<T> = std::result::Result<T, Failure>;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub operation: String,
    #[serde(default = "python")]
    pub language: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub stdin: String,
    #[serde(default)]
    pub artifact_id: String,
    #[serde(default)]
    pub values: Vec<i64>,
    #[serde(default = "max_events")]
    pub max_events: usize,
    #[serde(default)]
    pub instrument: bool,
    #[serde(default)]
    pub tools: BTreeMap<String, String>,
    #[serde(default)]
    pub work_dir: String,
}
fn python() -> String {
    "python".into()
}
fn max_events() -> usize {
    1000
}
pub fn handle(request: Request) -> Result<Value> {
    if request.source.len() > 100_000
        || request.stdin.len() > 100_000
        || request.values.len() > 64
        || request.max_events == 0
        || request.max_events > 10_000
    {
        return Err(Failure::new(
            "EXECUTION_INPUT_LIMIT",
            "Execution",
            "Source/stdin limit is 100 KB; arrays 64 values; traces 1–10,000 events.",
        ));
    }
    #[cfg(not(windows))]
    if request.operation == "probe" {
        return Ok(
            json!({"platform":std::env::consts::OS,"isolation":cfg!(target_os="linux"),"languages":["python","rust","cpp"]}),
        );
    }
    #[cfg(target_os = "linux")]
    {
        linux::handle(request)
    }
    #[cfg(all(windows, feature = "windows-native"))]
    {
        windows::handle(request)
    }
    #[cfg(all(windows, not(feature = "windows-native")))]
    {
        let _ = request;
        Err(Failure::new(
            "SANDBOX_UNAVAILABLE",
            "Execution",
            "Native Windows sandbox development is paused and this experimental backend is disabled.",
        ))
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = request;
        Err(Failure::new(
            "SANDBOX_UNAVAILABLE",
            "Execution",
            "This worker supports Windows AppContainer and native Linux.",
        ))
    }
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(all(windows, feature = "windows-native"))]
mod windows;
