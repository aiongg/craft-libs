//! Slider and range-slider behaviour in a headless egui context: the edit gesture of a drag, Esc,
//! the keys and the wheel, the direction, and the range slider's thumbs.

use egui::accesskit::{Action, ActionRequest, TreeId};
use egui::emath::TSTransform;
use egui::{
    Context, Event, Id, Key, Modifiers, MouseWheelSource, MouseWheelUnit, PointerButton, Pos2, RawInput, Rect, TouchPhase, Ui, Vec2, pos2, vec2,
};

use super::slider::{Scale, Track, bounds};
use super::{ControlSize, RangeSlider, Slider, SliderScale, SliderThumb};
use crate::direction::{self, Direction};
use crate::edit::{self, CancelReason, EditEvent, Ending, GestureHost, GestureId, UndoMarks};
use crate::primitives::Interactive;
use crate::theme::{Base, Tokens, TokensOf, install};

const RANGE: (f64, f64) = (0.0, 100.0);

/// A host with one slider and one range slider, in steps of 1 over 0 to 100 unless a test
/// changes them. It applies every value it is sent, and puts the start value back on a cancel
/// when `revert` is on.
struct Rig {
    ctx: Context,
    time: f64,
    modifiers: Modifiers,
    level: f64,
    range: (f64, f64),
    revert: bool,
    bounds: (f64, f64),
    step: Option<f64>,
    enabled: bool,
    default: Option<f64>,
    wheel_unfocused: bool,
    wheel: bool,
    kind: SliderScale,
    marks: Vec<f64>,
    thumb: SliderThumb,
    ticks: usize,
    /// The layer's transform: test positions are in the layer's coordinates.
    to_global: TSTransform,
    /// The range slider's thumbs.
    thumbs: (Id, Id),
    /// `smooth_scroll_delta` left after the controls: what a panel around them would scroll.
    scroll_left: Vec2,
    /// The host values at the last Begin.
    before: (f64, (f64, f64)),
    tracks: Option<(Track, Track)>,
    slider_id: egui::Id,
    level_event: Option<EditEvent<f64>>,
    range_event: Option<EditEvent<(f64, f64)>>,
    /// The slider's numeric value in the last AccessKit update.
    reported: Option<f64>,
    /// The range slider's thumbs' numeric values in the last AccessKit update.
    thumbs_reported: (Option<f64>, Option<f64>),
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
            bounds: RANGE,
            step: Some(1.0),
            enabled: true,
            default: None,
            wheel_unfocused: false,
            wheel: true,
            kind: SliderScale::Linear,
            marks: Vec::new(),
            thumb: SliderThumb::Round,
            ticks: 0,
            to_global: TSTransform::IDENTITY,
            thumbs: (Id::NULL, Id::NULL),
            scroll_left: Vec2::ZERO,
            before: (40.0, (20.0, 60.0)),
            tracks: None,
            slider_id: egui::Id::NULL,
            level_event: None,
            range_event: None,
            reported: None,
            thumbs_reported: (None, None),
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
            let value = |id: Id| update.nodes.iter().find(|(n, _)| *n == id.accesskit_id()).and_then(|(_, node)| node.numeric_value());
            self.reported = value(self.slider_id);
            self.thumbs_reported = (value(self.thumbs.0), value(self.thumbs.1));
        }
    }

    fn tracks(&self) -> (Track, Track) {
        self.tracks.unwrap_or_else(|| panic!("no frame has run"))
    }

    fn controls(&mut self, ui: &mut Ui) {
        ui.ctx().set_transform_layer(ui.layer_id(), self.to_global);
        let style = Slider::style(&Tokens::of(ui), ControlSize::Md);
        let range = self.bounds.0..=self.bounds.1;
        let scale = Scale::new(&range, self.step).with_kind(self.kind);
        let mut slider = Slider::new("rig-slider", "Level", self.level, range.clone())
            .enabled(self.enabled)
            .wheel_unfocused(self.wheel_unfocused)
            .wheel(self.wheel)
            .scale(self.kind)
            .marks(&self.marks)
            .thumb(self.thumb)
            .ticks(self.ticks);
        if let Some(default) = self.default {
            slider = slider.default_value(default);
        }
        let mut range_slider = RangeSlider::new("rig-range", ("From", "To"), self.range, range).enabled(self.enabled);
        if let Some(step) = self.step {
            (slider, range_slider) = (slider.step(step), range_slider.step(step));
        }
        let edit = slider.show(ui);
        let slider = Track::new(edit.response.rect, &style, scale);
        self.slider_id = edit.response.id;
        self.level_event.clone_from(&edit.event);
        if let Some(event) = edit.event {
            self.apply(event, |rig| &mut rig.level, |rig| rig.before.0);
        }
        let edit = range_slider.show(ui);
        let range_style = super::range_slider::range_style(&Tokens::of(ui), ControlSize::Md);
        self.tracks = Some((slider, Track::new(edit.response.rect, &range_style, scale)));
        let id = edit.response.id;
        self.thumbs = (Interactive::id_in(ui, id.with("lower")), Interactive::id_in(ui, id.with("upper")));
        self.range_event.clone_from(&edit.event);
        if let Some(event) = edit.event {
            self.apply(event, |rig| &mut rig.range, |rig| rig.before.1);
        }
        self.scroll_left = ui.input(|i| i.smooth_scroll_delta);
    }

    fn has_focus(&self, id: Id) -> bool {
        self.ctx.memory(|m| m.has_focus(id))
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
        let pos = self.to_global * pos;
        self.frame(vec![Event::PointerMoved(pos), button(pos, true)]);
    }

    fn drag(&mut self, pos: Pos2) {
        let pos = self.to_global * pos;
        self.frame(vec![Event::PointerMoved(pos)]);
    }

    fn release(&mut self, pos: Pos2) {
        let pos = self.to_global * pos;
        self.frame(vec![button(pos, false)]);
    }

    /// A press and release in one frame.
    fn tap(&mut self, pos: Pos2) {
        let pos = self.to_global * pos;
        self.frame(vec![Event::PointerMoved(pos), button(pos, true), button(pos, false)]);
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
        Some(EditEvent::Begin { id, .. }) => id,
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
    assert!(rig.has_focus(rig.thumbs.1), "the focus goes to the thumb that moves");
    rig.drag(rig.range_at(35.0, 0.0));
    assert_eq!(rig.range_event, update(id, (35.0, 50.0)));
    assert!(rig.has_focus(rig.thumbs.0));
    rig.release(rig.range_at(35.0, 0.0));
    assert_eq!(rig.range, (35.0, 50.0));
    assert!(rig.has_focus(rig.thumbs.0));
}

