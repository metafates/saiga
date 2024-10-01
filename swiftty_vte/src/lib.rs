mod param;
mod table;

use param::{Param, Params};
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

    /// Called when a device control string is terminated.
    ///
    /// The previously selected handler should be notified that the DCS has
    /// terminated.
    fn unhook(&mut self);

    /// Dispatch an operating system command.
    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool);
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
    idx: usize,
}

impl Intermediates {
    pub fn get(&self) -> &[u8] {
        &self.array[..self.idx]
    }

    pub fn is_full(&self) -> bool {
        self.idx == MAX_INTERMEDIATES
    }

    pub fn push(&mut self, byte: u8) {
        self.array[self.idx] = byte;
        self.idx += 1;
    }
}

#[derive(Default)]
pub struct Parser {
    state: State,

    osc_handler: OscHandler,
    params: Params,
    param: Param,

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

        self.exit_state(executor, byte);

        // transition
        if let Some(action) = action {
            self.execute_action(executor, action, byte);
        }

        self.state = state;
        self.entry_state(executor, byte);
    }

    fn entry_state<E: Executor>(&mut self, executor: &mut E, byte: u8) {
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

    fn exit_state<E: Executor>(&mut self, executor: &mut E, byte: u8) {
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
            Execute => executor.execute(byte),
            OscStart => self.osc_handler.start(),
            OscPut => self.osc_handler.put(byte),
            OscEnd => self.osc_handler.end(executor, byte),
            Hook => {
                if self.params.is_full() {
                    self.ignoring = true;
                } else {
                    self.params.push(self.param);
                }

                // TODO: hook executor
            }
            Unhook => executor.unhook(),
            Param => {
                if self.params.is_full() {
                    self.ignoring = true;
                    return;
                }

                if byte == param::SEPARATOR as u8 {
                    self.params.push(self.param);
                    self.param = 0;
                } else {
                    self.param = self.param.saturating_mul(10);
                    self.param = self.param.saturating_add((byte - b'0') as param::Param);
                }
            }
            CsiDispatch => {
                if self.params.is_full() {
                    self.ignoring = true
                } else {
                    self.params.push(self.param);
                }

                // TODO: dispatch executor
            }
            Collect => {
                if self.intermediates.is_full() {
                    self.ignoring = true
                } else {
                    self.intermediates.push(byte);
                }
            }
            Ignore => (),
            _ => (), // TODO
        }
    }
}
