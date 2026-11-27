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

use craft_behavior::{ColumnWidth, Insets, Mixed, resolve_columns};
use craft_theme::Tokens;
use craft_ui_rt::{BuildCx, Constraints, Element, InlineState, LayoutCx, NodeId, PaintCx, Pos2, Rect, Rgba, State, Vec2};

use super::container_shape::{ContainerShape, HeaderGeometry, WindowMode, window_mode_id};
use super::key::{component_key, fold, type_key};
use super::node::{ComponentNode, Kept, NodeInput, Place, Placed, ScopeNode};
use super::overlay::{BodyClose, LayerBody, LayerHost, LayerShow, Surface, show_layer_with};
use super::run::{Ran, finish_measured, holds, reach_layers, run_shaped, settle, sweep_tag};
use super::scope::{ScopeShow, show_scope};
use super::scroll::{ScrollKept, ScrollNode, overscan};
use super::sweep::Sweep;
use super::ui::{Dir, Flow, Ui};
use crate::contract::cx::{BodyCx, BodyOwner, RegionShare};
use crate::contract::label::{LabelIn, SideLabel};
use crate::contract::run::ShowAt;
use crate::contract::{
    Container, ContainerProps, EndRegion, FooterPlace, FooterRegion, HeaderRegion, Id, IdSource, Inner, Internal, Mount, Outcome, Overlay, PadEdges,
    QuietHeader, Response, Scroll, SectionRhythm, Sizing, TipFn, ValueOf, steps,
};
use crate::layout::{BodyLayout, ColumnGroup, GridFit, RowAlign};
use crate::runtime::access::scope::FieldMessage;
use crate::runtime::env::{Env, EnvScope, InBar, PaneEdge, Shrink, ThemeScope};
use crate::runtime::memory::{MemoryStage, sub};
use crate::runtime::motion::{Driver, spec};
use crate::style::ComponentStyle;

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
    /// The width a content column settled at when a held child measured narrower than it (`HeldKind`): the real run
    /// lays the children out at it, so the held child shows across the column.
    settled: Option<f32>,
    /// The plan narrows the units of a `Row` too long for its room (`layout::row_shrink`): `rooms` holds the width
    /// each shows at most, and the real run shows them under `Shrink::Apply`.
    shrinks: bool,
}

/// What a container node keeps besides its component's outputs: the open state of a collapsing body.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BodyKept {
    /// The open state when no binding owns it (`Container::open` is `None`); `None` before the first build.
    pub(crate) open: Option<bool>,
    /// The body's last measuring run met levelled units: its real runs follow a plan.
    pub(crate) levelled: bool,
    /// The body is a `Row` that overflowed its room in its last run: its real runs follow a plan that narrows it.
    pub(crate) shrinks: bool,
}

