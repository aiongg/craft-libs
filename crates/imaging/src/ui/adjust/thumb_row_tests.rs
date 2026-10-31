//! Thumb-row behaviour in a headless egui context, with a levels editor's two rows as the host:
//! black, gamma and white on the input row (the gamma between the others, at
//! `black + (white − black) · 0.5^gamma`), and two free thumbs on the output row.

use std::collections::HashMap;

use egui::accesskit::{Action, ActionRequest, Node, Role, TreeId};
use egui::{
    Color32, Context, Event, Id, Key, Modifiers, MouseWheelSource, MouseWheelUnit, PointerButton, Pos2, RawInput, Rect, TouchPhase, Ui, pos2, vec2,
};

use super::levels_editor::{LevelsEditor, gamma_at, gamma_from};
use craft_ui::controls::NumberField;
use craft_ui::controls::thumb_row::{Follow, Order, RowOp, Thumb, ThumbRow, ThumbRowStyle};
use craft_ui::edit::ops::{Bound, ScalarOp};
use craft_ui::edit::{CancelReason, EditEvent, EditPolicy, GestureHost, GestureId, UndoMarks};
use craft_ui::localized::LocaleNumbers;
use craft_ui::primitives::Interactive;
use craft_ui::theme::{Base, Tokens, TokensOf, install};

const GAP: f64 = 2.0;

/// A row as the last frame drew it.
#[derive(Clone, Debug, Default)]
struct Drawn {
    /// The thumb centre's x at levels 0 and 255.
    travel: (f32, f32),
    y: f32,
    ids: Vec<Id>,
    /// Thumb centres.
    xs: Vec<f32>,
    shown: Vec<f64>,
    dragged: Option<usize>,
}

/// The field above the rows: its value and rectangle as drawn, and this frame's event.
struct Field {
    value: f64,
    id: Id,
    rect: Rect,
    event: Option<EditEvent<f64>>,
}

/// The host: it applies every value it is sent, and puts the start value back on a cancel when
/// `revert` is on.
struct Rig {
    ctx: Context,
    time: f64,
    modifiers: Modifiers,
    input: [f64; 3],
    output: [f64; 2],
    revert: bool,
    enabled: bool,
    waits: bool,
    /// A text field above the rows, for another control's typing run.
    field: Option<Field>,
    before: ([f64; 3], [f64; 2]),
    input_event: Option<EditEvent<[f64; 3]>>,
    output_event: Option<EditEvent<[f64; 2]>>,
    rows: [Drawn; 2],
    nodes: HashMap<Id, Node>,
    /// The values live in a document behind a [`GestureHost`] instead.
    hosted: Option<(GestureHost<Levels>, Levels)>,
}

/// The input row's thumbs: black and white `GAP` apart, the gamma between them.
fn input_thumbs() -> [Thumb; 3] {
    let ordered = |below, above| Order::Between { below, above, gap: GAP };
    let gamma = Follow { at: gamma_at, value_at: gamma_from };
    [
        Thumb::new("Input black", Color32::BLACK).order(ordered(None, Some(2))).default_value(0.0),
        Thumb::new("Gamma", Color32::from_gray(128)).follows(&(0.01..=9.99), 0.01, gamma).default_value(1.0).unset_at(1.0).falling(),
        Thumb::new("Input white", Color32::WHITE).order(ordered(Some(0), None)).default_value(255.0),
    ]
}

/// The hosted rig's document: the input and output values, with an undo history of snapshots.
struct Levels {
    now: ([f64; 3], [f64; 2]),
    undo: Vec<([f64; 3], [f64; 2])>,
}

impl Levels {
    fn apply(&mut self, edit: impl FnOnce(&mut ([f64; 3], [f64; 2]))) {
        self.undo.push(self.now);
        edit(&mut self.now);
    }
}

impl UndoMarks for Levels {
    type Mark = usize;

    fn undo_mark(&self) -> usize {
        self.undo.len()
    }

    fn revert_to(&mut self, mark: usize) {
        if let Some(&before) = self.undo.get(mark) {
            self.now = before;
            self.undo.truncate(mark);
        }
    }

    fn squash_since(&mut self, mark: usize) {
        self.undo.truncate(mark + 1);
    }
}

