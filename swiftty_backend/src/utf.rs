use std::char;

pub fn find_utf8_start(bytes: &[u8]) -> Option<usize> {
    for (i, byte) in bytes.iter().enumerate() {
        match byte {
            0x0..=0x7F | 0xC2..=0xDF | 0xE0..=0xEF | 0xF0..=0xF4 => return Some(i),
            _ => (),
        }
    }

    None
}

#[derive(Clone, Copy, Debug)]
pub enum UTF8BytesCount {
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
}

pub fn expected_utf8_bytes_count(first_byte: u8) -> Option<UTF8BytesCount> {
    use UTF8BytesCount::*;

    match first_byte {
        0x0..=0x7F => Some(One),
        0xC2..=0xDF => Some(Two),
        0xE0..=0xEF => Some(Three),
        0xF0..=0xF4 => Some(Four),
        _ => None,
    }
}

pub fn char_from_utf8(utf8: &[u8]) -> char {
    let s = std::str::from_utf8(utf8).expect("Invalid UTF-8 sequence");

    s.chars().next().expect("No character found")
}
