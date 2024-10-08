use std::{
    char,
    cmp::{max, min},
};

use swiftty_vte::executor::Executor;

use crate::utf;

#[derive(Debug, PartialEq)]
enum Action {
    Print(char),
    Execute(u8),
    Put(u8),
    Unhook,
    CsiDispatch {
        params: Vec<Vec<u16>>,
        intermediates: Vec<u8>,
        ignore: bool,
        action: char,
    },
}

#[derive(Default)]
struct Dispatcher {
    dispatched: Vec<Action>,

    utf8_bytes: [u8; 4],
    utf8_len: usize,
    utf8_remaining_count: usize,
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

            if self.utf8_remaining_count > 0 {
                self.consume_utf8();
            }

            remaining_bytes = &remaining_bytes[next_sequence_start..];

            let mut i = 0;

            loop {
                parser.advance(self, remaining_bytes[i]);
                i += 1;

                if !(parser.in_escape_sequence() && i < remaining_bytes.len()) {
                    break;
                }
            }

            remaining_bytes = &remaining_bytes[i..];
        }
    }

    fn process_utf8(&mut self, bytes: &[u8]) {
        let mut remaining_bytes = bytes;

        while !remaining_bytes.is_empty() {
            let want_bytes_count = {
                if self.utf8_remaining_count > 0 {
                    self.utf8_remaining_count
                } else if let Some(count) = utf::expected_utf8_bytes_count(remaining_bytes[0]) {
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
                self.consume_utf8();
            }

            remaining_bytes = &remaining_bytes[bytes_count..];
        }
    }

    fn consume_utf8(&mut self) {
        let ch = utf::char_from_utf8(&self.utf8_bytes[..self.utf8_len]);
        self.print(ch);

        self.utf8_len = 0;
        self.utf8_remaining_count = 0;
    }
}

impl Executor for Dispatcher {
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
        self.dispatched.push(Action::CsiDispatch {
            params: params
                .into_iter()
                .map(|p| p.into_iter().collect())
                .collect(),
            intermediates: intermediates.into_iter().map(|b| *b).collect(),
            ignore,
            action,
        });
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
        dispatcher.process(&mut parser, &[0xD0]);
        dispatcher.process(&mut parser, &[0x96]);
        dispatcher.process(&mut parser, &[0xE6, 0xBC, 0xA2]);
        dispatcher.process(&mut parser, &[0xE6, 0xBC, 0x1B]); // abort utf8 sequence

        let actions = dispatcher.take_dispatched_actions();

        assert_eq!(
            actions,
            vec![
                Action::Print('h'),
                Action::Print('e'),
                Action::Print('l'),
                Action::Print('l'),
                Action::Print('o'),
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
