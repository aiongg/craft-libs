//! The Compact layout, and the parts both layouts show: the area and strip, the hex field, the opacity row and the
//! screen-pick button.

use std::sync::Arc;

use craft_behavior::Flow;
use craft_theme::ColorModel;
use craft_ui::buttons::IconButton;
use craft_ui::contract::{Bind, Cx, Edit as _, EditResponse, PaintUi, Props as _, SideLabel};
use craft_ui::pickers::Select;
use craft_ui::primitives::IconSpec;
use craft_ui::runtime::env::Label;
use craft_ui::text::TextField;
use craft_behavior::IconId;
use egui::{Rect, Response, Vec2};

use super::channel::{ChannelRule, ChannelUnit, channel_name, channel_slider};
use super::layout::{self, CompactInput};
use super::value::Rules;
use super::{ColorPicker, ColorPickerWords, PickedColor, ScreenPickGesture};
use crate::ui::{AreaMode, ColorArea, ColorStrip};

/// What a pass of the picker paints from.
pub(super) struct Pass<'p> {
    pub shown: PickedColor,
    pub rules: Rules<'p>,
    pub models: Vec<&'p dyn ColorModel>,
    pub mode: AreaMode,
    /// The model of the Compact rows.
    pub model: Option<&'static str>,
    pub flow: Flow,
    pub origin: Vec2,
    pub width: f32,
    pub words: Arc<ColorPickerWords>,
}

impl Pass<'_> {
    /// `logical` (from the start edge and top of the picker's room) as a physical rect.
    pub(super) fn at(&self, logical: Rect) -> Rect {
        self.flow.to_physical(logical.translate(self.origin))
    }

    /// The models whose channels the picker's parts are named after: the rows' model and the area's.
    pub(super) fn visible_in_compact(&self, model: Option<&dyn ColorModel>) -> Vec<&dyn ColorModel> {
        model.into_iter().chain([self.mode.model()]).collect()
    }
}