#[test]
fn a_still_press_on_a_thumb_leaves_an_inexact_value_as_it_is() {
    let mut rig = Rig::new();
    rig.step = None;
    (rig.level, rig.range) = (33.3, (20.3, 60.7));
    rig.frame(vec![]);
    rig.press(rig.level_at(33.3, 3.0));
    let id = begin(rig.level_event.clone());
    rig.frame(vec![]);
    assert_eq!(rig.level_event, None, "no update without a move");
    rig.release(rig.level_at(33.3, 3.0));
    assert_eq!(rig.level_event, commit(id, 33.3, false));

    for at in [20.3, 40.0, 60.7] {
        rig.press(rig.range_at(at, 2.0));
        let id = begin(rig.range_event.clone());
        rig.frame(vec![]);
        assert_eq!(rig.range_event, None, "no update without a move from {at}");
        rig.release(rig.range_at(at, 2.0));
        assert_eq!(rig.range_event, commit(id, (20.3, 60.7), false));
    }
}

#[test]
fn range_bounds_that_are_not_finite_fall_back_to_the_finite_bound() {
    assert_eq!(bounds(100.0, 0.0), (0.0, 100.0));
    assert_eq!(bounds(f64::NAN, 5.0), (5.0, 5.0));
    assert_eq!(bounds(-3.0, f64::INFINITY), (-3.0, -3.0));
    assert_eq!(bounds(f64::NEG_INFINITY, f64::NAN), (0.0, 0.0));
    let scale = Scale::new(&(0.0..=100.0), Some(1.0));
    assert_eq!([scale.place(f64::NAN), scale.place(f64::INFINITY), scale.place(150.0)], [0.0, 0.0, 100.0]);
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "finite bounds")]
fn a_range_bound_that_is_not_finite_is_a_debug_assertion() {
    let mut rig = Rig::new();
    rig.bounds = (0.0, f64::NAN);
    rig.frame(vec![]);
}

/// A value that isn't a finite number draws at the start and starts no drag; keys, taps and
/// screen-reader requests give the host a finite value from there.
#[test]
fn a_value_that_is_not_finite_draws_at_the_start_and_begins_no_drag() {
    let mut rig = Rig::new();
    for value in [f64::NAN, f64::INFINITY] {
        rig.level = value;
        rig.frame(vec![]);
        assert_eq!(rig.reported, None, "mixed to a screen reader");
        rig.press(rig.level_at(0.0, 0.0));
        rig.drag(rig.level_at(30.0, 0.0));
        assert_eq!(rig.level_event, None);
        rig.release(rig.level_at(30.0, 0.0));
        assert_eq!(rig.level_event, None);
        rig.key(Key::ArrowRight);
        assert_eq!(rig.level_event, set(1.0, Modifiers::NONE));
    }
    rig.range = (f64::NAN, 50.0);
    rig.frame(vec![]);
    rig.press(rig.range_at(30.0, 0.0));
    rig.drag(rig.range_at(35.0, 0.0));
    assert_eq!(rig.range_event, None);
    rig.release(rig.range_at(35.0, 0.0));
}

#[test]
fn a_range_with_a_value_that_is_not_finite_reports_both_thumbs_mixed() {
    let mut rig = Rig::new();
    assert_eq!(rig.thumbs_reported, (Some(20.0), Some(60.0)));
    rig.range = (f64::NAN, 50.0);
    rig.frame(vec![]);
    assert_eq!(rig.thumbs_reported, (None, None));
}

#[test]
fn disabling_a_slider_mid_drag_cancels_the_drag() {
    let mut rig = Rig::new();
    rig.press(rig.level_at(40.0, 0.0));
    let id = begin(rig.level_event.clone());
    rig.drag(rig.level_at(60.0, 0.0));
    rig.press(rig.range_at(20.0, 0.0));
    rig.enabled = false;
    rig.drag(rig.level_at(70.0, 0.0));
    assert_eq!(rig.level_event, Some(EditEvent::Cancel { id, reason: CancelReason::Disabled }));
    assert_eq!(rig.level, 40.0);
    rig.release(rig.level_at(70.0, 0.0));
    assert_eq!(rig.level_event, None);
}

