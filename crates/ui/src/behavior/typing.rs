//! [`Typing`]: one line of text typed into egui's `TextEdit` and reported through the edit
//! lifecycle ([`crate::edit`]).

use std::ops::Range;
use std::sync::Arc;

use egui::accesskit::{Invalid, Role};
use egui::text::{CCursorRange, LayoutJob};
use egui::text_edit::TextEditState;
use egui::{
    Align2, Color32, Event, EventFilter, FontId, Frame, Galley, Id, ImeEvent, InputState, Key, KeyboardShortcut, Margin, Modifiers, PointerButton,
    Rect, Response, SurrenderFocusOn, TextBuffer, TextEdit, Ui, Vec2, ViewportId,
};

use super::edit_menu;
use crate::edit::{EditPolicy, EditResponse, Gesture, GestureId, escape_pressed};
use crate::ime::is_ime_key;
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
    /// rate. Otherwise the typed text is a draft: its value is the draft read back (or the start
    /// while the draft reads as no value), the commit carries that value, and the run reports
    /// only the drafts that [apply as typed](Codec::applies_as_typed) as updates, holding the
    /// rest ([`Gesture::hold_updates`]).
    const DRAFTS_ARE_VALUES: bool = false;

    /// The text the field shows for `value`.
    fn format(&self, value: &Self::Value) -> String;

    /// The value `text` stands for, typed in a run that began at `start`; or why it stands for
    /// none.
    fn parse(&self, text: &str, start: &Self::Value) -> Result<Self::Value, NotAValue>;

    /// Whether the value of the draft `text`, which [`parse`](Codec::parse) reads as a value,
    /// applies as it is typed (an update of the run) rather than when the run commits: a plain
    /// number does, arithmetic (`5/2`) waits for the commit, so the host never applies `5` on the
    /// way to `5/2*2`. None does by default. Not asked where drafts are values.
    fn applies_as_typed(&self, _text: &str) -> bool {
        false
    }
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
    /// Cancels the run under way when the policy is [cancellable](EditPolicy::cancellable) (else
    /// the focus leaving commits it), drops rejected text, leaves the field and passes the press
    /// on: it stays in egui's input for the controls drawn after the field, and the field reports
    /// it ([`EditResponse::escaped`]), so a dialog or a tool acts on the same press. PhotoCraft's
    /// fields, and egui's, leave on Esc.
    #[default]
    PassThrough,
    /// Clears text there is (a change of the run, which commits the empty text), keeping the
    /// focus and the press; in an empty field, as [`PassThrough`](Escape::PassThrough). A search
    /// field's Esc: the first press clears, the next leaves.
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
///   is an update. Enter commits and leaves the field ([`EditResponse::submitted`]), as in
///   PhotoCraft; [`keep_focus`](Typing::keep_focus) keeps the focus. Enter is a plain press,
///   with no modifier but Shift and not a held key's repeat, so a dialog's OK fires once and
///   Cmd/Ctrl+Enter stays free (a multi-line field's commit). Moving the focus away commits; a
///   field no longer drawn ends as a commit with everything typed ([`Ending`](crate::edit::Ending)).
///   A screen reader's set-value request without the focus is one [`Set`](crate::edit::EditEvent::Set).
/// - **Esc**: as [`Escape`] says; by default it cancels the run, leaves the field and goes on to
///   the host ([`EditResponse::escaped`]).
/// - **Input methods**: marked (pre-edit) text is never part of the value; the value is the
///   committed text. Enter, Esc, Backspace, Delete and Space are the input method's while it
///   composes, and the first of them after a composition ends in the same pass (the key that
///   confirmed or cancelled it), as [`crate::ime`] describes: they don't commit, cancel, clear or
///   leave. An embedding control reads no keys of its own in such a pass ([`Typing::composing`]).
///   A composition open when the focus leaves keeps its marked text as typed, as `TextEdit`
///   does, so the commit carries it; one open when the field is no longer drawn is dropped.
/// - **In-field undo** (Cmd/Ctrl+Z, redo) is `TextEdit`'s, a change of the run like any other.
///   When the text changes from outside (a cancel, the host's own change) the undo history
///   starts again, so in-field undo never brings back text from before.
/// - **Host changes**: a host value other than the last reported cancels the run
///   ([`CancelReason::External`](crate::edit::CancelReason::External)) and shows the host's
///   text; the caret stays where it was, or at the end of a shorter text.
/// - **Focus**: a single-line field that gains the focus other than by a press in it (Tab, a dialog's first focus, a screen reader) selects the whole text; a click places the caret. An
///   enabled field is a stop of the [`FieldTab`](super::FieldTab) scope it is in, which a
///   dialog's Tab visits.
/// - **Keys of the embedding control**: [`event_filter`](Typing::event_filter) keeps more keys
///   with the field while it has the focus (a list's Up and Down, a popup's Esc).
/// - **Typed values** ([`Typing::with_codec`]): the field shows its value as the [`Codec`] writes
///   it. Where text is a draft rather than a value (a number), the run reports as updates only
///   the drafts that [apply as typed](Codec::applies_as_typed) and commits the draft read back,
///   so Enter, the focus leaving and a field no longer drawn all commit the same value. A draft
///   that can't become a value is marked invalid as it is typed ([`Typed::rejected`]; text that
///   still may, [`NotAValue::partial`], is not). A draft that reads as no value commits nothing
///   (the run ends at its start) and stays in the field, rejected with the codec's reason and
///   reported invalid, until the user fixes it, Esc restores the value, or the host's value
///   changes; a field no longer drawn drops it. An embedding
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
    keep_focus: bool,
    role: Role,
    invalid: bool,
    keys: EventFilter,
    tabular: bool,
    keep_rejected: bool,
    multiline: Option<usize>,
    edit_menu: bool,
    /// The menu the embedding control showed itself this pass.
    menu: Option<edit_menu::Outcome>,
}

