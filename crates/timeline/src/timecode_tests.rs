//! Formatting and parsing time.

#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// frame 1800 at 29.97 DF is 00;01;00;02, frame 17982 is 00;10;00;00.
#[test]
#[ignore = "skeleton"]
fn drop_frame_skips_two_labels_a_minute() {
    todo!()
}

// timecode, frames with an offset, feet + frames 35 and 16, samples and seconds.
#[test]
#[ignore = "skeleton"]
fn formats_every_display() {
    todo!()
}

// 01:02:03:04, 1.2.3.4, bare 1000, +15 and -1.00 relative input.
#[test]
#[ignore = "skeleton"]
fn parses_premiere_input() {
    todo!()
}

// empty input and 1:x give the errors.
#[test]
#[ignore = "skeleton"]
fn refuses_what_is_not_a_time() {
    todo!()
}
