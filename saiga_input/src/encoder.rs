use crate::{
    function_keys::{self, ModifyKeys},
    key::{Action, Key, KeyEvent, Mods},
};

pub struct Encoder<'a> {
    pub event: KeyEvent<'a>,
    pub modify_other_keys_state_2: bool,
}

impl Encoder<'_> {
    pub fn encode(self) -> Option<&'static [u8]> {
        self.encode_legacy()
    }

    fn encode_legacy(self) -> Option<&'static [u8]> {
        let all_mods = self.event.mods;
        let effective_mods = self.event.effective_mods();

        if self.event.action == Action::Release || self.event.composing {
            return None;
        }

        if let Some(seq) =
            pc_style_function_key(self.event.key, all_mods, self.modify_other_keys_state_2)
        {
            // TODO: implement this check. Taken from ghostty.
            //
            // If we have UTF-8 text, then we never emit PC style function
            // keys. Many function keys (escape, enter, backspace) have
            // a specific meaning when dead keys are active and so we don't
            // want to send that to the terminal. Examples:
            //
            //   - Japanese: escape clears the dead key state
            //   - Korean: escape commits the dead key state
            //   - Korean: backspace should delete a single preedit char
            //
            // if (self.event.utf8.len > 0) {
            //     switch (self.event.key) {
            //         else => {},
            //         .backspace => return "",
            //         .enter, .escape => break :pc_style,
            //     }
            // }

            return Some(seq);
        }

        if let Some(seq) = ctrl_seq(
            self.event.key,
            self.event.utf8,
            self.event.unshifted_char,
            all_mods,
        ) {
            // TODO: alt-as-esc prefixing
            //
            // if effective_mods.contains(Mods::LEFT_ALT) {
            //
            // }

            return Some(seq);
        }

        // TODO: others

        None
    }
}

fn pc_style_function_key(key: Key, mods: Mods, modify_other_keys: bool) -> Option<&'static [u8]> {
    let entries = function_keys::get_key_entries(key);

    entries.iter().find_map(|entry| {
        match entry.modify_other_keys {
            ModifyKeys::Any => {}
            ModifyKeys::Set => {
                if modify_other_keys {
                    return None;
                }
            }
            ModifyKeys::SetOther => {
                if !modify_other_keys {
                    return None;
                }
            }
        }

        if entry.mods.is_empty() {
            if !mods.is_empty() && !entry.mods_empty_is_any {
                return None;
            }
        } else if entry.mods != mods {
            return None;
        }

        Some(entry.sequence.as_bytes())
    })
}

/// Returns the C0 byte for the key event if it should be used.
/// This converts a key event into the expected terminal behavior
/// such as Ctrl+C turning into 0x03, amongst many other translations.
///
/// This will return null if the key event should not be converted
/// into a C0 byte. There are many cases for this and you should read
/// the source code to understand them.
fn ctrl_seq(
    logical_key: Key,
    utf8: &str,
    unshifted_char: char,
    mods: Mods,
) -> Option<&'static [u8]> {
    if !mods.contains(Mods::LEFT_CTRL) && !mods.contains(Mods::RIGHT_CTRL) {
        return None;
    }

    let (ch, unset_mods) = 'unset_mods: {
        // Remove alt from our modifiers because it does not impact whether
        // we are generating a ctrl sequence and we handle the ESC-prefix
        // logic separately.
        let mut unset_mods = mods.difference(Mods::LEFT_ALT.union(Mods::RIGHT_ALT));

        let mut ch = 'char: {
            if utf8.len() == 1 {
                break 'char utf8.chars().next().unwrap();
            };

            if let Some(ch) = logical_key.char() {
                if unset_mods != Mods::LEFT_CTRL && unset_mods != Mods::RIGHT_CTRL {
                    return None;
                }

                break 'char ch;
            }

            return None;
        };

        let is_us_letter = ch.is_ascii_uppercase();

        if (unset_mods.contains(Mods::LEFT_SHIFT) || unset_mods.contains(Mods::RIGHT_SHIFT))
            && !is_us_letter
            && ch != '@'
        {
            unset_mods.remove(Mods::LEFT_SHIFT.union(Mods::RIGHT_SHIFT));
        }

        if is_us_letter && unshifted_char != '\0' {
            ch = unshifted_char;
        }

        break 'unset_mods (ch, unset_mods);
    };

    if !unset_mods
        .difference(Mods::LEFT_CTRL.union(Mods::RIGHT_CTRL))
        .is_empty()
    {
        return None;
    }

    if ch as usize >= CTRL_ESCAPED_BYTES_SET.len() {
        None
    } else {
        CTRL_ESCAPED_BYTES_SET[ch as usize]
    }
}

