//! The clipboard handle `Commands::clipboard` returns, over `runtime::egui::clipboard`.

use crate::runtime::egui::clipboard;
use egui::{ColorImage, Context};
use std::sync::Arc;

/// A result that may not have arrived.
// F4-MERGE: motion track (`Pending` and background results, chapter 14)
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Pending<T, E> {
    /// Not there yet.
    #[non_exhaustive]
    Loading {
        /// How far along it is, when known.
        progress: Option<f32>,
    },
    /// The result.
    Ready(T),
    /// The reason there is none.
    Failed(E),
}

/// One clipboard item.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ClipItem {
    /// Plain text.
    Text(Arc<str>),
    /// HTML.
    Html(Arc<str>),
    /// An image.
    Image(Arc<ColorImage>),
    /// An item of an app-defined type.
    #[non_exhaustive]
    Custom {
        /// The MIME type.
        mime: Arc<str>,
        /// The content.
        bytes: Arc<[u8]>,
    },
}

/// What a paste delivers.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct ClipContent {
    /// The clipboard's text, when it holds any.
    pub text: Option<Arc<str>>,
}

/// Why a paste has no content.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ClipError(pub String);

/// The component's paste slot over the system clipboard. egui carries text and images, so `Html` and `Custom`
/// items are not written.
pub struct ClipboardHandle {
    ctx: Context,
    slot: Pending<ClipContent, ClipError>,
}

impl ClipboardHandle {
    pub(crate) fn new(ctx: &Context) -> Self {
        let slot = match clipboard::pasted_text(ctx) {
            Some(text) => Pending::Ready(ClipContent { text: Some(text) }),
            None => Pending::Loading { progress: None },
        };
        ClipboardHandle { ctx: ctx.clone(), slot }
    }

    /// Writes every flavour of `items` that the clipboard carries.
    pub fn put(&self, items: &[ClipItem]) {
        for item in items {
            match item {
                ClipItem::Text(text) => clipboard::copy_text(&self.ctx, text.to_string()),
                ClipItem::Image(image) => clipboard::copy_image(&self.ctx, (**image).clone()),
                ClipItem::Html(_) | ClipItem::Custom { .. } => {}
            }
        }
    }

    /// The pasted content: ready in the pass that carries the paste event.
    pub fn paste(&self) -> &Pending<ClipContent, ClipError> {
        &self.slot
    }
}
