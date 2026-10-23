//! Native-scale camera timings, including frame allocation; excludes Slint/GPU presentation.
use oj_agent::{
    animation,
    ir::*,
    renderer::{RasterSize, SkiaRenderer, Viewport},
    trace, visualization,
};
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let (a, t) = trace::simulate(&ProblemInput::default())?;
    let v = visualization::plan(&a, &t)?;
    let simple = animation::compile(&a, &t, &v)?;
    let mut dense = simple.clone();
    let object = dense
        .scene
        .objects
        .iter_mut()
        .find(|o| o.id == "values_view")
        .unwrap();
    object.kind = "graph".into();
    object.values = (0..128).collect();
    object.labels = (0..128).map(|i| format!("Readable node {i}")).collect();
    object.edges = (1..128).map(|i| [i - 1, i]).collect();
    dense.timeline = (0..20)
        .map(|i| AnimationAction {
            at: i as f64 * 0.8,
            duration: 0.32,
            action: Action::Highlight,
            target: "values_view".into(),
            text: String::new(),
            values: vec![],
            indices: vec![i % 128],
            structure: None,
        })
        .collect();
    dense.duration = 16.;
    dense.step_times = (0..20).map(|i| i as f64 * 0.8 + 0.32).collect();
    for (name, ir) in [("simple", &simple), ("dense graph", &dense)] {
        let scene = SkiaRenderer.prepare(ir)?;
        for (width, height) in [(960, 600), (2560, 1440), (3840, 2160)] {
            let viewport = if std::env::args().any(|arg| arg == "--uncropped") {
                Viewport::new(RasterSize::new(width, height)?, 0., 0., 1.)?
            } else {
                Viewport::for_content(
                    (width as f32, height as f32),
                    (scene.timeline.width as f32, scene.timeline.height as f32),
                    (0., 0.),
                    1.,
                )
                .unwrap()
            };
            scene.frame_viewport(0.08, viewport)?;
            let mut samples = Vec::new();
            for i in 0..90 {
                let time = (i % 20) as f64 * 0.8 + 0.08 + (i % 10) as f64 * 0.016;
                let started = Instant::now();
                std::hint::black_box(scene.frame_viewport(time.min(ir.duration), viewport)?);
                samples.push(started.elapsed().as_secs_f64() * 1000.);
            }
            samples.sort_by(f64::total_cmp);
            println!(
                "{name} window {width}x{height}, raster {}x{}: median {:.2} ms, p95 {:.2} ms",
                viewport.size.width, viewport.size.height, samples[45], samples[85]
            );
        }
    }
    Ok(())
}
