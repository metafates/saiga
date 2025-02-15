use saiga_macros::AllVariants;

const KEY_TO_CHAR_MAP_LEN: usize = Key::ALL_VARIANTS.len();
const KEY_TO_CHAR_MAP: [Option<char>; KEY_TO_CHAR_MAP_LEN] = build_key_to_char_map();

const ASCII_TO_KEY_MAP_LEN: usize = u8::MAX as usize + 1;
const ASCII_TO_KEY_MAP: [Option<Key>; ASCII_TO_KEY_MAP_LEN] = build_ascii_to_key_map();

const fn build_ascii_to_key_map() -> [Option<Key>; ASCII_TO_KEY_MAP_LEN] {
    let mut map = [None; ASCII_TO_KEY_MAP_LEN];

    let mut ascii: u8 = 0;

    const fn ascii_to_key(ascii: u8) -> Option<Key> {
        let mut i = 0;

        while i != Key::ALL_VARIANTS.len() {
            let key = Key::ALL_VARIANTS[i];

            if !key.is_keypad() {
                match KEY_TO_CHAR_MAP[key as u8 as usize] {
                    Some(ch) if ch == ascii as char => return Some(key),
                    _ => {}
                };
            }

            i += 1;
        }

        None
    }

    loop {
        map[ascii as usize] = ascii_to_key(ascii);

        if ascii == u8::MAX {
            break;
        }

        ascii += 1;
    }

    map
}