impl Rig {
    fn new(input: [f64; 3], output: [f64; 2]) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut rig = Self {
            ctx,
            time: 0.0,
            modifiers: Modifiers::NONE,
            input,
            output,
            revert: true,
            enabled: true,
            waits: false,
            field: None,
            before: (input, output),
            input_event: None,
            output_event: None,
            rows: Default::default(),
            nodes: HashMap::new(),
            hosted: None,
        };
        rig.frame(vec![]);
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let events = std::iter::once(Event::ModifiersChanged(self.modifiers)).chain(events).collect();
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let output = ctx.run_ui(input, |ui| self.rows(ui));
        if let Some(update) = output.platform_output.accesskit_update.clone() {
            for id in self.rows.iter().flat_map(|r| r.ids.clone()) {
                if let Some((_, node)) = update.nodes.iter().find(|(n, _)| *n == id.accesskit_id()) {
                    self.nodes.insert(id, node.clone());
                }
            }
        }
        output.drop_without_applying_deltas();
    }

    fn rows(&mut self, ui: &mut Ui) {
        let style = ThumbRowStyle::from(&LevelsEditor::style(&Tokens::of(ui)));
        // A level a point: the travel is 255 points.
        let width = 255.0 + 2.0 * style.inset();
        let range = 0.0..=255.0;
        if let Some(field) = &mut self.field {
            let numbers = LocaleNumbers::from_tag("en");
            let out = NumberField::new("rig-field", "Field", field.value, &numbers).range(0.0..=255.0).show(ui);
            field.id = out.response.id;
            field.rect = out.response.rect;
            field.event.clone_from(&out.event);
            if let Some(value) = out.event.and_then(|e| e.value().copied()) {
                field.value = value;
            }
        }
        let row = ThumbRow::new("rig-input", self.input, &range, Some(1.0), input_thumbs())
            .width(width)
            .policy(EditPolicy::default())
            .enabled(self.enabled)
            .part(self.waits);
        let out = row.show(ui, &style);
        self.rows[0] = drawn(ui, "rig-input", &style, out.edit.response.rect, out.shown.to_vec(), out.dragged);
        self.input_event.clone_from(&out.edit.event);
        if let Some((host, doc)) = &mut self.hosted {
            if let Some(event) = out.edit.event {
                host.handle(ui.ctx(), doc, event, |doc: &mut Levels, v| doc.apply(|now| now.0 = v));
            }
            host.end_frame(ui.ctx(), doc);
            self.input = doc.now.0;
        } else if let Some(event) = out.edit.event {
            self.apply(event, |rig| &mut rig.input, |rig| rig.before.0);
        }

        let thumbs = [Thumb::new("Output black", Color32::BLACK).default_value(0.0), Thumb::new("Output white", Color32::WHITE).default_value(255.0)];
        let row = ThumbRow::new("rig-output", self.output, &range, Some(1.0), thumbs)
            .width(width)
            .policy(EditPolicy::default())
            .enabled(self.enabled)
            .part(self.waits);
        let out = row.show(ui, &style);
        self.rows[1] = drawn(ui, "rig-output", &style, out.edit.response.rect, out.shown.to_vec(), out.dragged);
        self.output_event.clone_from(&out.edit.event);
        if let Some(event) = out.edit.event {
            self.apply(event, |rig| &mut rig.output, |rig| rig.before.1);
        }
    }

    fn apply<T: Clone>(&mut self, event: EditEvent<T>, value: impl Fn(&mut Self) -> &mut T, before: impl Fn(&Self) -> T) {
        match event {
            EditEvent::Begin { .. } => self.before = (self.input, self.output),
            EditEvent::Cancel { .. } if self.revert => *value(self) = before(self),
            EditEvent::Cancel { .. } => {}
            other => {
                if let Some(v) = other.value() {
                    *value(self) = v.clone();
                }
            }
        }
    }

    /// The point on row `row` at `level`, moved `dx` points.
    fn at(&self, row: usize, level: f64, dx: f32) -> Pos2 {
        let (x0, x1) = self.rows[row].travel;
        pos2(x0 + (x1 - x0) * (level / 255.0) as f32 + dx, self.rows[row].y)
    }

    /// The point on thumb `i` of row `row`, moved `dx` points.
    fn on(&self, row: usize, i: usize, dx: f32) -> Pos2 {
        pos2(self.rows[row].xs[i] + dx, self.rows[row].y)
    }

    fn id(&self, row: usize, i: usize) -> Id {
        self.rows[row].ids[i]
    }

    fn field_event(&self) -> Option<EditEvent<f64>> {
        self.field.as_ref().and_then(|f| f.event.clone())
    }

    fn field_focused(&self) -> bool {
        self.field.as_ref().is_some_and(|f| self.ctx.memory(|m| m.has_focus(f.id)))
    }

    fn has_focus(&self, row: usize, i: usize) -> bool {
        self.ctx.memory(|m| m.has_focus(self.id(row, i)))
    }

    fn focus(&mut self, row: usize, i: usize) {
        let id = self.id(row, i);
        self.ctx.memory_mut(|m| m.request_focus(id));
        self.frame(vec![]);
    }

    fn press(&mut self, pos: Pos2) {
        self.frame(vec![Event::PointerMoved(pos), button(pos, true)]);
    }

    fn drag(&mut self, pos: Pos2) {
        self.frame(vec![Event::PointerMoved(pos)]);
    }

    fn release(&mut self, pos: Pos2) {
        self.frame(vec![button(pos, false)]);
    }

    fn tap(&mut self, pos: Pos2) {
        self.frame(vec![Event::PointerMoved(pos), button(pos, true), button(pos, false)]);
    }

    fn key(&mut self, key: Key) {
        let modifiers = self.modifiers;
        self.frame(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]);
    }
}

