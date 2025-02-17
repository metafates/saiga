pub mod ansi;
pub mod param;

mod table;
mod utf8;

use ansi::c0;
use arrayvec::ArrayVec;
use param::{Params, Subparam, PARAM_SEPARATOR};
use std::{char, mem::MaybeUninit, str::from_utf8_unchecked};
use table::{Action, State};

/// X3.64 doesn’t place any limit on the number of intermediate characters allowed before a final character,
/// although it doesn’t define any control sequences with more than one.
/// Digital defined escape sequences with two intermediate characters,
/// and control sequences and device control strings with one.
const MAX_INTERMEDIATES: usize = 2;

/// There is no limit to the number of characters in a parameter string,
/// although a maximum of 16 parameters need be stored.
const MAX_OSC_PARAMS: usize = 16;

const MAX_OSC_RAW: usize = 1024;

pub trait Executor {
    /// Draw a character to the screen.
    fn print(&mut self, c: char);

    /// Execute C0 or C1 control function
    fn execute(&mut self, byte: u8);

    /// Pass bytes as part of a device control string to the handle chosen in `hook`. C0 controls
    /// will also be passed to the handler.
    fn put(&mut self, byte: u8);

    /// Invoked when a final character arrives in first part of device control string.
    ///
    /// The control function should be determined from the private marker, final character, and
    /// execute with a parameter list. A handler should be selected for remaining characters in the
    /// string; the handler function should subsequently be called by `put` for every character in
    /// the control string.
    ///
    /// The `ignore` flag indicates that more than two intermediates arrived and
    /// subsequent characters were ignored.
    fn hook(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char);

    /// Called when a device control string is terminated.
    ///
    /// The previously selected handler should be notified that the DCS has
    /// terminated.
    fn unhook(&mut self);

    /// Dispatch an operating system command.
    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool);

    /// The final character of an escape sequence has arrived.
    ///
    /// The `ignore` flag indicates that more than two intermediates arrived and
    /// subsequent characters were ignored.
    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8);

    /// A final character has arrived for a CSI sequence
    ///
    /// The `ignore` flag indicates that either more than two intermediates arrived
    /// or the number of parameters exceeded the maximum supported length,
    /// and subsequent characters were ignored.
    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char);
}
#[derive(Default)]
pub struct Intermediates {
    array: [u8; MAX_INTERMEDIATES],
    index: usize,
}

impl Intermediates {
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        &self.array[..self.index]
    }

    #[inline]
    pub const fn is_full(&self) -> bool {
        self.index == MAX_INTERMEDIATES
    }

    pub const fn push(&mut self, byte: u8) {
        self.array[self.index] = byte;
        self.index += 1;
    }

    #[inline]
    pub const fn clear(&mut self) {
        self.index = 0
    }
}

#[derive(Default)]
pub struct OscHandler {
    params: [(usize, usize); MAX_OSC_PARAMS],
    params_num: usize,
    raw: ArrayVec<u8, MAX_OSC_RAW>,
}

impl OscHandler {
    pub fn start(&mut self) {
        self.raw.clear();
        self.params_num = 0;
    }

    pub fn put(&mut self, byte: u8) {
        let idx = self.raw.len();

        if byte == PARAM_SEPARATOR {
            let param_idx = self.params_num;

            match param_idx {
                // Only process up to MAX_OSC_PARAMS
                MAX_OSC_PARAMS => return,

                // First param is special - 0 to current byte index
                0 => {
                    self.params[param_idx] = (0, idx);
                }

                // All other params depend on previous indexing
                _ => {
                    let prev = self.params[param_idx - 1];
                    let begin = prev.1;
                    self.params[param_idx] = (begin, idx);
                }
            }

            self.params_num += 1;
        } else {
            let _ = self.raw.try_push(byte);
        }
    }

    pub fn end(&mut self, executor: &mut impl Executor, byte: u8) {
        let param_idx = self.params_num;
        let idx = self.raw.len();

        match param_idx {
            // Finish last parameter if not already maxed
            MAX_OSC_PARAMS => (),

            // First param is special - 0 to current byte index
            0 => {
                self.params[param_idx] = (0, idx);
                self.params_num += 1;
            }

            // All other params depend on previous indexing
            _ => {
                let prev = self.params[param_idx - 1];
                let begin = prev.1;
                self.params[param_idx] = (begin, idx);
                self.params_num += 1;
            }
        }

        self.dispatch(executor, byte);
    }

