//! Stroke editor: the popover that edits a stroke, its opener and its samples. The same at every Size. Fields, selects and checks keep their own families; the samples draw in `text.normal`, `field.fill`, `text.faint` and `status.bad.fg`.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Stroke editors: the popover that edits a shape's stroke, its opener and its samples.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct StrokeEditorTokens {
    /// The popover body's width: `stroke_editor.width`.
    pub width: f32,
    /// The room kept between the popover and the window's edge: `stroke_editor.screen_margin`.
    pub screen_margin: f32,
    /// The room kept above and below the popover: `stroke_editor.screen_margin_y`.
    pub screen_margin_y: f32,
    /// The narrowest the body gets in a small window: `stroke_editor.min_width`.
    pub min_width: f32,
    /// The shortest the popover's scrolling body gets: `stroke_editor.min_height`.
    pub min_height: f32,
    /// The opener's width: `stroke_editor.opener_width`.
    pub opener_width: f32,
    /// The opener's height: `stroke_editor.opener_height`.
    pub opener_height: f32,
    /// A preset row's height: `stroke_editor.preset_height`.
    pub preset_height: f32,
    /// A choice select's width: `stroke_editor.choice_width`.
    pub choice_width: f32,
    /// A number field's width: `stroke_editor.field_width`.
    pub field_width: f32,
    /// The tall sample's height: `stroke_editor.sample_height`.
    pub sample_height: f32,
    /// The samples' line width: `stroke_editor.sample_stroke`.
    pub sample_stroke: f32,
    /// The tall sample's rectangle inset: `stroke_editor.sample_inset`.
    pub sample_inset: f32,
    /// The open sample line's distance from the sample's ends: `stroke_editor.sample_margin`.
    pub sample_margin: f32,
    /// The off opener's line width: `stroke_editor.off_stroke`.
    pub off_stroke: f32,
    /// The off opener's slash width: `stroke_editor.off_slash_stroke`.
    pub off_slash_stroke: f32,
}

/// This family has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StrokeEditorField {}

impl Family for StrokeEditorTokens {
    type Field = StrokeEditorField;
    const FIELDS: &'static [StrokeEditorField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { StrokeEditorTokens;
        "width" 0.0..=4096.0 => width;
        "screen_margin" 0.0..=512.0 => screen_margin;
        "screen_margin_y" 0.0..=512.0 => screen_margin_y;
        "min_width" 0.0..=4096.0 => min_width;
        "min_height" 0.0..=4096.0 => min_height;
        "opener_width" 0.0..=1024.0 => opener_width;
        "opener_height" 0.0..=512.0 => opener_height;
        "preset_height" 0.0..=512.0 => preset_height;
        "choice_width" 0.0..=1024.0 => choice_width;
        "field_width" 0.0..=1024.0 => field_width;
        "sample_height" 0.0..=512.0 => sample_height;
        "sample_stroke" 0.0..=16.0 => sample_stroke;
        "sample_inset" 0.0..=512.0 => sample_inset;
        "sample_margin" 0.0..=512.0 => sample_margin;
        "off_stroke" 0.0..=16.0 => off_stroke;
        "off_slash_stroke" 0.0..=16.0 => off_slash_stroke;
    };

    fn defaults(_: &Semantic<'_>) -> StrokeEditorTokens {
        StrokeEditorTokens {
            width: values::STROKE_EDITOR_WIDTH,
            screen_margin: values::STROKE_EDITOR_SCREEN_MARGIN,
            screen_margin_y: values::STROKE_EDITOR_SCREEN_MARGIN_Y,
            min_width: values::STROKE_EDITOR_MIN_WIDTH,
            min_height: values::STROKE_EDITOR_MIN_HEIGHT,
            opener_width: values::STROKE_EDITOR_OPENER_WIDTH,
            opener_height: values::STROKE_EDITOR_OPENER_HEIGHT,
            preset_height: values::STROKE_EDITOR_PRESET_HEIGHT,
            choice_width: values::STROKE_EDITOR_CHOICE_WIDTH,
            field_width: values::STROKE_EDITOR_FIELD_WIDTH,
            sample_height: values::STROKE_EDITOR_SAMPLE_HEIGHT,
            sample_stroke: values::STROKE_EDITOR_SAMPLE_STROKE,
            sample_inset: values::STROKE_EDITOR_SAMPLE_INSET,
            sample_margin: values::STROKE_EDITOR_SAMPLE_MARGIN,
            off_stroke: values::STROKE_EDITOR_OFF_STROKE,
            off_slash_stroke: values::STROKE_EDITOR_OFF_SLASH_STROKE,
        }
    }

    fn path(f: StrokeEditorField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<StrokeEditorField> {
        None
    }

    fn kind(f: StrokeEditorField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: StrokeEditorField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: StrokeEditorField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
