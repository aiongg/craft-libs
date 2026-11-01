//! `Rating` in a harness.
#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]

use egui::accesskit::Role;
use egui::{Color32, Key, Modifiers, Pos2, Ui};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::behavior::KeyClaim;
use crate::direction::{self, Direction};
use crate::edit::EditEvent;
use crate::theme::{Base, install};

#[derive(Default)]
struct Host {
    rating: u8,
    events: Vec<EditEvent<u8>>,
}

fn harness(direction: Direction) -> Harness<'static, Host> {
    HarnessBuilder::default()
        .with_size([240.0, 80.0])
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            direction::apply(ctx, direction);
        })
        .build_ui_state(
            |ui: &mut Ui, host: &mut Host| {
                let edit = Rating::new("rating", "Rating", &mut host.rating).show(ui);
                host.events.extend(edit.event);
            },
            Host::default(),
        )
}

fn star<'h>(harness: &'h Harness<'_, Host>, label: &'h str) -> egui_kittest::Node<'h> {
    harness.get_by(|n| n.role() == Role::RadioButton && n.label().as_deref() == Some(label))
}

fn sets(host: &Host) -> Vec<u8> {
    host.events
        .iter()
        .map(|e| match e {
            EditEvent::Set { value, .. } => *value,
            other => panic!("a rating reports only Set: {other:?}"),
        })
        .collect()
}

// Clicking star 3 of an unrated row gives 3; clicking star 3 again gives 0.
#[test]
fn a_click_sets_and_a_click_on_the_current_star_clears() {
    let mut harness = harness(Direction::Ltr);
    harness.run();
    star(&harness, "3 stars").click();
    harness.run();
    assert_eq!(harness.state().rating, 3);
    star(&harness, "3 stars").click();
    harness.run();
    assert_eq!(harness.state().rating, 0);
    assert_eq!(sets(harness.state()), [3, 0]);
}

// With the focus in the row, → raises, ← lowers (mirrored in right-to-left), and the digit 4 sets 4.
#[test]
fn arrows_and_digits_change_the_rating() {
    for (direction, raise, lower) in [(Direction::Ltr, Key::ArrowRight, Key::ArrowLeft), (Direction::Rtl, Key::ArrowLeft, Key::ArrowRight)] {
        let mut harness = harness(direction);
        harness.run();
        star(&harness, "1 star").focus();
        harness.run();
        harness.key_press(raise);
        harness.key_press(raise);
        harness.run();
        assert_eq!(harness.state().rating, 2, "{direction:?}");
        harness.key_press(lower);
        harness.run();
        assert_eq!(harness.state().rating, 1, "{direction:?}");
        harness.key_press(Key::Num4);
        harness.run();
        assert_eq!(harness.state().rating, 4, "{direction:?}");
        harness.key_press(Key::End);
        harness.key_press(raise);
        harness.run();
        assert_eq!(harness.state().rating, 5);
        assert_eq!(sets(harness.state()), [1, 2, 1, 4, 5], "a key that changes nothing reports nothing");
        harness.key_press(Key::Num0);
        harness.run();
        assert_eq!(harness.state().rating, 0);
    }
}

// The focused row claims the digits, so the app's tool keys leave them to it.
#[test]
fn the_focused_row_claims_the_digits() {
    let mut harness = harness(Direction::Ltr);
    harness.run();
    star(&harness, "1 star").focus();
    harness.run();
    let digit = egui::KeyboardShortcut::new(Modifiers::NONE, Key::Num3);
    assert!(KeyClaim::claimed(&harness.ctx, &digit));
}

// `paint_rating` returns 3 × 13 for three Medium stars and 0 for none.
#[test]
fn painted_stars_report_their_width() {
    let ctx = egui::Context::default();
    install(&ctx, &Base::suite_default());
    let mut widths = Vec::new();
    let output = ctx.run_ui(Default::default(), |ui| {
        let painter = ui.painter().clone();
        let at = Pos2::new(10.0, 10.0);
        widths = vec![
            paint_rating(&painter, at, 3, StarSize::Medium, Color32::WHITE),
            paint_rating(&painter, at, 0, StarSize::Medium, Color32::WHITE),
            paint_rating(&painter, at, 9, StarSize::Small, Color32::WHITE),
        ];
    });
    output.drop_without_applying_deltas();
    assert_eq!(widths, [39.0, 0.0, 45.0], "a rating over five is five stars");
}
