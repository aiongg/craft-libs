//! [`List`]: a list of [`ListRow`]s that lays out only the rows in view, with one focus stop, the
//! selection rules of [`ListSelection`], rename in place and a context-menu hook.

use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

use egui::accesskit::{Action, Role};
use egui::containers::scroll_area::ScrollSource;
use egui::mutex::Mutex;
use egui::os::OperatingSystem;
use egui::{
    Align, AsIdSalt, Context, CornerRadius, Event, EventFilter, Id, IdSalt, Key, KeyboardShortcut, Layout, Modifiers, PointerButton, Pos2, Rect,
    Response, ScrollArea, Sense, Ui, UiBuilder, vec2,
};

use super::rename;
use super::row::{ListRow, RenameEnd, RowEvent, RowResponse, ThumbSize, row_id};
use super::spans::Spans;
use super::toggle::{self, ToggleColumn, ToggleEdit};
use crate::behavior::{DisplayOrder, ListKey, ListOptions, ListSelection, Listbox, RowInfo, Selection, SelectionChange, SelectionMode};
use crate::edit::{CancelReason, EditEvent, GestureId};
use crate::primitives::FocusRing;
use crate::story::{Args, Level, Story};
use crate::theme::{Tokens, TokensOf as _};

/// What a [`List`] reports, in the order it happened.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ListEvent<K> {
    /// The selection changed (a click, a key, type-ahead); the list has applied it to the
    /// selection it was given. A host with its own selection rules runs the pick instead.
    Selection(SelectionChange<K>),
    /// The lead moved without changing the selection (Control+Up or Down, a screen reader's
    /// focus on a row).
    LeadMoved(K),
    /// The row's default action: Enter on the lead, or a double click (on the label only where
    /// the list renames).
    Activate(K),
    /// The host's context menu is asked for.
    ContextMenu(ContextMenuRequest<K>),
    /// What a row reported: toggle column edits (also those of a column's
    /// [key](super::ToggleColumn::key)) and renames.
    Row(RowEvent<K>),
}

/// A context menu asked for on a list: a secondary click, or the Menu key or Shift+F10 while the
/// list has the focus.
#[derive(Clone, Debug, PartialEq)]
pub struct ContextMenuRequest<K> {
    /// The row: the one clicked, or the lead for a key; `None` for the empty space below the rows
    /// or a list without rows.
    pub row: Option<K>,
    /// Whether the row is selected: a menu opened inside a selection acts on every selected row
    /// (PhotoCraft).
    pub selected: bool,
    /// The row's rect (the list's without a row), where a menu opened from the keyboard goes.
    pub anchor: Rect,
    /// Where the pointer was for a secondary click; `None` for a key.
    pub pointer: Option<Pos2>,
}

/// What [`List::show`] returns.
#[derive(Debug)]
#[must_use = "the host applies the list's events"]
pub struct ListOutput<K> {
    /// The list: its focus and its rect.
    pub response: Response,
    /// What happened this frame, in order.
    pub events: Vec<ListEvent<K>>,
}

/// The heights of a list's rows.
enum Heights<'a> {
    /// Every row as a [`ListRow`] with this thumbnail and, if true, a detail line.
    Rows(ThumbSize, bool),
    /// Every row this high.
    Fixed(f32),
    /// Each row's own height.
    Mixed(Box<dyn Fn(usize) -> f32 + 'a>),
}

type CanRename<'a, K> = Box<dyn Fn(&K) -> bool + 'a>;