#[test]
fn a_tap_sets_the_value_there_and_focuses_the_control() {
    let mut rig = Rig::new();
    rig.tap(rig.level_at(70.0, 0.0));
    assert_eq!(rig.level_event, set(70.0, Modifiers::NONE));
    assert!(rig.has_focus(rig.slider_id));
    rig.tap(rig.level_at(70.0, 2.0));
    assert_eq!(rig.level_event, None, "a tap on the thumb leaves the value");

    rig.tap(rig.range_at(80.0, 0.0));
    assert_eq!(rig.range_event, set((20.0, 80.0), Modifiers::NONE));
    assert!(rig.has_focus(rig.thumbs.1));
    rig.frame(vec![]);
    rig.key(Key::ArrowRight);
    assert_eq!(rig.range_event, set((20.0, 81.0), Modifiers::NONE));
}

#[test]
fn the_thumb_a_press_focused_keeps_the_focus_after_the_release() {
    let mut rig = Rig::new();
    // Between the thumbs, nearer the lower one: a click away from both.
    rig.press(rig.range_at(35.0, 0.0));
    let id = begin(rig.range_event.clone());
    rig.release(rig.range_at(35.0, 0.0));
    assert_eq!(rig.range_event, commit(id, (20.0, 60.0), false));
    assert!(rig.has_focus(rig.thumbs.0));
    rig.key(Key::ArrowRight);
    assert_eq!(rig.range_event, set((21.0, 60.0), Modifiers::NONE));
}

#[test]
fn keys_the_wheel_and_screen_reader_requests_do_nothing_during_a_drag() {
    let mut rig = Rig::new();
    rig.press(rig.level_at(40.0, 0.0));
    let id = begin(rig.level_event.clone());
    rig.key(Key::End);
    let increment = Event::AccessKitActionRequest(ActionRequest {
        action: Action::Increment,
        target_tree: TreeId::ROOT,
        target_node: rig.slider_id.accesskit_id(),
        data: None,
    });
    rig.frame(vec![increment]);
    rig.frame(vec![wheel(MouseWheelUnit::Line, 2.0)]);
    assert_eq!(rig.level_event, None);
    rig.release(rig.level_at(40.0, 0.0));
    assert_eq!(rig.level_event, commit(id, 40.0, false));
}

#[test]
fn a_drag_and_a_tap_in_a_scaled_layer_follow_the_pointer() {
    let mut rig = Rig::new();
    rig.to_global = TSTransform::from_scaling(2.0);
    rig.frame(vec![]);
    rig.press(rig.level_at(40.0, 3.0));
    let id = begin(rig.level_event.clone());
    rig.drag(rig.level_at(55.0, 3.0));
    assert_eq!(rig.level_event, update(id, 55.0));
    rig.release(rig.level_at(55.0, 3.0));
    rig.tap(rig.level_at(80.0, 0.0));
    assert_eq!(rig.level_event, set(80.0, Modifiers::NONE));
    rig.press(rig.range_at(40.0, 0.0));
    let id = begin(rig.range_event.clone());
    let dx = rig.tracks().1.x_of(50.0) - rig.tracks().1.x_of(40.0);
    rig.drag(rig.range_at(40.0, dx));
    assert_eq!(rig.range_event, update(id, (30.0, 70.0)));
}

#[test]
fn the_range_end_is_a_stop_when_the_step_does_not_divide_the_range() {
    let mut rig = Rig::new();
    (rig.bounds, rig.step, rig.level) = ((0.0, 10.0), Some(3.0), 9.0);
    rig.frame(vec![]);
    let slider = rig.slider_id;
    rig.focus(slider);
    let increment = |rig: &Rig| {
        Event::AccessKitActionRequest(ActionRequest {
            action: Action::Increment,
            target_tree: TreeId::ROOT,
            target_node: rig.slider_id.accesskit_id(),
            data: None,
        })
    };
    rig.frame(vec![increment(&rig)]);
    assert_eq!(rig.level_event, set(10.0, Modifiers::NONE), "an increment inside the bounds changes the value");
    rig.key(Key::ArrowLeft);
    assert_eq!(rig.level_event, set(9.0, Modifiers::NONE));
    rig.key(Key::ArrowLeft);
    assert_eq!(rig.level_event, set(6.0, Modifiers::NONE));
    rig.key(Key::End);
    assert_eq!(rig.level_event, set(10.0, Modifiers::NONE));
    let scale = Scale::new(&(0.0..=10.0), Some(3.0));
    assert_eq!([9.4, 9.6, 1.4].map(|v| scale.snap(v)), [9.0, 10.0, 0.0]);
    assert_eq!(Scale::new(&(0.0..=1.0), Some(0.1)).snap(0.3), 0.3, "a value on a stop stays as it is");
}

#[test]
fn the_wheel_takes_its_whole_notch_and_drops_a_fraction_once_the_pointer_leaves() {
    let mut rig = Rig::new();
    let slider = rig.slider_id;
    rig.focus(slider);
    let (over, away) = (rig.level_at(40.0, 0.0), pos2(700.0, 500.0));
    let line = rig.ctx.options(|o| o.input_options.line_scroll_speed);
    rig.drag(over);
    rig.frame(vec![wheel(MouseWheelUnit::Line, 1.0)]);
    assert_eq!(rig.level_event, set(41.0, Modifiers::NONE));
    // egui spreads the notch over frames; the rest isn't the panel's after the pointer leaves.
    rig.drag(away);
    assert_eq!(rig.scroll_left, Vec2::ZERO);
    for _ in 0..30 {
        rig.frame(vec![]);
        assert_eq!(rig.scroll_left, Vec2::ZERO);
    }

    // Half a line, away and back, half a line: the first half is dropped.
    rig.drag(over);
    rig.frame(vec![wheel(MouseWheelUnit::Point, line / 2.0)]);
    assert_eq!(rig.level_event, None);
    rig.drag(away);
    rig.drag(over);
    rig.frame(vec![wheel(MouseWheelUnit::Point, line / 2.0)]);
    assert_eq!(rig.level_event, None);
    rig.frame(vec![wheel(MouseWheelUnit::Point, line / 2.0)]);
    assert_eq!(rig.level_event, set(42.0, Modifiers::NONE));
}

