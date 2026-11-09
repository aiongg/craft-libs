//! The panel rail's request: which panel a click asks to open or close.

use craft_ui::rails::{PanelRail, PanelRailRequest, RailTool};
use craft_ui::runtime::commands::{CommandState, EditCommands};
use craft_ui::testing::{Fixture, trial};
use std::cell::RefCell;
use std::rc::Rc;

const UNDO: u8 = 1;
const REDO: u8 = 2;

