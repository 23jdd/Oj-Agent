use super::Event;
use crate::{
    animation::timeline::FrameKey,
    error::{AppError, Result},
    ir::AnimationIr,
    renderer::{PreparedScene, RasterSize, SkiaRenderer, Viewport},
    style::StyleConfig,
};
use slint::{Rgba8Pixel, SharedPixelBuffer};
use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex, mpsc},
};

pub type Pixels = SharedPixelBuffer<Rgba8Pixel>;
#[derive(Clone, Copy, PartialEq)]
pub enum PreviewView {
    Fit(RasterSize),
    Camera(Viewport),
}
impl From<RasterSize> for PreviewView {
    fn from(size: RasterSize) -> Self {
        Self::Fit(size)
    }
}
impl From<Viewport> for PreviewView {
    fn from(view: Viewport) -> Self {
        Self::Camera(view)
    }
}
impl PreviewView {
    fn size(self) -> RasterSize {
        match self {
            Self::Fit(size) => size,
            Self::Camera(view) => view.size,
        }
    }
}
struct Request {
    animation: Arc<AnimationIr>,
    time: f64,
    epoch: u64,
    view: PreviewView,
    style: Arc<StyleConfig>,
}
#[derive(Default)]
struct Pending {
    request: Option<Request>,
    closed: bool,
}

