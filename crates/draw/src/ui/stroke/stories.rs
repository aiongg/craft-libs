//! Stories of the stroke editor (task ST-PE1): stubs, not registered. Registering them (task
//! RG2) adds `stories()` to the draw stories and the components to `COMPONENTS`, and regenerates
//! the registry dump; until then the gallery would run the `todo!()` bodies.
//!
//! Every story uses a fake value held by the story and story words; the choice rows paint fake
//! arrowheads and profiles.
//!
//! | Story id | Shows |
//! |---|---|
//! | `composites/stroke-editor/panel` | The docked body; args: closed, dashed, mixed, sections (panel, popover), unit (widths, document) |
//! | `composites/stroke-editor/popover` | The opener and the popover; args: stroke on or off |
//! | `composites/stroke-editor/choice-rows` | Arrowhead start and end and profile rows with painted previews |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the stories land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
