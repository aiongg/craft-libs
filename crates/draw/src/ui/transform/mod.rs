//! Free Transform: the [`TransformBox`] with its handles and reference point, the options bar's
//! [`TransformFields`], and the open session around them ([`TransformSession`],
//! [`TransformOverlay`], [`TransformBar`]).

mod session;
mod session_bar;
mod session_overlay;
pub(crate) mod session_stories;
pub(crate) mod transform_box;
pub(crate) mod transform_fields;

#[cfg(test)]
mod session_bar_tests;
#[cfg(test)]
mod session_overlay_tests;
#[cfg(test)]
mod session_tests;

pub use session::{
    Interpolation, TransformChoice, TransformEdit, TransformMenuLabels, TransformPhase, TransformPreset, TransformRequest, TransformResult,
    TransformSession, TransformSettings, TransformWarp,
};
pub use session_bar::{TransformBar, TransformBarLabels, TransformBarOutput};
pub use session_overlay::{TransformOverlay, TransformOverlayResponse};
pub use transform_box::{
    BoxPress, BoxRelease, BoxTrack, RotateZone, TransformBox, TransformBoxResponse, TransformLabels, TransformNames, TransformStyle, paint_pivot,
};
pub use transform_fields::{BarGroups, FieldLabel, ParseContext, SizeUnit, TransformField, TransformFieldLabels, TransformFields, TransformRow};
