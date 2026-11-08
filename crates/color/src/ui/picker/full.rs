//! The Full layout of `ColorPicker` (the Color Picker dialog), and the warnings beside its compare swatch.

use craft_behavior::{IconId, Metrics, Sense};
use craft_theme::{ColorModel, Srgba};
use craft_ui::choice::{Checkbox, Radio, Swatch};
use craft_ui::contract::{
    Bind, Common, Component, Cx, Edit as _, FieldLayout, MeasureCx, Outcome, PaintUi, Props as _, Role, SideLabel, WordSetCx as _,
};
use craft_ui::display::{Icon, Shade, Text};
use craft_ui::primitives::IconSpec;
use craft_ui::runtime::env::Label;
use craft_ui::style::{ComponentStyle, StyleCx, TypeRole};
use craft_ui::text::NumberField;
use egui::{Rect, Response, Vec2};

use super::ColorPicker;
use super::channel::{ChannelRule, ChannelUnit, channel_name};
use super::compact::Pass;
use super::layout::{self, FullInput};
use crate::ui::AreaMode;

/// A warning the host shows beside the Full picker's compare swatch, as the out-of-gamut and web-colour warnings of
/// Photoshop's Color Picker: an icon over a chip of the nearest colour that passes. A click on it sets that colour;
/// AccessKit and the tooltip name it by its tip.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorWarning {
    tip: Label,
    nearest: Srgba,
    icon: IconId,
}

impl ColorWarning {
    /// A warning that says `tip` ("Out of gamut for printing") and offers `nearest`.
    pub fn new(tip: impl Into<Label>, nearest: Srgba) -> Self {
        ColorWarning { tip: tip.into(), nearest, icon: IconId::new("warning") }
    }

    /// A warning that the colour is not a web-safe colour (the cube Photoshop's Color Picker shows), offering
    /// `nearest`, the nearest web colour.
    pub fn web_colors(tip: impl Into<Label>, nearest: Srgba) -> Self {
        Self::new(tip, nearest).icon(IconId::new("web_colors"))
    }

    /// The colour that passes, nearest the warned one: what a click on the warning sets.
    pub fn nearest(&self) -> Srgba {
        self.nearest
    }

    /// The icon over the chip.
    #[must_use]
    pub fn icon(mut self, icon: IconId) -> Self {
        self.icon = icon;
        self
    }
}

/// The style of a warning chip: the icon over the chip, and their sizes.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct ColorWarningChipStyle {
    #[style(metrics)]
    metrics: Metrics,
    icon: f32,
    chip: f32,
    gap: f32,
}

/// What a [`ColorWarningChip`] reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct ColorWarningChipOutput {
    /// The chip's response.
    pub response: Response,
    /// The chip was activated by click, Enter or Space: the host applies [`ColorWarning::nearest`].
    pub clicked: bool,
}

impl Outcome for ColorWarningChipOutput {
    type Value = craft_ui::contract::NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// A [`ColorWarning`] as the Full picker draws it, for a host that draws it beside its own swatch: the warning's icon
/// over a swatch of the nearest passing colour. The host applies [`ColorWarning::nearest`] when it is clicked. The tip
/// is the tooltip and the accessible name.
#[must_use = "show the chip with `show`"]
#[derive(craft_ui::contract::Component, craft_ui::contract::Props)]
pub struct ColorWarningChip<'a> {
    #[common]
    common: Common,
    warning: &'a ColorWarning,
    clicked: bool,
}

impl<'a> ColorWarningChip<'a> {
    /// A chip for `warning`.
    pub fn new(warning: &'a ColorWarning) -> Self {
        ColorWarningChip { common: Common::new(), warning, clicked: false }.label(warning.tip.clone())
    }
}

impl Component for ColorWarningChip<'_> {
    const ROLE: Role = Role::GROUP;
    const PARTS: bool = false;
    const STOPS: bool = false;
    const OVERLAYS: bool = false;
    type Style = ColorWarningChipStyle;
    type Output = ColorWarningChipOutput;

    fn style(cx: &StyleCx<'_>, _: ColorWarningChipStyleKey) -> ColorWarningChipStyle {
        ColorWarningChipStyle { metrics: Metrics::new(), icon: cx.tokens.size.icon.md, chip: cx.control().height, gap: cx.tokens.space.gap.tight }
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(cx.style.icon.max(cx.style.chip), cx.style.icon + cx.style.gap + cx.style.chip)
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let style = cx.style;
        let icon = Rect::from_center_size(egui::pos2(cx.rect.center().x, cx.rect.top() + style.icon / 2.0), Vec2::splat(style.icon));
        let chip = Rect::from_center_size(egui::pos2(cx.rect.center().x, cx.rect.bottom() - style.chip / 2.0), Vec2::splat(style.chip));
        let _ = ui.child("icon", icon, Icon::new(IconSpec::svg(self.warning.icon.clone())));
        let swatch = Swatch::new(self.warning.tip.clone(), self.warning.nearest).tooltip(self.warning.tip.clone());
        self.clicked = ui.child("chip", chip, swatch).clicked;
    }

    fn finish(self, cx: Cx<'_, Self>) -> ColorWarningChipOutput {
        ColorWarningChipOutput { response: cx.response.clone(), clicked: self.clicked }
    }
}