/// A list of rows: a scroll area that lays out only the rows in view, one focus stop, the
/// selection rules of [`ListSelection`], rename in place, and a hook for the host's context menu.
///
/// ```
/// use craft_ui::behavior::{RowInfo, Selection};
/// use craft_ui::list::{List, ListEvent, ListRow};
/// # egui::__run_test_ui(|ui| {
/// let names = ["Headline", "Photo", "Background"];
/// let rows: Vec<RowInfo<usize>> = (0..names.len()).map(RowInfo::leaf).collect();
/// let mut selection = Selection::only(0);
/// let out = List::new("layers", "Layers", rows.as_slice())
///     .selection(&mut selection)
///     .renamable(true)
///     .show(ui, |_, &key| ListRow::new(key, names[key]));
/// for event in out.events {
///     if let ListEvent::Activate(row) = event {
///         // The row's default action (Enter, double click).
///     }
/// }
/// # });
/// ```
///
/// - **Rows.** The host gives the rows as a [`DisplayOrder`] (keys, and for a tree depths and
///   open groups) and builds each [`ListRow`] when the list asks for it, by position and key.
///   The list sets the row's `selected`, `focused`, `renaming` and `position_in_set`. Only the
///   rows in view are built and drawn, with one above and one below, so the list takes 10,000 rows;
///   rows keep their state by key. Rows have one height ([`row_height`](List::row_height), or the
///   height of a [`ListRow`] with [thumbnails](List::thumbnails) or
///   [detail lines](List::detail_lines)), or each its own ([`heights`](List::heights)).
/// - **Focus.** The list is one Tab stop and keeps the focus while its rows scroll away; the rows
///   aren't focus stops. The focus sits on the lead (the first row with no lead shown), which
///   draws the focus ring while the keyboard is in use. A press on a row or on the empty space
///   gives the list the focus; arrows don't move the focus out of it.
/// - **Selection** follows [`ListSelection`] over the display order: clicks with Shift and
///   Cmd/Ctrl, a click on the empty space below the rows, and the keys of [`ListKey`] while the
///   list has the focus. Changes are applied to the [selection](List::selection) and reported.
///   A key move scrolls the lead into view without animation, only when it is hidden;
///   [`reveal`](List::reveal) does the same for the host.
/// - **Keys.** Enter reports [`ListEvent::Activate`]; F2 renames the lead where the list
///   [renames](List::renamable); a toggle column's [key](super::ToggleColumn::key) toggles it on
///   the selected rows as one edit gesture; the Menu key and Shift+F10 ask for the context
///   menu. Type-ahead is off ([`type_ahead`](List::type_ahead)), so single-letter tool keys keep
///   working; Cmd/Ctrl+A, Delete and other commands stay the app's.
/// - **Rename.** A double click on a row's label, F2 or [`start_rename`](List::start_rename)
///   opens the row's rename field; one at a time, so starting another commits the open one. Tab
///   and Shift+Tab commit and rename the next and previous row; Enter and Esc give the focus
///   back to the list. A row being renamed stays drawn while it scrolls away, and commits when it
///   leaves the list.
/// - **Toggle sweeps.** The row that took the press of a sweep stays drawn until the release,
///   so the gesture ends there; when that row leaves the list the sweep is cancelled with
///   [`CancelReason::Removed`].
/// - **AccessKit.** The list is a `ListBox` named by its label, `multiselectable` in multiple
///   mode, with `size_of_set` the number of rows and `active_descendant` the lead row while it
///   has the focus. Rows report `position_in_set`; their actions are `Click` (select only it),
///   `Focus` (make it the lead) and `ScrollIntoView`.
#[must_use = "show the list with `show`"]
pub struct List<'a, K, O: ?Sized> {
    id_salt: IdSalt,
    label: String,
    rows: &'a O,
    selection: Option<&'a mut Selection<K>>,
    mode: SelectionMode,
    allow_empty: bool,
    heights: Heights<'a>,
    max_height: Option<f32>,
    can_rename: Option<CanRename<'a, K>>,
    type_ahead: bool,
    reveal: Option<K>,
    start_rename: Option<K>,
}