/// The row `salt` as drawn: the thumb centres follow from the shown values, the gamma thumb
/// between the black and the white one.
fn drawn(ui: &Ui, salt: &str, style: &ThumbRowStyle, rect: Rect, shown: Vec<f64>, dragged: Option<usize>) -> Drawn {
    let travel = (rect.left() + style.inset(), rect.right() - style.inset());
    let id = ui.make_persistent_id(egui::IdSalt::new(salt));
    let level = |i: usize| if shown.len() == 3 && i == 1 { gamma_at(&shown, shown[1]) } else { shown[i] };
    Drawn {
        travel,
        y: rect.center().y,
        ids: (0..shown.len()).map(|i| Interactive::id_in(ui, id.with(("thumb", i)))).collect(),
        xs: (0..shown.len()).map(|i| travel.0 + (travel.1 - travel.0) * (level(i) / 255.0) as f32).collect(),
        shown,
        dragged,
    }
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn begin<T: std::fmt::Debug>(event: Option<EditEvent<T>>) -> GestureId {
    match event {
        Some(EditEvent::Begin { id, .. }) => id,
        other => panic!("expected Begin, got {other:?}"),
    }
}

fn update<T>(id: GestureId, value: T) -> Option<EditEvent<T>> {
    Some(EditEvent::update(id, value, Modifiers::NONE))
}

fn commit<T>(id: GestureId, value: T, changed: bool) -> Option<EditEvent<T>> {
    Some(EditEvent::commit(id, value, changed, Modifiers::NONE))
}

fn set<T>(value: T, modifiers: Modifiers) -> Option<EditEvent<T>> {
    Some(EditEvent::set(value, modifiers))
}

/// The input values with the gamma rounded to hundredths, as its step gives it.
fn rounded(value: &[f64; 3]) -> [f64; 3] {
    [value[0], (value[1] * 100.0).round() / 100.0, value[2]]
}

#[test]
fn a_thumb_drag_is_one_gesture_that_grabs_the_thumb_where_it_is_and_esc_cancels_it() {
    let mut rig = Rig::new([20.0, 1.0, 200.0], [0.0, 255.0]);
    // A little off the black thumb's centre: it moves with the pointer, without a jump.
    rig.press(rig.at(0, 20.0, 3.0));
    let id = begin(rig.input_event.clone());
    assert!(rig.has_focus(0, 0));
    rig.drag(rig.at(0, 50.0, 3.0));
    assert_eq!((rig.input_event.clone(), rig.rows[0].dragged), (update(id, [50.0, 1.0, 200.0]), Some(0)));
    rig.release(rig.at(0, 50.0, 3.0));
    assert_eq!(rig.input_event, commit(id, [50.0, 1.0, 200.0], true));
    assert!(rig.has_focus(0, 0), "the pressed thumb keeps the focus");

    rig.press(rig.at(0, 200.0, -2.0));
    let id = begin(rig.input_event.clone());
    rig.release(rig.at(0, 200.0, -2.0));
    assert_eq!(rig.input_event, commit(id, [50.0, 1.0, 200.0], false), "a still press commits unchanged");

    rig.revert = false;
    rig.press(rig.at(0, 50.0, 0.0));
    let id = begin(rig.input_event.clone());
    rig.drag(rig.at(0, 80.0, 0.0));
    rig.key(Key::Escape);
    assert_eq!(rig.input_event, Some(EditEvent::Cancel { id, reason: CancelReason::Escape }));
    rig.frame(vec![]);
    assert_eq!((rig.input[0], rig.rows[0].shown[0]), (80.0, 50.0), "the row shows the start until the host echoes it");
}

#[test]
fn a_press_off_the_thumbs_moves_the_nearest_there_and_overlapping_thumbs_follow_the_drag() {
    let mut rig = Rig::new([20.0, 1.0, 200.0], [100.0, 100.0]);
    // The gamma thumb sits at 110; the white one is nearest.
    rig.press(rig.at(0, 180.0, 0.0));
    let id = begin(rig.input_event.clone());
    rig.frame(vec![]);
    assert_eq!(rig.input_event, update(id, [20.0, 1.0, 180.0]));
    assert!(rig.has_focus(0, 2));
    rig.release(rig.at(0, 180.0, 0.0));

    // Stacked output thumbs: a still press moves neither, then the drag's direction picks.
    rig.press(rig.at(1, 100.0, 0.0));
    let id = begin(rig.output_event.clone());
    rig.frame(vec![]);
    assert_eq!((rig.output_event.clone(), rig.rows[1].dragged), (None, None));
    assert!(rig.has_focus(1, 0), "the nearest thumb, the first in row order, has the focus");
    rig.drag(rig.at(1, 120.0, 0.0));
    assert_eq!(rig.output_event, update(id, [100.0, 120.0]));
    assert!(rig.has_focus(1, 1));
    rig.drag(rig.at(1, 90.0, 0.0));
    assert_eq!(rig.output_event, update(id, [90.0, 100.0]));
    assert!(rig.has_focus(1, 0));
    rig.release(rig.at(1, 90.0, 0.0));
    assert_eq!(rig.output, [90.0, 100.0]);

    // Input ends two levels apart with the gamma between: left picks black, right white.
    rig.input = [100.0, 1.0, 102.0];
    rig.frame(vec![]);
    rig.press(rig.at(0, 101.0, 0.0));
    let id = begin(rig.input_event.clone());
    assert!(rig.has_focus(0, 1), "the gamma thumb is nearest");
    rig.drag(rig.at(0, 120.0, 0.0));
    assert_eq!(rig.input_event, update(id, [100.0, 1.0, 121.0]));
    rig.drag(rig.at(0, 90.0, 0.0));
    assert_eq!(rig.input_event, update(id, [89.0, 1.0, 102.0]));
    rig.release(rig.at(0, 90.0, 0.0));
}

#[test]
fn input_ends_keep_their_gap_and_output_thumbs_cross() {
    let mut rig = Rig::new([20.0, 1.0, 200.0], [30.0, 220.0]);
    rig.press(rig.at(0, 20.0, 0.0));
    let id = begin(rig.input_event.clone());
    rig.drag(rig.at(0, 250.0, 0.0));
    assert_eq!(rig.input_event, update(id, [198.0, 1.0, 200.0]), "black stops two levels below white, without pushing it");
    rig.release(rig.at(0, 250.0, 0.0));

    rig.input = [20.0, 1.0, 200.0];
    rig.frame(vec![]);
    rig.press(rig.on(0, 2, 0.0));
    let id = begin(rig.input_event.clone());
    rig.drag(rig.at(0, 0.0, 0.0));
    assert_eq!(rig.input_event, update(id, [20.0, 1.0, 22.0]), "white stops two levels above black");
    rig.release(rig.at(0, 0.0, 0.0));

    rig.press(rig.at(1, 30.0, 0.0));
    let id = begin(rig.output_event.clone());
    rig.drag(rig.at(1, 240.0, 0.0));
    assert_eq!(rig.output_event, update(id, [240.0, 220.0]), "output black passes output white");
    rig.release(rig.at(1, 240.0, 0.0));
    assert_eq!(rig.output, [240.0, 220.0]);
}

#[test]
fn the_gamma_thumb_follows_black_and_white_and_its_drag_maps_through_the_half_power() {
    let mut rig = Rig::new([0.0, 1.0, 200.0], [0.0, 255.0]);
    let near = |a: f32, b: f32| (a - b).abs() < 1e-3;
    assert!(near(rig.rows[0].xs[1], rig.at(0, 100.0, 0.0).x));
    rig.press(rig.at(0, 0.0, 0.0));
    rig.drag(rig.at(0, 100.0, 0.0));
    rig.frame(vec![]);
    assert_eq!(rig.rows[0].shown, [100.0, 1.0, 200.0], "a black drag reports only black");
    assert!(near(rig.rows[0].xs[1], rig.at(0, 150.0, 0.0).x), "the gamma thumb moves in proportion");
    rig.release(rig.at(0, 100.0, 0.0));

    rig.input = [0.0, 1.0, 200.0];
    rig.frame(vec![]);
    rig.press(rig.on(0, 1, 0.0));
    let id = begin(rig.input_event.clone());
    let gamma = |rig: &Rig| match &rig.input_event {
        Some(EditEvent::Update { value, .. }) => rounded(value),
        other => panic!("expected an update, got {other:?}"),
    };
    rig.drag(rig.at(0, 50.0, 0.0));
    assert_eq!(gamma(&rig), [0.0, 2.0, 200.0], "a quarter of the way: ln 0.25 / ln 0.5");
    rig.drag(rig.at(0, 150.0, 0.0));
    assert_eq!(gamma(&rig), [0.0, 0.42, 200.0], "three quarters: 0.415 rounded");
    rig.drag(rig.at(0, -80.0, 0.0));
    assert_eq!(gamma(&rig), [0.0, 9.99, 200.0], "past black: the top of the gamma range");
    rig.frame(vec![]);
    assert!(rig.rows[0].xs[1] > rig.rows[0].xs[0], "the gamma thumb stays right of black");
    rig.drag(rig.at(0, 400.0, 0.0));
    assert_eq!(gamma(&rig), [0.0, 0.01, 200.0], "past white: the bottom of the gamma range");
    rig.frame(vec![]);
    assert!(rig.rows[0].xs[1] < rig.rows[0].xs[2], "the gamma thumb stays left of white");
    rig.release(rig.at(0, 400.0, 0.0));
    assert!(matches!(rig.input_event, Some(EditEvent::Commit { id: c, .. }) if c == id));
}

#[test]
fn keys_move_each_thumb_within_its_bounds_and_the_gamma_thumb_the_way_they_point() {
    let mut rig = Rig::new([20.0, 1.0, 200.0], [0.0, 255.0]);
    let key = |key, modifiers| Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers };
    let expect = |rig: &mut Rig, event: Event, modifiers: Modifiers, value: [f64; 3]| {
        rig.modifiers = modifiers;
        rig.frame(vec![event]);
        let got = match &rig.input_event {
            Some(EditEvent::Set { value, modifiers, .. }) => Some((rounded(value), *modifiers)),
            _ => None,
        };
        assert_eq!(got, Some((value, modifiers)), "{:?}", rig.input_event);
        rig.modifiers = Modifiers::NONE;
        rig.frame(vec![]);
        assert_eq!(rig.input_event, None, "one event per input");
    };
    rig.focus(0, 0);
    expect(&mut rig, key(Key::ArrowRight, Modifiers::NONE), Modifiers::NONE, [21.0, 1.0, 200.0]);
    expect(&mut rig, key(Key::ArrowUp, Modifiers::SHIFT), Modifiers::SHIFT, [31.0, 1.0, 200.0]);
    expect(&mut rig, key(Key::PageUp, Modifiers::NONE), Modifiers::NONE, [57.0, 1.0, 200.0]);
    expect(&mut rig, key(Key::End, Modifiers::NONE), Modifiers::NONE, [198.0, 1.0, 200.0]);
    expect(&mut rig, key(Key::Home, Modifiers::NONE), Modifiers::NONE, [0.0, 1.0, 200.0]);

    // The gamma thumb moves the way the keys point: right lowers the gamma.
    rig.focus(0, 1);
    expect(&mut rig, key(Key::ArrowRight, Modifiers::NONE), Modifiers::NONE, [0.0, 0.99, 200.0]);
    expect(&mut rig, key(Key::ArrowDown, Modifiers::SHIFT), Modifiers::SHIFT, [0.0, 1.09, 200.0]);
    expect(&mut rig, key(Key::PageUp, Modifiers::NONE), Modifiers::NONE, [0.0, 0.09, 200.0]);
    expect(&mut rig, key(Key::PageDown, Modifiers::NONE), Modifiers::NONE, [0.0, 1.09, 200.0]);
    expect(&mut rig, key(Key::Home, Modifiers::NONE), Modifiers::NONE, [0.0, 9.99, 200.0]);
    expect(&mut rig, key(Key::End, Modifiers::NONE), Modifiers::NONE, [0.0, 0.01, 200.0]);
    // A screen reader's Increment raises the value, as the gamma field's does.
    let increment = Event::AccessKitActionRequest(ActionRequest {
        action: Action::Increment,
        target_tree: TreeId::ROOT,
        target_node: rig.id(0, 1).accesskit_id(),
        data: None,
    });
    expect(&mut rig, increment, Modifiers::NONE, [0.0, 0.02, 200.0]);
    expect(&mut rig, key(Key::Backspace, Modifiers::COMMAND), Modifiers::COMMAND, [0.0, 1.0, 200.0]);
    // The wheel over the row moves the focused gamma thumb as Up does.
    rig.drag(rig.at(0, 240.0, 0.0));
    let wheel = Event::MouseWheel {
        unit: MouseWheelUnit::Line,
        delta: vec2(0.0, 1.0),
        phase: TouchPhase::Move,
        source: MouseWheelSource::Wheel,
        modifiers: Modifiers::NONE,
    };
    expect(&mut rig, wheel, Modifiers::NONE, [0.0, 0.99, 200.0]);

    rig.focus(0, 2);
    expect(&mut rig, key(Key::Home, Modifiers::NONE), Modifiers::NONE, [0.0, 0.99, 2.0]);
    rig.key(Key::ArrowLeft);
    assert_eq!(rig.input_event, None, "white is at its bound");
}