impl ColorPicker<'_> {
    pub(super) fn paint_full(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>, pass: &Pass<'_>) {
        let sizes = &cx.style.sizes;
        let input = FullInput {
            models: &pass.models,
            web: self.web_colors.is_some(),
            alpha: self.alpha,
            sample: self.eyedropper,
            compare: self.compare.is_some(),
            warnings: self.warnings.len(),
        };
        let plan = layout::full(sizes, input);
        let visible: Vec<&dyn ColorModel> = pass.models.iter().copied().chain([pass.mode.model()]).collect();
        self.area_and_strip(ui, cx, pass, &visible, pass.at(plan.area), pass.at(plan.strip));
        if let (Some(rect), Some(web)) = (plan.web, self.web_colors.as_mut()) {
            let side = SideLabel::new().layout(FieldLayout::After);
            let label = Label::from(pass.words.only_web_colors.as_str().to_owned());
            let out = ui.labelled("web", pass.at(rect), label, side, Checkbox::new(web.reborrowed()));
            self.events.web_on |= out.event.as_ref().and_then(|event| event.value()).copied().unwrap_or(false);
        }
        if let Some(rect) = plan.alpha {
            self.alpha_row(ui, cx, pass, pass.at(rect));
        }
        if let Some(rect) = plan.sample {
            self.sample_button(ui, pass, pass.at(rect));
        }
        if let (Some(current), Some(captions), Some(halves)) = (self.compare, plan.captions, plan.halves) {
            self.compare_swatch(ui, pass, current, captions, halves);
        }
        for (i, rect) in plan.warnings.iter().enumerate() {
            let chip = ui.child(("warning", i), pass.at(*rect), ColorWarningChip::new(&self.warnings[i]));
            if chip.clicked {
                self.events.clicks.push(self.warnings[i].nearest);
            }
        }
        let offered = Self::offered(&pass.models);
        for cell in &plan.cells {
            let model = pass.models[cell.model];
            let channel = model.channels()[cell.channel];
            let name = channel_name(model, cell.channel, &visible);
            let short = pass.words.grid_label.fill(&[("label", channel.label)]);
            let mode = offered.iter().find(|mode| mode.model().id() == model.id() && mode.strip() == cell.channel).copied();
            self.grid_label(ui, pass, cell.channel, model.id(), &short, name.clone(), mode, pass.at(cell.label));
            let rule = ChannelRule::of(&channel);
            let value = f64::from(pass.shown.values_in(model).values[cell.channel]);
            let field = NumberField::new(name, Bind::given(Some(value)))
                .range(rule.range.clone())
                .step(rule.step)
                .snap_typed(true)
                .commit_decimals(usize::from(rule.decimals));
            let field = match ChannelUnit::of(channel.unit) {
                ChannelUnit::None => field,
                ChannelUnit::Percent => field.unit_kind(craft_ui::runtime::env::UnitKind::Percent),
                ChannelUnit::Degrees => field.unit_kind(craft_ui::runtime::env::UnitKind::Degrees),
                ChannelUnit::Text(text) => field.unit(Label::from(text)),
            };
            let rules = &pass.rules;
            let edit = ui.child(("grid", model.id(), cell.channel), pass.at(cell.field), field);
            cx.adopt(&edit, &pass.shown, |v, mine| {
                if let Some(v) = v {
                    *mine = mine.with_channel(model, cell.channel, *v, rules);
                }
            });
        }
        let (hash, hex) = plan.hex;
        let _ = ui.child("hash", pass.at(hash), Text::new("#").role(TypeRole::Mono).shade(Shade::Muted));
        self.hex_field(ui, cx, pass, pass.at(hex));
    }

    /// A grid row's label: a radio with the short label when the channel chooses an area mode, the plain label
    /// otherwise.
    #[allow(clippy::too_many_arguments)]
    fn grid_label(
        &mut self,
        ui: &mut PaintUi<'_>,
        _pass: &Pass<'_>,
        channel: usize,
        model: &'static str,
        short: &str,
        name: Label,
        mode: Option<AreaMode>,
        rect: Rect,
    ) {
        match mode {
            Some(mode) => {
                let side = SideLabel::new().layout(FieldLayout::After);
                let _ = ui.labelled(("radio", model, channel), rect, Label::from(short.to_owned()), side, Radio::new(mode).label(name));
            }
            None => {
                let _ = ui.child(("label", model, channel), rect, Text::new(short.to_owned()).role(TypeRole::Body));
            }
        }
    }

    /// The compare swatch: the colour being edited over the current one, with their captions; a click on the current
    /// half sets it.
    fn compare_swatch(&mut self, ui: &mut PaintUi<'_>, pass: &Pass<'_>, current: Srgba, captions: [Rect; 2], halves: [Rect; 2]) {
        let words = &pass.words;
        let caption = |text: &str| Text::new(text.to_owned()).role(TypeRole::Caption).shade(Shade::Muted);
        let _ = ui.child("new-caption", pass.at(captions[0]), caption(words.new_caption.as_str()));
        let _ = ui.child("new", pass.at(halves[0]), Swatch::new(words.new_color.as_str().to_owned(), pass.shown.color).fill());
        let restore = words.restore.as_str().to_owned();
        let swatch = Swatch::new(restore.clone(), current).tooltip(restore).fill();
        if ui.child("current", pass.at(halves[1]), swatch).clicked {
            self.events.clicks.push(current);
        }
        let _ = ui.child("current-caption", pass.at(captions[1]), caption(words.current_caption.as_str()));
    }
}
