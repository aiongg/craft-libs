//! The item step (17.3): the collection's side of steps 4 to 17 and its content sub-steps I1 to I5.

pub(super) mod rows;

use super::ValueOf;
use super::collection::{
    ActivateOn, CollectionEvent, CollectionProps, CollectionResponse, ContextMenuRequest, Gestures, LeadOnOpen, LeadSource, OpenChange, Rename,
    RowState, SelectMode, ShownRow, SpaceAction,
};
use super::cx::{BodyCx, Pointer as _};
use super::run::{Measuring, ShapeHooks, ShowAt};
use super::steps::ShapeKind;
use super::{Bind, Collection, Cx, Internal, ItemCx, ItemFn, ItemOpener, ItemOutput as _, PartsCx, Props, Show as _, Sizing};
use crate::primitives::ScrollBar;
use crate::runtime::access::{self, Role, SetPosition};
use crate::runtime::egui::{alloc, areas, child, response};
use crate::runtime::env::{AppValues, ArenaId, Env, ItemFacts};
use crate::runtime::memory::{MemoryStage, sub};
use crate::runtime::{focus, frame, keys, layout_memory, overlay, wheel};
use crate::style::{StyleCx, scroll_bar_look};
use craft_behavior::layout_memory::measure_tolerance;
use craft_behavior::{
    Axis, CancelReason, ColumnKey, EditEvent, Flow, GestureId, Granularity, Intent, ItemLayout, ItemSource, Mixed, MoveMods, NavMove, NavState,
    NavStep, Outline, PressPhase, Proposal, ScrollPos, ScrollSpan, SelectionChange, SelectionModel, Sense, TypeAhead, WheelTaker, nav,
};
use egui::{Context, Id, Pos2, Rect, Ui, UiBuilder, Vec2};
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;

mod drag;
pub(super) mod empty_text;
mod header;
mod marquee;
mod request;

pub use drag::{SweepApply, SweepStep};
pub use header::ColumnCommands;
pub(crate) use header::put_toggle_names;
pub(crate) use request::lead_position;
pub use request::request_rename;

use drag::Drags;
use header::{Header, HeaderMemory};

/// The rows the item step builds in a pass and where it puts them: what `PartsCx::rect_of` and `offset` read.
pub(super) struct Rows<K> {
    pub(super) rects: Vec<(K, Rect)>,
    pub(super) offset: ScrollPos,
}

const SCROLL: craft_behavior::PartKey = craft_behavior::PartKey::new("items_scroll");
const SECTION: craft_behavior::PartKey = craft_behavior::PartKey::new("section");
/// The sub-node of a toggle cell of an item row; the index is the hash of the row's key and the column.
const TOGGLE_CELL: craft_behavior::PartKey = craft_behavior::PartKey::new("toggle_cell");
/// The part an item row's own access node stands for in the collection: AccessKit requests on the row's node reach
/// the collection under this part, the index being the hash of the row's key.
const ITEM_NODE: craft_behavior::PartKey = craft_behavior::PartKey::new("item_node");

/// The part of a collection that stands for the section row `key` (`ItemSource::section_stop`): the item step builds
/// no row for it, so the access node that AccessKit focus goes to while the section holds the lead, and the pointer
/// part over it, are this part of the collection. It is the collection contract's one section mechanism: a component
/// that draws its own section headers lays them out as this part, so the lead, the pointer and the access tree treat
/// them as the collection's section rows.
pub fn section_part<K: std::hash::Hash>(key: &K) -> craft_behavior::PartRef {
    SECTION.at(key_hash(key))
}

#[derive(Clone, Default)]
struct Scrolled {
    y: f64,
    x: f64,
}

#[derive(Clone, Default)]
struct View {
    extent: f32,
    view: Option<[f32; 4]>,
}

#[derive(Clone)]
struct Lead<K> {
    key: Option<K>,
    at: usize,
    /// A sheet's lead cell column.
    column: usize,
    /// The `CollectionProps.rows_stamp` of the pass that last set the lead.
    stamp: Option<u64>,
}

impl<K> Default for Lead<K> {
    fn default() -> Self {
        Lead { key: None, at: 0, column: 0, stamp: None }
    }
}

#[derive(Clone)]
struct Renaming<K> {
    key: Option<K>,
    draft: String,
    gesture: GestureId,
    focused: bool,
}

impl<K> Default for Renaming<K> {
    fn default() -> Self {
        Renaming { key: None, draft: String::new(), gesture: GestureId::default(), focused: false }
    }
}

#[derive(Clone, Default)]
struct Typing(TypeAhead);

/// The item layout of the last pass, kept per collection in session memory.
#[derive(Clone)]
struct LastLayout<L>(Option<Arc<L>>);

impl<L> Default for LastLayout<L> {
    fn default() -> Self {
        LastLayout(None)
    }
}

#[derive(Clone)]
struct Held<S>(S);

impl<S: Default> Default for Held<S> {
    fn default() -> Self {
        Held(S::default())
    }
}

pub(super) struct ItemsHooks<'s, 'r, C: Collection> {
    source: &'s dyn ItemSource<C::Key>,
    item: &'s mut ItemFn<'r, C>,
    layout: Option<Arc<C::ItemLayout>>,
    /// The layout's column count this pass, which a row pick's whole-row cells span.
    columns: u32,
    /// The layout of the last pass, which `Collection::item_layout_from` may reuse.
    previous: Option<Arc<C::ItemLayout>>,
    content: Vec2,
    room: Vec2,
    /// A measuring run under a body sized from its content: the axes (x, y) the collection takes from its content.
    from_content: (bool, bool),
    scrolls: bool,
    /// The axis the rows overflow along.
    overflow: Axis,
    selection: C::Selection,
    selection_at_read: C::Selection,
    events: Vec<CollectionEvent<C::Key, C::ItemEvent>>,
    /// The click count of the press behind this pass's pointer activation, 0 when no pointer activated.
    activation_clicks: u8,
    visible: Range<usize>,
    /// The rows built this pass with their rects, for `collection_response`.
    shown: Vec<ShownRow<C::Key>>,
    offset: f64,
    offset_at_read: f64,
    offset_x: f64,
    offset_x_at_read: f64,
    last: View,
    lead: Lead<C::Key>,
    renaming: Renaming<C::Key>,
    ended_rename: bool,
    /// The editor of the rename that ended still held focus, so focus goes back to the collection.
    refocus: bool,
    /// The mapping between the screen and the rows when `ContainerProps.transform` zooms them.
    xf: Option<areas::LayerMap>,
    header: Header,
    drags: Drags<C::Key>,
    /// The row a key move brought to the lead in a collection without its own scroll: the page's scroll owner shows it.
    page_reveal: Option<usize>,
}

/// The primary-ness, click count, position in the layout's coordinates and scroll offset of a press on a row.
#[derive(Clone, Copy)]
struct RowPress {
    primary: bool,
    clicks: u32,
    origin: Pos2,
    offset: ScrollPos,
}

/// Where the rows sit in the collection's pass: the rows rect, the scroll offset and the reading direction.
#[derive(Clone, Copy)]
struct Placement {
    rect: Rect,
    offset: ScrollPos,
    rtl: bool,
}

impl<'s, 'r, C: Collection> ItemsHooks<'s, 'r, C> {
    pub(super) fn new(source: &'s dyn ItemSource<C::Key>, item: &'s mut ItemFn<'r, C>) -> Self {
        ItemsHooks {
            source,
            item,
            layout: None,
            columns: 0,
            previous: None,
            content: Vec2::ZERO,
            room: Vec2::ZERO,
            from_content: (false, false),
            scrolls: true,
            overflow: Axis::Y,
            selection: C::Selection::default(),
            selection_at_read: C::Selection::default(),
            events: Vec::new(),
            activation_clicks: 0,
            visible: 0..0,
            shown: Vec::new(),
            offset: 0.0,
            offset_at_read: 0.0,
            offset_x: 0.0,
            offset_x_at_read: 0.0,
            last: View::default(),
            lead: Lead::default(),
            renaming: Renaming::default(),
            ended_rename: false,
            refocus: false,
            xf: None,
            header: Header::default(),
            drags: Drags::default(),
            page_reveal: None,
        }
    }

    /// The rect of the rows: the content rect below the header row.
    fn below_header(&self, area: Rect) -> Rect {
        Rect::from_min_max(Pos2::new(area.min.x, area.min.y + self.header.height), area.max)
    }

    /// Whether the collection takes its width from its widest row: `Sizing::Content`. A row's width holds its
    /// column cells at the widths the header resolves for them.
    fn content_sized(&self, comp: &C) -> bool {
        comp.props(Internal::new()).sizing == Sizing::Content
    }

    /// The widest ideal width of the rows in this pass: each row is built as `content` builds it, with its column
    /// cells at the widths the header resolves with no room to share (a fixed or toggle column at its own width),
    /// and measured without allocating.
    fn widest_row(&mut self, comp: &C, ui: &Ui, id: Id) -> f32 {
        let ctx = ui.ctx().clone();
        super::steps::measuring(&ctx, || self.widest_row_in(comp, ui, id))
    }

    /// Builds the rows once to measure the widest and records it under `sub(id, "widest")` for `extent`, when this
    /// pass's width comes from the rows and no record of this pass serves the room (`Measured::serves`).
    fn record_widest(&mut self, comp: &C, at: &Measuring<'_, '_>, id: Id) {
        let ctx = at.ui.ctx();
        let room = match at.at {
            ShowAt::Rect(rect) => rect.size(),
            ShowAt::Flow => crate::layout::flow_room(at.ui),
        };
        if self.source.is_empty() || (room.x.is_finite() && !self.content_sized(comp)) {
            return;
        }
        let key = sub(id, "widest");
        let tolerance = measure_tolerance(crate::runtime::egui::scale::pixels_per_point(ctx));
        // The collection's width comes from its rows here: it is not given the width.
        if layout_memory::measured(ctx, key)
            .is_some_and(|found| layout_memory::Measured::serves(room.x, found.room.x, false, found.took.x, tolerance))
        {
            return;
        }
        self.header.measure(at.scx);
        let widest = self.widest_row(comp, at.ui, id);
        layout_memory::record_measured(ctx, key, layout_memory::Measured { room, took: Vec2::new(widest, 0.0) });
    }

