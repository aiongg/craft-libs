//! Number-field behaviour in a headless egui context: typing runs read on commit, rejected text,
//! scrubbing, keys and the wheel, locales, units, mixed values, the pop-up slider and AccessKit.

use egui::accesskit::{Action, ActionData, ActionRequest, Node, Role, TreeId};
use egui::{Context, Event, Id, Key, Modifiers, MouseWheelUnit, PointerButton, Pos2, RawInput, Rect, Ui, pos2, vec2};

use super::NumberField;
use crate::behavior::{Typing, TypingLook};
use crate::edit::{CancelReason, EditEvent, EditPolicy, GestureHost, GestureId, UndoMarks};
use crate::localized::{LocaleNumbers, Unit, focus_first_field};
use crate::theme::{Base, install};

/// A host with one number field after an egui button. It applies every value it is sent and
/// puts the value from before the gesture back on a cancel.
struct Rig {
    ctx: Context,
    time: f64,
    value: f64,
    before: f64,
    numbers: LocaleNumbers,
    range: (f64, f64),
    decimals: (usize, usize),
    unit: Unit,
    mixed: bool,
    scrub: bool,
    popup: bool,
    arithmetic: bool,
    drawn: bool,
    policy: EditPolicy,
    id: Id,
    rect: Rect,
    event: Option<EditEvent<f64>>,
    /// Every event in order.
    events: Vec<EditEvent<f64>>,
    /// The field's AccessKit node and its children's roles in the last update.
    node: Option<Node>,
    children: Vec<Role>,
    /// Whether the value lives in `doc` behind `host`.
    hosted: bool,
    doc: Doc,
    host: GestureHost<Doc>,
}

impl Rig {
    fn new(value: f64) -> Self {
        Self::with(value, |_| {})
    }

