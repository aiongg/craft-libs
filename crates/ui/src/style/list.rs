//! Style of the `list` family.

use craft_theme::{Contrast, Rgba, visible_hover};

use crate::theme::UiTokens;

/// Offset of the drag ghost from the pointer.
const GHOST_OFFSET: [f32; 2] = [12.0, -10.0];
/// Opacity of the hover fill of a row.
const HOVER_OPACITY: f32 = 0.5;
/// Opacity of the stripe of an even row.
const STRIPE_OPACITY: f32 = 0.25;
/// Opacity of the accent tint in a layer row's hover fill.
const LAYER_TINT_OPACITY: f32 = 0.12;

/// Sizes and fills of list rows that the semantic tokens don't name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ListStyle {
    /// Height of a dense row.
    pub row_height_dense: f32,
    /// Least height of a row with a thumbnail.
    pub thumb_row_min: f32,
    /// Width of a toggle column.
    pub toggle_column: f32,
    /// Least width of the label.
    pub min_label: f32,
    /// Offset of the drag ghost from the pointer.
    pub ghost_offset: [f32; 2],
    /// Fill of a hovered row.
    pub hover: Rgba,
    /// Fill of a striped row.
    pub stripe: Rgba,
    /// Fill of a hovered layer row.
    pub layer_hover: Rgba,
}

impl ListStyle {
    /// The style for `tokens`. In high contrast the hover fills move until they set a row off from
    /// the panel and the selected fill, and the text still reads on them.
    pub fn new(tokens: &UiTokens) -> Self {
        let (space, size, color) = (&tokens.space, &tokens.size, &tokens.color);
        let high = tokens.appearance.contrast == Contrast::High;
        let mut hover = half(color.state.hover);
        let mut layer_hover = stack(color.accent.solid.faded(LAYER_TINT_OPACITY), hover);
        if high {
            hover = visible_hover(hover, color.surface.panel, &[color.state.selected], color.text.normal, true);
            let moved = visible_hover(layer_hover, color.surface.panel, &[], color.text.normal, true);
            let readable = craft_theme::contrast_ratio(color.text.normal, craft_theme::over(moved, color.surface.panel)) >= 7.0;
            layer_hover = if readable { moved } else { hover };
        }
        Self {
            row_height_dense: tokens.typography.body.line_height + 2.0 * space.gap.tight,
            thumb_row_min: size.thumb.md + 2.0 * space.gap.tight,
            toggle_column: size.control.md + space.gap.inline,
            min_label: 3.0 * size.control.md,
            ghost_offset: GHOST_OFFSET,
            hover,
            stripe: color.state.hover.faded(STRIPE_OPACITY),
            layer_hover,
        }
    }
}

/// An opaque hover colour at half opacity; a translucent one is already a partial overlay.
fn half(hover: Rgba) -> Rgba {
    if hover.0[3] == 255 { hover.faded(HOVER_OPACITY) } else { hover }
}

/// `top` painted over `bottom`, both possibly translucent.
fn stack(top: Rgba, bottom: Rgba) -> Rgba {
    let (at, ab) = (f64::from(top.0[3]) / 255.0, f64::from(bottom.0[3]) / 255.0);
    let a = at + ab * (1.0 - at);
    if a <= 0.0 {
        return Rgba::TRANSPARENT;
    }
    let channel = |i: usize| ((f64::from(top.0[i]) * at + f64::from(bottom.0[i]) * ab * (1.0 - at)) / a).round() as u8;
    Rgba([channel(0), channel(1), channel(2), (a * 255.0).round() as u8])
}
