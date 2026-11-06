//! Style of the `list` family.

use craft_theme::{Contrast, Identify, Oklab, Rgba, contrast_ratio, ensure_contrast, identifiable_outline, over, visible_hover};

use crate::theme::UiTokens;

/// Offset of the drag ghost from the pointer.
const GHOST_OFFSET: [f32; 2] = [12.0, -10.0];
/// Opacity of the hover fill of a row.
const HOVER_OPACITY: f32 = 0.5;
/// Opacity of the stripe of an even row.
const STRIPE_OPACITY: f32 = 0.25;
/// Opacity of the accent tint in a layer row's hover fill.
const LAYER_TINT_OPACITY: f32 = 0.12;

/// How far the selected row's quiet text moves from its text toward the selected fill.
const SELECTED_QUIET: f64 = 0.4;
/// Least contrast ratio of text on a fill: standard, high.
const TEXT_MIN: (f64, f64) = (4.5, 7.0);
/// Least contrast ratio of a state fill on the panel: standard, high.
const NON_TEXT_MIN: (f64, f64) = (3.0, 4.5);
/// Least step of a selected fill beside its hovered fill, as a contrast ratio.
const HOVER_STEP_MIN: f64 = 1.12;
/// Width of the layer row's selected outline.
pub const LAYER_OUTLINE_WIDTH: f32 = 1.0;

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
    /// Fill of a selected row.
    pub selected: Rgba,
    /// Fill of a selected row under the pointer.
    pub selected_hover: Rgba,
    /// Text of a selected row.
    pub selected_text: Rgba,
    /// Quiet text of a selected row (details, a hidden layer's name).
    pub selected_quiet: Rgba,
    /// Fill of a selected layer row.
    pub layer_selected: Rgba,
    /// Text of a selected layer row.
    pub layer_selected_text: Rgba,
    /// Quiet text of a selected layer row.
    pub layer_selected_quiet: Rgba,
    /// Outline of a selected layer row; transparent where the selected fill shows on the panel alone.
    pub layer_outline: Rgba,
}

/// A selected row's fills and text: unchanged in standard contrast, moved to identify the state on
/// the panel in high contrast.
struct SelectedLook {
    selected: Rgba,
    selected_hover: Rgba,
    text: Rgba,
    quiet: Rgba,
}

impl SelectedLook {
    /// `text` is the label on the unmoved fills; `edge` an outline that identifies the selected row
    /// as well as its fill.
    fn of(panel: Rgba, selected: Rgba, selected_hover: Rgba, text: Rgba, edge: Option<Rgba>, high: bool) -> Self {
        let on = [panel];
        let identify = || edge.map_or_else(|| Identify::on(&on, high), |e| Identify::on(&on, high).edge(e));
        let (moved, text_on) = identify().fill(selected, text);
        // A theme whose selected row has a distinct hover step keeps one.
        let stepped = contrast_ratio(over(selected_hover, panel), over(selected, panel)) >= HOVER_STEP_MIN;
        let apart: &[Rgba] = if stepped { &[moved] } else { &[] };
        let (moved_hover, text_on) = identify().apart(apart).with(&[moved]).fill(selected_hover, text_on);
        let fills = [moved, moved_hover];
        // Text that already reads on the unmoved fills stays as it is.
        let text = if fills == [selected, selected_hover] { text } else { readable_on(text_on, &fills, panel, high) };
        Self { selected: moved, selected_hover: moved_hover, text, quiet: quiet_on_selected(text, &fills, panel, high) }
    }
}

/// `text` moved until it reads (the text minimum) on every fill, over `rest`.
fn readable_on(text: Rgba, fills: &[Rgba], rest: Rgba, high: bool) -> Rgba {
    let min = if high { TEXT_MIN.1 } else { TEXT_MIN.0 };
    // Moving away from one fill can move toward the other only when they sit either side of the
    // text; a second round settles that case.
    (0..2).fold(text, |text, _| fills.iter().fold(text, |text, &fill| ensure_contrast(text, over(fill, rest), min)))
}

/// The selected row's text moved toward the selected fill, then as far back as text needs to read
/// on both selected fills.
fn quiet_on_selected(text: Rgba, fills: &[Rgba; 2], rest: Rgba, high: bool) -> Rgba {
    let quiet = mix(text, over(fills[0], rest), SELECTED_QUIET);
    readable_on(quiet, fills, rest, high)
}

/// Linear interpolation in OKLab, opaque.
fn mix(a: Rgba, b: Rgba, t: f64) -> Rgba {
    let (a, b) = (a.to_oklab(), b.to_oklab());
    let lerp = |x: f64, y: f64| x + (y - x) * t;
    Oklab { l: lerp(a.l, b.l), a: lerp(a.a, b.a), b: lerp(a.b, b.b) }.to_oklch().to_rgba()
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
        let panel = color.surface.panel;
        let plain = SelectedLook::of(panel, color.state.selected, color.state.selected_hover, color.text.on_selected, None, high);
        let bare = SelectedLook::of(panel, color.state.selected, color.state.selected, color.text.on_selected, None, high);
        // A selected layer carries an outline where its fill alone doesn't show on the panel.
        let min = if high { NON_TEXT_MIN.1 } else { NON_TEXT_MIN.0 };
        let shows = contrast_ratio(over(bare.selected, panel), panel) >= min;
        let outline = (!shows).then(|| identifiable_outline(color.line.control_edge, bare.selected, &[panel], high));
        let layer = SelectedLook::of(panel, color.state.selected, color.state.selected, color.text.on_selected, outline, high);
        Self {
            row_height_dense: tokens.typography.body.line_height + 2.0 * space.gap.tight,
            thumb_row_min: size.thumb.md + 2.0 * space.gap.tight,
            toggle_column: size.control.md + space.gap.inline,
            min_label: 3.0 * size.control.md,
            ghost_offset: GHOST_OFFSET,
            hover,
            stripe: color.state.hover.faded(STRIPE_OPACITY),
            layer_hover,
            selected: plain.selected,
            selected_hover: plain.selected_hover,
            selected_text: plain.text,
            selected_quiet: plain.quiet,
            layer_selected: layer.selected,
            layer_selected_text: layer.text,
            layer_selected_quiet: layer.quiet,
            layer_outline: outline.unwrap_or(Rgba::TRANSPARENT),
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
