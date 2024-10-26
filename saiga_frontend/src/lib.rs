use saiga_backend::{event::Event as TerminalEvent, grid::Dimensions, Terminal};
use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
    window::Window,
};

pub fn run() -> Result<(), EventLoopError> {
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
    window: Option<Window>,
    terminal: Terminal<EventListener>,
}

impl Application {
    pub fn new(event_loop_proxy: EventLoopProxy<Event>) -> Self {
        let terminal = Terminal::new(Dimensions::default(), EventListener::new(event_loop_proxy));

        Self {
            window: None,
            terminal,
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
                TerminalEvent::SetTitle(title) => todo!(),
                TerminalEvent::PtyWrite(payload) => todo!(),
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
            WindowEvent::RedrawRequested => {}
            _ => (),
        }
    }
}
