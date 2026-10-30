//! [`CommandPalette`]: search over the app's commands, tools and panels in a modal box.

use egui::accesskit::Role;
use egui::{Align2, Color32, Context, EventFilter, Frame, Id, Key, Margin, Modifiers, ScrollArea, Sense, Ui, UiBuilder, Vec2, vec2};

use crate::behavior::{CommandSearch, ListOptions, Listbox, PaletteEntry, take_key};
use crate::composites::live;
use crate::controls::{Kbd, SearchField};
use crate::dialog::pass::{self, Entry};
use crate::menu::row::{Columns, MenuStyle, RowContent, RowPainter, RowState};
use crate::menu::{self, ItemKind, MenuEntry, MenuItem};
use crate::primitives::{Content, Divider, Interactive, Surface, SurfaceKind, SurfaceStyle, Text};
use crate::story::{Level, Story, StoryArgs};
use crate::theme::{ToEgui, UiTokens, current};
use crate::typed_story;
use craft_theme::{ControlSize, TypeRole};

/// Between the menu titles of a command's path.
const PATH_SEPARATOR: &str = " › ";

/// Most matches the list holds (it scrolls past `max_rows`).
const MAX_RESULTS: usize = 100;

/// A palette's sizes and colours, from [`CommandPalette::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CommandPaletteStyle {
    /// Width of the box: `command_palette.width`.
    pub width: f32,
    /// From the window's top to the box's top: `command_palette.top`.
    pub top: f32,
    /// Inner margin of the box: `command_palette.pad`.
    pub pad: f32,
    /// Height of a result row: `command_palette.row_height`.
    pub row_height: f32,
    /// Result rows shown before the list scrolls: `command_palette.max_rows`.
    pub max_rows: usize,
    /// Commands remembered for the empty query: `command_palette.recent`.
    pub recent: usize,
    /// The dimmed window behind the box: `surface.scrim`.
    pub scrim: Color32,
    /// The box: a popup surface.
    pub surface: SurfaceStyle,
}

/// The texts of a palette, translated by the host.
#[derive(Clone, Debug)]
pub struct PaletteLabels {
    /// The dialog's accessible name: "Command palette".
    pub name: String,
    /// The search field's name: "Search commands".
    pub search: String,
    /// The field's placeholder: "Search commands, tools and panels…".
    pub placeholder: String,
    /// The field's clear button: "Clear".
    pub clear: String,
    /// Shown when nothing matches: "No matching commands".
    pub empty: String,
    /// The key hint under the list: "↑↓ navigate  Enter run  esc close".
    pub hint: String,
    /// The announcement of the number of results, `{n}` standing for it: "{n} results".
    pub results: String,
}

impl Default for PaletteLabels {
    fn default() -> Self {
        Self {
            name: "Command palette".into(),
            search: "Search commands".into(),
            placeholder: "Search commands, tools and panels…".into(),
            clear: "Clear".into(),
            empty: "No matching commands".into(),
            hint: "↑↓ navigate  Enter run  esc close".into(),
            results: "{n} results".into(),
        }
    }
}

/// What the palette keeps between passes while it is open.
#[derive(Clone, Debug, Default)]
struct State {
    query: String,
    /// The list box has been reset for this opening.
    ready: bool,
    /// Times the field was asked for the keyboard focus.
    focus_requests: u32,
}

/// How many times the field of the palette `id` was asked for the focus in this opening.
#[cfg(test)]
pub(super) fn focus_requests(ctx: &Context, id: &str) -> u32 {
    let state_id = Id::unique(("craft_ui::command_palette", id)).with("state");
    ctx.data(|d| d.get_temp::<State>(state_id)).map_or(0, |s| s.focus_requests)
}

/// The keys the field leaves to the list: ↑ and ↓ move its active row.
fn keys() -> EventFilter {
    EventFilter { horizontal_arrows: false, vertical_arrows: true, tab: false, escape: true }
}

