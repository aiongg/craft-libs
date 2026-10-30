use egui::{Pos2, Response};

/// What a [`Popover`](super::Popover) hangs from.
#[derive(Clone, Debug)]
pub enum PopoverAnchor {
    /// A control: the popover opens below it, or above it when there is more room there, and keeps
    /// that side while open. It starts at the control's leading edge, shifted to stay on screen.
    /// The control gets `has_popup` and `expanded` in AccessKit.
    Control(Response),
    /// A point (a right-click): the popover's top leading corner is at the point, shifted so the
    /// whole popover stays inside the window.
    Point(Pos2),
}
