//! The selection frame of picked objects, their outlines and grips: what the object-editing apps
//! draw over their canvas once something is selected.
//!
//! - [`SelectionFrame`]: the box round the selected objects, with eight grips that resize it and an
//!   inside that moves it (VectorCraft's bounding box, DesignCraft's frame, with the key object's
//!   thicker line and the centre mark).
//! - [`SelectionOutline`]: solid or dashed highlights over the selected shapes (VectorCraft's
//!   per-object outline, CadCraft's selection effect).
//! - [`GripSet`]: per-object grips with cold, warm and hot states (CadCraft's grips and dynamic
//!   grips).
//! - [`range_bands`] and [`rect_around`]: the outline and the fill of a range selection with its
//!   active cell cut out (GridCraft).
//!
//! None of these changes the document: the frame and the grips report [`EditEvent`]s, the host
//! applies them (`crate::edit`). Rotating a selection is Free Transform's (`craft_draw`'s
//! `TransformBox`), not the frame's.
//!
//! [`EditEvent`]: crate::edit::EditEvent

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

mod bands;
mod frame;
mod grips;
#[cfg(test)]
mod grips_tests;
mod outline;
mod style;
#[cfg(test)]
mod tests;

pub use bands::{range_bands, rect_around};
pub use frame::{SelectionFrame, SelectionFrameLabels, SelectionFrameResponse};
pub use grips::{Grip, GripClick, GripDrag, GripSet, GripSetLabels, GripSetResponse, GripShape, GripState};
pub use outline::{SelectionOutline, SelectionOutlineLook};
pub use style::{GripSetStyle, SelectionFrameStyle};