/// Search over the app's commands, tools and panels in one centred modal box with a scrim, as
/// PhotoCraft's Edit > Search palette.
///
/// ```
/// use craft_ui::patterns::{CommandPalette, PaletteEntry};
/// # egui::__run_test_ctx(|ctx| {
/// let entries = [PaletteEntry::new(1, "Levels…").path(["Image", "Adjustments"]), PaletteEntry::new(2, "Brush").kind("Tool")];
/// let mut open = true;
/// if let Some(entry) = CommandPalette::new("palette", &entries).show(ctx, &mut open) {
///     let _command = entry.command();
/// }
/// # });
/// ```
///
/// - The host sets `open` (Edit > Search, Cmd/Ctrl+F); the palette clears it when it closes. While
///   it is open it takes the keys below, and the host should not run its other shortcuts.
/// - The field keeps the keyboard focus, and is asked for it only while it lacks it. Up and Down
///   move the active row and wrap; Page Up and Page Down move a page; Enter runs the active
///   enabled entry and the press is taken, so a dialog the command opens does not read it as its
///   OK; Esc clears the text, then closes; a click on a row runs it and a click outside the box
///   closes. The pointer moves the active row only when it moves.
/// - The entries are ranked by [`CommandSearch`]. An empty query lists the commands last run,
///   newest first, else the first entries. The commands run are kept in egui's memory, or in the
///   host's list given to [`recents`](CommandPalette::recents).
/// - A row has the look of a menu row: the label, then the path of menu titles or the entry's
///   kind and its shortcut at the end. A disabled entry is listed and does not run.
/// - AccessKit: a `Dialog` named by the labels; the field is a `SearchInput` that controls the
///   `ListBox` and names the active `ListBoxOption` as its active descendant; the number of
///   results is announced politely.
#[must_use = "show the palette with `show`"]
pub struct CommandPalette<'a, 'r, T> {
    id: Id,
    entries: &'a [PaletteEntry<T>],
    labels: PaletteLabels,
    recents: Option<&'r mut Vec<T>>,
}

impl<'a, 'r, T> CommandPalette<'a, 'r, T> {
    pub fn new(id: impl std::hash::Hash + std::fmt::Debug, entries: &'a [PaletteEntry<T>]) -> Self {
        Self { id: Id::unique(("craft_ui::command_palette", id)), entries, labels: PaletteLabels::default(), recents: None }
    }

    /// The texts, translated.
    pub fn labels(mut self, labels: PaletteLabels) -> Self {
        self.labels = labels;
        self
    }

    /// The commands last run, newest first, kept by the host (to save them between sessions)
    /// instead of in egui's memory.
    pub fn recents(mut self, recents: &'r mut Vec<T>) -> Self {
        self.recents = Some(recents);
        self
    }
}

impl CommandPalette<'static, 'static, ()> {
    /// The palette's sizes and colours.
    pub fn style(tokens: &UiTokens) -> CommandPaletteStyle {
        let p = &tokens.component.command_palette;
        CommandPaletteStyle {
            width: p.width,
            top: p.top,
            pad: p.pad,
            row_height: p.row_height,
            max_rows: p.max_rows,
            recent: p.recent,
            scrim: tokens.color.surface.scrim.to_egui(),
            surface: Surface::style(tokens, SurfaceKind::Popup),
        }
    }
}

/// The rows shown, as the list box sees them.
struct Rows<'r, T> {
    shown: &'r [usize],
    entries: &'r [PaletteEntry<T>],
}

impl<T> ListOptions for Rows<'_, T> {
    fn len(&self) -> usize {
        self.shown.len()
    }

    fn option_at(&self, at: usize) -> usize {
        at
    }

    fn enabled(&self, option: usize) -> bool {
        self.entries[self.shown[option]].is_enabled()
    }

    fn text(&self, option: usize) -> std::borrow::Cow<'_, str> {
        self.entries[self.shown[option]].label().into()
    }
}

/// What a pass decided.
#[derive(Default)]
struct Outcome {
    chosen: Option<usize>,
    close: bool,
}

