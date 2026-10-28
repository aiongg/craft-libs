//! [`Typing`]: one line of text typed into egui's `TextEdit` and reported through the edit
//! lifecycle ([`crate::edit`]).

use std::ops::Range;

use egui::accesskit::{Invalid, Role};
use egui::text::CCursorRange;
use egui::text_edit::TextEditState;
use egui::{
    Align2, Color32, Event, EventFilter, FontId, Frame, Id, ImeEvent, Key, KeyboardShortcut, Margin, Modifiers, Response, TextEdit, Ui, Vec2,
};

use crate::edit::{EditPolicy, EditResponse, Gesture};
use crate::ime::is_ime_key;
use crate::primitives::FocusRing;

/// How a field draws the `TextEdit` it runs: the box `TextEdit` covers, the room the field keeps
/// at each end of the text (icons, a clear button), the font and the colours of the text, the
/// placeholder, the selection and the caret. The field paints the box itself.
#[derive(Clone, Debug)]
pub struct TypingLook {
    /// The size of the box; the text sits inside it, inset by `margin`.
    pub size: Vec2,
    /// Between the box's sides and the text.
    pub margin: Margin,
    /// Where the text sits in the box: vertically centred, at the reading start.
    pub align: Align2,
    pub font: FontId,
    pub text: Color32,
    pub placeholder: Color32,
    /// Behind selected text.
    pub selection: Color32,
    pub selected_text: Color32,
    pub caret: Color32,
}

/// What Esc does while the field has focus.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Escape {
    /// Cancels the run under way when the policy is [cancellable](EditPolicy::cancellable), or
    /// else leaves the field.
    #[default]
    Cancel,
    /// Clears text there is (a change of the run, which commits the empty text) and keeps the
    /// focus; in an empty field, leaves it. A search field's Esc.
    Clear,
}

/// One line of text the host owns, edited with egui's `TextEdit` and reported as an edit
/// gesture: the behaviour of [`TextField`](crate::controls::TextField) and
/// [`SearchField`](crate::controls::SearchField), for any control that edits a line of text.
///
/// ```
/// use craft_ui::behavior::{Typing, TypingLook};
/// use egui::{Align2, Color32, FontId, Margin, vec2};
/// # egui::__run_test_ui(|ui| { let name = "Layer 1";
/// let look = TypingLook {
///     size: vec2(160.0, 24.0),
///     margin: Margin::symmetric(6, 0),
///     align: Align2::LEFT_CENTER,
///     font: FontId::proportional(12.0),
///     text: Color32::BLACK,
///     placeholder: Color32::GRAY,
///     selection: Color32::LIGHT_BLUE,
///     selected_text: Color32::BLACK,
///     caret: Color32::BLACK,
/// };
/// let typed = Typing::new(ui.make_persistent_id("name"), "Name", name).show(ui, &look);
/// // Paint the box behind `typed.response().rect`.
/// let edit = typed.end(ui);
/// # });
/// ```
///
/// - **Typing run**: focus alone begins nothing. The first change of the text (typing, paste,
///   cut, delete, an input method's commit, in-field undo) begins the gesture; every later change
///   is an update. Enter commits and keeps the focus ([`EditResponse::submitted`]); moving the
///   focus away commits; a field no longer drawn ends as a commit ([`Ending`](crate::edit::Ending)).
///   A screen reader's set-value request without the focus is one [`Set`](crate::edit::EditEvent::Set).
/// - **Esc**: as [`Escape`] says.
/// - **Input methods**: marked (pre-edit) text is never part of the value; the value is the
///   committed text. Enter, Esc, Backspace, Delete and Space are the input method's while it
///   composes, and the first of them after a composition ends in the same pass (the key that
///   confirmed or cancelled it), as [`crate::ime`] describes: they don't commit, cancel, clear or
///   leave. A composition open when the focus leaves keeps its marked text as typed, as
///   `TextEdit` does, so the commit carries it; one open when the field is no longer drawn is
///   dropped.
/// - **In-field undo** (Cmd/Ctrl+Z, redo) is `TextEdit`'s, a change of the run like any other.
///   When the text changes from outside (a cancel, the host's own change) the undo history
///   starts again, so in-field undo never brings back text from before.
/// - **Host changes**: a host value other than the last reported cancels the run
///   ([`CancelReason::External`](crate::edit::CancelReason::External)) and shows the host's
///   text; the caret stays where it was, or at the end of a shorter text.
/// - **Focus**: Tab (or a screen reader) selects the whole text; a click places the caret.
#[must_use = "show the field with `show`"]
pub struct Typing<'a> {
    id: Id,
    label: &'a str,
    value: &'a str,
    placeholder: &'a str,
    policy: EditPolicy,
    enabled: bool,
    password: bool,
    char_limit: usize,
    escape: Escape,
    role: Role,
    invalid: bool,
}