#[test]
fn a_double_click_or_ctrl_backspace_resets_a_thumb_and_a_tap_is_a_set() {
    let mut rig = Rig::new([20.0, 2.0, 200.0], [50.0, 200.0]);
    rig.tap(rig.at(1, 100.0, 0.0));
    assert_eq!(rig.output_event, set([100.0, 200.0], Modifiers::NONE), "the nearest thumb moves there");
    assert!(rig.has_focus(1, 0));
    rig.tap(rig.at(1, 100.0, 0.0));
    assert_eq!(rig.output_event, set([0.0, 200.0], Modifiers::NONE), "the double-click resets it");

    rig.time += 1.0;
    rig.tap(rig.on(1, 1, 2.0));
    assert_eq!(rig.output_event, None, "a tap on a thumb moves nothing");
    assert!(rig.has_focus(1, 1));
    rig.modifiers = Modifiers::COMMAND;
    rig.key(Key::Backspace);
    assert_eq!(rig.output_event, set([0.0, 255.0], Modifiers::COMMAND), "the focused thumb's default");
    rig.modifiers = Modifiers::NONE;

    // The second click's press is a gesture: it commits the reset.
    rig.time += 1.0;
    rig.press(rig.on(0, 1, 2.0));
    rig.release(rig.on(0, 1, 2.0));
    rig.press(rig.on(0, 1, 2.0));
    let id = begin(rig.input_event.clone());
    rig.release(rig.on(0, 1, 2.0));
    assert_eq!(rig.input_event, commit(id, [20.0, 1.0, 200.0], true));
}

