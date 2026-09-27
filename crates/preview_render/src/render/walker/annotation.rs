//! Link rectangles accumulated between `pdf:bann` and `pdf:eann`.

use crate::links::{parse_link_action, Link, LinkAction, RectBp};
use crate::resource::ResourceProvider;

use super::super::SP_TO_BP;
use super::Walker;

/// How far above its baseline a link's box is taken to reach, as a fraction of
/// the font size. Roughly a capital's height plus the room an ascender needs.
const LINK_ASCENT: f32 = 0.75;

/// How far below the baseline, which is a descender's depth.
const LINK_DESCENT: f32 = 0.25;

/// Extent above the baseline for text of this size, in DVI units.
pub(super) fn ascent(size: i32) -> i32 {
    (size as f32 * LINK_ASCENT) as i32
}

/// Extent below the baseline for text of this size, in DVI units.
pub(super) fn descent(size: i32) -> i32 {
    (size as f32 * LINK_DESCENT) as i32
}

/// An annotation whose extent is still being discovered.
///
/// A link's rectangle is stated nowhere: it is whatever was typeset between
/// `pdf:bann` and `pdf:eann`, so it has to be accumulated as the page is
/// walked. Deliberately not part of [`WalkResume`] — an annotation cannot
/// cross a page boundary, and one left open at the end of a page is closed
/// there rather than carried into the next.
pub(super) struct OpenLink {
    action: LinkAction,
    /// Rectangles closed so far, one per line the link was broken across.
    closed: Vec<RectSp>,
    /// The rectangle still growing.
    current: Option<RectSp>,
}

/// A rectangle in DVI units, before it becomes a page coordinate.
#[derive(Clone, Copy)]
struct RectSp {
    left: i32,
    right: i32,
    top: i32,
    bottom: i32,
    /// Baseline it sits on, for telling a new line from a wider one.
    baseline: i32,
}

impl<P: ResourceProvider> Walker<'_, P> {
    /// Grow the open annotation to cover something typeset inside it.
    ///
    /// `above` and `below` are the extent either side of the baseline. They are
    /// taken from the font size rather than from glyph metrics: an overlay
    /// wants a box a cursor can hit, and reading real ascents would mean
    /// loading every face at analysis time to place a rectangle nobody sees.
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

        let (left, right) = (left.min(right), left.max(right));
        let top = baseline - above;
        let bottom = baseline + below;

        match open.current.as_mut() {
            // A baseline more than a line away is a line break, and a link
            // broken across one wants a box per line rather than one box
            // swallowing the text between them.
            Some(rect) if (baseline - rect.baseline).abs() > above.max(1) => {
                open.closed.push(*rect);
                open.current = Some(RectSp {
                    left,
                    right,
                    top,
                    bottom,
                    baseline,
                });
            }
            Some(rect) => {
                rect.left = rect.left.min(left);
                rect.right = rect.right.max(right);
                rect.top = rect.top.min(top);
                rect.bottom = rect.bottom.max(bottom);
            }
            None => {
                open.current = Some(RectSp {
                    left,
                    right,
                    top,
                    bottom,
                    baseline,
                });
            }
        }
    }

    /// Begin an annotation, if this dictionary describes a link that goes
    /// somewhere.
    pub(super) fn begin_link(&mut self, args: &str, x: i32, y: i32) {
        // A dangling `pdf:bann` would otherwise absorb the rest of the page.
        self.close_link();

        let Some(action) = parse_link_action(args) else {
            return;
        };

        self.open_link = Some(OpenLink {
            action,
            closed: Vec::new(),
            // Seeded with the special's own position: it sits exactly at the
            // left edge of what it introduces, which glyph positions alone
            // would only approximate.
            current: Some(RectSp {
                left: x,
                right: x,
                top: y,
                bottom: y,
                baseline: y,
            }),
        });
    }

    /// Close the open annotation, committing its rectangles to the page.
    pub(super) fn close_link(&mut self) {
        let Some(mut open) = self.open_link.take() else {
            return;
        };

        if let Some(rect) = open.current.take() {
            open.closed.push(rect);
        }

        let Ok(page) = self.page_mut() else {
            return;
        };

        for rect in open.closed {
            // A rectangle with no area is a link around something that was
            // never typeset, and nothing can click on it.
            if rect.right <= rect.left || rect.bottom <= rect.top {
                continue;
            }
            page.links.push(Link {
                rect: RectBp {
                    left: rect.left as f32 * SP_TO_BP,
                    top: rect.top as f32 * SP_TO_BP,
                    right: rect.right as f32 * SP_TO_BP,
                    bottom: rect.bottom as f32 * SP_TO_BP,
                },
                action: open.action.clone(),
            });
        }
    }
}
