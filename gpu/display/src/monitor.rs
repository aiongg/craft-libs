//! The monitors, the profile each display resolves to, and why. The platform reads the monitors
//! ([`craft_platform::monitors::read`]); the app hands the result to
//! [`DisplayCaches::set_monitors`](crate::DisplayCaches::set_monitors).

use std::sync::Arc;

use craft_cms::transform::{TransformOptions, cached};
use craft_cms::{Builtin, ColorSpace, Profile};

pub use craft_platform::monitors::{Monitor, MonitorsError};

/// Rendering intent of every display transform, with black point compensation: relative
/// colorimetric, as Photoshop's monitor conversion.
pub const DISPLAY_INTENT: craft_cms::Intent = craft_cms::Intent::RelativeColorimetric;

/// A display by its [`Monitor::id`]; `None` is "the main window's display".
pub type DisplayId = Option<u32>;

/// The monitor showing most of a window whose frame (the system's window coordinates, as
/// [`Monitor::frame`]) is `rect`: the one it overlaps most, as AppKit's `NSWindow.screen`.
/// `None` when it overlaps none (or the system gives no frames).
pub fn display_at(monitors: &[Monitor], rect: [f64; 4]) -> Option<u32> {
    let area = |m: &Monitor| {
        let w = (rect[0] + rect[2]).min(m.frame[0] + m.frame[2]) - rect[0].max(m.frame[0]);
        let h = (rect[1] + rect[3]).min(m.frame[1] + m.frame[3]) - rect[1].max(m.frame[1]);
        if w > 0.0 && h > 0.0 { w * h } else { 0.0 }
    };
    monitors.iter().map(|m| (m.id, area(m))).filter(|(_, a)| *a > 0.0).max_by(|a, b| a.1.total_cmp(&b.1)).map(|(id, _)| id)
}

/// Where the monitor list stands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Detection {
    /// No reader for this platform or session.
    #[default]
    Unsupported,
    /// Being read for the first time.
    Pending,
    /// The reader failed and nothing was read before.
    Failed {
        /// The reader's message.
        reason: String,
    },
    /// The list is the last reading.
    Found,
    /// The list is an earlier reading: the last one failed.
    Retained {
        /// The failed reading's message.
        reason: String,
    },
}

/// Why a display's monitor profile is what it is.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MonitorStatus {
    /// The display's own profile, as the system reports it.
    Detected,
    /// The profile the app chose (Color Settings › Monitor Profile).
    Chosen,
    /// sRGB: the system assigns the display no profile.
    NoProfile,
    /// sRGB: this platform reports no monitors.
    Unsupported,
    /// sRGB: the monitors haven't been read yet.
    Pending,
    /// sRGB: reading the monitors failed, with the reader's message.
    ReadFailed {
        /// The reader's message.
        reason: String,
    },
    /// sRGB: the profile can't be used, with the reason.
    Unusable(MonitorProblem),
}

/// Why a monitor profile can't be used. The UI words it (craft-color's
/// `ui::monitor_problem_label`), so a new reason is a change to that wording too.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MonitorProblem {
    /// The profile describes another colour space than RGB.
    NotRgb {
        /// The profile's description text, for display.
        description: String,
        /// The colour space the profile declares.
        space: ColorSpace,
    },
    /// The profile can't be the destination of a transform from sRGB.
    CannotLink {
        /// The profile's description text, for display.
        description: String,
        /// The colour engine's error text.
        reason: String,
    },
    /// The profile bytes don't parse.
    Unreadable {
        /// The parser's error text.
        reason: String,
    },
}

/// What a display's monitor profile resolved to, for Color Settings and the app's system
/// information.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonitorReport {
    /// Why it is this profile.
    pub status: MonitorStatus,
    /// The display it is for, when known.
    pub display: Option<String>,
    /// The description of the profile in use (from the profile itself).
    pub profile: String,
    /// The system's name for the display's profile, when known and in use.
    pub profile_name: Option<String>,
    /// The profile's content hash, to tell profiles with the same description apart.
    pub fingerprint: u64,
    /// Where the monitor list stands.
    pub detection: Detection,
}

/// A resolved monitor profile.
#[derive(Clone, Debug)]
pub(crate) struct Resolved {
    pub(crate) profile: Arc<Profile>,
    pub(crate) report: MonitorReport,
}

pub(crate) fn srgb() -> Arc<Profile> {
    Arc::new(Builtin::Srgb.profile().clone())
}

/// `profile` as a monitor profile: RGB, and usable as the destination of a transform from sRGB,
/// otherwise the canvas would silently stay unmanaged.
pub(crate) fn usable_monitor(profile: Profile) -> Result<Arc<Profile>, MonitorProblem> {
    if profile.color_space != ColorSpace::Rgb {
        return Err(MonitorProblem::NotRgb { description: profile.description, space: profile.color_space });
    }
    let opts = TransformOptions { intent: DISPLAY_INTENT, bpc: true, ..TransformOptions::default() };
    cached(Builtin::Srgb.profile(), &profile, opts)
        .map_err(|e| MonitorProblem::CannotLink { description: profile.description.clone(), reason: e.to_string() })?;
    Ok(Arc::new(profile))
}

/// The monitor profile of `shown` (the display, or the primary when it is unknown) given the
/// chosen profile and the detection state.
pub(crate) fn resolve(chosen: Option<&Resolved>, shown: Option<&Monitor>, detection: &Detection) -> Resolved {
    if let Some(c) = chosen {
        return Resolved {
            profile: c.profile.clone(),
            report: MonitorReport { display: shown.map(|m| m.name.clone()), detection: detection.clone(), ..c.report.clone() },
        };
    }
    let found = match (shown, detection) {
        (Some(m), _) => match &m.icc {
            Some(bytes) => match Profile::parse(bytes) {
                Ok(p) => usable_monitor(p).map_err(MonitorStatus::Unusable),
                Err(e) => Err(MonitorStatus::Unusable(MonitorProblem::Unreadable { reason: e.to_string() })),
            },
            None => Err(MonitorStatus::NoProfile),
        },
        (None, Detection::Pending) => Err(MonitorStatus::Pending),
        (None, Detection::Failed { reason }) => Err(MonitorStatus::ReadFailed { reason: reason.clone() }),
        (None, _) => Err(MonitorStatus::Unsupported),
    };
    let (profile, status) = match found {
        Ok(p) => (p, MonitorStatus::Detected),
        Err(status) => (srgb(), status),
    };
    let report = MonitorReport {
        profile_name: if status == MonitorStatus::Detected { shown.and_then(|m| m.profile_name.clone()) } else { None },
        status,
        display: shown.map(|m| m.name.clone()),
        profile: profile.description.clone(),
        fingerprint: profile.content_hash(),
        detection: detection.clone(),
    };
    Resolved { profile, report }
}

/// The app's chosen monitor profile, resolved once when chosen.
pub(crate) fn chosen(profile: Arc<Profile>) -> Resolved {
    let (profile, status) = match usable_monitor((*profile).clone()) {
        Ok(p) => (p, MonitorStatus::Chosen),
        Err(problem) => (srgb(), MonitorStatus::Unusable(problem)),
    };
    let report = MonitorReport {
        status,
        display: None,
        profile: profile.description.clone(),
        profile_name: None,
        fingerprint: profile.content_hash(),
        detection: Detection::default(),
    };
    Resolved { profile, report }
}
