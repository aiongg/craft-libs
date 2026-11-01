//! The column bar.

use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, install};
use egui::{Key, vec2};
use egui_kittest::{Harness, HarnessBuilder};

use super::column_bar::*;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

// Moving divider 1 of [1, 1, 1, 1] by 0.1 changes columns 1 and 2 only and keeps the sum.
#[test]
fn a_divider_moves_only_its_two_columns() {
    let mut widths = [1.0, 1.0, 1.0, 1.0];
    move_divider(&mut widths, 1, 0.1);
    assert_eq!((widths[0], widths[3]), (1.0, 1.0));
    assert!(close(widths[1], 1.4) && close(widths[2], 0.6), "{widths:?}");
    assert!(close(widths.iter().sum::<f64>(), 4.0));
    assert!(close(shares(&[2.0, 6.0])[0], 0.25));
    assert_eq!(shares(&[0.0, 0.0]), [0.5, 0.5]);
}

// Dragging a divider between two 0.01 columns far either way keeps both at a quarter of their pair.
#[test]
fn narrow_pairs_never_panic() {
    for delta in [1.0, -1.0, f64::MAX, f64::NEG_INFINITY] {
        let mut widths = [0.01, 0.01, 0.98];
        move_divider(&mut widths, 0, delta);
        if delta.is_finite() {
            assert!(close(widths[0].min(widths[1]), 0.005), "{widths:?}");
            assert!(close(widths[0] + widths[1], 0.02));
        } else {
            assert_eq!(widths, [0.01, 0.01, 0.98]);
        }
    }
    let mut none: [f64; 0] = [];
    move_divider(&mut none, 0, 0.1);
}

#[derive(Default)]
struct App {
    events: Vec<EditEvent<Vec<f64>>>,
}

fn open() -> Harness<'static, App> {
    let mut harness =
        HarnessBuilder::default().with_size(vec2(400.0, 100.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App| {
                app.events.extend(ColumnBar::new("bar", &[1.0, 1.0, 1.0, 1.0]).show(ui).edit);
            },
            App::default(),
        );
    harness.run();
    harness
}

// Tab to divider 0 and Right arrow reports Set with column 0 grown by 0.01 of the whole.
#[test]
fn arrow_keys_resize_the_focused_pair() {
    let mut harness = open();
    harness.key_press(Key::Tab);
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    let events = &harness.state().events;
    let [EditEvent::Set { value, .. }] = events.as_slice() else { panic!("{events:?}") };
    assert!(close(value[0], 1.04) && close(value[1], 0.96), "{value:?}");
    assert_eq!((value[2], value[3]), (1.0, 1.0));
}
