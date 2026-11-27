//! The `Ui` build cursor (docs/contract-rt.md section 3.1): what a scope's build hands to app and
//! component code. The skeleton lays children out one under another, the subset `Stack` and
//! `IconButton` need.

use std::sync::Arc;

use craft_theme::Tokens;
use craft_ui_rt::{BuildCx, NodeId, Pos2, Rect, Vec2, WindowId};
use egui::Id;

use super::key::Ordinals;
use super::node::Placed;
use crate::contract::IdSource;

/// The build cursor of one node: its ui-rt build context and the layout region its children go in.
pub struct Ui<'u, 'c> {
    pub(crate) cx: &'u mut BuildCx<'c>,
    pub(crate) flow: Flow,
}

/// The layout state of one node's build, apart from the build context so a child's run can lay
/// itself out while the context is lent to it.
pub(crate) struct Flow {
    /// The craft `Id` of the node being built.
    pub(crate) id: Id,
    /// The node's position in the window: children's offsets are taken from it.
    pub(crate) origin: Pos2,
    /// The rect children go in, in window coordinates.
    pub(crate) region: Rect,
    cursor: Pos2,
    gap: f32,
    /// The rect the children used so far.
    pub(crate) extent: Rect,
    ordinals: Ordinals,
    pub(crate) placed: Vec<Placed>,
    /// The tokens the node's build read.
    pub(crate) tokens: Arc<Tokens>,
}

impl Flow {
    pub(crate) fn new(id: Id, origin: Pos2, region: Rect, gap: f32, tokens: Arc<Tokens>) -> Flow {
        Flow { id, origin, region, cursor: region.min, gap, extent: Rect::NOTHING, ordinals: Ordinals::default(), placed: Vec::new(), tokens }
    }

    /// The craft `Id` and ui-rt child key of the next child of type `type_key`.
    pub(crate) fn child(&mut self, type_key: u64, source: IdSource) -> (Id, u64) {
        self.ordinals.child(self.id, type_key, source)
    }

    /// Where the next child starts.
    pub(crate) fn next_min(&self) -> Pos2 {
        if self.placed.is_empty() { self.cursor } else { self.cursor + Vec2::new(0.0, self.gap) }
    }

    /// The room the next child has.
    pub(crate) fn room(&self) -> Vec2 {
        (self.region.max - self.next_min()).max(Vec2::ZERO)
    }

    /// Lays out `node` at the next place with `size` and returns its window rect.
    pub(crate) fn allocate(&mut self, node: NodeId, size: Vec2) -> Rect {
        let rect = Rect::from_min_size(self.next_min(), size);
        self.cursor = Pos2::new(self.cursor.x, rect.max.y);
        self.extent = self.extent.union(rect);
        self.placed.push(Placed { node, offset: rect.min - self.origin, size });
        rect
    }

    /// The size of what the children used, from the region's start.
    pub(crate) fn content_size(&self) -> Vec2 {
        if self.placed.is_empty() { Vec2::ZERO } else { self.extent.max - self.region.min }
    }
}

impl<'u, 'c> Ui<'u, 'c> {
    pub(crate) fn new(cx: &'u mut BuildCx<'c>, flow: Flow) -> Ui<'u, 'c> {
        Ui { cx, flow }
    }

    /// The window this build is in.
    pub fn window(&self) -> WindowId {
        self.cx.window()
    }

    /// The craft `Id` of the node being built.
    pub fn id(&self) -> Id {
        self.flow.id
    }

    /// The rect children go in, in window coordinates.
    pub fn max_rect(&self) -> Rect {
        self.flow.region
    }

    /// The room the next child has.
    pub fn available_size(&self) -> Vec2 {
        self.flow.room()
    }

    /// Where the next child starts.
    pub fn cursor(&self) -> Pos2 {
        self.flow.next_min()
    }

    /// The rect the children used so far.
    pub fn min_rect(&self) -> Rect {
        self.flow.extent
    }

    /// The tokens this node's build read.
    pub fn tokens(&self) -> &Tokens {
        &self.flow.tokens
    }

    /// The ui-rt build context, for code that shows plain ui-rt elements.
    pub fn build_cx(&mut self) -> &mut BuildCx<'c> {
        self.cx
    }
}
