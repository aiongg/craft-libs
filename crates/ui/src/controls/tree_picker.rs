//! [`TreePicker`]: a field that opens a tree of items, folders for browsing, with a search.

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::{AsIdSalt, Context, EventFilter, Id, IdSalt, Key, Modifiers, Shape, Ui, pos2, vec2};

use super::option_list::{paint_chevron, text_galley};
use super::select::{Select, SelectStyle};
use super::text_field::{BoxState, field_box};
use super::{ControlSize, SearchField, TooltipExt as _};
use crate::behavior::{ListSelection, RowInfo, Selection, SelectionMode, Step, TreeMove, take_composing_keys, tree_key};
use crate::direction;
use crate::edit::{EditEvent, EditResponse};
use crate::icons::Icon;
use crate::list::{List, ListEvent, ListRow, ListRowStyle, ThumbSize};
use crate::localized::Labels as _;
use crate::popover::{Popover, PopoverAnchor, PopoverClose, PopoverSize};
use crate::primitives::Interactive;
use crate::theme::{Tokens, TokensOf as _, UiTokens};

crate::words! {
    /// The picker's words, English by default.
    #[non_exhaustive]
    pub struct TreePickerLabels {
        /// The field with nothing chosen (LightCraft's "Choose an album…" is the app's own).
        placeholder: "treePicker.placeholder" = "Choose…";
        search: "treePicker.search" = "Search";
        clear_search: "treePicker.clearSearch" = "Clear Search";
        no_match: "treePicker.noMatch" = "No matches";
        /// Between the folders of a path in the field and in search results.
        separator: "treePicker.separator" = " / ";
    }
}

/// One item of the tree, in tree order (each folder followed by what it holds).
#[derive(Clone, Debug, PartialEq)]
pub struct TreeEntry<K> {
    pub key: K,
    pub name: String,
    /// The folder holding it; `None` at the top.
    pub parent: Option<K>,
    /// A folder is for browsing, not picking.
    pub folder: bool,
    /// Why it can't be picked, when it can't: drawn dimmed, the reason as its tooltip.
    pub blocked: Option<String>,
    /// An icon before the name (LightCraft's album and smart album).
    pub icon: Option<Icon>,
}

/// The picker's sizes, from [`TreePicker::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TreePickerStyle {
    /// The closed box and the popup surface: a select's look at the size.
    pub select: SelectStyle,
    /// The popup's rows: the list row's look.
    pub rows: ListRowStyle,
    /// The list's tallest height before it scrolls: `tree_picker.list_height`.
    pub list_height: f32,
    /// The popup's narrowest width: `tree_picker.min_popup_width`.
    pub min_popup_width: f32,
}

/// A drop-down tree, after LightCraft's album picker.
///
/// ```ignore
/// let out = TreePicker::new("target", "Album", &mut rule.album, &entries).show(ui);
/// if let Some(EditEvent::Set { value: album, .. }) = out.event { save(rule) }
/// ```
///
/// - **Field.** Looks and sizes as a closed [`Select`](crate::controls::Select): the chosen
///   item's path ([`tree_path`]), or the placeholder; the whole path as its tooltip.
/// - **Popup.** A [`Popover`](crate::popover::Popover) under the field, at most 320 points high
///   (LightCraft's `MENU_HEIGHT`), with a [`SearchField`](crate::controls::SearchField) that takes
///   the focus, over a [`List`](crate::list::List) in tree mode. With nothing typed it shows the
///   tree with the folders closed except those holding the chosen item, which is selected and in
///   view. Typing lists the items (not folders) whose path holds every word typed, names that
///   start with the query first ([`tree_search`]), each with its path as the detail line.
/// - **Keys.** In the search field ↑ and ↓ move through the rows, → and ← open and close a folder
///   as the tree keys do, Enter picks the active item (on a folder it opens or closes it), Esc
///   closes the popup and gives the focus back to the field. On the closed field Enter, Space
///   and Alt+↓ open it.
/// - **Pointer.** A click on an item picks it and closes the popup; a click on a folder opens or
///   closes it; a blocked item does nothing and shows its reason. The press that closes the
///   popup outside it does nothing else (the menus' closing-press rule).
/// - **Report.** A pick is a one-shot [`Set`](crate::edit::EditEvent::Set) of the new value in
///   the [`EditResponse`], as a [`Select`](crate::controls::Select)'s; the value is already set.
///   Browsing, searching and Esc report nothing. While the popup is open it owns the keys
///   ([`focus::key_owner`](crate::focus::key_owner) reports a popup list).
/// - **AccessKit.** The field is a `ComboBox` named by its label with the path as its value; the
///   rows are the list's `Tree` and `TreeItem`s.
pub struct TreePicker<'a, K> {
    id_salt: IdSalt,
    label: String,
    value: &'a mut Option<K>,
    entries: &'a [TreeEntry<K>],
    placeholder: Option<String>,
    size: ControlSize,
    enabled: bool,
    labels: Option<TreePickerLabels>,
}

