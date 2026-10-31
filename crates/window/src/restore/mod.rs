//! Window restore: where a window opens and the file that remembers it. The core has no egui.
//!
//! - [`place`]: saved [`Placement`] + connected [`Monitor`]s + app [`Defaults`] → the geometry to
//!   open with. Pure; the rules are on its documentation.
//! - [`Store`]: `window.json` beside the app's `config.toml`, one [`Placement`] per window key
//!   (`"main"` for the first window).
//! - [`legacy`]: readers for what apps stored before, such as eframe's `"window"` entry in
//!   `app.ron`.
//! - With `restore-ui` or `restore-ui-glow`: [`install`] opens an eframe app's root window
//!   from the store, [`Tracker`] keeps a window's entry up to date, and [`cascade_window`] places
//!   further windows of a multi-window app from [`MAIN`].
//!
//! Keep one [`Store`] per app, shared by every window (for example behind an `Arc<Mutex<_>>`):
//! each write rewrites the whole file from that store's copy, so two stores in one process
//! overwrite each other.
//!
//! ```
//! use craft_window::restore::{Defaults, Monitor, MonitorKey, Store, place};
//!
//! let mut store = Store::in_memory();
//! let monitors = [Monitor::new(MonitorKey::new("DP-1", [1920, 1080]), [0, 0], [1920, 1080], 1.0)];
//! let defaults = Defaults::new([1200.0, 800.0], [480.0, 320.0]);
//! let opened = place(store.get("main").as_ref(), &monitors, Some(0), &defaults);
//! store.set("main", &opened);
//! assert_eq!(store.get("main"), Some(opened));
//! ```

#[cfg(feature = "restore-eframe")]
mod egui;
pub mod legacy;
mod place;
mod store;
#[cfg(feature = "restore-eframe")]
mod viewports;

#[cfg(all(feature = "restore-eframe", not(target_arch = "wasm32")))]
pub use self::egui::install;
#[cfg(feature = "restore-eframe")]
pub use self::egui::{Tracker, apply};
pub use place::{Defaults, Monitor, MonitorKey, NormalRect, Placement, place};
pub use store::{FILE_NAME, MAIN, Pending, ReadOnly, SCHEMA_VERSION, Store, write_pending};
#[cfg(feature = "restore-eframe")]
pub use viewports::cascade_window;
