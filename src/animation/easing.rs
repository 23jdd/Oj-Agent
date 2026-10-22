/// Monotone easing without overshoot, shared by every animated property.
pub fn smooth(progress: f32) -> f32 {
    let t = progress.clamp(0., 1.);
    t * t * (3. - 2. * t)
}

pub fn progress(time: f64, start: f64, duration: f64) -> f32 {
    if duration <= f64::EPSILON {
        1.
    } else {
        smooth(((time - start) / duration) as f32)
    }
}

pub fn lerp(from: f32, to: f32, progress: f32) -> f32 {
    from + (to - from) * progress
}
