//! Text-field and search-field behaviour in a headless egui context: the typing run, Esc, input
//! methods, in-field undo, host changes, disabling, and the search field's clearing.

use egui::{Context, Event, Id, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Ui, vec2};

use super::{SearchField, TextField};
use crate::edit::{self, CancelReason, EditEvent, EditPolicy, Ending, GestureHost, GestureId, UndoMarks};
use crate::theme::{Base, install};

/// A host with one field (a search field when `search`) between two egui buttons. It applies
/// every value it is sent and puts the value from before the run back on a cancel.
struct Rig {
    ctx: Context,
    time: f64,
    value: String,
    before: String,
    search: bool,
    drawn: bool,
    enabled: bool,
    policy: EditPolicy,
    id: Id,
    rect: Rect,
    /// The button before the field.
    before_button: Id,
    after_button: Id,
    /// The clear button's AccessKit node and rect, while it shows.
    clear: Option<(egui::accesskit::NodeId, Rect)>,
    submitted: bool,
    event: Option<EditEvent<String>>,
    /// The `Ui`'s next automatic id after the field.
    next_auto_id: Id,
    /// The field's AccessKit value in the last update.
    shown: Option<String>,
    /// Whether the value lives in `doc` behind `host` rather than in `value`.
    hosted: bool,
    doc: Doc,
    host: GestureHost<Doc>,
    /// What the host ended itself in the last frame.
    ended: Option<EditEvent<()>>,
}

impl Rig {
    fn new(value: &str) -> Self {
        Self::with(value, false)
    }

    fn search(value: &str) -> Self {
        Self::with(value, true)
    }

    /// A field whose text lives in a document behind a [`GestureHost`].
    fn hosted() -> Self {
        let mut rig = Self::with("", false);
        rig.hosted = true;
        rig
    }

    fn with(value: &str, search: bool) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let policy = if search { EditPolicy::default().cancellable(false) } else { EditPolicy::default() };
        let mut rig = Self {
            ctx,
            time: 0.0,
            value: value.to_owned(),
            before: value.to_owned(),
            search,
            drawn: true,
            enabled: true,
            policy,
            id: Id::NULL,
            rect: Rect::NOTHING,
            before_button: Id::NULL,
            after_button: Id::NULL,
            clear: None,
            submitted: false,
            event: None,
            next_auto_id: Id::NULL,
            shown: None,
            hosted: false,
            doc: Doc::default(),
            host: GestureHost::new(),
            ended: None,
        };
        rig.frame(vec![]);
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let output = ctx.run_ui(input, |ui| self.controls(ui));
        let update = output.platform_output.accesskit_update.clone();
        output.drop_without_applying_deltas();
        if let Some(update) = update {
            let id = self.id.accesskit_id();
            self.shown = update.nodes.iter().find(|(n, _)| *n == id).and_then(|(_, node)| node.value().map(str::to_owned));
            self.clear = update.nodes.iter().find(|(_, node)| node.label() == Some("Clear")).and_then(|(n, node)| {
                let b = node.bounds()?;
                Some((*n, Rect::from_min_max(Pos2::new(b.x0 as f32, b.y0 as f32), Pos2::new(b.x1 as f32, b.y1 as f32))))
            });
        }
    }

    fn controls(&mut self, ui: &mut Ui) {
        self.before_button = ui.button("Before").id;
        if self.drawn {
            let edit = if self.search {
                SearchField::new("rig-field", "Search", &self.value, "Clear").policy(self.policy).enabled(self.enabled).show(ui)
            } else {
                TextField::new("rig-field", "Name", &self.value).policy(self.policy).enabled(self.enabled).show(ui)
            };
            (self.id, self.rect) = (edit.response.id, edit.response.rect);
            self.submitted = edit.submitted;
            self.event.clone_from(&edit.event);
            if let Some(event) = edit.event {
                self.apply(ui, event);
            }
        } else {
            (self.submitted, self.event) = (false, None);
        }
        if self.hosted {
            self.ended = self.host.end_frame(ui.ctx(), &mut self.doc);
            self.value.clone_from(&self.doc.text);
        }
        self.next_auto_id = ui.next_auto_id();
        self.after_button = ui.button("After").id;
    }

    fn apply(&mut self, ui: &Ui, event: EditEvent<String>) {
        if self.hosted {
            self.host.handle(ui.ctx(), &mut self.doc, event, |doc, value| {
                doc.undo.push(doc.text.clone());
                doc.text = value;
            });
            self.value.clone_from(&self.doc.text);
            return;
        }
        match event {
            EditEvent::Begin { .. } => self.before.clone_from(&self.value),
            EditEvent::Cancel { reason: CancelReason::External, .. } => {}
            EditEvent::Cancel { .. } => self.value.clone_from(&self.before),
            other => {
                if let Some(v) = other.value() {
                    self.value.clone_from(v);
                }
            }
        }
    }

    fn type_text(&mut self, text: &str) {
        self.frame(vec![Event::Text(text.to_owned())]);
    }

    fn key(&mut self, key: Key) {
        self.chord(Modifiers::NONE, key);
    }

    fn chord(&mut self, modifiers: Modifiers, key: Key) {
        self.frame(vec![key_event(key, modifiers)]);
    }

    /// Gives the field the focus with the caret at the end.
    fn focus(&mut self) {
        self.ctx.memory_mut(|m| m.request_focus(self.id));
        self.frame(vec![]);
        self.key(Key::End);
    }

    fn has_focus(&self) -> bool {
        self.ctx.memory(|m| m.has_focus(self.id))
    }

    fn click(&mut self, pos: Pos2) {
        self.frame(vec![Event::PointerMoved(pos), button(pos, true)]);
        self.frame(vec![button(pos, false)]);
    }

    /// The caret's char index, or the selection's ends.
    fn cursor(&self) -> (usize, usize) {
        let range = egui::text_edit::TextEditState::load(&self.ctx, self.id).and_then(|s| s.cursor.char_range()).expect("a cursor");
        let [a, b] = range.sorted_cursors();
        (a.index.0, b.index.0)
    }
}

