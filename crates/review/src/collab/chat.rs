//! Cursor chat: a short message next to your pointer that others see (N5.09).

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

/// Seconds a bubble stays after its last change (FigCraft `live.rs:76`).
pub const CHAT_HOLD_S: f64 = 5.0;
/// Seconds it takes to fade out after that (`live.rs:77`).
pub const CHAT_FADE_S: f64 = 1.0;
/// The bubble's width in points (`live.rs:79`).
pub const CHAT_WIDTH: f32 = 220.0;

/// A cursor chat message and when it last changed (monotonic seconds).
#[derive(Clone, Debug, PartialEq)]
pub struct CursorChat {
    pub text: String,
    pub changed: f64,
}

/// The bubble's opacity `age` seconds after its last change: 1 for [`CHAT_HOLD_S`], then down to
/// 0 over [`CHAT_FADE_S`]; with `reduce_motion` it stays 1 until the fade would end, then 0.
// figcraft: crates/ui-egui/src/live.rs:214 `chat_fade`.
pub fn chat_alpha(age: f64, reduce_motion: bool) -> f32 {
    todo!()
}
