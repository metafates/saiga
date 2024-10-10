use std::cmp::min;

use crate::{Parser, utf8};
use crate::ansi::c0;
use crate::executor::Executor;

#[derive(Default)]
struct Processor {
    utf8: utf8::UTF8Collector,
}

impl Processor {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn process<E: Executor>(
        &mut self,
        parser: &mut Parser,
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
            let Some(next_sequence_start) = c0::first_index_of_c0(remaining_bytes) else {
                self.process_utf8(executor, remaining_bytes);
                return;
            };

            self.process_utf8(executor, &remaining_bytes[..next_sequence_start]);

            if self.utf8.remaining_count > 0 {
                executor.print(char::REPLACEMENT_CHARACTER);
                self.utf8.reset();
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
            let want_bytes_count: usize;

            if self.utf8.remaining_count > 0 {
                want_bytes_count = self.utf8.remaining_count
            } else if let Some(count) = utf8::expected_bytes_count(remaining_bytes[0]) {
                // Optimize for ASCII
                if count == 1 {
                    executor.print(remaining_bytes[0] as char);
                    remaining_bytes = &remaining_bytes[1..];
                    continue;
                }

                want_bytes_count = count;
            } else {
                want_bytes_count = 1;
            }

            let bytes_count = min(want_bytes_count, remaining_bytes.len());

            for i in 0..bytes_count {
                self.utf8.push(remaining_bytes[i]);
            }

            self.utf8.remaining_count = want_bytes_count - bytes_count;

            if self.utf8.remaining_count == 0 {
                self.consume_utf8(executor);
            }

            remaining_bytes = &remaining_bytes[bytes_count..];
        }
    }

    fn consume_utf8<E: Executor>(&mut self, executor: &mut E) {
        executor.print(self.utf8.char());

        self.utf8.reset();
    }
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
            params: &crate::param::Params,
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
            _params: &crate::param::Params,
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
        let mut parser = Parser::new();
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

#[cfg(test)]
mod bench {
    use super::*;
    extern crate test;

    const SAMPLE: &[u8] = b"this is a test for benchmarking processor\x07\x1b[38:2:255:0:255;1m\xD0\x96\xE6\xBC\xA2\xE6\xBC";

    #[derive(Default)]
    struct NopExecutor {}

    impl Executor for NopExecutor {
        fn print(&mut self, _c: char) {}

        fn execute(&mut self, _byte: u8) {}

        fn put(&mut self, _byte: u8) {}

        fn hook(
            &mut self,
            _params: &crate::param::Params,
            _intermediates: &[u8],
            _ignore: bool,
            _action: char,
        ) {
        }

        fn unhook(&mut self) {}

        fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

        fn esc_dispatch(&mut self, __intermediates: &[u8], _ignore: bool, _byte: u8) {}

        fn csi_dispatch(
            &mut self,
            _params: &crate::param::Params,
            _intermediates: &[u8],
            _ignore: bool,
            _action: char,
        ) {
        }
    }

    #[bench]
    fn process(b: &mut test::Bencher) {
        b.iter(|| {
            let mut parser = Parser::new();
            let mut executor = NopExecutor::default();
            let mut processor = Processor::new();

            processor.process(&mut parser, &mut executor, SAMPLE);
        })
    }

    #[bench]
    fn utf8(b: &mut test::Bencher) {
        b.iter(|| {
            let mut executor = NopExecutor::default();
            let mut processor = Processor::new();

            processor.process_utf8(
                &mut executor,
                b"this is a test for benchmarking utf8 processing speed",
            );
        })
    }
}