    fn widest_row_in(&mut self, comp: &C, ui: &Ui, id: Id) -> f32 {
        let ctx = ui.ctx().clone();
        let env = Env::of(ui);
        let mut memory = MemoryStage::open_peek(&ctx, id);
        let cells: Vec<(craft_behavior::ColumnKey, Rect)> =
            self.header.columns(0.0).into_iter().map(|(key, width)| (key, Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 0.0)))).collect();
        let columns: Vec<craft_behavior::ColumnKey> = cells.iter().map(|(key, _)| key.clone()).collect();
        let toggles: Vec<craft_behavior::ColumnKey> = self.header.toggle_columns().map(|(key, _)| key).collect();
        let len = self.source.len();
        let mut widest = 0.0f32;
        for at in 0..len {
            if self.source.section_stop(at) {
                continue;
            }
            let key = self.source.key(at);
            let item_id = sub(id, ("key", key_hash(&key)));
            let position =
                SetPosition::new(at - self.source.section_stops_before(at)).of(len - self.source.section_stops_before(len)).columns(cells.len());
            let mut icx = ItemCx {
                key: &key,
                at,
                rect: Rect::ZERO,
                row: RowState::default(),
                depth: self.source.depth(at),
                set: position,
                row_index: at,
                row_count: len,
                column_count: cells.len(),
                env: env.view(),
                id: sub(item_id, "item"),
                cells: cells.clone(),
                grid: None,
                tip: None,
                openers: Vec::new(),
                item: item_id,
                memory: &mut memory,
            };
            let item = (self.item)(&key, &mut icx);
            let mut item = Props::id(comp.dress(item, &RowState::default()), item_id);
            let facts =
                ItemFacts { item: item_id, flags: crate::style::Flags::empty(), position, columns: columns.clone(), toggles: toggles.clone() };
            let installed = Env::install_for(
                ui,
                env.derive_local(|scoped| {
                    scoped.item = Some(item_id);
                    scoped.row = Some(rows::row_group(item_id));
                    scoped.item_facts = Some(facts);
                }),
            );
            widest = widest.max(item.measure_in(ui, Proposal::Ideal).x);
            Env::restore(&ctx, installed);
        }
        widest
    }

    fn rows_rect(&self, cx: &Cx<'_, C>) -> Rect {
        self.below_header(cx.core.content_rect)
    }

    fn max_offset(&self, view: f32) -> f64 {
        f64::from((self.content.y - view).max(0.0))
    }

    fn page_rows(&self, layout: &C::ItemLayout, view: f32) -> usize {
        let row = if self.source.is_empty() { 0.0 } else { layout.rect(0, ScrollPos::default()).height() };
        if row > 0.0 { ((view / row) as usize).saturating_sub(1).max(1) } else { 1 }
    }

    fn select_mode(comp: &C) -> SelectMode {
        comp.items().select
    }

    /// Reports and applies a selection change.
    fn change(&mut self, change: SelectionChange<C::Key>) {
        self.selection.apply(change.clone());
        self.events.push(CollectionEvent::Selection(change));
    }

    fn range(&self, from: usize, to: usize) -> Vec<C::Key> {
        let keys = (from.min(to)..=from.max(to)).filter(|&at| self.source.enabled(at) && !self.source.is_section(at)).map(|at| self.source.key(at));
        if from <= to { keys.collect() } else { keys.rev().collect() }
    }

    fn move_lead(&mut self, to: usize, by: LeadSource) {
        let key = self.source.key(to);
        self.lead.key = Some(key.clone());
        self.lead.at = to;
        self.events.push(CollectionEvent::LeadMoved { key, by });
    }

    /// The position of `key`: the lead's last position is tried first, so the common lookups of a pass do not scan.
    fn index_of(&self, key: &C::Key) -> Option<usize> {
        let hint = self.lead.at;
        if hint < self.source.len() && self.source.key(hint) == *key { Some(hint) } else { self.source.index_of(key) }
    }

    /// `point` of the rows' coordinates on the screen.
    fn on_screen(&self, point: Pos2) -> Pos2 {
        self.xf.map_or(point, |map| map.to_screen(point))
    }

    fn anchor_at(&self, lead: usize) -> usize {
        self.selection.anchor().and_then(|key| self.index_of(key)).unwrap_or(lead)
    }

    /// A press or key that selects row `at` the way the modifiers say.
    fn pick(&mut self, comp: &C, at: usize, extend: bool, toggle: bool) {
        if self.source.section_stop(at) {
            return;
        }
        let key = self.source.key(at);
        match Self::select_mode(comp) {
            SelectMode::None => {}
            SelectMode::Single => self.change(SelectionChange::Set(vec![key])),
            SelectMode::Multi | SelectMode::Cells if (extend || !toggle) && self.selection.cells().is_some() => {
                let anchor = if extend { self.anchor_at(self.lead.at) } else { at };
                self.select_rows(comp, anchor, at);
            }
            SelectMode::Multi | SelectMode::Cells => {
                if extend {
                    let anchor = self.anchor_at(self.lead.at);
                    self.change(SelectionChange::Set(self.range(anchor, at)));
                } else if toggle {
                    self.change(SelectionChange::Toggle(key));
                } else {
                    self.change(SelectionChange::Set(vec![key]));
                }
            }
        }
    }
}

impl<C: Collection> ShapeHooks<C> for ItemsHooks<'_, '_, C> {
    fn shape(&self) -> ShapeKind {
        ShapeKind::Item
    }

    fn host(&mut self, _: &C, _: super::run::HostCx<'_>) -> Box<dyn super::run::Host> {
        Box::new(super::run::Inline)
    }

    fn arena(&self, _: &C) -> bool {
        true
    }

    /// A collection that keeps focus on press senses the press without taking focus.
    fn sense(&self, comp: &C, own: Sense) -> Sense {
        if comp.items().focus_on_press { own } else { own.difference(Sense::TAKE_FOCUS).union(Sense::KEEP_FOCUS) }
    }

    fn key_role(&self, comp: &C) -> craft_behavior::KeyRole {
        let props = comp.items();
        craft_behavior::KeyRole::new()
            .reorderable(props.reorder.is_some())
            .pages(props.pages)
            .rename(props.rename)
            .type_ahead(true)
            .digits(props.digit_keys)
            .space(true)
            .moves(true)
            .deletes(props.deletable)
            .owner_arrows(props.item_keys == super::collection::ItemKeys::Owner)
            .home_row(self.layout.as_ref().map_or(0, |layout| layout.first_navigable()))
    }

    fn instance_scope(&self, comp: &C) -> Option<crate::runtime::commands::CommandScope> {
        comp.props(Internal::new()).command_scope
    }

    fn read(&mut self, comp: &mut C, stage: &mut MemoryStage, id: Id, env: &Env) {
        let ctx = stage.ctx().clone();
        let natural = comp.items().natural_height;
        // The scroll along the rows' overflow axis belongs to the collection: a body that scrolls along it too is a
        // second owner.
        let overflow = comp.items().overflow;
        let shared = env.scroll_owner.is_some() && if overflow == Axis::X { env.unbounded.0 } else { env.unbounded.1 };
        debug_assert!(
            !shared || natural,
            "a collection owns its scroll: show it outside a body that scrolls along its rows, or give it the natural height (16.3)"
        );
        self.scrolls = !natural && !shared;
        self.overflow = overflow;
        self.selection = match comp.selection() {
            Some(bind) => match bind.get() {
                Some(Mixed::Same(held)) => held,
                _ => C::Selection::default(),
            },
            None => stage.session::<Held<C::Selection>>(sub(id, "selection")).0.clone(),
        };
        self.selection_at_read = self.selection.clone();
        self.previous = stage.session::<LastLayout<C::ItemLayout>>(sub(id, "item_layout")).0.take();
        self.header.read(stage, id, comp.items());
        self.drags = stage.session::<Drags<C::Key>>(sub(id, "drags")).clone();
        self.lead = stage.session::<Lead<C::Key>>(sub(id, "lead")).clone();
        // A collection in a layer starts from `lead_on_open` at every opening of the layer, not only the first.
        if overlay::drawing_parent(&ctx).is_some_and(|layer| overlay::opening(&ctx, layer)) {
            self.lead = Lead::default();
        }
        let stamp = comp.items().rows_stamp;
        if stamp != self.lead.stamp {
            self.lead = Lead::default();
            self.lead.stamp = stamp;
            // The selection follows the lead: the one the collection holds goes with it.
            if comp.selection().is_none() {
                self.selection = C::Selection::default();
            }
        }
        // A selection can hold items this source does not show; a lead that was never a shown row is not a lead that left.
        if self.lead.key.is_none() {
            self.lead.key = self.selection.lead().filter(|key| self.index_of(key).is_some()).cloned();
        }
        self.renaming = stage.session::<Renaming<C::Key>>(sub(id, "rename")).clone();
        self.last = stage.recall::<View>(sub(id, "view"));
        let scrolled = match comp.scroll() {
            Some(bind) => match bind.get() {
                Some(Mixed::Same(pos)) => Scrolled { y: pos.y, x: pos.x },
                _ => Scrolled::default(),
            },
            None => stage.session::<Scrolled>(sub(id, "scroll")).clone(),
        };
        (self.offset, self.offset_x) = if self.scrolls { (scrolled.y, scrolled.x) } else { (0.0, 0.0) };
        self.offset_at_read = self.offset;
        self.offset_x_at_read = self.offset_x;
        let view = self.last.view.map_or(0.0, |view| view[3] - view[1]);
        if self.scrolls {
            let quiet = stage.is_peek() || crate::runtime::measure::active(&ctx);
            let taken = if quiet { wheel::peek(&ctx, id) } else { wheel::take(&ctx, id) };
            if let Some(share) = taken {
                self.offset -= f64::from(share.scroll.y);
                self.offset_x -= f64::from(share.scroll.x);
                if share.zoom != 1.0
                    && let Some(about) = share.at
                {
                    self.events.push(CollectionEvent::Zoom { factor: share.zoom, about });
                }
            }
            if let (Some(rect), Some(bounds)) = (if quiet { super::peek_reveal(&ctx, id) } else { super::take_reveal(&ctx, id) }, self.last.view) {
                if rect.min.y < bounds[1] {
                    self.offset += f64::from(rect.min.y - bounds[1]);
                } else if rect.max.y > bounds[3] {
                    self.offset += f64::from(rect.max.y - bounds[3]);
                }
            }
            self.offset = self.offset.clamp(0.0, f64::from((self.last.extent - view).max(0.0)));
        }
    }

    fn measured_extent(&mut self, comp: &mut C, at: &Measuring<'_, '_>, cx: &mut Cx<'_, C>) -> Vec2 {
        let id = cx.core.id;
        self.record_widest(comp, at, id);
        self.extent(comp, at.ui, at.at, id, at.scx)
    }

    fn pass_extent(&mut self, comp: &mut C, at: &Measuring<'_, '_>, _style: &C::Style, _shown: Option<Arc<ValueOf<C>>>) -> Vec2 {
        let id = at.resolved.id;
        self.record_widest(comp, at, id);
        self.extent(comp, at.ui, at.at, id, at.scx)
    }

