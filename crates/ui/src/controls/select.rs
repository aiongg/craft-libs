//! [`Select`]: one of a list of options, chosen from a popup list under a field-like box.

use std::collections::VecDeque;
use std::sync::Arc;

use craft_theme::{ControlSize, TypeRole};
use egui::accesskit::{self, Role};
use egui::emath::GuiRounding as _;
use egui::text::{LayoutJob, TextWrapping};
use egui::{
    AsIdSalt, Color32, CornerRadius, EventFilter, Galley, Id, IdSalt, Key, Modifiers, Popup, PopupCloseBehavior, Rect, Sense, Shape, Stroke, Ui,
    UiBuilder, Vec2, pos2, vec2,
};

use super::text_field::{BoxState, field_box};
use super::{SIZE_NAMES, SearchField, Swatch, TextField, TextFieldStyle, icon_size, size_arg};
use crate::behavior::{ListOptions, Listbox};
use crate::combo::Menu;
use crate::direction;
use crate::edit::{self, CancelReason, EditEvent, EditPolicy, EditResponse, Gesture, GestureHost, UndoMarks};
use crate::icons::{self, Icon};
use crate::primitives::{Content, Gap, Interactive, Row, Stack, Surface, SurfaceKind, SurfaceStyle, Text};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, corner_radius, role_format};

/// The rows of a popup list, from [`Select::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ListboxStyle {
    /// Height of a row and of a group header: `listbox.height`.
    pub height: f32,
    /// Between a row's edge and its contents: `listbox.pad_x`.
    pub pad_x: f32,
    /// Between the check mark, an icon and the text: `listbox.gap`.
    pub gap: f32,
    /// Side of the check mark and of an option's icon: `listbox.icon`.
    pub icon: f32,
    /// Above and below a separator's line: `listbox.separator_pad`.
    pub separator_pad: f32,
    /// `listbox.label` and `listbox.group_label`.
    pub label: TypeRole,
    pub group_label: TypeRole,
    /// Corners of the active row's fill: `listbox.radius`.
    pub radius: CornerRadius,
    /// `listbox.text.rest` and `listbox.text.disabled`.
    pub text: Color32,
    pub text_disabled: Color32,
    /// `listbox.active.fill` and `listbox.active.text`.
    pub active_fill: Color32,
    pub active_text: Color32,
    /// `listbox.chosen.text`, `listbox.mark`, `listbox.group` and `listbox.separator`.
    pub chosen_text: Color32,
    pub mark: Color32,
    pub group: Color32,
    pub separator: Color32,
}

/// A select's sizes and colours, from [`Select::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SelectStyle {
    /// The closed box: a text field's look ([`TextField::style`]), with the chevron in the
    /// placeholder colour.
    pub field: TextFieldStyle,
    /// The popup list's rows.
    pub list: ListboxStyle,
    /// The popup list's surface: [`SurfaceKind::Popup`].
    pub popup: SurfaceStyle,
}

/// Rows the popup list shows at most before it scrolls.
const MAX_ROWS: f32 = 12.0;

/// One of a list of options:
///
/// ```
/// use craft_ui::controls::Select;
/// # #[derive(Clone, Debug, PartialEq)] enum Blend { Normal, Multiply, Screen }
/// # egui::__run_test_ui(|ui| { let mut blend = Blend::Normal;
/// let edit = Select::new("blend", "Blend mode", &mut blend)
///     .option(Blend::Normal, "Normal")
///     .separator()
///     .option(Blend::Multiply, "Multiply")
///     .option(Blend::Screen, "Screen")
///     .show(ui);
/// if let Some(blend) = edit.committed() {
///     // The host applies it; with `preview(true)` it also gets Begin, Update and Cancel.
/// }
/// # });
/// ```
///
/// A field-like box shows the chosen option's text and a chevron at its reading end; it opens a
/// list of the options below it (above it when there is more room there), on the popup surface,
/// at least as wide as the box. The chosen option has a check mark. It is a WAI-ARIA select-only
/// combo box: the keyboard focus stays on the box while the list is open, and AccessKit names the
/// active option as the box's active descendant ([`Listbox`]).
///
/// - **Pointer**: the list opens on the press, so one gesture can press the box, drag onto an
///   option and release to choose it. A plain click opens the list and leaves it open for a
///   second click; a release over the box keeps it open, a release elsewhere after a drag closes
///   it. A press on the box while the list is open closes it.
/// - **Keys on the closed box**: Down, Up, Alt+Down, Enter and Space open the list on the chosen
///   option; Home and End open it on the first and last option; typing opens it on the first
///   option whose text starts with what was typed.
/// - **Keys in the open list**: the arrows, Page Up and Page Down, Home, End and type-ahead move
///   the active option, skipping disabled ones; Enter, Space, Alt+Up and Tab choose it (Tab then
///   moves on); Esc closes without a change.
/// - **Searchable** ([`searchable`](Select::searchable)): a search field at the top of the list
///   takes the focus when it opens and filters the options as the user types; the arrows, Page
///   Up, Page Down, Enter and Esc work from it, Home and End stay with its text.
/// - **Preview** ([`preview`](Select::preview)): opening the list begins an edit gesture, every
///   option made active (by the pointer or the keys) is an `Update`, choosing commits, and Esc or
///   closing the list without a choice cancels. Without preview, a choice is one `Set`.
///
/// The control writes what it reports into the value: updates, commits and sets, and the value
/// from before the gesture on a cancel.
pub struct Select<'a, T> {
    id_salt: IdSalt,
    label: String,
    value: &'a mut T,
    entries: Vec<Entry<T>>,
    rows: Option<Rows<'a, T>>,
    search: Option<(String, String)>,
    preview: bool,
    placeholder: String,
    size: ControlSize,
    policy: EditPolicy,
    enabled: bool,
}

/// `count` options of one height, as (value, text) from `row`.
struct Rows<'a, T> {
    count: usize,
    row: Box<dyn Fn(usize) -> (T, String) + 'a>,
}

enum Source<'a, T> {
    Entries(Vec<Entry<T>>),
    Rows(Rows<'a, T>),
}

