//! `Slicer`: a tile click toggles its value, the funnel clears.

use std::cell::RefCell;
use std::rc::Rc;

use craft_table::ui::{Slicer, SlicerAction};
use craft_ui::testing::{Fixture, trial};