fn wheel(unit: MouseWheelUnit, delta: f32) -> Event {
    Event::MouseWheel { unit, delta: vec2(0.0, delta), phase: TouchPhase::Move, source: MouseWheelSource::Wheel, modifiers: Modifiers::NONE }
}

/// PhotoCraft's dialog sliders: with `wheel_unfocused` a gesture that starts over the slider
/// steps it without focus; one that started elsewhere keeps scrolling over it.
#[test]
fn an_unfocused_wheel_steps_the_slider_a_gesture_starts_over_when_asked() {
    let mut rig = Rig::new();
    let over = rig.level_at(40.0, 0.0);
    rig.frame(vec![Event::PointerMoved(over)]);
    rig.frame(vec![wheel(MouseWheelUnit::Line, 1.0)]);
    assert_eq!((rig.level_event.clone(), rig.level, rig.has_focus(rig.slider_id)), (None, 40.0, false), "off by default: the panel's");

    rig.wheel_unfocused = true;
    rig.time += 1.0;
    rig.frame(vec![wheel(MouseWheelUnit::Line, 1.0)]);
    assert_eq!((rig.level_event.clone(), rig.scroll_left), (set(41.0, Modifiers::NONE), Vec2::ZERO));
    rig.frame(vec![]);
    assert!(!rig.has_focus(rig.slider_id));

    // A gesture that starts off the slider scrolls on over it.
    rig.time += 1.0;
    rig.frame(vec![Event::PointerMoved(over + vec2(0.0, 80.0))]);
    rig.frame(vec![wheel(MouseWheelUnit::Line, 1.0)]);
    rig.frame(vec![Event::PointerMoved(over), wheel(MouseWheelUnit::Line, 1.0)]);
    assert_eq!(rig.level, 41.0);
}

/// A double-click on the track or the thumb resets; so does Ctrl/Cmd+Backspace while focused. A
/// press on the thumb doesn't move it.
#[test]
fn a_double_click_or_ctrl_backspace_sets_the_default_value() {
    let mut rig = Rig::new();
    rig.default = Some(25.0);
    rig.press(rig.level_at(40.0, 3.0));
    rig.release(rig.level_at(40.0, 3.0));
    assert_eq!(rig.level, 40.0, "a press on the thumb doesn't jump");
    rig.press(rig.level_at(40.0, 3.0));
    let id = begin(rig.level_event.clone());
    rig.release(rig.level_at(40.0, 3.0));
    assert_eq!((rig.level_event.clone(), rig.level), (commit(id, 25.0, true), 25.0), "the second click on the thumb resets: its gesture commits 25");

    rig.level = 70.0;
    rig.frame(vec![]);
    rig.time += 1.0;
    rig.tap(rig.level_at(90.0, 0.0));
    assert_eq!(rig.level, 90.0, "one click on the track moves the thumb");
    rig.tap(rig.level_at(90.0, 0.0));
    assert_eq!((rig.level_event.clone(), rig.level), (set(25.0, Modifiers::NONE), 25.0), "a double-click on the track");

    rig.level = 60.0;
    rig.frame(vec![]);
    rig.modifiers = Modifiers::COMMAND;
    rig.key(Key::Backspace);
    assert_eq!(rig.level_event, set(25.0, Modifiers::COMMAND), "the key resets the focused slider");
    rig.modifiers = Modifiers::NONE;

    // Keys change nothing during a drag.
    rig.time += 1.0;
    rig.press(rig.level_at(25.0, 0.0));
    let id = begin(rig.level_event.clone());
    rig.drag(rig.level_at(25.0, 40.0));
    let dragged = rig.level;
    assert!(dragged > 25.0 && rig.has_focus(rig.slider_id), "{dragged}");
    rig.modifiers = Modifiers::COMMAND;
    rig.key(Key::Backspace);
    rig.modifiers = Modifiers::NONE;
    assert_eq!((rig.level_event.clone(), rig.level), (None, dragged), "the reset key during the drag");
    rig.release(rig.level_at(25.0, 40.0));
    assert_eq!(rig.level_event, commit(id, dragged, true));
}

/// The meshes and the fill-coloured rects a slider with `stops` paints, and its track.
fn gradient_slider(stops: [craft_theme::Srgba; 2]) -> (Vec<egui::Mesh>, usize, Option<Track>) {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let (mut track, mut fill) = (None, egui::Color32::TRANSPARENT);
    let output = ctx.run_ui(RawInput::default(), |ui| {
        let style = Slider::style(&Tokens::of(ui), ControlSize::Md);
        fill = style.fill.rest;
        let rect = Slider::new("hue", "Hue", 50.0, 0.0..=100.0).track_colors(stops).show(ui).response.rect;
        track = Some(Track::new(rect, &style, Scale::new(&(0.0..=100.0), None)));
    });
    let (mut meshes, mut fills) = (Vec::new(), 0);
    for clipped in &output.shapes {
        match &clipped.shape {
            egui::Shape::Mesh(mesh) => meshes.push((**mesh).clone()),
            egui::Shape::Rect(r) if r.fill == fill => fills += 1,
            _ => {}
        }
    }
    output.drop_without_applying_deltas();
    (meshes, fills, track)
}