const fn build_key_to_char_map() -> [Option<char>; KEY_TO_CHAR_MAP_LEN] {
    let mut map = [None; KEY_TO_CHAR_MAP_LEN];

    use Key::*;

    macro_rules! set {
        ($key:ident $ch:literal) => {
            map[$key as usize] = Some($ch)
        };
    }

    set!(A 'a');
    set!(B 'b');
    set!(C 'c');
    set!(D 'd');
    set!(E 'e');
    set!(F 'f');
    set!(G 'g');
    set!(H 'h');
    set!(I 'i');
    set!(J 'j');
    set!(K 'k');
    set!(L 'l');
    set!(M 'm');
    set!(N 'n');
    set!(O 'o');
    set!(P 'p');
    set!(Q 'q');
    set!(R 'r');
    set!(S 's');
    set!(T 't');
    set!(U 'u');
    set!(V 'v');
    set!(W 'w');
    set!(X 'x');
    set!(Y 'y');
    set!(Z 'z');
    set!(Zero '0');
    set!(One '1');
    set!(Two '2');
    set!(Three '3');
    set!(Four '4');
    set!(Five '5');
    set!(Six '6');
    set!(Seven '7');
    set!(Eight '8');
    set!(Nine '9');
    set!(Semicolon ';');
    set!(Space ' ');
    set!(Apostrophe '\'');
    set!(Comma ',');
    set!(GraveAccent '`');
    set!(Period '.');
    set!(Slash '/');
    set!(Minus '-');
    set!(Plus '+');
    set!(Equal '=');
    set!(LeftBracket '[');
    set!(RightBracket ']');
    set!(Backslash '\\');

    set!(Tab '\t');

    set!(KP0 '0');
    set!(KP1 '1');
    set!(KP2 '2');
    set!(KP3 '3');
    set!(KP4 '4');
    set!(KP5 '5');
    set!(KP6 '6');
    set!(KP7 '7');
    set!(KP8 '8');
    set!(KP9 '9');
    set!(KPDecimal '.');
    set!(KPDivide '/');
    set!(KPMultiply '*');
    set!(KPSubtract '-');
    set!(KPAdd '+');
    set!(KPEqual '=');

    map
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Action {
    Release,
    Press,
    Repeat,
}

#[repr(u8)]
#[derive(Clone, Copy, AllVariants, Debug)]
pub enum Key {
    Invalid,

    // a-z
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    // numbers
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,

    // punctuation
    Semicolon,
    Space,
    Apostrophe,
    Comma,
    GraveAccent, // `
    Period,
    Slash,
    Minus,
    Plus,
    Equal,
    LeftBracket,  // [
    RightBracket, // ]
    Backslash,    // \

    // control
    Up,
    Down,
    Right,
    Left,
    Home,
    End,
    Insert,
    Delete,
    CapsLock,
    ScrollLock,
    NumLock,
    PageUp,
    PageDown,
    Escape,
    Enter,
    Tab,
    Backspace,
    PrintScreen,
    Pause,

    // function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    F25,

    // keypad
    KP0,
    KP1,
    KP2,
    KP3,
    KP4,
    KP5,
    KP6,
    KP7,
    KP8,
    KP9,
    KPDecimal,
    KPDivide,
    KPMultiply,
    KPSubtract,
    KPAdd,
    KPEnter,
    KPEqual,
    KPSeparator,
    KPLeft,
    KPRight,
    KPUp,
    KPdown,
    KPPageUp,
    KPPageDown,
    KPHome,
    KPEnd,
    KPInsert,
    KPDelete,
    KPBegin,

    // TODO: media keys

    // modifiers
    LeftShift,
    LeftControl,
    LeftAlt,
    LeftSuper,
    RightShift,
    RightControl,
    RightAlt,
    RightSuper,
}

impl Key {
    #[inline]
    pub const fn from_ascii(ascii: u8) -> Option<Key> {
        ASCII_TO_KEY_MAP[ascii as usize]
    }

    #[inline]
    pub const fn is_printable(self) -> bool {
        self.char().is_some()
    }

    #[inline]
    pub const fn char(self) -> Option<char> {
        KEY_TO_CHAR_MAP[self as usize]
    }

    #[inline]
    pub const fn is_shift(self) -> bool {
        use Key::*;

        matches!(self, LeftShift | RightShift)
    }

    #[inline]
    pub const fn is_alt(self) -> bool {
        use Key::*;

        matches!(self, LeftAlt | RightAlt)
    }

    #[inline]
    pub const fn is_control(self) -> bool {
        use Key::*;

        matches!(self, LeftControl | RightControl)
    }

    #[inline]
    pub const fn is_super(self) -> bool {
        use Key::*;

        matches!(self, LeftSuper | RightSuper)
    }

    #[inline]
    pub const fn is_modifier(self) -> bool {
        self.is_control() || self.is_shift() || self.is_alt() || self.is_super()
    }

    #[inline]
    pub const fn is_keypad(self) -> bool {
        use Key::*;

        matches!(
            self,
            KP0 | KP1
                | KP2
                | KP3
                | KP4
                | KP5
                | KP6
                | KP7
                | KP8
                | KP9
                | KPDecimal
                | KPDivide
                | KPMultiply
                | KPSubtract
                | KPAdd
                | KPEnter
                | KPEqual
                | KPSeparator
                | KPLeft
                | KPRight
                | KPUp
                | KPdown
                | KPPageUp
                | KPPageDown
                | KPHome
                | KPEnd
                | KPInsert
                | KPDelete
                | KPBegin
        )
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Mods: u8 {
        const LEFT_SHIFT  = 0b00000001;
        const LEFT_CTRL   = 0b00000010;
        const LEFT_ALT    = 0b00000100;
        const LEFT_SUPER  = 0b00001000;

        const RIGHT_SHIFT = 0b00010000;
        const RIGHT_CTRL  = 0b00100000;
        const RIGHT_ALT   = 0b01000000;
        const RIGHT_SUPER = 0b10000000;
    }
}

pub struct KeyEvent<'a> {
    pub action: Action,
    pub key: Key,
    pub physical_key: Key,
    pub mods: Mods,
    pub consumed_mods: Mods,
    pub composing: bool,
    pub utf8: &'a str,
    pub unshifted_char: char,
}

impl KeyEvent<'_> {
    pub const DEFAULT: Self = Self {
        action: Action::Press,
        key: Key::Invalid,
        physical_key: Key::Invalid,
        mods: Mods::empty(),
        consumed_mods: Mods::empty(),
        composing: false,
        utf8: "",
        unshifted_char: '\0',
    };

    #[inline]
    pub const fn effective_mods(&self) -> Mods {
        if self.utf8.is_empty() {
            self.mods
        } else {
            self.mods.difference(self.consumed_mods)
        }
    }
}

