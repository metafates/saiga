use std::{char, str};

use params::Params;
use table::{Action, State};

pub mod params;
mod table;

#[derive(Default)]
pub struct NopPerformer {}

impl Perform for NopPerformer {
    fn print(&mut self, _c: char) {}

    fn execute(&mut self, _byte: u8) {}

    fn put(&mut self, _byte: u8) {}

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}

    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}

    fn csi_dispatch(
        &mut self,
        _params: &Params,
        _intermediates: &[u8],
        _ignore: bool,
        _action: char,
    ) {
    }
}

#[derive(Default)]
pub struct Performer<Inner: Perform = NopPerformer> {
    inner: Inner,
}

impl Perform for Performer {
    #[inline(always)]
    fn print(&mut self, c: char) {
        self.inner.print(c)
    }

    #[inline(always)]
    fn execute(&mut self, byte: u8) {
        self.inner.execute(byte)
    }

    #[inline(always)]
    fn put(&mut self, byte: u8) {
        self.inner.put(byte)
    }

    #[inline(always)]
    fn hook(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        self.inner.hook(params, intermediates, ignore, action)
    }

    #[inline(always)]
    fn unhook(&mut self) {
        self.inner.unhook()
    }

    #[inline(always)]
    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        self.inner.osc_dispatch(params, bell_terminated)
    }

    #[inline(always)]
    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        self.inner.esc_dispatch(intermediates, ignore, byte)
    }

    #[inline(always)]
    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        self.inner
            .csi_dispatch(params, intermediates, ignore, action)
    }
}

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

#[derive(Default)]
pub struct Parser {
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
}

impl Parser {
    pub fn new() -> Self {
        Default::default()
    }

    #[inline]
    pub fn advance(&mut self, performer: &mut Performer, bytes: &[u8]) {
        let mut i = 0;

        // Handle partial codepoints from previous calls to `advance`.
        if self.partial_utf8_len != 0 {
            i += self.advance_partial_utf8(performer, bytes);
        }

        while i != bytes.len() {
            if self.state == State::Ground {
                i += self.advance_ground(performer, &bytes[i..])
            } else {
                let byte = bytes[i];
                self.change_state(performer, byte);
                i += 1;
            }
        }
    }

