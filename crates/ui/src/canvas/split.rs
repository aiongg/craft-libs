//! [`SplitViews`]: one canvas area shown as 1, 2 or 4 views of the same document, as After Effects'
//! View › Switch View Layout (EffectCraft). One view is active: it is the full interactive view
//! (tools, overlays, keys); the others show the document from their own view and become active on
//! a click. With Share View Options on, the grid, guides and rulers settings of the active view
//! apply to every view.
//!
//! Each pane has its own [`CanvasView`] the app keeps; a linked pane follows the active view's
//! zoom and position (View › Match Zoom and Location for panes).
//!
//! ```no_run
//! use craft_ui::canvas::{SplitLayout, SplitViews};
//! # fn demo(ui: &mut egui::Ui, rect: egui::Rect) {
//! let response = SplitViews::new("comp-1", SplitLayout::Two).show(ui, rect, |ui, pane, rect| {
//!     // draw pane `pane` in `rect`; pane 0 is the main view
//! });
//! # let _ = response;
//! # }
//! ```

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, IdSalt, Rect as ScreenRect, Ui};

use super::view::CanvasView;

/// How the canvas area divides (View › Switch View Layout).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SplitLayout {
    /// 1 View.
    #[default]
    One,
    /// 2 Views: side by side, the main view on the right.
    Two,
    /// 4 Views: quarters, the main view at the bottom right.
    Four,
}

impl SplitLayout {
    /// Points between panes.
    pub const GAP: f32 = 2.0;

    /// Number of panes.
    pub fn panes(self) -> usize {
        match self {
            SplitLayout::One => 1,
            SplitLayout::Two => 2,
            SplitLayout::Four => 4,
        }
    }

    /// The panes' rects in `full`, the main view (pane 0) first, the others in reading order.
    // effectcraft: crates/ui-egui/src/panels/viewer/view.rs:5 `aux_views`.
    pub fn rects(self, full: ScreenRect) -> Vec<ScreenRect> {
        todo!()
    }
}

/// One pane's view: its own, or linked to the main view's zoom and position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitPane {
    pub view: CanvasView,
    /// The pane follows the main view's centre and zoom (its rotation stays its own).
    pub linked: bool,
}

/// What the split did in a pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SplitResponse {
    /// The pane pressed this pass that was not the active one: the app makes it active.
    pub activated: Option<usize>,
}

/// The split of one canvas area. The active pane is the app's (it routes tools and keys there).
pub struct SplitViews {
    id_salt: IdSalt,
    layout: SplitLayout,
    active: usize,
}

impl SplitViews {
    pub fn new(id_salt: impl AsIdSalt, layout: SplitLayout) -> Self {
        Self { id_salt: IdSalt::new(id_salt), layout, active: 0 }
    }

    /// The active pane (0 by default): drawn with the accent edge (`focus.ring` colour, 1 pt)
    /// when there is more than one pane.
    pub fn active(mut self, active: usize) -> Self {
        self.active = active;
        self
    }

    /// Lay out the panes in `rect` and call `pane(ui, index, rect)` for each, clipped to it; a
    /// press on an inactive pane reports it in [`SplitResponse::activated`] and does not reach
    /// the pane's tool. Each pane is an AccessKit group named "View N".
    // effectcraft: crates/ui-egui/src/panels/viewer/view.rs:5 `aux_views`, panels/viewers.rs:208 `activate`.
    pub fn show(self, ui: &mut Ui, rect: ScreenRect, pane: impl FnMut(&mut Ui, usize, ScreenRect)) -> SplitResponse {
        todo!()
    }

    /// Copy the main view's centre and zoom into the linked panes, each keeping its own rect
    /// and rotation.
    pub fn follow(main: &CanvasView, panes: &mut [SplitPane]) {
        todo!()
    }
}
