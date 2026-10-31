//! Multi-window document host: the floating document windows Window › Arrange makes (Float in
//! Window, Float All, New Window for Document) and their return to tabs. Each window is a
//! [`super::Floating`] of the dock whose root holds a document group; in-window frames by
//! default, OS windows with [`super::DockArea::native_windows`].

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::hash::Hash;

use egui::Rect;

use super::{Action, Layout};

/// A document group: the documents in one tab strip, one of them shown.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentGroup<D> {
    pub id: u64,
    pub documents: Vec<D>,
    pub shown: Option<D>,
}

/// The document windows of the app: the tabbed groups in the canvas area and the floating ones.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocumentWindows<D> {
    pub docked: Vec<DocumentGroup<D>>,
    pub floating: Vec<(DocumentGroup<D>, Rect)>,
    next_id: u64,
}

impl<D: Clone + Eq + Hash> DocumentWindows<D> {
    /// Float `document` from its group into a new window at `rect`.
    // pc: crates/ui-egui/src/canvas/tabs.rs:362 `float_torn_off`.
    pub fn float(&mut self, document: &D, rect: Rect) -> Option<u64> {
        todo!()
    }

    /// Every document into its own floating window, cascaded from `area`'s top left.
    pub fn float_all(&mut self, area: Rect, size: egui::Vec2) {
        todo!()
    }

    /// Every document back into one docked group, the active one shown.
    pub fn consolidate(&mut self, active: Option<&D>) {
        todo!()
    }

    /// Split the docked documents into `n` groups for an n-up arrangement, in order.
    pub fn split_into(&mut self, n: usize) {
        todo!()
    }

    /// A second view of `document`: the same document id in a new window; the app keeps one
    /// view state per (document, window).
    pub fn new_window_for(&mut self, document: &D, rect: Rect) -> u64 {
        todo!()
    }

    /// The dock actions that put `self` into `layout`, for the dock to draw the windows.
    // new: bridge to the dock's Floating entries so the floating document windows share the
    // frame, move, resize and native-window code of floating panels.
    pub fn to_actions<P>(&self, layout: &Layout<P>, panel_of: impl Fn(u64) -> P) -> Vec<Action<P>> {
        todo!()
    }
}
