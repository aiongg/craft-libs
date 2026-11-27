//! Containers on the `Ui` cursor (docs/contract-rt.md sections 2.4, 3.3, 7 and 15.1): the regions of
//! `ContainerProps` (header with its controls at the end, end column, body, the region under the body,
//! footer), the rules and rhythm bands between them, the body's `BodyLayout` with its plan, collapse,
//! and the scroll body.
//!
//! A container's body closure runs whenever its scope's build reaches it. Its own hooks (update, paint,
//! access) run only when its key moved, it is marked, or its size moved; a region whose subtree is
//! marked runs only its own hook. Each region is a node of its own, so a kept region keeps its
//! children while the body's are reached again. The region hooks and the body get a `BodyCx` over the
//! container's step 1 (`Resolved`), its env and the body's memory.
//!
//! The plan is the body's measurement: where a `Row` block starts and how far its children spread,
//! a `Grid`'s column widths and row heights (or one column when `GridFit::Stack` and the room is too
//! narrow), and each fill child's share. It comes from a measuring run of the body, in which kept
//! children answer from their size cache and nothing runs, and it is reused while the container's
//! key, its body's room and its content are unchanged. When the real run finds children of other
//! sizes than a reused plan has, the container builds again in the same frame and measures.
//!
//! Under a right-to-left env the flow mirrors `Row` and `Grid` placement inside the body
//! (`BodyFlow::rtl`) and the end column lies at the start edge.

use std::sync::Arc;

use craft_behavior::{ColumnWidth, Mixed, resolve_columns};
use craft_theme::Tokens;
use craft_ui_rt::{BuildCx, Constraints, Element, InlineState, LayoutCx, NodeId, PaintCx, Pos2, Rect, Rgba, State, Vec2};

use super::key::{component_key, fold, type_key};
use super::node::{ComponentNode, Kept, Place, Placed, ScopeNode};
use super::run::{Ran, finish_measured, holds, reach_layers, run, settle, sweep_tag};
use super::scroll::{ScrollKept, ScrollNode, overscan};
use super::sweep::Sweep;
use super::ui::{Dir, Flow, Ui};
use crate::contract::cx::BodyCx;
use crate::contract::run::ShowAt;
use crate::contract::{
    Container, ContainerProps, EndRegion, FooterPlace, FooterRegion, HeaderRegion, Id, Inner, Internal, PadEdges, Scroll, SectionRhythm, Sizing,
    TipFn, steps,
};
use crate::layout::{BodyLayout, ColumnGroup, GridFit, RowAlign};
use crate::runtime::env::Env;
use crate::runtime::memory::{MemoryStage, sub};

/// A container's last measurement of its body (module docs).
#[derive(Clone, Debug)]
pub(crate) struct Plan {
    /// The body's room the plan was made for.
    room: Vec2,
    /// The children's sizes and whether each filled, from the measuring run.
    sizes: Vec<(Vec2, bool)>,
    /// The room each child had in the measuring run; the real run gives it the same.
    rooms: Vec<Vec2>,
    dir: Dir,
    shares: Vec<f32>,
    /// The level of each levelled unit of a `Row` body (`plan_levels`), in order.
    levels: Vec<usize>,
}

/// What a container node keeps besides its component's outputs: the open state of a collapsing body.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BodyKept {
    /// The open state when no binding owns it (`Container::open` is `None`); `None` before the first build.
    pub(crate) open: Option<bool>,
    /// The body's last measuring run met levelled units: its real runs follow a plan.
    pub(crate) levelled: bool,
}

/// What a body's flow does beyond placing children by its `Dir`: added to `Flow` (`Flow::body`).
#[derive(Clone, Debug, Default)]
pub(crate) struct BodyFlow {
    /// The body mirrors its children's placement inside its region (a right-to-left `Row` or `Grid`).
    pub(crate) rtl: bool,
    /// Each levelled unit's width per level, in order (a measuring run records them).
    pub(crate) units: Vec<Vec<f32>>,
    /// The level the plan gave each levelled unit, in order, and how many the real run showed so far.
    given: Vec<usize>,
    next: usize,
}

impl BodyFlow {
    /// The level of the next levelled unit: the plan's, or 0 without one.
    pub(crate) fn next_level(&mut self) -> usize {
        let level = self.given.get(self.next).copied().unwrap_or(0);
        self.next += 1;
        level
    }

    /// `rect`, placed in `region`, mirrored when the flow is right to left.
    pub(crate) fn place(&self, rect: Rect, region: Rect) -> Rect {
        if !self.rtl || !region.max.x.is_finite() {
            return rect;
        }
        let x = region.min.x + region.max.x - rect.max.x;
        Rect::from_min_size(Pos2::new(x, rect.min.y), rect.size())
    }
}

/// The rule bands between a container's regions (`HeaderRegion::rule`, `rule_above`, `FooterRegion::rule`,
/// `EndRegion::rule`) and the rhythm bands a spaced section rhythm adds around a rule above the header.
#[derive(Clone, Copy, Debug, Default)]
struct Bands {
    /// The room before the rule above the header (spaced rhythm, not at the flow's start).
    lead: f32,
    /// The rule above the header.
    above: f32,
    /// The room after the rule above the header (spaced rhythm).
    trail: f32,
    /// The rule under a stacked header.
    under: f32,
    /// The rule above the footer.
    footer: f32,
    /// The rule at the end region's inner edge.
    end: f32,
}

impl Bands {
    fn of(props: &ContainerProps, env: &Env, tokens: &Tokens, at_start: bool) -> Bands {
        let width = tokens.stroke.thin;
        let on = |rule: bool| if rule { width } else { 0.0 };
        let above = on(!at_start && props.header.is_some_and(|h| h.rule_above));
        let spaced = above > 0.0 && env.get::<SectionRhythm>() == SectionRhythm::Spaced;
        Bands {
            lead: if spaced { tokens.space.gap.section } else { 0.0 },
            above,
            trail: if spaced { tokens.space.gap.row } else { 0.0 },
            under: on(props.header.is_some_and(|h| h.rule && !h.inline)),
            footer: on(props.footer.is_some_and(|f| f.rule)),
            end: on(props.end.is_some_and(|e| e.rule)),
        }
    }

    fn top(&self) -> f32 {
        self.lead + self.above + self.trail
    }
}

/// The facts of `ContainerProps` the layout reads, resolved against the tokens.
struct Shape {
    layout: BodyLayout,
    pad: Vec2,
    sizing: Sizing,
    gap: f32,
    header: Option<HeaderRegion>,
    header_gap: f32,
    end: Option<EndRegion>,
    end_gap: f32,
    footer: Option<FooterRegion>,
    footer_gap: f32,
    scroll: Option<Scroll>,
    least: Vec2,
    /// The height the body offers its children in place of its own (`ContainerProps::room_height`).
    room_height: Option<f32>,
    bands: Bands,
    rtl: bool,
    /// The body is collapsed: it shows no children and takes no room.
    closed: bool,
    /// The labelled rows of the body share the widest label as one column (`ContainerProps::label_column`).
    label_column: bool,
}

