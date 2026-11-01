//! Tests of the brush Size row (task BR-BS2).
#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A slider drag on the log track reports whole pixels as one gesture while the host writes the
// rounded value back each pass (gap 67); the field shows the same value.
#[test]
#[ignore = "skeleton"]
fn size_field_and_slider_agree_on_a_log_scale() {
    todo!()
}

// With `restore(Some(80.0))` the button shows and reports `Set(80)`; values clamp to 1..=max.
#[test]
#[ignore = "skeleton"]
fn restore_sets_the_original_size() {
    todo!()
}
