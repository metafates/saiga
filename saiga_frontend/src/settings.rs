use crate::{font::Font, theme::ColorPalette};

#[cfg(target_os = "windows")]
const DEFAULT_SHELL: &str = "wsl.exe";

#[cfg(not(target_os = "windows"))]
const DEFAULT_SHELL: &str = "/bin/bash";

#[derive(Default, Clone)]
pub struct Settings {
    pub font: FontSettings,
    pub backend: BackendSettings,
    pub theme: ThemeSettings,
}

#[derive(Clone)]
pub struct BackendSettings {
    pub shell: String,
}

impl Default for BackendSettings {
    fn default() -> Self {
        Self {
            shell: DEFAULT_SHELL.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FontSettings {
    pub size: f32,
    pub line_scale_factor: f32,
    pub font_type: Font,
}

impl Default for FontSettings {
    fn default() -> Self {
        Self {
            size: 15.0,
            line_scale_factor: 1.3,
            font_type: Font::MONOSPACE,
        }
    }
}

#[derive(Default, Clone)]
pub struct ThemeSettings {
    pub color_palette: ColorPalette,
}

impl ThemeSettings {
    pub fn new(color_palette: ColorPalette) -> Self {
        Self { color_palette }
    }
}
