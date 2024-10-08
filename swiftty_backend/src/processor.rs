use std::cmp::min;

use swiftty_vte::executor::Executor;

use crate::utf8;

#[derive(Default)]
struct Processor {
    utf8_bytes: [u8; 4],
    utf8_len: usize,
    utf8_remaining_count: usize,
}

impl Processor {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn process<E: Executor>(
        &mut self,
        parser: &mut swiftty_vte::Parser,
        executor: &mut E,
        bytes: &[u8],
    ) {
        let mut i = 0;

        while parser.in_escape_sequence() && i < bytes.len() {
            parser.advance(executor, bytes[i]);
            i += 1;
        }

        let mut remaining_bytes = &bytes[i..];

        while !remaining_bytes.is_empty() {
            let Some(next_sequence_start) = index_of_c0(remaining_bytes) else {
                self.process_utf8(executor, remaining_bytes);
                return;
            };

            self.process_utf8(executor, &remaining_bytes[..next_sequence_start]);

            if self.utf8_remaining_count > 0 {
                self.consume_utf8(executor);
            }

            remaining_bytes = &remaining_bytes[next_sequence_start..];

            let mut i = 0;

            loop {
                parser.advance(executor, remaining_bytes[i]);
                i += 1;

                if !(parser.in_escape_sequence() && i < remaining_bytes.len()) {
                    break;
                }
            }

            remaining_bytes = &remaining_bytes[i..];
        }
    }

    fn process_utf8<E: Executor>(&mut self, executor: &mut E, bytes: &[u8]) {
        let mut remaining_bytes = bytes;

        while !remaining_bytes.is_empty() {
            let want_bytes_count = {
                if self.utf8_remaining_count > 0 {
                    self.utf8_remaining_count
                } else if let Some(count) = utf8::expected_bytes_count(remaining_bytes[0]) {
                    count
                } else {
                    1
                }
            };

            let bytes_count = min(want_bytes_count, remaining_bytes.len());

            for i in 0..bytes_count {
                self.utf8_bytes[self.utf8_len] = remaining_bytes[i];
                self.utf8_len += 1;
            }

            self.utf8_remaining_count = want_bytes_count - bytes_count;

            if self.utf8_remaining_count == 0 {
                self.consume_utf8(executor);
            }

            remaining_bytes = &remaining_bytes[bytes_count..];
        }
    }

    fn consume_utf8<E: Executor>(&mut self, executor: &mut E) {
        let ch = utf8::into_char(&self.utf8_bytes[..self.utf8_len]);
        executor.print(ch);

        self.utf8_len = 0;
        self.utf8_remaining_count = 0;
    }
}

fn index_of_c0(haystack: &[u8]) -> Option<usize> {
    static C0: [u8; 11] = [
        0x1B, // ESC
        0x0D, // Carriage return
        0x08, // Backspace
        0x07, // BEL
        0x00, // NUL
        0x09, 0x0A, 0x0B, 0x0C, 0x0E, 0x0F,
    ];

    for byte in C0 {
        let index = index_of(haystack, byte);

        if index.is_some() {
            return index;
        }
    }

    None
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

    #[derive(Debug, PartialEq)]
    enum Action {
        Print(char),
        CsiDispatch {
            params: Vec<Vec<u16>>,
            intermediates: Vec<u8>,
            ignore: bool,
            action: char,
        },
        Execute(u8),
    }

    #[derive(Default)]
    struct Dispatcher {
        actions: Vec<Action>,
    }

    impl Executor for Dispatcher {
        fn print(&mut self, c: char) {
            self.actions.push(Action::Print(c));
        }

        fn csi_dispatch(
            &mut self,
            params: &swiftty_vte::param::Params,
            intermediates: &[u8],
            ignore: bool,
            action: char,
        ) {
            self.actions.push(Action::CsiDispatch {
                params: params
                    .into_iter()
                    .map(|p| p.into_iter().collect())
                    .collect(),
                intermediates: intermediates.into(),
                ignore,
                action,
            });
        }

        fn execute(&mut self, byte: u8) {
            self.actions.push(Action::Execute(byte));
        }

        fn put(&mut self, _byte: u8) {
            unimplemented!()
        }

        fn hook(
            &mut self,
            _params: &swiftty_vte::param::Params,
            _intermediates: &[u8],
            _ignore: bool,
            _action: char,
        ) {
            unimplemented!()
        }

        fn unhook(&mut self) {
            unimplemented!()
        }

        fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {
            unimplemented!()
        }

        fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {
            unimplemented!()
        }
    }

    #[test]
    fn process_mixed() {
        let mut parser = swiftty_vte::Parser::new();
        let mut dispatcher = Dispatcher::default();
        let mut processor = Processor::new();

        processor.process(
            &mut parser,
            &mut dispatcher,
            b"hello\x07\x1b[38:2:255:0:255;1m",
        );
        processor.process(&mut parser, &mut dispatcher, &[0xD0]);
        processor.process(&mut parser, &mut dispatcher, &[0x96]);
        processor.process(&mut parser, &mut dispatcher, &[0xE6, 0xBC, 0xA2]);
        processor.process(&mut parser, &mut dispatcher, &[0xE6, 0xBC, 0x1B]); // abort utf8 sequence

        assert_eq!(
            dispatcher.actions,
            vec![
                Action::Print('h'),
                Action::Print('e'),
                Action::Print('l'),
                Action::Print('l'),
                Action::Print('o'),
                Action::Execute(0x07),
                Action::CsiDispatch {
                    params: vec![vec![38, 2, 255, 0, 255], vec![1]],
                    intermediates: vec![],
                    ignore: false,
                    action: 'm',
                },
                Action::Print('Ж'),
                Action::Print('漢'),
                Action::Print(char::REPLACEMENT_CHARACTER),
            ]
        );
    }
}
