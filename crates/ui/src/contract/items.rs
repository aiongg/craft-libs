//! The item step (17.3): the collection's side of steps 4 to 17 and its content sub-steps I1 to I5.

use super::collection::{CollectionEvent, CollectionResponse, ContextMenuRequest, OpenChange, Rename, ReorderMove, RowState, SelectMode};
use super::cx::BodyCx;
use super::run::{ShapeHooks, ShowAt};
use super::steps::ShapeKind;
use super::{Bind, Collection, Cx, ItemCx, ItemFn, PartsCx, Props, Show as _};
use crate::primitives::ScrollBar;
use crate::runtime::access::{Role, SetPosition};
use crate::runtime::egui::{access, alloc, child, memory as store};
use crate::runtime::env::{AppValues, ArenaId, Env};
use crate::runtime::memory::{MemoryStage, sub};
use crate::runtime::{focus, frame, wheel};
use crate::style::{StyleCx, scroll_bar_look};
use craft_behavior::{
    Axis, EditEvent, Flow, GestureId, Intent, ItemLayout, ItemSource, Mixed, NavState, NavStep, Outline, PressPhase, ScrollPos,
    ScrollSpan, SelectionChange, SelectionModel, TypeAhead, WheelTaker, nav,
};
use egui::{Context, Id, Pos2, Rect, Ui, UiBuilder, Vec2};
use std::ops::Range;

const SCROLL: craft_behavior::PartKey = craft_behavior::PartKey::new("items_scroll");

/// The flags the item step gives an item at step 12.
#[derive(Clone, Copy)]
struct RowFlags(crate::style::Flags);

pub(super) fn row_flags(ctx: &Context, id: Id) -> crate::style::Flags {
    store::peek_clone::<RowFlags>(ctx, sub(id, "row_flags")).map_or(crate::style::Flags::empty(), |flags| flags.0)
}

#[derive(Clone, Default)]
struct Scrolled {
    y: f64,
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
}

impl<K> Default for Lead<K> {
    fn default() -> Self {
        Lead { key: None, at: 0 }
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
    layout: Option<C::ItemLayout>,
    content: Vec2,
    room: Vec2,
    scrolls: bool,
    selection: C::Selection,
    selection_at_read: C::Selection,
    events: Vec<CollectionEvent<C::Key, C::ItemEvent>>,
    visible: Range<usize>,
    offset: f64,
    offset_at_read: f64,
    last: View,
    lead: Lead<C::Key>,
    renaming: Renaming<C::Key>,
    ended_rename: bool,
}

impl<'s, 'r, C: Collection> ItemsHooks<'s, 'r, C> {
    pub(super) fn new(source: &'s dyn ItemSource<C::Key>, item: &'s mut ItemFn<'r, C>) -> Self {
        ItemsHooks {
            source,
            item,
            layout: None,
            content: Vec2::ZERO,
            room: Vec2::ZERO,
            scrolls: true,
            selection: C::Selection::default(),
            selection_at_read: C::Selection::default(),
            events: Vec::new(),
            visible: 0..0,
            offset: 0.0,
            offset_at_read: 0.0,
            last: View::default(),
            lead: Lead::default(),
            renaming: Renaming::default(),
            ended_rename: false,
        }
    }

