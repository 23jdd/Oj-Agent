use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt};

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppError {
    pub code: String,
    pub kind: String,
    pub stage: String,
    pub message: String,
    pub recoverable: bool,
    pub context: Box<BTreeMap<String, String>>,
}

impl AppError {
    pub fn new(code: &str, kind: &str, stage: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            kind: kind.into(),
            stage: stage.into(),
            message: message.into(),
            recoverable: false,
            context: Box::default(),
        }
    }
    pub fn context(mut self, key: &str, value: impl ToString) -> Self {
        self.context.insert(key.into(), value.to_string());
        self
    }
    pub fn recoverable(mut self) -> Self {
        self.recoverable = true;
        self
    }
    pub fn validation(stage: &str, message: impl Into<String>) -> Self {
        Self::new("IR_INVALID", "Validation", stage, message).recoverable()
    }
    pub fn io(stage: &str, source: impl fmt::Display) -> Self {
        Self::new("IO_FAILED", "IO", stage, source.to_string())
    }
}
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [{}]: {}", self.stage, self.code, self.message)
    }
}
impl std::error::Error for AppError {}
