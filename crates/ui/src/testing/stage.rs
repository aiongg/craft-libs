//! The room a story that fills both axes is shown in.

use crate::layout::flow_room;
use crate::runtime::env::Env;
use egui::{Rect, Ui};

/// The height of a stage in the control heights of the ambient size, for a view that gives no bounded height (the
/// scrolled Overview).
const STAGE_CONTROLS: f32 = 15.0;

/// Allocates the stage of a story that fills both axes and returns its rect: the width left in `ui`, and the height
/// left when it is bounded, else [`STAGE_CONTROLS`] control heights. Show the story's root over it with `show_at`.
pub fn stage(ui: &mut Ui) -> Rect {
    let room = flow_room(ui);
    let env = Env::of(ui);
    let height = if room.y.is_finite() { room.y } else { STAGE_CONTROLS * env.control(env.size).height };
    let width = if room.x.is_finite() { room.x } else { ui.available_width() };
    crate::runtime::egui::alloc::space(ui, egui::vec2(width, height))
}
