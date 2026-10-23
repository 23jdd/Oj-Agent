//! Readable, stable geometry reserved across the entire timeline, independent of the camera.
use super::{Faces, font, layout};
use crate::{animation::timeline::Timeline, ir::*, style::StyleConfig};

#[derive(Clone, Copy, Default)]
pub(super) struct Geometry {
    pub cell: (f32, f32),
    pub pad: f32,
    pub radius: f32,
    pub gap: f32,
}

pub(super) fn text_size(size: f32, style: &StyleConfig) -> f32 {
    (size * style.fonts.animation_scale).max(style.fonts.minimum_text_pixels)
}

pub(super) fn lines(value: &str, font: &skia_safe::Font, width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in value.split('\n') {
        let mut line = String::new();
        let mut advance = 0.;
        for ch in paragraph.chars() {
            let next = font.measure_str(ch.to_string(), None).0;
            if !line.is_empty() && advance + next > width {
                lines.push(std::mem::take(&mut line));
                advance = 0.;
            }
            line.push(ch);
            advance += next;
        }
        lines.push(line);
    }
    lines
}

pub(super) fn label_size(
    value: &str,
    faces: &Faces,
    style: &StyleConfig,
    size: f32,
    bold: bool,
) -> (f32, f32) {
    let size = text_size(size, style);
    let f = font(faces, size, bold);
    let (_, metrics) = f.metrics();
    (
        value
            .split('\n')
            .map(|line| f.measure_str(line, None).0)
            .fold(0., f32::max),
        metrics.descent - metrics.ascent
            + value.matches('\n').count() as f32 * size * style.fonts.line_height,
    )
}

