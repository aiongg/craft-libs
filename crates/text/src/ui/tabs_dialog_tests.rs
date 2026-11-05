//! The Tabs dialog.

#![allow(clippy::expect_used, reason = "tests")]

use craft_ui::dialog::{DialogHost, DialogResult};
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::theme::{Base, install};
use egui::{Modifiers, vec2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use super::ruler::{TabAlign, TabLeader, TabStop};
use super::tabs_dialog::*;

const INCH: f32 = 72.0;

fn stop(position: f32, align: TabAlign) -> TabStop {
    TabStop { position, align, leader: TabLeader::None }
}

// With stops at 0.5 and 1 in and a 0.5 in interval, Set adds 1.5 in, left, no leader, selected.
#[test]
fn set_adds_the_next_free_stop() {
    let mut dialog = TabStopsDialog::new(vec![stop(0.5 * INCH, TabAlign::Right), stop(INCH, TabAlign::Center)], 0.5 * INCH);
    dialog.new_stop();
    let result = dialog.result();
    assert_eq!(result.stops.last(), Some(&stop(1.5 * INCH, TabAlign::Left)));
    assert_eq!(result.stops.len(), 3);

    // A position already waiting to be cleared is not offered again.
    dialog.select(Some(2));
    dialog.clear();
    dialog.new_stop();
    assert_eq!(dialog.result().stops.last().map(|s| s.position), Some(2.0 * INCH));
}

// Moving the 1 in stop onto 0.5 in and ending the edit leaves one stop with the edited row's alignment.
#[test]
fn editing_in_place_merges_on_normalize() {
    let mut dialog = TabStopsDialog::new(vec![stop(0.5 * INCH, TabAlign::Left), stop(INCH, TabAlign::Decimal)], 0.5 * INCH);
    dialog.select(Some(1));
    dialog.edit_position(1, 0.5 * INCH);
    dialog.normalize();
    let result = dialog.result();
    assert_eq!(result.stops, [stop(0.5 * INCH, TabAlign::Decimal)]);

    // Rows moved past each other come back in position order, the selection on its stop.
    let mut dialog = TabStopsDialog::new(vec![stop(INCH, TabAlign::Left), stop(2.0 * INCH, TabAlign::Right)], 0.5 * INCH);
    dialog.select(Some(0));
    dialog.edit_position(0, 3.0 * INCH);
    dialog.normalize();
    assert_eq!(dialog.result().stops, [stop(2.0 * INCH, TabAlign::Right), stop(3.0 * INCH, TabAlign::Left)]);
}

// Clear on a stop moves it to cleared; OK returns the remaining stops sorted, the cleared positions and the interval.
#[test]
fn ok_returns_stops_cleared_and_interval() {
    let mut dialog = TabStopsDialog::new(vec![stop(INCH, TabAlign::Left), stop(2.0 * INCH, TabAlign::Right), stop(3.0 * INCH, TabAlign::Left)], 36.0);
    dialog.select(Some(1));
    dialog.clear();
    let result = dialog.result();
    assert_eq!(result.stops, [stop(INCH, TabAlign::Left), stop(3.0 * INCH, TabAlign::Left)]);
    assert_eq!(result.cleared, [2.0 * INCH]);
    assert_eq!(result.default_interval, 36.0);

    dialog.clear_all();
    let result = dialog.result();
    assert!(result.stops.is_empty());
    assert_eq!(result.cleared, [2.0 * INCH, INCH, 3.0 * INCH]);
}

#[derive(Default)]
struct App {
    host: DialogHost,
    answers: Vec<TabStopsResult>,
}

// Pressing Set in the open dialog and then OK answers with the extra stop.
#[test]
fn the_dialog_sets_a_stop_and_answers_ok() {
    let mut app = App::default();
    app.host.open(TabStopsDialog::new(vec![stop(INCH, TabAlign::Left)], 0.5 * INCH));
    let mut harness: Harness<'_, App> =
        HarnessBuilder::default().with_size(vec2(800.0, 800.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App| {
                app.host.show(ui.ctx());
                if let Some(DialogResult::Ok(answer)) = app.host.take::<TabStopsDialog>() {
                    app.answers.push(answer);
                }
            },
            app,
        );
    harness.run();
    harness.get_by_label("Set").click();
    harness.run();
    harness.get_by_label("OK").click();
    harness.run();
    let answers = &harness.state().answers;
    assert_eq!(answers.len(), 1);
    assert_eq!(answers.first().map(|a| a.stops.clone()), Some(vec![stop(INCH, TabAlign::Left), stop(1.5 * INCH, TabAlign::Left)]));
}

// Esc in the middle of a position edit puts the stop back at the position the edit began from, and a commit keeps the new one.
#[test]
fn cancelling_a_position_edit_restores_the_start() {
    let mut dialog = TabStopsDialog::new(vec![stop(INCH, TabAlign::Left)], 0.5 * INCH);
    let id = GestureId::next();
    dialog.apply_position(0, EditEvent::begin(id));
    dialog.apply_position(0, EditEvent::update(id, 3.0 * INCH, Modifiers::NONE));
    dialog.apply_position(0, EditEvent::cancel(id, CancelReason::Escape));
    assert_eq!(dialog.result().stops, [stop(INCH, TabAlign::Left)]);

    let id = GestureId::next();
    dialog.apply_position(0, EditEvent::begin(id));
    dialog.apply_position(0, EditEvent::update(id, 3.0 * INCH, Modifiers::NONE));
    dialog.apply_position(0, EditEvent::commit(id, 3.0 * INCH, true, Modifiers::NONE));
    assert_eq!(dialog.result().stops, [stop(3.0 * INCH, TabAlign::Left)]);
}

// The leader list reads as Word's: None, then each fill after its number.
#[test]
fn leader_options_carry_word_numbers() {
    let labels = TabStopsLabels::default();
    let texts: Vec<String> = TabLeader::ALL.iter().map(|kind| leader_text(&labels, *kind)).collect();
    assert_eq!(texts, ["None", "1 ……", "2 ------", "3 ______", "4 ······"]);
}