const CTRL_ESCAPED_BYTES_SET: [Option<&[u8]>; u8::MAX as usize + 1] =
    build_ctrl_escaped_bytes_set();

const fn build_ctrl_escaped_bytes_set() -> [Option<&'static [u8]>; u8::MAX as usize + 1] {
    let mut result: [Option<&'static [u8]>; u8::MAX as usize + 1] = [None; u8::MAX as usize + 1];

    const fn get_seq(ch: char) -> Option<&'static [u8]> {
        let mut i = 0;

        while i < CTRL_ESCAPED_BYTES.len() {
            if CTRL_ESCAPED_BYTES[i].0 == ch {
                return Some(CTRL_ESCAPED_BYTES[i].1);
            }

            i += 1;
        }

        None
    }

    let mut byte = 0;

    loop {
        result[byte as usize] = get_seq(byte as char);

        if byte == u8::MAX {
            break;
        }

        byte += 1;
    }

    result
}

const CTRL_ESCAPED_BYTES: [(char, &[u8]); 43] = [
    (' ', &[0]),
    ('/', &[31]),
    ('0', &[48]),
    ('1', &[49]),
    ('2', &[0]),
    ('3', &[27]),
    ('4', &[28]),
    ('5', &[29]),
    ('6', &[30]),
    ('7', &[31]),
    ('8', &[127]),
    ('9', &[57]),
    ('?', &[127]),
    ('@', &[0]),
    ('\\', &[28]),
    (']', &[29]),
    ('^', &[30]),
    ('_', &[31]),
    ('a', &[1]),
    ('b', &[2]),
    ('c', &[3]),
    ('d', &[4]),
    ('e', &[5]),
    ('f', &[6]),
    ('g', &[7]),
    ('h', &[8]),
    ('j', &[10]),
    ('k', &[11]),
    ('l', &[12]),
    ('n', &[14]),
    ('o', &[15]),
    ('p', &[16]),
    ('q', &[17]),
    ('r', &[18]),
    ('s', &[19]),
    ('t', &[20]),
    ('u', &[21]),
    ('v', &[22]),
    ('w', &[23]),
    ('x', &[24]),
    ('y', &[25]),
    ('z', &[26]),
    ('~', &[30]),
];

// const fn build_ctrl_escaped_bytes() -> [Option<u8>; u8::MAX as usize] {
//     let mut result = [None; u8::MAX as usize];
//
//     macro_rules! set {
//         ($ch:literal $byte:literal) => {
//             result[$ch as usize] = Some($byte)
//         };
//     }
//
//     set!(' ' 0);
//     set!('/' 31);
//     set!('0' 48);
//     set!('1' 49);
//     set!('2' 0);
//     set!('3' 27);
//     set!('4' 28);
//     set!('5' 29);
//     set!('6' 30);
//     set!('7' 31);
//     set!('8' 127);
//     set!('9' 57);
//     set!('?' 127);
//     set!('@' 0);
//     set!('\\' 28);
//     set!(']' 29);
//     set!('^' 30);
//     set!('_' 31);
//     set!('a' 1);
//     set!('b' 2);
//     set!('c' 3);
//     set!('d' 4);
//     set!('e' 5);
//     set!('f' 6);
//     set!('g' 7);
//     set!('h' 8);
//     set!('j' 10);
//     set!('k' 11);
//     set!('l' 12);
//     set!('n' 14);
//     set!('o' 15);
//     set!('p' 16);
//     set!('q' 17);
//     set!('r' 18);
//     set!('s' 19);
//     set!('t' 20);
//     set!('u' 21);
//     set!('v' 22);
//     set!('w' 23);
//     set!('x' 24);
//     set!('y' 25);
//     set!('z' 26);
//     set!('~' 30);
//
//     result
// }
