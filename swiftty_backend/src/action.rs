enum Action {
    Print(char),
    Execute(u8),
    Put(u8),
    Unhook,
}

#[derive(Default)]
struct Dispatcher {
    dispatched: Vec<Action>,
}

impl Dispatcher {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn take_dispatched_actions(&mut self) -> Vec<Action> {
        std::mem::take(&mut self.dispatched)
    }

    pub fn process(&mut self, parser: &mut swiftty_vte::Parser, bytes: &[u8]) {
        let mut i = 0;

        while parser.in_escape_sequence() && i < bytes.len() {
            parser.advance(self, bytes[i]);
            i += 1;
        }

        let mut remaining_bytes = &bytes[i..];

        while !remaining_bytes.is_empty() {
            let Some(next_sequence_start) = index_of(remaining_bytes, 0x1B) else {
                self.process_utf8(remaining_bytes);
                return;
            };

            self.process_utf8(&remaining_bytes[..next_sequence_start]);

            remaining_bytes = &remaining_bytes[next_sequence_start..];

            let mut i = 0;

            loop {
                parser.advance(self, remaining_bytes[i]);
                i += 1;

                if parser.in_escape_sequence() && i < remaining_bytes.len() {
                    break;
                }
            }

            remaining_bytes = &remaining_bytes[i..];
        }
    }

    fn process_utf8(&mut self, bytes: &[u8]) {
        todo!()
    }
}

impl swiftty_vte::executor::Executor for Dispatcher {
    fn print(&mut self, c: char) {
        self.dispatched.push(Action::Print(c));
    }

    fn execute(&mut self, byte: u8) {
        self.dispatched.push(Action::Execute(byte));
    }

    fn put(&mut self, byte: u8) {
        self.dispatched.push(Action::Put(byte));
    }

    fn hook(
        &mut self,
        params: &swiftty_vte::param::Params,
        intermediates: &[u8],
        ignore: bool,
        action: char,
    ) {
        todo!()
    }

    fn unhook(&mut self) {
        self.dispatched.push(Action::Unhook);
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        todo!()
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        todo!()
    }

    fn csi_dispatch(
        &mut self,
        params: &swiftty_vte::param::Params,
        intermediates: &[u8],
        ignore: bool,
        action: char,
    ) {
        todo!()
    }
}

fn index_of(haystack: &[u8], needle: u8) -> Option<usize> {
    // TODO: simd

    for (i, byte) in haystack.iter().enumerate() {
        if *byte == needle {
            return Some(i);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_mixed() {
        let mut parser = swiftty_vte::Parser::new();
        let mut dispatcher = Dispatcher::new();

        dispatcher.process(&mut parser, b"hello\x1b[38:2:255:0:255;1m");
    }
}
