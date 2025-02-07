//! This is the list of "PC style function keys" that xterm supports for
//! the legacy keyboard protocols. These always take priority since at the
//! time of writing this, even the most modern keyboard protocols still
//! are backwards compatible with regards to these sequences.
//!
//! This is based on a variety of sources cross-referenced but mostly
//! based on foot's keymap.h: https://codeberg.org/dnkl/foot/src/branch/master/keymap.h
use crate::key::Mods;

pub enum CursorMode {
    Any,
    Normal,
    Application,
}

pub enum KeypadMode {
    Any,
    Normal,
    Application,
}

pub enum ModifyKeys {
    Any,
    Set,
    SetOther,
}

/// A single entry in the table of keys.
pub struct Entry {
    /// The exact set of modifiers that must be active for this entry to match.
    /// If mods_empty_is_any is true then empty mods means any set of mods can
    /// match. Otherwise, empty mods means no mods must be active.
    mods: Mods,
    mods_empty_is_any: bool,

    /// The state required for cursor/keypad mode.
    cursor: CursorMode,
    keypad: KeypadMode,

    /// Whether or not this entry should be used
    modify_other_keys: ModifyKeys,

    /// The sequence to send to the pty if this entry matches.
    sequence: Vec<u8>,
}

impl Default for Entry {
    fn default() -> Self {
        Self {
            mods: Mods::empty(),
            mods_empty_is_any: true,
            cursor: CursorMode::Any,
            keypad: KeypadMode::Any,
            modify_other_keys: ModifyKeys::Any,
            sequence: Vec::new(),
        }
    }
}

/// The list of modifier combinations for modify other key sequences.
/// The mode value is index + 2.
pub const MODIFIERS: [Mods; 15] = [
    Mods::SHIFT,
    Mods::ALT,
    Mods::SHIFT.union(Mods::ALT),
    Mods::CTRL,
    Mods::SHIFT.union(Mods::CTRL),
    Mods::ALT.union(Mods::CTRL),
    Mods::SHIFT.union(Mods::ALT).union(Mods::CTRL),
    Mods::META,
    Mods::SHIFT.union(Mods::META),
    Mods::ALT.union(Mods::META),
    Mods::SHIFT.union(Mods::ALT).union(Mods::META),
    Mods::CTRL.union(Mods::META),
    Mods::SHIFT.union(Mods::CTRL).union(Mods::META),
    Mods::ALT.union(Mods::CTRL).union(Mods::META),
    Mods::SHIFT
        .union(Mods::ALT)
        .union(Mods::CTRL)
        .union(Mods::META),
];

fn pc_style(fmt: fn(code: usize) -> String) -> Vec<Entry> {
    MODIFIERS
        .iter()
        .enumerate()
        .map(|(i, &mods)| {
            let code = i + 2;

            Entry {
                mods,
                sequence: fmt(code).into_bytes(),
                ..Default::default()
            }
        })
        .collect()
}
