//! The number field's own logic: stepping, the range on typed values, presets and the label scrub across a pointer warp.

use crate::label::kit::Field;
use craft_behavior::EditEvent;
use craft_behavior::{Mixed, NumberCodec, Reader};
use craft_host::target::ShellInfo;
use craft_host::{AppInfo, Host};
use craft_ui::contract::{Bind, Entry, Props as _, Role, Show as _, ShowContainer as _, SideLabel};
use craft_ui::runtime::InstallSpec;
use craft_ui::runtime::env::UnitKind;
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer::{click, moved, press};
use craft_ui::testing::windows::FakeSystem;
use craft_ui::testing::{Fixture, Frames, trial};
use craft_ui::text::{NumberField, NumberPick};
use egui::{Event, Key, Modifiers, MouseWheelSource, MouseWheelUnit, Pos2, Rect, TouchPhase, Ui, Vec2};
use std::cell::{Cell, RefCell};
use std::ops::RangeInclusive;
use std::rc::Rc;
use std::sync::Arc;

const START: f64 = 10.0;
/// The value of the first typed character, then of the second.
const TYPED: (f64, f64) = (7.0, 75.0);
/// How far two values may differ and still count as the same.
const SAME: f64 = 0.01;

/// A number field of a fixed id, driven frame by frame.
struct Rig {
    frames: Frames,
    value: RefCell<Option<f64>>,
    rect: Cell<Rect>,
    picked: Cell<Option<NumberPick>>,
    range: Option<RangeInclusive<f64>>,
    presets: Vec<Entry<NumberPick>>,
}

impl Rig {
    fn new(start: f64, range: Option<RangeInclusive<f64>>) -> Rig {
        let mut rig = Rig {
            frames: Frames::new(),
            value: RefCell::new(Some(start)),
            rect: Cell::new(Rect::NOTHING),
            picked: Cell::new(None),
            range,
            presets: Vec::new(),
        };
        rig.idle(3);
        rig
    }

    fn frame(&mut self, events: Vec<Event>) {
        let Rig { frames, value, rect, picked, range, presets } = self;
        frames.frame(events, |ui| show(ui, value, rect, picked, range, presets));
    }

    fn idle(&mut self, frames: usize) {
        for _ in 0..frames {
            self.frame(Vec::new());
        }
    }

    fn focus(&mut self) {
        let Rig { frames, value, rect, picked, range, presets } = self;
        let at = rect.get().center();
        click(frames, at, &mut |ui| show(ui, value, rect, picked, range, presets));
        self.idle(2);
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        self.frame(vec![key_press(key, modifiers)]);
        self.idle(3);
    }

    fn value(&self) -> Option<f64> {
        *self.value.borrow()
    }

    /// One wheel notch of `dy` lines over the field with `modifiers` held.
    fn wheel(&mut self, dy: f32, modifiers: Modifiers) {
        let at = self.rect.get().center();
        self.frame(vec![moved(at)]);
        self.frame(vec![Event::MouseWheel {
            unit: MouseWheelUnit::Line,
            delta: Vec2::new(0.0, dy),
            phase: TouchPhase::Move,
            source: MouseWheelSource::Wheel,
            modifiers,
        }]);
        self.idle(3);
    }
}

fn show(
    ui: &mut Ui,
    value: &RefCell<Option<f64>>,
    rect: &Cell<Rect>,
    picked: &Cell<Option<NumberPick>>,
    range: &Option<RangeInclusive<f64>>,
    presets: &[Entry<NumberPick>],
) {
    let mut guard = value.borrow_mut();
    let mut field = NumberField::new("Size", Bind::bound(&mut *guard)).id_salt("size");
    if let Some(range) = range {
        field = field.range(range.clone());
    }
    if !presets.is_empty() {
        field = field.presets(presets);
    }
    let out = field.show(ui);
    rect.set(out.response.rect);
    if out.preset.is_some() {
        picked.set(out.preset);
    }
}

/// An arrow goes to the next multiple of the step in its direction, Shift moves ten steps, and the range holds.
#[trial]
fn arrows_step_to_the_next_multiple_and_stay_in_range() {
    let mut rig = Rig::new(5.1, Some(0.0..=7.0));
    rig.focus();
    rig.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(rig.value(), Some(5.0), "down from 5.1 goes to 5");
    rig.key(Key::ArrowUp, Modifiers::NONE);
    rig.key(Key::ArrowUp, Modifiers::NONE);
    assert_eq!(rig.value(), Some(7.0), "up reaches the end of the range");
    rig.key(Key::ArrowUp, Modifiers::NONE);
    assert_eq!(rig.value(), Some(7.0), "up at the end stays there");
    rig.key(Key::ArrowDown, Modifiers::SHIFT);
    assert_eq!(rig.value(), Some(0.0), "Shift moves ten steps and stops at the start of the range");
}

