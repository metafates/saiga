mod display;

use pollster::FutureExt as _;

use std::{
    collections::HashMap,
    error::Error,
    io::{Read, Write},
    sync::Arc,
};

use display::Display;
use log::debug;
use saiga_backend::{event::Event as TerminalEvent, grid::Dimensions, pty::Pty, Terminal};
use saiga_vte::ansi::processor::Processor;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowAttributes, WindowId},
};

pub fn run() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = Application::new(event_loop.create_proxy());
    event_loop.run_app(&mut app)?;

    Ok(())
}

#[derive(Debug)]
enum Event {
    Terminal(TerminalEvent),
}

struct TerminalEventListener {
    event_loop_proxy: EventLoopProxy<Event>,
}

impl TerminalEventListener {
    fn new(event_loop_proxy: EventLoopProxy<Event>) -> Self {
        Self { event_loop_proxy }
    }
}

impl saiga_backend::event::EventListener for TerminalEventListener {
    fn on_event(&self, event: TerminalEvent) {
        self.event_loop_proxy
            .send_event(Event::Terminal(event))
            .expect("event loop closed");
    }
}

struct State<'a> {
    pty: Pty,
    display: Display<'a>,
    terminal: Terminal<TerminalEventListener>,
}

impl State<'_> {
    async fn new(window: Window, event_loop_proxy: EventLoopProxy<Event>) -> Self {
        let display = Display::new(window).await;
        let pty = Pty::try_new().unwrap();
        let terminal = Terminal::new(
            Dimensions::default(),
            TerminalEventListener::new(event_loop_proxy),
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
            rows: size.height as usize / 60,
            columns: size.width as usize / 30,
        });
        self.display.set_size(size.width, size.height);

        self.display.window.request_redraw();
    }

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
                debug!("error reading: {e:?}");

                false
            }
        }
    }

    fn draw(&mut self) {
        self.display.draw(&mut self.terminal);
    }
}

struct Application<'a> {
    processor: Processor,
    states: HashMap<WindowId, State<'a>>,
    event_loop_proxy: EventLoopProxy<Event>,
}

impl Application<'_> {
    pub fn new(event_loop_proxy: EventLoopProxy<Event>) -> Self {
        Self {
            processor: Processor::new(),
            states: HashMap::new(),
            event_loop_proxy,
        }
    }
}

impl ApplicationHandler<Event> for Application<'_> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes())
            .unwrap();

        let window_id = window.id();
        let state = State::new(window, self.event_loop_proxy.clone()).block_on();

        self.states.insert(window_id, state);
    }

    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, event: Event) {
        // TODO: get window id somehow

        for state in self.states.values_mut() {
            match &event {
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

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                state.set_scale_factor(scale_factor)
            }
            WindowEvent::Resized(size) => state.set_size(size),
            WindowEvent::RedrawRequested => {
                if state.advance(&mut self.processor) {
                    state.draw()
                }
            }
            _ => (),
        }
    }
}
