//! Native-density circle sprites. Reuse static fills without rerasterizing large antialiased paths.
use super::paint;
use crate::style::StyleConfig;
use skia_safe::{AlphaType, Canvas, Color, ColorType, Image, ImageInfo, Paint, surfaces};
use std::collections::{HashMap, VecDeque};

type Key = (u32, u32, u32, u32, u32);
struct Sprite {
    image: Image,
    offset: (f32, f32),
    bytes: usize,
}
#[derive(Default)]
pub(super) struct Cache {
    entries: HashMap<Key, Sprite>,
    order: VecDeque<Key>,
    bytes: usize,
}
impl Cache {
    pub fn draw(
        &mut self,
        canvas: &Canvas,
        point: (f32, f32),
        radius: f32,
        fill: Color,
        style: &StyleConfig,
    ) -> bool {
        let matrix = canvas.local_to_device_as_3x3();
        let scale = matrix.scale_x();
        if !matrix.is_scale_translate() || scale <= 0. || (scale - matrix.scale_y()).abs() > 0.0001
        {
            return false;
        }
        let cx = point.0 * scale + matrix.translate_x();
        let cy = point.1 * scale + matrix.translate_y();
        let phase = (cx - cx.floor(), cy - cy.floor());
        let key = (
            radius.to_bits(),
            scale.to_bits(),
            u32::from_be_bytes([fill.a(), fill.r(), fill.g(), fill.b()]),
            phase.0.to_bits(),
            phase.1.to_bits(),
        );
        if self.entries.contains_key(&key) {
            if let Some(index) = self.order.iter().position(|entry| *entry == key) {
                self.order.remove(index);
            }
            self.order.push_back(key);
        } else {
            let margin = (style.geometry.shadow_offset + style.geometry.border_width + 2.) * scale;
            let r = radius * scale;
            let left = (phase.0 - r - margin).floor();
            let top = (phase.1 - r - margin).floor();
            let width = (phase.0 + r + margin).ceil() - left;
            let height = (phase.1 + r + margin).ceil() - top;
            if width > 2048. || height > 2048. {
                return false;
            }
            let info = ImageInfo::new(
                (width as i32, height as i32),
                ColorType::RGBA8888,
                AlphaType::Premul,
                None,
            );
            let Some(mut surface) = surfaces::raster(&info, None, None) else {
                return false;
            };
            let c = surface.canvas();
            c.clear(Color::TRANSPARENT);
            c.scale((scale, scale));
            let center = ((phase.0 - left) / scale, (phase.1 - top) / scale);
            circle(c, center, radius, fill, style);
            let sprite = Sprite {
                image: surface.image_snapshot(),
                offset: (left, top),
                bytes: width as usize * height as usize * 4,
            };
            while self.bytes + sprite.bytes > 32 * 1024 * 1024 || self.order.len() >= 256 {
                let Some(old) = self.order.pop_front() else {
                    break;
                };
                if let Some(old) = self.entries.remove(&old) {
                    self.bytes -= old.bytes;
                }
            }
            self.bytes += sprite.bytes;
            self.order.push_back(key);
            self.entries.insert(key, sprite);
        }
        let sprite = &self.entries[&key];
        canvas.save();
        canvas.reset_matrix();
        canvas.draw_image(
            &sprite.image,
            (cx.floor() + sprite.offset.0, cy.floor() + sprite.offset.1),
            None,
        );
        canvas.restore();
        true
    }
}
pub(super) fn circle(
    canvas: &Canvas,
    point: (f32, f32),
    radius: f32,
    fill: Color,
    style: &StyleConfig,
) {
    canvas.draw_circle(
        (
            point.0 + style.geometry.shadow_offset,
            point.1 + style.geometry.shadow_offset,
        ),
        radius,
        &paint(style.colors.border.skia(), 1.),
    );
    canvas.draw_circle(point, radius, &paint(fill, 1.));
    let mut border: Paint = paint(style.colors.border.skia(), 1.);
    border.set_style(skia_safe::paint::Style::Stroke);
    border.set_stroke_width(style.geometry.border_width);
    canvas.draw_circle(point, radius, &border);
}