impl<'a, K, O> List<'a, K, O>
where
    K: Clone + Eq + Hash + Debug + Send + Sync + 'static,
    O: DisplayOrder<K> + ?Sized,
{
    /// A list named `label` for AccessKit, showing `rows`; `id_salt` keeps its focus, scroll
    /// position and open rename.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, rows: &'a O) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            rows,
            selection: None,
            mode: SelectionMode::Multiple,
            allow_empty: true,
            heights: Heights::Rows(ThumbSize::None, false),
            max_height: None,
            can_rename: None,
            type_ahead: false,
            reveal: None,
            start_rename: None,
        }
    }

    /// The host's selection, which the list changes and reports. Without it the list keeps one
    /// of its own.
    pub fn selection(mut self, selection: &'a mut Selection<K>) -> Self {
        self.selection = Some(selection);
        self
    }

    /// Single or multiple selection (default multiple).
    pub fn mode(mut self, mode: SelectionMode) -> Self {
        self.mode = mode;
        self
    }

    /// Whether a click on the empty space may clear the selection (default true); a list that
    /// always has a current row, such as a history, says false.
    pub fn allow_empty(mut self, allow_empty: bool) -> Self {
        self.allow_empty = allow_empty;
        self
    }

    /// Rows as high as a [`ListRow`] with thumbnails of `thumb` (`list.thumb_row`).
    pub fn thumbnails(mut self, thumb: ThumbSize) -> Self {
        let detail = matches!(self.heights, Heights::Rows(_, true));
        self.heights = Heights::Rows(thumb, detail);
        self
    }

    /// Rows as high as a [`ListRow`] with a detail line.
    pub fn detail_lines(mut self, detail: bool) -> Self {
        let thumb = if let Heights::Rows(thumb, _) = self.heights { thumb } else { ThumbSize::None };
        self.heights = Heights::Rows(thumb, detail);
        self
    }

    /// Every row `height` points high.
    pub fn row_height(mut self, height: f32) -> Self {
        self.heights = Heights::Fixed(height);
        self
    }

    /// Each row as high as `height` gives for its position (PhotoCraft's effect rows between
    /// layer rows). The list sums the heights each frame and finds the rows in view by bisection.
    pub fn heights(mut self, height: impl Fn(usize) -> f32 + 'a) -> Self {
        self.heights = Heights::Mixed(Box::new(height));
        self
    }

    /// The list's greatest height; it fills the height available otherwise.
    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height = Some(height);
        self
    }

    /// Whether rows can be renamed in place (default false).
    pub fn renamable(mut self, renamable: bool) -> Self {
        self.can_rename = renamable.then(|| Box::new(|_: &K| true) as CanRename<'a, K>);
        self
    }

    /// Rows can be renamed in place where `can_rename` says so; a rename moved on with Tab stops
    /// at a row that can't (PhotoCraft's Background).
    pub fn can_rename(mut self, can_rename: impl Fn(&K) -> bool + 'a) -> Self {
        self.can_rename = Some(Box::new(can_rename));
        self
    }

    /// Type-ahead (default off): typed characters move to the next row whose label starts with
    /// them, as in [`Listbox`]. For lists in dialogs and Preferences; a panel's list leaves the
    /// letters to the app's tool keys.
    pub fn type_ahead(mut self, type_ahead: bool) -> Self {
        self.type_ahead = type_ahead;
        self
    }

    /// Scrolls the row `key` into view this frame, without animation, if it is hidden (a new
    /// active layer). The host opens its ancestors first.
    pub fn reveal(mut self, key: K) -> Self {
        self.reveal = Some(key);
        self
    }

    /// Opens the rename field of the row `key` (a context menu's Rename), committing any other
    /// open rename, where the row can be renamed.
    pub fn start_rename(mut self, key: K) -> Self {
        self.start_rename = Some(key);
        self
    }

    /// Shows the list across the available width. `row` builds the row at a position, with its
    /// key; it is called for the rows drawn, and for rows a key needs to read (the selected rows'
    /// toggle states, labels for type-ahead).
    pub fn show<'r>(self, ui: &mut Ui, row: impl FnMut(usize, &K) -> ListRow<'r, K>) -> ListOutput<K> {
        let List { id_salt, label, rows, selection, mode, allow_empty, heights, max_height, can_rename, type_ahead, reveal, start_rename } = self;
        let id = ui.make_persistent_id(id_salt);
        let rules = ListSelection::new(mode).allow_empty(allow_empty);
        let mut state = State::<K>::load(ui.ctx(), id);
        let mut own = std::mem::take(&mut state.own);
        let selection: &mut Selection<K> = match selection {
            Some(selection) => selection,
            None => &mut own,
        };
        let focused = ui.memory(|m| m.has_focus(id));
        let style = ListRow::<'_, K>::style(&Tokens::of(ui));
        let spans = match &heights {
            Heights::Rows(thumb, detail) => Spans::uniform(style.height(*thumb, *detail), rows.len()),
            Heights::Fixed(height) => Spans::uniform(*height, rows.len()),
            Heights::Mixed(height) => Spans::mixed(rows.len(), height),
        };
        let mut pass = Pass {
            ctx: ui.ctx().clone(),
            rows,
            rules,
            selection,
            state: &mut state,
            events: Vec::new(),
            build: row,
            can_rename: can_rename.as_deref(),
            focus: false,
            lead_row: None,
        };

        pass.end_for_gone_rows();
        if let Some(key) = start_rename {
            pass.start_rename(key);
        }
        if let Some(key) = reveal {
            pass.state.reveal = Some(key);
        }
        if focused && !pass.state.focused && FocusRing::visible(ui.ctx()) {
            pass.focus_arrived();
        }
        let mut menu_key = false;
        if focused && ui.is_enabled() {
            let page = pass.state.viewport.map_or(1, |h| spans.page(pass.state.offset..pass.state.offset + h));
            menu_key = pass.keys(ui, page, type_ahead, id);
        }

        let scroll_to = pass.scroll_target(&spans);
        let multiple = mode == SelectionMode::Multiple;
        let builder = UiBuilder::new()
            .scope_id(id)
            .sense(Sense::CLICK | Sense::FOCUSABLE)
            .accessibility_role(Role::ListBox)
            .accessibility_label(label)
            .layout(Layout::top_down(Align::Min));
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
                let out = scroll.show_viewport(ui, |ui, viewport| pass.draw(ui, viewport, &spans, focused));
                pass.state.offset = out.state.clamped_offset().y;
                pass.state.viewport = Some(out.inner_rect.height());
            })
            .response;

        pass.background(&response);
        if menu_key {
            pass.menu_for_lead(response.rect);
        }
        if pass.focus {
            ui.memory_mut(|m| m.request_focus(id));
        }
        let focused = ui.memory(|m| m.has_focus(id));
        let active = pass.lead_row.map(|(row, _)| row).filter(|_| focused);
        let bounds = response.rect;
        ui.ctx().accesskit_node_builder(id, |node| {
            // The `Ui`'s node took its bounds before its rect was known.
            node.set_bounds(egui::accesskit::Rect {
                x0: bounds.min.x.into(),
                y0: bounds.min.y.into(),
                x1: bounds.max.x.into(),
                y1: bounds.max.y.into(),
            });
            node.remove_action(Action::Click);
            node.set_size_of_set(rows.len());
            if multiple {
                node.set_multiselectable();
            }
            match active {
                Some(row) => node.set_active_descendant(row.accesskit_id()),
                None => node.clear_active_descendant(),
            }
        });
        FocusRing::paint_if(ui, response.rect, CornerRadius::ZERO, focused && rows.is_empty());
        let events = pass.events;
        if focused {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(id, EventFilter { horizontal_arrows: true, vertical_arrows: true, tab: false, escape: false });
            });
        }
        state.focused = focused;
        state.own = own;
        state.store(ui.ctx(), id);
        ListOutput { response, events }
    }
}

