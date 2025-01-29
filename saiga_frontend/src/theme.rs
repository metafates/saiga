use std::collections::HashMap;

use iced::Color;

pub struct Theme {
    palette: ColorPalette,
    ansi256_colors: HashMap<u8, Color>,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            palette: Default::default(),
            ansi256_colors: build_ansi256_colors(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ColorPalette {
    pub foreground: Color,
    pub background: Color,
    pub black: Color,
    pub red: Color,
    pub green: Color,
    pub yellow: Color,
    pub blue: Color,
    pub magenta: Color,
    pub cyan: Color,
    pub white: Color,
    pub bright_black: Color,
    pub bright_red: Color,
    pub bright_green: Color,
    pub bright_yellow: Color,
    pub bright_blue: Color,
    pub bright_magenta: Color,
    pub bright_cyan: Color,
    pub bright_white: Color,
    pub bright_foreground: Option<Color>,
    pub dim_foreground: Color,
    pub dim_black: Color,
    pub dim_red: Color,
    pub dim_green: Color,
    pub dim_yellow: Color,
    pub dim_blue: Color,
    pub dim_magenta: Color,
    pub dim_cyan: Color,
    pub dim_white: Color,
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self {
            foreground: Color::from_rgb8(216, 216, 216),
            background: Color::from_rgb8(24, 24, 24),
            black: Color::from_rgb8(24, 24, 24),
            red: Color::from_rgb8(172, 66, 66),
            green: Color::from_rgb8(144, 169, 89),
            yellow: Color::from_rgb8(244, 191, 117),
            blue: Color::from_rgb8(106, 159, 181),
            magenta: Color::from_rgb8(170, 117, 159),
            cyan: Color::from_rgb8(117, 181, 170),
            white: Color::from_rgb8(216, 216, 216),
            bright_black: Color::from_rgb8(107, 107, 107),
            bright_red: Color::from_rgb8(197, 85, 85),
            bright_green: Color::from_rgb8(170, 196, 116),
            bright_yellow: Color::from_rgb8(254, 202, 136),
            bright_blue: Color::from_rgb8(130, 184, 200),
            bright_magenta: Color::from_rgb8(194, 140, 184),
            bright_cyan: Color::from_rgb8(147, 211, 195),
            bright_white: Color::from_rgb8(248, 248, 248),
            bright_foreground: None,
            dim_foreground: Color::from_rgb8(130, 132, 130),
            dim_black: Color::from_rgb8(15, 15, 15),
            dim_red: Color::from_rgb8(113, 43, 43),
            dim_green: Color::from_rgb8(95, 111, 58),
            dim_yellow: Color::from_rgb8(161, 126, 77),
            dim_blue: Color::from_rgb8(69, 104, 119),
            dim_magenta: Color::from_rgb8(112, 77, 104),
            dim_cyan: Color::from_rgb8(77, 119, 112),
            dim_white: Color::from_rgb8(142, 142, 142),
        }
    }
}

fn build_ansi256_colors() -> HashMap<u8, Color> {
    let mut colors = HashMap::new();

    for r in 0..6 {
        for g in 0..6 {
            for b in 0..6 {
                // Reserve the first 16 colors for config.
                let index = 16 + r * 36 + g * 6 + b;
                let color = Color::from_rgb8(
                    if r == 0 { 0 } else { r * 40 + 55 },
                    if g == 0 { 0 } else { g * 40 + 55 },
                    if b == 0 { 0 } else { b * 40 + 55 },
                );

                colors.insert(index, color);
            }
        }
    }

    const INDEX: u8 = 232;
    for i in 0..24 {
        let value = i * 10 + 8;
        colors.insert(INDEX + i, Color::from_rgb8(value, value, value));
    }

    colors
}
