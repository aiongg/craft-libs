//! [`ComboBox`]: a line of text with a popup list of suggestions that the typed text filters.

use std::borrow::Cow;
use std::collections::VecDeque;

use craft_theme::{ControlSize, TypeRole};
use egui::accesskit::{HasPopup, Role};
use egui::text::{CCursor, CCursorRange};
use egui::text_edit::TextEditState;
use egui::{Align2, AsIdSalt, EventFilter, Id, IdSalt, Key, Modifiers, Popup, PopupCloseBehavior, Rect, Sense, Shape, Ui, vec2};

use super::option_list::{
    Hover, MAX_ROWS, OptionRow, OptionRows, keep_focus, load_kept, option_id, paint_chevron, pressed_on, released, released_away, rows_in_view,
    store_kept,
};
use super::text_field::{BoxState, Held, field_box};
use super::{SIZE_NAMES, Select, size_arg};
use crate::behavior::{ListOptions, Listbox, Typed, Typing, TypingLook, take_enter, take_key};
use crate::direction;
use crate::edit::{self, CancelReason, EditPolicy, EditResponse, GestureId};
use crate::popup_list::{self, PopupList};
use crate::primitives::{Content, Gap, Interactive, Row, Stack, Surface, SurfaceKind, Text};
use crate::story::{Args, Level, Story};
use crate::theme::{Tokens, TokensOf, margin_x};

/// A line of text with a list of suggestions:
///
/// ```
/// use craft_ui::controls::ComboBox;
/// use craft_ui::edit::EditEvent;
/// # egui::__run_test_ui(|ui| { let mut size = String::from("12 pt");
/// let edit = ComboBox::new("font-size", "Font size", &size).options(["8 pt", "10 pt", "12 pt", "18 pt", "24 pt"]).show(ui);
/// match edit.event {
///     Some(EditEvent::Update { value, .. } | EditEvent::Set { value, .. } | EditEvent::Commit { value, .. }) => size = value,
///     // Begin and Cancel go to the document's undo history (`edit::GestureHost`).
///     _ => {}
/// }
/// # });
/// ```
///
/// A text field ([`TextField`](super::TextField)'s box and [`Typing`]'s rules) with a chevron at
/// its reading end that opens a list of the options below it, on the popup surface, drawn as
/// [`Select`]'s list. The text is the value: any text can be typed, and an option is a quick way
/// to type its text. It is a WAI-ARIA editable combo box with list autocomplete and manual
/// selection: the keyboard focus stays in the field, and AccessKit names the active option as the
/// field's active descendant ([`Listbox`]); the field reports an editable combo box with a list
/// box popup and, while open, the list it controls.
///
/// - **Typing** is a typing run as in a text field (one gesture, committed on Enter or when the
///   focus leaves; Enter leaves the field unless [`keep_focus`](ComboBox::keep_focus), Esc
///   cancels the run, leaves the field and goes on to the host, [`EditResponse::escaped`]).
///   Each change opens the list on the options matching the typed text, ignoring case: those
///   starting with it first, then those containing it. With no match the list closes. Work in the text takes the active option back to it: a change, a move of
///   the caret or the selection (Left, Right, Home, End, a press in the text), an input method's
///   composition. Enter then commits the typed text.
/// - **Opened without typing** (the chevron, Alt+Down) while no typing run is under way, the list
///   shows every option; the one whose text is the field's has a check mark.
/// - **The keys in the open list apply live**, as PhotoCraft's font picker: Down, Up, Page Down
///   and Page Up move the active option and put its text in the field as a change of the typing
///   run (one that begins the run when none is under way), so the host shows it at once. Enter
///   (Shift+Enter too, as the field counts it) and Alt+Up choose the active option and close the
///   list, and the field keeps the focus (PhotoCraft's font picker); Enter with none commits the
///   run ([`EditResponse::submitted`]), closes the list and leaves the field, as a text field's
///   Enter. Alt+Up with none closes the list. Esc is the open list's: it brings back the text
///   from before the keys put an option's in (cancelling the run they began, or changing the
///   typed text's run back) and closes the list; with nothing put in it only closes the list and
///   the run goes on. The press stays with the list. The next Esc is the text field's: it
///   cancels the run, leaves the field and goes on to the host. Left, Right, Home and End stay
///   with the caret.
///   Tab commits what the field shows and moves on. While an input method composes, the keys are
///   its own ([`Typing::composing`]) and the list reads none.
/// - **Keys on the closed field**: Up and Down step the text to the option before or after the
///   field's (with the text none of the options, Down goes to the first and Up to the last),
///   stopping at the ends, without opening the list: one `Set`, or the commit of the run under
///   way. Alt+Down opens the list on the field's option.
/// - **Choosing** an option (a click, the release of a press dragged onto it, Enter on the active
///   option) replaces the text and commits the run in that frame, so typing and choosing are one
///   undo step; with no run under way the choice is one `Set`. The caret goes to the end of the
///   chosen text, as after each step of the keys.
/// - **Pointer**: a press on the chevron opens the list, so the same press can drag onto an
///   option and choose it on release; a click leaves the list open, and a press on the chevron
///   while it is open closes it. A click in the text places the caret and neither opens nor
///   closes the list. The pointer over an option highlights it and leaves the active option as it
///   was, so Enter never puts in the text of a row the pointer passed over; in a [`Select`],
///   whose options are its only values, the option under the pointer becomes active.
/// - The list closes when the run it was open for ends outside it (an undo, a change from the
///   host), when the focus leaves the field and the list, and when the combo box isn't drawn.
pub struct ComboBox<'a> {
    id_salt: IdSalt,
    label: String,
    value: &'a str,
    options: Vec<Cow<'a, str>>,
    placeholder: String,
    size: ControlSize,
    policy: EditPolicy,
    enabled: bool,
    keep_focus: bool,
}