#[test]
fn track_colors_draw_the_gradient_under_the_thumb_and_no_fill() {
    let (red, blue) = (craft_theme::Srgba::rgb(1.0, 0.0, 0.0), craft_theme::Srgba::rgb(0.0, 0.0, 1.0));
    let (meshes, fills, track) = gradient_slider([red, blue]);
    let track = track.expect("the slider was shown");
    assert_eq!((meshes.len(), fills), (1, 0), "one gradient, no filled part");
    let colour_at = |x: f32| meshes[0].vertices.iter().find(|v| (v.pos.x - x).abs() < 1e-3).map(|v| v.color);
    let (start, end) = track.ends();
    assert_eq!((colour_at(start), colour_at(end)), (Some(red.into()), Some(blue.into())), "the stops lie where the thumb's centre is at the ends");
    assert!(meshes[0].vertices.iter().all(|v| (start - 0.01..=end + 0.01).contains(&v.pos.x)), "the gradient spans the track and no more");

    let (meshes, _, _) = gradient_slider([red.with_alpha(0.0), red]);
    assert_eq!(meshes.len(), 2, "a translucent stop shows the checkerboard under the gradient");
}

/// The hosted tests' document: the slider's level, the range slider's range, and a value an
/// options bar sets.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Values {
    level: f64,
    range: (f64, f64),
    other: i32,
}

/// A document whose undo history holds whole snapshots.
struct Doc {
    now: Values,
    undo: Vec<Values>,
}

impl Doc {
    fn apply(&mut self, edit: impl FnOnce(&mut Values)) {
        self.undo.push(self.now);
        edit(&mut self.now);
    }
}

impl UndoMarks for Doc {
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

/// The slider and the range slider behind a [`GestureHost`], with an options bar drawn after
/// them that sets a value.
struct Hosted {
    ctx: Context,
    doc: Doc,
    host: GestureHost<Doc>,
    tracks: Option<(Track, Track)>,
    level_event: Option<EditEvent<f64>>,
    range_event: Option<EditEvent<(f64, f64)>>,
}

/// What the options bar sets in a frame.
#[derive(Clone, Copy)]
enum Bar {
    Other(i32),
    Level(f64),
    Upper(f64),
}

impl Hosted {
    fn new() -> Self {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        let doc = Doc { now: Values { level: 40.0, range: (20.0, 60.0), other: 0 }, undo: Vec::new() };
        let mut hosted = Self { ctx, doc, host: GestureHost::new(), tracks: None, level_event: None, range_event: None };
        hosted.frame(vec![], None);
        hosted
    }

    fn frame(&mut self, events: Vec<Event>, bar: Option<Bar>) {
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), events, ..Default::default() };
        let ctx = self.ctx.clone();
        ctx.run_ui(input, |ui| {
            let style = Slider::style(&Tokens::of(ui), ControlSize::Md);
            let scale = Scale::new(&(RANGE.0..=RANGE.1), Some(1.0));
            let level = Slider::new("hosted-slider", "Level", self.doc.now.level, RANGE.0..=RANGE.1).step(1.0).show(ui);
            let range = RangeSlider::new("hosted-range", ("From", "To"), self.doc.now.range, RANGE.0..=RANGE.1).step(1.0).show(ui);
            let range_style = super::range_slider::range_style(&Tokens::of(ui), ControlSize::Md);
            self.tracks = Some((Track::new(level.response.rect, &style, scale), Track::new(range.response.rect, &range_style, scale)));
            (self.level_event, self.range_event) = (level.event.clone(), range.event.clone());
            if let Some(event) = level.event {
                self.host.handle(&ctx, &mut self.doc, event, |doc: &mut Doc, v| doc.apply(|now| now.level = v));
            }
            if let Some(event) = range.event {
                self.host.handle(&ctx, &mut self.doc, event, |doc: &mut Doc, v| doc.apply(|now| now.range = v));
            }
            match bar {
                Some(Bar::Other(v)) => {
                    self.host.handle(&ctx, &mut self.doc, EditEvent::set(v, Modifiers::NONE), |doc: &mut Doc, v| doc.apply(|now| now.other = v))
                }
                Some(Bar::Level(v)) => {
                    self.host.handle(&ctx, &mut self.doc, EditEvent::set(v, Modifiers::NONE), |doc: &mut Doc, v| doc.apply(|now| now.level = v))
                }
                Some(Bar::Upper(v)) => {
                    self.host.handle(&ctx, &mut self.doc, EditEvent::set(v, Modifiers::NONE), |doc: &mut Doc, v| doc.apply(|now| now.range.1 = v))
                }
                None => {}
            }
            self.host.end_frame(&ctx, &mut self.doc);
        })
        .drop_without_applying_deltas();
    }

    fn at(&self, range: bool, value: f64, dx: f32) -> Pos2 {
        let (level, range_track) = self.tracks.unwrap_or_else(|| panic!("no frame has run"));
        let track = if range { range_track } else { level };
        pos2(track.x_of(value) + dx, track.y)
    }
}

