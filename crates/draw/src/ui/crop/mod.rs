//! The Crop tool's overlays: [`CropFrame`] with its composition guides and the shield outside it,
//! and the Perspective Crop tool's [`PerspectiveCropQuad`].

pub(crate) mod crop_frame;
pub mod crop_guides;
pub(crate) mod perspective_crop;

pub use crop_frame::{CropBox, CropFrame, CropLabels, CropNames, CropPress, CropRelease, CropResponse, CropStyle, CropTrack};
pub use crop_guides::{CropGuide, CropShield, GuideShow, Polyline, ShieldColor};
pub use perspective_crop::{
    PerspectiveCropNames, PerspectiveCropQuad, PerspectiveCropResponse, PerspectiveCropStyle, PerspectiveHit, PerspectiveQuad,
};
