//! Tests of the selection overlay (task SE-F7), with a `MaskSource` over a disc that counts its row
//! reads, shown on a plain egui `Context`.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// The second pass with the same mask revision, zoom and a pan of a few pixels draws the cached
// trace (`retraced` false, no row reads); a new revision, a zoom that changes the step, or a pan
// past `256 · step` retraces.
#[test]
#[ignore = "skeleton"]
fn overlay_retraces_only_when_revision_step_or_region_changes() {
    todo!()
}

// With `offset` the ants' shapes move by whole pixels on screen; with `mapped` they follow the map
// and the whole selection is traced (parts outside the view included).
#[test]
#[ignore = "skeleton"]
fn overlay_offset_and_mapped_outline_follow_the_piece() {
    todo!()
}

// `visible(false)` draws no ants and asks for no repaint, and still draws the preview texture.
#[test]
#[ignore = "skeleton"]
fn overlay_hides_ants_when_not_visible() {
    todo!()
}

// The preview texture is at most 1024 texels on the long side and is rebuilt only when the revision,
// the mode, the look or the reduction changes.
#[test]
#[ignore = "skeleton"]
fn overlay_preview_texture_is_reduced_and_reused() {
    todo!()
}

// The AccessKit node is named "Selection" and valued with the bounds in the given unit, "No
// selection" for an empty mask, and the mode's name while a preview shows.
#[test]
#[ignore = "skeleton"]
fn selection_overlay_accessible_value() {
    todo!()
}
