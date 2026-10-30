//! Windows that live in their own OS viewport, or inside the main window where viewports are
//! embedded.
//!
//! [`ViewportWindow`] is the one place the suite calls egui's viewport functions. The body is the
//! same in both hostings ([`Hosting`]); only the chrome differs: an OS window supplies its own
//! title bar and close box, an embedded one is a resizable [`Dialog`](crate::dialog::Dialog).

mod window;

pub use window::{Hosting, ViewportResponse, ViewportWindow};