impl Shape {
    fn of(props: &ContainerProps, env: &Env, tokens: &Tokens, at_start: bool) -> Shape {
        let pad = props.pad.points(tokens);
        let pad = match props.pad_edges {
            PadEdges::Block => Vec2::new(0.0, pad),
            PadEdges::Inline => Vec2::new(pad, 0.0),
            _ => Vec2::splat(pad),
        };
        let gap = props.body_gap.points_in(tokens);
        Shape {
            layout: props.layout.clone(),
            pad,
            sizing: props.sizing,
            gap,
            header: props.header,
            header_gap: props.header.and_then(|h| h.gap).map_or(gap, |g| g.points_in(tokens)),
            end: props.end,
            end_gap: props.end.map_or(0.0, |e| e.gap.points_in(tokens)),
            footer: props.footer,
            footer_gap: props.footer.and_then(|f| f.gap).map_or(gap, |g| g.points_in(tokens)),
            scroll: props.scroll.filter(|_| !props.collection_body),
            least: Vec2::new(props.least_width.map_or(0.0, |l| l.points(tokens)), 0.0),
            room_height: props.room_height,
            bands: Bands::of(props, env, tokens, at_start),
            rtl: env.direction.is_rtl(),
            closed: false,
            label_column: props.label_column,
        }
    }

    fn has_regions(&self) -> bool {
        self.header.is_some() || self.end.is_some() || self.footer.is_some() || self.scroll.is_some() || self.bands.top() > 0.0 || self.closed
    }

    /// The container takes a share of what its parent's flow leaves along that flow's axis.
    fn fills(&self, parent: &Flow) -> bool {
        match self.sizing {
            Sizing::Fill => true,
            Sizing::FillHeight => parent.is_stack(),
            _ => false,
        }
    }

    /// The body's flow beyond its direction, with the levels a plan gave its levelled units.
    fn body_flow(&self, given: Vec<usize>) -> BodyFlow {
        BodyFlow { rtl: self.rtl && !matches!(self.layout, BodyLayout::Stack), given, ..BodyFlow::default() }
    }

    /// The flow of the body before any plan: `BodyLayout` as a direction.
    fn base_dir(&self, columns: usize) -> Dir {
        match &self.layout {
            BodyLayout::Row { wrap, .. } => Dir::Row { wrap: *wrap, start: 0.0, spread: 0.0 },
            BodyLayout::Grid { cells, .. } => Dir::Grid { widths: vec![0.0; columns.max(1)], heights: Vec::new(), cells: *cells },
            _ => Dir::Stack,
        }
    }

    fn columns(&self) -> &[ColumnWidth] {
        match &self.layout {
            BodyLayout::Grid { columns, .. } => columns,
            _ => &[],
        }
    }

    /// The body needs a plan: an aligned or spread `Row`, one with levelled units, a `Grid`, or fill children last
    /// time.
    fn needs_plan(&self, had_fills: bool, levelled: bool) -> bool {
        match &self.layout {
            BodyLayout::Row { wrap: false, align, .. } => *align != RowAlign::Start || had_fills || levelled,
            BodyLayout::Row { .. } => false,
            BodyLayout::Grid { .. } => true,
            _ => had_fills,
        }
    }

    /// The plan for a body of `room` whose measuring run found `sizes`.
    fn plan(&self, room: Vec2, sizes: Vec<(Vec2, bool)>, rooms: Vec<Vec2>, units: &[Vec<f32>], tokens: &Tokens) -> Plan {
        let fills = sizes.iter().filter(|(_, f)| *f).count();
        let gaps = self.gap * sizes.len().saturating_sub(1) as f32;
        // The levelled units take their levels in the room the other children leave (at their widest they stood in
        // the measuring run's sizes).
        let levels = if units.is_empty() {
            Vec::new()
        } else {
            let widest: f32 = units.iter().map(|u| u.first().copied().unwrap_or(0.0)).sum();
            let others: f32 = sizes.iter().map(|(s, _)| s.x).sum::<f32>() - widest + gaps;
            crate::layout::plan_levels(units, (room.x - others).max(0.0))
        };
        let (dir, shares) = match &self.layout {
            BodyLayout::Row { wrap, align, .. } => {
                let used: f32 = sizes.iter().filter(|(_, f)| !*f).map(|(s, _)| s.x).sum::<f32>() + gaps;
                let left = (room.x - used).max(0.0);
                let shares = if fills > 0 { vec![left / fills as f32; fills] } else { Vec::new() };
                let left = if fills > 0 { 0.0 } else { left };
                let (start, spread) = match align {
                    RowAlign::Center => (left / 2.0, 0.0),
                    RowAlign::End => (left, 0.0),
                    RowAlign::SpaceBetween if sizes.len() > 1 => (0.0, left / (sizes.len() - 1) as f32),
                    _ => (0.0, 0.0),
                };
                (Dir::Row { wrap: *wrap, start, spread }, shares)
            }
            BodyLayout::Grid { cells, fit, .. } => {
                let columns: Vec<ColumnWidth> = self.columns().iter().map(|c| c.in_unit(tokens.size.button)).collect();
                let n = columns.len().max(1);
                let mut content = vec![0.0_f32; n];
                for (i, (size, _)) in sizes.iter().enumerate() {
                    content[i % n] = content[i % n].max(size.x);
                }
                let least = content.iter().sum::<f32>() + self.gap * n.saturating_sub(1) as f32;
                // A grid whose columns do not fit the room stacks its cells in one column (`GridFit::Stack`).
                let n = if *fit == GridFit::Stack && least > room.x && n > 1 { 1 } else { n };
                let mut rows = vec![0.0_f32; sizes.len().div_ceil(n)];
                for (i, (size, _)) in sizes.iter().enumerate() {
                    rows[i / n] = rows[i / n].max(size.y);
                }
                let widths = if n == 1 && columns.len() > 1 { vec![room.x] } else { resolve_columns(&columns, &content, room.x, self.gap) };
                (Dir::Grid { widths, heights: rows, cells: *cells }, Vec::new())
            }
            _ => {
                let used: f32 = sizes.iter().filter(|(_, f)| !*f).map(|(s, _)| s.y).sum::<f32>() + gaps;
                let shares = if fills > 0 { vec![((room.y - used) / fills as f32).max(0.0); fills] } else { Vec::new() };
                (Dir::Stack, shares)
            }
        };
        Plan { room, sizes, rooms, dir, shares, levels }
    }

    /// The container's size from its inner content size, by its sizing, in `room`. A scroll body's inner size is
    /// already its view: as large as its content until the content outgrows the room.
    fn outer(&self, inner: Vec2, room: Vec2) -> Vec2 {
        let content = (inner + 2.0 * self.pad).max(self.least);
        match self.sizing {
            Sizing::Fill => room,
            Sizing::FillWidth => Vec2::new(room.x, content.y),
            Sizing::FillHeight => Vec2::new(content.x, room.y),
            _ => content,
        }
    }

    /// The axes along which the container takes its room rather than its content: (x, y).
    fn takes_room(&self) -> (bool, bool) {
        match self.sizing {
            Sizing::Fill => (true, true),
            Sizing::FillWidth => (true, false),
            Sizing::FillHeight => (false, true),
            _ => (false, false),
        }
    }
}

impl Flow {
    pub(crate) fn is_stack(&self) -> bool {
        matches!(self.dir_ref(), Dir::Stack)
    }

    pub(crate) fn is_grid(&self) -> bool {
        matches!(self.dir_ref(), Dir::Grid { .. })
    }

    /// The column the next child takes in a `Grid` flow; `None` in any other.
    pub(crate) fn grid_column(&self) -> Option<usize> {
        let Dir::Grid { widths, .. } = self.dir_ref() else { return None };
        Some(self.sizes.len() % widths.len().max(1))
    }
}

