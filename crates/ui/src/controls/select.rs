//! [`Select`]: one of a list of options, chosen from a popup list under a field-like box.

use std::borrow::Cow;
use std::collections::VecDeque;
use std::sync::Arc;

use craft_theme::{ControlSize, TypeRole};
use egui::accesskit::{self, HasPopup, Role};
use egui::emath::GuiRounding as _;
use egui::{
    AsIdSalt, Color32, CornerRadius, EventFilter, Id, IdSalt, Key, Modifiers, Popup, PopupCloseBehavior, Rect, Shape, Stroke, Ui, UiBuilder, pos2,
    vec2,
};

use super::field_wheel;
use super::option_list::{
    MAX_ROWS, OptionRow, OptionRows, keep_focus, load_kept, option_id, paint_chevron, pressed_on, released, released_away, rows_in_view, store_kept,
    text_galley,
};
use super::text_field::{BoxState, field_box};
use super::{SIZE_NAMES, SearchField, Swatch, TextField, TextFieldStyle, icon_size, size_arg};
use crate::behavior::{ListOptions, Listbox, take_composing_keys, take_key};
use crate::direction;
use crate::edit::{self, CancelReason, EditEvent, EditPolicy, EditResponse, Gesture, GestureHost, GestureId, UndoMarks};
use crate::icons::{self, Icon};
use crate::popup_list::{self, PopupList};
use crate::primitives::{Content, FocusRing, Gap, Interactive, Row, Stack, Surface, SurfaceKind, SurfaceStyle, Text};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, corner_radius};

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
    /// The keyboard's active row, inside its edge: the focus ring's stroke ([`FocusRing::style`],
    /// `line.focus`).
    pub focus: Stroke,
}

/// A select's sizes and colours, from [`Select::style`]; [`ComboBox`](super::ComboBox) draws
/// with the same.
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
/// // `blend` follows the open list. A host with an undo history hands `edit.event` to its
/// // `edit::GestureHost`, which makes the list's Begin, Updates and Commit one undo step.
/// if let Some(blend) = edit.committed() {
///     // Chosen.
/// }
/// # });
/// ```
///
/// A field-like box shows the chosen option's text and a chevron at its reading end; it opens a
/// list of the options below it (above it when there is more room there), on the popup surface,
/// at least as wide as the box. The chosen option has a check mark. It is a WAI-ARIA select-only
/// combo box: the keyboard focus stays on the box while the list is open, and AccessKit names the
/// active option as the box's active descendant ([`Listbox`]); the box reports a list box popup
/// and, while open, the list it controls.
///
/// - **The open list is one gesture**, as PhotoCraft's dropdowns: opening the list begins an
///   edit gesture, and every option the pointer or the keys make active is an `Update`, so the
///   value follows the list live and the chosen row (its check mark, AccessKit `selected`) is the
///   active one, but for a search's first match (Searchable below). Choosing commits. Closing without a choice cancels and puts back the value from
///   when the list opened: Esc ([`Escape`](CancelReason::Escape)), a click outside the list or a
///   press on the box ([`Dismissed`](CancelReason::Dismissed)), the focus leaving the box and the
///   list ([`FocusLost`](CancelReason::FocusLost)). A gesture that ends outside the list (an undo,
///   an external change) closes it. A host that applies only what is chosen acts on the commit
///   and leaves the updates to the box.
/// - **Pointer**: the list opens on the press, so one gesture can press the box, drag onto an
///   option and release to choose it. A plain click opens the list and leaves it open for a
///   second click; a release over the box keeps it open, a release elsewhere after a drag closes
///   it. A press on the box while the list is open closes it. The option under the pointer
///   becomes active, so Enter chooses the row the pointer is on, as in a macOS pop-up menu.
/// - **Keys on the closed box**: Down and Up step the value to the next and previous enabled
///   option without opening the list, one `Set` each, stopping at the ends (with the value none
///   of the options, Down goes to the first and Up to the last), as PhotoCraft's font picker
///   does. Alt+Down, Enter and Space open the list on the chosen option. Typing opens it on the
///   first option whose text starts with what was typed (no app convention; WAI-ARIA's).
/// - **Keys in the open list**: the arrows, Page Up and Page Down, Home, End and type-ahead move
///   the active option, skipping disabled ones ([`Listbox`]), and so the value; Enter, Space,
///   Alt+Up and Tab choose it (Tab then moves the focus on, from the search field too); Esc puts
///   the value back, closes the list and keeps the focus on the box. The active option shows the
///   focus ring inside its row while the keyboard moves it.
/// - **Wheel** ([`wheel_steps`](Select::wheel_steps), off by default): a wheel gesture that
///   starts over the closed box steps the value one option per notch (Shift too), stopping at
///   the ends, one `Set` a frame, and the panel under it doesn't scroll. PhotoCraft's Layers
///   blend mode does this. The box takes the wheel as a number field without the focus does: a
///   gesture that started elsewhere (a panel being scrolled) goes on scrolling over it, one that
///   leaves the box is the panel's from then on, and Alt+wheel, Ctrl/Cmd+wheel and a plain
///   sideways scroll stay the view's.
/// - **Searchable** ([`searchable`](Select::searchable)): a search field at the top of the list
///   takes the focus when it opens and filters the options as the user types; the arrows, Page
///   Up, Page Down, Enter and Esc work from it, Home and End stay with its text. The first match
///   becomes active for Enter without changing the value (PhotoCraft's font search applies only
///   the arrows' steps): the active option's ring marks the match, while the check mark and
///   AccessKit `selected` stay on the value's row. While an input method composes in it, the keys are the input method's
///   ([`Typing::composing`](crate::behavior::Typing::composing)).
///
/// The control writes what it reports into the value: updates, commits and sets, and on every
/// cancel but [`External`](CancelReason::External) the value from when the list opened.
pub struct Select<'a, T> {
    id_salt: IdSalt,
    label: String,
    value: &'a mut T,
    entries: Vec<Entry<T>>,
    rows: Option<Rows<'a, T>>,
    search: Option<(String, String)>,
    wheel_steps: bool,
    placeholder: String,
    size: ControlSize,
    policy: EditPolicy,
    enabled: bool,
}