    fn extent(&mut self, comp: &C, ui: &Ui, at: ShowAt, id: Id, scx: &StyleCx<'_>) -> Vec2 {
        let room = match at {
            ShowAt::Rect(rect) => rect.size(),
            ShowAt::Flow => crate::layout::flow_room(ui),
        };
        self.room = room;
        self.from_content = if crate::runtime::measure::active(ui.ctx()) { Env::of(ui).content } else { (false, false) };
        self.header.measure(scx);
        let bounded = room.x.is_finite();
        let mut widest = 0.0;
        let width = match (bounded, self.content_sized(comp)) {
            (true, false) => room.x,
            // No rows: the empty state has no ideal width, so it takes the room, or none where the room is unbounded.
            (_, _) if self.source.is_empty() => {
                if bounded {
                    room.x
                } else {
                    0.0
                }
            }
            // An unbounded room (a horizontally scrolling body) takes the widest row, as a content-sized collection does.
            (_, _) => {
                let floor = overlay::drawing_parent(ui.ctx()).map_or(0.0, |layer| super::overlay::floor_width(ui.ctx(), layer));
                widest = layout_memory::measured(ui.ctx(), sub(id, "widest")).map_or(0.0, |measured| measured.took.x);
                widest.max(comp.ideal_width(scx)).max(floor).min(room.x)
            }
        };
        let previous = self.layout.take().or_else(|| self.previous.take()).map(|held| Arc::try_unwrap(held).unwrap_or_else(|held| (*held).clone()));
        let mut layout = comp.item_layout_from(previous, self.source, width, scx);
        if self.header.has_columns() {
            layout.set_columns(&if self.content_sized(comp) { self.header.columns_measured(width, widest) } else { self.header.columns(width) });
        }
        let ctx = ui.ctx();
        rows::measured_heights(ctx, |height| layout.measured(&|at| height(rows::measured_id(id, &self.source.key(at)))));
        rows::measured_widths(ctx, |width| {
            let asking: Vec<usize> = (0..self.source.len()).filter(|&at| layout.is_auto_width(at)).collect();
            for at in asking {
                if let Some((ideal, min)) = width(rows::measured_width_id(id, &self.source.key(at))) {
                    layout.set_width(at, ideal, min);
                }
            }
        });
        let content = layout.content_size();
        self.content = Vec2::new(content.x as f32, content.y as f32);
        self.columns = layout.column_count() as u32;
        self.layout = Some(Arc::new(layout));
        let empty = if self.source.is_empty() { comp.empty_height(width, ui, scx) } else { 0.0 };
        let height = self.content.y.max(empty) + self.header.height;
        if !(self.scrolls && room.y.is_finite()) {
            // Under a body sized from its content, a `Fill` collection offers its least width here too: the body
            // has no room of its own to hand it, and the layout above was made at the enclosing room.
            if self.from_content.0 && comp.props(Internal::new()).sizing == Sizing::Fill {
                let wide = if self.header.has_columns() { self.header.least_width() } else { self.widest_row(comp, ui, id).min(room.x) };
                return Vec2::new(wide, height);
            }
            return Vec2::new(self.content.x, height);
        }
        // A scrolling collection in a bounded room: `Content` takes its rows up to the room, `Fill` the room.
        match comp.props(Internal::new()).sizing {
            // Under a body sized from its content, a measuring run takes the columns' least width and the rows up to
            // the room, as a content-sized collection does: the body has no room of its own to hand it.
            Sizing::Fill => {
                let (x, y) = self.from_content;
                let wide = if !x {
                    0.0
                } else if self.header.has_columns() {
                    self.header.least_width()
                } else {
                    self.widest_row(comp, ui, id).min(room.x)
                };
                Vec2::new(wide, if y { height.min(room.y) } else { 0.0 })
            }
            Sizing::Content | Sizing::FillWidth => Vec2::new(if self.content_sized(comp) { self.content.x } else { 0.0 }, height.min(room.y)),
        }
    }

    fn sized(&self, comp: &C, _: &Ui, size: Vec2) -> Vec2 {
        let sizing = comp.props(Internal::new()).sizing;
        // Only a `Fill` collection measures its width from its content under a content-sized body; `FillWidth` takes
        // the room it is given on every run.
        let width_from_content = self.from_content.0 && sizing == Sizing::Fill;
        let width = if self.room.x.is_finite() && !self.content_sized(comp) && !width_from_content { self.room.x.max(size.x) } else { size.x };
        let height = match (self.scrolls && self.room.y.is_finite(), sizing) {
            (true, Sizing::Fill) if !self.from_content.1 => self.room.y,
            (true, Sizing::Fill) => size.y.min(self.room.y),
            (true, Sizing::Content | Sizing::FillWidth) => size.y.min(self.room.y),
            (false, _) => size.y,
        };
        Vec2::new(width, height)
    }

    fn geometry(&mut self, _: &C, content: Rect, rtl: bool) -> Option<Rc<dyn std::any::Any>> {
        let view = self.below_header(content);
        let offset = ScrollPos { x: self.offset_x, y: self.offset.clamp(0.0, self.max_offset(view.height())) };
        let mut layout = self.layout.take()?;
        Arc::make_mut(&mut layout).set_view(view.size(), offset);
        let (_, order) = self.built(&layout, view, offset);
        let rects = order.into_iter().map(|at| (self.source.key(at), self.screen(&layout, view, at, offset, rtl))).collect();
        self.layout = Some(layout);
        Some(Rc::new(Rows { rects, offset }))
    }

    fn parts(&mut self, _: &mut C, _: Rect, pcx: &mut PartsCx<'_, C>) {
        self.header.parts(pcx);
        if !self.scrolls {
            return;
        }
        let view = self.below_header(pcx.content_rect);
        let thickness = scroll_bar_look(pcx.tokens).thickness;
        let track = if pcx.flow.is_rtl() {
            Rect::from_min_max(view.min, Pos2::new(view.min.x + thickness, view.max.y))
        } else {
            Rect::from_min_max(Pos2::new(view.max.x - thickness, view.min.y), view.max)
        };
        let content = f64::from(self.content.y).max(f64::from(view.height()));
        self.offset = self.offset.clamp(0.0, self.max_offset(view.height()));
        let span = ScrollSpan::new(0.0..content, self.offset..self.offset + f64::from(view.height()));
        pcx.scroll(SCROLL, track, Axis::Y, span);
    }

    fn sensed(&mut self, comp: &mut C, cx: &mut Cx<'_, C>) {
        cx.core.collection_selection = Some(Box::new(self.selection.clone()));
        if self.scrolls {
            // The root's wheel sense registered a taker for the whole rect before this step; the scroll region's taker
            // comes after it, so it wins the share where the two rects are equal: Shift turns its vertical wheel sideways.
            let view = self.below_header(cx.core.content_rect);
            wheel::register(
                &cx.core.ctx,
                WheelTaker::new(cx.core.id, view)
                    .vertical(self.overflow == Axis::Y)
                    .horizontal(self.content.x > view.width())
                    .scrolls(true)
                    .zoom(comp.zooms()),
            );
        }
    }

    fn updated(&mut self, comp: &mut C, cx: &mut Cx<'_, C>) {
        use super::Pointer as _;
        let rect = self.rows_rect(cx);
        let view = rect.height();
        let rtl = cx.core.flow.is_rtl();
        self.xf = comp.props(Internal::new()).transform.map(|zoom| areas::body_map(rect.min, zoom.scale, zoom.offset));
        if let Some(range) = cx.scroll_moved(SCROLL) {
            self.offset = range.start;
        }
        self.offset = self.offset.clamp(0.0, self.max_offset(view));
        let header_held = self.header_step(cx);
        self.offset_x = self.offset_x.clamp(0.0, f64::from((self.content.x - rect.width()).max(0.0)));
        self.open_lead(comp);
        self.follow_lead_source(comp);
        let lead_at = self.lead.key.as_ref().and_then(|key| self.index_of(key));
        let props = comp.items().clone();
        let Some(layout) = self.layout.take() else { return };
        let offset = ScrollPos { x: self.offset_x, y: self.offset };
        let at = Placement { rect, offset, rtl };
        // A press lands on the rows the user saw: the offset of the last pass, before this pass's wheel share (8.3).
        let seen = Placement { offset: ScrollPos { x: self.offset_x_at_read, y: self.offset_at_read }, ..at };
        self.press_step(comp, cx, &props, &layout, seen);
        if props.lead_follows_hover {
            self.hover_lead(comp, cx, &layout, seen);
        }
        self.drag_step(comp, cx, &layout, (rect, offset, rtl), &props);
        self.marquee_step(comp, cx, &layout, (rect, offset, rtl), &props);
        self.navigate(comp, cx, &layout, view, header_held);
        self.switch_page(comp, cx, &props, &layout, view);
        let lead_at = self.lead.key.as_ref().and_then(|key| self.index_of(key)).or(lead_at);
        self.intent_actions(comp, cx, &props, lead_at, header_held);
        self.context_key(cx, &layout, at, lead_at);
        self.delete_key(cx, &props, lead_at);
        self.toggle_by_access(cx, &layout, at);
        self.node_by_access(comp, cx, &layout, at);
        self.reorder_and_rename(comp, cx, &props, lead_at);
        self.requested_rename(comp, cx, &props, &layout, view);
        self.requested_reveal(comp, &layout, view);
        self.type_ahead(comp, cx, &layout, view, lead_at);
        request::record_lead(&cx.core.ctx, cx.core.id, self.lead.key.as_ref().and_then(|key| self.index_of(key)));
        if let Some(at) = self.page_reveal.take().filter(|&at| layout.shown(at)) {
            cx.core.reveal = Some(self.screen(&layout, rect, at, ScrollPos::default(), rtl));
        }
        self.layout = Some(layout);
    }

    fn apply(&mut self, comp: &mut C, cx: &mut Cx<'_, C>) {
        if self.selection == self.selection_at_read {
            return;
        }
        let id = cx.core.id;
        match comp.selection() {
            Some(mut bind) => {
                let (label, doc_key) = (cx.core.label_arc(), cx.core.resolved.doc_key);
                let before = self.selection_at_read.clone();
                bind.apply(&EditEvent::set(Mixed::Same(before), self.selection.clone(), label, doc_key));
            }
            None => cx.core.memory.session::<Held<C::Selection>>(sub(id, "selection")).0 = self.selection.clone(),
        }
        self.selection_at_read = self.selection.clone();
    }