impl<'a, K: Clone + Eq + Hash + Debug + Send + Sync + 'static> TreePicker<'a, K> {
    /// A picker named `label` (its accessible name) editing `value` over `entries` in tree order.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, value: &'a mut Option<K>, entries: &'a [TreeEntry<K>]) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            value,
            entries,
            placeholder: None,
            size: ControlSize::Md,
            enabled: true,
            labels: None,
        }
    }

    /// Words other than the installed ones.
    pub fn labels(mut self, labels: TreePickerLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The field's text with nothing chosen (as shown, translated).
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = Some(text.into());
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The picker's sizes and colours at `size`.
    pub fn style(tokens: &UiTokens, size: ControlSize) -> TreePickerStyle {
        let picker = &tokens.component.tree_picker;
        TreePickerStyle {
            select: Select::style(tokens, size),
            rows: ListRow::<usize>::style(tokens),
            list_height: picker.list_height,
            min_popup_width: picker.min_popup_width,
        }
    }

    // lightcraft: crates/ui-egui/src/widgets/album_picker.rs:201 show, :208 show_with_choices
    pub fn show(self, ui: &mut Ui) -> EditResponse<Option<K>> {
        let Self { id_salt, label, value, entries, placeholder, size, enabled, labels } = self;
        let labels = labels.unwrap_or_else(|| TreePickerLabels::of(ui));
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        let look = Self::style(&tokens, size);
        let style = look.select;
        let id = ui.make_persistent_id(id_salt);
        let enabled = enabled && ui.is_enabled();
        let index = Index::of(&ctx, id, entries);
        let chosen = value.as_ref().and_then(|key| index.positions.get(key).copied());
        let mut path = chosen.map(|at| index.path(entries, at, &labels.separator)).unwrap_or_default();

        let open_id = id.with("open");
        let mut open = enabled && ctx.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
        let width = if ui.layout().horizontal_justify() { ui.available_width() } else { style.field.width };
        let (_, rect) = ui.allocate_space(vec2(width, style.field.height));
        let response = Interactive::with_id(id, Role::ComboBox, label.as_str())
            .expanded(open)
            .enabled(enabled)
            .state_layer(false)
            .focus_ring(false)
            .interact_at(ui, rect)
            .finish(ui);
        if enabled {
            if response.clicked() {
                open = !open;
            } else if !open && response.has_focus() && ui.input_mut(|i| i.consume_key(Modifiers::ALT, Key::ArrowDown)) {
                open = true;
            }
        }

        let mut picked = None;
        let popup_salt = (id, "popup");
        let popup_min = look.min_popup_width;
        let popup_id = Popover::id_in(ui, popup_salt);
        let state_id = id.with("state");
        let shown = Popover::new(popup_salt, PopoverAnchor::Control(response.clone()))
            .size(PopoverSize::Fit)
            .close(PopoverClose::default())
            .toggle(id)
            .label(label.as_str())
            .show(ui, &mut open, |ui, pop| {
                ui.set_width(width.max(popup_min));
                let mut state = ctx.data(|d| d.get_temp::<Browsing>(state_id)).unwrap_or_else(|| Browsing::opening(&index, chosen));
                let found =
                    state.pick(ui, &Pass { entries, index: &index, chosen, labels: &labels, size, enabled }, |field| pop.initial_focus(field));
                ctx.data_mut(|d| d.insert_temp(state_id, state));
                if found.is_some() {
                    pop.close();
                }
                found
            });
        if let Some(shown) = shown {
            picked = shown.inner;
            ctx.memory_mut(|m| {
                m.swallow_presses_outside(egui::LayerId::new(egui::Order::Foreground, popup_id));
                m.allow_presses_in(response.rect);
            });
        } else {
            ctx.data_mut(|d| d.remove_temp::<Browsing>(state_id));
        }
        ctx.data_mut(|d| d.insert_temp(open_id, open));

        let mut event = None;
        let mut response = response;
        if let Some(at) = picked {
            let key = entries[at].key.clone();
            *value = Some(key.clone());
            path = index.path(entries, at, &labels.separator);
            event = Some(EditEvent::set(Some(key), ui.input(|i| i.modifiers)));
            response.mark_changed();
            ctx.memory_mut(|m| m.request_focus(id));
        }

        let text = (value.is_some() && !path.is_empty()).then_some(path.as_str());
        paint_field(ui, &style, &response, &Field { text, placeholder: placeholder.as_deref().unwrap_or(&labels.placeholder), enabled, open });
        ctx.accesskit_node_builder(id, |node| node.set_value(path.clone()));
        let response = if path.is_empty() { response } else { response.tooltip(&path) };
        EditResponse::new(response, event)
    }
}