/// A drag whose gesture the host ends for an options bar's change goes on with the pointer: the
/// next movement begins a new gesture with the value the pointer gives, counted from the press
/// once (the host reverted the drag), the release commits it, and a thumb grabbed off its centre
/// stays under the pointer also when the bar set the dragged value itself. The range slider's
/// thumb the drag doesn't move keeps the host's value, also one the bar set.
#[test]
fn a_drag_the_host_ended_for_another_controls_change_goes_on_with_the_pointer() {
    for set_level in [false, true] {
        let mut rig = Hosted::new();
        rig.frame(vec![Event::PointerMoved(rig.at(false, 40.0, 3.0)), button(rig.at(false, 40.0, 3.0), true)], None);
        let first = begin(rig.level_event.clone());
        rig.frame(vec![Event::PointerMoved(rig.at(false, 50.0, 3.0))], None);
        assert_eq!(rig.level_event, update(first, 50.0));
        let bar = if set_level { Bar::Level(10.0) } else { Bar::Other(9) };
        rig.frame(vec![], Some(bar));
        let reverted = if set_level { 10.0 } else { 40.0 };
        assert_eq!((rig.doc.now.level, rig.host.active()), (reverted, None));
        rig.frame(vec![Event::PointerMoved(rig.at(false, 70.0, 3.0))], None);
        let id = begin(rig.level_event.clone());
        assert!(id > first);
        rig.frame(vec![], None);
        assert_eq!(rig.level_event, update(id, 70.0), "set_level: {set_level}");
        rig.frame(vec![button(rig.at(false, 70.0, 3.0), false)], None);
        assert_eq!(rig.level_event, commit(id, 70.0, true));
        assert_eq!(rig.doc.now.level, 70.0);
    }

    for (bar, upper, other) in [(Bar::Other(9), 60.0, 9), (Bar::Upper(80.0), 80.0, 0)] {
        let mut rig = Hosted::new();
        rig.frame(vec![Event::PointerMoved(rig.at(true, 20.0, 2.0)), button(rig.at(true, 20.0, 2.0), true)], None);
        let first = begin(rig.range_event.clone());
        rig.frame(vec![Event::PointerMoved(rig.at(true, 30.0, 2.0))], None);
        assert_eq!(rig.range_event, update(first, (30.0, 60.0)));
        rig.frame(vec![], Some(bar));
        assert_eq!((rig.doc.now.range, rig.host.active()), ((20.0, upper), None));
        rig.frame(vec![Event::PointerMoved(rig.at(true, 45.0, 2.0))], None);
        let id = begin(rig.range_event.clone());
        rig.frame(vec![], None);
        assert_eq!(rig.range_event, update(id, (45.0, upper)), "the upper thumb keeps the host's value");
        rig.frame(vec![button(rig.at(true, 45.0, 2.0), false)], None);
        assert_eq!(rig.range_event, commit(id, (45.0, upper), true));
        assert_eq!((rig.doc.now, rig.doc.undo.len()), (Values { level: 40.0, range: (45.0, upper), other }, 2));
    }
}

fn md_style() -> super::SliderStyle {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut style = None;
    let output = ctx.run_ui(RawInput::default(), |ui| style = Some(Slider::style(&Tokens::of(ui), ControlSize::Md)));
    output.drop_without_applying_deltas();
    style.expect("the frame ran")
}

#[test]
fn a_power_scale_places_the_thumb_and_inverts_for_every_value() {
    let range = 0.0..=100.0;
    let scale = Scale::new(&range, None).with_kind(SliderScale::Power(2.0));
    let track = Track::new(Rect::from_min_size(pos2(10.0, 0.0), vec2(200.0, 24.0)), &md_style(), scale);
    let (x0, x1) = track.ends();
    assert!((track.x_of(25.0) - (x0 + x1) / 2.0).abs() < 1e-3, "a quarter of the range is half the track at k = 2");
    for i in 0..=1000 {
        let value = f64::from(i) / 10.0;
        assert!((track.value_at(track.x_of(value)) - value).abs() < 1e-3, "{value}");
        assert!((scale.at_position(scale.position(value)) - value).abs() < 1e-9, "{value}");
    }
    // The pointer moves the thumb along the track: the same movement is a smaller change low down.
    let low = track.dragged(1.0, track.x_of(1.0), track.x_of(1.0) + 5.0) - 1.0;
    let high = track.dragged(80.0, track.x_of(80.0), track.x_of(80.0) + 5.0) - 80.0;
    assert!(0.0 < low && low < high, "{low} {high}");

    // Only the track is curved: keys and Page move the value linearly, as PhotoCraft's steps do.
    assert_eq!((scale.key_step(), scale.page_step(), scale.moved(10.0, scale.key_step())), (1.0, 10.0, 11.0));

    let mut rig = Rig::new();
    (rig.step, rig.kind, rig.level) = (None, SliderScale::Power(2.0), 25.0);
    rig.frame(vec![]);
    rig.tap(rig.level_at(49.0, 0.0));
    let Some(EditEvent::Set { value, .. }) = rig.level_event else { panic!("{:?}", rig.level_event) };
    assert!((value - 49.0).abs() < 0.5, "a tap sets the value under the pointer: {value}");
}

