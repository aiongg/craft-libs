//! Number-field behaviour in a headless egui context: typing runs read on commit, rejected text,
//! scrubbing, keys and the wheel, locales, units, mixed values, the pop-up slider and AccessKit.

use egui::accesskit::{Action, ActionData, ActionRequest, Node, Role, TreeId};
use egui::epaint::{ClippedShape, Shape};
use egui::{Context, CursorIcon, Event, Id, Key, Modifiers, MouseWheelUnit, PointerButton, Pos2, RawInput, Rect, Ui, ViewportCommand, pos2, vec2};

use super::number_field::{Travel, travel};
use super::{NumberField, SIZE_NAMES, number_field};
use crate::behavior::{Typing, TypingLook};
use crate::direction::{self, Direction};
use crate::edit::{self, CancelReason, EditEvent, EditPolicy, GestureHost, GestureId, UndoMarks};
use crate::localized::{LocaleNumbers, Unit, focus_first_field};
use crate::story::Args;
use crate::theme::{Base, install};

/// A host with one number field after an egui button. It applies every value it is sent (as
/// `adjust` makes it), keeps a mixed field mixed until a commit or a set, and puts the value from
/// before the gesture back on a cancel.
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
    /// A drag on the text scrubs.
    scrub_text: bool,
    /// The controls are in a scroll area 100 points tall over a tall filler, scrolled `offset`.
    scroll: bool,
    offset: f32,
    /// A label before the field scrubs it.
    outside: bool,
    /// The button before the field and the outside label.
    button: Id,
    label: Rect,
    popup: bool,
    arithmetic: bool,
    drawn: bool,
    enabled: bool,
    /// What the host makes of a value it is sent (a host that snaps).
    adjust: fn(f64) -> f64,
    /// The host applies a gesture's value only on its commit, not its updates.
    on_commit: bool,
    policy: EditPolicy,
    id: Id,
    rect: Rect,
    event: Option<EditEvent<f64>>,
    escaped: bool,
    /// Every event in order.
    events: Vec<EditEvent<f64>>,
    /// The field's AccessKit node and its children's roles in the last update.
    node: Option<Node>,
    children: Vec<Role>,
    /// The pop-up slider's bounds while it is open.
    slider: Option<Rect>,
    cursor: CursorIcon,
    /// The viewport is full screen.
    fullscreen: bool,
    /// The field steps through these stops.
    stops: bool,
    /// The field wraps the pointer at the screen's edges.
    wrap: bool,
    edit_menu: bool,
    /// Every viewport command sent so far.
    commands: Vec<ViewportCommand>,
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
            scrub_text: true,
            scroll: false,
            offset: 0.0,
            outside: false,
            button: Id::NULL,
            label: Rect::NOTHING,
            popup: false,
            arithmetic: false,
            drawn: true,
            enabled: true,
            adjust: |v| v,
            on_commit: false,
            policy: EditPolicy::default(),
            id: Id::NULL,
            rect: Rect::NOTHING,
            event: None,
            escaped: false,
            events: Vec::new(),
            node: None,
            children: Vec::new(),
            slider: None,
            cursor: CursorIcon::Default,
            fullscreen: false,
            stops: false,
            wrap: false,
            edit_menu: true,
            commands: Vec::new(),
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
        let mut input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), time: Some(self.time), events, ..Default::default() };
        input.viewports.insert(egui::ViewportId::ROOT, egui::ViewportInfo { fullscreen: Some(self.fullscreen), ..Default::default() });
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let output = ctx.run_ui(input, |ui| self.controls(ui));
        let update = output.platform_output.accesskit_update.clone();
        self.cursor = output.platform_output.cursor_icon;
        if let Some(root) = output.viewport_output.get(&egui::ViewportId::ROOT) {
            self.commands.extend(root.commands.iter().cloned());
        }
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
            self.slider = update
                .nodes
                .iter()
                .find(|(_, n)| n.role() == Role::Slider)
                .and_then(|(_, n)| n.bounds())
                .map(|b| Rect::from_min_max(pos2(b.x0 as f32, b.y0 as f32), pos2(b.x1 as f32, b.y1 as f32)));
        }
    }

    fn controls(&mut self, ui: &mut Ui) {
        if !self.scroll {
            self.field_row(ui);
            return;
        }
        let out = egui::ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
            self.field_row(ui);
            ui.allocate_space(vec2(200.0, 1000.0));
        });
        self.offset = out.state.clamped_offset().y;
    }

    fn field_row(&mut self, ui: &mut Ui) {
        self.button = ui.button("Before").id;
        let label = self.outside.then(|| ui.label("Width"));
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
            .enabled(self.enabled)
            .scrub_text(self.scrub_text)
            .policy(self.policy);
        if self.scrub {
            field = field.scrub_label("W");
        }
        if self.stops {
            field = field.steps_with(zoom_stop);
        }
        field = field.scrub_wrap(self.wrap).edit_menu(self.edit_menu);
        if self.popup {
            field = field.popup_slider("Width slider");
        }
        if let Some(label) = &label {
            self.label = label.rect;
            field = field.scrub_by(label);
        }
        if self.arithmetic {
            field = field.or_parse(sum);
        }
        let edit = field.show(ui);
        (self.id, self.rect, self.escaped) = (edit.response.id, edit.response.rect, edit.escaped);
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
            EditEvent::Cancel { reason: CancelReason::External, .. } => {}
            EditEvent::Cancel { .. } => self.value = self.before,
            EditEvent::Commit { changed: false, .. } if self.mixed => {}
            // A mixed host stays mixed until the commit.
            EditEvent::Update { .. } if self.on_commit => {}
            EditEvent::Update { value, .. } => self.value = (self.adjust)(value),
            other => {
                if let Some(&v) = other.value() {
                    self.value = (self.adjust)(v);
                    self.mixed = false;
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
fn typing_commits_on_enter_and_on_focus_loss_with_the_value_read_rounded_and_clamped() {
    let mut rig = Rig::with(5.0, |r| r.range = (0.0, 100.0));
    rig.focus();
    let n = rig.events.len();
    for c in ["-", "1", "2", ".", "3", "4", "5"] {
        rig.type_text(c);
    }
    let id = begin_id(&since(&rig, n));
    rig.key(Key::Enter);
    // -12.345 rounds to two decimals, then clamps to the range.
    assert_eq!(rig.event, Some(commit(id, 0.0, true)));
    assert_eq!(rig.value, 0.0);
    assert!(!rig.has_focus(), "Enter leaves the field");

    rig.focus();
    let n = rig.events.len();
    rig.type_text("2.375");
    let id = begin_id(&since(&rig, n));
    rig.blur();
    // Half away from zero.
    assert_eq!(rig.event, Some(commit(id, 2.38, true)));
    assert_eq!(since(&rig, n).len(), 2);
}

/// A plain number applies as it is typed, one update a keystroke that makes one; `-` doesn't
/// yet, and arithmetic waits for the commit, the host keeping the last plain value until then.
#[test]
fn a_plain_number_applies_as_it_is_typed_and_arithmetic_only_on_the_commit() {
    let mut rig = Rig::with(5.0, |r| r.arithmetic = true);
    rig.focus();
    let n = rig.events.len();
    let mut applied = Vec::new();
    for c in ["-", "1", "2", "+", "3"] {
        rig.type_text(c);
        rig.frame(vec![]);
        applied.push(rig.value);
    }
    let id = begin_id(&since(&rig, n));
    assert_eq!(applied, [5.0, -1.0, -12.0, -12.0, -12.0], "`-` leaves 5, `-12+` and `-12+3` wait");
    rig.key(Key::Enter);
    assert_eq!(rig.event, Some(commit(id, -9.0, true)));
    assert_eq!(rig.value, -9.0);

    // Esc after numbers applied as typed brings the value from before the run back.
    rig.focus();
    let n = rig.events.len();
    rig.type_text("4");
    rig.frame(vec![]);
    assert_eq!(rig.value, 4.0);
    rig.key(Key::Escape);
    assert!(matches!(since(&rig, n).last(), Some(EditEvent::Cancel { reason: CancelReason::Escape, .. })), "{:?}", since(&rig, n));
    assert_eq!(rig.value, -9.0);
    assert!(!rig.has_focus() && rig.escaped, "Esc leaves the field and goes on");

    // Text that reads as no number commits the value from before the run.
    rig.focus();
    let n = rig.events.len();
    rig.type_text("7");
    rig.frame(vec![]);
    rig.type_text("x");
    assert_eq!(rig.value, 7.0, "the last number applied stays while the text is no number");
    rig.key(Key::Enter);
    let id = begin_id(&since(&rig, n));
    assert_eq!(rig.event, Some(commit(id, -9.0, false)));
    assert_eq!(rig.value, -9.0);
}

/// Types each of `keys` in a frame of its own, with a frame after it for the update.
fn type_each(rig: &mut Rig, keys: &[&str]) {
    for key in keys {
        rig.type_text(key);
        rig.frame(vec![]);
    }
}

#[test]
fn a_typing_run_goes_on_while_the_host_stays_mixed_snaps_or_applies_only_the_commit() {
    let update = |id, value| EditEvent::Update { id, value, modifiers: Modifiers::NONE };
    // A host that keeps the field mixed until the commit.
    let mut rig = Rig::with(20.0, |r| r.mixed = true);
    rig.focus();
    let n = rig.events.len();
    type_each(&mut rig, &["1", "2"]);
    assert!(rig.mixed);
    rig.key(Key::Enter);
    let id = begin_id(&since(&rig, n));
    assert_eq!(since(&rig, n)[1..], [update(id, 1.0), update(id, 12.0), commit(id, 12.0, true)]);
    assert_eq!((rig.value, rig.mixed), (12.0, false));
    // Esc puts the blank field back.
    rig.mixed = true;
    rig.focus();
    let n = rig.events.len();
    type_each(&mut rig, &["5"]);
    rig.key(Key::Escape);
    assert!(matches!(since(&rig, n).last(), Some(EditEvent::Cancel { reason: CancelReason::Escape, .. })), "{:?}", since(&rig, n));
    rig.frame(vec![]);
    assert_eq!((rig.value, rig.mixed, rig.shown()), (12.0, true, None), "blank, with its placeholder");

    // A host that snaps each value it applies to a multiple of 4.
    let mut rig = Rig::with(10.0, |r| r.adjust = |v| (v / 4.0).floor() * 4.0);
    rig.focus();
    let n = rig.events.len();
    type_each(&mut rig, &["1", "3"]);
    assert_eq!(rig.shown().as_deref(), Some("13"), "the text stays the user's");
    rig.key(Key::Enter);
    let id = begin_id(&since(&rig, n));
    assert_eq!(since(&rig, n)[1..], [update(id, 1.0), update(id, 13.0), commit(id, 13.0, true)]);
    assert_eq!(rig.value, 12.0);

    // A host that applies only the commit.
    let mut rig = Rig::with(10.0, |r| r.on_commit = true);
    rig.focus();
    let n = rig.events.len();
    type_each(&mut rig, &["4", "2"]);
    rig.key(Key::Enter);
    let id = begin_id(&since(&rig, n));
    assert_eq!(since(&rig, n)[1..], [update(id, 4.0), update(id, 42.0), commit(id, 42.0, true)]);
    assert_eq!(rig.value, 42.0);

    // A change of the host's value that answers no update is someone else's: the run ends. The
    // answer comes in the pass after the update.
    for (snaps, answer) in [(true, 4.0), (false, 10.0)] {
        let mut rig = Rig::with(10.0, |r| match snaps {
            true => r.adjust = |v| (v / 4.0).floor() * 4.0,
            false => r.on_commit = true,
        });
        rig.focus();
        let n = rig.events.len();
        type_each(&mut rig, &["7"]);
        rig.frame(vec![]);
        assert_eq!((rig.value, since(&rig, n).len()), (answer, 2));
        rig.value = 30.0;
        rig.frame(vec![]);
        let events = since(&rig, n);
        assert!(
            matches!(&events[..], [EditEvent::Begin { .. }, EditEvent::Update { .. }, EditEvent::Cancel { reason: CancelReason::External, .. }]),
            "{events:?}"
        );
    }
}

#[test]
fn esc_drops_text_that_is_no_number_when_the_policy_can_t_cancel() {
    let mut rig = Rig::with(7.0, |r| r.policy = EditPolicy::default().cancellable(false));
    rig.focus();
    let n = rig.events.len();
    rig.type_text("abc");
    rig.key(Key::Escape);
    let id = begin_id(&since(&rig, n));
    assert_eq!(rig.event, Some(commit(id, 7.0, false)), "the focus leaving commits the run");
    assert!(!rig.has_focus() && rig.escaped);
    rig.frame(vec![]);
    assert_eq!((rig.shown().as_deref(), rig.invalid()), (Some("7"), false), "the text goes with the run");
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
    assert!(!rig.has_focus() && rig.escaped, "Esc restores, leaves and goes on");
    rig.frame(vec![]);
    assert_eq!((rig.shown().as_deref(), rig.invalid(), rig.event.clone()), (Some("7"), false, None));

    // Text fixed and committed clears the mark.
    rig.focus();
    rig.type_text("x");
    rig.key(Key::Enter);
    rig.frame(vec![]);
    assert!(rig.invalid());
    rig.ctx.memory_mut(|m| m.request_focus(rig.id));
    rig.frame(vec![]);
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
fn the_arrows_step_nothing_while_an_input_method_composes() {
    let mut rig = Rig::new(5.0);
    rig.focus();
    let n = rig.events.len();
    rig.frame(vec![Event::Ime(egui::ImeEvent::Preedit { text: "１".to_owned(), active_range_chars: Some(0..1) })]);
    rig.key(Key::ArrowUp);
    let stepped = since(&rig, n).iter().any(|e| matches!(e, EditEvent::Set { .. } | EditEvent::Update { .. }));
    assert!(!stepped && rig.value == 5.0, "the arrows are the input method's: {:?}", since(&rig, n));
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
    rig.focus();
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
    rig.focus();
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

#[test]
fn a_label_before_the_field_scrubs_it_and_is_no_tab_stop() {
    let mut rig = Rig::with(10.0, |r| r.outside = true);
    let n = rig.events.len();
    rig.drag(rig.label.center(), &[10.0, 14.0], Modifiers::NONE, true);
    let events = since(&rig, n);
    let id = begin_id(&events);
    assert_eq!(events.last(), Some(&commit(id, 24.0, true)));
    rig.ctx.memory_mut(|m| m.request_focus(rig.button));
    rig.frame(vec![]);
    rig.key(Key::Tab);
    assert!(rig.has_focus(), "Tab goes from the button to the field");
}

fn wheel_notch() -> Event {
    wheel_lines(1.0, Modifiers::NONE)
}

/// `lines` wheel lines, up positive.
fn wheel_lines(lines: f32, modifiers: Modifiers) -> Event {
    Event::MouseWheel {
        unit: MouseWheelUnit::Line,
        delta: vec2(0.0, lines),
        modifiers,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
    }
}

/// A drag past egui's threshold anywhere on a field without the focus scrubs it, a click without
/// a drag gives it the focus with its text selected, and a drag on the focused field's text
/// selects it. `scrub_text(false)` leaves the text to the pointer.
#[test]
fn a_drag_on_a_field_without_the_focus_scrubs_it_and_a_click_edits_it() {
    let mut rig = Rig::new(10.0);
    let at = rig.rect.center();
    rig.frame(vec![Event::PointerMoved(at)]);
    assert_eq!(rig.cursor, CursorIcon::ResizeHorizontal);
    let n = rig.events.len();
    rig.drag(at, &[10.0, 15.0], Modifiers::NONE, true);
    let events = since(&rig, n);
    let id = begin_id(&events);
    assert_eq!(events.last(), Some(&commit(id, 25.0, true)));
    assert!(!rig.has_focus(), "a scrub doesn't focus the field");

    let n = rig.events.len();
    rig.drag(at, &[2.0], Modifiers::NONE, true);
    assert_eq!(since(&rig, n), [], "a press that moves less than the threshold is a click");
    assert!(rig.has_focus(), "the click gives the field the focus");
    rig.frame(vec![]);
    rig.type_text("3");
    rig.key(Key::Enter);
    assert_eq!(rig.value, 3.0, "with the text selected, so typing replaces it");

    rig.focus();
    let n = rig.events.len();
    rig.drag(rig.rect.center(), &[-20.0, -30.0], Modifiers::NONE, true);
    assert_eq!((since(&rig, n), rig.value, rig.has_focus()), (vec![], 3.0, true), "a drag in the focused field selects");

    let mut rig = Rig::with(10.0, |r| r.scrub_text = false);
    let n = rig.events.len();
    rig.drag(rig.rect.center(), &[10.0, 15.0], Modifiers::NONE, true);
    assert_eq!((since(&rig, n), rig.value, rig.has_focus()), (vec![], 10.0, true), "scrub_text(false): the press places the caret");
}

/// PhotoCraft's wheel: a gesture that starts over a field without the focus steps it, one that
/// starts elsewhere scrolls the panel on over it; the focused field takes the wheel wherever the
/// pointer is; Alt and Ctrl/Cmd stay the view's.
#[test]
fn the_wheel_steps_the_field_a_gesture_starts_over_and_the_focused_field_from_anywhere() {
    let mut rig = Rig::with(100.0, |r| r.scroll = true);
    let over = rig.rect.center();
    rig.frame(vec![Event::PointerMoved(over)]);
    let n = rig.events.len();
    rig.frame(vec![wheel_lines(1.0, Modifiers::NONE)]);
    rig.frame(vec![wheel_lines(1.0, Modifiers::SHIFT)]);
    for _ in 0..4 {
        rig.frame(vec![]);
    }
    assert_eq!(since(&rig, n), [set(101.0), set(111.0)], "a notch a step, Shift ten, no focus needed");
    assert_eq!(rig.offset, 0.0, "the panel under the field doesn't scroll");
    assert!(!rig.has_focus());

    rig.time += 1.0;
    rig.frame(vec![wheel_lines(1.0, Modifiers::ALT), wheel_lines(1.0, Modifiers::COMMAND)]);
    assert_eq!(rig.value, 111.0, "Alt and Cmd/Ctrl + wheel are the view's");

    // A gesture that starts below the field scrolls the panel, also once the field is under the
    // pointer.
    rig.time += 1.0;
    rig.frame(vec![Event::PointerMoved(over + vec2(0.0, 50.0))]);
    rig.frame(vec![wheel_lines(-1.0, Modifiers::NONE)]);
    let over = rig.rect.center();
    rig.frame(vec![Event::PointerMoved(over), wheel_lines(-1.0, Modifiers::NONE)]);
    for _ in 0..4 {
        rig.frame(vec![]);
    }
    assert_eq!(rig.value, 111.0, "the field passing under the pointer isn't stepped");
    assert!(rig.offset > 0.0, "the panel scrolled");

    // The focused field steps with the pointer away from it, and the panel stays.
    let mut rig = Rig::with(100.0, |r| r.scroll = true);
    rig.focus();
    let away = rig.rect.center() + vec2(0.0, 50.0);
    rig.frame(vec![Event::PointerMoved(away)]);
    rig.frame(vec![wheel_lines(-2.0, Modifiers::NONE)]);
    for _ in 0..4 {
        rig.frame(vec![]);
    }
    assert_eq!((rig.value, rig.offset), (98.0, 0.0));
    assert!(rig.has_focus());
}

#[test]
fn text_that_cannot_become_a_number_is_marked_as_it_is_typed() {
    let mut rig = Rig::with(7.0, |r| r.numbers = LocaleNumbers::from_tag("de"));
    rig.focus();
    rig.type_text("-");
    assert!(!rig.invalid(), "`-` may still become a number");
    rig.type_text("1,");
    assert!(!rig.invalid(), "nor may `-1,`");
    rig.type_text("x");
    assert!(rig.invalid(), "`-1,x` can't");
    rig.key(Key::Backspace);
    assert!(!rig.invalid());
    rig.type_text("5");
    rig.key(Key::Enter);
    assert_eq!((rig.value, rig.invalid()), (-1.5, false));
}

#[test]
fn a_step_on_text_that_is_no_number_keeps_the_text_and_the_value() {
    let mut rig = Rig::new(7.0);
    rig.focus();
    let n = rig.events.len();
    rig.type_text("12x");
    let id = begin_id(&since(&rig, n));
    rig.key(Key::ArrowUp);
    assert_eq!(rig.event, Some(commit(id, 7.0, false)), "the run ends as on Enter");
    rig.frame(vec![]);
    assert_eq!((rig.value, rig.shown().as_deref(), rig.invalid(), rig.has_focus()), (7.0, Some("12x"), true, true));

    // Kept, rejected text takes no step either, from the arrows or the wheel.
    let n = rig.events.len();
    rig.key(Key::ArrowDown);
    rig.frame(vec![Event::PointerMoved(rig.rect.center()), wheel_notch()]);
    rig.frame(vec![]);
    assert_eq!(since(&rig, n), []);
    assert_eq!((rig.value, rig.shown().as_deref(), rig.invalid()), (7.0, Some("12x"), true));
}

#[test]
fn an_undo_mid_scrub_leaves_the_reverted_value_until_the_release() {
    let mut rig = Rig::with(10.0, |r| {
        r.scrub = true;
        r.hosted = true;
    });
    let at = rig.handle();
    rig.drag(at, &[10.0, 20.0], Modifiers::NONE, false);
    assert_eq!(rig.value, 30.0);
    // Cmd+Z outside the field during a gesture, as the event-log story does it.
    let id = rig.host.active().expect("a scrub under way");
    rig.host.cancel(&mut rig.doc, id);
    rig.value = rig.doc.value;
    edit::cancel_all(&rig.ctx, CancelReason::UndoDuringGesture);
    let n = rig.events.len();
    for dx in [30.0, 40.0, 50.0] {
        rig.frame(vec![Event::PointerMoved(at + vec2(dx, 0.0))]);
        assert_eq!(rig.value, 10.0);
    }
    rig.frame(vec![pointer(at + vec2(50.0, 0.0), false)]);
    let events = since(&rig, n);
    assert!(events.iter().all(|e| matches!(e, EditEvent::Cancel { .. })), "{events:?}");
    assert_eq!((rig.value, rig.doc.undo.len()), (10.0, 0));
}

/// A scrub whose gesture the host ends for another control's change goes on: the host reverts
/// the scrub's steps, the next movement begins a new gesture from where the scrub was, and the
/// release commits it.
#[test]
fn a_scrub_the_host_ended_for_another_controls_change_goes_on_from_where_it_was() {
    let mut rig = Rig::with(10.0, |r| {
        r.scrub = true;
        r.hosted = true;
    });
    let at = rig.handle();
    rig.drag(at, &[10.0, 20.0], Modifiers::NONE, false);
    assert_eq!(rig.value, 30.0);
    let first = rig.host.active().expect("a scrub under way");
    // An options bar's change: the host reverts the scrub and keeps the change as a step.
    let ctx = rig.ctx.clone();
    rig.host.handle(&ctx, &mut rig.doc, EditEvent::set(1, Modifiers::NONE), |doc: &mut Doc, _| doc.undo.push(doc.value));
    rig.value = rig.doc.value;
    assert_eq!((rig.value, rig.host.active()), (10.0, None));
    let n = rig.events.len();
    rig.frame(vec![Event::PointerMoved(at + vec2(30.0, 0.0))]);
    rig.frame(vec![]);
    rig.frame(vec![pointer(at + vec2(30.0, 0.0), false)]);
    let events = since(&rig, n);
    let id = begin_id(&events);
    assert!(id > first);
    assert_eq!(&events[1..], &[EditEvent::Update { id, value: 40.0, modifiers: Modifiers::NONE }, commit(id, 40.0, true)][..]);
    assert_eq!((rig.value, rig.doc.undo.len()), (40.0, 2));
}

#[test]
fn a_host_that_snaps_a_scrub_cancels_it_once() {
    let mut rig = Rig::with(10.0, |r| {
        r.scrub = true;
        r.adjust = |v| (v / 4.0).floor() * 4.0;
    });
    let n = rig.events.len();
    rig.drag(rig.handle(), &[10.0, 20.0, 30.0, 40.0, 50.0], Modifiers::NONE, true);
    let events = since(&rig, n);
    let begins = events.iter().filter(|e| matches!(e, EditEvent::Begin { .. })).count();
    assert_eq!(begins, 1, "{events:?}");
    assert!(matches!(events.last(), Some(EditEvent::Cancel { reason: CancelReason::External, .. })), "{events:?}");
    assert_eq!(rig.value, 28.0);
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "a pop-up slider needs a finite range")]
fn a_popup_slider_without_a_finite_range_is_a_debug_assertion() {
    let _ = Rig::with(5.0, |r| r.popup = true);
}

#[test]
fn a_drag_inside_the_open_popup_slider_is_one_gesture_and_one_undo_step() {
    let mut rig = Rig::with(50.0, |r| {
        r.range = (0.0, 100.0);
        r.popup = true;
        r.hosted = true;
    });
    let arrow = pos2(rig.rect.right() - 8.0, rig.rect.center().y);
    rig.frame(vec![Event::PointerMoved(arrow), pointer(arrow, true)]);
    rig.frame(vec![pointer(arrow, false)]);
    rig.frame(vec![]);
    let slider = rig.slider.expect("the pop-up slider is open");
    let n = rig.events.len();
    rig.drag(slider.center(), &[10.0, 20.0, 30.0], Modifiers::NONE, true);
    let events = since(&rig, n);
    let id = begin_id(&events);
    assert_eq!(events.iter().filter(|e| matches!(e, EditEvent::Begin { .. })).count(), 1, "{events:?}");
    assert!(matches!(events.last(), Some(EditEvent::Commit { id: c, changed: true, .. }) if *c == id), "{events:?}");
    assert!(rig.value > 50.0, "{}", rig.value);
    assert_eq!(rig.doc.undo.len(), 1);
}

#[test]
fn a_disabled_field_shows_no_scrub_cursor_and_takes_no_scrub() {
    let mut rig = Rig::with(10.0, |r| r.scrub = true);
    rig.frame(vec![Event::PointerMoved(rig.handle())]);
    assert_eq!(rig.cursor, CursorIcon::ResizeHorizontal);

    let mut rig = Rig::with(10.0, |r| {
        r.scrub = true;
        r.enabled = false;
    });
    rig.frame(vec![Event::PointerMoved(rig.handle())]);
    assert_eq!(rig.cursor, CursorIcon::Default);
    let n = rig.events.len();
    rig.drag(rig.handle(), &[10.0, 20.0], Modifiers::NONE, true);
    assert_eq!((since(&rig, n), rig.value), (vec![], 10.0));
}

/// The texts the right-to-left story paints in `dir`, German digits: each one's text and rect.
fn right_to_left_story(dir: Direction) -> Vec<(String, Rect)> {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    direction::apply(&ctx, dir);
    let story = number_field::stories().into_iter().find(|s| s.id == "controls/number-field/right-to-left").expect("the story");
    let mut args = Args::new().with_choice("size", SIZE_NAMES, 1).with_choice("locale", &["en", "de", "ar-EG"], 1);
    let mut texts = Vec::new();
    for _ in 0..2 {
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), ..Default::default() };
        let output = ctx.run_ui(input, |ui| (story.show)(ui, &mut args));
        texts.clear();
        let mut shapes: Vec<Shape> = output.shapes.iter().map(|ClippedShape { shape, .. }| shape.clone()).collect();
        while let Some(shape) = shapes.pop() {
            match shape {
                Shape::Vec(inner) => shapes.extend(inner),
                Shape::Text(t) => texts.push((t.galley.text().to_owned(), t.galley.rect.translate(t.pos.to_vec2()))),
                _ => {}
            }
        }
        output.drop_without_applying_deltas();
    }
    texts
}

#[test]
fn right_to_left_keeps_the_digits_the_unit_and_the_scrub_label_in_place() {
    let parts = |dir| {
        let texts = right_to_left_story(dir);
        let find = |text: &str| texts.iter().find(|(t, _)| t == text).map(|(_, r)| *r).unwrap_or_else(|| panic!("{text} in {texts:?}"));
        (find("W"), find("12,5"), find(" cm"))
    };
    let (w, digits, unit) = parts(Direction::Rtl);
    assert!(w.right() < digits.left(), "the scrub label at the left: {w:?} {digits:?}");
    assert!((unit.left() - digits.right()).abs() < 0.5, "the unit after the digits: {digits:?} {unit:?}");
    let (ltr_w, ltr_digits, ltr_unit) = parts(Direction::Ltr);
    let from_w = |w: Rect, r: Rect| r.min - w.min;
    assert_eq!((from_w(w, digits), from_w(w, unit)), (from_w(ltr_w, ltr_digits), from_w(ltr_w, ltr_unit)));
}

#[test]
fn the_states_story_s_mixed_field_is_blank_again_after_esc_and_scrubs_while_mixed() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT as _, Queryable as _};
    let story = number_field::stories().into_iter().find(|s| s.id == "controls/number-field/states").expect("the story");
    let show = story.show;
    let mut harness = Harness::builder()
        .with_size([600.0, 600.0])
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(move |ui, args: &mut Args| show(ui, args), (story.args)());
    harness.run();
    let shown = |harness: &Harness<'_, Args>| harness.get_by_label("Size").accesskit_node().value().unwrap_or_default().to_owned();
    assert_eq!(shown(&harness), "", "mixed: blank");
    harness.get_by_label("Size").focus();
    harness.run();
    harness.event(Event::Text("5".to_owned()));
    harness.run();
    assert_eq!(shown(&harness), "5 pt");
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(shown(&harness), "", "Esc puts the blank field back");

    let from = harness.get_by_label("Size").rect().center();
    harness.event(Event::PointerMoved(from));
    harness.event(pointer(from, true));
    harness.run();
    for dx in [10.0, 20.0, 30.0] {
        harness.event(Event::PointerMoved(from + vec2(dx, 0.0)));
        harness.run();
    }
    harness.event(pointer(from + vec2(30.0, 0.0), false));
    harness.run();
    assert!(!shown(&harness).is_empty(), "the scrub went on to its release and committed a value");
}