enum Entry<T> {
    Option(Choice<T>),
    Separator,
    Group(String),
}

struct Choice<T> {
    value: T,
    text: String,
    icon: Option<Icon>,
    enabled: bool,
}

impl<'a, T: Clone + PartialEq + Send + Sync + 'static> Select<'a, T> {
    /// A select named `label` (its accessible name) choosing `value`; `id_salt` keeps its list
    /// state and gesture when the layout around it changes.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, value: &'a mut T) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            value,
            entries: Vec::new(),
            rows: None,
            search: None,
            preview: false,
            placeholder: String::new(),
            size: ControlSize::Md,
            policy: EditPolicy::default(),
            enabled: true,
        }
    }

    /// An option showing `text`, which chooses `value`.
    pub fn option(mut self, value: T, text: impl Into<String>) -> Self {
        self.entries.push(Entry::Option(Choice { value, text: text.into(), icon: None, enabled: true }));
        self
    }

    /// An option showing `icon` before `text`, which chooses `value`.
    pub fn option_icon(mut self, value: T, icon: Icon, text: impl Into<String>) -> Self {
        self.entries.push(Entry::Option(Choice { value, text: text.into(), icon: Some(icon), enabled: true }));
        self
    }

    /// Whether the last option added can be chosen; the keys skip a disabled one.
    pub fn option_enabled(mut self, enabled: bool) -> Self {
        if let Some(Entry::Option(choice)) = self.entries.iter_mut().rev().find(|e| matches!(e, Entry::Option(_))) {
            choice.enabled = enabled;
        }
        self
    }

    /// A line between the options before and after it.
    pub fn separator(mut self) -> Self {
        self.entries.push(Entry::Separator);
        self
    }

    /// A group of the options after it, up to the next group or separator, under the header
    /// `text`; AccessKit reports a group named `text`.
    pub fn group(mut self, text: impl Into<String>) -> Self {
        self.entries.push(Entry::Group(text.into()));
        self
    }

    /// `count` options of one height, from `row(index)` as (value, text), in place of
    /// [`option`](Select::option)s: a long list (fonts) that lays out only the rows in view.
    pub fn rows(mut self, count: usize, row: impl Fn(usize) -> (T, String) + 'a) -> Self {
        self.rows = Some(Rows { count, row: Box::new(row) });
        self
    }

    /// A search field at the top of the list that filters the options; `label` is its accessible
    /// name and placeholder ("Search fonts"), `clear_label` names its clear button.
    pub fn searchable(mut self, label: impl Into<String>, clear_label: impl Into<String>) -> Self {
        self.search = Some((label.into(), clear_label.into()));
        self
    }

    /// Reports the active option as a preview while the list is open (an edit gesture), so the
    /// host can show it; closing without a choice cancels.
    pub fn preview(mut self, preview: bool) -> Self {
        self.preview = preview;
        self
    }

    /// The text the box shows when the value is none of the options, in the placeholder colour.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// How a preview reports ([`EditPolicy`]); the default reports every update at once.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Shows the box, and the list while it is open, and returns this frame's edit. The response
    /// covers the box.
    pub fn show(self, ui: &mut Ui) -> EditResponse<T> {
        let Self { id_salt, label, value, entries, rows, search, preview, placeholder, size, policy, enabled } = self;
        let source = rows.map_or(Source::Entries(entries), Source::Rows);
        let tokens = Tokens::of(ui);
        let style = Select::style(&tokens, size);
        let id = Interactive::id_in(ui, id_salt);
        let enabled = enabled && ui.is_enabled();
        let mut state = PopupState::load(ui, id);
        let options = Options::new(&source);
        let chosen = options.find(value);

        let box_focused = ui.memory(|m| m.has_focus(id));
        let search_focused = state.search.is_some_and(|s| ui.memory(|m| m.has_focus(s)));
        let keys = EventFilter { horizontal_arrows: true, vertical_arrows: true, tab: false, escape: state.open };
        let mut listbox = Listbox::load(ui, id.with("list"));
        let shown = options.filter(if state.open { &state.query } else { "" });
        let active_before = listbox.active();
        let modifiers = ui.input(|i| i.modifiers);

        let mut action = None;
        if enabled && (box_focused || search_focused) {
            action = read_keys(ui, &mut listbox, &Shown { options: &options, shown: &shown }, &state, box_focused, chosen);
        }
        if !enabled && state.open {
            action = Some(Action::Close { reason: CancelReason::Disabled, focus: false });
        }

        let owners: Vec<Id> = std::iter::once(id).chain(state.search).collect();
        let mut gesture = Gesture::load(ui, id, &*value, policy).enabled(enabled).cancel_on_focus_loss(true).focus_owners(&owners).event_filter(keys);
        let start = gesture.start().cloned();

        let width = if ui.layout().horizontal_justify() { ui.available_width() } else { style.field.width };
        let (_, rect) = ui.allocate_space(vec2(width, style.field.height));
        let response = Interactive::with_id(id, Role::ComboBox, label.as_str())
            .expanded(state.open)
            .enabled(enabled)
            .state_layer(false)
            .focus_ring(false)
            .interact_at(ui, rect)
            .finish(ui);
        // A press, or a click whose press and release came in one frame.
        let pressed = enabled && ui.input(|i| i.pointer.primary_pressed()) && (response.is_pointer_button_down_on() || response.clicked());
        let reader_click = enabled && ui.input(|i| i.has_accesskit_action_request(id, accesskit::Action::Click));
        if action.is_none() && (pressed || reader_click) {
            action = Some(if state.open { Action::Close { reason: CancelReason::FocusLost, focus: pressed } } else { Action::Open(chosen) });
            state.pressing = pressed && !state.open;
        }

        let mut ctx =
            Pass { ui_id: id, keys, preview, modifiers, gesture: &mut gesture, listbox: &mut listbox, state: &mut state, options: &options };
        if let Some(action) = action {
            ctx.apply(ui, action, box_focused);
        }

        let mut drawn_active = None;
        if ctx.state.open {
            let popup_id = Popup::default_response_id(&response);
            let header = search.as_ref().map_or(0.0, |_| style.field.height + style.list.gap);
            let mut open = true;
            let menu = Menu::new(ui, &response)
                .open(&mut open)
                .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                .height(MAX_ROWS * style.list.height + header)
                .surface(style.popup);
            let list = List { style: &style, size, id, chosen, enabled, search: search.as_ref() };
            let picked = list.show(ui, menu, &mut ctx, &mut drawn_active);
            let released = ui.input(|i| i.pointer.primary_released() || !i.pointer.primary_down());
            if let Some(option) = picked {
                ctx.apply(ui, Action::Choose { option, focus: true }, box_focused);
            } else if !open {
                ctx.apply(ui, Action::Close { reason: CancelReason::FocusLost, focus: false }, box_focused);
            } else if ctx.state.pressing && released {
                // A release elsewhere ends a drag without a choice; one over the box or the list, or
                // a click, leaves the list open.
                let pos = ui.input(|i| i.pointer.interact_pos());
                let in_list = ui.memory(|m| m.area_rect(popup_id)).zip(pos).is_some_and(|(r, p)| r.contains(p));
                let on_box = pos.is_some_and(|p| response.rect.contains(p));
                if !in_list && !on_box && !ui.input(|i| i.pointer.any_click()) {
                    ctx.apply(ui, Action::Close { reason: CancelReason::FocusLost, focus: false }, box_focused);
                }
            }
            if ctx.state.pressing && released {
                ctx.state.pressing = false;
            }
            if ctx.state.open {
                ctx.keep_focus(ui, popup_id, box_focused, search_focused);
            }
        }
        if ctx.preview
            && ctx.gesture.is_active()
            && let Some(active) = ctx.listbox.active().filter(|a| Some(*a) != active_before)
        {
            ctx.gesture.update(options.value(active), modifiers);
        }

        let open = state.open;
        let search_id = state.search.filter(|_| open);
        listbox.store(ui);
        state.store(ui, id);
        let event = gesture.finish();
        let mut response = response;
        match &event {
            Some(EditEvent::Update { value: v, .. } | EditEvent::Set { value: v, .. }) => *value = v.clone(),
            Some(EditEvent::Commit { value: v, changed, .. }) => {
                *value = v.clone();
                if *changed {
                    response.mark_changed();
                }
            }
            Some(EditEvent::Cancel { reason, .. }) if *reason != CancelReason::External => {
                if let Some(start) = start {
                    *value = start;
                }
            }
            _ => {}
        }
        if matches!(event, Some(EditEvent::Set { .. })) {
            response.mark_changed();
        }

        let shown_option = options.find(value);
        let text = shown_option.map(|o| options.text(o));
        let focused = open || ui.memory(|m| m.has_focus(id));
        paint_box(
            ui,
            &style,
            &response,
            BoxContent { text: text.as_deref(), icon: shown_option.and_then(|o| options.icon(o)), placeholder: &placeholder, size, enabled, focused },
        );
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_value(text.clone().unwrap_or_default());
            node.set_expanded(open);
            match drawn_active.filter(|_| open) {
                Some(active) => node.set_active_descendant(option_id(id, active).accesskit_id()),
                None => node.clear_active_descendant(),
            }
        });
        if let (Some(search), Some(active)) = (search_id, drawn_active) {
            ui.ctx().accesskit_node_builder(search, |node| node.set_active_descendant(option_id(id, active).accesskit_id()));
        }
        EditResponse::new(response, event)
    }
}

