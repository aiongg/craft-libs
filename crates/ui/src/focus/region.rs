//! [`RegionFocus`]: F6 and Shift+F6 move the keyboard between the window's regions (the tools,
//! the options bar, the document, the panels), as Windows apps and Photoshop's panels do.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Context, Id, Rect, Ui};

/// A region of the window F6 visits. The host registers the regions it shows each frame, in F6
/// order; a region it doesn't register this frame is skipped.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Region {
    /// The automation id (`tools`, `options`, `document`, `panels`, or the app's own).
    pub id: String,
    /// The accessible name announced when the region takes the keyboard ("Tools").
    pub label: String,
}

impl Region {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Region {
        todo!()
    }
}

/// The commands F6 and Shift+F6 are bound to in the app's keymap (a user may rebind them).
// figcraft: crates/ui-egui/src/a11y.rs:29-30.
pub const NEXT_REGION_COMMAND: &str = "view.nextRegion";
pub const PREVIOUS_REGION_COMMAND: &str = "view.previousRegion";

/// Region focus for one window, kept in egui memory by window.
///
/// ```no_run
/// use craft_ui::focus::{Region, RegionFocus};
/// # fn demo(ui: &mut egui::Ui, tools_rect: egui::Rect, canvas_id: egui::Id) {
/// let focus = RegionFocus::new(ui.ctx());
/// focus.region(ui.ctx(), Region::new("tools", "Tools"), tools_rect, None);
/// focus.region(ui.ctx(), Region::new("document", "Document"), ui.max_rect(), Some(canvas_id));
/// // When the keymap runs view.nextRegion:
/// focus.step(ui.ctx(), 1);
/// # }
/// ```
///
/// - **Moving:** [`RegionFocus::step`] gives the next registered region the keyboard: its entry
///   widget (the one registered with it, else the region's first focusable widget) takes egui
///   focus. The order wraps. The document region's entry is the canvas, so tool keys work at once.
/// - **Ring:** the region that took the keyboard shows the focus ring around its rect until the
///   pointer is pressed anywhere (keyboard mode, as egui's own focus ring); a press in a region
///   makes it the current one without a ring.
/// - **Modal:** while a dialog, popup or menu is open, F6 is left to it (no step).
/// - **AccessKit:** the region's label is announced (a polite live region) when it takes the
///   keyboard.
#[derive(Clone, Copy, Debug)]
pub struct RegionFocus {
    id: Id,
}

impl RegionFocus {
    /// The region focus of the window `ctx` is drawing.
    pub fn new(ctx: &Context) -> RegionFocus {
        todo!()
    }

    /// Registers a region shown this frame, in F6 order, with its rect and its entry widget.
    // figcraft: crates/ui-egui/src/a11y.rs:44 (`Region::ALL`, F6 order), :580 `enter`.
    pub fn region(&self, ctx: &Context, region: Region, rect: Rect, entry: Option<Id>) {
        todo!()
    }

    /// Moves the keyboard `step` regions on (1 for F6, -1 for Shift+F6). Does nothing while a
    /// modal layer is open.
    // figcraft: crates/ui-egui/src/a11y.rs:131 `Nav::step`; wordcraft: crates/ui-egui/src/panes/host.rs:24 `route_f6`.
    pub fn step(&self, ctx: &Context, step: i32) {
        todo!()
    }

    /// Gives `region` the keyboard (automation's `ui.set {focusRegion}`).
    pub fn go(&self, ctx: &Context, region: &str) {
        todo!()
    }

    /// The region that has the keyboard, by id.
    pub fn current(&self, ctx: &Context) -> Option<String> {
        todo!()
    }

    /// Paints the ring around the current region when the keyboard moved there. Call it after
    /// the regions draw, on the window's top layer.
    // figcraft: crates/ui-egui/src/a11y.rs:284 `keyboard` (the ring shows only in keyboard mode).
    pub fn paint_ring(&self, ui: &Ui) {
        todo!()
    }
}
