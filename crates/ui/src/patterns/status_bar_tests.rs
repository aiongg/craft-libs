//! Status bar tests: the groups at the ends of the bar, reading order, the message giving way,
//! the empty message and mirroring.

use std::cell::Cell;
use std::rc::Rc;

use egui::accesskit::{Live, Role};
use egui::{Context, Rect, Ui};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::status_bar::stories;
use super::*;
use crate::controls::Button;
use crate::direction::{self, Direction};
use crate::testing::a11y;
use crate::theme::{Base, Tokens, TokensOf as _, install};
use craft_theme::ControlSize;

fn harness<State>(dir: Direction, width: f32) -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size([width, 60.0]).with_context_setup(move |ctx: &Context| {
        install(ctx, &Base::suite_default());
        direction::apply(ctx, dir);
    })
}

#[allow(clippy::expect_used)]
fn style() -> StatusBarStyle {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut style = None;
    ctx.run_ui(egui::RawInput::default(), |ui| style = Some(StatusBar::style(&Tokens::of(ui)))).drop_without_applying_deltas();
    style.expect("style")
}

const MESSAGE: &str = "Exported fourteen layers to the folder Documents/Exports/2026";

/// A bar with an info text, a message and two trailing buttons; the bar's rect goes to `bar_rect`.
fn bar(ui: &mut Ui, bar_rect: &Cell<Rect>, message: &str) {
    let message = message.to_owned();
    let response = StatusBar::new("test").show(ui, |bar| {
        bar.leading(|ui| {
            StatusBar::info(ui, "Info");
            StatusBar::message(ui, &message, false);
        });
        bar.trailing(|ui| {
            ui.add(Button::new("First").size(ControlSize::Sm));
            ui.add(Button::new("Second").size(ControlSize::Sm));
        });
    });
    bar_rect.set(response.rect);
}

fn run_bar(dir: Direction, width: f32, message: &str) -> (Harness<'static, ()>, Rect) {
    let rect = Rc::new(Cell::new(Rect::NOTHING));
    let seen = rect.clone();
    let message = message.to_owned();
    let mut harness = harness(dir, width).build_ui(move |ui| bar(ui, &seen, &message));
    harness.run();
    (harness, rect.get())
}

#[test]
fn the_bar_is_as_high_as_its_token_and_the_groups_sit_at_its_ends() {
    let style = style();
    let (harness, bar) = run_bar(Direction::Ltr, 500.0, "");
    assert_eq!(bar.height(), style.height);
    let (info, second) = (harness.get_by_label("Info").rect(), harness.get_by_label("Second").rect());
    assert!((info.left() - bar.left() - style.pad).abs() < 1.0, "leading starts at the pad: {info:?} in {bar:?}");
    assert!((bar.right() - style.pad - second.right()).abs() < 1.0, "trailing ends at the pad: {second:?} in {bar:?}");
    assert!(
        (info.center().y - bar.center().y).abs() < 1.0 && (second.center().y - bar.center().y).abs() < 1.0,
        "centred vertically: {info:?} {second:?} in {bar:?}"
    );
}

#[test]
fn both_groups_mirror_in_a_right_to_left_ui() {
    let style = style();
    let (harness, bar) = run_bar(Direction::Rtl, 500.0, "");
    let (info, second) = (harness.get_by_label("Info").rect(), harness.get_by_label("Second").rect());
    assert!((bar.right() - style.pad - info.right()).abs() < 1.0, "leading at the right: {info:?} in {bar:?}");
    assert!((second.left() - bar.left() - style.pad).abs() < 1.0, "trailing at the left: {second:?} in {bar:?}");
}

/// The trailing group is built in reading order: the first item added is the one nearer the start,
/// and Tab follows the leading group, then the trailing one in that order.
#[test]
fn the_trailing_group_keeps_reading_order_for_position_and_tab() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        let (mut harness, _) = run_bar(dir, 500.0, "");
        let (first, second) = (harness.get_by_label("First").rect(), harness.get_by_label("Second").rect());
        let ordered = if dir == Direction::Ltr { first.right() < second.left() } else { second.right() < first.left() };
        assert!(ordered, "{dir:?}: {first:?} {second:?}");
        let labels: Vec<_> = a11y::tab_order(&mut harness).into_iter().map(|stop| stop.label).collect();
        assert_eq!(labels, ["First", "Second"], "{dir:?}");
    }
}

