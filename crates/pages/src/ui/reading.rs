//! [`ReadingView`] (N3.20): Word's Read Mode: the document reflowed into full-height columns side
//! by side, turned one column at a time with the arrows, the wheel, the page-turn buttons or a
//! swipe, as a book.
//!
//! The app lays its text out into pages of [`reading_size`] and gives them to a
//! [`PageView`](super::view::PageView)-like strip; this module has the geometry and the turning.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Event, Rect, Ui, Vec2};

use super::style::ReadingStyle;
use super::view::PageSource;

/// The column widths Word's View › Column Width offers, page points (Narrow to Wide).
pub const REQUESTED_WIDTH: std::ops::RangeInclusive<f32> = 144.0..=612.0;

/// The most columns side by side (WordCraft).
pub const MAX_COLUMNS: usize = 5;

/// A window resize reflows the columns once the viewport has held still this long, seconds
/// (WordCraft's `RESIZE_SETTLE`).
pub const RESIZE_SETTLE: f64 = 0.16;

craft_ui::words! {
    /// The view's words (Word's Read Mode).
    #[non_exhaustive]
    pub struct ReadingLabels {
        previous: "reading.previous" = "Previous Screen";
        next: "reading.next" = "Next Screen";
    }
}

/// The reading zooms, as Word's View › Column Width offers them (WordCraft clamps 0.75–1.5).
pub const READING_ZOOM: std::ops::RangeInclusive<f32> = 0.75..=1.5;

/// The column size to lay out at, page points, for a viewport `avail` at `scale` screen points
/// per point. With a chosen column width (`requested`, within [`REQUESTED_WIDTH`] page points)
/// that width, at most the window less two gaps; otherwise about one column per
/// `style.column_target` screen points, 1 to [`MAX_COLUMNS`], each at least `style.column_min`.
/// The height leaves the gap and `style.nav_bar` (WordCraft's `READ_NAV_BAR`, #1772).
// wordcraft: crates/ui-egui/src/canvas/reading.rs:49 `reading_size`.
pub fn reading_size(avail: Vec2, scale: f32, style: &ReadingStyle, requested: Option<f32>) -> (f32, f32) {
    todo!()
}

/// The part of `full` the columns scroll in: as many whole columns as fit, centred, so none is
/// cut at the right edge (#1772); half a point of overrun allowed before a column is dropped.
// wordcraft: crates/ui-egui/src/canvas/reading.rs:70 `reading_viewport`.
pub fn reading_viewport(full: Rect, column: f32, gap: f32, scale: f32) -> Rect {
    todo!()
}

/// Whole wheel turns in this pass's events while the view is hovered: one notch is one column
/// (browsers send three lines a notch), touchpad pixels add up in `accum` to a turn every 55 pt;
/// at most 5 either way. ⌘/Ctrl + wheel is zoom, not a turn.
// wordcraft: crates/ui-egui/src/canvas/reading.rs:141 `read_wheel_turns`.
pub fn wheel_turns(events: &[Event], hovered: bool, accum: &mut f32) -> i32 {
    todo!()
}

/// One animation step toward an exact column boundary (time constant 1/18 s); a late frame never
/// overshoots, and the last half point snaps.
// wordcraft: crates/ui-egui/src/canvas/reading.rs:130 `reading_animation_step`.
pub fn animation_step(current: f32, target: f32, dt: f32) -> f32 {
    todo!()
}

/// The reading view's state, kept by the app.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReadingState {
    /// The column at the left edge.
    pub column: usize,
    /// Where turns are going (several ticks queue columns rather than rounding a half-turned
    /// position).
    pub target: Option<usize>,
    pub offset: f32,
    pub wheel: f32,
    /// The text at the start of the first column shown (app-defined position), kept across
    /// reflows instead of the editing caret.
    pub anchor: Option<u64>,
}

/// The columns, built per pass over pages the app laid out at [`reading_size`].
///
/// - Columns `style.column_gap` apart (WordCraft's `GAP`) in a single row, scrolled
///   horizontally inside [`reading_viewport`]; the page-turn buttons (previous, next: craft-ui
///   `IconButton`s named by [`ReadingLabels`]) at the bottom right in the nav bar; ← / → /
///   Page Up / Page Down / space turn a column, Home / End go to the ends (one Tab stop: the
///   view; the buttons are their own).
/// - A resize keeps the columns until [`RESIZE_SETTLE`] has passed, then the app reflows.
/// - The text stays selectable; the app's editing caret doesn't move the columns.
pub struct ReadingView<'a> {
    id_salt: egui::IdSalt,
    source: &'a dyn PageSource,
    state: &'a mut ReadingState,
    zoom: f32,
    labels: Option<ReadingLabels>,
}

/// What the view did this pass.
#[derive(Debug)]
pub struct ReadingViewResponse {
    pub response: egui::Response,
    /// The viewport has held still for [`RESIZE_SETTLE`] after a resize: the app reflows to a new
    /// [`reading_size`] in this pass. While it settles the view asks egui for one repaint at the
    /// settle time (`request_repaint_after`), never a timed poll.
    pub reflow: bool,
}

impl<'a> ReadingView<'a> {
    /// The view's sizes, from the tokens.
    pub fn style(tokens: &craft_ui::theme::UiTokens) -> ReadingStyle {
        ReadingStyle::of(tokens)
    }

    pub fn new(id_salt: impl egui::AsIdSalt, source: &'a dyn PageSource, state: &'a mut ReadingState) -> Self {
        ReadingView { id_salt: egui::IdSalt::new(id_salt), source, state, zoom: 1.0, labels: None }
    }

    pub fn labels(mut self, labels: ReadingLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom;
        self
    }

    // wordcraft: crates/ui-egui/src/canvas/reading.rs:25 `Settle::apply`, :119 `turn_read_page`,
    // :84 `place_read_pages`.
    pub fn show(self, ui: &mut Ui) -> ReadingViewResponse {
        todo!()
    }
}
