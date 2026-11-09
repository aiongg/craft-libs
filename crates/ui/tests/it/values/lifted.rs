//! `EditResponse::lifted`: a composite's response carries the lifted event and clear flags.

use craft_behavior::{EditEvent, GestureId, Mixed};
use craft_ui::contract::{EditResponse, Show as _, Validity};
use craft_ui::display::Text;
use craft_ui::testing::{Frames, trial};

use std::sync::Arc;

const LABEL: &str = "Gain";
const BEFORE: f64 = 1.0;
const AFTER: f64 = 2.0;

