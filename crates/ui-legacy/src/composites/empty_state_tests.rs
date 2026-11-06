//! Empty state tests: the status live region, the centred layout and the heading.

use egui::accesskit::{Live, Role};
use egui::{Context, Rect, Ui};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::controls::Button;
use crate::direction::{self, Direction};
use crate::testing::a11y;
use crate::theme::{Base, Tokens, TokensOf as _, install};

fn harness<'a>(size: [f32; 2], show: impl FnMut(&mut Ui) + 'a) -> Harness<'a, ()> {
    let builder: HarnessBuilder<()> = Harness::builder().with_size(size).with_context_setup(|ctx: &Context| {
        install(ctx, &Base::suite_default());
        direction::apply(ctx, Direction::Ltr);
    });
    builder.build_ui(show)
}

fn centred(ui: &mut Ui) {
    EmptyState::centred("No document")
        .description("Create a new document or open an existing file.")
        .action(Button::new("New document…"))
        .action(Button::new("Open…"))
        .show(ui);
}

fn button_rect(harness: &Harness<'_, ()>, label: &str) -> Rect {
    harness.get_by(|n| n.role() == Role::Button && n.label().as_deref() == Some(label)).rect()
}

#[test]
fn status_makes_a_polite_status_region() {
    let mut harness = harness([300.0, 100.0], |ui| {
        EmptyState::new("No matching commands").status().show(ui);
    });
    harness.run();
    let node = harness.get_by(|n| n.role() == Role::Status);
    assert_eq!(node.accesskit_node().live(), Live::Polite);
    assert_eq!(node.accesskit_node().label().as_deref(), Some("No matching commands"));
}

#[test]
fn a_plain_inline_message_is_a_label() {
    let mut harness = harness([300.0, 100.0], |ui| {
        EmptyState::new("No document").show(ui);
    });
    harness.run();
    assert!(harness.query_by(|n| matches!(n.role(), Role::Status | Role::Alert)).is_none());
    assert!(harness.query_by(|n| n.role() == Role::Label && n.value().as_deref() == Some("No document")).is_some());
}

#[test]
fn centred_actions_sit_side_by_side_at_the_full_width() {
    let mut harness = harness([640.0, 400.0], centred);
    harness.run();
    let (new, open) = (button_rect(&harness, "New document…"), button_rect(&harness, "Open…"));
    assert!((new.center().y - open.center().y).abs() < 0.5, "one row");
    assert!((new.width() - open.width()).abs() < 0.5, "a common width");
    let column = Rect::from_center_size(egui::pos2(320.0, 0.0), egui::vec2(460.0, 0.0));
    let centre = (new.left() + open.right()) / 2.0;
    assert!((centre - column.center().x).abs() < 1.0, "centred: {centre}");
}

#[test]
fn centred_actions_stack_when_narrow() {
    let mut harness = harness([240.0, 400.0], centred);
    harness.run();
    let (new, open) = (button_rect(&harness, "New document…"), button_rect(&harness, "Open…"));
    assert!(open.top() >= new.bottom(), "stacked: {new:?} {open:?}");
    assert!((new.left() - open.left()).abs() < 0.5);
}

#[test]
fn the_column_is_centred_vertically_after_the_first_frame() {
    let mut harness = harness([640.0, 400.0], centred);
    harness.run();
    let heading = harness.get_by(|n| n.role() == Role::Heading).rect();
    let last = button_rect(&harness, "Open…");
    let (top, bottom) = (heading.top(), last.bottom());
    assert!(((top + bottom) / 2.0 - 200.0).abs() < 8.0, "{top} {bottom}");
}

#[test]
fn two_centred_states_in_one_scope_are_each_centred() {
    let mut harness = harness([640.0, 400.0], |ui| {
        ui.columns(2, |columns| {
            EmptyState::centred("Left").show(&mut columns[0]);
            EmptyState::centred("Right")
                .description("Create a new document or open an existing file.")
                .action(Button::new("Open…"))
                .show(&mut columns[1]);
        });
    });
    harness.run();
    let left = harness.get_by(|n| n.role() == Role::Heading && n.label().as_deref() == Some("Left")).rect();
    assert!((left.center().y - 200.0).abs() < 8.0, "{left:?}");
    let right = harness.get_by(|n| n.role() == Role::Heading && n.label().as_deref() == Some("Right")).rect();
    let last = button_rect(&harness, "Open…");
    assert!(((right.top() + last.bottom()) / 2.0 - 200.0).abs() < 8.0, "{right:?} {last:?}");
}

#[test]
fn the_title_is_a_heading_and_names_the_column() {
    let mut harness = harness([640.0, 400.0], centred);
    harness.run();
    let heading = harness.get_by(|n| n.role() == Role::Heading);
    assert_eq!(heading.accesskit_node().label().as_deref(), Some("No document"));
    let column = harness.get_by(|n| n.role() == Role::GenericContainer && n.label().as_deref() == Some("No document"));
    assert_eq!(column.accesskit_node().live(), Live::Off);
    assert!(harness.query_by(|n| matches!(n.role(), Role::Status | Role::Alert)).is_none(), "no live region");
    a11y::assert_labelled(&harness);
}

#[test]
fn the_stories_are_labelled() {
    for story in empty_state::stories() {
        let mut args = (story.args)();
        let show = story.show;
        let mut harness = harness([900.0, 700.0], move |ui| show(ui, &mut args));
        harness.run();
        a11y::assert_labelled(&harness);
    }
}

#[test]
fn a_start_screen_has_a_brand_heading_equal_actions_and_a_slot_between_actions_and_footer() {
    let min = std::cell::Cell::new(0.0);
    let mut harness = harness([640.0, 500.0], |ui| {
        min.set(EmptyState::style(&Tokens::of(ui)).action_min_width);
        EmptyState::centred("Sample App")
            .brand("open source")
            .action(Button::new("New"))
            .action(Button::new("Open…"))
            .below_actions(|ui| {
                ui.label("Drop hint");
            })
            .footer(|ui| {
                ui.label("Footer");
            })
            .show(ui);
    });
    harness.run();
    let heading = harness.get_by(|n| n.role() == Role::Heading);
    assert_eq!(heading.accesskit_node().label().as_deref(), Some("Sample App"));
    let byline = harness.get_by(|n| n.role() == Role::Label && n.value().as_deref() == Some("open source"));
    assert!(byline.rect().left() >= heading.rect().right(), "byline after the title");
    let (new, open) = (button_rect(&harness, "New"), button_rect(&harness, "Open…"));
    let min = min.get();
    assert!((new.width() - open.width()).abs() < 0.5 && new.width() >= min - 0.5, "{new:?} {open:?} {min}");
    let hint = harness.get_by(|n| n.role() == Role::Label && n.value().as_deref() == Some("Drop hint")).rect();
    let footer = harness.get_by(|n| n.role() == Role::Label && n.value().as_deref() == Some("Footer")).rect();
    assert!(hint.top() >= new.bottom() && footer.top() >= hint.bottom(), "{new:?} {hint:?} {footer:?}");
}
