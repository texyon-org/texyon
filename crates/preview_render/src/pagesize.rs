//! Per-page media sizes with xdvipdfmx-compatible persistence rules.

#[cfg(test)]
use std::io::Cursor;

#[cfg(test)]
use tectonic_xdv::{XdvEvents, XdvParser};

use crate::units::{normalize_zero, SP_TO_BP};
use crate::RenderError;

pub(crate) const DEFAULT_BP: [f32; 2] = [612.0, 792.0];

/// Magnification declared by an XDV preamble.
pub(crate) fn preamble_magnification(xdv: &[u8]) -> Result<f32, RenderError> {
    if xdv.len() < 14 || xdv[0] != 247 {
        return Err(RenderError::Xdv("missing XDV preamble".into()));
    }
    let raw_mag = u32::from_be_bytes([xdv[10], xdv[11], xdv[12], xdv[13]]);
    Ok(if raw_mag == 0 {
        1.0
    } else {
        raw_mag as f32 / 1000.0
    })
}

#[cfg(test)]
pub(crate) fn scan_page_sizes(xdv: &[u8]) -> Result<Vec<[f32; 2]>, RenderError> {
    let mag = preamble_magnification(xdv)?;
    let (scanner, _) = XdvParser::process(Cursor::new(xdv), SizeScanner::new(mag))?;
    Ok(scanner.finish())
}

pub(crate) fn is_page_size_special(contents: &[u8]) -> bool {
    let (namespace, keyword, _) = special_head(contents);
    (namespace == Some(b"pdf".as_slice()) && keyword == Some(b"pagesize".as_slice()))
        || keyword == Some(b"papersize".as_slice())
}

/// Forward-only media-size state machine for one XDV walk.
///
/// A page's size is whatever the specials inside it leave in effect, so the
/// value is only final at end of page. Keeping it as a state machine lets the
/// analysis walk derive page sizes itself instead of paying for a second
/// complete parse of the document.
#[derive(Clone, Debug)]
pub(crate) struct PageSizeState {
    mag: f32,
    default_size: [f32; 2],
    current_size: [f32; 2],
    completed_pages: usize,
}

impl PageSizeState {
    pub(crate) fn new(mag: f32) -> Self {
        Self {
            mag,
            default_size: DEFAULT_BP,
            current_size: DEFAULT_BP,
            completed_pages: 0,
        }
    }

    /// Media size in effect for the page being walked.
    pub(crate) fn current(&self) -> [f32; 2] {
        self.current_size
    }

    /// Start a page from the persistent default.
    pub(crate) fn begin_page(&mut self) {
        self.current_size = self.default_size;
    }

    /// Retire the page whose size was just read.
    pub(crate) fn finish_page(&mut self) {
        self.completed_pages += 1;
    }

    pub(crate) fn apply(&mut self, contents: &[u8]) {
        let (namespace, keyword, rest) = special_head(contents);
        let first_page = self.completed_pages == 0;
        if namespace == Some(b"pdf") && keyword == Some(b"pagesize") {
            let mut size = self.current_size;
            let mut input = skip_ws(rest);
            loop {
                let (key, after_key) = parse_ident(input);
                let Some(key) = key else { break };
                if key == b"default" {
                    size = self.default_size;
                    input = skip_ws(after_key);
                    continue;
                }
                if !matches!(key, b"width" | b"height" | b"xoffset" | b"yoffset") {
                    break;
                }
                let Some((value, after_value)) = parse_length(skip_ws(after_key), self.mag) else {
                    return;
                };
                match key {
                    b"width" => size[0] = normalize_zero(value * self.mag),
                    b"height" => size[1] = normalize_zero(value * self.mag),
                    _ => {}
                }
                input = skip_ws(after_value);
            }
            self.current_size = size;
            if first_page {
                self.default_size = size;
            }
        } else if keyword == Some(b"papersize") {
            let mut input = skip_ws(rest);
            if input.first() == Some(&b'=') {
                input = skip_ws(&input[1..]);
            }
            if matches!(input.first(), Some(b'\'' | b'"')) {
                input = skip_ws(&input[1..]);
            }
            let Some((width, after_width)) = parse_length(input, 1.0) else {
                return;
            };
            let mut input = skip_ws(after_width);
            if input.first() == Some(&b',') {
                input = skip_ws(&input[1..]);
            }
            let Some((height, _)) = parse_length(input, 1.0) else {
                return;
            };
            let size = [normalize_zero(width), normalize_zero(height)];
            self.current_size = size;
            self.default_size = size;
        }
    }
}

#[cfg(test)]
struct SizeScanner {
    state: PageSizeState,
    page_open: bool,
    sizes: Vec<[f32; 2]>,
}

#[cfg(test)]
impl SizeScanner {
    fn new(mag: f32) -> Self {
        Self {
            state: PageSizeState::new(mag),
            page_open: false,
            sizes: Vec::new(),
        }
    }

    fn finish(mut self) -> Vec<[f32; 2]> {
        if self.page_open {
            self.sizes.push(self.state.current());
        }
        self.sizes
    }
}

#[cfg(test)]
impl XdvEvents for SizeScanner {
    type Error = RenderError;

    fn handle_begin_page(
        &mut self,
        _counters: &[i32],
        _previous_bop: i32,
    ) -> Result<(), Self::Error> {
        if self.page_open {
            self.sizes.push(self.state.current());
            self.state.finish_page();
        }
        self.page_open = true;
        self.state.begin_page();
        Ok(())
    }

    fn handle_special(&mut self, _x: i32, _y: i32, contents: &[u8]) -> Result<(), Self::Error> {
        if self.page_open {
            self.state.apply(contents);
        }
        Ok(())
    }
}

