//! Pen input.

use super::{PenFeed, PenInput, PenSample};

fn pen(pressure: f32, eraser: bool) -> PenSample {
    PenSample { pressure, eraser, ..PenSample::MOUSE }
}

// Without a pen, sample() is pressure 1, tilt 0, no eraser.
#[test]
fn a_mouse_sample_has_full_pressure_and_no_tilt() {
    let mut input = PenInput::new().with_feed(PenFeed::new());
    input.update(&[]);
    let sample = input.sample();
    assert_eq!((sample.pressure, sample.tilt, sample.eraser), (1.0, [0.0, 0.0], false));
    assert!(!input.pen_in_use());
}

// A curve through (0.5, 0.25) maps pressure 0.5 to 0.25.
#[test]
fn pressure_follows_the_pressure_curve() {
    let feed = PenFeed::new();
    let mut input = PenInput::new().with_feed(feed.clone());
    input.set_pressure_curve(&[[0.0, 0.0], [0.5, 0.25], [1.0, 1.0]]);
    feed.push(pen(0.5, false));
    input.update(&[]);
    assert!((input.sample().pressure - 0.25).abs() < 1e-6);
    // The preference off reads the pen like a mouse.
    input.use_pressure(false);
    assert_eq!(input.sample().pressure, 1.0);
}

// take_eraser_flip is Some(true) once after the eraser end arrives.
#[test]
fn the_eraser_end_reports_its_flip_once() {
    let feed = PenFeed::new();
    let mut input = PenInput::new().with_feed(feed.clone());
    feed.push(pen(0.4, true));
    input.update(&[]);
    assert_eq!(input.take_eraser_flip(), Some(true));
    assert_eq!(input.take_eraser_flip(), None);
    feed.push(pen(0.4, true));
    input.update(&[]);
    assert_eq!(input.take_eraser_flip(), None);
    feed.push(pen(0.4, false));
    input.update(&[]);
    assert_eq!(input.take_eraser_flip(), Some(false));
}