/// What a list keeps between frames, in egui memory under its id.
#[derive(Clone)]
struct State<K> {
    /// The selection of a list whose host gives none.
    own: Selection<K>,
    renaming: Option<Renaming<K>>,
    sweep: Option<SweepOwner<K>>,
    /// The scroll offset and viewport height of the last frame.
    offset: f32,
    viewport: Option<f32>,
    /// A row to scroll into view once the viewport is known.
    reveal: Option<K>,
    /// The scope of the `Ui` the rows are drawn in, which their ids derive from.
    rows_scope: Option<Id>,
    focused: bool,
}

impl<K> Default for State<K> {
    fn default() -> Self {
        Self { own: Selection::default(), renaming: None, sweep: None, offset: 0.0, viewport: None, reveal: None, rows_scope: None, focused: false }
    }
}

impl<K: Clone + Send + Sync + 'static> State<K> {
    fn load(ctx: &Context, id: Id) -> Self {
        ctx.data(|d| d.get_temp::<Self>(id.with("craft_ui::list"))).unwrap_or_default()
    }

    fn store(self, ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.insert_temp(id.with("craft_ui::list"), self));
    }
}

/// The open rename: its row, and the row's label when it was last drawn.
#[derive(Clone)]
struct Renaming<K> {
    key: K,
    label: String,
}

/// The row that took the press of a toggle sweep under way.
#[derive(Clone)]
struct SweepOwner<K> {
    key: K,
    id: GestureId,
    column: Option<usize>,
}

/// A key the list took this frame.
#[derive(Clone, Copy)]
enum Command {
    List(ListKey),
    Toggle(usize),
    Menu,
}

/// One frame of a list.
struct Pass<'p, K, O: ?Sized, F> {
    ctx: Context,
    rows: &'p O,
    rules: ListSelection,
    selection: &'p mut Selection<K>,
    state: &'p mut State<K>,
    events: Vec<ListEvent<K>>,
    build: F,
    can_rename: Option<&'p (dyn Fn(&K) -> bool + 'p)>,
    /// The list takes the focus at the end of the frame.
    focus: bool,
    /// The lead row's id and rect, once drawn.
    lead_row: Option<(Id, Rect)>,
}

