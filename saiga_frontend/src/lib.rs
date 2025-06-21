pub mod backend;
pub mod color;
pub mod display;
pub mod font;
pub mod settings;
pub mod size;
pub mod term_font;
pub mod terminal;
pub mod theme;

use std::{
    borrow::Cow,
    error::Error,
    ops::Deref,
    sync::{Arc, LazyLock, Mutex},
};

use display::Display;
use font::{Family, Font};
use pollster::FutureExt;
use saiga_backend::event::Event;
use saiga_input::Mods;
use settings::{BackendSettings, FontSettings, Settings};
use size::Size;
use terminal::Terminal;
use tokio::sync::mpsc;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{EventLoop, EventLoopProxy},
    keyboard::KeyCode,
    window::Window,
};

static CLIPBOARD: LazyLock<Option<Mutex<arboard::Clipboard>>> =
    LazyLock::new(|| arboard::Clipboard::new().ok().map(Mutex::new));

fn clipboard_get_text() -> Option<String> {
    let Some(clipboard) = CLIPBOARD.deref() else {
        return None;
    };

    let Ok(mut clipboard) = clipboard.lock() else {
        return None;
    };

    clipboard.get_text().ok()
}

fn clipboard_set_text<'a, T>(text: T)
where
    T: Into<Cow<'a, str>>,
{
    let Some(clipboard) = CLIPBOARD.deref() else {
        return;
    };

    let Ok(mut clipboard) = clipboard.lock() else {
        return;
    };

    let _ = clipboard.set_text(text);
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let settings = Settings {
        font: FontSettings {
            size: 16.0,
            font_type: Font {
                // family: Family::Name("JetBrains Mono"),
                family: Family::Name("JetBrainsMono Nerd Font Mono"),
                // family: Family::Monospace,
                ..Default::default()
            },
            ..Default::default()
        },
        backend: BackendSettings {
            shell: "fish".to_string(),
        },
        ..Default::default()
    };

    let runtime = tokio::runtime::Runtime::new()?;

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
    mods: saiga_input::Mods,
}

impl State<'_> {
    // pub async fn new(settings: Settings, sender: mpsc::Sender<Event>, window: Arc<Window>) -> Self {
    //     let mut display = Display::new(window).await;
    //
    //     let mut terminal = Terminal::new(1, &mut display.context.font_system, settings);
    //     terminal.init_backend(sender);
    //
    //     Self {
    //         terminal,
    //         display,
    //         mods: Mods::empty(),
    //     }
    // }

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

        self.terminal
            .resize(Some(size), Some(self.terminal.font.measure));
    }

    pub fn handle_key_key(&mut self, event: KeyEvent) {
        let KeyEvent {
            state,
            physical_key,
            text,
            repeat,
            ..
        } = event;

        // TODO: better structure this function. it's a mess right now

        let mods = match physical_key {
            winit::keyboard::PhysicalKey::Code(code) => match code {
                KeyCode::AltLeft => Some(Mods::LEFT_ALT),
                KeyCode::AltRight => Some(Mods::RIGHT_ALT),
                KeyCode::ControlLeft => Some(Mods::LEFT_CTRL),
                KeyCode::ControlRight => Some(Mods::RIGHT_CTRL),
                KeyCode::SuperLeft => Some(Mods::LEFT_SUPER),
                KeyCode::SuperRight => Some(Mods::RIGHT_SUPER),
                KeyCode::ShiftLeft => Some(Mods::LEFT_SHIFT),
                KeyCode::ShiftRight => Some(Mods::RIGHT_SHIFT),
                _ => None,
            },
            _ => None,
        };

        if let Some(mods) = mods {
            match state {
                ElementState::Pressed => self.mods.insert(mods),
                ElementState::Released => self.mods.remove(mods),
            }

            return;
        }

        let key = saiga_input::Key::from(physical_key);

        if state == ElementState::Pressed && self.mods == Mods::LEFT_SUPER {
            match key {
                saiga_input::Key::Minus => {
                    self.terminal.set_font(
                        &mut self.display.context.font_system,
                        FontSettings {
                            size: self.terminal.font.settings.size - 1.0,
                            ..self.terminal.font.settings
                        },
                    );
                    self.sync_size();
                    return;
                }
                saiga_input::Key::Equal => {
                    self.terminal.set_font(
                        &mut self.display.context.font_system,
                        FontSettings {
                            size: self.terminal.font.settings.size + 1.0,
                            ..self.terminal.font.settings
                        },
                    );
                    self.sync_size();
                    return;
                }
                saiga_input::Key::V => {
                    if let Some(text) = clipboard_get_text() {
                        // TODO: support bracketed paste
                        self.terminal.write(text.into_bytes());
                    }

                    return;
                }
                _ => {}
            }
        }

        if state == ElementState::Released {
            return;
        }

        // TODO: fill it properly
        let encoder = saiga_input::Encoder {
            event: saiga_input::KeyEvent {
                action: if repeat {
                    saiga_input::Action::Repeat
                } else {
                    match state {
                        ElementState::Pressed => saiga_input::Action::Press,
                        ElementState::Released => saiga_input::Action::Release,
                    }
                },
                key,
                physical_key: key,
                mods: self.mods,
                consumed_mods: Mods::LEFT_SHIFT.union(Mods::RIGHT_SHIFT),
                composing: false,
                utf8: text.as_ref().map(|s| s.as_str()).unwrap_or_default(),
                unshifted_char: '\0',
            },
            modify_other_keys_state_2: false,
        };

        if let Some(seq) = encoder.encode() {
            self.terminal.write(seq);
        } else if let Some(utf8) = text {
            self.terminal.write(utf8.to_string().into_bytes());
        }
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

    async fn init_state(
        &mut self,
        settings: Settings,
        sender: mpsc::Sender<Event>,
        window: Arc<Window>,
    ) {
        let mut display = Display::new(window).await;

        let mut terminal = Terminal::new(1, &mut display.context.font_system, settings);
        terminal.init_backend(sender);

        let state = State {
            terminal,
            display,
            mods: Mods::empty(),
        };

        self.state = Some(state);
    }
}

impl ApplicationHandler<Event> for App<'_> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let attrs = Window::default_attributes().with_title("Saiga");

        let window = event_loop
            .create_window(attrs)
            .expect("window must be created");

        let (sender, mut receiver) = mpsc::channel(100);

        self.init_state(self.settings.clone(), sender, Arc::new(window))
            .block_on();

        let proxy = self.event_loop_proxy.clone();

        tokio::spawn(async move {
            while let Some(event) = receiver.recv().await {
                proxy.send_event(event).unwrap();
            }
        });
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
            Event::ClipboardStore(_clipboard_type, data) => {
                // TODO: handle clipboard type
                // TODO: support bracketed paste

                clipboard_set_text(data);
            }
            Event::ClipboardLoad(_clipboard_type, fmt) => {
                let Some(text) = clipboard_get_text() else {
                    return;
                };

                // TODO: handle clipboard type
                // TODO: support bracketed paste

                state.terminal.write(fmt(&text).into_bytes());
            }
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
