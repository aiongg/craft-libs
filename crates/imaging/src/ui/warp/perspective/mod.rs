//! Perspective Warp (Edit › Perspective Warp): planes drawn in Layout and dragged in Warp
//! ([`PerspectiveSession`], [`PerspectiveWarp`], [`PerspectiveBar`]). The warp map is the app's;
//! the warped picture is drawn by the transform preview through `PreviewMap::function`.

mod bar;
mod overlay;
mod session;
pub(crate) mod stories;

#[cfg(test)]
mod bar_tests;
#[cfg(test)]
mod overlay_tests;
#[cfg(test)]
mod session_tests;

pub use bar::{PerspectiveBar, PerspectiveBarLabels, PerspectiveBarOutput};
pub use overlay::{PerspectiveLabels, PerspectiveResponse, PerspectiveWarp};
pub use session::{
    LINK_DISTANCE, PLANE_MINIMUM, PerspectiveEdit, PerspectiveSession, Plane, PlaneCorner, PlaneMode, Straighten, linked_corners, straighten, unify,
};