/// A host that owns the value writes the first typed character in the pass that begins the edit, and the gesture stays
/// open: Escape then puts the start value back.
#[trial]
fn a_given_value_written_in_the_begin_pass_keeps_the_gesture() {
    let mut frames = Frames::new();
    let value = RefCell::new(Some(START));
    let at = Cell::new(Pos2::ZERO);
    let events = RefCell::new(Vec::<EditEvent<Option<f64>>>::new());
    let mut show = |ui: &mut Ui| {
        let current = *value.borrow();
        let out = NumberField::new("Size", Bind::given(current)).id_salt("size").show(ui);
        at.set(out.response.rect.center());
        if let Some(event) = out.event.clone() {
            match &event {
                EditEvent::Begin { .. } => {
                    if let Some(Mixed::Same(shown)) = &out.shown {
                        *value.borrow_mut() = **shown;
                    }
                }
                EditEvent::Update { value: v, .. } | EditEvent::Commit { after: v, .. } => *value.borrow_mut() = *v,
                EditEvent::Cancel { before: Some(Mixed::Same(before)), .. } => *value.borrow_mut() = *before,
                _ => {}
            }
            events.borrow_mut().push(event);
        }
    };
    frames.idle(3, &mut show);
    click(&mut frames, at.get(), &mut show);
    frames.idle(2, &mut show);
    frames.frame(vec![key_press(Key::A, Modifiers::COMMAND)], &mut show);
    frames.frame(vec![Event::Text("7".into())], &mut show);
    assert_eq!(*value.borrow(), Some(TYPED.0), "the first typed character reaches the host in the pass that begins the edit");
    frames.idle(3, &mut show);
    assert!(!events.borrow().iter().any(|e| matches!(e, EditEvent::Cancel { .. })), "the host's write is the gesture's own: {:?}", events.borrow());
    frames.frame(vec![key_press(Key::Escape, Modifiers::NONE)], &mut show);
    frames.idle(3, &mut show);
    assert_eq!(*value.borrow(), Some(START), "Escape restores the start value");
}

/// A control drawn above a number field shows the typed value in the frame the typing happens, because the pass that
/// delivers an edit runs again before painting.
#[trial]
fn a_control_above_the_field_shows_the_typed_value_in_the_same_frame() {
    let mut frames = Frames::new();
    let value = RefCell::new(Some(START));
    let at = Cell::new(Pos2::ZERO);
    let above = Cell::new(None);
    let mut show = |ui: &mut Ui| {
        above.set(*value.borrow());
        let current = *value.borrow();
        let out = NumberField::new("Size", Bind::given(current)).id_salt("size").show(ui);
        at.set(out.response.rect.center());
        match &out.event {
            Some(EditEvent::Begin { .. }) => {
                if let Some(Mixed::Same(shown)) = &out.shown {
                    *value.borrow_mut() = **shown;
                }
            }
            Some(EditEvent::Update { value: v, .. } | EditEvent::Commit { after: v, .. }) => *value.borrow_mut() = *v,
            _ => {}
        }
    };
    frames.idle(3, &mut show);
    click(&mut frames, at.get(), &mut show);
    frames.idle(2, &mut show);
    frames.frame(vec![key_press(Key::A, Modifiers::COMMAND)], &mut show);
    frames.frame(vec![Event::Text("7".into())], &mut show);
    assert_eq!(above.get(), Some(TYPED.0), "the control above the field shows the first typed character in that frame");
    frames.frame(vec![Event::Text("5".into())], &mut show);
    assert_eq!(above.get(), Some(TYPED.1), "and the next one");
}

/// What a fixture of a number field with presets shows: the value, the field's id and the events it reported.
struct Shown {
    value: Rc<RefCell<Option<f64>>>,
    id: Rc<Cell<Option<egui::Id>>>,
    events: Rc<RefCell<Vec<EditEvent<Option<f64>>>>>,
    picked: Rc<Cell<Option<NumberPick>>>,
}

