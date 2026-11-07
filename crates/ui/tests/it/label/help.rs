//! A footer help line is part of the control's description.

use super::group::Group;
use super::kit::{Dial, Seen};
use craft_ui::contract::{ContainerProps, Props as _, Show as _, ShowContainer as _, SideLabel};
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Rect};
use std::cell::RefCell;

const ROW: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(310.0, 80.0) };

#[trial]
fn a_help_line_joins_the_description_of_the_control() {
    let mut frames = Frames::new();
    let seen = RefCell::new(Seen::default());
    let mut value = 1.0;
    frames.idle(3, |ui| {
        Group::new(ContainerProps::new().label(SideLabel::new()).help("Between 0 and 100")).id_salt("help").show_at(ui, ROW, |ui, _| {
            Dial::new(&mut value, &seen).show(ui);
        });
    });
    let id = seen.borrow().id.unwrap_or_else(|| panic!("the dial ran in a label scope"));
    assert!(frames.tree().description(id).is_some_and(|text| text.contains("Between 0 and 100")), "the help is in the description");
}
