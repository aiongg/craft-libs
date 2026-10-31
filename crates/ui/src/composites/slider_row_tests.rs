use egui::accesskit::Role;
use egui::{Context, Event, Modifiers, PointerButton, Pos2, Rect, Ui, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::slider_row::{AdjustmentsArgs, show_adjustments};
use super::*;
use crate::direction::{self, Direction};
use crate::edit::EditEvent;
use crate::localized::LocaleNumbers;
use crate::testing::a11y;
use crate::theme::{Base, Tokens, TokensOf as _, install};

fn harness<State>(dir: Direction) -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size([400.0, 200.0]).with_context_setup(move |ctx: &Context| {
        install(ctx, &Base::suite_default());
        direction::apply(ctx, dir);
    })
}

/// The value and every event the row reported.
type State = (f64, Vec<EditEvent<f64>>);

fn row_in(ui: &mut Ui, (value, events): &mut State, width: f32) {
    let numbers = LocaleNumbers::from_tag("en");
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        let row = SliderRow::new("opacity", "Opacity", *value, 0.0..=100.0, &numbers).show(ui);
        for event in row.edits() {
            if let Some(v) = event.value() {
                *value = *v;
            }
            events.push(event);
        }
    });
}

fn rects(harness: &Harness<'_, State>) -> (Rect, Rect, Rect) {
    let caption = harness.get_by(|n| n.role() == Role::Label && n.value().as_deref() == Some("Opacity")).rect();
    let field = harness.get_by(|n| n.role() == Role::SpinButton).rect();
    let slider = harness.get_by(|n| n.role() == Role::Slider).rect();
    (caption, field, slider)
}

#[allow(clippy::expect_used)]
fn style() -> SliderRowStyle {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut style = None;
    ctx.run_ui(egui::RawInput::default(), |ui| style = Some(SliderRow::style(&Tokens::of(ui)))).drop_without_applying_deltas();
    style.expect("style")
}

#[test]
fn caption_and_field_share_the_first_line_over_a_full_width_slider() {
    let style = style();
    for dir in [Direction::Ltr, Direction::Rtl] {
        let harness = harness(dir).build_ui_state(|ui, state: &mut State| row_in(ui, state, 260.0), (50.0, Vec::new()));
        let (caption, field, slider) = rects(&harness);
        assert!((field.width() - style.field_width).abs() < 0.5, "{field:?}");
        assert!((slider.width() - 260.0).abs() < 0.5, "the slider fills the row: {slider:?}");
        assert!(slider.top() >= field.bottom(), "the slider is under the line: {field:?} {slider:?}");
        assert!((caption.center().y - field.center().y).abs() < 1.0, "one line: {caption:?} {field:?}");
        if dir == Direction::Ltr {
            assert!(caption.left() <= slider.left() + 1.0 && (field.right() - slider.right()).abs() < 1.0, "{caption:?} {field:?} {slider:?}");
        } else {
            assert!(caption.right() >= slider.right() - 1.0 && (field.left() - slider.left()).abs() < 1.0, "{caption:?} {field:?} {slider:?}");
        }
    }
}

#[test]
fn caption_names_the_field_and_the_slider() {
    let harness = harness(Direction::Ltr).build_ui_state(|ui, state: &mut State| row_in(ui, state, 260.0), (50.0, Vec::new()));
    let caption = harness.get_by(|n| n.role() == Role::Label && n.value().as_deref() == Some("Opacity")).accesskit_node().locate().0;
    for role in [Role::SpinButton, Role::Slider] {
        let node = harness.get_by(|n| n.role() == role);
        assert_eq!(node.accesskit_node().data().labelled_by(), [caption], "{role:?}");
        assert_eq!(node.accesskit_node().label().as_deref(), Some("Opacity"), "{role:?}");
    }
    a11y::assert_labelled(&harness);
}

