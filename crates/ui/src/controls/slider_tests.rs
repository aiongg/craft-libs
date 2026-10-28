//! Slider and range-slider behaviour in a headless egui context: the edit gesture of a drag, Esc,
//! the keys and the wheel, the direction, and the range slider's thumbs.

use egui::{Context, Event, Key, Modifiers, MouseWheelSource, MouseWheelUnit, PointerButton, Pos2, RawInput, Rect, TouchPhase, Ui, pos2, vec2};

use super::slider::{Scale, Track};
use super::{ControlSize, RangeSlider, Slider};
use crate::direction::{self, Direction};
use crate::edit::{self, CancelReason, EditEvent, Ending, GestureId};
use crate::theme::{Base, Tokens, TokensOf, install};

const RANGE: (f64, f64) = (0.0, 100.0);

/// A host with one slider and one range slider in steps of 1 over 0 to 100. It applies every
/// value it is sent, and puts the start value back on a cancel when `revert` is on.
struct Rig {
    ctx: Context,
    time: f64,
    modifiers: Modifiers,
    level: f64,
    range: (f64, f64),
    revert: bool,
    /// The host values at the last Begin.
    before: (f64, (f64, f64)),
    tracks: Option<(Track, Track)>,
    slider_id: egui::Id,
    level_event: Option<EditEvent<f64>>,
    range_event: Option<EditEvent<(f64, f64)>>,
    /// The slider's numeric value in the last AccessKit update.
    reported: Option<f64>,
}

impl Rig {
    fn new() -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        ctx.enable_accesskit();
        let mut rig = Self {
            ctx,
            time: 0.0,
            modifiers: Modifiers::NONE,
            level: 40.0,
            range: (20.0, 60.0),
            revert: true,
            before: (40.0, (20.0, 60.0)),
            tracks: None,
            slider_id: egui::Id::NULL,
            level_event: None,
            range_event: None,
            reported: None,
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
        let output = ctx.run_ui(input, |ui| self.controls(ui));
        let update = output.platform_output.accesskit_update.clone();
        output.drop_without_applying_deltas();
        if let Some(update) = update {
            let id = self.slider_id.accesskit_id();
            self.reported = update.nodes.iter().find(|(n, _)| *n == id).and_then(|(_, node)| node.numeric_value());
        }
    }

    fn tracks(&self) -> (Track, Track) {
        self.tracks.unwrap_or_else(|| panic!("no frame has run"))
    }

    fn controls(&mut self, ui: &mut Ui) {
        let style = Slider::style(&Tokens::of(ui), ControlSize::Md);
        let scale = Scale::new(&(RANGE.0..=RANGE.1), Some(1.0));
        let edit = Slider::new("rig-slider", "Level", self.level, RANGE.0..=RANGE.1).step(1.0).show(ui);
        let slider = Track::new(edit.response.rect, &style, scale);
        self.slider_id = edit.response.id;
        self.level_event.clone_from(&edit.event);
        if let Some(event) = edit.event {
            self.apply(event, |rig| &mut rig.level, |rig| rig.before.0);
        }
        let edit = RangeSlider::new("rig-range", ("From", "To"), self.range, RANGE.0..=RANGE.1).step(1.0).show(ui);
        self.tracks = Some((slider, Track::new(edit.response.rect, &style, scale)));
        self.range_event.clone_from(&edit.event);
        if let Some(event) = edit.event {
            self.apply(event, |rig| &mut rig.range, |rig| rig.before.1);
        }
    }

    fn apply<T: Clone>(&mut self, event: EditEvent<T>, value: impl Fn(&mut Self) -> &mut T, before: impl Fn(&Self) -> T) {
        match event {
            EditEvent::Begin { .. } => self.before = (self.level, self.range),
            EditEvent::Cancel { .. } if self.revert => *value(self) = before(self),
            EditEvent::Cancel { .. } => {}
            other => {
                if let Some(v) = other.value() {
                    *value(self) = v.clone();
                }
            }
        }
    }

    /// The slider's thumb centre for `value`, moved `dx` points.
    fn level_at(&self, value: f64, dx: f32) -> Pos2 {
        let track = self.tracks().0;
        pos2(track.x_of(value) + dx, track.y)
    }

    fn range_at(&self, value: f64, dx: f32) -> Pos2 {
        let track = self.tracks().1;
        pos2(track.x_of(value) + dx, track.y)
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

    fn key(&mut self, key: Key) {
        let modifiers = self.modifiers;
        self.frame(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]);
    }