impl<'p, 'r, K, O, F> Pass<'p, K, O, F>
where
    K: Clone + Eq + Hash + Debug + Send + Sync + 'static,
    O: DisplayOrder<K> + ?Sized,
    F: FnMut(usize, &K) -> ListRow<'r, K>,
{
    /// The lead's position, or the first row's when no lead is shown: where the focus sits.
    fn lead_at(&self) -> Option<usize> {
        self.selection.lead.as_ref().and_then(|lead| self.rows.position(lead)).or_else(|| (!self.rows.is_empty()).then_some(0))
    }

    /// The selection as the keys see it: with the lead where the focus sits.
    fn keyed(&self) -> Selection<K> {
        let mut keyed = self.selection.clone();
        keyed.lead = self.lead_at().map(|at| self.rows.key(at).clone());
        keyed
    }

    fn apply(&mut self, change: SelectionChange<K>) {
        self.selection.clone_from(&change.selection);
        self.events.push(ListEvent::Selection(change));
    }

    fn renamable(&self, key: &K) -> bool {
        self.can_rename.is_some_and(|can| can(key))
    }

    /// The focus arrived: the lead is scrolled into view.
    fn focus_arrived(&mut self) {
        self.state.reveal = self.lead_at().map(|at| self.rows.key(at).clone());
    }

    /// Commits the open rename of a row that left the list, and cancels the sweep of one.
    fn end_for_gone_rows(&mut self) {
        if let Some(open) = self.state.renaming.take_if(|open| self.rows.position(&open.key).is_none()) {
            self.commit_rename(open, RenameEnd::Removed);
        }
        if let Some(sweep) = self.state.sweep.take_if(|sweep| self.rows.position(&sweep.key).is_none()) {
            if let (Some(scope), Some(column)) = (self.state.rows_scope, sweep.column) {
                toggle::clear_sweep(&self.ctx, scope, column);
            }
            self.events.push(ListEvent::Row(RowEvent::Toggle(EditEvent::cancel(sweep.id, CancelReason::Removed))));
        }
    }

    /// Opens the rename of `key` where it can be renamed, committing another open one.
    fn start_rename(&mut self, key: K) {
        if !self.renamable(&key) || self.state.renaming.as_ref().is_some_and(|open| open.key == key) {
            return;
        }
        if let Some(open) = self.state.renaming.take() {
            self.commit_rename(open, RenameEnd::Blur);
        }
        self.state.renaming = Some(Renaming { key: key.clone(), label: String::new() });
        self.state.reveal = Some(key);
    }

    /// Ends a rename whose row doesn't show its field any more, with the text typed into it.
    fn commit_rename(&mut self, open: Renaming<K>, end: RenameEnd) {
        let typed = self.state.rows_scope.and_then(|scope| rename::take_draft(&self.ctx, row_id(scope, &open.key)));
        if let Some(text) = typed.filter(|text| *text != open.label) {
            self.events.push(ListEvent::Row(RowEvent::Renamed { row: open.key.clone(), text }));
        }
        self.events.push(ListEvent::Row(RowEvent::RenameEnded { row: open.key, end }));
    }

    /// Takes the list's keys from the input and applies them. Returns whether the context menu
    /// was asked for.
    fn keys(&mut self, ui: &Ui, page: usize, type_ahead: bool, id: Id) -> bool {
        if self.state.renaming.is_some() {
            return false;
        }
        let mut listbox = type_ahead.then(|| Listbox::load(ui, id.with("type-ahead")));
        let typing = listbox.as_ref().is_some_and(|l| l.typing(ui));
        let mac = matches!(ui.ctx().os(), OperatingSystem::Mac | OperatingSystem::IOS);
        let column_keys = self.column_keys();
        let renames = self.lead_at().is_some_and(|at| self.renamable(self.rows.key(at)));
        let mut commands = Vec::new();
        ui.input_mut(|i| {
            i.events.retain(|event| {
                let Event::Key { key, pressed: true, modifiers, .. } = event else { return true };
                let command = take_command(*key, *modifiers, mac, &column_keys);
                let keep = match command {
                    Some(Command::List(ListKey::Rename)) => !renames,
                    Some(Command::List(ListKey::ToggleLead)) => typing,
                    Some(_) => false,
                    None => true,
                };
                if !keep && let Some(command) = command {
                    commands.push(command);
                }
                keep
            });
        });
        let mut menu = false;
        for command in commands {
            match command {
                Command::List(key) => self.list_key(key, page),
                Command::Toggle(column) => self.toggle_selected(column),
                Command::Menu => menu = true,
            }
        }
        if let Some(listbox) = &mut listbox {
            self.type_ahead(ui, listbox);
        }
        if let Some(listbox) = listbox {
            listbox.store(ui);
        }
        menu
    }

    fn list_key(&mut self, key: ListKey, page: usize) {
        let keyed = self.keyed();
        let lead = keyed.lead.clone();
        match key {
            ListKey::Select { step, extend } => {
                if let Some(change) = self.rules.select(&keyed, self.rows, step, page, extend) {
                    self.apply(change);
                }
            }
            ListKey::MoveLead(step) => {
                if let Some(to) = self.rules.move_lead(&keyed, self.rows, step, page) {
                    self.selection.lead = Some(to.clone());
                    self.events.push(ListEvent::LeadMoved(to));
                }
            }
            ListKey::ToggleLead => {
                if let Some(change) = self.rules.toggle_lead(&keyed, self.rows) {
                    self.apply(change);
                }
            }
            ListKey::Activate => {
                if let Some(lead) = lead.clone() {
                    self.events.push(ListEvent::Activate(lead));
                }
            }
            ListKey::Rename => {
                if let Some(lead) = lead.clone() {
                    self.start_rename(lead);
                }
            }
        }
        self.state.reveal = self.selection.lead.clone().or(lead);
    }

    /// The keys of the lead row's toggle columns, by column.
    fn column_keys(&mut self) -> Vec<(usize, KeyboardShortcut)> {
        let Some(at) = self.lead_at() else { return Vec::new() };
        let key = self.rows.key(at).clone();
        let row = (self.build)(at, &key);
        row.columns().iter().enumerate().filter_map(|(column, c)| c.key.map(|k| (column, k))).collect()
    }

    /// A column's key: the selected rows take the opposite of the lead's state (of the first
    /// selected row's when the lead isn't selected), as one gesture.
    fn toggle_selected(&mut self, column: usize) {
        let chosen: HashSet<&K> = self.selection.items.iter().collect();
        let positions: Vec<usize> = (0..self.rows.len()).filter(|&at| chosen.contains(self.rows.key(at))).collect();
        let lead = self.selection.lead.clone();
        let mut states = Vec::with_capacity(positions.len());
        for at in positions {
            let key = self.rows.key(at).clone();
            if let Some(on) = (self.build)(at, &key).columns().get(column).map(|c| c.on) {
                states.push((key, on));
            }
        }
        let Some(reference) = states.iter().find(|(key, _)| Some(key) == lead.as_ref()).or(states.first()).cloned() else { return };
        let on = !reference.1;
        let id = GestureId::next();
        let edit = |row: K| ToggleEdit { column, row, on };
        let mut changed = false;
        let mut events = vec![EditEvent::begin(id)];
        for (key, state) in states {
            if state != on {
                changed = true;
                events.push(EditEvent::update(id, edit(key), Modifiers::NONE));
            }
        }
        events.push(EditEvent::commit(id, edit(reference.0), changed, Modifiers::NONE));
        self.events.extend(events.into_iter().map(|e| ListEvent::Row(RowEvent::Toggle(e))));
    }

    fn type_ahead(&mut self, ui: &Ui, listbox: &mut Listbox) {
        if !ui.input(|i| i.events.iter().any(|e| matches!(e, Event::Text(_)))) {
            return;
        }
        let labels = Labels((0..self.rows.len()).map(|at| (self.build)(at, self.rows.key(at)).label().to_owned()).collect());
        listbox.set_active(self.lead_at());
        if listbox.type_ahead(ui, &labels)
            && let Some(at) = listbox.active()
        {
            if let Some(change) = self.rules.click(self.selection, self.rows, at, Modifiers::NONE) {
                self.apply(change);
            }
            self.state.reveal = Some(self.rows.key(at).clone());
        }
    }

    /// The scroll offset that reveals the row asked for, once the viewport is known.
    fn scroll_target(&mut self, spans: &Spans) -> Option<f32> {
        let height = self.state.viewport?;
        let key = self.state.reveal.take()?;
        spans.reveal(self.rows.position(&key)?, self.state.offset, height)
    }

    /// Lays out the rows in view, and those that must stay drawn: the lead while the list has
    /// the focus (the active descendant), the row being renamed (its field has the focus) and the
    /// row that took a sweep's press (it ends the sweep).
    fn draw(&mut self, ui: &mut Ui, viewport: Rect, spans: &Spans, focused: bool) {
        let content = ui.max_rect();
        let width = content.width();
        ui.set_min_height(spans.total());
        let lead = self.lead_at();
        let position = |key: Option<&K>| key.and_then(|k| self.rows.position(k));
        let mut drawn: Vec<usize> = spans.in_view(viewport.min.y..viewport.max.y).collect();
        drawn.extend(lead.filter(|_| focused));
        drawn.extend(position(self.state.renaming.as_ref().map(|r| &r.key)));
        drawn.extend(position(self.state.sweep.as_ref().map(|s| &s.key)));
        drawn.sort_unstable();
        drawn.dedup();
        let mut rows_ui = ui.new_child(UiBuilder::new().id_salt("rows").max_rect(content).layout(Layout::top_down(Align::Min)));
        self.state.rows_scope = Some(rows_ui.scope_id());
        for at in drawn {
            let rect = Rect::from_min_size(content.min + vec2(0.0, spans.top(at)), vec2(width, spans.bottom(at) - spans.top(at)));
            self.row(&mut rows_ui, at, rect, lead == Some(at), focused);
        }
    }

    /// Builds, draws and reads the row at `at`; `lead` says whether it is where the focus sits,
    /// `focused` whether the list has the focus.
    fn row(&mut self, ui: &mut Ui, at: usize, rect: Rect, lead: bool, focused: bool) {
        let key = self.rows.key(at).clone();
        let row = (self.build)(at, &key);
        debug_assert!(*row.key() == key, "the row built for {key:?} has the key {:?}", row.key());
        let renaming = match &mut self.state.renaming {
            Some(open) if open.key == key => {
                row.label().clone_into(&mut open.label);
                true
            }
            _ => false,
        };
        let out = row.selected(self.selection.contains(&key)).focused(lead && focused).renaming(renaming).position_in_set(at).show_at(ui, rect);
        if lead {
            self.lead_row = Some((out.response.id, rect));
        }
        ui.ctx().accesskit_node_builder(out.response.id, |node| {
            node.add_action(Action::Focus);
            node.add_action(Action::ScrollIntoView);
        });
        self.row_pointer(ui, at, &key, &out);
        for event in out.events {
            self.row_event(at, &event);
            self.events.push(ListEvent::Row(event));
        }
    }

    fn row_pointer(&mut self, ui: &Ui, at: usize, key: &K, out: &RowResponse<K>) {
        let response = &out.response;
        let modifiers = ui.input(|i| i.modifiers);
        if response.is_pointer_button_down_on() || response.clicked_by(PointerButton::Primary) {
            self.focus = true;
        }
        if response.double_clicked() {
            let on_label = ui.input(|i| i.pointer.interact_pos()).is_some_and(|p| out.label.contains(p));
            if on_label && self.renamable(key) {
                self.start_rename(key.clone());
            } else {
                self.events.push(ListEvent::Activate(key.clone()));
            }
        } else if response.clicked()
            && let Some(change) = self.rules.click(self.selection, self.rows, at, modifiers)
        {
            self.apply(change);
        }
        if response.secondary_clicked() {
            let request = ContextMenuRequest {
                row: Some(key.clone()),
                selected: self.selection.contains(key),
                anchor: response.rect,
                pointer: response.interact_pointer_pos(),
            };
            self.events.push(ListEvent::ContextMenu(request));
        }
        if ui.input(|i| i.has_accesskit_action_request(response.id, Action::Focus)) && self.selection.lead.as_ref() != Some(key) {
            self.selection.lead = Some(key.clone());
            self.events.push(ListEvent::LeadMoved(key.clone()));
            self.focus = true;
        }
    }

    /// Follows the sweep a row reports and the end of its rename.
    fn row_event(&mut self, at: usize, event: &RowEvent<K>) {
        match event {
            RowEvent::Toggle(EditEvent::Begin { id }) => {
                self.state.sweep = Some(SweepOwner { key: self.rows.key(at).clone(), id: *id, column: None });
            }
            RowEvent::Toggle(EditEvent::Update { id, value, .. }) => {
                if let Some(sweep) = self.state.sweep.as_mut().filter(|s| s.id == *id) {
                    sweep.column.get_or_insert(value.column);
                }
            }
            RowEvent::Toggle(EditEvent::Commit { id, .. } | EditEvent::Cancel { id, .. }) => {
                self.state.sweep.take_if(|s| s.id == *id);
            }
            RowEvent::RenameEnded { end, .. } => {
                self.state.renaming = None;
                let next = match end {
                    RenameEnd::Tab => Some(at + 1),
                    RenameEnd::ShiftTab => at.checked_sub(1),
                    RenameEnd::Enter | RenameEnd::Escape => {
                        self.focus = true;
                        None
                    }
                    RenameEnd::Blur | RenameEnd::Removed => None,
                };
                if let Some(next) = next.filter(|&n| n < self.rows.len()) {
                    self.start_rename(self.rows.key(next).clone());
                }
            }
            _ => {}
        }
    }

    /// A press on the empty space gives the list the focus; a click there clears the selection,
    /// and a secondary click asks for a context menu without a row.
    fn background(&mut self, response: &Response) {
        if response.is_pointer_button_down_on() || response.clicked_by(PointerButton::Primary) {
            self.focus = true;
        }
        if response.clicked_by(PointerButton::Primary)
            && let Some(change) = self.rules.click_empty(self.selection)
        {
            self.apply(change);
        }
        if response.secondary_clicked() {
            let request = ContextMenuRequest { row: None, selected: false, anchor: response.rect, pointer: response.interact_pointer_pos() };
            self.events.push(ListEvent::ContextMenu(request));
        }
    }

    /// The Menu key or Shift+F10: the context menu of the lead, anchored at its row.
    fn menu_for_lead(&mut self, list: Rect) {
        let lead = self.lead_at().map(|at| self.rows.key(at).clone());
        let anchor = self.lead_row.map_or(list, |(_, rect)| rect);
        let selected = lead.as_ref().is_some_and(|k| self.selection.contains(k));
        self.events.push(ListEvent::ContextMenu(ContextMenuRequest { row: lead, selected, anchor, pointer: None }));
    }
}

