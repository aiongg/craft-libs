//! Vector and shape tools: gradient editing, the Pen and path editing, strokes and corners, shapes, slices, artboards and connectors, Crop and Perspective Crop, Free Transform, measure, symmetry and brush presets.

pub mod geometry;
pub mod stroke;
#[cfg(feature = "ui")]
pub mod ui;

#[cfg(feature = "screen-testing")]
pub mod testing {
    //! The registrations of this crate's stories, for the `tests/it` harness.
    craft_ui::testing_registered!();
}
