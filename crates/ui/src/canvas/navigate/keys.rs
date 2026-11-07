//! Page keys: Photoshop's keyboard scrolling of a canvas, read from the intents a focused canvas takes.

use super::pan;
use craft_behavior::kurbo::{Point, Size, Vec2 as DocVec};
use craft_behavior::{CanvasView, MoveMods, NavMove};
use egui::Vec2;

/// What a page key does to the view.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum PageKey {
    /// Scroll by this many screen points (positive: show what is below or to the right).
    Scroll(Vec2),
    /// Show the document's top-left corner (Home) or bottom-right corner (End).
    Corner {
        /// `true` for End (bottom-right), `false` for Home (top-left).
        end: bool,
    },
}

impl PageKey {
    /// Points a Shift + page key scrolls.
    pub const SMALL_STEP: f32 = 10.0;

    /// Photoshop's page keys for the move a focused canvas took: Page Up and Page Down scroll a canvas height up or
    /// down, Cmd/Ctrl turns them sideways (Page Up left, Page Down right), Shift makes the step
    /// [`SMALL_STEP`](Self::SMALL_STEP); Home and End show the corners. `None` for other moves, with Alt, or with
    /// Shift or Cmd on Home and End. `canvas` is the canvas size in points.
    pub fn of(step: NavMove, mods: MoveMods, canvas: Vec2) -> Option<PageKey> {
        if mods.fine {
            return None;
        }
        let sign = match step {
            NavMove::PageUp => -1.0,
            NavMove::PageDown => 1.0,
            NavMove::Start | NavMove::End if !mods.extend && !mods.lead => return Some(PageKey::Corner { end: step == NavMove::End }),
            _ => return None,
        };
        let sideways = mods.lead;
        let length = match (mods.extend, sideways) {
            (true, _) => Self::SMALL_STEP,
            (false, true) => canvas.x,
            (false, false) => canvas.y,
        };
        Some(PageKey::Scroll(if sideways { Vec2::new(sign * length, 0.0) } else { Vec2::new(0.0, sign * length) }))
    }

    /// Applies the key to `view` for a document of `size`: a scroll, or the corner at the canvas corner the document
    /// runs on from. The zoom stays. With Overscroll off the caller clamps the result with the canvas's scroll
    /// limits. Returns whether the view changed.
    pub fn apply(self, view: &mut CanvasView, size: Size) -> bool {
        let before = *view;
        match self {
            PageKey::Scroll(delta) => pan(view, -delta),
            PageKey::Corner { end } => {
                // The document corner goes to the canvas corner the document runs on from: the top-left one in an
                // upright view, another when it is turned or mirrored.
                let (corner, inward) =
                    if end { (Point::new(size.width, size.height), DocVec::new(-1.0, -1.0)) } else { (Point::ORIGIN, DocVec::new(1.0, 1.0)) };
                let dir = view.map_vec(inward);
                let canvas = view.rect.size();
                let at = Vec2::new(-dir.x.signum() * canvas.x / 2.0, -dir.y.signum() * canvas.y / 2.0);
                let center = corner - view.unmap_vec(at);
                if center.x.is_finite() && center.y.is_finite() {
                    view.center = center;
                }
            }
        }
        *view != before
    }
}
