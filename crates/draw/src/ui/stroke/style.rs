//! [`StrokeEditorStyle`]: the stroke editor's and its button's look, from tokens.
//! Panel sizes: the same at both Sizes.

use egui::Color32;

use super::StrokeEditor;
use craft_ui::theme::{ToEgui, UiTokens};

/// What [`StrokeEditor`], [`StrokeButton`](super::StrokeButton) and
/// [`StrokePreview`](super::StrokePreview) draw. Fields, selects, checks and buttons keep their
/// own component tokens (`field.*`, `option_list.*`, `checkbox.*`, `popover.*`).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct StrokeEditorStyle {
    /// The popover body's width: `stroke_editor.width` (272), never wider than the window less
    /// `stroke_editor.screen_margin` (24), never narrower than `stroke_editor.min_width` (160).
    pub width: f32,
    #[allow(missing_docs)]
    pub screen_margin: f32,
    #[allow(missing_docs)]
    pub min_width: f32,
    /// The popover's scrolling height: the window's less `stroke_editor.screen_margin_y` (40),
    /// never under `stroke_editor.min_height` (80).
    pub screen_margin_y: f32,
    #[allow(missing_docs)]
    pub min_height: f32,
    /// The opener: `stroke_editor.opener_width` (138) by `stroke_editor.opener_height` (24).
    pub opener_width: f32,
    #[allow(missing_docs)]
    pub opener_height: f32,
    /// A preset row's height: `stroke_editor.preset_height` (32).
    pub preset_height: f32,
    /// A choice select's width: `stroke_editor.choice_width` (152).
    pub choice_width: f32,
    /// A number field's width: `stroke_editor.field_width` (74).
    pub field_width: f32,
    /// The tall sample's height: `stroke_editor.sample_height` (58).
    pub sample_height: f32,
    /// The samples' line width: `stroke_editor.sample_stroke` (2.5).
    pub sample_stroke: f32,
    /// The tall sample's rectangle inset: `stroke_editor.sample_inset` (12).
    pub sample_inset: f32,
    /// The open sample line's distance from the sample's ends: `stroke_editor.sample_margin` (4).
    pub sample_margin: f32,
    /// The samples' colour: `text.normal` on `field.fill` (4.5:1 in High contrast).
    pub sample_color: Color32,
    /// The opener's sample while the stroke is off: a `text.faint` line crossed by a
    /// `status.bad.fg` slash, `stroke_editor.off_stroke` (1) and `stroke_editor.off_slash_stroke`
    /// (1.5) wide.
    pub off_line: Color32,
    #[allow(missing_docs)]
    pub off_slash: Color32,
    #[allow(missing_docs)]
    pub off_stroke: f32,
    #[allow(missing_docs)]
    pub off_slash_stroke: f32,
    /// The error line's colour: `status.bad.fg`.
    pub error: Color32,
}

impl StrokeEditor<'_> {
    /// The stroke editor's look in `tokens`.
    pub fn style(tokens: &UiTokens) -> StrokeEditorStyle {
        // pc: crates/ui-egui/src/tools/shape_stroke_ui/editor.rs:130 (272, `- 24`, `max(160)`),
        // :92 (138), :43 (24 and 32), :266 (152), :291 (74), :230 (58), :332 (2.5), :322 (12),
        // :70-71 (the off sample: 1 pt faint line, 1.5 pt danger slash), :235 (danger).
        let (s, c) = (&tokens.component.stroke_editor, &tokens.color);
        StrokeEditorStyle {
            width: s.width,
            screen_margin: s.screen_margin,
            min_width: s.min_width,
            screen_margin_y: s.screen_margin_y,
            min_height: s.min_height,
            opener_width: s.opener_width,
            opener_height: s.opener_height,
            preset_height: s.preset_height,
            choice_width: s.choice_width,
            field_width: s.field_width,
            sample_height: s.sample_height,
            sample_stroke: s.sample_stroke,
            sample_inset: s.sample_inset,
            sample_margin: s.sample_margin,
            sample_color: c.text.normal.to_egui(),
            off_line: c.text.faint.to_egui(),
            off_slash: c.status.bad.fg.to_egui(),
            off_stroke: s.off_stroke,
            off_slash_stroke: s.off_slash_stroke,
            error: c.status.bad.fg.to_egui(),
        }
    }
}
