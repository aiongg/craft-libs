//! The `Ui` build cursor (docs/contract-rt.md section 3.1): what a scope's build hands to app and
//! component code. Its flow places children by the body's `BodyLayout` (one under another, side by
//! side with wrap and alignment, or in grid cells), gives fill children their share, and knows
//! whether it is a measuring run: there, children report their size and no node is reached, so a
//! kept child answers from its size cache instead of running.

use std::sync::Arc;

use craft_theme::Tokens;
use craft_ui_rt::{BuildCx, NodeId, Pos2, Rect, State, Vec2, WindowId};
use egui::Id;

use super::key::Ordinals;
use super::node::Placed;
use super::sweep::Sweep;
use crate::contract::IdSource;
use crate::layout::CellAlign;

/// The build cursor of one node: its ui-rt build context and the layout region its children go in.
pub struct Ui<'u, 'c> {
    pub(crate) cx: &'u mut BuildCx<'c>,
    pub(crate) flow: Flow,
}

/// How a flow places its children: the body's `BodyLayout` with what its container's plan decided
/// for this run.
#[derive(Clone, Debug)]
pub(crate) enum Dir {
    /// One under another.
    Stack,
    /// Side by side. `start` is where the block of children begins on its line (alignment) and
    /// `spread` the room added between children (`SpaceBetween`).
    Row { wrap: bool, start: f32, spread: f32 },
    /// Row-major cells of `widths`, rows `heights` high (empty in a measuring run).
    Grid { widths: Vec<f32>, heights: Vec<f32>, cells: CellAlign },
}

/// The layout state of one node's build, apart from the build context so a child's run can lay
/// itself out while the context is lent to it.
pub(crate) struct Flow {
    /// The craft `Id` of the node being built.
    pub(crate) id: Id,
    /// The window position the node's kept child offsets are taken from: the node's position, or a
    /// scroll body's content origin.
    pub(crate) origin: Pos2,
    /// The rect children go in, in window coordinates.
    pub(crate) region: Rect,
    cursor: Pos2,
    /// The height of the current line of a `Row`.
    line: f32,
    /// Children on the current line, or in the flow for a `Stack`.
    on_line: usize,
    gap: f32,
    dir: Dir,
    /// The rect the children used so far.
    pub(crate) extent: Rect,
    ordinals: Ordinals,
    pub(crate) placed: Vec<Placed>,
    /// Each child's size in order and whether it filled: what a measuring run hands its
    /// container's plan, and what the real run checks the plan against.
    pub(crate) sizes: Vec<(Vec2, bool)>,
    /// The tokens the node's build read.
    pub(crate) tokens: Arc<Tokens>,
    /// A measuring run, with the node whose kept children answer it (`None` when nothing is kept
    /// there). `None` for a real run.
    pub(crate) measuring: Option<Option<NodeId>>,
    /// The rect content is built for inside a scroll body (its view plus overscan), in window
    /// coordinates; `None` outside every scroll body.
    pub(crate) view: Option<Rect>,
    /// A `Lazy` in this flow or under it stayed out of view: the build covered only `view`.
    pub(crate) limited: bool,
    /// Where a revealed child starts, in window coordinates: its scroll body brings it to the view's
    /// start.
    pub(crate) reveal: Option<Pos2>,
    /// The size along the flow's axis each fill child takes in this run, from the plan, in order.
    shares: Vec<f32>,
    /// The room each child had, in order: a measuring run records it, and the real run that follows
    /// its plan gives each child the same room, so a child's cached size answers both.
    pub(crate) rooms: Vec<Vec2>,
    /// Rooms from the plan, in order.
    planned: Vec<Vec2>,
    /// Fill children met so far.
    pub(crate) fills: usize,
    /// The window's toggle sweep record, handed down from the window root.
    pub(crate) sweep: Option<State<Sweep>>,
    /// The intents the layer scope whose body this flow is took for it (an untaken Enter's Submit, a
    /// clearing Escape); empty in every other flow ([`super::intents`]).
    pub(crate) scope: super::intents::ScopeIntents,
}

impl Flow {
    pub(crate) fn new(id: Id, origin: Pos2, region: Rect, gap: f32, tokens: Arc<Tokens>) -> Flow {
        Flow {
            id,
            origin,
            region,
            cursor: region.min,
            line: 0.0,
            on_line: 0,
            gap,
            dir: Dir::Stack,
            extent: Rect::NOTHING,
            ordinals: Ordinals::default(),
            placed: Vec::new(),
            sizes: Vec::new(),
            tokens,
            measuring: None,
            view: None,
            limited: false,
            reveal: None,
            shares: Vec::new(),
            rooms: Vec::new(),
            planned: Vec::new(),
            fills: 0,
            sweep: None,
            scope: super::intents::ScopeIntents::default(),
        }
    }

    /// The flow places its children by `dir`.
    pub(crate) fn with_dir(mut self, dir: Dir) -> Flow {
        if let Dir::Row { start, .. } = dir {
            self.cursor.x = self.region.min.x + start;
        }
        self.dir = dir;
        self
    }

    /// Fill children take `shares` along the flow's axis, and each child the room of `rooms`, in
    /// order.
    pub(crate) fn with_plan(mut self, shares: Vec<f32>, rooms: Vec<Vec2>) -> Flow {
        self.shares = shares;
        self.planned = rooms;
        self
    }

    /// The window's toggle sweep record, handed down from the window root.
    pub(crate) fn with_sweep(mut self, sweep: Option<State<Sweep>>) -> Flow {
        self.sweep = sweep;
        self
    }

