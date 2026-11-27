//! Containers on the `Ui` cursor (docs/contract-rt.md sections 2.4, 3.3 and 7): the regions of
//! `ContainerProps` (header with its controls at the end, end column, footer, body), the body's
//! `BodyLayout` with its plan, and the scroll body.
//!
//! A container's body closure runs whenever its scope's build reaches it. Its own hooks (the
//! region hooks, then update, paint, access) run only when its key moved, it is marked, or its size
//! moved; a region whose subtree is marked runs only its own hook. Each region is a node of its
//! own, so a kept region keeps its children while the body's are reached again.
//!
//! The plan is the body's measurement: where a `Row` block starts and how far its children spread,
//! a `Grid`'s column widths and row heights, and each fill child's share. It comes from a measuring
//! run of the body, in which kept children answer from their size cache and nothing runs, and it is
//! reused while the container's key, its body's room and its content are unchanged. When the real
//! run finds children of other sizes than a reused plan has, the container builds again in the
//! same frame and measures.

use std::sync::Arc;

use craft_behavior::{ColumnWidth, resolve_columns};
use craft_theme::Tokens;
use craft_ui_rt::{BuildCx, InlineState, NodeId, Pos2, Rect, Vec2};
use egui::Id;

use super::key::{component_key, fold, type_key};
use super::node::{ComponentNode, Kept, Place, Placed, ScopeNode};
use super::run::{Component, Cx, Ran, holds, run};
use super::scroll::{ScrollKept, ScrollNode, overscan};
use super::ui::{Dir, Flow, Ui};
use crate::contract::{ContainerProps, EndRegion, FooterRegion, HeaderRegion, PadEdges, Scroll, Sizing};
use crate::layout::{BodyLayout, RowAlign};

/// A component with a body and regions. Its body closure runs whenever its scope's build reaches
/// it; its own hooks run only when its own inputs changed.
pub trait Container: Component {
    /// The container's props: layout, padding, gaps, regions, scroll and sizing.
    fn props(&self) -> &ContainerProps;

    /// Shows the header region's content (`ContainerProps::header`).
    fn header(&mut self, _ui: &mut Ui<'_, '_>) {}

    /// Shows the controls at the header's end edge.
    fn controls(&mut self, _ui: &mut Ui<'_, '_>) {}

    /// Shows the end region's content (`ContainerProps::end`).
    fn end(&mut self, _ui: &mut Ui<'_, '_>) {}

    /// Shows the footer region's content (`ContainerProps::footer`).
    fn footer(&mut self, _ui: &mut Ui<'_, '_>) {}
}

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
}

impl Shape {
    fn of(props: &ContainerProps, tokens: &Tokens) -> Shape {
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
            scroll: props.scroll,
            least: Vec2::new(props.least_width.map_or(0.0, |l| l.points(tokens)), props.least_height.unwrap_or(0.0)),
        }
    }

    fn has_regions(&self) -> bool {
        self.header.is_some() || self.end.is_some() || self.footer.is_some() || self.scroll.is_some()
    }

    /// The container takes a share of what its parent's flow leaves along that flow's axis.
    fn fills(&self, parent: &Flow) -> bool {
        match self.sizing {
            Sizing::Fill => true,
            Sizing::FillHeight => parent.is_stack(),
            _ => false,
        }
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

    /// The body needs a plan: an aligned or spread `Row`, a `Grid`, or fill children last time.
    fn needs_plan(&self, had_fills: bool) -> bool {
        match &self.layout {
            BodyLayout::Row { wrap: false, align, .. } => *align != RowAlign::Start || had_fills,
            BodyLayout::Row { .. } => false,
            BodyLayout::Grid { .. } => true,
            _ => had_fills,
        }
    }

    /// The plan for a body of `room` whose measuring run found `sizes`.
    fn plan(&self, room: Vec2, sizes: Vec<(Vec2, bool)>, rooms: Vec<Vec2>, tokens: &Tokens) -> Plan {
        let fills = sizes.iter().filter(|(_, f)| *f).count();
        let gaps = self.gap * sizes.len().saturating_sub(1) as f32;
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
            BodyLayout::Grid { cells, .. } => {
                let columns: Vec<ColumnWidth> = self.columns().iter().map(|c| c.in_unit(tokens.size.button)).collect();
                let n = columns.len().max(1);
                let mut content = vec![0.0_f32; n];
                let mut rows = vec![0.0_f32; sizes.len().div_ceil(n)];
                for (i, (size, _)) in sizes.iter().enumerate() {
                    content[i % n] = content[i % n].max(size.x);
                    rows[i / n] = rows[i / n].max(size.y);
                }
                (Dir::Grid { widths: resolve_columns(&columns, &content, room.x, self.gap), heights: rows, cells: *cells }, Vec::new())
            }
            _ => {
                let used: f32 = sizes.iter().filter(|(_, f)| !*f).map(|(s, _)| s.y).sum::<f32>() + gaps;
                let shares = if fills > 0 { vec![((room.y - used) / fills as f32).max(0.0); fills] } else { Vec::new() };
                (Dir::Stack, shares)
            }
        };
        Plan { room, sizes, rooms, dir, shares }
    }

    /// The container's size from its inner content size, by its sizing, in `room`.
    fn outer(&self, inner: Vec2, room: Vec2) -> Vec2 {
        let content = (inner + 2.0 * self.pad).max(self.least);
        match self.sizing {
            Sizing::Fill => room,
            Sizing::FillWidth => Vec2::new(room.x, content.y),
            Sizing::FillHeight => Vec2::new(content.x, room.y),
            _ if self.scroll.is_some() => Vec2::new(content.x, room.y),
            _ => content,
        }
    }
}

