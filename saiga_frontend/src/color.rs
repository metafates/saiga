/// A color in the `sRGB` color space.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Color {
    /// Red component, 0.0 - 1.0
    pub r: f32,
    /// Green component, 0.0 - 1.0
    pub g: f32,
    /// Blue component, 0.0 - 1.0
    pub b: f32,
    /// Transparency, 0.0 - 1.0
    pub a: f32,
}

impl Color {
    /// Creates a new [`Color`].
    ///
    /// In debug mode, it will panic if the values are not in the correct
    /// range: 0.0 - 1.0
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Color {
        debug_assert!((0.0..=1.0).contains(&r), "Red component must be on [0, 1]");
        debug_assert!(
            (0.0..=1.0).contains(&g),
            "Green component must be on [0, 1]"
        );
        debug_assert!((0.0..=1.0).contains(&b), "Blue component must be on [0, 1]");
        debug_assert!(
            (0.0..=1.0).contains(&a),
            "Alpha component must be on [0, 1]"
        );

        Color { r, g, b, a }
    }

    /// Creates a [`Color`] from its RGB8 components.
    pub fn from_rgb8(r: u8, g: u8, b: u8) -> Color {
        Color::from_rgba8(r, g, b, 1.0)
    }

    /// Creates a [`Color`] from its RGB8 components and an alpha value.
    pub fn from_rgba8(r: u8, g: u8, b: u8, a: f32) -> Color {
        Color {
            r: f32::from(r) / 255.0,
            g: f32::from(g) / 255.0,
            b: f32::from(b) / 255.0,
            a,
        }
    }

    pub fn as_linear(&self) -> [f32; 4] {
        [
            srgb_channel_to_linear(self.r),
            srgb_channel_to_linear(self.g),
            srgb_channel_to_linear(self.b),
            self.a,
        ]
    }
}

impl From<Color> for wgpu::Color {
    fn from(c: Color) -> Self {
        Self {
            r: c.r as f64,
            g: c.g as f64,
            b: c.b as f64,
            a: c.a as f64,
        }
    }
}

impl From<wgpu::Color> for Color {
    fn from(c: wgpu::Color) -> Self {
        Self::new(c.r as f32, c.g as f32, c.b as f32, c.a as f32)
    }
}

impl From<Color> for glyphon::Color {
    fn from(c: Color) -> Self {
        let r = c.r * u8::MAX as f32;
        let g = c.g * u8::MAX as f32;
        let b = c.b * u8::MAX as f32;
        let a = c.a * u8::MAX as f32;

        Self::rgba(r as u8, g as u8, b as u8, a as u8)
    }
}

fn srgb_channel_to_linear(s: f32) -> f32 {
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}