impl<'a> ComboBox<'a> {
    /// A combo box named `label` (its accessible name) showing the text `value`; `id_salt` keeps
    /// its run and list state when the layout around it changes.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, value: &'a str) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            value,
            options: Vec::new(),
            placeholder: String::new(),
            size: ControlSize::Md,
            policy: EditPolicy::default(),
            enabled: true,
            keep_focus: false,
        }
    }

    /// An option whose text choosing it puts in the field.
    pub fn option(mut self, text: impl Into<Cow<'a, str>>) -> Self {
        self.options.push(text.into());
        self
    }

    /// Options, in order, after those already added.
    pub fn options<S: Into<Cow<'a, str>>>(mut self, texts: impl IntoIterator<Item = S>) -> Self {
        self.options.extend(texts.into_iter().map(Into::into));
        self
    }

    /// The text shown while the field is empty, which screen readers read too.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// How a typing run reports ([`EditPolicy`]); the default reports every change at once.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Whether the field keeps the focus after an Enter that commits the typed text; off by
    /// default. Choosing an option keeps it either way.
    pub fn keep_focus(mut self, keep: bool) -> Self {
        self.keep_focus = keep;
        self
    }

    /// Shows the field, and the list while it is open, and returns this frame's edit. The
    /// response covers the field.
    pub fn show(self, ui: &mut Ui) -> EditResponse<String> {
        let Self { id_salt, label, value, options, placeholder, size, policy, enabled, keep_focus: keep } = self;
        let tokens = Tokens::of(ui);
        let style = Select::style(&tokens, size);
        let f = &style.field;
        let id = Interactive::id_in(ui, id_salt);
        let enabled = enabled && ui.is_enabled();
        let (mut state, fresh) = ComboState::load(ui, id, value);
        let mut listbox = Listbox::load(ui, id.with("list"));
        if !fresh {
            listbox.reset();
        }
        let mut shown = Matches::new(&options, state.query.as_deref());
        // The options may have changed since the last frame.
        if listbox.active().is_some_and(|a| shown.position(a).is_none()) {
            listbox.set_active(None);
        }
        let focused = enabled && ui.memory(|m| m.has_focus(id));

        let (mut choice, keys) = state.keys(ui, id, enabled, &mut listbox, &options, &shown);

        // The chevron's end of the box, where a press opens the list rather than placing the caret.
        let end_room = f.pad_x + f.icon + f.gap;
        let width = if ui.layout().horizontal_justify() { ui.available_width() } else { f.width };
        let (left, right) = if direction::is_rtl(ui) { (end_room, f.pad_x) } else { (f.pad_x, end_room) };
        let look = TypingLook {
            size: vec2(width, f.height),
            margin: margin_x(left, right),
            align: direction::align2(ui, Align2::LEFT_CENTER),
            font: tokens.font(f.role),
            text: if enabled { f.text } else { f.text_disabled },
            placeholder: if enabled { f.placeholder } else { f.placeholder_disabled },
            selection: f.selection,
            selected_text: f.selected_text,
            caret: f.caret,
        };
        // The box goes under the text, which `TextEdit` paints as it runs.
        let under = ui.painter().add(Shape::Noop);
        let mut typed = Typing::new(id, &label, value)
            .placeholder(&placeholder)
            .policy(policy)
            .enabled(enabled)
            .keep_focus(keep)
            .role(Role::EditableComboBox)
            .event_filter(key_filter(state.open))
            .show(ui, &look);
        let rect = typed.response().rect;
        if typed.caret_moved() {
            // Work in the text takes the active option back to it: Enter commits what was typed.
            listbox.set_active(None);
        }
        // The text the list puts in the field in this pass, which the field shows from the next.
        let listed = match keys {
            Some(KeyAction::Moved) => listbox.active().map(|option| state.apply(ui, &mut typed, options[option].clone().into_owned())),
            Some(KeyAction::Escape) => state.escape(ui, &mut typed, &mut listbox),
            _ => None,
        };

        let run = if typed.is_typing() { typed.gesture().id() } else { None };
        if state.open && state.run.is_some() && run != state.run {
            state.close(&mut listbox);
        }
        if run.is_some() && typed.text() != state.text {
            // The user's own text: Esc keeps it.
            state.before = None;
            state.query = Some(typed.text().to_owned());
            shown = Matches::new(&options, state.query.as_deref());
            // Typing takes the active option back to the text.
            listbox.set_active(None);
            match (shown.is_empty(), state.open) {
                (true, true) => state.close(&mut listbox),
                (false, false) => state.open(ui, &mut listbox, None, false),
                _ => {}
            }
        } else if run.is_none() && state.query.take().is_some() {
            shown = Matches::new(&options, None);
        }

        let forward = direction::forward(ui);
        let end = direction::end_x(ui, rect);
        let (a, b) = (end, end - forward * end_room);
        let hit = Rect::from_x_y_ranges(a.min(b)..=a.max(b), rect.y_range());
        // Not focusable: the focus stays in the field.
        let chevron = ui.interact(hit, id.with("chevron"), Sense::CLICK);
        if enabled && pressed_on(ui, &chevron) {
            if state.open {
                state.close(&mut listbox);
            } else if !shown.is_empty() {
                let active = chosen(&options, typed.text()).filter(|&c| shown.position(c).is_some());
                state.open(ui, &mut listbox, active, false);
                state.pressing = true;
            }
        }
        if enabled && (chevron.is_pointer_button_down_on() || chevron.clicked()) {
            // egui gives the focus up on a press or a click outside the text.
            ui.memory_mut(|m| m.request_focus(id));
        }

        let popup_id = Popup::default_response_id(typed.response());
        let mut drawn_active = None;
        if state.open {
            let mut open = true;
            let mut picked = None;
            let popup = PopupList::new(ui, typed.response())
                .open(&mut open)
                .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                .height(MAX_ROWS * style.list.height)
                .surface(style.popup);
            let checked = chosen(&options, typed.text());
            let rows = OptionRows::new(ui, &style.list, size, id, false, enabled, state.pressing).hover(Hover::Highlights);
            let mut page = state.page;
            let row_height = style.list.height;
            let _ = popup.show_ui(|ui| {
                // Rows touch: the row height alone spaces them.
                ui.spacing_mut().item_spacing.y = 0.0;
                page = rows_in_view(ui, row_height);
                // The popup's first pass measures it unseen; scroll once it shows.
                let reveal = if ui.is_sizing_pass() { None } else { listbox.take_reveal() };
                for at in 0..shown.len() {
                    let option = shown.option_at(at);
                    let row = OptionRow { option, text: &options[option], icon: None, chosen: checked == Some(option), enabled: true };
                    let outcome = rows.show(ui, &mut listbox, &row);
                    if outcome.picked {
                        picked = Some(option);
                    }
                    if outcome.active {
                        drawn_active = Some(option);
                    }
                    if reveal == Some(option) {
                        ui.scroll_to_rect(outcome.rect, None);
                    }
                }
            });
            state.page = page;
            let released = released(ui);
            if picked.is_some() {
                choice = picked;
            } else if !open || (state.pressing && released && released_away(ui, popup_id, rect)) {
                state.close(&mut listbox);
            }
            if released {
                state.pressing = false;
            }
            if state.open {
                keep_focus(ui, popup_id, focused.then_some(id));
            }
        }

        let mut caret = listed.as_ref().map(|text| text.chars().count());
        if let Some(option) = choice {
            let text = options[option].clone().into_owned();
            typed.commit_with(ui, text.clone());
            state.close(&mut listbox);
            // A click on an option took the focus from the field.
            ui.memory_mut(|m| m.request_focus(id));
            caret = Some(text.chars().count());
        } else if typed.submitted() {
            state.close(&mut listbox);
        }
        let open = state.open;
        // Read by the next pass's focus scope, before this field runs: an open list keeps its Tab.
        popup_list::record_open(ui.ctx(), popup_id, open);
        state.run = run.filter(|_| open);
        typed.set_event_filter(key_filter(open));

        let box_state = BoxState {
            enabled,
            focused: open || typed.response().has_focus(),
            hovered: typed.response().contains_pointer() || chevron.contains_pointer(),
            invalid: false,
        };
        ui.painter().set(under, Shape::Vec(field_box(ui, f, rect, box_state)));
        paint_chevron(ui, f, rect, size, if enabled { f.placeholder } else { f.placeholder_disabled });
        state.text = listed.unwrap_or_else(|| typed.text().to_owned());
        let edit = typed.end(ui);
        if let Some(chars) = caret {
            // The field shows the chosen text from the next frame; the caret waits at its end.
            let mut text_state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
            text_state.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(chars))));
            text_state.store(ui.ctx(), id);
        }
        listbox.store(ui);
        state.store(ui, id);
        ui.ctx().accesskit_node_builder(id, |node| {
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
        edit
    }
}

