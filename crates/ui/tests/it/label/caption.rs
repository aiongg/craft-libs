//! The caption a beside label draws: its language's colon after a text label, and none for the other layouts.

use super::group::Group;
use super::kit::{Dial, Seen};
use craft_ui::contract::{FieldLayout, Props as _, ShowContainer as _, SideLabel};
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Rect};
use std::cell::RefCell;

const ROW: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(310.0, 140.0) };

#[trial]
fn a_beside_label_draws_a_colon_and_the_other_layouts_do_not() {
    let mut frames = Frames::new();
    let seen = [RefCell::new(Seen::default()), RefCell::new(Seen::default()), RefCell::new(Seen::default())];
    let mut value = 1.0;
    frames.idle(3, |ui| {
        Group::column().id_salt("captions").show_at(ui, ROW, |ui, cx| {
            cx.labelled(ui, "a", "Width".into(), SideLabel::new(), Dial::new(&mut value, &seen[0]));
            cx.labelled(ui, "b", "Height:".into(), SideLabel::new(), Dial::new(&mut 1.0, &seen[1]));
            cx.labelled(ui, "c", "Snap".into(), SideLabel::new().layout(FieldLayout::After), Dial::new(&mut 1.0, &seen[2]));
        });
    });
    let drawn = |text: &str| frames.texts().iter().any(|run| run.text == text);
    assert!(drawn("Width:"), "a beside label ends in a colon");
    assert!(drawn("Height:") && !drawn("Height::"), "a colon the text has is not doubled");
    assert!(drawn("Snap") && !drawn("Snap:"), "a label after its control has no colon");
    assert_eq!(seen[0].borrow().label.as_ref().map(|label| label.text.as_str()), Some("Width"), "the control's name has no colon");
}
