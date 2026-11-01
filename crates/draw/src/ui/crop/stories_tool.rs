//! Stories of the Crop tool on the canvas tool host (task ST-TO1): stubs, not registered.
//! Registering them (task RG2) adds them to `craft_draw::ui::stories()` and their components to the
//! registry; until then the gallery would run the `todo!()` bodies.
//!
//! Every story runs a `CropTool` through `ToolHost` on a fake page (`craft_ui::testing::canvas_story`)
//! and keeps the settings and the tool in the story's state. Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `primitives/crop-tool/default-frame` | The tool picked on a fake page with and without a selection; a drag inside draws; a "Pick another tool" button leaves and applies |
//! | `primitives/crop-tool/image-mode` | Image mode and Classic Mode side by side with the view turning |
//! | `primitives/crop-tool/ratio-and-size` | Ratio presets and W x H x Resolution, with the resulting `CropTarget` in a status line |
//! | `primitives/crop-tool/straighten` | The armed button and the temporary Command line |
//! | `primitives/crop-tool/box-undo` | Edit › Undo and Redo buttons stepping the box |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
