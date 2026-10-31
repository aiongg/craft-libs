//! Stories of the common controls added in step 5: stubs, not yet added to the controls'
//! stories. Registering them changes `crates/ui/tests/dumps/stories/registry.txt`, and
//! `every_story_shows_with_its_default_args` would run the `todo!()` bodies; the stories fill
//! task writes them, registers them, adds their components to the `COMPONENTS` list and
//! regenerates the dump.

#![expect(dead_code, clippy::todo, reason = "skeleton: removed when the bodies land")]

use crate::story::Story;

/// The story ids (Controls level unless noted):
/// - `controls/chips/keywords`: a `ChipField` of a dozen keywords, three partial, `max_lines` arg.
/// - `controls/chips/filters`: the filter strip, with Clear All.
/// - `controls/tree-picker/albums`: a `TreePicker` over folders and albums, one blocked.
/// - `controls/icon-picker/icons`: an `IconPicker` of the suite's icons in four groups, in a
///   popover from a button (Composites).
/// - `controls/linked/size`: `LinkedFields` W and H with the ratio link, in pixels.
/// - `controls/linked/margins`: four margins in a column with the equal link.
/// - `controls/preview-select/underline`: a `Select` of `LineStyle::UNDERLINES` with
///   `SelectText::Never`; a second of `LineStyle::STROKES` with `SelectText::InList`.
/// - `controls/split-button/trace`: a `SplitButton` with five presets; a variant arg.
/// - `controls/breadcrumb/path`: a `BreadcrumbBar` of a deep path; a width arg.
/// - `controls/rating/stars`: `Rating` at both sizes, and `paint_rating` at both star sizes.
/// - `controls/date/picker`: a `DatePicker` at each precision set, today fixed.
/// - `controls/alt-text/field`: an `AltTextField`, a mixed arg.
/// - `controls/autocomplete/functions`: a text field with function suggestions and
///   descriptions, and the inline completion.
/// - `controls/mixed/apply-rows`: three `ApplyRow`s, one mixed.
/// - `controls/reset-dot/fields`: three fields with reset dots, one changed.
pub(super) const IDS: [&str; 15] = [
    "controls/chips/keywords",
    "controls/chips/filters",
    "controls/tree-picker/albums",
    "controls/icon-picker/icons",
    "controls/linked/size",
    "controls/linked/margins",
    "controls/preview-select/underline",
    "controls/split-button/trace",
    "controls/breadcrumb/path",
    "controls/rating/stars",
    "controls/date/picker",
    "controls/alt-text/field",
    "controls/autocomplete/functions",
    "controls/mixed/apply-rows",
    "controls/reset-dot/fields",
];

pub(super) fn stories() -> Vec<Story> {
    todo!()
}
