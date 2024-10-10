use log::debug;

use crate::Executor;

use super::{c0, handler::Handler};

#[derive(Default)]
pub struct Processor {
    parser: crate::Parser,
}

impl Processor {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn advance<H: Handler>(&mut self, handler: &mut H, bytes: &[u8]) {
        let mut executor = HandlerExecutor::new(handler);

        self.parser.advance(&mut executor, bytes);
    }
}

struct HandlerExecutor<'a, H: Handler> {
    handler: &'a mut H,
}

impl<'a, H: Handler + 'a> HandlerExecutor<'a, H> {
    fn new<'b>(handler: &'b mut H) -> HandlerExecutor<'b, H> {
        HandlerExecutor { handler }
    }
}

impl<'a, H: Handler> Executor for HandlerExecutor<'a, H> {
    fn print(&mut self, c: char) {
        todo!()
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            c0::HT => self.handler.put_tab(),
            c0::BS => self.handler.backspace(),
            c0::BEL => self.handler.ring_bell(),
            c0::LF | c0::VT | c0::FF => self.handler.linefeed(),
            // TODO: rest
            _ => debug!("[unhandled] execute byte={:02x}", byte),
        }
    }

    fn put(&mut self, byte: u8) {
        todo!()
    }

    fn hook(
        &mut self,
        params: &crate::param::Params,
        intermediates: &[u8],
        ignore: bool,
        action: char,
    ) {
        todo!()
    }

    fn unhook(&mut self) {
        todo!()
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        todo!()
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        todo!()
    }

    fn csi_dispatch(
        &mut self,
        params: &crate::param::Params,
        intermediates: &[u8],
        ignore: bool,
        action: char,
    ) {
        todo!()
    }
}