impl Select<'static, ()> {
    pub fn style(tokens: &UiTokens, size: ControlSize) -> SelectStyle {
        let l = &tokens.component.listbox;
        SelectStyle {
            field: TextField::style(tokens, size),
            list: ListboxStyle {
                height: l.height.get(size),
                pad_x: l.pad_x,
                gap: l.gap,
                icon: l.icon.get(size),
                separator_pad: l.separator_pad,
                label: l.label,
                group_label: l.group_label,
                radius: corner_radius(l.radius),
                text: l.text.rest.to_egui(),
                text_disabled: l.text.disabled.to_egui(),
                active_fill: l.active_fill.to_egui(),
                active_text: l.active_text.to_egui(),
                chosen_text: l.chosen_text.to_egui(),
                mark: l.mark.to_egui(),
                group: l.group.to_egui(),
                separator: l.separator.to_egui(),
            },
            popup: Surface::style(tokens, SurfaceKind::Popup),
        }
    }
}

/// The id of option `option`'s row in the list of the select `select`.
fn option_id(select: Id, option: usize) -> Id {
    select.with(("option", option))
}

/// What a frame does to the list.
#[derive(Clone, Copy, Debug)]
enum Action {
    /// Opens the list with this option active.
    Open(Option<usize>),
    /// Closes the list without a choice; with `focus`, the box takes the focus.
    Close { reason: CancelReason, focus: bool },
    /// Chooses the option and closes the list; with `focus`, the box takes the focus back.
    Choose { option: usize, focus: bool },
}

/// The keys of a frame, while the box or the search field has the focus.
fn read_keys<T: Clone + PartialEq>(
    ui: &Ui,
    listbox: &mut Listbox,
    shown: &Shown<'_, '_, '_, T>,
    state: &PopupState,
    on_box: bool,
    chosen: Option<usize>,
) -> Option<Action> {
    let consume = |modifiers, key| ui.input_mut(|i| i.consume_key(modifiers, key));
    if state.open {
        let choose = |focus| listbox.active().filter(|&a| shown.options.enabled(a)).map(|option| Action::Choose { option, focus });
        if consume(Modifiers::NONE, Key::Escape) {
            return Some(Action::Close { reason: CancelReason::Escape, focus: true });
        }
        let space = on_box && !listbox.typing(ui) && consume(Modifiers::NONE, Key::Space);
        if consume(Modifiers::ALT, Key::ArrowUp) || consume(Modifiers::NONE, Key::Enter) || space {
            return choose(true);
        }
        if ui.input(|i| i.key_pressed(Key::Tab)) {
            return choose(false);
        }
        listbox.keys(ui, shown, state.page, on_box);
        if on_box {
            listbox.type_ahead(ui, shown);
        }
        return None;
    }
    if !on_box {
        return None;
    }
    let first = shown.shown.iter().copied().find(|&o| shown.options.enabled(o));
    let last = shown.shown.iter().rev().copied().find(|&o| shown.options.enabled(o));
    let opening = [(Modifiers::ALT, Key::ArrowDown), (Modifiers::NONE, Key::ArrowDown), (Modifiers::NONE, Key::ArrowUp)]
        .into_iter()
        .chain([(Modifiers::NONE, Key::Enter), (Modifiers::NONE, Key::Space)]);
    for (modifiers, key) in opening {
        if consume(modifiers, key) {
            return Some(Action::Open(chosen.or(first)));
        }
    }
    if consume(Modifiers::NONE, Key::Home) {
        return Some(Action::Open(first));
    }
    if consume(Modifiers::NONE, Key::End) {
        return Some(Action::Open(last));
    }
    listbox.set_active(chosen);
    listbox.type_ahead(ui, shown).then(|| Action::Open(listbox.active()))
}

