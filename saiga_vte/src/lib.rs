use std::{char, mem::MaybeUninit, str};

use params::Params;
use table::{Action, State};

pub mod params;
mod table;

pub trait Perform {
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

const MAX_INTERMEDIATES: usize = 2;
const MAX_OSC_PARAMS: usize = 16;
const MAX_OSC_RAW: usize = 1024;

pub struct Parser<P: Perform> {
    state: State,

    intermediates: [u8; MAX_INTERMEDIATES],
    intermediate_idx: usize,

    params: Params,
    param: u16,

    osc_raw: Vec<u8>,
    osc_params: [(usize, usize); MAX_OSC_PARAMS],
    osc_num_params: usize,

    ignoring: bool,

    partial_utf8: [u8; 4],
    partial_utf8_len: usize,

    next_step: fn(&mut Self, &mut P, &[u8]) -> usize,
}

impl<P: Perform> Default for Parser<P> {
    fn default() -> Self {
        Self {
            state: Default::default(),
            intermediates: Default::default(),
            intermediate_idx: Default::default(),
            params: Default::default(),
            param: Default::default(),
            osc_raw: Vec::with_capacity(1024),
            osc_params: Default::default(),
            osc_num_params: Default::default(),
            ignoring: Default::default(),
            partial_utf8: Default::default(),
            partial_utf8_len: Default::default(),
            next_step: Self::advance_ground,
        }
    }
}

impl<P: Perform> Parser<P> {
    #[inline]
    fn advance_change_state(parser: &mut Self, performer: &mut P, bytes: &[u8]) -> usize {
        parser.change_state(performer, bytes[0]);
        1
    }

    const ACTIONS: [fn(&mut Self, &mut P, u8); 17] = {
        let mut result: [fn(&mut Self, &mut P, u8); 17] = [Self::action_nop; 17];

        result[Action::Print as usize] = Self::action_print;
        result[Action::Put as usize] = Self::action_put;
        result[Action::Execute as usize] = Self::action_execute;
        result[Action::OscStart as usize] = Self::action_osc_start;
        result[Action::OscPut as usize] = Self::action_osc_put;
        result[Action::OscPutParam as usize] = Self::action_osc_put_param;
        result[Action::OscEnd as usize] = Self::action_osc_end;
        result[Action::Hook as usize] = Self::action_hook;
        result[Action::Unhook as usize] = Self::action_unhook;
        result[Action::Param as usize] = Self::action_param;
        result[Action::NextParam as usize] = Self::action_param_next;
        result[Action::Subparam as usize] = Self::action_subparam;
        result[Action::CsiDispatch as usize] = Self::action_csi_dispatch;
        result[Action::Collect as usize] = Self::action_collect;
        result[Action::EscDispatch as usize] = Self::action_esc_dispatch;
        result[Action::Clear as usize] = Self::action_clear;

        result
    };

    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    #[inline]
    pub fn advance(&mut self, performer: &mut P, bytes: &[u8]) {
        let mut i = 0;

        // Handle partial codepoints from previous calls to `advance`.
        // if self.partial_utf8_len != 0 {
        // i += self.advance_partial_utf8(performer, bytes);
        // }

        while i != bytes.len() {
            println!("{:?} {:?}", self.state, &bytes[i..]);
            i += (self.next_step)(self, performer, &bytes[i..]);

            // if self.state == State::Ground {
            // i += self.advance_ground(performer, &bytes[i..])
            // } else {
            // let byte = bytes[i];
            // self.change_state(performer, byte);
            // i += 1;
            // }
        }
    }