/// A press on a part while another control's typing run has the focus takes the focus from the
/// run, waits for its Commit to go out, and begins in the pass after it, from the host's value
/// then. Without a run a part's press runs at once.
#[test]
fn a_press_that_waits_runs_the_pass_after_the_typing_runs_commit() {
    let mut rig = Rig::new([20.0, 1.0, 200.0], [0.0, 255.0]);
    rig.waits = true;
    rig.field = Some(Field { value: 40.0, id: Id::NULL, rect: Rect::NOTHING, event: None });
    rig.frame(vec![]);
    let pos = rig.field.as_ref().unwrap().rect.center();
    rig.tap(pos);
    rig.frame(vec![Event::Text("7".to_owned())]);
    let event = rig.field_event();
    let run = begin(event.clone());
    assert_eq!(event, Some(EditEvent::begin_with(run, 7.0)), "the field's draft reaches the host on the Begin");
    rig.frame(vec![]);

    // The field is drawn before the rows, so its Commit goes out one pass after the press.
    let at = rig.at(1, 100.0, 0.0);
    rig.press(at);
    assert_eq!(rig.output_event, None, "the press waits");
    assert!(!rig.field_focused(), "and takes the focus from the field, which ends the run");
    rig.frame(vec![]);
    assert_eq!(rig.field_event(), commit(run, 7.0, true));
    assert_eq!(rig.output_event, None, "the pass of the Commit is not the press's");
    rig.output = [10.0, 255.0];
    rig.frame(vec![]);
    let id = begin(rig.output_event.clone());
    rig.frame(vec![]);
    assert_eq!(rig.output_event, update(id, [100.0, 255.0]), "the jump, from the value the host has now");
    rig.drag(rig.at(1, 120.0, 0.0));
    rig.release(rig.at(1, 120.0, 0.0));
    assert_eq!(rig.output_event, commit(id, [120.0, 255.0], true));

    // No run: a press begins in its own pass.
    rig.field = None;
    rig.press(rig.at(1, 50.0, 0.0));
    begin(rig.output_event.clone());
}

#[test]
fn values_outside_the_range_draw_at_the_ends_and_one_that_is_not_finite_begins_no_drag() {
    let mut rig = Rig::new([10.0, f64::NAN, 200.0], [-20.0, 300.0]);
    assert_eq!(rig.rows[1].shown, [0.0, 255.0]);
    assert_eq!(rig.rows[0].shown, [10.0, 1.0, 200.0], "an unset gamma draws at 1");
    rig.press(rig.on(0, 0, 0.0));
    rig.drag(rig.at(0, 40.0, 0.0));
    assert_eq!(rig.input_event, None);
    rig.release(rig.at(0, 40.0, 0.0));
    assert_eq!(rig.input_event, None);

    rig.press(rig.on(1, 0, 0.0));
    let id = begin(rig.output_event.clone());
    rig.drag(rig.at(1, 40.0, 0.0));
    assert_eq!(rig.output_event, update(id, [40.0, 300.0]), "an edit leaves the other value as it is");
    rig.enabled = false;
    rig.frame(vec![]);
    assert_eq!(rig.output_event, Some(EditEvent::Cancel { id, reason: CancelReason::Disabled }), "disabling mid-drag cancels");
}

/// An unset (NaN) value is reported to a screen reader as mixed: no value, the bounds, and both
/// steps offered; an Increment sets it from where its thumb is drawn.
#[test]
fn a_thumb_whose_value_is_not_finite_is_reported_mixed() {
    let mut rig = Rig::new([10.0, f64::NAN, 200.0], [0.0, 255.0]);
    let gamma = &rig.nodes[&rig.id(0, 1)];
    assert_eq!((gamma.numeric_value(), gamma.min_numeric_value(), gamma.max_numeric_value()), (None, Some(0.01), Some(9.99)));
    assert!(gamma.supports_action(Action::Increment) && gamma.supports_action(Action::Decrement));
    assert_eq!(rig.nodes[&rig.id(0, 0)].numeric_value(), Some(10.0), "a set value keeps its value");
    let increment = Event::AccessKitActionRequest(ActionRequest {
        action: Action::Increment,
        target_tree: TreeId::ROOT,
        target_node: rig.id(0, 1).accesskit_id(),
        data: None,
    });
    rig.frame(vec![increment]);
    let Some(EditEvent::Set { value, .. }) = &rig.input_event else { panic!("{:?}", rig.input_event) };
    assert!(value[1].is_finite(), "{value:?}");
}

#[test]
fn each_thumb_is_a_named_slider_and_a_tab_stop_in_row_order() {
    let mut rig = Rig::new([20.0, 1.5, 200.0], [0.0, 255.0]);
    let expected = [
        (0, 0, "Input black", 20.0, 0.0, 198.0),
        (0, 1, "Gamma", 1.5, 0.01, 9.99),
        (0, 2, "Input white", 200.0, 22.0, 255.0),
        (1, 0, "Output black", 0.0, 0.0, 255.0),
        (1, 1, "Output white", 255.0, 0.0, 255.0),
    ];
    for (row, i, label, value, min, max) in expected {
        let node = &rig.nodes[&rig.id(row, i)];
        assert_eq!(node.role(), Role::Slider);
        assert_eq!(node.label(), Some(label));
        assert_eq!((node.numeric_value(), node.min_numeric_value(), node.max_numeric_value()), (Some(value), Some(min), Some(max)), "{label}");
    }
    rig.focus(0, 0);
    for (row, i) in [(0, 1), (0, 2), (1, 0), (1, 1)] {
        rig.key(Key::Tab);
        assert!(rig.has_focus(row, i), "Tab reaches thumb {i} of row {row}");
    }
}

