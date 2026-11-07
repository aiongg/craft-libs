//! The ghost layer's painter: a layer above every popup that takes no input.

use egui::{Context, Id, LayerId, Order, Painter};

// F5-MERGE: F5a (the overlay host owns the ghost layer; this is its one call)
/// The painter of the ghost layer, over the whole screen.
pub(crate) fn painter(ctx: &Context) -> Painter {
    ctx.layer_painter(LayerId::new(Order::Tooltip, Id::unique("craft_ui.ghost")))
}
