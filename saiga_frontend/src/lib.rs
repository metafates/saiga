pub mod backend;
pub mod color;
pub mod display;
pub mod font;
pub mod settings;
pub mod size;
pub mod term_font;
pub mod terminal;
pub mod theme;

use std::{borrow::Cow, error::Error, sync::Arc};

use backend::Backend;
use color::Color;
use display::Display;
use font::{Family, Font};
use pollster::FutureExt;
use saiga_backend::{event::Event, grid::GridCell};
use saiga_vte::ansi::handler::Color as AnsiColor;
use settings::{BackendSettings, FontSettings, Settings};
use size::Size;
use terminal::Terminal;
use tokio::{runtime, sync::mpsc};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{EventLoop, EventLoopProxy},
    keyboard::KeyCode,
    window::Window,
};

pub fn run() -> Result<(), Box<dyn Error>> {
    let runtime = tokio::runtime::Runtime::new()?;

    let settings = Settings {
        font: FontSettings {
            size: 16.0,
            font_type: Font {
                family: Family::Name("JetBrainsMono Nerd Font Mono"),
                ..Default::default()
            },
            ..Default::default()
        },
        backend: BackendSettings {
            shell: "fish".to_string(),
        },
        ..Default::default()
    };

    runtime.block_on(async {
        let event_loop = EventLoop::with_user_event().build()?;
        let mut app = App::new(settings, event_loop.create_proxy());

        event_loop.run_app(&mut app)
    })?;

    Ok(())
}

struct State<'a> {
    terminal: Terminal,
    display: Display<'a>,
}

impl State<'_> {
    pub async fn new(settings: Settings, sender: mpsc::Sender<Event>, window: Arc<Window>) -> Self {
        let mut display = Display::new(window).await;

        let mut terminal = Terminal::new(1, &mut display.context.font_system, settings);
        terminal.init_backend(sender);

        Self { terminal, display }
    }

    pub fn render(&mut self) {
        self.display.render(&mut self.terminal);
    }

    pub fn sync_size(&mut self) {
        self.display.sync_size();

        let size = self.display.window().inner_size();

        let size = size.to_logical(self.display.window().scale_factor());

        let size = Size::new(size.width, size.height);

        if let Some(ref backend) = self.terminal.backend {
            let term_size = backend.size();

            let resize_increment = LogicalSize::new(term_size.cell_width, term_size.cell_height);
            self.display
                .window()
                .set_resize_increments(Some(resize_increment));
        }

        self.terminal.resize(Some(size), None);
    }

    pub fn handle_key_key(&self, event: KeyEvent) {
        let KeyEvent {
            state: ElementState::Pressed,
            physical_key,
            text: Some(text),
            ..
        } = event
        else {
            return;
        };

        // TODO: use saiga_input for it
        let sequence = match physical_key {
            winit::keyboard::PhysicalKey::Code(key_code) => match key_code {
                KeyCode::Backspace => "\x7f".to_string(),
                KeyCode::Enter => "\x0d".to_string(),
                KeyCode::Escape => "\x1b".to_string(),
                _ => text.chars().as_str().to_string(),
            },
            _ => text.chars().as_str().to_string(),
        };

        self.terminal.write(sequence.into_bytes());
    }

    pub fn request_redraw(&self) {
        self.display.window().request_redraw();
    }
}

struct App<'a> {
    settings: Settings,
    state: Option<State<'a>>,
    event_loop_proxy: EventLoopProxy<Event>,
}

impl App<'_> {
    pub fn new(settings: Settings, proxy: EventLoopProxy<Event>) -> Self {
        Self {
            settings,
            state: None,
            event_loop_proxy: proxy,
        }
    }
}

impl ApplicationHandler<Event> for App<'_> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let attrs = Window::default_attributes().with_title("Saiga");

        let window = event_loop
            .create_window(attrs)
            .expect("window must be created");

        let (sender, mut receiver) = mpsc::channel(100);
        let state = State::new(self.settings.clone(), sender, Arc::new(window)).block_on();

        let proxy = self.event_loop_proxy.clone();

        tokio::spawn(async move {
            while let Some(event) = receiver.recv().await {
                proxy.send_event(event).unwrap();
            }
        });

        self.state = Some(state)
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: Event) {
        let Some(ref mut state) = self.state else {
            return;
        };

        match event {
            Event::Wakeup => {
                state.request_redraw();
            }
            Event::Title(title) => {
                state.display.window().set_title(&title);
            }
            Event::PtyWrite(payload) => state.terminal.write(payload.into_bytes()),
            Event::Exit => event_loop.exit(),
            // TODO: check if it's slow? since we are acquire mutex for it in backend.color()
            //
            // Event::ColorRequest(index, fmt) => {
            //     let Some(ref backend) = state.terminal.backend else {
            //         return;
            //     };
            //
            //     let color = backend.color(index).unwrap_or_else(|| {
            //         let color = state
            //             .terminal
            //             .theme
            //             .get_color(AnsiColor::Indexed(index as u8));
            //
            //         color.into()
            //     });
            //
            //     let sequence = fmt(color);
            //
            //     state.terminal.write(sequence.into_bytes());
            // }
            // _ => println!("{event:?}"),
            _ => {}
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let Some(ref mut state) = self.state else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => state.sync_size(),
            WindowEvent::RedrawRequested => {
                state.render();
            }
            WindowEvent::KeyboardInput { event, .. } => state.handle_key_key(event),
            _ => {}
        }
    }
}
