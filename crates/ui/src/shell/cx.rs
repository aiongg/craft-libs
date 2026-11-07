//! `ShellCx`: what a body or a slot container reads of the window skeleton around it.

use super::regions::{BodyMode, ShellRegions, SlotKey, WindowKey, reveal_index};
use crate::contract::{BodyCx, PaneEdge};
use crate::runtime::env::{AppValues, EnvValue};
use egui::Rect;
use std::sync::Arc;

/// What the window skeleton shows on this pass, read only.
///
/// The shell sets the value in the env before any slot container runs, so a slot body reads it with
/// [`BodyCx::shell`] in the same pass. [`ShellCx::detached`] and [`ShellCx::put`] set it the same way for tests and the
/// gallery. Cloning is cheap: the value is shared.
#[derive(Clone, Debug, PartialEq)]
pub struct ShellCx {
    inner: Arc<Inner>,
}

#[derive(Clone, Debug, PartialEq)]
struct Inner {
    window: WindowKey,
    mode: BodyMode,
    tab_reveal: bool,
    /// The edge reveal flags, in the order of `REVEAL_EDGES`.
    edges: [bool; 3],
    tabs: bool,
    regions: ShellRegions,
    float_bounds: Rect,
}

impl ShellCx {
    /// The skeleton of `window` in `mode` with the layout of this pass. No reveal is on, the document tabs show as the
    /// mode allows and floating frames may go over the body.
    pub fn new(window: WindowKey, mode: BodyMode, regions: ShellRegions) -> Self {
        let float_bounds = regions.body;
        let inner = Inner { window, mode, tab_reveal: false, edges: [false; 3], tabs: true, regions, float_bounds };
        ShellCx { inner: Arc::new(inner) }
    }

    /// A skeleton with no slots around `body`, for tests and the gallery without an `AppShell`.
    pub fn detached(window: WindowKey, mode: BodyMode, body: Rect) -> Self {
        Self::new(window, mode, ShellRegions::new(body))
    }

    /// Whether the host shows the document tabs; they show only in [`BodyMode::Normal`].
    pub fn with_tabs(mut self, tabs: bool) -> Self {
        Arc::make_mut(&mut self.inner).tabs = tabs;
        self
    }

    /// The Tab reveal is on or off.
    pub fn with_tab_reveal(mut self, on: bool) -> Self {
        let inner = Arc::make_mut(&mut self.inner);
        inner.tab_reveal = on;
        inner.sync_revealed();
        self
    }

    /// The edge reveal of `edge` is on or off. The bottom edge and the centre never reveal: the call changes nothing.
    pub fn with_edge_reveal(mut self, edge: PaneEdge, on: bool) -> Self {
        if let Some(index) = reveal_index(edge) {
            let inner = Arc::make_mut(&mut self.inner);
            inner.edges[index] = on;
            inner.sync_revealed();
        }
        self
    }

    /// Where floating frames may go: the body rect plus the area under bars that overlay the body.
    pub fn with_float_bounds(mut self, bounds: Rect) -> Self {
        Arc::make_mut(&mut self.inner).float_bounds = bounds;
        self
    }

    /// Sets this value in `values`, where [`BodyCx::shell`] reads it. The shell calls it on the values of the scope
    /// that holds its slots and body.
    pub fn put(&self, values: &mut AppValues) {
        values.set(Entry(Some(self.clone())));
    }

    /// The window's key; components in slots key their ids and memory by it.
    pub fn window(&self) -> &WindowKey {
        &self.inner.window
    }

    /// The body mode of this pass.
    pub fn body_mode(&self) -> BodyMode {
        self.inner.mode
    }

    /// Is the slot with this key shown on this pass?
    pub fn slot_visible(&self, key: SlotKey) -> bool {
        self.inner.regions.slots.iter().any(|(shown, _)| *shown == key)
    }

    /// Whether the dock shows: in normal mode, during the Tab reveal and during the end-edge reveal.
    pub fn dock_visible(&self) -> bool {
        self.inner.mode == BodyMode::Normal || self.inner.tab_reveal || self.edge_revealed(PaneEdge::End)
    }

    /// Whether floating frames show: always in normal mode; in body-only mode only during the Tab reveal.
    pub fn floating_visible(&self) -> bool {
        self.inner.mode == BodyMode::Normal || self.inner.tab_reveal
    }

    /// Whether the document tabs show: the host's flag in normal mode, never in body-only mode.
    pub fn tabs_visible(&self) -> bool {
        self.inner.mode == BodyMode::Normal && self.inner.tabs
    }

    /// Are slots shown by a reveal (Tab or edge) on this pass?
    pub fn revealed(&self) -> bool {
        self.inner.regions.revealed
    }

    /// Is the edge reveal on for `edge`? Only the top, start and end edges reveal. False in normal mode and during the
    /// Tab reveal.
    pub fn edge_revealed(&self, edge: PaneEdge) -> bool {
        let inner = &*self.inner;
        inner.mode == BodyMode::Only && !inner.tab_reveal && reveal_index(edge).is_some_and(|index| inner.edges[index])
    }

    /// The body rect and the shown slots of this pass.
    pub fn regions(&self) -> &ShellRegions {
        &self.inner.regions
    }

    /// Where floating frames may go: the body rect plus the area under bars that overlay the body.
    pub fn float_bounds(&self) -> Rect {
        self.inner.float_bounds
    }
}

impl Inner {
    /// `regions.revealed` follows the reveal flags, so one state answers `revealed`.
    fn sync_revealed(&mut self) {
        let on = self.mode == BodyMode::Only && (self.tab_reveal || self.edges.iter().any(|edge| *edge));
        self.regions.revealed = on;
    }
}

/// The env entry that carries the shell to its slots and body. A context with no shell holds `None`.
#[derive(Clone, PartialEq)]
struct Entry(Option<ShellCx>);

impl EnvValue for Entry {
    fn root() -> Self {
        Entry(None)
    }
}

impl BodyCx<'_> {
    /// The skeleton around this body, when the body runs inside an `AppShell` (or under [`ShellCx::put`]).
    pub fn shell(&self) -> Option<ShellCx> {
        self.env.get::<Entry>().0
    }
}