pub(super) fn prepare(
    timeline: &mut Timeline,
    animation: &AnimationIr,
    faces: &Faces,
    style: &StyleConfig,
) -> Vec<Geometry> {
    let originals: Vec<_> = timeline
        .objects
        .iter()
        .map(|track| track.object.clone())
        .collect();
    let mut geometry = vec![Geometry::default(); originals.len()];
    for (index, track) in timeline.objects.iter_mut().enumerate() {
        let object = &mut track.object;
        let actions: Vec<_> = animation
            .timeline
            .iter()
            .filter(|a| a.target == object.id)
            .collect();
        if object.kind == "text" {
            let title = object.id == "title";
            let subtitle = object.id == "subtitle";
            let state = object.id == "state_view";
            let size = text_size(
                if title {
                    28.
                } else if subtitle {
                    14.
                } else if state {
                    17.
                } else {
                    18.
                },
                style,
            );
            let f = font(faces, size, title);
            let (_, metrics) = f.metrics();
            let inset = if title || subtitle { 0. } else { 36. };
            let mut height = object.height;
            for text in std::iter::once(object.text.as_str()).chain(
                actions
                    .iter()
                    .filter(|a| a.action == Action::UpdateText)
                    .map(|a| a.text.as_str()),
            ) {
                // One wide glyph must also fit, even when the source box was narrower than a letter.
                object.width = object.width.max(
                    text.chars()
                        .map(|c| f.measure_str(c.to_string(), None).0 + inset)
                        .fold(0., f32::max),
                );
                let count = lines(text, &f, (object.width - inset).max(1.)).len();
                let padding = if title || subtitle {
                    0.
                } else if state {
                    36. + label_size("STATE", faces, style, 13., true).1
                } else {
                    24.
                };
                height = height.max(
                    metrics.descent - metrics.ascent
                        + (count - 1) as f32 * size * style.fonts.line_height
                        + padding
                        + 8.,
                );
            }
            object.height = height;
            continue;
        }
        let graph = matches!(object.kind.as_str(), "tree" | "graph" | "linked-list");
        let mut g = Geometry {
            cell: (64., 48.),
            pad: 48.,
            radius: 25.,
            gap: label_size("0", faces, style, 13., false).1 + 12.,
        };
        let mut count = object.values.len();
        let mut node_gap = 40f32;
        let variants: Vec<_> = std::iter::once((object.values.as_slice(), object.structure()))
            .chain(
                actions
                    .iter()
                    .filter(|a| a.action == Action::UpdateValue)
                    .map(|a| {
                        (
                            a.values.as_slice(),
                            a.structure.clone().unwrap_or_else(|| object.structure()),
                        )
                    }),
            )
            .collect();
        for (values, structure) in &variants {
            count = count.max(values.len());
            for label in values
                .iter()
                .map(ToString::to_string)
                .chain(structure.labels.iter().cloned())
            {
                let (w, h) = label_size(&label, faces, style, if graph { 20. } else { 24. }, true);
                g.cell.0 = g.cell.0.max(w + 24.);
                g.cell.1 = g.cell.1.max(h + 16.);
                g.radius = g.radius.max((w + 16.).hypot(h + 8.) / 2.);
            }
            let mut stacks = std::collections::BTreeMap::<usize, f32>::new();
            for pointer in &structure.pointers {
                let (w, h) = label_size(&pointer.name, faces, style, 13., true);
                *stacks.entry(pointer.target).or_default() += h + 8.;
                g.pad = g.pad.max(w / 2. + 24.);
                node_gap = node_gap.max(w + 24.);
            }
            g.pad = g.pad.max(stacks.values().copied().fold(0., f32::max) + 64.);
            g.gap = g.gap.max(stacks.values().copied().fold(0., f32::max) + 32.);
            node_gap = node_gap.max(stacks.values().copied().fold(0., f32::max) + 32.);
            for edge in &structure.edge_labels {
                let (w, h) = label_size(edge, faces, style, 12., false);
                g.pad = g.pad.max(w / 2. + h + 24.);
                node_gap = node_gap.max(w + 24.).max(h + 24.);
            }
        }
        if graph {
            g.pad += g.radius;
            object.width = object.width.max(2. * g.pad + 96.);
            object.height = object.height.max(2. * g.pad + 96.);
            // Reserve the worst frame so nodes never shrink or jump when labels change.
            for (values, structure) in &variants {
                let points = layout::graph_points_padded(
                    object,
                    values.len(),
                    &structure.positions,
                    &structure.edges,
                    g.pad,
                );
                let spacing = 2. * g.radius + node_gap;
                let (mut sx, mut sy) = (1f32, 1f32);
                for (i, a) in points.iter().enumerate() {
                    for b in &points[i + 1..] {
                        let (dx, dy) = ((a.0 - b.0).abs(), (a.1 - b.1).abs());
                        if dx.hypot(dy) < spacing {
                            if dx >= dy {
                                sx = sx.max(spacing / dx.max(0.001));
                            } else {
                                sy = sy.max(spacing / dy.max(0.001));
                            }
                        }
                    }
                }
                // A deep chain needs vertical space, not an equally huge empty horizontal canvas.
                object.width = 2. * g.pad + (object.width - 2. * g.pad) * sx;
                object.height = 2. * g.pad + (object.height - 2. * g.pad) * sy;
            }
        } else {
            let (cols, rows) = layout::cell_shape(object, count);
            object.width = object
                .width
                .max(2. * g.pad + cols as f32 * (g.cell.0 + g.gap));
            object.height = object
                .height
                .max(2. * g.pad + rows as f32 * (g.cell.1 + g.gap));
        }
        geometry[index] = g;
    }
    // Preserve the source's rows/columns and gaps while moving later panels out of expanded content.
    let mut order: Vec<_> = (0..originals.len()).collect();
    order.sort_by(|a, b| {
        originals[*a]
            .y
            .total_cmp(&originals[*b].y)
            .then(originals[*a].x.total_cmp(&originals[*b].x))
    });
    for (position, &index) in order.iter().enumerate() {
        let original = &originals[index];
        let (mut dx, mut dy) = (0f32, 0f32);
        for &before in &order[..position] {
            let old = &originals[before];
            let new = &timeline.objects[before].object;
            if old.y + old.height <= original.y {
                dy = dy.max(new.y + new.height - old.y - old.height);
            } else if old.x + old.width <= original.x {
                dx = dx.max(new.x + new.width - old.x - old.width);
            }
        }
        let object = &mut timeline.objects[index].object;
        object.x += dx;
        object.y += dy;
        timeline.width = timeline
            .width
            .max((object.x + object.width + 32.).ceil() as u32);
        timeline.height = timeline
            .height
            .max((object.y + object.height + 32.).ceil() as u32);
    }
    geometry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{animation, renderer::SkiaRenderer, trace, visualization};
    fn fixture() -> AnimationIr {
        let (a, t) = trace::simulate(&ProblemInput::default()).unwrap();
        let v = visualization::plan(&a, &t).unwrap();
        let mut ir = animation::compile(&a, &t, &v).unwrap();
        ir.timeline.clear();
        ir.step_times = vec![0.];
        ir
    }
    #[test]
    fn reserves_all_future_text_and_values_without_truncation() {
        let mut ir = fixture();
        let values = vec![i64::MIN; 64];
        ir.timeline.push(AnimationAction {
            at: 0.,
            duration: 0.32,
            action: Action::UpdateValue,
            target: "values_view".into(),
            text: String::new(),
            values,
            indices: vec![],
            structure: None,
        });
        let text = "完整说明 long text\n".repeat(200) + "LAST LINE";
        ir.timeline.push(AnimationAction {
            at: 0.,
            duration: 0.32,
            action: Action::UpdateText,
            target: "explanation".into(),
            text: text.clone(),
            values: vec![],
            indices: vec![],
            structure: None,
        });
        let scene = SkiaRenderer.prepare(&ir).unwrap();
        let index = scene
            .timeline
            .objects
            .iter()
            .position(|t| t.object.id == "values_view")
            .unwrap();
        let object = &scene.timeline.objects[index].object;
        let g = scene.geometry[index];
        let cells = layout::natural_cells(object, 64, g.cell, g.pad, g.gap);
        assert!(
            cells[0].width
                >= label_size(&i64::MIN.to_string(), &scene.faces, &scene.style, 24., true).0 + 24.
        );
        assert!(scene.timeline.width > ir.scene.width);
        let explanation = &scene
            .timeline
            .objects
            .iter()
            .find(|t| t.object.id == "explanation")
            .unwrap()
            .object;
        assert!(explanation.y >= object.y + object.height);
        let f = font(&scene.faces, text_size(18., &scene.style), false);
        let wrapped = lines(&text, &f, explanation.width - 36.);
        assert_eq!(wrapped.join(""), text.replace('\n', ""));
        assert_eq!(wrapped.last().unwrap(), "LAST LINE");
        assert!(explanation.height > 4800.);
        assert!(scene.timeline.height as f32 > explanation.y + explanation.height);
        let camera = super::super::Viewport::new(
            super::super::RasterSize::new(896, 80).unwrap(),
            explanation.x,
            explanation.y + explanation.height - 80.,
            1.,
        )
        .unwrap();
        assert_ne!(
            scene.frame_viewport(0., camera).unwrap().rgba,
            scene.frame_viewport(0.4, camera).unwrap().rgba,
            "the final lines must be painted after scrolling"
        );
    }
    #[test]
    fn dense_deep_and_wide_structures_keep_readable_geometry() {
        for kind in ["graph", "tree", "linked-list", "array", "grid"] {
            let mut ir = fixture();
            let object = ir
                .scene
                .objects
                .iter_mut()
                .find(|o| o.id == "values_view")
                .unwrap();
            object.kind = kind.into();
            let count = if kind == "grid" { 64 } else { 128 };
            object.values = vec![i64::MIN; count];
            object.edges = (1..count).map(|i| [i - 1, i]).collect();
            if kind == "grid" {
                object.rows = 1;
                object.cols = count as u32;
            }
            let scene = SkiaRenderer.prepare(&ir).unwrap();
            let index = scene
                .timeline
                .objects
                .iter()
                .position(|t| t.object.id == "values_view")
                .unwrap();
            let object = &scene.timeline.objects[index].object;
            let g = scene.geometry[index];
            if kind == "tree" {
                assert!(
                    object.width < 4096.,
                    "deep chains should only expand vertically"
                );
            }
            if matches!(kind, "tree" | "graph" | "linked-list") {
                assert!(g.radius >= 25.);
                let points = layout::graph_points_padded(object, count, &[], &object.edges, g.pad);
                for (i, point) in points.iter().enumerate() {
                    assert!(point.0 - g.radius >= object.x && point.1 - g.radius >= object.y);
                    assert!(
                        point.0 + g.radius < object.x + object.width
                            && point.1 + g.radius < object.y + object.height
                    );
                    for other in &points[i + 1..] {
                        assert!(
                            (point.0 - other.0).hypot(point.1 - other.1)
                                >= 2. * g.radius + 24. - 0.1,
                            "{kind}"
                        );
                    }
                }
            } else {
                let cells = layout::natural_cells(object, count, g.cell, g.pad, g.gap);
                assert!(cells.iter().all(|b| b.width >= 64.
                    && b.height >= 48.
                    && b.x + b.width < object.x + object.width
                    && b.y + b.height < object.y + object.height));
            }
            assert!(
                scene.timeline.width > ir.scene.width || scene.timeline.height > ir.scene.height,
                "{kind}"
            );
        }
    }
}