/// What the list keeps between frames.
#[derive(Clone, Debug, Default)]
struct PopupState {
    open: bool,
    /// The press that opened the list is still down.
    pressing: bool,
    query: String,
    /// The search field takes the focus when it is next drawn.
    focus_search: bool,
    /// The search field's id, once drawn.
    search: Option<Id>,
    /// Rows the list shows at once.
    page: usize,
}

impl PopupState {
    fn load(ui: &Ui, id: Id) -> Self {
        ui.data(|d| d.get_temp::<Self>(Self::key(id))).unwrap_or_default()
    }

    fn store(self, ui: &Ui, id: Id) {
        ui.data_mut(|d| d.insert_temp(Self::key(id), self));
    }

    fn key(id: Id) -> Id {
        id.with("craft_ui::controls::select")
    }
}

/// Everything a frame's actions change.
struct Pass<'f, 's, 'a, T> {
    ui_id: Id,
    keys: EventFilter,
    preview: bool,
    modifiers: Modifiers,
    gesture: &'f mut Gesture<T>,
    listbox: &'f mut Listbox,
    state: &'f mut PopupState,
    options: &'f Options<'s, 'a, T>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> Pass<'_, '_, '_, T> {
    fn apply(&mut self, ui: &Ui, action: Action, box_focused: bool) {
        match action {
            Action::Open(active) => {
                self.state.open = true;
                self.state.query.clear();
                self.state.focus_search = true;
                self.state.search = None;
                self.listbox.reveal(active);
                if self.preview {
                    self.gesture.begin();
                }
                if !box_focused {
                    ui.memory_mut(|m| m.request_focus_with_filter(self.ui_id, self.keys));
                }
                ui.ctx().request_repaint();
            }
            Action::Close { reason, focus } => {
                if self.gesture.is_active() {
                    self.gesture.cancel(reason);
                }
                self.close(ui, focus && !box_focused);
            }
            Action::Choose { option, focus } => {
                let value = self.options.value(option);
                if self.gesture.is_active() {
                    self.gesture.update(value, self.modifiers);
                    self.gesture.commit();
                } else {
                    self.gesture.set(value);
                }
                self.close(ui, focus && !box_focused);
            }
        }
    }

    fn close(&mut self, ui: &Ui, focus_box: bool) {
        self.state.open = false;
        self.state.pressing = false;
        self.state.query.clear();
        self.state.search = None;
        self.listbox.reset();
        if focus_box {
            ui.memory_mut(|m| m.request_focus_with_filter(self.ui_id, self.keys));
        }
    }

    /// Keeps the focus with the list when a press inside it took the focus away (a disabled
    /// option, a group header): egui drops the focus on a click elsewhere.
    fn keep_focus(&mut self, ui: &Ui, popup: Id, box_focused: bool, search_focused: bool) {
        let pressed_in = ui.input(|i| (i.pointer.any_pressed() || i.pointer.any_click()).then(|| i.pointer.interact_pos()).flatten());
        let in_list = ui.memory(|m| m.area_rect(popup)).zip(pressed_in).is_some_and(|(r, p)| r.contains(p));
        let focused = ui.memory(|m| m.focused());
        if !in_list || focused.is_some() {
            return;
        }
        match (box_focused, search_focused.then_some(self.state.search).flatten()) {
            (true, _) => ui.memory_mut(|m| m.request_focus_with_filter(self.ui_id, self.keys)),
            (false, Some(search)) => ui.memory_mut(|m| m.request_focus(search)),
            (false, None) => {}
        }
    }
}

/// The options of a select, by the index each has among them.
struct Options<'s, 'a, T> {
    source: &'s Source<'a, T>,
    /// The options of a list of entries; empty for rows.
    choices: Vec<&'s Choice<T>>,
}

impl<'s, 'a, T: Clone + PartialEq> Options<'s, 'a, T> {
    fn new(source: &'s Source<'a, T>) -> Self {
        let choices = match source {
            Source::Entries(entries) => entries
                .iter()
                .filter_map(|e| match e {
                    Entry::Option(choice) => Some(choice),
                    _ => None,
                })
                .collect(),
            Source::Rows(_) => Vec::new(),
        };
        Self { source, choices }
    }

    fn len(&self) -> usize {
        match self.source {
            Source::Entries(_) => self.choices.len(),
            Source::Rows(rows) => rows.count,
        }
    }

    fn value(&self, option: usize) -> T {
        match self.source {
            Source::Entries(_) => self.choices[option].value.clone(),
            Source::Rows(rows) => (rows.row)(option).0,
        }
    }

    fn text(&self, option: usize) -> String {
        match self.source {
            Source::Entries(_) => self.choices[option].text.clone(),
            Source::Rows(rows) => (rows.row)(option).1,
        }
    }

    fn icon(&self, option: usize) -> Option<Icon> {
        self.choices.get(option).and_then(|c| c.icon)
    }

    fn enabled(&self, option: usize) -> bool {
        self.choices.get(option).is_none_or(|c| c.enabled)
    }

    /// Whether any option has an icon, so every row keeps room for one.
    fn has_icons(&self) -> bool {
        self.choices.iter().any(|c| c.icon.is_some())
    }

