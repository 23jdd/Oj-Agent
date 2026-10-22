//! CPU draw timings only; window presentation and GPU upload are not measured.
use oj_agent::{animation, ir::*, renderer::SkiaRenderer, trace, visualization};
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    for (algorithm, values, target) in [
        (Algorithm::TwoSum, vec![2, 7, 11, 15], 9),
        (Algorithm::BubbleSort, (1..=16).rev().collect(), 0),
        (Algorithm::Bfs, (1..=31).collect(), 0),
        (Algorithm::Fibonacci, vec![0], 40),
    ] {
        let input = ProblemInput {
            algorithm,
            values,
            target,
            ..Default::default()
        };
        let (a, t) = trace::simulate(&input)?;
        let v = visualization::plan(&a, &t)?;
        let animation = animation::compile(&a, &t, &v)?;
        let scene = SkiaRenderer.prepare(&animation)?;
        scene.frame(0.)?;
        let mut samples = Vec::new();
        for i in 0..180 {
            let time = animation.duration * i as f64 / 179.;
            let start = Instant::now();
            std::hint::black_box(scene.frame(time)?);
            samples.push(start.elapsed().as_secs_f64() * 1000.);
        }
        samples.sort_by(f64::total_cmp);
        println!(
            "{}: median {:.2} ms, p95 {:.2} ms (960x720, 180 samples)",
            a.name, samples[90], samples[171]
        );
    }
    Ok(())
}