impl Flow {
    pub(crate) fn is_stack(&self) -> bool {
        matches!(self.dir_ref(), Dir::Stack)
    }
}

/// The region nodes' keys under a container node.
#[derive(Clone, Copy, Debug, Hash)]
enum Region {
    Header,
    Controls,
    End,
    Body,
    Footer,
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
}

/// Where a body's children live and what decides whether its plan holds.
#[derive(Clone, Copy)]
struct BodyAt {
    node: Option<NodeId>,
    own_kept: bool,
    marked: bool,
}

/// Shows a container: its body always runs; its own hooks run when its key, size or a region's
/// content moved, or it is marked. In a measuring run it runs its regions and body as a measuring
/// run, under its kept node, and reports its size.
pub fn show_container<C: Container, R>(component: C, ui: &mut Ui<'_, '_>, mut body: impl FnMut(&mut Ui<'_, '_>) -> R) -> (R, C::Output) {
    let (id, child) = ui.flow.child(type_key::<C>(), component.id_source());
    let key = component_key(&component);
    let shape = Shape::of(component.props(), &ui.flow.tokens);
    let fills = shape.fills(&ui.flow);
    let room = if fills { ui.flow.fill_room() } else { ui.flow.room() };
    let origin = ui.flow.next_min();
    let mut component = component;
    if let Some(under) = ui.flow.measuring {
        let node = under.and_then(|u| ui.cx.peek::<ComponentNode>(u, child)).map(|p| p.node);
        let tokens = ui.flow.tokens.clone();
        let into = Host { node, own_kept: false, marked: true, kept: None };
        let (out, size, flow) = lay_out(&mut component, ui.cx, &shape, into, (id, origin, room), &tokens, &mut body);
        ui.flow.absorb(&flow);
        let rect = ui.flow.allocate(None, size, fills);
        return (out, component.finish(&Cx::settled(rect, &tokens)));
    }
    let Ui { cx, flow } = ui;
    cx.keyed(child, |cx| {
        cx.inline(ComponentNode { key: key.map_or(0, fold) }, |kept, state, cx| {
            let own_kept = holds(kept, key, state);
            let tokens = if own_kept { flow.tokens.clone() } else { cx.tokens() };
            let into = Host { node: Some(cx.node()), own_kept, marked: state.path || state.marked, kept: Some(&mut *kept) };
            let (out, size, inner) = lay_out(&mut component, cx, &shape, into, (id, origin, room), &tokens, &mut body);
            flow.absorb(&inner);
            let rect = flow.allocate(Some(cx.node()), size, fills);
            if own_kept && size == kept.size {
                kept.cache(room, size);
                return (out, component.finish(&Cx::settled(rect, &tokens)));
            }
            (out, run(component, kept, cx, Ran { id, key, place: Place { room }, size, rect, tokens: &tokens }))
        })
    })
}