    /// The option whose value is `value`.
    fn find(&self, value: &T) -> Option<usize> {
        match self.source {
            Source::Entries(_) => self.choices.iter().position(|c| c.value == *value),
            Source::Rows(rows) => (0..rows.count).find(|&i| (rows.row)(i).0 == *value),
        }
    }

    /// The options whose text contains `query`, ignoring case; all of them for an empty query.
    fn filter(&self, query: &str) -> Vec<usize> {
        let query = query.trim().to_lowercase();
        (0..self.len()).filter(|&o| query.is_empty() || self.text(o).to_lowercase().contains(&query)).collect()
    }
}

/// The options a frame shows, for the [`Listbox`].
struct Shown<'o, 's, 'a, T> {
    options: &'o Options<'s, 'a, T>,
    shown: &'o [usize],
}

impl<T: Clone + PartialEq> ListOptions for Shown<'_, '_, '_, T> {
    fn shown(&self) -> &[usize] {
        self.shown
    }

    fn enabled(&self, option: usize) -> bool {
        self.options.enabled(option)
    }

    fn text(&self, option: usize) -> String {
        self.options.text(option)
    }
}

/// A line of a list of entries, after a search filtered it.
enum Line<'s> {
    Option(usize),
    Separator,
    Group { text: &'s str, options: Vec<usize> },
}

/// The lines showing `shown`: groups with none of their options shown go, and so do separators
/// while a search filters.
fn lines<'s, T>(source: &'s Source<'_, T>, shown: &[usize], filtering: bool) -> Vec<Line<'s>> {
    let Source::Entries(entries) = source else { return shown.iter().map(|&o| Line::Option(o)).collect() };
    let mut out = Vec::new();
    let mut group: Option<(&str, Vec<usize>)> = None;
    let mut option = 0;
    let flush = |group: &mut Option<(&'s str, Vec<usize>)>, out: &mut Vec<Line<'s>>| {
        if let Some((text, options)) = group.take()
            && !options.is_empty()
        {
            out.push(Line::Group { text, options });
        }
    };
    for entry in entries {
        match entry {
            Entry::Option(_) => {
                if shown.contains(&option) {
                    match &mut group {
                        Some((_, options)) => options.push(option),
                        None => out.push(Line::Option(option)),
                    }
                }
                option += 1;
            }
            Entry::Separator => {
                flush(&mut group, &mut out);
                if !filtering && !out.is_empty() && !matches!(out.last(), Some(Line::Separator)) {
                    out.push(Line::Separator);
                }
            }
            Entry::Group(text) => {
                flush(&mut group, &mut out);
                group = Some((text, Vec::new()));
            }
        }
    }
    flush(&mut group, &mut out);
    if matches!(out.last(), Some(Line::Separator)) {
        out.pop();
    }
    out
}

/// The popup list of a frame.
struct List<'l> {
    style: &'l SelectStyle,
    size: ControlSize,
    id: Id,
    chosen: Option<usize>,
    enabled: bool,
    search: Option<&'l (String, String)>,
}