/// Zoom levels 12.5, 25, 50, 100, 200.
fn zoom_stop(value: f64, direction: i32) -> f64 {
    let stops = [12.5, 25.0, 50.0, 100.0, 200.0];
    if direction > 0 {
        stops.into_iter().find(|s| *s > value).unwrap_or(value)
    } else {
        stops.into_iter().rev().find(|s| *s < value).unwrap_or(value)
    }
}

#[test]
fn f32_and_i32_values_and_ranges_are_taken_as_they_are() {
    let numbers = LocaleNumbers::from_tag("en");
    let ctx = Context::default();
    let output = ctx.run_ui(RawInput::default(), |ui| {
        let a = NumberField::new("a", "A", 1.5_f32, &numbers).range(0.0_f32..=2.0).show(ui);
        let b = NumberField::new("b", "B", 7_i32, &numbers).range(0_i32..=10).show(ui);
        assert!(a.event.is_none() && b.event.is_none());
        assert_eq!(crate::controls::Slider::new("c", "C", 0.25_f32, 0.0_f32..=1.0).step(0.05_f32).default_value(0.5_f32).show(ui).event, None);
    });
    output.drop_without_applying_deltas();
}

#[test]
fn steps_with_zoom_levels_step_to_the_next_and_previous_level_and_clamp() {
    let mut rig = Rig::with(100.0, |r| {
        r.stops = true;
        r.range = (0.0, 150.0);
        r.decimals = (0, 1);
    });
    rig.focus();
    let n = rig.events.len();
    rig.key(Key::ArrowDown);
    assert_eq!(rig.value, 50.0);
    rig.key(Key::ArrowDown);
    rig.key(Key::ArrowDown);
    assert_eq!(rig.value, 12.5, "snapped by the decimals, not by the step");
    rig.key(Key::ArrowUp);
    rig.key(Key::ArrowUp);
    rig.key(Key::ArrowUp);
    rig.key(Key::ArrowUp);
    assert_eq!(rig.value, 150.0, "the stop 200 is clamped to the range");
    rig.key(Key::ArrowUp);
    assert_eq!(rig.value, 150.0);
    let events = since(&rig, n);
    assert!(events.iter().all(|e| matches!(e, EditEvent::Set { .. })), "each step is one Set: {events:?}");
    assert_eq!(events[0], set(50.0));
    rig.chord(Modifiers::SHIFT, Key::ArrowDown);
    assert_eq!(rig.value, 100.0, "modifiers do not scale a stop");
}

