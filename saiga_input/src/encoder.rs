use crate::key::{Action, Key, KeyEvent, Mods};

pub enum MacosOptionAsAlt {
    None,
    Both,
    Left,
    Right,
}

pub struct KeyEncoder {
    event: KeyEvent,
    macos_option_as_alt: MacosOptionAsAlt,
}

impl KeyEncoder {
    fn encode_legacy(&self, buf: &[u8]) -> Option<String> {
        let all_mods = self.event.mods;
        let effective_mods = self.event.effective_mods();

        if self.event.action != Action::Press && self.event.action != Action::Repeat {
            return None;
        }

        if self.event.composing {
            return None;
        }

        todo!()
    }
}

/// Determines whether the key should be encoded in the xterm
/// "PC-style Function Key" syntax (roughly). This is a hardcoded
/// table of keys and modifiers that result in a specific sequence.
fn pc_style_function_key(
    keyval: Key,
    mods: Mods,
    cursor_key_application: bool,
    keypad_key_application_req: bool,
    ignore_keypad_with_numlock: bool,
    modify_other_keys: bool, // True if state 2
) -> String {
    let keypad_key_application = ignore_keypad_with_numlock || keypad_key_application_req;

    todo!()
}