impl List<'_> {
    /// Shows the list in `menu` and returns the option chosen in it this frame; `drawn_active` is
    /// the active option when its row was drawn.
    fn show<T: Clone + PartialEq + Send + Sync + 'static>(
        &self,
        ui: &Ui,
        menu: Menu<'_>,
        pass: &mut Pass<'_, '_, '_, T>,
        drawn_active: &mut Option<usize>,
    ) -> Option<usize> {
        let mut picked = None;
        let pointer_moved = ui.input(|i| i.pointer.delta() != Vec2::ZERO);
        let released = ui.input(|i| i.pointer.primary_released());
        let options = pass.options;
        let pressing = pass.state.pressing;
        let icons = options.has_icons();
        let mut query = pass.state.query.clone();
        // The field takes the focus once its id is known from the pass before, so the gesture
        // counts it among the select's focus owners from the start.
        let focus_search = pass.state.focus_search && pass.state.search.is_some();
        let mut search = pass.state.search;
        let mut focused_search = false;
        let mut page = pass.state.page;
        let listbox = &mut *pass.listbox;
        let keys = pass.keys;
        let header = |ui: &mut Ui| {
            let before = query.clone();
            if let Some((label, clear)) = self.search {
                let salt = "craft-select-search";
                let field = Interactive::id_in(ui, IdSalt::new(salt));
                // egui drops a focus given during a pass that sizes the popup (its widgets are
                // disabled) or one with a click outside the field (the click that opened the list).
                if focus_search && !ui.is_sizing_pass() && !ui.input(|i| i.pointer.any_click()) {
                    ui.memory_mut(|m| m.request_focus_with_filter(field, keys));
                    focused_search = true;
                }
                let edit =
                    SearchField::new(salt, label.as_str(), &query, clear.as_str()).placeholder(label.as_str()).size(self.size).keys(keys).show(ui);
                if let Some(text) = edit.event.as_ref().and_then(|e| e.value()) {
                    query.clone_from(text);
                }
                search = Some(field);
                ui.add_space(self.style.list.gap);
            }
            // Rows touch: the row height alone spaces them, as `show_rows` counts it.
            ui.spacing_mut().item_spacing.y = 0.0;
            Filtered { shown: options.filter(&query), changed: query != before, filtering: !query.trim().is_empty() }
        };
        let mut option_row = |ui: &mut Ui, option: usize, listbox: &mut Listbox| {
            let text = options.text(option);
            let enabled = self.enabled && options.enabled(option);
            let size = vec2(ui.available_width().max(self.row_width(ui, &text, icons)), self.style.list.height);
            let response = Interactive::with_id(option_id(self.id, option), Role::ListBoxOption, text.as_str())
                .sense(Sense::CLICK)
                .selected(self.chosen == Some(option))
                .enabled(enabled)
                .state_layer(false)
                .focus_ring(false)
                .interact(ui, size)
                .finish(ui);
            if enabled && response.contains_pointer() && (pointer_moved || pressing) {
                listbox.set_active(Some(option));
            }
            if enabled && (response.clicked() || (pressing && released && response.contains_pointer())) {
                picked = Some(option);
            }
            let active = listbox.active() == Some(option);
            if active {
                *drawn_active = Some(option);
            }
            let look = RowLook { text: &text, icon: options.icon(option), icons, chosen: self.chosen == Some(option), active, enabled };
            paint_row(ui, &self.style.list, self.size, response.rect, &look);
            response.rect
        };
        let row_height = self.style.list.height;
        let measure = |ui: &Ui| (ui.clip_rect().height() / row_height).floor().max(1.0) as usize;
        match options.source {
            Source::Entries(_) => {
                let _ = menu.show_ui_with_header(header, |ui, filtered| {
                    keep_active_shown(listbox, options, &filtered);
                    // The popup's first pass measures it unseen; scroll once it shows.
                    let reveal = if ui.is_sizing_pass() { None } else { listbox.take_reveal() };
                    page = measure(ui);
                    let mut add = |ui: &mut Ui, option: usize, listbox: &mut Listbox| {
                        let rect = option_row(ui, option, listbox);
                        if reveal == Some(option) {
                            ui.scroll_to_rect(rect, None);
                        }
                    };
                    for line in lines(options.source, &filtered.shown, filtered.filtering) {
                        match line {
                            Line::Option(option) => add(ui, option, listbox),
                            Line::Separator => self.separator(ui),
                            Line::Group { text, options: members } => {
                                let builder = UiBuilder::new().accessibility_role(Role::Group).accessibility_label(text);
                                ui.scope_builder(builder, |ui| {
                                    self.group_header(ui, text);
                                    for option in members {
                                        add(ui, option, listbox);
                                    }
                                });
                            }
                        }
                    }
                });
            }
            Source::Rows(_) => {
                let total = |filtered: &Filtered| filtered.shown.len();
                let _ = menu.show_rows_with_header(header, row_height, total, |ui, range, filtered| {
                    keep_active_shown(listbox, options, &filtered);
                    page = measure(ui);
                    let reveal = if ui.is_sizing_pass() { None } else { listbox.take_reveal() };
                    if let Some(p) = reveal.and_then(|option| filtered.shown.iter().position(|&o| o == option)) {
                        // The row may be out of view and not laid out: scroll to where it would be.
                        let frame = ui.max_rect();
                        let y = frame.top() + (p as f32 - range.start as f32) * row_height;
                        ui.scroll_to_rect(Rect::from_min_size(pos2(frame.left(), y), vec2(frame.width(), row_height)), None);
                    }
                    for p in range {
                        if let Some(&option) = filtered.shown.get(p) {
                            option_row(ui, option, listbox);
                        }
                    }
                });
            }
        }
        pass.state.query = query;
        pass.state.search = search;
        pass.state.page = page;
        if focused_search {
            pass.state.focus_search = false;
        } else if pass.state.focus_search && self.search.is_some() {
            ui.ctx().request_repaint();
        }
        picked
    }

    /// The width a row needs: its text, the check mark and the icon with their gaps, and the
    /// padding.
    fn row_width(&self, ui: &Ui, text: &str, icons: bool) -> f32 {
        let s = &self.style.list;
        let galley = text_galley(ui, s.label, text, Color32::PLACEHOLDER, f32::INFINITY);
        2.0 * s.pad_x + text_offset(s, icons) + galley.size().x
    }

    fn separator(&self, ui: &mut Ui) {
        let s = &self.style.list;
        let (_, rect) = ui.allocate_space(vec2(ui.available_width(), 2.0 * s.separator_pad + 1.0));
        let y = rect.center().y;
        ui.painter().hline(rect.x_range(), y, Stroke::new(1.0, s.separator));
    }

    fn group_header(&self, ui: &mut Ui, text: &str) {
        let s = &self.style.list;
        let (_, rect) = ui.allocate_space(vec2(ui.available_width(), s.height));
        let galley = text_galley(ui, s.group_label, text, s.group, rect.width() - 2.0 * s.pad_x);
        let x = direction::start_x(ui, rect) + direction::forward(ui) * s.pad_x;
        let x = if direction::is_rtl(ui) { x - galley.size().x } else { x };
        let pos = pos2(x, rect.center().y - galley.size().y / 2.0).round_to_pixels(ui.pixels_per_point());
        ui.painter().galley(pos, galley, s.group);
    }
}

/// The options a search leaves shown.
struct Filtered {
    shown: Vec<usize>,
    /// The search changed this frame.
    changed: bool,
    /// A search is under way.
    filtering: bool,
}

/// After a search changed, or when the active option is no longer shown, the first enabled
/// option shown becomes active.
fn keep_active_shown<T: Clone + PartialEq>(listbox: &mut Listbox, options: &Options<'_, '_, T>, filtered: &Filtered) {
    let hidden = listbox.active().is_some_and(|a| !filtered.shown.contains(&a));
    if filtered.changed || hidden {
        listbox.reveal(filtered.shown.iter().copied().find(|&o| options.enabled(o)));
    }
}

/// How one row is drawn.
struct RowLook<'r> {
    text: &'r str,
    icon: Option<Icon>,
    /// Every row keeps room for an icon.
    icons: bool,
    chosen: bool,
    active: bool,
    enabled: bool,
}

/// From a row's reading start to its text, inside the padding: the check mark and, when the list
/// has icons, an icon, each followed by the gap.
fn text_offset(s: &ListboxStyle, icons: bool) -> f32 {
    (s.icon + s.gap) * if icons { 2.0 } else { 1.0 }
}

