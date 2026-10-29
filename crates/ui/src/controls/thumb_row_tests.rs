//! Thumb-row behaviour in a headless egui context, with a levels editor's two rows as the host:
//! black, gamma and white on the input row (the gamma between the others, at
//! `black + (white − black) · 0.5^gamma`), and two free thumbs on the output row.

use std::collections::HashMap;

use egui::accesskit::{Action, ActionRequest, Node, Role, TreeId};
use egui::{
    Color32, Context, Event, Id, Key, Modifiers, MouseWheelSource, MouseWheelUnit, PointerButton, Pos2, RawInput, Rect, TouchPhase, Ui, pos2, vec2,
};

use super::thumb_row::{Follow, Order, Thumb, ThumbRow, ThumbRowStyle};
use crate::composites::levels_editor::{LevelsEditor, gamma_at, gamma_from};
use crate::controls::NumberField;
use crate::edit::{CancelReason, EditEvent, EditPolicy, GestureHost, GestureId, UndoMarks};
use crate::localized::LocaleNumbers;
use crate::primitives::Interactive;
use crate::theme::{Base, Tokens, TokensOf, install};

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
        let ordered = |below, above| Order::Between { below, above, gap: GAP };
        let gamma = Follow { at: gamma_at, value_at: gamma_from };
        let thumbs = [
            Thumb::new("Input black", Color32::BLACK).order(ordered(None, Some(2))).default_value(0.0),
            Thumb::new("Gamma", Color32::from_gray(128)).follows(&(0.01..=9.99), 0.01, gamma).default_value(1.0).unset_at(1.0).falling(),
            Thumb::new("Input white", Color32::WHITE).order(ordered(Some(0), None)).default_value(255.0),
        ];
        let row = ThumbRow::new("rig-input", self.input, &range, Some(1.0), thumbs)
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
        Some(EditEvent::Begin { id }) => id,
        other => panic!("expected Begin, got {other:?}"),
    }
}

fn update<T>(id: GestureId, value: T) -> Option<EditEvent<T>> {
    Some(EditEvent::Update { id, value, modifiers: Modifiers::NONE })
}

fn commit<T>(id: GestureId, value: T, changed: bool) -> Option<EditEvent<T>> {
    Some(EditEvent::Commit { id, value, changed, modifiers: Modifiers::NONE })
}

fn set<T>(value: T, modifiers: Modifiers) -> Option<EditEvent<T>> {
    Some(EditEvent::Set { value, modifiers })
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
            Some(EditEvent::Set { value, modifiers }) => Some((rounded(value), *modifiers)),
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
    let run = begin(rig.field_event());
    rig.frame(vec![]);
    assert_eq!(rig.field_event(), update(run, 7.0), "the field's draft reaches the host");

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
