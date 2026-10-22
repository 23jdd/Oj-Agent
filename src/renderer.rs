use crate::{
    animation::{
        easing::lerp,
        timeline::{FrameKey, ObjectSample, Timeline},
    },
    error::{AppError, Result},
    ir::*,
    style::StyleConfig,
};
use skia_safe::{
    AlphaType, Canvas, Color, ColorType, Font, FontMgr, ImageInfo, Paint, Rect, Typeface, surfaces,
};
use std::cell::RefCell;
pub mod layout;
pub mod viewport;
use layout::Box2;
pub use viewport::RasterSize;

pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
pub trait Renderer {
    fn frame(&self, animation: &AnimationIr, time: f64) -> Result<Frame>;
}
pub struct SkiaRenderer;
pub struct PreparedScene {
    pub timeline: Timeline,
    style: StyleConfig,
}

pub mod palette {
    use skia_safe::Color;
    pub const PAPER: Color = Color::from_rgb(255, 255, 255);
    pub const SURFACE: Color = Color::from_rgb(245, 245, 245);
    pub const INK: Color = Color::from_rgb(0, 0, 0);
    pub const SECONDARY: Color = Color::from_rgb(90, 90, 90);
    pub const BORDER: Color = Color::from_rgb(0, 0, 0);
    pub const ACTIVE: Color = Color::from_rgb(255, 219, 51);
    pub const SEAL: Color = INK;
}