fn fixture_of(range: Option<RangeInclusive<f64>>, presets: Vec<Entry<NumberPick>>, slider: bool) -> (Fixture, Shown) {
    let shown = Shown { value: Rc::new(RefCell::new(Some(START))), id: Rc::default(), events: Rc::default(), picked: Rc::default() };
    let (value, id, events, picked) = (shown.value.clone(), shown.id.clone(), shown.events.clone(), shown.picked.clone());
    let mut fixture = Fixture::new(move |ui| {
        let mut guard = value.borrow_mut();
        let mut field = NumberField::new("Size", Bind::bound(&mut *guard)).id_salt("size");
        if let Some(range) = &range {
            field = field.range(range.clone());
            if slider {
                field = field.popup_slider(range.clone());
            }
        }
        if !presets.is_empty() {
            field = field.presets(&presets);
        }
        let out = field.show(ui);
        id.set(Some(out.response.id));
        events.borrow_mut().extend(out.event.clone());
        if out.preset.is_some() {
            picked.set(out.preset);
        }
    });
    fixture.settle();
    (fixture, shown)
}

fn presets() -> Vec<Entry<NumberPick>> {
    vec![Entry::new(NumberPick::Number(8.0), "Small"), Entry::new(NumberPick::Choice(3), "Auto"), Entry::new(NumberPick::Number(24.0), "Large")]
}

/// Alt+Down opens the presets list with focus staying in the field; Down then Enter picks the second row, and a
/// reopened list starts at its first row again. The chevron's node is named by its word and reports expanded.
#[trial]
fn presets_open_over_the_field_and_a_pick_sets_the_value() {
    let (mut fixture, shown) = fixture_of(None, presets(), false);
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    let field = shown.id.get().expect("the field showed");
    let chevron = |fixture: &Fixture| fixture.part(field, craft_behavior::PartKey::new("presets")).expect("the chevron has a node");
    assert_eq!(fixture.tree().expanded_of(chevron(&fixture).node), Some(false), "the closed list's chevron is expandable");
    fixture.key(Key::ArrowDown, Modifiers::ALT);
    fixture.idle(4);
    let typing = craft_ui::testing::text::widget_id(field, super::kit::TEXT);
    assert_eq!(fixture.focused(), Some(typing), "the list opens with focus still in the field's text, where typing goes");
    let target = chevron(&fixture);
    assert_eq!(fixture.tree().name_of(target.node), Some("Presets"), "the chevron is named by its word");
    assert_eq!(fixture.tree().expanded_of(target.node), Some(true), "the chevron reports the list open");
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(3);
    assert_eq!(*shown.value.borrow(), Some(START), "a choice row leaves the value");
    assert_eq!(shown.picked.get(), Some(NumberPick::Choice(3)), "the pick names the choice");
    fixture.key(Key::ArrowDown, Modifiers::ALT);
    fixture.idle(4);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(3);
    const FIRST: f64 = 8.0;
    assert_eq!(*shown.value.borrow(), Some(FIRST), "the reopened list starts at the first row, a number row");
}

/// One drag of the popup slider is one Begin and one Commit, not a set per frame.
#[trial]
fn a_popup_slider_drag_is_one_gesture() {
    let (mut fixture, shown) = fixture_of(Some(0.0..=100.0), Vec::new(), true);
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.key(Key::ArrowDown, Modifiers::ALT);
    fixture.idle(4);
    let slider = fixture.by_role(Role::SLIDER).into_iter().next().expect("the popup shows a slider");
    shown.events.borrow_mut().clear();
    fixture.drag(&slider, Vec2::new(30.0, 0.0));
    fixture.idle(2);
    let events = shown.events.borrow();
    let count = |wanted: fn(&EditEvent<Option<f64>>) -> bool| events.iter().filter(|event| wanted(event)).count();
    assert_eq!(count(|event| matches!(event, EditEvent::Begin { .. })), 1, "one Begin: {events:?}");
    assert_eq!(count(|event| matches!(event, EditEvent::Commit { .. })), 1, "one Commit: {events:?}");
    assert_eq!(count(|event| matches!(event, EditEvent::Set { .. })), 0, "no per-frame Set: {events:?}");
}

const ROW: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(310.0, 40.0) };
/// The window's right edge as the scrub's pointer reaches it, and the point the warp puts it at.
const EDGE_X: f32 = 799.0;
const LANDED_X: f32 = 4.0;
/// The label drag after the warp, in points.
const DRAG: f32 = 30.0;

