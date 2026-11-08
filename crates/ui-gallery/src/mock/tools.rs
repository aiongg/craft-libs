//! What the window keeps across passes in its root container: the canvas tools.

use craft_behavior::CancelReason;
use craft_ui::canvas::{CanvasTool, HandTool, Momentary, NavigationOptions, ToolAction, ToolContext, ToolPath, ToolPointer, ZoomTool};
use craft_ui::primitives::Draw;

/// The tools the canvas runs: Hand, Zoom and the momentary pair of both.
#[derive(Clone, Debug)]
pub(super) struct Tools {
    pub(super) hand: HandTool,
    pub(super) zoom: ZoomTool,
    pub(super) momentary: Momentary,
}

impl Default for Tools {
    fn default() -> Self {
        let options = NavigationOptions::default();
        Tools { hand: HandTool::new(options), zoom: ZoomTool::new(options), momentary: Momentary::new(options) }
    }
}

/// The freehand tools of the rail: they take the press and draw nothing.
#[derive(Debug)]
pub(super) struct Freehand;

impl CanvasTool for Freehand {
    fn press(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) -> ToolAction {
        ToolAction::Ignored
    }

    fn drag(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {}

    fn release(&mut self, _: &ToolPointer, _: &mut ToolContext<'_>) {}

    fn cancel(&mut self, _: CancelReason, _: &mut ToolContext<'_>) {}

    fn paint(&self, _: &Draw<'_>, _: &ToolContext<'_>) {}

    fn command_form(&self) -> ToolPath {
        ToolPath::Freehand
    }
}