/// A narrow bar keeps the trailing group whole and cuts the message, which ends in an ellipsis
/// and keeps its full text in the tree.
#[test]
fn a_narrow_bar_cuts_the_message_and_keeps_the_trailing_group() {
    let style = style();
    let (wide, _) = run_bar(Direction::Ltr, 900.0, MESSAGE);
    let wide_second = wide.get_by_label("Second").rect();
    let (narrow, bar) = run_bar(Direction::Ltr, 300.0, MESSAGE);
    let (first, second) = (narrow.get_by_label("First").rect(), narrow.get_by_label("Second").rect());
    assert!((bar.right() - style.pad - second.right()).abs() < 1.0, "trailing still at the end: {second:?} in {bar:?}");
    assert!((second.width() - wide_second.width()).abs() < 0.5, "trailing keeps its width");
    let message = narrow.get_all_by_label(MESSAGE).find(|n| n.accesskit_node().role() == Role::Label).expect("message label").rect();
    assert!(message.right() <= first.left(), "the message ends before the trailing group: {message:?} {first:?}");
    let wide_message = wide.get_all_by_label(MESSAGE).find(|n| n.accesskit_node().role() == Role::Label).expect("message label").rect();
    assert!(message.width() < wide_message.width(), "the message is cut: {message:?} against {wide_message:?}");
}

/// An empty message adds nothing to the bar, its separator included; a message adds a separator
/// and is announced politely.
#[test]
fn an_empty_message_draws_no_separator() {
    let style = style();
    let after = |message: &'static str| {
        let marker = Rc::new(Cell::new(Rect::NOTHING));
        let seen = marker.clone();
        let mut harness = harness(Direction::Ltr, 500.0).build_ui(move |ui| {
            StatusBar::new("test").show(ui, |bar| {
                let seen = seen.clone();
                bar.leading(move |ui| {
                    StatusBar::info(ui, "Info");
                    StatusBar::message(ui, message, false);
                    seen.set(ui.add(Button::new("Marker").size(ControlSize::Sm)).rect);
                });
            });
        });
        harness.run();
        let info = harness.get_by_label("Info").rect();
        (info, marker.get(), harness)
    };
    let (info, marker, _) = after("");
    assert!((marker.left() - info.right() - style.gap).abs() < 1.0, "no separator: {info:?} {marker:?}");
    let (info, marker, harness) = after("Saved");
    assert!(marker.left() - info.right() > style.gap + style.divider_box, "a separator and the message sit between: {info:?} {marker:?}");
    let region = harness.get_all_by_role(Role::Status).next().expect("live region");
    assert_eq!(region.accesskit_node().live(), Live::Polite);
    assert_eq!(region.accesskit_node().label().as_deref(), Some("Saved"));
}

#[test]
fn the_bar_is_a_group_named_by_the_host() {
    let mut harness = harness(Direction::Ltr, 400.0).build_ui(|ui| {
        StatusBar::new("test").label("Document status").show(ui, |bar| bar.leading(|ui| drop(StatusBar::info(ui, "Info"))));
    });
    harness.run();
    let group = harness.get_all_by_role(Role::Group).find(|n| n.accesskit_node().label().as_deref() == Some("Document status"));
    assert!(group.is_some());
}

#[test]
fn every_story_names_every_control() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        for story in stories() {
            let mut args = (story.args)();
            let show = story.show;
            let mut harness = harness(dir, 800.0).build_ui(move |ui| show(ui, &mut args));
            harness.run();
            a11y::assert_labelled(&harness);
        }
    }
}

/// The progress item is a progress indicator named by its label with the fraction as its value
/// (none while it has reported nothing), and its cancel button is named and reports a click.
#[test]
fn the_progress_item_reports_its_fraction_and_a_cancel_click() {
    let cancelled = Rc::new(Cell::new(false));
    let seen = cancelled.clone();
    let mut harness = harness(Direction::Ltr, 500.0).build_ui(move |ui| {
        StatusBar::new("test").show(ui, |bar| {
            let seen = seen.clone();
            bar.trailing(move |ui| {
                if StatusBar::progress(ui, StatusProgress::new("Gaussian Blur", Some(0.45)).percent("45%").cancel("Cancel")) {
                    seen.set(true);
                }
            });
        });
    });
    harness.run();
    let bar = harness.get_by_role(Role::ProgressIndicator);
    assert_eq!(bar.accesskit_node().label().as_deref(), Some("Gaussian Blur"));
    assert!((bar.accesskit_node().numeric_value().expect("value") - 0.45).abs() < 1e-6);
    harness.get_by_label("45%");
    harness.get_by_label("Cancel").click();
    harness.run();
    assert!(cancelled.get());

    let mut busy = harness_for_indeterminate();
    busy.step();
    assert_eq!(busy.get_by_role(Role::ProgressIndicator).accesskit_node().numeric_value(), None);
    assert!(busy.query_by_label("Cancel").is_none());
}

fn harness_for_indeterminate() -> Harness<'static, ()> {
    harness(Direction::Ltr, 500.0).build_ui(|ui| {
        StatusBar::new("test").show(ui, |bar| {
            bar.trailing(|ui| {
                StatusBar::progress(ui, StatusProgress::new("Working", None));
            });
        });
    })
}
