//! Validated, versioned appearance preferences shared by Slint and Skia.
use crate::error::{AppError, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    io::{Read, Write},
    path::Path,
};

pub(crate) fn invalid(message: impl Into<String>) -> AppError {
    AppError::new("STYLE_INVALID", "Configuration", "StyleConfig", message).recoverable()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub [u8; 3]);
impl Rgb {
    pub fn parse(value: &str) -> Result<Self> {
        let value = value.trim();
        if value.len() != 7
            || !value.starts_with('#')
            || !value[1..].bytes().all(|c| c.is_ascii_hexdigit())
        {
            return Err(invalid("Color must use #RRGGBB, for example #ffdb33."));
        }
        Ok(Self(std::array::from_fn(|i| {
            u8::from_str_radix(&value[1 + i * 2..3 + i * 2], 16).expect("validated hex")
        })))
    }
    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0[0], self.0[1], self.0[2])
    }
    pub fn skia(self) -> skia_safe::Color {
        skia_safe::Color::from_rgb(self.0[0], self.0[1], self.0[2])
    }
    pub fn slint(self) -> slint::Color {
        slint::Color::from_rgb_u8(self.0[0], self.0[1], self.0[2])
    }
}
impl Serialize for Rgb {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.hex())
    }
}
impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        Self::parse(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Fonts {
    pub ui_family: String,
    pub code_family: String,
    pub animation_family: String,
    pub text_size: f32,
    pub label_size: f32,
    pub heading_size: f32,
    pub animation_scale: f32,
    pub minimum_text_pixels: f32,
    pub line_height: f32,
}
impl Default for Fonts {
    fn default() -> Self {
        let family = if cfg!(target_os = "windows") {
            "Microsoft YaHei UI"
        } else {
            "Noto Sans CJK SC"
        };
        Self {
            ui_family: family.into(),
            code_family: if cfg!(target_os = "windows") {
                "Consolas"
            } else {
                "monospace"
            }
            .into(),
            animation_family: family.into(),
            text_size: 14.,
            label_size: 13.,
            heading_size: 24.,
            animation_scale: 1.,
            minimum_text_pixels: 13.,
            line_height: 1.45,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Colors {
    pub background: Rgb,
    pub surface: Rgb,
    pub surface_hover: Rgb,
    pub surface_active: Rgb,
    pub border: Rgb,
    pub text_primary: Rgb,
    pub text_secondary: Rgb,
    pub text_disabled: Rgb,
    pub disabled: Rgb,
    pub accent: Rgb,
    pub focus: Rgb,
    pub success: Rgb,
    pub danger: Rgb,
}
impl Default for Colors {
    fn default() -> Self {
        Self {
            background: Rgb([245, 245, 245]),
            surface: Rgb([255, 255, 255]),
            surface_hover: Rgb([250, 229, 131]),
            surface_active: Rgb([255, 204, 0]),
            border: Rgb([0, 0, 0]),
            text_primary: Rgb([0, 0, 0]),
            text_secondary: Rgb([90, 90, 90]),
            text_disabled: Rgb([115, 115, 115]),
            disabled: Rgb([229, 229, 229]),
            accent: Rgb([255, 219, 51]),
            focus: Rgb([95, 79, 230]),
            success: Rgb([40, 114, 61]),
            danger: Rgb([169, 31, 45]),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Geometry {
    pub border_width: f32,
    pub shadow_offset: f32,
    pub radius: f32,
    pub control_height: f32,
}
impl Default for Geometry {
    fn default() -> Self {
        Self {
            border_width: 2.,
            shadow_offset: 4.,
            radius: 0.,
            control_height: 36.,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct StyleConfig {
    pub version: u32,
    pub fonts: Fonts,
    pub colors: Colors,
    pub geometry: Geometry,
}
impl Default for StyleConfig {
    fn default() -> Self {
        Self {
            version: 1,
            fonts: Fonts::default(),
            colors: Colors::default(),
            geometry: Geometry::default(),
        }
    }
}
impl StyleConfig {
    pub fn validate(&self) -> Result<()> {
        if self.version != 1 {
            return Err(invalid("Unsupported style version; expected 1."));
        }
        for (name, value) in [
            ("UI font", &self.fonts.ui_family),
            ("Code font", &self.fonts.code_family),
            ("Animation font", &self.fonts.animation_family),
        ] {
            if value.trim().is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
                return Err(invalid(format!("{name} must contain 1–128 characters.")));
            }
        }
        for (name, value, min, max) in [
            ("Text size", self.fonts.text_size, 12., 18.),
            ("Label size", self.fonts.label_size, 12., 16.),
            ("Heading size", self.fonts.heading_size, 18., 28.),
            ("Animation scale", self.fonts.animation_scale, 0.8, 1.6),
            (
                "Minimum text pixels",
                self.fonts.minimum_text_pixels,
                12.,
                18.,
            ),
            ("Line height", self.fonts.line_height, 1.2, 1.8),
            ("Border width", self.geometry.border_width, 1., 3.),
            ("Shadow offset", self.geometry.shadow_offset, 0., 6.),
            ("Corner radius", self.geometry.radius, 0., 8.),
            ("Control height", self.geometry.control_height, 36., 48.),
        ] {
            if !value.is_finite() || !(min..=max).contains(&value) {
                return Err(invalid(format!("{name} must be between {min} and {max}.")));
            }
        }
        Ok(())
    }
    pub fn load(path: &Path) -> Result<Self> {
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(AppError::io("StyleConfig", e)),
        };
        let mut bytes = Vec::new();
        file.take(65537)
            .read_to_end(&mut bytes)
            .map_err(|e| AppError::io("StyleConfig", e))?;
        if bytes.len() > 64 * 1024 {
            return Err(invalid("Style config exceeds 64 KiB."));
        }
        let value: Self = serde_json::from_slice(&bytes).map_err(|e| invalid(e.to_string()))?;
        value.validate()?;
        Ok(value)
    }
    /// Replace only after validation and a successful write, keeping the previous file on failure.
    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent).map_err(|e| AppError::io("StyleConfig", e))?;
        let mut file =
            tempfile::NamedTempFile::new_in(parent).map_err(|e| AppError::io("StyleConfig", e))?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| AppError::io("StyleConfig", e))?;
        file.write_all(&bytes)
            .and_then(|_| file.as_file_mut().sync_all())
            .map_err(|e| AppError::io("StyleConfig", e))?;
        file.persist(path)
            .map_err(|e| AppError::io("StyleConfig", e))?;
        Ok(())
    }
}
