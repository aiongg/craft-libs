//! One trial per shape: the steps run in the order of the pipeline, with the content step for a body and items.

use super::kit::{Panel, Probe, RowProbe, Rows, Seen, Table};
use craft_ui::contract::{Show as _, ShowCollection as _, ShowContainer as _};
use craft_ui::testing::contract::steps;
use craft_ui::testing::{Frames, trial};
use egui::Rect;
use std::cell::RefCell;
use std::sync::Mutex;

const LEAF: [&str; 16] = [
    "env", "style", "id", "value", "measure", "place", "parts", "sense", "update", "overlay", "apply", "state", "paint", "settle", "report", "finish",
];

const WITH_CONTENT: [&str; 17] = [
    "env", "style", "id", "value", "measure", "place", "parts", "sense", "update", "overlay", "apply", "state", "paint", "content", "settle",
    "report", "finish",
];

#[trial]
fn a_leaf_runs_every_step_but_content_in_order() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Seen::default());
    frames.frame(vec![], |ui| {
        Probe::new(&seen).show(ui);
    });
    let id = seen.borrow().id;
    assert_eq!(steps(&frames.ctx, id.expect("the leaf updated")), LEAF);
}

#[trial]
fn a_body_runs_all_17_steps_and_its_closure_once_per_pass() {
    let mut frames = Frames::new();
    let ran = RefCell::new(0);
    let mut id = None;
    let passes = frames.frame(vec![], |ui| {
        let out = Panel::new().show(ui, |ui, _| {
            *ran.borrow_mut() += 1;
            let seen = RefCell::new(Seen::default());
            Probe::new(&seen).show(ui);
        });
        assert!(out.inner.is_some(), "the body ran");
        id = Some(out.output.id);
    });
    assert_eq!(*ran.borrow(), passes, "the body ran once per pass");
    assert_eq!(steps(&frames.ctx, id.expect("the panel was shown")), WITH_CONTENT);
}

static ROWS: Mutex<Vec<(usize, Rect)>> = Mutex::new(Vec::new());

#[trial]
fn items_run_all_17_steps_and_show_a_row_per_visible_key() {
    let mut frames = Frames::new();
    if let Ok(mut rows) = ROWS.lock() {
        rows.clear();
    }
    let mut id = None;
    frames.frame(vec![], |ui| {
        id = Some(Table::new(20.0).show_items(ui, &Rows(4), &mut |key, _| RowProbe::new(*key, &ROWS)).id);
    });
    assert_eq!(steps(&frames.ctx, id.expect("the table was shown")), WITH_CONTENT);
    let rows = ROWS.lock().map(|rows| rows.clone()).unwrap_or_default();
    let last_pass: Vec<usize> = rows.iter().rev().take(4).rev().map(|(key, _)| *key).collect();
    assert_eq!(last_pass, [0, 1, 2, 3], "one row per key, in order");
    let tops: Vec<f32> = rows.iter().rev().take(4).rev().map(|(_, rect)| rect.top()).collect();
    assert!(tops.windows(2).all(|pair| pair[0] < pair[1]), "rows stack top to bottom: {tops:?}");
}