    fn rows_rect(&self, cx: &Cx<'_, C>) -> Rect {
        cx.core.content_rect
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

    fn move_lead(&mut self, to: usize) {
        let key = self.source.key(to);
        self.lead = Lead { key: Some(key.clone()), at: to };
        self.events.push(CollectionEvent::LeadMoved(key));
    }

    fn anchor_at(&self, lead: usize) -> usize {
        self.selection.anchor().and_then(|key| self.source.index_of(key)).unwrap_or(lead)
    }

    /// A press or key that selects row `at` the way the modifiers say.
    fn pick(&mut self, comp: &C, at: usize, extend: bool, toggle: bool) {
        let key = self.source.key(at);
        match Self::select_mode(comp) {
            SelectMode::None => {}
            SelectMode::Single => self.change(SelectionChange::Set(vec![key])),
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

    fn host(&mut self, _: &C) -> Box<dyn super::run::Host> {
        Box::new(super::run::Inline)
    }

    fn arena(&self, _: &C) -> bool {
        true
    }

    fn key_role(&self, comp: &C) -> craft_behavior::KeyRole {
        let props = comp.items();
        craft_behavior::KeyRole::new()
            .multi(props.select != SelectMode::None)
            .reorderable(props.reorder.is_some())
            .rename(props.rename)
            .type_ahead(true)
    }

    fn read(&mut self, comp: &mut C, stage: &mut MemoryStage, id: Id, env: &Env) {
        let ctx = stage.ctx().clone();
        self.scrolls = !env.in_scroll;
        self.selection = match comp.selection() {
            Some(bind) => match bind.get() {
                Some(Mixed::Same(held)) => held,
                _ => C::Selection::default(),
            },
            None => stage.session::<Held<C::Selection>>(sub(id, "selection")).0.clone(),
        };
        self.selection_at_read = self.selection.clone();
        self.lead = stage.session::<Lead<C::Key>>(sub(id, "lead")).clone();
        self.renaming = stage.session::<Renaming<C::Key>>(sub(id, "rename")).clone();
        self.last = stage.recall::<View>(sub(id, "view"));
        self.offset = if self.scrolls { stage.session::<Scrolled>(sub(id, "scroll")).y } else { 0.0 };
        self.offset_at_read = self.offset;
        let view = self.last.view.map_or(0.0, |view| view[3] - view[1]);
        if self.scrolls {
            if let Some(share) = wheel::take(&ctx, id) {
                self.offset -= f64::from(share.scroll.y);
            }
            if let (Some(rect), Some(bounds)) = (super::take_reveal(&ctx, id), self.last.view) {
                if rect.min.y < bounds[1] {
                    self.offset += f64::from(rect.min.y - bounds[1]);
                } else if rect.max.y > bounds[3] {
                    self.offset += f64::from(rect.max.y - bounds[3]);
                }
            }
            self.offset = self.offset.clamp(0.0, f64::from((self.last.extent - view).max(0.0)));
        }
    }

    fn extent(&mut self, comp: &C, ui: &Ui, at: ShowAt, _: Id, scx: &StyleCx<'_>) -> Vec2 {
        let room = match at {
            ShowAt::Rect(rect) => rect.size(),
            ShowAt::Flow => ui.available_size_before_wrap(),
        };
        self.room = room;
        let width = if room.x.is_finite() { room.x } else { 0.0 };
        let layout = comp.item_layout(self.source, width, scx);
        let content = layout.content_size();
        self.content = Vec2::new(content.x as f32, content.y as f32);
        self.layout = Some(layout);
        if self.scrolls && room.y.is_finite() { Vec2::ZERO } else { self.content }
    }

    fn sized(&self, _: &C, _: &Ui, size: Vec2) -> Vec2 {
        let width = if self.room.x.is_finite() { self.room.x.max(size.x) } else { size.x };
        let height = if self.scrolls && self.room.y.is_finite() { self.room.y } else { size.y };
        Vec2::new(width, height)
    }

    fn parts(&mut self, _: &mut C, _: Rect, pcx: &mut PartsCx<'_, C>) {
        if !self.scrolls {
            return;
        }
        let view = pcx.content_rect;
        let ctx = pcx.ui.ctx().clone();
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
        wheel::register(&ctx, WheelTaker::new(pcx.id, view).vertical(true));
    }

    fn updated(&mut self, comp: &mut C, cx: &mut Cx<'_, C>) {
        use super::Pointer as _;
        let ctx = cx.core.ctx.clone();
        let rect = self.rows_rect(cx);
        let view = rect.height();
        let (rtl, id) = (cx.core.flow.is_rtl(), cx.core.id);
        if let Some(range) = cx.scroll_moved(SCROLL) {
            self.offset = range.start;
        }
        self.offset = self.offset.clamp(0.0, self.max_offset(view));
        let len = self.source.len();
        // A lead whose key left the source moves to the key now in its place, else the one before it.
        if let Some(key) = self.lead.key.clone()
            && self.source.index_of(&key).is_none()
        {
            self.lead.key = None;
            if len > 0 {
                let to = self.lead.at.min(len - 1);
                self.move_lead(to);
                if self.selection.contains(&key) {
                    self.pick(comp, to, false, false);
                }
            }
        } else if let Some(at) = self.lead.key.as_ref().and_then(|key| self.source.index_of(key)) {
            self.lead.at = at;
        }
        let lead_at = self.lead.key.as_ref().and_then(|key| self.source.index_of(key));
        let mode = Self::select_mode(comp);
        let props = comp.items().clone();
        let Some(layout) = self.layout.take() else { return };
        let offset = ScrollPos { x: 0.0, y: self.offset };
        // A press on a row selects it; a double press activates it; a right press asks for its menu.
        let flow = Flow::new(Rect::from_min_size(Pos2::ZERO, rect.size()), rtl);
        let local = |at: Pos2| {
            let at = at - rect.min.to_vec2();
            if layout.mirrors() && rtl { flow.to_physical(Rect::from_min_size(at, Vec2::ZERO)).min } else { at }
        };
        if let PressPhase::Down { start, .. } = cx.core.press.phase
            && let Some(at) = layout.hit(local(start.origin), offset)
            && self.source.enabled(at)
        {
            let mods = crate::runtime::pointer::frame(&ctx).mods;
            let secondary = start.button == craft_behavior::PointerButton::Secondary;
            let key = self.source.key(at);
            if !secondary || !self.selection.contains(&key) {
                self.pick(comp, at, mods.shift, mods.cmd);
            }
            self.move_lead(at);
            if secondary {
                let place = self.screen(&layout, rect, at, offset, rtl).left_bottom();
                self.events.push(CollectionEvent::ContextMenu(ContextMenuRequest { key: key.clone(), at: place }));
            } else if start.clicks == 2 {
                self.events.push(CollectionEvent::Activate(key));
            }
        }
        let outline = Outline::new(self.source);
        let page = self.page_rows(&layout, view);
        while let Some((mv, mods)) = cx.core.intents.take_move() {
            if len == 0 {
                break;
            }
            let lead = self.lead.key.as_ref().and_then(|key| self.source.index_of(key));
            let mut state = NavState::new().page(page);
            state.lead = lead;
            state.anchor = lead.map(|at| self.anchor_at(at)).filter(|_| mods.extend);
            match nav::step(layout.nav(), &outline, &mut state, mv, mods) {
                NavStep::Moved(to) => {
                    self.move_lead(to);
                    if mods.extend && matches!(mode, SelectMode::Multi | SelectMode::Cells) {
                        let anchor = state.anchor.unwrap_or(to);
                        self.change(SelectionChange::Set(self.range(anchor, to)));
                    } else if !mods.lead && !mods.extend {
                        self.pick(comp, to, false, false);
                    }
                    self.reveal_with(&layout, to, view);
                }
                NavStep::Expand(at) => self.events.push(CollectionEvent::Open(OpenChange { key: self.source.key(at), open: true })),
                NavStep::Collapse(at) => self.events.push(CollectionEvent::Open(OpenChange { key: self.source.key(at), open: false })),
                _ => {}
            }
        }
        let lead_at = self.lead.key.as_ref().and_then(|key| self.source.index_of(key)).or(lead_at);
        if cx.core.intents.take(Intent::Toggle)
            && let Some(at) = lead_at
        {
            self.pick(comp, at, false, mode != SelectMode::Single);
        }
        if cx.core.intents.take(Intent::Activate)
            && let Some(at) = lead_at
        {
            self.events.push(CollectionEvent::Activate(self.source.key(at)));
        }
        if cx.core.intents.take(Intent::ContextMenu)
            && let Some(at) = lead_at
        {
            let place = self.screen(&layout, rect, at, offset, rtl).left_bottom();
            self.events.push(CollectionEvent::ContextMenu(ContextMenuRequest { key: self.source.key(at), at: place }));
        }
        if let Some(forward) = take_move_item(&mut cx.core.intents)
            && props.reorder.is_some()
            && let Some(at) = lead_at
        {
            let to = if forward { (at + 1).min(len - 1) } else { at.saturating_sub(1) };
            if to != at {
                let key = self.source.key(at);
                let id = crate::runtime::edit::next_gesture_id(&ctx);
                let label = cx.core.label_arc();
                let before = ReorderMove { key: key.clone(), to: at };
                self.events.push(CollectionEvent::Reorder(EditEvent::set(id, Mixed::Same(before), ReorderMove { key, to }, label, None)));
            }
        }
        if props.rename
            && cx.core.intents.take(Intent::Rename)
            && let Some(at) = lead_at
        {
            let key = self.source.key(at);
            let draft = self.source.text(at).map(|text| text.into_owned()).unwrap_or_default();
            let gesture = crate::runtime::edit::next_gesture_id(&ctx);
            let before = Rename { key: key.clone(), name: draft.clone() };
            self.events.push(CollectionEvent::Rename(EditEvent::begin(gesture, Mixed::Same(before), cx.core.label_arc(), None)));
            self.renaming = Renaming { key: Some(key), draft, gesture, focused: false };
        }
        while let Some(text) = cx.core.intents.take_text() {
            let now = frame::clock(&ctx);
            let mut typing = cx.core.memory.session::<Typing>(sub(id, "typing")).0.clone();
            let mut prefix = String::new();
            for letter in text.chars() {
                prefix = typing.push(letter, now).to_owned();
            }
            cx.core.memory.session::<Typing>(sub(id, "typing")).0 = typing;
            let labels: Vec<_> = (0..len).map(|at| self.source.text(at)).collect();
            let from = lead_at;
            if let Some(to) = TypeAhead::pick(&prefix, from, len, |at| labels[at].as_deref().unwrap_or("")) {
                self.move_lead(to);
                self.pick(comp, to, false, false);
                self.reveal_with(&layout, to, view);
            }
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
                bind.apply(&EditEvent::set(GestureId::default(), Mixed::Same(before), self.selection.clone(), label, doc_key));
            }
            None => cx.core.memory.session::<Held<C::Selection>>(sub(id, "selection")).0 = self.selection.clone(),
        }
        self.selection_at_read = self.selection.clone();
    }

    fn content(&mut self, comp: &mut C, ui: &mut Ui, env: &Env, cx: &mut Cx<'_, C>, values: &AppValues) {
        let Some(layout) = self.layout.take() else { return };
        let (id, ctx) = (cx.core.id, cx.core.ctx.clone());
        let rect = self.rows_rect(cx);
        let rtl = env.direction.is_rtl();
        let node = matches!(C::ROLE, Role::Node(_));
        let offset = ScrollPos { x: 0.0, y: self.offset };
        let mut shown = Vec::new();
        layout.visible(offset, rect.size(), &mut shown);
        self.visible = shown.first().copied().unwrap_or(0)..shown.last().map_or(0, |last| last + 1);
        // Keep-alive: the lead, the anchor and the renaming row stay built while scrolled out of view.
        let keep = [
            self.lead.key.as_ref(),
            self.selection.anchor(),
            self.renaming.key.as_ref(),
        ];
        let extra: Vec<usize> = keep.into_iter().flatten().filter_map(|key| self.source.index_of(key)).filter(|at| !shown.contains(at)).collect();
        let order: Vec<usize> = shown.iter().copied().chain(extra).collect();
        let command_scope = comp.items().command_scope;
        if let Some(scope) = command_scope {
            crate::runtime::commands::register_instance(&ctx, scope, id);
        }
        let base_env = env.derive(|scoped| {
            scoped.values.extend(values);
            scoped.arena = Some(ArenaId::new(id));
            if let Some(scope) = command_scope {
                scoped.commands = scoped.commands.pushed(scope, id);
            }
        });
        let len = self.source.len();
        let tree = matches!(layout.nav(), craft_behavior::NavShape::Tree);
        let focused = focus::is_focused(&ctx, id);
        let lead = self.lead.key.clone();
        for at in order {
            let key = self.source.key(at);
            let item_id = sub(id, ("key", key_hash(&key)));
            let row = self.screen(&layout, rect, at, offset, rtl);
            let state = RowState {
                selected: self.selection.contains(&key),
                lead: lead.as_ref() == Some(&key),
                renaming: self.renaming.key.as_ref() == Some(&key),
                open: self.source.has_children(at) && self.source.open(at),
                ..RowState::default()
            };
            let mut flags = crate::style::Flags::empty();
            flags.set(crate::style::Flags::SELECTED, state.selected);
            flags.set(crate::style::Flags::LEAD, state.lead && focused);
            flags.set(crate::style::Flags::OPEN, state.open);
            store::put(&ctx, sub(item_id, "row_flags"), RowFlags(flags));
            let mut position = SetPosition::new(at).of(len);
            if tree {
                position = position.level(self.source.depth(at) + 1);
            }
            crate::runtime::access::put_item_position(&ctx, item_id, position);
            let mut cells = Vec::new();
            layout.cells(at, offset, &mut cells);
            let mut icx = ItemCx {
                key: &key,
                at,
                rect: row,
                row: state,
                depth: self.source.depth(at),
                set: position,
                env: cx.core.env,
                id: sub(item_id, "item"),
                cells,
                grid: layout.grid(),
                tip: None,
                memory: &mut *cx.core.memory,
            };
            icx.memory.claim(icx.id);
            let mut item = (self.item)(&key, &mut icx);
            let tip = icx.tip.take();
            drop(icx);
            let builder = UiBuilder::new().max_rect(row).layout(child::column_layout(rtl)).scope_id(sub(item_id, "row"));
            let builder = if node { access::with_parent(builder, id) } else { builder };
            let mut row_ui = alloc::child(ui, builder);
            row_ui.set_clip_rect(rect.intersect(ui.clip_rect()));
            let installed = Env::install_for(&row_ui, base_env.derive(|scoped| scoped.item = Some(item_id)));
            item = Props::id(item, item_id);
            let out = match tip {
                Some(tip) => item.show_at_with_tip(&mut row_ui, row, tip),
                None => item.show_at(&mut row_ui, row),
            };
            if let Some(event) = super::ItemOutput::into_event(out) {
                self.events.push(CollectionEvent::Item(key.clone(), event));
            }
            if state.renaming {
                let mut draft = std::mem::take(&mut self.renaming.draft);
                let editor = comp.rename_editor(&mut row_ui, Bind::bound(&mut draft));
                self.renaming.draft = draft;
                if let Some(editor) = editor {
                    if !self.renaming.focused {
                        focus::request_now(&ctx, focus::FocusTarget::Id(editor.response.id));
                        self.renaming.focused = true;
                    }
                    let gesture = self.renaming.gesture;
                    let before = Mixed::Same(Rename { key: key.clone(), name: self.source.text(at).map(|text| text.into_owned()).unwrap_or_default() });
                    match editor.event {
                        Some(EditEvent::Commit { after, .. }) => {
                            let after = Rename { key: key.clone(), name: after };
                            self.events.push(CollectionEvent::Rename(EditEvent::commit(gesture, before, after)));
                            self.ended_rename = true;
                        }
                        Some(EditEvent::Cancel { reason, .. }) => {
                            self.events.push(CollectionEvent::Rename(EditEvent::cancel(gesture, reason, Some(before))));
                            self.ended_rename = true;
                        }
                        _ => {}
                    }
                }
            }
            Env::restore(&ctx, installed);
        }
        if self.ended_rename {
            self.renaming = Renaming::default();
            focus::request_now(&ctx, focus::FocusTarget::Id(id));
        }
        if focused && let Some(at) = lead.as_ref().and_then(|key| self.source.index_of(key)) {
            access::set_focus(&ctx, sub(id, ("key", key_hash(&self.source.key(at)))));
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

    fn settle(&mut self, _: &mut C, cx: &mut Cx<'_, C>) {
        let id = cx.core.id;
        if self.scrolls && self.offset != self.offset_at_read {
            cx.core.memory.session::<Scrolled>(sub(id, "scroll")).y = self.offset;
        }
        *cx.core.memory.session::<Lead<C::Key>>(sub(id, "lead")) = self.lead.clone();
        *cx.core.memory.session::<Renaming<C::Key>>(sub(id, "rename")) = self.renaming.clone();
        cx.core.collection_events = Some(Box::new(std::mem::take(&mut self.events)));
        cx.core.visible_rows = self.visible.clone();
    }

    fn record(&mut self, stage: &mut MemoryStage, _: &Context, id: Id) {
        let view = stage.shown::<View>(sub(id, "view"));
        view.extent = self.content.y;
    }
}

impl<C: Collection> ItemsHooks<'_, '_, C> {
    fn reveal_with(&mut self, layout: &C::ItemLayout, at: usize, view: f32) {
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
        let local = layout.rect(at, offset);
        let local = if rtl && layout.mirrors() { Flow::new(Rect::from_min_size(Pos2::ZERO, view.size()), true).to_physical(local) } else { local };
        local.translate(view.min.to_vec2())
    }
}

fn take_move_item(intents: &mut crate::runtime::keys::Intents) -> Option<bool> {
    if intents.take(Intent::move_item(true)) {
        Some(true)
    } else if intents.take(Intent::move_item(false)) {
        Some(false)
    } else {
        None
    }
}

fn key_hash<K: std::hash::Hash>(key: &K) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

impl<C: Collection> Cx<'_, C> {
    /// The output of a collection: its response, every event in drawing order and the positions it showed.
    pub fn collection_response(&mut self) -> CollectionResponse<C::Key, C::ItemEvent> {
        let events = self
            .core
            .collection_events
            .take()
            .and_then(|events| events.downcast::<Vec<CollectionEvent<C::Key, C::ItemEvent>>>().ok())
            .map(|events| *events)
            .unwrap_or_default();
        CollectionResponse::new(self.core.response.clone(), events, self.core.visible_rows.clone())
    }
}
