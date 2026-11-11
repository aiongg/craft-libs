//! Formatting and parsing time.

use crate::rates::{FrameRate, SampleRate};
use crate::time::{TICKS_PER_SECOND, Tick};
use crate::timecode::{TimeDisplay, TimeFormat, TimecodeFields, fields_to_frames, format_time, frames_to_fields, parse_time};
use craft_ui::testing::trial;

fn df() -> TimeFormat {
    TimeFormat::timecode(FrameRate::FPS_29_97).drop_frame(true)
}

#[trial]
fn drop_frame_skips_two_labels_a_minute() {
    let rate = FrameRate::FPS_29_97;
    assert_eq!(format_time(rate.tick_of(1800), &df()), "00;01;00;02");
    assert_eq!(format_time(rate.tick_of(17_982), &df()), "00;10;00;00");
    assert_eq!(format_time(rate.tick_of(1799), &df()), "00;00;59;29");
    for frame in [0, 1, 1799, 1800, 17_981, 17_982, 107_892, -1800] {
        let fields = frames_to_fields(frame, rate, true);
        assert_eq!(fields_to_frames(fields, rate, true) * if fields.negative { -1 } else { 1 }, frame, "frame {frame}");
    }
    assert_eq!(frames_to_fields(-30, rate, false), TimecodeFields { negative: true, hours: 0, minutes: 0, seconds: 1, frames: 0 });
}

#[trial]
fn parses_premiere_input() {
    let rate = FrameRate::FPS_25;
    let tc = TimeFormat::timecode(rate);
    let frames = |input: &str, current: i64| rate.frame_at(parse_time(input, &tc, rate.tick_of(current)).expect(input));
    assert_eq!(frames("01:02:03:04", 0), 93_079);
    assert_eq!(frames("01;02;03;04", 0), 93_079);
    assert_eq!(frames("1.2.3.4", 0), 93_079);
    assert_eq!(frames("1000", 0), 250);
    assert_eq!(frames("0:0:0:90", 0), 90);
    assert_eq!(frames("+15", 100), 115);
    assert_eq!(frames("-1.00", 100), 75);
    assert_eq!(parse_time("01;00;00;02", &df(), Tick::ZERO), Ok(FrameRate::FPS_29_97.tick_of(107_894)));

    let other = |display, input: &str| parse_time(input, &tc.display(display), Tick::ZERO).map(|t| rate.frame_at(t));
    assert_eq!(other(TimeDisplay::Frames, "250"), Ok(250));
    assert_eq!(parse_time("250", &TimeFormat { frame_offset: 1, ..tc }.display(TimeDisplay::Frames), Tick::ZERO).map(|t| rate.frame_at(t)), Ok(249));
    assert_eq!(other(TimeDisplay::Feet35, "15+10"), Ok(250));
    assert_eq!(other(TimeDisplay::Feet16, "6+10"), Ok(250));
    assert_eq!(other(TimeDisplay::Seconds, "10.5"), Ok(262));
    assert_eq!(parse_time("00:00:10:24000", &tc.display(TimeDisplay::AudioSamples), Tick::ZERO), Ok(Tick(TICKS_PER_SECOND * 21 / 2)));
    assert_eq!(SampleRate::default().hz(), 48_000);
}
