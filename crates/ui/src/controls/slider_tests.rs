//! Slider and range-slider behaviour in a headless egui context: the edit gesture of a drag, Esc,
//! the keys and the wheel, the direction, and the range slider's thumbs.

use egui::accesskit::{Action, ActionRequest, TreeId};
use egui::emath::TSTransform;
use egui::{
    Context, Event, Id, Key, Modifiers, MouseWheelSource, MouseWheelUnit, PointerButton, Pos2, RawInput, Rect, TouchPhase, Ui, Vec2, pos2, vec2,
};

use super::slider::{Scale, Track, bounds};
use super::{ControlSize, RangeSlider, Slider};
use crate::direction::{self, Direction};
use crate::edit::{self, CancelReason, EditEvent, Ending, GestureId};
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
            to_global: TSTransform::IDENTITY,
            thumbs: (Id::NULL, Id::NULL),
            scroll_left: Vec2::ZERO,
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
        ui.ctx().set_transform_layer(ui.layer_id(), self.to_global);
        let style = Slider::style(&Tokens::of(ui), ControlSize::Md);
        let range = self.bounds.0..=self.bounds.1;
        let scale = Scale::new(&range, self.step);
        let mut slider = Slider::new("rig-slider", "Level", self.level, range.clone()).enabled(self.enabled).wheel_unfocused(self.wheel_unfocused);
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
        self.tracks = Some((slider, Track::new(edit.response.rect, &style, scale)));
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
        assert_eq!(rig.reported, Some(0.0));
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
    rig.release(rig.level_at(40.0, 3.0));
    assert_eq!(rig.level, 25.0, "the second click on the thumb resets");

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
}