    /// A measuring run whose kept children live under `under`.
    pub(crate) fn measuring(mut self, under: Option<NodeId>) -> Flow {
        self.measuring = Some(under);
        self
    }

    /// Content inside a scroll body, built for `view`.
    pub(crate) fn in_view(mut self, view: Option<Rect>) -> Flow {
        self.view = view;
        self
    }

    /// The craft `Id` and ui-rt child key of the next child of type `type_key`.
    pub(crate) fn child(&mut self, type_key: u64, source: IdSource) -> (Id, u64) {
        self.ordinals.child(self.id, type_key, source)
    }

    fn grid_cell(&self) -> (usize, usize) {
        let Dir::Grid { widths, .. } = &self.dir else { return (0, 0) };
        let columns = widths.len().max(1);
        (self.sizes.len() % columns, self.sizes.len() / columns)
    }

    /// Where the next child starts.
    pub(crate) fn next_min(&self) -> Pos2 {
        match &self.dir {
            Dir::Stack if self.on_line > 0 => self.cursor + Vec2::new(0.0, self.gap),
            Dir::Stack => self.cursor,
            Dir::Row { spread, .. } if self.on_line > 0 => self.cursor + Vec2::new(self.gap + spread, 0.0),
            Dir::Row { .. } => self.cursor,
            Dir::Grid { widths, heights, .. } => {
                let (column, row) = self.grid_cell();
                let x = widths.iter().take(column).sum::<f32>() + self.gap * column as f32;
                let y = heights.iter().take(row).sum::<f32>() + self.gap * row as f32;
                self.region.min + Vec2::new(x, y)
            }
        }
    }

    /// The room the next child has. A fill child's room along the flow's axis is its share.
    pub(crate) fn room(&self) -> Vec2 {
        if let Some(room) = self.planned.get(self.sizes.len()) {
            return *room;
        }
        let min = self.next_min();
        match &self.dir {
            Dir::Stack => Vec2::new(self.region.width(), self.region.max.y - min.y).max(Vec2::ZERO),
            Dir::Row { wrap: true, .. } => Vec2::new(self.region.width(), self.region.height()).max(Vec2::ZERO),
            Dir::Row { .. } => Vec2::new(self.region.max.x - min.x, self.region.height()).max(Vec2::ZERO),
            Dir::Grid { widths, .. } => {
                let width = widths.get(self.grid_cell().0).copied().unwrap_or(self.region.width());
                Vec2::new(width, self.region.max.y - min.y).max(Vec2::ZERO)
            }
        }
    }

    /// The room of the next child when it fills along the flow's axis: its share once the plan
    /// decided it, otherwise all the room left (a measuring run, or a flow with no plan).
    pub(crate) fn fill_room(&self) -> Vec2 {
        let mut room = self.room();
        if let Some(share) = self.shares.get(self.fills).copied() {
            match self.dir {
                Dir::Row { .. } => room.x = share,
                _ => room.y = share,
            }
        }
        room
    }

    /// Lays out the next child with `size` and returns its window rect; `node` is `None` in a
    /// measuring run, which reaches no node.
    pub(crate) fn allocate(&mut self, node: Option<NodeId>, size: Vec2, filled: bool) -> Rect {
        let room = if filled { self.fill_room() } else { self.room() };
        self.rooms.push(room);
        let mut min = self.next_min();
        if let Dir::Row { wrap: true, .. } = self.dir
            && self.on_line > 0
            && min.x + size.x > self.region.max.x
        {
            self.cursor = Pos2::new(self.region.min.x, self.cursor.y + self.line + self.gap);
            self.line = 0.0;
            self.on_line = 0;
            min = self.cursor;
        }
        let mut rect = Rect::from_min_size(min, size);
        match &self.dir {
            Dir::Stack => {
                self.cursor.y = rect.max.y;
                self.on_line += 1;
            }
            Dir::Row { .. } => {
                self.cursor.x = rect.max.x;
                self.line = self.line.max(size.y);
                self.on_line += 1;
            }
            Dir::Grid { heights, cells, .. } => {
                let row = self.grid_cell().1;
                if let (Some(height), CellAlign::Center) = (heights.get(row), cells) {
                    rect = rect.translate(Vec2::new(0.0, ((height - size.y) / 2.0).max(0.0)));
                }
            }
        }
        if filled {
            self.fills += 1;
        }
        self.extent = self.extent.union(rect);
        self.sizes.push((size, filled));
        if let Some(node) = node {
            self.placed.push(Placed { node, offset: rect.min - self.origin, size });
        }
        rect
    }

    /// The size of what the children used, from the region's start.
    pub(crate) fn content_size(&self) -> Vec2 {
        if self.sizes.is_empty() { Vec2::ZERO } else { (self.extent.max - self.region.min).max(Vec2::ZERO) }
    }

    pub(crate) fn dir_ref(&self) -> &Dir {
        &self.dir
    }

    /// Takes over what a nested flow found that its scroll body needs: a `Lazy` out of view, a
    /// reveal.
    pub(crate) fn absorb(&mut self, inner: &Flow) {
        self.limited |= inner.limited;
        if self.reveal.is_none() {
            self.reveal = inner.reveal;
        }
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

    /// This is a measuring run: what is shown here only reports its size, and outputs carry no
    /// input.
    pub fn is_measuring(&self) -> bool {
        self.flow.measuring.is_some()
    }

    /// The tokens this node's build read.
    pub fn tokens(&self) -> &Tokens {
        &self.flow.tokens
    }

    /// The ui-rt build context, for code that shows plain ui-rt elements. A measuring run must not
    /// reconcile children through it.
    pub fn build_cx(&mut self) -> &mut BuildCx<'c> {
        self.cx
    }
}
