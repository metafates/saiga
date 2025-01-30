use iced::Font;

use crate::theme::ColorPalette;

#[cfg(target_os = "windows")]
const DEFAULT_SHELL: &str = "wsl.exe";

#[cfg(not(target_os = "windows"))]
const DEFAULT_SHELL: &str = "/bin/bash";

#[derive(Default)]
pub struct Settings {
    pub font: FontSettings,
    pub backend: BackendSettings,
    pub theme: ThemeSettings,
}

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

pub struct FontSettings {
    pub size: f32,
    pub scale_factor: f32,
    pub font_type: iced::Font,
}

impl Default for FontSettings {
    fn default() -> Self {
        Self {
            size: 15.0,
            scale_factor: 1.3,
            font_type: Font::MONOSPACE,
        }
    }
}

#[derive(Default)]
pub struct ThemeSettings {
    pub color_palette: ColorPalette,
}
