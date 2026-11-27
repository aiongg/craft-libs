//! The item step's hooks: the collection driver's calls into a collection's rows (`run::show_items`).
//!
//! The driver runs them in this order on a run of the collection's node: `read` (step 4), `extent` (step 5, also in a
//! measuring run, with the `Ui` of the build it runs in), `sized`, `geometry`, `parts` (step 7), `sensed` (8),
//! `updated` (9), `apply` (11), `paint` (before the component's own paint, 13), `content` (13, whenever the
//! collection's scope build reaches it or its rows view is marked, with the `Ui` of the collection node's build) and
//! `settle` (15). `sense`, `key_role`, `sized` and `first_line` are facts the driver
//! asks when it needs them.

use super::drag::Drags;
use super::header::HeaderMemory;
use super::measure::{RowSizes, collect_cells};
use super::members::Member;
use super::{Held, ItemsHooks, LastLayout, Lead, Placement, Renaming, Rows, Source, key_hash};
use super::{request, rows};
use crate::contract::ValueOf;
use crate::contract::collection::{
    CollectionEvent, CollectionProps, ColumnLayout, DropAt, Gestures, HoverLead, ItemsOut, PendingDrop, Rename, RowState, ShownRow,
};
use crate::contract::cx::BodyCx;
use crate::contract::rt::items::{RowFacts, RowsView, rows_view};
use crate::contract::run::{Measuring, ShapeHooks};
use crate::contract::{
    Bind, Collection, Context, Cx, Internal, ItemCx, ItemOutput as _, PaintUi, PartsCx, Props, Scroll, Show as _, Sizing, Ui, UiBuilder,
};
use crate::primitives::Id;
use crate::runtime::access::Role;
use crate::runtime::env::{AppValues, Env, ItemFacts};
use crate::runtime::focus;
use crate::runtime::memory::{MemoryStage, sub};
use crate::style::ComponentStyle as _;
use craft_behavior::{Axis, CancelReason, EditEvent, Intent, ItemLayout, Mixed, Proposal, ScrollPos, SelectionModel, Sense};
use craft_keymap::Chord;
use craft_ui_rt::{Pos2, Rect, Vec2};
use std::rc::Rc;
use std::sync::Arc;

impl<C: Collection> ShapeHooks<C> for ItemsHooks<'_, '_, C> {
    /// A collection that keeps focus on press senses the press without taking focus.
    fn sense(&self, comp: &C, own: Sense) -> Sense {
        if comp.items().focus_on_press { own } else { own.difference(Sense::TAKE_FOCUS).union(Sense::KEEP_FOCUS) }
    }

    fn key_role(&self, comp: &C) -> Option<craft_behavior::KeyRole> {
        let props = comp.items();
        let role = craft_behavior::KeyRole::new()
            .reorderable(props.reorder.map(|rule| rule.keys))
            .reorder_axis(props.overflow)
            .pages(props.pages)
            .rename(props.rename)
            .type_ahead(props.type_ahead)
            .digits(props.digit_keys)
            .space(true)
            .multi_select(matches!(props.select, crate::contract::collection::SelectMode::Multi | crate::contract::collection::SelectMode::Cells))
            .moves(true)
            .deletes(props.deletable)
            .owner_arrows(props.item_keys == crate::contract::collection::ItemKeys::Owner)
            .stacked(self.stacked(comp))
            .home_row(self.layout.as_ref().map_or(0, |layout| layout.first_navigable()));
        Some(role)
    }

    /// Step 4. `stage` is the collection node's memory: in a measuring run the driver lends the kept node's memory
    /// when the node exists, so its kept row measures serve.
    fn read(&mut self, comp: &mut C, stage: &mut MemoryStage, id: Id, env: &Env) {
        let ctx = stage.ctx().clone();
        self.sizes = RowSizes::read(stage, id);
        let natural = comp.items().natural_height;
        let overflow = comp.items().overflow;
        self.capped = comp.items().max_rows.is_some();
        debug_assert!(
            !shares_scroll(comp.items(), env) || natural || self.capped,
            "a collection owns its scroll: show it outside a body that scrolls along its rows, cap its rows, or give it the natural height (16.3)"
        );
        self.scrolls = scrolls_rows(comp.items(), env);
        self.own_wheel = crate::contract::Component::sense(comp).intersects(Sense::WHEELS);
        self.overflow = overflow;
        self.selection = match comp.selection() {
            Some(bind) => match bind.get() {
                Some(Mixed::Same(held)) => held,
                _ => C::Selection::default(),
            },
            None => stage.session::<Held<C::Selection>>(sub(id, "selection")).0.clone(),
        };
        self.selection_at_read = self.selection.clone();
        self.renaming = stage.session::<Renaming<C::Key>>(sub(id, "rename")).clone();
        // The pass walks the collection's view of the source. The view keeps the row whose rename runs, or is asked
        // for, whether or not it matches, so the row is named in place.
        let kept = self.renaming.key.clone().or_else(|| request::peek_rename::<C::Key>(&ctx, id));
        if let Source::Given(given) = self.source
            && let Some(view) = comp.filtered(given, env, kept.as_ref())
        {
            self.source = Source::Filtered(view);
        }
        self.previous = stage.session::<LastLayout<C::ItemLayout>>(sub(id, "item_layout")).0.take();
        let bound = comp.column_layout().map(|bind| match bind.get() {
            Some(Mixed::Same(layout)) => layout,
            _ => ColumnLayout::default(),
        });
        self.column_layout_at_read = bound.clone();
        let hosted: Vec<_> = comp.items().columns.iter().map(|spec| spec.key.clone()).filter(|key| comp.hosts_header(key)).collect();
        self.header.read(stage, id, comp.items(), bound, hosted);
        self.drags = stage.session::<Drags<C::Key>>(sub(id, "drags")).clone();
        self.lead = stage.session::<Lead<C::Key>>(sub(id, "lead")).clone();
        // A collection in a layer starts from `lead_on_open` at every opening of the layer: a closed layer drops its
        // nodes, and their memory with them.
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
        (self.lead_at_read, self.renaming_at_read) = (self.lead.key.clone(), self.renaming.key.clone());
        // The rows view holds the offset (`rt/items.rs`): step 5 reads it from the collection's build; a bound
        // offset is the app's.
        let bound = comp.scroll().and_then(|bind| match bind.get() {
            Some(Mixed::Same(pos)) => Some(pos),
            _ => None,
        });
        self.scroll_bound = bound.is_some();
        let bound = bound.filter(|_| self.scrolls).unwrap_or_default();
        (self.offset, self.offset_x) = (bound.y, bound.x);
        self.offset_at_read = self.offset;
        self.offset_x_at_read = self.offset_x;
    }