    fn focus(&mut self, id: egui::Id) {
        self.ctx.memory_mut(|m| m.request_focus(id));
        self.frame(vec![]);
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

#[test]
fn a_drag_begins_updates_and_commits_and_a_press_on_the_track_moves_the_thumb_there() {
    let mut rig = Rig::new();
    // On the thumb, a little off its centre: the value doesn't jump.
    rig.press(rig.level_at(40.0, 3.0));
    let id = begin(rig.level_event.clone());
    rig.drag(rig.level_at(50.0, 3.0));
    assert_eq!(rig.level_event, update(id, 50.0));
    // The press focused the slider, but a drag still ends as a cancel if the slider goes away.
    assert!(rig.ctx.memory(|m| m.has_focus(rig.slider_id)));
    assert_eq!(edit::ending(&rig.ctx, id), Ending::Cancel);
    rig.drag(rig.level_at(70.0, 3.0));
    assert_eq!(rig.level_event, update(id, 70.0));
    rig.release(rig.level_at(70.0, 3.0));
    assert_eq!(rig.level_event, commit(id, 70.0, true));
    assert_eq!(rig.level, 70.0);

    rig.press(rig.level_at(70.0, 0.0));
    let id = begin(rig.level_event.clone());
    rig.release(rig.level_at(70.0, 0.0));
    assert_eq!(rig.level_event, commit(id, 70.0, false), "a press without a move commits unchanged");

    // Away from the thumb: Begin, then the thumb at the pointer.
    rig.press(rig.level_at(20.0, 0.0));
    let id = begin(rig.level_event.clone());
    rig.frame(vec![]);
    assert_eq!(rig.level_event, update(id, 20.0));
    rig.release(rig.level_at(20.0, 0.0));
    assert_eq!(rig.level_event, commit(id, 20.0, true));
}

#[test]
fn esc_mid_drag_cancels_and_the_slider_shows_the_start_value_until_the_host_echoes_it() {
    let mut rig = Rig::new();
    rig.revert = false;
    rig.press(rig.level_at(40.0, 0.0));
    let id = begin(rig.level_event.clone());
    rig.drag(rig.level_at(65.0, 0.0));
    assert_eq!((rig.level_event.clone(), rig.reported), (update(id, 65.0), Some(65.0)));
    rig.key(Key::Escape);
    assert_eq!(rig.level_event, Some(EditEvent::Cancel { id, reason: CancelReason::Escape }));
    rig.frame(vec![]);
    assert_eq!((rig.level, rig.reported), (65.0, Some(40.0)), "the host still shows the drag; the slider shows the start");
    rig.level = 40.0;
    rig.drag(rig.level_at(80.0, 0.0));
    rig.release(rig.level_at(80.0, 0.0));
    assert_eq!((rig.level_event.clone(), rig.level, rig.reported), (None, 40.0, Some(40.0)), "the drag Esc ended goes no further");
}

#[test]
fn keys_and_the_wheel_each_give_one_set_with_their_modifiers() {
    let mut rig = Rig::new();
    let slider = rig.slider_id;
    rig.focus(slider);
    let expect = |rig: &mut Rig, event: Event, modifiers: Modifiers, value: f64| {
        rig.modifiers = modifiers;
        rig.frame(vec![event]);
        assert_eq!(rig.level_event, set(value, modifiers), "after {:?}", rig.level_event);
        rig.modifiers = Modifiers::NONE;
        rig.frame(vec![]);
        assert_eq!(rig.level_event, None, "one event per input");
    };
    let key = |key, modifiers| Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers };
    expect(&mut rig, key(Key::ArrowRight, Modifiers::NONE), Modifiers::NONE, 41.0);
    expect(&mut rig, key(Key::ArrowUp, Modifiers::NONE), Modifiers::NONE, 42.0);
    expect(&mut rig, key(Key::ArrowLeft, Modifiers::SHIFT), Modifiers::SHIFT, 32.0);
    expect(&mut rig, key(Key::ArrowDown, Modifiers::NONE), Modifiers::NONE, 31.0);
    expect(&mut rig, key(Key::PageUp, Modifiers::NONE), Modifiers::NONE, 41.0);
    expect(&mut rig, key(Key::PageDown, Modifiers::NONE), Modifiers::NONE, 31.0);
    expect(&mut rig, key(Key::End, Modifiers::NONE), Modifiers::NONE, 100.0);
    expect(&mut rig, key(Key::Home, Modifiers::NONE), Modifiers::NONE, 0.0);

    let over = rig.level_at(0.0, 0.0);
    rig.frame(vec![Event::PointerMoved(over)]);
    let wheel = |lines: f32, modifiers| Event::MouseWheel {
        unit: MouseWheelUnit::Line,
        delta: vec2(0.0, lines),
        phase: TouchPhase::Move,
        source: MouseWheelSource::Wheel,
        modifiers,
    };
    expect(&mut rig, wheel(2.0, Modifiers::NONE), Modifiers::NONE, 2.0);
    expect(&mut rig, wheel(-1.0, Modifiers::NONE), Modifiers::NONE, 1.0);
    expect(&mut rig, wheel(1.0, Modifiers::SHIFT), Modifiers::SHIFT, 11.0);
    assert!(rig.level_event.is_none());

    // Without focus the wheel is the panel's.
    rig.ctx.memory_mut(|m| m.surrender_focus(slider));
    rig.frame(vec![wheel(1.0, Modifiers::NONE)]);
    assert_eq!((rig.level_event.clone(), rig.level), (None, 11.0));
}

/// The value grows to the right in either direction (`Unmirrored::SliderValue`).
#[test]
fn right_to_left_keeps_the_drag_direction_and_the_arrow_keys() {
    let mut rig = Rig::new();
    direction::apply(&rig.ctx, Direction::Rtl);
    rig.frame(vec![]);
    assert!(rig.tracks().0.x_of(60.0) > rig.tracks().0.x_of(40.0));
    rig.press(rig.level_at(40.0, 0.0));
    let id = begin(rig.level_event.clone());
    rig.drag(rig.level_at(45.0, 0.0));
    assert_eq!(rig.level_event, update(id, 45.0));
    rig.release(rig.level_at(45.0, 0.0));
    rig.key(Key::ArrowRight);
    assert_eq!(rig.level_event, set(46.0, Modifiers::NONE));
}

#[test]
fn range_slider_thumbs_do_not_cross_and_a_drag_between_them_moves_both() {
    let mut rig = Rig::new();
    rig.press(rig.range_at(20.0, 0.0));
    let id = begin(rig.range_event.clone());
    rig.drag(rig.range_at(90.0, 0.0));
    assert_eq!(rig.range_event, update(id, (60.0, 60.0)), "the lower thumb stops at the upper one");
    assert_eq!(edit::ending(&rig.ctx, id), Ending::Cancel);
    rig.drag(rig.range_at(30.0, 0.0));
    assert_eq!(rig.range_event, update(id, (30.0, 60.0)));
    rig.release(rig.range_at(30.0, 0.0));
    assert_eq!(rig.range_event, commit(id, (30.0, 60.0), true));

    // Between the thumbs: both move, keeping their distance, and stop at the end.
    rig.press(rig.range_at(45.0, 0.0));
    let id = begin(rig.range_event.clone());
    let dx = rig.tracks().1.x_of(55.0) - rig.tracks().1.x_of(45.0);
    rig.drag(rig.range_at(45.0, dx));
    assert_eq!(rig.range_event, update(id, (40.0, 70.0)));
    rig.drag(rig.range_at(45.0, 10.0 * dx));
    assert_eq!(rig.range_event, update(id, (70.0, 100.0)));
    rig.release(rig.range_at(45.0, 10.0 * dx));
    assert_eq!(rig.range, (70.0, 100.0));

    // Beyond the upper thumb: it comes to the pointer, and the press focuses it.
    rig.range = (20.0, 60.0);
    rig.frame(vec![]);
    rig.press(rig.range_at(80.0, 0.0));
    let id = begin(rig.range_event.clone());
    rig.frame(vec![]);
    assert_eq!(rig.range_event, update(id, (20.0, 80.0)));
    rig.release(rig.range_at(80.0, 0.0));
    rig.key(Key::Home);
    assert_eq!(rig.range_event, set((20.0, 20.0), Modifiers::NONE), "Home takes the upper thumb to the lower");
    rig.key(Key::ArrowLeft);
    assert_eq!(rig.range_event, None, "the upper thumb can't pass the lower");
}

#[test]
fn where_the_thumbs_meet_the_drag_direction_picks_the_thumb() {
    let mut rig = Rig::new();
    rig.range = (50.0, 50.0);
    rig.frame(vec![]);
    rig.press(rig.range_at(50.0, 0.0));
    let id = begin(rig.range_event.clone());
    rig.drag(rig.range_at(60.0, 0.0));
    assert_eq!(rig.range_event, update(id, (50.0, 60.0)));
    rig.drag(rig.range_at(35.0, 0.0));
    assert_eq!(rig.range_event, update(id, (35.0, 50.0)));
    rig.release(rig.range_at(35.0, 0.0));
    assert_eq!(rig.range, (35.0, 50.0));
}
