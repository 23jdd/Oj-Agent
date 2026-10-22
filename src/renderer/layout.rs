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
    graph_points_for(object, count, &[], &object.edges)
}

/// Uses normalized explicit positions when available, then an explicit tree/list
/// layout, and finally the legacy graph/circular fallback.
pub fn graph_points_for(
    object: &SceneObject,
    count: usize,
    positions: &[[f32; 2]],
    edges: &[[usize; 2]],
) -> Vec<(f32, f32)> {
    if count == 0 {
        return vec![];
    }
    let pad = 48.;
    let width = (object.width - 2. * pad).max(1.);
    let height = (object.height - 2. * pad).max(1.);
    if positions.len() == count {
        return positions
            .iter()
            .map(|[x, y]| {
                (
                    object.x + pad + x.clamp(0., 1.) * width,
                    object.y + pad + y.clamp(0., 1.) * height,
                )
            })
            .collect();
    }
    if object.kind == "tree" && !edges.is_empty() {
        return tree_points(object, count, edges, pad, width, height);
    }
    if object.kind == "linked-list" {
        return linked_points(object, count, edges, pad, width, height);
    }
    if object.kind == "tree" {
        return complete_tree_points(object, count, pad, width, height);
    }
    if count <= 16 {
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

fn complete_tree_points(
    object: &SceneObject,
    count: usize,
    pad: f32,
    width: f32,
    height: f32,
) -> Vec<(f32, f32)> {
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
}

fn tree_points(
    object: &SceneObject,
    count: usize,
    edges: &[[usize; 2]],
    pad: f32,
    width: f32,
    height: f32,
) -> Vec<(f32, f32)> {
    let mut children = vec![Vec::new(); count];
    let mut incoming = vec![false; count];
    for [from, to] in edges {
        if *from < count && *to < count && *from != *to {
            children[*from].push(*to);
            incoming[*to] = true;
        }
    }
    let mut roots: Vec<usize> = (0..count).filter(|i| !incoming[*i]).collect();
    if roots.is_empty() {
        roots.push(0);
    }
    fn leaf_count(
        index: usize,
        children: &[Vec<usize>],
        leaves: &mut [usize],
        seen: &mut [bool],
    ) -> usize {
        if seen[index] {
            return 1;
        }
        seen[index] = true;
        let n = if children[index].is_empty() {
            1
        } else {
            let mut total = 0;
            for child in children[index].clone() {
                total += leaf_count(child, children, leaves, seen);
            }
            total.max(1)
        };
        leaves[index] = n;
        n
    }
    let mut leaves = vec![0usize; count];
    let mut seen = vec![false; count];
    let mut total_leaves = 0;
    for root in roots.clone() {
        total_leaves += leaf_count(root, &children, &mut leaves, &mut seen);
    }
    let mut xs = vec![0f32; count];
    let mut depth = vec![0usize; count];
    let max_depth = {
        fn visit(
            index: usize,
            level: usize,
            children: &[Vec<usize>],
            depth: &mut [usize],
            seen: &mut [bool],
        ) {
            if seen[index] {
                return;
            }
            seen[index] = true;
            depth[index] = level;
            for child in children[index].clone() {
                visit(child, level + 1, children, depth, seen);
            }
        }
        let mut seen = vec![false; count];
        for root in &roots {
            visit(*root, 0, &children, &mut depth, &mut seen);
        }
        depth.iter().copied().max().unwrap_or(0)
    };
    fn assign(
        index: usize,
        next_leaf: &mut f32,
        children: &[Vec<usize>],
        xs: &mut [f32],
        seen: &mut [bool],
    ) {
        if seen[index] {
            return;
        }
        seen[index] = true;
        if children[index].is_empty() {
            xs[index] = *next_leaf;
            *next_leaf += 1.;
        } else {
            for child in children[index].clone() {
                assign(child, next_leaf, children, xs, seen);
            }
            let child_count = children[index].len().max(1) as f32;
            xs[index] = children[index].iter().map(|child| xs[*child]).sum::<f32>() / child_count;
        }
    }
    let mut next_leaf = 0f32;
    let mut seen = vec![false; count];
    for root in &roots {
        assign(*root, &mut next_leaf, &children, &mut xs, &mut seen);
    }
    let max_x = (total_leaves.max(1) - 1) as f32;
    (0..count)
        .map(|i| {
            let x = if max_x > 0. { xs[i] / max_x } else { 0.5 };
            let y = if max_depth > 0 {
                depth[i] as f32 / max_depth as f32
            } else {
                0.5
            };
            (object.x + pad + x * width, object.y + pad + y * height)
        })
        .collect()
}

fn linked_points(
    object: &SceneObject,
    count: usize,
    edges: &[[usize; 2]],
    pad: f32,
    width: f32,
    height: f32,
) -> Vec<(f32, f32)> {
    let mut next = vec![None; count];
    let mut incoming = vec![false; count];
    for [from, to] in edges {
        if *from < count && *to < count && next[*from].is_none() {
            next[*from] = Some(*to);
            incoming[*to] = true;
        }
    }
    let mut order = Vec::with_capacity(count);
    let mut seen = vec![false; count];
    let root = (0..count).find(|i| !incoming[*i]).unwrap_or(0);
    let mut current = Some(root);
    while let Some(i) = current {
        if seen[i] {
            break;
        }
        seen[i] = true;
        order.push(i);
        current = next[i];
    }
    for (i, was_seen) in seen.iter().enumerate() {
        if !was_seen {
            order.push(i);
        }
    }
    let y = object.y + object.height / 2. + height * 0.2;
    let mut xs = vec![0f32; count];
    for (position, index) in order.iter().enumerate() {
        xs[*index] = if count <= 1 {
            0.5
        } else {
            position as f32 / (count - 1) as f32
        };
    }
    (0..count)
        .map(|index| (object.x + pad + xs[index] * width, y))
        .collect()
}

pub fn grid_cells(object: &SceneObject) -> Vec<Box2> {
    let rows = object.rows.max(1) as usize;
    let cols = object.cols.max(1) as usize;
    let gap = 8f32;
    let width = ((object.width - gap * (cols + 1) as f32) / cols as f32).max(1.);
    let height = ((object.height - gap * (rows + 1) as f32) / rows as f32).max(1.);
    (0..rows * cols)
        .map(|i| Box2 {
            x: object.x + gap + (i % cols) as f32 * (width + gap),
            y: object.y + gap + (i / cols) as f32 * (height + gap),
            width,
            height,
        })
        .collect()
}