thread_local! { static FACES: RefCell<Option<(String, Typeface, Typeface)>> = const { RefCell::new(None) }; }
fn font(family: &str, size: f32, bold: bool) -> Result<Font> {
    FACES.with(|cached| {
        if cached
            .borrow()
            .as_ref()
            .is_none_or(|faces| faces.0 != family)
        {
            let mgr = FontMgr::new();
            let regular = [
                family,
                "Microsoft YaHei UI",
                "Microsoft YaHei",
                "Noto Sans CJK SC",
                "Arial",
            ]
            .into_iter()
            .find_map(|name| mgr.match_family_style(name, Default::default()))
            .or_else(|| mgr.legacy_make_typeface(None, Default::default()))
            .ok_or_else(|| {
                AppError::new(
                    "RENDER_FONT_MISSING",
                    "Render",
                    "Rendering",
                    "No usable system font is installed.",
                )
            })?;
            let heavy = mgr
                .match_family_style(regular.family_name(), skia_safe::FontStyle::bold())
                .unwrap_or_else(|| regular.clone());
            *cached.borrow_mut() = Some((family.to_owned(), regular, heavy));
        }
        let faces = cached.borrow();
        let faces = faces.as_ref().expect("font faces initialized");
        let mut font = Font::from_typeface(
            if bold {
                faces.2.clone()
            } else {
                faces.1.clone()
            },
            size,
        );
        font.set_edging(skia_safe::font::Edging::AntiAlias);
        font.set_hinting(skia_safe::FontHinting::Normal);
        font.set_subpixel(false);
        font.set_baseline_snap(true);
        Ok(font)
    })
}
fn paint(color: Color, opacity: f32) -> Paint {
    let mut p = Paint::default();
    p.set_anti_alias(true);
    p.set_color(color);
    p.set_alpha_f(opacity.clamp(0., 1.));
    p
}
fn rect(b: Box2) -> Rect {
    Rect::from_xywh(b.x, b.y, b.width, b.height)
}
fn blend(from: Color, to: Color, t: f32) -> Color {
    Color::from_rgb(
        lerp(from.r() as f32, to.r() as f32, t).round() as u8,
        lerp(from.g() as f32, to.g() as f32, t).round() as u8,
        lerp(from.b() as f32, to.b() as f32, t).round() as u8,
    )
}
struct Painter<'a> {
    canvas: &'a Canvas,
    style: &'a StyleConfig,
}
impl Painter<'_> {
    fn centered(&self, value: &str, b: Box2, size: f32, bold: bool, opacity: f32) -> Result<()> {
        let canvas = self.canvas;
        let scale = canvas.local_to_device_as_3x3().scale_x().abs().max(0.01);
        let size = (size * self.style.fonts.animation_scale)
            .max((self.style.fonts.minimum_text_pixels - 1.) / scale);
        let size = if bold {
            size.min(b.height * 0.85)
        } else {
            size
        };
        let mut f = font(&self.style.fonts.animation_family, size, bold)?;
        let measured = f.measure_str(value, None).0;
        if measured > (b.width - 12.).max(1.) {
            f.set_size(size * (b.width - 12.).max(1.) / measured);
        }
        let width = f.measure_str(value, None).0;
        let (_, metrics) = f.metrics();
        let center = b.center();
        canvas.draw_str(
            value,
            (
                center.0 - width / 2.,
                center.1 - (metrics.ascent + metrics.descent) / 2.,
            ),
            &f,
            &paint(self.style.colors.text_primary.skia(), opacity),
        );
        Ok(())
    }
    fn text_block(
        &self,
        value: &str,
        b: Box2,
        size: f32,
        color: Color,
        opacity: f32,
        bold: bool,
    ) -> Result<()> {
        let canvas = self.canvas;
        if opacity <= 0.001 || b.width <= 0. || b.height <= 0. {
            return Ok(());
        }
        let scale = canvas.local_to_device_as_3x3().scale_x().abs().max(0.01);
        let size = (size * self.style.fonts.animation_scale)
            .max(self.style.fonts.minimum_text_pixels / scale);
        let f = font(&self.style.fonts.animation_family, size, bold)?;
        let (_, metrics) = f.metrics();
        let stride = size * self.style.fonts.line_height;
        let glyph_height = metrics.descent - metrics.ascent;
        let max_lines = (1. + ((b.height - glyph_height).max(0.) / stride).floor()) as usize;
        let mut lines = Vec::new();
        let mut line = String::new();
        let mut truncated = false;
        for ch in value.chars() {
            if ch == '\n' {
                lines.push(std::mem::take(&mut line));
            } else {
                line.push(ch);
                if f.measure_str(&line, None).0 > b.width {
                    line.pop();
                    lines.push(std::mem::take(&mut line));
                    if !ch.is_whitespace() {
                        line.push(ch);
                    }
                }
            }
            if lines.len() >= max_lines {
                truncated = true;
                break;
            }
        }
        if !line.is_empty() && lines.len() < max_lines {
            lines.push(line);
        }
        if truncated && let Some(last) = lines.last_mut() {
            while !last.is_empty() && f.measure_str(format!("{last}…"), None).0 > b.width {
                last.pop();
            }
            last.push('…');
        }
        for (i, line) in lines.iter().enumerate() {
            canvas.draw_str(
                line,
                (b.x, b.y - metrics.ascent + i as f32 * stride),
                &f,
                &paint(color, opacity),
            );
        }
        Ok(())
    }
}