#[cfg(feature = "winit")]
impl From<winit::keyboard::PhysicalKey> for Key {
    fn from(key: winit::keyboard::PhysicalKey) -> Self {
        use winit::keyboard::{KeyCode, PhysicalKey};

        match key {
            PhysicalKey::Unidentified(_) => Self::Invalid,
            PhysicalKey::Code(code) => match code {
                KeyCode::Backquote => Key::GraveAccent,
                KeyCode::Backslash => Key::Backslash,
                KeyCode::BracketLeft => Key::LeftBracket,
                KeyCode::BracketRight => Key::RightBracket,
                KeyCode::Comma => Key::Comma,
                KeyCode::Digit0 => Key::Zero,
                KeyCode::Digit1 => Key::One,
                KeyCode::Digit2 => Key::Two,
                KeyCode::Digit3 => Key::Three,
                KeyCode::Digit4 => Key::Four,
                KeyCode::Digit5 => Key::Five,
                KeyCode::Digit6 => Key::Six,
                KeyCode::Digit7 => Key::Seven,
                KeyCode::Digit8 => Key::Eight,
                KeyCode::Digit9 => Key::Nine,
                KeyCode::Equal => Key::Equal,
                KeyCode::KeyA => Key::A,
                KeyCode::KeyB => Key::B,
                KeyCode::KeyC => Key::C,
                KeyCode::KeyD => Key::D,
                KeyCode::KeyE => Key::E,
                KeyCode::KeyF => Key::F,
                KeyCode::KeyG => Key::G,
                KeyCode::KeyH => Key::H,
                KeyCode::KeyI => Key::I,
                KeyCode::KeyJ => Key::J,
                KeyCode::KeyK => Key::K,
                KeyCode::KeyL => Key::L,
                KeyCode::KeyM => Key::M,
                KeyCode::KeyN => Key::N,
                KeyCode::KeyO => Key::O,
                KeyCode::KeyP => Key::P,
                KeyCode::KeyQ => Key::Q,
                KeyCode::KeyR => Key::R,
                KeyCode::KeyS => Key::S,
                KeyCode::KeyT => Key::T,
                KeyCode::KeyU => Key::U,
                KeyCode::KeyV => Key::V,
                KeyCode::KeyW => Key::W,
                KeyCode::KeyX => Key::X,
                KeyCode::KeyY => Key::Y,
                KeyCode::KeyZ => Key::Z,
                KeyCode::Minus => Key::Minus,
                KeyCode::Period => Key::Period,
                KeyCode::Quote => Key::Apostrophe,
                KeyCode::Semicolon => Key::Semicolon,
                KeyCode::Slash => Key::Slash,
                KeyCode::AltLeft => Key::LeftAlt,
                KeyCode::AltRight => Key::RightAlt,
                KeyCode::Backspace => Key::Backspace,
                KeyCode::CapsLock => Key::CapsLock,
                KeyCode::ControlLeft => Key::LeftControl,
                KeyCode::ControlRight => Key::RightControl,
                KeyCode::Enter => Key::Enter,
                KeyCode::SuperLeft => Key::LeftSuper,
                KeyCode::SuperRight => Key::RightSuper,
                KeyCode::ShiftLeft => Key::LeftShift,
                KeyCode::ShiftRight => Key::RightShift,
                KeyCode::Space => Key::Space,
                KeyCode::Tab => Key::Tab,
                KeyCode::Delete => Key::Delete,
                KeyCode::End => Key::End,
                KeyCode::Home => Key::Home,
                KeyCode::PageDown => Key::PageDown,
                KeyCode::PageUp => Key::PageUp,
                KeyCode::ArrowDown => Key::Down,
                KeyCode::ArrowLeft => Key::Left,
                KeyCode::ArrowRight => Key::Right,
                KeyCode::ArrowUp => Key::Up,
                KeyCode::NumLock => Key::NumLock,
                KeyCode::Numpad0 => Key::KP0,
                KeyCode::Numpad1 => Key::KP1,
                KeyCode::Numpad2 => Key::KP2,
                KeyCode::Numpad3 => Key::KP3,
                KeyCode::Numpad4 => Key::KP4,
                KeyCode::Numpad5 => Key::KP5,
                KeyCode::Numpad6 => Key::KP6,
                KeyCode::Numpad7 => Key::KP7,
                KeyCode::Numpad8 => Key::KP8,
                KeyCode::Numpad9 => Key::KP9,
                KeyCode::NumpadAdd => Key::KPAdd,
                KeyCode::NumpadDecimal => Key::KPDecimal,
                KeyCode::NumpadDivide => Key::KPDivide,
                KeyCode::NumpadEnter => Key::KPEnter,
                KeyCode::NumpadEqual => Key::KPEqual,
                KeyCode::NumpadMultiply => Key::KPMultiply,
                KeyCode::NumpadSubtract => Key::KPSubtract,
                KeyCode::F1 => Key::F1,
                KeyCode::F2 => Key::F2,
                KeyCode::F3 => Key::F3,
                KeyCode::F4 => Key::F4,
                KeyCode::F5 => Key::F5,
                KeyCode::F6 => Key::F6,
                KeyCode::F7 => Key::F7,
                KeyCode::F8 => Key::F8,
                KeyCode::F9 => Key::F9,
                KeyCode::F10 => Key::F10,
                KeyCode::F11 => Key::F11,
                KeyCode::F12 => Key::F12,
                KeyCode::F13 => Key::F13,
                KeyCode::F14 => Key::F14,
                KeyCode::F15 => Key::F15,
                KeyCode::F16 => Key::F16,
                KeyCode::F17 => Key::F17,
                KeyCode::F18 => Key::F18,
                KeyCode::F19 => Key::F19,
                KeyCode::F20 => Key::F20,
                KeyCode::F21 => Key::F21,
                KeyCode::F22 => Key::F22,
                KeyCode::F23 => Key::F23,
                KeyCode::F24 => Key::F24,
                KeyCode::F25 => Key::F25,
                _ => Key::Invalid,
            },
        }
    }
}
