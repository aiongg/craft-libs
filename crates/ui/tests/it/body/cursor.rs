//! A body sets the cursor in the pass it learns what the cursor should be, through the one cursor service: a body
//! whose panes report a refused drag shows the refusal over its parts' own cursors.

use super::kit::Slab;
use craft_behavior::{Cursor, CursorIcon, CursorPriority, PartKey, Sense};
use craft_ui::contract::{ContainerProps, Props as _, ShowContainer as _};
use craft_ui::runtime::cursor::current;
use craft_ui::testing::pointer;
use craft_ui::testing::sensing::{ProbePart, ProbeRoot, show};
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Rect, Ui, Vec2};
use std::cell::Cell;

#[trial]
fn a_body_cursor_above_a_parts_cursor_wins_in_the_pass_it_is_set() {
    let area = Rect::from_min_size(Pos2::new(40.0, 60.0), Vec2::new(120.0, 40.0));
    let grip = ProbeRoot::new("grip", area, Sense::HOVER).part(ProbePart::new(PartKey::new("grip"), area, Sense::HOVER).cursor(CursorIcon::Grab));
    let refused = Cell::new(false);
    let mut show_frame = |ui: &mut Ui| {
        Slab::new(ContainerProps::new()).id_salt("panel").show(ui, |ui, body| {
            show(ui, std::slice::from_ref(&grip));
            // The body learns of the refusal after its children ran.
            if refused.get() {
                body.cursor(CursorIcon::NotAllowed, CursorPriority::Drag);
            }
        });
    };
    let mut frames = Frames::new();
    frames.idle(2, &mut show_frame);
    frames.frame(vec![pointer::moved(area.center())], &mut show_frame);
    frames.idle(1, &mut show_frame);
    assert_eq!(current(&frames.ctx), Some(Cursor::System(CursorIcon::Grab)), "the hovered part shows its own cursor");
    refused.set(true);
    frames.idle(1, &mut show_frame);
    assert_eq!(current(&frames.ctx), Some(Cursor::System(CursorIcon::NotAllowed)), "the body's cursor wins in the pass it is set");
    refused.set(false);
    frames.idle(1, &mut show_frame);
    assert_eq!(current(&frames.ctx), Some(Cursor::System(CursorIcon::Grab)), "a pass that sets no body cursor keeps the part's");
}