/// What the popup needs from the picker while it is drawn.
struct Pass<'a, K> {
    entries: &'a [TreeEntry<K>],
    index: &'a Index<K>,
    chosen: Option<usize>,
    labels: &'a TreePickerLabels,
    size: ControlSize,
    enabled: bool,
}

/// What the popup keeps while it is open.
#[derive(Clone, Default)]
struct Browsing {
    query: String,
    /// The folders shown open, as entry positions.
    open: HashSet<usize>,
    selection: Selection<usize>,
    reveal: Option<usize>,
    /// The search results for `query` (positions), with the entries' signature they were made from.
    results: Option<(String, u64, Vec<usize>)>,
}

impl Browsing {
    /// The popup as it opens: folders closed except those holding the chosen item, which is
    /// selected and in view.
    fn opening<K>(index: &Index<K>, chosen: Option<usize>) -> Self {
        let mut open = HashSet::new();
        let mut at = chosen;
        while let Some(parent) = at.and_then(|a| index.parents[a]) {
            if !open.insert(parent) {
                break;
            }
            at = Some(parent);
        }
        Self { query: String::new(), open, selection: chosen.map_or_else(Selection::new, Selection::only), reveal: chosen, results: None }
    }
}

impl Browsing {
    /// Draws the popup and returns the position of the item picked this pass.
    fn pick<K: Clone + Eq + Hash + Debug + Send + Sync + 'static>(
        &mut self,
        ui: &mut Ui,
        pass: &Pass<'_, K>,
        initial_focus: impl Fn(Id),
    ) -> Option<usize> {
        let Pass { entries, index, chosen, labels, size, enabled } = *pass;
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        let search_salt = "craft-tree-picker-search";
        let search_id = Interactive::id_in(ui, search_salt);
        initial_focus(search_id);
        let rtl = direction::is_rtl(ui);
        let look = TreePicker::<K>::style(&tokens, size);
        let row_style = look.rows;

        // The keys of the search field, which has the focus while the popup is used from the keyboard.
        let mut activate = false;
        let in_search = enabled && ctx.memory(|m| m.has_focus(search_id)) && !take_composing_keys(ui, search_id);
        if in_search {
            let (searching, rows) = self.rows(entries, index);
            let rules = ListSelection::new(SelectionMode::Single).allow_empty(false);
            let page = (look.list_height / row_style.height(ThumbSize::None, searching)) as usize;
            let mut moved = None;
            ui.input_mut(|i| {
                for (key, step) in
                    [(Key::ArrowDown, Step::Next), (Key::ArrowUp, Step::Previous), (Key::PageDown, Step::PageDown), (Key::PageUp, Step::PageUp)]
                {
                    if i.consume_key(Modifiers::NONE, key) {
                        moved = rules.target(&self.selection, rows.as_slice(), step, page.saturating_sub(1)).or(moved);
                    }
                }
                if !searching {
                    for (key, toward_end) in [(Key::ArrowRight, !rtl), (Key::ArrowLeft, rtl)] {
                        if !i.consume_key(Modifiers::NONE, key) {
                            continue;
                        }
                        let lead = self.selection.lead.and_then(|lead| rows.iter().position(|r| r.key == lead));
                        match lead.and_then(|at| tree_key(rows.as_slice(), at, toward_end)) {
                            Some(TreeMove::Open(at)) => {
                                self.open.insert(rows[at].key);
                            }
                            Some(TreeMove::Close(at)) => {
                                self.open.remove(&rows[at].key);
                            }
                            Some(TreeMove::To(at)) => moved = Some(at),
                            None => {}
                        }
                    }
                }
                activate = i.consume_key(Modifiers::NONE, Key::Enter);
            });
            if let Some(at) = moved {
                let key = rows[at].key;
                self.selection = Selection::only(key);
                self.reveal = Some(key);
            }
        }

        let before = self.query.clone();
        let edit = SearchField::new(search_salt, labels.search.as_str(), &self.query, labels.clear_search.as_str())
            .placeholder(labels.search.as_str())
            .size(size)
            .keys(EventFilter { horizontal_arrows: true, vertical_arrows: true, escape: true, ..EventFilter::default() })
            .edit_menu(false)
            .show(ui);
        if let Some(text) = edit.event.as_ref().and_then(|e| e.value()) {
            self.query.clone_from(text);
        }
        let (searching, rows) = self.rows(entries, index);
        if self.query != before {
            // A new query lists new rows: the first match is the active one, ready for Enter.
            let first = if searching { rows.first().map(|r| r.key) } else { chosen };
            self.selection = first.map_or_else(Selection::new, Selection::only);
            self.reveal = first;
        }
        let row_height = row_style.height(ThumbSize::None, searching);
        ui.add_space(ui.spacing().item_spacing.y);

        let mut picked = None;
        let mut events = Vec::new();
        if searching && rows.is_empty() {
            ui.label(labels.no_match.as_str());
        } else {
            let height = (rows.len() as f32 * row_height).clamp(row_height, look.list_height);
            let mut list = List::new(("craft-tree-picker-list", searching), labels.search.as_str(), rows.as_slice())
                .selection(&mut self.selection)
                .mode(SelectionMode::Single)
                .allow_empty(false)
                .tree(!searching)
                .detail_lines(searching)
                .max_height(height);
            if let Some(key) = self.reveal.take() {
                list = list.reveal(key);
            }
            let out = list.show(ui, |_, &at| {
                let entry = &entries[at];
                let mut row = ListRow::new(at, entry.name.as_str());
                if let Some(icon) = entry.icon {
                    row = row.icon(icon);
                }
                if searching {
                    row = row.detail(index.path(entries, at, &labels.separator));
                }
                if let Some(reason) = &entry.blocked {
                    row = row.dimmed(true).tooltip(reason.as_str());
                }
                row
            });
            events = out.events;
        }

        let keyboard_enter = activate;
        // Search results are items only (`tree_search` skips folders), so a folder row exists only in the tree.
        let mut act = |at: usize, this: &mut Self| {
            let entry = &entries[at];
            if entry.folder && !searching {
                if !this.open.remove(&at) {
                    this.open.insert(at);
                }
            } else if !entry.folder && entry.blocked.is_none() {
                picked = Some(at);
            }
        };
        let mut listed = false;
        for event in events {
            listed = true;
            match event {
                ListEvent::Clicked(at) => act(at, self),
                ListEvent::Activate(at) if ui.input(|i| i.key_pressed(Key::Enter)) => act(at, self),
                ListEvent::Open(change) => {
                    let folders = entries.iter().enumerate().filter(|(_, e)| e.folder).map(|(at, _)| at);
                    if change.all {
                        for at in folders {
                            if change.open {
                                self.open.insert(at);
                            } else {
                                self.open.remove(&at);
                            }
                        }
                    }
                    if change.open {
                        self.open.insert(change.row);
                    } else {
                        self.open.remove(&change.row);
                    }
                }
                _ => {}
            }
        }
        if keyboard_enter && let Some(at) = self.selection.lead {
            act(at, self);
        }
        // A press on a row gave the list the focus; the search field keeps it so typing goes on.
        if listed && picked.is_none() {
            ctx.memory_mut(|m| m.request_focus(search_id));
        }
        picked
    }

    /// The rows shown and whether they are search results: the tree with its open folders, or the
    /// items that match the query.
    fn rows<K: Eq + Hash>(&mut self, entries: &[TreeEntry<K>], index: &Index<K>) -> (bool, Vec<RowInfo<usize>>) {
        if self.query.trim().is_empty() {
            return (false, self.tree_rows(entries, index));
        }
        let stale = self.results.as_ref().is_none_or(|(query, signature, _)| *query != self.query || *signature != index.signature);
        if stale {
            self.results = Some((self.query.clone(), index.signature, tree_search(entries, &self.query)));
        }
        let found = self.results.as_ref().map(|(_, _, found)| found.as_slice()).unwrap_or_default();
        (true, found.iter().map(|&at| RowInfo::leaf(at)).collect())
    }

    /// The tree as rows: a folder's children follow it while it is open.
    fn tree_rows<K>(&self, entries: &[TreeEntry<K>], index: &Index<K>) -> Vec<RowInfo<usize>> {
        let mut depth = vec![0usize; entries.len()];
        let mut visible = vec![true; entries.len()];
        let mut rows = Vec::with_capacity(entries.len());
        for (at, entry) in entries.iter().enumerate() {
            if let Some(parent) = index.parents[at].filter(|&p| p < at) {
                depth[at] = depth[parent] + 1;
                visible[at] = visible[parent] && self.open.contains(&parent);
            }
            if visible[at] {
                rows.push(RowInfo {
                    key: at,
                    depth: depth[at],
                    has_children: entry.folder,
                    open: entry.folder && self.open.contains(&at),
                    section: false,
                });
            }
        }
        rows
    }
}

