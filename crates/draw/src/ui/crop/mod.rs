//! The Crop tool: the [`CropFrame`] overlay with its composition guides and the shield outside
//! it, the tool on the canvas tool host ([`CropTool`]) with its pending frame ([`CropSession`]) and
//! options ([`CropSettings`]), the options bar ([`CropOptionsBar`]) with its overlay and gear menus,
//! and the Perspective Crop tool's [`PerspectiveCropQuad`].
//!
//! The tool reports events and never crops the document: the app crops on
//! [`CropEvent::Apply`] with the [`CropCommit`] it carries.

mod bar;
#[cfg(test)]
mod bar_tests;
pub(crate) mod crop_frame;
pub mod crop_guides;
mod menus;
#[cfg(test)]
mod menus_tests;
mod options;
#[cfg(test)]
mod options_tests;
pub(crate) mod perspective_crop;
mod session;
#[cfg(test)]
mod session_tests;
pub(crate) mod stories_bar;
pub(crate) mod stories_tool;
mod style;
mod tool;
#[cfg(test)]
mod tool_tests;

pub use bar::{CropBarEvent, CropBarLabels, CropBarOutput, CropOptionsBar};
pub use crop_frame::{
    CropBox, CropFrame, CropKeyAction, CropLabels, CropNames, CropPress, CropPressKind, CropReadout, CropRelease, CropResponse, CropStyle, CropTrack,
    hit_rect,
};
pub use crop_guides::{CropGuide, CropShield, GuideShow, Polyline, ShieldColor};
pub use menus::{CropOptionsMenu, CropOptionsMenuResponse, CropOverlayMenu};
pub use options::{
    CROP_RATIOS, CROP_SIZES, CropFill, CropPresetChoice, CropRatio, CropSettings, CropSizeFields, CropSizePreset, CropTarget, CropUserPreset,
    FIELD_MAX, RATIO_DECIMALS, RATIO_MAX, ResolutionUnit,
};
pub use perspective_crop::{
    PerspectiveCropNames, PerspectiveCropQuad, PerspectiveCropResponse, PerspectiveCropStyle, PerspectiveHit, PerspectiveQuad,
};
pub use session::{CropDocKey, CropDocument, CropKept, CropSession, CropStep, LeaveFrame, StraightenState};
pub use style::CropBarStyle;
pub use tool::{CropCommit, CropEvent, CropInput, CropOutput, CropTool, CropToolPass};
