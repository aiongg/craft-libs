//! `show_viewport_immediate`, viewport-builder conversion and `embed_viewports`.

use egui::{Context, Ui, Vec2, ViewportBuilder, ViewportId};

/// Whether the integration opens OS viewports: false where egui embeds them in the root (web, tests).
pub(crate) fn available(ctx: &Context) -> bool {
    !ctx.embed_viewports()
}

/// Opens (or keeps) the OS viewport `id` and runs `body` once for each pass the viewport gets in this call.
pub(crate) fn show_immediate<T>(ctx: &Context, id: ViewportId, title: &str, size: Option<Vec2>, mut body: impl FnMut(&mut Ui) -> T) -> T {
    let mut builder = ViewportBuilder::default().with_title(title);
    if let Some(size) = size {
        builder = builder.with_inner_size(size);
    }
    ctx.show_viewport_immediate(id, builder, |ui, _| body(ui))
}

/// The viewport id of the container `id`.
pub(crate) fn viewport_id(id: egui::Id) -> ViewportId {
    ViewportId::from_hash_of(id)
}
