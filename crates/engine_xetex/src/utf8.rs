//! UTF-8 decoding tables from the Unicode consortium's ConvertUTF sample code.

pub(crate) const OFFSETS_FROM_UTF8: [u32; 6] = [
    0x0000_0000,
    0x0000_3080,
    0x000E_2080,
    0x03C8_2080,
    0xFA08_2080,
    0x8208_2080,
];

#[rustfmt::skip]
pub(crate) const BYTES_FROM_UTF8: [u8; 256] = [
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1, 1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
    2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2, 3,3,3,3,3,3,3,3,4,4,4,4,5,5,5,5,
];

pub(crate) const FIRST_BYTE_MARK: [u8; 7] = [0x00, 0x00, 0xC0, 0xE0, 0xF0, 0xF8, 0xFC];

/// Decodes one UTF-8 sequence from the non-empty `s` without validation,
/// advancing it. The end of `s` cuts the sequence short.
pub(crate) fn next_utf8_char(s: &mut &[u8]) -> u32 {
    let (&first, mut rest) = s.split_first().expect("next_utf8_char on empty input");
    let mut rval = first as u32;
    let extra_bytes = BYTES_FROM_UTF8[first as usize] as usize;
    for _ in 0..extra_bytes {
        rval <<= 6;
        if let Some((&b, tail)) = rest.split_first() {
            rval += b as u32;
            rest = tail;
        }
    }
    *s = rest;
    rval - OFFSETS_FROM_UTF8[extra_bytes]
}

/// The UTF-16 encoding of `c` and its length in units.
pub(crate) fn utf16_units(c: u32) -> ([u16; 2], usize) {
    if c > 0xFFFF {
        let c = c - 0x10000;
        (
            [(0xD800 + c / 0x0400) as u16, (0xDC00 + c % 0x0400) as u16],
            2,
        )
    } else {
        ([c as u16, 0], 1)
    }
}
