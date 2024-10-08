use std::char;

pub fn expected_bytes_count(first_byte: u8) -> Option<usize> {
    #[rustfmt::skip]
    static LENGTHS: [usize; 32] = [
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        0, 0, 0, 0, 0, 0, 0, 0,
        2, 2, 2, 2,
        3, 3,
        4,
        0,
    ];

    let len = LENGTHS[first_byte as usize >> 3];

    if len == 0 {
        None
    } else {
        Some(len)
    }
}

pub fn into_char(utf8: &[u8]) -> char {
    match simdutf8::basic::from_utf8(utf8) {
        Ok(s) => s.chars().next().expect("No character found"),
        Err(_) => char::REPLACEMENT_CHARACTER,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_count() {
        for case in [
            (0x0, Some(1)),
            (0x7F, Some(1)),
            (0xC2, Some(2)),
            (0xDF, Some(2)),
            (0xE0, Some(3)),
            (0xEF, Some(3)),
            (0xF0, Some(4)),
            (0xF4, Some(4)),
            (u8::MAX, None),
        ] {
            assert_eq!(case.1, expected_bytes_count(case.0))
        }
    }
}