fn key_event(key: Key, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn begin(event: Option<EditEvent<String>>) -> GestureId {
    match event {
        Some(EditEvent::Begin { id }) => id,
        other => panic!("expected Begin, got {other:?}"),
    }
}

fn update(id: GestureId, value: &str) -> Option<EditEvent<String>> {
    Some(EditEvent::Update { id, value: value.to_owned(), modifiers: Modifiers::NONE })
}

fn commit(id: GestureId, value: &str, changed: bool) -> Option<EditEvent<String>> {
    Some(EditEvent::Commit { id, value: value.to_owned(), changed, modifiers: Modifiers::NONE })
}

fn cancel(id: GestureId, reason: CancelReason) -> Option<EditEvent<String>> {
    Some(EditEvent::Cancel { id, reason })
}

/// Types `text` one character a frame into the focused field: Begin, then an update per
/// character after the first.
fn type_run(rig: &mut Rig, text: &str) -> GestureId {
    let mut typed = rig.value.clone();
    let mut chars = text.chars();
    let first = chars.next().expect("text");
    rig.type_text(&first.to_string());
    let id = begin(rig.event.take());
    // The first change goes out the frame after Begin.
    rig.frame(vec![]);
    typed.push(first);
    assert_eq!(rig.event, update(id, &typed));
    for c in chars {
        rig.type_text(&c.to_string());
        typed.push(c);
        assert_eq!(rig.event, update(id, &typed));
    }
    id
}

#[test]
fn a_typing_run_is_one_gesture_that_enter_commits_keeping_the_focus() {
    let mut rig = Rig::new("");
    rig.focus();
    assert_eq!(rig.event, None, "focus alone begins nothing");
    let id = type_run(&mut rig, "abc");
    rig.key(Key::Enter);
    assert_eq!(rig.event, commit(id, "abc", true));
    assert!(rig.submitted && rig.has_focus());
    assert_eq!(rig.shown.as_deref(), Some("abc"));

    // Enter with nothing typed submits without an event.
    rig.key(Key::Enter);
    assert_eq!((rig.event.clone(), rig.submitted), (None, true));

    // Moving the focus away commits.
    let id = type_run(&mut rig, "d");
    rig.key(Key::Tab);
    assert_eq!(rig.event, commit(id, "abcd", true));
    assert!(!rig.has_focus() && !rig.submitted);

    // Typing and deleting back to the start commits unchanged.
    rig.focus();
    let id = type_run(&mut rig, "e");
    rig.key(Key::Backspace);
    assert_eq!(rig.event, update(id, "abcd"));
    rig.key(Key::Enter);
    assert_eq!(rig.event, commit(id, "abcd", false));
}

#[test]
fn a_typing_run_is_one_undo_step_and_a_field_no_longer_drawn_keeps_its_run() {
    let mut rig = Rig::hosted();
    rig.focus();
    let id = type_run(&mut rig, "abc");
    rig.key(Key::Enter);
    assert_eq!(rig.event, commit(id, "abc", true));
    assert_eq!((rig.doc.text.as_str(), rig.doc.undo.len()), ("abc", 1));

    // A drag that selects text doesn't make the run a drag: removed, it is kept as a step.
    let id = type_run(&mut rig, "d");
    let (start, end) = (rig.rect.left_center() + vec2(8.0, 0.0), rig.rect.center());
    rig.frame(vec![Event::PointerMoved(start), button(start, true)]);
    rig.frame(vec![Event::PointerMoved(end)]);
    rig.frame(vec![button(end, false)]);
    assert_eq!(edit::ending(&rig.ctx, id), Ending::Commit { changed: true });
    rig.drawn = false;
    rig.frame(vec![]);
    assert_eq!(rig.ended, Some(EditEvent::Commit { id, value: (), changed: true, modifiers: Modifiers::NONE }));
    assert_eq!((rig.doc.text.as_str(), rig.doc.undo.len()), ("abcd", 2));
}

/// A document of one text with an undo history of whole snapshots.
#[derive(Clone, Debug, Default)]
struct Doc {
    text: String,
    undo: Vec<String>,
}

impl UndoMarks for Doc {
    type Mark = usize;

    fn undo_mark(&self) -> usize {
        self.undo.len()
    }

    fn revert_to(&mut self, mark: usize) {
        if let Some(text) = self.undo.get(mark) {
            self.text.clone_from(text);
            self.undo.truncate(mark);
        }
    }

    fn squash_since(&mut self, mark: usize) {
        self.undo.truncate(mark + 1);
    }
}

#[test]
fn esc_cancels_the_run_and_keeps_the_focus_and_a_second_esc_leaves() {
    let mut rig = Rig::new("ab");
    rig.focus();
    let id = type_run(&mut rig, "cd");
    rig.key(Key::Escape);
    assert_eq!(rig.event, cancel(id, CancelReason::Escape));
    assert!(rig.has_focus());
    assert_eq!(rig.shown.as_deref(), Some("ab"));
    rig.key(Key::Escape);
    assert!(!rig.has_focus());

    // Not cancellable: Esc ends the run as a commit and leaves the field.
    rig.policy = EditPolicy::default().cancellable(false);
    rig.focus();
    let id = type_run(&mut rig, "e");
    rig.key(Key::Escape);
    assert_eq!(rig.event, commit(id, "abe", true));
    assert!(!rig.has_focus());
}

#[test]
fn after_a_cancel_the_field_shows_the_start_until_the_host_echoes_it() {
    let mut rig = Rig::new("ab");
    rig.focus();
    let id = type_run(&mut rig, "c");
    // A host that hasn't put the start back yet still shows the run's text.
    let mut keep = rig.value.clone();
    rig.key(Key::Escape);
    assert_eq!(rig.event, cancel(id, CancelReason::Escape));
    std::mem::swap(&mut keep, &mut rig.value);
    assert_eq!(rig.value, "abc");
    rig.frame(vec![]);
    assert_eq!(rig.shown.as_deref(), Some("ab"));
    rig.value = keep;
    rig.frame(vec![]);
    assert_eq!(rig.shown.as_deref(), Some("ab"));
}

#[test]
fn in_field_undo_steps_are_updates_of_the_run_and_never_reach_text_from_before_a_cancel() {
    let mut rig = Rig::new("");
    rig.focus();
    let id = type_run(&mut rig, "ab");
    rig.time += 2.0;
    rig.type_text(" c");
    assert_eq!(rig.event, update(id, "ab c"));
    rig.time += 2.0;
    rig.chord(Modifiers::COMMAND, Key::Z);
    assert_eq!(rig.event, update(id, "ab"));
    rig.chord(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
    assert_eq!(rig.event, update(id, "ab c"));

    rig.key(Key::Escape);
    assert_eq!((rig.event.clone(), rig.value.as_str()), (cancel(id, CancelReason::Escape), ""));
    rig.time += 2.0;
    rig.chord(Modifiers::COMMAND, Key::Z);
    assert_eq!((rig.event.clone(), rig.value.as_str()), (None, ""));
}

#[test]
fn a_host_change_during_a_run_cancels_it_and_the_caret_stays_inside_the_new_text() {
    let mut rig = Rig::new("");
    rig.focus();
    let id = type_run(&mut rig, "abcdef");
    assert_eq!(rig.cursor(), (6, 6));
    rig.value = "xy".to_owned();
    rig.frame(vec![]);
    assert_eq!(rig.event, cancel(id, CancelReason::External));
    rig.frame(vec![]);
    assert_eq!((rig.shown.as_deref(), rig.cursor()), (Some("xy"), (2, 2)));

    // With no run, a host change keeps a caret that is still inside.
    rig.key(Key::Home);
    rig.key(Key::ArrowRight);
    rig.value = "xyz".to_owned();
    rig.frame(vec![]);
    assert_eq!((rig.event.clone(), rig.cursor()), (None, (1, 1)));
}

#[test]
fn disabling_a_field_cancels_its_run_and_a_disabled_field_takes_no_input() {
    let mut rig = Rig::new("");
    let auto_id = rig.next_auto_id;
    rig.focus();
    let id = type_run(&mut rig, "ab");
    rig.enabled = false;
    rig.frame(vec![]);
    assert_eq!(rig.event, cancel(id, CancelReason::Disabled));
    assert_eq!(rig.next_auto_id, auto_id, "flipping enabled keeps the next automatic id");
    rig.frame(vec![]);
    assert!(!rig.has_focus());
    rig.click(rig.rect.center());
    rig.type_text("x");
    assert_eq!((rig.event.clone(), rig.value.as_str(), rig.has_focus()), (None, "", false));
    rig.enabled = true;
    rig.frame(vec![]);
    assert_eq!(rig.next_auto_id, auto_id);
}

#[test]
fn tab_selects_the_whole_text_and_a_click_places_the_caret() {
    let mut rig = Rig::new("hello");
    // Focus the button before the field, then Tab into the field.
    let before = rig.before_button;
    rig.ctx.memory_mut(|m| m.request_focus(before));
    rig.frame(vec![]);
    rig.key(Key::Tab);
    assert!(rig.has_focus());
    rig.frame(vec![]);
    assert_eq!(rig.cursor(), (0, 5));

    rig.key(Key::Tab);
    rig.click(rig.rect.left_center() + vec2(1.0, 0.0));
    assert!(rig.has_focus());
    assert_eq!(rig.cursor(), (0, 0));
}

#[test]
fn a_search_field_clears_on_esc_and_leaves_when_empty() {
    let mut rig = Rig::search("cat");
    rig.focus();
    rig.key(Key::Escape);
    assert_eq!(rig.event, Some(EditEvent::Set { value: String::new(), modifiers: Modifiers::NONE }));
    assert!(rig.has_focus());
    rig.key(Key::Escape);
    assert_eq!(rig.event, None);
    assert!(!rig.has_focus());

    // During a run, Esc clears as a change of the run and commits it.
    rig.focus();
    let id = type_run(&mut rig, "do");
    rig.key(Key::Escape);
    assert_eq!(rig.event, commit(id, "", false));
    assert!(rig.has_focus());
}

#[test]
fn the_clear_button_clears_commits_and_gives_the_field_the_focus_but_is_no_tab_stop() {
    let mut rig = Rig::search("cat");
    let (_, clear) = rig.clear.expect("a clear button while there is text");
    rig.click(clear.center());
    assert_eq!(rig.value, "");
    assert!(rig.has_focus());
    rig.frame(vec![]);
    assert!(rig.clear.is_none(), "no clear button in an empty field");

    rig.value = "cat".to_owned();
    rig.frame(vec![]);
    rig.focus();
    rig.key(Key::Tab);
    assert!(!rig.has_focus());
    assert!(rig.clear.is_some());
    assert_eq!(rig.ctx.memory(|m| m.focused()), Some(rig.after_button), "Tab skips the clear button");
}

#[cfg(feature = "screen-testing")]
mod ime {
    use super::*;
    use egui::ImeEvent;

    use crate::testing::{JAPANESE, KOREAN, ime_events};

    impl Rig {
        fn preedit(&mut self, text: &str) {
            let active = Some(0..text.chars().count());
            self.frame(vec![Event::Ime(ImeEvent::Preedit { text: text.to_owned(), active_range_chars: active })]);
        }
    }

    fn commit_event(text: &str) -> Event {
        Event::Ime(ImeEvent::Commit(text.to_owned()))
    }

    /// No event carries marked text: the composition's commit is the run's first change.
    #[test]
    fn marked_text_is_never_a_value() {
        let mut rig = Rig::new("a");
        rig.focus();
        let mut events = Vec::new();
        for event in ime_events(&JAPANESE) {
            rig.frame(vec![event]);
            events.extend(rig.event.take());
        }
        rig.frame(vec![]);
        events.extend(rig.event.take());
        let id = begin(events.first().cloned());
        assert_eq!(events[1..], [update(id, "a日本語").unwrap()]);
    }

    #[test]
    fn enter_that_confirms_a_composition_does_not_commit_the_run() {
        let mut rig = Rig::new("");
        rig.focus();
        let id = type_run(&mut rig, "a");
        for text in KOREAN.preedit {
            rig.preedit(text);
            assert_eq!(rig.event, None);
        }
        // The platform sends the commit and forwards the Enter that confirmed it.
        rig.frame(vec![commit_event(KOREAN.commit), key_event(Key::Enter, Modifiers::NONE)]);
        assert_eq!((rig.event.clone(), rig.submitted), (update(id, "a한"), false));
        rig.key(Key::Enter);
        assert_eq!(rig.event, commit(id, "a한", true));
    }

    #[test]
    fn esc_that_cancels_a_composition_does_not_cancel_the_run() {
        let mut rig = Rig::new("");
        rig.focus();
        let id = type_run(&mut rig, "a");
        rig.preedit("に");
        rig.frame(vec![Event::Ime(ImeEvent::Preedit { text: String::new(), active_range_chars: None }), key_event(Key::Escape, Modifiers::NONE)]);
        assert_eq!(rig.event, None);
        assert!(rig.has_focus());
        assert_eq!(rig.shown.as_deref(), Some("a"));
        rig.key(Key::Escape);
        assert_eq!(rig.event, cancel(id, CancelReason::Escape));

        // Esc while composing with no run under way doesn't leave the field either.
        rig.preedit("に");
        rig.key(Key::Escape);
        assert!(rig.has_focus());
    }

    #[test]
    fn a_composition_open_at_focus_loss_is_kept_as_typed_and_committed() {
        let mut rig = Rig::new("");
        rig.focus();
        let id = type_run(&mut rig, "a");
        rig.preedit("にほ");
        rig.key(Key::Tab);
        assert_eq!(rig.event, commit(id, "aにほ", true));
        assert_eq!(rig.shown.as_deref(), Some("aにほ"));

        // A composition with no run before it is one Set.
        // The field runs once without the focus, which ends the composition in `TextEdit`.
        rig.frame(vec![]);
        rig.focus();
        rig.preedit("に");
        rig.key(Key::Tab);
        assert_eq!(rig.event, Some(EditEvent::Set { value: "aにほに".to_owned(), modifiers: Modifiers::NONE }));
    }

    #[test]
    fn a_composition_open_when_the_field_goes_is_dropped() {
        let mut rig = Rig::hosted();
        rig.focus();
        let id = type_run(&mut rig, "a");
        rig.preedit("にほ");
        rig.drawn = false;
        rig.frame(vec![]);
        assert_eq!(rig.ended, Some(EditEvent::Commit { id, value: (), changed: true, modifiers: Modifiers::NONE }));
        assert_eq!((rig.doc.text.as_str(), rig.doc.undo.len()), ("a", 1));
    }
}
