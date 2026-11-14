//! [`GradientFillEditor`]: the gradient fill rows of a shape or a Gradient Fill layer.

use craft_behavior::EditEvents;
use craft_color::ui::ScreenPickGesture;
use craft_color::ui::gradient::{GradientStops, StopColor};
use craft_theme::Srgba;
use craft_ui::contract::{Bind, NoValue, Outcome};
use craft_ui::patterns::presets::PresetState;
use craft_ui::runtime::env::Label;
use egui::{Response, Ui};

use super::{GradientFill, GradientGroup, GradientRef};

craft_ui::words! {
    /// The words of the gradient fill rows.
    pub struct GradientFillWords {
        linear: "gradientFill.linear" = "Linear";
        radial: "gradientFill.radial" = "Radial";
        angle_style: "gradientFill.angleStyle" = "Angle";
        reflected: "gradientFill.reflected" = "Reflected";
        diamond: "gradientFill.diamond" = "Diamond";
        style: "gradientFill.style" = "Style";
        angle: "gradientFill.angle" = "Angle";
        scale: "gradientFill.scale" = "Scale";
        reverse: "gradientFill.reverse" = "Reverse";
        dither: "gradientFill.dither" = "Dither";
        align: "gradientFill.align" = "Align with layer";
        reset_alignment: "gradientFill.resetAlignment" = "Reset Alignment";
        options: "gradientFill.options" = "Gradient Options";
    }
}

/// The shape gradient tab and the fill layer's Gradient Fill rows: the palette, the stop editor, the
/// style, angle, scale, reverse, dither and align rows, and Reset Alignment. Each child's events
/// become events of the whole value; the editor keeps no state between passes.
#[must_use = "show the editor with `show`"]
#[derive(craft_ui::contract::Pattern)]
pub struct GradientFillEditor<'a> {
    label: Label,
    value: Bind<'a, GradientFill>,
    groups: &'a [GradientGroup],
    state: Bind<'a, PresetState<GradientRef>>,
    recent: &'a [GradientStops],
    sampled: Option<Srgba>,
}

/// What [`GradientFillEditor::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies the editor's edits"]
pub struct GradientFillEditorOutput {
    /// The editor.
    pub response: Response,
    /// The edits of the whole value.
    pub events: EditEvents<GradientFill>,
    /// A preset picked in the palette.
    pub picked: Option<GradientRef>,
    /// The colour stop whose colour the host is asked to edit.
    pub color_requested: Option<StopColor>,
    /// The popover picker's screen-pick press; the host answers with `sampled`.
    pub screen_pick: Option<ScreenPickGesture>,
}

impl Outcome for GradientFillEditorOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

impl<'a> GradientFillEditor<'a> {
    /// The editor of `value`, offering the preset `groups` with their palette `state`.
    pub fn new(
        label: impl Into<Label>,
        value: Bind<'a, GradientFill>,
        groups: &'a [GradientGroup],
        state: Bind<'a, PresetState<GradientRef>>,
    ) -> Self {
        GradientFillEditor { label: label.into(), value, groups, state, recent: &[], sampled: None }
    }

    /// The recently used gradients, shown first in the palette.
    pub fn recent(mut self, recent: &'a [GradientStops]) -> Self {
        self.recent = recent;
        self
    }

    /// The colour a screen pick sampled.
    pub fn sampled(mut self, sampled: Option<Srgba>) -> Self {
        self.sampled = sampled;
        self
    }

    /// Shows the editor.
    pub fn show(self, ui: &mut Ui) -> GradientFillEditorOutput {
        let GradientFillEditor { label, value, groups, state, recent, sampled } = self;
        let _ = (ui, label, value, groups, state, recent, sampled);
        todo!("tl.T35")
    }
}
