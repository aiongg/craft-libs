//! `craft_platform::clipboard`.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// While synthetic, write_text and write_image answer Fenced and the backend keeps the user's
// copy; reads still work; after set_synthetic(false) writes go through.
#[test]
#[ignore = "skeleton"]
fn synthetic_input_never_replaces_what_the_user_copied() {
    todo!()
}

// An image over ClipboardLimits::image_bytes and text over text_bytes fail with TooLarge and the
// limit; an image whose rgba length doesn't match its size reads as None.
#[test]
#[ignore = "skeleton"]
fn reads_over_the_limits_fail_and_broken_images_are_dropped() {
    todo!()
}

// After write_image, holds_ours is true while the backend returns the same pixels and false once
// another image (or text) replaced them; forget_ours clears it.
#[test]
#[ignore = "skeleton"]
fn holds_ours_tells_our_copy_from_another_apps() {
    todo!()
}

// An EventClipboard reads text, an RGBA8 image and a text/uri-list (CRLF line ends trimmed) from
// the last offer, and hands writes to its out closure.
#[test]
#[ignore = "skeleton"]
fn event_clipboard_reads_the_offered_paste() {
    todo!()
}

// The probe reports a backend's has() verdict without blocking the caller, sends again only on a
// change, and dropping it ends the worker within PROBE_STOP_WAIT.
#[test]
#[ignore = "skeleton"]
fn probe_answers_off_the_ui_thread_and_stops_on_drop() {
    todo!()
}
