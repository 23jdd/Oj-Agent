use super::Attachment;
use crate::error::{AppError, Result};
use sha2::{Digest, Sha256};
use std::{io::Read, path::Path};

pub fn attach(path: &Path) -> Result<Attachment> {
    if !path.is_file() {
        return Err(AppError::new(
            "ATTACHMENT_NOT_FILE",
            "Input",
            "Chat",
            "Choose a regular text or source file.",
        ));
    }
    let file = std::fs::File::open(path).map_err(|e| AppError::io("Attachment", e))?;
    let mut bytes = Vec::new();
    file.take(100_001)
        .read_to_end(&mut bytes)
        .map_err(|e| AppError::io("Attachment", e))?;
    if bytes.len() > 100_000 || bytes.contains(&0) {
        return Err(AppError::new(
            "ATTACHMENT_LIMIT",
            "Input",
            "Chat",
            "Attachments must be UTF-8 text under 100 KB.",
        ));
    }
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    let text = String::from_utf8(bytes).map_err(|_| {
        AppError::new(
            "ATTACHMENT_ENCODING",
            "Input",
            "Chat",
            "This file is not UTF-8 text.",
        )
    })?;
    Ok(Attachment {
        id: uuid::Uuid::new_v4().to_string(),
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        text,
        sha256,
    })
}