    /// Step 5 in both runs: what the rows take in the room `at` gives.
    fn pass_extent(&mut self, comp: &mut C, at: &Measuring<'_, '_>, _style: &C::Style, _shown: Option<Arc<ValueOf<C>>>) -> Vec2 {
        let id = at.resolved.id;
        self.record_largest(comp, at, id);
        self.items_extent(comp, at, id)
    }

    fn sized(&self, comp: &C, size: Vec2) -> Vec2 {
        let sizing = comp.props(Internal::new()).sizing;
        // A collection that measured its width from its content takes that width; the column it sits in holds it.
        // Otherwise it fills the width its room gives.
        let width = if self.content_sized(comp) || self.from_content.0 || !self.room.x.is_finite() { size.x } else { self.room.x };
        let height = match (self.scrolls && self.room.y.is_finite(), sizing) {
            (true, Sizing::Fill | Sizing::FillHeight) if !self.from_content.1 && !self.capped => self.room.y,
            (true, Sizing::Fill | Sizing::FillHeight) => size.y.min(self.room.y),
            (true, Sizing::Content | Sizing::FillWidth) => size.y.min(self.room.y),
            (false, _) => size.y,
        };
        Vec2::new(width, height)
    }

    /// The surface the props name, under the header and the rows, as a container's lies under its regions.
    fn paint(&self, comp: &C, ui: &mut PaintUi<'_>, rect: Rect) {
        if let Some(kind) = comp.props(Internal::new()).surface {
            ui.draw().surface(rect, kind, crate::style::Corners::All);
        }
    }

    /// The member rows' looks this run recorded; empty for a collection of band rows.
    fn take_member_looks(&mut self) -> Vec<crate::contract::rt::items::MemberLooks> {
        std::mem::take(&mut self.member_looks)
    }

    /// A collection whose rows run along X is one line; one whose rows run along Y has no line of its own.
    fn first_line(&self, comp: &C, height: f32) -> Option<f32> {
        (comp.items().overflow == Axis::X).then_some(height / 2.0)
    }

    fn geometry(&mut self, _: &C, _mcx: &crate::contract::MeasureCx<'_, C>, (_, content): (Rect, Rect), _rtl: bool) -> Option<Rc<dyn std::any::Any>> {
        let view = RowsView::new(self.below_header(content), ScrollPos { x: self.offset_x, y: self.offset }, self.scrolls);
        let layout = self.layout.take()?;
        let mut band = Vec::new();
        view.band_rows(&*layout, &mut band);
        let (_, order) = self.held_rows(&layout, band);
        let rects: Vec<(C::Key, Rect)> = order.iter().map(|&at| (self.source.key(at), view.row_rect(&*layout, at))).collect();
        self.row_parts.clear();
        for (&at, (key, rect)) in order.iter().zip(&rects) {
            let shown = rect.intersect(view.rect);
            if C::MEMBERS && !self.source.section_stop(at) && shown.is_positive() {
                self.row_parts.push((key_hash(key), at, shown));
            }
        }
        self.layout = Some(layout);
        Some(Rc::new(Rows { rects, offset: view.offset, view: view.rect }))
    }

    /// The header's parts, the edge band and the member rows' parts. The rows' scroll bar is the rows view's.
    fn parts(&mut self, comp: &mut C, _: Rect, pcx: &mut PartsCx<'_, C>) {
        self.header.parts(pcx, self.offset_x as f32);
        self.edge_parts(comp, pcx);
        self.row_parts(pcx);
        self.notch = comp.items().wheel_notch(pcx.tokens.typography.body.line_height);
    }

    /// The rows' wheel is the rows view's (`rt/items.rs`): a wheel step moves the content node in layout and runs
    /// nothing. A collection that senses the wheel itself (a tab strip stepping its tabs) takes it on its own node.
    fn sensed(&mut self, _comp: &mut C, cx: &mut Cx<'_, C>) {
        cx.core.collection_selection = Some(Box::new(self.selection.clone()));
    }