/// What one sweep of a body offers its children beyond its flow.
#[derive(Clone, Copy, PartialEq, Eq)]
enum BodySweep {
    Plain,
    /// The first measuring sweep of a content-sized column, which offers the children no room.
    Column,
    /// The least sweep of a `Row` too long for its room: every leaf takes its least width (`Shrink::Least`).
    Least,
    /// The sweep of a `Row` whose plan narrows its units: each shows at most the room the plan gave it.
    Apply,
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
    /// `start` is where the container stands in its body's flow: `Some(after_rule)` at the flow's start, with whether a
    /// rule precedes that start (`Scoped::after_rule`), `None` past it. A rule above separates the container from what
    /// precedes it: right after a rule there is none, and at its flow's start no rhythm band stands it off.
    /// `gap` is the gap of the flow the container stands in, which the rhythm band before its rule replaces.
    fn of(props: &ContainerProps, env: &Env, tokens: &Tokens, start: Option<bool>, gap: f32) -> Bands {
        let width = tokens.stroke.thin;
        let on = |rule: bool| if rule { width } else { 0.0 };
        let above = on(start != Some(true) && props.header.is_some_and(|h| h.rule_above));
        let spaced = above > 0.0 && env.get::<SectionRhythm>() == SectionRhythm::Spaced;
        Bands {
            lead: if spaced && start.is_none() { (tokens.space.gap.section - gap).max(0.0) } else { 0.0 },
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
    /// The padding between the container's edge and its regions at the top left and at the bottom right: the style's
    /// padding (`ComponentStyle::metrics`, mirrored right to left) plus the props' `pad` on the edges it applies to.
    pad_min: Vec2,
    pad_max: Vec2,
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
    /// The share of the body's height the container shows while its collapse moves (1 open, toward 0 closing).
    share: f32,
    /// The header heads the body as a rule would (`HeaderRegion::heads` under `QuietHeader::Heads`): the body's flow
    /// starts as after a rule.
    heads: bool,
    /// The labelled rows of the body share the widest label as one column (`ContainerProps::label_column`).
    label_column: bool,
    /// The body is a content column: a content-sized `Stack` whose width comes from its content, not from the flow it
    /// stands in (a row's or a grid's unit, or a column inside another content column). A content-sized container in
    /// a `Stack` given its width passes that width on and is none.
    content_column: bool,
    /// The edge of an edge pane that faces the window's centre, where `ContainerProps::edge_rule` draws its divider.
    edge_rule: Option<PaneEdge>,
    /// The colour of the band behind the header (`HeaderRegion::fill`).
    band: Option<Rgba>,
    /// The container is a row of a vertical flow, not a member of a row's line (`label::in_column`): its field label
    /// lines up with the form's control column.
    in_column: bool,
    /// The container has a field label (`ContainerProps::label`): the label step places its body.
    labelled: bool,
}

impl Shape {
    /// `parent` is the flow the container is shown in; `padding` is the style's.
    fn of(props: &ContainerProps, env: &Env, tokens: &Tokens, parent: &Flow, padding: Insets) -> Shape {
        let at_start = parent.sizes.is_empty();
        let pad = props.pad.points(tokens);
        let pad = match props.pad_edges {
            PadEdges::Block => Vec2::new(0.0, pad),
            PadEdges::Inline => Vec2::new(pad, 0.0),
            _ => Vec2::splat(pad),
        };
        let rtl = env.direction.is_rtl();
        let (left, right) = if rtl { (padding.end, padding.start) } else { (padding.start, padding.end) };
        let gap = props.body_gap.points_in(tokens);
        Shape {
            layout: props.layout.clone(),
            pad_min: pad + Vec2::new(left, padding.top),
            pad_max: pad + Vec2::new(right, padding.bottom),
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
            bands: Bands::of(props, env, tokens, at_start.then_some(env.after_rule), parent.gap()),
            rtl: env.direction.is_rtl(),
            closed: false,
            share: 1.0,
            heads: props.header.is_some_and(|h| h.heads && !h.rule) && env.get::<QuietHeader>() == QuietHeader::Heads,
            label_column: props.label_column,
            content_column: matches!(props.layout, BodyLayout::Stack)
                && matches!(props.sizing, Sizing::Content | Sizing::FillHeight)
                && (!parent.is_stack() || env.content_column),
            edge_rule: match props.mount {
                Mount::Pane(pane) if props.edge_rule => match pane.edge {
                    PaneEdge::Start => Some(PaneEdge::End),
                    PaneEdge::End => Some(PaneEdge::Start),
                    PaneEdge::Top => Some(PaneEdge::Bottom),
                    PaneEdge::Bottom => Some(PaneEdge::Top),
                    PaneEdge::Center => None,
                },
                _ => None,
            },
            band: props.header.and_then(|h| h.fill).map(|fill| crate::primitives::items::rgba(crate::style::band_color(&env.tokens, fill))),
            in_column: !matches!(parent.dir_ref(), Dir::Row { .. }) || env.column_row,
            labelled: props.label.is_some(),
        }
    }

    /// A content-sized `Stack` container that is the content of its own popup layer (`Mount::Layer`, `hosted`) is a
    /// content column: the layer takes its width from the container's content, as a row's unit does.
    fn in_popup(mut self, props: &ContainerProps, hosted: bool) -> Shape {
        if hosted && matches!(props.mount, Mount::Layer(_)) {
            self.content_column |= matches!(props.layout, BodyLayout::Stack) && matches!(self.sizing, Sizing::Content | Sizing::FillHeight);
        }
        self
    }

    /// An edge pane fills the strip of its flow its mount gives it (`show_pane`); only the measuring run before its
    /// first show (`measure_container`) finds its content's extent.
    fn in_pane(mut self, props: &ContainerProps) -> Shape {
        if matches!(props.mount, Mount::Pane(pane) if pane.at.is_none() && pane.edge != PaneEdge::Center) {
            self.sizing = Sizing::Fill;
        }
        self
    }

    /// A container that fits its content in a bar (`ContainerProps::content_in_bar`) takes its content width among a
    /// bar's members, whatever its sizing says.
    fn in_bar(mut self, props: &ContainerProps, env: &Env) -> Shape {
        if props.content_in_bar && env.get::<InBar>().0 {
            self.sizing = match self.sizing {
                Sizing::Fill | Sizing::FillHeight => Sizing::FillHeight,
                Sizing::FillWidth => Sizing::default(),
                other => other,
            };
        }
        self
    }

    /// The collapse is moving: the body shows `share` of its height (1 when settled open).
    fn collapsing(&self) -> bool {
        self.share < 1.0 && !self.closed
    }

    fn has_regions(&self) -> bool {
        self.header.is_some()
            || self.end.is_some()
            || self.footer.is_some()
            || self.scroll.is_some()
            || self.bands.top() > 0.0
            || self.closed
            || self.collapsing()
            || self.edge_rule.is_some()
            || self.labelled
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
    /// The body is a `Row` that narrows its units to fit its room: an unwrapped row whose body does not scroll.
    fn shrinks(&self) -> bool {
        matches!(self.layout, BodyLayout::Row { wrap: false, .. }) && self.scroll.is_none()
    }

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
        Plan { room, sizes, rooms, dir, shares, levels, settled: None, shrinks: false }
    }

    /// The container's size from its inner content size, by its sizing, in `room`. A scroll body's inner size is
    /// already its view: as large as its content until the content outgrows the room.
    fn outer(&self, inner: Vec2, room: Vec2) -> Vec2 {
        let content = (inner + self.pad_min + self.pad_max).max(self.least);
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
    /// The band behind the header (`HeaderRegion::fill`).
    HeaderBand,
    Controls,
    End,
    Body,
    BodyEnd,
    Footer,
    RuleAbove,
    RuleUnder,
    RuleFooter,
    RuleEnd,
    RuleEdge,
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
    /// The container's style and body gap, which every region's `BodyCx` reads (`BodyCx::style`, `gap`).
    owner: BodyOwner<'f>,
    /// The gate's shown copy for a container whose value gates its content (`ContainerProps::gates`), as read at the
    /// start of the build; the header's `BodyCx::gate` binds to it.
    gate: Option<bool>,
    gate_at_read: Option<bool>,
    gate_mixed: bool,
    /// The Escape the scope around the body left untaken in this run (`BodyCx::take_cancel`), for the body's first
    /// real run.
    cancel: bool,
    /// A rule precedes the body's start (the header's rule, or a quiet header that heads the body): the body's first
    /// child draws no rule above it (`Scoped::after_rule`).
    after_rule: bool,
    /// A drag of the container's move band runs (`BodyCx::moving`), as the container's last run left it.
    moving: bool,
    /// Where the header lay in this build, for the move and reorder bands its run declares.
    geometry: HeaderGeometry,
    /// The message cell of the container's label scope (`scope::note_message`), which the footer reads.
    message: Option<State<Option<FieldMessage>>>,
}

/// Which region a `BodyCx` is made for: the header gets the gate, the body the gate's env and the scope's cancel.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Hook {
    Header,
    Region,
    /// The footer: its `BodyCx` holds the field message (`BodyCx::field_message`).
    Footer,
    Body,
}

impl<'f> Frame<'f> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        body_id: Id,
        env: &'f Env,
        resolved: &'f crate::contract::Resolved<'f>,
        stage: MemoryStage,
        open: bool,
        owner: BodyOwner<'f>,
        gate: (Option<bool>, bool),
        cancel: bool,
    ) -> Frame<'f> {
        Frame {
            body_id,
            env,
            body_env: None,
            resolved,
            stage,
            open,
            open_at_read: open,
            close: None,
            ran: false,
            owner,
            gate: gate.0,
            gate_at_read: gate.0,
            gate_mixed: gate.1,
            cancel,
            after_rule: false,
            moving: false,
            geometry: HeaderGeometry::default(),
            message: None,
        }
    }

    /// Runs a region hook at `rect` under the regions' env.
    fn with<T>(&mut self, ui: &mut Ui<'_, '_>, rect: Rect, f: impl FnOnce(&mut Ui<'_, '_>, &mut BodyCx<'_>) -> T) -> T {
        let env = self.env.clone();
        self.run(ui, env, rect, Hook::Region, f)
    }

    /// Runs the footer hook at `rect`: its `BodyCx` holds the field message, read tracked by the footer's region node.
    fn with_footer<T>(&mut self, ui: &mut Ui<'_, '_>, rect: Rect, f: impl FnOnce(&mut Ui<'_, '_>, &mut BodyCx<'_>) -> T) -> T {
        let env = self.env.clone();
        self.run(ui, env, rect, Hook::Footer, f)
    }

    /// Runs the header hook at `rect`: its `BodyCx` holds the gate.
    fn with_header<T>(&mut self, ui: &mut Ui<'_, '_>, rect: Rect, f: impl FnOnce(&mut Ui<'_, '_>, &mut BodyCx<'_>) -> T) -> T {
        let env = self.env.clone();
        self.run(ui, env, rect, Hook::Header, f)
    }

    /// Runs the body at `rect` under the body's env: disabled while the gate is off, with the room facts of `sweep`.
    fn with_body<T>(&mut self, ui: &mut Ui<'_, '_>, rect: Rect, sweep: BodySweep, f: impl FnOnce(&mut Ui<'_, '_>, &mut BodyCx<'_>) -> T) -> T {
        let mut env = self.body_env.clone().unwrap_or_else(|| self.env.clone());
        let (gated, after_rule, row) = (self.gate == Some(false), self.after_rule, self.body_id);
        if gated || after_rule || sweep != BodySweep::Plain {
            env = env.derive(|scoped| {
                scoped.enabled &= !gated;
                scoped.after_rule = after_rule;
                match sweep {
                    BodySweep::Plain => {}
                    BodySweep::Column => scoped.content_column = true,
                    BodySweep::Least => {
                        scoped.shrink = Some(Shrink::Least(row));
                        scoped.least = true;
                    }
                    BodySweep::Apply => scoped.shrink = Some(Shrink::Apply(row)),
                }
            });
        }
        self.run(ui, env, rect, Hook::Body, f)
    }

    fn run<T>(&mut self, ui: &mut Ui<'_, '_>, env: Env, rect: Rect, hook: Hook, f: impl FnOnce(&mut Ui<'_, '_>, &mut BodyCx<'_>) -> T) -> T {
        let header = hook == Hook::Header;
        // A measuring run of the footer peeks the message: only the footer's region node reads it, so a change reruns
        // the footer alone.
        let field_message = match (hook, self.message) {
            (Hook::Footer, Some(cell)) if ui.flow.measuring.is_some() => ui.build_cx().peek_state(cell).clone(),
            (Hook::Footer, Some(cell)) => cell.get(ui.build_cx()),
            _ => None,
        };
        let field_message = field_message.map(|held| (held.message, held.tone));
        let shared = RegionShare { gate_mixed: header && self.gate_mixed, field_message, owner: self.owner };
        let gate = if header { self.gate.as_mut() } else { None };
        // A measuring run leaves the cancel for the real run that follows it.
        let cancel = hook == Hook::Body && ui.flow.measuring.is_none() && std::mem::take(&mut self.cancel);
        let mut bcx = BodyCx::new(self.body_id, self.env.view(), self.resolved, rect, &mut self.stage, &mut self.open, gate, Some(shared))
            .with_cancel(cancel)
            .with_moving(self.moving);
        let out = Env::run_in(ui, env, |ui| f(ui, &mut bcx));
        if self.close.is_none() {
            self.close = bcx.close_requested();
        }
        out
    }
}

/// The gate a container's value holds (`ContainerProps::gates`): `(on, mixed)`; a mixed value reads off. `None` for a
/// container that does not gate or whose value is not a `bool`.
fn gate_of<V: 'static>(gates: bool, value: &steps::ValueStep<V>) -> (Option<bool>, bool) {
    if !gates {
        return (None, false);
    }
    let on = value.value.as_deref().and_then(|v| (v as &dyn std::any::Any).downcast_ref::<bool>()).copied();
    match on {
        Some(on) => (Some(on && !value.mixed), value.mixed),
        None => (Some(false), value.mixed),
    }
}

/// Writes a gate the header changed to the container's binding as one set edit, as a toggle's apply step writes its
/// value.
fn write_gate<C: Container>(component: &mut C, on: bool, resolved: &crate::contract::Resolved<'_>) {
    fn as_value<V: 'static>(on: bool) -> Option<V> {
        (Box::new(on) as Box<dyn std::any::Any>).downcast::<V>().ok().map(|v| *v)
    }
    let (Some(after), Some(before)) = (as_value::<ValueOf<C>>(on), as_value::<ValueOf<C>>(!on)) else { return };
    let Some(mut bind) = component.bind() else { return };
    let before = bind.get().unwrap_or(craft_behavior::Mixed::Same(before));
    let _ = bind.apply(&craft_behavior::EditEvent::set(before, after, Arc::from(resolved.label), None));
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
        scoped.after_rule = false;
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
///
/// A container mounted on a layer (`Mount::Layer`, `FullWindow`) is shown as the content of a layer child of the node
/// that shows it, unless it is that content already (the layer provides its `LayerHost` and the container is shown in
/// the layer body's flow): then it fills the layer's region and its close goes to the layer. A `Viewport` mount is the
/// content of a window the showing node hosts (`show_in_window`). A container with a theme (`ContainerProps::theme`)
/// is shown inside a scope that gives it and everything under it that theme.
pub(crate) fn show_container<C: Container, R>(
    slot: &mut Option<C>,
    ui: &mut Ui<'_, '_>,
    (at, place): (ShowAt, Env),
    body: &mut dyn FnMut(&mut Ui<'_, '_>, &mut BodyCx<'_>) -> R,
    tip: Option<TipFn<'_>>,
) -> Inner<C::Output, R> {
    let props = slot.as_ref().expect("a container is shown once").props(Internal::new());
    // A full-window mount is a layer over the whole window with the props' focus scope.
    let layer = match &props.mount {
        Mount::Layer(overlay) => Some(overlay.clone()),
        Mount::FullWindow => Some(Overlay::full_window(props.focus)),
        _ => None,
    };
    let (layer_like, theme) = (matches!(props.mount, Mount::FullWindow | Mount::Viewport { .. }), props.theme.clone());
    let mount = props.mount.clone();
    let hosted = hosted_by(ui);
    if layer_like && let Some(output) = unshown(slot, ui) {
        return Inner { output, inner: None, settle: None };
    }
    if let Mount::Viewport { fallback, spec } = &mount
        && hosted.is_none()
    {
        if ui.flow.measuring.is_none() {
            return show_in_window(slot, ui, fallback, spec.as_ref(), body);
        }
        // Like a layer mount, it takes no room in the flow that shows it.
        let rect = Rect::from_min_size(ui.flow.next_min(), ui.flow.room());
        return show_plain(slot, ui, (ShowAt::Rect(rect), place), body, tip, None);
    }
    if let Some(overlay) = layer
        && hosted.is_none()
    {
        if ui.flow.measuring.is_none() {
            return show_on_layer(slot, ui, &overlay, body);
        }
        // A layer mount takes no room in the flow that shows it: its measuring run places it at a rect of its own.
        let rect = Rect::from_min_size(ui.flow.next_min(), ui.flow.room());
        return show_plain(slot, ui, (ShowAt::Rect(rect), place), body, tip, None);
    }
    if let Mount::Pane(pane) = mount
        && pane.at.is_none()
        && pane.edge != PaneEdge::Center
        && hosted.is_none()
    {
        return show_pane(slot, ui, pane, place, body, (tip, theme));
    }
    show_themed(slot, ui, (at, place), body, tip, hosted, theme)
}

/// Shows an edge pane (`Mount::Pane` at an edge, not laid out by its caller) in a strip of the flow's region at its
/// edge, as deep as `container_shape::pane_shown` says, and leaves the flow the rest of the region: the panes and the
/// centre shown after it lay out there. Before its first show the pane is as deep as its content, which its session
/// memory keeps from then on, until its grip gives it another extent.
fn show_pane<C: Container, R>(
    slot: &mut Option<C>,
    ui: &mut Ui<'_, '_>,
    pane: crate::contract::ShellPane,
    place: Env,
    body: &mut dyn FnMut(&mut Ui<'_, '_>, &mut BodyCx<'_>) -> R,
    (tip, theme): (Option<TipFn<'_>>, Option<ThemeScope>),
) -> Inner<C::Output, R> {
    let component = slot.as_mut().expect("a container is shown once");
    let source = component.common_mut(Internal::new()).id.clone();
    let (id, _) = ui.flow.peek_child(type_key::<C>(), source);
    let ctx = ui.ctx().clone();
    let region = ui.flow.region;
    let along_x = matches!(pane.edge, PaneEdge::Start | PaneEdge::End);
    let extent = match super::container_shape::pane_shown(&ctx, id, &pane) {
        Some(extent) => extent,
        None => {
            let size = measure_container(component, ui, craft_behavior::Proposal::Ideal, &mut *body);
            let extent = if along_x { size.x } else { size.y };
            if ui.flow.measuring.is_none() {
                super::container_shape::note_pane_extent(&ctx, id, extent);
            }
            extent
        }
    };
    let extent = extent.min(if along_x { region.width() } else { region.height() }).max(0.0);
    // The start edge is the left in a left-to-right env, the right in a right-to-left one.
    let rtl = place.direction.is_rtl();
    let at_left = matches!((pane.edge, rtl), (PaneEdge::Start, false) | (PaneEdge::End, true));
    let (strip, rest) = match pane.edge {
        PaneEdge::Top => (
            Rect::from_min_size(region.min, Vec2::new(region.width(), extent)),
            Rect::from_min_max(Pos2::new(region.min.x, region.min.y + extent), region.max),
        ),
        PaneEdge::Bottom => (
            Rect::from_min_max(Pos2::new(region.min.x, region.max.y - extent), region.max),
            Rect::from_min_max(region.min, Pos2::new(region.max.x, region.max.y - extent)),
        ),
        _ if at_left => (
            Rect::from_min_size(region.min, Vec2::new(extent, region.height())),
            Rect::from_min_max(Pos2::new(region.min.x + extent, region.min.y), region.max),
        ),
        _ => (
            Rect::from_min_max(Pos2::new(region.max.x - extent, region.min.y), region.max),
            Rect::from_min_max(region.min, Pos2::new(region.max.x - extent, region.max.y)),
        ),
    };
    let shown = show_themed(slot, ui, (ShowAt::Rect(strip), place), body, tip, None, theme);
    ui.flow.cut_region(rest);
    shown
}

/// `show_plain` inside a scope that gives the container and everything under it `theme`, when it has one.
fn show_themed<C: Container, R>(
    slot: &mut Option<C>,
    ui: &mut Ui<'_, '_>,
    (at, place): (ShowAt, Env),
    body: &mut dyn FnMut(&mut Ui<'_, '_>, &mut BodyCx<'_>) -> R,
    tip: Option<TipFn<'_>>,
    hosted: Option<LayerHost>,
    theme: Option<ThemeScope>,
) -> Inner<C::Output, R> {
    let Some(theme) = theme else { return show_plain(slot, ui, (at, place), body, tip, hosted) };
    // The scope always runs its content: the container's body runs whenever its scope's build reaches it.
    let scope = EnvScope::new().theme(theme);
    let show = ScopeShow { id: IdSource::Auto, type_key: type_key::<EnvScope>(), memo: None, estimate: 0.0, reveal: false, lazy: false };
    let (mut out, mut tip) = (None, tip);
    let _ = show_scope(ui, show, None, &mut |ui| {
        scope.enter(ui);
        let themed = Env::of(ui);
        let place = place.derive(|scoped| scoped.inherited_mut().tokens = themed.tokens.clone());
        out = Some(show_plain(&mut *slot, ui, (at, place.clone()), &mut *body, tip.take(), hosted.clone()));
    });
    out.expect("a theme scope with no out-of-view rule always runs its content")
}

/// The output of a layer-like container that is closed (its open binding or collapse says so), when its output has an
/// unshown form: it then shows nothing and runs no hook.
fn unshown<C: Container>(slot: &mut Option<C>, ui: &mut Ui<'_, '_>) -> Option<C::Output> {
    let component = slot.as_mut().expect("a container is shown once");
    let props = component.props(Internal::new()).clone();
    let ctx = ui.ctx().clone();
    if read_open(component, &props, None, &ctx) {
        return None;
    }
    let (id, _) = ui.flow.peek_child(type_key::<C>(), component.common_mut(Internal::new()).id.clone());
    C::Output::unshown(Response::inert(id, Rect::NOTHING))
}

/// The layer whose content a container shown now in `ui` is: the provided `LayerHost` of the layer
/// whose body flow `ui` is.
fn hosted_by(ui: &Ui<'_, '_>) -> Option<LayerHost> {
    ui.cx.provided::<LayerHost>().filter(|host| host.id == ui.flow.id)
}

/// Shows a container mounted on a layer as the content of a layer child of the node `ui` builds,
/// keyed by the container's id. A layer-like container that is closed (its open binding or collapse
/// says so) shows nothing when its output has an unshown form.
fn show_on_layer<C: Container, R>(
    slot: &mut Option<C>,
    ui: &mut Ui<'_, '_>,
    overlay: &Overlay,
    body: &mut dyn FnMut(&mut Ui<'_, '_>, &mut BodyCx<'_>) -> R,
) -> Inner<C::Output, R> {
    let component = slot.as_mut().expect("a container is shown once");
    let props = component.props(Internal::new()).clone();
    let driving = driving_opener(component, &props);
    let ctx = ui.ctx().clone();
    // A layer its openers drive: while it is closed, the container is a zero-size node in this flow that runs the
    // openers' triggers and hold timer (`show_plain`); the layer child appears in the build round after it opens.
    if let Some(first) = &driving
        && !crate::contract::overlay::slot_open(&ctx, first.id)
    {
        let at = Rect::from_min_size(ui.flow.next_min(), Vec2::ZERO);
        let shown = show_plain(slot, ui, (ShowAt::Rect(at), Env::of(ui)), body, None, None);
        if crate::contract::overlay::slot_open(&ctx, first.id) {
            ui.cx.rebuild();
        }
        return shown;
    }
    if driving.is_none()
        && let Some(output) = unshown(slot, ui)
    {
        return Inner { output, inner: None, settle: None };
    }
    let source = slot.as_mut().expect("a container is shown once").common_mut(Internal::new()).id.clone();
    let (id, _) = ui.flow.peek_child(type_key::<C>(), source.clone());
    // A dismissal ui-rt or the layer's rules decided since the owner's last build closes a driven layer before it
    // shows: the openers' slot takes it, and the container's node in this flow holds the closed state.
    if let Some(first) = &driving
        && let Some(why) = crate::contract::overlay::closes_before_show(ui.cx, overlay, super::overlay::layer_of(overlay), id.value(), true)
    {
        crate::contract::overlay::set_opener_open(ui.cx, &ctx, first, false);
        if let Some(component) = slot.as_mut() {
            component.closed(why);
        }
        let at = Rect::from_min_size(ui.flow.next_min(), Vec2::ZERO);
        return show_plain(slot, ui, (ShowAt::Rect(at), Env::of(ui)), body, None, None);
    }
    let (id, _) = ui.flow.child(type_key::<C>(), source);
    // The content shows under the layer's id, which the container takes as its own (`show_plain`, hosted).
    let (tokens, sweep, rtl) = (ui.flow.tokens.clone(), ui.flow.sweep, Env::of(ui).direction.is_rtl());
    let anchor = Rect::from_min_size(ui.flow.next_min(), Vec2::ZERO);
    // Where the user moved the layer and how large they made it (`container_shape`): the position wins over the
    // overlay's own place unless the caller places the layer; the size over the content's.
    let save = slot.as_ref().and_then(|c| c.props(Internal::new()).save.clone());
    let placement = super::container_shape::placement_of(ui.ctx(), id, save.as_ref());
    let cx = &mut *ui.cx;
    // A driven layer hangs from the opener that opened it: its anchor, and a press on it is not outside the layer.
    let hung = driving.as_ref().map(|first| crate::contract::overlay::hung_from(&ctx, first, &overlay.place));
    let anchored;
    let overlay = match &hung {
        Some((_, anchor)) => {
            anchored = overlay.clone().anchor(anchor.clone());
            &anchored
        }
        None => overlay,
    };
    let base = LayerShow::of(overlay, cx.node(), anchor, rtl, &tokens);
    let mut show = LayerShow { sweep, surface: Surface::Plain, pad: 0.0, host: Some(id), ..base };
    if let Some((opener, _)) = hung {
        show.spec = show.spec.hung_from(super::ids::node_of(cx, opener).or(Some(cx.node())));
    }
    if overlay.placed.is_none()
        && overlay.placed_at.is_none()
        && let Some(pos) = placement.pos
    {
        show.spec.placement = craft_ui_rt::Placement::At(Pos2::from(pos));
    }
    let size = placement.size.map(Vec2::from);
    let layer = show.layer;
    let mut content = LayerContent { slot, body, size, shown: None };
    show_layer_with(cx, id, id.value(), show, true, &mut content);
    // The layer's content closed it (a row ran, an untaken Enter), or a trigger toggled it in the content's run: the
    // openers' slot takes the close, and the owner builds again in this frame without the layer.
    if let Some(first) = &driving {
        if crate::contract::overlay::closed_after_show(cx, layer, id.value()).is_some() {
            crate::contract::overlay::set_opener_open(cx, &ctx, first, false);
        }
        if !crate::contract::overlay::slot_open(&ctx, first.id) {
            cx.rebuild();
        }
    }
    content.shown.expect("a layer mount runs its content in every build that shows it")
}

/// Shows a `Viewport`-mounted container as the whole content of a window the node `ui` builds hosts
/// (`BuildCx::hosted_window`), built inside that node's build, so the borrowed body runs there. The window opens with
/// the container's first open build (a dialog window for a modal `fallback`, a tool window otherwise, unless `spec`
/// says how), at the container's ideal size, owned by the showing window; it closes when a build stops showing the
/// container (closed, or no longer shown). A window the user closes closes the container (`Container::closed`,
/// `Requested`). The container keeps the id it has in the showing flow and shares the showing window's env cells,
/// job board and command board.
fn show_in_window<C: Container, R>(
    slot: &mut Option<C>,
    ui: &mut Ui<'_, '_>,
    fallback: &Overlay,
    spec: Option<&crate::runtime::windows::WindowSpec>,
    body: &mut dyn FnMut(&mut Ui<'_, '_>, &mut BodyCx<'_>) -> R,
) -> Inner<C::Output, R> {
    if let Some(output) = unshown(slot, ui) {
        return Inner { output, inner: None, settle: None };
    }
    let component = slot.as_mut().expect("a container is shown once");
    let source = component.common_mut(Internal::new()).id.clone();
    let (id, _) = ui.flow.child(type_key::<C>(), source);
    let key = ("craft-viewport", id.value());
    // The spec is made once, when the window is first asked for: the ideal size needs a measuring run.
    let spec = match ui.cx.hosted_state(key) {
        Some(_) => None,
        None => {
            let size = measure_container(component, ui, craft_behavior::Proposal::Ideal, &mut *body);
            let title = component.common(Internal::new()).label.as_str().to_owned();
            let base = spec.cloned().unwrap_or_else(|| {
                use crate::runtime::windows::WindowSpec;
                if fallback.rank.is_modal() { WindowSpec::dialog() } else { WindowSpec::tool() }
            });
            let frame_of = |_: craft_host::target::WindowKind| None;
            let mut rt = super::windows::rt_spec(&base, &title, None, &frame_of);
            rt.size = Some(size);
            rt.owner = Some(ui.cx.window());
            Some(rt)
        }
    };
    let window = match ui.cx.hosted_window(key, || spec.expect("made when first asked")) {
        craft_ui_rt::HostedWindow::Open(window) => window,
        craft_ui_rt::HostedWindow::Opening => {
            let output = C::Output::unshown(Response::inert(id, Rect::NOTHING));
            if let Some(output) = output {
                return Inner { output, inner: None, settle: None };
            }
            // A container with no unshown form shows once more in the showing flow until its window opens.
            return show_plain(slot, ui, (ShowAt::Rect(Rect::from_min_size(ui.flow.next_min(), Vec2::ZERO)), Env::of(ui)), body, None, None);
        }
        craft_ui_rt::HostedWindow::Closed => {
            component.closed(crate::runtime::overlay::CloseReason::Requested);
            let output = C::Output::unshown(Response::inert(id, Rect::NOTHING));
            if let Some(output) = output {
                return Inner { output, inner: None, settle: None };
            }
            return show_plain(slot, ui, (ShowAt::Rect(Rect::from_min_size(ui.flow.next_min(), Vec2::ZERO)), Env::of(ui)), body, None, None);
        }
    };
    let env = super::scope::WindowEnv {
        cells: super::EnvCells::of(ui.cx),
        jobs: ui.cx.provided::<super::pending::JobBoard>(),
        commands: ui.cx.provided::<super::commands::CommandBoard>(),
    };
    let escape = super::dialog::scope_escape(fallback);
    let host = LayerHost { id, closed: BodyClose::default() };
    let theme = component.props(Internal::new()).theme.clone();
    let shown = ui.cx.window_content(window, |cx| {
        super::scope::window_root_with(cx, env, |ui| {
            ui.cx.provide(escape);
            let rect = ui.flow.region;
            let place = Env::of(ui);
            show_themed(slot, ui, (ShowAt::Rect(rect), place), body, None, Some(host), theme)
        })
    });
    shown.expect("an open hosted window builds in its requester's build")
}

/// A container shown as the content of its own layer child (`show_on_layer`).
struct LayerContent<'s, 'b, C: Container, R> {
    slot: &'s mut Option<C>,
    body: &'b mut dyn FnMut(&mut Ui<'_, '_>, &mut BodyCx<'_>) -> R,
    /// The size the user gave the layer with its grips, which its content then fills.
    size: Option<Vec2>,
    shown: Option<Inner<C::Output, R>>,
}

impl<C: Container, R> LayerBody<()> for LayerContent<'_, '_, C, R> {
    fn measure(&mut self, ui: &mut Ui<'_, '_>) -> Vec2 {
        let room = ui.flow.region.size();
        if let Some(size) = self.size {
            return size.min(room);
        }
        let body = &mut *self.body;
        self.slot.as_mut().map_or(Vec2::ZERO, |container| measure_in(container, ui, craft_behavior::Proposal::Ideal, body, true).min(room))
    }

    /// The container is the layer body's one child in flow: it starts at the region's start, and the layer is as large
    /// as the container shows (the layer's real size is its body flow's content size).
    fn show(&mut self, ui: &mut Ui<'_, '_>, _: &BodyClose) {
        let place = Env::of(ui);
        self.shown = Some(show_container(self.slot, ui, (ShowAt::Flow, place), self.body, None));
    }
}

/// `show_container` past the layer and theme rules. `hosted` is the layer the container is the
/// content of: the container takes the layer's id, and its close closes the layer.
fn show_plain<C: Container, R>(
    slot: &mut Option<C>,
    ui: &mut Ui<'_, '_>,
    (at, place): (ShowAt, Env),
    body: &mut dyn FnMut(&mut Ui<'_, '_>, &mut BodyCx<'_>) -> R,
    tip: Option<TipFn<'_>>,
    hosted: Option<LayerHost>,
) -> Inner<C::Output, R> {
    let mut component = slot.take().expect("a container is shown once");
    component.common_mut(Internal::new()).command_scope =
        component.props(Internal::new()).command_scope.or(component.common(Internal::new()).command_scope);
    let begun = super::run::begin(&mut component, ui);
    let common = &begun.common;
    let command = begun.step1.command.clone();
    let source = match &hosted {
        Some(host) => IdSource::Id(host.id),
        None => common.id.clone(),
    };
    let (id, child) = ui.flow.child(type_key::<C>(), source);
    let key = component_key(&component, &common);
    let props = component.props(Internal::new()).clone();
    // A layer mount its openers open and close: shown outside its layer, it is the zero-size node in the owner's flow
    // that holds the open decision while the layer is closed (`show_on_layer`).
    let driving = driving_opener(&mut component, &props);
    let gate = driving.is_some() && hosted.is_none();
    let mut shape = Shape::of(&props, &place, &ui.flow.tokens, &ui.flow, ComponentStyle::metrics(&*begun.style).padding)
        .in_bar(&props, &place)
        .in_popup(&props, hosted.is_some())
        .in_pane(&props);
    // A window mount is the whole content of its window.
    let at = match (&props.mount, at) {
        (Mount::Window { .. }, ShowAt::Flow) => ShowAt::Rect(ui.ctx().content_rect()),
        (_, at) => at,
    };
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
    // The layer scope's untaken Escape reaches the body of the container that is the layer's content.
    let cancel = hosted.is_some() && measuring.is_none() && std::mem::take(&mut ui.flow.scope.clear);
    let stage = if measuring.is_some() { MemoryStage::open(&ctx, body_id).peek_only() } else { MemoryStage::open(&ctx, body_id) };
    let regions = regions_env(&props, &place, resolved.enabled, shape.takes_room().0);
    // A container that is an instance of a command scope heads the command chain its regions and body run under: the
    // scope's commands resolve, publish and fire for this instance there.
    let regions = match props.command_scope.or(common.command_scope) {
        Some(scope) => regions.derive(|scoped| scoped.inherited_mut().commands = scoped.commands.pushed(scope, id)),
        None => regions,
    };
    let owner = BodyOwner::new(&*begun.style, shape.gap);
    let mut inner_value = None;
    let mut call = |ui: &mut Ui<'_, '_>, bcx: &mut BodyCx<'_>| {
        inner_value = Some(body(ui, bcx));
    };
    if let Some(under) = measuring {
        let peeked = under.and_then(|u| ui.cx.peek::<ComponentNode>(u, child)).map(|p| (p.node, p.local.body.open));
        let (node, kept_open) = (peeked.map(|p| p.0), peeked.and_then(|p| p.1));
        let open = read_open(&mut component, &props, kept_open, &ctx);
        shape.closed = !open;
        let value = steps::value(&mut component, &mut MemoryStage::open_peek(&ctx, id), id, command.as_deref());
        let mut frame = Frame::new(body_id, &regions, &resolved, stage, open, owner, gate_of(props.gates, &value), false);
        let tokens = ui.flow.tokens.clone();
        let into = Host { node, own_kept: false, marked: true, kept: None, sweep, ctx: ctx.clone() };
        let (size, flow) = lay_out(&mut component, ui.cx, &shape, into, &mut frame, (id, origin, room), &tokens, &mut call);
        ui.flow.absorb(&flow);
        let rect = match at {
            ShowAt::Flow => ui.flow.allocate(None, size, fills),
            ShowAt::Rect(rect) => Rect::from_min_size(rect.min, size),
        };
        let output = finish_measured(component, id, rect, &begun, value, ctx.clone());
        return Inner { output, inner: inner_value, settle: None };
    }
    let mut frame = Frame::new(body_id, &regions, &resolved, stage, true, owner, (None, false), cancel);
    let Ui { cx, flow, .. } = ui;
    let output = cx.keyed(child, |cx| {
        cx.inline(ComponentNode { key: key.map_or(0, fold), tag, text: component.text_input() }, |kept, state, cx| {
            // The kept stage runs in this window for this node; its session and saved slots go back to the runtime's
            // store when the run ends.
            kept.memory = std::mem::take(&mut kept.memory).within(&ctx, id).appearing(state.new);
            super::node::DescendantWatch::provide(kept, cx, false);
            let value = steps::value(&mut component, &mut kept.memory, id, command.as_deref());
            (frame.gate, frame.gate_mixed) = gate_of(props.gates, &value);
            frame.gate_at_read = frame.gate;
            let open = match &driving {
                Some(first) => {
                    let open = crate::contract::overlay::opener_open(cx, &mut kept.memory, first);
                    if first.all().any(crate::contract::overlay::Opener::claims_activation) {
                        crate::contract::overlay::claim_activation(cx);
                    }
                    open && !gate
                }
                None => read_open(&mut component, &props, kept.body.open, &ctx),
            };
            (frame.open, frame.open_at_read) = (open, open);
            // The collapse moves the body's height toward its new state; the node builds again each frame while it
            // moves (`Driver::anim` owes the next frame) and not after.
            if props.collapse.is_some_and(|c| c.animated) {
                let driver = Driver { stage: &kept.memory, base: id, policy: place.motion };
                shape.share = driver.anim_bool("collapse", open, spec::OPEN_RISE);
            }
            shape.closed = gate || (!open && shape.share <= 0.0);
            if matches!(props.mount, Mount::Layer(_)) {
                frame.moving = kept.memory.recall::<WindowMode>(window_mode_id(id)).moving();
            }
            let own_kept = holds(kept, key, state);
            let tokens = if own_kept { flow.tokens.clone() } else { super::env::tokens(cx) };
            let into = Host { node: Some(cx.node()), own_kept, marked: state.path || state.marked, kept: Some(&mut *kept), sweep, ctx: ctx.clone() };
            crate::layout::begin_capture(&ctx);
            let (size, mut inner) = lay_out(&mut component, cx, &shape, into, &mut frame, (id, origin, room), &tokens, &mut call);
            let size = if gate { Vec2::ZERO } else { size };
            kept.column_reports = crate::layout::end_capture(&ctx);
            // A container that owns its body's scroll without a scroll region (a canvas with a view of its own) takes
            // the reveals its children ask for: they reach its run as input (`Focus::take_reveal`). Its least move
            // brings the point to the edge of its view, and the scroll bodies around it bring that place into theirs.
            if props.scroll_owner
                && shape.scroll.is_none()
                && let Some(at) = inner.reveal.take()
            {
                kept.queue.push(NodeInput::Reveal(Rect::from_min_size(at, Vec2::ZERO)));
                let view = Rect::from_min_size(origin, size);
                inner.reveal = Some(at.clamp(view.min, view.max));
            }
            flow.absorb(&inner);
            // The header changed the open state or the gate after the body was laid out by the old one: the body read
            // it, so the container builds again in this frame (the same-frame rerun rule).
            if frame.open != frame.open_at_read {
                write_open(&mut component, kept, frame.open, &resolved, &props, &ctx);
                cx.rebuild();
            }
            if let Some(on) = frame.gate.filter(|on| Some(*on) != frame.gate_at_read) {
                write_gate(&mut component, on, &resolved);
                cx.rebuild();
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
                if let Some(host) = &hosted {
                    host.close(why);
                }
            }
            if own_kept && size == kept.size {
                kept.cache(room, size);
                // The body reconciled the children; the layer children are reached the same way.
                reach_layers(&mut component, kept, cx, rect, &begun, &tokens, ctx.clone());
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
            // The band, grip and pane drags run on the node's own run; a run that moved the layer or the pane asks for
            // another build round, so it stands where the step put it in this frame. The run's flow holds no region of
            // the container: the regions `lay_out` placed stay, with whatever the run's paint showed after them.
            let mut hooks = ContainerShape::new(&props, frame.geometry, &tokens);
            let placed = std::mem::take(&mut kept.placed);
            let output = run_shaped(component, kept, cx, ran, &mut hooks);
            let shown = std::mem::replace(&mut kept.placed, placed);
            kept.placed.extend(shown);
            if hooks.moved {
                cx.rebuild();
            }
            // The props' scope and roving group, where the component's hooks name none.
            if scope != crate::runtime::focus::FocusPolicy::NONE {
                kept.focus.scope = scope;
                cx.keep_focus_on_empty_presses(kept.focus.keeps_focus_on_empty_presses());
                cx.take_released_focus(kept.focus.takes_released_focus());
            }
            if roving.is_some() {
                kept.focus.roving = roving;
            }
            note_access(kept, &props, open);
            kept.memory.flush();
            output
        })
    });
    let ran = frame.ran;
    let settle = if ran {
        Some(crate::contract::settle::SettleAt::new(frame.stage, body_id))
    } else {
        frame.stage.write_back();
        None
    };
    Inner { output, inner: inner_value, settle }
}

/// What the props say about the container's access node beyond its component's `access` hook: its place in its set,
/// its live region, and the expanded state of a collapsing body.
fn note_access(kept: &mut Kept, props: &ContainerProps, open: bool) {
    let Some(node) = kept.access.as_mut() else { return };
    if let Some(position) = props.position {
        node.set_position_in_set(position.index);
        if let Some(size) = position.size {
            node.set_size_of_set(size);
        }
        if let Some(level) = position.level {
            node.set_level(level);
        }
    }
    if let Some(politeness) = props.live {
        node.set_live(match politeness {
            crate::runtime::access::Politeness::Polite => accesskit::Live::Polite,
            crate::runtime::access::Politeness::Assertive => accesskit::Live::Assertive,
        });
    }
    if props.collapse.is_some() {
        node.set_expanded(open);
    }
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
    let hosted = hosted_by(ui).is_some();
    measure_in(component, ui, proposal, body, hosted)
}

/// `measure_container`; `hosted`: the container is the content of its own layer, which measures it before it provides
/// the `LayerHost` (`LayerContent::measure`).
fn measure_in<C: Container, R>(
    component: &mut C,
    ui: &mut Ui<'_, '_>,
    proposal: craft_behavior::Proposal,
    body: &mut dyn FnMut(&mut Ui<'_, '_>, &mut BodyCx<'_>) -> R,
    hosted: bool,
) -> Vec2 {
    let room = match proposal {
        craft_behavior::Proposal::Size(size) => size,
        craft_behavior::Proposal::Width(width) => Vec2::new(width, ui.flow.room().y),
        craft_behavior::Proposal::Min => Vec2::ZERO,
        _ => ui.flow.room(),
    };
    let begun = super::run::begin(component, ui);
    let (id, child) = ui.flow.peek_child(type_key::<C>(), begun.common.id.clone());
    let key = component_key(component, &begun.common);
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
    let mut shape = Shape::of(&props, &begun.env, &ui.flow.tokens, &ui.flow, ComponentStyle::metrics(&*begun.style).padding)
        .in_bar(&props, &begun.env)
        .in_popup(&props, hosted);
    let open = read_open(component, &props, peeked.as_ref().and_then(|p| p.1), &ctx);
    shape.closed = !open;
    let size = {
        let resolved = begun.resolved(id);
        let body_id = sub(id, "body");
        let regions = regions_env(&props, &begun.env, resolved.enabled, shape.takes_room().0);
        let stage = MemoryStage::open(&ctx, body_id).peek_only();
        let value = steps::value(component, &mut MemoryStage::open_peek(&ctx, id), id, begun.step1.command.as_deref());
        let gate = gate_of(props.gates, &value);
        let owner = BodyOwner::new(&*begun.style, shape.gap);
        let mut frame = Frame::new(body_id, &regions, &resolved, stage, open, owner, gate, false);
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

/// The open state: the caller's binding (a collapsing body's, or a layer-like container's shown
/// state), else its openers' slot for a layer mount with openers, else the node's kept state, else
/// the state saved under `save`, else the collapse's initial state. A container with neither a
/// binding, openers nor a `Collapse` is always open.
fn read_open<C: Container>(component: &mut C, props: &ContainerProps, kept: Option<bool>, ctx: &crate::contract::Context) -> bool {
    if let Some(bound) = component.open().filter(|b| !b.is_kept()).and_then(|b| b.get()) {
        return matches!(bound, Mixed::Same(true));
    }
    if let Mount::Layer(overlay) = &props.mount
        && let Some(first) = &overlay.opener
    {
        return crate::contract::overlay::slot_open(ctx, first.id);
    }
    let Some(collapse) = props.collapse else { return true };
    let saved = || props.save.as_ref().and_then(|key| MemoryStage::peek_saved::<OpenState>(ctx, key).0);
    kept.or_else(saved).unwrap_or(collapse.initially_open)
}

/// The first opener of a layer mount whose openers open and close it: no caller binding holds its open state.
fn driving_opener<C: Container>(component: &mut C, props: &ContainerProps) -> Option<crate::contract::overlay::Opener> {
    let Mount::Layer(overlay) = &props.mount else { return None };
    let first = overlay.opener.as_ref()?;
    component.open().filter(|b| !b.is_kept()).is_none().then(|| first.clone())
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
    let inner = Rect::from_min_size(origin + shape.pad_min, (room - shape.pad_min - shape.pad_max).max(Vec2::ZERO));
    let real = into.kept.is_some();
    frame.geometry = HeaderGeometry { body_top: inner.min.y, ..HeaderGeometry::default() };
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
    // The header line: the controls at its end, measured first so the header knows its width. An inline header
    // stands at its measured width at the start of the body's line, and the body takes the rest of it.
    let mut header_w = 0.0;
    let (mut main_min, mut main_max) = (main_min, main_max);
    // The height and width an inline header takes on the body's line.
    let mut inline = (0.0_f32, 0.0_f32);
    if let Some(h) = shape.header {
        // The band is created before the header's nodes so it paints beneath them; it is placed once the header's
        // height is known, across the container's width from its top edge, whatever the padding.
        let band = shape.band.and_then(|color| rule_node(cx, real, Region::HeaderBand, color));
        let band_top = origin.y;
        let inset = h.inset.points(tokens);
        let line_rect = Rect::from_min_max(Pos2::new(main_min + inset, top), Pos2::new(main_max - inset, inner.max.y));
        let controls_w = region_size(cx, &into, Region::Controls, id, line_rect, tokens, &mut |ui| {
            frame.with(ui, line_rect, |ui, bcx| component.controls(ui, bcx))
        })
        .x;
        let line_rect = if h.inline {
            let measured = region_size(cx, &into, Region::Header, id, line_rect, tokens, &mut |ui| {
                frame.with_header(ui, line_rect, |ui, bcx| component.header(ui, bcx))
            });
            let width = (measured.x + controls_w).min(line_rect.width());
            match shape.rtl {
                true => Rect::from_min_max(Pos2::new(line_rect.max.x - width, top), line_rect.max),
                false => Rect::from_min_max(line_rect.min, Pos2::new(line_rect.min.x + width, inner.max.y)),
            }
        } else {
            line_rect
        };
        let (header_rect, controls_rect) = if shape.rtl {
            (
                Rect::from_min_max(Pos2::new(line_rect.min.x + controls_w, top), line_rect.max),
                Rect::from_min_max(line_rect.min, Pos2::new(line_rect.min.x + controls_w, inner.max.y)),
            )
        } else {
            (
                Rect::from_min_max(line_rect.min, Pos2::new(line_rect.max.x - controls_w, inner.max.y)),
                Rect::from_min_max(Pos2::new(line_rect.max.x - controls_w, top), line_rect.max),
            )
        };
        let (hn, header) = region(cx, &into, Region::Header, id, header_rect, line(), tokens, &mut |ui| {
            frame.with_header(ui, header_rect, |ui, bcx| component.header(ui, bcx))
        });
        let (cn, controls) = region(cx, &into, Region::Controls, id, controls_rect, line(), tokens, &mut |ui| {
            frame.with(ui, controls_rect, |ui, bcx| component.controls(ui, bcx))
        });
        place(hn, Rect::from_min_size(header_rect.min, header));
        place(cn, Rect::from_min_size(controls_rect.min, controls));
        header_w = header.x + controls.x + 2.0 * inset;
        frame.geometry.line = Some(Rect::from_min_max(Pos2::new(line_rect.min.x, top), Pos2::new(line_rect.max.x, top + header.y.max(controls.y))));
        let band_bottom = top + header.y.max(controls.y);
        let band_width = if room.x.is_finite() { room.x } else { header_w + shape.pad_min.x + shape.pad_max.x };
        place(band, Rect::from_min_size(Pos2::new(origin.x, band_top), Vec2::new(band_width, band_bottom - band_top)));
        if h.inline {
            // The body shares the header's line: it starts past the header (before it, right to left).
            let used = line_rect.width() + inset + shape.header_gap;
            if shape.rtl {
                main_max -= used
            } else {
                main_min += used
            }
            inline = (header.y.max(controls.y), used);
        } else {
            top += header.y.max(controls.y);
        }
        frame.after_rule = shape.bands.under > 0.0 || shape.heads;
        if shape.bands.under > 0.0 {
            let rule = Rect::from_min_size(Pos2::new(inner.min.x, top), Vec2::new(inner.width(), shape.bands.under));
            place(rule_node(cx, real, Region::RuleUnder, divider), rule);
            top += shape.bands.under;
        }
        // An empty header (nothing shown this pass) takes no gap.
        if !shape.closed && !h.inline && header.y.max(controls.y) > 0.0 {
            top += shape.header_gap;
        }
    }
    // The footer's height and the region under the body, known before the body that lies above them.
    // A footer at the foot or the top of the end region (`FooterPlace::EndBottom`, `EndTop`) is as wide as that region.
    let in_end = shape.end.is_some() && shape.footer.is_some_and(|f| matches!(f.place, FooterPlace::EndBottom | FooterPlace::EndTop));
    // An across footer beside a full-height end region lies under the body only, as wide as the body's column.
    let in_body_place = shape.footer.is_some_and(|f| f.place == FooterPlace::Body);
    let beside = shape.end.is_some_and(|e| e.full_height) && shape.footer.is_some() && !in_end && !in_body_place;
    let column = Rect::from_min_max(Pos2::new(main_min, inner.min.y), Pos2::new(main_max, inner.max.y));
    let footer_room = match (in_end, beside) {
        (true, _) => Rect::from_min_size(inner.min, Vec2::new(end_w, inner.height())),
        (false, true) => column,
        (false, false) => inner,
    };
    let footer_h = match shape.footer {
        Some(_) => {
            region_size(cx, &into, Region::Footer, id, footer_room, tokens, &mut |ui| {
                frame.with_footer(ui, footer_room, |ui, bcx| component.footer(ui, bcx))
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
    frame.geometry.body_top = main.min.y;
    let body_end_h = if shape.closed {
        0.0
    } else {
        region_size(cx, &into, Region::BodyEnd, id, main, tokens, &mut |ui| frame.with(ui, main, |ui, bcx| component.body_end(ui, bcx))).y
    };
    let body_end_band = if body_end_h > 0.0 { body_end_h + shape.gap } else { 0.0 };
    let body_rect = Rect::from_min_max(main.min, Pos2::new(main.max.x, (inner.max.y - footer_band - body_end_band).max(top)));
    // The container's field label beside or above the body: the body runs in the rect the label leaves.
    let row = body_rect;
    let (body_rect, label_rect) = match component.props(Internal::new()).label.clone().filter(|_| !shape.closed) {
        Some(side) => label_step(component, cx, &into, frame, shape, (id, origin, room), body_rect, &side, tokens, &mut place),
        None => (body_rect, Rect::NOTHING),
    };
    let (body_size, body_node, flow) = if shape.closed {
        (Vec2::ZERO, None, Flow::new(id, body_rect.min, body_rect, shape.gap, tokens.clone()))
    } else {
        match shape.scroll {
            Some(scroll) => scroll_body(cx, shape, &into, frame, id, body_rect, scroll, tokens, body),
            None => plain_body(cx, shape, &into, frame, id, body_rect, tokens, body),
        }
    };
    // While the collapse moves, the container shows the moving share of the body's height.
    let body_size = if shape.collapsing() { Vec2::new(body_size.x, body_size.y * shape.share) } else { body_size };
    place(body_node, Rect::from_min_size(body_rect.min, body_size));
    // With a field label the body's line holds the label and the body: as large as the two together.
    let body_size = match label_rect.is_positive() {
        true => (Rect::from_min_size(body_rect.min, body_size).union(label_rect).max - row.min).max(Vec2::ZERO),
        false => body_size,
    };
    let mut below = (top + body_size.y).max(top + inline.0);
    // An inline header's move band spans its whole line: header, body and end region.
    if shape.header.is_some_and(|h| h.inline) {
        frame.geometry.inline_line = Some(Rect::from_min_max(Pos2::new(inner.min.x, top), Pos2::new(inner.max.x, below)));
    }
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
                frame.with_footer(ui, footer_rect, |ui, bcx| component.footer(ui, bcx))
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
    let footer_top = match fills_height {
        true => inner.max.y - footer_h,
        false if beside => below + shape.footer_gap + shape.bands.footer,
        false => end_bottom + shape.footer_gap + shape.bands.footer,
    };
    let (footer_min, footer_max) = if beside { (column.min.x, column.max.x) } else { (inner.min.x, inner.max.x) };
    let mut footer_w = 0.0;
    // The body's real run placed its slot; a measuring run places none, and the footer takes no room of its own.
    let slot = if in_body && real { crate::contract::footer_slot::take(&ctx, id) } else { None };
    if let Some(slot) = slot {
        let (fnode, footer) = region(cx, &into, Region::Footer, id, slot, Dir::Stack, tokens, &mut |ui| {
            frame.with_footer(ui, slot, |ui, bcx| component.footer(ui, bcx))
        });
        place(fnode, Rect::from_min_size(slot.min, footer));
    } else if shape.footer.is_some() && !in_body && !in_end {
        if shape.bands.footer > 0.0 {
            let rule = Rect::from_min_max(Pos2::new(footer_min, footer_top - shape.bands.footer), Pos2::new(footer_max, footer_top));
            place(rule_node(cx, real, Region::RuleFooter, divider), rule);
        }
        let footer_rect = Rect::from_min_max(Pos2::new(footer_min, footer_top), Pos2::new(footer_max, footer_top + footer_h));
        let footer_dir = if shape.footer.is_some_and(|f| f.place == FooterPlace::Body) { Dir::Stack } else { line() };
        let (fnode, footer) = region(cx, &into, Region::Footer, id, footer_rect, footer_dir, tokens, &mut |ui| {
            frame.with_footer(ui, footer_rect, |ui, bcx| component.footer(ui, bcx))
        });
        place(fnode, Rect::from_min_size(footer_rect.min, footer));
        footer_w = footer.x;
    }
    let width = (body_size.x + inline.1 + end_band).max(header_w + end_band).max(footer_w);
    let height = match beside {
        true => end_bottom.max(below + footer_band) - inner.min.y,
        false => (end_bottom - inner.min.y) + footer_band,
    };
    let size = shape.outer(Vec2::new(width, height), room);
    // The divider along the edge of an edge pane that faces the window's centre (the start edge mirrors right to left).
    if let Some(edge) = shape.edge_rule {
        let thin = tokens.stroke.thin;
        let start_left = !shape.rtl;
        let rule = match edge {
            PaneEdge::Top => Rect::from_min_size(origin, Vec2::new(size.x, thin)),
            PaneEdge::Bottom => Rect::from_min_size(origin + Vec2::new(0.0, size.y - thin), Vec2::new(size.x, thin)),
            PaneEdge::Start if start_left => Rect::from_min_size(origin, Vec2::new(thin, size.y)),
            PaneEdge::End if !start_left => Rect::from_min_size(origin, Vec2::new(thin, size.y)),
            _ => Rect::from_min_size(origin + Vec2::new(size.x - thin, 0.0), Vec2::new(thin, size.y)),
        };
        place(rule_node(cx, real, Region::RuleEdge, divider), rule);
    }
    if let Some(kept) = into.kept
        && kept.placed != placed
    {
        kept.placed = placed;
        cx.request_layout();
    }
    (size, flow)
}

/// The container's label step (`ContainerProps::label`) in `row`, the body's rect: shows the label in the container's
/// node, gives the body the label scope its controls name (`label::under_label`), with the message cell the footer
/// reads. Returns the rect left for the body and the label's rect.
#[allow(clippy::too_many_arguments)]
fn label_step<C: Container>(
    component: &C,
    cx: &mut BuildCx<'_>,
    into: &Host<'_>,
    frame: &mut Frame<'_>,
    shape: &Shape,
    (id, origin, room): (Id, Pos2, Vec2),
    row: Rect,
    side: &SideLabel,
    tokens: &Arc<Tokens>,
    place: &mut dyn FnMut(Option<NodeId>, Rect),
) -> (Rect, Rect) {
    let props = component.props(Internal::new());
    let real = into.kept.is_some();
    let resolved = frame.resolved;
    let input = LabelIn {
        scope: id,
        owner: id,
        parent: Some(id),
        row,
        // A content-sized container is as wide as its content took: the choice between beside and above reads the room.
        room: if shape.sizing.content_width() && room.x.is_finite() { room.x.max(row.width()) } else { row.width() },
        content_row: false,
        egui_cell: frame.env.egui_cell,
        text: resolved.label_value,
        side,
        help: props.help.as_ref(),
        enabled: resolved.enabled,
        size: resolved.size,
        control: id,
        control_min: 0.0,
        leaf: false,
        in_column: shape.in_column,
        claims: false,
        status: props.footer.is_some_and(|footer| footer.status),
        trailing: Vec2::ZERO,
    };
    let flow = Flow::new(id.with("label"), origin, row, tokens.space.gap.row, tokens.clone()).with_sweep(into.sweep);
    let flow = if real { flow } else { flow.measuring(into.node) };
    let mut ui = Ui::new(cx, flow);
    let (placed, mut label) = crate::contract::label::run(&mut ui, frame.env, &input, None);
    let Ui { cx, flow, .. } = ui;
    for at in &flow.placed {
        place(Some(at.node), Rect::from_min_size(origin + at.offset, at.size));
    }
    label.message = real.then(|| crate::runtime::access::scope::message_cell(cx, id));
    frame.message = label.message;
    let body_env = frame.body_env.take().unwrap_or_else(|| frame.env.clone());
    frame.body_env = Some(crate::contract::label::under_label(&body_env, label));
    (placed.control, placed.label)
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
        cx.inline(ScopeNode { key: 0, clip: false }, |rk, state, cx| {
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
            let Ui { cx, mut flow, .. } = ui;
            super::node::set_drawn(&mut rk.drawn, cx, flow.take_drawn(flow.origin));
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
        // While the collapse moves, the body node is as high as the share it shows and clips its children to it.
        cx.inline(ScopeNode { key: 0, clip: shape.collapsing() }, |rk, state, cx| {
            let node = cx.node();
            let at = BodyAt { node: Some(node), own_kept: into.own_kept, marked: state.path || state.marked, sweep: into.sweep };
            let (mut flow, ran) = run_body(cx, shape, at, Some(&mut *rk), frame, id, region, rect.min, None, tokens, body);
            frame.ran |= ran;
            super::node::set_drawn(&mut rk.drawn, cx, flow.take_drawn(flow.origin));
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
    let fresh_in = |region: Rect, dir: Dir, given: Vec<usize>| {
        let mut flow = Flow::new(id, origin, region, shape.gap, tokens.clone()).with_dir(dir).in_view(view).with_sweep(at.sweep);
        flow.body = shape.body_flow(given);
        flow
    };
    let fresh = |dir: Dir, given: Vec<usize>| fresh_in(region, dir, given);
    // A grid's cells share a label column per grid column; the rows of a `label_column` body share one.
    // `column`: the first measuring sweep of a content-sized column (`Scoped::content_column`).
    let mut call = |ui: &mut Ui<'_, '_>, sweep: BodySweep| {
        let mut run = |ui: &mut Ui<'_, '_>| frame.with_body(ui, region, sweep, |ui, bcx| body(ui, bcx));
        if columns > 0 {
            ColumnGroup::grid(ui, sub(id, "grid"), columns, run);
        } else if shape.label_column {
            ColumnGroup::scope(ui, "labels", |ui, _| run(ui));
        } else {
            run(ui);
        }
    };
    let Some(kept) = kept else {
        // A content-sized `Stack` column settles its width from its children: its first sweep offers them no room
        // (`Scoped::content_column`), so a picture measures at its least and never sets the width. A held child that
        // measured narrower than the settled width shows at that width in the real run, so the children sweep again
        // offered it, and the column measures as tall as it shows; the others answer from their size cache.
        let content_column = shape.content_column;
        let mut ui = Ui::new(&mut *cx, fresh(shape.base_dir(columns), Vec::new()).measuring(at.node));
        let ctx = ui.ctx().clone();
        let first = if content_column { BodySweep::Column } else { BodySweep::Plain };
        let ((), narrowest) = crate::layout::held_sweep(&ctx, id, content_column, || call(&mut ui, first));
        let flow = ui.flow;
        let width = flow.content_size().x;
        // Half a point: less than a pixel shows at any scale the box runs.
        if narrowest >= width - 0.5 {
            return (flow, false);
        }
        let settled = Rect::from_min_size(region.min, Vec2::new(width, region.height()));
        let mut ui = Ui::new(cx, fresh_in(settled, shape.base_dir(columns), Vec::new()).measuring(at.node));
        call(&mut ui, BodySweep::Plain);
        return (ui.flow, false);
    };
    let had_fills = kept.body_fills.unwrap_or(true);
    let reusable = kept.plan.take().filter(|p| at.own_kept && !at.marked && p.room == room);
    let (plan, measured) = match reusable {
        Some(plan) => (Some(plan), false),
        // A content column's plan holds the width it settled at, from the same two sweeps as its measuring run, and
        // is reused while its room and key are unchanged and nothing under it is marked.
        None if shape.content_column => {
            let mut ui = Ui::new(&mut *cx, fresh(shape.base_dir(columns), Vec::new()).measuring(at.node));
            let ctx = ui.ctx().clone();
            let ((), narrowest) = crate::layout::held_sweep(&ctx, id, true, || call(&mut ui, BodySweep::Column));
            let mut flow = ui.flow;
            let width = flow.content_size().x;
            let settled = (narrowest < width - 0.5).then_some(width);
            if let Some(width) = settled {
                let column = Rect::from_min_size(region.min, Vec2::new(width, region.height()));
                let mut ui = Ui::new(&mut *cx, fresh_in(column, shape.base_dir(columns), Vec::new()).measuring(at.node));
                call(&mut ui, BodySweep::Plain);
                flow = ui.flow;
            }
            let units = std::mem::take(&mut flow.body.units);
            kept.body.levelled = !units.is_empty();
            let mut plan = shape.plan(room, std::mem::take(&mut flow.sizes), std::mem::take(&mut flow.rooms), &units, tokens);
            plan.settled = settled;
            (Some(plan), true)
        }
        None if shape.needs_plan(had_fills, kept.body.levelled || kept.body.shrinks) => {
            let mut ui = Ui::new(&mut *cx, fresh(shape.base_dir(columns), Vec::new()).measuring(at.node));
            call(&mut ui, BodySweep::Plain);
            let mut sizes = std::mem::take(&mut ui.flow.sizes);
            let mut rooms = std::mem::take(&mut ui.flow.rooms);
            let units = std::mem::take(&mut ui.flow.body.units);
            kept.body.levelled = !units.is_empty();
            // The shrink step of a row too long for its room: its children measure in no room (their ideal width) and at
            // their least, the units give way evenly down to their least, and a last sweep measures each at the width
            // it got: the plan's sizes, which the real run shows under `Shrink::Apply`.
            let mut shrinks = false;
            if shape.shrinks() && units.is_empty() && crate::layout::row_overflows(&sizes, shape.gap, room.x) {
                let open = Rect::from_min_size(region.min, Vec2::new(f32::INFINITY, region.height()));
                let mut sweep = |sweep: BodySweep, flow: Flow| {
                    let mut ui = Ui::new(&mut *cx, flow.measuring(at.node));
                    call(&mut ui, sweep);
                    (std::mem::take(&mut ui.flow.sizes), std::mem::take(&mut ui.flow.rooms))
                };
                let (ideal, _) = sweep(BodySweep::Plain, fresh_in(open, shape.base_dir(columns), Vec::new()));
                let (least, _) = sweep(BodySweep::Least, fresh_in(open, shape.base_dir(columns), Vec::new()));
                if ideal.len() == sizes.len() && least.len() == sizes.len() {
                    let caps = crate::layout::row_shrink(&ideal, &least, shape.gap, room.x);
                    // A unit's room is its cap alone: the plain sweep gave later children only what the earlier ones
                    // left at their ideal widths.
                    let given: Vec<Vec2> = rooms.iter().zip(&caps).map(|(room, cap)| cap.map_or(*room, |cap| Vec2::new(cap, room.y))).collect();
                    let flow = fresh(shape.base_dir(columns), Vec::new()).with_plan(Vec::new(), given.clone());
                    (sizes, _) = sweep(BodySweep::Apply, flow);
                    rooms = given;
                    shrinks = true;
                }
            }
            kept.body.shrinks = shrinks;
            let mut plan = shape.plan(room, sizes, rooms, &units, tokens);
            plan.shrinks = shrinks;
            (Some(plan), true)
        }
        None => (None, false),
    };
    let flow = match &plan {
        Some(plan) => {
            let region = plan.settled.map_or(region, |width| Rect::from_min_size(region.min, Vec2::new(width, region.height())));
            fresh_in(region, plan.dir.clone(), plan.levels.clone()).with_plan(plan.shares.clone(), plan.rooms.clone())
        }
        None => fresh(shape.base_dir(columns), Vec::new()),
    };
    let mut ui = Ui::new(cx, flow);
    call(&mut ui, if plan.as_ref().is_some_and(|plan| plan.shrinks) { BodySweep::Apply } else { BodySweep::Plain });
    let Ui { cx, flow, .. } = ui;
    // A row that overflows its room with no plan narrows its units by one from the next run on.
    let overflows = plan.is_none() && shape.shrinks() && crate::layout::row_overflows(&flow.sizes, shape.gap, room.x);
    kept.body.shrinks |= overflows;
    let stale = match &plan {
        // A content column with no settled width took its sizes offered no room: only its fill shares are planned.
        Some(plan) if shape.content_column && plan.settled.is_none() && flow.fills == 0 => false,
        Some(plan) => !same_sizes(&plan.sizes, &flow.sizes),
        None => flow.fills > 0 || overflows,
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
                cx.inline(ScopeNode { key: 0, clip: false }, |ck, cstate, cx| {
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
