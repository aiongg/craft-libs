//! [`ArrowheadsPopover`]: the Line tool's arrowhead settings.

use super::Arrowheads;
use craft_behavior::EditEvent;
use craft_ui::contract::Bind;
use egui::{Response, Ui};

/// The arrowhead popover: which ends carry a head and the head's width, length and concavity.
#[must_use = "show the popover with `show`"]
#[derive(craft_ui::contract::Pattern)]
pub struct ArrowheadsPopover<'a> {
    value: Bind<'a, Arrowheads>,
    open: Bind<'a, bool>,
}

/// What [`ArrowheadsPopover::show`] reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct ArrowheadsOutput {
    /// The button's response.
    pub response: Response,
    /// The edit made this pass, if any.
    pub edit: Option<EditEvent<Arrowheads>>,
}

impl<'a> ArrowheadsPopover<'a> {
    /// The popover of `value`, open while `open` is set.
    pub fn new(value: Bind<'a, Arrowheads>, open: Bind<'a, bool>) -> Self {
        ArrowheadsPopover { value, open }
    }

    /// Shows the button and, while open, the popover.
    pub fn show(self, ui: &mut Ui) -> ArrowheadsOutput {
        let ArrowheadsPopover { value, open } = self;
        let _ = (ui, value, open);
        todo!("tl.T29")
    }
}
