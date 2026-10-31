//! Dragging a document tab: reorder in its strip, tear it off into a floating window, or drop
//! it into another document group (F16.07). Built on [`crate::tab_strip::TabStrip`]'s drag and
//! the dock's drop zones.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Pos2, Rect, Ui};

use crate::tab_strip::TabStrip;

/// A document as its tab shows it.
pub struct DocumentTab<D> {
    pub id: D,
    pub title: String,
    pub modified: bool,
    pub closable: bool,
}

/// What a document tab drag did this pass.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum DocumentTabEvent<D> {
    Selected(D),
    CloseRequested(D),
    /// Reordered inside its group: now before `before` (`None`: last).
    Reordered {
        document: D,
        before: Option<D>,
    },
    /// Dragged out of its strip and released outside every group: float it at `at`.
    TornOff {
        document: D,
        at: Pos2,
    },
    /// Released over another group's strip, before `before`.
    DroppedInto {
        document: D,
        group: u64,
        before: Option<D>,
    },
}

/// One document group's tab strip with a drag that can leave it.
pub struct DocumentTabs<'a, D> {
    group: u64,
    tabs: &'a [DocumentTab<D>],
    selected: Option<&'a D>,
    strip: TabStrip<'a, D>,
}

impl<'a, D: Clone + PartialEq> DocumentTabs<'a, D> {
    /// `group` identifies the strip for drops from other strips.
    pub fn new(group: u64, tabs: &'a [DocumentTab<D>], selected: Option<&'a D>, strip: TabStrip<'a, D>) -> Self {
        Self { group, tabs, selected, strip }
    }

    /// Draws the strip; a drag past the strip's edge shows the drop line in another strip or
    /// the torn-off preview. Escape cancels the drag and keeps the order.
    // pc: crates/ui-egui/src/canvas/tabs.rs:331 `tab_drag`, :362 `float_torn_off`, :388 `tab_drop_line`.
    pub fn show(self, ui: &mut Ui) -> Vec<DocumentTabEvent<D>> {
        todo!()
    }

    /// The strip rects every document group registered this pass, for cross-group drops.
    pub fn strips(ctx: &egui::Context) -> Vec<(u64, Rect)> {
        todo!()
    }
}