impl<'a> Typing<'a> {
    /// The text `value` in the `TextEdit` with id `id`, named `label` for screen readers.
    pub fn new(id: Id, label: &'a str, value: &'a str) -> Self {
        Self {
            id,
            label,
            value,
            placeholder: "",
            policy: EditPolicy::default(),
            enabled: true,
            password: false,
            char_limit: usize::MAX,
            escape: Escape::Cancel,
            role: Role::TextInput,
            invalid: false,
        }
    }

    /// The text shown while the field is empty, which screen readers read too.
    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Shows a bullet per character; AccessKit reports a `PasswordInput`.
    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    /// The most characters typing can make.
    pub fn char_limit(mut self, limit: usize) -> Self {
        self.char_limit = limit;
        self
    }

    pub fn escape(mut self, escape: Escape) -> Self {
        self.escape = escape;
        self
    }

    /// The AccessKit role: `TextInput` by default; `SearchInput` for a search field. A password
    /// field is a `PasswordInput` whatever this says.
    pub fn role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }

    /// Reports the value invalid to AccessKit.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Takes this pass's input, runs `TextEdit` in the field's box at the `Ui`'s cursor, and
    /// reports the field to AccessKit. Paint the box and anything over it, then [`Typed::end`].
    pub fn show(self, ui: &mut Ui, look: &TypingLook) -> Typed {
        let id = self.id;
        let enabled = self.enabled && ui.is_enabled();
        let seen = ui.data(|d| d.get_temp::<Seen>(seen_key(id)));
        let mut state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
        let preedit = state.preedit().map(|(range, text)| (range, text.to_owned()));
        take_ime_keys(ui, preedit.is_some());
        let focused = enabled && ui.memory(|m| m.has_focus(id));
        let submitted = focused && ui.input(|i| i.key_pressed(Key::Enter));
        let was_focused = seen.is_some_and(|s| s.focused);

        let mut gesture = Gesture::load(ui, id, &self.value.to_owned(), self.policy).pointer_selects(true).enabled(enabled);
        if self.escape == Escape::Clear && focused && !gesture.shown().is_empty() && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            clear(ui, &mut gesture);
        }

        let shown = gesture.shown().clone();
        // Text that isn't what `TextEdit` was left with changed outside it: its undo history and
        // any marked text belong to the text before.
        let outside = seen.is_none_or(|s| s.text != egui::util::hash(&shown));
        let mut buffer = shown.clone();
        if outside {
            state.clear_undoer();
        } else if let Some((range, text)) = preedit.as_ref().filter(|_| seen.is_some_and(|s| s.composing)) {
            insert_chars(&mut buffer, range.start, text);
        }
        state.store(ui.ctx(), id);

        let edit = TextEdit::singleline(&mut buffer)
            .id(id)
            .return_key(None::<KeyboardShortcut>)
            .frame(Frame::NONE.inner_margin(look.margin))
            .font(look.font.clone())
            .text_color(look.text)
            .desired_width(look.size.x)
            .min_size(look.size)
            .align(look.align)
            .hint_text(self.placeholder)
            .interactive(enabled)
            .password(self.password)
            .char_limit(self.char_limit);
        let filter = edit.get_event_filter();
        let saved = ui.style().clone();
        let visuals = ui.visuals_mut();
        visuals.selection.bg_fill = look.selection;
        visuals.selection.stroke.color = look.selected_text;
        visuals.text_cursor.stroke.color = look.caret;
        visuals.weak_text_color = Some(look.placeholder);
        let output = edit.show(ui);
        ui.set_style(saved);

        let marked = output.state.preedit().map(|(range, _)| range);
        let composing = marked.is_some();
        let text = remove_chars(&buffer, marked);
        let response = output.response.response;
        if text != shown {
            if response.has_focus() {
                gesture.begin();
                gesture.update(text.clone(), ui.input(|i| i.modifiers));
            } else {
                gesture.set(text.clone());
            }
        }
        self.report(ui, enabled);
        Typed { id, escape: self.escape, was_focused, gesture, response, text, buffer, composing, submitted, filter }
    }

    fn report(&self, ui: &Ui, enabled: bool) {
        ui.ctx().accesskit_node_builder(self.id, |node| {
            node.set_label(self.label);
            if !self.password {
                node.set_role(self.role);
            }
            if self.invalid {
                node.set_invalid(Invalid::True);
            }
            if !enabled {
                node.clear_read_only();
                node.set_disabled();
            }
        });
    }
}