    fn updated(&mut self, comp: &mut C, cx: &mut Cx<'_, C>) {
        let rect = self.rows_rect(cx);
        let view = rect.height();
        let rtl = cx.core.flow.is_rtl();
        self.offset = self.offset.clamp(0.0, self.max_offset(view));
        let header_held = self.header_step(cx);
        self.offset_x = self.offset_x.clamp(0.0, f64::from((self.content.x - rect.width()).max(0.0)));
        self.open_lead(comp);
        self.follow_lead_source(comp);
        // A lead set in place (from the selection, `open_lead`, a row that left) has not rested yet: a lead that comes
        // back to a row it rested on before reports the rest again.
        if self.lead.rested != self.lead.key {
            self.lead.rested = None;
        }
        let lead_at = self.lead.key.as_ref().and_then(|key| self.index_of(key));
        let props = comp.items().clone();
        let Some(layout) = self.layout.take() else { return };
        let offset = ScrollPos { x: self.offset_x, y: self.offset };
        let at = Placement { rect, offset, rtl };
        // A press lands on the rows the user saw: the rows view's kept offset, which only layout moves.
        self.edge_step(cx);
        self.press_step(comp, cx, &props, &layout, at);
        if props.select_under_drag {
            self.drag_select(comp, cx, &layout, at);
        }
        self.drag_step(comp, cx, &layout, (rect, offset, rtl), &props);
        self.marquee_step(comp, cx, &layout, (rect, offset, rtl), &props);
        if props.hover_lead != HoverLead::Off {
            self.hover_lead(comp, cx, &layout, props.hover_lead);
        }
        self.navigate(comp, cx, &layout, view, header_held);
        self.switch_page(comp, cx, &props, &layout);
        let lead_at = self.lead.key.as_ref().and_then(|key| self.index_of(key)).or(lead_at);
        if matches!(props.select, crate::contract::SelectMode::Multi | crate::contract::SelectMode::Cells)
            && !header_held
            && self.renaming.key.is_none()
            && cx.core.intents.untaken(&Intent::Cancel)
            && !self.placed().rows().is_empty()
            && cx.core.intents.take(Intent::Cancel)
        {
            self.change(craft_behavior::SelectionChange::Clear);
        }
        self.intent_actions(comp, cx, &props, lead_at, header_held);
        self.context_key(cx, &layout, at, lead_at);
        self.delete_key(cx, &props, lead_at);
        self.toggle_by_access(cx, &layout, at);
        self.node_by_access(comp, cx, &props, &layout, at);
        self.reorder_and_rename(comp, cx, &props, lead_at);
        self.requested_rename(comp, cx, &props, &layout);
        self.requested_reveal(comp, &layout);
        self.type_ahead(comp, cx, &layout, lead_at);
        self.rest_lead(cx, &layout);
        request::record_lead(cx.context(), cx.core.id, self.lead.key.as_ref().and_then(|key| self.index_of(key)));
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
    }