/// `count` options of one height, each `value(index)` showing `text(index)`.
struct Rows<'a, T> {
    count: usize,
    value: Box<dyn Fn(usize) -> T + 'a>,
    text: Box<dyn Fn(usize) -> Cow<'a, str> + 'a>,
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
            wheel_steps: false,
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

    /// `count` options of one height in place of [`option`](Select::option)s: a long list
    /// (fonts) that lays out only the rows in view. Option `index` chooses `value(index)` and
    /// shows `text(index)`.
    ///
    /// The select calls them for what it shows and searches: while the list is closed, the
    /// chosen option's value and text, a few calls a frame. It looks through every value when the
    /// value or the count changes, and through every text once per search.
    pub fn rows<S: Into<Cow<'a, str>>>(mut self, count: usize, value: impl Fn(usize) -> T + 'a, text: impl Fn(usize) -> S + 'a) -> Self {
        self.rows = Some(Rows { count, value: Box::new(value), text: Box::new(move |index| text(index).into()) });
        self
    }

    /// A search field at the top of the list that filters the options; `label` is its accessible
    /// name and placeholder ("Search fonts"), `clear_label` names its clear button.
    pub fn searchable(mut self, label: impl Into<String>, clear_label: impl Into<String>) -> Self {
        self.search = Some((label.into(), clear_label.into()));
        self
    }

    /// Whether a wheel gesture that starts over the closed box steps the value, one option per
    /// notch (Wheel on [`Select`]); off by default, so the wheel scrolls the panel. For a select
    /// changed often in place (a layer's blend mode).
    pub fn wheel_steps(mut self, wheel_steps: bool) -> Self {
        self.wheel_steps = wheel_steps;
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

    /// How the open list's gesture reports ([`EditPolicy`]); the default reports every update at
    /// once.
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
        let Self { id_salt, label, value, entries, rows, search, wheel_steps, placeholder, size, policy, enabled } = self;
        let source = rows.map_or(Source::Entries(entries), Source::Rows);
        let tokens = Tokens::of(ui);
        let style = Select::style(&tokens, size);
        let id = Interactive::id_in(ui, id_salt);
        let enabled = enabled && ui.is_enabled();
        let options = Options::new(&source);
        let (mut state, fresh) = PopupState::load(ui, id);
        let mut listbox = Listbox::load(ui, id.with("list"));
        if !fresh {
            listbox.reset();
        }
        let chosen = state.chosen(&options, value);
        let filtered = if state.open { filter(&mut state.filter, &options, &state.query) } else { None };
        let shown = Shown { options: &options, filtered: filtered.as_deref() };
        // The options may have changed since the last frame.
        if listbox.active().is_some_and(|a| shown.position(a).is_none()) {
            listbox.set_active(None);
        }

        let box_focused = ui.memory(|m| m.has_focus(id));
        let search_focused = state.search.is_some_and(|s| ui.memory(|m| m.has_focus(s)));
        let active_before = listbox.active();
        let modifiers = ui.input(|i| i.modifiers);

        let mut action = None;
        // The search field's input method has its keys while it composes; the gesture, loaded
        // before the field runs, would read its Esc.
        let composing = state.search.is_some_and(|s| take_composing_keys(ui, s));
        if enabled && (box_focused || search_focused) && !composing {
            action = read_keys(ui, id, &mut listbox, &shown, &state, box_focused, chosen);
        }
        let owners: Vec<Id> = std::iter::once(id).chain(state.search).collect();
        let mut gesture = Gesture::load(ui, id, &*value, policy).enabled(enabled).cancel_on_focus_loss(true).focus_owners(&owners);
        if state.open && state.gesture.is_some() && gesture.id() != state.gesture {
            action = Some(Action::Ended);
        } else if !enabled && state.open {
            action = Some(Action::Close { reason: CancelReason::Disabled, focus: false });
        }

        let width = if ui.layout().horizontal_justify() { ui.available_width() } else { style.field.width };
        let (_, rect) = ui.allocate_space(vec2(width, style.field.height));
        let response = Interactive::with_id(id, Role::ComboBox, label.as_str())
            .expanded(state.open)
            .enabled(enabled)
            .state_layer(false)
            .focus_ring(false)
            .interact_at(ui, rect)
            .finish(ui);
        if wheel_steps && enabled && !state.open {
            let step = wheel_step(ui, id, &shown, chosen, response.rect);
            action = action.or(step);
        }
        // A press, or a click whose press and release came in one frame.
        let pressed = enabled && pressed_on(ui, &response);
        let reader_click = enabled && ui.input(|i| i.has_accesskit_action_request(id, accesskit::Action::Click));
        if action.is_none() && (pressed || reader_click) {
            action = Some(match state.open {
                true => Action::Close { reason: CancelReason::Dismissed, focus: pressed },
                false => Action::Open { active: chosen, keyboard: !pressed },
            });
            state.pressing = pressed && !state.open;
        }

        let mut pass = Pass { id, modifiers, searched: false, gesture: &mut gesture, listbox: &mut listbox, state: &mut state, options: &options };
        if let Some(action) = action {
            pass.apply(ui, action, box_focused);
        }

        let popup_id = Popup::default_response_id(&response);
        let mut drawn_active = None;
        if pass.state.open {
            let header = search.as_ref().map_or(0.0, |_| style.field.height + style.list.gap);
            let mut open = true;
            let owners: Vec<Id> = std::iter::once(id).chain(pass.state.search).collect();
            let popup = PopupList::new(ui, &response)
                .open(&mut open)
                .focus_owners(&owners)
                .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                .height(MAX_ROWS * style.list.height + header)
                .surface(style.popup);
            let list = List { style: &style, size, id, chosen, enabled, search: search.as_ref() };
            let picked = list.show(ui, popup, &mut pass, &mut drawn_active);
            let released = released(ui);
            if let Some(option) = picked {
                pass.apply(ui, Action::Choose { option, focus: true }, box_focused);
            } else if !open {
                let reason = match ui.input(|i| (i.key_pressed(Key::Escape), i.pointer.any_pressed() || i.pointer.any_click())) {
                    (true, _) => CancelReason::Escape,
                    (false, true) => CancelReason::Dismissed,
                    (false, false) => CancelReason::FocusLost,
                };
                pass.apply(ui, Action::Close { reason, focus: false }, box_focused);
            } else if pass.state.pressing && released && released_away(ui, popup_id, response.rect) {
                pass.apply(ui, Action::Close { reason: CancelReason::Dismissed, focus: false }, box_focused);
            }
            if pass.state.pressing && released {
                pass.state.pressing = false;
            }
            if pass.state.open {
                let owner = if box_focused { Some(id) } else { pass.state.search.filter(|_| search_focused) };
                keep_focus(ui, popup_id, owner);
            }
        }
        // The list's value follows the pointer and the keys; a search's first match waits for Enter.
        if !pass.searched
            && pass.gesture.is_active()
            && let Some(active) = pass.listbox.active().filter(|a| Some(*a) != active_before)
        {
            pass.gesture.update(options.value(active), modifiers);
        }

        let open = state.open;
        // Read by the next pass's focus scope, before this select runs: an open list keeps its Tab.
        popup_list::record_open(ui.ctx(), popup_id, open);
        let search_id = state.search.filter(|_| open);
        listbox.store(ui);
        // Built after the frame's action: a list opened in this frame keeps Esc for its box.
        let event = gesture.event_filter(key_filter(open)).finish();
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
                if let Some(start) = &state.start {
                    *value = start.clone();
                }
            }
            _ => {}
        }
        if matches!(event, Some(EditEvent::Set { .. })) {
            response.mark_changed();
        }
        if !open {
            state.start = None;
        }
        let shown_option = if event.is_some() { state.chosen(&options, value) } else { chosen };
        state.store(ui, id);

        let text = shown_option.map(|o| options.text(o));
        let focused = open || ui.memory(|m| m.has_focus(id));
        paint_box(
            ui,
            &style,
            &response,
            BoxContent { text: text.as_deref(), icon: shown_option.and_then(|o| options.icon(o)), placeholder: &placeholder, size, enabled, focused },
        );
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_value(text.as_deref().unwrap_or_default());
            node.set_expanded(open);
            node.set_has_popup(HasPopup::Listbox);
            match open {
                true => node.set_controls(vec![popup_id.accesskit_id()]),
                false => node.clear_controls(),
            }
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
    /// The select's sizes and colours at `size`.
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
                focus: FocusRing::style(tokens).stroke,
            },
            popup: Surface::style(tokens, SurfaceKind::Popup),
        }
    }
}

