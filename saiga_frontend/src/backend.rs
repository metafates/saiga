use std::{borrow::Cow, io, sync::Arc};

use saiga_backend::{
    event::{Event, EventListener, Notify as _, OnResize as _, WindowSize},
    event_loop::{EventLoop, Notifier},
    grid::{Dimensions, Grid},
    index::{Column, Line},
    sync::FairMutex,
    term::{self, Term, cell::Cell},
    tty,
};
use tokio::sync::mpsc;

use crate::{settings::BackendSettings, size::Size};

pub struct Backend {
    term: Arc<FairMutex<Term<EventProxy>>>,
    size: TermSize,
    notifier: Notifier,
    prev_grid: Grid<Cell>,
}

impl Backend {
    pub fn new(
        id: u64,
        event_sender: mpsc::Sender<Event>,
        settings: BackendSettings,
        font_size: Size<f32>,
    ) -> io::Result<Self> {
        let pty_config = tty::Options {
            shell: Some(tty::Shell::new(settings.shell, vec![])),
            ..Default::default()
        };

        let config = term::Config::default();

        let term_size = TermSize {
            cell_width: font_size.width as u16,
            cell_height: font_size.height as u16,
            ..Default::default()
        };

        let pty = tty::new(&pty_config, term_size.into(), id)?;
        let event_proxy = EventProxy(event_sender);

        let term = Term::new(config, &term_size, event_proxy.clone());

        let prev_grid = term.grid().clone();

        let term = Arc::new(FairMutex::new(term));
        let pty_event_loop = EventLoop::new(term.clone(), event_proxy, pty, false)?;
        let notifier = Notifier(pty_event_loop.channel());

        // TODO: use it?
        let _pty_join_handle = pty_event_loop.spawn();

        Ok(Self {
            term,
            prev_grid,
            size: term_size,
            notifier,
        })
    }

    pub fn prev_grid(&self) -> &Grid<Cell> {
        &self.prev_grid
    }

    pub fn size(&self) -> &TermSize {
        &self.size
    }

    pub fn sync(&mut self) {
        self.prev_grid = self.term.lock().grid().clone();
    }

    pub fn resize(
        &mut self,
        terminal: &mut Term<EventProxy>,
        layout_size: Option<Size<f32>>,
        font_measure: Option<Size<f32>>,
    ) {
        if let Some(size) = layout_size {
            self.size.layout_height = size.height;
            self.size.layout_width = size.width;
        };

        if let Some(size) = font_measure {
            self.size.cell_height = size.height as u16;
            self.size.cell_width = size.width as u16;
        }

        let lines = (self.size.layout_height / self.size.cell_height as f32).floor() as u16;
        let cols = (self.size.layout_width / self.size.cell_width as f32).floor() as u16;
        if lines > 0 && cols > 0 {
            self.size.num_lines = lines;
            self.size.num_cols = cols;
            self.notifier.on_resize(self.size.into());

            terminal.resize(self.size);
        }
    }

    pub fn write<I: Into<Cow<'static, [u8]>>>(&self, input: I) {
        self.notifier.notify(input);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TermSize {
    pub cell_width: u16,
    pub cell_height: u16,

    num_cols: u16,
    num_lines: u16,
    layout_width: f32,
    layout_height: f32,
}

impl Default for TermSize {
    fn default() -> Self {
        Self {
            cell_width: 1,
            cell_height: 1,
            num_cols: 80,
            num_lines: 50,
            layout_width: 80.0,
            layout_height: 50.0,
        }
    }
}

impl Dimensions for TermSize {
    fn total_lines(&self) -> usize {
        self.screen_lines()
    }

    fn columns(&self) -> usize {
        self.num_cols as usize
    }

    fn last_column(&self) -> Column {
        Column(self.num_cols as usize - 1)
    }

    fn bottommost_line(&self) -> Line {
        Line(self.num_lines as i32 - 1)
    }

    fn screen_lines(&self) -> usize {
        self.num_lines as usize
    }
}

impl From<TermSize> for WindowSize {
    fn from(size: TermSize) -> Self {
        Self {
            num_lines: size.num_lines,
            num_cols: size.num_cols,
            cell_width: size.cell_width,
            cell_height: size.cell_height,
        }
    }
}

#[derive(Clone)]
pub struct EventProxy(mpsc::Sender<Event>);

impl EventListener for EventProxy {
    fn send_event(&self, event: Event) {
        let _ = self.0.blocking_send(event);
    }
}
