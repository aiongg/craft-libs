//! The `Ui` build cursor (docs/contract-rt.md section 3.1): what a scope's build hands to app and
//! component code. Its flow places children by the body's `BodyLayout` (one under another, side by
//! side with wrap and alignment, or in grid cells), gives fill children their share, and knows
//! whether it is a measuring run: there, children report their size and no node is reached, so a
//! kept child answers from its size cache instead of running.

use std::hash::Hash;
use std::sync::Arc;

use craft_theme::Tokens;
use craft_ui_rt::{BuildCx, DisplayList, NodeId, Pos2, Rect, Sense, State, Vec2, WindowId};

use super::key::Ordinals;
use super::node::Placed;
use super::response::InputFacts;
use super::sweep::Sweep;
use crate::contract::Response;
use crate::contract::context::Context;
use crate::contract::{IdSource, MemoKey, MemoProps};
use crate::layout::CellAlign;
use crate::primitives::Id;
use crate::runtime::env::Env;

/// The build cursor of one node: its ui-rt build context, the layout region its children go in,
/// the env its children inherit and the window's [`Context`].
pub struct Ui<'u, 'c> {
    pub(crate) cx: &'u mut BuildCx<'c>,
    pub(crate) flow: Flow,
    /// The window's context: one handle per window, found where the window root provided it.
    pub(crate) ctx: Context,
    /// The env this cursor's children inherit; [`Env::run_in`] changes it for a body.
    pub(crate) env: Env,
}

/// How a child cursor made with [`Ui::scope_builder`] is placed: a craft type with egui's builder
/// names. A builder with no rect continues where the parent's next child would start.
#[derive(Clone, Debug, Default)]
pub struct UiBuilder {
    max_rect: Option<Rect>,
    id_salt: Option<u64>,
    sense: Option<Sense>,
}

impl UiBuilder {
    /// A builder that places the child cursor where the parent's next child goes.
    pub fn new() -> UiBuilder {
        UiBuilder::default()
    }

    /// The child cursor lays its children out in `rect` (window coordinates).
    #[must_use]
    pub fn max_rect(mut self, rect: Rect) -> UiBuilder {
        self.max_rect = Some(rect);
        self
    }

    /// The child cursor's ids hash from `salt` under the parent's id.
    #[must_use]
    pub fn id_salt(mut self, salt: impl Hash) -> UiBuilder {
        self.id_salt = Some(craft_ui_rt::hash_key(&salt));
        self
    }

    /// What the child region senses. A plain cursor has no hit shape of its own, so this is kept for
    /// the callers that name it; a region that must take input is a component.
    #[must_use]
    pub fn sense(mut self, sense: Sense) -> UiBuilder {
        self.sense = Some(sense);
        self
    }
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
    /// What paint recorded through this cursor is clipped to, in window coordinates.
    pub(crate) clip: Rect,
    /// The opacity content built through this cursor paints at.
    pub(crate) opacity: f32,
    /// What a container body's flow does beyond its `Dir` (right-to-left mirroring).
    pub(crate) body: super::container::BodyFlow,
    /// What the build drew through [`Ui::draw`], in window coordinates: the lists under and over the node's
    /// children. The driver moves them into the node's paint when the build ends ([`Flow::take_drawn`]).
    pub(crate) drawn: (DisplayList, DisplayList),
}