    fn content(&mut self, comp: &mut C, ui: &mut Ui, env: &Env, cx: &mut Cx<'_, C>, values: &AppValues) {
        let Some(mut layout) = self.layout.take() else { return };
        let (id, ctx) = (cx.core.id, cx.core.ctx.clone());
        let rect = self.rows_rect(cx);
        let rtl = env.direction.is_rtl();
        let node = matches!(C::ROLE, Role::Node(_));
        let offset = ScrollPos { x: self.offset_x, y: self.offset };
        Arc::make_mut(&mut layout).set_view(rect.size(), offset);
        if self.scrolls {
            let size = self.content.max(rect.size());
            let left = if rtl { rect.max.x + self.offset_x as f32 - size.x } else { rect.min.x - self.offset_x as f32 };
            let content = Rect::from_min_size(Pos2::new(left, rect.min.y - self.offset as f32), size);
            crate::runtime::egui::widgets::note_scroll_region(&ctx, rect.intersect(ui.clip_rect()), content);
        }
        let sheet = Self::selects_cells(comp);
        let columns: Vec<u64> = match layout.grid() {
            Some(grid) => grid.sections.iter().flat_map(|section| section.columns.clone().map(u64::from)).collect(),
            None => (0..layout.column_count() as u64).collect(),
        };
        let grid = layout.grid().map(|grid| self.grid_screen(&layout, grid, rect, rtl));
        let (shown, order) = self.built(&layout, rect, offset);
        self.visible = shown.first().copied().unwrap_or(0)..shown.last().map_or(0, |last| last + 1);
        let command_scope = comp.props(Internal::new()).command_scope;
        if let Some(scope) = command_scope {
            crate::runtime::commands::register_instance(&ctx, scope, id);
        }
        let base_env = env.derive(|scoped| {
            scoped.values.extend(values);
            scoped.arena = Some(ArenaId::new(id));
        });
        self.header.paint(ui, cx, env, id, rtl);
        let len = self.source.len();
        let tree = matches!(layout.nav(), craft_behavior::NavShape::Tree);
        focus::registration::record_tree(&ctx, id, tree);
        let focused = focus::is_focused(&ctx, id);
        let lead = self.lead.key.clone();
        let mut lead_child = None;
        let striped = comp.items().striped;
        let current: Option<C::Key> = comp.items().current.as_deref().and_then(|key| key.downcast_ref::<C::Key>()).cloned();
        let mut stripe: Option<(usize, usize)> = None;
        let transform = comp.props(Internal::new()).transform;
        let (parent, clip) = (ui.layer_id(), rect.intersect(ui.clip_rect()));
        let forward =
            areas::Forward { id: sub(id, "transform"), sense: response::to_egui(cx.core.root_sense), rect: cx.core.rect.intersect(ui.clip_rect()) };
        let placed = self.selection.placed(self.source);
        let mut row_groups = Vec::with_capacity(order.len());
        let mut shown_rows: Vec<ShownRow<C::Key>> = Vec::with_capacity(order.len());
        // A section row the component lays out as its own `section_part` reports its own node; any other is the
        // collection's: a heading named by the row's text.
        for &at in order.iter().filter(|&&at| self.source.section_stop(at) && layout.shown(at)) {
            if cx.part_if_declared(section_part(&self.source.key(at))).is_none() {
                let row = self.screen(&layout, rect, at, offset, rtl);
                self.report_section_node(&ctx, id, at, row, cx.core.resolved.enabled);
            }
        }
        // The pointer counts as over a row only while the sensing pass puts it on this collection, so a popup or another
        // widget above it takes the hover as it does for the rows' parts.
        let hover_at = crate::primitives::sensing::read_outcome(&ctx, ArenaId::new(cx.core.id)).hover.and(ctx.pointer_latest_pos());
        let place_rows = |ui: &mut Ui| {
            for at in order {
                if self.source.section_stop(at) {
                    continue;
                }
                let key = self.source.key(at);
                let item_id = sub(id, ("key", key_hash(&key)));
                let row_group = rows::row_group(item_id);
                row_groups.push(row_group);
                let mut row = self.screen(&layout, rect, at, offset, rtl);
                let state = RowState {
                    selected: placed.holds_row(at),
                    lead: lead.as_ref() == Some(&key),
                    renaming: self.renaming.key.as_ref() == Some(&key),
                    striped: striped && {
                        let run = self.stripe_run(stripe, at);
                        stripe = Some((at, run));
                        run.is_multiple_of(2) && !self.source.is_section(at)
                    },
                    dragging: self.drags.grabbed.as_ref() == Some(&key),
                    drop_target: self.drags.over.as_ref() == Some(&key),
                    current: current.as_ref() == Some(&key),
                    open: self.source.has_children(at) && self.source.open(at),
                    hovered: hover_at.is_some_and(|pos| row.contains(pos)),
                    ..RowState::default()
                };
                let mut flags = crate::style::Flags::empty();
                flags.set(crate::style::Flags::SELECTED, state.selected);
                let child = if state.lead { rows::child_focus(&ctx, row_group) } else { None };
                lead_child = lead_child.or(child);
                // A list that never takes key focus (a popup list: the host forwards its keys) shows its lead always.
                flags.set(crate::style::Flags::LEAD, state.lead && (focused || child.is_some() || !comp.items().focus_on_press));
                flags.set(crate::style::Flags::EXPANDED, state.open);
                flags.set(crate::style::Flags::DRAGGING, state.dragging);
                flags.set(crate::style::Flags::DROP_TARGET, state.drop_target);
                flags.set(crate::style::Flags::CURRENT, state.current);
                let mut position = SetPosition::new(at - self.source.section_stops_before(at))
                    .of(len - self.source.section_stops_before(len))
                    .columns(columns.len());
                if tree {
                    position = position.level(self.source.depth(at) + 1);
                }
                let mut cells = Vec::new();
                layout.cells(at, offset, &mut cells);
                for (_, cell) in &mut cells {
                    *cell = self.to_screen(&layout, rect, *cell, rtl);
                }
                if sheet {
                    rows::put_cell_flags(&ctx, item_id, self.cell_flag_list(&placed, at, &columns, focused));
                }
                let column_keys: Vec<craft_behavior::ColumnKey> = cells.iter().map(|(key, _)| key.clone()).collect();
                let shown_cells = cells.clone();
                let toggle_keys: Vec<craft_behavior::ColumnKey> = self.header.toggle_columns().map(|(key, _)| key).collect();
                let mut icx = ItemCx {
                    key: &key,
                    at,
                    rect: row,
                    row: state,
                    depth: self.source.depth(at),
                    set: position,
                    row_index: at,
                    row_count: len,
                    column_count: columns.len(),
                    env: cx.core.env,
                    id: sub(item_id, "item"),
                    cells,
                    grid: grid.as_ref(),
                    tip: None,
                    openers: Vec::new(),
                    item: item_id,
                    memory: &mut *cx.core.memory,
                };
                icx.memory.claim(icx.id);
                let item = (self.item)(&key, &mut icx);
                let mut item = comp.dress(item, &state);
                let tip = icx.tip.take();
                let openers = std::mem::take(&mut icx.openers);
                drop(icx);
                let builder = UiBuilder::new().max_rect(row).layout(child::column_layout(rtl)).scope_id(sub(item_id, "row"));
                let builder = if node { access::with_parent(builder, id) } else { builder };
                let mut row_ui = alloc::child(ui, builder);
                // A scrolled row slides under the frozen rows: it never paints into their band.
                let below_frozen = Rect::from_min_max(Pos2::new(rect.min.x, rect.min.y + layout.frozen_band(at)), rect.max);
                row_ui.set_clip_rect(below_frozen.intersect(ui.clip_rect()));
                let installed = Env::install_for(
                    &row_ui,
                    base_env.derive_local(|scoped| {
                        scoped.item = Some(item_id);
                        scoped.row = Some(row_group);
                        scoped.item_facts = Some(ItemFacts { item: item_id, flags, position, columns: column_keys, toggles: toggle_keys });
                    }),
                );
                item = Props::id(item, item_id);
                if layout.is_auto(at) {
                    let measured = item.measure_in(&row_ui, Proposal::Width(row.width())).y;
                    rows::record_measure(&ctx, rows::measured_id(id, &key), Vec2::new(row.width(), measured));
                    if measured != row.height() {
                        Arc::make_mut(&mut layout).set_height(at, measured);
                        row = self.screen(&layout, rect, at, offset, rtl);
                    }
                }
                if layout.is_auto_width(at) {
                    let ideal = item.measure_in(&row_ui, Proposal::Ideal).x;
                    let min = item.measure_in(&row_ui, Proposal::Min).x;
                    rows::record_measure(&ctx, rows::measured_width_id(id, &key), Vec2::new(ideal, min));
                    Arc::make_mut(&mut layout).set_width(at, ideal, min);
                    row = self.screen(&layout, rect, at, offset, rtl);
                }
                shown_rows.push(ShownRow::new(key.clone(), row, shown_cells.clone()));
                let out = match tip {
                    Some(tip) => item.show_at_with_tip(&mut row_ui, row, tip),
                    None => item.show_at(&mut row_ui, row),
                };
                // After the row is shown: its toggle cells have recorded their names by then.
                self.report_toggle_cells(&ctx, (id, item_id), (at, &key), (shown_cells.as_slice(), cx.core.resolved.enabled), &cx.core.env);
                let label = out.label_rect().unwrap_or(row);
                crate::runtime::keys::redirect_node(&ctx, item_id, id, ITEM_NODE.at(key_hash(&key)));
                if let Some(row) = shown_rows.last_mut() {
                    row.openers = self.item_openers(&row_ui, &ctx, id, cx.core.resolved.enabled, openers);
                }
                if let Some(ask) = out.open_request() {
                    self.events.push(CollectionEvent::Open(OpenChange { key: key.clone(), open: ask.open, all: ask.all }));
                }
                if let Some(event) = out.into_event() {
                    self.events.push(CollectionEvent::Item(key.clone(), event));
                }
                if state.renaming {
                    let mut draft = std::mem::take(&mut self.renaming.draft);
                    let name = self.source.text(at).map(|text| text.into_owned()).unwrap_or_default();
                    let editor = comp.rename_editor(&mut row_ui, Bind::bound(&mut draft), label, &name);
                    self.renaming.draft = draft;
                    if let Some(editor) = editor {
                        if !self.renaming.focused {
                            focus::request_now(&ctx, focus::FocusTarget::Id(editor.response.id));
                            self.renaming.focused = true;
                        }
                        let gesture = self.renaming.gesture;
                        let before = Mixed::Same(Rename { key: key.clone(), name: name.clone() });
                        if matches!(editor.event, Some(EditEvent::Commit { .. } | EditEvent::Cancel { .. })) {
                            self.refocus = focus::is_focused(&ctx, editor.response.id);
                        }
                        // The editor claims Escape for the whole rename, as a text part with a draft does: Escape does not leave the
                        // row. One the editor leaves untaken (it holds no draft yet) cancels the rename.
                        keys::hold_escape(&ctx, editor.response.id);
                        let escaped = editor.event.is_none() && focus::is_focused(&ctx, editor.response.id) && {
                            let mut intents = keys::intents_of(&ctx, Some(editor.response.id), rtl);
                            let left = intents.take(Intent::Cancel);
                            keys::settle(&ctx, Some(editor.response.id), &intents);
                            left
                        };
                        match editor.event {
                            None if escaped => {
                                self.events.push(CollectionEvent::Rename(EditEvent::cancel(gesture, CancelReason::Escape, Some(before))));
                                self.ended_rename = true;
                                self.refocus = true;
                            }
                            Some(EditEvent::Commit { after, .. }) => {
                                let after = Rename { key: key.clone(), name: after };
                                self.events.push(CollectionEvent::Rename(EditEvent::commit(gesture, before, after)));
                                self.ended_rename = true;
                            }
                            Some(EditEvent::Cancel { reason, .. }) => {
                                self.events.push(CollectionEvent::Rename(EditEvent::cancel(gesture, reason, Some(before))));
                                self.ended_rename = true;
                            }
                            // Enter on a name nobody typed in opens no edit run: the rename ends with no event and no undo step, as a
                            // text field's empty commit does.
                            None if editor.submitted => {
                                self.ended_rename = true;
                                self.refocus = true;
                            }
                            None if editor.left_by_tab.is_some() => {
                                self.events.push(CollectionEvent::Rename(EditEvent::cancel(gesture, CancelReason::Blur, Some(before))));
                                self.ended_rename = true;
                            }
                            _ => {}
                        }
                        if let Some(forward) = editor.left_by_tab.filter(|_| self.ended_rename) {
                            // Tab continues the rename on the next renamable row; past the last one focus returns to the collection.
                            let renamable = |row: &usize| !self.source.section_stop(*row) && comp.can_rename(&self.source.key(*row));
                            let next = if forward { (at + 1..len).find(renamable) } else { (0..at).rev().find(renamable) };
                            self.refocus = next.is_none();
                            if let Some(next) = next {
                                request::request_rename(&ctx, id, &self.source.key(next));
                            }
                        }
                    }
                }
                Env::restore(&ctx, installed);
            }
        };
        match transform {
            Some(transform) => {
                let layer = areas::body_layer(parent, id);
                let tf = areas::body_transform(rect.min, transform.scale, transform.offset);
                crate::primitives::sensing::set_layer(&ctx, ArenaId::new(id), layer);
                let body_layer = areas::BodyLayer { parent, layer, transform: tf, at: rect.min, clip, forward: Some(forward) };
                areas::show_body_layer(&ctx, body_layer, place_rows);
            }
            None => place_rows(ui),
        }
        self.paint_marquee(ui, env, &layout, (rect, offset, rtl));
        focus::register_row_groups(&ctx, id, rtl, row_groups);
        self.shown = shown_rows;
        cx.core.grid = crate::contract::GridPlace::grid(len, columns.len());
        let size = layout.content_size();
        self.content = Vec2::new(size.x as f32, size.y as f32);
        if self.ended_rename {
            self.renaming = Renaming::default();
            if self.refocus {
                focus::request_now(&ctx, focus::FocusTarget::Id(id));
            }
            self.refocus = false;
        }
        if (focused || lead_child.is_some())
            && let Some(at) = lead.as_ref().and_then(|key| self.index_of(key))
        {
            let key = self.source.key(at);
            if self.source.section_stop(at) {
                access::set_focus(&ctx, crate::runtime::access::sub_node_id(id, section_part(&key)));
            } else {
                let item_id = sub(id, ("key", key_hash(&key)));
                let cell = (sheet && self.selection.lead_column().is_some()).then_some(self.lead.column);
                access::set_focus(&ctx, crate::runtime::access::items::focus_node(item_id, lead_child, cell));
            }
        }
        if self.source.is_empty() {
            let mut open = true;
            let builder = UiBuilder::new().max_rect(rect).layout(child::column_layout(rtl)).scope_id(sub(id, "empty"));
            let mut empty_ui = alloc::child(ui, builder);
            let installed = Env::install_for(&empty_ui, env.derive(|scoped| scoped.values.extend(values)));
            let (resolved, view) = (cx.core.resolved, cx.core.env);
            let mut bcx = BodyCx::new(sub(id, "body"), view, resolved, rect, &mut *cx.core.memory, &mut open, None);
            comp.empty(&mut empty_ui, &mut bcx);
            Env::restore(&ctx, installed);
        }
        if self.scrolls {
            let look = scroll_bar_look(&env.tokens);
            let thickness = look.thickness;
            let track = if rtl {
                Rect::from_min_max(rect.min, Pos2::new(rect.min.x + thickness, rect.max.y))
            } else {
                Rect::from_min_max(Pos2::new(rect.max.x - thickness, rect.min.y), rect.max)
            };
            let content = f64::from(self.content.y).max(f64::from(rect.height()));
            if content > f64::from(rect.height()) {
                let span = ScrollSpan::new(0.0..content, self.offset..self.offset + f64::from(rect.height()));
                let (at, len) = craft_behavior::scroll::thumb(&span, track.height(), look.min_thumb);
                let thumb = super::cx::thumb_rect(track, Axis::Y, rtl, at, len);
                let result = cx.core.results.get(&(SCROLL, Some(super::cx::THUMB))).cloned().unwrap_or(crate::primitives::PartResult::IDLE);
                let state = super::steps::part_state(&result, cx.core.resolved.enabled, None);
                ui.painter().extend(ScrollBar::shapes(&look, track, thumb, &state));
            }
        }
        self.layout = Some(layout);
    }

