//! Commands as the contexts of a container body and the fixture's command input give them.

use super::kit::{Dock, Pane, PaneCommand};
use craft_behavior::Mixed;
use craft_ui::buttons::IconButton;
use craft_ui::contract::{Show as _, ShowContainer as _};
use craft_ui::runtime::commands::{CommandState, Commands as _};
use craft_ui::testing::{Fixture, trial};
use std::cell::RefCell;
use std::rc::Rc;

