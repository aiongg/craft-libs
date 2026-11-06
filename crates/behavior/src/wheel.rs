//! Wheel gesture values, as craft-behavior's copy of egui's types so the wheel split is egui-free.

/// How far one wheel event reaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum WheelUnit {
    /// Points.
    #[default]
    Point,
    /// Lines of text.
    Line,
    /// Pages.
    Page,
}

/// A wheel gesture's phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum WheelPhase {
    /// The first event of a gesture.
    Start,
    /// A later event.
    #[default]
    Move,
    /// The fingers lifted.
    End,
    /// The system cancelled the gesture.
    Cancel,
}
