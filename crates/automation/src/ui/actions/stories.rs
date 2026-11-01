//! Stories of the Actions panel (task ST7): stubs, not registered. Registering them (task RG2)
//! adds `stories()` to the facade's automation stories, the words and counts to the registry and
//! regenerates the registry dump; until then the story tests would run the `todo!()` bodies.
//!
//! Every story uses fakes: an `ActionList` built in the story, a fake `ActionsHost` (step labels
//! from the command id, a recording flag, pending steps), and story words (the gallery's
//! pseudo-locale test).
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/automation/actions/sets` | two sets, a loose action, an expanded action with steps, one step off |
//! | `patterns/automation/actions/recording` | a recording row with pending steps and the dot |
//! | `patterns/automation/actions/button-mode` | the six colours, gray and none |
//! | `patterns/automation/actions/empty` | the empty state |

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::Story;

/// The four stories above.
pub(crate) fn stories() -> Vec<Story> {
    // new: see the module's table.
    todo!()
}
