//! [`StrokeEditorStyle`]: the stroke editor's and its button's look, from tokens.
//! Panel sizes: the same at both Sizes.

use egui::Color32;

use super::StrokeEditor;
use craft_ui::theme::{ToEgui, UiTokens};

/// The popover is never narrower than this.
const MIN_WIDTH: f32 = 160.0;
/// The popover's scrolling height is never under this.
const MIN_HEIGHT: f32 = 80.0;
/// Widths of the choice select, the number field and the opener, in control heights.
const CHOICE_WIDTH_CONTROLS: f32 = 5.0;
const FIELD_WIDTH_CONTROLS: f32 = 2.5;
const OPENER_WIDTH_CONTROLS: f32 = 4.5;

/// What [`StrokeEditor`], [`StrokeButton`](super::StrokeButton) and
/// [`StrokePreview`](super::StrokePreview) draw. Fields, selects, checks and buttons keep their
/// own styles.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct StrokeEditorStyle {
    /// The popover body's width: a choice select beside a number field with their gaps and the
    /// panel padding, never wider than the window less the screen margin, never narrower than
    /// the least width.
    pub width: f32,
    #[allow(missing_docs)]
    pub screen_margin: f32,
    #[allow(missing_docs)]
    pub min_width: f32,
    /// The popover's scrolling height: the window's less this margin, never under the least height.
    pub screen_margin_y: f32,
    #[allow(missing_docs)]
    pub min_height: f32,
    /// The opener's width.
    pub opener_width: f32,
    #[allow(missing_docs)]
    pub opener_height: f32,
    /// A preset row's height.
    pub preset_height: f32,
    /// A choice select's width.
    pub choice_width: f32,
    /// A number field's width.
    pub field_width: f32,
    /// The tall sample's height.
    pub sample_height: f32,
    /// The samples' line width.
    pub sample_stroke: f32,
    /// The tall sample's rectangle inset.
    pub sample_inset: f32,
    /// The open sample line's distance from the sample's ends.
    pub sample_margin: f32,
    /// The samples' colour: `text.normal` on `field.fill` (4.5:1 in High contrast).
    pub sample_color: Color32,
    /// The opener's sample while the stroke is off: a `text.faint` line crossed by a
    /// `status.bad.fg` slash, `stroke.thin` and `stroke.medium` wide.
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
        let (c, size, space, stroke) = (&tokens.color, &tokens.size, &tokens.space, &tokens.stroke);
        let choice_width = size.control.md * CHOICE_WIDTH_CONTROLS;
        let field_width = size.control.md * FIELD_WIDTH_CONTROLS;
        StrokeEditorStyle {
            width: choice_width + field_width + space.gap.stack + 2.0 * space.pad.panel,
            screen_margin: space.pad.dialog,
            min_width: MIN_WIDTH,
            screen_margin_y: space.pad.dialog + size.bar.title,
            min_height: MIN_HEIGHT,
            opener_width: size.control.md * OPENER_WIDTH_CONTROLS,
            opener_height: size.control.sm,
            preset_height: size.control.md + 2.0 * space.gap.inline,
            choice_width,
            field_width,
            sample_height: size.bar.options,
            sample_stroke: stroke.thick,
            sample_inset: space.pad.panel,
            sample_margin: space.gap.tight,
            sample_color: c.text.normal.to_egui(),
            off_line: c.text.faint.to_egui(),
            off_slash: c.status.bad.fg.to_egui(),
            off_stroke: stroke.thin,
            off_slash_stroke: stroke.medium,
            error: c.status.bad.fg.to_egui(),
        }
    }
}
