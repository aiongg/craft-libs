//! Stories of the colour dialogs, panels and tools: stubs, not yet added to [`super::stories`].
//! Registering them changes `craft-ui`'s `tests/dumps/stories/registry.txt`, and
//! `every_story_shows_with_its_default_args` would run the `todo!()` bodies; the stories fill task
//! writes them, registers them, adds their components to [`super::COMPONENTS`] and regenerates
//! the dump.

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The story ids, one per component (each a Composites-level story unless noted):
/// - `patterns/color/picker-dialog`: a "Show dialog" tile opening `ColorPickerDialog` on a
///   `DialogHost`, args for gamut, Only Web Colors, preview; prints the result.
/// - `patterns/color/panel`: `ColorPanel` with a mode and a ramp arg, the panel menu beside it.
/// - `patterns/color/swatches`: `SwatchesPanel` on the default library, the panel menu beside it.
/// - `patterns/color/eyedropper-options`: `EyedropperOptionsBar` in an `OptionsBar`.
/// - `patterns/color/samplers`: four points on a gradient image in a `CanvasView`, readouts under it.
/// - `patterns/color/paint-chip`: `PaintChip` for each paint kind, with and without No Color.
/// - `patterns/color/choice`: `ColorChoice` with PhotoCraft's pasteboard rows.
/// - `patterns/color/harmony`: `HarmonyWheel` and `HarmonyPanel` on one base colour, a rule arg.
/// - `patterns/color/lut-picker`: `LutPicker` with no folder.
/// - `patterns/color/profile-mismatch`: a "Show dialog" tile for both questions.
pub(super) const IDS: [&str; 10] = [
    "patterns/color/picker-dialog",
    "patterns/color/panel",
    "patterns/color/swatches",
    "patterns/color/eyedropper-options",
    "patterns/color/samplers",
    "patterns/color/paint-chip",
    "patterns/color/choice",
    "patterns/color/harmony",
    "patterns/color/lut-picker",
    "patterns/color/profile-mismatch",
];

/// The stories of [`IDS`].
// new: one story per component, on the pattern of `full_stories.rs` and step 2's
// `craft_ui::dialog::framework_stories` (dialogs on a `DialogHost` kept in egui memory).
pub(super) fn stories() -> Vec<Story> {
    todo!()
}
