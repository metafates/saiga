#[allow(dead_code)]
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

// Based on https://vt100.net/emu/dec_ansi_parser
pub fn change_state(state: State, byte: u8) -> Option<(State, Option<Action>)> {
    use Action::*;
    use State::*;

    match state {
        Anywhere => match byte {
            0x18 | 0x1A | 0x80..=0x8F | 0x91..=0x97 | 0x99 | 0x9A => Some((Ground, Some(Execute))),
            0x9C => Some((Ground, None)),
            0x1B => Some((Escape, None)),
            0x98 | 0x9E | 0x9F => Some((SosPmApcString, None)),
            0x90 => Some((DcsEntry, None)),
            0x9D => Some((OscString, None)),
            0x9B => Some((CsiEntry, None)),

            _ => None,
        },

        Ground => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => Some((Anywhere, Some(Execute))),
            0x20..=0x7F => Some((Anywhere, Some(Print))),

            _ => None,
        },

        Escape => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => Some((Anywhere, Some(Execute))),
            0x7F => Some((Anywhere, Some(Ignore))),

            0x5D => Some((OscString, None)),
            0x50 => Some((DcsEntry, None)),
            0x5B => Some((CsiEntry, None)),
            0x58 | 0x5E | 0x5F => Some((SosPmApcString, None)),
            0x20..=0x2F => Some((EscapeIntermediate, Some(Collect))),
            0x30..=0x4F | 0x51..=0x57 | 0x59 | 0x5A | 0x5C | 0x60..=0x7E => {
                Some((Ground, Some(EscDispatch)))
            }

            _ => None,
        },

        EscapeIntermediate => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => Some((Anywhere, Some(Execute))),
            0x20..=0x2F => Some((Anywhere, Some(Collect))),
            0x7F => Some((Anywhere, Some(Ignore))),
            0x30..=0x7E => Some((Ground, Some(EscDispatch))),

            _ => None,
        },

        CsiEntry => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => Some((Anywhere, Some(Execute))),
            0x7F => Some((Anywhere, Some(Ignore))),
            0x20..=0x2F => Some((CsiIntermediate, Some(Collect))),
            0x40..=0x7E => Some((Ground, Some(CsiDispatch))),

            // 0x3A ':' (colon) should result CsiIgnore state according to the parser
            // specification. However, this parser implements subparameters separated by colon
            0x30..=0x3B => Some((CsiParam, Some(Param))),
            0x3C..=0x3F => Some((CsiParam, Some(Collect))),

            _ => None,
        },

        CsiParam => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => Some((Anywhere, Some(Execute))),

            // 0x3A ':' (colon) should result CsiIgnore state according to the parser
            // specification. However, our parser implements subparameters separated by colon
            0x30..=0x3B => Some((Anywhere, Some(Param))),

            0x7F => Some((Anywhere, Some(Ignore))),

            0x3C..=0x3F => Some((CsiIgnore, None)),
            0x20..=0x2F => Some((CsiIntermediate, None)),
            0x40..=0x7E => Some((Ground, Some(CsiDispatch))),

            _ => None,
        },

        CsiIntermediate => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => Some((Anywhere, Some(Execute))),
            0x20..=0x2F => Some((Anywhere, Some(Collect))),
            0x7F => Some((Anywhere, Some(Ignore))),
            0x30..=0x3F => Some((CsiIgnore, None)),
            0x40..=0x7E => Some((Ground, Some(CsiDispatch))),

            _ => None,
        },

        CsiIgnore => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => Some((Anywhere, Some(Execute))),
            0x20..=0x3F | 0x7F => Some((Anywhere, Some(Ignore))),
            0x40..=0x7E => Some((Ground, None)),

            _ => None,
        },

        DcsEntry => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F | 0x7F => Some((Anywhere, Some(Ignore))),
            0x20..=0x2F => Some((DcsIntermediate, Some(Collect))),
            0x30..=0x39 | 0x3B => Some((DcsParam, Some(Param))),
            0x3C..=0x3F => Some((DcsParam, Some(Collect))),
            0x40..=0x7E => Some((DcsPassthrough, None)),
            0x3A => Some((DcsIgnore, None)),

            _ => None,
        },

        DcsParam => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F | 0x7F => Some((Anywhere, Some(Ignore))),
            0x30..=0x39 | 0x3B => Some((Anywhere, Some(Param))),
            0x3A | 0x3C..=0x3F => Some((DcsIgnore, None)),
            0x20..=0x2F => Some((DcsIntermediate, Some(Collect))),
            0x40..=0x7E => Some((DcsPassthrough, None)),

            _ => None,
        },

        DcsIntermediate => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F | 0x7F => Some((Anywhere, Some(Ignore))),
            0x20..=0x2F => Some((Anywhere, Some(Collect))),

            0x30..=0x3F => Some((DcsIgnore, None)),
            0x40..=0x7E => Some((DcsPassthrough, None)),

            _ => None,
        },

        DcsPassthrough => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F | 0x20..=0x7E => Some((Anywhere, Some(Put))),
            0x7F => Some((Anywhere, Some(Ignore))),

            0x9C => Some((Ground, None)),

            _ => None,
        },

        DcsIgnore => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F | 0x20..=0x7F => Some((Anywhere, Some(Ignore))),

            0x9C => Some((Ground, None)),

            _ => None,
        },

        OscString => match byte {
            0x07 => Some((Ground, None)),
            0x00..=0x06 | 0x08..=0x17 | 0x19 | 0x1C..=0x1F => Some((Anywhere, Some(Ignore))),
            0x20..=0xFF => Some((Anywhere, Some(OscPut))),

            _ => None,
        },

        SosPmApcString => match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F | 0x20..=0x7F => Some((Anywhere, Some(Ignore))),

            0x9C => Some((Ground, None)),

            _ => None,
        },
    }
}
