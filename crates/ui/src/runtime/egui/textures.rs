//! Texture loading for `TextureCache`.

use egui::{Color32, ColorImage, Context, Painter, Rect, TextureHandle, TextureId, TextureOptions};

/// Uploads `image` as a filtered texture called `name`. The texture lives as long as the handle.
pub(crate) fn load(ctx: &Context, name: String, image: ColorImage) -> TextureHandle {
    ctx.load_texture(name, image, TextureOptions::LINEAR)
}

/// Paints `texture` in `rect`, the part `uv` of it, tinted by `tint`.
pub(crate) fn paint(painter: &Painter, texture: TextureId, rect: Rect, uv: Rect, tint: Color32) {
    painter.image(texture, rect, uv, tint);
}
