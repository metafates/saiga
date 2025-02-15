use crate::key::{Key, Mods};

const BUFFER_SIZE: usize = 30;

#[derive(Clone, Copy, Debug)]
pub enum CursorMode {
    Any,
    Normal,
    Application,
}

#[derive(Clone, Copy, Debug)]
pub enum KeypadMode {
    Any,
    Normal,
    Application,
}

#[derive(Clone, Copy, Debug)]
pub enum ModifyKeys {
    Any,
    Set,
    SetOther,
}

#[derive(Clone, Copy, Debug)]
pub struct Sequence {
    buffer: [u8; BUFFER_SIZE],
    len: usize,
}

impl Sequence {
    const fn empty() -> Self {
        Self {
            buffer: [0; BUFFER_SIZE],
            len: 0,
        }
    }

    const fn new(bytes: &[u8]) -> Self {
        let mut seq = Self::empty();
        seq.concat(bytes);

        seq
    }

    const fn push(&mut self, byte: u8) -> &mut Self {
        self.buffer[self.len] = byte;
        self.len += 1;

        self
    }

    const fn concat(&mut self, bytes: &[u8]) -> &mut Self {
        let mut i = 0;

        while i < bytes.len() {
            self.push(bytes[i]);

            i += 1;
        }

        self
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer[..self.len]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub mods: Mods,
    pub mods_empty_is_any: bool,

    pub cursor: CursorMode,
    pub keypad: KeypadMode,

    pub modify_other_keys: ModifyKeys,

    pub sequence: Sequence,
}

impl Entry {
    const DEFAULT: Self = Self {
        mods: Mods::empty(),
        mods_empty_is_any: true,
        cursor: CursorMode::Any,
        keypad: KeypadMode::Any,
        modify_other_keys: ModifyKeys::Any,
        sequence: Sequence::empty(),
    };
}

/// The list of modifier combinations for modify other key sequences.
/// The mode value is index + 2.
pub const MODIFIERS: [Mods; 15] = [
    Mods::LEFT_SHIFT,
    Mods::LEFT_ALT,
    Mods::LEFT_SHIFT.union(Mods::LEFT_ALT),
    Mods::LEFT_CTRL,
    Mods::LEFT_SHIFT.union(Mods::LEFT_CTRL),
    Mods::LEFT_ALT.union(Mods::LEFT_CTRL),
    Mods::LEFT_SHIFT
        .union(Mods::LEFT_ALT)
        .union(Mods::LEFT_CTRL),
    Mods::LEFT_SUPER,
    Mods::LEFT_SHIFT.union(Mods::LEFT_SUPER),
    Mods::LEFT_ALT.union(Mods::LEFT_SUPER),
    Mods::LEFT_SHIFT
        .union(Mods::LEFT_ALT)
        .union(Mods::LEFT_SUPER),
    Mods::LEFT_CTRL.union(Mods::LEFT_SUPER),
    Mods::LEFT_SHIFT
        .union(Mods::LEFT_CTRL)
        .union(Mods::LEFT_SUPER),
    Mods::LEFT_ALT
        .union(Mods::LEFT_CTRL)
        .union(Mods::LEFT_SUPER),
    Mods::LEFT_SHIFT
        .union(Mods::LEFT_ALT)
        .union(Mods::LEFT_CTRL)
        .union(Mods::LEFT_SUPER),
];

const KEY_ENTRIES_SET_LEN: usize = Key::ALL_VARIANTS.len() as usize;
const KEY_ENTRIES_SET: [&[Entry]; KEY_ENTRIES_SET_LEN] = build_key_entries_set();

const fn build_key_entries_set() -> [&'static [Entry]; KEY_ENTRIES_SET_LEN] {
    let mut set: [&'static [Entry]; KEY_ENTRIES_SET_LEN] = [&[]; KEY_ENTRIES_SET_LEN];

    let mut i = 0;

    const fn get_entries(key: Key) -> &'static [Entry] {
        let mut i = 0;

        while i < KEY_ENTRIES.len() {
            if key as u8 == KEY_ENTRIES[i].0 as u8 {
                return KEY_ENTRIES[i].1;
            }

            i += 1;
        }

        &[]
    }

    while i < KEY_ENTRIES_SET_LEN {
        let key = Key::ALL_VARIANTS[i];

        set[key as usize] = get_entries(key);

        i += 1;
    }

    set
}

#[inline]
pub const fn get_key_entries(key: Key) -> &'static [Entry] {
    KEY_ENTRIES_SET[key as usize]
}

