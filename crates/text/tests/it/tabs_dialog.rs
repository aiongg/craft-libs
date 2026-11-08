//! The Tabs dialog: Set, in-place edits, the result and the leader options.

use craft_text::ui::ruler::{TabAlign, TabLeader, TabStop};
use craft_text::ui::tabs_dialog::{TabStopsDialog, TabStopsWords, leader_text};
use craft_ui::dialog::{DialogResult, DialogStack, StackDialog as _};
use craft_ui::runtime::env::Env;
use craft_ui::testing::{Fixture, trial};
use std::cell::RefCell;
use std::rc::Rc;

fn stop(position: f32) -> TabStop {
    TabStop::new(position, TabAlign::Left, TabLeader::None)
}

fn positions(dialog: &mut TabStopsDialog) -> Vec<f32> {
    dialog.result().stops.iter().map(|stop| stop.position).collect()
}

#[trial]
fn set_adds_the_next_free_stop() {
    let mut dialog = TabStopsDialog::new(vec![stop(72.0)], 36.0);
    dialog.new_stop();
    dialog.new_stop();
    assert_eq!(positions(&mut dialog), vec![72.0, 108.0, 144.0]);
    dialog.select(Some(0));
    dialog.clear();
    assert_eq!(dialog.result().cleared, vec![72.0]);
}

#[trial]
fn editing_in_place_merges_on_normalize() {
    let mut dialog = TabStopsDialog::new(vec![stop(72.0), stop(144.0), stop(216.0)], 36.0);
    dialog.edit_position(2, 72.2);
    // The rows keep their order until the edit ends, then the edited row wins over the stop it landed on.
    dialog.normalize();
    assert_eq!(positions(&mut dialog), vec![72.2, 144.0]);
}

#[trial]
fn ok_returns_stops_cleared_and_interval() {
    let mut dialog = TabStopsDialog::new(vec![stop(72.0), stop(144.0)], 36.0);
    dialog.select(Some(1));
    dialog.clear();
    let result = dialog.result();
    assert_eq!(result.stops, vec![stop(72.0)]);
    assert_eq!(result.cleared, vec![144.0]);
    assert!((result.default_interval - 36.0).abs() < f32::EPSILON);
    dialog.clear_all();
    assert_eq!(dialog.result().cleared, vec![144.0, 72.0]);
}

#[trial]
fn the_dialog_sets_a_stop_and_answers_ok() {
    let dialog = Rc::new(RefCell::new(TabStopsDialog::new(Vec::new(), 36.0)));
    let answer = Rc::new(RefCell::new(None));
    let (held, taken) = (dialog.clone(), answer.clone());
    let mut fixture = Fixture::new(move |ui| {
        if let Some(result) = held.borrow_mut().show(ui, &DialogStack::default()) {
            *taken.borrow_mut() = Some(result);
        }
    });
    fixture.settle();
    let set = fixture.by_label("Set").expect("the Set button is a node");
    fixture.click(&set);
    let ok = fixture.by_label("OK").expect("the OK button is a node");
    fixture.click(&ok);
    match answer.borrow().clone() {
        Some(DialogResult::Ok(result)) => assert_eq!(result.stops.len(), 1),
        other => panic!("expected OK with a stop, got {other:?}"),
    }
}

#[trial]
fn leader_options_carry_word_numbers() {
    let texts = Rc::new(RefCell::new(Vec::new()));
    let seen = texts.clone();
    let mut fixture = Fixture::new(move |ui| {
        let words = Env::of(ui).words::<TabStopsWords>();
        *seen.borrow_mut() = TabLeader::ALL.iter().map(|leader| leader_text(&words, *leader)).collect::<Vec<_>>();
    });
    fixture.settle();
    assert_eq!(*texts.borrow(), vec!["None", "1 ……", "2 ------", "3 ______", "4 ······"]);
}
