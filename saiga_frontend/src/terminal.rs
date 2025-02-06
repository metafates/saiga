use saiga_backend::event::Event;
use tokio::sync::mpsc;

use crate::{
    backend::Backend,
    settings::{BackendSettings, Settings},
    size::Size,
    term_font::TermFont,
    theme::Theme,
};

pub struct Terminal {
    pub id: u64,
    pub font: TermFont,
    pub theme: Theme,
    pub backend: Option<Backend>,
    backend_settings: BackendSettings,
}

impl Terminal {
    pub fn new(id: u64, font_system: &mut glyphon::FontSystem, settings: Settings) -> Self {
        Self {
            id,
            font: TermFont::new(font_system, settings.font),
            theme: Theme::new(settings.theme),
            backend: None,
            backend_settings: settings.backend,
        }
    }

    pub fn init_backend(&mut self, event_sender: mpsc::Sender<Event>) {
        let backend = Backend::new(
            self.id,
            event_sender,
            self.backend_settings.clone(),
            self.font.measure,
        )
        .unwrap();

        self.backend = Some(backend);
    }

    pub fn resize(&mut self, surface_size: Option<Size<f32>>, font_measure: Option<Size<f32>>) {
        let Some(ref mut backend) = self.backend else {
            return;
        };

        backend.resize(surface_size, font_measure);
    }
}