    fn with(value: f64, setup: impl FnOnce(&mut Self)) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut rig = Self {
            ctx,
            time: 0.0,
            value,
            before: value,
            numbers: LocaleNumbers::from_tag("en"),
            range: (f64::NEG_INFINITY, f64::INFINITY),
            decimals: (0, 2),
            unit: Unit::default(),
            mixed: false,
            scrub: false,
            popup: false,
            arithmetic: false,
            drawn: true,
            policy: EditPolicy::default(),
            id: Id::NULL,
            rect: Rect::NOTHING,
            event: None,
            events: Vec::new(),
            node: None,
            children: Vec::new(),
            hosted: false,
            doc: Doc::default(),
            host: GestureHost::new(),
        };
        setup(&mut rig);
        rig.doc.value = rig.value;
        rig.frame(vec![]);
        rig.frame(vec![]);
        rig
    }

    /// Runs one frame; keys pressed in it are released at its end.
    fn frame(&mut self, events: Vec<Event>) {
        self.frame_with(Modifiers::NONE, events);
    }

    /// Runs one frame with `modifiers` held.
    fn frame_with(&mut self, modifiers: Modifiers, mut events: Vec<Event>) {
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
        events.insert(0, Event::ModifiersChanged(modifiers));
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let output = ctx.run_ui(input, |ui| self.controls(ui));
        let update = output.platform_output.accesskit_update.clone();
        output.drop_without_applying_deltas();
        if let Some(update) = update {
            let id = self.id.accesskit_id();
            let node = update.nodes.iter().find(|(n, _)| *n == id).map(|(_, node)| node.clone());
            self.children = node
                .iter()
                .flat_map(|n| n.children().to_vec())
                .filter_map(|child| update.nodes.iter().find(|(n, _)| *n == child).map(|(_, node)| node.role()))
                .collect();
            self.node = node;
        }
    }

    fn controls(&mut self, ui: &mut Ui) {
        let _ = ui.button("Before");
        if !self.drawn {
            self.event = None;
            if self.hosted {
                self.host.end_frame(ui.ctx(), &mut self.doc);
                self.value = self.doc.value;
            }
            return;
        }
        let numbers = self.numbers.clone();
        let sum = |text: &str| text.split_once('+').and_then(|(a, b)| Some(a.trim().parse::<f64>().ok()? + b.trim().parse::<f64>().ok()?));
        let mut field = NumberField::new("rig-number", "Width", self.value, &numbers)
            .range(self.range.0..=self.range.1)
            .decimals(self.decimals.0..=self.decimals.1)
            .unit(self.unit.clone())
            .mixed(self.mixed)
            .policy(self.policy);
        if self.scrub {
            field = field.scrub_label("W");
        }
        if self.popup {
            field = field.popup_slider("Width slider");
        }
        if self.arithmetic {
            field = field.or_parse(sum);
        }
        let edit = field.show(ui);
        (self.id, self.rect) = (edit.response.id, edit.response.rect);
        self.event.clone_from(&edit.event);
        if let Some(event) = edit.event {
            self.events.push(event.clone());
            self.apply(ui, event);
        }
        if self.hosted {
            self.host.end_frame(ui.ctx(), &mut self.doc);
            self.value = self.doc.value;
        }
    }

    fn apply(&mut self, ui: &Ui, event: EditEvent<f64>) {
        if self.hosted {
            self.host.handle(ui.ctx(), &mut self.doc, event, |doc, value| {
                doc.undo.push(doc.value);
                doc.value = value;
            });
            self.value = self.doc.value;
            return;
        }
        match event {
            EditEvent::Begin { .. } => self.before = self.value,
            EditEvent::Cancel { reason: CancelReason::External, .. } | EditEvent::Commit { changed: false, .. } => {}
            EditEvent::Cancel { .. } => self.value = self.before,
            other => {
                if let Some(&v) = other.value() {
                    self.value = v;
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
        self.frame(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]);
    }

    /// Gives the field the focus with its whole text selected, as a dialog's first field.
    fn focus(&mut self) {
        focus_first_field(&self.ctx, true);
        self.frame(vec![]);
        self.frame(vec![]);
    }

    fn blur(&mut self) {
        self.ctx.memory_mut(|m| m.surrender_focus(self.id));
        self.frame(vec![]);
    }

    fn has_focus(&self) -> bool {
        self.ctx.memory(|m| m.has_focus(self.id))
    }

    /// The text the field shows, with its unit, as AccessKit reads it; `None` while blank.
    fn shown(&self) -> Option<String> {
        self.node.as_ref().and_then(|n| n.value().filter(|v| !v.is_empty()).map(str::to_owned))
    }

    fn invalid(&self) -> bool {
        self.node.as_ref().is_some_and(|n| n.invalid().is_some())
    }

    /// Presses the primary button at `from`, moves to each x offset in `moves` (a frame each)
    /// with `modifiers` held, and releases there when `release`.
    fn drag(&mut self, from: Pos2, moves: &[f32], modifiers: Modifiers, release: bool) {
        self.frame(vec![Event::PointerMoved(from), pointer(from, true)]);
        for dx in moves {
            self.frame_with(modifiers, vec![Event::PointerMoved(from + vec2(*dx, 0.0))]);
        }
        if release {
            let at = from + vec2(moves.last().copied().unwrap_or(0.0), 0.0);
            self.frame_with(modifiers, vec![pointer(at, false)]);
        }
    }

    /// The inside scrub label.
    fn handle(&self) -> Pos2 {
        pos2(self.rect.left() + 8.0, self.rect.center().y)
    }

    fn request(&mut self, action: Action, data: Option<ActionData>) {
        let request = ActionRequest { action, target_tree: TreeId::ROOT, target_node: self.id.accesskit_id(), data };
        self.frame(vec![Event::AccessKitActionRequest(request)]);
    }
}

fn pointer(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

#[derive(Clone, Debug, Default)]
struct Doc {
    value: f64,
    undo: Vec<f64>,
}

impl UndoMarks for Doc {
    type Mark = usize;

    fn undo_mark(&self) -> usize {
        self.undo.len()
    }

    fn revert_to(&mut self, mark: usize) {
        if let Some(&v) = self.undo.get(mark) {
            self.value = v;
            self.undo.truncate(mark);
        }
    }

    fn squash_since(&mut self, mark: usize) {
        self.undo.truncate(mark + 1);
    }
}

fn begin_id(events: &[EditEvent<f64>]) -> GestureId {
    match events.first() {
        Some(EditEvent::Begin { id }) => *id,
        other => panic!("expected Begin, got {other:?}"),
    }
}

fn since(rig: &Rig, from: usize) -> Vec<EditEvent<f64>> {
    rig.events[from..].to_vec()
}

fn commit(id: GestureId, value: f64, changed: bool) -> EditEvent<f64> {
    EditEvent::Commit { id, value, changed, modifiers: Modifiers::NONE }
}

fn set(value: f64) -> EditEvent<f64> {
    EditEvent::Set { value, modifiers: Modifiers::NONE }
}

#[test]
fn typing_commits_once_on_enter_and_on_focus_loss_with_the_value_read_rounded_and_clamped() {
    let mut rig = Rig::with(5.0, |r| r.range = (0.0, 100.0));
    rig.focus();
    let n = rig.events.len();
    // Partial text isn't a value: Begin, then nothing until Enter.
    for c in ["-", "1", "2", ".", "3", "4", "5"] {
        rig.type_text(c);
    }
    let id = begin_id(&since(&rig, n));
    assert_eq!(since(&rig, n).len(), 1, "no updates while typing: {:?}", since(&rig, n));
    rig.key(Key::Enter);
    // -12.345 rounds to two decimals, then clamps to the range.
    assert_eq!(rig.event, Some(commit(id, 0.0, true)));
    assert_eq!(rig.value, 0.0);
    assert!(rig.has_focus());

    rig.chord(Modifiers::COMMAND, Key::A);
    let n = rig.events.len();
    rig.type_text("2.375");
    let id = begin_id(&since(&rig, n));
    rig.blur();
    // Half away from zero.
    assert_eq!(rig.event, Some(commit(id, 2.38, true)));
    assert_eq!(since(&rig, n).len(), 2);
}

#[test]
fn text_that_is_no_number_keeps_the_value_stays_outlined_and_esc_restores() {
    let mut rig = Rig::new(7.0);
    rig.focus();
    let n = rig.events.len();
    rig.type_text("abc");
    let id = begin_id(&since(&rig, n));
    rig.key(Key::Enter);
    assert_eq!(rig.event, Some(commit(id, 7.0, false)));
    rig.frame(vec![]);
    assert_eq!((rig.value, rig.shown().as_deref(), rig.invalid()), (7.0, Some("abc"), true));
    // It stays while the focus leaves and comes back.
    rig.blur();
    assert_eq!((rig.shown().as_deref(), rig.invalid()), (Some("abc"), true));
    rig.ctx.memory_mut(|m| m.request_focus(rig.id));
    rig.frame(vec![]);
    rig.key(Key::Escape);
    assert!(rig.has_focus(), "Esc restores and keeps the focus");
    rig.frame(vec![]);
    assert_eq!((rig.shown().as_deref(), rig.invalid(), rig.event.clone()), (Some("7"), false, None));

    // Text fixed and committed clears the mark.
    rig.chord(Modifiers::COMMAND, Key::A);
    rig.type_text("x");
    rig.key(Key::Enter);
    rig.frame(vec![]);
    assert!(rig.invalid());
    rig.chord(Modifiers::COMMAND, Key::A);
    rig.type_text("9");
    rig.key(Key::Enter);
    rig.frame(vec![]);
    assert_eq!((rig.value, rig.invalid(), rig.shown().as_deref()), (9.0, false, Some("9")));
}

#[test]
fn a_field_no_longer_drawn_commits_its_typed_number_as_one_step_and_drops_bad_text() {
    let mut rig = Rig::with(3.0, |r| r.hosted = true);
    rig.focus();
    rig.type_text("42");
    rig.drawn = false;
    rig.frame(vec![]);
    rig.frame(vec![]);
    assert_eq!((rig.value, rig.doc.undo.len()), (42.0, 1));

    rig.drawn = true;
    rig.frame(vec![]);
    rig.focus();
    rig.type_text("nope");
    rig.drawn = false;
    rig.frame(vec![]);
    rig.frame(vec![]);
    assert_eq!((rig.value, rig.doc.undo.len()), (42.0, 1));
    rig.drawn = true;
    rig.frame(vec![]);
    rig.frame(vec![]);
    assert_eq!((rig.shown().as_deref(), rig.invalid()), (Some("42"), false));
}

#[test]
fn a_scrub_begins_updates_and_commits_a_step_a_point_with_shift_and_alt() {
    let mut rig = Rig::with(10.0, |r| r.scrub = true);
    let n = rig.events.len();
    rig.drag(rig.handle(), &[10.0, 15.0], Modifiers::NONE, true);
    let events = since(&rig, n);
    let id = begin_id(&events);
    // The points moved before egui starts the drag count too.
    assert_eq!(&events[1..], &[EditEvent::Update { id, value: 25.0, modifiers: Modifiers::NONE }, commit(id, 25.0, true)][..]);
    assert_eq!(rig.value, 25.0);

    rig.drag(rig.handle(), &[10.0, 12.0], Modifiers::SHIFT, true);
    assert_eq!(rig.value, 25.0 + 10.0 * 12.0);
    rig.drag(rig.handle(), &[10.0, 25.0], Modifiers::ALT, true);
    assert_eq!(rig.value, 145.0 + 2.0, "a tenth as fast, the rest carried");
}

#[test]
fn esc_cancels_a_scrub_and_a_still_press_leaves_the_value_exactly() {
    let mut rig = Rig::with(10.0, |r| r.scrub = true);
    rig.drag(rig.handle(), &[10.0, 20.0], Modifiers::NONE, false);
    rig.frame(vec![]);
    assert_eq!(rig.value, 30.0);
    rig.key(Key::Escape);
    assert!(matches!(rig.event, Some(EditEvent::Cancel { reason: CancelReason::Escape, .. })), "{:?}", rig.event);
    assert_eq!(rig.value, 10.0);
    rig.frame(vec![pointer(rig.handle() + vec2(20.0, 0.0), false)]);
    assert_eq!(rig.value, 10.0);

    // A value with more decimals than shown survives a press that doesn't move.
    let mut rig = Rig::with(33.333, |r| {
        r.scrub = true;
        r.decimals = (0, 1);
    });
    let n = rig.events.len();
    let at = rig.handle();
    rig.frame(vec![Event::PointerMoved(at), pointer(at, true)]);
    rig.time += 1.0;
    for _ in 0..3 {
        rig.frame(vec![]);
    }
    rig.frame(vec![pointer(at, false)]);
    let events = since(&rig, n);
    assert!(events.iter().all(|e| matches!(e, EditEvent::Begin { .. } | EditEvent::Commit { changed: false, .. })), "{events:?}");
    assert_eq!(rig.value, 33.333);
}

#[test]
fn arrows_and_the_wheel_each_give_one_set_and_a_typing_run_commits_its_step() {
    let mut rig = Rig::with(5.1, |r| r.arithmetic = true);
    rig.focus();
    let n = rig.events.len();
    rig.key(Key::ArrowUp);
    assert_eq!(since(&rig, n), [set(6.0)]);
    rig.chord(Modifiers::SHIFT, Key::ArrowDown);
    assert_eq!(rig.value, -4.0);
    rig.chord(Modifiers::COMMAND, Key::ArrowUp);
    assert_eq!(rig.value, -3.9);
    let n = rig.events.len();
    let over = rig.rect.center();
    let wheel = Event::MouseWheel {
        unit: MouseWheelUnit::Line,
        delta: vec2(0.0, 2.0),
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
    };
    rig.frame(vec![Event::PointerMoved(over), wheel]);
    assert_eq!(since(&rig, n), [set(-2.0)]);

    // Typed arithmetic, then an arrow: the run commits its result stepped.
    rig.chord(Modifiers::COMMAND, Key::A);
    let n = rig.events.len();
    rig.type_text("5+5");
    let id = begin_id(&since(&rig, n));
    rig.key(Key::ArrowUp);
    assert_eq!(rig.event, Some(commit(id, 11.0, true)));
    assert_eq!(since(&rig, n).len(), 2);
    rig.frame(vec![]);
    assert_eq!(rig.shown().as_deref(), Some("11"));
}

#[test]
fn german_and_arabic_digits_round_trip_with_the_unit_typed_or_not() {
    let mut rig = Rig::with(1234.5, |r| {
        r.numbers = LocaleNumbers::from_tag("de");
        r.unit = Unit::new("{n} px");
    });
    assert_eq!(rig.shown().as_deref(), Some("1.234,5 px"));
    rig.focus();
    rig.type_text("2,25 px");
    rig.key(Key::Enter);
    assert_eq!(rig.value, 2.25);
    rig.chord(Modifiers::COMMAND, Key::A);
    rig.type_text("7,5");
    rig.key(Key::Enter);
    assert_eq!(rig.value, 7.5);

    let mut rig = Rig::with(12.5, |r| r.numbers = LocaleNumbers::from_tag("ar-EG"));
    let shown = rig.shown().expect("a value");
    assert!(shown.chars().any(|c| ('\u{660}'..='\u{669}').contains(&c)), "{shown}");
    rig.focus();
    rig.type_text("٣٫٢٥");
    rig.key(Key::Enter);
    assert_eq!(rig.value, 3.25);
}

#[test]
fn the_text_as_shown_keeps_decimals_the_field_does_not_show() {
    let mut rig = Rig::with(1.23456, |r| r.decimals = (0, 2));
    assert_eq!(rig.shown().as_deref(), Some("1.23"));
    rig.focus();
    rig.key(Key::End);
    rig.type_text("4");
    rig.key(Key::Backspace);
    rig.key(Key::Enter);
    assert!(matches!(rig.event, Some(EditEvent::Commit { changed: false, .. })), "{:?}", rig.event);
    assert_eq!(rig.value, 1.23456);
}

#[test]
fn a_mixed_field_is_blank_commits_a_typed_number_and_steps_from_the_host_value() {
    let mut rig = Rig::with(20.0, |r| r.mixed = true);
    assert_eq!(rig.shown(), None);
    assert_eq!(rig.node.as_ref().and_then(Node::numeric_value), None);
    rig.focus();
    rig.key(Key::Enter);
    assert_eq!(rig.event, None, "blank commits nothing");
    rig.type_text("20");
    rig.key(Key::Enter);
    assert!(matches!(rig.event, Some(EditEvent::Commit { value: 20.0, changed: true, .. })), "the host's value typed is a change: {:?}", rig.event);

    let mut rig = Rig::with(20.0, |r| r.mixed = true);
    rig.focus();
    rig.key(Key::ArrowUp);
    assert_eq!(rig.event, Some(set(21.0)));

    let mut rig = Rig::new(f64::NAN);
    assert_eq!(rig.shown(), None);
    rig.focus();
    rig.key(Key::ArrowUp);
    assert_eq!(rig.event, Some(set(1.0)));
}

#[test]
fn accesskit_sees_one_spin_button_with_its_value_bounds_step_and_requests() {
    let mut rig = Rig::with(40.0, |r| {
        r.range = (0.0, 100.0);
        r.unit = Unit::new("{n}%");
    });
    let node = rig.node.clone().expect("a node");
    assert_eq!(node.role(), Role::SpinButton);
    assert!(!rig.children.contains(&Role::TextInput), "{:?}", rig.children);
    assert_eq!((node.label(), node.value()), (Some("Width"), Some("40%")));
    let numbers = (node.numeric_value(), node.min_numeric_value(), node.max_numeric_value(), node.numeric_value_step());
    assert_eq!(numbers, (Some(40.0), Some(0.0), Some(100.0), Some(1.0)));
    assert!(node.supports_action(Action::Increment) && node.supports_action(Action::Decrement));
    rig.request(Action::Increment, None);
    assert_eq!(rig.event, Some(set(41.0)));
    rig.request(Action::SetValue, Some(ActionData::NumericValue(250.0)));
    assert_eq!(rig.event, Some(set(100.0)));
    rig.frame(vec![]);
    assert!(!rig.node.as_ref().expect("a node").supports_action(Action::Increment));
}

#[test]
fn a_press_on_the_popup_arrow_and_a_drag_is_one_gesture_and_closes_the_slider() {
    let mut rig = Rig::with(50.0, |r| {
        r.range = (0.0, 100.0);
        r.popup = true;
        r.hosted = true;
    });
    let arrow = pos2(rig.rect.right() - 8.0, rig.rect.center().y);
    let n = rig.events.len();
    rig.drag(arrow, &[10.0, 20.0, 40.0], Modifiers::NONE, false);
    assert!(egui::Popup::is_any_open(&rig.ctx));
    rig.frame(vec![pointer(arrow + vec2(40.0, 0.0), false)]);
    let events = since(&rig, n);
    let id = begin_id(&events);
    assert!(matches!(events.last(), Some(EditEvent::Commit { id: c, changed: true, .. }) if *c == id), "{events:?}");
    assert!(rig.value > 50.0, "{}", rig.value);
    assert_eq!(rig.doc.undo.len(), 1);
    rig.frame(vec![]);
    assert!(!egui::Popup::is_any_open(&rig.ctx));
}

#[test]
fn tabular_digits_keep_the_text_width_when_the_digits_change() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let look = TypingLook {
        size: vec2(160.0, 24.0),
        margin: egui::Margin::symmetric(6, 0),
        align: egui::Align2::RIGHT_CENTER,
        font: egui::FontId::proportional(13.0),
        text: egui::Color32::BLACK,
        placeholder: egui::Color32::GRAY,
        selection: egui::Color32::LIGHT_BLUE,
        selected_text: egui::Color32::BLACK,
        caret: egui::Color32::BLACK,
    };
    let width = |text: &str| {
        let mut width = 0.0;
        for _ in 0..2 {
            let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), ..Default::default() };
            let output = ctx.run_ui(input, |ui| {
                let typed = Typing::new(Id::unique("tabular"), "Digits", text).tabular(true).show(ui, &look);
                width = typed.text_rect().width();
                let _ = typed.end(ui);
            });
            output.drop_without_applying_deltas();
        }
        width
    };
    assert!(width("1111") > 0.0);
    assert_eq!(width("1111"), width("8888"));
}

#[test]
fn focus_first_field_selects_the_text_so_typing_replaces_it() {
    let mut rig = Rig::new(640.0);
    focus_first_field(&rig.ctx, true);
    rig.frame(vec![]);
    assert!(rig.has_focus());
    rig.type_text("3");
    rig.key(Key::Enter);
    assert_eq!(rig.value, 3.0);
}
