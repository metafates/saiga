//! C0 set of 7-bit control characters (from ANSI X3.4-1977).

/// Null filler, terminal should ignore this character.
pub const NUL: u8 = 0x00;
/// Start of Header.
pub const SOH: u8 = 0x01;
/// Start of Text, implied end of header.
pub const STX: u8 = 0x02;
/// End of Text, causes some terminal to respond with ACK or NAK.
pub const ETX: u8 = 0x03;
/// End of Transmission.
pub const EOT: u8 = 0x04;
/// Enquiry, causes terminal to send ANSWER-BACK ID.
pub const ENQ: u8 = 0x05;
/// Acknowledge, usually sent by terminal in response to ETX.
pub const ACK: u8 = 0x06;
/// Bell, triggers the bell, buzzer, or beeper on the terminal.
pub const BEL: u8 = 0x07;
/// Backspace, can be used to define overstruck characters.
pub const BS: u8 = 0x08;
/// Horizontal Tabulation, move to next predetermined position.
pub const HT: u8 = 0x09;
/// Linefeed, move to same position on next line (see also NL).
pub const LF: u8 = 0x0A;
/// Vertical Tabulation, move to next predetermined line.
pub const VT: u8 = 0x0B;
/// Form Feed, move to next form or page.
pub const FF: u8 = 0x0C;
/// Carriage Return, move to first character of current line.
pub const CR: u8 = 0x0D;
/// Shift Out, switch to G1 (other half of character set).
pub const SO: u8 = 0x0E;
/// Shift In, switch to G0 (normal half of character set).
pub const SI: u8 = 0x0F;
/// Data Link Escape, interpret next control character specially.
pub const DLE: u8 = 0x10;
/// (DC1) Terminal is allowed to resume transmitting.
pub const XON: u8 = 0x11;
/// Device Control 2, causes ASR-33 to activate paper-tape reader.
pub const DC2: u8 = 0x12;
/// (DC2) Terminal must pause and refrain from transmitting.
pub const XOFF: u8 = 0x13;
/// Device Control 4, causes ASR-33 to deactivate paper-tape reader.
pub const DC4: u8 = 0x14;
/// Negative Acknowledge, used sometimes with ETX and ACK.
pub const NAK: u8 = 0x15;
/// Synchronous Idle, used to maintain timing in Sync communication.
pub const SYN: u8 = 0x16;
/// End of Transmission block.
pub const ETB: u8 = 0x17;
/// Cancel (makes VT100 abort current escape sequence if any).
pub const CAN: u8 = 0x18;
/// End of Medium.
pub const EM: u8 = 0x19;
/// Substitute (VT100 uses this to display parity errors).
pub const SUB: u8 = 0x1A;
/// Prefix to an escape sequence.
pub const ESC: u8 = 0x1B;
/// File Separator.
pub const FS: u8 = 0x1C;
/// Group Separator.
pub const GS: u8 = 0x1D;
/// Record Separator (sent by VT132 in block-transfer mode).
pub const RS: u8 = 0x1E;
/// Unit Separator.
pub const US: u8 = 0x1F;
/// Delete, should be ignored by terminal.
pub const DEL: u8 = 0x7f;

const ALL: [u8; 32] = [
    NUL, SOH, STX, ETX, EOT, ENQ, ACK, BEL, BS, HT, LF, VT, FF, CR, SO, SI, DLE, XON, DC2, XOFF,
    DC4, SYN, ETB, CAN, EM, SUB, ESC, FS, GS, RS, US, DEL,
];

const C0_SET_LEN: usize = u8::MAX as usize + 1;

static C0_SET: [bool; C0_SET_LEN] = build_c0_set();

const fn build_c0_set() -> [bool; C0_SET_LEN] {
    let mut set: [bool; C0_SET_LEN] = [false; C0_SET_LEN];

    let mut byte = 0;

    loop {
        set[byte as usize] = is_c0(byte);

        if byte == u8::MAX {
            break;
        }

        byte += 1;
    }

    set
}

const fn is_c0(byte: u8) -> bool {
    let mut i = 0;

    while i != ALL.len() {
        if ALL[i] == byte {
            return true;
        }

        i += 1
    }

    false
}

#[inline]
pub fn first_index_of_c0(haystack: &[u8]) -> Option<usize> {
    haystack
        .iter()
        .enumerate()
        .find_map(|(i, &b)| if C0_SET[b as usize] { Some(i) } else { None })
}

#[cfg(test)]
mod bench {
    use super::*;

    extern crate test;

    const SAMPLE: &[u8] = include_bytes!("test.ansi");

    #[bench]
    fn first_index_of_c0_bench(b: &mut test::Bencher) {
        b.iter(|| {
            let mut i = 0;

            while let Some(idx) = first_index_of_c0(&SAMPLE[i..]) {
                i += idx + 1
            }
        })
    }
}
