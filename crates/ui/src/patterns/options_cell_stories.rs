//! Stories of the step 3 option cells. Not yet in the options bar's stories: registering them
//! would change `tests/dumps/stories/registry.txt`. The fill task that lands the cells registers
//! them and regenerates the dump.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::Ui;

use crate::story::{Level, Story, StoryArgs, Surface};
use crate::typed_story;

#[derive(StoryArgs, Debug, PartialEq)]
struct CellsArgs {
    narrow: bool,
    expressions: bool,
    #[arg(name = "digit keys")]
    digit_keys: bool,
}

pub(super) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/options-bar/cells",
            Level::Patterns,
            "OptionsBar",
            "Value cells",
            CellsArgs { narrow: false, expressions: true, digit_keys: true },
            show_cells
        )
        .on(Surface::Chrome),
    ]
}

/// A marquee-like bar: Feather (number cell), Opacity and Flow (percent cells, digit keys when
/// the story has the keyboard), Fixed Size Width and Height (unit cells with the units menu), and
/// `reveal_focused_control` in a narrow bar.
fn show_cells(ui: &mut Ui, args: &mut CellsArgs) {
    todo!()
}