fn paint_row(ui: &Ui, s: &ListboxStyle, size: ControlSize, rect: Rect, look: &RowLook<'_>) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    let ppp = ui.pixels_per_point();
    if look.active && look.enabled {
        ui.painter().rect_filled(rect.round_to_pixels(ppp), s.radius, s.active_fill);
    }
    let (text, mark) = match () {
        () if !look.enabled => (s.text_disabled, s.text_disabled),
        () if look.active => (s.active_text, s.active_text),
        () if look.chosen => (s.chosen_text, s.mark),
        () => (s.text, s.mark),
    };
    let forward = direction::forward(ui);
    let start = direction::start_x(ui, rect) + forward * s.pad_x;
    let center = |from_start: f32| pos2(start + forward * (from_start + s.icon / 2.0), rect.center().y);
    let square = |from_start: f32| Rect::from_center_size(center(from_start), vec2(s.icon, s.icon)).round_to_pixels(ppp);
    if look.chosen {
        icons::paint(ui, square(0.0), Icon::Check, icon_size(size), mark);
    }
    if let Some(icon) = look.icon.filter(|_| look.icons) {
        icons::paint(ui, square(s.icon + s.gap), icon, icon_size(size), text);
    }
    let offset = text_offset(s, look.icons);
    let galley = text_galley(ui, s.label, look.text, text, f32::INFINITY);
    let x = start + forward * offset;
    let x = if direction::is_rtl(ui) { x - galley.size().x } else { x };
    let pos = pos2(x, rect.center().y - galley.size().y / 2.0).round_to_pixels(ppp);
    ui.painter().galley(pos, galley, text);
}

/// `text` in `role` and `color` on one line, cut with an ellipsis past `width`.
fn text_galley(ui: &Ui, role: TypeRole, text: &str, color: Color32, width: f32) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(text.to_owned(), role_format(&Tokens::of(ui), role, color));
    job.wrap = TextWrapping::truncate_at_width(width.max(0.0));
    ui.painter().layout_job(job)
}

/// What the closed box shows.
struct BoxContent<'b> {
    text: Option<&'b str>,
    icon: Option<Icon>,
    placeholder: &'b str,
    size: ControlSize,
    enabled: bool,
    focused: bool,
}