    fn settle(&mut self, comp: &mut C, cx: &mut Cx<'_, C>) {
        let id = cx.core.id;
        // A collection's focus move reveals through its lead (`page_reveal`), never the collection's whole rect.
        crate::runtime::focus::take_reveal_of(&cx.core.ctx, id);
        if self.scrolls && (self.offset != self.offset_at_read || self.offset_x != self.offset_x_at_read) {
            match comp.scroll() {
                Some(mut bind) => {
                    let (label, doc_key) = (cx.core.label_arc(), cx.core.resolved.doc_key);
                    let (before, after) =
                        (ScrollPos { x: self.offset_x_at_read, y: self.offset_at_read }, ScrollPos { x: self.offset_x, y: self.offset });
                    bind.apply(&EditEvent::set(Mixed::Same(before), after, label, doc_key));
                }
                None => {
                    let scrolled = cx.core.memory.session::<Scrolled>(sub(id, "scroll"));
                    (scrolled.y, scrolled.x) = (self.offset, self.offset_x);
                }
            }
        }
        *cx.core.memory.session::<Lead<C::Key>>(sub(id, "lead")) = self.lead.clone();
        cx.core.memory.session::<LastLayout<C::ItemLayout>>(sub(id, "item_layout")).0 = self.layout.clone();
        *cx.core.memory.session::<HeaderMemory>(sub(id, "columns")) = self.header.persist();
        *cx.core.memory.session::<Drags<C::Key>>(sub(id, "drags")) = self.drags.clone();
        self.header.report(&cx.core.ctx, id, cx);
        *cx.core.memory.session::<Renaming<C::Key>>(sub(id, "rename")) = self.renaming.clone();
        cx.core.collection_selection = Some(Box::new(self.selection.clone()));
        cx.core.collection_events = Some(Box::new((std::mem::take(&mut self.events), std::mem::take(&mut self.shown), self.activation_clicks)));
        cx.core.visible_rows = self.visible.clone();
        cx.core.gestures = Gestures::of(comp.items());
    }

    fn record(&mut self, stage: &mut MemoryStage, _: &Context, id: Id) {
        let view = stage.shown::<View>(sub(id, "view"));
        view.extent = self.content.y;
    }
}

impl<C: Collection> ItemsHooks<'_, '_, C> {
    /// The positions the pass builds: the rows in view, and all of them followed by the rows kept alive while
    /// scrolled out of view (the lead, the anchor, the renaming row and the grabbed row).
    fn built(&self, layout: &C::ItemLayout, view: Rect, offset: ScrollPos) -> (Vec<usize>, Vec<usize>) {
        let mut shown = Vec::new();
        layout.visible(offset, view.size(), &mut shown);
        let keep = [self.lead.key.as_ref(), self.selection.anchor(), self.renaming.key.as_ref(), self.drags.grabbed.as_ref()];
        let mut extra: Vec<usize> = Vec::new();
        // An item of a closed group has no rect: it is never built.
        for at in keep.into_iter().flatten().filter_map(|key| self.index_of(key)).filter(|&at| layout.shown(at)) {
            if !shown.contains(&at) && !extra.contains(&at) {
                extra.push(at);
            }
        }
        let order = shown.iter().copied().chain(extra).collect();
        (shown, order)
    }

    /// The rows since the last section row up to and including `at` (0 for a section row), continuing from `last`, the
    /// result for an earlier row: stripes start again after a section.
    fn stripe_run(&self, last: Option<(usize, usize)>, at: usize) -> usize {
        match last {
            Some((from, run)) if from <= at => (from + 1..=at).fold(run, |run, row| if self.source.is_section(row) { 0 } else { run + 1 }),
            _ => (0..=at).rev().take_while(|&row| !self.source.is_section(row)).count(),
        }
    }

    /// The lead of a collection that shows rows for the first time, or in a layer that opens: the first row that can
    /// be picked when the props ask for it.
    fn open_lead(&mut self, comp: &C) {
        if self.lead.key.is_none()
            && comp.items().lead_on_open == LeadOnOpen::First
            && let Some(at) = (0..self.source.len())
                .find(|&at| self.source.enabled(at) && !self.source.is_section(at) && self.layout.as_ref().is_none_or(|layout| layout.shown(at)))
        {
            self.lead.key = Some(self.source.key(at));
            self.lead.at = at;
        }
    }

    /// The lead follows a moving pointer over the rows, and the selection follows the lead the way it does for the
    /// arrow keys.
    fn hover_lead(&mut self, comp: &C, cx: &Cx<'_, C>, layout: &C::ItemLayout, at: Placement) {
        let Placement { rect, offset, rtl } = at;
        let frame = crate::runtime::pointer::frame(&cx.core.ctx);
        let hovered = cx.core.response.hovered() || crate::primitives::sensing::read_outcome(&cx.core.ctx, ArenaId::new(cx.core.id)).hover.is_some();
        let Some(pos) = frame.pos.filter(|_| hovered && frame.delta != Vec2::ZERO) else { return };
        let pos = self.xf.map_or(pos, |map| map.to_layer(pos));
        if !rect.contains(pos) {
            return;
        }
        let local = pos - rect.min.to_vec2();
        let local = if layout.mirrors() && rtl {
            Flow::new(Rect::from_min_size(Pos2::ZERO, rect.size()), rtl).to_physical(Rect::from_min_size(local, Vec2::ZERO)).min
        } else {
            local
        };
        if let Some(row) = layout.hit(local, offset).filter(|&row| self.source.enabled(row) && !self.source.is_section(row) && layout.shown(row))
            && self.lead.key.as_ref() != Some(&self.source.key(row))
        {
            self.move_lead(row, LeadSource::Pointer);
            self.pick(comp, row, false, false);
        }
    }

    /// A lead whose key left the source moves to the key now in its place, else the one before it.
    fn follow_lead_source(&mut self, comp: &C) {
        let len = self.source.len();
        if let Some(key) = self.lead.key.clone()
            && self.index_of(&key).is_none()
        {
            self.lead.key = None;
            if len > 0 {
                let to = self.lead.at.min(len - 1);
                self.move_lead(to, LeadSource::Keys);
                if self.holds_key(&key, to) {
                    self.pick(comp, to, false, false);
                }
            }
        } else if let Some(at) = self.lead.key.as_ref().and_then(|key| self.index_of(key)) {
            self.lead.at = at;
        }
    }