/// What a pressed key does in a focused list: a column's key, the context-menu keys, or a list
/// key.
fn take_command(key: Key, modifiers: Modifiers, mac: bool, column_keys: &[(usize, KeyboardShortcut)]) -> Option<Command> {
    if let Some(&(column, _)) = column_keys.iter().find(|(_, s)| s.logical_key == key && modifiers.matches_logically(s.modifiers)) {
        return Some(Command::Toggle(column));
    }
    if (key == Key::Menu && modifiers.matches_exact(Modifiers::NONE)) || (key == Key::F10 && modifiers.matches_exact(Modifiers::SHIFT)) {
        return Some(Command::Menu);
    }
    ListKey::from_key(key, modifiers, mac).map(Command::List)
}

/// The rows' labels, for type-ahead.
struct Labels(Vec<String>);

impl ListOptions for Labels {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn option_at(&self, at: usize) -> usize {
        at
    }

    fn enabled(&self, _option: usize) -> bool {
        true
    }

    fn text(&self, option: usize) -> std::borrow::Cow<'_, str> {
        self.0.get(option).map_or("", String::as_str).into()
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![Story {
        id: "composites/list/ten-thousand-rows",
        level: Level::Composites,
        component: "List",
        name: "10,000 rows",
        args: || {
            Args::new()
                .with_choice("selection", &["Multiple", "Single"], 0)
                .with_bool("mixed heights", false)
                .with_number("height", 320.0, 120.0, 600.0)
        },
        show: show_rows,
    }]
}

