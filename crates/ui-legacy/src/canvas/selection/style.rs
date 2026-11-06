//! [`SelectionFrameStyle`] and [`GripSetStyle`]: the looks of the selection frame, outlines and
//! grips, from tokens. Every size is the same at every Size: they sit on the document.

use craft_theme::Rgba;
use egui::Color32;

use super::{GripSet, SelectionFrame};
use crate::primitives::{Handle, HandleShape, HandleStyle};
use crate::theme::{ToEgui, UiTokens};

const CENTRE_MARK_SIDE: f32 = 3.5;
const GROW_STEP: f32 = 1.0;
const GRIP_WARM_PAD: f32 = 2.0;
/// A grip at rest.
const GRIP: Rgba = Rgba::rgb(0x2f, 0x6b, 0xff);
/// A grip the pointer is near.
const GRIP_WARM: Rgba = Rgba::rgb(0xff, 0x7f, 0x9f);
/// A grip picked for editing.
const GRIP_HOT: Rgba = Rgba::rgb(0xff, 0x3b, 0x3b);
/// A dynamic grip at rest.
const GRIP_DYNAMIC: Rgba = Rgba::rgb(0x26, 0xc6, 0xe8);
/// The outline of every grip.
const GRIP_STROKE: Rgba = Rgba::rgb(0x10, 0x30, 0x80);

/// What [`SelectionFrame`] and [`SelectionOutline`](super::SelectionOutline) draw.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SelectionFrameStyle {
    /// The default line colour when the host gives none: `overlay.line`.
    pub color: Color32,
    /// The box line and a solid outline: `stroke.thin`.
    pub line: f32,
    /// The key object's box line: `stroke.thick`.
    pub key_line: f32,
    /// The centre mark's side.
    pub centre: f32,
    /// A dashed outline's width: `stroke.medium`.
    pub outline: f32,
    /// A dashed outline's dash and gap: `stroke.dash` and `stroke.dash_gap`.
    pub dash: f32,
    /// The dash gap of a dashed outline, in screen points.
    pub gap: f32,
    /// What one [`grow`](SelectionFrame::grow) step adds to a grip.
    pub grow_step: f32,
    /// The grips: `Handle::style(tokens, HandleShape::Square)` (a square's side);
    /// the frame replaces the outline colour with its own.
    pub handle: HandleStyle,
}

/// What [`GripSet`] draws.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GripSetStyle {
    /// A square grip's side, a midpoint's width: `size.handle.md`.
    pub size: f32,
    /// A midpoint grip's height: half of `size.handle.md`.
    pub midpoint_height: f32,
    /// The outline width: `stroke.thin`.
    pub stroke: f32,
    /// The hit area beyond a grip's bounds: `size.hit.slop`.
    pub hit_pad: f32,
    /// The warm area beyond a grip's bounds: a constant.
    pub warm_pad: f32,
    /// Fills by state: #2f6bff, #ff7f9f and #ff3b3b.
    pub cold: Color32,
    #[allow(missing_docs)]
    pub warm: Color32,
    #[allow(missing_docs)]
    pub hot: Color32,
    /// A dynamic grip at rest: #26c6e8.
    pub dynamic: Color32,
    /// The outline colour: #103080.
    pub outline: Color32,
}

impl SelectionFrame<'_> {
    /// The selection frame's look in `tokens`.
    pub fn style(tokens: &UiTokens) -> SelectionFrameStyle {
        SelectionFrameStyle {
            color: tokens.color.overlay.line.to_egui(),
            line: tokens.stroke.thin,
            key_line: tokens.stroke.thick,
            centre: CENTRE_MARK_SIDE,
            outline: tokens.stroke.medium,
            dash: tokens.stroke.dash,
            gap: tokens.stroke.dash_gap,
            grow_step: GROW_STEP,
            handle: Handle::style(tokens, HandleShape::Square),
        }
    }
}

impl GripSet<'_> {
    /// The grips' look in `tokens`.
    pub fn style(tokens: &UiTokens) -> GripSetStyle {
        GripSetStyle {
            size: tokens.size.handle.md,
            midpoint_height: tokens.size.handle.md / 2.0,
            stroke: tokens.stroke.thin,
            hit_pad: tokens.size.hit.slop,
            warm_pad: GRIP_WARM_PAD,
            cold: GRIP.to_egui(),
            warm: GRIP_WARM.to_egui(),
            hot: GRIP_HOT.to_egui(),
            dynamic: GRIP_DYNAMIC.to_egui(),
            outline: GRIP_STROKE.to_egui(),
        }
    }
}