    /// What a press on an enabled row does besides selecting it: it activates the row at once, or on release, or opens
    /// its name for editing.
    fn press_activation(&mut self, comp: &C, props: &CollectionProps, layout: &C::ItemLayout, key: C::Key, at: usize, press: RowPress) {
        let RowPress { primary, .. } = press;
        // A section stop activates on the first primary press whatever `activate` says: the press is its toggle.
        let activates = match props.activate {
            _ if self.source.section_stop(at) => primary && press.clicks == 1,
            ActivateOn::DoubleClick => press.clicks == 2,
            ActivateOn::Click => false,
        };
        let on_release = primary && !self.source.section_stop(at) && matches!(props.activate, ActivateOn::Click);
        let names = primary
            && press.clicks == 2
            && props.rename
            && props.rename_column.is_some()
            && layout.column_at(press.origin, press.offset).and_then(|column| self.header.key_at(column)) == props.rename_column.as_ref()
            && comp.can_rename(&key);
        if names && !self.source.section_stop(at) {
            self.drags.rename = Some(key);
        } else if activates {
            self.activation_clicks = press.clicks.min(u8::MAX as u32) as u8;
            self.events.push(CollectionEvent::Activate(key));
        } else if on_release {
            self.drags.activate = Some(key);
        }
    }

    /// A press on a row selects it; a double press activates it; a right press asks for its menu.
    fn press_step(&mut self, comp: &C, cx: &mut Cx<'_, C>, props: &CollectionProps, layout: &C::ItemLayout, at: Placement) {
        let Placement { rect, offset, rtl } = at;
        let ctx = cx.core.ctx.clone();
        let flow = Flow::new(Rect::from_min_size(Pos2::ZERO, rect.size()), rtl);
        let xf = self.xf;
        // A screen point in the layout's logical coordinates, relative to the rows' rect.
        let local = |at: Pos2| {
            let at = xf.map_or(at, |map| map.to_layer(at)) - rect.min.to_vec2();
            if layout.mirrors() && rtl { flow.to_physical(Rect::from_min_size(at, Vec2::ZERO)).min } else { at }
        };
        let mods_now = crate::runtime::pointer::frame(&ctx).mods;
        if let PressPhase::Down { start, .. } = cx.core.press.phase
            && local(start.origin).y >= 0.0
        {
            let primary = start.button == craft_behavior::PointerButton::Primary;
            if let Some(at) = layout.hit(local(start.origin), offset).filter(|&at| self.source.enabled(at) && layout.shown(at)) {
                let on_toggle = self.toggle_cell(props, layout, at, offset, local(start.origin));
                // A secondary press on a toggle cell asks for the cell's menu and neither toggles nor picks.
                let secondary_on_toggle = !primary && on_toggle.is_some();
                let toggle = on_toggle.filter(|_| primary);
                if let Some((column, toggle)) = toggle {
                    self.begin_toggle(cx, at, column, toggle);
                } else if !secondary_on_toggle {
                    let mods = crate::runtime::pointer::frame(&ctx).mods;
                    let key = self.source.key(at);
                    let column = if Self::selects_cells(comp) { layout.column_at(local(start.origin), offset) } else { None };
                    // A plain press on a selected row of a multiple selection leaves the selection whole, so a drag
                    // carries all of it; the release as a click narrows it to the row. In a single selection that
                    // allows an empty one, the release as a click empties it.
                    let holds = match Self::select_mode(comp) {
                        SelectMode::Multi => true,
                        SelectMode::Single => props.allow_empty,
                        _ => false,
                    };
                    let hold = primary && !mods.shift && !mods.cmd && holds && self.holds(at);
                    // A secondary press on a row of the selection leaves the selection whole: its menu acts on all of it.
                    let keep = !primary && matches!(Self::select_mode(comp), SelectMode::Multi) && self.holds(at);
                    match column {
                        Some(column) => self.cell_press(comp, layout, at, column, mods.shift, mods.cmd),
                        None if hold => self.drags.collapse = Some(key.clone()),
                        None if keep => {}
                        None => self.pick(comp, at, mods.shift, mods.cmd),
                    }
                    self.move_lead(at, LeadSource::Pointer);
                    // A secondary press takes focus the way a primary press does: egui and the press classification only move it for the primary button.
                    if !primary && props.focus_on_press {
                        focus::request_now(&ctx, focus::FocusTarget::Id(cx.core.id));
                    }
                    self.press_activation(
                        comp,
                        props,
                        layout,
                        key,
                        at,
                        RowPress { primary, clicks: start.clicks, origin: local(start.origin), offset },
                    );
                }
            } else if primary
                && props.allow_empty
                && !(props.marquee && (mods_now.shift || mods_now.cmd || mods_now.alt))
                && !matches!(Self::select_mode(comp), SelectMode::None)
                && self.selection != C::Selection::default()
            {
                self.change(SelectionChange::Clear);
            }
        }
        // A press that a part of an item won leaves the selection alone and moves the lead to its row. A press on the
        // part of a section row (`section_part`) is the section's own, so the lead goes to it as well. Key focus goes
        // to the collection's stop with the lead, as a press on the row's own area takes it, unless the part takes
        // focus itself or keeps it where it is.
        let outcome = crate::primitives::sensing::read_outcome(&ctx, ArenaId::new(cx.core.id));
        if let (Some(PressPhase::Down { start, .. }), Some(target)) = (outcome.phase, outcome.target)
            && target.part.is_some()
            && local(start.origin).y >= 0.0
            && let Some(at) = layout.hit(local(start.origin), offset).filter(|&at| self.source.enabled(at) && layout.shown(at))
            && (target.owner != cx.core.id || target.part.is_some_and(|part| part.key == SECTION))
        {
            self.move_lead(at, LeadSource::Pointer);
            if props.focus_on_press && !target.sense.intersects(Sense::TAKE_FOCUS | Sense::KEEP_FOCUS) {
                focus::request_now(&ctx, focus::FocusTarget::Id(cx.core.id));
            }
        }
        // A click on a row's thumbnail is also a click on the row: the item reports the thumbnail's click and an unmodified click picks
        // in the same pass.
        if let (Some(PressPhase::Release { click: true, start, .. }), Some(target)) = (outcome.phase, outcome.target)
            && start.button == craft_behavior::PointerButton::Primary
            && target.part.is_some()
            && target.sense.contains(Sense::PICKS_ITEM)
            && local(start.origin).y >= 0.0
            && let Some(at) = layout.hit(local(start.origin), offset).filter(|&at| self.source.enabled(at) && layout.shown(at))
        {
            // A modified click is a command on the thumbnail (load as selection, and so on), not a pick.
            let mods = crate::runtime::pointer::frame(&ctx).mods;
            if !(mods.shift || mods.cmd || mods.alt) {
                self.pick(comp, at, false, false);
            }
        }
        match cx.core.press.phase {
            PressPhase::Release { click, start, pos, .. } => {
                if click
                    && props.menu_on_click
                    && start.button == craft_behavior::PointerButton::Primary
                    && crate::primitives::sensing::read_outcome(&ctx, ArenaId::new(cx.core.id))
                        .hover
                        .is_none_or(|hit| hit.part.is_none() || !hit.sense.takes_press())
                    && let Some(row) = layout.hit(local(pos), offset).filter(|&row| self.source.enabled(row) && layout.shown(row))
                {
                    let request = self.menu_request(layout, Placement { rect, offset, rtl }, row, local(pos));
                    self.events.push(CollectionEvent::ContextMenu(request));
                }
                if let Some(key) = self.drags.rename.take()
                    && click
                    && let Some(at) = self.index_of(&key)
                {
                    self.begin_rename(cx, at);
                }
                if let Some(key) = self.drags.activate.take()
                    && click
                    && self.index_of(&key).is_some()
                {
                    self.activation_clicks = start.clicks.min(u8::MAX as u32) as u8;
                    self.events.push(CollectionEvent::Activate(key));
                }
                if let Some(key) = self.drags.collapse.take()
                    && click
                    && let Some(at) = self.index_of(&key)
                {
                    if Self::select_mode(comp) == SelectMode::Single {
                        self.change(SelectionChange::Clear);
                    } else {
                        self.pick(comp, at, false, false);
                    }
                }
            }
            PressPhase::Cancel { .. } => {
                self.drags.collapse = None;
                self.drags.rename = None;
                self.drags.activate = None;
            }
            _ => {}
        }
        // A secondary release on a row asks for its menu (9.2 rule 3); the row joins the selection unless it is in it.
        // Something over the collection, or a part of a row or of the collection that takes presses and is under the
        // pointer, keeps the request away.
        let released = crate::runtime::pointer::frame(&ctx);
        if released.release == Some(craft_behavior::PointerButton::Secondary)
            && cx.core.response.hovered()
            && crate::primitives::sensing::read_outcome(&ctx, ArenaId::new(cx.core.id))
                .hover
                .is_none_or(|hit| hit.part.is_none() || !hit.sense.takes_press())
            && let Some(pos) = released.pos.filter(|pos| rect.contains(xf.map_or(*pos, |map| map.to_layer(*pos))))
            && let Some(at) = layout.hit(local(pos), offset)
            && self.source.enabled(at)
        {
            if !self.holds(at) && self.toggle_cell(props, layout, at, offset, local(pos)).is_none() {
                let column = if Self::selects_cells(comp) { layout.column_at(local(pos), offset) } else { None };
                match column {
                    Some(column) => self.cell_press(comp, layout, at, column, false, false),
                    None => self.pick(comp, at, false, false),
                }
            }
            self.move_lead(at, LeadSource::Pointer);
            let request = self.menu_request(layout, Placement { rect, offset, rtl }, at, local(pos));
            self.events.push(CollectionEvent::ContextMenu(request));
        }
    }

    /// The request for the menu of the row `at`, opened by a pointer at `point` (the layout's coordinates): at the
    /// row's bottom start corner, with the column the point lies in.
    fn menu_request(&self, layout: &C::ItemLayout, at: Placement, row: usize, point: Pos2) -> ContextMenuRequest<C::Key> {
        let Placement { rect, offset, rtl } = at;
        let place = self.on_screen(self.screen(layout, rect, row, offset, rtl).left_bottom());
        let mut cells = Vec::new();
        layout.cells(row, offset, &mut cells);
        let column = cells.into_iter().find(|(_, cell)| cell.contains(point)).map(|(column, _)| column);
        ContextMenuRequest { key: self.source.key(row), at: place, column }
    }

