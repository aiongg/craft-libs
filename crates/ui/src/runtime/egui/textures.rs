//! Texture loading for `TextureCache`.

use egui::{ColorImage, Context, TextureHandle, TextureOptions};

/// Uploads `image` as a filtered texture called `name`. The texture lives as long as the handle.
pub(crate) fn load(ctx: &Context, name: String, image: ColorImage) -> TextureHandle {
    ctx.load_texture(name, image, TextureOptions::LINEAR)
}