    pub fn dispatch(&self, executor: &mut impl Executor, byte: u8) {
        let mut slices: [MaybeUninit<&[u8]>; MAX_OSC_PARAMS] =
            unsafe { MaybeUninit::uninit().assume_init() };

        for (i, slice) in slices.iter_mut().enumerate().take(self.params_num) {
            let indices = self.params[i];
            *slice = MaybeUninit::new(&self.raw[indices.0..indices.1]);
        }

        unsafe {
            let num_params = self.params_num;
            let params = &slices[..num_params] as *const [MaybeUninit<&[u8]>] as *const [&[u8]];
            executor.osc_dispatch(&*params, byte == 0x07);
        }
    }
}

#[derive(Default)]
pub struct Parser {
    state: State,

    osc_handler: OscHandler,

    params: Params,
    subparam: Subparam,

    intermediate_handler: Intermediates,

    ignoring: bool,
    // utf8: utf8::UTF8Collector,
}

impl Parser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn advance(&mut self, executor: &mut impl Executor, bytes: &[u8]) {
        let mut i = 0;

        while self.in_escape_sequence() && i < bytes.len() {
            self.advance_sequence(executor, bytes[i]);
            i += 1;
        }

        let mut remaining_bytes = &bytes[i..];

        while !remaining_bytes.is_empty() {
            let Some(next_sequence_start) = c0::first_index_of_c0(remaining_bytes) else {
                self.advance_utf8(executor, remaining_bytes);
                return;
            };

            self.advance_utf8(executor, &remaining_bytes[..next_sequence_start]);

            remaining_bytes = &remaining_bytes[next_sequence_start..];

            let mut i = 0;

            loop {
                self.advance_sequence(executor, remaining_bytes[i]);
                i += 1;

                if !(self.in_escape_sequence() && i < remaining_bytes.len()) {
                    break;
                }
            }

            remaining_bytes = &remaining_bytes[i..];
        }
    }

    fn advance_utf8(&mut self, executor: &mut impl Executor, bytes: &[u8]) {
        match simdutf8::compat::from_utf8(bytes) {
            Ok(s) => {
                for c in s.chars() {
                    executor.print(c);
                }
            }
            Err(err) => {
                let up_to = err.valid_up_to();

                let s = unsafe { from_utf8_unchecked(&bytes[..up_to]) };

                for c in s.chars() {
                    executor.print(c);
                }

                executor.print(char::REPLACEMENT_CHARACTER);
            }
        }
        //
        // let mut remaining_bytes = bytes;
        //
        // while !remaining_bytes.is_empty() {
        //     let want_bytes_count: usize;
        //
        //     if self.utf8.remaining_count != 0 {
        //         want_bytes_count = self.utf8.remaining_count
        //     } else if let Some(count) = utf8::expected_bytes_count(remaining_bytes[0]) {
        //         // Optimize for ASCII
        //         if count == 1 {
        //             executor.print(remaining_bytes[0] as char);
        //             remaining_bytes = &remaining_bytes[1..];
        //             continue;
        //         }
        //
        //         want_bytes_count = count;
        //     } else {
        //         want_bytes_count = 1;
        //     }
        //
        //     let bytes_count = want_bytes_count.min(remaining_bytes.len());
        //
        //     for b in remaining_bytes[..bytes_count].iter() {
        //         self.utf8.push(*b);
        //     }
        //
        //     self.utf8.remaining_count = want_bytes_count - bytes_count;
        //
        //     if self.utf8.remaining_count == 0 {
        //         self.consume_utf8(executor);
        //     }
        //
        //     remaining_bytes = &remaining_bytes[bytes_count..];
        // }
    }

    // fn consume_utf8(&mut self, executor: &mut impl Executor) {
    //     executor.print(self.utf8.char());
    //
    //     self.utf8.reset();
    // }

    fn advance_sequence(&mut self, executor: &mut impl Executor, byte: u8) {
        let change = table::change_state(State::Anywhere, byte)
            .or_else(|| table::change_state(self.state, byte));

        let Some((state, action)) = change else {
            return;
        };

        self.state_change(executor, state, action, byte);
    }

    #[inline]
    fn in_escape_sequence(&self) -> bool {
        self.state != State::Ground
    }

    fn state_change<E: Executor>(
        &mut self,
        executor: &mut E,
        state: State,
        action: Option<Action>,
        byte: u8,
    ) {
        // moving to Anywhere means executing current action right away
        match state {
            State::Anywhere => {
                let Some(action) = action else {
                    return;
                };

                self.execute_action(executor, action, byte);
            }
            state => {
                self.execute_state_exit_action(executor, byte);

                // transition
                if let Some(action) = action {
                    self.execute_action(executor, action, byte);
                }

                self.state = state;

                self.execute_state_entry_action(executor, byte);
            }
        }
    }

    fn execute_state_entry_action(&mut self, executor: &mut impl Executor, byte: u8) {
        match self.state {
            State::CsiEntry | State::DcsEntry | State::Escape => {
                self.execute_action(executor, Action::Clear, byte);
            }
            State::OscString => {
                self.execute_action(executor, Action::OscStart, byte);
            }
            State::DcsPassthrough => {
                self.execute_action(executor, Action::Hook, byte);
            }
            _ => (),
        }
    }

    fn execute_state_exit_action(&mut self, executor: &mut impl Executor, byte: u8) {
        match self.state {
            State::DcsPassthrough => {
                self.execute_action(executor, Action::Unhook, byte);
            }
            State::OscString => {
                self.execute_action(executor, Action::OscEnd, byte);
            }
            _ => {}
        }
    }

    fn execute_action(&mut self, executor: &mut impl Executor, action: Action, byte: u8) {
        use Action::*;

        match action {
            Print => executor.print(byte as char),
            Put => executor.put(byte),
            Execute => executor.execute(byte),
            OscStart => self.osc_handler.start(),
            OscPut => self.osc_handler.put(byte),
            OscEnd => self.osc_handler.end(executor, byte),
            Hook => {
                if self.params.is_full() {
                    self.ignoring = true;
                } else {
                    self.params.push_subparam(self.subparam);
                    self.params.next_param();
                }

                executor.hook(
                    &self.params,
                    self.intermediate_handler.as_slice(),
                    self.ignoring,
                    byte as char,
                );
            }
            Unhook => executor.unhook(),
            Param => {
                if self.params.is_full() {
                    self.ignoring = true;
                    return;
                }

                match byte {
                    param::PARAM_SEPARATOR => {
                        self.params.push_subparam(self.subparam);
                        self.params.next_param();
                        self.subparam = Subparam::default();
                    }
                    param::SUBPARAM_SEPARATOR => {
                        self.params.push_subparam(self.subparam);
                        self.subparam = Subparam::default();
                    }
                    byte => {
                        self.subparam = self.subparam.saturating_mul(10);
                        self.subparam = self.subparam.saturating_add((byte - b'0').into());
                    }
                };
            }
            CsiDispatch => {
                if self.params.is_full() {
                    self.ignoring = true
                } else {
                    self.params.push_subparam(self.subparam);
                    self.params.next_param();
                }

                executor.csi_dispatch(
                    &self.params,
                    self.intermediate_handler.as_slice(),
                    self.ignoring,
                    byte as char,
                );
            }
            Collect => {
                if self.intermediate_handler.is_full() {
                    self.ignoring = true
                } else {
                    self.intermediate_handler.push(byte);
                }
            }
            EscDispatch => {
                executor.esc_dispatch(self.intermediate_handler.as_slice(), self.ignoring, byte);
            }
            Clear => {
                self.subparam = Subparam::default();
                self.params.clear();

                self.ignoring = false;

                self.intermediate_handler.clear();
            }
            Ignore => (),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Dispatcher {
        dispatched: Vec<Sequence>,
    }

    #[derive(Debug, PartialEq, Eq)]
    enum Sequence {
        Osc(Vec<Vec<u8>>, bool),
        Csi(Vec<Vec<u16>>, Vec<u8>, bool, char),
        Esc(Vec<u8>, bool, u8),
        DcsHook(Vec<Vec<u16>>, Vec<u8>, bool, char),
        DcsPut(u8),
        DcsUnhook,
        Execute(u8),
        Print(char),
    }

    impl Executor for Dispatcher {
        fn print(&mut self, c: char) {
            self.dispatched.push(Sequence::Print(c));
        }

        fn execute(&mut self, byte: u8) {
            self.dispatched.push(Sequence::Execute(byte))
        }

        fn put(&mut self, byte: u8) {
            self.dispatched.push(Sequence::DcsPut(byte));
        }

        fn hook(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
            let params = params
                .as_slice()
                .iter()
                .map(|param| param.as_slice().to_vec())
                .collect();

            let intermediates = intermediates.to_vec();

            self.dispatched
                .push(Sequence::DcsHook(params, intermediates, ignore, c));
        }

        fn unhook(&mut self) {
            self.dispatched.push(Sequence::DcsUnhook);
        }

        fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
            let params = params.iter().map(|p| p.to_vec()).collect();

            self.dispatched.push(Sequence::Osc(params, bell_terminated));
        }

        fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
            let intermediates = intermediates.to_vec();

            self.dispatched
                .push(Sequence::Esc(intermediates, ignore, byte));
        }

        fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
            let params = params
                .as_slice()
                .iter()
                .map(|param| param.as_slice().to_vec())
                .collect();

            let intermediates = intermediates.to_vec();

            self.dispatched
                .push(Sequence::Csi(params, intermediates, ignore, c));
        }
    }

    mod c0_or_c1 {
        use super::*;

        #[test]
        fn all() {
            let mut dispatcher = Dispatcher::default();
            let mut parser = Parser::new();

            parser.advance(&mut dispatcher, b"\x07\x08\x00");

            assert_eq!(
                dispatcher.dispatched,
                vec![
                    Sequence::Execute(0x07),
                    Sequence::Execute(0x08),
                    Sequence::Execute(0x00),
                ]
            )
        }
    }

    mod osc {
        use super::*;

        static OSC_BYTES: &[u8] = &[
            0x1b, 0x5d, // Begin OSC
            b'2', b';', b'j', b'w', b'i', b'l', b'm', b'@', b'j', b'w', b'i', b'l', b'm', b'-',
            b'd', b'e', b's', b'k', b':', b' ', b'~', b'/', b'c', b'o', b'd', b'e', b'/', b's',
            b'a', b'i', b'g', b'a', 0x9c, // End OSC
        ];

        #[test]
        fn parse() {
            let mut dispatcher = Dispatcher::default();
            let mut parser = Parser::new();

            parser.advance(&mut dispatcher, OSC_BYTES);

            assert_eq!(dispatcher.dispatched.len(), 1);
            match &dispatcher.dispatched[0] {
                Sequence::Osc(params, _) => {
                    assert_eq!(params.len(), 2);
                    assert_eq!(params[0], &OSC_BYTES[2..3]);
                    assert_eq!(params[1], &OSC_BYTES[4..(OSC_BYTES.len() - 1)]);
                }
                _ => panic!("expected osc sequence"),
            }
        }

        #[test]
        fn parse_empty() {
            let mut dispatcher = Dispatcher::default();
            let mut parser = Parser::new();

            parser.advance(&mut dispatcher, &[0x1b, 0x5d, 0x07]);

            assert_eq!(dispatcher.dispatched.len(), 1);
            match &dispatcher.dispatched[0] {
                Sequence::Osc(..) => (),
                _ => panic!("expected osc sequence"),
            }
        }

        #[test]
        fn parse_max_params() {
            let params = ";".repeat(param::MAX_PARAMS + 1);
            let input = format!("\x1b]{}\x1b", &params[..]).into_bytes();
            let mut dispatcher = Dispatcher::default();
            let mut parser = Parser::new();

            parser.advance(&mut dispatcher, input.as_slice());

            assert_eq!(dispatcher.dispatched.len(), 1);

            match &dispatcher.dispatched[0] {
                Sequence::Osc(params, _) => {
                    assert_eq!(params.len(), MAX_OSC_PARAMS);
                    assert!(params.iter().all(Vec::is_empty));
                }
                _ => panic!("expected osc sequence"),
            }
        }

        #[test]
        fn exceed_max_buffer_size() {
            static NUM_BYTES: usize = MAX_OSC_PARAMS + 100;
            static INPUT_START: &[u8] = &[0x1b, b']', b'5', b'2', b';', b's'];
            static INPUT_END: &[u8] = b"\x07";

            let mut dispatcher = Dispatcher::default();
            let mut parser = Parser::new();

            // Create valid OSC escape
            parser.advance(&mut dispatcher, INPUT_START);

            // Exceed max buffer size
            parser.advance(&mut dispatcher, [b'a'].repeat(NUM_BYTES).as_slice());

            // Terminate escape for dispatch
            parser.advance(&mut dispatcher, INPUT_END);

            assert_eq!(dispatcher.dispatched.len(), 1);
            match &dispatcher.dispatched[0] {
                Sequence::Osc(params, _) => {
                    assert_eq!(params.len(), 2);
                    assert_eq!(params[0], b"52");
                    assert_eq!(params[1].len(), NUM_BYTES + INPUT_END.len());
                }
                _ => panic!("expected osc sequence"),
            }
        }

        #[test]
        fn bell_terminated() {
            static INPUT: &[u8] = b"\x1b]11;ff/00/ff\x07";
            let mut dispatcher = Dispatcher::default();
            let mut parser = Parser::new();

            parser.advance(&mut dispatcher, INPUT);

            assert_eq!(dispatcher.dispatched.len(), 1);
            match &dispatcher.dispatched[0] {
                Sequence::Osc(_, true) => (),
                _ => panic!("expected osc with bell terminator"),
            }
        }
    }

    mod csi {
        use super::*;

        #[test]
        fn parse_max_params() {
            // This will build a list of repeating '1;'s
            // The length is MAX_PARAMS - 1 because the last semicolon is interpreted
            // as an implicit zero, making the total number of parameters MAX_PARAMS
            let params = "1;".repeat(param::MAX_PARAMS - 1);
            let input = format!("\x1b[{}p", &params[..]).into_bytes();

            let mut dispatcher = Dispatcher::default();
            let mut parser = Parser::new();

            parser.advance(&mut dispatcher, &input);

            assert_eq!(dispatcher.dispatched.len(), 1);
            match &dispatcher.dispatched[0] {
                Sequence::Csi(params, _, ignore, _) => {
                    assert_eq!(params.len(), param::MAX_PARAMS);
                    assert!(!ignore);
                }
                _ => panic!("expected csi sequence"),
            }
        }

        #[test]
        fn parse_params_ignore_long_params() {
            // This will build a list of repeating '1;'s
            // The length is MAX_PARAMS because the last semicolon is interpreted
            // as an implicit zero, making the total number of parameters MAX_PARAMS + 1
            let params = "1;".repeat(param::MAX_PARAMS);
            let input = format!("\x1b[{}p", &params[..]).into_bytes();

            let mut dispatcher = Dispatcher::default();
            let mut parser = Parser::new();

            parser.advance(&mut dispatcher, &input);

            assert_eq!(dispatcher.dispatched.len(), 1);
            match &dispatcher.dispatched[0] {
                Sequence::Csi(params, _, ignore, _) => {
                    assert_eq!(params.len(), param::MAX_PARAMS);
                    assert!(ignore);
                }
                _ => panic!("expected csi sequence"),
            }
        }

        #[test]
        fn parse_params_trailing_semicolon() {
            let mut dispatcher = Dispatcher::default();
            let mut parser = Parser::new();

            parser.advance(&mut dispatcher, b"\x1b[4;m");

            assert_eq!(dispatcher.dispatched.len(), 1);

            match &dispatcher.dispatched[0] {
                Sequence::Csi(params, ..) => assert_eq!(params, &[[4], [0]]),
                _ => panic!("expected csi sequence"),
            }
        }

        #[test]
        fn parse_params_leading_semicolon() {
            // Create dispatcher and check state
            let mut dispatcher = Dispatcher::default();
            let mut parser = Parser::new();

            parser.advance(&mut dispatcher, b"\x1b[;4m");

            assert_eq!(dispatcher.dispatched.len(), 1);
            match &dispatcher.dispatched[0] {
                Sequence::Csi(params, ..) => assert_eq!(params, &[[0], [4]]),
                _ => panic!("expected csi sequence"),
            }
        }

        #[test]
        fn parse_long_param() {
            // The important part is the parameter, which is (i64::MAX + 1)
            static INPUT: &[u8] = b"\x1b[9223372036854775808m";

            let mut dispatcher = Dispatcher::default();
            let mut parser = Parser::new();

            parser.advance(&mut dispatcher, INPUT);

            assert_eq!(dispatcher.dispatched.len(), 1);
            match &dispatcher.dispatched[0] {
                Sequence::Csi(params, ..) => assert_eq!(params, &[[u16::MAX]]),
                _ => panic!("expected csi sequence"),
            }
        }

        #[test]
        fn reset() {
            static INPUT: &[u8] = b"\x1b[3;1\x1b[?1049h";

            let mut dispatcher = Dispatcher::default();
            let mut parser = Parser::new();

            parser.advance(&mut dispatcher, INPUT);

            assert_eq!(dispatcher.dispatched.len(), 1);
            match &dispatcher.dispatched[0] {
                Sequence::Csi(params, intermediates, ignore, _) => {
                    assert_eq!(intermediates, b"?");
                    assert_eq!(params, &[[1049]]);
                    assert!(!ignore);
                }
                _ => panic!("expected csi sequence"),
            }
        }

        #[test]
        fn subparameters() {
            static INPUT: &[u8] = b"\x1b[38:2:255:0:255;1m";

            let mut dispatcher = Dispatcher::default();
            let mut parser = Parser::new();

            parser.advance(&mut dispatcher, INPUT);

            assert_eq!(dispatcher.dispatched.len(), 1);

            match &dispatcher.dispatched[0] {
                Sequence::Csi(params, intermediates, ignore, _) => {
                    assert_eq!(params, &[vec![38, 2, 255, 0, 255], vec![1]]);
                    assert_eq!(intermediates, &[]);
                    assert!(!ignore);
                }
                _ => panic!("expected csi sequence"),
            }
        }

        #[test]
        fn params_buffer_filled_with_subparam() {
            static INPUT: &[u8] = b"\x1b[::::::::::::::::::::::::::::::::;;;;;;;;;;;;;;;;x\x1b";

            let mut dispatcher = Dispatcher::default();
            let mut parser = Parser::new();

            parser.advance(&mut dispatcher, INPUT);

            assert_eq!(dispatcher.dispatched.len(), 1);

            match &dispatcher.dispatched[0] {
                Sequence::Csi(params, intermediates, ignore, c) => {
                    assert_eq!(intermediates, &[]);
                    assert_eq!(
                        *params,
                        vec![
                            vec![0; 32],
                            vec![0],
                            vec![0],
                            vec![0],
                            vec![0],
                            vec![0],
                            vec![0],
                            vec![0],
                            vec![0],
                            vec![0],
                            vec![0],
                            vec![0],
                            vec![0],
                            vec![0],
                            vec![0],
                            vec![0],
                        ]
                    );
                    assert_eq!(c, &'x');
                    assert!(ignore);
                }
                _ => panic!("expected csi sequence"),
            }
        }
    }

    mod dcs {
        use super::*;

        #[test]
        fn parse_max_params() {
            let params = "1;".repeat(param::MAX_PARAMS + 1);
            let input = format!("\x1bP{}p", &params[..]).into_bytes();
            let mut dispatcher = Dispatcher::default();
            let mut parser = Parser::new();

            parser.advance(&mut dispatcher, &input);

            assert_eq!(dispatcher.dispatched.len(), 1);
            match &dispatcher.dispatched[0] {
                Sequence::DcsHook(params, _, ignore, _) => {
                    assert_eq!(params.len(), param::MAX_PARAMS);
                    assert!(params.iter().all(|param| param == &[1]));
                    assert!(ignore);
                }
                _ => panic!("expected dcs sequence"),
            }
        }

        #[test]
        fn reset() {
            static INPUT: &[u8] = b"\x1b[3;1\x1bP1$tx\x9c";
            let mut dispatcher = Dispatcher::default();
            let mut parser = Parser::new();

            parser.advance(&mut dispatcher, INPUT);

            assert_eq!(dispatcher.dispatched.len(), 3);

            match &dispatcher.dispatched[0] {
                Sequence::DcsHook(params, intermediates, ignore, _) => {
                    assert_eq!(intermediates, b"$");
                    assert_eq!(params, &[[1]]);
                    assert!(!ignore);
                }
                _ => panic!("expected dcs sequence"),
            }

            assert_eq!(dispatcher.dispatched[1], Sequence::DcsPut(b'x'));
            assert_eq!(dispatcher.dispatched[2], Sequence::DcsUnhook);
        }

        #[test]
        fn parse() {
            static INPUT: &[u8] = b"\x1bP0;1|17/ab\x9c";
            let mut dispatcher = Dispatcher::default();
            let mut parser = Parser::new();

            parser.advance(&mut dispatcher, INPUT);

            assert_eq!(dispatcher.dispatched.len(), 7);

            match &dispatcher.dispatched[0] {
                Sequence::DcsHook(params, _, _, c) => {
                    assert_eq!(params, &[[0], [1]]);
                    assert_eq!(c, &'|');
                }
                _ => panic!("expected dcs sequence"),
            }

            for (i, byte) in b"17/ab".iter().enumerate() {
                assert_eq!(dispatcher.dispatched[1 + i], Sequence::DcsPut(*byte));
            }

            assert_eq!(dispatcher.dispatched[6], Sequence::DcsUnhook);
        }

        #[test]
        fn intermediate_reset_on_exit() {
            static INPUT: &[u8] = b"\x1bP=1sZZZ\x1b+\x5c";
            let mut dispatcher = Dispatcher::default();
            let mut parser = Parser::new();

            parser.advance(&mut dispatcher, INPUT);

            assert_eq!(dispatcher.dispatched.len(), 6);
            match &dispatcher.dispatched[5] {
                Sequence::Esc(intermediates, ..) => assert_eq!(intermediates, b"+"),
                _ => panic!("expected esc sequence"),
            }
        }
    }

    mod esc {
        use super::*;

        #[test]
        fn reset() {
            static INPUT: &[u8] = b"\x1b[3;1\x1b(A";
            let mut dispatcher = Dispatcher::default();
            let mut parser = Parser::new();

            parser.advance(&mut dispatcher, INPUT);

            assert_eq!(dispatcher.dispatched.len(), 1);
            match &dispatcher.dispatched[0] {
                Sequence::Esc(intermediates, ignore, byte) => {
                    assert_eq!(intermediates, b"(");
                    assert_eq!(*byte, b'A');
                    assert!(!ignore);
                }
                _ => panic!("expected esc sequence"),
            }
        }
    }

    mod utf8 {
        use super::*;

        #[test]
        fn process_mixed() {
            let mut parser = Parser::new();
            let mut dispatcher = Dispatcher::default();

            parser.advance(&mut dispatcher, b"hello\x07\x1b[38:2:255:0:255;1m");
            parser.advance(&mut dispatcher, &[0xD0]);
            parser.advance(&mut dispatcher, &[0x96]);
            parser.advance(&mut dispatcher, &[0xE6, 0xBC, 0xA2]);
            parser.advance(&mut dispatcher, &[0xE6, 0xBC, 0x1B]); // abort utf8 sequence

            assert_eq!(
                dispatcher.dispatched,
                vec![
                    Sequence::Print('h'),
                    Sequence::Print('e'),
                    Sequence::Print('l'),
                    Sequence::Print('l'),
                    Sequence::Print('o'),
                    Sequence::Execute(0x07),
                    Sequence::Csi(vec![vec![38, 2, 255, 0, 255], vec![1]], vec![], false, 'm',),
                    Sequence::Print('Ж'),
                    Sequence::Print('漢'),
                    Sequence::Print(char::REPLACEMENT_CHARACTER),
                ]
            );
        }
    }
}

