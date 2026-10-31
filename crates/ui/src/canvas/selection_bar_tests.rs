//! The selection bar.

use egui::{Context, Event, PointerButton, Rect, pos2};
use egui_kittest::Harness;

use super::SelectionBar;
use super::selection_bar::place;
use crate::theme::{Base, install};

const CANVAS: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(800.0, 600.0));

struct Host {
    selection: Option<Rect>,
    above: bool,
    inner: Option<()>,
    rect: Option<Rect>,
}

fn host(selection: Option<Rect>, above: bool) -> Harness<'static, Host> {
    let app = |ui: &mut egui::Ui, host: &mut Host| {
        let response = SelectionBar::new("bar").above(host.above).show(ui, CANVAS, host.selection, |ui| {
            ui.label("Controls");
        });
        host.inner = response.inner;
        host.rect = response.rect;
    };
    let mut h = Harness::<Host>::builder()
        .with_size([800.0, 600.0])
        .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
        .build_ui_state(app, Host { selection, above, inner: None, rect: None });
    h.run();
    h
}

fn bar() -> craft_theme::SelectionBarTokens {
    Base::suite_default().shown(craft_theme::Mode::Light).resolved.tokens.component.selection_bar
}

fn selection(min: (f32, f32), max: (f32, f32)) -> Rect {
    Rect::from_min_max(pos2(min.0, min.1), pos2(max.0, max.1))
}

#[test]
fn bar_sits_above_the_selection_and_flips_below() {
    let middle = selection((300.0, 200.0), (500.0, 300.0));
    let width = 200.0;
    assert_eq!(place(CANVAS, middle, width, false, &bar()), pos2(300.0, 314.0), "14 pt below, centred");
    assert_eq!(place(CANVAS, middle, width, true, &bar()), pos2(300.0, 200.0 - 36.0 - 8.0), "8 pt above, centred");

    let high = selection((300.0, 20.0), (500.0, 100.0));
    assert_eq!(place(CANVAS, high, width, true, &bar()).y, 108.0, "no room above: below");

    let low = selection((300.0, 400.0), (500.0, 590.0));
    assert_eq!(place(CANVAS, low, width, false, &bar()).y, 400.0 - 36.0 - 14.0, "no room below: above");

    let tall = selection((300.0, 20.0), (500.0, 590.0));
    assert_eq!(place(CANVAS, tall, width, false, &bar()).y, 600.0 - 8.0 - 36.0, "neither fits: the canvas's bottom less 8 pt");

    let corner = selection((0.0, 200.0), (20.0, 300.0));
    assert_eq!(place(CANVAS, corner, width, false, &bar()).x, 8.0, "x stays 8 pt inside the canvas");

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
