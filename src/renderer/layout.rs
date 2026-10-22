use crate::{animation::easing::lerp, ir::SceneObject};

#[derive(Debug, Clone, Copy)]
pub struct Box2 {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
impl Box2 {
    pub fn center(self) -> (f32, f32) {
        (self.x + self.width / 2., self.y + self.height / 2.)
    }
    pub fn mix(self, to: Self, t: f32) -> Self {
        Self {
            x: lerp(self.x, to.x, t),
            y: lerp(self.y, to.y, t),
            width: lerp(self.width, to.width, t),
            height: lerp(self.height, to.height, t),
        }
    }
    pub fn offset(self, x: f32, y: f32) -> Self {
        Self {
            x: self.x + x,
            y: self.y + y,
            ..self
        }
    }
}
impl From<&SceneObject> for Box2 {
    fn from(o: &SceneObject) -> Self {
        Self {
            x: o.x,
            y: o.y,
            width: o.width,
            height: o.height,
        }
    }
}

/// Labels and gaps are included in the layout budget, including 64-value views.
pub fn cells(object: &SceneObject, count: usize) -> Vec<Box2> {
    if count == 0 {
        return vec![];
    }
    let stack = object.kind == "stack";
    let columns = if stack {
        count.div_ceil(6)
    } else if object.kind == "table" {
        count.min(8)
    } else {
        count.min(12)
    };
    let rows = count.div_ceil(columns);
    let gap = 12f32.min(object.width / (columns.max(1) * 4) as f32);
    let width = ((object.width - gap * (columns + 1) as f32) / columns as f32).clamp(1., 88.);
    let row_stride = ((object.height - 64.).max(1.) / rows as f32).min(92.);
    let height = (row_stride - 24.).clamp(1., 64.);
    let left = object.x + (object.width - columns as f32 * width - (columns - 1) as f32 * gap) / 2.;
    let top = object.y + (object.height - rows as f32 * row_stride) / 2.;
    (0..count)
        .map(|i| {
            let (column, row) = if stack {
                (i / rows, rows - 1 - i % rows)
            } else {
                (i % columns, i / columns)
            };
            Box2 {
                x: left + column as f32 * (width + gap),
                y: top + row as f32 * row_stride,
                width,
                height,
            }
        })
        .collect()
}

pub fn motion(from: Box2, to: Box2, progress: f32) -> Box2 {
    let mut position = from.mix(to, progress);
    if (from.x - to.x).abs() > 1. {
        let arc = (from.height.max(to.height) / 2. + 8.).min(48.);
        position.y +=
            (std::f32::consts::PI * progress).sin() * if from.x < to.x { -arc } else { arc };
    }
    position
}

pub fn graph_points(object: &SceneObject, count: usize) -> Vec<(f32, f32)> {
    if count == 0 {
        return vec![];
    }
    let pad = 48.;
    let width = (object.width - 2. * pad).max(1.);
    let height = (object.height - 2. * pad).max(1.);
    if object.kind == "tree" {
        fn leaves(i: usize, n: usize) -> usize {
            if i >= n {
                0
            } else if 2 * i + 1 >= n {
                1
            } else {
                leaves(2 * i + 1, n) + leaves(2 * i + 2, n)
            }
        }
        fn place(
            i: usize,
            left: f32,
            right: f32,
            level: usize,
            out: &mut [(f32, f32)],
            y: f32,
            stride: f32,
        ) {
            let count = out.len();
            if i >= count {
                return;
            }
            let a = leaves(2 * i + 1, count);
            let b = leaves(2 * i + 2, count);
            let split = if a + b == 0 {
                (left + right) / 2.
            } else {
                left + (right - left) * a as f32 / (a + b) as f32
            };
            place(2 * i + 1, left, split, level + 1, out, y, stride);
            place(2 * i + 2, split, right, level + 1, out, y, stride);
            let x = match (2 * i + 1 < count, 2 * i + 2 < count) {
                (true, true) => (out[2 * i + 1].0 + out[2 * i + 2].0) / 2.,
                (true, false) => out[2 * i + 1].0,
                _ => (left + right) / 2.,
            };
            out[i] = (x, y + level as f32 * stride);
        }
        let mut points = vec![(0., 0.); count];
        place(
            0,
            object.x + pad,
            object.x + pad + width,
            0,
            &mut points,
            object.y + pad,
            height / count.ilog2().max(1) as f32,
        );
        points
    } else if count <= 16 {
        (0..count)
            .map(|i| {
                let angle =
                    i as f32 / count as f32 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                (
                    object.x + object.width / 2. + angle.cos() * width / 2.,
                    object.y + object.height / 2. + angle.sin() * height / 2.,
                )
            })
            .collect()
    } else {
        let columns = ((count as f32 * width / height).sqrt().ceil() as usize).min(count);
        let rows = count.div_ceil(columns);
        (0..count)
            .map(|i| {
                (
                    object.x + pad + (i % columns) as f32 * width / (columns - 1).max(1) as f32,
                    object.y + pad + (i / columns) as f32 * height / (rows - 1).max(1) as f32,
                )
            })
            .collect()
    }
}
