//! Window restore: where a window opens and the file that remembers it. No egui.
//!
//! - [`place`]: saved [`Placement`] + connected [`Monitor`]s + app [`Defaults`] → the geometry to
//!   open with. Pure; the rules are on its documentation.
//! - [`Store`]: `window.json` beside the app's `config.toml`, one [`Placement`] per window key
//!   (`"main"` for the first window).
//! - [`legacy`]: readers for what apps stored before, such as eframe's `"window"` entry in
//!   `app.ron`.
//!
//! ```
//! use craft_window::restore::{Defaults, Monitor, MonitorKey, Store, place};
//!
//! let mut store = Store::in_memory();
//! let monitors = [Monitor { key: MonitorKey::new("DP-1", [1920, 1080]), pos_px: [0, 0], size_px: [1920, 1080], scale: 1.0 }];
//! let defaults = Defaults::new([1200.0, 800.0], [480.0, 320.0]);
//! let opened = place(store.get("main").as_ref(), &monitors, Some(0), &defaults);
//! store.set("main", &opened);
//! assert_eq!(store.get("main"), Some(opened));
//! ```

pub mod legacy;
mod place;
mod store;

pub use place::{Defaults, Monitor, MonitorKey, Placement, Rect, place};
pub use store::{FILE_NAME, MAIN, ReadOnly, SCHEMA_VERSION, Store};
