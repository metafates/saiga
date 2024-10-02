mod param;
mod table;

use param::{Params, Subparam};
use table::{Action, State};

/// X3.64 doesn’t place any limit on the number of intermediate characters allowed before a final character,
/// although it doesn’t define any control sequences with more than one.
/// Digital defined escape sequences with two intermediate characters,
/// and control sequences and device control strings with one.
const MAX_INTERMEDIATES: usize = 2;

/// There is no limit to the number of characters in a parameter string,
/// although a maximum of 16 parameters need be stored.
const MAX_OSC_PARAMS: usize = 16;

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
struct OscHandler {
    params: [(usize, usize); MAX_OSC_PARAMS],
    params_num: usize,
    raw: Vec<u8>,
}

impl OscHandler {
    fn start(&mut self) {
        self.raw.clear();
        self.params_num = 0;
    }

    fn put(&mut self, byte: u8) {
        let idx = self.raw.len();

        if byte != b';' {
            self.raw.push(byte);
            return;
        }

        // handle param separator

        match self.params_num {
            MAX_OSC_PARAMS => return,

            0 => self.params[0] = (0, idx),

            param_idx => {
                let prev = self.params[param_idx - 1];

                self.params[param_idx] = (prev.1, idx)
            }
        }

        self.params_num += 1;
    }

    fn end<E: Executor>(&mut self, executor: &mut E, byte: u8) {
        let idx = self.raw.len();

        match self.params_num {
            MAX_OSC_PARAMS => (),

            0 => {
                self.params[0] = (0, idx);
                self.params_num += 1;
            }

            param_idx => {
                let prev = self.params[param_idx - 1];

                self.params[param_idx] = (prev.1, idx);
                self.params_num += 1;
            }
        }

        self.dispatch(executor, byte);
    }

    fn dispatch<E: Executor>(&self, executor: &mut E, byte: u8) {
        let slices: Vec<&[u8]> = self
            .params
            .iter()
            .map(|(start, end)| &self.raw[*start..*end])
            .collect();

        let params = &slices[..self.params_num];

        executor.osc_dispatch(params, byte == 0x07)
    }
}

#[derive(Default)]
struct Intermediates {
    array: [u8; MAX_INTERMEDIATES],
    index: usize,
}

impl Intermediates {
    pub fn as_slice(&self) -> &[u8] {
        &self.array[..self.index]
    }

    pub fn is_full(&self) -> bool {
        self.index == MAX_INTERMEDIATES
    }

    pub fn push(&mut self, byte: u8) {
        if self.is_full() {
            return;
        }

        self.array[self.index] = byte;
        self.index += 1;
    }

    pub fn clear(&mut self) {
        self.index = 0
    }
}

#[derive(Default)]
pub struct Parser {
    state: State,

    osc_handler: OscHandler,

    params: Params,
    subparam: Subparam,

    intermediates: Intermediates,

    ignoring: bool,
}

impl Parser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn advance<E: Executor>(&mut self, executor: &mut E, byte: u8) {
        //if let State::UTF8 = self.state {
        //    // TODO: process UTF8
        //    return;
        //}

        let change = table::change_state(State::Anywhere, byte)
            .or_else(|| table::change_state(self.state, byte));

        let (state, action) = change.expect("must be known");

        self.state_change(executor, state, action, byte);
    }

    fn state_change<E: Executor>(
        &mut self,
        executor: &mut E,
        state: State,
        action: Option<Action>,
        byte: u8,
    ) {
        // moving to Anywhere means executing current action right away
        if state == State::Anywhere && action.is_some() {
            self.execute_action(executor, action.unwrap(), byte);
            return;
        }

        self.execute_state_exit_action(executor, byte);

        // transition
        if let Some(action) = action {
            self.execute_action(executor, action, byte);
        }

        self.state = state;
        self.execute_state_entry_action(executor, byte);
    }

    fn execute_state_entry_action<E: Executor>(&mut self, executor: &mut E, byte: u8) {
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

    fn execute_state_exit_action<E: Executor>(&mut self, executor: &mut E, byte: u8) {
        match self.state {
            State::DcsPassthrough => {
                self.execute_action(executor, Action::Unhook, byte);
            }
            State::OscString => {
                self.execute_action(executor, Action::OscEnd, byte);
            }
            _ => (),
        }
    }

    fn execute_action<E: Executor>(&mut self, executor: &mut E, action: Action, byte: u8) {
        use Action::*;

        if byte == b';' {
            let _ = 5;
        }

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
                    self.params.push(self.subparam);
                }

                executor.hook(
                    &self.params,
                    self.intermediates.as_slice(),
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
                        self.params.push(self.subparam);
                    }
                    param::SUBPARAM_SEPARATOR => {
                        self.params.extend(self.subparam);
                        self.subparam = Subparam::default();
                    }
                    byte => {
                        self.subparam = self.subparam.saturating_mul(10);
                        self.subparam = self.subparam.saturating_add((byte - b'0').into());
                    }
                }
            }
            CsiDispatch => {
                if self.params.is_full() {
                    self.ignoring = true
                } else {
                    self.params.push(self.subparam);
                }

                executor.csi_dispatch(
                    &self.params,
                    self.intermediates.as_slice(),
                    self.ignoring,
                    byte as char,
                );
            }
            Collect => {
                if self.intermediates.is_full() {
                    self.ignoring = true
                } else {
                    self.intermediates.push(byte);
                }
            }
            EscDispatch => {
                executor.esc_dispatch(self.intermediates.as_slice(), self.ignoring, byte);
            }
            Clear => {
                self.subparam = Subparam::default();
                self.params.clear();

                self.ignoring = false;

                self.intermediates.clear();
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
    }

    impl Executor for Dispatcher {
        fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
            let params = params.iter().map(|p| p.to_vec()).collect();
            self.dispatched.push(Sequence::Osc(params, bell_terminated));
        }

        fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
            let params = params
                .iter()
                .map(|param| param.to_slice().to_vec())
                .collect();
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
            let params = params
                .iter()
                .map(|param| param.to_slice().to_vec())
                .collect();
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

        fn print(&mut self, _c: char) {}

        fn execute(&mut self, _byte: u8) {}
    }

    mod osc {
        use super::*;

        static OSC_BYTES: &[u8] = &[
            0x1b, 0x5d, // Begin OSC
            b'2', b';', b'j', b'w', b'i', b'l', b'm', b'@', b'j', b'w', b'i', b'l', b'm', b'-',
            b'd', b'e', b's', b'k', b':', b' ', b'~', b'/', b'c', b'o', b'd', b'e', b'/', b'a',
            b'l', b'a', b'c', b'r', b'i', b't', b't', b'y', 0x9c, // End OSC
        ];

        #[test]
        fn parse() {
            let mut dispatcher = Dispatcher::default();
            let mut parser = Parser::new();

            for byte in OSC_BYTES {
                parser.advance(&mut dispatcher, *byte);
            }

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

            for byte in &[0x1b, 0x5d, 0x07] {
                parser.advance(&mut dispatcher, *byte);
            }

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

            for byte in input {
                parser.advance(&mut dispatcher, byte);
            }

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
        fn bell_terminated() {
            static INPUT: &[u8] = b"\x1b]11;ff/00/ff\x07";
            let mut dispatcher = Dispatcher::default();
            let mut parser = Parser::new();

            for byte in INPUT {
                parser.advance(&mut dispatcher, *byte);
            }

            assert_eq!(dispatcher.dispatched.len(), 1);
            match &dispatcher.dispatched[0] {
                Sequence::Osc(_, true) => (),
                _ => panic!("expected osc with bell terminator"),
            }
        }
    }
}
