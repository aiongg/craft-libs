//! Clipboard and open-URL output commands.

use egui::{ColorImage, Context};

/// Puts `text` on the system clipboard.
pub(crate) fn copy_text(ctx: &Context, text: String) {
    ctx.copy_text(text);
}

/// Asks the platform to open `url`, in a new tab where the platform has tabs and `new_tab` is set.
pub(crate) fn open_url(ctx: &Context, url: &str, new_tab: bool) {
    ctx.open_url(if new_tab { egui::OpenUrl::new_tab(url) } else { egui::OpenUrl::same_tab(url) });
}

/// Puts `image` on the system clipboard.
pub(crate) fn copy_image(ctx: &Context, image: ColorImage) {
    ctx.copy_image(image);
}

/// Asks the platform for the clipboard text, which arrives as a paste event.
pub(crate) fn request_paste(ctx: &Context) {
    ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
}