impl Renderer for SkiaRenderer {
    fn frame(&self, animation: &AnimationIr, time: f64) -> Result<Frame> {
        self.prepare(animation)?.frame(time)
    }
}
impl SkiaRenderer {
    pub fn prepare(&self, animation: &AnimationIr) -> Result<PreparedScene> {
        self.prepare_styled(animation, &StyleConfig::default())
    }
    pub fn prepare_styled(
        &self,
        animation: &AnimationIr,
        style: &StyleConfig,
    ) -> Result<PreparedScene> {
        style.validate()?;
        Ok(PreparedScene {
            timeline: Timeline::new(animation)?,
            style: style.clone(),
        })
    }
}
impl PreparedScene {
    pub fn frame_key(&self, time: f64) -> FrameKey {
        self.timeline.frame_key(time)
    }
    pub fn frame(&self, time: f64) -> Result<Frame> {
        self.frame_at_size(
            time,
            RasterSize::new(self.timeline.width, self.timeline.height)?,
        )
    }
    pub fn frame_at_size(&self, time: f64, size: RasterSize) -> Result<Frame> {
        RasterSize::new(size.width, size.height)?;
        let (width, height) = (size.width, size.height);
        let mut rgba = vec![0; width as usize * height as usize * 4];
        self.frame_into(time, size, &mut rgba)?;
        Ok(Frame {
            width,
            height,
            rgba,
        })
    }
    pub(crate) fn frame_into(&self, time: f64, size: RasterSize, rgba: &mut [u8]) -> Result<()> {
        let time = self.timeline.clamp_time(time)?;
        RasterSize::new(size.width, size.height)?;
        let (width, height) = (size.width, size.height);
        let info = ImageInfo::new(
            (width as i32, height as i32),
            ColorType::RGBA8888,
            AlphaType::Premul,
            None,
        );
        let mut surface =
            surfaces::wrap_pixels(&info, rgba, width as usize * 4, None).ok_or_else(|| {
                AppError::new(
                    "RENDER_RESOURCE_FAILED",
                    "Render",
                    "Rendering",
                    "Cannot wrap frame pixels.",
                )
            })?;
        let canvas = surface.canvas();
        canvas.clear(self.style.colors.surface.skia());
        let scale = (width as f32 / self.timeline.width as f32)
            .min(height as f32 / self.timeline.height as f32);
        canvas.translate((
            (width as f32 - self.timeline.width as f32 * scale) / 2.,
            (height as f32 - self.timeline.height as f32 * scale) / 2.,
        ));
        canvas.scale((scale, scale));
        let painter = Painter {
            canvas,
            style: &self.style,
        };
        for track in &self.timeline.objects {
            let sample = track.sample(time);
            if sample.opacity <= 0.001 {
                continue;
            }
            let bounds = Box2::from(sample.object);
            if sample.opacity < 0.999 {
                canvas.save_layer_alpha_f(rect(bounds), sample.opacity);
            } else {
                canvas.save();
            }
            canvas.clip_rect(rect(bounds), None, true);
            match sample.object.kind.as_str() {
                "text" => painter.draw_text_object(&sample)?,
                "graph" | "tree" => painter.draw_graph(&sample)?,
                _ => painter.draw_cells(&sample)?,
            }
            canvas.restore();
        }
        Ok(())
    }
}

