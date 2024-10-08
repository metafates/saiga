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

pub fn expected_utf8_bytes_count(first_byte: u8) -> Option<usize> {
    // TODO: make it branchless
    match first_byte {
        0x0..=0x7F => Some(1),
        0xC2..=0xDF => Some(2),
        0xE0..=0xEF => Some(3),
        0xF0..=0xF4 => Some(4),
        _ => None,
    }
}

pub fn char_from_utf8(utf8: &[u8]) -> char {
    match simdutf8::basic::from_utf8(utf8) {
        Ok(s) => s.chars().next().expect("No character found"),
        Err(_) => char::REPLACEMENT_CHARACTER,
    }
}
