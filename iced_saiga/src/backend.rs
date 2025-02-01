use iced::{keyboard::Modifiers, Size};
use saiga_backend::{
    event::{Event, EventListener, Notify as _, OnResize as _, WindowSize},
    event_loop::{EventLoop, Notifier},
    grid::{Dimensions, Grid, Scroll},
    index::{Column, Line, Point},
    selection::{SelectionRange, SelectionType},
    sync::FairMutex,
    term::{self, cell::Cell, Term, TermMode},
    tty,
};
use saiga_vte::ansi::handler::CursorStyle;
use std::{borrow::Cow, io, sync::Arc};
use tokio::sync::mpsc;

use crate::{actions::Action, settings::BackendSettings};

#[derive(Debug, Clone)]
pub enum BackendCommand {
    Write(Vec<u8>),
    Scroll(i32),
    Resize(Option<Size<f32>>, Option<Size<f32>>),
    SelectStart(SelectionType, (f32, f32)),
    SelectUpdate((f32, f32)),
    MouseReport(MouseButton, Modifiers, Point, bool),
    ProcessTermEvent(Event),
}

#[derive(Debug, Clone)]
pub enum MouseMode {
    Sgr,
    Normal(bool),
}

impl From<TermMode> for MouseMode {
    fn from(term_mode: TermMode) -> Self {
        if term_mode.contains(TermMode::SGR_MOUSE) {
            MouseMode::Sgr
        } else if term_mode.contains(TermMode::UTF8_MOUSE) {
            MouseMode::Normal(true)
        } else {
            MouseMode::Normal(false)
        }
    }
}

#[derive(Debug, Clone)]
pub enum MouseButton {
    LeftButton = 0,
    MiddleButton = 1,
    RightButton = 2,
    LeftMove = 32,
    MiddleMove = 33,
    RightMove = 34,
    NoneMove = 35,
    ScrollUp = 64,
    ScrollDown = 65,
    Other = 99,
}

pub struct Backend {
    term: Arc<FairMutex<Term<EventProxy>>>,
    size: TermSize,
    notifier: Notifier,
    last_content: RenderableContent,
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

        let mut term = Term::new(config, &term_size, event_proxy.clone());
        let cursor = term.grid_mut().cursor_cell().clone();

        let initial_content = RenderableContent {
            grid: term.grid().clone(),
            selectable_range: None,
            cursor: cursor.clone(),
            term_mode: *term.mode(),
            cursor_style: term.cursor_style(),
            term_size,
        };

        let term = Arc::new(FairMutex::new(term));
        let pty_event_loop = EventLoop::new(term.clone(), event_proxy, pty, false, false)?;
        let notifier = Notifier(pty_event_loop.channel());

        // TODO: use it
        let _pty_join_handle = pty_event_loop.spawn();

        Ok(Self {
            term,
            size: term_size,
            notifier,
            last_content: initial_content,
        })
    }

    pub fn process_command(&mut self, cmd: BackendCommand) -> Action {
        let term = self.term.clone();
        let mut term = term.lock();

        match cmd {
            BackendCommand::ProcessTermEvent(event) => match event {
                Event::Wakeup => {
                    self.internal_sync(&mut term);

                    Action::Redraw
                }
                Event::Exit => Action::Shutdown,
                Event::Title(title) => Action::ChangeTitle(title),
                _ => Action::Ignore,
            },
            BackendCommand::Write(input) => {
                self.write(input);
                term.scroll_display(Scroll::Bottom);

                Action::Ignore
            }
            BackendCommand::Resize(layout_size, font_measure) => {
                self.resize(&mut term, layout_size, font_measure);
                self.internal_sync(&mut term);

                Action::Redraw
            }
            _ => Action::Ignore, // BackendCommand::Scroll(delta) => {
                                 //     self.scroll(&mut term, delta);
                                 //     self.internal_sync(&mut term);
                                 //     action = Action::Redraw;
                                 // }
                                 // BackendCommand::SelectStart(selection_type, (x, y)) => {
                                 //     self.start_selection(&mut term, selection_type, x, y);
                                 //     self.internal_sync(&mut term);
                                 //     action = Action::Redraw;
                                 // }
                                 // BackendCommand::SelectUpdate((x, y)) => {
                                 //     self.update_selection(&mut term, x, y);
                                 //     self.internal_sync(&mut term);
                                 //     action = Action::Redraw;
                                 // }
                                 // BackendCommand::ProcessLink(link_action, point) => {
                                 //     action = self.process_link_action(&term, link_action, point);
                                 // }
                                 // BackendCommand::MouseReport(button, modifiers, point, pressed) => {
                                 //     self.process_mouse_report(button, modifiers, point, pressed);
                                 //     action = Action::Redraw;
                                 // }
        }
    }

    fn resize(
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

    fn write<I: Into<Cow<'static, [u8]>>>(&self, input: I) {
        self.notifier.notify(input);
    }

    pub fn sync(&mut self) {
        let term = self.term.clone();
        let mut term = term.lock();
        self.internal_sync(&mut term);
    }

    fn internal_sync(&mut self, terminal: &mut Term<EventProxy>) {
        let selectable_range = match &terminal.selection {
            Some(s) => s.to_range(terminal),
            None => None,
        };

        let cursor = terminal.grid_mut().cursor_cell().clone();

        self.last_content.grid = terminal.grid().clone();
        self.last_content.selectable_range = selectable_range;
        self.last_content.cursor = cursor.clone();
        self.last_content.term_mode = *terminal.mode();
        self.last_content.term_size = self.size;
        self.last_content.cursor_style = terminal.cursor_style();
    }

    pub fn renderable_content(&self) -> &RenderableContent {
        &self.last_content
    }
}

pub struct RenderableContent {
    pub grid: Grid<Cell>,
    pub selectable_range: Option<SelectionRange>,
    pub cursor: Cell,
    pub term_mode: TermMode,
    pub term_size: TermSize,
    pub cursor_style: CursorStyle,
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
