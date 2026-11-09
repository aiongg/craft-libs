//! The panel rail's request: which panel a click asks to open or close.

use craft_ui::rails::{PanelRail, PanelRailRequest, RailTool};
use craft_ui::runtime::commands::{CommandState, EditCommands};
use craft_ui::testing::{Fixture, trial};
use std::cell::RefCell;
use std::rc::Rc;

const UNDO: u8 = 1;
const REDO: u8 = 2;

#[trial]
fn a_click_on_a_panels_icon_asks_to_open_or_close_it() {
    {
        let requests = Rc::new(RefCell::new(Vec::new()));
        let seen = requests.clone();
        let mut fixture = Fixture::new(move |ui| {
            let panels = [RailTool::new(UNDO, EditCommands::Undo), RailTool::new(REDO, EditCommands::Redo)];
            if let Some(request) = PanelRail::new("Panels", &panels).show_rail(ui, |_| {}).request {
                seen.borrow_mut().push(request);
            }
        });
        fixture.settle();
        assert!(fixture.by_label("Undo").is_some(), "the first panel has an icon");
        let icon = fixture.by_label("Redo").expect("the second panel has an icon");
        fixture.click(&icon);
        fixture.idle(3);
        assert_eq!(*requests.borrow(), [PanelRailRequest::Open(REDO)]);
    }
    {
        let requests = Rc::new(RefCell::new(Vec::new()));
        let seen = requests.clone();
        let mut fixture = Fixture::new(move |ui| {
            let panels = [RailTool::new(UNDO, EditCommands::Undo), RailTool::new(REDO, EditCommands::Redo)];
            if let Some(request) = PanelRail::new("Panels", &panels).show_rail(ui, |_| {}).request {
                seen.borrow_mut().push(request);
            }
        });
        fixture.command_state(EditCommands::Redo, CommandState::new().checked(true));
        fixture.settle();
        let icon = fixture.by_label("Redo").expect("the second panel has an icon");
        fixture.click(&icon);
        assert_eq!(*requests.borrow(), [PanelRailRequest::Close(REDO)]);
    }
}