/// The option whose text is `text`.
fn chosen(options: &[Cow<'_, str>], text: &str) -> Option<usize> {
    options.iter().position(|o| o == text)
}

/// The keys the field keeps from egui while it has the focus, on top of `TextEdit`'s: the arrows
/// (Up and Down step the value or the list), and Esc while the list is open.
fn key_filter(open: bool) -> EventFilter {
    EventFilter { horizontal_arrows: true, vertical_arrows: true, tab: false, escape: open }
}

/// What the keys of a frame do to the list.
#[derive(Clone, Copy, Debug)]
enum KeyAction {
    Open {
        active: Option<usize>,
    },
    Close,
    Choose(usize),
    /// The active option moved: its text goes in the field.
    Moved,
    /// Esc in the open list: the text from before the list put any in comes back.
    Escape,
    /// Up or Down on the closed field: the option's text goes in the field.
    Step(usize),
}

/// The keys of a frame while the field has the focus, read before the field takes the rest.
fn read_keys(
    ui: &Ui,
    listbox: &mut Listbox,
    options: &[Cow<'_, str>],
    shown: &Matches<'_, '_>,
    state: &ComboState,
    chosen: Option<usize>,
) -> Option<KeyAction> {
    let key = |modifiers, key| take_key(ui, modifiers, key);
    if state.open {
        if key(Modifiers::NONE, Key::Escape) {
            return Some(KeyAction::Escape);
        }
        if let Some(option) = listbox.active()
            && (key(Modifiers::ALT, Key::ArrowUp) || take_enter(ui))
        {
            return Some(KeyAction::Choose(option));
        }
        if key(Modifiers::ALT, Key::ArrowUp) {
            return Some(KeyAction::Close);
        }
        return listbox.keys(ui, shown, state.page, false).then_some(KeyAction::Moved);
    }
    if key(Modifiers::ALT, Key::ArrowDown) {
        let active = chosen.filter(|&c| shown.position(c).is_some());
        return (!shown.is_empty()).then_some(KeyAction::Open { active });
    }
    let all = Matches::new(options, None);
    for (k, next) in [(Key::ArrowDown, true), (Key::ArrowUp, false)] {
        if key(Modifiers::NONE, k) {
            return Listbox::neighbour(&all, chosen, next).map(KeyAction::Step);
        }
    }
    None
}

/// What the list keeps between frames.
#[derive(Clone, Default)]
struct ComboState {
    open: bool,
    /// The press that opened the list is still down.
    pressing: bool,
    /// The text of the typing run under way, which filters the list; `None` shows every option.
    query: Option<String>,
    /// The field's text at the end of the last frame.
    text: String,
    /// The typing run the open list goes with.
    run: Option<GestureId>,
    /// Rows the list shows at once.
    page: usize,
    /// The text from before the open list put an option's in, which Esc brings back.
    before: Option<Before>,
}

/// The field's text before the open list put an option's in.
#[derive(Clone)]
struct Before {
    text: String,
    /// The list's first option began the typing run, so Esc cancels it.
    began_run: bool,
}

impl ComboState {
    /// The state as the last pass left it, and whether that pass drew the combo box: one missing
    /// from a pass has been removed, and starts closed ([`load_kept`]).
    fn load(ui: &Ui, id: Id, value: &str) -> (Self, bool) {
        match load_kept(ui, Self::key(id)) {
            Some(state) => (state, true),
            None => (Self { text: value.to_owned(), ..Self::default() }, false),
        }
    }

    fn store(self, ui: &Ui, id: Id) {
        store_kept(ui, Self::key(id), self);
    }

    fn key(id: Id) -> Id {
        id.with("craft_ui::controls::combo_box")
    }

    /// Opens the list with `active` active; `keyboard` when the keyboard opened it.
    fn open(&mut self, ui: &Ui, listbox: &mut Listbox, active: Option<usize>, keyboard: bool) {
        self.open = true;
        listbox.reveal(active, keyboard);
        ui.ctx().request_repaint();
    }

    fn close(&mut self, listbox: &mut Listbox) {
        self.open = false;
        self.pressing = false;
        self.run = None;
        self.before = None;
        listbox.reset();
    }

    /// Reads the keys of a frame before the field runs and opens, closes or chooses as they
    /// say; returns the option to choose and what is left for after the field ran.
    fn keys(
        &mut self,
        ui: &Ui,
        id: Id,
        enabled: bool,
        listbox: &mut Listbox,
        options: &[Cow<'_, str>],
        shown: &Matches<'_, '_>,
    ) -> (Option<usize>, Option<KeyAction>) {
        let focused = enabled && ui.memory(|m| m.has_focus(id));
        if !enabled && self.open {
            self.close(listbox);
        } else if focused && Typing::composing(ui, id) {
            // The input method's keys are its own; its composition takes the active option back
            // to the text, as typing does.
            listbox.set_active(None);
        } else if focused {
            match read_keys(ui, listbox, options, shown, self, chosen(options, &self.text)) {
                Some(KeyAction::Open { active }) => self.open(ui, listbox, active, true),
                Some(KeyAction::Close) => self.close(listbox),
                Some(KeyAction::Choose(option) | KeyAction::Step(option)) => return (Some(option), None),
                other => return (None, other),
            }
        }
        (None, None)
    }

    /// Puts `text`, the active option's, in the field as a change of the typing run, which it
    /// begins when none is under way; returns it.
    fn apply(&mut self, ui: &Ui, typed: &mut Typed, text: String) -> String {
        let began_run = !typed.is_typing();
        self.before.get_or_insert_with(|| Before { text: typed.text().to_owned(), began_run });
        if began_run {
            typed.gesture().begin();
        }
        typed.gesture().update(text.clone(), ui.input(|i| i.modifiers));
        text
    }

    /// Esc in the open list: brings back the text from before the list put an option's in
    /// (cancels the run the list began, or else changes the run back) and closes the list.
    /// Returns that text, if the list put any in.
    fn escape(&mut self, ui: &Ui, typed: &mut Typed, listbox: &mut Listbox) -> Option<String> {
        let before = self.before.take();
        self.close(listbox);
        let before = before?;
        match before.began_run {
            true => typed.gesture().cancel(CancelReason::Escape),
            false => typed.gesture().update(before.text.clone(), ui.input(|i| i.modifiers)),
        }
        Some(before.text)
    }
}

/// The options a frame shows, for the [`Listbox`]: all of them, or those matching the typed text.
struct Matches<'o, 'a> {
    options: &'o [Cow<'a, str>],
    /// The options starting with the text, then those containing it; `None` for all of them.
    shown: Option<Vec<usize>>,
}

impl<'o, 'a> Matches<'o, 'a> {
    /// The options matching `query`, ignoring case and the spaces around it; all of them for
    /// `None` or an empty query.
    fn new(options: &'o [Cow<'a, str>], query: Option<&str>) -> Self {
        let query = query.map(|q| q.trim().to_lowercase()).filter(|q| !q.is_empty());
        let shown = query.map(|query| {
            let (mut starting, mut containing) = (Vec::new(), Vec::new());
            for (index, option) in options.iter().enumerate() {
                let option = option.to_lowercase();
                if option.starts_with(&query) {
                    starting.push(index);
                } else if option.contains(&query) {
                    containing.push(index);
                }
            }
            starting.extend(containing);
            starting
        });
        Self { options, shown }
    }
}

impl ListOptions for Matches<'_, '_> {
    fn len(&self) -> usize {
        self.shown.as_ref().map_or(self.options.len(), Vec::len)
    }

    fn option_at(&self, at: usize) -> usize {
        self.shown.as_ref().map_or(at, |shown| shown[at])
    }

    fn position(&self, option: usize) -> Option<usize> {
        match &self.shown {
            None => (option < self.options.len()).then_some(option),
            Some(shown) => shown.iter().position(|&o| o == option),
        }
    }

    fn enabled(&self, option: usize) -> bool {
        option < self.options.len()
    }

    fn text(&self, option: usize) -> Cow<'_, str> {
        Cow::Borrowed(&self.options[option])
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Combo box"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "controls/combo-box/states",
            level: Level::Controls,
            component: "Combo box",
            name: "States",
            args: || Args::new().with_choice("size", SIZE_NAMES, 1),
            show: show_states,
        },
        Story {
            id: "controls/combo-box/font-size",
            level: Level::Controls,
            component: "Combo box",
            name: "Font size",
            args: || Args::new().with_choice("size", SIZE_NAMES, 1),
            show: show_font_size,
        },
    ]
}