/// Lays out the regions and the body of a container at `origin` in `room` and returns the body's
/// value, the container's size and the body's flow (for what a scroll body above needs). A real run
/// reconciles the region nodes and places them in the container's kept placement; a measuring run
/// reaches no node.
fn lay_out<C: Container, R>(
    component: &mut C,
    cx: &mut BuildCx<'_>,
    shape: &Shape,
    mut into: Host<'_>,
    (id, origin, room): (Id, Pos2, Vec2),
    tokens: &Arc<Tokens>,
    body: &mut impl FnMut(&mut Ui<'_, '_>) -> R,
) -> (R, Vec2, Flow) {
    let inner = Rect::from_min_size(origin + shape.pad, (room - 2.0 * shape.pad).max(Vec2::ZERO));
    if !shape.has_regions() {
        let at = BodyAt { node: into.node, own_kept: into.own_kept, marked: into.marked };
        let (out, mut flow) = run_body(cx, shape, at, into.kept.as_deref_mut(), id, inner, inner.min, None, tokens, body);
        if let Some(kept) = into.kept
            && kept.placed != flow.placed
        {
            kept.placed = std::mem::take(&mut flow.placed);
            cx.request_layout();
        }
        let size = shape.outer(flow.content_size(), room);
        return (out, size, flow);
    }
    let mut placed = Vec::new();
    let mut place = |node: Option<NodeId>, rect: Rect| {
        if let Some(node) = node {
            placed.push(Placed { node, offset: rect.min - origin, size: rect.size() });
        }
    };
    let line = || Dir::Row { wrap: false, start: 0.0, spread: 0.0 };
    // The header line: the controls at its end, measured first so the header knows its width.
    let mut top = inner.min.y;
    let mut header_w = 0.0;
    if shape.header.is_some() {
        let controls_w = region_size(cx, &into, Region::Controls, id, inner, tokens, &mut |ui| component.controls(ui)).x;
        let header_rect = Rect::from_min_max(inner.min, Pos2::new(inner.max.x - controls_w, inner.max.y));
        let (hn, header) = region(cx, &into, Region::Header, id, header_rect, line(), tokens, &mut |ui| component.header(ui));
        let controls_rect = Rect::from_min_max(Pos2::new(inner.max.x - controls_w, inner.min.y), inner.max);
        let (cn, controls) = region(cx, &into, Region::Controls, id, controls_rect, line(), tokens, &mut |ui| component.controls(ui));
        place(hn, Rect::from_min_size(header_rect.min, header));
        place(cn, Rect::from_min_size(controls_rect.min, controls));
        top += header.y.max(controls.y) + shape.header_gap;
        header_w = header.x + controls.x;
    }
    // The footer's height and the end column's width, known before the body that lies beside them.
    let footer_h = match shape.footer {
        Some(_) => region_size(cx, &into, Region::Footer, id, inner, tokens, &mut |ui| component.footer(ui)).y,
        None => 0.0,
    };
    let footer_band = if shape.footer.is_some() { footer_h + shape.footer_gap } else { 0.0 };
    let end_w = match shape.end {
        Some(_) => region_size(cx, &into, Region::End, id, inner, tokens, &mut |ui| component.end(ui)).x,
        None => 0.0,
    };
    let end_band = if shape.end.is_some() { end_w + shape.end_gap } else { 0.0 };
    let body_rect = Rect::from_min_max(Pos2::new(inner.min.x, top), Pos2::new(inner.max.x - end_band, (inner.max.y - footer_band).max(top)));
    let (out, body_size, body_node, flow) = match shape.scroll {
        Some(scroll) => scroll_body(cx, shape, &into, id, body_rect, scroll, tokens, body),
        None => plain_body(cx, shape, &into, id, body_rect, tokens, body),
    };
    place(body_node, Rect::from_min_size(body_rect.min, body_size));
    if shape.end.is_some() {
        let end_rect = Rect::from_min_max(Pos2::new(inner.max.x - end_w, top), Pos2::new(inner.max.x, inner.max.y));
        let (en, end) = region(cx, &into, Region::End, id, end_rect, Dir::Stack, tokens, &mut |ui| component.end(ui));
        place(en, Rect::from_min_size(end_rect.min, end));
    }
    let fills_height = shape.sizing.fills_height() || shape.scroll.is_some();
    let footer_top = if fills_height { inner.max.y - footer_h } else { top + body_size.y + shape.footer_gap };
    let mut footer_w = 0.0;
    if shape.footer.is_some() {
        let footer_rect = Rect::from_min_max(Pos2::new(inner.min.x, footer_top), Pos2::new(inner.max.x, footer_top + footer_h));
        let (fnode, footer) = region(cx, &into, Region::Footer, id, footer_rect, line(), tokens, &mut |ui| component.footer(ui));
        place(fnode, Rect::from_min_size(footer_rect.min, footer));
        footer_w = footer.x;
    }
    let width = (body_size.x + end_band).max(header_w).max(footer_w);
    let height = (top - inner.min.y) + body_size.y + footer_band;
    if let Some(kept) = into.kept
        && kept.placed != placed
    {
        kept.placed = placed;
        cx.request_layout();
    }
    (out, shape.outer(Vec2::new(width, height), room), flow)
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
    let flow =
        Flow::new(id.with(region), room.min, room, tokens.space.gap.row, tokens.clone()).with_dir(Dir::Row { wrap: false, start: 0.0, spread: 0.0 });
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
    let flow = || Flow::new(id.with(region), rect.min, rect, gap, tokens.clone()).with_dir(dir.clone());
    if into.kept.is_none() {
        let under = into.node.and_then(|n| peek_region(cx, n, region));
        let mut ui = Ui::new(cx, flow().measuring(under));
        hook(&mut ui);
        return (None, ui.flow.content_size());
    }
    cx.keyed(region, |cx| {
        cx.inline(ScopeNode { key: 0 }, |rk, state, cx| {
            let room = rect.size();
            if !state.new && region_keeps(into, rk, state, room) {
                cx.keep_children();
                return (Some(cx.node()), rk.size);
            }
            let mut ui = Ui::new(cx, flow());
            hook(&mut ui);
            let Ui { cx, flow } = ui;
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
fn plain_body<R>(
    cx: &mut BuildCx<'_>,
    shape: &Shape,
    into: &Host<'_>,
    id: Id,
    rect: Rect,
    tokens: &Arc<Tokens>,
    body: &mut impl FnMut(&mut Ui<'_, '_>) -> R,
) -> (R, Vec2, Option<NodeId>, Flow) {
    if into.kept.is_none() {
        let peeked = into.node.and_then(|n| cx.peek::<ScopeNode>(n, Region::Body)).map(|p| p.node);
        let at = BodyAt { node: peeked, own_kept: false, marked: true };
        let (out, flow) = run_body(cx, shape, at, None, id, rect, rect.min, None, tokens, body);
        return (out, flow.content_size(), None, flow);
    }
    cx.keyed(Region::Body, |cx| {
        cx.inline(ScopeNode { key: 0 }, |rk, state, cx| {
            let node = cx.node();
            let at = BodyAt { node: Some(node), own_kept: into.own_kept, marked: state.path || state.marked };
            let (out, mut flow) = run_body(cx, shape, at, Some(&mut *rk), id, rect, rect.min, None, tokens, body);
            let content = flow.content_size();
            if rk.placed != flow.placed || rk.size != content {
                rk.placed = std::mem::take(&mut flow.placed);
                rk.size = content;
                cx.request_layout();
            }
            (out, content, Some(node), flow)
        })
    })
}

/// Runs the body in `region` (children placed from `origin`), with the plan its layout needs:
/// reused, or made by a measuring run first. `kept` is the `Kept` of the node holding the body's
/// children (`None` in a measuring run); the plan is kept there. Returns the body's value and its
/// flow.
#[allow(clippy::too_many_arguments)]
fn run_body<R>(
    cx: &mut BuildCx<'_>,
    shape: &Shape,
    at: BodyAt,
    kept: Option<&mut Kept>,
    id: Id,
    region: Rect,
    origin: Pos2,
    view: Option<Rect>,
    tokens: &Arc<Tokens>,
    body: &mut impl FnMut(&mut Ui<'_, '_>) -> R,
) -> (R, Flow) {
    let columns = shape.columns().len();
    let room = region.size();
    let fresh = |dir: Dir| Flow::new(id, origin, region, shape.gap, tokens.clone()).with_dir(dir).in_view(view);
    let Some(kept) = kept else {
        let mut ui = Ui::new(cx, fresh(shape.base_dir(columns)).measuring(at.node));
        let out = body(&mut ui);
        return (out, ui.flow);
    };
    let had_fills = kept.body_fills.unwrap_or(true);
    let reusable = kept.plan.take().filter(|p| at.own_kept && !at.marked && p.room == room);
    let (plan, measured) = match reusable {
        Some(plan) => (Some(plan), false),
        None if shape.needs_plan(had_fills) => {
            let mut ui = Ui::new(cx, fresh(shape.base_dir(columns)).measuring(at.node));
            body(&mut ui);
            let sizes = std::mem::take(&mut ui.flow.sizes);
            let rooms = std::mem::take(&mut ui.flow.rooms);
            (Some(shape.plan(room, sizes, rooms, tokens)), true)
        }
        None => (None, false),
    };
    let flow = match &plan {
        Some(plan) => fresh(plan.dir.clone()).with_plan(plan.shares.clone(), plan.rooms.clone()),
        None => fresh(shape.base_dir(columns)),
    };
    let mut ui = Ui::new(cx, flow);
    let out = body(&mut ui);
    let Ui { cx, flow } = ui;
    let stale = match &plan {
        Some(plan) => !same_sizes(&plan.sizes, &flow.sizes),
        None => flow.fills > 0,
    };
    if stale && !measured {
        cx.rebuild();
    }
    kept.body_fills = Some(flow.fills > 0);
    kept.plan = plan.filter(|_| !stale || measured);
    (out, flow)
}

/// The non-fill children kept the sizes the plan measured (fill children take their share, so
/// their measured size is not compared).
fn same_sizes(plan: &[(Vec2, bool)], real: &[(Vec2, bool)]) -> bool {
    plan.len() == real.len() && plan.iter().zip(real).all(|((a, af), (b, bf))| af == bf && (*af || a == b))
}

/// The scroll body: a `ScrollNode` whose children are laid out in content coordinates and moved by
/// its offset in layout. Content is built for the view plus overscan; a `Lazy` outside that stays
/// out of view, and the node notes the band it built.
#[allow(clippy::too_many_arguments)]
fn scroll_body<R>(
    cx: &mut BuildCx<'_>,
    shape: &Shape,
    into: &Host<'_>,
    id: Id,
    view: Rect,
    scroll: Scroll,
    tokens: &Arc<Tokens>,
    body: &mut impl FnMut(&mut Ui<'_, '_>) -> R,
) -> (R, Vec2, Option<NodeId>, Flow) {
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
        let at = BodyAt { node, own_kept: false, marked: true };
        let (out, flow) = run_body(cx, shape, at, None, id, region, region.min, Some(overscan(view)), tokens, body);
        return (out, view.size(), None, flow);
    }
    cx.keyed(Region::Body, |cx| {
        cx.inline(ScrollNode { scroll }, |sk: &mut ScrollKept, state, cx| {
            let node = cx.node();
            sk.subscribe(cx);
            let region = content(sk.offset);
            let marked = state.path || state.marked;
            let (out, mut flow, inner) = cx.keyed(Region::Body, |cx| {
                cx.inline(ScopeNode { key: 0 }, |ck, cstate, cx| {
                    let at = BodyAt { node: Some(cx.node()), own_kept: into.own_kept, marked: marked || cstate.path || cstate.marked };
                    let (out, flow) = run_body(cx, shape, at, Some(&mut *ck), id, region, region.min, Some(overscan(view)), tokens, body);
                    let extent = flow.content_size();
                    if ck.placed != flow.placed || ck.size != extent {
                        ck.placed = flow.placed.clone();
                        ck.size = extent;
                        cx.request_layout();
                    }
                    (out, flow, cx.node())
                })
            });
            sk.finish_build(cx, view, inner, &mut flow, scroll, tokens);
            (out, view.size(), Some(node), flow)
        })
    })
}
