//! Send only the part of a page that changed since it was last drawn.
//!
//! Pixels are laid out as width and height in little-endian `u32`, then RGBA
//! rows. A host that copies pixels between processes sends a keystroke's
//! worth of change instead of the whole page.

use texyon_preview_protocol::session::PixelRect;

const HEADER_BYTES: usize = 8;
const PIXEL_BYTES: usize = 4;

/// Pages kept to diff against, most recent last. A page that falls out is sent whole.
const RETAINED_PAGES: usize = 8;

/// Width and height of a pixel buffer, if its length matches them.
fn dimensions(pixels: &[u8]) -> Option<(usize, usize)> {
    let width = u32::from_le_bytes(pixels.get(0..4)?.try_into().ok()?);
    let height = u32::from_le_bytes(pixels.get(4..8)?.try_into().ok()?);
    let (width, height) = (usize::try_from(width).ok()?, usize::try_from(height).ok()?);
    let expected = width.checked_mul(height)?.checked_mul(PIXEL_BYTES)?;
    (pixels.len() == HEADER_BYTES.checked_add(expected)?).then_some((width, height))
}

/// The smallest rectangle outside which `new` equals `old`, or `None` when
/// their sizes differ. An empty rectangle means nothing changed.
pub fn changed_rect<'a>(old: &'a [u8], new: &'a [u8]) -> Option<PixelRect> {
    let (width, height) = dimensions(new)?;
    if dimensions(old)? != (width, height) {
        return None;
    }
    let stride = width * PIXEL_BYTES;
    let row = |pixels: &'a [u8], y: usize| -> &'a [u8] {
        &pixels[HEADER_BYTES + y * stride..HEADER_BYTES + (y + 1) * stride]
    };

    let Some(top) = (0..height).find(|&y| row(old, y) != row(new, y)) else {
        return Some(PixelRect::default());
    };
    let bottom = (top..height)
        .rev()
        .find(|&y| row(old, y) != row(new, y))
        .unwrap_or(top);

    let (mut left, mut right) = (width, 0);
    for y in top..=bottom {
        let (before, after) = (row(old, y), row(new, y));
        if let Some(first) = before.iter().zip(after).position(|(a, b)| a != b) {
            let last = before
                .iter()
                .zip(after)
                .rposition(|(a, b)| a != b)
                .unwrap_or(first);
            left = left.min(first / PIXEL_BYTES);
            right = right.max(last / PIXEL_BYTES + 1);
        }
    }

    Some(PixelRect {
        x: u32::try_from(left).ok()?,
        y: u32::try_from(top).ok()?,
        width: u32::try_from(right - left).ok()?,
        height: u32::try_from(bottom + 1 - top).ok()?,
    })
}

/// The pixels inside `rect`, in the same layout as a whole page.
pub fn crop(pixels: &[u8], rect: PixelRect) -> Option<Vec<u8>> {
    let (width, _) = dimensions(pixels)?;
    let [x, y, w, h] = [rect.x, rect.y, rect.width, rect.height].map(usize::try_from);
    let (x, y, w, h) = (x.ok()?, y.ok()?, w.ok()?, h.ok()?);
    let mut out = Vec::with_capacity(HEADER_BYTES + w * h * PIXEL_BYTES);
    out.extend_from_slice(&rect.width.to_le_bytes());
    out.extend_from_slice(&rect.height.to_le_bytes());
    let stride = width * PIXEL_BYTES;
    for row in y..y + h {
        let start = HEADER_BYTES + row * stride + x * PIXEL_BYTES;
        out.extend_from_slice(pixels.get(start..start + w * PIXEL_BYTES)?);
    }
    Some(out)
}

/// What each recently drawn page looked like, and at which scale.
#[derive(Default)]
pub struct DrawnPages {
    pages: Vec<(usize, u32, Vec<u8>)>,
}

impl DrawnPages {
    /// Remember a page as the client now has it, returning what it had before
    /// at the same scale.
    pub fn replace(&mut self, index: usize, scale: f32, pixels: Vec<u8>) -> Option<Vec<u8>> {
        let scale = scale.to_bits();
        let previous = self
            .pages
            .iter()
            .position(|(page, _, _)| *page == index)
            .map(|at| self.pages.remove(at))
            .filter(|(_, drawn_at, _)| *drawn_at == scale)
            .map(|(_, _, pixels)| pixels);
        if self.pages.len() >= RETAINED_PAGES {
            self.pages.remove(0);
        }
        self.pages.push((index, scale, pixels));
        previous
    }
}

#[cfg(test)]
mod tests {
    use super::{changed_rect, crop, DrawnPages, PixelRect};

    /// A `width` by `height` page of one colour, with `marks` set to another.
    fn page(width: u32, height: u32, marks: &[(u32, u32)]) -> Vec<u8> {
        let mut pixels = width.to_le_bytes().to_vec();
        pixels.extend_from_slice(&height.to_le_bytes());
        for y in 0..height {
            for x in 0..width {
                let value = if marks.contains(&(x, y)) { 0 } else { 255 };
                pixels.extend_from_slice(&[value, value, value, 255]);
            }
        }
        pixels
    }

    #[test]
    fn finds_the_bounding_box_of_every_change() {
        let old = page(6, 5, &[]);
        let new = page(6, 5, &[(1, 1), (4, 3)]);
        assert_eq!(
            changed_rect(&old, &new),
            Some(PixelRect {
                x: 1,
                y: 1,
                width: 4,
                height: 3
            })
        );
    }

    #[test]
    fn identical_pages_change_nothing_and_resized_ones_have_no_diff() {
        let old = page(4, 4, &[(2, 2)]);
        assert_eq!(changed_rect(&old, &old), Some(PixelRect::default()));
        assert_eq!(changed_rect(&old, &page(4, 5, &[])), None);
        assert_eq!(changed_rect(&old[..9], &old), None);
    }

    #[test]
    fn a_crop_pasted_over_the_old_page_gives_the_new_one() {
        let old = page(7, 6, &[(0, 0)]);
        let new = page(7, 6, &[(0, 0), (2, 4), (5, 2)]);
        let rect = changed_rect(&old, &new).unwrap();
        let patch = crop(&new, rect).unwrap();

        let mut pasted = old.clone();
        let (w, x, y) = (rect.width as usize, rect.x as usize, rect.y as usize);
        for row in 0..rect.height as usize {
            let from = 8 + row * w * 4;
            let to = 8 + ((y + row) * 7 + x) * 4;
            pasted[to..to + w * 4].copy_from_slice(&patch[from..from + w * 4]);
        }
        assert_eq!(pasted, new);
    }

    #[test]
    fn only_a_page_drawn_at_the_same_scale_is_a_base() {
        let mut drawn = DrawnPages::default();
        assert_eq!(drawn.replace(3, 2.0, vec![1]), None);
        assert_eq!(drawn.replace(3, 2.0, vec![2]), Some(vec![1]));
        assert_eq!(drawn.replace(3, 1.5, vec![3]), None);
        assert_eq!(drawn.replace(3, 1.5, vec![4]), Some(vec![3]));
    }

    #[test]
    fn the_oldest_page_is_forgotten_first() {
        let mut drawn = DrawnPages::default();
        for index in 0..=super::RETAINED_PAGES {
            drawn.replace(index, 1.0, vec![]);
        }
        assert_eq!(drawn.replace(0, 1.0, vec![]), None);
        assert_eq!(
            drawn.replace(super::RETAINED_PAGES, 1.0, vec![]),
            Some(vec![])
        );
    }
}
