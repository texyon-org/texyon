//! Link rectangles accumulated between `pdf:bann` and `pdf:eann`.

use crate::links::{parse_link_action, Link, LinkAction, RectBp};
use crate::resource::ResourceProvider;
use crate::units::sp_to_bp;

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
    closed: Vec<RectSp>,
    current: Option<RectSp>,
}

/// A rectangle in DVI units.
#[derive(Clone, Copy)]
struct RectSp {
    left: i32,
    right: i32,
    top: i32,
    bottom: i32,
    baseline: i32,
}

impl RectSp {
    fn around(left: i32, right: i32, baseline: i32, above: i32, below: i32) -> Self {
        Self {
            left: left.min(right),
            right: left.max(right),
            top: baseline.wrapping_sub(above),
            bottom: baseline.wrapping_add(below),
            baseline,
        }
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
        let Some(open) = self.open_link.as_mut() else {
            return;
        };
        let rect = RectSp::around(left, right, baseline, above, below);
        match open.current.as_mut() {
            Some(current)
                if baseline.wrapping_sub(current.baseline).wrapping_abs() <= above.max(1) =>
            {
                current.grow(rect);
            }
            // A new line starts a new box instead of swallowing the text between.
            _ => open.closed.extend(open.current.replace(rect)),
        }
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
            current: Some(RectSp::around(x, x, y, 0, 0)),
        });
    }

    /// Commit the open link's rectangles to the page.
    pub(super) fn close_link(&mut self) {
        let Some(mut open) = self.open_link.take() else {
            return;
        };
        open.closed.extend(open.current.take());

        let Ok(page) = self.page_mut() else {
            return;
        };

        for rect in open.closed {
            if rect.right <= rect.left || rect.bottom <= rect.top {
                continue;
            }
            page.links.push(Link {
                rect: RectBp {
                    left: sp_to_bp(rect.left),
                    top: sp_to_bp(rect.top),
                    right: sp_to_bp(rect.right),
                    bottom: sp_to_bp(rect.bottom),
                },
                action: open.action.clone(),
            });
        }
    }
}