impl<'a, T: Clone + PartialEq + Send + Sync + 'static> CommandPalette<'a, '_, T> {
    /// Shows the palette while `open`; returns the entry chosen this pass, which the host runs.
    /// Choosing an entry or closing the box clears `open`.
    pub fn show(mut self, ctx: &Context, open: &mut bool) -> Option<&'a PaletteEntry<T>> {
        let state_id = self.id.with("state");
        if !*open {
            ctx.data_mut(|d| d.remove_temp::<State>(state_id));
            return None;
        }
        let mut state: State = ctx.data(|d| d.get_temp(state_id)).unwrap_or_default();
        let tokens = current(ctx);
        let style = CommandPalette::style(&tokens);
        let recents_id = self.id.with("recent");
        let recent: Vec<T> = match &self.recents {
            Some(host) => (*host).clone(),
            None => ctx.data(|d| d.get_temp(recents_id)).unwrap_or_default(),
        };
        let width = style.width.min(ctx.content_rect().width() - 2.0 * SCREEN_MARGIN).max(0.0);
        let area = egui::Modal::default_area(self.id).anchor(Align2::CENTER_TOP, vec2(0.0, style.top)).accessible_name(self.labels.name.clone());
        let mut outcome = Outcome::default();
        let shown = egui::Modal::new(self.id).area(area).backdrop_color(style.scrim).frame(Frame::NONE).show(ctx, |ui| {
            Surface::new(SurfaceKind::Popup).show(ui, |ui| {
                Frame::NONE.inner_margin(Margin::same(style.pad.round() as i8)).show(ui, |ui| {
                    ui.set_width((width - 2.0 * style.pad).max(0.0));
                    ui.spacing_mut().item_spacing.y = 0.0;
                    outcome = self.contents(ui, &style, &tokens, &mut state, &recent);
                });
            });
        });
        pass::record(ctx, Entry { id: self.id, layer: shown.response.layer_id, rect: shown.response.rect, modal: true });
        if shown.backdrop_response.clicked() {
            outcome.close = true;
        }
        ctx.data_mut(|d| d.insert_temp(state_id, state));
        let chosen = outcome.chosen.map(|i| &self.entries[i]);
        if let Some(entry) = chosen {
            let mut list = recent;
            list.retain(|c| c != entry.command());
            list.insert(0, entry.command().clone());
            list.truncate(style.recent);
            match self.recents.take() {
                Some(host) => *host = list,
                None => ctx.data_mut(|d| {
                    d.insert_temp(recents_id, list);
                }),
            }
        }
        if chosen.is_some() || outcome.close {
            *open = false;
            ctx.data_mut(|d| d.remove_temp::<State>(state_id));
        }
        chosen
    }

    /// The entries the list shows for `query`: the matches, or with no query the commands last
    /// run, else the first entries.
    fn shown(&self, query: &str, recent: &[T], style: &CommandPaletteStyle) -> Vec<usize> {
        if !query.trim().is_empty() {
            return CommandSearch::new(self.entries).query(query).dedup().results(MAX_RESULTS).into_iter().map(|m| m.index).collect();
        }
        let last: Vec<usize> = recent.iter().filter_map(|c| self.entries.iter().position(|e| e.command() == c)).take(style.recent).collect();
        if last.is_empty() { (0..self.entries.len().min(style.max_rows)).collect() } else { last }
    }

    fn contents(&self, ui: &mut Ui, style: &CommandPaletteStyle, tokens: &UiTokens, state: &mut State, recent: &[T]) -> Outcome {
        let mut outcome = Outcome::default();
        let id = self.id;
        let mut shown = self.shown(&state.query, recent, style);
        let mut listbox = Listbox::load(ui, id.with("list"));
        if !state.ready {
            listbox.reset();
            state.ready = true;
        }
        let options = Rows { shown: &shown, entries: self.entries };
        if listbox.active().is_none_or(|a| a >= options.len() || !options.enabled(a)) {
            listbox.reveal(Listbox::neighbour(&options, None, true), false);
        }
        move_active(ui, &mut listbox, &options, style.max_rows);
        let query_before = state.query.clone();

        let field = Interactive::id_in(ui, "query");
        let focused = ui.memory(|m| m.has_focus(field));
        // Only while the field lacks the focus: a request each pass interrupts an input method.
        if !focused && !ui.is_sizing_pass() {
            ui.memory_mut(|m| m.request_focus_with_filter(field, keys()));
            state.focus_requests += 1;
        }
        let edit = SearchField::new("query", &self.labels.search, &state.query, &self.labels.clear)
            .placeholder(&self.labels.placeholder)
            .size(ControlSize::Md)
            .keep_focus(true)
            .keys(keys())
            .show(ui);
        if let Some(text) = edit.event.as_ref().and_then(|e| e.value()) {
            state.query.clone_from(text);
        }
        let escaped = edit.escaped || (!focused && take_key(ui, Modifiers::NONE, Key::Escape));
        if state.query != query_before {
            shown = self.shown(&state.query, recent, style);
            listbox.reset();
            let options = Rows { shown: &shown, entries: self.entries };
            listbox.reveal(Listbox::neighbour(&options, None, true), false);
            ui.ctx().request_repaint();
        }

        ui.add_space(style.pad * 0.6);
        Divider::horizontal().show(ui);
        ui.add_space(style.pad * 0.6);

        let options = Rows { shown: &shown, entries: self.entries };
        let list_id = id.with("results");
        let count = if state.query.trim().is_empty() { None } else { Some(shown.len()) };
        let announcement = match count {
            Some(0) => Some(self.labels.empty.clone()),
            Some(n) => Some(self.labels.results.replace("{n}", &n.to_string())),
            None => None,
        };
        if let Some(text) = announcement {
            live::region(ui, id.with("count"), Role::Status, live::Politeness::Polite, |ui, labels| {
                labels.add(text);
                if shown.is_empty() {
                    ui.add(Text::new(self.labels.empty.as_str()).content(Content::Muted));
                }
            });
        }
        let clicked = self.list(ui, style, tokens, &options, &mut listbox, list_id);
        let active = listbox.active();
        ui.ctx().accesskit_node_builder(field, |n| {
            n.set_controls(vec![list_id.accesskit_id()]);
            match active {
                Some(a) => n.set_active_descendant(list_id.with(("option", a)).accesskit_id()),
                None => n.clear_active_descendant(),
            }
        });
        listbox.store(ui);

        ui.add_space(style.pad * 0.4);
        ui.add(Text::new(self.labels.hint.as_str()).role(TypeRole::Caption).content(Content::Faint));

        if let Some(row) = clicked {
            outcome.chosen = Some(shown[row]);
        } else if edit.submitted {
            // The press is taken so a dialog the command opens does not read it as its OK.
            take_key(ui, Modifiers::NONE, Key::Enter);
            outcome.chosen = active.filter(|&a| options.enabled(a)).map(|a| shown[a]);
        }
        outcome.close = escaped;
        outcome
    }

    /// The scrolling list of rows; returns the row clicked.
    fn list(
        &self,
        ui: &mut Ui,
        style: &CommandPaletteStyle,
        tokens: &UiTokens,
        options: &Rows<'_, T>,
        listbox: &mut Listbox,
        list_id: Id,
    ) -> Option<usize> {
        let menu_style = MenuStyle::new(tokens, ControlSize::Md);
        let rows = RowPainter { style: &menu_style, tokens, size: ControlSize::Md };
        let reveal = listbox.take_reveal();
        let pointer_moved = ui.input(|i| i.pointer.delta() != Vec2::ZERO);
        let height = style.row_height * style.max_rows.min(options.len()) as f32;
        let mut clicked = None;
        ui.scope_builder(UiBuilder::new().scope_id(list_id).accessibility_role(Role::ListBox), |ui| {
            ui.ctx().accesskit_node_builder(list_id, |n| n.set_label(self.labels.name.clone()));
            ScrollArea::vertical().id_salt(list_id.with("scroll")).max_height(height).auto_shrink([false, true]).show(ui, |ui| {
                let row_width = ui.available_width();
                for (at, &index) in options.shown.iter().enumerate() {
                    let entry = &self.entries[index];
                    let enabled = entry.is_enabled();
                    let content = RowContent {
                        label: entry.label(),
                        mnemonic: None,
                        icon: None,
                        kind: ItemKind::Plain,
                        shortcut: entry.shortcut_of(),
                        submenu: false,
                        tint: None,
                    };
                    let r = Interactive::with_id(list_id.with(("option", at)), Role::ListBoxOption, entry.label())
                        .sense(Sense::CLICK)
                        .enabled(enabled)
                        .selected(listbox.active() == Some(at))
                        .position_in_set(at)
                        .state_layer(false)
                        .focus_ring(false)
                        .interact(ui, vec2(row_width, style.row_height))
                        .finish(ui);
                    if enabled && r.contains_pointer() && pointer_moved {
                        listbox.set_active(Some(at));
                    }
                    if enabled && r.clicked() {
                        clicked = Some(at);
                    }
                    if reveal == Some(at) {
                        r.scroll_to_me(None);
                    }
                    let cols = Columns { marks: false, icons: false, width: row_width };
                    let state = RowState { enabled, active: listbox.active() == Some(at), open: false };
                    rows.paint(ui, r.rect, &content, &cols, state);
                    let detail = if entry.path_of().is_empty() {
                        entry.kind_of().unwrap_or_default().to_owned()
                    } else {
                        entry.path_of().join(PATH_SEPARATOR)
                    };
                    let gap = rows.style.shortcut_gap;
                    let shortcut = entry.shortcut_of().map_or(0.0, |k| rows.shortcut_width(ui, k) + gap);
                    let used = rows.width(ui, &RowContent { shortcut: None, ..content }, &cols);
                    rows.detail(ui, r.rect, &detail, shortcut, row_width - used - gap - shortcut, state);
                    menu::row::access(&r, &content, false);
                }
            });
        });
        clicked
    }
}

