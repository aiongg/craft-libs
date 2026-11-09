//! `FilterButton`: Alt+Down on the focused button opens the filter menu.

use std::cell::RefCell;
use std::rc::Rc;

use craft_table::ui::{FilterButton, FilterButtonEvent};
use craft_ui::contract::Show as _;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};

