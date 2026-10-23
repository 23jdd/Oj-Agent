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
mod presentation;
pub mod viewport;
use layout::Box2;
pub use viewport::{RasterSize, Viewport};

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
    faces: Faces,
    geometry: Vec<presentation::Geometry>,
    export_size: RasterSize,
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

#[derive(Clone)]
struct Faces {
    regular: Typeface,
    heavy: Typeface,
}
thread_local! { static FACES: RefCell<Option<(String, Faces)>> = const { RefCell::new(None) }; }
fn faces(family: &str, file: &str) -> Result<Faces> {
    if !file.is_empty() {
        let face = crate::style::fonts::load(std::path::Path::new(file))?.face;
        return Ok(Faces {
            regular: face.clone(),
            heavy: face,
        });
    }
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
            *cached.borrow_mut() = Some((family.to_owned(), Faces { regular, heavy }));
        }
        let faces = cached.borrow();
        Ok(faces.as_ref().expect("font faces initialized").1.clone())
    })
}
fn font(faces: &Faces, size: f32, bold: bool) -> Font {
    let mut font = Font::from_typeface(
        if bold {
            faces.heavy.clone()
        } else {
            faces.regular.clone()
        },
        size,
    );
    font.set_edging(skia_safe::font::Edging::AntiAlias);
    font.set_hinting(skia_safe::FontHinting::Normal);
    font.set_subpixel(false);
    font.set_baseline_snap(true);
    font
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
    faces: &'a Faces,
    geometry: presentation::Geometry,
}
impl Painter<'_> {
    fn centered(&self, value: &str, b: Box2, size: f32, bold: bool, opacity: f32) -> Result<()> {
        let canvas = self.canvas;
        let size = presentation::text_size(size, self.style);
        let f = font(self.faces, size, bold);
        let (_, metrics) = f.metrics();
        let center = b.center();
        let lines: Vec<_> = value.split('\n').collect();
        let stride = size * self.style.fonts.line_height;
        for (i, line) in lines.iter().enumerate() {
            canvas.draw_str(
                line,
                (
                    center.0 - f.measure_str(line, None).0 / 2.,
                    center.1 - (metrics.ascent + metrics.descent) / 2.
                        + (i as f32 - (lines.len() - 1) as f32 / 2.) * stride,
                ),
                &f,
                &paint(self.style.colors.text_primary.skia(), opacity),
            );
        }
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
        let size = presentation::text_size(size, self.style);
        let f = font(self.faces, size, bold);
        let (_, metrics) = f.metrics();
        let stride = size * self.style.fonts.line_height;
        let lines = presentation::lines(value, &f, b.width);
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
        let mut timeline = Timeline::new(animation)?;
        let faces = faces(&style.fonts.animation_family, &style.fonts.animation_file)?;
        let geometry = presentation::prepare(&mut timeline, animation, &faces, style);
        Ok(PreparedScene {
            timeline,
            style: style.clone(),
            faces,
            geometry,
            export_size: RasterSize::fit(
                (animation.scene.width, animation.scene.height),
                (
                    animation.scene.width.min(1920) as f32,
                    animation.scene.height.min(1080) as f32,
                ),
                1.,
            )
            .ok_or_else(|| AppError::validation("Rendering", "Invalid export size."))?,
        })
    }
}
impl PreparedScene {
    pub fn export_size(&self) -> RasterSize {
        self.export_size
    }
    pub fn frame_key(&self, time: f64) -> FrameKey {
        self.timeline.frame_key(time)
    }
    pub fn frame(&self, time: f64) -> Result<Frame> {
        self.frame_at_size(time, self.export_size)
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
        self.render_into(time, size, None, rgba)
    }
    pub fn frame_viewport(&self, time: f64, viewport: Viewport) -> Result<Frame> {
        Viewport::new(viewport.size, viewport.x, viewport.y, viewport.scale)?;
        let mut rgba = vec![0; viewport.size.width as usize * viewport.size.height as usize * 4];
        self.frame_viewport_into(time, viewport, &mut rgba)?;
        Ok(Frame {
            width: viewport.size.width,
            height: viewport.size.height,
            rgba,
        })
    }
    pub(crate) fn frame_viewport_into(
        &self,
        time: f64,
        viewport: Viewport,
        rgba: &mut [u8],
    ) -> Result<()> {
        let viewport = Viewport::new(viewport.size, viewport.x, viewport.y, viewport.scale)?;
        self.render_into(time, viewport.size, Some(viewport), rgba)
    }
    fn render_into(
        &self,
        time: f64,
        size: RasterSize,
        viewport: Option<Viewport>,
        rgba: &mut [u8],
    ) -> Result<()> {
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
        if let Some(viewport) = viewport {
            canvas.scale((viewport.scale, viewport.scale));
            canvas.translate((-viewport.x, -viewport.y));
        } else {
            let scale = (width as f32 / self.timeline.width as f32)
                .min(height as f32 / self.timeline.height as f32);
            canvas.translate((
                (width as f32 - self.timeline.width as f32 * scale) / 2.,
                (height as f32 - self.timeline.height as f32 * scale) / 2.,
            ));
            canvas.scale((scale, scale));
        }
        for (track, geometry) in self.timeline.objects.iter().zip(&self.geometry) {
            let painter = Painter {
                canvas,
                style: &self.style,
                faces: &self.faces,
                geometry: *geometry,
            };
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
            match sample.object.kind.as_str() {
                "text" => painter.draw_text_object(&sample)?,
                "graph" | "tree" | "linked-list" => painter.draw_graph(&sample)?,
                "grid" => painter.draw_grid(&sample)?,
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
            let header_height =
                presentation::label_size("STATE", self.faces, self.style, 13., true).1;
            self.text_block(
                "STATE",
                Box2 {
                    height: header_height,
                    ..b
                },
                13.,
                self.style.colors.text_secondary.skia(),
                1.,
                true,
            )?;
            b.y += header_height + 12.;
            b.height = (b.height - header_height - 12.).max(1.);
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
    fn state_fill(&self, state: &str, highlight: f32) -> Color {
        let base = match state {
            "active" | "current" | "visiting" => self.style.colors.accent.skia(),
            "done" | "visited" | "success" => self.style.colors.success.skia(),
            "error" | "failed" => self.style.colors.danger.skia(),
            _ => self.style.colors.background.skia(),
        };
        blend(base, self.style.colors.accent.skia(), highlight)
    }
    fn draw_cell(&self, value: i64, b: Box2, highlight: f32, opacity: f32) -> Result<()> {
        self.draw_cell_label(&value.to_string(), b, highlight, opacity, "normal")
    }
    fn draw_cell_label(
        &self,
        label: &str,
        b: Box2,
        highlight: f32,
        opacity: f32,
        state: &str,
    ) -> Result<()> {
        let canvas = self.canvas;
        if opacity <= 0.001 || state == "hidden" {
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
            &paint(self.state_fill(state, highlight), opacity),
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
        if state == "empty" && label.is_empty() {
            return Ok(());
        }
        self.centered(label, b, 24., true, opacity)
    }
    fn draw_pointer(
        &self,
        name: &str,
        x: f32,
        anchor_y: f32,
        stack: f32,
        opacity: f32,
    ) -> Result<()> {
        let (width, height) = presentation::label_size(name, self.faces, self.style, 13., true);
        let label_y = anchor_y - height - 8. - stack;
        let mut line = paint(self.style.colors.text_primary.skia(), opacity);
        line.set_stroke_width(1.5);
        self.canvas
            .draw_line((x, label_y + height), (x, anchor_y), &line);
        self.canvas
            .draw_line((x - 4., anchor_y - 4.), (x, anchor_y), &line);
        self.canvas
            .draw_line((x + 4., anchor_y - 4.), (x, anchor_y), &line);
        self.centered(
            name,
            Box2 {
                x: x - width / 2.,
                y: label_y,
                width,
                height,
            },
            13.,
            true,
            opacity,
        )
    }
    fn draw_pointers(
        &self,
        pointers: &[ScenePointer],
        anchors: &[(f32, f32)],
        opacity: f32,
    ) -> Result<()> {
        let mut stacks = std::collections::BTreeMap::<usize, f32>::new();
        for pointer in pointers {
            let Some((x, y)) = anchors.get(pointer.target) else {
                continue;
            };
            let stack = stacks.entry(pointer.target).or_insert(0.);
            self.draw_pointer(&pointer.name, *x, *y, *stack, opacity)?;
            *stack +=
                presentation::label_size(&pointer.name, self.faces, self.style, 13., true).1 + 8.;
        }
        Ok(())
    }
    fn draw_edge(
        &self,
        start: (f32, f32),
        end: (f32, f32),
        active: f32,
        label: &str,
        directed: bool,
        opacity: f32,
    ) -> Result<()> {
        let mut edge = paint(
            blend(
                self.style.colors.text_secondary.skia(),
                self.style.colors.text_primary.skia(),
                active,
            ),
            (0.4 + 0.6 * active) * opacity,
        );
        edge.set_stroke_width(2. + active);
        self.canvas.draw_line(start, end, &edge);
        if directed {
            let delta = (end.0 - start.0, end.1 - start.1);
            let distance = delta.0.hypot(delta.1).max(1.);
            let direction = (delta.0 / distance, delta.1 / distance);
            for sign in [-1., 1.] {
                self.canvas.draw_line(
                    end,
                    (
                        end.0 - direction.0 * 8. + direction.1 * 4. * sign,
                        end.1 - direction.1 * 8. - direction.0 * 4. * sign,
                    ),
                    &edge,
                );
            }
        }
        if !label.is_empty() {
            let (width, height) =
                presentation::label_size(label, self.faces, self.style, 12., false);
            self.centered(
                label,
                Box2 {
                    x: (start.0 + end.0) / 2. - width / 2.,
                    y: (start.1 + end.1) / 2. - height - 4.,
                    width,
                    height,
                },
                12.,
                false,
                opacity,
            )?;
        }
        Ok(())
    }
    fn empty_marker(&self, object: &SceneObject) -> Result<()> {
        let b = Box2::from(object);
        let mut outline = paint(self.style.colors.text_secondary.skia(), 0.4);
        outline.set_style(skia_safe::paint::Style::Stroke);
        outline.set_stroke_width(2.);
        let marker = Box2 {
            x: b.x + b.width / 2. - 52.,
            y: b.y + b.height / 2. - 24.,
            width: 104.,
            height: 48.,
        };
        self.canvas.draw_rect(rect(marker), &outline);
        self.centered("EMPTY", marker, 14., false, 0.6)
    }
    fn draw_cells(&self, s: &ObjectSample<'_>) -> Result<()> {
        let canvas = self.canvas;
        let to = layout::natural_cells(
            s.object,
            s.values.len(),
            self.geometry.cell,
            self.geometry.pad,
            self.geometry.gap,
        );
        let from = layout::natural_cells(
            s.object,
            s.previous_values.len(),
            self.geometry.cell,
            self.geometry.pad,
            self.geometry.gap,
        );
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
        let anchors: Vec<(f32, f32)> = to.iter().map(|b| (b.center().0, b.y)).collect();
        self.draw_pointers(&s.structure.pointers, &anchors, 1.)?;
        if s.values.is_empty() {
            self.empty_marker(s.object)?;
        }
        Ok(())
    }
}

pub fn graph_layout(object: &SceneObject) -> Vec<(f32, f32)> {
    layout::graph_points(object, object.values.len())
}
impl Painter<'_> {
    fn draw_grid(&self, s: &ObjectSample<'_>) -> Result<()> {
        if s.values.is_empty() {
            return self.empty_marker(s.object);
        }
        let cells = layout::natural_cells(
            s.object,
            s.values.len(),
            self.geometry.cell,
            self.geometry.pad,
            self.geometry.gap,
        );
        let trim = cells
            .first()
            .map(|cell| cell.width.min(cell.height) * 0.45)
            .unwrap_or(8.);
        for (i, [a, b]) in s.structure.edges.iter().enumerate() {
            let (Some(from), Some(to)) = (cells.get(*a), cells.get(*b)) else {
                continue;
            };
            let start = from.center();
            let end = to.center();
            let delta = (end.0 - start.0, end.1 - start.1);
            let distance = delta.0.hypot(delta.1).max(1.);
            let direction = (delta.0 / distance, delta.1 / distance);
            let start = (start.0 + direction.0 * trim, start.1 + direction.1 * trim);
            let end = (end.0 - direction.0 * trim, end.1 - direction.1 * trim);
            let active = s.highlights[*a].min(s.highlights[*b]);
            let label = s
                .structure
                .edge_labels
                .get(i)
                .map(String::as_str)
                .unwrap_or("");
            self.draw_edge(start, end, active, label, s.object.directed, 1.)?;
        }
        for (i, cell) in cells.iter().enumerate() {
            let Some(value) = s.values.get(i) else {
                continue;
            };
            let state = s
                .structure
                .states
                .get(i)
                .map(String::as_str)
                .unwrap_or("normal");
            let label = s
                .structure
                .labels
                .get(i)
                .filter(|label| !label.is_empty())
                .cloned()
                .unwrap_or_else(|| value.to_string());
            self.draw_cell_label(&label, *cell, s.highlights[i], 1., state)?;
        }
        let anchors: Vec<(f32, f32)> = cells.iter().map(|cell| (cell.center().0, cell.y)).collect();
        self.draw_pointers(&s.structure.pointers, &anchors, 1.)
    }
    fn draw_graph(&self, s: &ObjectSample<'_>) -> Result<()> {
        let canvas = self.canvas;
        if s.values.is_empty() {
            return self.empty_marker(s.object);
        }
        let points = layout::graph_points_padded(
            s.object,
            s.values.len(),
            &s.structure.positions,
            &s.structure.edges,
            self.geometry.pad,
        );
        let radius = self.geometry.radius;
        let hidden: Vec<bool> = (0..s.values.len())
            .map(|i| {
                s.structure
                    .states
                    .get(i)
                    .is_some_and(|state| state == "hidden")
            })
            .collect();
        for (i, [a, b]) in s.structure.edges.iter().enumerate() {
            if hidden.get(*a).copied().unwrap_or(false) || hidden.get(*b).copied().unwrap_or(false)
            {
                continue;
            }
            let (Some(from), Some(to)) = (points.get(*a), points.get(*b)) else {
                continue;
            };
            let delta = (to.0 - from.0, to.1 - from.1);
            let distance = delta.0.hypot(delta.1).max(1.);
            let direction = (delta.0 / distance, delta.1 / distance);
            let start = (from.0 + direction.0 * radius, from.1 + direction.1 * radius);
            let end = (to.0 - direction.0 * radius, to.1 - direction.1 * radius);
            let active = s.highlights[*a].min(s.highlights[*b]);
            let label = s
                .structure
                .edge_labels
                .get(i)
                .map(String::as_str)
                .unwrap_or("");
            self.draw_edge(start, end, active, label, s.object.directed, 1.)?;
        }
        for (i, value) in s.values.iter().enumerate() {
            if hidden.get(i).copied().unwrap_or(false) {
                continue;
            }
            let point = points[i];
            let highlight = s.highlights[i];
            let state = s
                .structure
                .states
                .get(i)
                .map(String::as_str)
                .unwrap_or("normal");
            canvas.draw_circle(
                (
                    point.0 + self.style.geometry.shadow_offset,
                    point.1 + self.style.geometry.shadow_offset,
                ),
                radius,
                &paint(self.style.colors.border.skia(), 1.),
            );
            canvas.draw_circle(point, radius, &paint(self.state_fill(state, highlight), 1.));
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
            let label = s
                .structure
                .labels
                .get(i)
                .filter(|label| !label.is_empty())
                .cloned()
                .unwrap_or_else(|| value.to_string());
            self.centered(&label, node, 20., true, 1.)?;
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
        let anchors: Vec<(f32, f32)> = points
            .iter()
            .map(|point| (point.0, point.1 - radius - 6.))
            .collect();
        self.draw_pointers(&s.structure.pointers, &anchors, 1.)
    }
}
