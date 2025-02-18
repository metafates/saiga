#[allow(dead_code)]
#[repr(u8)]
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    #[default]
    Ground,

    Escape,
    EscapeIntermediate,

    CsiEntry,
    CsiParam,
    CsiIntermediate,
    CsiIgnore,

    DcsEntry,
    DcsParam,
    DcsIntermediate,
    DcsPassthrough,
    DcsIgnore,

    OscString,

    // ignored
    SosPmApcString,

    Anywhere,
}

impl State {
    const fn from_byte(byte: u8) -> Self {
        use State::*;

        match byte {
            0 => Ground,
            1 => Escape,
            2 => EscapeIntermediate,
            3 => CsiEntry,
            4 => CsiParam,
            5 => CsiIntermediate,
            6 => CsiIgnore,
            7 => DcsEntry,
            8 => DcsParam,
            9 => DcsIntermediate,
            10 => DcsPassthrough,
            11 => DcsIgnore,
            12 => OscString,
            13 => SosPmApcString,
            _ => Anywhere,
        }
    }
}

/// An event may cause one of these actions to occur with or without a change of state.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// This action causes the current private flag,
    /// intermediate characters, final character and parameters to be forgotten.
    Clear,

    /// The private marker or intermediate character should be stored for
    /// later use in selecting a control function to be executed when a final character arrives.
    Collect,

    /// A final character has arrived, so determine the control function to be executed
    /// from private marker, intermediate character(s) and final character,
    /// and execute it, passing in the parameter list.
    CsiDispatch,

    /// The final character of an escape sequence has arrived, so determined
    /// the control function to be executed from the intermediate character(s) and
    /// final character, and execute it.
    EscDispatch,

    /// The C0 or C1 control function should be executed
    Execute,

    /// This action is invoked when a final character arrives in the first part
    /// of a device control string. It determines the control function from the private marker,
    /// intermediate character(s) and final character, and executes it,
    /// passing in the parameter list. It also selects a handler function for the
    /// rest of the characters in the control string.
    ///
    /// This handler function will be called by the put action for every character in the
    /// control string as it arrives.
    Hook,

    /// The character or control is not processed.
    Ignore,

    /// This action is called when the OSC string is terminated by ST, CAN, SUB or ESC,
    /// to allow the OSC handler to finish neatly.
    OscEnd,

    /// This action passes characters from the control string to the OSC Handler as they arrive.
    /// There is therefore no need to buffer characters until the end of the control string is recognised.
    OscPut,

    /// When the control function OSC is recognised, this action initializes an external parser
    /// (the “OSC Handler”) to handle the characters from the control string.
    ///
    /// OSC control strings are not structured in the same way as device control strings,
    /// so there is no choice of parsers.
    OscStart,

    /// This action collects the characters of a parameter string for a control
    /// sequence or device control sequence and builds a list of parameters.
    Param,

    /// The current code should be mapped to a glyph according to the
    /// character set mappings and shift states in effect, and that glyph should be displayed.
    Print,

    /// This action passes characters from the data string part of a device control string
    /// to a handler that has previously been selected by the Hook action.
    Put,

    /// When a device control string is terminated by ST, CAN, SUB or ESC,
    /// this action calls the previously selected handler function with an “end of data” parameter.
    ///
    /// This allows the handler to finish neatly.
    Unhook,
}

static TABLE: [[Option<(State, Action)>; 256]; 15] = {
    let mut table = [[None; 256]; 15];

    let mut byte: u8 = 0;

    while byte != u8::MAX {
        let mut state_byte: u8 = 0;

        while state_byte != 15 {
            let state = State::from_byte(state_byte);

            table[state as usize][byte as usize] = change_state_raw(state, byte);

            state_byte += 1;
        }

        byte += 1;
    }

    table
};

#[inline(always)]
pub const fn change_state(state: State, byte: u8) -> Option<(State, Action)> {
    TABLE[state as usize][byte as usize]
}

#[inline]
pub const fn state_exit_action(state: State) -> Action {
    const LEN: usize = State::Anywhere as usize + 1;
    static ACTIONS: [Action; LEN] = {
        let mut result = [Action::Ignore; LEN];

        result[State::DcsPassthrough as usize] = Action::Unhook;
        result[State::OscString as usize] = Action::OscEnd;

        result
    };

    ACTIONS[state as usize]
}

#[inline]
pub const fn state_entry_action(state: State) -> Action {
    const LEN: usize = State::Anywhere as usize + 1;
    static ACTIONS: [Action; LEN] = {
        let mut result = [Action::Ignore; LEN];

        result[State::CsiEntry as usize] = Action::Clear;
        result[State::DcsEntry as usize] = Action::Clear;
        result[State::Escape as usize] = Action::Clear;
        result[State::OscString as usize] = Action::OscStart;
        result[State::DcsPassthrough as usize] = Action::Hook;

        result
    };

    ACTIONS[state as usize]
}