/// The keys the box or the search field keeps from egui while it has the focus: the arrows, and
/// Esc while the list is open.
fn key_filter(open: bool) -> EventFilter {
    EventFilter { horizontal_arrows: true, vertical_arrows: true, tab: false, escape: open }
}

/// What a frame does to the list.
#[derive(Clone, Copy, Debug)]
enum Action {
    /// Opens the list with `active` active; `keyboard` when the keyboard opened it.
    Open { active: Option<usize>, keyboard: bool },
    /// Closes the list without a choice; with `focus`, the box takes the focus.
    Close { reason: CancelReason, focus: bool },
    /// Chooses the option and closes the list; with `focus`, the box takes the focus back.
    Choose { option: usize, focus: bool },
    /// Sets the value to the option with the list closed: Up, Down or the wheel on the box.
    Step { option: usize },
    /// Closes the list whose gesture ended outside it: an external change, an undo
    /// ([`cancel_all`](edit::cancel_all)), the focus leaving.
    Ended,
}

/// The keys of a frame, while the box or the search field has the focus.
fn read_keys<T: Clone + PartialEq>(
    ui: &Ui,
    id: Id,
    listbox: &mut Listbox,
    shown: &Shown<'_, '_, '_, T>,
    state: &PopupState<T>,
    on_box: bool,
    chosen: Option<usize>,
) -> Option<Action> {
    let key = |modifiers, key| take_key(ui, modifiers, key);
    if state.open {
        let choose = |listbox: &Listbox, focus| listbox.active().filter(|&a| shown.enabled(a)).map(|option| Action::Choose { option, focus });
        if key(Modifiers::NONE, Key::Escape) {
            return Some(Action::Close { reason: CancelReason::Escape, focus: true });
        }
        let space = on_box && !listbox.typing(ui) && key(Modifiers::NONE, Key::Space);
        if key(Modifiers::ALT, Key::ArrowUp) || key(Modifiers::NONE, Key::Enter) || space {
            return choose(listbox, true);
        }
        if ui.input(|i| i.key_pressed(Key::Tab)) {
            // egui moves the focus on from the box when the box senses later in this pass.
            if !on_box {
                ui.memory_mut(|m| m.request_focus(id));
            }
            return choose(listbox, false);
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
    let opening = [(Modifiers::ALT, Key::ArrowDown), (Modifiers::NONE, Key::Enter), (Modifiers::NONE, Key::Space)];
    if opening.into_iter().any(|(modifiers, k)| key(modifiers, k)) {
        let first = Listbox::neighbour(shown, None, true);
        return Some(Action::Open { active: chosen.or(first), keyboard: true });
    }
    for (k, next) in [(Key::ArrowDown, true), (Key::ArrowUp, false)] {
        if key(Modifiers::NONE, k) {
            return Listbox::neighbour(shown, chosen, next).map(|option| Action::Step { option });
        }
    }
    listbox.set_active(chosen);
    listbox.type_ahead(ui, shown).then(|| Action::Open { active: listbox.active(), keyboard: true })
}

/// The step this frame's wheel over the closed box at `rect` makes from `chosen`: one option per
/// notch, Shift too, stopping at the ends, for a wheel gesture that starts over the box, by the
/// rules of a number field without the focus ([`field_wheel`]). The wheel it takes scrolls nothing
/// else.
fn wheel_step<T: Clone + PartialEq>(ui: &Ui, id: Id, shown: &Shown<'_, '_, '_, T>, chosen: Option<usize>, rect: Rect) -> Option<Action> {
    // Up is positive: the option before.
    let notches = field_wheel::notches(ui, id, false, rect, true, 1.0);
    let mut to = chosen;
    for _ in 0..(notches.abs() as usize).min(shown.len()) {
        match Listbox::neighbour(shown, to, notches < 0.0) {
            Some(next) => to = Some(next),
            None => break,
        }
    }
    to.filter(|&to| Some(to) != chosen).map(|option| Action::Step { option })
}

/// What the list keeps between frames.
#[derive(Clone)]
struct PopupState<T> {
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
    /// The gesture begun when the list opened, and the value then.
    gesture: Option<GestureId>,
    start: Option<T>,
    /// The option of the value the select last looked for.
    found: Option<Found<T>>,
    /// The options the last search left.
    filter: Option<Filter>,
}

#[derive(Clone)]
struct Found<T> {
    value: T,
    /// How many options there were.
    count: usize,
    option: Option<usize>,
}

#[derive(Clone)]
struct Filter {
    /// Trimmed and lowercase.
    query: String,
    count: usize,
    shown: Arc<[usize]>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> PopupState<T> {
    /// The state as the last pass left it, and whether that pass drew the select: a select
    /// missing from a pass has been removed, and starts closed ([`load_kept`]).
    fn load(ui: &Ui, id: Id) -> (Self, bool) {
        match load_kept(ui, Self::key(id)) {
            Some(state) => (state, true),
            None => (Self::new(), false),
        }
    }

    fn new() -> Self {
        Self {
            open: false,
            pressing: false,
            query: String::new(),
            focus_search: false,
            search: None,
            page: 0,
            gesture: None,
            start: None,
            found: None,
            filter: None,
        }
    }

    fn store(self, ui: &Ui, id: Id) {
        store_kept(ui, Self::key(id), self);
    }

    fn key(id: Id) -> Id {
        id.with("craft_ui::controls::select")
    }

    /// The option whose value is `value`. Of rows, the one found last time is checked with one
    /// look at its value; they are looked through again only when the value or their count
    /// changed, or that row's value did. Entries, built every frame, are looked through every
    /// frame.
    fn chosen(&mut self, options: &Options<'_, '_, T>, value: &T) -> Option<usize> {
        let count = options.len();
        if options.rows()
            && let Some(found) = &self.found
            && found.count == count
            && found.value == *value
            && found.option.is_none_or(|o| options.value(o) == *value)
        {
            return found.option;
        }
        let option = options.find(value);
        self.found = Some(Found { value: value.clone(), count, option });
        option
    }
}

/// The options whose text contains `query`, ignoring case; `None` (all of them) for an empty
/// query. Rows come from `cache` when it holds the same query and count; entries, built every
/// frame, are filtered every frame.
fn filter<T: Clone + PartialEq>(cache: &mut Option<Filter>, options: &Options<'_, '_, T>, query: &str) -> Option<Arc<[usize]>> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return None;
    }
    let count = options.len();
    if let Some(cached) = cache.as_ref().filter(|c| options.rows() && c.query == query && c.count == count) {
        return Some(cached.shown.clone());
    }
    let shown: Arc<[usize]> = (0..count).filter(|&o| options.text(o).to_lowercase().contains(&query)).collect();
    *cache = Some(Filter { query, count, shown: shown.clone() });
    Some(shown)
}

/// Everything a frame's actions change.
struct Pass<'f, 's, 'a, T> {
    id: Id,
    modifiers: Modifiers,
    /// A search made its first match active in this pass.
    searched: bool,
    gesture: &'f mut Gesture<T>,
    listbox: &'f mut Listbox,
    state: &'f mut PopupState<T>,
    options: &'f Options<'s, 'a, T>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> Pass<'_, '_, '_, T> {
    fn keys(&self) -> EventFilter {
        key_filter(self.state.open)
    }

    fn apply(&mut self, ui: &Ui, action: Action, box_focused: bool) {
        match action {
            Action::Open { active, keyboard } => {
                self.state.open = true;
                self.state.query.clear();
                self.state.focus_search = true;
                self.state.search = None;
                self.listbox.reveal(active, keyboard);
                self.gesture.begin();
                self.state.gesture = self.gesture.id();
                self.state.start = self.gesture.start().cloned();
                if !box_focused {
                    ui.memory_mut(|m| m.request_focus_with_filter(self.id, self.keys()));
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
                self.state.found = Some(Found { value: value.clone(), count: self.options.len(), option: Some(option) });
                if self.gesture.is_active() {
                    self.gesture.update(value, self.modifiers);
                    self.gesture.commit();
                } else {
                    self.gesture.set(value);
                }
                // A click on an option took the focus from the box earlier in this pass.
                self.close(ui, focus);
            }
            Action::Step { option } => {
                let value = self.options.value(option);
                self.state.found = Some(Found { value: value.clone(), count: self.options.len(), option: Some(option) });
                self.gesture.set(value);
            }
            Action::Ended => self.close(ui, false),
        }
    }

    fn close(&mut self, ui: &Ui, focus_box: bool) {
        self.state.open = false;
        self.state.pressing = false;
        self.state.query.clear();
        self.state.search = None;
        self.state.gesture = None;
        self.listbox.reset();
        if focus_box {
            ui.memory_mut(|m| m.request_focus_with_filter(self.id, self.keys()));
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

    /// Whether the options are [`rows`](Select::rows), whose values and texts come from the app's
    /// functions.
    fn rows(&self) -> bool {
        matches!(self.source, Source::Rows(_))
    }

    fn value(&self, option: usize) -> T {
        match self.source {
            Source::Entries(_) => self.choices[option].value.clone(),
            Source::Rows(rows) => (rows.value)(option),
        }
    }

    fn text(&self, option: usize) -> Cow<'s, str> {
        match self.source {
            Source::Entries(_) => {
                let choice: &'s Choice<T> = self.choices[option];
                Cow::Borrowed(choice.text.as_str())
            }
            Source::Rows(rows) => (rows.text)(option),
        }
    }

    fn icon(&self, option: usize) -> Option<Icon> {
        self.choices.get(option).and_then(|c| c.icon)
    }

    /// Whether `option` is one of the options and can be chosen.
    fn enabled(&self, option: usize) -> bool {
        match self.source {
            Source::Entries(_) => self.choices.get(option).is_some_and(|c| c.enabled),
            Source::Rows(rows) => option < rows.count,
        }
    }

    /// Whether any option has an icon, so every row keeps room for one.
    fn has_icons(&self) -> bool {
        self.choices.iter().any(|c| c.icon.is_some())
    }

    /// The option whose value is `value`, looking through them all.
    fn find(&self, value: &T) -> Option<usize> {
        match self.source {
            Source::Entries(_) => self.choices.iter().position(|c| c.value == *value),
            Source::Rows(rows) => (0..rows.count).find(|&i| (rows.value)(i) == *value),
        }
    }
}

/// The options a frame shows, for the [`Listbox`]: all of them, or the ones a search left.
struct Shown<'o, 's, 'a, T> {
    options: &'o Options<'s, 'a, T>,
    /// Ascending.
    filtered: Option<&'o [usize]>,
}

impl<T: Clone + PartialEq> ListOptions for Shown<'_, '_, '_, T> {
    fn len(&self) -> usize {
        self.filtered.map_or(self.options.len(), <[usize]>::len)
    }

    fn option_at(&self, at: usize) -> usize {
        self.filtered.map_or(at, |f| f[at])
    }

    fn position(&self, option: usize) -> Option<usize> {
        match self.filtered {
            None => (option < self.options.len()).then_some(option),
            Some(filtered) => filtered.binary_search(&option).ok(),
        }
    }

    fn enabled(&self, option: usize) -> bool {
        self.options.enabled(option)
    }

    fn text(&self, option: usize) -> Cow<'_, str> {
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
fn lines<'s, T: Clone + PartialEq>(source: &'s Source<'_, T>, shown: &Shown<'_, '_, '_, T>) -> Vec<Line<'s>> {
    let Source::Entries(entries) = source else { return (0..shown.len()).map(|at| Line::Option(shown.option_at(at))).collect() };
    let filtering = shown.filtered.is_some();
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
                if shown.position(option).is_some() {
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
    /// Shows the list in `popup` and returns the option chosen in it this frame; `drawn_active` is
    /// the active option when its row was drawn.
    fn show<T: Clone + PartialEq + Send + Sync + 'static>(
        &self,
        ui: &Ui,
        popup: PopupList<'_>,
        pass: &mut Pass<'_, '_, '_, T>,
        drawn_active: &mut Option<usize>,
    ) -> Option<usize> {
        let mut picked = None;
        let options = pass.options;
        let rows = OptionRows::new(ui, &self.style.list, self.size, self.id, options.has_icons(), self.enabled, pass.state.pressing);
        let keys = pass.keys();
        let mut query = pass.state.query.clone();
        let mut cache = pass.state.filter.take();
        // The field takes the focus once its id is known from the pass before, so the gesture
        // counts it among the select's focus owners from the start.
        let focus_search = pass.state.focus_search && pass.state.search.is_some();
        let mut search = pass.state.search;
        let mut focused_search = false;
        let mut page = pass.state.page;
        let mut searched = false;
        let listbox = &mut *pass.listbox;
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
            Filtered { shown: filter(&mut cache, options, &query), changed: query != before }
        };
        let mut option_row = |ui: &mut Ui, option: usize, listbox: &mut Listbox| {
            let text = options.text(option);
            let row =
                OptionRow { option, text: &text, icon: options.icon(option), chosen: self.chosen == Some(option), enabled: options.enabled(option) };
            let outcome = rows.show(ui, listbox, &row);
            if outcome.picked {
                picked = Some(option);
            }
            if outcome.active {
                *drawn_active = Some(option);
            }
            outcome.rect
        };
        let row_height = self.style.list.height;
        match options.source {
            Source::Entries(_) => {
                let _ = popup.show_ui_with_header(header, |ui, filtered| {
                    let shown = Shown { options, filtered: filtered.shown.as_deref() };
                    searched |= keep_active_shown(listbox, &shown, filtered.changed);
                    // The popup's first pass measures it unseen; scroll once it shows.
                    let reveal = if ui.is_sizing_pass() { None } else { listbox.take_reveal() };
                    page = rows_in_view(ui, row_height);
                    let mut add = |ui: &mut Ui, option: usize, listbox: &mut Listbox| {
                        let rect = option_row(ui, option, listbox);
                        if reveal == Some(option) {
                            ui.scroll_to_rect(rect, None);
                        }
                    };
                    for line in lines(options.source, &shown) {
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
                let total = |filtered: &Filtered| filtered.shown.as_ref().map_or(options.len(), |shown| shown.len());
                let _ = popup.show_rows_with_header(header, row_height, total, |ui, range, filtered| {
                    let shown = Shown { options, filtered: filtered.shown.as_deref() };
                    searched |= keep_active_shown(listbox, &shown, filtered.changed);
                    page = rows_in_view(ui, row_height);
                    let reveal = if ui.is_sizing_pass() { None } else { listbox.take_reveal() };
                    if let Some(at) = reveal.and_then(|option| shown.position(option)) {
                        // The row may be out of view and not laid out: scroll to where it would be.
                        let frame = ui.max_rect();
                        let y = frame.top() + (at as f32 - range.start as f32) * row_height;
                        ui.scroll_to_rect(Rect::from_min_size(pos2(frame.left(), y), vec2(frame.width(), row_height)), None);
                    }
                    for at in range.filter(|&at| at < shown.len()) {
                        option_row(ui, shown.option_at(at), listbox);
                    }
                });
            }
        }
        pass.state.query = query;
        pass.state.filter = cache;
        pass.state.search = search;
        pass.state.page = page;
        pass.searched = searched;
        if focused_search {
            pass.state.focus_search = false;
        } else if pass.state.focus_search && self.search.is_some() {
            ui.ctx().request_repaint();
        }
        picked
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
    /// Ascending; `None` while no search filters.
    shown: Option<Arc<[usize]>>,
    /// The search changed this frame.
    changed: bool,
}

/// After a search changed, or when the active option is no longer shown, the first enabled
/// option shown becomes active. Returns whether a search made it active: that match waits for
/// Enter, while an option made active because the last one went applies as any other.
fn keep_active_shown<T: Clone + PartialEq>(listbox: &mut Listbox, shown: &Shown<'_, '_, '_, T>, changed: bool) -> bool {
    let hidden = listbox.active().is_some_and(|a| shown.position(a).is_none());
    if changed || hidden {
        let keyboard = changed || listbox.keyboard();
        listbox.reveal(Listbox::neighbour(shown, None, true), keyboard);
    }
    changed
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
    paint_chevron(ui, f, rect, content.size, muted);
    let end = direction::end_x(ui, rect) - forward * (f.pad_x + f.icon / 2.0);
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
            id: "controls/select/wheel-steps",
            level: Level::Controls,
            component: "Select",
            name: "Wheel steps",
            args: || Args::new().with_choice("size", SIZE_NAMES, 1).with_bool("wheel steps", true),
            show: show_wheel_steps,
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

/// The blend-mode story's document: a blend mode and an undo history of whole snapshots.
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
struct WheelStepsState {
    doc: BlendDoc,
    host: GestureHost<BlendDoc>,
    /// Newest first.
    log: VecDeque<String>,
}

/// Events kept in the blend-mode story's log.
const LOG_LEN: usize = 12;

/// Blend modes under a [`GestureHost`]: a swatch shows the document's colour, which follows the
/// value while the list is open, Up and Down on the closed box and, with "wheel steps", the wheel
/// over it; the events newest first and the undo-step count.
fn show_wheel_steps(ui: &mut Ui, args: &mut Args) {
    let size = size_arg(args.choice("size"));
    let wheel_steps = args.bool("wheel steps");
    let state_id = ui.scope_id().with("select-wheel-steps");
    let mut state = ui.data(|d| d.get_temp::<WheelStepsState>(state_id)).unwrap_or_default();
    Stack::new(Gap::Stack).show(ui, |ui| {
        row(ui, "Blend mode", |ui| {
            let mut mode = state.doc.mode;
            let mut select = Select::new("select-blend", "Blend mode", &mut mode).size(size).wheel_steps(wheel_steps);
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
                .rows(FONT_STEMS.len() * FONT_STYLES.len(), |i| i, font)
                .searchable("Search fonts", "Clear")
                .show(ui);
        });
    });
}