    /// Step 13 on rt (docs/contract-rt.md 15.5): the band rows as nodes in the rows view, or the member rows as parts
    /// of the collection's node; the header, the empty state and the marquee. It runs whenever the collection's scope
    /// build reaches it, as a container's body does, so the item fn sees the app's rows; a row whose key and facts
    /// hold keeps its node and runs nothing.
    fn content(&mut self, comp: &mut C, ui: &mut Ui, env: &Env, cx: &mut Cx<'_, C>, values: &AppValues) {
        let Some(mut layout) = self.layout.take() else { return };
        let (id, ctx) = (cx.core.id, ui.ctx().clone());
        let rect = self.rows_rect(cx);
        let surface = comp.props(Internal::new()).surface;
        let rtl = env.direction.is_rtl();
        // The rows sit on the surface the collection paints, else on the one the collection sits on.
        let own = surface.or_else(|| cx.style.surface().and_then(|look| look.kind_in(&cx.core.state)));
        let base_env = Self::rows_env(comp, env, id, cx.core.resolved.size, own).derive(|scoped| scoped.extend_values(values));
        let known = self.known(comp, false);
        let view = RowsView::new(rect, ScrollPos { x: self.offset_x, y: self.offset }, self.scrolls);
        // A key move, a rename request or `Collection::reveal` brings its row into view by the rows view's glide,
        // aimed from the kept extents.
        let reveal = self.revealing.take().and_then(|at| view.reveal(&*layout, at));
        let sheet = Self::selects_cells(comp);
        let columns: Vec<u64> = match layout.grid() {
            Some(grid) => grid.sections.iter().flat_map(|section| section.columns.clone().map(u64::from)).collect(),
            None => (0..layout.column_count() as u64).collect(),
        };
        let grid = layout.grid().map(|grid| self.grid_screen(&layout, grid, view.rect, rtl));
        let mut band = Vec::new();
        view.band_rows(&*layout, &mut band);
        // The rows a gesture holds (the lead, the anchor, the renamed and the grabbed row) are built though out of the
        // band: their key handling and editor live on their nodes.
        let (_, order) = self.held_rows(&layout, band);
        self.visible = order.iter().copied().min().unwrap_or(0)..order.iter().copied().max().map_or(0, |last| last + 1);
        self.header.paint(ui, cx, comp, env, id, rtl);
        let len = self.source.len();
        let tree = matches!(layout.nav(), craft_behavior::NavShape::Tree);
        let focused = cx.core.focus.focused;
        let striped = comp.items().striped;
        let mut stripe: Option<(usize, usize)> = None;
        let placed = known.placed.clone();
        let menu_open = self.drags.menu.is_some();
        let fit = self.header.fit.as_ref().map(|(column, _)| column.clone());
        let mut fit_widest = 0.0_f32;
        let aligns = self.header.aligns();
        let mut shown_rows: Vec<ShownRow<C::Key>> = Vec::with_capacity(order.len());
        let mut members = Vec::new();
        let mut moved = false;
        let size = layout.content_size();
        let extent = Vec2::new(size.x as f32, size.y as f32);
        // The rows view scrolls along the axis the rows overflow; a sheet's sideways scroll is shared with its header.
        let scroll = self.scrolls.then(|| if self.overflow == Axis::X { Scroll::horizontal() } else { Scroll::vertical() });
        // A bound offset is the app's, and a drag's edge scroll moved the offset in this run: both set the rows view's.
        let jump = (self.scrolls && (self.scroll_bound || self.drags.edge_at.is_some())).then(|| Vec2::new(self.offset_x as f32, self.offset as f32));
        let mut place_rows = |ui: &mut Ui, facts: Option<&RowFacts>| {
            for at in order {
                if self.source.section_stop(at) {
                    continue;
                }
                let key = self.source.key(at);
                let item_id = sub(id, ("key", key_hash(&key)));
                let row = view.row_rect(&*layout, at);
                let odd = striped && {
                    let run = self.stripe_run(stripe, at);
                    stripe = Some((at, run));
                    run.is_multiple_of(2) && !self.source.is_section(at)
                };
                let (known_state, mut flags) = self.known_row(at, &known, odd);
                // Hover is the row's own `Hovered` fact, read in its paint: never a row fact, so a hover runs nothing.
                let state = RowState {
                    dragging: self.drags.grabbed.as_ref() == Some(&key),
                    drop_target: matches!(&self.drags.pending, Some(PendingDrop::Drop(DropAt::Onto(over))) if *over == key),
                    pressed: self.pressed.as_ref().filter(|(held, _)| *held == at).map(|(_, pressed)| pressed.clone()),
                    menu_open: self.drags.menu.as_ref().is_some_and(|request| request.key == key) && menu_open,
                    activated_by_key: self.keyed_activation.as_ref() == Some(&key),
                    ..known_state
                };
                // A list that never takes key focus (a popup list: the host forwards its keys) shows its lead always.
                flags.set(crate::style::Flags::LEAD, state.lead && (focused || !comp.items().focus_on_press));
                flags.set(crate::style::Flags::DRAGGING, state.dragging);
                flags.set(crate::style::Flags::DROP_TARGET, state.drop_target);
                let position = self.row_position(at, columns.len(), tree);
                let mut cells = Vec::new();
                layout.cells(at, ScrollPos::default(), &mut cells);
                for (_, cell) in &mut cells {
                    *cell = cell.translate(view.content_origin().to_vec2());
                }
                if sheet && let Some(facts) = facts {
                    facts.put_cells(ui.build_cx(), item_id, self.cell_flag_list(&placed, at, &columns, focused));
                }
                let column_keys: Vec<craft_behavior::ColumnKey> = cells.iter().map(|(key, _)| key.clone()).collect();
                let shown_cells = cells.clone();
                let toggle_keys: Vec<craft_behavior::ColumnKey> = self.header.toggle_columns().map(|(key, _)| key).collect();
                let closed_toggles: Vec<craft_behavior::ColumnKey> = toggle_keys.iter().filter(|key| !self.toggle_open(at, key)).cloned().collect();
                let mut icx = ItemCx {
                    key: &key,
                    at,
                    rect: row,
                    row: state.clone(),
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
                    detail: None,
                    openers: Vec::new(),
                    item: item_id,
                    memory: &mut *cx.core.memory,
                };
                let item = (self.item)(&key, &mut icx);
                let mut item = comp.dress(item, &state);
                let tip = icx.tip.take();
                let detail = icx.take_detail(layout.is_auto(at));
                let openers = std::mem::take(&mut icx.openers);
                drop(icx);
                item = Props::id(item, item_id);
                let row_facts = ItemFacts {
                    item: item_id,
                    flags,
                    position,
                    columns: column_keys,
                    toggles: toggle_keys,
                    toggle_keys: self.header.toggle_keys(),
                    closed_toggles,
                    aligns: aligns.clone(),
                    pressed: state.pressed.clone(),
                    menu_open: state.menu_open,
                };
                // An auto row takes the height its measure gives at the row's width; a kept row answers from its size
                // cache and runs nothing. A height other than the layout's moves the rows after it: the collection
                // builds again in this frame, laid out by the new heights.
                let mut row = row;
                let hash = key_hash(&key);
                if layout.auto_widths() {
                    let widths = Vec2::new(item.measure_in(ui, Proposal::Ideal).x, item.measure_in(ui, Proposal::Min).x);
                    self.sizes.keep_shown_widths(hash, widths);
                    Arc::make_mut(&mut layout).set_width(at, widths.x, widths.y);
                    let wide = view.row_rect(&*layout, at);
                    moved |= wide != row;
                    row = wide;
                }
                if layout.is_auto(at) && detail.is_none() {
                    let measured = item.measure_in(ui, Proposal::Width(row.width())).y;
                    self.sizes.keep_shown_height(hash, Vec2::new(row.width(), measured));
                    let fit = rows::fits(&item).then_some(row.width());
                    if self.sizes.keep_shown_fit(hash, fit) {
                        Arc::make_mut(&mut layout).set_fit(at, fit);
                    }
                    if measured != row.height() {
                        let held = Arc::make_mut(&mut layout);
                        held.set_height(at, measured);
                        moved = true;
                        row = view.row_rect(&*layout, at);
                    }
                }
                if let Some(column) = &fit {
                    fit_widest = fit_widest.max(self.cell_width(&mut item, ui, hash, column).unwrap_or_default());
                }
                // A member has no node of its own: it reports as a sub-node of the collection's.
                let row_node = if C::MEMBERS { super::members::item_node(id, &key) } else { item_id };
                shown_rows.push(ShownRow::new(key.clone(), row_node, row, shown_cells.clone()));
                let Some(facts) = facts else {
                    debug_assert!(tip.is_none() && detail.is_none(), "a member of a collection's run takes no rich tooltip and no detail body");
                    let shown = shown_rows.len() - 1;
                    members.push(Member { at, key, item, row, clip: rect, flags, position, renaming: state.renaming, openers, shown });
                    continue;
                };
                facts.put(ui.build_cx(), item_id, row_facts);
                let out = match tip {
                    Some(tip) => item.show_at_with_tip(ui, row, tip),
                    None => item.show_at(ui, row),
                };
                if let Some(body) = detail {
                    // The body's stops follow the leaf's: it shows inside the item's scope, under the leaf.
                    let below = Rect::from_min_size(Pos2::new(row.min.x, row.max.y), Vec2::new(row.width(), f32::INFINITY));
                    let total = ui.scope_builder(UiBuilder::new().max_rect(below).id_salt(sub(item_id, "detail")), |body_ui| {
                        body(body_ui);
                        row.height() + body_ui.min_rect().height()
                    });
                    if total != row.height() {
                        self.sizes.keep_shown_height(hash, Vec2::new(row.width(), total));
                        Arc::make_mut(&mut layout).set_height(at, total);
                        moved = true;
                    }
                }
                self.report_toggle_cells(&ctx, (id, item_id), (at, &key), (shown_cells.as_slice(), cx.core.resolved.enabled), &cx.core.env);
                let label = out.label_rect().unwrap_or(row);
                self.slow_click_label(&key, label);
                if let Some(shown) = shown_rows.last_mut() {
                    shown.openers = self.item_openers(ui, &ctx, id, cx.core.resolved.enabled, openers);
                }
                self.report_item_output(&ctx, id, &key, out);
                if state.renaming {
                    self.rename_row(comp, ui, (&ctx, id, &cx.core), (at, &key), label);
                }
            }
        };
        Env::run_in(ui, base_env.clone(), |ui| {
            if C::MEMBERS {
                place_rows(ui, None);
            } else {
                rows_view(ui, view, (scroll, jump), extent, |ui, facts| {
                    if let Some(at) = reveal {
                        ui.flow.reveal = Some(at);
                    }
                    place_rows(ui, Some(facts))
                });
            }
        });
        if C::MEMBERS {
            Env::run_in(ui, base_env, |ui| self.show_members(comp, ui, cx, ((&ctx, id, rtl), env), members, &mut shown_rows));
        }
        if moved && !ui.is_measuring() {
            ui.build_cx().rebuild();
        }
        if self.header.fit.is_some() {
            self.header.apply_fit(ui, &cx.core.env, fit_widest);
            self.take_column_events();
        }
        self.paint_marquee(ui, env, &layout, (rect, view.offset, rtl));
        self.shown = shown_rows;
        cx.core.grid = crate::contract::GridPlace::grid(len, columns.len());
        self.content = extent;
        // The renamed item left the source (removed, renamed or hidden by the app, or folded away): its editor is
        // gone, so the rename ends rather than waiting for the item to return and open the old draft again.
        if let Some(key) = self.renaming.key.clone()
            && !self.ended_rename
            && !ui.is_measuring()
            && self.index_of(&key).is_none()
        {
            let before = Rename { key, name: self.renaming.draft.clone() };
            self.events.push(CollectionEvent::Rename(EditEvent::cancel(self.renaming.gesture, CancelReason::Removed, Some(Mixed::Same(before)))));
            self.ended_rename = true;
        }
        if self.ended_rename {
            self.renaming = Renaming::default();
            if self.refocus {
                ui.request_focus(focus::FocusTarget::Id(id));
            }
            self.refocus = false;
        }
        if self.source.is_empty() {
            let mut open = true;
            ui.scope_builder(UiBuilder::new().max_rect(rect).id_salt(sub(id, "empty")), |empty_ui| {
                Env::run_in(empty_ui, env.derive(|scoped| scoped.extend_values(values)), |empty_ui| {
                    let (resolved, view) = (cx.core.resolved, cx.core.env);
                    let mut bcx = BodyCx::new(sub(id, "body"), view, resolved, rect, &mut *cx.core.memory, &mut open, None, None);
                    comp.empty(empty_ui, &mut bcx);
                });
            });
        }
        self.layout = Some(layout);
    }

