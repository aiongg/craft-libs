//! Stories of the preset library, the named preset row and the picker (task ST4): stubs, not
//! registered. Registering them (task RG2) adds `stories()` to the preset stories and the
//! components to `story::DECLARED`; this file declares no `COMPONENTS` before then.
//!
//! Every story uses fakes: a fake `Files` as the step 4 stories use, so Load…, Replace…, Save…
//! and Import… show with no platform; every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `composites/preset-browser/footer-in-panel` | a browser with `footer(..)` in a short dock group: the list scrolls, the footer stays; size slider, count, Delete, New, New Group |
//! | `composites/preset-browser/drop-on-canvas` | `drop_zone` over a fake canvas; the status line names the dropped preset and the position |
//! | `composites/preset-browser/hover-preview` | `out.hovered` drives a preview tile |
//! | `composites/preset-library/groups-and-folders` | `PresetLibrary<T>` with nested folders (nesting 3): rename, delete, move, drag into folders, a name clash message |
//! | `composites/preset-picker/pattern-grid` | `Square` field, 6 columns, 30 patterns, no groups |
//! | `composites/preset-picker/shape-groups` | `Wide` field, 7 columns, group headers |
//! | `composites/preset-picker/contour-with-files` | `Square` field, 4 columns, built-in and user contours, Delete, Load, Replace, Save through the fake `Files` |
//! | `composites/preset-picker/names-list` | `PickerField::Name`, List view, groups (LightCraft's curve presets, SoundCraft's menu) |
//! | `composites/preset-picker/keeps-open` | `OnActivate`: a click selects, a double-click closes |
//! | `composites/named-preset/contour-buttons` | style `Buttons`: New… with the suggested name, a name clash error, Load…, Save… |
//! | `composites/named-preset/trace-menu` | style `Menu`: Save as New Preset…, Rename…, Delete, "Custom" when nothing matches |
//! | `composites/named-preset/rtl` | the row and the picker in right to left |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use crate::story::Story;

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
