//! `DropVeil`: files dropped anywhere in the window reach the output once, and a hover that ends without a drop
//! reports nothing.

use craft_ui::contract::{Bind, OsFiles, Props as _, Show as _, ShowContainer as _};
use craft_ui::choice::Checkbox;
use craft_ui::popover::DropVeil;
use craft_ui::testing::pointer::moved;
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Ui};
use std::cell::RefCell;

fn show(ui: &mut Ui, dropped: &RefCell<Vec<OsFiles>>) {
    let out = DropVeil::new("Import 2 photos")
        .show(ui, |ui, _| {
            Checkbox::new(Bind::given(false)).label("Keep originals").show(ui);
        });
    dropped.borrow_mut().extend(out.dropped.clone());
}

#[trial]
fn dropped_files_reach_the_output_once() {
    let mut frames = Frames::new();
    let dropped = RefCell::new(Vec::new());
    let mut run = |ui: &mut Ui| show(ui, &dropped);
    frames.frame(vec![moved(Pos2::new(300.0, 200.0))], &mut run);
    frames.hover_files(&["a.png", "b.png"]);
    frames.idle(3, &mut run);
    assert!(dropped.borrow().is_empty(), "hovering files are not a drop");
    frames.drop_files(&["a.png", "b.png"]);
    frames.idle(4, &mut run);
    let dropped = dropped.borrow();
    assert_eq!(dropped.len(), 1, "one drop in one pass");
    let names: Vec<_> = dropped[0].files.iter().map(|file| file.name.as_str()).collect();
    assert_eq!(names, ["a.png", "b.png"]);
}

#[trial]
fn files_that_leave_without_a_drop_report_nothing() {
    let mut frames = Frames::new();
    let dropped = RefCell::new(Vec::new());
    let mut run = |ui: &mut Ui| show(ui, &dropped);
    frames.frame(vec![moved(Pos2::new(300.0, 200.0))], &mut run);
    frames.hover_files(&["a.png"]);
    frames.idle(3, &mut run);
    frames.leave_files();
    frames.idle(4, &mut run);
    assert!(dropped.borrow().is_empty());
}