    /// The pass's moves: lead, selection, expand and collapse.
    fn navigate(&mut self, comp: &C, cx: &mut Cx<'_, C>, layout: &C::ItemLayout, view: f32, header_held: bool) {
        let len = self.source.len();
        let mode = Self::select_mode(comp);
        let outline = Outline::new(self.source).columns(layout.column_count()).layout(layout);
        let cells = layout.column_count() > 0 && Self::selects_cells(comp);
        let page = self.page_rows(layout, view);
        while !header_held && let Some((mv, mods)) = cx.core.intents.take_move() {
            if len == 0 {
                break;
            }
            if self.row_child_move(cx, layout, mv, mods) {
                continue;
            }
            let lead = self.lead.key.as_ref().and_then(|key| self.index_of(key));
            if let Some(at) = lead.filter(|&at| self.source.section_stop(at))
                && self.section_move(at, mv, mods)
            {
                continue;
            }
            // A range of cells that extends moves from its head, which is not the lead (the active cell).
            let start = lead
                .map(|at| (at, self.lead.column))
                .map(|lead| if cells && mods.extend { self.cell_head(comp, layout).unwrap_or(lead) } else { lead });
            let mut state = NavState::new().page(page).column(start.map_or(self.lead.column, |(_, column)| column));
            state.lead = start.map(|(at, _)| at);
            state.anchor = lead.map(|at| self.anchor_at(at)).filter(|_| mods.extend);
            let step = match nav::step(layout.nav(), &outline, &mut state, mv, mods) {
                // The ends of the outline, and the first move into it, land on an item, not on a section stop.
                NavStep::Moved(to) if self.source.section_stop(to) && (lead.is_none() || matches!(mv, NavMove::Start | NavMove::End)) => {
                    let forward = !matches!(mv, NavMove::End | NavMove::Back | NavMove::Up | NavMove::PageUp);
                    NavStep::Moved(self.off_section(layout, to, forward))
                }
                step => step,
            };
            match step {
                NavStep::Moved(to) if cells => {
                    let reveal = self.cell_move(comp, layout, mv, to, state.column, mods);
                    self.reveal_with(layout, reveal, view);
                }
                NavStep::Column(column) if cells => {
                    if let Some((at, _)) = start {
                        let reveal = self.cell_move(comp, layout, mv, at, column, mods);
                        self.reveal_with(layout, reveal, view);
                    }
                }
                // An arrow at the edge of the grid still runs the rules: it collapses the selection to the active cell.
                NavStep::Stay if cells && !mods.lead && rows::cell_direction(mv, mods).is_some() => {
                    if let Some((at, column)) = start {
                        self.cell_move(comp, layout, mv, at, column, mods);
                    }
                }
                NavStep::Column(column) => self.lead.column = column,
                NavStep::Moved(to) => {
                    self.move_lead(to, LeadSource::Keys);
                    if mods.extend && matches!(mode, SelectMode::Multi | SelectMode::Cells) {
                        let anchor = state.anchor.unwrap_or(to);
                        self.change(SelectionChange::Set(self.range(anchor, to)));
                    } else if !mods.lead && !mods.extend && comp.items().arrows_select {
                        self.pick(comp, to, false, false);
                    }
                    self.reveal_with(layout, to, view);
                }
                NavStep::Expand(at) => self.events.push(CollectionEvent::Open(OpenChange::new(self.source.key(at), true))),
                NavStep::Collapse(at) => self.events.push(CollectionEvent::Open(OpenChange::new(self.source.key(at), false))),
                _ => {}
            }
        }
    }

    /// Ctrl+Tab and its platform equivalents at a collection of pages: the lead and the selection move to the next
    /// (or previous) item that can be taken, wrapping at the ends.
    fn switch_page(&mut self, comp: &C, cx: &mut Cx<'_, C>, props: &CollectionProps, layout: &C::ItemLayout, view: f32) {
        if !props.pages {
            return;
        }
        let forward = if cx.core.intents.take_exact(&Intent::switch_page(true)) {
            true
        } else if cx.core.intents.take_exact(&Intent::switch_page(false)) {
            false
        } else {
            return;
        };
        let len = self.source.len();
        let Some(from) = self.lead.key.as_ref().and_then(|key| self.index_of(key)) else { return };
        let takes = |row: &usize| self.source.enabled(*row) && !self.source.is_section(*row) && layout.shown(*row);
        let to = (1..len).map(|step| if forward { (from + step) % len } else { (from + len - step) % len }).find(takes);
        if let Some(to) = to {
            self.move_lead(to, LeadSource::Keys);
            self.pick(comp, to, false, false);
            self.reveal_with(layout, to, view);
        }
    }

    /// The nearest row from `at` in the direction that is an item the lead can take, `at` when there is none.
    fn off_section(&self, layout: &C::ItemLayout, at: usize, forward: bool) -> usize {
        let item = |row: &usize| self.source.enabled(*row) && !self.source.is_section(*row) && layout.shown(*row);
        let found = if forward { (at..self.source.len()).find(item) } else { (0..=at).rev().find(item) };
        found.unwrap_or(at)
    }

    /// Forward and Back on the lead section `at` open and close it when it has children, the tree rule; whether or not
    /// they do, a plain Forward or Back is the section's and moves nothing else. `true` when the move was taken.
    fn section_move(&mut self, at: usize, mv: NavMove, mods: MoveMods) -> bool {
        let wanted = match mv {
            NavMove::Forward => true,
            NavMove::Back => false,
            _ => return false,
        };
        if mods != MoveMods::PLAIN {
            return false;
        }
        if self.source.has_children(at) && self.source.open(at) != wanted {
            self.events.push(CollectionEvent::Open(OpenChange::new(self.source.key(at), wanted)));
        }
        true
    }

    /// Space and activation at the lead.
    fn intent_actions(&mut self, comp: &C, cx: &mut Cx<'_, C>, props: &CollectionProps, lead_at: Option<usize>, header_held: bool) {
        if !header_held
            && cx.core.intents.take(Intent::Space)
            && let Some(row) = lead_at
        {
            self.space_key(comp, cx, props, row);
        }
        // The pass's `update_inputs` turned an `Activate` intent into the keyed activation.
        let activated = cx.core.intents.take(Intent::Activate) || cx.core.activation.is_some_and(|activation| activation.by_key);
        if activated
            && !header_held
            && let Some(row) = lead_at
        {
            self.events.push(CollectionEvent::Activate(self.source.key(row)));
        }
    }

    /// The response of each part an item asked to open a menu from (`ItemCx::part_opener`): the part's click this pass
    /// and its rect, the rect the press was hit-tested against.
    fn item_openers(&self, ui: &Ui, ctx: &Context, id: Id, enabled: bool, openers: Vec<ItemOpener>) -> Vec<(ItemOpener, egui::Response)> {
        if openers.is_empty() {
            return Vec::new();
        }
        let outcome = crate::primitives::sensing::read_outcome(ctx, ArenaId::new(id));
        openers
            .into_iter()
            .filter_map(|opener| {
                let rect = crate::runtime::pointer::part_bounds(ctx, opener.item, opener.part)?;
                let on_part =
                    |hit: Option<crate::primitives::sensing::Hit>| hit.is_some_and(|hit| hit.owner == opener.item && hit.part == Some(opener.part));
                let view = response::MemberView {
                    hovered: on_part(outcome.hover),
                    pressed: on_part(outcome.target) && outcome.phase.is_some(),
                    clicked: on_part(outcome.activated),
                    pointer: ctx.pointer_latest_pos(),
                    ..response::MemberView::default()
                };
                let response = response::member(ui, sub(opener.item, ("opener", opener.part)), rect, enabled, view);
                Some((opener, response))
            })
            .collect()
    }

    /// The access sub-node of each toggle cell of row `(at, key)`, named by its cell, else by its column's header, and carrying its state,
    /// below the row's node `item`; an AccessKit click on it reaches `toggle_by_access`.
    fn report_toggle_cells(
        &self,
        ctx: &Context,
        (id, item): (Id, Id),
        (at, key): (usize, &C::Key),
        (cells, enabled): (&[(ColumnKey, Rect)], bool),
        env: &crate::runtime::env::EnvView<'_>,
    ) {
        for (column, toggle) in self.header.toggle_columns() {
            let Some((_, rect)) = cells.iter().find(|(held, _)| *held == column) else { continue };
            let Some((name, mixed)) = self.header.cell_name(ctx, item, &column, env) else { continue };
            let mut node =
                access::SubNode::new(TOGGLE_CELL.at(key_hash(&(key, &column))), if toggle.exclusive { Role::RADIO_BUTTON } else { Role::SWITCH });
            node.name = access::Name::Text(name);
            node.flags.set(crate::style::Flags::CHECKED, self.source.toggled(at, &column));
            node.flags.set(crate::style::Flags::MIXED, mixed);
            let decl = access::SubDecl {
                owner: id,
                component: "",
                node: &node,
                rect: *rect,
                enabled: enabled && self.source.enabled(at) && self.toggle_open(at, &column),
                focusable: false,
                clickable: true,
                active: false,
                face: None,
                label: "",
                parent: item,
            };
            access::report_sub(ctx, &decl);
        }
    }

    /// The collection's node for the section row `at` that the component does not lay out itself: a heading named by
    /// the row's text, expandable when it has children, that serves Click, Expand and Collapse.
    fn report_section_node(&self, ctx: &Context, id: Id, at: usize, rect: Rect, enabled: bool) {
        let mut node = access::SubNode::new(section_part(&self.source.key(at)), Role::HEADING);
        node.name = access::Name::Text(self.source.text(at).map(|text| text.into_owned()).unwrap_or_default());
        node.level(self.source.depth(at) + 1);
        if self.source.has_children(at) {
            node.expanded = Some(self.source.open(at));
        }
        let decl = access::SubDecl {
            owner: id,
            component: "",
            node: &node,
            rect,
            enabled: enabled && self.source.enabled(at),
            focusable: false,
            clickable: true,
            active: false,
            face: None,
            label: "",
            parent: id,
        };
        access::report_sub(ctx, &decl);
    }

    /// AccessKit requests on an item row's node or on a section row's collection node (`report_section_node`): Focus
    /// moves the lead to the row and key focus to the collection; Click does that and picks an item or activates a
    /// section, as a press does; Expand and Collapse open and close a row that has children.
    fn node_by_access(&mut self, comp: &C, cx: &mut Cx<'_, C>, layout: &C::ItemLayout, place: Placement) {
        use craft_behavior::AccessAction;
        let (_, built) = self.built(layout, place.rect, place.offset);
        let rows: Vec<(u64, usize)> = built
            .into_iter()
            .filter(|&at| !self.source.section_stop(at) || cx.part_if_declared(section_part(&self.source.key(at))).is_none())
            .map(|at| (key_hash(&self.source.key(at)), at))
            .collect();
        let wanted = |request: &craft_behavior::AccessRequest| {
            request.target.is_some_and(|part| {
                (part.key == ITEM_NODE || part.key == SECTION) && part.index.is_some_and(|hash| rows.iter().any(|(held, _)| *held == hash))
            })
        };
        while let Some(request) = cx.core.intents.take_access_where(wanted) {
            let Some(at) = request.target.and_then(|part| part.index).and_then(|hash| rows.iter().find(|(held, _)| *held == hash)).map(|(_, at)| *at)
            else {
                continue;
            };
            if !self.source.enabled(at) {
                continue;
            }
            let key = self.source.key(at);
            match request.action {
                AccessAction::Focus | AccessAction::Click => {
                    if self.lead.key.as_ref() != Some(&key) {
                        self.move_lead(at, LeadSource::Keys);
                    }
                    focus::request_now(&cx.core.ctx, focus::FocusTarget::Id(cx.core.id));
                    if request.action == AccessAction::Click {
                        if self.source.section_stop(at) {
                            self.events.push(CollectionEvent::Activate(key));
                        } else {
                            self.pick(comp, at, false, false);
                        }
                    }
                }
                AccessAction::Expand | AccessAction::Collapse => {
                    let open = request.action == AccessAction::Expand;
                    if self.source.has_children(at) && self.source.open(at) != open {
                        self.events.push(CollectionEvent::Open(OpenChange::new(key, open)));
                    }
                }
                _ => {}
            }
        }
    }

