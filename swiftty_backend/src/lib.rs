use std::cmp::min;

mod utf;

#[derive(Default)]
pub struct Backend {
    parser: swiftty_vte::Parser,
    executor: Executor,
}

impl Backend {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn process(&mut self, bytes: &[u8]) {
        self.executor.process(&mut self.parser, bytes);
    }
}

#[derive(Default)]
struct Executor {
    trailing_utf8_bytes: [u8; 4],
    trailing_utf8_bytes_len: usize,
    remaining_utf8_bytes_count: usize,
}

impl Executor {
    fn new() -> Self {
        Default::default()
    }

    fn process(&mut self, parser: &mut swiftty_vte::Parser, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }

        let mut remaining_bytes = bytes;

        if self.remaining_utf8_bytes_count != 0 {
            let mut consumed_bytes_count = 0;

            if remaining_bytes.len() >= self.remaining_utf8_bytes_count {
                consumed_bytes_count = self.remaining_utf8_bytes_count;

                match self.remaining_utf8_bytes_count {
                    1 => {
                        self.trailing_utf8_bytes[self.trailing_utf8_bytes_len] = bytes[0];
                        self.trailing_utf8_bytes_len += 1;
                    }
                    2 => {
                        self.trailing_utf8_bytes[self.trailing_utf8_bytes_len] = bytes[0];
                        self.trailing_utf8_bytes_len += 1;

                        self.trailing_utf8_bytes[self.trailing_utf8_bytes_len] = bytes[1];
                        self.trailing_utf8_bytes_len += 1;
                    }
                    3 => {
                        self.trailing_utf8_bytes[self.trailing_utf8_bytes_len] = bytes[0];
                        self.trailing_utf8_bytes_len += 1;

                        self.trailing_utf8_bytes[self.trailing_utf8_bytes_len] = bytes[1];
                        self.trailing_utf8_bytes_len += 1;

                        self.trailing_utf8_bytes[self.trailing_utf8_bytes_len] = bytes[2];
                        self.trailing_utf8_bytes_len += 1;
                    }
                    _ => unreachable!("at most 3 bytes should remain"),
                }

                // TODO: avoid cloning
                let utf8_bytes = self.trailing_utf8_bytes.clone();

                self.process_utf8(&utf8_bytes);

                self.remaining_utf8_bytes_count = 0;
                self.trailing_utf8_bytes_len = 0;
            } else {
                consumed_bytes_count = remaining_bytes.len();

                match remaining_bytes.len() {
                    1 => {
                        self.trailing_utf8_bytes[self.trailing_utf8_bytes_len] = bytes[0];
                        self.trailing_utf8_bytes_len += 1;
                    }
                    2 => {
                        self.trailing_utf8_bytes[self.trailing_utf8_bytes_len] = bytes[0];
                        self.trailing_utf8_bytes_len += 1;

                        self.trailing_utf8_bytes[self.trailing_utf8_bytes_len] = bytes[1];
                        self.trailing_utf8_bytes_len += 1;
                    }
                    _ => unreachable!(),
                }
            }

            // TODO: +1 ?
            remaining_bytes =
                &remaining_bytes[..min(remaining_bytes.len(), consumed_bytes_count + 1)];
        }

        while !remaining_bytes.is_empty() {
            let Some(utf8_start) = utf::find_utf8_start(remaining_bytes) else {
                for byte in bytes {
                    parser.advance(self, *byte);
                }

                return;
            };

            for i in 0..utf8_start {
                parser.advance(self, bytes[i]);
            }

            remaining_bytes = &remaining_bytes[utf8_start..];

            let utf8_bytes_count =
                utf::expected_utf8_bytes_count(bytes[0]).expect("UTF-8 leading byte must be found");

            if remaining_bytes.len() < utf8_bytes_count as usize {
                self.remaining_utf8_bytes_count =
                    (utf8_bytes_count as usize) - remaining_bytes.len();

                self.trailing_utf8_bytes_len = remaining_bytes.len();

                match remaining_bytes.len() {
                    1 => {
                        self.trailing_utf8_bytes[0] = remaining_bytes[0];
                    }
                    2 => {
                        self.trailing_utf8_bytes[0] = remaining_bytes[0];
                        self.trailing_utf8_bytes[1] = remaining_bytes[1];
                    }
                    3 => {
                        self.trailing_utf8_bytes[0] = remaining_bytes[0];
                        self.trailing_utf8_bytes[1] = remaining_bytes[1];
                        self.trailing_utf8_bytes[2] = remaining_bytes[2];
                    }
                    _ => unreachable!("more than 3 bytes should not occur here"),
                }
            } else {
                let utf8_bytes = &remaining_bytes[..utf8_bytes_count as usize];
                remaining_bytes = &remaining_bytes[utf8_bytes_count as usize..];

                self.process_utf8(utf8_bytes);
            }
        }
    }

    fn process_utf8(&mut self, utf8: &[u8]) {
        println!("process utf8: {:?}", utf8)
    }
}

impl swiftty_vte::executor::Executor for Executor {
    fn print(&mut self, _c: char) {
        todo!()
    }

    fn execute(&mut self, _byte: u8) {
        todo!()
    }

    fn put(&mut self, _byte: u8) {
        todo!()
    }

    fn hook(
        &mut self,
        _params: &swiftty_vte::param::Params,
        _intermediates: &[u8],
        _ignore: bool,
        _action: char,
    ) {
        todo!()
    }

    fn unhook(&mut self) {
        todo!()
    }

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {
        todo!()
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {
        todo!()
    }

    fn csi_dispatch(
        &mut self,
        _params: &swiftty_vte::param::Params,
        _intermediates: &[u8],
        _ignore: bool,
        _action: char,
    ) {
        todo!()
    }
}