/// Room kept between the box and the window's sides.
const SCREEN_MARGIN: f32 = 16.0;

/// Up and Down move the active row and wrap; Page Up and Page Down move a page. Home and End stay
/// with the text field.
fn move_active<T>(ui: &Ui, listbox: &mut Listbox, options: &Rows<'_, T>, page: usize) {
    let down = take_key(ui, Modifiers::NONE, Key::ArrowDown);
    let up = take_key(ui, Modifiers::NONE, Key::ArrowUp);
    if down || up {
        let next = Listbox::neighbour(options, listbox.active(), down).or_else(|| Listbox::neighbour(options, None, down));
        listbox.reveal(next, true);
    }
    listbox.keys(ui, options, page.min(options.len().max(1)), false);
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["CommandPalette"];

#[derive(StoryArgs, Debug, PartialEq)]
struct PaletteArgs {
    open: bool,
    /// Starts with commands already run, as the palette's empty query lists them.
    recents: bool,
}

fn item(label: &str, command: &'static str) -> MenuEntry<&'static str> {
    MenuItem::new(label, command).into()
}

/// PhotoCraft's palette: the menu commands, its tools, a disabled command and a recent list.
fn entries() -> Vec<PaletteEntry<&'static str>> {
    use egui::{Key, KeyboardShortcut};
    let chord = |key| Kbd::new(&KeyboardShortcut::new(Modifiers::COMMAND, key));
    let menus = vec![
        MenuItem::submenu(
            "Edit",
            vec![
                MenuItem::new("Search", "edit.search").shortcut(chord(Key::F)).into(),
                MenuItem::new("Redo", "edit.redo").enabled(false).shortcut(chord(Key::Y)).into(),
            ],
        ),
        MenuItem::submenu(
            "Image",
            vec![
                MenuItem::submenu(
                    "Adjustments",
                    vec![item("Levels…", "image.levels"), item("Curves…", "image.curves"), item("Hue/Saturation…", "image.hue")],
                )
                .into(),
            ],
        ),
        MenuItem::submenu("Select", vec![MenuItem::new("All", "select.all").shortcut(chord(Key::A)).into(), item("Deselect", "select.none")]),
        MenuItem::submenu("Filter", vec![MenuItem::submenu("Blur", vec![item("Gaussian Blur…", "filter.gaussian")]).into()]),
    ];
    let mut entries = PaletteEntry::from_menu(&menus);
    entries.extend(
        [("tool:brush", "Brush"), ("tool:eraser", "Eraser"), ("tool:move", "Move")].map(|(id, label)| PaletteEntry::new(id, label).kind("Tool")),
    );
    entries
}

fn show_palette(ui: &mut Ui, args: &mut PaletteArgs) {
    if !crate::story::dialog_shown(ui, args.open) {
        return;
    }
    let ctx = ui.ctx().clone();
    let key = ui.make_persistent_id("craft_ui::patterns::command-palette-story");
    let (mut recents, mut last): (Vec<&'static str>, Option<String>) = ctx
        .data(|d| d.get_temp(key))
        .unwrap_or_else(|| (if args.recents { vec!["image.levels", "tool:brush", "edit.search"] } else { Vec::new() }, None));
    let entries = entries();
    let mut open = true;
    if let Some(entry) = CommandPalette::new("story-palette", &entries).recents(&mut recents).show(&ctx, &mut open) {
        last = Some(entry.label().to_owned());
    }
    ui.label(format!("Last run: {}", last.as_deref().unwrap_or("none")));
    ctx.data_mut(|d| d.insert_temp(key, (recents, last)));
    if !open {
        crate::story::dialog_closed(ui, &mut args.open);
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/command-palette/photocraft",
            Level::Patterns,
            "CommandPalette",
            "PhotoCraft",
            PaletteArgs { open: true, recents: false },
            show_palette
        ),
        typed_story!(
            "patterns/command-palette/recents",
            Level::Patterns,
            "CommandPalette",
            "Recent commands",
            PaletteArgs { open: true, recents: true },
            show_palette
        ),
    ]
}
