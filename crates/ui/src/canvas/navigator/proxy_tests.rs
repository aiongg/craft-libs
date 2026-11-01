//! The Navigator's pure geometry: the proxy quad, the zoom to a rectangle, the proxy colours.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Rotated and flipped views give the canvas corners mapped into the preview; upright gives the
// axis-aligned rectangle of PhotoCraft's size and centre.
#[test]
#[ignore = "skeleton"]
fn navigator_proxy_is_a_quad_on_a_rotated_view() {
    todo!()
}

// `zoom_to_document_rect`: the 16-points-per-pixel case, the pixels-per-point and aspect case, a
// rectangle clipped to the document, the under-2-pixel rung step, and NaN and off-document no-ops
// (PhotoCraft's `navigator_zoom_to_fits_the_rectangle_in_the_canvas`, panels.rs:362).
#[test]
#[ignore = "skeleton"]
fn navigator_ctrl_drag_zooms_to_the_rectangle() {
    todo!()
}

// Each `ProxyColor` resolves; `Default` reads `navigator.proxy`; `Custom` keeps its value; the
// settings round-trip through JSON as `{"proxyColor": ..}`.
#[test]
#[ignore = "skeleton"]
fn proxy_color_choice_overrides_the_token() {
    todo!()
}
