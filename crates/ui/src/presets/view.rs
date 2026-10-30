//! The virtualised column of headers, list rows and grid cells, with pointer, keys and drag
//! (DS-44). One engine draws both views: the lead, the focus ring, the drop markers and the
//! AccessKit tree are the same for a grid and a list.

use std::cell::RefCell;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::Range;
use std::sync::Arc;

use egui::accesskit::{Action, Role};
use egui::containers::scroll_area::ScrollSource;
use egui::os::OperatingSystem;
use egui::text::{LayoutJob, TextWrapping};
use egui::{
    Align, Context, CornerRadius, CursorIcon, DragAndDrop, Event, EventFilter, Id, Key, Layout, Modifiers, Painter, PointerButton, Pos2, Rect,
    Response, ScrollArea, Sense, StrokeKind, Ui, UiBuilder, pos2, vec2,
};

use super::PresetBrowser;
use super::drag::{Dragging, Order, PresetReorder, Surface, all_shown};
use super::event::{PresetEvent, PresetOutput};
use super::labels::PresetLabels;
use super::layout::{Metrics, VisualRow, VisualRows, shown};
use super::model::{PresetRow, ThumbInfo, has_captions};
use super::state::{DefaultOpen, PresetState, PresetView};
use super::style::PresetStyle;
use crate::behavior::{
    Axis, DisplayOrder, DragEnd, DropPosition, GridKey, ListDrag, Move, Reorder, ReorderGesture, Resolved, StackKey, TreeMove, parent, tree_key,
};
use crate::controls::TooltipExt as _;
use crate::direction;
use crate::icons::{self, Icon};
use crate::list::drag::paint_ghost;
use crate::list::spans::Spans;
use crate::list::{DropMark, ListRow, ListRowStyle, OpenChange, RowEvent, ThumbSize, TreeRow};
use crate::popup_list;
use crate::primitives::{FocusRing, Interactive};
use crate::theme::{Tokens, TokensOf as _, UiTokens, role_format};

/// A context menu the browser was asked for, on the item or header `key`, which the menu code
/// opens and clears.
#[derive(Clone, Debug)]
pub(crate) struct MenuRequest<K> {
    /// The preset or group.
    pub key: K,
    /// Where the menu opens when it was asked for by key: the row or cell.
    pub anchor: Rect,
    /// Where the secondary click was.
    pub pointer: Option<Pos2>,
}

/// What a browser keeps between frames, in egui memory under its id.
#[derive(Clone)]
pub(crate) struct State<K> {
    /// Where the keyboard is: the focus ring's cell. The selection is the host's.
    pub lead: Option<K>,
    /// The column the vertical keys aim at.
    goal: Option<usize>,
    offset: f32,
    viewport: Option<f32>,
    view: Option<Rect>,
    reveal: Option<K>,
    focused: bool,
    /// The width the last frame measured for the width available, and what was available.
    width: Option<(f32, f32)>,
    drag: Option<Dragging<K>>,
    /// The preset or group whose rename bar is open.
    pub renaming: Option<K>,
    /// A context menu asked for.
    pub menu: Option<MenuRequest<K>>,
    /// The rect the lead was drawn in, for a menu opened by key.
    lead_rect: Option<Rect>,
}

impl<K> Default for State<K> {
    fn default() -> Self {
        Self {
            lead: None,
            goal: None,
            offset: 0.0,
            viewport: None,
            view: None,
            reveal: None,
            focused: false,
            width: None,
            drag: None,
            renaming: None,
            menu: None,
            lead_rect: None,
        }
    }
}

impl<K: Clone + Send + Sync + 'static> State<K> {
    /// The state of the browser with id `id`.
    pub(crate) fn load(ctx: &Context, id: Id) -> Self {
        ctx.data(|d| d.get_temp::<Self>(id.with("craft_ui::presets"))).unwrap_or_default()
    }

    /// Keeps the state.
    pub(crate) fn store(self, ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.insert_temp(id.with("craft_ui::presets"), self));
    }
}

/// A key the browser took this frame.
#[derive(Clone, Copy)]
enum Command {
    Nav(GridKey),
    Select,
    Activate,
    Rename,
    Menu,
    Stack(StackKey, Modifiers),
    /// ← or →, with Alt for every group.
    Tree {
        right: bool,
        all: bool,
    },
}

/// A cell's outline, painted after every cell so that it lies over its neighbours.
enum Mark {
    Hover(Rect),
    Selected(Rect),
    Insert(Rect),
}

