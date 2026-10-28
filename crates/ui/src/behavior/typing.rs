//! [`Typing`]: one line of text typed into egui's `TextEdit` and reported through the edit
//! lifecycle ([`crate::edit`]).

use std::ops::Range;
use std::sync::Arc;

use egui::accesskit::{Invalid, Role};
use egui::text::{CCursorRange, LayoutJob};
use egui::text_edit::TextEditState;
use egui::{
    Align2, Color32, Event, EventFilter, FontId, Frame, Galley, Id, ImeEvent, InputState, Key, KeyboardShortcut, Margin, Modifiers, Rect, Response,
    TextBuffer, TextEdit, Ui, Vec2, ViewportId,
};

use crate::edit::{EditPolicy, EditResponse, Gesture, GestureId};
use crate::ime::is_ime_key;
use crate::primitives::FocusRing;
use crate::theme::TABULAR_FIGURES;

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

/// How a field writes its value as text and reads typed text back: [`PlainText`] for text,
/// whose every text is a value; a number field's codec, whose typed text is a draft until the
/// run commits.
pub trait Codec {
    type Value: Clone + PartialEq + Send + Sync + 'static;

    /// Whether every text is a value. Then the typing run reports each change at the policy's
    /// rate. Otherwise the typed text is a draft: the run reports no updates
    /// ([`Gesture::hold_updates`]), its value is the draft read back (or the start while the
    /// draft reads as no value), and its commit carries that value.
    const DRAFTS_ARE_VALUES: bool = false;

    /// The text the field shows for `value`.
    fn format(&self, value: &Self::Value) -> String;

    /// The value `text` stands for, typed in a run that began at `start`; or why it stands for
    /// none.
    fn parse(&self, text: &str, start: &Self::Value) -> Result<Self::Value, NotAValue>;
}

/// Why text reads as no value, from [`Codec::parse`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotAValue {
    /// In words for the user: the invalid field's tooltip.
    pub reason: String,
    /// The text may still become a value as the user types on (`-`, `1,`): a typing run doesn't
    /// mark it invalid. A commit rejects it all the same.
    pub partial: bool,
}

/// Text as it is: the codec of [`Typing::new`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlainText;

impl Codec for PlainText {
    type Value = String;

    const DRAFTS_ARE_VALUES: bool = true;

    fn format(&self, value: &String) -> String {
        value.clone()
    }

    fn parse(&self, text: &str, _start: &String) -> Result<String, NotAValue> {
        Ok(text.to_owned())
    }
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
///   is an update. Enter commits and keeps the focus ([`EditResponse::submitted`]): a plain press,
///   with no modifier but Shift and not a held key's repeat, so a dialog's OK fires once and
///   Cmd/Ctrl+Enter stays free (a multi-line field's commit). Moving the focus away commits; a
///   field no longer drawn ends as a commit with everything typed ([`Ending`](crate::edit::Ending)).
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
/// - **Keys of the embedding control**: [`event_filter`](Typing::event_filter) keeps more keys
///   with the field while it has the focus (a list's Up and Down, a popup's Esc).
/// - **Typed values** ([`Typing::with_codec`]): the field shows its value as the [`Codec`] writes
///   it. Where text is a draft rather than a value (a number), the run reports no updates and
///   commits the draft read back, so Enter, the focus leaving and a field no longer drawn all
///   commit the same value. A draft that can't become a value is marked invalid as it is typed
///   ([`Typed::rejected`]; text that still may, [`NotAValue::partial`], is not). A draft that
///   reads as no value commits nothing (the run ends unchanged) and stays in the field, rejected
///   with the codec's reason and reported invalid, until the user fixes it, Esc restores the
///   value, or the host's value changes; a field no longer drawn drops it. An embedding
///   control's own input (a drag, a step) goes through [`Typed::gesture`],
///   [`Typed::commit_with`] and [`Typed::commit`].
#[must_use = "show the field with `show`"]
pub struct Typing<'a, C: Codec = PlainText> {
    id: Id,
    label: &'a str,
    value: C::Value,
    codec: C,
    placeholder: &'a str,
    policy: EditPolicy,
    enabled: bool,
    password: bool,
    char_limit: usize,
    escape: Escape,
    role: Role,
    invalid: bool,
    keys: EventFilter,
    tabular: bool,
}