const fn concat_entries<const A: usize, const B: usize, const C: usize>(
    a: [Entry; A],
    b: [Entry; B],
) -> [Entry; C] {
    let mut result = [Entry::DEFAULT; C];

    let mut i = 0;

    while i < A {
        result[i] = a[i];

        i += 1;
    }

    while (i - A) < B {
        result[i] = b[i - A];

        i += 1;
    }

    result
}

const KEY_ENTRIES: [(Key, &[Entry]); 8] = [
    (
        Key::Up,
        &concat_entries::<15, 2, 17>(pc_style(b"\x1b[1;", b"A"), cursor_key(b"\x1b[A", b"\x1bOA")),
    ),
    (
        Key::Down,
        &concat_entries::<15, 2, 17>(pc_style(b"\x1b[1;", b"B"), cursor_key(b"\x1b[B", b"\x1bOB")),
    ),
    (
        Key::Right,
        &concat_entries::<15, 2, 17>(pc_style(b"\x1b[1;", b"C"), cursor_key(b"\x1b[C", b"\x1bOC")),
    ),
    (
        Key::Left,
        &concat_entries::<15, 2, 17>(pc_style(b"\x1b[1;", b"D"), cursor_key(b"\x1b[D", b"\x1bOD")),
    ),
    (
        Key::Backspace,
        &[
            Entry {
                mods: Mods::LEFT_CTRL,
                sequence: Sequence::new(b"\x08"),
                ..Entry::DEFAULT
            },
            Entry {
                sequence: Sequence::new(b"\x7f"),
                ..Entry::DEFAULT
            },
        ],
    ),
    (
        Key::Tab,
        &[
            Entry {
                mods: Mods::LEFT_SHIFT,
                sequence: Sequence::new(b"\x1b[Z"),
                modify_other_keys: ModifyKeys::Set,
                ..Entry::DEFAULT
            },
            Entry {
                mods: Mods::LEFT_SHIFT,
                sequence: Sequence::new(b"\x1b[27;2;9~"),
                modify_other_keys: ModifyKeys::SetOther,
                ..Entry::DEFAULT
            },
            Entry {
                sequence: Sequence::new(b"\t"),
                ..Entry::DEFAULT
            },
        ],
    ),
    (
        Key::Enter,
        &[Entry {
            sequence: Sequence::new(b"\r"),
            ..Entry::DEFAULT
        }],
    ),
    (
        Key::Escape,
        &[Entry {
            sequence: Sequence::new(b"\x1b"),
            ..Entry::DEFAULT
        }],
    ),
];

const fn pc_style(left: &'static [u8], right: &'static [u8]) -> [Entry; 15] {
    let mut entries = [Entry::DEFAULT; 15];

    let mut i = 0;

    const CODES: [&[u8]; 15] = [
        b"2", b"3", b"4", b"5", b"6", b"7", b"8", b"9", b"10", b"11", b"12", b"13", b"14", b"15",
        b"16",
    ];

    while i != MODIFIERS.len() {
        entries[i].mods = MODIFIERS[i];

        entries[i]
            .sequence
            .concat(left)
            .concat(CODES[i])
            .concat(right);

        i += 1;
    }

    entries
}

const fn cursor_key(normal: &'static [u8], application: &'static [u8]) -> [Entry; 2] {
    [
        Entry {
            cursor: CursorMode::Normal,
            sequence: Sequence::new(normal),
            ..Entry::DEFAULT
        },
        Entry {
            cursor: CursorMode::Application,
            sequence: Sequence::new(application),
            ..Entry::DEFAULT
        },
    ]
}
