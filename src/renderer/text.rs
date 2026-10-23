use super::{Faces, font, presentation};
use crate::style::StyleConfig;
use skia_safe::Font;
use std::{
    collections::{HashMap, VecDeque},
    rc::Rc,
};

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
struct Key {
    size: u32,
    bold: bool,
    width: Option<u32>,
}
pub(super) struct Text {
    pub font: Font,
    pub lines: Vec<String>,
    pub widths: Vec<f32>,
    pub ascent: f32,
    pub descent: f32,
    pub stride: f32,
}
#[derive(Default)]
pub(super) struct Cache {
    entries: HashMap<Key, HashMap<String, Rc<Text>>>,
    order: VecDeque<(Key, String)>,
    bytes: usize,
}
impl Cache {
    pub fn get(
        &mut self,
        value: &str,
        size: f32,
        bold: bool,
        width: Option<f32>,
        faces: &Faces,
        style: &StyleConfig,
    ) -> Rc<Text> {
        let size = presentation::text_size(size, style);
        let key = Key {
            size: size.to_bits(),
            bold,
            width: width.map(f32::to_bits),
        };
        if let Some(hit) = self
            .entries
            .get(&key)
            .and_then(|entries| entries.get(value))
        {
            return hit.clone();
        }
        let f = font(faces, size, bold);
        let lines = width
            .map(|width| presentation::lines(value, &f, width))
            .unwrap_or_else(|| value.split('\n').map(str::to_owned).collect());
        let widths = lines
            .iter()
            .map(|line| f.measure_str(line, None).0)
            .collect();
        let (_, metrics) = f.metrics();
        let text = Rc::new(Text {
            font: f,
            lines,
            widths,
            ascent: metrics.ascent,
            descent: metrics.descent,
            stride: size * style.fonts.line_height,
        });
        while self.order.len() >= 512 || self.bytes + value.len() > 1024 * 1024 {
            let Some((key, value)) = self.order.pop_front() else {
                break;
            };
            self.bytes -= value.len();
            if let Some(entries) = self.entries.get_mut(&key) {
                entries.remove(&value);
                if entries.is_empty() {
                    self.entries.remove(&key);
                }
            }
        }
        if value.len() <= 1024 * 1024 {
            self.bytes += value.len();
            self.order.push_back((key, value.into()));
            self.entries
                .entry(key)
                .or_default()
                .insert(value.into(), text.clone());
        }
        text
    }
}