    /// Advance the parser state from ground.
    ///
    /// The ground state is handled separately since it can only be left using
    /// the escape character (`\x1b`). This allows more efficient parsing by
    /// using SIMD search with [`memchr`].
    #[inline]
    fn advance_ground(&mut self, performer: &mut P, bytes: &[u8]) -> usize {
        // Find the next escape character.
        let num_bytes = bytes.len();
        let plain_chars = memchr::memchr(0x1B, bytes).unwrap_or(num_bytes);

        // If the next character is ESC, just process it and short-circuit.
        if plain_chars == 0 {
            // self.state = State::Escape;
            // self.reset_params();
            self.next_step = Self::advance_change_state;
            return 0;
        }

        match simdutf8::compat::from_utf8(&bytes[..plain_chars]) {
            Ok(parsed) => {
                Self::ground_dispatch(performer, parsed);

                // If there's another character, it must be escape so process it directly.
                if plain_chars < num_bytes {
                    // self.state = State::Escape;
                    // self.reset_params();
                    self.next_step = Self::advance_change_state;
                    // plain_chars + 1
                    plain_chars
                } else {
                    plain_chars
                }
            }
            // Handle invalid and partial utf8.
            Err(err) => {
                // Dispatch all the valid bytes.
                let valid_bytes = err.valid_up_to();
                let parsed = unsafe { str::from_utf8_unchecked(&bytes[..valid_bytes]) };

                Self::ground_dispatch(performer, parsed);

                match err.error_len() {
                    Some(len) => {
                        // Execute C1 escapes or emit replacement character.
                        if len == 1 && bytes[valid_bytes] <= 0x9F {
                            performer.execute(bytes[valid_bytes]);
                        } else {
                            performer.print(char::REPLACEMENT_CHARACTER);
                        }

                        // Restart processing after the invalid bytes.
                        //
                        // While we could theoretically try to just re-parse
                        // `bytes[valid_bytes + len..plain_chars]`, it's easier
                        // to just skip it and invalid utf8 is pretty rare anyway.
                        self.next_step = Self::advance_change_state;
                        valid_bytes + len
                    }
                    None => {
                        if plain_chars < num_bytes {
                            // Process bytes cut off by escape.
                            performer.print(char::REPLACEMENT_CHARACTER);
                            // self.state = State::Escape;
                            // self.reset_params();
                            self.next_step = Self::advance_change_state;
                            // plain_chars + 1
                            plain_chars
                        } else {
                            // Process bytes cut off by the buffer end.
                            let extra_bytes = num_bytes - valid_bytes;
                            let partial_len = self.partial_utf8_len + extra_bytes;
                            self.partial_utf8[self.partial_utf8_len..partial_len]
                                .copy_from_slice(&bytes[valid_bytes..valid_bytes + extra_bytes]);
                            self.partial_utf8_len = partial_len;
                            self.next_step = Self::advance_partial_utf8;
                            num_bytes
                        }
                    }
                }
            }
        }
    }

    /// Handle ground dispatch of print/execute for all characters in a string.
    #[inline]
    fn ground_dispatch(performer: &mut P, text: &str) {
        for c in text.chars() {
            match c {
                '\x00'..='\x1f' | '\u{80}'..='\u{9f}' => performer.execute(c as u8),
                _ => performer.print(c),
            }
        }
    }

    #[inline]
    fn change_state(&mut self, performer: &mut P, byte: u8) {
        let change = table::change_state(State::Anywhere, byte)
            .or_else(|| table::change_state(self.state, byte));

        let Some((state, action)) = change else {
            return;
        };

        if state == State::Anywhere {
            self.execute_action(performer, action, byte);
        } else {
            self.execute_state_exit_action(performer, byte);

            self.execute_action(performer, action, byte);

            self.state = state;

            self.execute_state_entry_action(performer, byte);
        }
    }

    #[inline(always)]
    fn execute_state_exit_action(&mut self, performer: &mut P, byte: u8) {
        self.execute_action(performer, table::state_exit_action(self.state), byte);
    }

    #[inline(always)]
    fn execute_state_entry_action(&mut self, performer: &mut P, byte: u8) {
        self.execute_action(performer, table::state_entry_action(self.state), byte);
    }

    #[inline(always)]
    fn execute_action(&mut self, performer: &mut P, action: Action, byte: u8) {
        Self::ACTIONS[action as usize](self, performer, byte)
    }

