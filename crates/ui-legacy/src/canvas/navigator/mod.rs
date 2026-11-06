//! The Navigator panel body (F13.04): a preview of the whole document with the view's proxy
//! rectangle, and the zoom row (Photoshop's Window › Navigator).
//!
//! The app converts its own view into a [`CanvasView`](super::CanvasView) once per pass, as for
//! rulers and scrollbars, and hands it to [`Navigator::show`], which edits it:
//!
//! ```ignore
//! let mut view = app_view.to_canvas_view(canvas_rect);
//! let out = Navigator::new("navigator", doc_size, &numbers)
//!     .preview(NavigatorPreview::Texture(cached))
//!     .navigation(&prefs.navigation)
//!     .settings(&mut ui_state.navigator)
//!     .show(ui, &mut view);
//! if out.view_changed {
//!     app_view.set_from(&view);
//!     if out.animate_zoom { ZoomAnimation::begin(ctx, canvas_id, &before, None); }
//! }
//! if out.settings_changed { prefs.save(&ui_state.navigator); }
//! ```
//!
//! | Part | Names |
//! |---|---|
//! | The widget | [`Navigator`], [`NavigatorPreview`], [`NavigatorOutput`], [`NavigatorLabels`], [`NavigatorStyle`] |
//! | Geometry, for tests and apps with their own preview | [`preview_rect`], [`proxy_quad`], [`zoom_to_document_rect`], [`view_center_for`] |
//! | Panel Options and the panel menu | [`NavigatorSettings`], [`NavigatorOptions`], [`ProxyColor`], [`NavigatorOptionsDialog`], [`NavigatorPanel`], [`NavigatorMenuAction`] |
//!
//! The preview picture is a [`CachedTexture`](super::textures::CachedTexture) from the app's own
//! [`TextureCache`](super::textures::TextureCache) (damage updates through
//! [`put_region`](super::textures::TextureCache::put_region)), a paint callback (PhotoCraft's GPU
//! canvas), or nothing yet. The app never hands the widget a `TextureHandle`.

mod labels;
mod menu;
mod options;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod options_tests;
mod preview;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod preview_tests;
mod proxy;
#[cfg(test)]
mod proxy_tests;
pub(crate) mod stories;
mod style;
mod zoom_row;

pub use labels::NavigatorLabels;
pub use menu::{NavigatorMenuAction, NavigatorPanel};
pub use options::{NAVIGATOR_PROXY_COLOR_KEY, NavigatorOptions, NavigatorOptionsDialog, NavigatorSettings, ProxyColor};
pub use preview::{Navigator, NavigatorOutput, NavigatorPaint, NavigatorPreview};
pub use proxy::{preview_rect, proxy_quad, view_center_for, zoom_to_document_rect};
pub use style::NavigatorStyle;