/// The box: a text field's box with the chosen option's icon and text from the reading start and
/// the chevron at the reading end.
fn paint_box(ui: &Ui, style: &SelectStyle, response: &egui::Response, content: BoxContent<'_>) {
    let f = &style.field;
    let rect = response.rect;
    if !ui.is_rect_visible(rect) {
        return;
    }
    let state = BoxState { enabled: content.enabled, focused: content.focused, hovered: response.contains_pointer(), invalid: false };
    ui.painter().add(Shape::Vec(field_box(ui, f, rect, state)));
    let ppp = ui.pixels_per_point();
    let forward = direction::forward(ui);
    let (text_color, muted) = if content.enabled { (f.text, f.placeholder) } else { (f.text_disabled, f.placeholder_disabled) };
    let square = |x: f32| Rect::from_center_size(pos2(x, rect.center().y), vec2(f.icon, f.icon)).round_to_pixels(ppp);
    let end = direction::end_x(ui, rect) - forward * (f.pad_x + f.icon / 2.0);
    icons::paint(ui, square(end), Icon::ChevronDown, icon_size(content.size), muted);
    let mut start = direction::start_x(ui, rect) + forward * f.pad_x;
    if let Some(icon) = content.icon {
        icons::paint(ui, square(start + forward * f.icon / 2.0), icon, icon_size(content.size), text_color);
        start += forward * (f.icon + f.gap);
    }
    let (text, color) = match content.text {
        Some(text) => (text, text_color),
        None => (content.placeholder, muted),
    };
    let room = (end - forward * (f.icon / 2.0 + f.gap) - start).abs();
    let galley = text_galley(ui, f.role, text, color, room);
    let x = if direction::is_rtl(ui) { start - galley.size().x } else { start };
    let pos = pos2(x, rect.center().y - galley.size().y / 2.0).round_to_pixels(ppp);
    ui.painter().galley(pos, galley, color);
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Select"];

pub(crate) fn stories() -> Vec<Story> {
    let args = || Args::new().with_choice("size", SIZE_NAMES, 1).with_bool("disabled", false);
    vec![
        Story { id: "controls/select/states", level: Level::Controls, component: "Select", name: "States", args, show: show_states },
        Story {
            id: "controls/select/preview",
            level: Level::Controls,
            component: "Select",
            name: "Preview",
            args: || Args::new().with_choice("size", SIZE_NAMES, 1).with_bool("preview", true),
            show: show_preview,
        },
        Story {
            id: "controls/select/searchable",
            level: Level::Controls,
            component: "Select",
            name: "Searchable",
            args: || Args::new().with_choice("size", SIZE_NAMES, 1),
            show: show_searchable,
        },
    ]
}

/// `label` at the reading start of a row, then the control.
fn row(ui: &mut Ui, label: &str, control: impl FnOnce(&mut Ui)) {
    Row::new(Gap::Inline).show(ui, |ui| {
        ui.add(Text::new(label).content(Content::Muted));
        control(ui);
    });
}

/// The stories' values, kept between frames.
fn with_choices<const N: usize>(ui: &mut Ui, key: &str, initial: [usize; N], show: impl FnOnce(&mut Ui, &mut [usize; N])) {
    let id = ui.scope_id().with(key);
    let mut choices = ui.data(|d| d.get_temp::<[usize; N]>(id).unwrap_or(initial));
    show(ui, &mut choices);
    ui.data_mut(|d| d.insert_temp(id, choices));
}

const UNITS: [&str; 4] = ["Pixels", "Inches", "Centimetres", "Points"];

/// Plain, grouped with a separator and a disabled option, with icons, with no option chosen, and
/// disabled.
fn show_states(ui: &mut Ui, args: &mut Args) {
    let (size, enabled) = (size_arg(args.choice("size")), !args.bool("disabled"));
    with_choices(ui, "select-states", [0, 1, 0, usize::MAX], |ui, choices| {
        Stack::new(Gap::Stack).show(ui, |ui| {
            let [units, sampling, tool, empty] = choices;
            row(ui, "Units", |ui| {
                let mut select = Select::new("select-units", "Units", units).size(size).enabled(enabled);
                for (i, unit) in UNITS.into_iter().enumerate() {
                    select = select.option(i, unit);
                }
                let _ = select.show(ui);
            });
            row(ui, "Resampling", |ui| {
                let _ = Select::new("select-resampling", "Resampling", sampling)
                    .size(size)
                    .enabled(enabled)
                    .option(0, "Automatic")
                    .separator()
                    .group("Smooth")
                    .option(1, "Bicubic")
                    .option(2, "Bilinear")
                    .group("Sharp")
                    .option(3, "Nearest neighbour")
                    .option(4, "Preserve details (AI)")
                    .option_enabled(false)
                    .show(ui);
            });
            row(ui, "Tool", |ui| {
                let _ = Select::new("select-tool", "Tool", tool)
                    .size(size)
                    .enabled(enabled)
                    .option_icon(0, Icon::Brush, "Brush")
                    .option_icon(1, Icon::Eraser, "Eraser")
                    .option_icon(2, Icon::Eyedropper, "Eyedropper")
                    .show(ui);
            });
            row(ui, "No choice", |ui| {
                let mut select = Select::new("select-empty", "Profile", empty).size(size).enabled(enabled).placeholder("Choose a profile");
                for (i, name) in ["sRGB", "Display P3", "Adobe RGB"].into_iter().enumerate() {
                    select = select.option(i, name);
                }
                let _ = select.show(ui);
            });
            row(ui, "Disabled", |ui| {
                let mut fixed = 2;
                let mut select = Select::new("select-disabled", "Locked units", &mut fixed).size(size).enabled(false);
                for (i, unit) in UNITS.into_iter().enumerate() {
                    select = select.option(i, unit);
                }
                let _ = select.show(ui);
            });
        });
    });
}

const BLEND_MODES: [(&str, [u8; 3]); 8] = [
    ("Normal", [96, 140, 210]),
    ("Darken", [40, 70, 120]),
    ("Multiply", [30, 50, 100]),
    ("Lighten", [150, 190, 240]),
    ("Screen", [180, 210, 250]),
    ("Overlay", [70, 120, 200]),
    ("Soft light", [110, 150, 210]),
    ("Difference", [200, 120, 60]),
];

/// The preview story's document: a blend mode and an undo history of whole snapshots.
#[derive(Clone, Debug, Default)]
struct BlendDoc {
    mode: usize,
    undo: Vec<usize>,
}

impl UndoMarks for BlendDoc {
    type Mark = usize;

    fn undo_mark(&self) -> usize {
        self.undo.len()
    }

    fn revert_to(&mut self, mark: usize) {
        if let Some(&mode) = self.undo.get(mark) {
            self.mode = mode;
            self.undo.truncate(mark);
        }
    }

    fn squash_since(&mut self, mark: usize) {
        self.undo.truncate(mark + 1);
    }
}

#[derive(Clone, Debug, Default)]
struct PreviewState {
    doc: BlendDoc,
    host: GestureHost<BlendDoc>,
    /// Newest first.
    log: VecDeque<String>,
}

/// Events kept in the preview story's log.
const LOG_LEN: usize = 12;

/// Blend modes under a [`GestureHost`]: a swatch shows the document's colour, which follows the
/// active mode while the list previews; the events newest first and the undo-step count.
fn show_preview(ui: &mut Ui, args: &mut Args) {
    let size = size_arg(args.choice("size"));
    let preview = args.bool("preview");
    let state_id = ui.scope_id().with("select-preview");
    let mut state = ui.data(|d| d.get_temp::<PreviewState>(state_id)).unwrap_or_default();
    Stack::new(Gap::Stack).show(ui, |ui| {
        row(ui, "Blend mode", |ui| {
            let mut mode = state.doc.mode;
            let mut select = Select::new("select-blend", "Blend mode", &mut mode).size(size).preview(preview);
            for (i, (name, _)) in BLEND_MODES.iter().enumerate() {
                select = select.option(i, *name);
                if i == 0 || i == 2 || i == 4 {
                    select = select.separator();
                }
            }
            let edit = select.show(ui);
            if let Some(event) = edit.event {
                state.log.push_front(edit::describe(&event.clone().map(|i| BLEND_MODES[i].0)));
                state.log.truncate(LOG_LEN);
                state.host.handle(ui.ctx(), &mut state.doc, event, |doc, mode| {
                    doc.undo.push(doc.mode);
                    doc.mode = mode;
                });
            }
            let [r, g, b] = BLEND_MODES[state.doc.mode].1;
            ui.add(Swatch::new("select-blend-result", "Result", Color32::from_rgb(r, g, b)));
        });
        if let Some(event) = state.host.end_frame(ui.ctx(), &mut state.doc) {
            state.log.push_front(format!("host: {}", edit::describe(&event)));
        }
        let status = format!("{}, {} undo steps", BLEND_MODES[state.doc.mode].0, state.doc.undo.len());
        ui.add(Text::new(status).content(Content::Muted));
        Surface::new(SurfaceKind::Sunken).show(ui, |ui| {
            ui.set_min_width(LOG_WIDTH);
            if state.log.is_empty() {
                ui.add(Text::new("No events yet").content(Content::Faint));
            }
            for line in &state.log {
                ui.add(Text::new(line.as_str()).role(TypeRole::Mono));
            }
        });
    });
    ui.data_mut(|d| d.insert_temp(state_id, state));
}

const LOG_WIDTH: f32 = 360.0;

const FONT_STEMS: [&str; 12] =
    ["Garamond", "Helvetica", "Futura", "Baskerville", "Bodoni", "Caslon", "Didot", "Frutiger", "Gill", "Optima", "Rockwell", "Univers"];
const FONT_STYLES: [&str; 6] = ["", " Light", " Medium", " Bold", " Condensed", " Display"];

/// A searchable font list of 72 rows, laid out only in view.
fn show_searchable(ui: &mut Ui, args: &mut Args) {
    let size = size_arg(args.choice("size"));
    let font = |i: usize| format!("{}{}", FONT_STEMS[i / FONT_STYLES.len()], FONT_STYLES[i % FONT_STYLES.len()]);
    with_choices(ui, "select-searchable", [6], |ui, choices| {
        row(ui, "Font", |ui| {
            let _ = Select::new("select-font", "Font", &mut choices[0])
                .size(size)
                .rows(FONT_STEMS.len() * FONT_STYLES.len(), |i| (i, font(i)))
                .searchable("Search fonts", "Clear")
                .show(ui);
        });
    });
}
