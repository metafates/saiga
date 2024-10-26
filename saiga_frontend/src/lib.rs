use std::{
    error::Error,
    io::{Read, Write},
};

use log::debug;
use saiga_backend::{event::Event as TerminalEvent, grid::Dimensions, pty::Pty, Terminal};
use saiga_vte::ansi::processor::Processor;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
    window::Window,
};

pub fn run() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = Application::new(event_loop.create_proxy(), Pty::try_new()?);
    event_loop.run_app(&mut app)?;

    Ok(())
}

#[derive(Debug)]
enum Event {
    Terminal(TerminalEvent),
}

struct EventListener {
    event_loop_proxy: EventLoopProxy<Event>,
}

impl EventListener {
    fn new(event_loop_proxy: EventLoopProxy<Event>) -> Self {
        Self { event_loop_proxy }
    }
}

impl saiga_backend::event::EventListener for EventListener {
    fn on_event(&self, event: TerminalEvent) {
        self.event_loop_proxy
            .send_event(Event::Terminal(event))
            .expect("event loop closed");
    }
}

struct Application {
    processor: Processor,
    pty: Pty,
    window: Option<Window>,
    terminal: Terminal<EventListener>,
}

impl Application {
    pub fn new(event_loop_proxy: EventLoopProxy<Event>, pty: Pty) -> Self {
        let terminal = Terminal::new(Dimensions::default(), EventListener::new(event_loop_proxy));

        Self {
            processor: Processor::new(),
            window: None,
            terminal,
            pty,
        }
    }
}

impl ApplicationHandler<Event> for Application {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes())
            .unwrap();

        self.window = Some(window);
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: Event) {
        match event {
            Event::Terminal(event) => match event {
                TerminalEvent::SetTitle(title) => {
                    let Some(window) = &self.window else {
                        return;
                    };

                    window.set_title(&title);
                }
                TerminalEvent::PtyWrite(payload) => {
                    self.pty.write(&payload).unwrap();
                }
            },
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                let mut read_buffer = [0; 65536];

                let res = self.pty.read(&mut read_buffer);

                match res {
                    Ok(size) => {
                        self.processor
                            .advance(&mut self.terminal, &read_buffer[..size]);
                    }
                    Err(e) => {
                        debug!("error reading: {e:?}");
                        return;
                    }
                };

                for c in self.terminal.grid().iter() {
                    debug!("cell: {c:?}")
                }
            }
            _ => (),
        }
    }
}