/// One frame of a browser.
struct Pass<'p, K, F> {
    ctx: Context,
    id: Id,
    rows: &'p [PresetRow<K>],
    order: Order<'p, K>,
    vrows: &'p VisualRows,
    spans: &'p Spans,
    metrics: Metrics,
    columns: usize,
    edge: f32,
    style: PresetStyle,
    row_style: ListRowStyle,
    labels: &'p PresetLabels,
    rtl: bool,
    view: PresetView,
    flush: bool,
    tree: bool,
    icons_only: bool,
    default_open: DefaultOpen,
    selected: Option<&'p K>,
    pstate: &'p mut PresetState<K>,
    st: &'p mut State<K>,
    events: Vec<PresetEvent<K>>,
    paint: F,
    reorder: Option<&'p PresetReorder<'p, K>>,
    rules: Option<&'p Reorder<'p, K>>,
    gesture: Option<ReorderGesture<K>>,
    target: Option<Resolved<K>>,
    can_rename: Option<&'p (dyn Fn(&K) -> bool + 'p)>,
    focus: bool,
    lead_row: Option<(Id, Rect)>,
    area: Rect,
    marks: Vec<Mark>,
    tokens: Arc<UiTokens>,
}

pub(super) fn show<'a, K, F>(browser: PresetBrowser<'a, K>, id: Id, ui: &mut Ui, paint: F) -> PresetOutput<K>
where
    K: Clone + Eq + Hash + Debug + Send + Sync + 'static,
    F: FnMut(&Painter, Rect, ThumbInfo<'_, K>),
{
    let PresetBrowser {
        label,
        state: pstate,
        rows,
        selected,
        shape,
        flush,
        thumb_range,
        default_open,
        reorder,
        labels,
        can_rename,
        view_lock,
        max_height,
        ..
    } = browser;
    let tokens = Tokens::of(ui);
    let row_style = ListRow::<K>::style(&tokens);
    let style = PresetStyle::from_tokens(&tokens);
    let rtl = direction::is_rtl(ui);
    let view = view_lock.unwrap_or(pstate.view);
    let mut st = State::<K>::load(ui.ctx(), id);

    let thumb = pstate.thumb_in(&thumb_range);
    let captions = has_captions(rows);
    let side = thumb.clamp(style.thumb_list_min, style.thumb_list_max).round();
    let icons_only = rows.iter().filter(|r| !r.is_group()).all(|r| r.icon.is_some());
    let mut metrics = Metrics::new(thumb, shape, flush, style.grid_gap, style.grid_inset);
    metrics.captions = captions && view == PresetView::Grid;
    metrics.caption_line = row_style.detail_line;
    metrics.caption_gap = style.caption_gap;
    metrics.header_height = style.header_height;
    metrics.list_row = if icons_only { row_style.height(ThumbSize::None, captions) } else { row_style.height(ThumbSize::Px(side as u16), captions) };

    let avail = ui.available_width();
    let width = match st.width {
        Some((was, measured)) if (was - avail).abs() < 0.5 => measured,
        _ => (avail - ui.spacing().scroll.allocated_width()).max(1.0),
    };
    let columns = metrics.columns(width);
    let shown_rows = shown(rows, pstate, default_open);
    let vrows = VisualRows::new(rows, shown_rows, view, columns);
    let spans = vrows.spans(&metrics);
    let open: Vec<bool> = {
        let test = pstate.open_test(rows, default_open);
        vrows.shown.iter().map(|&at| rows[at].is_group() && test(at)).collect()
    };
    let order = Order { rows, shown: &vrows.shown, open: &open };
    let rules = reorder.as_ref().map(|r| Reorder::new(Axis::Vertical).accepts(move |mv: &Move<K>, modifiers| r.allows(rows, mv, modifiers)));
    let focused = ui.memory(|m| m.has_focus(id));
    let grid_edge = if view == PresetView::Grid { style.ring_selected.width + style.ring_halo.width } else { 0.0 };
    let tree = rows.iter().any(PresetRow::is_group);
    let item_count = vrows.shown.iter().filter(|&&at| !rows[at].is_group()).count();

    let mut pass = Pass {
        ctx: ui.ctx().clone(),
        id,
        rows,
        order,
        vrows: &vrows,
        spans: &spans,
        metrics,
        columns,
        edge: grid_edge,
        style,
        row_style,
        labels: &labels,
        rtl,
        view,
        flush,
        tree,
        icons_only,
        default_open,
        selected,
        pstate,
        st: &mut st,
        events: Vec::new(),
        paint,
        reorder: reorder.as_ref(),
        rules: rules.as_ref(),
        gesture: reorder.is_some().then(|| ReorderGesture::load(ui, id)),
        target: None,
        can_rename: can_rename.as_deref(),
        focus: false,
        lead_row: None,
        area: Rect::NOTHING,
        marks: Vec::new(),
        tokens,
    };

    pass.end_for_gone_rows();
    if focused && !pass.st.focused && FocusRing::visible(ui.ctx()) {
        pass.st.reveal = pass.lead_key();
    }
    if focused && ui.is_enabled() && pass.st.renaming.is_none() && !popup_list::any_open(ui.ctx()) {
        let page = pass.st.viewport.map_or(1, |h| spans.page(pass.st.offset..pass.st.offset + h));
        pass.keys(ui, page);
    }
    let scroll_to = pass.scroll_target().or_else(|| pass.drag_scroll());

    let builder = UiBuilder::new()
        .scope_id(id)
        .sense(Sense::CLICK | Sense::FOCUSABLE)
        .accessibility_role(if tree { Role::Tree } else { Role::ListBox })
        .accessibility_label(label)
        .layout(Layout::top_down(Align::Min));
    let mut measured = width;
    let response = ui
        .scope_builder(builder, |ui| {
            let mut scroll = ScrollArea::vertical()
                .id_salt("scroll")
                .auto_shrink([false, false])
                .content_margin(0.0)
                .scroll_source(ScrollSource::SCROLL_BAR | ScrollSource::MOUSE_WHEEL);
            if let Some(height) = max_height {
                scroll = scroll.max_height(height);
            }
            if let Some(offset) = scroll_to {
                scroll = scroll.vertical_scroll_offset(offset);
            }
            let out = scroll.show_viewport(ui, |ui, viewport| pass.draw(ui, viewport, focused));
            measured = out.inner;
            pass.st.offset = out.state.clamped_offset().y;
            pass.st.viewport = Some(out.inner_rect.height());
            pass.st.view = Some(out.inner_rect);
        })
        .response;

    pass.drag_frame(ui, response.rect.width());
    if response.is_pointer_button_down_on() || response.clicked_by(PointerButton::Primary) {
        pass.focus = true;
    }
    if pass.focus {
        ui.memory_mut(|m| m.request_focus(id));
    }
    let gesture_event = pass.gesture.take().and_then(ReorderGesture::finish);
    if let Some(event) = gesture_event {
        pass.events.push(PresetEvent::Reorder(event));
    }
    let focused = ui.memory(|m| m.has_focus(id));
    let active = pass.lead_row.map(|(row, _)| row).filter(|_| focused);
    let bounds = response.rect;
    ui.ctx().accesskit_node_builder(id, |node| {
        node.set_bounds(egui::accesskit::Rect { x0: bounds.min.x.into(), y0: bounds.min.y.into(), x1: bounds.max.x.into(), y1: bounds.max.y.into() });
        node.remove_action(Action::Click);
        node.set_size_of_set(item_count);
        match active {
            Some(row) => node.set_active_descendant(row.accesskit_id()),
            None => node.clear_active_descendant(),
        }
    });
    if focused {
        ui.memory_mut(|m| m.set_focus_lock_filter(id, EventFilter { horizontal_arrows: true, vertical_arrows: true, tab: false, escape: false }));
    }
    let drag = pass.st.drag.as_ref().map(|d| ListDrag { list: id, items: d.items.clone(), label: d.label.clone() });
    let events = pass.events;
    st.focused = focused;
    if (measured - width).abs() > 0.5 {
        st.width = Some((avail, measured));
        st.store(ui.ctx(), id);
        ui.ctx().request_discard("preset browser width");
    } else {
        st.store(ui.ctx(), id);
    }
    PresetOutput { response, events, drag }
}

impl<'p, K, F> Pass<'p, K, F>
where
    K: Clone + Eq + Hash + Debug + Send + Sync + 'static,
    F: FnMut(&Painter, Rect, ThumbInfo<'_, K>),
{
    fn row_at(&self, pos: usize) -> &'p PresetRow<K> {
        &self.rows[self.vrows.shown[pos]]
    }

    /// The lead's position, or the first when none is set or it is not shown.
    fn lead_pos(&self) -> Option<usize> {
        self.st.lead.as_ref().and_then(|key| self.order.position(key)).or((!self.order.is_empty()).then_some(0))
    }

    fn lead_key(&self) -> Option<K> {
        self.lead_pos().map(|pos| self.order.key(pos).clone())
    }

    fn set_lead(&mut self, pos: usize) {
        self.st.lead = Some(self.order.key(pos).clone());
    }

    fn renamable(&self, key: &K) -> bool {
        self.can_rename.is_none_or(|can| can(key))
    }

    /// Ends the drag of rows that left the list and forgets one that ended without the browser
    /// (Esc, [`cancel_all`](crate::edit::cancel_all), a pass that didn't draw it).
    fn end_for_gone_rows(&mut self) {
        if self.st.renaming.take_if(|key| self.order.position(key).is_none()).is_some() {
            self.ctx.request_repaint();
        }
        if !self.gesture.as_ref().is_some_and(ReorderGesture::is_active) && self.st.drag.take().is_some() && self.has_payload() {
            DragAndDrop::clear_payload(&self.ctx);
        }
        if self.st.drag.take_if(|drag| !all_shown(&self.order, &drag.items)).is_some() {
            self.end_drag(DragEnd::Removed, Modifiers::NONE);
        }
    }

    /// Takes the browser's keys from the input and applies them.
    fn keys(&mut self, ui: &Ui, page: usize) {
        let mac = matches!(ui.ctx().os(), OperatingSystem::Mac | OperatingSystem::IOS);
        let lead_pos = self.lead_pos();
        let on_header = lead_pos.is_some_and(|pos| self.row_at(pos).is_group());
        let tree_arrows = on_header || self.view == PresetView::List;
        let renames = self.lead_key().is_some_and(|key| self.renamable(&key));
        let stacks = self.reorder.is_some();
        let mut commands = Vec::new();
        ui.input_mut(|i| {
            i.events.retain(|event| {
                let Event::Key { key, pressed: true, modifiers, .. } = event else { return true };
                let command = if let Some(stack) = stacks.then(|| StackKey::from_key(*key, *modifiers, mac)).flatten() {
                    Some(Command::Stack(stack, *modifiers))
                } else {
                    take_command(*key, *modifiers, tree_arrows)
                };
                let keep = match command {
                    Some(Command::Rename) => !renames,
                    Some(_) => false,
                    None => true,
                };
                if !keep && let Some(command) = command {
                    commands.push(command);
                }
                keep
            });
        });
        for command in commands {
            self.command(command, page);
        }
    }

    fn command(&mut self, command: Command, page: usize) {
        let Some(pos) = self.lead_pos() else { return };
        let key = self.order.key(pos).clone();
        let group = self.row_at(pos).is_group();
        match command {
            Command::Nav(grid_key) => {
                let nav = self.vrows.nav(self.rtl, page);
                let step = nav.step(pos, grid_key, self.st.goal);
                self.st.goal = Some(step.goal);
                self.set_lead(step.index);
            }
            Command::Select if !group => {
                let modifiers = self.ctx.input(|i| i.modifiers);
                self.events.push(PresetEvent::Select { key: key.clone(), modifiers });
            }
            Command::Activate if group => self.open_change(pos, OpenChange { row: key.clone(), open: !self.order.open[pos], all: false }),
            Command::Activate => {
                let modifiers = self.ctx.input(|i| i.modifiers);
                self.events.push(PresetEvent::Activate { key: key.clone(), modifiers });
            }
            Command::Rename => self.st.renaming = Some(key.clone()),
            Command::Menu => {
                let anchor = self.st.lead_rect.unwrap_or(Rect::NOTHING);
                self.st.menu = Some(MenuRequest { key: key.clone(), anchor, pointer: None });
            }
            Command::Stack(stack, modifiers) => {
                let mv = self.rules.and_then(|rules| rules.key_move(&self.order, std::slice::from_ref(&key), stack, modifiers));
                if let (Some(mv), Some(gesture)) = (mv, self.gesture.as_mut().filter(|g| !g.is_active())) {
                    gesture.set(mv);
                }
            }
            Command::Tree { right, all } => self.tree_key(pos, right != self.rtl, all),
            Command::Select => {}
        }
        self.st.reveal = self.lead_key();
    }

    /// The tree keys on the lead: toward the end opens or enters, toward the start closes or
    /// leaves; with `all`, every group opens or closes.
    fn tree_key(&mut self, pos: usize, toward_end: bool, all: bool) {
        if all {
            let row = self.order.key(pos).clone();
            self.open_change(pos, OpenChange { row, open: toward_end, all: true });
            return;
        }
        match tree_key(&self.order, pos, toward_end) {
            Some(TreeMove::Open(at)) => self.open_change(at, OpenChange { row: self.order.key(at).clone(), open: true, all: false }),
            Some(TreeMove::Close(at)) => self.open_change(at, OpenChange { row: self.order.key(at).clone(), open: false, all: false }),
            Some(TreeMove::To(to)) => self.set_lead(to),
            None => {}
        }
    }

    /// Opens or closes a group, or every group. The lead moves to the group that closes over
    /// it (to the topmost one when all close), since the rows below leave the display.
    fn open_change(&mut self, at: usize, change: OpenChange<K>) {
        self.pstate.apply_open(self.rows, &change, self.default_open);
        if !change.open
            && let Some(lead) = self.lead_pos()
        {
            let target = if change.all {
                let mut top = lead;
                while let Some(up) = parent(&self.order, top) {
                    top = up;
                }
                Some(top)
            } else {
                let depth = self.row_at(at).depth;
                let below = (at + 1..self.order.len()).take_while(|&p| self.row_at(p).depth > depth).any(|p| p == lead);
                below.then_some(at)
            };
            if let Some(target) = target.filter(|&t| t != lead) {
                self.set_lead(target);
            }
        }
        self.events.push(PresetEvent::Open(change));
        self.ctx.request_repaint();
    }

    /// The scroll offset that reveals the row asked for, once the viewport is known.
    fn scroll_target(&mut self) -> Option<f32> {
        let height = self.st.viewport?;
        let key = self.st.reveal.take()?;
        let line = self.vrows.visual_row_of(self.order.position(&key)?);
        self.spans.reveal(line, self.st.offset, height)
    }

    /// The scroll offset while a drag is held near the top or bottom edge.
    fn drag_scroll(&self) -> Option<f32> {
        let (rules, view) = (self.rules?, self.st.view?);
        self.st.drag.as_ref()?;
        let pointer = self.ctx.pointer_latest_pos()?;
        let delta = rules.auto_scroll(pointer, view, self.ctx.input(|i| i.stable_dt));
        let offset = (self.st.offset + delta).clamp(0.0, (self.spans.total() + 2.0 * self.edge - view.height()).max(0.0));
        if offset == self.st.offset {
            return None;
        }
        self.ctx.request_repaint();
        Some(offset)
    }

    fn surface(&self) -> Surface<'_> {
        Surface { vrows: self.vrows, spans: self.spans, metrics: &self.metrics, area: self.area, columns: self.columns, rtl: self.rtl }
    }

    /// The target of the drag under way, while the pointer is over the browser.
    fn drop_target(&self, ui: &Ui) -> Option<Resolved<K>> {
        let (drag, rules) = (self.st.drag.as_ref()?, self.rules?);
        let (pointer, modifiers) = ui.input(|i| (i.pointer.hover_pos(), i.modifiers));
        if !ui.rect_contains_pointer(ui.clip_rect()) {
            return None;
        }
        self.surface().resolve(rules, &self.order, pointer?, &drag.items, modifiers)
    }

    /// Lays out the visual rows in view, and those that must stay drawn: the lead while the
    /// browser has the focus (the active descendant), the row a drag grabbed and the target's.
    /// Returns the width of the content.
    fn draw(&mut self, ui: &mut Ui, viewport: Rect, focused: bool) -> f32 {
        let content = ui.max_rect();
        let total = self.spans.total();
        ui.set_min_height(total + 2.0 * self.edge);
        self.area = Rect::from_min_size(content.min + vec2(0.0, self.edge), vec2(content.width(), total));
        self.target = self.drop_target(ui);
        let lead = self.lead_pos();
        let start = viewport.min.y - self.edge;
        let mut lines: Vec<usize> = self.spans.in_view(start..(viewport.max.y - self.edge).max(start)).collect();
        let line_of = |pos: usize| self.vrows.visual_row_of(pos);
        lines.extend(lead.filter(|_| focused).map(line_of));
        lines.extend(self.st.drag.as_ref().and_then(|d| self.order.position(&d.items[0])).map(line_of));
        lines.extend(self.target.as_ref().map(|t| line_of(t.at)));
        lines.sort_unstable();
        lines.dedup();
        for line in lines {
            self.line(ui, line, lead, focused);
        }
        self.paint_marks(ui);
        content.width()
    }

    fn line(&mut self, ui: &mut Ui, line: usize, lead: Option<usize>, focused: bool) {
        let top = self.area.top() + self.spans.top(line);
        let rect = Rect::from_min_size(pos2(self.area.left(), top), vec2(self.area.width(), self.spans.bottom(line) - self.spans.top(line)));
        match &self.vrows.rows[line] {
            VisualRow::Header(pos) => self.header(ui, *pos, rect, lead == Some(*pos) && focused, lead == Some(*pos)),
            VisualRow::Item(pos) => self.list_item(ui, *pos, rect, lead == Some(*pos) && focused, lead == Some(*pos)),
            VisualRow::Cells { range, run } => self.cells(ui, range.clone(), run.clone(), lead, focused),
        }
    }

    /// The drop marker of the row at `pos`, for a row that draws its own.
    fn mark_of(&self, pos: usize) -> Option<DropMark> {
        self.target.as_ref().filter(|t| t.at == pos).map(|t| match t.mv.position {
            DropPosition::Before => DropMark::Before,
            DropPosition::After => DropMark::After,
            DropPosition::Into => DropMark::Into,
        })
    }

    fn row_ui(id: Id, ui: &mut Ui, key: &K, rect: Rect) -> Ui {
        ui.new_child(UiBuilder::new().id_salt(id.with(("preset-row", key))).max_rect(rect).layout(Layout::top_down(Align::Min)))
    }

    fn header(&mut self, ui: &mut Ui, pos: usize, rect: Rect, focused: bool, lead: bool) {
        let row = self.row_at(pos);
        let (open, siblings) = (self.order.open[pos], siblings(&self.order, pos));
        let mut list_row = ListRow::new(row.key.clone(), row.label.clone())
            .icon(if open { Icon::FolderOpen } else { Icon::Folder })
            .dimmed(row.dimmed)
            .focused(focused)
            .drop_mark(self.mark_of(pos))
            .position_in_set(siblings.0);
        if let Some(count) = row.count {
            let format = role_format(&self.tokens, self.row_style.detail_role, self.style.count);
            let text = count.to_string();
            let width = ui.painter().layout_no_wrap(text.clone(), format.font_id.clone(), format.color).size().x;
            list_row = list_row.trailing_item(width, move |ui| {
                ui.label(egui::RichText::new(text).font(format.font_id).color(format.color));
            });
        }
        let mut child = Self::row_ui(self.id, ui, &row.key, rect);
        let out = TreeRow::new(list_row, row.depth, true, open).text(self.labels.tree.clone()).show(&mut child);
        if lead {
            self.lead_row = Some((out.response.id, rect));
            self.st.lead_rect = Some(rect);
        }
        let value = row.count.map(|n| self.labels.group_count.replace("{n}", &n.to_string()));
        self.ctx.accesskit_node_builder(out.response.id, |node| {
            node.add_action(Action::Focus);
            node.add_action(Action::ScrollIntoView);
            node.set_size_of_set(siblings.1);
            if let Some(value) = value {
                node.set_value(value);
            }
        });
        self.pointer(ui, pos, &out.response);
        for event in out.events {
            if let RowEvent::Open(change) = event {
                self.focus = true;
                self.open_change(pos, change);
            }
        }
    }

    fn list_item(&mut self, ui: &mut Ui, pos: usize, rect: Rect, focused: bool, lead: bool) {
        let row = self.row_at(pos);
        let selected = self.selected == Some(&row.key);
        let siblings = siblings(&self.order, pos);
        let side = self.metrics.thumb.clamp(self.style.thumb_list_min, self.style.thumb_list_max).round();
        let mut list_row = ListRow::new(row.key.clone(), row.label.clone())
            .dimmed(row.dimmed)
            .selected(selected)
            .focused(focused)
            .drop_mark(self.mark_of(pos))
            .position_in_set(siblings.0);
        if let Some(caption) = &row.caption {
            list_row = list_row.detail(caption.clone());
        }
        if let Some(preview) = &row.preview {
            let (aspect, paint) = (preview.aspect, preview.paint.clone());
            let width = (side * aspect).max(1.0);
            list_row = list_row.optional_item(width, move |ui| {
                let at = Rect::from_center_size(ui.max_rect().center(), vec2(width, side));
                paint(&ui.painter().with_clip_rect(at), at);
            });
        }
        let hovered = ui.rect_contains_pointer(rect) && ui.is_enabled();
        let ctx = self.ctx.clone();
        let icon_color = self.row_style.icon;
        let paint = &mut self.paint;
        let key = &row.key;
        list_row = match (self.icons_only, row.icon) {
            (true, Some(icon)) => list_row.icon(icon),
            (_, icon) => list_row.thumb(ThumbSize::Px(side as u16), move |painter, at| match icon {
                Some(icon) => paint_icon(painter, &ctx, at, icon, at.width() * 0.7, icon_color),
                None => paint(painter, at, ThumbInfo { key, size: at.size(), selected, hovered }),
            }),
        };
        let mut child = Self::row_ui(self.id, ui, &row.key, rect);
        let out = if self.tree {
            TreeRow::new(list_row, row.depth, false, false).show(&mut child)
        } else {
            list_row.role(Role::ListBoxOption).show(&mut child)
        };
        if lead {
            self.lead_row = Some((out.response.id, rect));
            self.st.lead_rect = Some(rect);
        }
        self.ctx.accesskit_node_builder(out.response.id, |node| {
            node.add_action(Action::Focus);
            node.add_action(Action::ScrollIntoView);
            node.set_size_of_set(siblings.1);
            if !self.tree {
                node.clear_level();
            }
        });
        self.pointer(ui, pos, &out.response);
    }

    /// One line of grid cells.
    fn cells(&mut self, ui: &mut Ui, range: Range<usize>, run: Range<usize>, lead: Option<usize>, focused: bool) {
        let first = self.vrows.visual_row_of(run.start);
        let top = self.spans.top(first);
        let geometry = self.metrics.run_geometry(self.area, top, run.len(), self.columns, self.rtl);
        let radius = if self.flush { CornerRadius::ZERO } else { self.style.cell_radius };
        let target = self.target.as_ref().map(|t| (t.at, t.mv.position));
        for pos in range {
            let row = self.row_at(pos);
            let cell = geometry.cell_rect(pos - run.start);
            let thumb = Rect::from_min_size(cell.min, self.metrics.shape.size(self.metrics.thumb));
            let selected = self.selected == Some(&row.key);
            let (index, size) = siblings(&self.order, pos);
            let name = match &row.caption {
                Some(caption) => format!("{}, {caption}", row.label),
                None => row.label.clone(),
            };
            let role = if self.tree { Role::TreeItem } else { Role::ListBoxOption };
            let id = self.id.with(("preset-cell", &row.key));
            let response = Interactive::with_id(id, role, name)
                .sense(Sense::CLICK | Sense::DRAG)
                .selected(selected)
                .position_in_set(index)
                .state_layer(false)
                .focus_ring(false)
                .interact_at(ui, cell)
                .finish(ui);
            let depth = row.depth;
            self.ctx.accesskit_node_builder(response.id, |node| {
                node.set_level(depth);
                node.set_size_of_set(size);
                node.add_action(Action::Focus);
                node.add_action(Action::ScrollIntoView);
            });
            let response = response.tooltip(row.tooltip.clone().unwrap_or_else(|| row.label.clone()));
            let hovered = response.hovered() && ui.is_enabled();
            if ui.is_rect_visible(cell) {
                self.paint_cell(ui, row, cell, thumb, radius, ThumbInfo { key: &row.key, size: thumb.size(), selected, hovered });
            }
            if lead == Some(pos) {
                if focused {
                    FocusRing::paint_if(ui, thumb, radius, true);
                }
                self.lead_row = Some((response.id, cell));
                self.st.lead_rect = Some(cell);
            }
            if selected {
                self.marks.push(Mark::Selected(thumb));
            } else if hovered {
                self.marks.push(Mark::Hover(thumb));
            }
            if let Some((_, position)) = target.filter(|&(at, _)| at == pos) {
                let gap = self.metrics.gap;
                let left = (position == DropPosition::Before) != self.rtl;
                let x = if left { cell.left() - gap / 2.0 } else { cell.right() + gap / 2.0 };
                self.marks.push(Mark::Insert(Rect::from_center_size(pos2(x, thumb.center().y), vec2(self.style.insert_width, thumb.height()))));
            }
            self.pointer(ui, pos, &response);
        }
    }

    fn paint_cell(&mut self, ui: &Ui, row: &PresetRow<K>, cell: Rect, thumb: Rect, radius: CornerRadius, info: ThumbInfo<'_, K>) {
        let clip = ui.clip_rect();
        let mut painter = ui.painter().with_clip_rect(thumb.intersect(clip));
        let selected = info.selected;
        if row.dimmed {
            painter.multiply_opacity(0.45);
        }
        match row.icon {
            Some(icon) => paint_icon(&painter, &self.ctx, thumb, icon, thumb.width().min(thumb.height()) * 0.6, self.row_style.icon),
            None => (self.paint)(&painter, thumb, info),
        }
        if !self.flush {
            ui.painter().rect_stroke(thumb, radius, self.style.border, StrokeKind::Inside);
        }
        if let Some(caption) = &row.caption {
            let color = if selected { self.style.caption_selected } else { self.style.caption };
            let mut job = LayoutJob::single_section(caption.clone(), role_format(&self.tokens, self.row_style.detail_role, color));
            job.wrap = TextWrapping::truncate_at_width(cell.width());
            let galley = ui.painter().layout_job(job);
            let at = pos2(cell.center().x - galley.size().x / 2.0, thumb.bottom() + self.metrics.caption_gap);
            ui.painter().with_clip_rect(cell.intersect(clip)).galley(at, galley, color);
        }
    }

    /// The outlines of the hovered and selected cells and the insertion line, over every cell.
    fn paint_marks(&mut self, ui: &Ui) {
        let radius = if self.flush { CornerRadius::ZERO } else { self.style.cell_radius };
        let painter = ui.painter();
        for mark in std::mem::take(&mut self.marks) {
            match mark {
                Mark::Hover(rect) => {
                    painter.rect_stroke(rect, radius, self.style.ring_hover, StrokeKind::Outside);
                }
                Mark::Selected(rect) => {
                    painter.rect_stroke(rect.expand(self.style.ring_selected.width), radius, self.style.ring_halo, StrokeKind::Outside);
                    painter.rect_stroke(rect, radius, self.style.ring_selected, StrokeKind::Outside);
                }
                Mark::Insert(rect) => {
                    let halo = self.row_style.drop_halo.width;
                    painter.rect_filled(rect.expand(halo), 0.0, self.style.insert_halo);
                    painter.rect_filled(rect, 0.0, self.style.insert);
                }
            }
        }
    }

    /// What a press, click, drag or screen reader request on the cell, header or row at `pos`
    /// does.
    fn pointer(&mut self, ui: &Ui, pos: usize, response: &Response) {
        let row = self.row_at(pos);
        let key = &row.key;
        let modifiers = ui.input(|i| i.modifiers);
        if response.is_pointer_button_down_on() || response.clicked_by(PointerButton::Primary) {
            self.focus = true;
        }
        if response.double_clicked() {
            self.set_lead(pos);
            self.st.goal = None;
            if !row.is_group() {
                self.events.push(PresetEvent::Activate { key: key.clone(), modifiers });
            }
        } else if response.clicked() {
            self.set_lead(pos);
            self.st.goal = None;
            if row.is_group() {
                self.open_change(pos, OpenChange { row: key.clone(), open: !self.order.open[pos], all: modifiers.alt });
            } else {
                self.events.push(PresetEvent::Select { key: key.clone(), modifiers });
            }
        }
        if self.rules.is_some() && self.st.drag.is_none() && response.drag_started_by(PointerButton::Primary) {
            self.start_drag(key, &row.label, row.is_group());
        }
        if response.secondary_clicked() {
            self.focus = true;
            self.set_lead(pos);
            self.st.menu = Some(MenuRequest { key: key.clone(), anchor: response.rect, pointer: response.interact_pointer_pos() });
        }
        if ui.input(|i| i.has_accesskit_action_request(response.id, Action::Focus)) {
            self.set_lead(pos);
            self.focus = true;
        }
    }

    /// Starts a drag of the preset or group `key`. The selection does not change.
    fn start_drag(&mut self, key: &K, label: &str, group: bool) {
        if group && !self.reorder.is_some_and(|r| r.groups) {
            return;
        }
        let Some(gesture) = self.gesture.as_mut() else { return };
        gesture.begin();
        if !gesture.is_active() {
            return;
        }
        let payload = ListDrag { list: self.id, items: vec![key.clone()], label: label.to_owned() };
        self.st.drag = Some(Dragging { items: payload.items.clone(), label: payload.label.clone(), group });
        DragAndDrop::set_payload(&self.ctx, payload);
    }

    /// Ends the drag under way when it was released, Esc was pressed or the window lost the
    /// focus; otherwise draws the ghost and the copy cursor.
    fn drag_frame(&mut self, ui: &Ui, width: f32) {
        let Some(drag) = self.st.drag.take() else { return };
        let ctx = self.ctx.clone();
        let (released, down, modifiers, pointer) =
            ui.input(|i| (i.pointer.any_released(), i.pointer.any_down(), i.modifiers, i.pointer.latest_pos()));
        let end = if crate::focus::lost(&ctx) {
            Some(DragEnd::FocusLost)
        } else if released || !down {
            Some(DragEnd::Released(self.target.as_ref().map(|t| t.mv.clone())))
        } else if !self.has_payload() {
            // egui's drag and drop takes Esc and drops the payload before the gesture sees the key.
            Some(if DragAndDrop::aborted_by_escape(&ctx) { DragEnd::Escape } else { DragEnd::PayloadCleared })
        } else {
            None
        };
        if let Some(end) = end {
            self.end_drag(end, modifiers);
            return;
        }
        if self.rules.is_some_and(|rules| rules.copies(modifiers)) {
            ctx.set_cursor_icon(CursorIcon::Copy);
        }
        if let Some(pointer) = pointer {
            let row = self.order.position(&drag.items[0]).map(|pos| self.row_at(pos));
            let (rows_ctx, paint, tokens) = (&self.ctx, RefCell::new(&mut self.paint), &self.row_style);
            let ghost = |painter: &Painter, at: Rect| match (drag.group, row) {
                (true, _) => paint_icon(painter, rows_ctx, at, Icon::Folder, at.width() * 0.8, tokens.icon),
                (false, Some(row)) => match row.icon {
                    Some(icon) => paint_icon(painter, rows_ctx, at, icon, at.width() * 0.8, tokens.icon),
                    None => (paint.borrow_mut())(painter, at, ThumbInfo { key: &row.key, size: at.size(), selected: false, hovered: false }),
                },
                (false, None) => {}
            };
            paint_ghost(ui, self.id, &self.row_style, &drag.label, pointer, width, Some((self.style.ghost_thumb, &ghost)));
        }
        self.st.drag = Some(drag);
    }

    /// Whether egui's drag and drop holds this browser's rows.
    fn has_payload(&self) -> bool {
        DragAndDrop::payload::<ListDrag<K>>(&self.ctx).is_some_and(|payload| payload.list == self.id)
    }

    /// Reports the end of a drag. A drop on the browser or a cancel clears the payload, so no
    /// other widget takes it; a release elsewhere leaves it to the widget under the pointer.
    fn end_drag(&mut self, end: DragEnd<K>, modifiers: Modifiers) {
        if !matches!(end, DragEnd::Released(None)) && self.has_payload() {
            DragAndDrop::clear_payload(&self.ctx);
        }
        if let Some(gesture) = self.gesture.as_mut() {
            gesture.end(end, modifiers);
        }
    }
}