/// The state of the story's list.
struct Rows {
    order: Vec<RowInfo<usize>>,
    names: Vec<String>,
    visible: Vec<bool>,
    selection: Selection<usize>,
    last: String,
}

impl Rows {
    const COUNT: usize = 10_000;

    fn new() -> Self {
        Rows {
            order: (0..Self::COUNT).map(RowInfo::leaf).collect(),
            names: (1..=Self::COUNT).map(|n| format!("Layer {n}")).collect(),
            visible: vec![true; Self::COUNT],
            selection: Selection::only(0),
            last: "Double-click a row, press Enter or open its menu".to_owned(),
        }
    }
}

/// 10,000 layers with an eye column (Cmd/Ctrl+, hides the selected layers), rename by double
/// click or F2, and with mixed heights a taller row with a detail line every fourth row. The line
/// under the list shows the last default action or context menu asked for.
pub(super) fn show_rows(ui: &mut Ui, args: &mut Args) {
    let mode = if args.choice("selection") == "Single" { SelectionMode::Single } else { SelectionMode::Multiple };
    let mixed = args.bool("mixed heights");
    let height = args.number("height") as f32;
    let style = ListRow::<'_, usize>::style(&Tokens::of(ui));
    let (plain, tall) = (style.height(ThumbSize::None, false), style.height(ThumbSize::None, true));
    let state_id = ui.scope_id().with("list-story");
    let shared = ui.data_mut(|d| d.get_temp_mut_or_insert_with(state_id, || Arc::new(Mutex::new(Rows::new()))).clone());
    let mut rows = shared.lock();
    let Rows { order, names, visible, selection, last } = &mut *rows;
    let hide = KeyboardShortcut::new(Modifiers::COMMAND, Key::Comma);
    ui.allocate_ui(vec2(320.0, height), |ui| {
        let mut list = List::new("layers", "Layers", order.as_slice()).selection(selection).mode(mode).renamable(true).max_height(height);
        if mixed {
            list = list.heights(move |at| if at % 4 == 3 { tall } else { plain });
        }
        let out = list.show(ui, |at, &key| {
            let row = ListRow::new(key, names[key].clone()).toggle(ToggleColumn::eye("Visibility", visible[key]).key(hide)).dimmed(!visible[key]);
            if mixed && at % 4 == 3 { row.detail("Smart filter") } else { row }
        });
        for event in out.events {
            match event {
                ListEvent::Row(RowEvent::Toggle(EditEvent::Update { value, .. } | EditEvent::Set { value, .. })) => visible[value.row] = value.on,
                ListEvent::Row(RowEvent::Renamed { row, text }) if !text.is_empty() => names[row] = text,
                ListEvent::Activate(row) => *last = format!("Open {}", names[row]),
                ListEvent::ContextMenu(request) => {
                    *last = request.row.map_or_else(|| "Menu for the list".to_owned(), |row| format!("Menu for {}", names[row]));
                }
                _ => {}
            }
        }
    });
    ui.label(last.as_str());
}