impl Ui<'_, '_> {
    /// Runs `body` as a measuring run in a column `width` wide at the next place and returns what it took; kept
    /// children under this build's node answer from their size cache.
    pub(crate) fn measure_column(&mut self, width: f32, body: &mut dyn FnMut(&mut Ui<'_, '_>)) -> Vec2 {
        let start = self.flow.next_min();
        let tokens = self.flow.tokens.clone();
        let under = self.flow.measuring.unwrap_or_else(|| Some(self.cx.node()));
        let region = Rect::from_min_size(start, Vec2::new(width, f32::INFINITY));
        let flow = Flow::new(self.flow.id.with("slot"), self.flow.origin, region, tokens.space.gap.stack, tokens).measuring(under);
        let mut inner = Ui::new(&mut *self.cx, flow);
        body(&mut inner);
        inner.flow.content_size()
    }

    /// Runs `show` in a flow of its own at the next place, a column under the unit's id (a levelled unit's content,
    /// a page of `Pages`), and returns what it took, at least `least`. A measuring unit takes no room (the caller
    /// allocates); one that `places` is allocated in this flow, and its children are placed with this flow's.
    pub(crate) fn unit(&mut self, key: Id, places: bool, least: Vec2, show: &mut dyn FnMut(&mut Ui<'_, '_>)) -> Vec2 {
        let start = self.flow.next_min();
        let room = self.flow.room();
        let width = if places { room.x } else { f32::INFINITY };
        let region = Rect::from_min_size(start, Vec2::new(width, room.y));
        let tokens = self.flow.tokens.clone();
        let mut flow = Flow::new(self.flow.id.with(("unit", key)), self.flow.origin, region, tokens.space.gap.stack, tokens)
            .in_view(self.flow.view)
            .with_sweep(self.flow.sweep);
        flow.measuring = self.flow.measuring;
        let mut inner = Ui::new(&mut *self.cx, flow);
        show(&mut inner);
        let inner = inner.flow;
        let size = inner.content_size().max(least);
        self.flow.absorb(&inner);
        if places {
            self.flow.allocate(None, size, false);
            self.flow.placed.extend(inner.placed);
        }
        size
    }
}

/// The region nodes' and rules' keys under a container node.
#[derive(Clone, Copy, Debug, Hash)]
enum Region {
    Header,
    Controls,
    End,
    Body,
    BodyEnd,
    Footer,
    RuleAbove,
    RuleUnder,
    RuleFooter,
    RuleEnd,
}

/// A rule between regions: a band its node fills with the divider colour.
struct RuleNode {
    color: Rgba,
}

impl Element for RuleNode {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        u64::from_le_bytes([self.color.0[0], self.color.0[1], self.color.0[2], self.color.0[3], 0, 0, 0, 0])
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn layout(&self, _: &mut (), _: &mut LayoutCx<'_>, constraints: Constraints) -> Vec2 {
        constraints.max
    }

    fn paint(&self, _: &(), cx: &mut PaintCx<'_>) {
        let bounds = cx.bounds();
        cx.fill_rect(bounds, self.color);
    }
}

/// What a container's layout knows of the node it builds into, read before its children are
/// reached.
struct Host<'k> {
    /// The node; `None` in a measuring run of a container no build showed yet.
    node: Option<NodeId>,
    /// The container's own hooks keep their outputs (real run only).
    own_kept: bool,
    /// Something under the node is marked.
    marked: bool,
    /// The node's kept outputs; `None` in a measuring run.
    kept: Option<&'k mut Kept>,
    /// The window's toggle sweep record, handed to every flow under the container.
    sweep: Option<State<Sweep>>,
    /// The window's context.
    ctx: crate::contract::Context,
}

/// Where a body's children live and what decides whether its plan holds.
#[derive(Clone, Copy)]
struct BodyAt {
    node: Option<NodeId>,
    own_kept: bool,
    marked: bool,
    sweep: Option<State<Sweep>>,
}

/// What every region hook and the body share in one build of a container: the env the container's
/// children are placed under, its step 1, the body's memory and its open state. Each hook call gets a
/// `BodyCx` over it at the region's rect.
struct Frame<'f> {
    body_id: Id,
    /// The env the regions are built under (`regions_env`).
    env: &'f Env,
    /// The env the body is built under: the regions' env with the footer slot of a footer the body places.
    body_env: Option<Env>,
    resolved: &'f crate::contract::Resolved<'f>,
    stage: MemoryStage,
    open: bool,
    /// The open state at the start of the build, to tell a header's toggle.
    open_at_read: bool,
    close: Option<crate::runtime::overlay::CloseReason>,
    /// A real run called the body.
    ran: bool,
}