impl Painter<'_> {
    fn draw_text_object(&self, s: &ObjectSample<'_>) -> Result<()> {
        let canvas = self.canvas;
        let mut b = Box2::from(s.object);
        let title = s.object.id == "title";
        let state = s.object.id == "state_view";
        if title {
            return self.text_block(
                s.text,
                b,
                28.,
                self.style.colors.text_primary.skia(),
                1.,
                true,
            );
        }
        if s.object.id == "subtitle" {
            return self.text_block(
                s.text,
                b,
                14.,
                self.style.colors.text_secondary.skia(),
                1.,
                false,
            );
        }
        canvas.draw_rect(rect(b), &paint(self.style.colors.background.skia(), 1.));
        canvas.draw_rect(
            Rect::from_xywh(b.x, b.y, 4., b.height),
            &paint(
                if state {
                    self.style.colors.border.skia()
                } else {
                    self.style.colors.accent.skia()
                },
                1.,
            ),
        );
        b.x += 20.;
        b.y += 12.;
        b.width = (b.width - 36.).max(1.);
        b.height = (b.height - 24.).max(1.);
        if state {
            self.text_block(
                "STATE",
                Box2 { height: 16., ..b },
                13.,
                self.style.colors.text_secondary.skia(),
                1.,
                true,
            )?;
            b.y += 24.;
            b.height = (b.height - 24.).max(1.);
            if s.text.is_empty() && s.previous_text.is_empty() {
                return self.text_block(
                    "No auxiliary state in this step.",
                    b,
                    17.,
                    self.style.colors.text_secondary.skia(),
                    0.8,
                    false,
                );
            }
        }
        let size = if state { 17. } else { 18. };
        let color = if state {
            self.style.colors.text_secondary.skia()
        } else {
            self.style.colors.text_primary.skia()
        };
        if s.previous_text != s.text && s.text_progress < 1. {
            self.text_block(
                s.previous_text,
                b.offset(0., -4. * s.text_progress),
                size,
                color,
                (1. - 2. * s.text_progress).max(0.),
                false,
            )?;
            self.text_block(
                s.text,
                b.offset(0., 4. * (1. - s.text_progress)),
                size,
                color,
                if s.previous_text.is_empty() {
                    s.text_progress
                } else {
                    (2. * s.text_progress - 1.).max(0.)
                },
                false,
            )
        } else {
            self.text_block(s.text, b, size, color, 1., false)
        }
    }
    fn draw_cell(&self, value: i64, b: Box2, highlight: f32, opacity: f32) -> Result<()> {
        let canvas = self.canvas;
        if opacity <= 0.001 {
            return Ok(());
        }
        canvas.draw_round_rect(
            rect(b.offset(
                self.style.geometry.shadow_offset,
                self.style.geometry.shadow_offset,
            )),
            self.style.geometry.radius,
            self.style.geometry.radius,
            &paint(self.style.colors.border.skia(), opacity),
        );
        canvas.draw_round_rect(
            rect(b),
            self.style.geometry.radius,
            self.style.geometry.radius,
            &paint(
                blend(
                    self.style.colors.background.skia(),
                    self.style.colors.accent.skia(),
                    highlight,
                ),
                opacity,
            ),
        );
        let mut border = paint(self.style.colors.border.skia(), opacity);
        border.set_style(skia_safe::paint::Style::Stroke);
        border.set_stroke_width(self.style.geometry.border_width);
        canvas.draw_round_rect(
            rect(b),
            self.style.geometry.radius,
            self.style.geometry.radius,
            &border,
        );
        self.centered(
            &value.to_string(),
            b,
            (b.height * 0.55).clamp(9., 24.),
            true,
            opacity,
        )
    }
    fn draw_cells(&self, s: &ObjectSample<'_>) -> Result<()> {
        let canvas = self.canvas;
        let to = layout::cells(s.object, s.values.len());
        let from = layout::cells(s.object, s.previous_values.len());
        let p = s.value_progress;
        // Stable slot labels stay still while value tiles move between them.
        for (i, b) in to.iter().enumerate() {
            self.centered(
                &format!("{i:02}"),
                Box2 {
                    y: b.y + b.height + 6.,
                    height: 16.,
                    ..*b
                },
                13.,
                false,
                0.65,
            )?;
        }
        if p < 1. {
            for (i, value) in s.previous_values.iter().enumerate() {
                let used = s.origins.is_none_or(|origins| origins.contains(&Some(i)));
                if !used {
                    self.draw_cell(*value, from[i].offset(0., -16. * p), 0., 1. - p)?;
                }
            }
        }
        for (i, value) in s.values.iter().enumerate() {
            let origin = s
                .origins
                .and_then(|origins| origins[i])
                .or_else(|| s.origins.is_none().then_some(i));
            let highlight = s.highlights[i];
            let mut b = to[i];
            let mut opacity = 1.;
            if let Some(source) = origin.and_then(|index| from.get(index)) {
                b = layout::motion(*source, b, p);
                let source_index = origin.expect("source index exists");
                if s.previous_values[source_index] != *value && p < 1. {
                    self.draw_cell(s.previous_values[source_index], b, highlight, 1. - p)?;
                    opacity = p;
                }
            } else {
                b = b.offset(0., 16. * (1. - p));
                opacity = p;
            }
            self.draw_cell(*value, b, highlight, opacity)?;
            if highlight > 0.01 {
                let x = to[i].center().0;
                let mut arrow = paint(self.style.colors.text_primary.skia(), highlight);
                arrow.set_stroke_width(2.);
                canvas.draw_line((x - 4., to[i].y - 10.), (x, to[i].y - 6.), &arrow);
                canvas.draw_line((x + 4., to[i].y - 10.), (x, to[i].y - 6.), &arrow);
            }
        }
        if s.values.is_empty() {
            let b = Box2::from(s.object);
            let mut outline = paint(self.style.colors.text_secondary.skia(), 0.4);
            outline.set_style(skia_safe::paint::Style::Stroke);
            outline.set_stroke_width(2.);
            let marker = Box2 {
                x: b.x + b.width / 2. - 52.,
                y: b.y + b.height / 2. - 24.,
                width: 104.,
                height: 48.,
            };
            canvas.draw_rect(rect(marker), &outline);
            self.centered("EMPTY", marker, 14., false, 0.6)?;
        }
        Ok(())
    }
}

