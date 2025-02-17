use core::str;
use std::char;

use simdutf8::basic::Utf8Error;

const MAX_LENGTH: usize = 4;

#[derive(Default)]
pub struct UTF8Collector {
    bytes: [u8; MAX_LENGTH],
    len: usize,
    pub remaining_count: usize,
}

impl UTF8Collector {
    #[inline]
    pub const fn push(&mut self, byte: u8) {
        self.bytes[self.len] = byte;
        self.len += 1;
    }

    #[inline]
    fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    #[inline]
    pub const fn reset(&mut self) {
        self.len = 0;
        self.remaining_count = 0;
    }

    #[inline]
    pub fn char(&self) -> char {
        into_char(self.as_slice())
    }
}

#[inline]
pub const fn expected_bytes_count(first_byte: u8) -> Option<usize> {
    #[rustfmt::skip]
    const LENGTHS: [usize; 32] = [
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        0, 0, 0, 0, 0, 0, 0, 0,
        2, 2, 2, 2,
        3, 3,
        4,
        0,
    ];

    const EXPECTED: [Option<usize>; 5] = [None, Some(1), Some(2), Some(3), Some(4)];

    EXPECTED[LENGTHS[first_byte as usize >> 3]]
}

#[inline]
pub fn from_utf8(utf8: &[u8]) -> Result<&str, Utf8Error> {
    simdutf8::basic::from_utf8(utf8)
}

pub fn into_char(utf8: &[u8]) -> char {
    match from_utf8(utf8) {
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

// #[cfg(test)]
// mod bench {
//     use super::*;
//     extern crate test;
//
//     #[bench]
//     fn char(b: &mut test::Bencher) {
//         b.iter(|| into_char(b"\xD1\x86"))
//     }
// }
