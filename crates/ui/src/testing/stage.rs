//! The room a story that fills both axes is shown in.

use crate::layout::flow_room;
use crate::runtime::env::Env;
use egui::{Rect, Ui};

/// Allocates the stage of a story of a canvas control or a root that fills both axes, and returns its rect: the width
/// left in `ui` between the least and greatest stage width, and the stage height, all from the size tokens. Show the
/// story's root over it with `show_at`.
pub fn stage(ui: &mut Ui) -> Rect {
    let env = Env::of(ui);
    let canvas = env.tokens.tokens().size.canvas;
    let room = flow_room(ui).x;
    let room = if room.is_finite() { room } else { ui.available_width() };
    let width = room.clamp(canvas.stage_width_min, canvas.stage_width_max);
    crate::runtime::egui::alloc::space(ui, egui::vec2(width, canvas.stage_height))
}