fn special_head(input: &[u8]) -> (Option<&[u8]>, Option<&[u8]>, &[u8]) {
    let (first, mut rest) = parse_ident(skip_ws(input));
    let Some(first) = first else {
        return (None, None, rest);
    };
    if matches!(first, b"pdf" | b"x" | b"dvipdfmx") {
        rest = skip_ws(rest);
        if rest.first() == Some(&b':') {
            let (keyword, rest) = parse_ident(skip_ws(&rest[1..]));
            return (Some(first), keyword, rest);
        }
    }
    (None, Some(first), rest)
}

fn skip_ws(input: &[u8]) -> &[u8] {
    let count = input
        .iter()
        .take_while(|byte| matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0 | 0x0c))
        .count();
    &input[count..]
}

fn parse_ident(input: &[u8]) -> (Option<&[u8]>, &[u8]) {
    if !input
        .first()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
    {
        return (None, input);
    }
    let count = input
        .iter()
        .take_while(|byte| byte.is_ascii_alphanumeric() || **byte == b'_')
        .count();
    (Some(&input[..count]), &input[count..])
}

fn parse_number(input: &[u8]) -> Option<(f32, &[u8])> {
    let mut end = usize::from(matches!(input.first(), Some(b'+' | b'-')));
    let digits_start = end;
    while input.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    if input.get(end) == Some(&b'.') {
        end += 1;
        while input.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
    }
    if end == digits_start {
        return None;
    }
    let value = std::str::from_utf8(&input[..end]).ok()?.parse().ok()?;
    Some((value, &input[end..]))
}

fn parse_length(input: &[u8], mag: f32) -> Option<(f32, &[u8])> {
    let (value, after_number) = parse_number(input)?;
    let (unit, mut rest) = parse_ident(skip_ws(after_number));
    let Some(mut unit) = unit else {
        return Some((value, after_number));
    };
    let mut true_scale = 1.0;
    if let Some(suffix) = unit.strip_prefix(b"true") {
        true_scale /= mag;
        unit = suffix;
        if unit.is_empty() {
            let (following, after_unit) = parse_ident(skip_ws(rest));
            unit = following?;
            rest = after_unit;
        }
    }
    let scale = match unit {
        b"bp" => 1.0,
        b"pt" => 72.0 / 72.27,
        b"in" => 72.0,
        b"cm" => 72.0 / 2.54,
        b"mm" => 72.0 / 25.4,
        b"pc" => 12.0 * 72.0 / 72.27,
        b"dd" => 1238.0 / 1157.0 * 72.0 / 72.27,
        b"cc" => 12.0 * 1238.0 / 1157.0 * 72.0 / 72.27,
        b"sp" => SP_TO_BP,
        _ => return None,
    };
    Some((value * true_scale * scale, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xdv_with_specials(specials: &[&[&str]], mag: u32) -> Vec<u8> {
        let mut bytes = vec![247, 7];
        bytes.extend_from_slice(&25_400_000u32.to_be_bytes());
        bytes.extend_from_slice(&473_628_672u32.to_be_bytes());
        bytes.extend_from_slice(&mag.to_be_bytes());
        bytes.push(0);
        for page in specials {
            bytes.push(139);
            bytes.extend_from_slice(&[0; 40]);
            bytes.extend_from_slice(&(-1i32).to_be_bytes());
            for special in *page {
                bytes.push(239);
                bytes.push(special.len() as u8);
                bytes.extend_from_slice(special.as_bytes());
            }
            bytes.push(140);
        }
        bytes.push(248);
        bytes.extend_from_slice(&0u32.to_be_bytes());
        bytes.extend_from_slice(&25_400_000u32.to_be_bytes());
        bytes.extend_from_slice(&473_628_672u32.to_be_bytes());
        bytes.extend_from_slice(&mag.to_be_bytes());
        bytes.extend_from_slice(&[0; 8]);
        bytes.extend_from_slice(&2u16.to_be_bytes());
        bytes.extend_from_slice(&(specials.len() as u16).to_be_bytes());
        bytes.push(249);
        bytes.extend_from_slice(&0u32.to_be_bytes());
        bytes.push(7);
        bytes.extend_from_slice(&[223; 4]);
        bytes
    }

    fn close(actual: [f32; 2], expected: [f32; 2]) {
        assert!((actual[0] - expected[0]).abs() < 0.01, "{actual:?}");
        assert!((actual[1] - expected[1]).abs() < 0.01, "{actual:?}");
    }

    #[test]
    fn first_pdf_size_becomes_default_but_later_pdf_size_is_local() {
        let xdv = xdv_with_specials(
            &[
                &["pdf:pagesize width 400bp height 500bp"],
                &[],
                &["pdf:pagesize width 200bp height 300bp"],
                &[],
            ],
            1000,
        );
        let sizes = scan_page_sizes(&xdv).unwrap();
        assert_eq!(sizes.len(), 4);
        close(sizes[0], [400.0, 500.0]);
        close(sizes[1], [400.0, 500.0]);
        close(sizes[2], [200.0, 300.0]);
        close(sizes[3], [400.0, 500.0]);
    }

    #[test]
    fn dvips_papersize_persists_and_units_are_converted() {
        let xdv = xdv_with_specials(&[&["papersize=8.5in,11in"], &[]], 1000);
        let sizes = scan_page_sizes(&xdv).unwrap();
        close(sizes[0], [612.0, 792.0]);
        close(sizes[1], [612.0, 792.0]);
    }

    #[test]
    fn true_dimensions_cancel_magnification() {
        let xdv = xdv_with_specials(&[&["pdf:pagesize width 2truein height 3truein"]], 2000);
        close(scan_page_sizes(&xdv).unwrap()[0], [144.0, 216.0]);
    }
}