/// Presses at `from`, moves to `to` and releases.
fn drag(harness: &mut Harness<'_, State>, from: Pos2, to: Pos2) {
    let button = |pos: Pos2, pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    harness.event(Event::PointerMoved(from));
    harness.run();
    harness.event(button(from, true));
    harness.run();
    harness.event(Event::PointerMoved(to));
    harness.run();
    harness.event(Event::PointerMoved(to + vec2(8.0, 0.0)));
    harness.run();
    harness.event(button(to + vec2(8.0, 0.0), false));
    harness.run();
}

fn commits(state: &State) -> usize {
    state.1.iter().filter(|e| matches!(e, EditEvent::Commit { .. })).count()
}

#[test]
fn a_slider_drag_and_a_caption_scrub_each_end_in_one_commit() {
    let mut harness = harness(Direction::Ltr).build_ui_state(|ui, state: &mut State| row_in(ui, state, 260.0), (50.0, Vec::new()));
    let (caption, _, slider) = rects(&harness);

    drag(&mut harness, slider.center(), slider.center() + vec2(40.0, 0.0));
    assert_eq!(commits(harness.state()), 1, "{:?}", harness.state().1);
    assert!(harness.state().0 > 50.0, "the slider moved the value: {}", harness.state().0);

    let before = harness.state().0;
    harness.state_mut().1.clear();
    drag(&mut harness, caption.center(), caption.center() + vec2(20.0, 0.0));
    assert_eq!(commits(harness.state()), 1, "{:?}", harness.state().1);
    assert_ne!(harness.state().0, before, "the caption scrubbed the field");
}

#[test]
fn the_story_names_every_control_at_every_width() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        for (width, disabled) in [(120.0, false), (260.0, true), (400.0, false)] {
            let mut args = AdjustmentsArgs { disabled, width };
            let harness = harness::<()>(dir).build_ui(move |ui| show_adjustments(ui, &mut args));
            a11y::assert_labelled(&harness);
        }
    }
}

