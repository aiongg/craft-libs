//! The window's size and position, kept by craft-window's window-restore store.

use std::sync::{Arc, Mutex, PoisonError};

use craft_window::restore::{self, Defaults, Monitor, Placement, Store, Tracker};
use egui::{Context, ViewportBuilder};

use super::{DEFAULT_SIZE, MIN_SIZE};

/// Where the Preferences window opens and what keeps that up to date: one entry of the app's
/// window-restore [`Store`] under `key`.
///
/// The placement is worked out once when the window opens ([`restore::place`]: an
/// off-screen rectangle moves onto a connected monitor and the size is fitted to it; nothing
/// saved gives 1040×800) and held until the window closes. While it is open a [`Tracker`] writes
/// the user's size and position back, as for the app's other windows.
pub struct Remember {
    store: Arc<Mutex<Store>>,
    key: String,
    monitors: Vec<Monitor>,
    defaults: Defaults,
    placed: Option<Placement>,
    tracker: Option<Tracker>,
}

impl Remember {
    /// Remembers the window under `key` in `store`, the one store the app shares between its
    /// windows.
    pub fn new(store: &Arc<Mutex<Store>>, key: impl Into<String>) -> Self {
        Self {
            store: Arc::clone(store),
            key: key.into(),
            monitors: Vec::new(),
            defaults: Defaults::new(DEFAULT_SIZE.into(), MIN_SIZE.into()),
            placed: None,
            tracker: None,
        }
    }

    /// The connected monitors, for the off-screen check. Without them the saved geometry is
    /// used as it is.
    pub fn monitors(mut self, monitors: Vec<Monitor>) -> Self {
        self.monitors = monitors;
        self
    }

    /// Replaces the default and least size and the platform rules (Wayland: no position).
    pub fn defaults(mut self, defaults: Defaults) -> Self {
        self.defaults = defaults;
        self
    }

    /// The placement the window opens with.
    pub fn placement(&mut self) -> &Placement {
        let (store, key, monitors, defaults) = (&self.store, &self.key, &self.monitors, &self.defaults);
        self.placed.get_or_insert_with(|| {
            let saved = store.lock().unwrap_or_else(PoisonError::into_inner).get(key);
            restore::place(saved.as_ref(), monitors, None, defaults)
        })
    }

    pub(super) fn apply(&mut self, builder: ViewportBuilder, zoom: f32) -> ViewportBuilder {
        restore::apply(builder, self.placement(), zoom)
    }

    /// Call every pass from the window's own viewport.
    pub(super) fn track(&mut self, ctx: &Context) {
        let (store, key, defaults) = (&self.store, &self.key, &self.defaults);
        self.tracker.get_or_insert_with(|| Tracker::new(store, key, defaults)).frame(ctx);
    }

    /// The window closed: the next open works out its placement afresh.
    pub(super) fn close(&mut self) {
        self.placed = None;
        self.tracker = None;
    }
}