impl Frame<'_> {
    /// Runs a region hook at `rect` under the regions' env.
    fn with<T>(&mut self, ui: &mut Ui<'_, '_>, rect: Rect, f: impl FnOnce(&mut Ui<'_, '_>, &mut BodyCx<'_>) -> T) -> T {
        let env = self.env.clone();
        self.run(ui, env, rect, f)
    }

    /// Runs the body at `rect` under the body's env.
    fn with_body<T>(&mut self, ui: &mut Ui<'_, '_>, rect: Rect, f: impl FnOnce(&mut Ui<'_, '_>, &mut BodyCx<'_>) -> T) -> T {
        let env = self.body_env.clone().unwrap_or_else(|| self.env.clone());
        self.run(ui, env, rect, f)
    }

    fn run<T>(&mut self, ui: &mut Ui<'_, '_>, env: Env, rect: Rect, f: impl FnOnce(&mut Ui<'_, '_>, &mut BodyCx<'_>) -> T) -> T {
        let mut bcx = BodyCx::new(self.body_id, self.env.view(), self.resolved, rect, &mut self.stage, &mut self.open, None, None);
        let out = Env::run_in(ui, env, |ui| f(ui, &mut bcx));
        if self.close.is_none() {
            self.close = bcx.close_requested();
        }
        out
    }
}

/// The env a container's regions and body are built under: the place env with what the props give the container's
/// content (its enabled state, label scheme, control size and glyph role, fallback collection, paragraph width), and
/// the room, inside label and cell fill of the container's own place cleared.
fn regions_env(props: &ContainerProps, place: &Env, enabled: bool, takes_width: bool) -> Env {
    place.derive(|scoped| {
        scoped.enabled &= enabled;
        if let Some(collection) = props.fallback_collection {
            scoped.fallback_collection = Some(collection);
        }
        if let Some(scheme) = props.labels {
            scoped.inherited_mut().values.set(scheme);
        }
        if let Some(size) = props.child_size {
            scoped.size = size;
        }
        if let Some(glyphs) = props.child_glyphs {
            scoped.glyphs = glyphs;
        }
        scoped.start_room();
        scoped.inside_label = None;
        scoped.cell_fills = false;
        // A container given its width (a resized dialog) leaves its paragraphs the room it has.
        if let Some(wrap) = props.wrap_width.filter(|_| !takes_width) {
            scoped.measure = Some(wrap.points(scoped.tokens.tokens()));
        }
    })
}

/// Shows a container: its body always runs; its own hooks run when its key, size or a region's
/// content moved, or it is marked. In a measuring run it runs its regions and body as a measuring
/// run, under its kept node, and reports its size. `place` is the env the container's children are
/// placed under.
pub(crate) fn show_container<C: Container, R>(
    slot: &mut Option<C>,
    ui: &mut Ui<'_, '_>,
    (at, place): (ShowAt, Env),
    body: &mut dyn FnMut(&mut Ui<'_, '_>, &mut BodyCx<'_>) -> R,
    tip: Option<TipFn<'_>>,
) -> Inner<C::Output, R> {
    let mut component = slot.take().expect("a container is shown once");
    let begun = super::run::begin(&mut component, ui);
    let common = &begun.common;
    let command = begun.step1.command.clone();
    let (id, child) = ui.flow.child(type_key::<C>(), common.id.clone());
    let key = component_key(&component);
    let props = component.props(Internal::new()).clone();
    let at_start = ui.flow.sizes.is_empty();
    let mut shape = Shape::of(&props, &place, &ui.flow.tokens, at_start);
    let fills = shape.fills(&ui.flow);
    let (origin, room) = match at {
        ShowAt::Rect(rect) => (rect.min, rect.size()),
        ShowAt::Flow if fills => (ui.flow.next_min(), ui.flow.fill_room()),
        ShowAt::Flow => (ui.flow.next_min(), ui.flow.room()),
    };
    let tag = sweep_tag(&component, &ui.flow);
    let sweep = ui.flow.sweep;
    let (scope, roving) = (props.focus, props.roving);
    let body_id = sub(id, "body");
    let ctx = ui.ctx().clone();
    let resolved = begun.resolved(id);
    let measuring = ui.flow.measuring;
    let stage = if measuring.is_some() { MemoryStage::open_peek(&ctx, body_id) } else { MemoryStage::open(&ctx, body_id) };
    let regions = regions_env(&props, &place, resolved.enabled, shape.takes_room().0);
    let mut frame =
        Frame { body_id, env: &regions, body_env: None, resolved: &resolved, stage, open: true, open_at_read: true, close: None, ran: false };
    let mut inner_value = None;
    let mut call = |ui: &mut Ui<'_, '_>, bcx: &mut BodyCx<'_>| {
        inner_value = Some(body(ui, bcx));
    };
    if let Some(under) = measuring {
        let peeked = under.and_then(|u| ui.cx.peek::<ComponentNode>(u, child)).map(|p| (p.node, p.local.body.open));
        let (node, kept_open) = (peeked.map(|p| p.0), peeked.and_then(|p| p.1));
        let open = read_open(&mut component, &props, kept_open, &ctx);
        (frame.open, frame.open_at_read) = (open, open);
        shape.closed = !open;
        let tokens = ui.flow.tokens.clone();
        let into = Host { node, own_kept: false, marked: true, kept: None, sweep, ctx: ctx.clone() };
        let (size, flow) = lay_out(&mut component, ui.cx, &shape, into, &mut frame, (id, origin, room), &tokens, &mut call);
        ui.flow.absorb(&flow);
        let rect = match at {
            ShowAt::Flow => ui.flow.allocate(None, size, fills),
            ShowAt::Rect(rect) => Rect::from_min_size(rect.min, size),
        };
        let value = steps::value(&mut component, &mut MemoryStage::open_peek(&ctx, id), id, command.as_deref());
        let output = finish_measured(component, id, rect, &begun, value, ctx.clone());
        return Inner { output, inner: inner_value, settle: None };
    }
    let Ui { cx, flow, .. } = ui;
    let output = cx.keyed(child, |cx| {
        cx.inline(ComponentNode { key: key.map_or(0, fold), tag, stop: C::STOPS, text: component.text_input() }, |kept, state, cx| {
            // The kept stage runs in this window for this node; its session and saved slots go back to the runtime's
            // store when the run ends.
            kept.memory = std::mem::take(&mut kept.memory).within(&ctx, id).appearing(state.new);
            let value = steps::value(&mut component, &mut kept.memory, id, command.as_deref());
            let open = read_open(&mut component, &props, kept.body.open, &ctx);
            (frame.open, frame.open_at_read) = (open, open);
            shape.closed = !open;
            let own_kept = holds(kept, key, state);
            let tokens = if own_kept { flow.tokens.clone() } else { super::env::tokens(cx) };
            let into = Host { node: Some(cx.node()), own_kept, marked: state.path || state.marked, kept: Some(&mut *kept), sweep, ctx: ctx.clone() };
            crate::layout::begin_capture(&ctx);
            let (size, inner) = lay_out(&mut component, cx, &shape, into, &mut frame, (id, origin, room), &tokens, &mut call);
            kept.column_reports = crate::layout::end_capture(&ctx);
            flow.absorb(&inner);
            if frame.open != frame.open_at_read {
                write_open(&mut component, kept, frame.open, &resolved, &props, &ctx);
            }
            let rect = match at {
                ShowAt::Flow => flow.allocate(Some(cx.node()), size, fills),
                ShowAt::Rect(rect) => {
                    let rect = Rect::from_min_size(rect.min, size);
                    flow.placed.push(Placed { node: cx.node(), offset: rect.min - flow.origin, size });
                    rect
                }
            };
            if let Some(why) = frame.close {
                component.closed(why);
            }
            if own_kept && size == kept.size {
                kept.cache(room, size);
                // The body reconciled the children; the layer children are reached the same way.
                reach_layers(&mut component, kept, cx, rect, &begun, &tokens, sweep, ctx.clone());
                let output = settle(component, kept, cx, rect, &begun, value, ctx.clone());
                kept.memory.flush();
                return output;
            }
            let ran = Ran {
                id,
                key,
                place: Place { room },
                size,
                rect,
                tokens: &tokens,
                sweep,
                begun: &begun,
                value,
                tip,
                context: ctx.clone(),
                flow_reveal: &mut flow.reveal,
            };
            let output = run(component, kept, cx, ran);
            // The props' scope and roving group, where the component's hooks name none.
            if scope != crate::runtime::focus::FocusPolicy::NONE {
                kept.focus.scope = scope;
            }
            if roving.is_some() {
                kept.focus.roving = roving;
            }
            kept.memory.flush();
            output
        })
    });
    let ran = frame.ran;
    frame.stage.write_back();
    let settle = ran.then(|| crate::contract::settle::SettleAt { ctx, body: body_id, owner: id });
    Inner { output, inner: inner_value, settle }
}

/// Shows a container at `rect` and takes the rect's room in the flow of `ui`: what is shown after
/// it lies past it.
pub(crate) fn show_container_taking<C: Container, R>(
    slot: &mut Option<C>,
    ui: &mut Ui<'_, '_>,
    rect: Rect,
    body: &mut dyn FnMut(&mut Ui<'_, '_>, &mut BodyCx<'_>) -> R,
) -> Inner<C::Output, R> {
    let place = Env::of(ui);
    let shown = show_container(slot, ui, (ShowAt::Rect(rect), place), body, None);
    ui.advance_cursor_after_rect(rect);
    shown
}

/// A container's size for `proposal` without showing it: a measuring run of its regions and body under the id the
/// next `show` takes, in which kept children answer from their size cache. A kept container node with an unchanged
/// key and no mark answers from its own size cache without running anything.
pub(crate) fn measure_container<C: Container, R>(
    component: &mut C,
    ui: &mut Ui<'_, '_>,
    proposal: craft_behavior::Proposal,
    body: &mut dyn FnMut(&mut Ui<'_, '_>, &mut BodyCx<'_>) -> R,
) -> Vec2 {
    let room = match proposal {
        craft_behavior::Proposal::Size(size) => size,
        craft_behavior::Proposal::Width(width) => Vec2::new(width, ui.flow.room().y),
        craft_behavior::Proposal::Min => Vec2::ZERO,
        _ => ui.flow.room(),
    };
    let begun = super::run::begin(component, ui);
    let (id, child) = ui.flow.peek_child(type_key::<C>(), begun.common.id.clone());
    let key = component_key(component);
    let under = ui.flow.measuring.unwrap_or_else(|| Some(ui.cx.node()));
    let peeked = under.and_then(|u| ui.cx.peek::<ComponentNode>(u, child)).map(|p| {
        let cached = holds(p.local, key, p.state).then(|| p.local.cached(room)).flatten();
        (p.node, p.local.body.open, cached, p.local.column_reports.clone())
    });
    let ctx = ui.ctx().clone();
    if let Some((_, _, Some(size), reports)) = &peeked {
        crate::layout::replay(&ctx, reports);
        *component.common_mut(Internal::new()) = begun.common;
        return *size;
    }
    let props = component.props(Internal::new()).clone();
    let mut shape = Shape::of(&props, &begun.env, &ui.flow.tokens, ui.flow.sizes.is_empty());
    let open = read_open(component, &props, peeked.as_ref().and_then(|p| p.1), &ctx);
    shape.closed = !open;
    let size = {
        let resolved = begun.resolved(id);
        let body_id = sub(id, "body");
        let regions = regions_env(&props, &begun.env, resolved.enabled, shape.takes_room().0);
        let stage = MemoryStage::open_peek(&ctx, body_id);
        let mut frame =
            Frame { body_id, env: &regions, body_env: None, resolved: &resolved, stage, open, open_at_read: open, close: None, ran: false };
        let into = Host { node: peeked.map(|p| p.0), own_kept: false, marked: true, kept: None, sweep: ui.flow.sweep, ctx: ctx.clone() };
        let tokens = ui.flow.tokens.clone();
        let origin = ui.flow.next_min();
        let mut call = |ui: &mut Ui<'_, '_>, bcx: &mut BodyCx<'_>| {
            body(ui, bcx);
        };
        lay_out(component, ui.cx, &shape, into, &mut frame, (id, origin, room), &tokens, &mut call).0
    };
    *component.common_mut(Internal::new()) = begun.common;
    size
}

/// The open state a container saves under `ContainerProps::save`.
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
struct OpenState(Option<bool>);

/// The open state: the caller's binding, else the node's kept state, else the state saved under
/// `save`, else the collapse's initial state. A container with no `Collapse` is always open.
fn read_open<C: Container>(component: &mut C, props: &ContainerProps, kept: Option<bool>, ctx: &crate::contract::Context) -> bool {
    let Some(collapse) = props.collapse else { return true };
    if let Some(bound) = component.open().and_then(|b| b.get()).and_then(|m| Mixed::same(&m).copied()) {
        return bound;
    }
    let saved = || props.save.as_ref().and_then(|key| MemoryStage::peek_saved::<OpenState>(ctx, key).0);
    kept.or_else(saved).unwrap_or(collapse.initially_open)
}

/// Writes a changed open state to the caller's binding, else to the node and to the save key.
fn write_open<C: Container>(
    component: &mut C,
    kept: &mut Kept,
    open: bool,
    resolved: &crate::contract::Resolved<'_>,
    props: &ContainerProps,
    ctx: &crate::contract::Context,
) {
    match component.open() {
        Some(mut bind) if !bind.is_kept() => {
            let before = bind.get().unwrap_or(Mixed::Same(!open));
            let _ = bind.apply(&craft_behavior::EditEvent::set(before, open, Arc::from(resolved.label), None));
        }
        _ => {
            kept.body.open = Some(open);
            if let Some(key) = &props.save {
                crate::runtime::memory::set_saved_value(ctx, key, OpenState(Some(open)));
            }
        }
    }
}

/// Lays out the regions and the body of a container at `origin` in `room` and returns the
/// container's size and the body's flow (for what a scroll body above needs). A real run reconciles
/// the region nodes and places them in the container's kept placement; a measuring run reaches no
/// node.
#[allow(clippy::too_many_arguments)]
fn lay_out<C: Container>(
    component: &mut C,
    cx: &mut BuildCx<'_>,
    shape: &Shape,
    mut into: Host<'_>,
    frame: &mut Frame<'_>,
    (id, origin, room): (Id, Pos2, Vec2),
    tokens: &Arc<Tokens>,
    body: &mut dyn FnMut(&mut Ui<'_, '_>, &mut BodyCx<'_>),
) -> (Vec2, Flow) {
    let inner = Rect::from_min_size(origin + shape.pad, (room - 2.0 * shape.pad).max(Vec2::ZERO));
    let real = into.kept.is_some();
    if !shape.has_regions() {
        let at = BodyAt { node: into.node, own_kept: into.own_kept, marked: into.marked, sweep: into.sweep };
        let region = body_region(shape, inner);
        let (mut flow, ran) = run_body(cx, shape, at, into.kept.as_deref_mut(), frame, id, region, origin, None, tokens, body);
        frame.ran |= ran && real;
        if let Some(kept) = into.kept
            && kept.placed != flow.placed
        {
            kept.placed = std::mem::take(&mut flow.placed);
            cx.request_layout();
        }
        let size = shape.outer(flow.content_size(), room);
        return (size, flow);
    }
    let mut placed = Vec::new();
    let mut place = |node: Option<NodeId>, rect: Rect| {
        if let Some(node) = node {
            placed.push(Placed { node, offset: rect.min - origin, size: rect.size() });
        }
    };
    let divider = tokens.color.line.divider;
    let line = || Dir::Row { wrap: false, start: 0.0, spread: 0.0 };
    // The end column lies at the end edge: the right in a left-to-right env, the left in a right-to-left one.
    let end_w = match shape.end {
        Some(_) => region_size(cx, &into, Region::End, id, inner, tokens, &mut |ui| frame.with(ui, inner, |ui, bcx| component.end(ui, bcx))).x,
        None => 0.0,
    };
    let end_band = if shape.end.is_some() { end_w + shape.end_gap + shape.bands.end } else { 0.0 };
    let (main_min, main_max) = if shape.rtl { (inner.min.x + end_band, inner.max.x) } else { (inner.min.x, inner.max.x - end_band) };
    let mut top = inner.min.y;
    // The rhythm and rule bands above the header.
    if shape.bands.above > 0.0 {
        top += shape.bands.lead;
        let rule = Rect::from_min_size(Pos2::new(inner.min.x, top), Vec2::new(inner.width(), shape.bands.above));
        place(rule_node(cx, real, Region::RuleAbove, divider), rule);
        top += shape.bands.above + shape.bands.trail;
    }
    // The header line: the controls at its end, measured first so the header knows its width.
    let mut header_w = 0.0;
    if shape.header.is_some() {
        let line_rect = Rect::from_min_max(Pos2::new(main_min, top), Pos2::new(main_max, inner.max.y));
        let controls_w = region_size(cx, &into, Region::Controls, id, line_rect, tokens, &mut |ui| {
            frame.with(ui, line_rect, |ui, bcx| component.controls(ui, bcx))
        })
        .x;
        let (header_rect, controls_rect) = if shape.rtl {
            (
                Rect::from_min_max(Pos2::new(main_min + controls_w, top), line_rect.max),
                Rect::from_min_max(line_rect.min, Pos2::new(main_min + controls_w, inner.max.y)),
            )
        } else {
            (
                Rect::from_min_max(line_rect.min, Pos2::new(main_max - controls_w, inner.max.y)),
                Rect::from_min_max(Pos2::new(main_max - controls_w, top), line_rect.max),
            )
        };
        let (hn, header) = region(cx, &into, Region::Header, id, header_rect, line(), tokens, &mut |ui| {
            frame.with(ui, header_rect, |ui, bcx| component.header(ui, bcx))
        });
        let (cn, controls) = region(cx, &into, Region::Controls, id, controls_rect, line(), tokens, &mut |ui| {
            frame.with(ui, controls_rect, |ui, bcx| component.controls(ui, bcx))
        });
        place(hn, Rect::from_min_size(header_rect.min, header));
        place(cn, Rect::from_min_size(controls_rect.min, controls));
        top += header.y.max(controls.y);
        header_w = header.x + controls.x;
        if shape.bands.under > 0.0 {
            let rule = Rect::from_min_size(Pos2::new(inner.min.x, top), Vec2::new(inner.width(), shape.bands.under));
            place(rule_node(cx, real, Region::RuleUnder, divider), rule);
            top += shape.bands.under;
        }
        if !shape.closed {
            top += shape.header_gap;
        }
    }
    // The footer's height and the region under the body, known before the body that lies above them.
    // A footer at the foot or the top of the end region (`FooterPlace::EndBottom`, `EndTop`) is as wide as that region.
    let in_end = shape.end.is_some() && shape.footer.is_some_and(|f| matches!(f.place, FooterPlace::EndBottom | FooterPlace::EndTop));
    let footer_room = if in_end { Rect::from_min_size(inner.min, Vec2::new(end_w, inner.height())) } else { inner };
    let footer_h = match shape.footer {
        Some(_) => {
            region_size(cx, &into, Region::Footer, id, footer_room, tokens, &mut |ui| {
                frame.with(ui, footer_room, |ui, bcx| component.footer(ui, bcx))
            })
            .y
        }
        None => 0.0,
    };
    // A footer the body places (`FooterPlace::Body`) takes no band: the body's `FooterSlot` is as large as it measured.
    let in_body = shape.footer.is_some_and(|f| f.place == FooterPlace::Body);
    let footer_band = if shape.footer.is_some() && !in_body && !in_end { footer_h + shape.footer_gap + shape.bands.footer } else { 0.0 };
    let ctx = into.ctx.clone();
    if in_body {
        let size = Vec2::new(inner.width(), footer_h);
        frame.body_env = Some(frame.env.derive(|scoped| scoped.footer_slot = Some(crate::runtime::env::FooterSlotOf { owner: id, size })));
        crate::contract::footer_slot::clear(&ctx, id);
    }
    let main = Rect::from_min_max(Pos2::new(main_min, top), Pos2::new(main_max, inner.max.y));
    let body_end_h = if shape.closed {
        0.0
    } else {
        region_size(cx, &into, Region::BodyEnd, id, main, tokens, &mut |ui| frame.with(ui, main, |ui, bcx| component.body_end(ui, bcx))).y
    };
    let body_end_band = if body_end_h > 0.0 { body_end_h + shape.gap } else { 0.0 };
    let body_rect = Rect::from_min_max(main.min, Pos2::new(main.max.x, (inner.max.y - footer_band - body_end_band).max(top)));
    let (body_size, body_node, flow) = if shape.closed {
        (Vec2::ZERO, None, Flow::new(id, body_rect.min, body_rect, shape.gap, tokens.clone()))
    } else {
        match shape.scroll {
            Some(scroll) => scroll_body(cx, shape, &into, frame, id, body_rect, scroll, tokens, body),
            None => plain_body(cx, shape, &into, frame, id, body_rect, tokens, body),
        }
    };
    place(body_node, Rect::from_min_size(body_rect.min, body_size));
    let mut below = top + body_size.y;
    if body_end_h > 0.0 {
        let rect = Rect::from_min_max(Pos2::new(main.min.x, below + shape.gap), Pos2::new(main.max.x, below + shape.gap + body_end_h));
        let (bn, size) =
            region(cx, &into, Region::BodyEnd, id, rect, Dir::Stack, tokens, &mut |ui| frame.with(ui, rect, |ui, bcx| component.body_end(ui, bcx)));
        place(bn, Rect::from_min_size(rect.min, size));
        below = rect.max.y;
    }
    let fills_height = shape.sizing.fills_height() || shape.scroll.is_some_and(|_| shape.takes_room().1);
    // How far down the end column reaches, its footer included.
    let mut end_bottom = below;
    if shape.end.is_some() {
        let end_top = if shape.end.is_some_and(|e| e.full_height) || shape.header.is_none() { inner.min.y } else { main.min.y };
        let (end_min, end_max) = if shape.rtl { (inner.min.x, inner.min.x + end_w) } else { (inner.max.x - end_w, inner.max.x) };
        let top_footer = in_end && shape.footer.is_some_and(|f| f.place == FooterPlace::EndTop);
        let content_top = if top_footer { end_top + footer_h + shape.footer_gap } else { end_top };
        let end_rect = Rect::from_min_max(Pos2::new(end_min, content_top), Pos2::new(end_max, inner.max.y));
        let (en, end) =
            region(cx, &into, Region::End, id, end_rect, Dir::Stack, tokens, &mut |ui| frame.with(ui, end_rect, |ui, bcx| component.end(ui, bcx)));
        place(en, Rect::from_min_size(end_rect.min, end));
        end_bottom = end_bottom.max(content_top + end.y);
        if in_end {
            let footer_top = match top_footer {
                true => end_top,
                false if fills_height => inner.max.y - footer_h,
                // At the foot of the column: level with the body's end, or under the end region's content.
                false => (content_top + end.y + shape.footer_gap).max(below - footer_h),
            };
            let footer_rect = Rect::from_min_size(Pos2::new(end_min, footer_top), Vec2::new(end_w, footer_h));
            let (fnode, footer) = region(cx, &into, Region::Footer, id, footer_rect, Dir::Stack, tokens, &mut |ui| {
                frame.with(ui, footer_rect, |ui, bcx| component.footer(ui, bcx))
            });
            place(fnode, Rect::from_min_size(footer_rect.min, footer));
            end_bottom = end_bottom.max(footer_top + footer.y);
        }
        if shape.bands.end > 0.0 {
            let x = if shape.rtl { end_max + shape.end_gap / 2.0 } else { end_min - shape.end_gap / 2.0 - shape.bands.end };
            let rule = Rect::from_min_size(Pos2::new(x, end_top), Vec2::new(shape.bands.end, end_bottom - end_top));
            place(rule_node(cx, real, Region::RuleEnd, divider), rule);
        }
    }
    let footer_top = if fills_height { inner.max.y - footer_h } else { end_bottom + shape.footer_gap + shape.bands.footer };
    let mut footer_w = 0.0;
    // The body's real run placed its slot; a measuring run places none, and the footer takes no room of its own.
    let slot = if in_body && real { crate::contract::footer_slot::take(&ctx, id) } else { None };
    if let Some(slot) = slot {
        let (fnode, footer) =
            region(cx, &into, Region::Footer, id, slot, Dir::Stack, tokens, &mut |ui| frame.with(ui, slot, |ui, bcx| component.footer(ui, bcx)));
        place(fnode, Rect::from_min_size(slot.min, footer));
    } else if shape.footer.is_some() && !in_body && !in_end {
        if shape.bands.footer > 0.0 {
            let rule = Rect::from_min_size(Pos2::new(inner.min.x, footer_top - shape.bands.footer), Vec2::new(inner.width(), shape.bands.footer));
            place(rule_node(cx, real, Region::RuleFooter, divider), rule);
        }
        let footer_rect = Rect::from_min_max(Pos2::new(inner.min.x, footer_top), Pos2::new(inner.max.x, footer_top + footer_h));
        let footer_dir = if shape.footer.is_some_and(|f| f.place == FooterPlace::Body) { Dir::Stack } else { line() };
        let (fnode, footer) = region(cx, &into, Region::Footer, id, footer_rect, footer_dir, tokens, &mut |ui| {
            frame.with(ui, footer_rect, |ui, bcx| component.footer(ui, bcx))
        });
        place(fnode, Rect::from_min_size(footer_rect.min, footer));
        footer_w = footer.x;
    }
    let width = (body_size.x + end_band).max(header_w + end_band).max(footer_w);
    let height = (end_bottom - inner.min.y) + footer_band;
    if let Some(kept) = into.kept
        && kept.placed != placed
    {
        kept.placed = placed;
        cx.request_layout();
    }
    (shape.outer(Vec2::new(width, height), room), flow)
}

/// The body's rect in the container's inner rect: as tall as `room_height` when the props give one.
fn body_region(shape: &Shape, inner: Rect) -> Rect {
    match shape.room_height {
        Some(height) => Rect::from_min_size(inner.min, Vec2::new(inner.width(), height)),
        None => inner,
    }
}

/// A rule between regions as a node of the container; none in a measuring run.
fn rule_node(cx: &mut BuildCx<'_>, real: bool, key: Region, color: Rgba) -> Option<NodeId> {
    real.then(|| cx.keyed(key, |cx| cx.inline(RuleNode { color }, |_, _, cx| cx.node())))
}

/// The region node of `region` under the container node `node`, read without reaching it.
fn peek_region(cx: &BuildCx<'_>, node: NodeId, region: Region) -> Option<NodeId> {
    cx.peek::<ScopeNode>(node, region).map(|p| p.node)
}

/// A region's size before it is placed: its kept size when its node keeps its content, otherwise a
/// measuring run of its hook under its node.
fn region_size(
    cx: &mut BuildCx<'_>,
    into: &Host<'_>,
    region: Region,
    id: Id,
    room: Rect,
    tokens: &Arc<Tokens>,
    hook: &mut dyn FnMut(&mut Ui<'_, '_>),
) -> Vec2 {
    let peeked =
        into.node.and_then(|n| cx.peek::<ScopeNode>(n, region)).map(|p| (p.node, region_keeps(into, p.local, p.state, room.size()), p.local.size));
    if let Some((_, true, size)) = peeked {
        return size;
    }
    let under = peeked.map(|(node, _, _)| node);
    let flow = Flow::new(id.with(region), room.min, room, tokens.space.gap.row, tokens.clone())
        .with_dir(Dir::Row { wrap: false, start: 0.0, spread: 0.0 })
        .with_sweep(into.sweep);
    let mut ui = Ui::new(cx, flow.measuring(under));
    hook(&mut ui);
    ui.flow.content_size()
}

/// A region node keeps its content: its container's own hooks are kept, nothing under it is
/// marked and its room is the one it was laid out in.
fn region_keeps(into: &Host<'_>, kept: &Kept, state: InlineState, room: Vec2) -> bool {
    into.own_kept && !state.path && !state.marked && kept.place.room == room
}

/// Shows a region at its place: keeps it, or runs its hook into its node. A measuring run runs the
/// hook as a measuring run under the region's node.
#[allow(clippy::too_many_arguments)]
fn region(
    cx: &mut BuildCx<'_>,
    into: &Host<'_>,
    region: Region,
    id: Id,
    rect: Rect,
    dir: Dir,
    tokens: &Arc<Tokens>,
    hook: &mut dyn FnMut(&mut Ui<'_, '_>),
) -> (Option<NodeId>, Vec2) {
    let gap = tokens.space.gap.row;
    let flow = || Flow::new(id.with(region), rect.min, rect, gap, tokens.clone()).with_dir(dir.clone()).with_sweep(into.sweep);
    if into.kept.is_none() {
        let under = into.node.and_then(|n| peek_region(cx, n, region));
        let mut ui = Ui::new(cx, flow().measuring(under));
        hook(&mut ui);
        return (None, ui.flow.content_size());
    }
    cx.keyed(region, |cx| {
        cx.inline(ScopeNode { key: 0 }, |rk, state, cx| {
            let room = rect.size();
            let ctx = &into.ctx;
            if !state.new && region_keeps(into, rk, state, room) {
                cx.keep_children();
                crate::layout::replay(ctx, &rk.column_reports);
                return (Some(cx.node()), rk.size);
            }
            let mut ui = Ui::new(cx, flow());
            crate::layout::begin_capture(ctx);
            hook(&mut ui);
            rk.column_reports = crate::layout::end_capture(ctx);
            let Ui { cx, flow, .. } = ui;
            let size = flow.content_size();
            if rk.placed != flow.placed || rk.size != size {
                rk.placed = flow.placed;
                rk.size = size;
                cx.request_layout();
            }
            rk.place = Place { room };
            (Some(cx.node()), size)
        })
    })
}

/// A body beside regions, in a region node of its own.
#[allow(clippy::too_many_arguments)]
fn plain_body(
    cx: &mut BuildCx<'_>,
    shape: &Shape,
    into: &Host<'_>,
    frame: &mut Frame<'_>,
    id: Id,
    rect: Rect,
    tokens: &Arc<Tokens>,
    body: &mut dyn FnMut(&mut Ui<'_, '_>, &mut BodyCx<'_>),
) -> (Vec2, Option<NodeId>, Flow) {
    let region = body_region(shape, rect);
    if into.kept.is_none() {
        let peeked = into.node.and_then(|n| cx.peek::<ScopeNode>(n, Region::Body)).map(|p| p.node);
        let at = BodyAt { node: peeked, own_kept: false, marked: true, sweep: into.sweep };
        let (flow, _) = run_body(cx, shape, at, None, frame, id, region, rect.min, None, tokens, body);
        return (flow.content_size(), None, flow);
    }
    cx.keyed(Region::Body, |cx| {
        cx.inline(ScopeNode { key: 0 }, |rk, state, cx| {
            let node = cx.node();
            let at = BodyAt { node: Some(node), own_kept: into.own_kept, marked: state.path || state.marked, sweep: into.sweep };
            let (mut flow, ran) = run_body(cx, shape, at, Some(&mut *rk), frame, id, region, rect.min, None, tokens, body);
            frame.ran |= ran;
            let content = flow.content_size();
            if rk.placed != flow.placed || rk.size != content {
                rk.placed = std::mem::take(&mut flow.placed);
                rk.size = content;
                cx.request_layout();
            }
            (content, Some(node), flow)
        })
    })
}

/// Runs the body in `region` (children placed from `origin`), with the plan its layout needs:
/// reused, or made by a measuring run first. `kept` is the `Kept` of the node holding the body's
/// children (`None` in a measuring run); the plan is kept there. Returns the body's flow and whether
/// a real run called the body.
#[allow(clippy::too_many_arguments)]
fn run_body(
    cx: &mut BuildCx<'_>,
    shape: &Shape,
    at: BodyAt,
    kept: Option<&mut Kept>,
    frame: &mut Frame<'_>,
    id: Id,
    region: Rect,
    origin: Pos2,
    view: Option<Rect>,
    tokens: &Arc<Tokens>,
    body: &mut dyn FnMut(&mut Ui<'_, '_>, &mut BodyCx<'_>),
) -> (Flow, bool) {
    let columns = shape.columns().len();
    let room = region.size();
    let fresh = |dir: Dir, given: Vec<usize>| {
        let mut flow = Flow::new(id, origin, region, shape.gap, tokens.clone()).with_dir(dir).in_view(view).with_sweep(at.sweep);
        flow.body = shape.body_flow(given);
        flow
    };
    // A grid's cells share a label column per grid column; the rows of a `label_column` body share one.
    let mut call = |ui: &mut Ui<'_, '_>| {
        let mut run = |ui: &mut Ui<'_, '_>| frame.with_body(ui, region, |ui, bcx| body(ui, bcx));
        if columns > 0 {
            ColumnGroup::grid(ui, sub(id, "grid"), columns, run);
        } else if shape.label_column {
            ColumnGroup::scope(ui, "labels", |ui, _| run(ui));
        } else {
            run(ui);
        }
    };
    let Some(kept) = kept else {
        let mut ui = Ui::new(cx, fresh(shape.base_dir(columns), Vec::new()).measuring(at.node));
        call(&mut ui);
        return (ui.flow, false);
    };
    let had_fills = kept.body_fills.unwrap_or(true);
    let reusable = kept.plan.take().filter(|p| at.own_kept && !at.marked && p.room == room);
    let (plan, measured) = match reusable {
        Some(plan) => (Some(plan), false),
        None if shape.needs_plan(had_fills, kept.body.levelled) => {
            let mut ui = Ui::new(cx, fresh(shape.base_dir(columns), Vec::new()).measuring(at.node));
            call(&mut ui);
            let sizes = std::mem::take(&mut ui.flow.sizes);
            let rooms = std::mem::take(&mut ui.flow.rooms);
            let units = std::mem::take(&mut ui.flow.body.units);
            kept.body.levelled = !units.is_empty();
            (Some(shape.plan(room, sizes, rooms, &units, tokens)), true)
        }
        None => (None, false),
    };
    let flow = match &plan {
        Some(plan) => fresh(plan.dir.clone(), plan.levels.clone()).with_plan(plan.shares.clone(), plan.rooms.clone()),
        None => fresh(shape.base_dir(columns), Vec::new()),
    };
    let mut ui = Ui::new(cx, flow);
    call(&mut ui);
    let Ui { cx, flow, .. } = ui;
    let stale = match &plan {
        Some(plan) => !same_sizes(&plan.sizes, &flow.sizes),
        None => flow.fills > 0,
    };
    if stale && !measured {
        cx.rebuild();
    }
    kept.body_fills = Some(flow.fills > 0);
    kept.plan = plan.filter(|_| !stale || measured);
    (flow, true)
}

/// The non-fill children kept the sizes the plan measured (fill children take their share, so
/// their measured size is not compared).
fn same_sizes(plan: &[(Vec2, bool)], real: &[(Vec2, bool)]) -> bool {
    plan.len() == real.len() && plan.iter().zip(real).all(|((a, af), (b, bf))| af == bf && (*af || a == b))
}

/// The view of a scroll body whose content is `extent` in the body rect `view`: along an axis the
/// container takes its room, the whole rect; along a scrolled axis, as large as the content until it
/// outgrows the rect; along the other axes, the content's size.
fn view_of(shape: &Shape, scroll: Scroll, view: Rect, extent: Vec2) -> Vec2 {
    let (room_x, room_y) = shape.takes_room();
    let axis = |takes: bool, scrolls: bool, extent: f32, room: f32| {
        if takes {
            room
        } else if scrolls {
            extent.min(room)
        } else {
            extent
        }
    };
    Vec2::new(axis(room_x, scroll.horizontal_on(), extent.x, view.width()), axis(room_y, scroll.vertical_on(), extent.y, view.height()))
}

/// The scroll body: a `ScrollNode` whose children are laid out in content coordinates and moved by
/// its offset in layout. Content is built for the view plus overscan; a `Lazy` outside that stays
/// out of view, and the node notes the band it built. The view is as large as the content until
/// the content outgrows the body's rect (`view_of`).
#[allow(clippy::too_many_arguments)]
fn scroll_body(
    cx: &mut BuildCx<'_>,
    shape: &Shape,
    into: &Host<'_>,
    frame: &mut Frame<'_>,
    id: Id,
    view: Rect,
    scroll: Scroll,
    tokens: &Arc<Tokens>,
    body: &mut dyn FnMut(&mut Ui<'_, '_>, &mut BodyCx<'_>),
) -> (Vec2, Option<NodeId>, Flow) {
    let view = body_region(shape, view);
    let content = |offset: Vec2| {
        let size = Vec2::new(
            if scroll.horizontal_on() { f32::INFINITY } else { view.width() },
            if scroll.vertical_on() { f32::INFINITY } else { view.height() },
        );
        Rect::from_min_size(view.min - offset, size)
    };
    if into.kept.is_none() {
        let peeked = into.node.and_then(|n| cx.peek::<ScrollNode>(n, Region::Body)).map(|p| (p.node, p.local.offset));
        let (scroll_node, offset) = peeked.map_or((None, Vec2::ZERO), |(n, o)| (Some(n), o));
        let node = scroll_node.and_then(|n| peek_region(cx, n, Region::Body));
        let region = content(offset);
        let at = BodyAt { node, own_kept: false, marked: true, sweep: into.sweep };
        let (flow, _) = run_body(cx, shape, at, None, frame, id, region, region.min, Some(overscan(view)), tokens, body);
        return (view_of(shape, scroll, view, flow.content_size()), None, flow);
    }
    cx.keyed(Region::Body, |cx| {
        cx.inline(ScrollNode { scroll }, |sk: &mut ScrollKept, state, cx| {
            let node = cx.node();
            sk.subscribe(cx);
            let region = content(sk.offset);
            let marked = state.path || state.marked;
            let (mut flow, inner) = cx.keyed(Region::Body, |cx| {
                cx.inline(ScopeNode { key: 0 }, |ck, cstate, cx| {
                    let at =
                        BodyAt { node: Some(cx.node()), own_kept: into.own_kept, marked: marked || cstate.path || cstate.marked, sweep: into.sweep };
                    let (flow, ran) = run_body(cx, shape, at, Some(&mut *ck), frame, id, region, region.min, Some(overscan(view)), tokens, body);
                    frame.ran |= ran;
                    let extent = flow.content_size();
                    if ck.placed != flow.placed || ck.size != extent {
                        ck.placed = flow.placed.clone();
                        ck.size = extent;
                        cx.request_layout();
                    }
                    (flow, cx.node())
                })
            });
            let shown = view_of(shape, scroll, view, flow.content_size());
            sk.finish_build(cx, Rect::from_min_size(view.min, shown), inner, &mut flow, scroll, tokens);
            (shown, Some(node), flow)
        })
    })
}
