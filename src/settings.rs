//! Versioned local model settings, including the provider credentials.
//!
//! The file is user-private: Unix saves use mode 0600. `Debug` redacts the key
//! so configuration errors and logs never print credentials.
use crate::{
    agent::Provider,
    error::{AppError, Result},
};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    io::{Read, Write},
    path::Path,
};
pub const VERSION: u32 = 1;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ModelSettings {
    pub version: u32,
    pub provider: Provider,
    pub model: String,
    pub endpoint: String,
    pub api_key: String,
}
impl Default for ModelSettings {
    fn default() -> Self {
        Self {
            version: VERSION,
            provider: Provider::OpenAi,
            model: "gpt-4.1-mini".into(),
            endpoint: String::new(),
            api_key: String::new(),
        }
    }
}
impl fmt::Debug for ModelSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModelSettings")
            .field("version", &self.version)
            .field("provider", &self.provider)
            .field("model", &self.model)
            .field("endpoint", &self.endpoint)
            .field("api_key", &"<redacted>")
            .finish()
    }
}
fn invalid(message: impl Into<String>) -> AppError {
    AppError::new(
        "MODEL_CONFIG_INVALID",
        "Configuration",
        "ModelSettings",
        message,
    )
    .recoverable()
}
impl ModelSettings {
    pub fn validate(&self) -> Result<()> {
        if self.version != VERSION {
            return Err(invalid("Unsupported model settings version; expected 1."));
        }
        if self.model.trim().is_empty() {
            return Err(invalid("Model ID must not be empty."));
        }
        if self.model.len() > 128 || self.model.chars().any(char::is_control) {
            return Err(invalid("Model ID must be 1–128 printable characters."));
        }
        if self.endpoint.len() > 2048 || self.endpoint.chars().any(char::is_control) {
            return Err(invalid(
                "Endpoint must be at most 2048 printable characters.",
            ));
        }
        if self.provider == Provider::Compatible && self.endpoint.is_empty() {
            return Err(invalid(
                "A compatible provider requires an HTTPS or local HTTP endpoint.",
            ));
        }
        if !self.endpoint.is_empty()
            && !(self.endpoint.starts_with("https://")
                || self.endpoint.starts_with("http://localhost:")
                || self.endpoint.starts_with("http://127.0.0.1:"))
        {
            return Err(invalid("Use HTTPS, or a local HTTP endpoint."));
        }
        if self.api_key.len() > 16 * 1024 || self.api_key.chars().any(char::is_control) {
            return Err(invalid(
                "API key must be at most 16 KiB of printable characters.",
            ));
        }
        Ok(())
    }
    pub fn load(path: &Path) -> Result<Self> {
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(AppError::io("ModelSettings", e)),
        };
        let mut bytes = Vec::new();
        file.take(65537)
            .read_to_end(&mut bytes)
            .map_err(|e| AppError::io("ModelSettings", e))?;
        if bytes.len() > 64 * 1024 {
            return Err(invalid("Model settings exceed 64 KiB."));
        }
        let value: Self = serde_json::from_slice(&bytes).map_err(|e| invalid(e.to_string()))?;
        value.validate()?;
        Ok(value)
    }
    /// Replaces the file atomically and restricts it to the current user.
    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent).map_err(|e| AppError::io("ModelSettings", e))?;
        let mut file = tempfile::NamedTempFile::new_in(parent)
            .map_err(|e| AppError::io("ModelSettings", e))?;
        self.restrict(file.as_file())?;
        let bytes =
            serde_json::to_vec_pretty(self).map_err(|e| AppError::io("ModelSettings", e))?;
        file.write_all(&bytes)
            .and_then(|_| file.as_file_mut().sync_all())
            .map_err(|e| AppError::io("ModelSettings", e))?;
        file.persist(path)
            .map_err(|e| AppError::io("ModelSettings", e))?;
        self.restrict(&std::fs::File::open(path).map_err(|e| AppError::io("ModelSettings", e))?)
    }
    #[cfg(unix)]
    fn restrict(&self, file: &std::fs::File) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|e| AppError::io("ModelSettings", e))
    }
    #[cfg(not(unix))]
    fn restrict(&self, _file: &std::fs::File) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn round_trip_keeps_credentials_and_missing_file_uses_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("model.json");
        assert_eq!(
            ModelSettings::load(&path).unwrap(),
            ModelSettings::default()
        );
        let settings = ModelSettings {
            provider: Provider::DeepSeek,
            model: "deepseek-chat".into(),
            endpoint: "https://api.deepseek.com".into(),
            api_key: "test-only-key".into(),
            ..ModelSettings::default()
        };
        settings.save(&path).unwrap();
        assert_eq!(ModelSettings::load(&path).unwrap(), settings);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }
    #[test]
    fn invalid_fields_are_rejected_before_writing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("model.json");
        let invalid = ModelSettings {
            provider: Provider::Compatible,
            endpoint: String::new(),
            ..ModelSettings::default()
        };
        assert!(invalid.save(&path).is_err());
        assert!(!path.exists());
        let file = ModelSettings {
            model: "gpt-4.1-mini".into(),
            ..ModelSettings::default()
        };
        file.save(&path).unwrap();
        let mut handle = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&path)
            .unwrap();
        handle
            .write_all(b"{\"version\":1,\"model\":\"x\",\"api_key\":\"y\",\"extra\":1}")
            .unwrap();
        drop(handle);
        assert!(ModelSettings::load(&path).is_err());
    }
    #[test]
    fn provider_aliases_are_accepted() {
        let settings: ModelSettings = serde_json::from_value(serde_json::json!({
            "version": 1,
            "provider": "deepseek",
            "model": "deepseek-chat",
            "api_key": ""
        }))
        .unwrap();
        assert_eq!(settings.provider, Provider::DeepSeek);
    }
    #[test]
    fn debug_output_redacts_the_key() {
        let settings = ModelSettings {
            api_key: "secret-value".into(),
            ..ModelSettings::default()
        };
        let text = format!("{settings:?}");
        assert!(text.contains("<redacted>"));
        assert!(!text.contains("secret-value"));
    }
}
