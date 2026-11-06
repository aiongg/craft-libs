//! Galleys and layout jobs.

use egui::text::LayoutJob;
use egui::{Context, Galley};
use std::sync::Arc;

/// Lays `job` out with the context's fonts.
pub(crate) fn layout(ctx: &Context, job: LayoutJob) -> Arc<Galley> {
    ctx.fonts_mut(|fonts| fonts.layout_job(job))
}