    fn settle(&mut self, comp: &mut C, cx: &mut Cx<'_, C>) {
        let id = cx.core.id;
        // A collection's focus move reveals through its lead (`page_reveal`), never the collection's whole rect. A
        // header cell's reveal moves the sideways offset the header and the rows share, to show the whole cell.
        if let Some(part) = cx.core.focus.part.filter(|_| cx.core.focus.focused)
            && self.scrolls
            && let Some(by) = self.header.reveal_x(part, cx.core.flow.is_rtl())
        {
            let max = f64::from((self.content.x - self.rows_rect(cx).width()).max(0.0));
            self.offset_x = (self.offset_x + by).clamp(0.0, max);
        }
        // A bound offset is the app's: the read took it, and a move of this run goes back through the binding.
        if self.scrolls
            && self.scroll_bound
            && (self.offset != self.offset_at_read || self.offset_x != self.offset_x_at_read)
            && let Some(mut bind) = comp.scroll()
        {
            let (label, doc_key) = (cx.core.label_arc(), cx.core.resolved.doc_key);
            let (before, after) = (ScrollPos { x: self.offset_x_at_read, y: self.offset_at_read }, ScrollPos { x: self.offset_x, y: self.offset });
            bind.apply(&EditEvent::set(Mixed::Same(before), after, label, doc_key));
        }
        *cx.core.memory.session::<Lead<C::Key>>(sub(id, "lead")) = self.lead.clone();
        std::mem::take(&mut self.sizes).keep(cx.core.memory, id);
        cx.core.memory.session::<LastLayout<C::ItemLayout>>(sub(id, "item_layout")).0 = self.layout.clone();
        let requests = std::mem::take(&mut cx.core.column_requests);
        if !requests.is_empty() {
            self.header.apply_requests(cx.context(), requests, cx.core.flow.is_rtl());
            self.take_column_events();
        }
        if let Some(before) = self.column_layout_at_read.take()
            && *self.header.layout() != before
            && let Some(mut bind) = comp.column_layout()
        {
            let (label, doc_key) = (cx.core.label_arc(), cx.core.resolved.doc_key);
            bind.apply(&EditEvent::set(Mixed::Same(before), self.header.layout().clone(), label, doc_key));
        }
        *cx.core.memory.session::<HeaderMemory>(sub(id, "columns")) = self.header.persist();
        // The menu a request of this pass asks for opens after the collection shows, so it counts as open in this
        // pass; an older request goes once no menu hung from the collection is open.
        let asked = self.events.iter().any(|event| matches!(event, CollectionEvent::ContextMenu(_)));
        if !asked && !crate::contract::overlay::container_open::opened_from(cx.context(), id) {
            self.drags.menu = None;
        }
        *cx.core.memory.session::<Drags<C::Key>>(sub(id, "drags")) = self.drags.clone();
        self.header.report(cx.context(), id, cx);
        *cx.core.memory.session::<Renaming<C::Key>>(sub(id, "rename")) = self.renaming.clone();
        cx.core.collection_selection = Some(Box::new(self.selection.clone()));
        cx.core.collection_events = Some(Box::new(ItemsOut {
            events: std::mem::take(&mut self.events),
            shown: std::mem::take(&mut self.shown),
            activation_clicks: self.activation_clicks,
            pending_drop: self.drags.pending.clone(),
            hovered: self.hovered.take(),
            menu: self.drags.menu.clone().filter(|request| self.index_of(&request.key).is_some()),
            auto_scrolling: false,
        }));
        cx.core.visible_rows = self.visible.clone();
        cx.core.gestures = Gestures::of(comp.items());
    }
}