const FONT_SIZES: [&str; 16] =
    ["8 pt", "9 pt", "10 pt", "11 pt", "12 pt", "14 pt", "16 pt", "18 pt", "20 pt", "24 pt", "28 pt", "32 pt", "36 pt", "48 pt", "60 pt", "72 pt"];

const FONTS: [&str; 8] = ["Baskerville", "Bodoni", "Caslon", "Futura", "Garamond", "Gill Sans", "Helvetica", "Optima"];

/// `label` at the reading start of a row, then the control.
fn row(ui: &mut Ui, label: &str, control: impl FnOnce(&mut Ui)) {
    Row::new(Gap::Inline).show(ui, |ui| {
        ui.add(Text::new(label).content(Content::Muted));
        control(ui);
    });
}

/// Empty with a placeholder, an option's text, free text, and disabled.
fn show_states(ui: &mut Ui, args: &mut Args) {
    let size = size_arg(args.choice("size"));
    let initial = [Held::new(""), Held::new("12 pt"), Held::new("13.5 pt"), Held::new("Garamond")];
    super::slider::with_values(ui, "combo-box-states", initial, |ui, values| {
        Stack::new(Gap::Stack).show(ui, |ui| {
            let [empty, option, free, font] = values;
            row(ui, "Empty", |ui| {
                empty.apply(ComboBox::new("combo-empty", "Font size", &empty.text).options(FONT_SIZES).placeholder("Size").size(size).show(ui))
            });
            row(ui, "Option", |ui| option.apply(ComboBox::new("combo-option", "Font size", &option.text).options(FONT_SIZES).size(size).show(ui)));
            row(ui, "Free text", |ui| free.apply(ComboBox::new("combo-free", "Font size", &free.text).options(FONT_SIZES).size(size).show(ui)));
            row(ui, "Font", |ui| font.apply(ComboBox::new("combo-font", "Font", &font.text).options(FONTS).size(size).show(ui)));
            row(ui, "Disabled", |ui| {
                let _ = ComboBox::new("combo-disabled", "Locked size", "12 pt").options(FONT_SIZES).size(size).enabled(false).show(ui);
            });
        });
    });
}

