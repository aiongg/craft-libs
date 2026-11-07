//! Clipboard and open-URL output commands.

use egui::{ColorImage, Context, Event};
use std::sync::Arc;

/// Puts `text` on the system clipboard.
pub(crate) fn copy_text(ctx: &Context, text: String) {
    ctx.copy_text(text);
}

/// Puts `image` on the system clipboard.
pub(crate) fn copy_image(ctx: &Context, image: ColorImage) {
    ctx.copy_image(image);
}

/// The text of the paste event this pass carries.
pub(crate) fn pasted_text(ctx: &Context) -> Option<Arc<str>> {
    ctx.input(|input| {
        input.events.iter().find_map(|event| match event {
            Event::Paste(text) => Some(Arc::<str>::from(text.as_str())),
            _ => None,
        })
    })
}