impl<'a> Typing<'a> {
    /// The text `value` in the `TextEdit` with id `id`, named `label` for screen readers.
    pub fn new(id: Id, label: &'a str, value: &str) -> Self {
        Self::with_codec(id, label, value.to_owned(), PlainText)
    }
}

impl<'a, C: Codec> Typing<'a, C> {
    /// The value `value`, written and read back by `codec`, in the `TextEdit` with id `id`, named
    /// `label` for screen readers.
    pub fn with_codec(id: Id, label: &'a str, value: C::Value, codec: C) -> Self {
        Self {
            id,
            label,
            value,
            codec,
            placeholder: "",
            policy: EditPolicy::default(),
            enabled: true,
            password: false,
            char_limit: usize::MAX,
            escape: Escape::Cancel,
            role: Role::TextInput,
            invalid: false,
            keys: EventFilter::default(),
            tabular: false,
        }
    }

    /// Draws digits with tabular (equal-width) figures, so a number keeps its width as its
    /// digits change.
    pub fn tabular(mut self, tabular: bool) -> Self {
        self.tabular = tabular;
        self
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

    /// The AccessKit role: `TextInput` by default; `SearchInput` for a search field,
    /// `SpinButton` for a number field. A password field is a `PasswordInput` whatever this says.
    pub fn role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }

    /// Reports the value invalid to AccessKit; rejected text is reported invalid too.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Keys the embedding control reads while the field has the focus, on top of `TextEdit`'s
    /// and the run's Esc: egui keeps the focus on them (Tab, the arrows, Esc) and leaves them in
    /// the input. A searchable combo box keeps Up, Down and Esc for its open list; a number field
    /// keeps Up and Down for stepping.
    pub fn event_filter(mut self, filter: EventFilter) -> Self {
        self.keys = filter;
        self
    }

    /// Takes this pass's input, runs `TextEdit` in the field's box at the `Ui`'s cursor, and
    /// reports the field to AccessKit. Paint the box and anything over it, then [`Typed::end`].
    pub fn show(self, ui: &mut Ui, look: &TypingLook) -> Typed<C> {
        let id = self.id;
        let enabled = self.enabled && ui.is_enabled();
        let seen = Seen::<C::Value>::load(ui, id);
        let mut state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
        // A composition goes on only in a field that kept it and the focus since the last pass.
        let kept = seen.as_ref().is_some_and(|s| s.composing) && ui.memory(|m| m.owns_ime_events(id));
        let preedit = state.preedit().filter(|_| kept).map(|(range, text)| (range, text.to_owned()));
        take_ime_keys(ui, preedit.is_some());
        let focused = enabled && ui.memory(|m| m.has_focus(id));
        let submitted = focused && ui.input(enter_pressed);
        let was_focused = seen.as_ref().is_some_and(|s| s.focused);

        let mut gesture = Gesture::load(ui, id, &self.value, self.policy).pointer_selects(true).enabled(enabled);
        let seen_text = seen.as_ref().map(|s| s.text);
        let (draft, rejected) = seen.map_or((None, None), |s| (s.draft, s.rejected));
        // A run that ended since the last pass ended by a cancel (a commit ends in its own pass):
        // its text goes, rejected or not.
        let run_cancelled = draft.as_ref().is_some_and(|d| gesture.id() != Some(d.run));
        let draft = draft.filter(|_| !run_cancelled);
        let mut rejected = rejected.filter(|r| !run_cancelled && enabled && r.host == self.value);
        if rejected.is_some() && draft.is_none() && focused && !gesture.is_active() && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            rejected = None;
        }
        if self.escape == Escape::Clear
            && focused
            && !self.codec.format(gesture.shown()).is_empty()
            && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            clear(ui, &self.codec, &mut gesture);
        }

        let shown = match (&draft, &rejected) {
            (Some(d), _) => d.text.clone(),
            (None, Some(r)) if !gesture.is_active() => r.text.clone(),
            _ => self.codec.format(gesture.shown()),
        };
        // Text that isn't what `TextEdit` was left with changed outside it: its undo history and
        // any marked text belong to the text before.
        let outside = seen_text.is_none_or(|text| text != egui::util::hash(&shown));
        let mut buffer = shown.clone();
        if outside {
            state.clear_undoer();
        } else if let Some((range, text)) = &preedit {
            insert_chars(&mut buffer, range.start, text);
        }
        state.store(ui.ctx(), id);

        let mut layouter = |ui: &Ui, text: &dyn TextBuffer, _wrap: f32| tabular_line(ui, look, self.password, text.as_str());
        let mut edit = TextEdit::singleline(&mut buffer)
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
        if self.tabular {
            edit = edit.layouter(&mut layouter);
        }
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
        // `galley_pos` is where the text starts; a right-aligned galley's rect starts left of 0.
        let text_rect = Rect::from_min_size(output.galley_pos, output.galley.size());
        let response = output.response.response;
        let mut typed = Typed {
            id,
            escape: self.escape,
            codec: self.codec,
            host: self.value,
            was_focused,
            gesture,
            response,
            text,
            buffer,
            text_rect,
            composing,
            submitted,
            filter: either(filter, self.keys),
            draft,
            rejected,
            replaced: false,
            commit: false,
        };
        if typed.text != shown {
            let text = typed.text.clone();
            typed.change(ui, text);
        }
        report(ui, id, self.label, self.role, self.password, self.invalid || typed.rejected().is_some(), enabled);
        typed
    }
}