    /// Advance the parser while processing a partial utf8 codepoint.
    #[inline]
    fn advance_partial_utf8(&mut self, performer: &mut P, bytes: &[u8]) -> usize {
        // Try to copy up to 3 more characters, to ensure the codepoint is complete.
        let old_bytes = self.partial_utf8_len;
        let to_copy = bytes.len().min(self.partial_utf8.len() - old_bytes);

        self.partial_utf8[old_bytes..old_bytes + to_copy].copy_from_slice(&bytes[..to_copy]);
        self.partial_utf8_len += to_copy;

        // Parse the unicode character.
        match simdutf8::compat::from_utf8(&self.partial_utf8[..self.partial_utf8_len]) {
            // If the entire buffer is valid, use the first character and continue parsing.
            Ok(parsed) => {
                let c = unsafe { parsed.chars().next().unwrap_unchecked() };
                performer.print(c);

                self.partial_utf8_len = 0;

                self.next_step = Self::advance_ground;

                c.len_utf8() - old_bytes
            }
            Err(err) => {
                let valid_bytes = err.valid_up_to();
                // If we have any valid bytes, that means we partially copied another
                // utf8 character into `partial_utf8`. Since we only care about the
                // first character, we just ignore the rest.
                if valid_bytes > 0 {
                    let c = unsafe {
                        let parsed = str::from_utf8_unchecked(&self.partial_utf8[..valid_bytes]);
                        parsed.chars().next().unwrap_unchecked()
                    };

                    performer.print(c);

                    self.partial_utf8_len = 0;
                    self.next_step = Self::advance_ground;
                    return valid_bytes - old_bytes;
                }

                match err.error_len() {
                    // If the partial character was also invalid, emit the replacement
                    // character.
                    Some(invalid_len) => {
                        performer.print(char::REPLACEMENT_CHARACTER);

                        self.partial_utf8_len = 0;
                        self.next_step = Self::advance_ground;
                        invalid_len - old_bytes
                    }
                    // If the character still isn't complete, wait for more data.
                    None => to_copy,
                }
            }
        }
    }

    #[inline(always)]
    fn intermediates(&self) -> &[u8] {
        &self.intermediates[..self.intermediate_idx]
    }

    #[inline]
    fn osc_put_param(&mut self) {
        let idx = self.osc_raw.len();

        match self.osc_num_params {
            // First param is special - 0 to current byte index.
            0 => self.osc_params[0] = (0, idx),

            // Only process up to MAX_OSC_PARAMS.
            MAX_OSC_PARAMS => return,

            // All other params depend on previous indexing.
            param_idx => {
                let prev = self.osc_params[param_idx - 1];
                let begin = prev.1;
                self.osc_params[param_idx] = (begin, idx);
            }
        }

        self.osc_num_params += 1;
    }

    #[inline(always)]
    fn action_nop(_parser: &mut Self, _performer: &mut P, _byte: u8) {}

    #[inline(always)]
    fn action_osc_put_param(parser: &mut Self, _performer: &mut P, _byte: u8) {
        parser.osc_put_param()
    }

    #[inline(always)]
    fn action_print(_parser: &mut Self, performer: &mut P, byte: u8) {
        performer.print(byte as char)
    }

    #[inline(always)]
    fn action_put(_parser: &mut Self, performer: &mut P, byte: u8) {
        performer.put(byte)
    }

    #[inline(always)]
    fn action_execute(_parser: &mut Self, performer: &mut P, byte: u8) {
        performer.execute(byte)
    }

    #[inline]
    fn action_osc_start(parser: &mut Self, _performer: &mut P, _byte: u8) {
        parser.osc_raw.clear();
        parser.osc_num_params = 0;
    }

    #[inline(always)]
    fn action_osc_put(parser: &mut Self, _performer: &mut P, byte: u8) {
        parser.osc_raw.push(byte);
    }

    #[inline]
    fn action_osc_end(parser: &mut Self, performer: &mut P, byte: u8) {
        parser.osc_put_param();
        Self::action_osc_dispatch(parser, performer, byte);
        parser.osc_raw.clear();
        parser.osc_num_params = 0;
    }

    #[inline]
    fn action_osc_dispatch(parser: &mut Self, performer: &mut P, byte: u8) {
        let mut slices: [MaybeUninit<&[u8]>; MAX_OSC_PARAMS] =
            unsafe { MaybeUninit::uninit().assume_init() };

        for (i, slice) in slices.iter_mut().enumerate().take(parser.osc_num_params) {
            let indices = parser.osc_params[i];
            *slice = MaybeUninit::new(&parser.osc_raw[indices.0..indices.1]);
        }

        unsafe {
            let num_params = parser.osc_num_params;
            let params = &slices[..num_params] as *const [MaybeUninit<&[u8]>] as *const [&[u8]];
            performer.osc_dispatch(&*params, byte == 0x07);
        }
    }