/// One pending request prevents a drag/seek burst from building an unbounded queue.
/// Prepared property tracks and a 32 MiB pixel cache live entirely on the render thread.
pub struct PreviewWorker(Arc<(Mutex<Pending>, Condvar)>);
impl PreviewWorker {
    pub fn new(events: mpsc::Sender<Event>) -> Self {
        let shared = Arc::new((Mutex::new(Pending::default()), Condvar::new()));
        let worker = shared.clone();
        std::thread::spawn(move || {
            let mut current: Option<Arc<AnimationIr>> = None;
            let mut prepared: Option<PreparedScene> = None;
            let mut cache = VecDeque::<(FrameKey, Pixels)>::new();
            let mut last_sent = None;
            let mut camera = None;
            let mut current_style: Option<Arc<StyleConfig>> = None;
            loop {
                let request = {
                    let (pending, wake) = &*worker;
                    let Ok(guard) = pending.lock() else {
                        break;
                    };
                    let Ok(mut guard) =
                        wake.wait_while(guard, |state| state.request.is_none() && !state.closed)
                    else {
                        break;
                    };
                    if guard.closed {
                        break;
                    }
                    guard.request.take().expect("render request is ready")
                };
                let mut canvas = (
                    request.animation.scene.width,
                    request.animation.scene.height,
                );
                let result = (|| {
                    if !current
                        .as_ref()
                        .is_some_and(|a| Arc::ptr_eq(a, &request.animation))
                        || current_style
                            .as_ref()
                            .is_none_or(|s| !Arc::ptr_eq(s, &request.style))
                    {
                        prepared =
                            Some(SkiaRenderer.prepare_styled(&request.animation, &request.style)?);
                        current_style = Some(request.style.clone());
                        current = Some(request.animation.clone());
                        cache.clear();
                        last_sent = None;
                    }
                    let scene = prepared.as_ref().expect("render scene prepared");
                    canvas = (scene.timeline.width, scene.timeline.height);
                    if camera != Some(request.view) {
                        cache.clear();
                        last_sent = None;
                        camera = Some(request.view);
                    }
                    let time = scene.timeline.clamp_time(request.time)?;
                    let key = scene.frame_key(time);
                    if last_sent == Some((key, request.epoch)) {
                        return Ok(None);
                    }
                    let pixels = if let Some(index) = cache.iter().position(|(k, _)| *k == key) {
                        let hit = cache.remove(index).expect("cache position exists");
                        let pixels = hit.1.clone();
                        cache.push_back(hit);
                        pixels
                    } else {
                        let size = request.view.size();
                        let (width, height) = (size.width, size.height);
                        let frame_bytes = width as usize * height as usize * 4;
                        let limit = (32 * 1024 * 1024 / frame_bytes).clamp(1, 16);
                        let mut pixels = if cache.len() >= limit {
                            cache
                                .pop_front()
                                .expect("cache contains a reusable buffer")
                                .1
                        } else {
                            Pixels::new(width, height)
                        };
                        match request.view {
                            PreviewView::Fit(size) => {
                                scene.frame_into(time, size, pixels.make_mut_bytes())?
                            }
                            PreviewView::Camera(view) => {
                                scene.frame_viewport_into(time, view, pixels.make_mut_bytes())?
                            }
                        }
                        cache.push_back((key, pixels.clone()));
                        pixels
                    };
                    last_sent = Some((key, request.epoch));
                    Ok(Some(pixels))
                })();
                let frame = match result {
                    Ok(None) => continue,
                    Ok(Some(pixels)) => Ok(pixels),
                    Err(e) => Err(e),
                };
                if events
                    .send(Event::Preview {
                        animation: request.animation,
                        epoch: request.epoch,
                        canvas,
                        frame,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self(shared)
    }
    pub fn request(
        &self,
        animation: Arc<AnimationIr>,
        time: f64,
        epoch: u64,
        view: impl Into<PreviewView>,
        style: Arc<StyleConfig>,
    ) -> Result<()> {
        let view = view.into();
        let size = view.size();
        RasterSize::new(size.width, size.height)?;
        if let PreviewView::Camera(view) = view {
            Viewport::new(view.size, view.x, view.y, view.scale)?;
        }
        let (pending, wake) = &*self.0;
        let mut state = pending.lock().map_err(|_| {
            AppError::new(
                "RENDER_WORKER_STOPPED",
                "Render",
                "Rendering",
                "The preview renderer has stopped.",
            )
        })?;
        state.request = Some(Request {
            animation,
            time,
            epoch,
            view,
            style,
        });
        wake.notify_one();
        Ok(())
    }
}
impl Drop for PreviewWorker {
    fn drop(&mut self) {
        let (pending, wake) = &*self.0;
        if let Ok(mut state) = pending.lock() {
            state.closed = true;
            wake.notify_one();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{animation, ir::ProblemInput, trace, visualization};
    use std::time::Duration;
    fn fixture() -> Arc<AnimationIr> {
        let (a, t) = trace::simulate(&ProblemInput::default()).unwrap();
        let v = visualization::plan(&a, &t).unwrap();
        Arc::new(animation::compile(&a, &t, &v).unwrap())
    }
    #[test]
    fn tween_frames_change_and_seek_reuses_settled_pixels() {
        let animation = fixture();
        let (tx, rx) = mpsc::channel();
        let worker = PreviewWorker::new(tx);
        let receive = || match rx.recv_timeout(Duration::from_secs(5)).unwrap() {
            Event::Preview {
                epoch,
                frame: Ok(frame),
                ..
            } => (epoch, frame),
            _ => panic!("missing preview"),
        };
        let style = Arc::new(StyleConfig::default());
        let size = RasterSize::new(960, 720).unwrap();
        worker
            .request(animation.clone(), 0.08, 0, size, style.clone())
            .unwrap();
        let (_, first) = receive();
        worker
            .request(animation.clone(), 0.24, 0, size, style.clone())
            .unwrap();
        let (_, middle) = receive();
        assert_ne!(first.as_bytes(), middle.as_bytes());
        worker
            .request(animation.clone(), 0.4, 1, size, style.clone())
            .unwrap();
        let (_, settled) = receive();
        let exported = SkiaRenderer
            .prepare(&animation)
            .unwrap()
            .frame(0.4)
            .unwrap();
        assert_eq!(settled.as_bytes(), exported.rgba);
        worker
            .request(animation.clone(), 0.5, 2, size, style.clone())
            .unwrap();
        let (epoch, cached) = receive();
        assert_eq!(epoch, 2);
        assert_eq!(settled.as_bytes().as_ptr(), cached.as_bytes().as_ptr());
        let larger = RasterSize::new(1440, 1080).unwrap();
        worker
            .request(animation.clone(), 0.5, 3, larger, style.clone())
            .unwrap();
        let (_, resized) = receive();
        assert_eq!((resized.width(), resized.height()), (1440, 1080));
        let expected = SkiaRenderer
            .prepare(&animation)
            .unwrap()
            .frame_at_size(0.5, larger)
            .unwrap();
        assert_eq!(resized.as_bytes(), expected.rgba);
        let mut changed = (*style).clone();
        changed.colors.surface = crate::style::Rgb([20, 40, 60]);
        let changed = Arc::new(changed);
        worker
            .request(animation.clone(), 0.5, 4, larger, changed.clone())
            .unwrap();
        let (epoch, styled) = receive();
        assert_eq!(epoch, 4);
        assert_ne!(styled.as_bytes(), resized.as_bytes());
        assert_eq!(
            styled.as_bytes(),
            SkiaRenderer
                .prepare_styled(&animation, &changed)
                .unwrap()
                .frame_at_size(0.5, larger)
                .unwrap()
                .rgba
        );
    }
    #[test]
    fn scrolling_a_paused_scene_invalidates_pixels_and_returning_restores_them() {
        let animation = fixture();
        let (tx, rx) = mpsc::channel();
        let worker = PreviewWorker::new(tx);
        let style = Arc::new(StyleConfig::default());
        let size = RasterSize::new(400, 300).unwrap();
        let mut frames = Vec::new();
        for (epoch, x, y) in [(1, 0., 0.), (2, 200., 400.), (3, 0., 0.)] {
            let view = Viewport::new(size, x, y, 1.).unwrap();
            worker
                .request(animation.clone(), 0.4, epoch, view, style.clone())
                .unwrap();
            match rx.recv_timeout(Duration::from_secs(5)).unwrap() {
                Event::Preview {
                    epoch: received,
                    canvas,
                    frame: Ok(pixels),
                    ..
                } => {
                    assert_eq!(received, epoch);
                    assert!(
                        canvas.0 >= animation.scene.width && canvas.1 >= animation.scene.height
                    );
                    assert_eq!(
                        pixels.as_bytes(),
                        SkiaRenderer
                            .prepare(&animation)
                            .unwrap()
                            .frame_viewport(0.4, view)
                            .unwrap()
                            .rgba
                    );
                    frames.push(pixels);
                }
                _ => panic!("missing camera frame"),
            }
        }
        assert_ne!(frames[0].as_bytes(), frames[1].as_bytes());
        assert_eq!(frames[0].as_bytes(), frames[2].as_bytes());
    }
}