fn report(ui: &Ui, id: Id, label: &str, role: Role, password: bool, invalid: bool, enabled: bool) {
    ui.ctx().accesskit_node_builder(id, |node| {
        node.set_label(label);
        if !password {
            node.set_role(role);
        }
        if invalid {
            node.set_invalid(Invalid::True);
        }
        if !enabled {
            node.clear_read_only();
            node.set_disabled();
        }
    });
}

/// The one line `TextEdit` lays out, as its own layouter does, with tabular figures.
fn tabular_line(ui: &Ui, look: &TypingLook, password: bool, text: &str) -> Arc<Galley> {
    let text = if password { "\u{2022}".repeat(text.chars().count()) } else { text.to_owned() };
    let mut job = LayoutJob::simple_singleline(text, look.font.clone(), look.text);
    job.halign = look.align.x();
    job.language = ui.language();
    job.keep_trailing_whitespace = true;
    let line_height = ui.fonts_mut(|f| f.row_height(&look.font)) + ui.spacing().extra_text_line_spacing;
    for section in &mut job.sections {
        section.format.line_height = Some(line_height);
        section.format.font_features.push(TABULAR_FIGURES, 1);
    }
    ui.fonts_mut(|f| f.layout_job(job))
}

/// A field whose `TextEdit` ran this pass, from [`Typing::show`]: paint it, then [`end`](Typed::end).
#[must_use = "end the field with `end` to get its edit"]
pub struct Typed<C: Codec = PlainText> {
    id: Id,
    escape: Escape,
    codec: C,
    /// The host's value this pass.
    host: C::Value,
    /// The field had the focus at the end of the last pass.
    was_focused: bool,
    gesture: Gesture<C::Value>,
    response: Response,
    text: String,
    /// The text with the marked text in it.
    buffer: String,
    /// Where the laid-out text is.
    text_rect: Rect,
    composing: bool,
    submitted: bool,
    /// `TextEdit`'s focus filter with the embedding control's keys.
    filter: EventFilter,
    /// The typing run's text, where text is a draft.
    draft: Option<Draft>,
    rejected: Option<Rejected<C::Value>>,
    /// The embedding control ended the run with a value of its own.
    replaced: bool,
    /// The embedding control ended the run as Enter does.
    commit: bool,
}

/// The text of a typing run whose text is a draft.
#[derive(Clone, Debug)]
struct Draft {
    run: GestureId,
    text: String,
    /// Why the text reads as no value.
    error: Option<NotAValue>,
}

/// Text whose run committed nothing because it read as no value.
#[derive(Clone, Debug)]
struct Rejected<T> {
    text: String,
    reason: String,
    /// The host's value when it was rejected; any other drops it.
    host: T,
}

impl<C: Codec> Typed<C> {
    /// `TextEdit`'s response; its rect is the field's box.
    pub fn response(&self) -> &Response {
        &self.response
    }

    /// The committed text shown, without marked text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Where the laid-out text is, in the field's box (an empty rect at the text's start in an
    /// empty field): for a unit drawn beside it.
    pub fn text_rect(&self) -> Rect {
        self.text_rect
    }

    /// The value the field stands for this pass: during a typing run its draft read back (the
    /// start while the draft reads as no value), else what the gesture shows.
    pub fn value(&self) -> &C::Value {
        self.gesture.shown()
    }