/// What a pressed key does in a focused browser. `tree_arrows`: ← and → open and close groups
/// (a header is the lead, or the view is a list) rather than move between cells.
fn take_command(key: Key, modifiers: Modifiers, tree_arrows: bool) -> Option<Command> {
    let none = modifiers.matches_exact(Modifiers::NONE);
    match key {
        Key::Menu if none => Some(Command::Menu),
        Key::F10 if modifiers.matches_exact(Modifiers::SHIFT) => Some(Command::Menu),
        Key::F2 if none => Some(Command::Rename),
        Key::Space if none => Some(Command::Select),
        Key::Enter if none => Some(Command::Activate),
        Key::ArrowLeft | Key::ArrowRight => {
            let all = modifiers.matches_exact(Modifiers::ALT);
            if all || (none && tree_arrows) {
                Some(Command::Tree { right: key == Key::ArrowRight, all })
            } else {
                none.then(|| GridKey::from_key(key).map(Command::Nav)).flatten()
            }
        }
        _ => none.then(|| GridKey::from_key(key).map(Command::Nav)).flatten(),
    }
}

/// The zero-based position of the row at `at` among its siblings (the rows of its depth under
/// the same parent, or the top-level rows), and their number.
fn siblings<K>(order: &(impl DisplayOrder<K> + ?Sized), at: usize) -> (usize, usize) {
    let depth = order.depth(at);
    let start = parent(order, at).map_or(0, |p| p + 1);
    let (mut position, mut size) = (0, 0);
    for row in (start..order.len()).take_while(|&row| order.depth(row) >= depth) {
        if order.depth(row) == depth {
            position += usize::from(row < at);
            size += 1;
        }
    }
    (position, size)
}

/// Paints `icon` `points` wide, centred in `rect`, with `painter` (a ghost's layer).
fn paint_icon(painter: &Painter, ctx: &Context, rect: Rect, icon: Icon, points: f32, color: egui::Color32) {
    let ppp = ctx.pixels_per_point();
    let px = icons::pixels(points, ppp);
    let side = px as f32 / ppp;
    let at = Rect::from_center_size(rect.center(), vec2(side, side));
    painter.image(icons::texture(ctx, icon, px), at, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), color);
}
