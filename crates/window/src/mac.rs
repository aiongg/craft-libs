//! The macOS traffic lights, measured, so an app-drawn title bar fits around them.
//!
//! AppKit places the lights; their size and position change between macOS versions (they grew
//! in macOS 26). [`TrafficLights::update`] measures the zoom button of a titled window with a
//! full-size content view and returns the [`MacInset`] the title bar reserves. The result is
//! cached: the AppKit calls run again only after leaving full screen or when the scale changes.
//! Before a window exists it returns [`default_inset`] for the running macOS version.
//!
//! ```rust,ignore
//! // Once per frame, before laying out the bar; `lights` is app state.
//! let inset = lights.update(ctx); // None in full screen: no inset
//! ```
//!
//! # Checklist on a Mac
//!
//! Run an app that calls [`TrafficLights::update`] on macOS 26 and 27, one window with the
//! app-drawn bar, and compare against the real lights:
//!
//! - The reserved leading space ends one gap after the green button; the bar's content centres
//!   on the lights' centre line.
//! - A second window of the same app gets the same inset.
//! - Full screen: the inset is `None` and the bar's content starts at the left edge. Leaving
//!   full screen restores the inset with no frame at the default.
//! - Dragging the window between a Retina and a non-Retina display re-measures; the inset stays
//!   equal to the lights.

use crate::policy::MacInset;

/// The three buttons as AppKit laid them out, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lights {
    /// The zoom (rightmost) button's right edge from the window's left edge.
    pub right: f32,
    /// The lights' vertical centre below the window's top.
    pub centre_from_top: f32,
    /// A button's diameter.
    pub size: f32,
}

/// Largest plausible distance of the lights' centre from the window top, in points.
const MAX_CENTRE_FROM_TOP: f32 = 40.0;
/// Largest plausible distance of the zoom button's right edge from the window's left, in points.
const MAX_RIGHT: f32 = 200.0;

/// The inset for measured lights: the lights are centred in a band `2 * centre_from_top` tall,
/// and content starts the band's side gap after them, the same gap they keep from the top.
/// `None` for a measurement that is not a plausible button.
pub fn inset_from_lights(lights: Lights) -> Option<MacInset> {
    let Lights { right, centre_from_top, size } = lights;
    // The bounds keep out a full-screen window, whose lights sit in the toolbar window about
    // 1000 pt from the left.
    let sane = [right, centre_from_top, size].iter().all(|v| v.is_finite() && *v > 0.0)
        && size <= 2.0 * centre_from_top
        && centre_from_top <= MAX_CENTRE_FROM_TOP
        && right <= MAX_RIGHT;
    sane.then(|| MacInset { leading: right + (centre_from_top - size / 2.0), centre_from_top })
}

/// The inset for macOS `major` (26 for Tahoe) before a window exists to measure.
///
/// Before 26 the lights are 12 pt, 20 pt apart, 7 pt from the left, in a 28 pt band: 67 / 14.
/// DeckCraft measured macOS 27 at 14 pt, 23 pt apart, 9 pt from the left, in a 32 pt band:
/// 78 / 16. macOS 26 has not been measured; it shares 27's redesigned lights, so it uses the same.
pub fn default_inset(major: u32) -> MacInset {
    let lights = if major >= 26 {
        Lights { right: 9.0 + 2.0 * 23.0 + 14.0, centre_from_top: 16.0, size: 14.0 }
    } else {
        Lights { right: 7.0 + 2.0 * 20.0 + 12.0, centre_from_top: 14.0, size: 12.0 }
    };
    inset_from_lights(lights).unwrap_or_default()
}

/// The cache behind [`TrafficLights`], with no AppKit or egui in it.
#[derive(Clone, Debug, Default)]
pub struct Cache {
    measured: Option<MacInset>,
    scale: Option<u32>,
}

impl Cache {
    /// The inset for this frame. `measure` runs only when there is no measurement, the scale
    /// changed since it, or full screen ended since it; it returns `None` while no window can be
    /// measured, and the next frame tries again. Without a measurement the result is `default`.
    /// `None` in full screen.
    pub fn inset(
        &mut self,
        scale: f32,
        fullscreen: bool,
        default: impl FnOnce() -> MacInset,
        measure: impl FnOnce() -> Option<MacInset>,
    ) -> Option<MacInset> {
        if fullscreen {
            // The lights move or hide in full screen; what was measured is out of date.
            self.measured = None;
            return None;
        }
        let key = scale.to_bits();
        if self.scale != Some(key) {
            self.measured = None;
        }
        if self.measured.is_none() {
            self.measured = measure();
            self.scale = self.measured.map(|_| key);
        }
        Some(self.measured.unwrap_or_else(default))
    }
}

/// Whether the window is in full screen, where there are no lights to leave room for.
#[cfg(feature = "ui")]
pub fn is_fullscreen(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.viewport().fullscreen).unwrap_or(false)
}

/// The measured traffic-lights inset, kept as app state. Cheap once measured.
#[cfg(feature = "ui")]
#[derive(Clone, Debug, Default)]
pub struct TrafficLights {
    cache: Cache,
}

#[cfg(feature = "ui")]
impl TrafficLights {
    pub fn new() -> Self {
        Self::default()
    }