#[test]
fn a_log_scale_round_trips() {
    let range = 1.0..=1000.0;
    let scale = Scale::new(&range, None).with_kind(SliderScale::Log);
    assert!((scale.position(10.0) - 1.0 / 3.0).abs() < 1e-9);
    for v in [1.0, 1.5, 10.0, 99.0, 1000.0] {
        assert!((scale.at_position(scale.position(v)) - v).abs() < 1e-9 * v, "{v}");
    }
    // An arrow key is a tenth of a decade of the current value; Page is a tenth of the track.
    let up = scale.snap(scale.moved(10.0, scale.key_step()));
    assert!((up / 10.0 - 10f64.powf(0.1)).abs() < 1e-9, "{up}");
    let page = scale.moved(10.0, scale.page_step());
    assert!((scale.position(page) - (1.0 / 3.0 + 0.1)).abs() < 1e-9);

    let mut rig = Rig::new();
    (rig.bounds, rig.step, rig.kind, rig.level) = ((1.0, 1000.0), None, SliderScale::Log, 10.0);
    rig.frame(vec![]);
    rig.focus(rig.slider_id);
    rig.key(Key::ArrowRight);
    let Some(EditEvent::Set { value, .. }) = rig.level_event else { panic!("{:?}", rig.level_event) };
    assert!((value / 10.0 - 10f64.powf(0.1)).abs() < 1e-9, "{value}");
}

#[test]
fn a_drag_near_a_mark_snaps_to_it() {
    let mut rig = Rig::new();
    (rig.marks, rig.step, rig.level) = (vec![0.0, 25.0, 50.0], None, 40.0);
    rig.frame(vec![]);
    let snap = md_style().snap;
    let (near, far) = (rig.level_at(50.0, snap - 1.0), rig.level_at(50.0, snap + 3.0));
    rig.press(rig.level_at(40.0, 0.0));
    let id = begin(rig.level_event.clone());
    rig.drag(near);
    assert_eq!(rig.level_event, update(id, 50.0), "within the snap distance the thumb takes the mark");
    rig.drag(far);
    let Some(EditEvent::Update { value, .. }) = rig.level_event else { panic!("{:?}", rig.level_event) };
    assert!(value > 50.0 && value != 50.0, "beyond it the thumb follows the pointer: {value}");
    rig.release(far);
    // Grabbed 4 points off its centre, the thumb is within the snap distance of the mark while
    // the pointer is beyond it: the pointer decides.
    rig.level = 40.0;
    rig.frame(vec![]);
    let grab = 4.0;
    rig.press(rig.level_at(40.0, grab));
    let id = begin(rig.level_event.clone());
    rig.drag(rig.level_at(50.0, snap + 0.5));
    let Some(EditEvent::Update { value, .. }) = rig.level_event else { panic!("{:?}", rig.level_event) };
    assert_ne!(value, 50.0, "the pointer is beyond the snap distance although the thumb is within it");
    rig.drag(rig.level_at(50.0, snap - 1.0));
    assert_eq!(rig.level_event, update(id, 50.0));
    rig.release(rig.level_at(50.0, snap - 1.0));
    rig.tap(rig.level_at(25.0, -2.0));
    assert_eq!(rig.level_event, set(25.0, Modifiers::NONE), "a click near a mark snaps too");
}

/// The x of the line segments in `colour` that a frame of the slider paints, left to right.
fn marks_drawn(rig: &mut Rig, colour: egui::Color32) -> Vec<f32> {
    let ctx = rig.ctx.clone();
    let events = vec![Event::ModifiersChanged(rig.modifiers)];
    let input =
        RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), time: Some(rig.time), events, ..Default::default() };
    let output = ctx.run_ui(input, |ui| rig.controls(ui));
    let mut xs: Vec<f32> = output
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            egui::Shape::LineSegment { points, stroke } if stroke.color == colour => Some(points[0].x),
            _ => None,
        })
        .collect();
    output.drop_without_applying_deltas();
    xs.sort_by(f32::total_cmp);
    xs
}

#[test]
fn ticks_draw_count_plus_one_marks() {
    let mut rig = Rig::new();
    let colour = md_style().mark_color;
    assert_eq!(marks_drawn(&mut rig, colour), Vec::<f32>::new());
    rig.ticks = 4;
    let ticks = marks_drawn(&mut rig, colour);
    assert_eq!(ticks.len(), 5);
    let step = (ticks[4] - ticks[0]) / 4.0;
    assert!(step > 0.0);
    for pair in ticks.windows(2) {
        assert!((pair[1] - pair[0] - step).abs() < 0.01, "evenly spaced: {ticks:?}");
    }
}

#[test]
fn triangle_thumb_has_the_round_thumbs_hit_area() {
    let reach = md_style().thumb / 2.0;
    let mut outcomes = Vec::new();
    for thumb in [SliderThumb::Round, SliderThumb::Triangle] {
        let mut rig = Rig::new();
        rig.thumb = thumb;
        rig.frame(vec![]);
        // Within the thumb's reach a press leaves the value; just beyond it the thumb jumps there.
        let on = rig.level_at(40.0, reach - 0.5);
        rig.press(on);
        rig.frame(vec![]);
        let held = rig.level_event.as_ref().and_then(EditEvent::value).copied();
        rig.release(on);
        let off = rig.level_at(40.0, reach + 6.0);
        rig.press(off);
        rig.frame(vec![]);
        outcomes.push((held, rig.level_event.as_ref().and_then(EditEvent::value).copied()));
    }
    assert_eq!(outcomes[0], outcomes[1]);
    assert!(matches!(outcomes[1], (None | Some(40.0), Some(v)) if v > 40.0), "{outcomes:?}");
}

/// A range slider's thumbs are PhotoCraft's range handles: 5 by 12 rectangles at Compact, flat.
#[test]
fn range_slider_thumbs_are_narrow_flat_rectangles() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    ctx.run_ui(RawInput::default(), |ui| {
        let style = super::range_slider::range_style(&Tokens::of(ui), ControlSize::Md);
        assert_eq!((style.thumb_width, style.thumb), (5.0, 12.0));
        assert!(style.thumb_surface.shadow.is_none());
    })
    .drop_without_applying_deltas();
}

