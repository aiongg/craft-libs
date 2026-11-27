//! `Slider`, `RangeSlider`, `AngleDial`, `PointPicker` and `ThumbRow`, built on `ValueDrag`.

#[cfg(spike_full)]
pub(crate) mod angle_dial;
#[cfg(spike_full)]
pub(crate) mod point_picker;
#[cfg(spike_full)]
pub(crate) mod range_slider;
pub(crate) mod slider;
#[cfg(spike_full)]
pub(crate) mod thumb_row;
pub(crate) mod track;
#[cfg(spike_full)]
mod words;

#[cfg(spike_full)]
pub use angle_dial::{AngleDial, AngleDialStyle, AngleOrientation};
#[cfg(spike_full)]
pub use point_picker::{Backdrop, PointMark, PointPicker, PointPickerStyle, PointPickerWords, SamePoint};
#[cfg(spike_full)]
pub use range_slider::{RangeSlider, RangeSliderStyle};
pub use slider::{Slider, SliderStyle, SliderThumb, TrackPress};
#[cfg(spike_full)]
pub use thumb_row::{
    Anchor, Follow, Neighbours, OffThumbs, Order, RowOp, RowRules, SpanInk, Thumb, ThumbHold, ThumbOp, ThumbRow, ThumbRowOutput, ThumbRowStyle,
    ThumbSpan,
};
pub use track::{SliderScale, TrackFace};
#[cfg(spike_full)]
pub use words::ValueDragWords;