impl<'a> Typing<'a> {
    /// The text `value` in the `TextEdit` with id `id`, named `label` for screen readers.
    pub fn new(id: Id, label: &'a str, value: &str) -> Self {
        Self::with_codec(id, label, value.to_owned(), PlainText)
    }

    /// Whether an input method composes in the field `id` in this pass: a composition was open
    /// when the pass began, or one opens or ends in its input. Its keys are its own then (the
    /// field takes them, as the type's doc says), so an embedding control asks this before it
    /// reads keys of its own and reads none in such a pass: the Enter that confirms a
    /// composition chooses no option, the Esc that cancels one closes no list. False while the
    /// field doesn't have the focus.
    pub fn composing(ui: &Ui, id: Id) -> bool {
        ime_pass(ui, id, false)
    }
}

/// [`Typing::composing`], also taking the keys the input method owns in this pass out of the
/// input, as the field's `show` does: for an embedding control whose own gesture would read Esc
/// before the field runs (a select's search field, drawn later in its popup).
pub(crate) fn take_composing_keys(ui: &Ui, id: Id) -> bool {
    ime_pass(ui, id, true)
}

/// Follows field `id`'s input method through this pass's events; with `take`, takes its keys.
fn ime_pass(ui: &Ui, id: Id, take: bool) -> bool {
    if !ui.memory(|m| m.has_focus(id)) {
        return false;
    }
    let state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
    let mut ime = ImeKeys::new(kept_preedit(ui, id, Seen::load(ui, id).as_ref(), &state).is_some());
    match take {
        true => ui.input_mut(|i| i.events.retain(|event| !ime.takes(event))),
        false => ui.input(|i| {
            for event in &i.events {
                ime.takes(event);
            }
        }),
    }
    ime.composed
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
            escape: Escape::PassThrough,
            keep_focus: false,
            role: Role::TextInput,
            invalid: false,
            keys: EventFilter::default(),
            tabular: false,
            keep_rejected: true,
            multiline: None,
            edit_menu: false,
            menu: None,
        }
    }

    /// A field of `rows` lines that wraps its text: Enter inserts a line break, and Cmd/Ctrl+Enter
    /// is what Enter is in a line (commits, leaves the field unless it
    /// [keeps the focus](Typing::keep_focus), and reports [`submitted`](EditResponse::submitted)).
    /// Tab leaves the field. The caller sizes the box in the [`TypingLook`] and names the AccessKit
    /// [role](Typing::role).
    pub fn multiline(mut self, rows: usize) -> Self {
        self.multiline = Some(rows.max(1));
        self
    }

    /// Whether a secondary click, or the Menu key or Shift+F10 in the focused field, opens the
    /// [`EditMenu`](super::EditMenu); off by default.
    pub fn edit_menu(mut self, on: bool) -> Self {
        self.edit_menu = on;
        self
    }

    /// Tells the field that its embedding control showed the edit menu itself this pass, ahead of
    /// its own key readers ([`edit_menu::show`]), and what came of it. The field then does not
    /// show it again.
    pub(crate) fn edit_menu_shown(mut self, outcome: edit_menu::Outcome) -> Self {
        self.menu = Some(outcome);
        self
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

    /// Whether the field keeps the focus after Enter commits; off by default: Enter leaves the
    /// field, as in PhotoCraft.
    pub fn keep_focus(mut self, keep: bool) -> Self {
        self.keep_focus = keep;
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

    /// Whether a draft that reads as no value stays in the field when its run ends, marked
    /// invalid (the default), or goes, so that the field shows the value again: PhotoCraft's hex
    /// field snaps back when the focus leaves it.
    pub fn keep_rejected(mut self, keep: bool) -> Self {
        self.keep_rejected = keep;
        self
    }

    /// Keys the embedding control reads while the field has the focus, on top of `TextEdit`'s
    /// and the run's Esc: egui keeps the focus on them (Tab, the arrows, Esc) and leaves them in
    /// the input. A searchable combo box keeps Up, Down and Esc for its open list; a number field
    /// keeps Up and Down for stepping. A control that keeps Esc takes the press before the field
    /// runs: one left in the input is the field's ([`Escape`]).
    pub fn event_filter(mut self, filter: EventFilter) -> Self {
        self.keys = filter;
        self
    }

    /// Takes this pass's input, runs `TextEdit` in the field's box at the `Ui`'s cursor, and
    /// reports the field to AccessKit. Paint the box and anything over it, then [`Typed::end`].
    pub fn show(self, ui: &mut Ui, look: &TypingLook) -> Typed<C> {
        let id = self.id;
        let enabled = self.enabled && ui.is_enabled();
        if enabled {
            super::FieldTab::register(ui, id);
        }
        let menu = match (self.edit_menu && enabled, self.menu) {
            (true, None) => edit_menu::show(ui, id, self.password, None),
            (true, Some(shown)) => shown,
            (false, _) => edit_menu::Outcome::default(),
        };
        let seen = Seen::load(ui, id);
        let mut state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
        let caret = state.cursor.char_range();
        let preedit = kept_preedit(ui, id, seen.as_ref(), &state);
        take_ime_keys(ui, preedit.is_some());
        let focused = enabled && ui.memory(|m| m.has_focus(id));
        let submitted = focused
            && match self.multiline {
                Some(_) => take_submit(ui),
                None => ui.input(enter_pressed),
            };
        let was_focused = seen.as_ref().is_some_and(|s| s.focused);
        // The field's Esc goes on to the host; one with the focus elsewhere (a scrub of a number
        // field) is the gesture's, as in every control.
        let passing = self.escape == Escape::PassThrough && enabled && (focused || was_focused);
        let gesture = match passing {
            true => Gesture::load_passing_escape(ui, id, &self.value, self.policy),
            false => Gesture::load(ui, id, &self.value, self.policy),
        };
        let mut gesture = gesture.pointer_selects(true).enabled(enabled);
        let seen_text = seen.as_ref().map(|s| s.text);
        let rejected = seen.as_ref().and_then(|s| s.rejected(ui, id));
        let draft = seen.and_then(|s| s.draft);
        // A run that ended since the last pass ended by a cancel (a commit ends in its own pass):
        // its text goes, rejected or not.
        let run_cancelled = draft.as_ref().is_some_and(|d| gesture.id() != Some(d.run));
        let draft = draft.filter(|_| !run_cancelled);
        let mut rejected = rejected.filter(|r| !run_cancelled && enabled && r.host == self.value);
        if self.escape == Escape::Clear
            && focused
            && !self.codec.format(gesture.shown()).is_empty()
            && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            clear(ui, &self.codec, &mut gesture);
        }
        // An Esc left in the input leaves the field and goes on. egui has already dropped the
        // focus on it unless the field kept Esc (a cancellable run under way).
        let escaped = enabled && (focused || was_focused) && ui.input(escape_pressed);
        if escaped {
            rejected = None;
            ui.memory_mut(|m| m.surrender_focus(id));
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
        let edit = match self.multiline {
            Some(rows) => TextEdit::multiline(&mut buffer).desired_rows(rows).lock_focus(false),
            None => TextEdit::singleline(&mut buffer).return_key(None::<KeyboardShortcut>),
        };
        let mut edit = edit
            .id(id)
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
        // A click in the edit menu is no click away from the field.
        let surrender = ui.ctx().options(|o| o.input_options.surrender_focus_on);
        if self.edit_menu && menu.pointer_on(ui.ctx(), id) {
            ui.ctx().options_mut(|o| o.input_options.surrender_focus_on = SurrenderFocusOn::Never);
        }
        let mut output = edit.show(ui);
        ui.ctx().options_mut(|o| o.input_options.surrender_focus_on = surrender);
        ui.set_style(saved);
        edit_menu::withdraw(ui, id);
        if self.edit_menu && enabled {
            keep_selection_under_secondary_press(ui, id, caret, &mut output.state, &output.response.response);
        }

        let marked = output.state.preedit().map(|(range, _)| range);
        let caret_moved = output.state.cursor.char_range() != caret || output.response.response.is_pointer_button_down_on();
        let composing = marked.is_some();
        let text = remove_chars(&buffer, marked);
        // `galley_pos` is where the text starts; a right-aligned galley's rect starts left of 0.
        let text_rect = Rect::from_min_size(output.galley_pos, output.galley.size());
        let response = output.response.response;
        let mut typed = Typed {
            id,
            escape: self.escape,
            keep_focus: self.keep_focus,
            keep_rejected: self.keep_rejected,
            escaped,
            codec: self.codec,
            host: self.value,
            was_focused,
            single_line: self.multiline.is_none(),
            gesture,
            response,
            text,
            buffer,
            text_rect,
            composing,
            submitted,
            caret_moved,
            filter,
            keys: self.keys,
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
    keep_focus: bool,
    keep_rejected: bool,
    /// Esc left the field in this pass and goes on to the host.
    escaped: bool,
    codec: C,
    /// The host's value this pass.
    host: C::Value,
    /// The field had the focus at the end of the last pass.
    was_focused: bool,
    single_line: bool,
    gesture: Gesture<C::Value>,
    response: Response,
    text: String,
    /// The text with the marked text in it.
    buffer: String,
    /// Where the laid-out text is.
    text_rect: Rect,
    composing: bool,
    submitted: bool,
    caret_moved: bool,
    /// `TextEdit`'s focus filter.
    filter: EventFilter,
    /// The embedding control's keys.
    keys: EventFilter,
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

    /// Whether the caret or the selection moved in this pass (the keys, a press or a drag in
    /// the text, a change of the text), or a press in the text placed it: the user works in the
    /// text. A combo box gives its active option back to the text then.
    pub fn caret_moved(&self) -> bool {
        self.caret_moved
    }

    /// Whether a plain Enter press commits the run in this pass ([`EditResponse::submitted`]).
    pub fn submitted(&self) -> bool {
        self.submitted
    }

    /// Replaces the keys given with [`Typing::event_filter`]: the embedding control's keys
    /// changed after the field ran in this pass (a list that opened or closed).
    pub fn set_event_filter(&mut self, filter: EventFilter) {
        self.keys = filter;
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

    /// Whether Esc left the field in this pass and goes on to the host
    /// ([`EditResponse::escaped`]).
    pub fn escaped(&self) -> bool {
        self.escaped
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
        if self.submitted && !self.keep_focus {
            ui.memory_mut(|m| m.surrender_focus(self.id));
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
        // finishes): text that reads as no value stays, rejected, unless Esc ended the run (a
        // policy that can't cancel commits it) or the field doesn't keep rejected text.
        let ending = self.submitted || self.commit || !focused || self.replaced;
        if let Some(draft) = self.draft.take_if(|_| ending)
            && self.keep_rejected
            && !self.replaced
            && !self.escaped
            && self.gesture.id() == Some(draft.run)
            && let Some(start) = self.gesture.start()
            && let Err(error) = self.codec.parse(&draft.text, start)
        {
            self.rejected = Some(Rejected { text: draft.text, reason: error.reason, host: self.host.clone() });
        }
        // A press in the field places the caret; any other way in (Tab, a dialog's first focus,
        // `request_focus`) selects the text.
        let pointer = ui.input(|i| i.pointer.any_pressed() || i.pointer.any_released()) && self.response.hovered();
        if focused && !self.was_focused && self.single_line && !pointer {
            select_all(ui, self.id, &self.buffer);
        }
        // Esc stays with the field while it is the input method's or clears text.
        let composing = self.composing && focused;
        let escape = composing || (self.escape == Escape::Clear && !self.codec.format(self.gesture.shown()).is_empty());
        let seen = Seen::new(ui, egui::util::hash(&self.text), focused, composing, self.draft.take());
        seen.store(ui, self.id, self.rejected.take());
        let filter = either(self.filter, self.keys);
        let event = self.gesture.event_filter(EventFilter { escape: escape || filter.escape, ..filter }).finish_as(report);
        EditResponse::new(self.response, event).submitted(self.submitted).escaped(self.escaped)
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
            // The host keeps the last value applied as typed while a draft reads as none or waits
            // for the commit.
            self.gesture.hold_updates(error.is_some() || !self.codec.applies_as_typed(&text));
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

/// What a field kept from its last pass. Rejected text, which carries the codec's value type,
/// is kept beside it under its own key, so the rest reads without the codec
/// ([`Typing::composing`]).
#[derive(Clone, Debug)]
struct Seen {
    /// The viewport and its pass that stored it.
    viewport: ViewportId,
    pass: u64,
    /// Hash of the committed text `TextEdit` was left with.
    text: u64,
    focused: bool,
    /// A composition was open and the field kept the focus: the marked text goes back in.
    composing: bool,
    draft: Option<Draft>,
}

impl Seen {
    fn new(ui: &Ui, text: u64, focused: bool, composing: bool, draft: Option<Draft>) -> Self {
        let (viewport, pass) = (ui.ctx().viewport_id(), ui.ctx().cumulative_pass_nr());
        Self { viewport, pass, text, focused, composing, draft }
    }

    /// What field `id` kept, when it was drawn in the last pass of this viewport (or earlier in
    /// this one, as [`Gesture::load`] counts); a field missing from a pass starts afresh.
    fn load(ui: &Ui, id: Id) -> Option<Self> {
        let (viewport, pass) = (ui.ctx().viewport_id(), ui.ctx().cumulative_pass_nr());
        ui.data(|d| d.get_temp::<Self>(Self::key(id))).filter(|s| s.viewport == viewport && s.pass + 1 >= pass)
    }

    /// The rejected text field `id` kept with this.
    fn rejected<T: Clone + Send + Sync + 'static>(&self, ui: &Ui, id: Id) -> Option<Rejected<T>> {
        ui.data(|d| d.get_temp::<Option<Rejected<T>>>(Self::rejected_key(id))).flatten()
    }

    fn store<T: Clone + Send + Sync + 'static>(self, ui: &Ui, id: Id, rejected: Option<Rejected<T>>) {
        ui.data_mut(|d| {
            d.insert_temp(Self::key(id), self);
            d.insert_temp(Self::rejected_key(id), rejected);
        });
    }

    fn key(id: Id) -> Id {
        id.with("craft_ui::behavior::typing")
    }

    fn rejected_key(id: Id) -> Id {
        id.with("craft_ui::behavior::typing::rejected")
    }
}

/// The marked text of field `id` (in `state`, its `TextEdit` state) that goes back into the
/// field, with its range: a composition goes on only in a field that kept it and the focus since
/// the last pass.
fn kept_preedit(ui: &Ui, id: Id, seen: Option<&Seen>, state: &TextEditState) -> Option<(Range<usize>, String)> {
    let kept = seen.is_some_and(|s| s.composing) && ui.memory(|m| m.owns_ime_events(id));
    state.preedit().filter(|_| kept).map(|(range, text)| (range, text.to_owned()))
}

/// A plain press of Enter: no modifier but Shift (as `consume_key` ignores it), not a held
/// key's repeat.
fn is_enter_press(event: &Event) -> bool {
    matches!(event, Event::Key { key: Key::Enter, pressed: true, repeat: false, modifiers, .. } if Modifiers { shift: false, ..*modifiers }.is_none())
}

/// Whether this pass's input has a plain press of Enter ([`is_enter_press`]).
fn enter_pressed(input: &InputState) -> bool {
    input.events.iter().any(is_enter_press)
}

/// Takes this pass's plain presses of Enter out of `ui`'s input, as the field would count them:
/// for an embedding control whose Enter does something else (a combo box choosing its active
/// option), so the field doesn't commit on it too.
pub(crate) fn take_enter(ui: &Ui) -> bool {
    ui.input_mut(|i| {
        let before = i.events.len();
        i.events.retain(|event| !is_enter_press(event));
        i.events.len() < before
    })
}

/// Takes this pass's Cmd/Ctrl+Enter presses out of `ui`'s input (a multi-line field's Enter): true
/// when one was no key repeat.
fn take_submit(ui: &Ui) -> bool {
    ui.input_mut(|i| {
        let mut pressed = false;
        i.events.retain(|event| match event {
            Event::Key { key: Key::Enter, pressed: true, repeat, modifiers, .. } if modifiers.command => {
                pressed |= !repeat;
                false
            }
            _ => true,
        });
        pressed
    })
}

/// A secondary press in the field moves the caret like a primary one; inside the selection the
/// field had before, the selection stays, so the menu's Cut and Copy act on it.
fn keep_selection_under_secondary_press(ui: &Ui, id: Id, before: Option<CCursorRange>, state: &mut TextEditState, response: &Response) {
    let (pressed, origin) = ui.input(|i| (i.pointer.button_pressed(PointerButton::Secondary), i.pointer.press_origin()));
    let Some(before) = before.filter(|b| !b.is_empty()) else { return };
    let inside = pressed && origin.is_some_and(|p| response.rect.contains(p));
    let range = before.as_sorted_char_range();
    if inside && state.cursor.char_range().is_some_and(|now| range.contains(&now.primary.index) || range.end == now.primary.index) {
        state.cursor.set_char_range(Some(before));
        state.clone().store(ui.ctx(), id);
    }
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
fn take_ime_keys(ui: &Ui, composing: bool) {
    let mut ime = ImeKeys::new(composing);
    ui.input_mut(|i| i.events.retain(|event| !ime.takes(event)));
}

/// An input method followed through a pass's events, in order.
struct ImeKeys {
    /// A composition is open.
    composing: bool,
    /// A composition ended and no key or text has come since.
    ended: bool,
    /// A composition was open at some point.
    composed: bool,
}

impl ImeKeys {
    /// Before the pass's first event; `composing` when a composition was open as it began.
    fn new(composing: bool) -> Self {
        Self { composing, ended: false, composed: composing }
    }

    /// Whether `event` is a key the input method takes ([`take_ime_keys`]).
    fn takes(&mut self, event: &Event) -> bool {
        match event {
            Event::Ime(ImeEvent::Preedit { text, .. }) if !text.is_empty() => {
                (self.composing, self.ended, self.composed) = (true, false, true);
                false
            }
            Event::Ime(ImeEvent::Preedit { .. } | ImeEvent::Commit(_)) => {
                (self.composing, self.ended) = (false, self.composing);
                false
            }
            Event::Key { key, pressed: true, .. } if is_ime_key(*key) && (self.composing || std::mem::take(&mut self.ended)) => true,
            Event::Key { pressed: true, .. } | Event::Text(_) => {
                self.ended = false;
                false
            }
            _ => false,
        }
    }
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
