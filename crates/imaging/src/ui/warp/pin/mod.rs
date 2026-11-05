//! Puppet Warp (Edit › Puppet Warp): pins on a deformed mesh ([`PinWarpSession`], [`PinWarp`],
//! [`PinWarpBar`]). The solver is the app's ([`PinSolver`]); the warped picture is drawn by the
//! transform preview through `PreviewMap::pins`.

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

pub use bar::{PinWarpBar, PinWarpBarLabels, PinWarpBarOutput};
pub use overlay::{PinWarp, PinWarpLabels, PinWarpResponse};
pub use session::{Pin, PinDensity, PinEdit, PinMeshView, PinMode, PinSolver, PinWarpOptions, PinWarpSession, PinWarpSettings};
