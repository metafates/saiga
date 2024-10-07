mod executor;
mod intermediate;
mod osc;
mod param;
mod table;

use executor::Executor;
use osc::MAX_OSC_PARAMS;
use param::{Params, Subparam};
use table::{Action, State};

#[derive(Default)]
pub struct Parser {
    state: State,

    osc_handler: osc::Handler,

    params: Params,
    subparam: Subparam,

    intermediate_handler: intermediate::Handler,

    ignoring: bool,
}

impl Parser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn advance<E: Executor>(&mut self, executor: &mut E, byte: u8) {
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
                }
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
        fn exceed_max_buffer_size() {
            static NUM_BYTES: usize = MAX_OSC_PARAMS + 100;
            static INPUT_START: &[u8] = &[0x1b, b']', b'5', b'2', b';', b's'];
            static INPUT_END: &[u8] = &[b'\x07'];

            let mut dispatcher = Dispatcher::default();
            let mut parser = Parser::new();

            // Create valid OSC escape
            for byte in INPUT_START {
                parser.advance(&mut dispatcher, *byte);
            }

            // Exceed max buffer size
            for _ in 0..NUM_BYTES {
                parser.advance(&mut dispatcher, b'a');
            }

            // Terminate escape for dispatch
            for byte in INPUT_END {
                parser.advance(&mut dispatcher, *byte);
            }

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

            for byte in input {
                parser.advance(&mut dispatcher, byte);
            }

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

            for byte in input {
                parser.advance(&mut dispatcher, byte);
            }

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

            for byte in b"\x1b[4;m" {
                parser.advance(&mut dispatcher, *byte);
            }

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

            for byte in b"\x1b[;4m" {
                parser.advance(&mut dispatcher, *byte);
            }

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

            for byte in INPUT {
                parser.advance(&mut dispatcher, *byte);
            }

            assert_eq!(dispatcher.dispatched.len(), 1);
            match &dispatcher.dispatched[0] {
                Sequence::Csi(params, ..) => assert_eq!(params, &[[std::u16::MAX as u16]]),
                _ => panic!("expected csi sequence"),
            }
        }

        #[test]
        fn reset() {
            static INPUT: &[u8] = b"\x1b[3;1\x1b[?1049h";

            let mut dispatcher = Dispatcher::default();
            let mut parser = Parser::new();

            for byte in INPUT {
                parser.advance(&mut dispatcher, *byte);
            }

            assert_eq!(dispatcher.dispatched.len(), 1);
            match &dispatcher.dispatched[0] {
                Sequence::Csi(params, intermediates, ignore, _) => {
                    assert_eq!(intermediates, &[b'?']);
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

            for byte in INPUT {
                parser.advance(&mut dispatcher, *byte);
            }

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

            for byte in INPUT {
                parser.advance(&mut dispatcher, *byte);
            }

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

            for byte in input {
                parser.advance(&mut dispatcher, byte);
            }

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

            for byte in INPUT {
                parser.advance(&mut dispatcher, *byte);
            }

            assert_eq!(dispatcher.dispatched.len(), 3);

            match &dispatcher.dispatched[0] {
                Sequence::DcsHook(params, intermediates, ignore, _) => {
                    assert_eq!(intermediates, &[b'$']);
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

            for byte in INPUT {
                parser.advance(&mut dispatcher, *byte);
            }

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

            for byte in INPUT {
                parser.advance(&mut dispatcher, *byte);
            }

            assert_eq!(dispatcher.dispatched.len(), 6);
            match &dispatcher.dispatched[5] {
                Sequence::Esc(intermediates, ..) => assert_eq!(intermediates, &[b'+']),
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

            for byte in INPUT {
                parser.advance(&mut dispatcher, *byte);
            }

            assert_eq!(dispatcher.dispatched.len(), 1);
            match &dispatcher.dispatched[0] {
                Sequence::Esc(intermediates, ignore, byte) => {
                    assert_eq!(intermediates, &[b'(']);
                    assert_eq!(*byte, b'A');
                    assert!(!ignore);
                }
                _ => panic!("expected esc sequence"),
            }
        }
    }
}