/// What the closed field shows.
struct Field<'a> {
    text: Option<&'a str>,
    placeholder: &'a str,
    enabled: bool,
    open: bool,
}

/// The closed field: a select's box with the chosen item's path, or the placeholder.
fn paint_field(ui: &Ui, style: &SelectStyle, response: &egui::Response, field: &Field<'_>) {
    let f = &style.field;
    let rect = response.rect;
    if !ui.is_rect_visible(rect) {
        return;
    }
    let focused = field.open || response.has_focus();
    let mut lit = *f;
    if field.enabled && (field.open || response.contains_pointer()) {
        lit.surface.fill = style.hover_fill;
    }
    let state = BoxState { enabled: field.enabled, focused, hovered: false, invalid: false };
    ui.painter().add(Shape::Vec(field_box(ui, &lit, rect, state)));
    let (text_color, muted) = if field.enabled { (f.text, f.placeholder) } else { (f.text_disabled, f.placeholder_disabled) };
    paint_chevron(ui, f, rect, text_color);
    let forward = direction::forward(ui);
    let start = direction::start_x(ui, rect) + forward * f.pad_x;
    let end = direction::end_x(ui, rect) - forward * (f.pad_x + f.icon + f.gap);
    let (text, color) = match field.text {
        Some(text) => (text, text_color),
        None => (field.placeholder, muted),
    };
    let galley = text_galley(ui, f.role, text, color, (end - start).abs());
    let x = if direction::is_rtl(ui) { start - galley.size().x } else { start };
    let pos = pos2(x, rect.center().y - galley.size().y / 2.0).round_to_pixels(ui.pixels_per_point());
    ui.painter().galley(pos, galley, color);
}