pub fn graph_layout(object: &SceneObject) -> Vec<(f32, f32)> {
    layout::graph_points(object, object.values.len())
}
impl Painter<'_> {
    fn draw_graph(&self, s: &ObjectSample<'_>) -> Result<()> {
        let canvas = self.canvas;
        let points = layout::graph_points(s.object, s.values.len());
        let separation = points
            .iter()
            .enumerate()
            .flat_map(|(i, a)| {
                points
                    .iter()
                    .skip(i + 1)
                    .map(move |b| (a.0 - b.0).hypot(a.1 - b.1))
            })
            .fold(64., f32::min);
        let radius = (separation * 0.4).clamp(6., 25.);
        for [a, b] in &s.object.edges {
            let (Some(from), Some(to)) = (points.get(*a), points.get(*b)) else {
                continue;
            };
            let delta = (to.0 - from.0, to.1 - from.1);
            let distance = delta.0.hypot(delta.1).max(1.);
            let direction = (delta.0 / distance, delta.1 / distance);
            let start = (from.0 + direction.0 * radius, from.1 + direction.1 * radius);
            let end = (to.0 - direction.0 * radius, to.1 - direction.1 * radius);
            let active = s.highlights[*a].min(s.highlights[*b]);
            let mut edge = paint(
                blend(
                    self.style.colors.text_secondary.skia(),
                    self.style.colors.text_primary.skia(),
                    active,
                ),
                0.4 + 0.6 * active,
            );
            edge.set_stroke_width(2. + active);
            canvas.draw_line(start, end, &edge);
            if s.object.directed {
                for sign in [-1., 1.] {
                    canvas.draw_line(
                        end,
                        (
                            end.0 - direction.0 * 8. + direction.1 * 4. * sign,
                            end.1 - direction.1 * 8. - direction.0 * 4. * sign,
                        ),
                        &edge,
                    );
                }
            }
        }
        for (i, value) in s.values.iter().enumerate() {
            let point = points[i];
            let highlight = s.highlights[i];
            canvas.draw_circle(
                (
                    point.0 + self.style.geometry.shadow_offset,
                    point.1 + self.style.geometry.shadow_offset,
                ),
                radius,
                &paint(self.style.colors.border.skia(), 1.),
            );
            canvas.draw_circle(
                point,
                radius,
                &paint(
                    blend(
                        self.style.colors.background.skia(),
                        self.style.colors.accent.skia(),
                        highlight,
                    ),
                    1.,
                ),
            );
            let mut border = paint(self.style.colors.border.skia(), 1.);
            border.set_style(skia_safe::paint::Style::Stroke);
            border.set_stroke_width(self.style.geometry.border_width);
            canvas.draw_circle(point, radius, &border);
            let node = Box2 {
                x: point.0 - radius,
                y: point.1 - radius,
                width: 2. * radius,
                height: 2. * radius,
            };
            self.centered(
                &value.to_string(),
                node,
                (radius * 0.8).clamp(8., 20.),
                true,
                1.,
            )?;
            self.centered(
                &i.to_string(),
                Box2 {
                    y: point.1 + radius + 4.,
                    height: 14.,
                    ..node
                },
                13.,
                false,
                0.7,
            )?;
        }
        Ok(())
    }
}
