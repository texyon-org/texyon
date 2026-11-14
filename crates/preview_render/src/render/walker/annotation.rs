//! Link rectangles accumulated between `pdf:bann` and `pdf:eann`.

use crate::links::{Link, LinkAction, RectBp, parse_link_action};
use crate::resource::ResourceProvider;

use super::Walker;

/// Link box extent above the baseline, as a fraction of the font size.
const LINK_ASCENT: f32 = 0.75;

/// Link box extent below the baseline, as a fraction of the font size.
const LINK_DESCENT: f32 = 0.25;

pub(super) fn ascent(size: i32) -> i32 {
    (size as f32 * LINK_ASCENT) as i32
}

pub(super) fn descent(size: i32) -> i32 {
    (size as f32 * LINK_DESCENT) as i32
}

/// A link whose extent is whatever gets typeset before its `pdf:eann`.
pub(super) struct OpenLink {
    action: LinkAction,
    /// One rectangle per line the link was broken across.
    closed: Vec<RectBp>,
    current: Option<LinkLine>,
}

/// The part of a link on one line, grouped by its DVI baseline.
struct LinkLine {
    baseline: i32,
    rect: RectBp,
}

impl RectBp {
    fn around_points(points: impl IntoIterator<Item = (f32, f32)>) -> Self {
        let mut rect = Self {
            left: f32::INFINITY,
            top: f32::INFINITY,
            right: f32::NEG_INFINITY,
            bottom: f32::NEG_INFINITY,
        };
        for (x, y) in points {
            rect.left = rect.left.min(x);
            rect.right = rect.right.max(x);
            rect.top = rect.top.min(y);
            rect.bottom = rect.bottom.max(y);
        }
        rect
    }

    fn grow(&mut self, other: Self) {
        self.left = self.left.min(other.left);
        self.right = self.right.max(other.right);
        self.top = self.top.min(other.top);
        self.bottom = self.bottom.max(other.bottom);
    }
}

impl<P: ResourceProvider> Walker<'_, P> {
    /// Grow the open link over something typeset inside it.
    ///
    /// The vertical extent comes from the font size, since loading faces for
    /// real ascents is not worth it for a click target.
    pub(super) fn extend_link(
        &mut self,
        left: i32,
        right: i32,
        baseline: i32,
        above: i32,
        below: i32,
    ) {
        if self.open_link.is_none() {
            return;
        }
        let rect = self.page_rect(left, right, baseline, above, below);
        let Some(open) = self.open_link.as_mut() else {
            return;
        };
        match open.current.as_mut() {
            Some(current)
                if baseline.wrapping_sub(current.baseline).wrapping_abs() <= above.max(1) =>
            {
                current.rect.grow(rect);
            }
            // A new line starts a new box instead of swallowing the text between.
            _ => open.closed.extend(
                open.current
                    .replace(LinkLine { baseline, rect })
                    .map(|line| line.rect),
            ),
        }
    }

    /// Where a DVI box lands on the page, through any active transform.
    fn page_rect(&self, left: i32, right: i32, baseline: i32, above: i32, below: i32) -> RectBp {
        let top = baseline.wrapping_sub(above);
        let bottom = baseline.wrapping_add(below);
        let graphics = &self.state.pdf_graphics;
        RectBp::around_points(
            [(left, top), (right, top), (left, bottom), (right, bottom)]
                .map(|(x, y)| graphics.page_point(x, y)),
        )
    }

    /// Begin a link if this annotation dictionary goes somewhere.
    pub(super) fn begin_link(&mut self, args: &str, x: i32, y: i32) {
        // A dangling `pdf:bann` would otherwise absorb the rest of the page.
        self.close_link();

        let Some(action) = parse_link_action(args) else {
            return;
        };

        self.open_link = Some(OpenLink {
            action,
            closed: Vec::new(),
            // The special sits exactly at the left edge of what it introduces.
            current: Some(LinkLine {
                baseline: y,
                rect: self.page_rect(x, x, y, 0, 0),
            }),
        });
    }

    /// Commit the open link's rectangles to the page.
    pub(super) fn close_link(&mut self) {
        let Some(mut open) = self.open_link.take() else {
            return;
        };
        open.closed
            .extend(open.current.take().map(|line| line.rect));

        let Ok(page) = self.page_mut() else {
            return;
        };

        for rect in open.closed {
            if rect.right <= rect.left || rect.bottom <= rect.top {
                continue;
            }
            page.links.push(Link {
                rect,
                action: open.action.clone(),
            });
        }
    }
}