/// The parent of each entry as a position, and where each key is, kept in egui memory for as long
/// as the entries' keys stay the same (a host rebuilds the entries every pass).
struct Index<K> {
    parents: Vec<Option<usize>>,
    positions: HashMap<K, usize>,
    /// Over the number of entries and every key, so a cached search is made again when they change.
    signature: u64,
}

impl<K: Clone + Eq + Hash + Send + Sync + 'static> Index<K> {
    fn of(ctx: &Context, id: Id, entries: &[TreeEntry<K>]) -> Arc<Self> {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        entries.len().hash(&mut hasher);
        for entry in entries {
            entry.key.hash(&mut hasher);
            entry.parent.hash(&mut hasher);
            entry.name.hash(&mut hasher);
            entry.folder.hash(&mut hasher);
        }
        let signature = hasher.finish();
        let slot = id.with("index");
        if let Some(cached) = ctx.data(|d| d.get_temp::<Arc<Self>>(slot)).filter(|cached| cached.signature == signature) {
            return cached;
        }
        let positions: HashMap<K, usize> = entries.iter().enumerate().map(|(at, e)| (e.key.clone(), at)).collect();
        let parents = entries.iter().map(|e| e.parent.as_ref().and_then(|p| positions.get(p).copied())).collect();
        let index = Arc::new(Self { parents, positions, signature });
        ctx.data_mut(|d| d.insert_temp(slot, Arc::clone(&index)));
        index
    }
}

