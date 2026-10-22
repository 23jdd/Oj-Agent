use crate::{
    error::{AppError, Result},
    ir::AnimationIr,
    renderer::SkiaRenderer,
    style::StyleConfig,
};
use std::{fs::File, path::Path};
mod palette;

pub fn gif(animation: &AnimationIr, path: &Path, fps: u16) -> Result<()> {
    gif_cancellable(
        animation,
        path,
        fps,
        &std::sync::atomic::AtomicBool::new(false),
    )
}
pub fn gif_cancellable(
    animation: &AnimationIr,
    path: &Path,
    fps: u16,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<()> {
    gif_cancellable_styled(animation, path, fps, cancel, &StyleConfig::default())
}
pub fn gif_cancellable_styled(
    animation: &AnimationIr,
    path: &Path,
    fps: u16,
    cancel: &std::sync::atomic::AtomicBool,
    style: &StyleConfig,
) -> Result<()> {
    animation.validate()?;
    let scene = SkiaRenderer.prepare_styled(animation, style)?;
    if !(1..=30).contains(&fps) {
        return Err(AppError::validation(
            "Export",
            "GIF frame rate must be 1–30.",
        ));
    }
    let count = (animation.duration * fps as f64 - 1e-9).ceil().max(1.) as usize;
    if count > 3000 {
        return Err(AppError::new(
            "EXPORT_FRAME_LIMIT",
            "Export",
            "Export",
            "Export exceeds 3,000 frames; reduce frame rate or duration.",
        ));
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut output =
        tempfile::NamedTempFile::new_in(parent).map_err(|e| AppError::io("Export", e))?;
    {
        let palette = palette::theme_palette(&style.colors);
        let mut encoder = gif::Encoder::new(
            output.as_file_mut(),
            animation.scene.width as u16,
            animation.scene.height as u16,
            &palette.colors,
        )
        .map_err(|e| AppError::io("Encoding", e))?;
        encoder
            .set_repeat(gif::Repeat::Infinite)
            .map_err(|e| AppError::io("Encoding", e))?;
        for i in 0..count {
            crate::agent::check_cancelled(cancel)?;
            // GIF delays are centiseconds; distributing rounding preserves 30 fps timing.
            let total = (animation.duration * 100.).round().max(1.);
            let end = (((i + 1) as f64 * 100. / fps as f64).round()).min(total);
            let start = ((i as f64 * 100. / fps as f64).round()).min(total);
            let delay = (end - start) as u16;
            if delay == 0 {
                continue;
            }
            let frame = scene.frame(i as f64 / fps as f64)?;
            let gif_frame = gif::Frame {
                width: frame.width as u16,
                height: frame.height as u16,
                delay,
                buffer: std::borrow::Cow::Owned(palette.index(&frame.rgba)),
                ..Default::default()
            };
            encoder
                .write_frame(&gif_frame)
                .map_err(|e| AppError::io("Encoding", e))?;
        }
    }
    output
        .as_file_mut()
        .sync_all()
        .map_err(|e| AppError::io("Export", e))?;
    output
        .persist(path)
        .map_err(|e| AppError::io("Export", e))?;
    Ok(())
}
pub fn png(animation: &AnimationIr, path: &Path, time: f64) -> Result<()> {
    png_styled(animation, path, time, &StyleConfig::default())
}
pub fn png_styled(
    animation: &AnimationIr,
    path: &Path,
    time: f64,
    style: &StyleConfig,
) -> Result<()> {
    let frame = SkiaRenderer.prepare_styled(animation, style)?.frame(time)?;
    write_png(&frame, path)
}
fn write_png(frame: &crate::renderer::Frame, path: &Path) -> Result<()> {
    let info = skia_safe::ImageInfo::new(
        (frame.width as i32, frame.height as i32),
        skia_safe::ColorType::RGBA8888,
        skia_safe::AlphaType::Unpremul,
        None,
    );
    let data = skia_safe::Data::new_copy(&frame.rgba);
    let image = skia_safe::images::raster_from_data(&info, data, frame.width as usize * 4)
        .ok_or_else(|| {
            AppError::new(
                "EXPORT_IMAGE_FAILED",
                "Export",
                "Export",
                "Cannot construct PNG image.",
            )
        })?;
    let data = image
        .encode(None, skia_safe::EncodedImageFormat::PNG, None)
        .ok_or_else(|| {
            AppError::new(
                "EXPORT_ENCODER_FAILED",
                "Export",
                "Encoding",
                "Cannot encode PNG.",
            )
        })?;
    use std::io::Write;
    let mut file = File::create(path).map_err(|e| AppError::io("Export", e))?;
    file.write_all(data.as_bytes())
        .map_err(|e| AppError::io("Export", e))
}

pub fn ffmpeg_path() -> std::path::PathBuf {
    std::env::var_os("OJ_FFMPEG")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            let local = std::path::PathBuf::from("target/tools").join(if cfg!(windows) {
                "ffmpeg.exe"
            } else {
                "ffmpeg"
            });
            if local.is_file() {
                local
            } else {
                "ffmpeg".into()
            }
        })
}
pub fn video(
    animation: &AnimationIr,
    path: &Path,
    format: &str,
    fps: u16,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<()> {
    animation.validate()?;
    if !["mp4", "webm"].contains(&format) || !(1..=60).contains(&fps) {
        return Err(AppError::validation(
            "Export",
            "Select MP4/WebM and 1–60 fps.",
        ));
    }
    let count = (animation.duration * fps as f64).ceil() as usize;
    if count > 3600 {
        return Err(AppError::new(
            "EXPORT_FRAME_LIMIT",
            "Export",
            "Export",
            "Video export is limited to 3,600 frames.",
        ));
    }
    let frames = tempfile::tempdir().map_err(|e| AppError::io("Export", e))?;
    let scene = SkiaRenderer.prepare(animation)?;
    let start = std::time::Instant::now();
    for i in 0..count {
        crate::agent::check_cancelled(cancel)?;
        if start.elapsed() > std::time::Duration::from_secs(180) {
            return Err(AppError::new(
                "EXPORT_TIMEOUT",
                "Timeout",
                "Rendering",
                "Video frame rendering exceeded 180 seconds.",
            ));
        }
        write_png(
            &scene.frame(i as f64 / fps as f64)?,
            &frames.path().join(format!("{i:06}.png")),
        )?;
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let output = tempfile::NamedTempFile::new_in(parent).map_err(|e| AppError::io("Export", e))?;
    let mut command = std::process::Command::new(ffmpeg_path());
    command
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-framerate",
            &fps.to_string(),
            "-i",
        ])
        .arg(frames.path().join("%06d.png"));
    if format == "mp4" {
        command.args([
            "-c:v",
            "libx264",
            "-preset",
            "fast",
            "-pix_fmt",
            "yuv420p",
            "-vf",
            "pad=ceil(iw/2)*2:ceil(ih/2)*2",
            "-movflags",
            "+faststart",
        ]);
    } else {
        command.args([
            "-c:v",
            "libvpx-vp9",
            "-b:v",
            "0",
            "-crf",
            "32",
            "-pix_fmt",
            "yuv420p",
        ]);
    }
    command.args(["-f", format]).arg(output.path());
    let result = crate::process::run(
        &mut command,
        vec![],
        std::time::Duration::from_secs(120),
        1024 * 1024,
        cancel,
    )
    .map_err(|e| AppError::new("EXPORT_ENCODER_FAILED", "Export", "Encoding", e.to_string()))?;
    if !result.status.success() {
        return Err(AppError::new(
            "EXPORT_ENCODER_FAILED",
            "Export",
            "Encoding",
            String::from_utf8_lossy(&result.stderr),
        ));
    }
    output
        .persist(path)
        .map_err(|e| AppError::io("Export", e))?;
    Ok(())
}
pub fn output(
    animation: &AnimationIr,
    path: &Path,
    fps: u16,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<()> {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "gif" => gif_cancellable(animation, path, fps.min(30), cancel),
        "mp4" => video(animation, path, "mp4", fps, cancel),
        "webm" => video(animation, path, "webm", fps, cancel),
        "png" => png(animation, path, 0.),
        _ => Err(AppError::new(
            "EXPORT_FORMAT_INVALID",
            "Export",
            "Export",
            "Use a .gif, .png, .mp4 or .webm output path.",
        )),
    }
}
