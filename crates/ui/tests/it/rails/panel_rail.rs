//! The panel rail's request: which panel a click asks to open or close.

use craft_ui::contract::{Role, Show as _};
use craft_ui::rails::{PanelRail, PanelRailRequest, RailTool};
use craft_ui::runtime::commands::EditCommands;
use craft_ui::testing::{Fixture, trial};
use std::cell::RefCell;
use std::rc::Rc;

const UNDO: u8 = 1;
const REDO: u8 = 2;

#[trial]
fn a_click_on_a_closed_panels_icon_asks_to_open_that_panel() {
    let requests = Rc::new(RefCell::new(Vec::new()));
    let seen = requests.clone();
    let mut fixture = Fixture::new(move |ui| {
        let panels = [RailTool::new(UNDO, EditCommands::Undo), RailTool::new(REDO, EditCommands::Redo)];
        if let Some(request) = PanelRail::new("Panels", &panels).show_rail(ui, |_| {}).request {
            seen.borrow_mut().push(request);
        }
    });
    fixture.settle();
    let icons = fixture.by_role(Role::BUTTON);
    assert_eq!(icons.len(), 2, "one icon per panel");
    fixture.click(&icons[1]);
    fixture.idle(3);
    assert_eq!(*requests.borrow(), [PanelRailRequest::Open(REDO)]);
}