#[test]
fn the_wrap_keeps_a_scrub_continuous_across_the_edge() {
    let edges = Some((0.0, 800.0));
    let start = travel(790.0, 799.0, None, edges);
    assert_eq!(start, Travel { delta: 9.0, last_x: 799.0, warp: Some(4.0), send: Some(4.0) });
    // The pointer has not moved yet: nothing counts, nothing is sent again.
    let wait = travel(799.0, 799.0, start.warp, edges);
    assert_eq!((wait.delta, wait.warp, wait.send), (0.0, Some(4.0), None));
    // It shows up at the other edge: the jump adds nothing.
    let arrived = travel(799.0, 4.0, wait.warp, edges);
    assert_eq!(arrived, Travel { delta: 0.0, last_x: 4.0, warp: None, send: None });
    assert_eq!(travel(4.0, 10.0, None, edges).delta, 6.0);
    // Leftwards likewise.
    assert_eq!(travel(10.0, 1.0, None, edges).send, Some(796.0));
    // A pointer that moves elsewhere instead (no warp on this platform) is an ordinary move.
    let moved = travel(799.0, 780.0, Some(4.0), edges);
    assert_eq!((moved.delta, moved.warp), (-19.0, None));
    // Away from the edges, or without them, nothing is sent.
    assert_eq!(travel(100.0, 200.0, None, edges).send, None);
    assert_eq!(travel(790.0, 799.0, None, None).send, None);
}

