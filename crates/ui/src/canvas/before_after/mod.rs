//! Before and after views of an edit: side by side, stacked, wipes and hold-to-show-original
//! ([`BeforeAfter`]).
//!
//! The layout and the divider are the library's; the pictures are the host's. The host fits its
//! image into a pane ([`BeforeAfter::panes`]), then [`show`](BeforeAfter::show) senses the divider
//! and calls the host's painter once per side, clipped to that side, and draws the divider line,
//! its knob and the captions over them.
//!
//! ```no_run
//! use craft_ui::canvas::before_after::{BeforeAfter, BeforeAfterMode, BeforeAfterSide};
//! # fn fit(pane: egui::Rect) -> egui::Rect { pane }
//! # fn demo(ui: &mut egui::Ui, area: egui::Rect, split: &mut f32) {
//! let compare = BeforeAfter::new("develop-compare", BeforeAfterMode::Split).fraction(*split).original_while(egui::Key::Backslash);
//! let [_, after_pane] = compare.panes(ui, area);
//! let image = fit(after_pane);
//! let out = compare.show(ui, area, image, |_painter, pane| match pane.side {
//!     BeforeAfterSide::Before => { /* paint the original into pane.image */ }
//!     BeforeAfterSide::After => { /* paint the edited image into pane.image */ }
//! });
//! // View state, not an undo step: keep the latest value.
//! if let Some(value) = out.fraction.and_then(|e| e.value().copied()) {
//!     *split = value;
//! }
//! # }
//! ```

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

pub(crate) mod stories;
mod style;
#[cfg(test)]
mod tests;

use egui::{AsIdSalt, IdSalt, Key, Painter, Rect, Response, Ui};

use crate::edit::EditEvent;

pub use style::BeforeAfterStyle;

/// How [`BeforeAfter`] lays out the two pictures (Lightroom's Before/After views, LightCraft's
/// `BeforeAfter`, FigCraft's compare modes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BeforeAfterMode {
    /// Only the after picture; hold-to-show-original still works (LightCraft's Off).
    #[default]
    After,
    /// Only the before picture (LightCraft's Show Original).
    Original,
    /// Two panes in a row, before at the reading start, `before_after.gap_side` apart.
    SideBySide,
    /// Two panes, before above, `before_after.gap_stacked` apart.
    Stacked,
    /// One picture wiped by a vertical divider, before on the reading-start side (FigCraft's
    /// Overlay).
    Split,
    /// One picture wiped by a horizontal divider, before above.
    SplitStacked,
}

impl BeforeAfterMode {
    /// Every mode, in Lightroom's menu order after the single views.
    pub const ALL: [BeforeAfterMode; 6] = [
        BeforeAfterMode::After,
        BeforeAfterMode::Original,
        BeforeAfterMode::SideBySide,
        BeforeAfterMode::Stacked,
        BeforeAfterMode::Split,
        BeforeAfterMode::SplitStacked,
    ];

    /// Whether the mode shows two panes ([`SideBySide`](Self::SideBySide),
    /// [`Stacked`](Self::Stacked)).
    pub fn has_two_panes(self) -> bool {
        // lightcraft: crates/ui-egui/src/panels/detail.rs:157 (`split`).
        todo!()
    }

    /// Whether the mode is a wipe with a divider ([`Split`](Self::Split),
    /// [`SplitStacked`](Self::SplitStacked)).
    pub fn has_divider(self) -> bool {
        // lightcraft: crates/ui-egui/src/panels/detail.rs:158 (`split_view`).
        todo!()
    }
}

/// Which picture a [`BeforeAfterPane`] holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BeforeAfterSide {
    Before,
    After,
}

/// Where one side is painted, in screen points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BeforeAfterPane {
    pub side: BeforeAfterSide,
    /// The pane: half of the area (two panes) or the whole area.
    pub area: Rect,
    /// Where the side may draw: the pane, or its side of the divider in a wipe, cut to the area.
    pub clip: Rect,
    /// Where the picture goes: the host's `image` (fitted to the after pane) moved to this pane's
    /// centre for two panes, as it is otherwise.
    pub image: Rect,
}

/// What [`BeforeAfter::show`] reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct BeforeAfterOutput {
    /// The mode shown this pass: [`BeforeAfterMode::Original`] while the hold key is down, else
    /// the host's.
    pub mode: BeforeAfterMode,
    /// The before side; `None` in [`BeforeAfterMode::After`].
    pub before: Option<BeforeAfterPane>,
    /// The after side; `None` in [`BeforeAfterMode::Original`].
    pub after: Option<BeforeAfterPane>,
    /// A divider drag (`Begin`, `Update`, `Commit`; Esc cancels) or one key press's `Set`, as the
    /// fraction 0..=1 measured from the reading start (0: all after, 1: all before). View state:
    /// the host keeps the value and makes no undo step.
    pub fraction: Option<EditEvent<f32>>,
    /// The divider's knob in a wipe (a Tab stop), for focus requests.
    pub divider: Option<Response>,
}

crate::words! {
    /// The captions and AccessKit names of [`BeforeAfter`], English by default.
    #[non_exhaustive]
    pub struct BeforeAfterLabels {
        before: "beforeAfter.before" = "Before";
        after: "beforeAfter.after" = "After";
        divider: "beforeAfter.divider" = "Compare position";
    }
}

