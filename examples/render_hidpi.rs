//! Render the same vector scene at the physical resolution of a 150% display.
use oj_agent::{
    animation,
    ir::ProblemInput,
    renderer::{RasterSize, SkiaRenderer},
    trace, visualization,
};

fn main() -> anyhow::Result<()> {
    let input = ProblemInput::default();
    let (algorithm, trace) = trace::simulate(&input)?;
    let plan = visualization::plan(&algorithm, &trace)?;
    let animation = animation::compile(&algorithm, &trace, &plan)?;
    let scene = SkiaRenderer.prepare(&animation)?;
    std::fs::create_dir_all("target/font-review")?;
    for (name, viewport, dpi) in [("hidpi", (960., 720.), 1.5), ("compact", (600., 450.), 1.)] {
        let size = RasterSize::fit((960, 720), viewport, dpi).unwrap();
        let frame = scene.frame_at_size(0.4, size)?;
        let info = skia_safe::ImageInfo::new(
            (frame.width as i32, frame.height as i32),
            skia_safe::ColorType::RGBA8888,
            skia_safe::AlphaType::Premul,
            None,
        );
        let image = skia_safe::images::raster_from_data(
            &info,
            skia_safe::Data::new_copy(&frame.rgba),
            frame.width as usize * 4,
        )
        .ok_or_else(|| anyhow::anyhow!("Cannot create image"))?;
        let png = image
            .encode(None, skia_safe::EncodedImageFormat::PNG, None)
            .ok_or_else(|| anyhow::anyhow!("Cannot encode image"))?;
        let path = format!("target/font-review/{name}.png");
        std::fs::write(&path, png.as_bytes())?;
        println!("{path} ({}×{})", frame.width, frame.height);
    }
    Ok(())
}