impl<C: Collection> ItemsHooks<'_, '_, C> {
    /// The width `item` (the row `key`) gives its cell in `column` at its ideal width; `None` when it has no cell
    /// there. A row that answers from its size cache reports no cells: the ones it reported when it last ran serve.
    fn cell_width(&mut self, item: &mut impl crate::contract::Show, ui: &mut Ui, key: u64, column: &craft_behavior::ColumnKey) -> Option<f32> {
        let (_, reported) = collect_cells(|| item.measure_in(ui, Proposal::Ideal));
        if let Some(cells) = reported {
            self.sizes.keep_cells(key, cells);
        }
        self.sizes.cells_of(key)?.iter().find(|(held, _)| held == column).map(|(_, width)| *width)
    }

    /// Goes on renaming after a Shift+Enter commit: the field holds the applied name `name`, selected whole when focus
    /// is requested again, and the next commit is its own undo step.
    fn continue_rename(&mut self, core: &crate::contract::CxCore<'_>, name: String) {
        self.renaming.draft = name;
        self.renaming.focused = false;
        self.renaming.proposed = false;
        self.renaming.gesture = core.next_gesture_id();
        self.refocus = false;
    }

    /// The rename editor of the row `(at, key)` being renamed, over its label rect `label`, and the end of the rename
    /// it reports.
    pub(super) fn rename_row(
        &mut self,
        comp: &mut C,
        row_ui: &mut Ui,
        (ctx, id, core): (&Context, Id, &crate::contract::CxCore<'_>),
        (at, key): (usize, &C::Key),
        label: Rect,
    ) {
        let len = self.source.len();
        let mut draft = std::mem::take(&mut self.renaming.draft);
        let name = self.source.text(at).map(|text| text.into_owned()).unwrap_or_default();
        let editor = comp.rename_editor(row_ui, Bind::bound(&mut draft), label, &name);
        self.renaming.draft = draft;
        if let Some(editor) = editor {
            let gesture = self.renaming.gesture;
            let shifted = editor.submitted && editor.submit_mods.shift;
            let before = Mixed::Same(Rename { key: key.clone(), name: name.clone() });
            let (end, escaped) = rename_input(row_ui, &editor, &mut self.renaming.focused, &mut self.refocus);
            // A pass that only measures keeps no focus, so it neither opens nor ends the hold.
            let real = !row_ui.is_measuring();
            let has_focus = row_ui.is_focused(editor.response.id);
            // The field lost the focus with nothing typed (a click elsewhere): the text field reports no event for an
            // unedited name, so the rename ends here, as it does by Tab and as a typed name does by committing.
            let blurred = real && self.renaming.held && !has_focus;
            self.renaming.held |= real && has_focus;
            match end {
                None if !editor.events.is_empty() => {}
                None if escaped => {
                    self.events.push(CollectionEvent::Rename(EditEvent::cancel(gesture, CancelReason::Escape, Some(before))));
                    self.ended_rename = true;
                    self.refocus = true;
                }
                Some(EditEvent::Commit { after, .. }) => {
                    let after = Rename { key: key.clone(), name: after };
                    let applied = after.name.clone();
                    self.events.push(CollectionEvent::Rename(EditEvent::commit(gesture, before, after)));
                    if shifted {
                        self.continue_rename(core, applied);
                    } else {
                        self.ended_rename = true;
                    }
                }
                Some(EditEvent::Cancel { reason, .. }) => {
                    self.events.push(CollectionEvent::Rename(EditEvent::cancel(gesture, reason, Some(before))));
                    self.ended_rename = true;
                }
                // Enter on a proposed draft nobody typed in commits it. Enter on the row's own name opens no edit run: the
                // rename ends with no event and no undo step, as a text field's empty commit does.
                None if editor.submitted => {
                    let applied = self.renaming.draft.clone();
                    if self.renaming.proposed {
                        let after = Rename { key: key.clone(), name: applied.clone() };
                        self.events.push(CollectionEvent::Rename(EditEvent::commit(gesture, before, after)));
                    }
                    if shifted {
                        self.continue_rename(core, applied);
                    } else {
                        self.ended_rename = true;
                        self.refocus = true;
                    }
                }
                None if editor.left_by_tab.is_some() => {
                    self.events.push(CollectionEvent::Rename(EditEvent::cancel(gesture, CancelReason::Blur, Some(before))));
                    self.ended_rename = true;
                }
                None if blurred => {
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
                    request::request_rename(ctx, id, &self.source.key(next));
                }
            }
        }
    }
}

/// The rename editor's end, and whether Escape ended it. The editor is a node of its own: it takes its keys and
/// reports Escape as a cancel (`Component::ESCAPE`), so the collection reads both from its response.
fn rename_input(
    ui: &mut Ui,
    editor: &crate::contract::EditResponse<String>,
    focused: &mut bool,
    refocus: &mut bool,
) -> (Option<EditEvent<String>>, bool) {
    let id = editor.response.id;
    if !*focused {
        ui.request_focus(focus::FocusTarget::Id(id));
        *focused = true;
    }
    let end = editor.events.iter().rev().find(|event| matches!(event, EditEvent::Commit { .. } | EditEvent::Cancel { .. })).cloned();
    let escaped = matches!(end, Some(EditEvent::Cancel { reason: CancelReason::Escape, .. }));
    if end.is_some() {
        *refocus = ui.is_focused(id);
    }
    (end, escaped)
}

/// The height of the first `rows` rows of `layout` over `len` items: the rows it lays out, and the row height `row` for
/// each one it does not have.
pub(super) fn rows_height(layout: &impl ItemLayout, len: usize, rows: u16, row: f32) -> f32 {
    let rows = usize::from(rows);
    let laid = rows.min(len);
    let top = if len == 0 { 0.0 } else { layout.rect(0, ScrollPos::default()).min.y };
    let built = if laid == 0 { 0.0 } else { layout.rect(laid - 1, ScrollPos::default()).max.y - top };
    built + (rows - laid) as f32 * row
}

impl<C: Collection> ItemsHooks<'_, '_, C> {
    /// Step 5 in both runs: what the rows take in the room `measuring` gives. The auto rows in view are measured
    /// first (`measure_ahead`), so the collection takes its size, and places its rows, by this pass's measure.
    fn items_extent(&mut self, comp: &C, at: &Measuring<'_, '_>, id: Id) -> Vec2 {
        let (env, room) = (at.place, at.room);
        // The layout reads the component's own motion memory (`StyleCx::exiting`): an item leaving its source keeps
        // its room until its exit ends.
        let scx = &at.scx.owned_by(at.memory, id, env.motion);
        if self.scrolls && !self.scroll_bound {
            let kept = at.with_ui(|ui| crate::contract::rt::items::rows_offset(ui));
            (self.offset, self.offset_x) = (kept.y, kept.x);
        }
        self.room = room;
        self.indent_step = scx.control().icon;
        let room = Self::inner_room(comp, scx.tokens, room);
        // Where the enclosing body does not give the width, the collection takes its width from its content, whether
        // it fills or fills the width, as a container does. Held in a content-sized column it measures so, and shows
        // across the width the column settles on.
        let held = env.content_column;
        self.from_content = (!env.gives_width() && (at.with_ui(|ui| ui.is_measuring()) || !held), env.content.1);
        self.header.measure(scx);
        let bounded = room.x.is_finite();
        let item = comp.measures_items().then(|| self.sizes.largest()).flatten();
        // Under a body that does not give the width, the measuring run lays the rows out at the width the collection
        // asks for, so the height it reports is the one it has at that width (a grid's rows at its columns).
        let from_content = self.from_content.0 && !self.content_sized(comp);
        let asked = (from_content && !self.source.is_empty()).then(|| at.with_ui(|ui| self.width_from_content(comp, (ui, env), id, room, item, scx)));
        let width = match (bounded, self.content_sized(comp)) {
            (true, false) => asked.unwrap_or(room.x),
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
                let floor = env.get::<crate::contract::overlay::LayerFloor>().0;
                let widest = self.sizes.largest().map_or(0.0, |took| took.x);
                // Columns side by side at their own widths: a `Content` column takes its widest cell, not the share of
                // the widest row it would get.
                let columns = if self.header.has_columns() { self.header.content_width() } else { 0.0 };
                widest.max(columns).max(comp.ideal_width(item, self.source.len(), scx)).max(floor).min(room.x)
            }
        };
        let previous = self.layout.take().or_else(|| self.previous.take()).map(|held| Arc::try_unwrap(held).unwrap_or_else(|held| (*held).clone()));
        let source = &*self.source;
        let mut layout = at.with_ui(|ui| super::with_measuring(ui, scx, |scx| comp.item_layout_from(previous, source, width, item, scx)));
        // An auto-width layout sets its items side by side: where its width comes from its rows, it takes their run at
        // their ideal widths, not the widest one (a tab strip in a bar's unbounded row).
        let run = (layout.auto_widths() && (asked.is_some() || !bounded) && !self.source.is_empty())
            .then(|| self.run_in(room).min(room.x))
            .filter(|run| *run > width);
        let (width, asked) = match run {
            Some(run) => {
                let source = &*self.source;
                layout = at.with_ui(|ui| super::with_measuring(ui, scx, |scx| comp.item_layout_from(Some(layout), source, run, item, scx)));
                (run, asked.map(|asked| asked.max(run)))
            }
            None => (width, asked),
        };
        if self.header.has_columns() {
            layout.set_columns(&self.header.columns(width));
        }
        self.restore_sizes(&mut layout);
        self.measure_ahead(comp, at, id, &mut layout, (width, room));
        let content = layout.content_size();
        self.content = Vec2::new(content.x as f32, content.y as f32);
        let props = comp.items();
        let row = props.row_height.of(scx);
        let floor = rows_height(&layout, self.source.len(), props.min_rows, row);
        let cap = props.max_rows.map(|rows| rows_height(&layout, self.source.len(), rows, row).max(floor));
        self.columns = layout.column_count() as u32;
        self.layout = Some(Arc::new(layout));
        let empty = if self.source.is_empty() { at.with_ui(|ui| comp.empty_height(width, ui, scx)) } else { 0.0 };
        let rows = self.content.y.max(empty).max(floor);
        let height = cap.map_or(rows, |cap| rows.min(cap)) + self.header.height;
        if !(self.scrolls && room.y.is_finite()) {
            let wide = if from_content {
                asked.unwrap_or_else(|| at.with_ui(|ui| self.width_from_content(comp, (ui, env), id, room, item, scx)))
            } else {
                self.content.x
            };
            return Vec2::new(wide, height);
        }
        // A scrolling collection in a bounded room: `Content` takes its rows up to the room, `Fill` the room.
        let wide = if self.content_sized(comp) {
            self.content.x
        } else if from_content {
            asked.unwrap_or_else(|| at.with_ui(|ui| self.width_from_content(comp, (ui, env), id, room, item, scx)))
        } else {
            0.0
        };
        match comp.props(Internal::new()).sizing {
            // Under a body sized from its content along the height, the rows take up to the room, as a content-sized
            // collection's do: the body has no room of its own to hand it.
            Sizing::Fill | Sizing::FillHeight => Vec2::new(wide, if self.from_content.1 || self.capped { height.min(room.y) } else { 0.0 }),
            Sizing::Content | Sizing::FillWidth => Vec2::new(wide, height.min(room.y)),
        }
    }
}

/// Whether a body around the collection, as its scope `env` names it, scrolls along the rows' overflow axis too: the
/// scroll along that axis belongs to the collection, so such a body is a second owner.
fn shares_scroll(props: &CollectionProps, env: &Env) -> bool {
    env.scroll_owner.is_some() && if props.overflow == Axis::X { env.unbounded.0 } else { env.unbounded.1 }
}

/// Whether the collection scrolls its rows itself in the scope `env`: a capped one does, bounded by its rows, inside a
/// scrolling body too; one with the natural height, or inside a body that scrolls along its rows, leaves the scroll to
/// that body. Its rows' scope names it as their scroll owner then (`rows_env`).
pub(super) fn scrolls_rows(props: &CollectionProps, env: &Env) -> bool {
    props.max_rows.is_some() || (!props.natural_height && !shares_scroll(props, env))
}
