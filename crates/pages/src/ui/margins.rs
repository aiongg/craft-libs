//! [`MarginHandles`] (N3.11): the page margins as draggable edges on the rulers (Word, Excel's
//! Page Layout view): the grey part of each ruler is the margin, the white the text area; the
//! pointer over an edge shows a resize cursor and the margin's name, and a drag moves the margin
//! as one edit gesture (one undo step).
//!
//! ```ignore
//! let r = MarginHandles::new("margins", &placed, &setup).header_footer(Some((36.0, 36.0))).show(ui, top_ruler, left_ruler);
//! match r.edit {
//!     Some(EditEvent::Commit { value, .. }) => doc.set_margin(value.edge, value.points),
//!     Some(EditEvent::Update { value, .. }) => doc.preview_margin(value.edge, value.points),
//!     _ => {}
//! }
//! ```

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::edit::EditEvent;
use egui::{Rect, Ui};

use super::view::PlacedPage;
use crate::setup::Margins;

/// The least text width (or height) a margin drag leaves, points: one inch (Word). GridCraft
/// leaves 36 pt; an app sets its own with [`MarginHandles::min_text`].
pub const MIN_TEXT: f64 = 72.0;

/// The step a dragged margin snaps to, points: 1/16 inch (Word). Alt drags freely.
pub const SNAP: f64 = 4.5;

/// Screen points either side of an edge that grab it (GridCraft's `EDGE_GRAB`).
pub const EDGE_GRAB: f32 = 4.0;

craft_ui::words! {
    /// The edges' names, shown as tooltips and AccessKit names.
    #[non_exhaustive]
    pub struct MarginLabels {
        left: "pages.leftMargin" = "Left Margin";
        right: "pages.rightMargin" = "Right Margin";
        top: "pages.topMargin" = "Top Margin";
        bottom: "pages.bottomMargin" = "Bottom Margin";
        header: "pages.headerMargin" = "Header Margin";
        footer: "pages.footerMargin" = "Footer Margin";
    }
}

/// An edge a margin drag moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MarginEdge {
    Left,
    Right,
    Top,
    Bottom,
    /// The header's distance from the top edge.
    Header,
    /// The footer's distance from the bottom edge.
    Footer,
}

/// A margin's new value: the stored edge (on an even page of mirrored margins the visible left
/// edge is the stored right margin) and its points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarginChange {
    pub edge: MarginEdge,
    pub points: f64,
}

/// What a margin drag works against: the page, its margins and the drag's rules.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarginRules {
    /// Page width and height, points.
    pub page: (f64, f64),
    pub margins: Margins,
    pub gutter: f64,
    /// An even page of mirrored margins.
    pub mirrored_even: bool,
    /// The snap step, points; 0 for none.
    pub snap: f64,
    /// The least text width or height left, points.
    pub min_text: f64,
}

/// The margin a drag of the visible `edge` to `at` points from the page's left (or top) edge
/// gives: snapped to `rules.snap` (none when 0), never negative, leaving `rules.min_text` between
/// it and the opposite margin (and the gutter). On an even page of mirrored margins the pointer
/// is reflected across the page and the edge swapped (WordCraft #2914).
// wordcraft: crates/ui-egui/src/canvas/margins.rs:37 `dragged_margin`, :54 `margin_drag`;
// gridcraft: crates/ui-egui/src/paper_view.rs:695 `margin_from_drag`.
pub fn dragged_margin(edge: MarginEdge, at: f64, rules: &MarginRules) -> MarginChange {
    todo!()
}

/// What the handles did this pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MarginResponse {
    pub edit: Option<EditEvent<MarginChange>>,
}

/// The margin edges of one page (the page the caret is on, or the first page in view) on the
/// horizontal and vertical ruler strips.
///
/// - Each edge grabs within [`EDGE_GRAB`] of it on its ruler; the cursor is ResizeHorizontal or
///   ResizeVertical; the tooltip names it, and while dragging shows the value in the unit
///   ("1.25 in", "3.18 cm").
/// - A drag reports Begin, Update per move and Commit on release, [`dragged_margin`]'s value;
///   Esc cancels. The view doesn't follow the caret during the drag (the page would move under the
///   pointer).
/// - Not Tab stops (Page Setup and the Layout menus are the keyboard path); each edge is an
///   AccessKit Splitter named by [`MarginLabels`].
pub struct MarginHandles<'a> {
    id_salt: egui::IdSalt,
    page: &'a PlacedPage,
    margins: Margins,
    gutter: f64,
    mirrored_even: bool,
    header_footer: Option<(f64, f64)>,
    snap: f64,
    min_text: f64,
    labels: Option<MarginLabels>,
}

impl<'a> MarginHandles<'a> {
    /// The handles of `page`, which has `margins`.
    pub fn new(id_salt: impl egui::AsIdSalt, page: &'a PlacedPage, margins: Margins) -> Self {
        MarginHandles {
            id_salt: egui::IdSalt::new(id_salt),
            page,
            margins,
            gutter: 0.0,
            mirrored_even: false,
            header_footer: None,
            snap: SNAP,
            min_text: MIN_TEXT,
            labels: None,
        }
    }

    pub fn gutter(mut self, gutter: f64) -> Self {
        self.gutter = gutter;
        self
    }

    /// The page is an even page of mirrored margins.
    pub fn mirrored_even(mut self, on: bool) -> Self {
        self.mirrored_even = on;
        self
    }

    /// The header and footer distances, as two more edges on the vertical ruler (Excel).
    pub fn header_footer(mut self, distances: Option<(f64, f64)>) -> Self {
        self.header_footer = distances;
        self
    }

    pub fn snap(mut self, points: f64) -> Self {
        self.snap = points;
        self
    }

    pub fn min_text(mut self, points: f64) -> Self {
        self.min_text = points;
        self
    }

    pub fn labels(mut self, labels: MarginLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draw the margin shading on the two ruler strips and run the drags.
    // wordcraft: crates/ui-egui/src/canvas/margins.rs:67 `margin_handle`; gridcraft:
    // crates/ui-egui/src/paper_view.rs:779 `ruler_margins`.
    pub fn show(self, ui: &mut Ui, horizontal: Rect, vertical: Rect) -> MarginResponse {
        todo!()
    }
}
