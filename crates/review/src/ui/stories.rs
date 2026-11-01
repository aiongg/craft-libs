//! Stories of the review widgets. Not registered yet: registering them would change the registry
//! dump and run their bodies. The fill task adds them to the gallery and regenerates the dump.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::{Level, NoArgs, Story};
use craft_ui::typed_story;
use egui::Ui;

pub(super) fn stories() -> Vec<Story> {
    vec![
        typed_story!("patterns/comments/panel", Level::Patterns, "Comments", "Panel", NoArgs {}, comments_panel),
        typed_story!("patterns/comments/thread", Level::Patterns, "Comments", "Thread", NoArgs {}, thread),
        typed_story!("patterns/annotate/toolbar", Level::Patterns, "Annotate", "Toolbar", NoArgs {}, annotate),
        typed_story!("patterns/signature-pad/dialog", Level::Patterns, "Signature pad", "Dialog", NoArgs {}, signature_pad),
        typed_story!("patterns/version-history/panel", Level::Patterns, "Version history", "Panel", NoArgs {}, versions),
        typed_story!("patterns/revisions/panel", Level::Patterns, "Revisions", "Panel", NoArgs {}, revisions),
    ]
}

// new: six threads on three pages (one Accepted, one marked, one locked, one with two replies),
// the second selected.
fn comments_panel(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: four pins (two clustered) over a grey canvas and an open popover with reactions and an
// @mention list.
fn thread(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: the toolbar with the Draw flyout open over a page with a highlight, a rectangle and ink,
// the rectangle selected with its handles.
fn annotate(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: the pad on the Draw tab with two strokes.
fn signature_pad(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: two branches, Current version, three named versions and an autosave group.
fn versions(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: inserted, changed and deleted slides.
fn revisions(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}