/// A thumb drag whose gesture the host ends for an options bar's change goes on with the
/// pointer: the next movement begins a new gesture, the dragged thumb follows the pointer from
/// where the drag began (also when the bar set it), the other thumbs keep the host's values (a
/// white point the bar set stays), and the release commits.
#[test]
fn a_thumb_drag_the_host_ended_for_another_controls_change_goes_on_with_the_pointer() {
    for (bar_thumb, bar_value) in [(2, 180.0), (0, 5.0)] {
        let mut rig = Rig::new([20.0, 1.0, 200.0], [0.0, 255.0]);
        rig.hosted = Some((GestureHost::new(), Levels { now: (rig.input, rig.output), undo: Vec::new() }));
        rig.press(rig.at(0, 20.0, 3.0));
        let first = begin(rig.input_event.clone());
        rig.drag(rig.at(0, 50.0, 3.0));
        assert_eq!(rig.input_event, update(first, [50.0, 1.0, 200.0]));
        let ctx = rig.ctx.clone();
        let (host, doc) = rig.hosted.as_mut().expect("hosted");
        let set_thumb = move |doc: &mut Levels, v: f64| doc.apply(|now| now.0[bar_thumb] = v);
        host.handle(&ctx, doc, EditEvent::set(bar_value, Modifiers::NONE), set_thumb);
        let mut after = [20.0, 1.0, 200.0];
        after[bar_thumb] = bar_value;
        assert_eq!((doc.now.0, host.active()), (after, None));
        rig.input = doc.now.0;
        rig.drag(rig.at(0, 70.0, 3.0));
        let id = begin(rig.input_event.clone());
        assert!(id > first);
        rig.frame(vec![]);
        let white = if bar_thumb == 2 { bar_value } else { 200.0 };
        assert_eq!(rig.input_event, update(id, [70.0, 1.0, white]), "the bar set thumb {bar_thumb}");
        rig.release(rig.at(0, 70.0, 3.0));
        assert_eq!(rig.input_event, commit(id, [70.0, 1.0, white], true));
        assert_eq!(rig.input, [70.0, 1.0, white]);
    }
}

/// A thumb's operation applies within its bounds on the row it is applied to: a step and an End
/// stop at the neighbour less the gap, the other thumbs keep their values (an unset gamma stays
/// unset), and an unset gamma steps from where it is drawn.
#[test]
fn a_row_operation_applies_within_the_thumbs_bounds_on_the_row_it_meets() {
    let rules = ThumbRow::rules(&(0.0..=255.0), Some(1.0), &input_thumbs());
    let apply = |thumb, op, row: [f64; 3]| rules.apply(&RowOp { thumb, op }, &row);
    assert_eq!(apply(0, ScalarOp::By(1.0), [40.0, 1.0, 200.0]), [41.0, 1.0, 200.0]);
    assert_eq!(apply(0, ScalarOp::By(1.0), [40.0, 1.0, 42.0]), [40.0, 1.0, 42.0], "black stays the gap below white");
    assert_eq!(apply(2, ScalarOp::ToBound(Bound::Low), [40.0, 1.0, 200.0]), [40.0, 1.0, 42.0], "Home goes to black plus the gap");
    assert_eq!(apply(0, ScalarOp::Set(250.0), [40.0, 1.0, 200.0]), [198.0, 1.0, 200.0], "a set stops where a drag would");
    let unset = apply(0, ScalarOp::By(1.0), [0.0, f64::NAN, 255.0]);
    assert!(unset[0] == 1.0 && unset[1].is_nan() && unset[2] == 255.0, "{unset:?}");
    let gamma = apply(1, ScalarOp::By(0.01), [0.0, f64::NAN, 255.0]);
    assert!((gamma[1] - 1.01).abs() < 1e-9, "an unset gamma steps from 1: {gamma:?}");
}

/// One row of `N` thumbs on `0..=range` with a point to a level, for the split and wrapping
/// tests; the host applies every value it is sent.
struct Mini<const N: usize> {
    ctx: Context,
    time: f64,
    modifiers: Modifiers,
    value: [f64; N],
    range: f64,
    wrap: Option<f64>,
    /// The anchor and the span of a row ordered round from one thumb.
    anchored: Option<(usize, f64)>,
    thumbs: fn() -> [Thumb; N],
    event: Option<EditEvent<[f64; N]>>,
    travel: (f32, f32),
    y: f32,
    labels: Vec<String>,
    /// The half labels are translated.
    translated: bool,
}

impl<const N: usize> Mini<N> {
    fn new(value: [f64; N], range: f64, wrap: Option<f64>, thumbs: fn() -> [Thumb; N]) -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut mini = Self {
            ctx,
            time: 0.0,
            modifiers: Modifiers::NONE,
            value,
            range,
            wrap,
            anchored: None,
            thumbs,
            event: None,
            travel: (0.0, 0.0),
            y: 0.0,
            labels: Vec::new(),
            translated: false,
        };
        mini.frame(vec![]);
        mini
    }

    fn frame(&mut self, events: Vec<Event>) {
        let events = std::iter::once(Event::ModifiersChanged(self.modifiers)).chain(events).collect();
        let input =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), time: Some(self.time), events, ..Default::default() };
        self.time += 1.0 / 60.0;
        let ctx = self.ctx.clone();
        let output = ctx.run_ui(input, |ui| {
            let mut style = ThumbRow::<N>::style(&Tokens::of(ui));
            if self.translated {
                style = style.localized(|key| match key {
                    "thumbRow.low" => Some("{name} bas".to_owned()),
                    "thumbRow.high" => Some("{name} haut".to_owned()),
                    _ => None,
                });
            }
            // A level a point.
            let mut row =
                ThumbRow::new("mini", self.value, &(0.0..=self.range), Some(1.0), (self.thumbs)()).width(self.range as f32 + 2.0 * style.inset());
            if let (Some(period), Some((anchor, span))) = (self.wrap, self.anchored) {
                row = row.wrap_from(period, anchor, span);
            } else if let Some(period) = self.wrap {
                row = row.wrap(period);
            }
            let out = row.show(ui, &style);
            let rect = out.edit.response.rect;
            self.travel = (rect.left() + style.inset(), rect.right() - style.inset());
            self.y = rect.center().y;
            self.event.clone_from(&out.edit.event);
            if let Some(v) = out.edit.event.and_then(|e| e.value().copied()) {
                self.value = v;
            }
        });
        if let Some(update) = output.platform_output.accesskit_update.clone() {
            self.labels = update.nodes.iter().filter(|(_, n)| n.role() == Role::Slider).filter_map(|(_, n)| n.label().map(str::to_owned)).collect();
        }
        output.drop_without_applying_deltas();
    }

    fn at(&self, level: f64, dx: f32) -> Pos2 {
        pos2(self.travel.0 + (self.travel.1 - self.travel.0) * (level / self.range) as f32 + dx, self.y)
    }

    fn press(&mut self, pos: Pos2) {
        self.frame(vec![Event::PointerMoved(pos), button(pos, true)]);
    }

    fn drag(&mut self, pos: Pos2) {
        self.frame(vec![Event::PointerMoved(pos)]);
    }

    fn release(&mut self, pos: Pos2) {
        self.frame(vec![button(pos, false)]);
    }

    fn tap(&mut self, pos: Pos2) {
        self.frame(vec![Event::PointerMoved(pos), button(pos, true), button(pos, false)]);
    }
}

