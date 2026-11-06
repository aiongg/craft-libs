//! The selection bar.

use egui::{Context, Event, PointerButton, Rect, pos2};
use egui_kittest::Harness;

use super::SelectionBar;
use super::selection_bar::{BarMetrics, place};
use crate::theme::{Base, install};

const CANVAS: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(800.0, 600.0));

struct Host {
    selection: Option<Rect>,
    above: bool,
    inner: Option<()>,
    rect: Option<Rect>,
    pinned_at: Option<egui::Pos2>,
}

fn host(selection: Option<Rect>, above: bool) -> Harness<'static, Host> {
    let app = |ui: &mut egui::Ui, host: &mut Host| {
        let response = SelectionBar::new("bar").above(host.above).show(ui, CANVAS, host.selection, |ui| {
            ui.label("Controls");
        });
        host.inner = response.inner;
        host.rect = response.rect;
        host.pinned_at = response.pinned_at.or(host.pinned_at);
    };
    let mut h = Harness::<Host>::builder()
        .with_size([800.0, 600.0])
        .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
        .build_ui_state(app, Host { selection, above, inner: None, rect: None, pinned_at: None });
    h.run();
    h
}

fn bar() -> BarMetrics {
    BarMetrics::of(&Base::suite_default().shown(craft_theme::Mode::Light).resolved.tokens)
}

fn selection(min: (f32, f32), max: (f32, f32)) -> Rect {
    Rect::from_min_max(pos2(min.0, min.1), pos2(max.0, max.1))
}

#[test]
fn bar_sits_above_the_selection_and_flips_below() {
    let tokens = Base::suite_default().shown(craft_theme::Mode::Light).resolved.tokens.clone();
    let (height, above_gap, below_gap, inset) = (tokens.size.bar.options, tokens.space.gap.stack, tokens.space.pad.panel, tokens.space.gap.stack);
    let middle = selection((300.0, 200.0), (500.0, 300.0));
    let width = 200.0;
    assert_eq!(place(CANVAS, middle, width, false, &bar()), pos2(300.0, 300.0 + below_gap), "below, centred");
    assert_eq!(place(CANVAS, middle, width, true, &bar()), pos2(300.0, 200.0 - height - above_gap), "above, centred");

    let high = selection((300.0, 20.0), (500.0, 100.0));
    assert_eq!(place(CANVAS, high, width, true, &bar()).y, 100.0 + above_gap, "no room above: below");

    let low = selection((300.0, 400.0), (500.0, 590.0));
    assert_eq!(place(CANVAS, low, width, false, &bar()).y, 400.0 - height - below_gap, "no room below: above");

    let tall = selection((300.0, 20.0), (500.0, 590.0));
    assert_eq!(place(CANVAS, tall, width, false, &bar()).y, 600.0 - inset - height, "neither fits: the canvas's bottom less the inset");

    let corner = selection((0.0, 200.0), (20.0, 300.0));
    assert_eq!(place(CANVAS, corner, width, false, &bar()).x, inset, "x stays an inset inside the canvas");

    // The drawn bar uses the width it measured on the previous frame.
    let mut h = host(Some(middle), false);
    h.run();
    let rect = h.state().rect.expect("shown");
    let expected = place(CANVAS, middle, rect.width(), false, &bar());
    assert!((rect.min - expected).abs().max_elem() <= 0.5, "{:?} vs {expected:?}", rect.min);
}

#[test]
fn bar_hides_while_dragging_and_without_selection() {
    let h = host(None, false);
    assert!(h.state().inner.is_none() && h.state().rect.is_none(), "no selection, no bar");

    let mut h = host(Some(selection((300.0, 200.0), (500.0, 300.0))), false);
    assert!(h.state().inner.is_some());

    let away = pos2(60.0, 500.0);
    h.hover_at(away);
    h.event(Event::PointerButton { pos: away, button: PointerButton::Primary, pressed: true, modifiers: Default::default() });
    h.run();
    assert!(h.state().inner.is_none(), "a press on the canvas hides the bar");
    h.event(Event::PointerButton { pos: away, button: PointerButton::Primary, pressed: false, modifiers: Default::default() });
    h.run();
    assert!(h.state().inner.is_some(), "and it returns on release");
}

#[test]
// With the focus on the grip the arrow keys move the bar and pin it there.
fn arrow_keys_on_the_grip_move_the_bar() {
    use egui_kittest::kittest::Queryable as _;
    let mut h = host(Some(selection((300.0, 200.0), (500.0, 300.0))), false);
    h.get_by_label("Move Bar").focus();
    h.run();
    let before = h.state().rect.unwrap();
    h.key_press(egui::Key::ArrowRight);
    h.run();
    h.key_press(egui::Key::ArrowDown);
    h.run();
    let after = h.state().rect.unwrap();
    assert!(after.min.x > before.min.x && after.min.y > before.min.y, "{before:?} -> {after:?}");
    assert!(h.state().pinned_at.is_some());
}
