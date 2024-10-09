use core::str;
use std::{
    cmp::min,
    collections::HashSet,
    simd::{cmp::SimdPartialEq, num::SimdUint, u8x16, Simd},
    sync::LazyLock,
};

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
            let Some(next_sequence_start) = first_index_of_c0(remaining_bytes) else {
                self.process_utf8(executor, remaining_bytes);
                return;
            };

            self.process_utf8(executor, &remaining_bytes[..next_sequence_start]);

            if self.is_ready_to_consume_utf8() {
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

            if self.is_ready_to_consume_utf8() {
                self.consume_utf8(executor);
            }

            remaining_bytes = &remaining_bytes[bytes_count..];
        }
    }

    #[inline]
    fn is_ready_to_consume_utf8(&self) -> bool {
        self.utf8_remaining_count == 0 && self.utf8_len > 0
    }

    fn consume_utf8<E: Executor>(&mut self, executor: &mut E) {
        let ch = utf8::into_char(&self.utf8_bytes[..self.utf8_len]);
        executor.print(ch);

        self.utf8_len = 0;
        self.utf8_remaining_count = 0;
    }
}

static C0_ARRAY: [u8; 11] = [
    0x1B, // Escape
    0x0D, // Carriage return
    0x08, // Backspace
    0x07, // Bell
    0x00, // Null
    0x09, // Horizontal Tabulation
    0x0A, // Line Feed
    0x0B, // Vertical Tabulation
    0x0C, // Form Feed
    0x0E, // Shift Out
    0x0F, // Shift In
];

static C0_SET: LazyLock<HashSet<u8>> = LazyLock::new(|| C0_ARRAY.into_iter().collect());

static C0_SPLATS: LazyLock<[Simd<u8, 16>; 11]> = LazyLock::new(|| C0_ARRAY.map(u8x16::splat));

fn first_index_of_c0_scalar(haystack: &[u8]) -> Option<usize> {
    for (i, b) in haystack.iter().enumerate() {
        if C0_SET.contains(b) {
            return Some(i);
        }
    }

    None
}

fn first_index_of_c0(haystack: &[u8]) -> Option<usize> {
    const LANES: usize = 16;

    let indices = u8x16::from_array([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
    let nulls = u8x16::splat(u8::MAX);

    let mut pos = 0;
    let mut left = haystack.len();

    while left > 0 {
        if left < LANES {
            return first_index_of_c0_scalar(haystack);
        }

        let h = u8x16::from_slice(&haystack[pos..pos + LANES]);

        let index = C0_SPLATS
            .into_iter()
            .filter_map(|splat| {
                let matches = h.simd_eq(splat);

                if matches.any() {
                    let result = matches.select(indices, nulls);

                    Some(result.reduce_min() as usize + pos)
                } else {
                    None
                }
            })
            .min();

        if index.is_some() {
            return index;
        }

        pos += LANES;
        left -= LANES;
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
            _params: &swiftty_vte::param::Params,
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
            _params: &swiftty_vte::param::Params,
            _intermediates: &[u8],
            _ignore: bool,
            _action: char,
        ) {
        }
    }

    #[bench]
    fn first_index_of_scalar(b: &mut test::Bencher) {
        b.iter(|| {
            first_index_of_c0_scalar(SAMPLE);
        })
    }
    #[bench]
    fn first_index_of_simd(b: &mut test::Bencher) {
        b.iter(|| {
            first_index_of_c0(SAMPLE);
        })
    }

    #[bench]
    fn process(b: &mut test::Bencher) {
        b.iter(|| {
            let mut parser = swiftty_vte::Parser::new();
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
