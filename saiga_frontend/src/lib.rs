mod display;

use pollster::FutureExt as _;

use std::{
    collections::HashMap,
    error::Error,
    io::{Read, Write},
};

use display::Display;
use log::{debug, error, warn};
use saiga_backend::{event::Event as TerminalEvent, grid::Dimensions, pty::Pty, Terminal};
use saiga_vte::ansi::processor::Processor;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowId},
};

pub fn run() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = Application::new(event_loop.create_proxy());
    event_loop.run_app(&mut app)?;

    Ok(())
}

type ScopedEvent = (WindowId, Event);

#[derive(Debug)]
enum Event {
    Terminal(TerminalEvent),
}

struct TerminalEventListener {
    window_id: WindowId,
    event_loop_proxy: EventLoopProxy<ScopedEvent>,
}

impl TerminalEventListener {
    fn new(window_id: WindowId, event_loop_proxy: EventLoopProxy<ScopedEvent>) -> Self {
        Self {
            window_id,
            event_loop_proxy,
        }
    }
}

impl saiga_backend::event::EventListener for TerminalEventListener {
    fn on_event(&self, event: TerminalEvent) {
        self.event_loop_proxy
            .send_event((self.window_id, Event::Terminal(event)))
            .expect("event loop closed");
    }
}

struct State<'a> {
    pty: Pty,
    display: Display<'a>,
    terminal: Terminal<TerminalEventListener>,
}

impl State<'_> {
    async fn new(window: Window, event_loop_proxy: EventLoopProxy<ScopedEvent>) -> Self {
        let window_id = window.id();
        let display = Display::new(window).await;
        let pty = Pty::try_new().unwrap();
        let terminal = Terminal::new(
            Dimensions::default(),
            TerminalEventListener::new(window_id, event_loop_proxy),
        );

        Self {
            pty,
            display,
            terminal,
        }
    }

    fn set_scale_factor(&mut self, scale_factor: f64) {
        self.display.set_scale_factor(scale_factor);
        self.display.window.request_redraw();
    }

    fn set_size(&mut self, size: PhysicalSize<u32>) {
        // TODO: compute this properly
        self.terminal.resize(Dimensions {
            lines: size.height as usize / 60,
            columns: size.width as usize / 30,
        });
        self.display.set_size(size.width, size.height);

        self.request_redraw();
    }

    /// Read PTY and process output.
    /// Return value indicate whether PTY contains new data or not
    fn advance(&mut self, processor: &mut Processor) -> bool {
        let mut read_buffer = [0; 65536];

        let res = self.pty.read(&mut read_buffer);

        match res {
            Ok(0) => false,
            Ok(size) => {
                processor.advance(&mut self.terminal, &read_buffer[..size]);

                true
            }
            Err(e) => {
                error!("error reading pty: {e:?}");

                false
            }
        }
    }

    fn draw(&mut self) {
        self.display.draw(&mut self.terminal);
    }

    fn request_redraw(&self) {
        self.display.window.request_redraw();
    }
}

struct Application<'a> {
    processor: Processor,
    states: HashMap<WindowId, State<'a>>,
    event_loop_proxy: EventLoopProxy<ScopedEvent>,
}

impl Application<'_> {
    pub fn new(event_loop_proxy: EventLoopProxy<ScopedEvent>) -> Self {
        Self {
            processor: Processor::new(),
            states: HashMap::new(),
            event_loop_proxy,
        }
    }
}

impl ApplicationHandler<ScopedEvent> for Application<'_> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes())
            .unwrap();

        let window_id = window.id();
        let state = State::new(window, self.event_loop_proxy.clone()).block_on();

        self.states.insert(window_id, state);
    }

    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, event: ScopedEvent) {
        let Some(state) = self.states.get_mut(&event.0) else {
            warn!(
                "received event for window {:?} which does not exist",
                event.0
            );
            return;
        };

        match &event.1 {
            Event::Terminal(event) => match event {
                TerminalEvent::SetTitle(title) => {
                    state.display.window.set_title(&title);
                }
                TerminalEvent::PtyWrite(payload) => {
                    state.pty.write(&payload).unwrap();
                }
            },
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let Some(state) = self.states.get_mut(&window_id) else {
            return;
        };

        if state.advance(&mut self.processor) {
            state.request_redraw();
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                state.set_scale_factor(scale_factor)
            }
            WindowEvent::Resized(size) => state.set_size(size),
            WindowEvent::RedrawRequested => state.draw(),
            _ => (),
        }
    }
}
