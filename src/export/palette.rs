use crate::style::Colors;

/// A stable palette for the selected theme, built once per export.
/// Global indexed colors avoid running a general neural quantizer on every frame.
pub struct Palette {
    pub colors: Vec<u8>,
    lookup: Vec<u8>,
    exact: Vec<([u8; 3], u8)>,
}
pub fn theme_palette(theme: &Colors) -> Palette {
    let ramps = [
        (theme.border.0, theme.surface.0),
        (theme.text_primary.0, theme.surface.0),
        (theme.text_secondary.0, theme.surface.0),
        (theme.background.0, theme.surface.0),
        (theme.background.0, theme.accent.0),
        (theme.border.0, theme.accent.0),
        (theme.accent.0, theme.surface.0),
        (theme.text_primary.0, theme.background.0),
    ];
    let colors: Vec<u8> = (0..256)
        .flat_map(|i| {
            let t = (i % 32) as f32 / 31.;
            let (from, to) = ramps[i / 32];
            std::array::from_fn::<_, 3, _>(|j| {
                (f32::from(from[j]) + (f32::from(to[j]) - f32::from(from[j])) * t).round() as u8
            })
        })
        .collect();
    let exact = [
        theme.surface.0,
        theme.background.0,
        theme.accent.0,
        theme.text_primary.0,
        theme.border.0,
        theme.text_secondary.0,
    ]
    .into_iter()
    .map(|rgb| {
        let index = colors
            .as_chunks::<3>()
            .0
            .iter()
            .position(|c| *c == rgb)
            .expect("theme endpoints are in palette");
        (rgb, index as u8)
    })
    .collect();
    let lookup = (0..32768)
        .map(|key| {
            let sample = [
                ((key >> 10) & 31) * 8 + 4,
                ((key >> 5) & 31) * 8 + 4,
                (key & 31) * 8 + 4,
            ];
            colors
                .as_chunks::<3>()
                .0
                .iter()
                .enumerate()
                .min_by_key(|(_, color)| {
                    let d: [i32; 3] = std::array::from_fn(|j| sample[j] - i32::from(color[j]));
                    2 * d[0] * d[0] + 4 * d[1] * d[1] + d[2] * d[2]
                })
                .expect("palette is populated")
                .0 as u8
        })
        .collect();
    Palette {
        colors,
        lookup,
        exact,
    }
}
impl Palette {
    pub fn index(&self, rgba: &[u8]) -> Vec<u8> {
        rgba.as_chunks::<4>()
            .0
            .iter()
            .map(|pixel| {
                // Preserve the configured flat colors exactly before antialias quantization.
                if let Some((_, index)) = self.exact.iter().find(|(rgb, _)| pixel[..3] == *rgb) {
                    *index
                } else {
                    self.lookup[((pixel[0] as usize >> 3) << 10)
                        | ((pixel[1] as usize >> 3) << 5)
                        | (pixel[2] as usize >> 3)]
                }
            })
            .collect()
    }
}