    /// An AccessKit click on a toggle cell's sub-node flips that cell, as a click on it does.
    fn toggle_by_access(&mut self, cx: &mut Cx<'_, C>, layout: &C::ItemLayout, place: Placement) {
        let wanted = |request: &craft_behavior::AccessRequest| {
            request.action == craft_behavior::AccessAction::Click && request.target.is_some_and(|part| part.key == TOGGLE_CELL)
        };
        while let Some(request) = cx.core.intents.take_access_where(wanted) {
            let Some(part) = request.target else { continue };
            let (_, built) = self.built(layout, place.rect, place.offset);
            let found = built.into_iter().find_map(|at| {
                let key = self.source.key(at);
                self.header
                    .toggle_columns()
                    .find(|(column, _)| TOGGLE_CELL.at(key_hash(&(&key, column))) == part)
                    .map(|(column, toggle)| (at, column, toggle))
            });
            if let Some((at, column, toggle)) = found {
                self.flip_toggle(cx, at, column, toggle);
            }
        }
    }

    /// The context-menu key at the lead: the event's request is at the row's bottom start corner, and the context
    /// request a `Trigger::Context` opener reads (9.2 rule 3) is the row's rect. With no lead the intent stays
    /// untaken, so the collection's own context request covers it, as a secondary release on its empty area does.
    fn context_key(&mut self, cx: &mut Cx<'_, C>, layout: &C::ItemLayout, at: Placement, lead_at: Option<usize>) {
        if let Some(row) = lead_at
            && cx.core.intents.take(Intent::ContextMenu)
        {
            let rect = self.screen(layout, at.rect, row, at.offset, at.rtl);
            crate::runtime::keys::request_context(&cx.core.ctx, cx.core.id, rect);
            let place = self.on_screen(rect.left_bottom());
            self.events.push(CollectionEvent::ContextMenu(ContextMenuRequest { key: self.source.key(row), at: place, column: None }));
        }
    }

    /// Delete and Backspace at the lead of a `deletable` collection: the request names the selected items when the lead
    /// is one of them, else the lead alone. The keys stay untaken (shortcut candidates) in a collection that is not
    /// `deletable`.
    fn delete_key(&mut self, cx: &mut Cx<'_, C>, props: &CollectionProps, lead_at: Option<usize>) {
        if !props.deletable {
            return;
        }
        let forward = cx.core.intents.take_exact(&Intent::delete(true, Granularity::Unit));
        let back = cx.core.intents.take_exact(&Intent::delete(false, Granularity::Unit));
        let Some(row) = lead_at.filter(|_| forward || back) else { return };
        let lead = self.source.key(row);
        let keys = if self.holds(row) { self.selected_keys(&self.selection) } else { vec![lead] };
        self.events.push(CollectionEvent::Delete(keys));
    }

    /// Space on the lead row `row`, as `CollectionProps.space` says.
    fn space_key(&mut self, comp: &C, cx: &mut Cx<'_, C>, props: &CollectionProps, row: usize) {
        match props.space {
            SpaceAction::Activate => match Self::select_mode(comp) {
                SelectMode::None => self.events.push(CollectionEvent::Activate(self.source.key(row))),
                mode => self.pick(comp, row, false, mode != SelectMode::Single),
            },
            SpaceAction::ToggleCell => self.toggle_by_key(cx, row),
            SpaceAction::None => {}
        }
    }

    /// Flips the first enabled toggle cell of row `row`; when the row is selected, the same cell of every selected row
    /// follows to the value the row took, as a sweep would.
    fn toggle_by_key(&mut self, cx: &mut Cx<'_, C>, row: usize) {
        let Some((column, toggle)) = self.header.toggle_columns().find(|(column, _)| self.source.toggle_enabled(row, column)) else { return };
        let selected = self.holds(row);
        let others: Vec<usize> =
            if selected && !toggle.exclusive { self.placed().rows().into_iter().filter(|&at| at != row).collect() } else { Vec::new() };
        let Some(on) = self.flip_toggle(cx, row, column.clone(), toggle) else { return };
        for at in others {
            if self.source.enabled(at)
                && !self.source.is_section(at)
                && self.source.toggle_enabled(at, &column)
                && self.source.toggled(at, &column) != on
            {
                self.flip_toggle(cx, at, column.clone(), toggle);
            }
        }
    }

    /// Alt+arrow moves the lead's row; the rename command starts an edit of its name.
    fn reorder_and_rename(&mut self, comp: &C, cx: &mut Cx<'_, C>, props: &CollectionProps, lead_at: Option<usize>) {
        if let Some(forward) = crate::contract::reorder::take_move_item(&mut cx.core.intents)
            && props.reorder.is_some()
            && let Some(at) = lead_at
            && !self.source.section_stop(at)
            && comp.can_drag(&self.source.key(at))
        {
            self.nudge_reorder(cx, props, at, forward);
        }
        if props.rename
            && cx.core.intents.take(Intent::Rename)
            && let Some(at) = lead_at
            && !self.source.section_stop(at)
            && comp.can_rename(&self.source.key(at))
        {
            self.begin_rename(cx, at);
        }
    }

    /// Opens the rename field of the item at `at`.
    fn begin_rename(&mut self, cx: &mut Cx<'_, C>, at: usize) {
        let key = self.source.key(at);
        let draft = self.source.text(at).map(|text| text.into_owned()).unwrap_or_default();
        let gesture = crate::runtime::edit::next_gesture_id(&cx.core.ctx);
        let before = Rename { key: key.clone(), name: draft.clone() };
        self.events.push(CollectionEvent::Rename(EditEvent::begin(gesture, Mixed::Same(before), cx.core.label_arc(), None)));
        self.renaming = Renaming { key: Some(key), draft, gesture, focused: false };
    }

    /// A rename requested from outside (`request_rename`): the lead moves to the item, the item is revealed and its
    /// rename field opens.
    fn requested_rename(&mut self, comp: &C, cx: &mut Cx<'_, C>, props: &CollectionProps, layout: &C::ItemLayout, view: f32) {
        let Some(key) = request::take_rename::<C::Key>(&cx.core.ctx, cx.core.id) else { return };
        if let Some(at) = self.index_of(&key)
            && !self.source.section_stop(at)
            && props.rename
            && comp.can_rename(&key)
        {
            self.move_lead(at, LeadSource::Keys);
            self.reveal_with(layout, at, view);
            self.begin_rename(cx, at);
        }
    }

    /// The item the collection was asked to reveal (`Collection::reveal`, a one-shot request of the pass): the lead
    /// moves to it and it scrolls into view.
    fn requested_reveal(&mut self, comp: &C, layout: &C::ItemLayout, view: f32) {
        if let Some(at) = comp.reveal().and_then(|key| self.index_of(key)).filter(|&at| !self.source.section_stop(at)) {
            self.move_lead(at, LeadSource::Keys);
            self.reveal_with(layout, at, view);
        }
    }

    /// Typed text moves the lead to the next row whose label starts with it.
    fn type_ahead(&mut self, comp: &C, cx: &mut Cx<'_, C>, layout: &C::ItemLayout, view: f32, lead_at: Option<usize>) {
        let (ctx, id, len) = (cx.core.ctx.clone(), cx.core.id, self.source.len());
        while let Some(text) = cx.core.intents.take_text() {
            let now = frame::clock(&ctx);
            let mut typing = cx.core.memory.session::<Typing>(sub(id, "typing")).0.clone();
            let mut prefix = String::new();
            for letter in text.chars() {
                prefix = typing.push(letter, now).to_owned();
            }
            cx.core.memory.session::<Typing>(sub(id, "typing")).0 = typing;
            let source = self.source;
            if let Some(to) = TypeAhead::pick(&prefix, lead_at, len, |at| source.text(at).unwrap_or_default()) {
                self.move_lead(to, LeadSource::Keys);
                self.pick(comp, to, false, false);
                self.reveal_with(layout, to, view);
            }
        }
    }

    /// Brings row `at` into view: the collection's own scroll moves, or, without one, the page's scroll owner is asked
    /// at the end of the item step (`page_reveal`).
    fn reveal_with(&mut self, layout: &C::ItemLayout, at: usize, view: f32) {
        if !layout.shown(at) {
            return;
        }
        if !self.scrolls {
            self.page_reveal = Some(at);
            return;
        }
        let rect = layout.rect(at, ScrollPos { x: 0.0, y: self.offset });
        if rect.min.y < 0.0 {
            self.offset += f64::from(rect.min.y);
        } else if rect.max.y > view {
            self.offset += f64::from(rect.max.y - view);
        }
        self.offset = self.offset.clamp(0.0, self.max_offset(view));
    }

    /// The screen rect of row `at`: logical layout coordinates mirrored under right-to-left, then moved to the view.
    fn screen(&self, layout: &C::ItemLayout, view: Rect, at: usize, offset: ScrollPos, rtl: bool) -> Rect {
        self.to_screen(layout, view, layout.rect(at, offset), rtl)
    }

    /// A rect of the layout's logical, viewport-relative coordinates on the screen.
    fn to_screen(&self, layout: &C::ItemLayout, view: Rect, local: Rect, rtl: bool) -> Rect {
        let local = if rtl && layout.mirrors() { Flow::new(Rect::from_min_size(Pos2::ZERO, view.size()), true).to_physical(local) } else { local };
        local.translate(view.min.to_vec2())
    }

    /// The layout's grid view on the screen: sections placed in the view and mirrored under right-to-left.
    fn grid_screen(&self, layout: &C::ItemLayout, grid: &craft_behavior::GridView, view: Rect, rtl: bool) -> craft_behavior::GridView {
        let mirrored = rtl && layout.mirrors();
        let edge = |x: f32| if mirrored { view.max.x - x } else { view.min.x + x };
        craft_behavior::GridView::new(grid.sections.iter().map(|section| {
            let edges = section.edges.iter().map(|&x| edge(x)).collect();
            craft_behavior::GridSection::new(self.to_screen(layout, view, section.rect, rtl), section.columns.clone(), edges)
        }))
    }
}

fn key_hash<K: std::hash::Hash>(key: &K) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

impl<C: Collection> Cx<'_, C> {
    /// The selection the item step holds this pass: the app's binding when it has one, else the one the item step
    /// keeps. The collection's own `update` reads it as the pass began; its `finish` reads it with the pass's
    /// selection changes applied.
    pub fn collection_selection(&self) -> Option<&C::Selection> {
        self.core.collection_selection.as_ref().and_then(|held| held.downcast_ref::<C::Selection>())
    }

    /// The output of a collection: its response, every event in drawing order and the positions it showed.
    pub fn collection_response(&mut self) -> CollectionResponse<C::Key, C::ItemEvent> {
        let events = self
            .core
            .collection_events
            .take()
            .and_then(|held| held.downcast::<(Vec<CollectionEvent<C::Key, C::ItemEvent>>, Vec<ShownRow<C::Key>>, u8)>().ok())
            .map(|held| *held)
            .unwrap_or_default();
        CollectionResponse::new(self.core.response.clone(), events, self.core.visible_rows.clone(), self.core.gestures)
    }
}
