//! Tests of Liquify's canvas and workspace (task WS-LQ2).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Alt + right-drag and Control + Alt + drag change the size (horizontal) and the density
// (vertical), keep the outline at the anchor, and report no stroke.
#[test]
#[ignore = "skeleton"]
fn liquify_alt_right_drag_resizes_size_and_density_and_paints_nothing() {
    todo!()
}

// With the Size number field focused, ⌘Z still undoes the last stroke (Replay is reported) and
// the field keeps its text.
#[test]
#[ignore = "skeleton"]
fn liquify_undo_works_while_a_number_field_has_focus() {
    todo!()
}

// In Front, Behind and Blend tint the image and the backdrop as the table in `BackdropLayering`
// says; no backdrop leaves the image white.
#[test]
#[ignore = "skeleton"]
fn liquify_backdrop_layers_follow_the_mode_and_opacity() {
    todo!()
}

// Turning Show Backdrop on reports BackdropWanted(None); picking a layer in Use reports its id.
#[test]
#[ignore = "skeleton"]
fn liquify_show_backdrop_asks_the_app_for_the_backdrop() {
    todo!()
}

// ⌘+ zooms the workspace's view, not the canvas behind the dialog; the title shows the new
// percent.
#[test]
#[ignore = "skeleton"]
fn liquify_zoom_keys_zoom_the_preview() {
    todo!()
}

// The brush outline follows the pointer, hides off the canvas, and stays at the anchor while the
// brush is resized.
#[test]
#[ignore = "skeleton"]
fn liquify_brush_outline_follows_the_pointer_except_while_resizing() {
    todo!()
}