/// A before and after view of the host's two pictures with a draggable, keyboard-operable divider.
///
/// - Layout ([`panes`](Self::panes)): two panes split the area minus `before_after.gap_side`
///   across (side by side) or `before_after.gap_stacked` down (stacked); before is at the reading
///   start, so it is on the right in a right-to-left interface. A wipe or a single view has one
///   pane, the whole area. With [`captions`](Self::captions) each pane gives up a caption strip at
///   its top (`type.label` line height plus `before_after.caption_gap`).
/// - Painting ([`show`](Self::show)): the host's painter runs for before, then after, each with a
///   painter clipped to [`BeforeAfterPane::clip`]; then the divider line (`before_after.line`
///   over an `overlay.halo` halo, the full height or width of the shown picture), the knob
///   (`before_after.knob` across, `accent.strong`, a two-way arrow in `text.on_strong`) at the
///   line's middle, and the captions (`type.label`, `text.muted`) above each pane, or at the two
///   ends of the picture's top in a wipe.
/// - Divider, wipes only: a press on the picture (or only near the line, with
///   [`drag_anywhere`](Self::drag_anywhere) off) moves the divider to the pointer and drags it,
///   with the `ResizeHorizontal` or `ResizeVertical` pointer. The fraction runs across the shown
///   part of the picture (the picture cut to the area), from the reading start: in a
///   right-to-left interface 0 is the right edge. The picture itself never mirrors.
/// - Keys, knob focused (one Tab stop): ← and → (↑ and ↓ for a horizontal divider) move it 1 %,
///   with ⇧ 10 %, Home to 0 % and End to 100 %; in a right-to-left interface ← moves toward the end.
///   Each press is one `Set`.
/// - Hold to show the original ([`original_while`](Self::original_while)): while the key is down
///   and the keyboard is the app's (no text field, menu or modal: `focus::key_owner`), the view
///   is [`BeforeAfterMode::Original`]; the host's mode comes back on release and nothing is stored.
/// - AccessKit: the knob is a `Slider` named [`BeforeAfterLabels::divider`] with the percentage
///   as its value; each pane is an `Image` named by its caption word.
pub struct BeforeAfter {
    id_salt: IdSalt,
    mode: BeforeAfterMode,
    fraction: f32,
    original_key: Option<Key>,
    captions: bool,
    drag_anywhere: bool,
    labels: Option<BeforeAfterLabels>,
}

impl BeforeAfter {
    /// A view in `mode`, divider in the middle.
    pub fn new(id_salt: impl AsIdSalt, mode: BeforeAfterMode) -> Self {
        Self { id_salt: IdSalt::new(id_salt), mode, fraction: 0.5, original_key: None, captions: false, drag_anywhere: true, labels: None }
    }

    /// The divider's place, 0..=1 from the reading start (0: all after, 1: all before); clamped.
    pub fn fraction(mut self, fraction: f32) -> Self {
        self.fraction = fraction;
        self
    }

    /// Shows only the original while `key` is held (LightCraft's Backslash).
    pub fn original_while(mut self, key: Key) -> Self {
        self.original_key = Some(key);
        self
    }

    /// Shows the "Before" and "After" captions above the panes; off by default.
    pub fn captions(mut self, on: bool) -> Self {
        self.captions = on;
        self
    }

    /// Whether a press anywhere on the picture moves the divider (FigCraft's compare); true by
    /// default. Off, only a press within `before_after.knob` / 2 of the line does, so the host's
    /// own drag (LightCraft's loupe pans) keeps the rest.
    pub fn drag_anywhere(mut self, on: bool) -> Self {
        self.drag_anywhere = on;
        self
    }

    /// The captions and AccessKit names; the installed words by default.
    pub fn labels(mut self, labels: BeforeAfterLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The `[before, after]` pane rects of this view in `area` (the same rect twice for a wipe or
    /// a single view), with the caption strips taken off when captions are on. The host fits its
    /// picture into the after pane and passes that rect to [`show`](Self::show).
    pub fn panes(&self, ui: &Ui, area: Rect) -> [Rect; 2] {
        // lightcraft: crates/ui-egui/src/panels/detail.rs:475-494 `view_areas` (half the width minus 6,
        // half the height minus 14: gaps 12 and 28); figcraft: crates/ui-egui/src/panels/dev_compare.rs:320-323
        // (room for the labels above). Gaps from `BeforeAfter::style`; RTL swaps the side-by-side
        // panes.
        todo!()
    }

    /// Lays out, senses the divider and the hold key, runs `paint` for each shown side with a
    /// painter clipped to it, then draws the divider, the knob and the captions over them. `image`
    /// is the picture's rect fitted to the after pane ([`panes`](Self::panes)); a wipe uses it
    /// as it is for both sides.
    pub fn show(self, ui: &mut Ui, area: Rect, image: Rect, paint: impl FnMut(&Painter, &BeforeAfterPane)) -> BeforeAfterOutput {
        // lightcraft: crates/ui-egui/src/panels/detail.rs:245 (Backslash shows the original), :476-494
        // (pane rects), :430-449 `draw_wipe_before` (before on its side of the line), :637-647
        // `draw_wipe_line`; crates/ui-egui/src/region.rs:379-400 `wipe_line`, `window_clip`.
        // figcraft: crates/ui-egui/src/panels/dev_compare.rs:336-358 (one placement, before clipped to
        // the split, a click or drag anywhere on the picture sets it, `ResizeHorizontal`, 2 pt line,
        // 20 pt knob with a two-way glyph).
        // new: keys, RTL and AccessKit as the type's doc says; the look from `BeforeAfter::style`.
        // needs: lead an `Icon::SwapVertical` (icons.toml, lucide "arrow-up-down") for the
        // SplitStacked knob; the Split knob uses `Icon::SwapHorizontal`.
        todo!()
    }
}