impl Flow {
    /// The gap the flow puts between its children.
    pub(crate) fn gap(&self) -> f32 {
        self.gap
    }

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
            clip: Rect::EVERYTHING,
            opacity: 1.0,
            body: super::container::BodyFlow::default(),
            drawn: (DisplayList::default(), DisplayList::default()),
        }
    }

    /// A flow nested in this one in the same node, laying out in `region` with ids under `id`: it
    /// keeps the node's origin, tokens, view, sweep, clip and opacity, and measures when this one does.
    pub(crate) fn nested(&self, id: Id, region: Rect) -> Flow {
        let mut flow = Flow::new(id, self.origin, region, self.gap, self.tokens.clone()).in_view(self.view).with_sweep(self.sweep);
        flow.measuring = self.measuring;
        flow.clip = self.clip;
        flow.opacity = self.opacity;
        flow
    }

    /// Takes over what a nested flow of the same node laid out: its placed nodes, and its extent as
    /// one block the cursor moves past.
    pub(crate) fn take_nested(&mut self, mut inner: Flow) {
        self.absorb(&inner);
        let (under, over) = std::mem::take(&mut inner.drawn);
        self.drawn.0.items.extend(under.items);
        self.drawn.1.items.extend(over.items);
        self.placed.extend(inner.placed);
        if inner.extent.is_positive() || !inner.sizes.is_empty() {
            self.advance(inner.extent);
        }
    }

    /// The region shrinks to `rest`, the part an edge pane left (`Mount::Pane`): later children lay out inside it.
    pub(crate) fn cut_region(&mut self, rest: Rect) {
        (self.cursor, self.region) = (self.cursor.max(rest.min), rest);
    }

    /// Moves the cursor past `rect` as if a child took it, without a child slot in the plan.
    pub(crate) fn advance(&mut self, rect: Rect) {
        match self.dir {
            Dir::Stack => self.cursor.y = self.cursor.y.max(rect.max.y),
            Dir::Row { .. } => {
                self.cursor.x = self.cursor.x.max(rect.max.x);
                self.line = self.line.max(rect.height());
            }
            Dir::Grid { .. } => {}
        }
        self.on_line += 1;
        self.extent = self.extent.union(rect);
    }

    /// Moves the cursor `amount` along the flow's axis.
    pub(crate) fn add_space(&mut self, amount: f32) {
        match self.dir {
            Dir::Row { .. } => self.cursor.x += amount,
            _ => self.cursor.y += amount,
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

    /// The id and child key the next `child` call with these arguments gives, without counting it.
    pub(crate) fn peek_child(&self, type_key: u64, source: IdSource) -> (Id, u64) {
        self.ordinals.clone().child(self.id, type_key, source)
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

    /// Places `node` at `rect` with no allocation (`show_at`): the cursor and the extent stay.
    pub(crate) fn place_at(&mut self, node: Option<NodeId>, rect: Rect) -> Rect {
        if let Some(node) = node {
            self.placed.push(Placed { node, offset: rect.min - self.origin, size: rect.size() });
        }
        rect
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
        let rect = self.body.place(rect, self.region);
        self.extent = self.extent.union(rect);
        self.sizes.push((size, filled));
        if let Some(node) = node {
            self.placed.push(Placed { node, offset: rect.min - self.origin, size });
        }
        rect
    }

    /// The current line of a `Row` is at least `height` high.
    pub(crate) fn set_row_height(&mut self, height: f32) {
        if let Dir::Row { .. } = self.dir {
            self.line = self.line.max(height);
        }
    }

    /// The next child of a `Row` starts a new line under the current one.
    pub(crate) fn end_row(&mut self) {
        if let Dir::Row { .. } = self.dir
            && self.on_line > 0
        {
            self.cursor = Pos2::new(self.region.min.x, self.cursor.y + self.line + self.gap);
            self.line = 0.0;
            self.on_line = 0;
        }
    }

    /// The size of what the children used, from the region's start.
    pub(crate) fn content_size(&self) -> Vec2 {
        if self.sizes.is_empty() { Vec2::ZERO } else { (self.extent.max - self.region.min).max(Vec2::ZERO) }
    }

    pub(crate) fn dir_ref(&self) -> &Dir {
        &self.dir
    }

    /// What the build drew through [`Ui::draw`], moved into the node whose top-left corner is `origin` (window
    /// coordinates): the lists under and over its children, in its coordinates. The driver calls it when the build
    /// ends and puts them under and over the paint the node's hooks recorded.
    pub(crate) fn take_drawn(&mut self, origin: Pos2) -> (DisplayList, DisplayList) {
        let (mut under, mut over) = std::mem::take(&mut self.drawn);
        for item in under.items.iter_mut().chain(over.items.iter_mut()) {
            crate::primitives::items::translate(item, -origin.to_vec2());
        }
        (under, over)
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
    /// The cursor of a node's build: the window's context and the env the enclosing scope gives
    /// (section 15.2), brought up to date with the env cells.
    pub(crate) fn new(cx: &'u mut BuildCx<'c>, flow: Flow) -> Ui<'u, 'c> {
        let ctx = Context::of_build(cx);
        let env = Env::for_build(cx, &ctx);
        Ui { cx, flow, ctx, env }
    }

    /// A cursor of the same node over `flow`, inheriting this one's env and context.
    pub(crate) fn nested_ui(&mut self, flow: Flow) -> Ui<'_, 'c> {
        Ui { cx: &mut *self.cx, flow, ctx: self.ctx.clone(), env: self.env.clone() }
    }

    /// Brings this cursor's env up to date with the env cells its node sees now: a scope calls it after
    /// providing cells of its own, before it runs its content.
    pub(crate) fn refresh_env(&mut self) {
        let env = self.env.clone();
        self.env = env.refreshed(self.cx, &self.ctx);
    }

    /// The window's context.
    pub fn ctx(&self) -> &Context {
        &self.ctx
    }

    /// The window this build is in.
    pub fn window(&self) -> WindowId {
        self.cx.window()
    }

    /// The craft `Id` of the node being built.
    pub fn id(&self) -> Id {
        self.flow.id
    }

    /// The craft `Id` of the enclosing node (egui's name for [`Ui::id`]).
    pub fn scope_id(&self) -> Id {
        self.flow.id
    }

    /// The window's pixels per point; tracked.
    pub fn pixels_per_point(&self) -> f32 {
        self.ctx.pixels_per_point()
    }

    /// The rect children go in, in window coordinates.
    pub fn max_rect(&self) -> Rect {
        self.flow.region
    }

    /// The room left for the next child, from where it starts.
    pub fn available_rect_before_wrap(&self) -> Rect {
        Rect::from_min_size(self.flow.next_min(), self.flow.room())
    }

    /// The room the next child has.
    pub fn available_size(&self) -> Vec2 {
        self.flow.room()
    }

    /// The width the next child has.
    pub fn available_width(&self) -> f32 {
        self.flow.room().x
    }

    /// The height the next child has.
    pub fn available_height(&self) -> f32 {
        self.flow.room().y
    }

    /// Children go no wider than `width`.
    pub fn set_max_width(&mut self, width: f32) {
        let region = &mut self.flow.region;
        region.max.x = region.min.x + width.max(0.0);
    }

    /// Children go no taller than `height`.
    pub fn set_max_height(&mut self, height: f32) {
        let region = &mut self.flow.region;
        region.max.y = region.min.y + height.max(0.0);
    }

    /// The children's extent is at least `size` from the region's start.
    pub fn set_min_size(&mut self, size: Vec2) {
        let min = self.flow.region.min;
        self.flow.extent = self.flow.extent.union(Rect::from_min_size(min, size));
    }

    /// The children's extent is at least `height` high from the region's start.
    pub fn set_min_height(&mut self, height: f32) {
        let min = self.flow.region.min;
        self.flow.extent = self.flow.extent.union(Rect::from_min_size(min, Vec2::new(0.0, height)));
    }

    /// This is a measuring run: what is shown here only reports its size, and outputs carry no
    /// input.
    pub fn is_measuring(&self) -> bool {
        self.flow.measuring.is_some()
    }

    /// A child cursor in the same node, placed by `builder`; the parent's cursor moves past what the
    /// child laid out.
    pub fn scope_builder<R>(&mut self, builder: UiBuilder, f: impl FnOnce(&mut Ui<'_, '_>) -> R) -> R {
        let region = builder.max_rect.unwrap_or_else(|| self.available_rect_before_wrap());
        let id = match builder.id_salt {
            Some(salt) => self.flow.id.with(salt),
            None => self.flow.id.with(("scope", self.flow.sizes.len(), self.flow.placed.len())),
        };
        let flow = self.flow.nested(id, region);
        let mut inner = self.nested_ui(flow);
        let out = f(&mut inner);
        let flow = inner.flow;
        self.flow.take_nested(flow);
        out
    }

    /// A child cursor whose ids hash from `salt` under this one's.
    pub fn push_id<R>(&mut self, salt: impl Hash, f: impl FnOnce(&mut Ui<'_, '_>) -> R) -> R {
        self.scope_builder(UiBuilder::new().id_salt(salt), f)
    }

    /// The rect the children used so far.
    pub fn min_rect(&self) -> Rect {
        self.flow.extent
    }

    /// Where the next child starts, as egui's cursor rect: `min` is the next position, `max` the
    /// region's end.
    pub fn cursor(&self) -> Rect {
        Rect::from_min_max(self.flow.next_min(), self.flow.region.max)
    }

    /// What paint recorded through this cursor is clipped to.
    pub fn clip_rect(&self) -> Rect {
        self.flow.clip
    }

    /// Clips paint recorded through this cursor to `clip`.
    pub fn set_clip_rect(&mut self, clip: Rect) {
        self.flow.clip = clip;
    }

    /// The children's extent includes `rect`.
    pub fn expand_to_include_rect(&mut self, rect: Rect) {
        self.flow.extent = self.flow.extent.union(rect);
    }

    /// The cursor moves past `rect`, as if a child took it.
    pub fn advance_cursor_after_rect(&mut self, rect: Rect) {
        self.flow.advance(rect);
    }

    /// The cursor moves `amount` along the flow's axis.
    pub fn add_space(&mut self, amount: f32) {
        self.flow.add_space(amount);
    }

    /// Lays out a blank child of `size` and returns its rect.
    pub fn allocate(&mut self, size: Vec2) -> Rect {
        self.flow.allocate(None, size, false)
    }

    /// Brings `rect` into view: the nearest scroll body around reveals it with its glide.
    pub fn scroll_to_rect(&mut self, rect: Rect) {
        if self.flow.reveal.is_none() {
            self.flow.reveal = Some(rect.min);
        }
    }

    /// The enclosing node's response: its rect and interaction state, with no input facts (the input a
    /// node's run takes reaches its own `Cx`). Interaction facts read from it are tracked.
    pub fn response(&self) -> Response {
        let node = (!self.is_measuring()).then(|| self.cx.node());
        let interact = node.map(|node| self.cx.interaction(node)).unwrap_or_default();
        Response::new(self.flow.id, self.flow.region, node, interact, false, InputFacts::default())
    }

    /// The platform and the window's facts as the host gives them to every window root (`ShellInfo`):
    /// the OS, the capabilities, the window's kind, chrome, state and insets. The default outside an
    /// app host.
    pub fn shell(&self) -> craft_host::target::ShellInfo {
        self.cx.provided::<craft_host::target::ShellInfo>().unwrap_or_default()
    }

    /// The tokens this node's build read.
    pub fn tokens(&self) -> &Tokens {
        &self.flow.tokens
    }

    /// The window's toast queue ([`Context::toasts`]): a change made through it builds the node that shows the stack.
    pub fn toasts(&self) -> crate::contract::context::Toasts<'_> {
        self.ctx.toasts()
    }

    /// The content-paint API over this build: what it records joins the node's paint at the call position's
    /// coordinates (window coordinates), cut to this cursor's clip and at its fade, under the node's children (over
    /// them through `Canvas::over`). The node keeps it until its next build. A measuring run records nothing that
    /// shows.
    pub fn draw(&mut self) -> crate::primitives::Draw<'_> {
        let clip = self.flow.clip;
        self.draw_at(clip)
    }

    /// [`Ui::draw`] cut to `clip` (window coordinates) inside this cursor's clip.
    pub fn draw_at(&mut self, clip: Rect) -> crate::primitives::Draw<'_> {
        let scale = self.cx.scale();
        let clip = clip.intersect(self.flow.clip);
        let fonts = crate::fonts::stack_facts(self.cx);
        let (text, images) = self.cx.paint_services();
        let services = crate::primitives::FrameServices::new(text, images, scale, fonts).with_context(self.ctx.clone());
        let canvas = crate::primitives::Canvas::owned(services, Pos2::ZERO, Rect::EVERYTHING).at_opacity(self.flow.opacity);
        let env = crate::primitives::DrawEnv { tokens: &self.env.tokens, direction: self.env.direction, ink: self.env.ink };
        crate::primitives::Draw::owned(canvas, Some(clip), &mut self.flow.drawn, env).text_direction(self.env.text_direction)
    }

    /// The ui-rt build context, for the bridge's own code. A measuring run must not reconcile
    /// children through it.
    pub(crate) fn build_cx(&mut self) -> &mut BuildCx<'c> {
        self.cx
    }

    /// A value derived from `key` by `build`, kept in a slot of the node being built and built again
    /// only when `key` moves.
    pub fn derived<K: MemoProps, V: Send + Sync + 'static>(&mut self, key: &K, build: impl FnOnce() -> V) -> Arc<V> {
        let mut hasher = MemoKey::new();
        key.key(&mut hasher);
        let hash = hasher.finish();
        // The slot holds `V`; the key's type name tells derived values of one `V` from different keys apart.
        let slot = self.cx.slot(("craft-derived", std::any::TypeId::of::<V>(), std::any::type_name::<K>()), || None::<(u128, Arc<V>)>);
        if let Some((kept, value)) = self.cx.peek_state(slot)
            && *kept == hash
        {
            return value.clone();
        }
        let value = Arc::new(build());
        // The node's own slot, which nothing reads with a recorded read: the write marks no one.
        slot.set(self.cx, Some((hash, value.clone())));
        value
    }

    /// Runs `f` on the session slot of type `T` under `id`: memory of the runtime that outlives the
    /// nodes that use it, made with its default on first use.
    pub fn with_session<T: crate::runtime::memory::Slot, R>(&mut self, id: Id, f: impl FnOnce(&mut T) -> R) -> R {
        crate::runtime::memory::with_session(&self.ctx, id, f)
    }

    /// Writes `value` as the saved value under `key`; it is stored through the host when its
    /// serialized form differs from the saved one. Nothing is written in a measuring run.
    pub fn set_saved_value<T: crate::runtime::memory::Savable>(&mut self, key: &crate::runtime::memory::SaveKey, value: T) {
        if !self.is_measuring() {
            crate::runtime::memory::set_saved_value(&self.ctx, key, value);
        }
    }

    /// Runs `f` in a measuring run of this cursor's flow: what it shows only reports its size, no
    /// node is reached and no input is taken, and this cursor does not move. Kept children of the
    /// node being built answer from their size cache.
    pub fn measuring_scope<R>(&mut self, f: impl FnOnce(&mut Ui<'_, '_>) -> R) -> R {
        let under = match self.flow.measuring {
            Some(under) => under,
            None => Some(self.cx.node()),
        };
        let flow = self.flow.nested(self.flow.id, self.available_rect_before_wrap()).measuring(under);
        let mut inner = self.nested_ui(flow);
        f(&mut inner)
    }

    // Edit gestures (section 15.9): the runtime's edit facts.

    /// A gesture id no gesture of this runtime has had: ids grow in the order gestures begin,
    /// across components and windows.
    pub fn next_gesture_id(&mut self) -> craft_behavior::GestureId {
        self.ctx.memory().with(|memory| {
            memory.edits.newest = memory.edits.newest.next();
            memory.edits.newest
        })
    }

    /// Cancels every running gesture: each ends with `reason` at its next value step and stays
    /// cancelled under its press. The app's undo and redo call it before they change the document.
    pub fn cancel_edits(&mut self, reason: craft_behavior::CancelReason) {
        if self.is_measuring() {
            return;
        }
        crate::runtime::edit::cancel_edits(self.cx, &self.ctx, reason);
    }

    /// Runs `body` and says whether a gesture of a component shown inside it was running after the
    /// component's apply step (a drag or a typing run under way). A measuring run sees none.
    pub fn watch_gestures<R>(&mut self, body: impl FnOnce(&mut Ui<'_, '_>) -> R) -> (R, bool) {
        if self.is_measuring() {
            return (body(self), false);
        }
        let before = self.ctx.memory().with(|memory| memory.edits.running);
        let out = body(self);
        let after = self.ctx.memory().with(|memory| memory.edits.running);
        (out, after != before)
    }

    // Id-map lookups (section 15.9): the node a craft `Id` names, through the window's map.

    /// Whether the pointer is over the node `id` names; tracked (`Hovered`).
    pub fn hovers(&mut self, id: Id) -> bool {
        super::ids::hovers(self.cx, id)
    }

    /// Whether the node `id` names has key focus; tracked (`Focused`).
    pub fn is_focused(&mut self, id: Id) -> bool {
        super::ids::is_focused(self.cx, id)
    }

    /// Whether focus is inside the node `scope` names; `None` when no live node answers to it.
    /// Tracked (`FocusWithin`).
    pub fn focus_in_scope(&mut self, scope: Id) -> Option<bool> {
        super::ids::focus_in_scope(self.cx, scope)
    }

    /// Moves key focus to the node `target` names, once the build has made it. `false` when no
    /// live node answers to it, and in a measuring run.
    pub fn request_focus(&mut self, target: crate::runtime::focus::FocusTarget) -> bool {
        !self.is_measuring() && super::ids::request_focus(self.cx, target)
    }

    /// Cancels the press running on the node `owner` names.
    pub fn cancel_press_of(&mut self, owner: Id) {
        if !self.is_measuring() {
            super::ids::cancel_press_of(self.cx, owner);
        }
    }

    /// The part of the node `owner` names that a press runs on, if any.
    pub fn pressed_part(&self, owner: Id) -> Option<craft_behavior::PartRef> {
        super::ids::pressed_part(self.cx, owner)
    }

    /// The window's pointer as a tool reads it: its position, the buttons held, the modifiers and the frame time.
    /// Tracked through the window's pointer cell, so a reader runs again at every pointer move; code that only needs
    /// hover or press on its own node reads its response instead. Presses, releases, wheel and gestures reach the
    /// node they target as its input, so the frame's event fields are empty.
    pub fn pointer(&mut self) -> crate::runtime::pointer::PointerFrame {
        use craft_behavior::{Buttons, PointerButton};
        let facts = self.cx.env().pointer.get(self.cx);
        let buttons = [PointerButton::Primary, PointerButton::Secondary, PointerButton::Middle, PointerButton::Extra1, PointerButton::Extra2]
            .into_iter()
            .filter(|button| facts.held(*button))
            .fold(Buttons::empty(), |held, button| held | Buttons::of(button));
        let mods = self.ctx.modifiers();
        let hit = self.flow.tokens.size.hit;
        crate::runtime::pointer::PointerFrame::held(facts.pos, buttons, mods, self.cx.now(), hit.slop, hit.target)
    }

    /// The focused stop of this window, from the focused node's kept facts. Tracked through the window's focused-node
    /// cell: a reader runs again when focus moves, and at no other input.
    pub fn focused_stop(&mut self) -> Option<crate::runtime::focus::StopRec> {
        let node = self.cx.env().focused.get(self.cx)?;
        let rt = self.cx.runtime();
        let kept = rt
            .local::<super::node::ComponentNode>(node)
            .or_else(|| rt.local::<super::node::ScopeNode>(node))
            .or_else(|| rt.local::<super::overlay::LayerScope>(node).map(|lk| &lk.kept))?;
        let keys = kept.focus.stop?;
        Some(crate::runtime::focus::StopRec::of_kept(kept.id, keys, kept.focus.press_only))
    }

    /// The pointer position when it is inside `rect`. Tracked: a reader runs again at every pointer
    /// move, so code that only needs hover over its own node reads its response instead.
    pub fn hover_point_in(&mut self, rect: Rect) -> Option<Pos2> {
        let pointer = self.cx.env().pointer.get(self.cx);
        pointer.pos.filter(|at| rect.contains(*at))
    }

    // Layer queries (section 15.9): the window's open layers, as each layer child's entry holds them. Each read is
    // tracked through the window's open-layer cell, so a reader runs again when a layer opens, changes or closes.

    fn open_layers(&mut self) -> Vec<super::layers::Open> {
        super::layers::track(self.cx);
        super::layers::open(self.cx.runtime(), self.cx.window())
    }

    /// Whether the layer `id` is open and not fading out.
    pub fn layer_open(&mut self, id: Id) -> bool {
        super::layers::find(&self.open_layers(), id).is_some_and(|open| !open.entry.exiting)
    }

    /// Whether the layer `id` opened in this frame.
    pub fn layer_opening(&mut self, id: Id) -> bool {
        let now = self.cx.now();
        super::layers::find(&self.open_layers(), id).is_some_and(|open| !open.entry.exiting && open.since == Some(now))
    }

    /// The entry of the layer `id`, while it shows.
    pub fn layer_entry(&mut self, id: Id) -> Option<crate::runtime::overlay::OpenEntry> {
        super::layers::find(&self.open_layers(), id).map(|open| open.entry.clone())
    }

    /// Whether a layer that `filter` counts is open in the window.
    pub fn any_layer_open(&mut self, filter: crate::runtime::overlay::OpenFilter) -> bool {
        self.open_layers().iter().any(|open| super::layers::counts(&open.entry, filter))
    }

    /// The top layer that takes keys, if any.
    pub fn top_layer(&mut self) -> Option<crate::runtime::overlay::OpenEntry> {
        let open = self.open_layers();
        open.into_iter().rev().find(|open| super::layers::counts(&open.entry, crate::runtime::overlay::OpenFilter::Keys)).map(|open| open.entry)
    }

    /// The id of the layer this cursor builds in; `None` on the window's content.
    pub fn layer_parent(&mut self) -> Option<Id> {
        let open = self.open_layers();
        let child = super::layers::layer_child_of(self.cx.runtime(), self.cx.node())?;
        open.iter().find(|o| o.node == child).map(|o| o.entry.id).or_else(|| self.layer_body_id(child))
    }

    /// The id of the layer child `node` from its kept facts, for the layer whose body is running (its entry is not
    /// readable while its own build runs).
    fn layer_body_id(&self, node: craft_ui_rt::NodeId) -> Option<Id> {
        super::ids::id_of(self.cx, node)
    }

    /// Whether this cursor builds inside the top modal layer.
    pub fn in_top_modal(&mut self) -> bool {
        super::layers::track(self.cx);
        super::layers::in_top_modal(self.cx.runtime(), self.cx.node())
    }

    /// Whether the layer `id` is the top modal layer of the window.
    pub fn is_top_modal(&mut self, id: Id) -> bool {
        super::layers::track(self.cx);
        super::layers::top_modal(self.cx.runtime(), self.cx.window()).is_some_and(|top| top.entry.id == id)
    }

    /// The dialog of a dialog host this cursor builds in (`stack_place`), if it builds in one.
    pub fn running_dialog(&mut self) -> Option<Id> {
        super::overlay::running_dialog(self.cx)
    }

    // Focus services (section 15.9).

    /// Moves focus back out of the layer a pick was made in, before the pick's command runs, so the command reaches
    /// the stop focus returns to: the layer's focus return (its opener), as its closing would, and past a menu bar
    /// that F10 or a lone Alt focused, to the stop the bar took the focus from. Nothing in a measuring
    /// run, or when the layer has no live node.
    pub fn return_from_pick(&mut self, picked: crate::runtime::focus::PickedIn) {
        if self.is_measuring() {
            return;
        }
        let layer = match picked {
            crate::runtime::focus::PickedIn::Menus(layer) | crate::runtime::focus::PickedIn::Layer(layer) => layer,
        };
        let Some(node) = super::ids::node_of(self.cx, layer) else { return };
        let rt = self.cx.runtime();
        // A menu pick walks out of every menu under the first: the outermost layer child that has a return names it.
        let mut to = rt.focus_return(self.cx.window(), node);
        if matches!(picked, crate::runtime::focus::PickedIn::Menus(_)) {
            while let Some(at) = to {
                match super::layers::layer_child_of(rt, at).and_then(|child| rt.focus_return(self.cx.window(), child)) {
                    Some(next) => to = Some(next),
                    None => break,
                }
            }
        }
        if let Some(to) = to {
            let to = super::node::MenuBar::past_bar(self.cx, to);
            self.cx.focus_node(to);
        }
    }

    /// The id of the focus scope of the top layer that takes keys, if one is open: the scope a newly opened layer
    /// nests in. Tracked through the open-layer cell.
    pub fn open_focus_scope(&mut self) -> Option<Id> {
        self.top_layer().filter(|entry| entry.scoped).map(|entry| entry.id)
    }

    /// The menu layer this cursor builds in, if it builds in one.
    pub fn running_menu(&mut self) -> Option<Id> {
        let of = self.cx.provided::<super::layers::LayerChildOf>()?;
        (of.1 == Some(craft_behavior::OwnerClass::Menu)).then_some(of.0)
    }

    // Window services (section 15.9).

    /// Adds the running tool's `gesture` to what a modal layer lets through to `canvas`; it applies from the modal
    /// body's next run. Nothing in a measuring run.
    pub fn pass_tool_gesture(&mut self, canvas: Id, gesture: crate::runtime::sampler::CanvasGesture) {
        if !self.is_measuring() {
            crate::runtime::sampler::pass_tool_gesture(&self.ctx, canvas, gesture);
        }
    }

    /// The width of `column` in the column group the innermost open frame reads, `None` when no group is open; the row
    /// reports `content`.
    pub fn ambient_width(&mut self, column: &craft_behavior::ColumnKey, content: f32) -> Option<f32> {
        crate::layout::ambient_width(&self.ctx, column, content)
    }

    /// Shows `rows` beside the pointer at `at` during a gesture, in this scope's env: a layer child of this node on the
    /// drag layer, which takes no input, recording [`ghost::record_readout`](crate::contract::ghost::record_readout).
    /// It shows while this node's builds keep calling it and goes with the first build that does not. Nothing in a
    /// measuring run.
    pub fn ghost_readout(&mut self, at: Pos2, rows: &[(crate::runtime::env::Label, String)]) {
        if self.is_measuring() || rows.is_empty() {
            return;
        }
        let env = self.env.clone();
        let window = Rect::from_min_size(Pos2::ZERO, self.ctx.content_rect().max.to_vec2());
        let id = self.flow.id.with("craft-ghost-readout");
        let show = super::overlay::LayerShow {
            layer: craft_ui_rt::Layer::Drag,
            spec: craft_ui_rt::LayerSpec::new(craft_ui_rt::Placement::Fill),
            surface: super::overlay::Surface::Plain,
            pad: 0.0,
            share: Vec2::splat(1.0),
            anchor: None,
            role: accesskit::Role::GenericContainer,
            hover: false,
            sweep: None,
            focus: crate::runtime::focus::FocusPolicy::NONE,
            entry: None,
            host: None,
            on_activate: false,
            enter: false,
        };
        let key = craft_ui_rt::hash_key(&id);
        super::overlay::show_layer(self.cx, id, key, show, true, &mut |ui: &mut Ui<'_, '_>| {
            let draw = ui.draw();
            crate::contract::ghost::record_readout(&draw, &env.view(), at, window, rows);
        });
    }

    /// Asks the collection `collection` to start renaming the item `key`: the collection builds in a later round of
    /// this frame and takes the request in its item step. Nothing in a measuring run.
    pub fn request_rename<K: Clone + Send + Sync + 'static>(&mut self, collection: Id, key: &K) {
        if !self.is_measuring() {
            crate::contract::items::request_rename(&self.ctx, collection, key);
        }
    }

    // Crate-private surface for ui-contract's own code (section 15.2).

    /// A fresh id under this cursor's, unique among the ids it handed out in this run.
    pub(crate) fn unique_id(&mut self) -> Id {
        self.flow.id.with(("unique", self.flow.sizes.len(), self.flow.placed.len(), self.flow.on_line))
    }

    /// The opacity content built through this cursor paints at.
    pub(crate) fn opacity(&self) -> f32 {
        self.flow.opacity
    }

    /// Content built through this cursor paints at `opacity` (multiplied into the parent's).
    pub(crate) fn set_opacity(&mut self, opacity: f32) {
        self.flow.opacity = opacity.clamp(0.0, 1.0);
    }

    /// A child cursor in `rect`, with ids under `salt`: [`Ui::scope_builder`] for the bridge.
    pub(crate) fn child<R>(&mut self, rect: Rect, salt: impl Hash, f: impl FnOnce(&mut Ui<'_, '_>) -> R) -> R {
        self.scope_builder(UiBuilder::new().max_rect(rect).id_salt(salt), f)
    }

    /// Whether the scope is enabled (the env's `enabled`).
    pub(crate) fn is_enabled(&self) -> bool {
        self.env.enabled
    }

    /// The children's extent reaches `x` horizontally.
    pub(crate) fn expand_to_include_x(&mut self, x: f32) {
        let extent = &mut self.flow.extent;
        if extent.is_negative() {
            let y = self.flow.region.min.y;
            *extent = Rect::from_min_max(Pos2::new(x, y), Pos2::new(x, y));
        } else {
            extent.min.x = extent.min.x.min(x);
            extent.max.x = extent.max.x.max(x);
        }
    }

    /// The room before a wrapping row would wrap.
    pub(crate) fn available_size_before_wrap(&self) -> Vec2 {
        self.flow.room()
    }

    /// The current line of a `Row` flow is at least `height` high.
    pub(crate) fn set_row_height(&mut self, height: f32) {
        self.flow.set_row_height(height);
    }

    /// The next child of a `Row` flow starts a new line under the current one.
    pub(crate) fn end_row(&mut self) {
        self.flow.end_row();
    }
}