    /// Whether a typing run is under way.
    pub fn is_typing(&self) -> bool {
        match &self.draft {
            Some(d) => self.gesture.id() == Some(d.run),
            None => C::DRAFTS_ARE_VALUES && self.gesture.is_active(),
        }
    }

    /// Why the text shown reads as no value, while it is marked invalid: rejected text the field
    /// keeps, or a typing run's draft that can't become a value (one that still may, `-` or
    /// `1,`, isn't marked).
    pub fn rejected(&self) -> Option<&str> {
        match (&self.rejected, self.draft()) {
            (Some(r), _) => Some(r.reason.as_str()),
            (None, Some(d)) => d.error.as_ref().filter(|e| !e.partial).map(|e| e.reason.as_str()),
            (None, None) => None,
        }
    }

    /// Whether the text shown reads as a value: not while the field keeps rejected text or a
    /// typing run's draft reads as none (`-` included). A step from the text needs one.
    pub fn reads_as_value(&self) -> bool {
        match self.draft() {
            Some(d) => d.error.is_none(),
            None => self.rejected.is_none(),
        }
    }

    /// The draft of the typing run under way.
    fn draft(&self) -> Option<&Draft> {
        self.draft.as_ref().filter(|d| self.gesture.id() == Some(d.run))
    }

    /// The field's gesture, for the embedding control's own input: a drag begins, updates and
    /// commits it. One gesture runs at a time: commit a typing run before beginning another
    /// (the run's text is read then, as on focus loss).
    pub fn gesture(&mut self) -> &mut Gesture<C::Value> {
        &mut self.gesture
    }

    /// Ends the typing run under way as Enter does, without [`submitted`](EditResponse::submitted):
    /// its text read back commits, or, when it reads as no value, the run ends unchanged and the
    /// text stays, rejected. Nothing without a run.
    pub fn commit(&mut self) {
        self.commit = self.is_typing();
    }

    /// Ends the typing run under way with `value` in place of its text (a step from the typed
    /// value); without a run, a one-shot change.
    pub fn commit_with(&mut self, ui: &Ui, value: C::Value) {
        if self.is_typing() {
            self.gesture.update(value, ui.input(|i| i.modifiers));
            self.gesture.commit();
            self.replaced = true;
        } else {
            self.gesture.set(value);
        }
    }

    /// Clears the text, as Esc does with [`Escape::Clear`], and gives the field the focus: a
    /// clear button's click. The field shows the empty text from the next pass.
    pub fn clear(&mut self, ui: &Ui) {
        clear(ui, &self.codec, &mut self.gesture);
        ui.memory_mut(|m| m.request_focus(self.id));
        ui.ctx().request_repaint();
    }

    /// Commits on Enter, keeps the focus rules and returns this pass's edit.
    pub fn end(self, ui: &Ui) -> EditResponse<C::Value> {
        self.end_as(ui, |value| value)
    }