/// A thumb's outline is centred on the face's edge (PhotoCraft `widgets/slider.rs:204-209`): the face
/// is the thumb size and the outline reaches half its width beyond it. A bevel stays inside.
#[test]
fn a_thumb_outline_is_centred_on_the_edge_of_the_face() {
    use egui::{Shape, StrokeKind};
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut output = ctx.run_ui(RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(200.0, 100.0))), ..Default::default() }, |ui| {
        let style = super::slider::Slider::style(&Tokens::of(ui), ControlSize::Md);
        let paint =
            super::slider::ThumbPaint { fill: egui::Color32::WHITE, border: egui::Color32::BLACK, ring: egui::Color32::TRANSPARENT, enabled: true };
        let rect = super::slider::paint_thumb(ui, &style, pos2(50.0, 50.0), paint);
        assert_eq!((rect.width(), rect.height()), (style.thumb, style.thumb));
    });
    output.textures_delta.clear();
    let mut shapes = Vec::new();
    fn flatten(shape: Shape, out: &mut Vec<Shape>) {
        match shape {
            Shape::Vec(v) => v.into_iter().for_each(|s| flatten(s, out)),
            s => out.push(s),
        }
    }
    output.shapes.into_iter().for_each(|c| flatten(c.shape, &mut shapes));
    let rects: Vec<_> = shapes.iter().filter_map(|s| if let Shape::Rect(r) = s { Some(r) } else { None }).collect();
    let outline = rects.iter().find(|r| r.stroke.width > 0.0).expect("the thumb has an outline");
    assert_eq!(outline.stroke_kind, StrokeKind::Middle);
    let face = rects.iter().find(|r| r.fill == egui::Color32::WHITE).expect("the thumb has a face");
    assert_eq!(face.rect, outline.rect);
    assert_eq!(face.stroke.width, 0.0);
}

/// Photoshop's Properties panel: with the wheel off a slider never steps by it, focused or not,
/// and the panel under it scrolls.
#[test]
fn a_slider_with_the_wheel_off_ignores_it_focused_or_not() {
    let mut rig = Rig::new();
    let slider = rig.slider_id;
    rig.focus(slider);
    let over = rig.level_at(40.0, 0.0);
    rig.drag(over);
    rig.frame(vec![wheel(MouseWheelUnit::Line, 1.0)]);
    assert_eq!(rig.level_event, set(41.0, Modifiers::NONE), "on by default: the focused slider steps");

    rig.wheel = false;
    rig.wheel_unfocused = true;
    rig.time += 1.0;
    rig.frame(vec![wheel(MouseWheelUnit::Line, 1.0)]);
    assert_eq!((rig.level_event.clone(), rig.level), (None, 41.0), "focused");
    assert!(rig.has_focus(slider));
    rig.ctx.memory_mut(|m| m.surrender_focus(slider));
    rig.time += 1.0;
    rig.frame(vec![wheel(MouseWheelUnit::Line, 1.0)]);
    assert_eq!((rig.level_event.clone(), rig.level), (None, 41.0), "unfocused, even with wheel_unfocused");
}

/// The shaped text of a frame of a labelled slider: its rect, and the labels' rects left to right.
fn labelled_slider(labels: &[(f64, &str)], width: f32) -> (Rect, Vec<Rect>) {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut rect = Rect::NOTHING;
    let mut slider = Slider::new("indexed", "Colors", 64.0, 8.0..=256.0).width(width);
    if !labels.is_empty() {
        slider = slider.marks_labelled(labels.iter().copied());
    }
    let output = ctx.run_ui(RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))), ..Default::default() }, |ui| {
        rect = slider.clone().show(ui).response.rect
    });
    let mut texts: Vec<Rect> = output
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            egui::Shape::Text(t) => Some(t.galley.rect.translate(t.pos.to_vec2())),
            _ => None,
        })
        .collect();
    output.drop_without_applying_deltas();
    texts.sort_by(|a, b| a.left().total_cmp(&b.left()));
    (rect, texts)
}

#[test]
fn labelled_marks_sit_under_their_ticks_inside_the_slider_and_add_one_line_of_height() {
    let (plain, none) = labelled_slider(&[], 200.0);
    assert!(none.is_empty());
    let marks = [(8.0, "8"), (64.0, "64"), (128.0, "128"), (256.0, "256")];
    let (rect, labels) = labelled_slider(&marks, 200.0);
    assert_eq!(labels.len(), 4);
    assert!(rect.height() > plain.height(), "{rect:?} {plain:?}");
    let style = md_style();
    let line = labels[0].height();
    assert!(rect.height() - plain.height() < line + 4.0 * style.mark, "grows by about a line: {rect:?}");
    for label in &labels {
        assert!(rect.contains_rect(*label), "{label:?} in {rect:?}");
    }
    let track = Track::new(Rect::from_min_size(rect.min, vec2(rect.width(), style.height)), &style, Scale::new(&(8.0..=256.0), None));
    for (label, (value, _)) in labels.iter().zip(marks).filter(|(_, (v, _))| *v == 64.0 || *v == 128.0) {
        assert!((label.center().x - track.x_of(value)).abs() < 0.6, "centred under {value}: {label:?}");
    }
    assert!(labels[0].left() >= rect.left() && labels[3].right() <= rect.right() + 0.01, "ends stay inside: {labels:?}");
    assert!(labels.iter().all(|l| l.top() > rect.top() + style.height / 2.0), "under the track");
}
