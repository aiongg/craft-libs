//! egui's fonts and glyph atlas as render-epaint's [`FontsAccess`]: rt text is shaped by the
//! app's own fonts, its glyphs land in egui's atlas (texture `TextureId::default()`, uploaded by
//! egui at the end of the pass), and they look exactly like the app's other text.

use craft_ui_render_epaint::FontsAccess;
use epaint::text::{FontDefinitions, FontsView};
use epaint::{ImageDelta, TessellationOptions, Tessellator};

/// [`FontsAccess`] over an egui context's fonts. They serve the context's own scale, which the
/// bridge gives the rt window. Never called inside another `Context` lock.
pub struct EguiFonts {
    ctx: egui::Context,
}

impl EguiFonts {
    pub fn new(ctx: egui::Context) -> EguiFonts {
        EguiFonts { ctx }
    }
}

impl FontsAccess for EguiFonts {
    fn with_fonts<R>(&mut self, _scale: f32, f: impl FnOnce(&mut FontsView<'_>) -> R) -> R {
        self.ctx.fonts_mut(f)
    }

    fn with_definitions<R>(&self, f: impl FnOnce(&FontDefinitions) -> R) -> R {
        self.ctx.fonts(|fonts| f(fonts.definitions()))
    }

    /// The app owns its font definitions.
    fn replace_definitions(&mut self, _: FontDefinitions) -> bool {
        false
    }

    fn atlas_generation(&self) -> u64 {
        self.ctx.fonts(|fonts| fonts.atlas_generation())
    }

    fn atlas_size(&self) -> [usize; 2] {
        self.ctx.fonts(|fonts| fonts.font_image_size())
    }

    /// Without the atlas' prepared discs, which egui does not expose: small circles are
    /// tessellated as paths.
    fn tessellator(&self, scale: f32, options: TessellationOptions) -> Tessellator {
        Tessellator::new(scale, options, self.atlas_size(), Vec::new())
    }

    /// egui uploads its atlas.
    fn take_atlas_delta(&mut self) -> Option<ImageDelta> {
        None
    }

    /// egui's own pass flushes galleys and clears a full atlas.
    fn begin_pass(&mut self) {}
}