impl ColorPicker<'_> {
    pub(super) fn paint_compact(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>, pass: &Pass<'_>) {
        let sizes = &cx.style.sizes;
        let model = pass.models.iter().copied().find(|m| Some(m.id()) == pass.model);
        let input = CompactInput {
            channels: model.map_or(0, |m| m.channels().len()),
            switch: pass.models.len() > 1,
            sample: self.eyedropper,
            alpha: self.alpha,
        };
        let plan = layout::compact(sizes, pass.width, input);
        let visible = pass.visible_in_compact(model);
        self.area_and_strip(ui, cx, pass, &visible, pass.at(plan.area), pass.at(plan.strip));
        if let Some(model) = model {
            for (i, rect) in plan.channels.iter().enumerate() {
                self.channel_row(ui, cx, pass, model, i, &visible, pass.at(*rect));
            }
            if let Some(rect) = plan.switch {
                self.model_switch(ui, pass, model, pass.at(rect));
            }
        }
        self.hex_field(ui, cx, pass, pass.at(plan.hex));
        if let Some(rect) = plan.sample {
            self.sample_button(ui, pass, pass.at(rect));
        }
        if let Some(rect) = plan.alpha {
            self.alpha_row(ui, cx, pass, pass.at(rect));
        }
    }

    /// The area `area` large and the strip at its side.
    pub(super) fn area_and_strip(
        &mut self,
        ui: &mut PaintUi<'_>,
        cx: &Cx<'_, Self>,
        pass: &Pass<'_>,
        visible: &[&dyn ColorModel],
        area: Rect,
        strip: Rect,
    ) {
        let mode = pass.mode;
        let model = mode.model();
        let [across, down] = mode.area();
        let values = pass.shown.values_in(model);
        let rules = &pass.rules;
        let quantize = self.quantize();
        let mut area_child = ColorArea::new(Label::from(mode.name()), mode, Bind::given(values))
            .names([channel_name(model, across, visible), channel_name(model, down, visible)]);
        let mut strip_child = ColorStrip::new(channel_name(model, mode.strip(), visible), mode, Bind::given(values));
        if let Some(levels) = quantize {
            area_child = area_child.quantize(levels);
            strip_child = strip_child.quantize(levels);
        }
        let edit = ui.child("area", area, area_child);
        cx.adopt(&edit, &pass.shown, |v, mine| *mine = mine.with_values(v, model, rules));
        let edit = ui.child("strip", strip, strip_child);
        cx.adopt(&edit, &pass.shown, |v, mine| *mine = mine.with_values(v, model, rules));
    }

    /// Channel `i` of `model` as a line.
    pub(super) fn channel_row(
        &mut self,
        ui: &mut PaintUi<'_>,
        cx: &Cx<'_, Self>,
        pass: &Pass<'_>,
        model: &dyn ColorModel,
        i: usize,
        visible: &[&dyn ColorModel],
        rect: Rect,
    ) {
        let channel = model.channels()[i];
        let rule = ChannelRule::of(&channel);
        let values = pass.shown.values_in(model);
        let name = channel_name(model, i, visible);
        let track = self.dynamic_tracks.then(|| (model.track(i, &values), false));
        let slider = channel_slider(name.clone(), Bind::given(f64::from(values.values[i])), &rule, ChannelUnit::of(channel.unit), track);
        let rules = &pass.rules;
        let edit = ui.labelled(("channel", model.id(), i), rect, name, SideLabel::new().scrub(true), slider);
        cx.adopt(&edit, &pass.shown, |v, mine| *mine = mine.with_channel(model, i, *v, rules));
    }

    /// The model switch: a select of the models.
    fn model_switch(&mut self, ui: &mut PaintUi<'_>, pass: &Pass<'_>, model: &dyn ColorModel, rect: Rect) {
        let entries: Vec<craft_ui::contract::Entry<&'static str>> =
            pass.models.iter().map(|m| craft_ui::contract::Entry::new(m.id(), Label::from(m.name()))).collect();
        let label = pass.words.color_model.fill(&[("model", model.name())]);
        let out: EditResponse<&'static str> = ui.child("model", rect, Select::new(label, Bind::given(model.id()), &entries));
        if let Some(picked) = out.event.as_ref().and_then(|event| event.value()) {
            self.events.model_pick = Some(*picked);
        }
    }

    /// The hex field.
    pub(super) fn hex_field(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>, pass: &Pass<'_>, rect: Rect) {
        let rules = &pass.rules;
        let hex = TextField::hex(pass.words.hex.as_str().to_owned(), Bind::given(pass.shown.color.to_rgba()));
        let edit = ui.child("hex", rect, hex);
        cx.adopt(&edit, &pass.shown, |v, mine| *mine = mine.with_hex(*v, rules));
    }

    /// The "Opacity" row, 0 to 100 % over the checkerboard.
    pub(super) fn alpha_row(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>, pass: &Pass<'_>, rect: Rect) {
        let name = Label::from(pass.words.opacity.as_str().to_owned());
        let opaque = pass.shown.color.with_alpha(1.0);
        let track = Some((vec![opaque.with_alpha(0.0), opaque], true));
        let slider = channel_slider(name.clone(), Bind::given(pass.shown.alpha_percent()), &ChannelRule::ALPHA, ChannelUnit::Percent, track);
        let rules = &pass.rules;
        let edit = ui.labelled("alpha", rect, name, SideLabel::new().scrub(true), slider);
        cx.adopt(&edit, &pass.shown, |v, mine| *mine = mine.with_alpha(*v, rules));
    }

    /// The screen-pick button; its press is the output's `screen_pick`.
    pub(super) fn sample_button(&mut self, ui: &mut PaintUi<'_>, pass: &Pass<'_>, rect: Rect) {
        let label = pass.words.sample_screen.as_str().to_owned();
        let button = IconButton::new(IconSpec::svg(IconId::new("eyedropper"))).label(label.clone()).tooltip(label).enabled(!self.eyedropper_busy);
        let response: Response = ui.child("sample", rect, button);
        let at = response.interact_pointer_pos().unwrap_or(response.rect.center());
        self.events.screen_pick = if response.drag_stopped() {
            Some(ScreenPickGesture::Released { at })
        } else if response.dragged() {
            Some(ScreenPickGesture::Held { at })
        } else if response.clicked() {
            Some(ScreenPickGesture::Clicked)
        } else {
            None
        };
    }
}