    /// Advance the parser state from ground.
    ///
    /// The ground state is handled separately since it can only be left using
    /// the escape character (`\x1b`). This allows more efficient parsing by
    /// using SIMD search with [`memchr`].
    #[inline]
    fn advance_ground(&mut self, performer: &mut Performer, bytes: &[u8]) -> usize {
        // Find the next escape character.
        let num_bytes = bytes.len();
        let plain_chars = memchr::memchr(0x1B, bytes).unwrap_or(num_bytes);

        // If the next character is ESC, just process it and short-circuit.
        if plain_chars == 0 {
            self.state = State::Escape;
            self.reset_params();
            return 1;
        }

        match simdutf8::compat::from_utf8(&bytes[..plain_chars]) {
            Ok(parsed) => {
                Self::ground_dispatch(performer, parsed);
                let mut processed = plain_chars;

                // If there's another character, it must be escape so process it directly.
                if processed < num_bytes {
                    self.state = State::Escape;
                    self.reset_params();
                    processed += 1;
                }

                processed
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
                        valid_bytes + len
                    }
                    None => {
                        if plain_chars < num_bytes {
                            // Process bytes cut off by escape.
                            performer.print(char::REPLACEMENT_CHARACTER);
                            self.state = State::Escape;
                            self.reset_params();
                            plain_chars + 1
                        } else {
                            // Process bytes cut off by the buffer end.
                            let extra_bytes = num_bytes - valid_bytes;
                            let partial_len = self.partial_utf8_len + extra_bytes;
                            self.partial_utf8[self.partial_utf8_len..partial_len]
                                .copy_from_slice(&bytes[valid_bytes..valid_bytes + extra_bytes]);
                            self.partial_utf8_len = partial_len;
                            num_bytes
                        }
                    }
                }
            }
        }
    }

    /// Handle ground dispatch of print/execute for all characters in a string.
    #[inline]
    fn ground_dispatch<P: Perform>(performer: &mut P, text: &str) {
        for c in text.chars() {
            match c {
                '\x00'..='\x1f' | '\u{80}'..='\u{9f}' => performer.execute(c as u8),
                _ => performer.print(c),
            }
        }
    }

    /// Reset escape sequence parameters and intermediates.
    #[inline]
    fn reset_params(&mut self) {
        self.intermediate_idx = 0;
        self.ignoring = false;
        self.param = 0;

        self.params.clear();
    }

    #[inline]
    fn change_state(&mut self, performer: &mut Performer, byte: u8) {
        let change = table::change_state(State::Anywhere, byte)
            .or_else(|| table::change_state(self.state, byte));

        let Some((state, action)) = change else {
            return;
        };

        match state {
            State::Anywhere => {
                self.execute_action(performer, action, byte);
            }
            state => {
                self.execute_state_entry_action(performer, byte);

                self.execute_action(performer, action, byte);

                self.state = state;

                self.execute_state_exit_action(performer, byte);
            }
        }
    }

    #[inline(always)]
    fn execute_state_exit_action(&mut self, performer: &mut Performer, byte: u8) {
        let action = table::state_exit_action(self.state);

        self.execute_action(performer, action, byte);
    }

    #[inline(always)]
    fn execute_state_entry_action(&mut self, performer: &mut Performer, byte: u8) {
        let action = table::state_entry_action(self.state);

        self.execute_action(performer, action, byte);
    }

    #[inline(always)]
    fn execute_action(&mut self, performer: &mut Performer, action: Action, byte: u8) {
        ACTIONS[action as usize](self, performer, byte)
    }

    /// Advance the parser while processing a partial utf8 codepoint.
    #[inline]
    fn advance_partial_utf8(&mut self, performer: &mut Performer, bytes: &[u8]) -> usize {
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
                    return valid_bytes - old_bytes;
                }

                match err.error_len() {
                    // If the partial character was also invalid, emit the replacement
                    // character.
                    Some(invalid_len) => {
                        performer.print(char::REPLACEMENT_CHARACTER);

                        self.partial_utf8_len = 0;
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
}

static ACTIONS: [fn(&mut Parser, &mut Performer, u8); 14] = {
    let mut result: [fn(&mut Parser, &mut Performer, u8); 14] = [action_nop; 14];

    result[Action::Print as usize] = action_print;
    result[Action::Put as usize] = action_put;
    result[Action::Execute as usize] = action_execute;
    result[Action::OscStart as usize] = action_osc_start;
    result[Action::OscPut as usize] = action_osc_put;
    result[Action::OscEnd as usize] = action_osc_end;
    result[Action::Hook as usize] = action_hook;
    result[Action::Unhook as usize] = action_unhook;
    result[Action::Param as usize] = action_param;
    result[Action::CsiDispatch as usize] = action_csi_dispatch;
    result[Action::Collect as usize] = action_collect;
    result[Action::EscDispatch as usize] = action_esc_dispatch;
    result[Action::Clear as usize] = action_clear;

    result
};

#[inline(always)]
fn action_nop(_parser: &mut Parser, _performer: &mut Performer, _byte: u8) {}

#[inline(always)]
fn action_print(_parser: &mut Parser, performer: &mut Performer, byte: u8) {
    performer.print(byte as char)
}

#[inline(always)]
fn action_put(_parser: &mut Parser, performer: &mut Performer, byte: u8) {
    performer.put(byte)
}

#[inline(always)]
fn action_execute(_parser: &mut Parser, performer: &mut Performer, byte: u8) {
    performer.execute(byte)
}

#[inline]
fn action_osc_start(parser: &mut Parser, _performer: &mut Performer, _byte: u8) {
    parser.osc_raw.clear();
    parser.osc_num_params = 0;
}

#[inline(always)]
fn action_osc_put(parser: &mut Parser, _performer: &mut Performer, byte: u8) {
    parser.osc_raw.push(byte);
}

#[inline]
fn action_osc_end(parser: &mut Parser, performer: &mut Performer, byte: u8) {
    parser.osc_put_param();
    action_esc_dispatch(parser, performer, byte);
    parser.osc_raw.clear();
    parser.osc_num_params = 0;
}

#[inline]
fn action_hook(parser: &mut Parser, performer: &mut Performer, byte: u8) {
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
fn action_unhook(_parser: &mut Parser, performer: &mut Performer, _byte: u8) {
    performer.unhook()
}

#[inline]
fn action_param(parser: &mut Parser, _performer: &mut Performer, _byte: u8) {
    if parser.params.is_full() {
        parser.ignoring = true;
    } else {
        parser.params.push(parser.param);
        parser.param = 0;
    }
}

#[inline]
fn action_csi_dispatch(parser: &mut Parser, performer: &mut Performer, byte: u8) {
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
fn action_collect(parser: &mut Parser, _performer: &mut Performer, byte: u8) {
    if parser.intermediate_idx == MAX_INTERMEDIATES {
        parser.ignoring = true;
    } else {
        parser.intermediates[parser.intermediate_idx] = byte;
        parser.intermediate_idx += 1;
    }
}

#[inline(always)]
fn action_esc_dispatch(parser: &mut Parser, performer: &mut Performer, byte: u8) {
    performer.esc_dispatch(parser.intermediates(), parser.ignoring, byte);
}

#[inline]
fn action_clear(parser: &mut Parser, _performer: &mut Performer, _byte: u8) {
    parser.param = 0;
    parser.params.clear();

    parser.ignoring = false;

    parser.intermediate_idx = 0;
}