fn split_pair() -> [Thumb; 2] {
    [Thumb::new("Black", Color32::BLACK).split(), Thumb::new("Black", Color32::BLACK)]
}

fn hues() -> [Thumb; 2] {
    [Thumb::new("Hue from", Color32::RED), Thumb::new("Hue to", Color32::YELLOW)]
}

#[test]
fn alt_drag_splits_and_a_half_moves_alone() {
    let mut mini = Mini::new([100.0, 100.0], 255.0, None, split_pair);
    mini.labels.sort();
    assert_eq!(mini.labels, ["Black high", "Black low"], "a split thumb is two sliders");
    mini.translated = true;
    mini.frame(vec![]);
    mini.labels.sort();
    assert_eq!(mini.labels, ["Black bas", "Black haut"], "the half names come from the style's labels");
    // Alt held at the press: the halves coincide, so the drag's direction picks the half.
    mini.modifiers = Modifiers::ALT;
    mini.press(mini.at(100.0, 0.0));
    let id = begin(mini.event.clone());
    mini.frame(vec![]);
    mini.drag(mini.at(130.0, 0.0));
    let alt = |value| Some(EditEvent::update(id, value, Modifiers::ALT));
    assert_eq!(mini.event, alt([100.0, 130.0]));
    // Releasing Alt mid-drag changes nothing: the press decided.
    mini.modifiers = Modifiers::NONE;
    mini.drag(mini.at(140.0, 0.0));
    assert_eq!(mini.event, update(id, [100.0, 140.0]));
    mini.release(mini.at(140.0, 0.0));
    assert_eq!(mini.value, [100.0, 140.0]);

    // The low half stops at the high half.
    mini.modifiers = Modifiers::ALT;
    mini.press(mini.at(100.0, 0.0));
    let id = begin(mini.event.clone());
    mini.drag(mini.at(200.0, 0.0));
    assert_eq!(mini.event, Some(EditEvent::update(id, [140.0, 140.0], Modifiers::ALT)));
    mini.release(mini.at(200.0, 0.0));
}

#[test]
fn an_unsplit_pair_moves_together_keeping_its_spread() {
    let mut mini = Mini::new([100.0, 130.0], 255.0, None, split_pair);
    // The high half is grabbed; the low half goes along.
    mini.press(mini.at(130.0, 0.0));
    let id = begin(mini.event.clone());
    mini.drag(mini.at(150.0, 0.0));
    assert_eq!(mini.event, update(id, [120.0, 150.0]));
    // At the end of the range the pair stops with its spread intact.
    mini.drag(mini.at(300.0, 0.0));
    assert_eq!(mini.event, update(id, [225.0, 255.0]));
    mini.release(mini.at(300.0, 0.0));
    assert_eq!(mini.value, [225.0, 255.0]);
}

#[test]
fn a_wrapped_row_crosses_the_period_and_keeps_the_short_way_distance() {
    let mut mini = Mini::new([350.0, 100.0], 360.0, Some(360.0), hues);
    // A thumb dragged past the end appears at the start.
    mini.press(mini.at(350.0, 0.0));
    let id = begin(mini.event.clone());
    mini.drag(mini.at(350.0, 20.0));
    assert_eq!(mini.event, update(id, [10.0, 100.0]));
    mini.release(mini.at(350.0, 20.0));
    assert_eq!(mini.value, [10.0, 100.0]);

    // A press 20 degrees across the seam from the first thumb is nearer to it than to the
    // second (at 100), though 330 points away on the bar.
    mini.value = [350.0, 100.0];
    mini.frame(vec![]);
    mini.tap(mini.at(10.0, 0.0));
    assert_eq!(mini.event, set([10.0, 100.0], Modifiers::NONE));
}

fn range_bar() -> [Thumb; 4] {
    [
        Thumb::new("Outer start", Color32::GRAY),
        Thumb::new("Inner start", Color32::WHITE),
        Thumb::new("Inner end", Color32::WHITE),
        Thumb::new("Outer end", Color32::GRAY),
    ]
}

/// A hue range bar's rules: four thumbs, wrap 360, the first thumb the anchor, a span of 359.
fn range_rules() -> craft_ui::controls::thumb_row::RowRules<4> {
    ThumbRow::new("rules", [0.0; 4], &(0.0..=360.0), Some(1.0), range_bar()).wrap_from(360.0, 0, 359.0).row_rules()
}

