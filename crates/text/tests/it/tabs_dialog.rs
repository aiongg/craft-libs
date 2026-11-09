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

