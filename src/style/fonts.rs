//! Bounded local font loading and content-addressed copies owned by the profile.
use super::invalid;
use crate::error::{AppError, Result};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const MAX_FONT_BYTES: u64 = 32 * 1024 * 1024;

pub struct LocalFont {
    pub family: String,
    pub bytes: Vec<u8>,
    pub face: skia_safe::Typeface,
}

pub fn load(path: &Path) -> Result<LocalFont> {
    let file = std::fs::File::open(path)
        .map_err(|error| invalid(format!("无法读取字体 {}：{error}", path.display())))?;
    if !file
        .metadata()
        .map_err(|e| AppError::io("StyleFont", e))?
        .is_file()
    {
        return Err(invalid("请选择字体文件。"));
    }
    let mut bytes = Vec::new();
    file.take(MAX_FONT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| AppError::io("StyleFont", e))?;
    if bytes.len() as u64 > MAX_FONT_BYTES {
        return Err(invalid("字体文件不能超过 32 MiB。"));
    }
    let face = skia_safe::FontMgr::new()
        .new_from_data(skia_safe::Data::new_copy(&bytes), None)
        .ok_or_else(|| invalid("无法解析字体，请选择有效的 TTF、OTF 或 TTC 文件。"))?;
    Ok(LocalFont {
        family: face.family_name(),
        bytes,
        face,
    })
}

impl LocalFont {
    /// The original file can be moved after saving. Never overwrite an existing copy.
    pub fn store(&self, config: &Path) -> Result<PathBuf> {
        let dir = config.parent().unwrap_or(Path::new(".")).join("fonts");
        std::fs::create_dir_all(&dir).map_err(|e| AppError::io("StyleFont", e))?;
        let path = dir.join(format!("{:x}.font", Sha256::digest(&self.bytes)));
        if !path.exists() {
            let mut file =
                tempfile::NamedTempFile::new_in(&dir).map_err(|e| AppError::io("StyleFont", e))?;
            file.write_all(&self.bytes)
                .and_then(|_| file.as_file_mut().sync_all())
                .map_err(|e| AppError::io("StyleFont", e))?;
            match file.persist_noclobber(&path) {
                Ok(_) => {}
                Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(AppError::io("StyleFont", e)),
            }
        }
        // Do not adopt a corrupted or externally replaced copy just because its name matches.
        if load(&path)?.bytes != self.bytes {
            return Err(invalid("已保存的字体副本内容不匹配，请检查配置目录。"));
        }
        std::fs::canonicalize(path).map_err(|e| AppError::io("StyleFont", e))
    }
}