    /// [`end`](Typed::end) for a control that reports its value as another type: `report` maps
    /// the edit, as [`Gesture::finish_as`] does.
    pub fn end_as<U: Clone + Send + Sync + 'static>(mut self, ui: &Ui, report: impl Fn(C::Value) -> U) -> EditResponse<U> {
        if self.submitted || self.commit {
            self.gesture.commit();
        }
        let focused = ui.memory(|m| m.has_focus(self.id));
        if self.composing && !focused {
            // `TextEdit` keeps the marked text as typed once it has lost the focus.
            let text = std::mem::take(&mut self.buffer);
            self.text.clone_from(&text);
            self.change(ui, text);
        }
        if self.gesture.is_active() && !self.is_typing() {
            // Another gesture (a drag) shows the value as it goes.
            self.rejected = None;
        }
        // A run of drafts ends on Enter or when the focus leaves (the gesture commits it as it
        // finishes): text that reads as no value stays, rejected.
        let ending = self.submitted || self.commit || !focused || self.replaced;
        if let Some(draft) = self.draft.take_if(|_| ending)
            && !self.replaced
            && self.gesture.id() == Some(draft.run)
            && let Some(start) = self.gesture.start()
            && let Err(error) = self.codec.parse(&draft.text, start)
        {
            self.rejected = Some(Rejected { text: draft.text, reason: error.reason, host: self.host.clone() });
        }
        if focused && !self.was_focused && FocusRing::visible(ui.ctx()) {
            select_all(ui, self.id, &self.buffer);
        }
        // Esc stays with the field while it is the input method's, clears text or restores
        // rejected text.
        let composing = self.composing && focused;
        let escape = composing || (self.escape == Escape::Clear && !self.codec.format(self.gesture.shown()).is_empty()) || self.rejected.is_some();
        let seen = Seen::new(ui, egui::util::hash(&self.text), focused, composing, self.draft.take(), self.rejected.take());
        seen.store(ui, self.id);
        let event = self.gesture.event_filter(EventFilter { escape: escape || self.filter.escape, ..self.filter }).finish_as(report);
        EditResponse::new(self.response, event).submitted(self.submitted)
    }

    /// A change of the text to `text`: with the focus, a change of the typing run (which it
    /// begins); without it (a screen reader's set-value), a one-shot change.
    fn change(&mut self, ui: &Ui, text: String) {
        let modifiers = ui.input(|i| i.modifiers);
        if !self.response.has_focus() && !(self.composing && self.is_typing()) {
            if let Ok(value) = self.codec.parse(&text, self.gesture.shown()) {
                self.gesture.set(value);
            }
            return;
        }
        if !self.gesture.is_active() {
            self.gesture.begin();
            if !C::DRAFTS_ARE_VALUES {
                self.gesture.hold_updates();
            }
        }
        let Some(start) = self.gesture.start().cloned() else { return };
        // The run's draft carries any mark from here.
        self.rejected = None;
        let error = match self.codec.parse(&text, &start) {
            Ok(value) => {
                self.gesture.update(value, modifiers);
                None
            }
            Err(error) => {
                self.gesture.update(start, modifiers);
                Some(error)
            }
        };
        if !C::DRAFTS_ARE_VALUES
            && let Some(run) = self.gesture.id()
        {
            self.draft = Some(Draft { run, text, error });
        }
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
#[derive(Clone, Debug)]
struct Seen<T> {
    /// The viewport and its pass that stored it.
    viewport: ViewportId,
    pass: u64,
    /// Hash of the committed text `TextEdit` was left with.
    text: u64,
    focused: bool,
    /// A composition was open and the field kept the focus: the marked text goes back in.
    composing: bool,
    draft: Option<Draft>,
    rejected: Option<Rejected<T>>,
}

impl<T: Clone + Send + Sync + 'static> Seen<T> {
    fn new(ui: &Ui, text: u64, focused: bool, composing: bool, draft: Option<Draft>, rejected: Option<Rejected<T>>) -> Self {
        let (viewport, pass) = (ui.ctx().viewport_id(), ui.ctx().cumulative_pass_nr());
        Self { viewport, pass, text, focused, composing, draft, rejected }
    }

    /// What field `id` kept, when it was drawn in the last pass of this viewport (or earlier in
    /// this one, as [`Gesture::load`] counts); a field missing from a pass starts afresh.
    fn load(ui: &Ui, id: Id) -> Option<Self> {
        let (viewport, pass) = (ui.ctx().viewport_id(), ui.ctx().cumulative_pass_nr());
        ui.data(|d| d.get_temp::<Self>(Self::key(id))).filter(|s| s.viewport == viewport && s.pass + 1 >= pass)
    }

    fn store(self, ui: &Ui, id: Id) {
        ui.data_mut(|d| d.insert_temp(Self::key(id), self));
    }

    fn key(id: Id) -> Id {
        id.with("craft_ui::behavior::typing")
    }
}

/// A plain press of Enter: no modifier but Shift (as `consume_key` ignores it), not a held
/// key's repeat.
fn enter_pressed(input: &InputState) -> bool {
    input.events.iter().any(|event| {
        matches!(event, Event::Key { key: Key::Enter, pressed: true, repeat: false, modifiers, .. } if Modifiers { shift: false, ..*modifiers }.is_none())
    })
}

/// The keys of both filters.
fn either(a: EventFilter, b: EventFilter) -> EventFilter {
    EventFilter {
        tab: a.tab || b.tab,
        horizontal_arrows: a.horizontal_arrows || b.horizontal_arrows,
        vertical_arrows: a.vertical_arrows || b.vertical_arrows,
        escape: a.escape || b.escape,
    }
}

/// Clears the text as one change of the run, committed at once; nothing where empty text reads
/// as no value.
fn clear<C: Codec>(ui: &Ui, codec: &C, gesture: &mut Gesture<C::Value>) {
    let Ok(empty) = codec.parse("", gesture.shown()) else { return };
    gesture.begin();
    gesture.update(empty, ui.input(|i| i.modifiers));
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