// #[cfg(test)]
// mod bench {
//
//     use super::*;
//
//     extern crate test;
//
//     #[derive(Default)]
//     struct NopExecutor {}
//
//     impl Executor for NopExecutor {
//         fn print(&mut self, _c: char) {}
//
//         fn execute(&mut self, _byte: u8) {}
//
//         fn put(&mut self, _byte: u8) {}
//
//         fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
//
//         fn unhook(&mut self) {}
//
//         fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}
//
//         fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
//
//         fn csi_dispatch(
//             &mut self,
//             _params: &Params,
//             _intermediates: &[u8],
//             _ignore: bool,
//             _action: char,
//         ) {
//         }
//     }
//
//     const INPUT: &[u8] = include_bytes!("ansi/test.ansi");
//
//     #[bench]
//     fn advance_batch(b: &mut test::Bencher) {
//         let mut parser = Parser::new();
//         let mut executor = NopExecutor::default();
//
//         b.iter(|| {
//             parser.advance(&mut executor, INPUT);
//         })
//     }
//
//     #[bench]
//     fn advance_sequential(b: &mut test::Bencher) {
//         let mut parser = Parser::new();
//         let mut executor = NopExecutor::default();
//
//         b.iter(|| {
//             for byte in INPUT {
//                 parser.advance(&mut executor, &[*byte]);
//             }
//         })
//     }
//
//     mod utf8 {
//         use super::*;
//
//         // TODO: make inputs of the same size
//
//         static INPUT_NON_ASCII: &[u8] = r#"
//         Лорем ипсум долор сит амет, пер цлита поссит ех, ат мунере фабулас петентиум сит. Иус цу цибо саперет сцрипсерит, нец виси муциус лабитур ид. Ет хис нонумес нолуиссе дигниссим.
//         側経意責家方家閉討店暖育田庁載社転線宇。得君新術治温抗添代話考振投員殴大闘北裁。品間識部案代学凰処済準世一戸刻法分。悼測済諏計飯利安凶断理資沢同岩面文認革。内警格化再薬方久化体教御決数詭芸得筆代。
//         पढाए हिंदी रहारुप अनुवाद कार्यलय मुख्य संस्था सोफ़तवेर निरपेक्ष उनका आपके बाटते आशाआपस मुख्यतह उशकी करता। शुरुआत संस्था कुशलता मेंभटृ अनुवाद गएआप विशेष सकते परिभाषित लाभान्वित प्रति देकर समजते दिशामे प्राप्त जैसे वर्णन संस्थान निर्माता प्रव्रुति भाति चुनने उपलब्ध बेंगलूर अर्थपुर्ण
//         լոռեմ իպսում դոլոռ սիթ ամեթ, լաբոռե մոդեռաթիուս եթ հաս, պեռ ոմնիս լաթինե դիսպութաթիոնի աթ, վիս ֆեուգաիթ ծիվիբուս եխ. վիվենդում լաբոռամուս ելաբոռառեթ նամ ին.
//         국민경제의 발전을 위한 중요정책의 수립에 관하여 대통령의 자문에 응하기 위하여 국민경제자문회의를 둘 수 있다.
//         Λορεμ ιπσθμ δολορ σιτ αμετ, μει ιδ νοvθμ φαβελλασ πετεντιθμ vελ νε, ατ νισλ σονετ οπορτερε εθμ. Αλιι δοcτθσ μει ιδ, νο αθτεμ αθδιρε ιντερεσσετ μελ, δοcενδι cομμθνε οπορτεατ τε cθμ.
//         旅ロ京青利セムレ弱改フヨス波府かばぼ意送でぼ調掲察たス日西重ケアナ住橋ユムミク順待ふかんぼ人奨貯鏡すびそ。
//         غينيا واستمر العصبة ضرب قد. وباءت الأمريكي الأوربيين هو به،, هو العالم، الثقيلة بال. مع وايرلندا الأوروبيّون كان, قد بحق أسابيع العظمى واعتلاء. انه كل وإقامة المواد.
//         כדי יסוד מונחים מועמדים של, דת דפים מאמרשיחהצפה זאת. אתה דת שונה כלשהו, גם אחר ליום בשפות, או ניווט פולנית לחיבור ארץ. ויש בקלות ואמנות אירועים או, אל אינו כלכלה שתי.
//         "#.as_bytes();
//
//         static INPUT_ASCII: &[u8] = r#"
//         Lorem ipsum dolor sit amet, consectetur adipiscing elit. Vestibulum tincidunt venenatis justo eu bibendum. Quisque blandit molestie mattis. Cras porta leo et magna aliquam, in facilisis felis dapibus.
//         Lorem ipsum dolor sit amet, consectetur adipiscing elit. Vestibulum tincidunt venenatis justo eu bibendum. Quisque blandit molestie mattis. Cras porta leo et magna aliquam, in facilisis felis dapibus.
//         Lorem ipsum dolor sit amet, consectetur adipiscing elit. Vestibulum tincidunt venenatis justo eu bibendum. Quisque blandit molestie mattis. Cras porta leo et magna aliquam, in facilisis felis dapibus.
//         Lorem ipsum dolor sit amet, consectetur adipiscing elit. Vestibulum tincidunt venenatis justo eu bibendum. Quisque blandit molestie mattis. Cras porta leo et magna aliquam, in facilisis felis dapibus.
//         Lorem ipsum dolor sit amet, consectetur adipiscing elit. Vestibulum tincidunt venenatis justo eu bibendum. Quisque blandit molestie mattis. Cras porta leo et magna aliquam, in facilisis felis dapibus.
//         Lorem ipsum dolor sit amet, consectetur adipiscing elit. Vestibulum tincidunt venenatis justo eu bibendum. Quisque blandit molestie mattis. Cras porta leo et magna aliquam, in facilisis felis dapibus.
//         Lorem ipsum dolor sit amet, consectetur adipiscing elit. Vestibulum tincidunt venenatis justo eu bibendum. Quisque blandit molestie mattis. Cras porta leo et magna aliquam, in facilisis felis dapibus.
//         Lorem ipsum dolor sit amet, consectetur adipiscing elit. Vestibulum tincidunt venenatis justo eu bibendum. Quisque blandit molestie mattis. Cras porta leo et magna aliquam, in facilisis felis dapibus.
//         Lorem ipsum dolor sit amet, consectetur adipiscing elit. Vestibulum tincidunt venenatis justo eu bibendum. Quisque blandit molestie mattis. Cras porta leo et magna aliquam, in facilisis felis dapibus.
//         "#.as_bytes();
//
//         #[bench]
//         fn non_ascii(b: &mut test::Bencher) {
//             let mut executor = NopExecutor::default();
//             let mut parser = Parser::new();
//
//             b.iter(|| {
//                 parser.advance_utf8(&mut executor, INPUT_NON_ASCII);
//             })
//         }
//
//         #[bench]
//         fn ascii(b: &mut test::Bencher) {
//             let mut executor = NopExecutor::default();
//             let mut parser = Parser::new();
//
//             b.iter(|| {
//                 parser.advance_utf8(&mut executor, INPUT_ASCII);
//             })
//         }
//     }
// }