fn warps(rig: &Rig) -> Vec<Pos2> {
    rig.commands.iter().filter_map(|c| if let ViewportCommand::CursorPosition(p) = c { Some(*p) } else { None }).collect()
}

#[test]
fn scrub_wrap_moves_the_pointer_only_in_a_full_screen_viewport() {
    for (fullscreen, wrap) in [(true, true), (false, true), (true, false)] {
        let mut rig = Rig::with(10.0, |r| {
            r.scrub = true;
            r.fullscreen = fullscreen;
            r.wrap = wrap;
        });
        let from = rig.handle();
        rig.frame(vec![Event::PointerMoved(from), pointer(from, true)]);
        rig.frame(vec![Event::PointerMoved(from + vec2(10.0, 0.0))]);
        rig.frame(vec![Event::PointerMoved(pos2(799.0, from.y))]);
        let sent = warps(&rig);
        if fullscreen && wrap {
            assert_eq!(sent, vec![pos2(4.0, from.y)]);
        } else {
            assert!(sent.is_empty(), "fullscreen {fullscreen}, wrap {wrap}: {sent:?}");
        }
        rig.frame(vec![pointer(pos2(799.0, from.y), false)]);
    }
}

#[test]
fn a_secondary_click_opens_the_edit_menu_on_the_text_unless_the_field_turns_it_off() {
    for on in [true, false] {
        let mut rig = Rig::with(12.0, |r| r.edit_menu = on);
        let at = rig.rect.center();
        let secondary = |pressed| Event::PointerButton { pos: at, button: egui::PointerButton::Secondary, pressed, modifiers: Modifiers::NONE };
        rig.frame(vec![Event::PointerMoved(at)]);
        rig.frame(vec![secondary(true)]);
        rig.frame(vec![secondary(false)]);
        rig.frame(vec![]);
        assert_eq!(crate::behavior::edit_menu::is_open(&rig.ctx, rig.id), on);
        assert_eq!(rig.has_focus(), on, "the press focuses the field when the menu opens on its text");
        assert_eq!(rig.events, vec![], "the press changes no value");
    }
}

#[test]
fn the_menu_key_then_down_down_enter_runs_a_command_and_neither_steps_nor_commits() {
    let mut rig = Rig::with(12.0, |_| {});
    rig.focus();
    rig.key(Key::ArrowRight);
    rig.key(Key::Menu);
    rig.frame(vec![]);
    assert!(crate::behavior::edit_menu::is_open(&rig.ctx, rig.id));
    // Undo is highlighted; Cut, Copy and Delete are disabled, so ↓ ↓ reaches Select All.
    rig.key(Key::ArrowDown);
    rig.key(Key::ArrowDown);
    rig.key(Key::Enter);
    rig.frame(vec![]);
    assert!(!crate::behavior::edit_menu::is_open(&rig.ctx, rig.id) && rig.has_focus());
    assert_eq!(rig.value, 12.0, "the arrows moved in the menu and did not step the value");
    assert_eq!(rig.events, vec![], "Enter chose the row and committed nothing");
    let range = egui::text_edit::TextEditState::load(&rig.ctx, rig.id).and_then(|s| s.cursor.char_range()).expect("a cursor");
    assert!(!range.is_empty(), "Select All ran");
}