/// PhotoCraft's `move_hue_range_handle` cases on a row (its `adjust_editors` tests, and the
/// rules in its comments): the pointer's hue sets a handle, measured from handle 0.
#[test]
fn an_anchored_row_moves_a_thumb_as_the_hue_range_bar_does() {
    let rules = range_rules();
    let set = |thumb, to: f64, row: [f64; 4]| rules.apply(&RowOp { thumb, op: ScalarOp::Set(to) }, &row)[thumb];

    // The canonical bounds [-360, -10, -1, -1] (here 0, 350, 359, 359): handle 2 to 354.
    assert_eq!(set(2, 354.0, [0.0, 350.0, 359.0, 359.0]), 354.0);
    // The same row as it comes in, not yet canonical: [0, 350, 100, 200].
    assert_eq!(set(2, 354.0, [0.0, 350.0, 100.0, 200.0]), 354.0);
    // An ordinary move between neighbours.
    assert_eq!(set(2, 65.0, [0.0, 30.0, 60.0, 90.0]), 65.0);
    // A handle stops at its neighbours.
    assert_eq!(set(2, 20.0, [0.0, 30.0, 60.0, 90.0]), 30.0);
    assert_eq!(set(2, 120.0, [0.0, 30.0, 60.0, 90.0]), 90.0);

    // Spans over half a period: the inner handles 250 degrees apart, the outer one 300 from the anchor.
    let wide = [0.0, 10.0, 260.0, 300.0];
    assert_eq!(set(1, 240.0, wide), 240.0, "inner start 240 past the anchor, over half a period");
    assert_eq!(set(3, 355.0, wide), 355.0);
    assert_eq!(set(3, 200.0, wide), 260.0, "the last stops at the thumb before it");
    // Dragging past the cap: the last handle stops 359 past the anchor, whatever is 360 past.
    assert_eq!(set(3, 359.0, wide), 359.0);
    assert_eq!(set(3, 359.9, wide), 359.0);

    // The same across the seam: anchor 200, the others at 210, 40 (250 on) and 140 (300 on).
    let seam = [200.0, 210.0, 40.0, 140.0];
    assert_eq!(set(3, 190.0, seam), 190.0, "350 past the anchor");
    assert_eq!(set(3, 199.5, seam), 199.0, "the cap, 359 past the anchor");
    assert_eq!(set(1, 30.0, seam), 30.0, "up to the next handle at 40, past the seam");
    assert_eq!(set(1, 100.0, seam), 40.0);
    assert_eq!(set(1, 150.0, seam), 40.0, "away from the anchor, not before it");

    // The anchor goes the short way and stays within half a period before the next handle.
    let row = [0.0, 100.0, 200.0, 300.0];
    assert_eq!(set(0, 50.0, row), 50.0);
    assert_eq!(set(0, 120.0, row), 100.0, "not past the next handle");
    assert_eq!(set(0, 20.0, [0.0, 250.0, 260.0, 270.0]), 70.0, "at most 180 before the next handle (250 - 180)");
    assert_eq!(set(0, 300.0, row), 301.0, "the short way round, stopping where the last handle is 359 on");
    assert_eq!(set(0, 300.0, [0.0, 100.0, 120.0, 130.0]), 300.0, "the short way round, 60 back");
    // The others keep their values when the anchor moves.
    let moved = rules.apply(&RowOp { thumb: 0, op: ScalarOp::Set(50.0) }, &row);
    assert_eq!(moved, [50.0, 100.0, 200.0, 300.0]);
}

#[test]
fn keys_and_home_and_end_keep_an_anchored_row_in_order_and_end_leaves_the_start() {
    let rules = range_rules();
    let op = |thumb, op, row: [f64; 4]| rules.apply(&RowOp { thumb, op }, &row)[thumb];
    let row = [350.0, 10.0, 30.0, 60.0];
    assert_eq!(op(1, ScalarOp::By(1.0), row), 11.0);
    assert_eq!(op(1, ScalarOp::By(100.0), row), 30.0, "up to the next handle");
    assert_eq!(op(1, ScalarOp::ToBound(Bound::Low), row), 350.0, "Home: the anchor's place");
    assert_eq!(op(1, ScalarOp::ToBound(Bound::High), row), 30.0, "End: the next handle's place");
    assert_eq!(op(3, ScalarOp::ToBound(Bound::High), row), 349.0, "End of the last: 359 past the anchor");
    // A step of a thumb across the seam appears at the start of the range.
    assert_eq!(op(2, ScalarOp::By(1.0), [300.0, 340.0, 359.0, 20.0]), 0.0);

    // End in a wrapping row without neighbours goes to the range's end less a step, not to Home's value.
    let free = ThumbRow::new("free", [0.0; 2], &(0.0..=360.0), Some(1.0), hues()).wrap(360.0).row_rules();
    let at = |bound| free.apply(&RowOp { thumb: 0, op: ScalarOp::ToBound(bound) }, &[100.0, 200.0])[0];
    assert_eq!(at(Bound::Low), 0.0);
    assert_eq!(at(Bound::High), 359.0);
}

#[test]
fn dragging_an_anchored_row_stops_at_the_neighbours_and_the_span_and_the_anchor_turns() {
    let mut mini = Mini::new([340.0, 10.0, 50.0, 80.0], 360.0, Some(360.0), range_bar);
    mini.anchored = Some((0, 359.0));
    mini.frame(vec![]);
    // The last handle, dragged on round: it stops 359 past the anchor (339).
    mini.press(mini.at(80.0, 0.0));
    let id = begin(mini.event.clone());
    mini.drag(mini.at(80.0, 330.0));
    assert_eq!(mini.event, update(id, [340.0, 10.0, 50.0, 339.0]));
    mini.release(mini.at(80.0, 330.0));
    assert_eq!(mini.value, [340.0, 10.0, 50.0, 339.0]);

    // The anchor dragged forward over the seam: it stops at the next handle.
    let mut mini = Mini::new([340.0, 10.0, 50.0, 80.0], 360.0, Some(360.0), range_bar);
    mini.anchored = Some((0, 359.0));
    mini.frame(vec![]);
    mini.press(mini.at(340.0, 0.0));
    let id = begin(mini.event.clone());
    mini.drag(mini.at(340.0, 30.0));
    assert_eq!(mini.event, update(id, [10.0, 10.0, 50.0, 80.0]));
    mini.release(mini.at(340.0, 30.0));
}
