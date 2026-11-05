//! Revisions.

#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]

use craft_ui::testing::symbols;
use craft_ui::theme::{Base, install};
use egui::Ui;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::revisions::RevisionsPanel;
use crate::revisions::{Revision, RevisionAction, RevisionKind};

struct App {
    revisions: Vec<Revision>,
    actions: Vec<RevisionAction>,
}

fn show(ui: &mut Ui, app: &mut App) {
    let out = RevisionsPanel::new("revisions", "deck-v2", &app.revisions).show(ui);
    app.actions.extend(out.actions);
}

fn harness(revisions: Vec<Revision>) -> Harness<'static, App> {
    let mut harness = Harness::<App>::builder()
        .with_size([480.0, 400.0])
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
        })
        .build_ui_state(show, App { revisions, actions: Vec::new() });
    harness.run();
    harness
}

fn revision(kind: RevisionKind, heading: &str) -> Revision {
    Revision { kind, heading: heading.into(), details: vec!["Title text changed".into()] }
}

// per-row buttons and the All buttons report their actions.
#[test]
fn accept_and_reject_report_their_row() {
    let mut harness = harness(vec![revision(RevisionKind::Changed, "Slide 3 changed"), revision(RevisionKind::Inserted, "Slide inserted after 2")]);
    harness.get_by_label("Accept All (2)");
    harness.get_all_by_role_and_label(Role::Button, "Accept").nth(1).expect("the second card's Accept").click();
    harness.run();
    harness.get_all_by_role_and_label(Role::Button, "Reject").next().expect("the first card's Reject").click();
    harness.run();
    harness.get_by_role_and_label(Role::Link, "Slide 3 changed").click();
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Reject All (2)").click();
    harness.run();
    assert_eq!(harness.state().actions, [RevisionAction::Accept(1), RevisionAction::Reject(0), RevisionAction::Go(0), RevisionAction::RejectAll]);
}

// an empty list shows End Review.
#[test]
fn no_changes_offers_end_review() {
    let mut harness = harness(Vec::new());
    harness.get_by_label("No differences left.");
    harness.get_by_role_and_label(Role::Button, "End Review").click();
    harness.run();
    assert_eq!(harness.state().actions, [RevisionAction::EndReview]);
}

// each card's bar carries its kind as a word for a screen reader.
#[test]
fn a_cards_bar_names_its_kind() {
    let harness = harness(vec![revision(RevisionKind::Changed, "Slide 3 changed"), revision(RevisionKind::Deleted, "Slide 4 deleted")]);
    harness.get_by_label("Changed");
    harness.get_by_label("Deleted");
}

// detail lines get their bullet from the words, and Accept All counts the changes.
#[test]
fn details_are_bulleted_by_a_word() {
    let harness = harness(vec![revision(RevisionKind::Changed, "Slide 3 changed")]);
    harness.get_by_label("\u{2022} Title text changed");
    harness.get_by_label("Accept All (1)");
}