    #[inline]
    fn action_hook(parser: &mut Self, performer: &mut P, byte: u8) {
        if parser.params.is_full() {
            parser.ignoring = true;
        } else {
            parser.params.push(parser.param);
        }

        performer.hook(
            &parser.params,
            parser.intermediates(),
            parser.ignoring,
            byte as char,
        );
    }

    #[inline(always)]
    fn action_unhook(_parser: &mut Self, performer: &mut P, _byte: u8) {
        performer.unhook()
    }

    /// Advance to the next parameter.
    #[inline]
    fn action_param(parser: &mut Self, _performer: &mut P, _byte: u8) {
        if parser.params.is_full() {
            parser.ignoring = true;
        } else {
            parser.params.push(parser.param);
            parser.param = 0;
        }
    }

    /// Advance inside the parameter without terminating it.
    #[inline]
    fn action_param_next(parser: &mut Self, _performer: &mut P, byte: u8) {
        if parser.params.is_full() {
            parser.ignoring = true;
        } else {
            // Continue collecting bytes into param.
            parser.param = parser.param.saturating_mul(10);
            parser.param = parser.param.saturating_add((byte - b'0') as u16);
        }
    }

    /// Advance to the next subparameter.
    #[inline]
    fn action_subparam(parser: &mut Self, _performer: &mut P, _byte: u8) {
        if parser.params.is_full() {
            parser.ignoring = true;
        } else {
            parser.params.extend(parser.param);
            parser.param = 0;
        }
    }

    #[inline]
    fn action_csi_dispatch(parser: &mut Self, performer: &mut P, byte: u8) {
        if parser.params.is_full() {
            parser.ignoring = true;
        } else {
            parser.params.push(parser.param);
        }

        performer.csi_dispatch(
            &parser.params,
            parser.intermediates(),
            parser.ignoring,
            byte as char,
        );
    }

    #[inline]
    fn action_collect(parser: &mut Self, _performer: &mut P, byte: u8) {
        if parser.intermediate_idx == MAX_INTERMEDIATES {
            parser.ignoring = true;
        } else {
            parser.intermediates[parser.intermediate_idx] = byte;
            parser.intermediate_idx += 1;
        }
    }

    #[inline(always)]
    fn action_esc_dispatch(parser: &mut Self, performer: &mut P, byte: u8) {
        performer.esc_dispatch(parser.intermediates(), parser.ignoring, byte);
    }

    #[inline]
    fn action_clear(parser: &mut Self, _performer: &mut P, _byte: u8) {
        parser.param = 0;
        parser.ignoring = false;
        parser.intermediate_idx = 0;

        parser.params.clear();
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
        Print(char),
        Execute(u8),
        DcsUnhook,
    }

    impl Perform for Dispatcher {
        fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
            let params = params.iter().map(|p| p.to_vec()).collect();
            self.dispatched.push(Sequence::Osc(params, bell_terminated));
        }

        fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
            let params = params.iter().map(|subparam| subparam.to_vec()).collect();
            let intermediates = intermediates.to_vec();
            self.dispatched
                .push(Sequence::Csi(params, intermediates, ignore, c));
        }

        fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
            let intermediates = intermediates.to_vec();
            self.dispatched
                .push(Sequence::Esc(intermediates, ignore, byte));
        }

        fn hook(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
            let params = params.iter().map(|subparam| subparam.to_vec()).collect();
            let intermediates = intermediates.to_vec();
            self.dispatched
                .push(Sequence::DcsHook(params, intermediates, ignore, c));
        }

        fn put(&mut self, byte: u8) {
            self.dispatched.push(Sequence::DcsPut(byte));
        }

        fn unhook(&mut self) {
            self.dispatched.push(Sequence::DcsUnhook);
        }

        fn print(&mut self, c: char) {
            self.dispatched.push(Sequence::Print(c));
        }

        fn execute(&mut self, byte: u8) {
            self.dispatched.push(Sequence::Execute(byte));
        }
    }

    #[test]
    fn parse_empty_osc() {
        let mut dispatcher = Dispatcher::default();
        let mut parser = Parser::new();

        parser.advance(&mut dispatcher, &[0x1B, 0x5D, 0x07]);

        assert_eq!(dispatcher.dispatched.len(), 1);
        match &dispatcher.dispatched[0] {
            Sequence::Osc(..) => (),
            _ => panic!("expected osc sequence, got {:?}", dispatcher.dispatched),
        }
    }

    #[test]
    fn parse_osc_max_params() {
        let params = ";".repeat(params::MAX_PARAMS + 1);
        let input = format!("\x1b]{}\x1b", &params[..]).into_bytes();
        let mut dispatcher = Dispatcher::default();
        let mut parser = Parser::new();
        // let mut performer = Performer

        parser.advance(&mut dispatcher, &input);

        assert_eq!(dispatcher.dispatched.len(), 1);
        match &dispatcher.dispatched[0] {
            Sequence::Osc(params, _) => {
                assert_eq!(params.len(), MAX_OSC_PARAMS);
                assert!(params.iter().all(Vec::is_empty));
            }
            _ => panic!("expected osc sequence, got {:?}", dispatcher.dispatched),
        }
    }

    #[test]
    fn osc_bell_terminated() {
        const INPUT: &[u8] = b"\x1b]11;ff/00/ff\x07";
        let mut dispatcher = Dispatcher::default();
        let mut parser = Parser::new();

        parser.advance(&mut dispatcher, INPUT);

        assert_eq!(dispatcher.dispatched.len(), 1);
        match &dispatcher.dispatched[0] {
            Sequence::Osc(_, true) => (),
            _ => panic!(
                "expected osc with bell terminator, got {:?}",
                dispatcher.dispatched
            ),
        }
    }

    #[test]
    fn osc_c0_st_terminated() {
        const INPUT: &[u8] = b"\x1b]11;ff/00/ff\x1b\\";
        let mut dispatcher = Dispatcher::default();
        let mut parser = Parser::new();

        parser.advance(&mut dispatcher, INPUT);

        assert_eq!(dispatcher.dispatched.len(), 2);
        match &dispatcher.dispatched[0] {
            Sequence::Osc(_, false) => (),
            _ => panic!("expected osc with ST terminator"),
        }
    }

    #[test]
    fn parse_osc_with_utf8_arguments() {
        const INPUT: &[u8] = &[
            0x0D, 0x1B, 0x5D, 0x32, 0x3B, 0x65, 0x63, 0x68, 0x6F, 0x20, 0x27, 0xC2, 0xAF, 0x5C,
            0x5F, 0x28, 0xE3, 0x83, 0x84, 0x29, 0x5F, 0x2F, 0xC2, 0xAF, 0x27, 0x20, 0x26, 0x26,
            0x20, 0x73, 0x6C, 0x65, 0x65, 0x70, 0x20, 0x31, 0x07,
        ];
        let mut dispatcher = Dispatcher::default();
        let mut parser = Parser::new();

        parser.advance(&mut dispatcher, INPUT);

        assert_eq!(dispatcher.dispatched[0], Sequence::Execute(b'\r'));
        let osc_data = INPUT[5..(INPUT.len() - 1)].into();
        assert_eq!(
            dispatcher.dispatched[1],
            Sequence::Osc(vec![vec![b'2'], osc_data], true)
        );
        assert_eq!(dispatcher.dispatched.len(), 2);
    }

    #[test]
    fn osc_containing_string_terminator() {
        const INPUT: &[u8] = b"\x1b]2;\xe6\x9c\xab\x1b\\";
        let mut dispatcher = Dispatcher::default();
        let mut parser = Parser::new();

        parser.advance(&mut dispatcher, INPUT);

        assert_eq!(dispatcher.dispatched.len(), 2);
        match &dispatcher.dispatched[0] {
            Sequence::Osc(params, _) => {
                assert_eq!(params[1], &INPUT[4..(INPUT.len() - 2)]);
            }
            _ => panic!("expected osc sequence"),
        }
    }

    #[test]
    fn exceed_max_buffer_size() {
        const NUM_BYTES: usize = MAX_OSC_RAW + 100;
        const INPUT_START: &[u8] = b"\x1b]52;s";
        const INPUT_END: &[u8] = b"\x07";

        let mut dispatcher = Dispatcher::default();
        let mut parser = Parser::new();

        // Create valid OSC escape
        parser.advance(&mut dispatcher, INPUT_START);

        // Exceed max buffer size
        parser.advance(&mut dispatcher, &[b'a'; NUM_BYTES]);

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
    fn parse_csi_max_params() {
        // This will build a list of repeating '1;'s
        // The length is MAX_PARAMS - 1 because the last semicolon is interpreted
        // as an implicit zero, making the total number of parameters MAX_PARAMS
        let params = "1;".repeat(params::MAX_PARAMS - 1);
        let input = format!("\x1b[{}p", &params[..]).into_bytes();

        let mut dispatcher = Dispatcher::default();
        let mut parser = Parser::new();

        parser.advance(&mut dispatcher, &input);

        assert_eq!(dispatcher.dispatched.len(), 1);
        match &dispatcher.dispatched[0] {
            Sequence::Csi(params, _, ignore, _) => {
                assert_eq!(params.len(), params::MAX_PARAMS);
                assert!(!ignore);
            }
            _ => panic!("expected csi sequence"),
        }
    }

    #[test]
    fn parse_csi_params_ignore_long_params() {
        // This will build a list of repeating '1;'s
        // The length is MAX_PARAMS because the last semicolon is interpreted
        // as an implicit zero, making the total number of parameters MAX_PARAMS + 1
        let params = "1;".repeat(params::MAX_PARAMS);
        let input = format!("\x1b[{}p", &params[..]).into_bytes();

        let mut dispatcher = Dispatcher::default();
        let mut parser = Parser::new();

        parser.advance(&mut dispatcher, &input);

        assert_eq!(dispatcher.dispatched.len(), 1);
        match &dispatcher.dispatched[0] {
            Sequence::Csi(params, _, ignore, _) => {
                assert_eq!(params.len(), params::MAX_PARAMS);
                assert!(ignore);
            }
            _ => panic!("expected csi sequence"),
        }
    }

    #[test]
    fn parse_csi_params_trailing_semicolon() {
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
    fn parse_csi_params_leading_semicolon() {
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
    fn parse_long_csi_param() {
        // The important part is the parameter, which is (i64::MAX + 1)
        const INPUT: &[u8] = b"\x1b[9223372036854775808m";
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
    fn csi_reset() {
        const INPUT: &[u8] = b"\x1b[3;1\x1b[?1049h";
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
    fn csi_subparameters() {
        const INPUT: &[u8] = b"\x1b[38:2:255:0:255;1m";
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
    fn parse_dcs_max_params() {
        let params = "1;".repeat(params::MAX_PARAMS + 1);
        let input = format!("\x1bP{}p", &params[..]).into_bytes();
        let mut dispatcher = Dispatcher::default();
        let mut parser = Parser::new();

        parser.advance(&mut dispatcher, &input);

        assert_eq!(dispatcher.dispatched.len(), 1);
        match &dispatcher.dispatched[0] {
            Sequence::DcsHook(params, _, ignore, _) => {
                assert_eq!(params.len(), params::MAX_PARAMS);
                assert!(params.iter().all(|param| param == &[1]));
                assert!(ignore);
            }
            _ => panic!("expected dcs sequence"),
        }
    }

    #[test]
    fn dcs_reset() {
        const INPUT: &[u8] = b"\x1b[3;1\x1bP1$tx\x9c";
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
    fn parse_dcs() {
        const INPUT: &[u8] = b"\x1bP0;1|17/ab\x9c";
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
    fn intermediate_reset_on_dcs_exit() {
        const INPUT: &[u8] = b"\x1bP=1sZZZ\x1b+\x5c";
        let mut dispatcher = Dispatcher::default();
        let mut parser = Parser::new();

        parser.advance(&mut dispatcher, INPUT);

        assert_eq!(dispatcher.dispatched.len(), 6);
        match &dispatcher.dispatched[5] {
            Sequence::Esc(intermediates, ..) => assert_eq!(intermediates, b"+"),
            _ => panic!("expected esc sequence"),
        }
    }

    #[test]
    fn esc_reset() {
        const INPUT: &[u8] = b"\x1b[3;1\x1b(A";
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

    #[test]
    fn esc_reset_intermediates() {
        const INPUT: &[u8] = b"\x1b[?2004l\x1b#8";
        let mut dispatcher = Dispatcher::default();
        let mut parser = Parser::new();

        parser.advance(&mut dispatcher, INPUT);

        assert_eq!(dispatcher.dispatched.len(), 2);
        assert_eq!(
            dispatcher.dispatched[0],
            Sequence::Csi(vec![vec![2004]], vec![63], false, 'l')
        );
        assert_eq!(dispatcher.dispatched[1], Sequence::Esc(vec![35], false, 56));
    }

    #[test]
    fn params_buffer_filled_with_subparam() {
        const INPUT: &[u8] = b"\x1b[::::::::::::::::::::::::::::::::x\x1b";
        let mut dispatcher = Dispatcher::default();
        let mut parser = Parser::new();

        parser.advance(&mut dispatcher, INPUT);

        assert_eq!(dispatcher.dispatched.len(), 1);
        match &dispatcher.dispatched[0] {
            Sequence::Csi(params, intermediates, ignore, c) => {
                assert_eq!(intermediates, &[]);
                assert_eq!(params, &[[0; 32]]);
                assert_eq!(c, &'x');
                assert!(ignore);
            }
            _ => panic!("expected csi sequence"),
        }
    }

    #[test]
    fn unicode() {
        const INPUT: &[u8] = b"\xF0\x9F\x8E\x89_\xF0\x9F\xA6\x80\xF0\x9F\xA6\x80_\xF0\x9F\x8E\x89";

        let mut dispatcher = Dispatcher::default();
        let mut parser = Parser::new();

        parser.advance(&mut dispatcher, INPUT);

        assert_eq!(dispatcher.dispatched.len(), 6);
        assert_eq!(dispatcher.dispatched[0], Sequence::Print('🎉'));
        assert_eq!(dispatcher.dispatched[1], Sequence::Print('_'));
        assert_eq!(dispatcher.dispatched[2], Sequence::Print('🦀'));
        assert_eq!(dispatcher.dispatched[3], Sequence::Print('🦀'));
        assert_eq!(dispatcher.dispatched[4], Sequence::Print('_'));
        assert_eq!(dispatcher.dispatched[5], Sequence::Print('🎉'));
    }

    #[test]
    fn invalid_utf8() {
        const INPUT: &[u8] = b"a\xEF\xBCb";

        let mut dispatcher = Dispatcher::default();
        let mut parser = Parser::new();

        parser.advance(&mut dispatcher, INPUT);

        assert_eq!(dispatcher.dispatched.len(), 3);
        assert_eq!(dispatcher.dispatched[0], Sequence::Print('a'));
        assert_eq!(dispatcher.dispatched[1], Sequence::Print('�'));
        assert_eq!(dispatcher.dispatched[2], Sequence::Print('b'));
    }

    #[test]
    fn partial_utf8() {
        const INPUT: &[u8] = b"\xF0\x9F\x9A\x80";

        let mut dispatcher = Dispatcher::default();
        let mut parser = Parser::new();

        parser.advance(&mut dispatcher, &INPUT[..1]);
        parser.advance(&mut dispatcher, &INPUT[1..2]);
        parser.advance(&mut dispatcher, &INPUT[2..3]);
        parser.advance(&mut dispatcher, &INPUT[3..]);

        assert_eq!(dispatcher.dispatched.len(), 1);
        assert_eq!(dispatcher.dispatched[0], Sequence::Print('🚀'));
    }

    #[test]
    fn partial_utf8_separating_utf8() {
        // This is different from the `partial_utf8` test since it has a multi-byte UTF8
        // character after the partial UTF8 state, causing a partial byte to be present
        // in the `partial_utf8` buffer after the 2-byte codepoint.

        // "ĸ🎉"
        const INPUT: &[u8] = b"\xC4\xB8\xF0\x9F\x8E\x89";

        let mut dispatcher = Dispatcher::default();
        let mut parser = Parser::new();

        parser.advance(&mut dispatcher, &INPUT[..1]);
        parser.advance(&mut dispatcher, &INPUT[1..]);

        assert_eq!(dispatcher.dispatched.len(), 2);
        assert_eq!(dispatcher.dispatched[0], Sequence::Print('ĸ'));
        assert_eq!(dispatcher.dispatched[1], Sequence::Print('🎉'));
    }

    #[test]
    fn partial_invalid_utf8() {
        const INPUT: &[u8] = b"a\xEF\xBCb";

        let mut dispatcher = Dispatcher::default();
        let mut parser = Parser::new();

        parser.advance(&mut dispatcher, &INPUT[..1]);
        parser.advance(&mut dispatcher, &INPUT[1..2]);
        parser.advance(&mut dispatcher, &INPUT[2..3]);
        parser.advance(&mut dispatcher, &INPUT[3..]);

        assert_eq!(dispatcher.dispatched.len(), 3);
        assert_eq!(dispatcher.dispatched[0], Sequence::Print('a'));
        assert_eq!(dispatcher.dispatched[1], Sequence::Print('�'));
        assert_eq!(dispatcher.dispatched[2], Sequence::Print('b'));
    }

    #[test]
    fn partial_invalid_utf8_split() {
        const INPUT: &[u8] = b"\xE4\xBF\x99\xB5";

        let mut dispatcher = Dispatcher::default();
        let mut parser = Parser::new();

        parser.advance(&mut dispatcher, &INPUT[..2]);
        parser.advance(&mut dispatcher, &INPUT[2..]);

        assert_eq!(dispatcher.dispatched[0], Sequence::Print('俙'));
        assert_eq!(dispatcher.dispatched[1], Sequence::Print('�'));
    }

    #[test]
    fn partial_utf8_into_esc() {
        const INPUT: &[u8] = b"\xD8\x1b012";

        let mut dispatcher = Dispatcher::default();
        let mut parser = Parser::new();

        parser.advance(&mut dispatcher, INPUT);

        assert_eq!(dispatcher.dispatched.len(), 4);
        assert_eq!(dispatcher.dispatched[0], Sequence::Print('�'));
        assert_eq!(
            dispatcher.dispatched[1],
            Sequence::Esc(Vec::new(), false, b'0')
        );
        assert_eq!(dispatcher.dispatched[2], Sequence::Print('1'));
        assert_eq!(dispatcher.dispatched[3], Sequence::Print('2'));
    }

    #[test]
    fn c1s() {
        const INPUT: &[u8] = b"\x00\x1f\x80\x90\x98\x9b\x9c\x9d\x9e\x9fa";

        let mut dispatcher = Dispatcher::default();
        let mut parser = Parser::new();

        parser.advance(&mut dispatcher, INPUT);

        assert_eq!(
            dispatcher.dispatched.len(),
            11,
            "{:?}",
            dispatcher.dispatched
        );
        assert_eq!(dispatcher.dispatched[0], Sequence::Execute(0));
        assert_eq!(dispatcher.dispatched[1], Sequence::Execute(31));
        assert_eq!(dispatcher.dispatched[2], Sequence::Execute(128));
        assert_eq!(dispatcher.dispatched[3], Sequence::Execute(144));
        assert_eq!(dispatcher.dispatched[4], Sequence::Execute(152));
        assert_eq!(dispatcher.dispatched[5], Sequence::Execute(155));
        assert_eq!(dispatcher.dispatched[6], Sequence::Execute(156));
        assert_eq!(dispatcher.dispatched[7], Sequence::Execute(157));
        assert_eq!(dispatcher.dispatched[8], Sequence::Execute(158));
        assert_eq!(dispatcher.dispatched[9], Sequence::Execute(159));
        assert_eq!(dispatcher.dispatched[10], Sequence::Print('a'));
    }

    #[test]
    fn execute_anywhere() {
        const INPUT: &[u8] = b"\x18\x1a";

        let mut dispatcher = Dispatcher::default();
        let mut parser = Parser::new();

        parser.advance(&mut dispatcher, INPUT);

        assert_eq!(dispatcher.dispatched.len(), 2);
        assert_eq!(dispatcher.dispatched[0], Sequence::Execute(0x18));
        assert_eq!(dispatcher.dispatched[1], Sequence::Execute(0x1A));
    }
}
