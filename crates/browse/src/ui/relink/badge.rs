//! `StatusBadge`: a link's status as a glyph, never colour alone.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_docpath::links::LinkStatus;
use egui::{Painter, Rect};

use super::{LinkListLabels, RelinkStyle};

/// A link's status badge in a 16 box (`link.badge`): Missing a `status.bad.solid` disc with `?`
/// in `text.on_status`; Modified a `status.warn.solid` triangle with `!`; Unreadable a
/// `status.bad.solid` disc with `×`; Refused a `status.bad.solid` disc with `Icon::Ban`; Embedded
/// `Icon::Image` in `text.muted`; Ok nothing. Has no AccessKit node of its own: the row's name
/// carries [`StatusBadge::words`], and the tooltip repeats it.
pub struct StatusBadge;

impl StatusBadge {
    /// Paints the badge for `status` centred in `rect`; `false` for `Ok` (nothing drawn).
    pub fn paint(painter: &Painter, rect: Rect, status: LinkStatus, style: &RelinkStyle) -> bool {
        // vectorcraft: crates/ui-egui/src/panels/links.rs:240 (`badge`).
        todo!()
    }

    /// The status in words, for the row's name and the tooltip ("Missing", "Modified",
    /// "Unreadable", "Not followed: the path is on another computer", "Embedded"); empty for Ok.
    pub fn words(status: LinkStatus, labels: &LinkListLabels) -> String {
        // vectorcraft: crates/ui-egui/src/panels/links.rs:260 (`status_tip`).
        todo!()
    }
}