// Based on https://vt100.net/emu/dec_ansi_parser
const fn change_state_raw(state: State, byte: u8) -> Option<(State, Action)> {
    use Action::*;
    use State::*;

    match state {
        Anywhere => match byte {
            0x18 | 0x1A | 0x80..=0x8F | 0x91..=0x97 | 0x99 | 0x9A => Some((Ground, Execute)),
            0x1B => Some((Escape, Ignore)),
            0x9C => Some((Ground, Ignore)),
            0x98 | 0x9E | 0x9F => Some((SosPmApcString, Ignore)),
            0x90 => Some((DcsEntry, Ignore)),
            0x9D => Some((OscString, Ignore)),
            0x9B => Some((CsiEntry, Ignore)),

            _ => None,
        },

        Ground => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => Some((Anywhere, Execute)),
            0x20..=0x7F => Some((Anywhere, Print)),

            _ => None,
        },

        Escape => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => Some((Anywhere, Execute)),
            0x7F => Some((Anywhere, Ignore)),

            0x5D => Some((OscString, Ignore)),
            0x50 => Some((DcsEntry, Ignore)),
            0x5B => Some((CsiEntry, Ignore)),
            0x58 | 0x5E | 0x5F => Some((SosPmApcString, Ignore)),
            0x20..=0x2F => Some((EscapeIntermediate, Collect)),
            0x30..=0x4F | 0x51..=0x57 | 0x59 | 0x5A | 0x5C | 0x60..=0x7E => {
                Some((Ground, EscDispatch))
            }

            _ => None,
        },

        EscapeIntermediate => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => Some((Anywhere, Execute)),
            0x20..=0x2F => Some((Anywhere, Collect)),
            0x7F => Some((Anywhere, Ignore)),
            0x30..=0x7E => Some((Ground, EscDispatch)),

            _ => None,
        },

        CsiEntry => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => Some((Anywhere, Execute)),
            0x7F => Some((Anywhere, Ignore)),
            0x20..=0x2F => Some((CsiIntermediate, Collect)),
            0x40..=0x7E => Some((Ground, CsiDispatch)),

            // 0x3A ':' (colon) should result CsiIgnore state according to the parser
            // specification. However, this parser implements subparameters separated by colon
            0x30..=0x3B => Some((CsiParam, Param)),
            0x3C..=0x3F => Some((CsiParam, Collect)),

            _ => None,
        },

        CsiParam => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => Some((Anywhere, Execute)),

            // 0x3A ':' (colon) should result CsiIgnore state according to the parser
            // specification. However, our parser implements subparameters separated by colon
            0x30..=0x3B => Some((Anywhere, Param)),

            0x7F => Some((Anywhere, Ignore)),

            0x3C..=0x3F => Some((CsiIgnore, Ignore)),
            0x20..=0x2F => Some((CsiIntermediate, Collect)),
            0x40..=0x7E => Some((Ground, CsiDispatch)),

            _ => None,
        },

        CsiIntermediate => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => Some((Anywhere, Execute)),
            0x20..=0x2F => Some((Anywhere, Collect)),
            0x7F => Some((Anywhere, Ignore)),
            0x30..=0x3F => Some((CsiIgnore, Ignore)),
            0x40..=0x7E => Some((Ground, CsiDispatch)),

            _ => None,
        },

        CsiIgnore => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => Some((Anywhere, Execute)),
            0x20..=0x3F | 0x7F => Some((Anywhere, Ignore)),
            0x40..=0x7E => Some((Ground, Ignore)),

            _ => None,
        },

        DcsEntry => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F | 0x7F => Some((Anywhere, Ignore)),
            0x20..=0x2F => Some((DcsIntermediate, Collect)),
            0x30..=0x39 | 0x3B => Some((DcsParam, Param)),
            0x3C..=0x3F => Some((DcsParam, Collect)),
            0x40..=0x7E => Some((DcsPassthrough, Ignore)),
            0x3A => Some((DcsIgnore, Ignore)),

            _ => None,
        },

        DcsParam => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F | 0x7F => Some((Anywhere, Ignore)),
            0x30..=0x39 | 0x3B => Some((Anywhere, Param)),
            0x3A | 0x3C..=0x3F => Some((DcsIgnore, Ignore)),
            0x20..=0x2F => Some((DcsIntermediate, Collect)),
            0x40..=0x7E => Some((DcsPassthrough, Ignore)),

            _ => None,
        },

        DcsIntermediate => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F | 0x7F => Some((Anywhere, Ignore)),
            0x20..=0x2F => Some((Anywhere, Collect)),

            0x30..=0x3F => Some((DcsIgnore, Ignore)),
            0x40..=0x7E => Some((DcsPassthrough, Ignore)),

            _ => None,
        },

        DcsPassthrough => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F | 0x20..=0x7E => Some((Anywhere, Put)),
            0x7F => Some((Anywhere, Ignore)),

            0x9C => Some((Ground, Ignore)),

            _ => None,
        },

        DcsIgnore => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F | 0x20..=0x7F => Some((Anywhere, Ignore)),

            0x9C => Some((Ground, Ignore)),

            _ => None,
        },

        OscString => match byte {
            0x07 => Some((Ground, Ignore)),
            0x00..=0x06 | 0x08..=0x17 | 0x19 | 0x1C..=0x1F => Some((Anywhere, Ignore)),
            0x20..=0xFF => Some((Anywhere, OscPut)),

            _ => None,
        },

        SosPmApcString => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F | 0x20..=0x7F => Some((Anywhere, Ignore)),

            0x9C => Some((Ground, Ignore)),

            _ => None,
        },
    }
}
