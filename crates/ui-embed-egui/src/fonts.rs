//! egui's fonts and glyph atlas as render-epaint's [`FontsAccess`].
use craft_ui_render_epaint::FontsAccess;

/// [`FontsAccess`] over an egui context's fonts: galleys come from the app's own fonts and atlas.
pub struct EguiFonts {
    ctx: egui::Context,
}

impl EguiFonts {
    pub fn new(ctx: egui::Context) -> EguiFonts {
        EguiFonts { ctx }
    }
}

impl FontsAccess for EguiFonts {
    fn with_fonts<R>(&mut self, _scale: f32, f: impl FnOnce(&mut epaint::text::FontsView<'_>) -> R) -> R { let _ = (&self.ctx, f); todo!() }
    fn with_definitions<R>(&self, f: impl FnOnce(&epaint::text::FontDefinitions) -> R) -> R { let _ = f; todo!() }
    fn replace_definitions(&mut self, _: epaint::text::FontDefinitions) -> bool { false }
    fn atlas_generation(&self) -> u64 { todo!() }
    fn atlas_size(&self) -> [usize; 2] { todo!() }
    fn tessellator(&self, _scale: f32, _options: epaint::TessellationOptions) -> epaint::Tessellator { todo!() }
    fn take_atlas_delta(&mut self) -> Option<epaint::ImageDelta> { None }
    fn begin_pass(&mut self) {}
}