/// A field whose `TextEdit` ran this pass, from [`Typing::show`]: paint it, then [`end`](Typed::end).
#[must_use = "end the field with `end` to get its edit"]
pub struct Typed {
    id: Id,
    escape: Escape,
    /// The field had the focus at the end of the last pass.
    was_focused: bool,
    gesture: Gesture<String>,
    response: Response,
    text: String,
    /// The text with the marked text in it.
    buffer: String,
    composing: bool,
    submitted: bool,
    filter: EventFilter,
}

impl Typed {
    /// `TextEdit`'s response; its rect is the field's box.
    pub fn response(&self) -> &Response {
        &self.response
    }

    /// The committed text shown, without marked text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Clears the text, as Esc does with [`Escape::Clear`], and gives the field the focus: a
    /// clear button's click. The field shows the empty text from the next pass.
    pub fn clear(&mut self, ui: &Ui) {
        clear(ui, &mut self.gesture);
        ui.memory_mut(|m| m.request_focus(self.id));
        ui.ctx().request_repaint();
    }

    /// Commits on Enter, keeps the focus rules and returns this pass's edit.
    pub fn end(mut self, ui: &Ui) -> EditResponse<String> {
        if self.submitted {
            self.gesture.commit();
        }
        let focused = ui.memory(|m| m.has_focus(self.id));
        if self.composing && !focused {
            // `TextEdit` keeps the marked text as typed once it has lost the focus.
            self.text = std::mem::take(&mut self.buffer);
            match self.gesture.is_active() {
                true => self.gesture.update(self.text.clone(), ui.input(|i| i.modifiers)),
                false => self.gesture.set(self.text.clone()),
            }
        }
        if focused && !self.was_focused && FocusRing::visible(ui.ctx()) {
            select_all(ui, self.id, &self.buffer);
        }
        // Esc stays with the field while it is the input method's or clears text.
        let composing = self.composing && focused;
        let escape = composing || (self.escape == Escape::Clear && !self.gesture.shown().is_empty());
        ui.data_mut(|d| d.insert_temp(seen_key(self.id), Seen { text: egui::util::hash(&self.text), focused, composing }));
        let event = self.gesture.event_filter(EventFilter { escape, ..self.filter }).finish();
        EditResponse::new(self.response, event).submitted(self.submitted)
    }
}

/// Selects the whole text from the next pass on: focus that arrived from the keyboard (Tab may
/// move it in the middle of a pass, after the field ran).
fn select_all(ui: &Ui, id: Id, text: &str) {
    let mut state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::select_all_str(text)));
    state.store(ui.ctx(), id);
    ui.ctx().request_repaint();
}

/// What a field kept from its last pass.
#[derive(Clone, Copy, Debug)]
struct Seen {
    /// Hash of the committed text `TextEdit` was left with.
    text: u64,
    focused: bool,
    /// A composition was open and the field kept the focus: the marked text goes back in.
    composing: bool,
}

fn seen_key(id: Id) -> Id {
    id.with("craft_ui::behavior::typing")
}

/// Clears the text as one change of the run, committed at once.
fn clear(ui: &Ui, gesture: &mut Gesture<String>) {
    let modifiers = ui.input(|i| i.modifiers);
    gesture.begin();
    gesture.update(String::new(), modifiers);
    gesture.commit();
}

/// Takes out of `ui`'s input the keys an input method uses (Enter, Escape, Backspace, Delete,
/// Space) while a composition is open, and the first of them after one ends in this pass: some
/// platforms forward the key that confirmed or cancelled it. `composing` is whether one was open
/// when the pass began.
fn take_ime_keys(ui: &Ui, mut composing: bool) {
    let mut ended = false;
    ui.input_mut(|i| {
        i.events.retain(|event| match event {
            Event::Ime(ImeEvent::Preedit { text, .. }) if !text.is_empty() => {
                (composing, ended) = (true, false);
                true
            }
            Event::Ime(ImeEvent::Preedit { .. } | ImeEvent::Commit(_)) => {
                (composing, ended) = (false, composing);
                true
            }
            Event::Key { key, pressed: true, .. } if is_ime_key(*key) && (composing || std::mem::take(&mut ended)) => false,
            Event::Key { pressed: true, .. } | Event::Text(_) => {
                ended = false;
                true
            }
            _ => true,
        });
    });
}

/// `text` with `insert` put in at char index `at`; at the end when `at` is past it.
fn insert_chars(text: &mut String, at: usize, insert: &str) {
    let byte = text.char_indices().nth(at).map_or(text.len(), |(i, _)| i);
    text.insert_str(byte, insert);
}

/// `text` without the chars in `range`.
fn remove_chars(text: &str, range: Option<Range<usize>>) -> String {
    match range {
        Some(range) => text.chars().enumerate().filter(|(i, _)| !range.contains(i)).map(|(_, c)| c).collect(),
        None => text.to_owned(),
    }
}