#[test]
fn the_slider_gestures_reach_the_slider_and_the_field_stays_linear() {
    let mut harness = harness(Direction::Ltr).with_step_dt(0.02).build_ui_state(
        |ui, state: &mut State| {
            let numbers = LocaleNumbers::from_tag("en");
            let row = SliderRow::new("amount", "Amount", state.0, 0.0..=100.0, &numbers).default_value(10.0).curve(2.0).show(ui);
            for event in row.edits() {
                if let Some(v) = event.value() {
                    state.0 = *v;
                }
            }
        },
        (50.0, Vec::new()),
    );
    // On a power-2 track the middle is a quarter of the range.
    let slider = harness.get_by(|n| n.role() == Role::Slider).rect();
    drag(&mut harness, slider.center(), slider.center());
    let middle = harness.state().0;
    assert!((middle - 25.0).abs() < 4.0, "the track is curved: {middle}");
    // A double-click on the track resets.
    let button = |pressed| Event::PointerButton { pos: slider.center(), button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    harness.event(Event::PointerMoved(slider.center()));
    harness.run();
    for pressed in [true, false, true, false] {
        harness.event(button(pressed));
        harness.run();
    }
    assert_eq!(harness.state().0, 10.0, "a double-click sets the default");
}

/// A wheel notch over the slider moves the value by `wheel_step`; without
/// one it moves a hundredth of the range.
#[test]
fn a_wheel_notch_over_the_slider_moves_the_value_by_the_wheel_step() {
    for (wheel_step, notch) in [(None, 10.0), (Some(5.0), 5.0)] {
        let mut harness = harness(Direction::Ltr).build_ui_state(
            move |ui, state: &mut State| {
                let numbers = LocaleNumbers::from_tag("en");
                let mut row = SliderRow::new("amount", "Amount", state.0, 0.0..=1000.0, &numbers).wheel(true);
                if let Some(step) = wheel_step {
                    row = row.wheel_step(step);
                }
                for event in row.show(ui).edits() {
                    if let Some(v) = event.value() {
                        state.0 = *v;
                    }
                }
            },
            (500.0, Vec::new()),
        );
        harness.run();
        let slider = harness.get_by(|n| n.role() == Role::Slider).rect();
        harness.event(Event::PointerMoved(slider.center()));
        harness.run();
        let wheel = |modifiers| Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: vec2(0.0, 1.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers,
        };
        harness.event(wheel(Modifiers::NONE));
        harness.run();
        assert_eq!(harness.state().0, 500.0 + notch, "one notch, with the range a thousand wide");
    }
}

/// A row with a step reports only values on it, so a host that stores them as reported keeps the
/// gesture, and a typed value lands on the step.
#[test]
fn a_stepped_row_reports_values_on_the_step_and_keeps_its_gesture() {
    fn stepped_row(ui: &mut Ui, (value, events): &mut State) {
        let numbers = LocaleNumbers::from_tag("en");
        ui.allocate_ui(vec2(260.0, 0.0), |ui| {
            for event in SliderRow::new("opacity", "Opacity", *value, 0.0..=100.0, &numbers).step(5.0).show(ui).edits() {
                if let Some(v) = event.value() {
                    *value = *v;
                }
                events.push(event);
            }
        });
    }
    let mut harness = harness(Direction::Ltr).build_ui_state(stepped_row, (50.0, Vec::new()));
    harness.run();
    let (_, _, slider) = rects(&harness);
    let (start, y) = (slider.left() + 7.0 + (slider.width() - 14.0) * 0.5, slider.center().y);
    harness.input_mut().events.extend([
        Event::PointerMoved(Pos2::new(start, y)),
        Event::PointerButton { pos: Pos2::new(start, y), button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE },
    ]);
    harness.run();
    for dx in [7.3, 13.1, 21.9, 31.7] {
        harness.input_mut().events.push(Event::PointerMoved(Pos2::new(start + dx, y)));
        harness.run();
    }
    harness.input_mut().events.push(Event::PointerButton {
        pos: Pos2::new(start + 31.7, y),
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    let (value, events) = harness.state();
    let values: Vec<f64> = events.iter().filter_map(|e| e.value().copied()).collect();
    assert!(values.len() >= 3 && *value > 50.0, "{events:?}");
    assert!(values.iter().all(|v| (v / 5.0).fract() == 0.0), "on the step: {values:?}");
    assert!(events.iter().all(|e| !matches!(e, EditEvent::Cancel { .. })), "the gesture is kept: {events:?}");
    assert!(matches!(events.last(), Some(EditEvent::Commit { .. })), "{events:?}");
}

fn wheel_event() -> Event {
    Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: vec2(0.0, 1.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: Modifiers::NONE,
    }
}

/// `wheel(false)` keeps the wheel from the focused slider too; without it the focused slider steps.
#[test]
fn a_row_without_the_wheel_keeps_its_value_under_the_wheel_even_when_focused() {
    for (wheel, moves) in [(Some(false), false), (None, true)] {
        let mut harness = harness(Direction::Ltr).build_ui_state(
            move |ui, state: &mut State| {
                let numbers = LocaleNumbers::from_tag("en");
                let mut row = SliderRow::new("amount", "Amount", state.0, 0.0..=1000.0, &numbers);
                if let Some(on) = wheel {
                    row = row.wheel(on);
                }
                for event in row.show(ui).edits() {
                    if let Some(v) = event.value() {
                        state.0 = *v;
                    }
                }
            },
            (500.0, Vec::new()),
        );
        harness.run();
        harness.get_by(|n| n.role() == Role::Slider).focus();
        harness.run();
        let slider = harness.get_by(|n| n.role() == Role::Slider).rect();
        harness.event(Event::PointerMoved(slider.center()));
        harness.run();
        harness.event(wheel_event());
        harness.run();
        assert_eq!(harness.state().0 != 500.0, moves, "wheel {wheel:?}: {}", harness.state().0);
    }
}

/// A host that stores the reported f64 as an f32 passes back the rounding, which is not an outside
/// change: the drag goes on and ends on the stepped value.
#[test]
fn an_f32_host_keeps_the_gesture_through_a_drag_in_steps_of_a_hundredth_and_a_bit() {
    type F32State = (f32, Vec<EditEvent<f64>>);
    let mut harness = harness(Direction::Ltr).build_ui_state(
        |ui, (value, events): &mut F32State| {
            let numbers = LocaleNumbers::from_tag("en");
            ui.allocate_ui(vec2(260.0, 0.0), |ui| {
                for event in SliderRow::new("exposure", "Exposure", *value, 0.0..=1.0, &numbers).step(0.07).show(ui).edits() {
                    if let Some(v) = event.value() {
                        *value = *v as f32;
                    }
                    events.push(event);
                }
            });
        },
        (0.0, Vec::new()),
    );
    harness.run();
    let slider = harness.get_by(|n| n.role() == Role::Slider).rect();
    let (start, y) = (slider.left() + 7.0, slider.center().y);
    harness.input_mut().events.extend([
        Event::PointerMoved(Pos2::new(start, y)),
        Event::PointerButton { pos: Pos2::new(start, y), button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE },
    ]);
    harness.run();
    for dx in [9.0, 17.0, 26.0, 40.0, 55.0] {
        harness.input_mut().events.push(Event::PointerMoved(Pos2::new(start + dx, y)));
        harness.run();
    }
    harness.input_mut().events.push(Event::PointerButton {
        pos: Pos2::new(start + 55.0, y),
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    let (value, events) = harness.state();
    assert!(events.iter().all(|e| !matches!(e, EditEvent::Cancel { .. })), "the gesture is kept: {events:?}");
    let Some(EditEvent::Commit { value: last, .. }) = events.last() else { panic!("{events:?}") };
    assert!((*last / 0.07).round() > 1.0 && (*last / 0.07 - (*last / 0.07).round()).abs() < 1e-9, "on the step: {last}");
    assert_eq!(*value, *last as f32);
}

/// A host that stores `v / 100` as an `f32` and shows it times 100 as an `f64` is a few `f32` ulps
/// off the reported value: the drag still is one gesture.
#[test]
fn a_host_storing_a_scaled_f32_keeps_the_gesture_through_a_drag() {
    type ScaledState = (f32, Vec<EditEvent<f64>>);
    let mut harness = harness(Direction::Ltr).build_ui_state(
        |ui, (stored, events): &mut ScaledState| {
            let numbers = LocaleNumbers::from_tag("en");
            ui.allocate_ui(vec2(260.0, 0.0), |ui| {
                for event in SliderRow::new("opacity", "Opacity", f64::from(*stored) * 100.0, 0.0..=100.0, &numbers).step(0.01).show(ui).edits() {
                    if let Some(v) = event.value() {
                        *stored = (*v / 100.0) as f32;
                    }
                    events.push(event);
                }
            });
        },
        (0.0, Vec::new()),
    );
    harness.run();
    let slider = harness.get_by(|n| n.role() == Role::Slider).rect();
    let (start, y) = (slider.left() + 7.0, slider.center().y);
    harness.input_mut().events.extend([
        Event::PointerMoved(Pos2::new(start, y)),
        Event::PointerButton { pos: Pos2::new(start, y), button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE },
    ]);
    harness.run();
    for dx in [9.0, 17.0, 26.0, 40.0, 55.0] {
        harness.input_mut().events.push(Event::PointerMoved(Pos2::new(start + dx, y)));
        harness.run();
    }
    harness.input_mut().events.push(Event::PointerButton {
        pos: Pos2::new(start + 55.0, y),
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    let (_, events) = harness.state();
    assert!(events.iter().all(|e| !matches!(e, EditEvent::Cancel { .. })), "the gesture is kept: {events:?}");
    assert_eq!(events.iter().filter(|e| matches!(e, EditEvent::Commit { .. })).count(), 1, "{events:?}");
}
