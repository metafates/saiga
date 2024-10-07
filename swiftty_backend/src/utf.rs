pub fn find_utf8_start(bytes: &[u8]) -> Option<usize> {
    _ = bytes;

    None
}

#[derive(Clone, Copy)]
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
