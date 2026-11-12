//! The app's display caches: the monitor list, the profile each display resolves to, and the
//! [`DisplayTransform`] per composite profile and display. One per app, shared by every window.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use craft_cms::{CmsError, Profile};

use crate::monitor::{self, Detection, DisplayId, Monitor, MonitorReport, MonitorsError, Resolved};
use crate::transform::{DisplayTransform, signature};

/// Most cached transforms before the cache is cleared.
const MAX_TRANSFORMS: usize = 32;

#[derive(Default)]
struct State {
    monitors: Vec<Monitor>,
    detection: Detection,
    chosen: Option<Resolved>,
    resolved: HashMap<DisplayId, Resolved>,
}

/// The monitor list, the monitor profile per display and the display transform per composite
/// profile and display. `Sync`.
#[derive(Default)]
pub struct DisplayCaches {
    state: Mutex<State>,
    transforms: Mutex<HashMap<(u64, DisplayId), Arc<DisplayTransform>>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

const _: () = {
    const fn shared<T: Send + Sync>() {}
    shared::<DisplayCaches>();
};

impl DisplayCaches {
    /// Empty caches: no monitors read, no profile chosen.
    pub fn new() -> DisplayCaches {
        DisplayCaches::default()
    }

    fn changed(&self, state: &mut State) {
        state.resolved.clear();
        lock(&self.transforms).clear();
    }

    /// The monitors are being read for the first time (a re-read keeps the earlier list
    /// meanwhile, so this changes nothing once a list is there).
    pub fn set_pending(&self) {
        let mut state = lock(&self.state);
        if state.monitors.is_empty() {
            state.detection = Detection::Pending;
            self.changed(&mut state);
        }
    }

    /// Records the platform's reading of the monitors. A failed reading keeps a list read before
    /// ([`Detection::Retained`]) and returns its reason for the log.
    pub fn set_monitors(&self, read: Result<Vec<Monitor>, MonitorsError>) -> Option<String> {
        let mut state = lock(&self.state);
        let reason = match read {
            Ok(list) if !list.is_empty() => {
                state.monitors = list;
                state.detection = Detection::Found;
                self.changed(&mut state);
                return None;
            }
            Ok(_) => "the platform reported no monitors".to_string(),
            Err(MonitorsError::Failed(reason)) => reason,
            Err(MonitorsError::Unsupported) => {
                if state.monitors.is_empty() {
                    state.detection = Detection::Unsupported;
                    self.changed(&mut state);
                }
                return None;
            }
        };
        if state.monitors.is_empty() {
            state.detection = Detection::Failed { reason };
            self.changed(&mut state);
            None
        } else {
            state.detection = Detection::Retained { reason: reason.clone() };
            self.changed(&mut state);
            Some(reason)
        }
    }

    /// The app's choice of monitor profile (Color Settings), or `None` to use each display's own.
    pub fn choose_monitor(&self, profile: Option<Arc<Profile>>) {
        let mut state = lock(&self.state);
        state.chosen = profile.map(monitor::chosen);
        self.changed(&mut state);
    }

    /// The monitors as last read, primary first.
    pub fn monitors(&self) -> Vec<Monitor> {
        lock(&self.state).monitors.clone()
    }

    fn resolve(&self, display: DisplayId) -> Resolved {
        let mut state = lock(&self.state);
        if let Some(r) = state.resolved.get(&display) {
            return r.clone();
        }
        let shown = display.and_then(|id| state.monitors.iter().find(|m| m.id == id)).or(state.monitors.first());
        let r = monitor::resolve(state.chosen.as_ref(), shown, &state.detection);
        state.resolved.insert(display, r.clone());
        r
    }

    /// The monitor profile for `display`: the chosen one, else the display's own (the primary's
    /// when `display` is unknown), else sRGB.
    pub fn monitor_for(&self, display: DisplayId) -> Arc<Profile> {
        self.resolve(display).profile
    }

    /// What [`monitor_for`](Self::monitor_for) resolved to, and why.
    pub fn monitor_report(&self, display: DisplayId) -> MonitorReport {
        self.resolve(display).report
    }

    /// How a canvas whose composite is in `composite`'s profile shows on `display`, cached per
    /// composite content and display.
    pub fn canvas_display_for(&self, composite: &Arc<Profile>, display: DisplayId) -> Result<Arc<DisplayTransform>, CmsError> {
        let monitor = self.monitor_for(display);
        let cache_key = (composite.content_hash(), display);
        if let Some(d) = lock(&self.transforms).get(&cache_key)
            && d.monitor.content_hash() == monitor.content_hash()
        {
            return Ok(d.clone());
        }
        let d = Arc::new(DisplayTransform::new(composite.clone(), monitor)?);
        let mut cache = lock(&self.transforms);
        if cache.len() >= MAX_TRANSFORMS {
            cache.clear();
        }
        cache.insert(cache_key, d.clone());
        Ok(d)
    }

    /// Changes whenever the transform of `composite` on `display` changes (composite, monitor,
    /// chosen profile): what a canvas compares to know its display colour is stale. Equals the
    /// transform's [`DisplayTransform::key`]. Proof colours and previews are the app's and go
    /// into its own key.
    pub fn display_signature(&self, composite: &Profile, display: DisplayId) -> u64 {
        signature(composite, &self.monitor_for(display))
    }
}