#[trial]
fn a_wrapped_scrub_keeps_its_value_continuous() {
    let system = Arc::new(FakeSystem::new());
    let mut frames = Frames::uninstalled();
    let installed = craft_ui::install(&frames.ctx, InstallSpec::new(ShellInfo::default(), system.clone()));
    assert!(installed.is_ok(), "the context takes the runtime once");
    let mut value = Some(START);
    let show = |ui: &mut Ui, value: &mut Option<f64>| {
        Field::new("Size", SideLabel::new()).id_salt("warp").show_at(ui, ROW, |ui, _| {
            NumberField::new("Size", Bind::bound(&mut *value)).scrub_by_label().id_salt("scrubbed").show(ui);
        });
    };
    frames.idle(4, |ui| show(ui, &mut value));
    let from = ROW.min + Vec2::new(4.0, 12.0);
    let step = |frames: &mut Frames, value: &mut Option<f64>, events: Vec<Event>, at: Pos2| {
        system.set_pointer(Some([at.x, at.y]));
        frames.frame(events, |ui| show(ui, value));
    };
    step(&mut frames, &mut value, vec![moved(from)], from);
    step(&mut frames, &mut value, vec![press(from)], from);
    let along = from + Vec2::new(20.0, 0.0);
    step(&mut frames, &mut value, vec![moved(along)], along);
    let edge = Pos2::new(EDGE_X, from.y);
    step(&mut frames, &mut value, vec![moved(edge)], edge);
    assert_eq!(system.warps().len(), 1, "reaching the edge warps the pointer once");
    let before = value.unwrap_or_default();
    let landed = Pos2::new(LANDED_X, from.y);
    step(&mut frames, &mut value, vec![moved(landed)], landed);
    assert!((value.unwrap_or_default() - before).abs() < SAME, "the warp does not move the value: {before} -> {value:?}");
    let on = landed + Vec2::new(DRAG, 0.0);
    step(&mut frames, &mut value, vec![moved(on)], on);
    assert!(
        (value.unwrap_or_default() - before - f64::from(DRAG)).abs() < SAME,
        "the drag goes on from the value before the warp: {before} -> {value:?}"
    );
}

/// A field of `kind` (plain without one) under `host`'s locale, with `text` typed over its value and committed.
fn typed(host: Option<Arc<Host>>, kind: Option<UnitKind>, text: &str) -> Option<f64> {
    typed_with(host, text, |field| match kind {
        Some(kind) => field.unit_kind(kind),
        None => field,
    })
}

/// A field set up by `configure` under `host`'s locale, with `text` typed over its value and committed.
fn typed_with(host: Option<Arc<Host>>, text: &str, configure: impl for<'a> Fn(NumberField<'a>) -> NumberField<'a>) -> Option<f64> {
    let mut frames = Frames::new();
    if let Some(host) = host {
        frames.host(host);
    }
    let value = RefCell::new(Some(START));
    let at = Cell::new(Pos2::ZERO);
    let mut show = |ui: &mut Ui| {
        let mut guard = value.borrow_mut();
        let field = configure(NumberField::new("Size", Bind::bound(&mut *guard)).id_salt("size"));
        at.set(field.show(ui).response.rect.center());
    };
    frames.idle(3, &mut show);
    click(&mut frames, at.get(), &mut show);
    frames.frame(vec![key_press(Key::A, Modifiers::COMMAND)], &mut show);
    frames.frame(vec![Event::Text(text.into())], &mut show);
    frames.idle(1, &mut show);
    frames.frame(vec![key_press(Key::Enter, Modifiers::NONE)], &mut show);
    frames.idle(3, &mut show);
    value.into_inner()
}

/// Under a decimal-comma locale the comma is the decimal separator.
#[trial]
fn a_localised_decimal_types_in() {
    let (i18n, problems) = craft_i18n::I18n::from_owned([("de".to_owned(), String::new(), "@id\ttrial.save\tSpeichern\n".to_owned())], &["de"]);
    assert!(problems.is_empty(), "{problems:?}");
    i18n.set_format(craft_i18n::Format::from_tag("de"));
    let host = Arc::new(Host::builder(AppInfo { id: "trial", name: "Trial", version: "0" }).i18n(Arc::new(i18n)).build());
    assert_eq!(typed(Some(host), None, "12,5"), Some(12.5));
}

/// The field with a codec that reads "Auto" as -1.
fn auto(field: NumberField<'_>) -> NumberField<'_> {
    field.codec(NumberCodec::new(Reader::new(), 1).or_parse(|text| text.trim().eq_ignore_ascii_case("auto").then_some(-1.0)))
}
