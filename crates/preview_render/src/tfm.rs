//! Minimal, bounds-checked TeX Font Metric parsing for DVI advances.

#[derive(Clone)]
pub(crate) struct Tfm {
    first_char: i32,
    last_char: i32,
    char_info: Vec<u32>,
    widths: Vec<i32>,
}

impl Tfm {
    pub(crate) fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 24 {
            return None;
        }
        let values: Vec<usize> = (0..12)
            .map(|index| {
                let offset = index * 2;
                u16::from_be_bytes([bytes[offset], bytes[offset + 1]]) as usize
            })
            .collect();
        let [lf, lh, bc, ec, nw, nh, nd, ni, nl, nk, ne, np] = values.as_slice() else {
            return None;
        };
        if *lh < 2 || *ec > 255 || *ne > 256 || *bc > ec + 1 {
            return None;
        }
        let char_count = if bc > ec { 0 } else { ec - bc + 1 };
        let expected = 6 + lh + char_count + nw + nh + nd + ni + nl + nk + ne + np;
        if expected != *lf || bytes.len() < lf * 4 {
            return None;
        }
        let char_start = 6 + lh;
        let mut char_info = Vec::with_capacity(char_count);
        for word in char_start..char_start + char_count {
            char_info.push(read_word(bytes, word)? as u32);
        }
        let mut widths = Vec::with_capacity(*nw);
        for word in char_start + char_count..char_start + char_count + nw {
            widths.push(read_word(bytes, word)?);
        }
        Some(Self {
            first_char: *bc as i32,
            last_char: *ec as i32,
            char_info,
            widths,
        })
    }

    pub(crate) fn advance(&self, character: i32, scale: u32) -> i32 {
        if character < self.first_char || character > self.last_char {
            return 0;
        }
        let info = self.char_info[(character - self.first_char) as usize];
        let width_index = (info >> 24) as usize;
        let width = self.widths.get(width_index).copied().unwrap_or(0);
        ((width as i64 * scale as i64) >> 20) as i32
    }
}

fn read_word(bytes: &[u8], word: usize) -> Option<i32> {
    let offset = word.checked_mul(4)?;
    Some(i32::from_be_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn synthetic_tfm() -> Vec<u8> {
        let (lh, bc, ec, nw) = (2usize, 65usize, 65usize, 2usize);
        let (nh, nd, ni, nl, nk, ne, np) = (1usize, 1usize, 1usize, 0, 0, 0, 0);
        let lf = 6 + lh + 1 + nw + nh + nd + ni + nl + nk + ne + np;
        let mut bytes = vec![0; lf * 4];
        for (index, value) in [lf, lh, bc, ec, nw, nh, nd, ni, nl, nk, ne, np]
            .into_iter()
            .enumerate()
        {
            bytes[index * 2..index * 2 + 2].copy_from_slice(&(value as u16).to_be_bytes());
        }
        let char_word = 6 + lh;
        bytes[char_word * 4..char_word * 4 + 4].copy_from_slice(&(1u32 << 24).to_be_bytes());
        let width_word = char_word + 1;
        bytes[(width_word + 1) * 4..(width_word + 2) * 4]
            .copy_from_slice(&(1i32 << 19).to_be_bytes());
        bytes
    }

    #[test]
    fn parses_width_indirection_and_scales_advance() {
        let tfm = Tfm::parse(&synthetic_tfm()).unwrap();
        assert_eq!(tfm.advance(65, 10 << 16), 5 << 16);
        assert_eq!(tfm.advance(66, 10 << 16), 0);
    }
}
