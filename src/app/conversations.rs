//! In-memory view state, keyed by conversation; messages remain in SQLite.
use crate::chat::Attachment;
use uuid::Uuid;

pub struct Conversation {
    pub draft: String,
    pub attachments: Vec<Attachment>,
    pub file_request: Option<Uuid>,
    pub status: String,
    pub error: String,
}
impl Default for Conversation {
    fn default() -> Self {
        Self {
            draft: String::new(),
            attachments: vec![],
            file_request: None,
            status: "就绪".into(),
            error: String::new(),
        }
    }
}
