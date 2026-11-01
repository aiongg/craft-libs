//! [`SelectionFrameStyle`] and [`GripSetStyle`]: the looks of the selection frame, outlines and
//! grips, from tokens (task TK1). Every size is the same at every Size: they sit on the document.

use egui::Color32;

use super::{GripSet, SelectionFrame};
use crate::primitives::{Handle, HandleShape, HandleStyle};
use crate::theme::{ToEgui, UiTokens};

/// What [`SelectionFrame`] and [`SelectionOutline`](super::SelectionOutline) draw.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SelectionFrameStyle {
    /// The default line colour when the host gives none: `overlay.line`.
    pub colour: Color32,
    /// The box line and a solid outline: `selection_frame.line` (1, DesignCraft).
    pub line: f32,
    /// The key object's box line: `selection_frame.key_line` (3, DesignCraft; VectorCraft 2.5).
    pub key_line: f32,
    /// The centre mark's side: `selection_frame.centre` (3.5, DesignCraft).
    pub centre: f32,
    /// A dashed outline's width: `selection_frame.outline` (1.5, CadCraft).
    pub outline: f32,
    /// A dashed outline's dash and gap: `selection_frame.dash` (4) and `selection_frame.gap` (3).
    pub dash: f32,
    pub gap: f32,
    /// What one [`grow`](SelectionFrame::grow) step adds to a grip: `selection_frame.grow_step`
    /// (1, VectorCraft's Size steps).
    pub grow_step: f32,
    /// The grips: `Handle::style(tokens, HandleShape::Square)` (`handle.size` 7, `handle.hit` 14);
    /// the frame replaces the outline colour with its own.
    pub handle: HandleStyle,
}

/// What [`GripSet`] draws.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GripSetStyle {
    /// A square grip's side, a midpoint's width: `grip.size` (10, CadCraft `gripsize` 5 × 2).
    pub size: f32,
    /// A midpoint grip's height: `grip.midpoint_height` (5).
    pub midpoint_height: f32,
    /// The outline width: `grip.stroke` (1).
    pub stroke: f32,
    /// The hit area beyond a grip's bounds: `grip.hit_pad` (3, CadCraft `grips.rs:95`).
    pub hit_pad: f32,
    /// The warm area beyond a grip's bounds: `grip.warm_pad` (2, CadCraft `canvas.rs:1062`).
    pub warm_pad: f32,
    /// Fills by state: `overlay.grip` (#2f6bff), `overlay.grip_warm` (#ff7f9f), `overlay.grip_hot`
    /// (#ff3b3b), CadCraft's pinned colours.
    pub cold: Color32,
    pub warm: Color32,
    pub hot: Color32,
    /// The outline colour: `overlay.grip_stroke` (#103080).
    pub outline: Color32,
}

impl SelectionFrame<'_> {
    /// The selection frame's look in `tokens`.
    pub fn style(tokens: &UiTokens) -> SelectionFrameStyle {
        // designcraft: crates/ui-egui/src/canvas/frames.rs:180 (key 3), :249 (centre 3.5);
        // cadcraft: crates/ui-egui/src/canvas/draw.rs:269 (1.5 dashed); the handle from
        // `Handle::style(tokens, HandleShape::Square)`.
        let f = &tokens.component.selection_frame;
        SelectionFrameStyle {
            colour: tokens.color.overlay.line.to_egui(),
            line: f.line,
            key_line: f.key_line,
            centre: f.centre,
            outline: f.outline,
            dash: f.dash,
            gap: f.gap,
            grow_step: f.grow_step,
            handle: Handle::style(tokens, HandleShape::Square),
        }
    }
}

impl GripSet<'_> {
    /// The grips' look in `tokens`.
    pub fn style(tokens: &UiTokens) -> GripSetStyle {
        // cadcraft: crates/ui-egui/src/canvas.rs:1062-1074 (warm 2, fills, 1 pt #103080 outline);
        // crates/ui-egui/src/canvas/grips.rs:71-75 (size), :95 (hit 3).
        let (g, o) = (&tokens.component.grip, &tokens.color.overlay);
        GripSetStyle {
            size: g.size,
            midpoint_height: g.midpoint_height,
            stroke: g.stroke,
            hit_pad: g.hit_pad,
            warm_pad: g.warm_pad,
            cold: o.grip.to_egui(),
            warm: o.grip_warm.to_egui(),
            hot: o.grip_hot.to_egui(),
            outline: o.grip_stroke.to_egui(),
        }
    }
}
