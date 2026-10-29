use egui::accesskit::Role;
use egui::{Context, Event, Modifiers, PointerButton, Pos2, Rect, Ui, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::slider_row::show_adjustments;
use super::*;
use crate::direction::{self, Direction};
use crate::edit::EditEvent;
use crate::localized::LocaleNumbers;
use crate::story::Args;
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
            let mut args = Args::new().with_bool("disabled", disabled).with_number("width", width, 120.0, 400.0);
            let harness = harness::<()>(dir).build_ui(move |ui| show_adjustments(ui, &mut args));
            a11y::assert_labelled(&harness);
        }
    }
}
