mod display;

use pollster::FutureExt as _;

use std::{
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

struct Application<'a> {
    processor: Processor,

    // TODO: move into separate struct so that multiple windows could be supported
    pty: Pty,
    display: Option<Display<'a>>,
    terminal: Terminal<TerminalEventListener>,
}

impl Application<'_> {
    pub fn new(event_loop_proxy: EventLoopProxy<Event>, pty: Pty) -> Self {
        let terminal = Terminal::new(
            Dimensions::default(),
            TerminalEventListener::new(event_loop_proxy),
        );

        Self {
            processor: Processor::new(),
            display: None,
            terminal,
            pty,
        }
    }

    fn redraw(&mut self) {
        if let Some(display) = &mut self.display {
            display.draw(&mut self.terminal);
        }

        //let mut read_buffer = [0; 65536];
        //
        //let res = self.pty.read(&mut read_buffer);
        //
        //match res {
        //    Ok(0) => return,
        //    Ok(size) => {
        //        self.processor
        //            .advance(&mut self.terminal, &read_buffer[..size]);
        //
        //        if let Some(display) = &mut self.display {
        //            display.draw(&mut self.terminal);
        //        }
        //    }
        //    Err(e) => {
        //        debug!("error reading: {e:?}");
        //    }
        //};
    }
}

impl ApplicationHandler<Event> for Application<'_> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes())
            .unwrap();

        let window = Arc::new(window);

        let display = Display::new(window).block_on();

        self.display = Some(display);
    }

    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, event: Event) {
        match event {
            Event::Terminal(event) => match event {
                TerminalEvent::SetTitle(title) => {
                    let Some(display) = &self.display else {
                        return;
                    };

                    display.window.set_title(&title);
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
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let Some(display) = &mut self.display else {
                    return;
                };

                display.set_scale_factor(scale_factor);
                display.window.request_redraw();
            }
            WindowEvent::Resized(size) => {
                let Some(display) = &mut self.display else {
                    return;
                };

                display.set_size(size.width, size.height);
                display.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                self.redraw();
            }
            _ => (),
        }
    }
}
