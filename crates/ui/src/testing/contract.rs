//! What the contract trials read: the decision records and findings of the pipeline.

use crate::runtime::inspect::{self, Finding};
use egui::{Context, Id};

/// The steps the latest show of `id` ran, in order.
pub fn steps(ctx: &Context, id: Id) -> Vec<&'static str> {
    inspect::steps_of(ctx, id)
}

/// Every finding reported on `ctx`.
pub fn findings(ctx: &Context) -> Vec<Finding> {
    inspect::findings(ctx)
}

/// The reveal request addressed to the scroll owner `owner`, once.
pub fn take_reveal(ctx: &Context, owner: Id) -> Option<egui::Rect> {
    crate::contract::take_reveal(ctx, owner)
}
