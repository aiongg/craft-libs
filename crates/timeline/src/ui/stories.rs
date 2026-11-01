//! Stories of the timeline widgets. Not registered yet: registering them would change the
//! registry dump and run their bodies. The fill task adds them to the gallery and regenerates the
//! dump.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::story::{Level, NoArgs, Story};
use craft_ui::typed_story;
use egui::Ui;

pub(super) fn stories() -> Vec<Story> {
    vec![
        typed_story!("patterns/timeline/panel", Level::Patterns, "Timeline", "Panel", NoArgs {}, timeline_panel),
        typed_story!("composites/timeline-ruler/ruler", Level::Composites, "Timeline ruler", "Ruler", NoArgs {}, ruler),
        typed_story!("composites/keyframes/lane", Level::Composites, "Keyframes", "Lane", NoArgs {}, keyframe_lane),
        typed_story!("patterns/graph-editor/editor", Level::Patterns, "Graph editor", "Editor", NoArgs {}, graph_editor),
        typed_story!("composites/transport/bar", Level::Composites, "Transport", "Bar", NoArgs {}, transport_bar),
        typed_story!("composites/transport/media-controls", Level::Composites, "Transport", "Media controls", NoArgs {}, media_controls),
        typed_story!("controls/timecode-field/field", Level::Controls, "Timecode field", "Field", NoArgs {}, timecode_field),
        typed_story!("patterns/timecode-panel/panel", Level::Patterns, "Timecode panel", "Panel", NoArgs {}, timecode_panel),
        typed_story!("composites/frame-preview/preview", Level::Composites, "Frame preview", "Preview", NoArgs {}, frame_preview),
        typed_story!("patterns/marker-dialog/dialog", Level::Patterns, "Marker dialog", "Dialog", NoArgs {}, marker_dialog),
        typed_story!("composites/timeline-overview/overview", Level::Composites, "Timeline overview", "Overview", NoArgs {}, overview),
        typed_story!("patterns/render-queue/list", Level::Patterns, "Render queue", "List", NoArgs {}, render_queue),
    ]
}

// new: a 30 s sequence at 29.97 drop-frame over an in-memory model: V1–V2 with four clips (one
// selected, one disabled), A1–A2 with two, a marker, In and Out set, the playhead at 00;00;05;00,
// and a keyframe lane under V1.
fn timeline_panel(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: the ruler at three zooms (frames, seconds, minutes), with a work area and cache bars.
fn ruler(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: a lane with Linear, Hold and Bezier keys, two selected, and the navigator beside it.
fn keyframe_lane(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: Position X and Y curves with four keys, one selected with handles, value graph.
fn graph_editor(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: the Program row, playing and looping.
fn transport_bar(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: the media controls at 0:12.50 of 1:05.00 with trim marks.
fn media_controls(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: a large and a small field at 29.97 drop-frame, one being typed in.
fn timecode_field(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: Current Time, Duration and Remaining rows.
fn timecode_panel(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: a 4:3 test card letterboxed in a 16:9 box with a status line, and a cropped one.
fn frame_preview(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: the Marker dialog on a range marker.
fn marker_dialog(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: eight tracks of runs and the visible box.
fn overview(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}

// new: Ready, Encoding 42 %, Done, Failed and Cancelled items.
fn render_queue(ui: &mut Ui, _: &mut NoArgs) {
    todo!()
}
