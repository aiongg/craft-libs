//! Text-field and search-field behaviour in a headless egui context: the typing run, Esc, input
//! methods, in-field undo, host changes, disabling, and the search field's clearing.

use std::time::Duration;

use egui::{Context, Event, EventFilter, Id, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Ui, ViewportCommand, vec2};

use super::{SearchField, TextField};
use crate::behavior::{Typing, TypingLook};
use crate::edit::{self, CancelReason, EditEvent, EditPolicy, Ending, GestureHost, GestureId, UndoMarks, UpdateRate};
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
    keep_focus: bool,
    submitted: bool,
    escaped: bool,
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
    /// The keys pressed that were still in the input after the field.
    keys_after: Vec<Key>,
    edit_menu: bool,
    password: bool,
    /// A second field after the first, with its text.
    second: Option<String>,
    /// The keys still in the input after the fields in the frame of the last `choose` click.
    keys_chosen: Vec<Key>,
    multiline: Option<usize>,
    /// The AccessKit nodes of the last frame with a label: label, rect, disabled.
    nodes: Vec<(String, Rect, bool)>,
    /// The field's AccessKit role in the last frame.
    role: Option<egui::accesskit::Role>,
    /// What the last frame sent to the window and the clipboard.
    commands: Vec<ViewportCommand>,
    copied: Vec<String>,
    /// Every text copied since the rig began.
    ever_copied: Vec<String>,
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
            keep_focus: false,
            submitted: false,
            escaped: false,
            event: None,
            next_auto_id: Id::NULL,
            shown: None,
            hosted: false,
            doc: Doc::default(),
            host: GestureHost::new(),
            ended: None,
            keys_after: Vec::new(),
            edit_menu: true,
            password: false,
            second: None,
            keys_chosen: Vec::new(),
            multiline: None,
            nodes: Vec::new(),
            role: None,
            commands: Vec::new(),
            copied: Vec::new(),
            ever_copied: Vec::new(),
        };
        rig.frame(vec![]);
        rig
    }

    /// Runs one frame. Every key pressed in it is released at its end, so the next press is no
    /// repeat; a press marked as a repeat stays down (a held key).
    fn frame(&mut self, mut events: Vec<Event>) {
        let releases: Vec<Event> = events
            .iter()
            .filter_map(|e| match e {
                Event::Key { key, physical_key, pressed: true, repeat: false, modifiers } => {
                    Some(Event::Key { key: *key, physical_key: *physical_key, pressed: false, repeat: false, modifiers: *modifiers })
                }
                _ => None,
            })
            .collect();
        events.extend(releases);
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let output = ctx.run_ui(input, |ui| self.controls(ui));
        let update = output.platform_output.accesskit_update.clone();
        self.copied = output
            .platform_output
            .commands
            .iter()
            .filter_map(|c| if let egui::OutputCommand::CopyText(t) = c { Some(t.clone()) } else { None })
            .collect();
        self.ever_copied.extend(self.copied.iter().cloned());
        self.commands = output.viewport_output.values().flat_map(|v| v.commands.iter().cloned()).collect();
        output.drop_without_applying_deltas();
        if let Some(update) = update {
            let id = self.id.accesskit_id();
            self.role = update.nodes.iter().find(|(n, _)| *n == id).map(|(_, node)| node.role());
            self.nodes = update
                .nodes
                .iter()
                .filter_map(|(_, node)| {
                    let b = node.bounds()?;
                    let rect = Rect::from_min_max(Pos2::new(b.x0 as f32, b.y0 as f32), Pos2::new(b.x1 as f32, b.y1 as f32));
                    Some((node.label()?.to_owned(), rect, node.is_disabled()))
                })
                .collect();
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
                SearchField::new("rig-field", "Search", &self.value, "Clear")
                    .policy(self.policy)
                    .enabled(self.enabled)
                    .keep_focus(self.keep_focus)
                    .show(ui)
            } else {
                let field = TextField::new("rig-field", "Name", &self.value)
                    .policy(self.policy)
                    .enabled(self.enabled)
                    .keep_focus(self.keep_focus)
                    .edit_menu(self.edit_menu)
                    .password(self.password);
                match self.multiline {
                    Some(rows) => field.multiline(rows),
                    None => field,
                }
                .show(ui)
            };
            (self.id, self.rect) = (edit.response.id, edit.response.rect);
            (self.submitted, self.escaped) = (edit.submitted, edit.escaped);
            self.event.clone_from(&edit.event);
            if let Some(event) = edit.event {
                self.apply(ui, event);
            }
        } else {
            (self.submitted, self.escaped, self.event) = (false, false, None);
        }
        if let Some(second) = &mut self.second {
            let _ = TextField::new("rig-second", "Other", second).show(ui);
        }
        self.keys_after = pressed_keys(ui);
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

    /// A secondary click at `pos`.
    fn secondary_click(&mut self, pos: Pos2) {
        let button = |pressed| Event::PointerButton { pos, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE };
        self.frame(vec![Event::PointerMoved(pos)]);
        self.frame(vec![button(true)]);
        self.frame(vec![button(false)]);
        // The menu is measured in the pass that opens it and placed in the next.
        self.frame(vec![]);
        self.frame(vec![]);
    }

    fn menu_open(&self) -> bool {
        crate::behavior::edit_menu::is_open(&self.ctx, self.id)
    }

    /// The menu row labelled `label`: its rect and whether it is disabled.
    fn row(&self, label: &str) -> Option<(Rect, bool)> {
        self.nodes.iter().find(|(l, ..)| l == label).map(|(_, rect, disabled)| (*rect, *disabled))
    }

    /// Clicks the menu row `label`: the press and the release come in frames of their own, or in
    /// one frame when `one_frame`.
    fn click_row(&mut self, label: &str, one_frame: bool) {
        let (rect, disabled) = self.row(label).unwrap_or_else(|| panic!("a {label} row"));
        assert!(!disabled, "{label} is enabled");
        let at = rect.center();
        self.frame(vec![Event::PointerMoved(at)]);
        if one_frame {
            self.frame(vec![button(at, true), button(at, false)]);
        } else {
            self.frame(vec![Event::PointerMoved(at), button(at, true)]);
            self.frame(vec![button(at, false)]);
        }
        // The command runs in the frame of the release.
        let (commands, copied) = (self.commands.clone(), self.copied.clone());
        self.keys_chosen.clone_from(&self.keys_after);
        self.frame(vec![]);
        self.commands.extend(commands);
        self.copied.extend(copied);
    }

    /// Clicks the menu row `label`.
    fn choose(&mut self, label: &str) {
        self.click_row(label, false);
    }

    /// Clicks the menu row `label` with the press and the release in one frame, as a fast click
    /// or a touch tap arrives.
    fn choose_in_one_frame(&mut self, label: &str) {
        self.click_row(label, true);
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

fn pressed_keys(ui: &Ui) -> Vec<Key> {
    ui.input(|i| i.events.iter().filter_map(|e| if let Event::Key { key, pressed: true, .. } = e { Some(*key) } else { None }).collect())
}

fn key_event(key: Key, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn begin(event: Option<EditEvent<String>>) -> GestureId {
    match event {
        Some(EditEvent::Begin { id, .. }) => id,
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
fn a_typing_run_is_one_gesture_that_enter_commits_and_leaves_the_field() {
    let mut rig = Rig::new("");
    rig.focus();
    assert_eq!(rig.event, None, "focus alone begins nothing");
    let id = type_run(&mut rig, "abc");
    rig.key(Key::Enter);
    assert_eq!(rig.event, commit(id, "abc", true));
    assert!(rig.submitted && !rig.has_focus(), "Enter commits, submits and leaves the field");
    assert_eq!(rig.shown.as_deref(), Some("abc"));

    // Enter with nothing typed submits without an event, and leaves too.
    rig.focus();
    rig.key(Key::Enter);
    assert_eq!((rig.event.clone(), rig.submitted, rig.has_focus()), (None, true, false));

    // `keep_focus(true)`: the field stays.
    rig.keep_focus = true;
    rig.focus();
    let id = type_run(&mut rig, "d");
    rig.key(Key::Enter);
    assert_eq!(rig.event, commit(id, "abcd", true));
    assert!(rig.submitted && rig.has_focus());

    // Moving the focus away commits.
    let id = type_run(&mut rig, "e");
    rig.key(Key::Tab);
    assert_eq!(rig.event, commit(id, "abcde", true));
    assert!(!rig.has_focus() && !rig.submitted);

    // Typing and deleting back to the start commits unchanged.
    rig.focus();
    let id = type_run(&mut rig, "f");
    rig.key(Key::Backspace);
    assert_eq!(rig.event, update(id, "abcde"));
    rig.key(Key::Enter);
    assert_eq!(rig.event, commit(id, "abcde", false));
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
    rig.focus();
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

/// A run whose last text no update reported (reported only on release, or a debounce still
/// waiting) is kept with all of it when the field is no longer drawn.
#[test]
fn a_field_no_longer_drawn_keeps_the_text_no_update_reported_as_one_step() {
    for policy in [EditPolicy::default().on_release_only(true), EditPolicy::default().update(UpdateRate::Debounced(Duration::from_secs(1)))] {
        let mut rig = Rig::hosted();
        rig.policy = policy;
        rig.focus();
        rig.type_text("a");
        let id = begin(rig.event.take());
        rig.type_text("b");
        assert_eq!((rig.event.clone(), rig.doc.text.as_str()), (None, ""), "{policy:?}");
        rig.drawn = false;
        rig.frame(vec![]);
        assert_eq!(rig.ended, Some(EditEvent::Commit { id, value: (), changed: true, modifiers: Modifiers::NONE }), "{policy:?}");
        assert_eq!((rig.doc.text.as_str(), rig.doc.undo.as_slice()), ("ab", &[String::new()][..]), "{policy:?}");
    }
}

/// Enter submits and commits only as a plain press: Shift may be held, Cmd/Ctrl may not, and a
/// held key's repeats don't count.
#[test]
fn only_a_plain_enter_press_submits() {
    let mut rig = Rig::new("");
    rig.keep_focus = true;
    rig.focus();
    let id = type_run(&mut rig, "a");
    rig.chord(Modifiers::COMMAND, Key::Enter);
    assert_eq!((rig.event.clone(), rig.submitted), (None, false), "Cmd/Ctrl+Enter");
    rig.chord(Modifiers::SHIFT, Key::Enter);
    assert_eq!((rig.event.clone(), rig.submitted), (commit(id, "a", true), true), "Shift+Enter");

    let id = type_run(&mut rig, "b");
    let held = Event::Key { key: Key::Enter, physical_key: None, pressed: true, repeat: true, modifiers: Modifiers::NONE };
    rig.frame(vec![held.clone()]);
    assert_eq!((rig.event.clone(), rig.submitted), (commit(id, "ab", true), true), "a held Enter's first press");
    rig.frame(vec![held]);
    assert!(!rig.submitted, "its repeats");
}

/// A control embedding the field (a combo box's list, a number field's stepping) keeps the keys
/// it adds to the focus filter: the field keeps the focus and the keys reach the control.
#[test]
fn an_embedding_control_gets_the_keys_it_adds_to_the_focus_filter() {
    let ctx = Context::default();
    let id = Id::unique("embedded-field");
    let look = TypingLook {
        size: vec2(160.0, 24.0),
        margin: egui::Margin::symmetric(6, 0),
        align: egui::Align2::LEFT_CENTER,
        font: egui::FontId::proportional(12.0),
        text: egui::Color32::BLACK,
        placeholder: egui::Color32::GRAY,
        selection: egui::Color32::LIGHT_BLUE,
        selected_text: egui::Color32::BLACK,
        caret: egui::Color32::BLACK,
    };
    let keys = EventFilter { vertical_arrows: true, escape: true, ..Default::default() };
    // The control reads its Esc before the field runs, as an open list does.
    let frame = |events: Vec<Event>| {
        let mut seen = Vec::new();
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), events, ..Default::default() };
        let output = ctx.run_ui(input, |ui| {
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                seen.push(Key::Escape);
            }
            let edit = Typing::new(id, "Name", "ab").event_filter(keys).show(ui, &look).end(ui);
            assert!(!edit.escaped);
            seen.extend(pressed_keys(ui));
            let _ = ui.button("After");
        });
        output.drop_without_applying_deltas();
        seen
    };
    ctx.memory_mut(|m| m.request_focus(id));
    frame(vec![]);
    for key in [Key::ArrowDown, Key::ArrowUp, Key::Escape] {
        assert_eq!(frame(vec![key_event(key, Modifiers::NONE)]), [key]);
        assert!(ctx.memory(|m| m.has_focus(id)), "{key:?} keeps the focus");
    }
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
fn esc_cancels_the_run_leaves_the_field_and_passes_the_press_on() {
    let mut rig = Rig::new("ab");
    rig.focus();
    let id = type_run(&mut rig, "cd");
    rig.key(Key::Escape);
    assert_eq!(rig.event, cancel(id, CancelReason::Escape));
    assert!(!rig.has_focus(), "one Esc leaves");
    assert!(rig.escaped);
    assert_eq!(rig.keys_after, [Key::Escape], "the controls after the field see the press");
    rig.frame(vec![]);
    assert_eq!(rig.shown.as_deref(), Some("ab"));

    // Without a run, Esc leaves and goes on too.
    rig.focus();
    rig.key(Key::Escape);
    assert_eq!((rig.event.clone(), rig.has_focus(), rig.escaped), (None, false, true));
    assert_eq!(rig.keys_after, [Key::Escape]);

    // A field without the focus reports no Esc.
    rig.key(Key::Escape);
    assert!(!rig.escaped);

    // Not cancellable: Esc ends the run as a commit, leaves the field and goes on.
    rig.policy = EditPolicy::default().cancellable(false);
    rig.focus();
    let id = type_run(&mut rig, "e");
    rig.key(Key::Escape);
    assert_eq!(rig.event, commit(id, "abe", true));
    assert!(!rig.has_focus() && rig.escaped);
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
    // The pass after the run's last update takes the host's value as its write-back.
    rig.frame(vec![]);
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
fn a_field_focused_without_a_pointer_selects_its_text() {
    // A dialog's initial focus is a plain `request_focus`, with no key before it.
    let mut rig = Rig::new("Layer 1");
    rig.ctx.memory_mut(|m| m.request_focus(rig.id));
    rig.frame(vec![]);
    rig.frame(vec![]);
    assert_eq!(rig.cursor(), (0, 7));
}

#[test]
fn a_search_field_clears_on_esc_and_the_next_esc_leaves_and_goes_on() {
    let mut rig = Rig::search("cat");
    rig.focus();
    rig.key(Key::Escape);
    assert_eq!(rig.event, Some(EditEvent::Set { value: String::new(), modifiers: Modifiers::NONE }));
    assert!(rig.has_focus());
    assert!(!rig.escaped && rig.keys_after.is_empty(), "the clearing press is the field's");
    rig.key(Key::Escape);
    assert_eq!(rig.event, None);
    assert!(!rig.has_focus());
    assert!(rig.escaped && rig.keys_after == [Key::Escape], "the press that leaves goes on");

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
        assert!(!rig.has_focus());

        // Esc while composing with no run under way doesn't leave the field either.
        rig.focus();
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

    /// A field drawn again starts without the composition it had when it went, also when it gets
    /// the focus in the pass it comes back.
    #[test]
    fn a_field_drawn_again_starts_without_its_old_composition() {
        let mut rig = Rig::new("ab");
        rig.focus();
        rig.preedit("にほ");
        rig.drawn = false;
        rig.frame(vec![]);

        rig.drawn = true;
        rig.ctx.memory_mut(|m| m.request_focus(rig.id));
        rig.key(Key::Backspace);
        let id = begin(rig.event.take());
        rig.frame(vec![]);
        assert_eq!(rig.event, update(id, "a"));
        assert_eq!(rig.shown.as_deref(), Some("a"));
        let state = egui::text_edit::TextEditState::load(&rig.ctx, rig.id).expect("state");
        assert_eq!(state.preedit(), None);
    }

    /// A field drawn again without the focus leaves the input method's keys to the focused widget.
    #[test]
    fn a_field_drawn_again_without_the_focus_takes_no_keys() {
        let mut rig = Rig::new("ab");
        rig.focus();
        rig.preedit("にほ");
        rig.drawn = false;
        rig.frame(vec![]);

        rig.drawn = true;
        rig.ctx.memory_mut(|m| m.request_focus(rig.after_button));
        let keys = [Key::Space, Key::Backspace, Key::Enter];
        rig.frame(keys.iter().map(|&key| key_event(key, Modifiers::NONE)).collect());
        assert_eq!(rig.keys_after, keys);
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

#[test]
fn the_edit_menu_opens_on_a_secondary_press_and_keeps_the_focus() {
    let mut rig = Rig::new("hello");
    rig.secondary_click(rig.rect.center());
    assert!(rig.menu_open() && rig.has_focus(), "a secondary click opens the menu on the focused field");
    for label in ["Undo", "Cut", "Copy", "Paste", "Delete", "Select All"] {
        assert!(rig.row(label).is_some(), "{label} is in the menu");
    }
    // A click in the menu is no click away from the field: its run is not committed.
    rig.key(Key::Escape);
    rig.focus();
    rig.type_text("!");
    assert!(matches!(rig.event, Some(EditEvent::Begin { .. })));
    rig.secondary_click(rig.rect.center());
    rig.choose("Select All");
    assert!(!matches!(rig.event, Some(EditEvent::Commit { .. })), "the click that chose did not commit the run");
    assert!(!rig.menu_open() && rig.has_focus(), "choosing closes the menu and the field keeps the focus");
    assert_eq!(rig.cursor(), (0, 6), "Select All sent Cmd/Ctrl+A to the field");

    // Esc closes only the menu: the field neither leaves nor reports it.
    rig.secondary_click(rig.rect.center());
    assert!(rig.menu_open());
    rig.key(Key::Escape);
    assert!(!rig.escaped, "Esc is not the field's");
    rig.frame(vec![]);
    assert!(!rig.menu_open(), "Esc closes the menu");
    assert!(rig.has_focus(), "and has the focus after it");
}

#[test]
fn a_click_that_presses_and_releases_in_one_frame_chooses_and_keeps_the_focus() {
    let mut rig = Rig::new("hello");
    rig.focus();
    rig.type_text("!");
    assert!(matches!(rig.event, Some(EditEvent::Begin { .. })));
    rig.secondary_click(rig.rect.center());
    rig.choose_in_one_frame("Select All");
    assert!(!matches!(rig.event, Some(EditEvent::Commit { .. })), "the click that chose did not commit the run");
    assert!(!rig.menu_open() && rig.has_focus(), "choosing closes the menu and the field keeps the focus");
    assert_eq!(rig.cursor(), (0, 6), "Select All ran");
}

#[test]
fn a_command_is_the_input_the_shortcut_sends() {
    let mut rig = Rig::new("hello");
    rig.secondary_click(rig.rect.center());
    rig.choose("Select All");
    rig.secondary_click(rig.rect.center());
    rig.choose("Cut");
    rig.frame(vec![]);
    assert_eq!(rig.value, "", "Cut removes the selection");
    assert_eq!(rig.ever_copied, ["hello"], "Cut copies what it removes");
    rig.secondary_click(rig.rect.center());
    rig.choose("Undo");
    rig.frame(vec![]);
    assert_eq!(rig.value, "hello", "Undo is the field's own");
}

#[test]
fn cut_and_copy_are_disabled_without_a_selection() {
    let mut rig = Rig::new("hello");
    rig.secondary_click(rig.rect.right_center() - vec2(2.0, 0.0));
    for label in ["Cut", "Copy", "Delete"] {
        assert_eq!(rig.row(label).map(|(_, disabled)| disabled), Some(true), "{label} needs a selection");
    }
    for label in ["Undo", "Paste", "Select All"] {
        assert_eq!(rig.row(label).map(|(_, disabled)| disabled), Some(false), "{label} does not");
    }
    rig.key(Key::Escape);
    rig.focus();
    rig.chord(Modifiers::COMMAND, Key::A);
    // A secondary press inside the selection keeps it.
    rig.secondary_click(rig.rect.center());
    assert_eq!(rig.cursor(), (0, 5));
    for label in ["Cut", "Copy", "Delete"] {
        assert_eq!(rig.row(label).map(|(_, disabled)| disabled), Some(false), "{label} acts on the selection");
    }
    rig.choose("Copy");
    assert_eq!(rig.copied, ["hello"]);
}

#[test]
fn paste_sends_one_paste_event_to_the_focused_field() {
    let mut rig = Rig::new("ab");
    rig.secondary_click(rig.rect.center());
    rig.choose("Paste");
    let mut asked = rig.commands.iter().filter(|c| matches!(c, ViewportCommand::RequestPaste)).count();
    rig.frame(vec![]);
    asked += rig.commands.iter().filter(|c| matches!(c, ViewportCommand::RequestPaste)).count();
    assert_eq!(asked, 1, "the window is asked for the clipboard once");
    assert!(rig.has_focus());
    rig.frame(vec![Event::Paste("xy".to_owned())]);
    rig.frame(vec![]);
    assert_eq!(rig.value, "abxy");
}

#[test]
fn edit_menu_false_leaves_the_press_to_the_host() {
    let mut rig = Rig::new("hello");
    rig.edit_menu = false;
    rig.secondary_click(rig.rect.center());
    assert!(!rig.menu_open() && rig.row("Cut").is_none());
    rig.chord(Modifiers::SHIFT, Key::F10);
    assert!(!rig.menu_open());
}

#[test]
fn the_menu_key_opens_the_menu_of_the_focused_field() {
    let mut rig = Rig::new("hello");
    rig.focus();
    rig.key(Key::Menu);
    rig.frame(vec![]);
    assert!(rig.menu_open() && rig.has_focus());
}

#[test]
fn multiline_enter_inserts_a_break_and_ctrl_enter_submits() {
    let mut rig = Rig::new("ab");
    rig.multiline = Some(3);
    rig.frame(vec![]);
    rig.focus();
    rig.key(Key::Enter);
    rig.type_text("c");
    rig.frame(vec![]);
    assert_eq!(rig.value, "ab\nc", "Enter inserts a line break");
    assert!(!rig.submitted && rig.has_focus());
    rig.chord(Modifiers::COMMAND, Key::Enter);
    assert!(rig.submitted && !rig.has_focus(), "Cmd/Ctrl+Enter submits and leaves");
    assert_eq!(rig.value, "ab\nc", "and inserts nothing");
    assert!(matches!(rig.event, Some(EditEvent::Commit { .. })));
    // Tab leaves a multi-line field too.
    rig.focus();
    rig.key(Key::Tab);
    assert!(!rig.has_focus());
}

#[test]
fn a_multiline_field_is_taller_by_a_line_a_row_and_is_a_multiline_text_input() {
    let mut rig = Rig::new("ab");
    let one = rig.rect.height();
    assert_eq!(rig.role, Some(egui::accesskit::Role::TextInput));
    rig.multiline = Some(4);
    rig.frame(vec![]);
    rig.frame(vec![]);
    assert_eq!(rig.role, Some(egui::accesskit::Role::MultilineTextInput));
    assert!(rig.rect.height() > one * 2.0, "{} against {one}", rig.rect.height());
    rig.multiline = Some(1);
    rig.frame(vec![]);
    rig.frame(vec![]);
    assert_eq!(rig.rect.height(), one, "one row is a line's height");
}

#[test]
fn installed_labels_name_the_menu_rows() {
    let mut rig = Rig::new("hello");
    crate::behavior::EditMenu::install(
        &rig.ctx,
        crate::behavior::EditMenuLabels {
            undo: "Widerrufen".into(),
            cut: "Ausschneiden".into(),
            copy: "Kopieren".into(),
            paste: "Einfügen".into(),
            delete: "Löschen".into(),
            select_all: "Alles auswählen".into(),
        },
    );
    rig.secondary_click(rig.rect.center());
    assert!(rig.row("Alles auswählen").is_some() && rig.row("Select All").is_none());
}

#[test]
fn the_menu_key_then_down_down_enter_runs_a_command_and_neither_steps_nor_commits() {
    let mut rig = Rig::new("hello");
    rig.focus();
    rig.type_text("!");
    rig.key(Key::Menu);
    rig.frame(vec![]);
    assert!(rig.menu_open());
    // Undo is highlighted; Cut, Copy and Delete are disabled, so ↓ ↓ reaches Select All.
    rig.key(Key::ArrowDown);
    rig.key(Key::ArrowDown);
    rig.key(Key::Enter);
    rig.frame(vec![]);
    assert!(!rig.menu_open() && rig.has_focus());
    assert!(!rig.submitted && !matches!(rig.event, Some(EditEvent::Commit { .. })), "Enter chose the row; the field did not commit");
    assert_eq!(rig.value, "hello!", "the value is unchanged");
    assert_eq!(rig.cursor(), (0, 6), "the chosen command (Select All) ran");
}

#[test]
fn a_chosen_command_reaches_no_reader_after_the_field() {
    let mut rig = Rig::new("hello");
    rig.secondary_click(rig.rect.center());
    rig.choose("Select All");
    assert_eq!(rig.cursor(), (0, 5));
    assert!(rig.keys_chosen.is_empty(), "the Cmd/Ctrl+A given to the field is out of the input after it: {:?}", rig.keys_chosen);
    rig.secondary_click(rig.rect.center());
    rig.choose("Delete");
    assert!(rig.keys_chosen.is_empty(), "so is the Delete key: {:?}", rig.keys_chosen);
    rig.frame(vec![]);
    assert_eq!(rig.value, "", "and the field still acted on it");
}

#[test]
fn a_chosen_command_does_not_fire_on_a_later_field() {
    let mut rig = Rig::new("hello");
    rig.second = Some("other".to_owned());
    rig.frame(vec![]);
    rig.secondary_click(rig.rect.center());
    rig.choose("Select All");
    rig.key(Key::Tab);
    rig.frame(vec![]);
    assert!(!rig.has_focus() && rig.ctx.memory(|m| m.focused()).is_some(), "the focus moved on to the second field");
    assert_eq!(rig.second.as_deref(), Some("other"));
    rig.frame(vec![]);
    assert_eq!(rig.value, "hello");
}

#[test]
fn a_password_field_offers_neither_cut_nor_copy() {
    let mut rig = Rig::new("secret");
    rig.password = true;
    rig.frame(vec![]);
    rig.focus();
    rig.chord(Modifiers::COMMAND, Key::A);
    rig.secondary_click(rig.rect.center());
    assert_eq!(rig.row("Cut").map(|(_, disabled)| disabled), Some(true), "Cut would delete without copying");
    assert_eq!(rig.row("Copy").map(|(_, disabled)| disabled), Some(true));
    assert_eq!(rig.row("Delete").map(|(_, disabled)| disabled), Some(false), "Delete works on the selection");
}

#[test]
fn installed_labels_survive_a_memory_reset() {
    let rig = Rig::new("hello");
    let labels = crate::behavior::EditMenuLabels { undo: "Widerrufen".into(), ..Default::default() };
    crate::behavior::EditMenu::install(&rig.ctx, labels);
    rig.ctx.memory_mut(|m| *m = egui::Memory::default());
    assert_eq!(crate::behavior::EditMenu::labels(&rig.ctx).undo, "Widerrufen");
}
