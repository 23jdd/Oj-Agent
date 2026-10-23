use crate::error::{AppError, Result};

/// A camera into a logical canvas. Raster allocation depends on the window, never scene size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub size: RasterSize,
    pub x: f32,
    pub y: f32,
    pub scale: f32,
}
impl Viewport {
    pub fn new(size: RasterSize, x: f32, y: f32, scale: f32) -> Result<Self> {
        RasterSize::new(size.width, size.height)?;
        if ![x, y, scale].iter().all(|v| v.is_finite()) || x < 0. || y < 0. || scale <= 0. {
            return Err(AppError::validation("Rendering", "Invalid preview camera."));
        }
        Ok(Self {
            size,
            x: (x * scale).round() / scale,
            y: (y * scale).round() / scale,
            scale,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RasterSize {
    pub width: u32,
    pub height: u32,
}
impl RasterSize {
    pub fn viewport(width: f32, height: f32, dpi: f32) -> Option<Self> {
        if ![width, height, dpi]
            .iter()
            .all(|v| v.is_finite() && *v > 0.)
        {
            return None;
        }
        // Physical window dimensions are independent of the scrollable canvas dimensions.
        let w = f64::from(width);
        let h = f64::from(height);
        let scale = f64::from(dpi)
            .min(4096. / w)
            .min(4096. / h)
            .min((8_388_608. / (w * h)).sqrt());
        Self::new(
            (w * scale).floor().max(1.) as u32,
            (h * scale).floor().max(1.) as u32,
        )
        .ok()
    }
    pub fn new(width: u32, height: u32) -> Result<Self> {
        if width == 0
            || height == 0
            || width > 4096
            || height > 4096
            || u64::from(width) * u64::from(height) > 8_388_608
        {
            return Err(AppError::validation(
                "Rendering",
                "Preview size exceeds the 4096px / 32 MiB pixel budget.",
            ));
        }
        Ok(Self { width, height })
    }
    /// Slint supplies logical pixels; the frame must be rasterized at the monitor's physical size.
    pub fn fit(scene: (u32, u32), viewport: (f32, f32), dpi: f32) -> Option<Self> {
        let (w, h) = (scene.0 as f64, scene.1 as f64);
        if scene.0 == 0
            || scene.1 == 0
            || ![viewport.0, viewport.1, dpi]
                .iter()
                .all(|v| v.is_finite() && *v > 0.)
        {
            return None;
        }
        let scale = (f64::from(viewport.0) * f64::from(dpi) / w)
            .min(f64::from(viewport.1) * f64::from(dpi) / h)
            .min(4096. / w)
            .min(4096. / h)
            .min((8_388_608. / (w * h)).sqrt());
        Self::new(
            (w * scale).floor().max(1.) as u32,
            (h * scale).floor().max(1.) as u32,
        )
        .ok()
    }
}