    /// The inset to reserve this frame: the measured one, [`default_inset`] until a window can
    /// be measured, `None` in full screen. Elsewhere than macOS the default is [`MacInset::default`].
    pub fn update(&mut self, ctx: &egui::Context) -> Option<MacInset> {
        self.cache.inset(ctx.pixels_per_point(), is_fullscreen(ctx), os_default, measure)
    }

    /// One measurement of the first titled full-size-content window, without the cache.
    pub fn measure(_ctx: &egui::Context) -> Option<MacInset> {
        measure()
    }
}

#[cfg(all(feature = "ui", target_os = "macos"))]
fn os_default() -> MacInset {
    use objc2_foundation::NSProcessInfo;
    let major = NSProcessInfo::processInfo().operatingSystemVersion().majorVersion;
    default_inset(u32::try_from(major).unwrap_or(0))
}

#[cfg(all(feature = "ui", not(target_os = "macos")))]
fn os_default() -> MacInset {
    MacInset::default()
}

#[cfg(all(feature = "ui", target_os = "macos"))]
fn measure() -> Option<MacInset> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSWindowButton, NSWindowStyleMask};

    // eframe runs the app on the main thread; anywhere else AppKit is out of reach.
    let mtm = MainThreadMarker::new()?;
    // Every titled window with a full-size content view has the same geometry; floating panels
    // hide the buttons and are skipped.
    // A window in full screen is skipped: its buttons live in the full-screen toolbar window.
    let window = NSApplication::sharedApplication(mtm).windows().to_vec().into_iter().find(|w| {
        let mask = w.styleMask();
        mask.contains(NSWindowStyleMask::Titled)
            && mask.contains(NSWindowStyleMask::FullSizeContentView)
            && !mask.contains(NSWindowStyleMask::FullScreen)
            && w.standardWindowButton(NSWindowButton::CloseButton).is_some_and(|close| !close.isHidden())
            && w.standardWindowButton(NSWindowButton::ZoomButton).and_then(|zoom| zoom.window()).is_some_and(|owner| owner == *w)
    })?;
    let zoom = window.standardWindowButton(NSWindowButton::ZoomButton)?;
    // Window coordinates count up from the bottom.
    let frame = zoom.convertRect_toView(zoom.bounds(), None);
    let window_height = window.frame().size.height;
    inset_from_lights(Lights {
        right: (frame.origin.x + frame.size.width) as f32,
        centre_from_top: (window_height - (frame.origin.y + frame.size.height / 2.0)) as f32,
        size: frame.size.height as f32,
    })
}

#[cfg(all(feature = "ui", not(target_os = "macos")))]
fn measure() -> Option<MacInset> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn lights(right: f32, centre: f32, size: f32) -> Lights {
        Lights { right, centre_from_top: centre, size }
    }

    #[test]
    fn inset_adds_the_top_gap_after_the_lights() {
        let inset = inset_from_lights(lights(69.0, 14.0, 12.0)).unwrap();
        assert_eq!(inset, MacInset { leading: 77.0, centre_from_top: 14.0 });
    }

    #[test]
    fn implausible_measurements_are_rejected() {
        for l in [
            lights(0.0, 14.0, 12.0),
            lights(f32::NAN, 14.0, 12.0),
            lights(69.0, -1.0, 12.0),
            lights(69.0, 14.0, 0.0),
            lights(69.0, 5.0, 12.0),
            lights(69.0, 41.0, 12.0),
            lights(201.0, 14.0, 12.0),
            lights(1000.0, 14.0, 12.0),
        ] {
            assert_eq!(inset_from_lights(l), None, "{l:?}");
        }
    }

    #[test]
    fn defaults_follow_the_macos_version() {
        assert_eq!(default_inset(15), MacInset { leading: 67.0, centre_from_top: 14.0 });
        assert_eq!(default_inset(25), default_inset(15));
        assert_eq!(default_inset(27), MacInset { leading: 78.0, centre_from_top: 16.0 });
        assert_eq!(default_inset(26), default_inset(27));
    }

    #[test]
    fn measures_once_then_serves_the_cache() {
        let mut cache = Cache::default();
        let calls = Cell::new(0);
        let found = MacInset { leading: 80.0, centre_from_top: 15.0 };
        let mut frame = |scale, fullscreen| {
            cache.inset(
                scale,
                fullscreen,
                || default_inset(27),
                || {
                    calls.set(calls.get() + 1);
                    Some(found)
                },
            )
        };
        assert_eq!(frame(2.0, false), Some(found));
        assert_eq!(frame(2.0, false), Some(found));
        assert_eq!(calls.get(), 1);
        // A scale change measures again.
        frame(1.0, false);
        assert_eq!(calls.get(), 2);
        // Full screen: no inset, no measuring; leaving it measures again.
        assert_eq!(frame(1.0, true), None);
        assert_eq!(calls.get(), 2);
        assert_eq!(frame(1.0, false), Some(found));
        assert_eq!(calls.get(), 3);
    }

    #[test]
    fn without_a_window_the_default_serves_and_measuring_retries() {
        let mut cache = Cache::default();
        let calls = Cell::new(0);
        let mut frame = || {
            cache.inset(
                2.0,
                false,
                || default_inset(27),
                || {
                    calls.set(calls.get() + 1);
                    None
                },
            )
        };
        assert_eq!(frame(), Some(default_inset(27)));
        assert_eq!(frame(), Some(default_inset(27)));
        assert_eq!(calls.get(), 2);
    }
}
