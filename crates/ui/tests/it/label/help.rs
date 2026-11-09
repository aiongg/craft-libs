//! A footer help line is part of the control's description.

use super::group::Group;
use super::kit::{Dial, Seen};
use craft_ui::contract::{ContainerProps, Props as _, Show as _, ShowContainer as _, SideLabel};
use craft_ui::testing::{Frames, trial};
use egui::{Pos2, Rect};
use std::cell::RefCell;

const ROW: Rect = Rect { min: Pos2::new(10.0, 10.0), max: Pos2::new(310.0, 80.0) };

