//! `Entry::disabled_reason`: a disabled option says why to assistive technology.

use craft_ui::contract::{Bind, Entry, Props as _, Show as _};
use craft_ui::pickers::Select;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};

const REASON: &str = "Needs a layer with a mask";

