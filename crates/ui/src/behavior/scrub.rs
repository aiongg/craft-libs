//! Scrubbing: a horizontal drag that changes a value (a number field's text or label, a scrubby
//! caption in the options bar). [`travel`] measures each pointer move; in a full-screen viewport
//! the pointer wraps from one screen edge to the other and the jump adds nothing.
//!
//! The wrap moves the system pointer with [`Scrub::warp`]: through the warp service the app
//! installed ([`Scrub::install_warp`], craft-platform's `pointer::warp` on macOS, where egui's
//! `ViewportCommand::CursorPosition` lags a frame and the pointer must be re-associated), else
//! through egui. Where the platform does not move the pointer on request (Wayland without a
//! pointer lock) the scrub stops at the edge.

use std::sync::Arc;

use egui::{Context, Id, Pos2};

/// How far from the screen's edge the pointer counts as at it, and how far inside the opposite
/// edge it is put.
const WRAP_EDGE: f32 = 2.0;

/// What one pointer position adds to a scrub.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Travel {
    /// Points moved, not counting a warp.
    pub delta: f32,
    /// The x to measure the next move from.
    pub last_x: f32,
    /// The x the pointer was asked to go to and has not been seen at.
    pub warp: Option<f32>,
    /// The x to move the pointer to now.
    pub send: Option<f32>,
}

/// The points the pointer moved from `last_x` to `x`. With the horizontal `edges` of the screen,
/// a move that reaches one asks for the pointer at the other (`send`), once: the request stays
/// pending until the pointer is seen nearer the target than it was, and the position that
/// then shows up there is not a move. A pointer that stays where it was (a platform that does not
/// move it) waits; one that moves elsewhere is an ordinary move, and the request is not sent
/// again.
pub fn travel(last_x: f32, x: f32, warp: Option<f32>, edges: Option<(f32, f32)>) -> Travel {
    let mut now = Travel { delta: x - last_x, last_x: x, warp: None, send: None };
    if let Some(target) = warp {
        if x == last_x {
            return Travel { delta: 0.0, warp, ..now };
        }
        if (x - target).abs() < (x - last_x).abs() {
            return Travel { delta: 0.0, ..now };
        }
        return Travel { warp, ..now };
    }
    if let Some((left, right)) = edges {
        let target = if now.delta > 0.0 && x >= right - WRAP_EDGE {
            Some(left + 2.0 * WRAP_EDGE)
        } else if now.delta < 0.0 && x <= left + WRAP_EDGE {
            Some(right - 2.0 * WRAP_EDGE)
        } else {
            None
        };
        (now.warp, now.send) = (target, target);
    }
    now
}

/// A function that moves the system pointer to a point in the viewport (egui points) and
/// returns whether it did.
#[expect(dead_code, reason = "skeleton: used when the bodies land")]
type Warp = Arc<dyn Fn(&Context, Pos2) -> bool + Send + Sync>;

/// The pointer warp service of a context.
pub struct Scrub;

impl Scrub {
    /// Install the app's pointer warp for `ctx` (once at startup). craft-platform's
    /// `pointer::warp` is the macOS one; an app without one leaves egui's.
    #[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the body lands")]
    pub fn install_warp(ctx: &Context, warp: impl Fn(&Context, Pos2) -> bool + Send + Sync + 'static) {
        todo!()
    }

    /// Move the system pointer to `pos` (egui points in the current viewport) for a scrub's
    /// wrap: the installed service if any, else `ViewportCommand::CursorPosition`. Returns
    /// `false` when the installed service could not move it (the scrub then waits at the edge,
    /// see [`travel`]).
    // pc: crates/ui-egui/src/input/scrub_wrap.rs:15 `wrap_target`, number_field.rs `ViewportCommand::CursorPosition`.
    #[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the body lands")]
    pub fn warp(ctx: &Context, pos: Pos2) -> bool {
        todo!()
    }

    #[expect(dead_code, reason = "skeleton: used when the bodies land")]
    fn id() -> Id {
        Id::unique("craft-ui-scrub-warp")
    }
}