/// The font-size story's state: the size and the events newest first.
#[derive(Clone, Debug)]
struct FontSize {
    held: Held,
    log: VecDeque<String>,
}

/// Events kept in the font-size story's log.
const LOG_LEN: usize = 12;

/// A font size typed or picked: a sample drawn at the size the text reads as, and the events
/// newest first.
fn show_font_size(ui: &mut Ui, args: &mut Args) {
    let size = size_arg(args.choice("size"));
    let initial = [FontSize { held: Held::new("12 pt"), log: VecDeque::new() }];
    super::slider::with_values(ui, "combo-box-font-size", initial, |ui, values| {
        let state = &mut values[0];
        Stack::new(Gap::Stack).show(ui, |ui| {
            row(ui, "Size", |ui| {
                let edit =
                    ComboBox::new("combo-font-size", "Font size", &state.held.text).options(FONT_SIZES).placeholder("Size").size(size).show(ui);
                if let Some(event) = &edit.event {
                    state.log.push_front(edit::describe(event));
                    state.log.truncate(LOG_LEN);
                }
                state.held.apply(edit);
            });
            let points = state.held.text.trim_end_matches("pt").trim().parse::<f32>().ok().filter(|p| (1.0..=200.0).contains(p));
            match points {
                Some(points) => {
                    ui.label(egui::RichText::new("The quick brown fox").size(points));
                }
                None => {
                    ui.add(Text::new("Type a size in points").content(Content::Faint));
                }
            }
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
    });
}

const LOG_WIDTH: f32 = 360.0;