impl<K> Index<K> {
    /// The names from the top down to entry `at`, joined with `separator`.
    fn path(&self, entries: &[TreeEntry<K>], at: usize, separator: &str) -> String {
        let mut names = vec![entries[at].name.as_str()];
        let mut at = at;
        while let Some(parent) = self.parents[at] {
            if names.len() > entries.len() {
                break;
            }
            names.push(entries[parent].name.as_str());
            at = parent;
        }
        names.reverse();
        names.join(separator)
    }
}

/// "Folder / Sub / Name" for `key`, joined with `separator`; empty for a key not in `entries`.
// lightcraft: crates/ui-egui/src/widgets/album_picker.rs:118 path_label
pub fn tree_path<K: Eq>(entries: &[TreeEntry<K>], key: &K, separator: &str) -> String {
    let Some(mut at) = entries.iter().position(|e| e.key == *key) else { return String::new() };
    let mut names = vec![entries[at].name.as_str()];
    while let Some(parent) = entries[at].parent.as_ref().and_then(|p| entries.iter().position(|e| e.key == *p)) {
        if names.len() > entries.len() {
            break;
        }
        names.push(entries[parent].name.as_str());
        at = parent;
    }
    names.reverse();
    names.join(separator)
}

/// The items (not folders) whose path, joined with " / ", holds every whitespace-separated word
/// of `query`, ignoring case; those whose own name starts with the whole trimmed query first,
/// then the rest, each in tree order. Indexes into `entries`; empty for an empty query.
// lightcraft: crates/ui-egui/src/widgets/album_picker.rs:131 search
pub fn tree_search<K: Eq + Hash>(entries: &[TreeEntry<K>], query: &str) -> Vec<usize> {
    let whole = query.trim().to_lowercase();
    if whole.is_empty() {
        return Vec::new();
    }
    let words: Vec<&str> = whole.split_whitespace().collect();
    let positions: HashMap<&K, usize> = entries.iter().enumerate().map(|(at, e)| (&e.key, at)).collect();
    let (mut first, mut rest) = (Vec::new(), Vec::new());
    for (at, entry) in entries.iter().enumerate().filter(|(_, e)| !e.folder) {
        let mut path = vec![entry.name.as_str()];
        let mut parent = entry.parent.as_ref().and_then(|p| positions.get(p).copied());
        while let Some(p) = parent.filter(|_| path.len() <= entries.len()) {
            path.push(entries[p].name.as_str());
            parent = entries[p].parent.as_ref().and_then(|k| positions.get(k).copied());
        }
        path.reverse();
        let path = path.join(" / ").to_lowercase();
        if words.iter().all(|word| path.contains(word)) {
            if entry.name.to_lowercase().starts_with(&whole) { first.push(at) } else { rest.push(at) }
        }
    }
    first.extend(rest);
    first
}
