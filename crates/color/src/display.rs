//! The display transform (F04.11): how a canvas shows a document's colours on a monitor. Pixels
//! in the document's profile go through one cached transform to the monitor's profile; when that
//! transform changes nothing, the canvas skips it.
//!
//! The monitor profile comes from the platform (craft-platform's display reading, or the ICC
//! bytes of a [`craft_platform::screen::ScreenImage`]) or from the app's Color Settings; a
//! profile that is missing, unreadable, not RGB or unusable as a destination falls back to sRGB,
//! and [`MonitorStatus`] says why.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use craft_cms::{CmsError, Profile, Transform};

/// Rendering intent and black point compensation of every display transform: relative
/// colorimetric with BPC, as Photoshop's default monitor conversion and PhotoCraft's.
pub const DISPLAY_INTENT: craft_cms::Intent = craft_cms::Intent::RelativeColorimetric;

/// A display, as the platform numbers it; `None` is "the main window's display".
pub type DisplayId = Option<u32>;

/// How one document profile shows on one monitor.
#[derive(Debug)]
pub struct DisplayTransform {
    /// The profile of the values the canvas texture holds (the document's composite profile).
    pub source: Arc<Profile>,
    /// The monitor profile.
    pub monitor: Arc<Profile>,
    /// Texture values to monitor values; `None` when that is the identity within half an 8-bit
    /// step on every lattice point (the neutral axis only for a grey source).
    pub transform: Option<Arc<Transform>>,
    /// Changes whenever any of the above changes, for the canvas's LUT and texture caches.
    pub key: u64,
}

impl DisplayTransform {
    /// The transform from `source` to `monitor` ([`DISPLAY_INTENT`], BPC), built through
    /// `craft_cms::cached`; `transform` is `None` when it is the identity.
    // pc: crates/engine/src/display_color.rs:413 `canvas_display_for` (the build part) and :151
    // `is_identity` (9³ lattice, half an 8-bit step; the 33 grey steps for a grey source).
    pub fn new(source: Arc<Profile>, monitor: Arc<Profile>) -> Result<DisplayTransform, CmsError> {
        todo!()
    }

    /// Values go to the screen unchanged.
    pub fn is_identity(&self) -> bool {
        self.transform.is_none()
    }

    /// `rgb` (0–1 each, in the source profile) as the monitor shows it.
    // pc: crates/engine/src/display_color.rs:38 `CanvasDisplay` (callers use `transform.eval`).
    pub fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        todo!()
    }

    /// RGBA8 pixels converted in place (alpha untouched); nothing happens for the identity.
    // new: the canvas's CPU path and the screen picker's loupe convert whole rows.
    pub fn apply_rgba8(&self, pixels: &mut [u8]) {
        todo!()
    }
}

/// Why the monitor profile is what it is.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MonitorStatus {
    /// The display's own profile, as the platform reported it.
    Detected,
    /// The profile the app chose (Color Settings › Monitor Profile).
    Chosen,
    /// sRGB: the platform reports no profile for the display.
    NoProfile,
    /// sRGB: the profile could not be used, with the reason (not RGB, unreadable, cannot link).
    Unusable(String),
}

/// One monitor's profile and how it was resolved.
#[derive(Clone, Debug)]
struct MonitorEntry {
    profile: Arc<Profile>,
    status: MonitorStatus,
}

/// The app's display caches: the monitor profile per display, and the [`DisplayTransform`] per
/// document profile and display. Shared by every window; `Sync`.
#[derive(Default)]
pub struct DisplayCaches {
    monitors: Mutex<HashMap<DisplayId, MonitorEntry>>,
    chosen: Mutex<Option<Arc<Profile>>>,
    transforms: Mutex<HashMap<(u64, DisplayId), Arc<DisplayTransform>>>,
}

const _: () = {
    const fn shared<T: Send + Sync>() {}
    shared::<DisplayCaches>();
};

impl DisplayCaches {
    pub fn new() -> DisplayCaches {
        DisplayCaches::default()
    }

    /// Records the platform's reading of `display`'s profile (`icc` bytes, `None` when it has
    /// none). A reading that fails to parse keeps the previous good one. Drops the cached
    /// transforms of that display.
    // pc: crates/engine/src/display_color.rs:301 (`set_displays`: a failed re-read keeps the
    // displays read before) and the monitor cache at :268.
    pub fn set_monitor(&self, display: DisplayId, icc: Option<&[u8]>) {
        todo!()
    }

    /// The app's choice of monitor profile (Color Settings), or `None` to use each display's own.
    /// Drops every cached transform.
    // pc: crates/engine/src/display_color.rs:338 `resolved_monitor` (the `auto` / chosen branch).
    pub fn choose_monitor(&self, profile: Option<Arc<Profile>>) {
        todo!()
    }

    /// The monitor profile for `display`: the chosen one, else the display's own, else the
    /// primary display's when `display` is unknown, else sRGB.
    // pc: crates/engine/src/display_color.rs:284 `monitor_for`, :338 `resolved_monitor`.
    pub fn monitor_for(&self, display: DisplayId) -> Arc<Profile> {
        todo!()
    }

    /// What [`monitor_for`](Self::monitor_for) resolved to, and why.
    // pc: crates/engine/src/display_color.rs:294 `monitor_status_for`.
    pub fn monitor_status(&self, display: DisplayId) -> MonitorStatus {
        todo!()
    }

    /// How a canvas in `source`'s profile shows on `display`, cached per source content and
    /// display.
    // pc: crates/engine/src/display_color.rs:413 `canvas_display_for`.
    pub fn canvas_display_for(&self, source: &Arc<Profile>, display: DisplayId) -> Result<Arc<DisplayTransform>, CmsError> {
        todo!()
    }

    /// Changes whenever the transform of `source` on `display` changes (source, monitor, chosen
    /// profile): what a canvas compares to know its texture or LUT is stale.
    // pc: crates/engine/src/display_color.rs:453 `display_signature_for` (without Proof Colors,
    // Gamut Warning and the 32-bit preview: those are the app's and go into its own key).
    pub fn display_signature(&self, source: &Profile, display: DisplayId) -> u64 {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    /// sRGB on an sRGB monitor is the identity: no transform, `apply` returns its input.
    #[test]
    #[ignore = "skeleton"]
    fn srgb_on_an_srgb_monitor_is_the_identity() {
        todo!()
    }

    /// An unreadable monitor profile falls back to sRGB and reports `Unusable`.
    #[test]
    #[ignore = "skeleton"]
    fn an_unreadable_monitor_profile_falls_back_to_srgb() {
        todo!()
    }

    /// Choosing another monitor profile changes the signature and drops the cached transform.
    #[test]
    #[ignore = "skeleton"]
    fn choosing_a_monitor_profile_changes_the_signature() {
        todo!()
    }
}
