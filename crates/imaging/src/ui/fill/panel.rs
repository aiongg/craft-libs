//! [`FillPanel`]: the Content-Aware Fill panel (task WS-CA2).

use std::ops::RangeInclusive;

use egui::text::LayoutJob;
use egui::{AsIdSalt, Color32, IdSalt, Label, Response, Ui};

use craft_theme::{ControlSize, TypeRole};
use craft_ui::composites::{SectionHeader, SliderRow};
use craft_ui::controls::{Checkbox, IconButton, Segmented, Select, Swatch};
use craft_ui::dialog::DialogLabels;
use craft_ui::dialog::full_window::FullWindowStyle;
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, LocaleNumbers, Unit};
use craft_ui::primitives::{Content, Gap, Row, Stack, Text};
use craft_ui::theme::{Tokens, TokensOf as _};

use super::FillLabels;
use super::session::{ColorAdaptation, FillOutputTo, FillReset, FillSettings, RotationAdaptation, SamplingMode};

/// What [`FillPanel::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FillPanelOutput {
    pub response: Response,
    /// A setting changed (written to the settings): the tint and the preview follow.
    pub changed: bool,
    pub dragging: bool,
    /// A sampling option picked (the session changes: `FillSession::set_sampling`).
    pub sampling: Option<SamplingMode>,
    /// A section's Reset pressed.
    pub reset: Option<FillReset>,
    /// The Color swatch pressed: the workspace asks the app for a colour (`ColorRequest`, key
    /// `fill.overlayColor`).
    pub pick_color: bool,
}

/// The settings column headed "Content-Aware Fill" (lead call 10), top to bottom:
///
/// - **Sampling Area Overlay** (collapsed at first): Show Sampling Area, Opacity (0..=100 %),
///   Color (a swatch), Indicates (Sampling Area, Excluded Area).
/// - **Sampling Area Options** with a Reset icon button at the header's trailing end: Auto,
///   Rectangular, Custom (a segmented choice), Sample All Layers.
/// - **Fill Settings** with its Reset: Color Adaptation (None, Default, High, Very High), Rotation
///   Adaptation (None, Low, Medium, High, Full), Scale, Mirror.
/// - **Output Settings**: Output To (Current Layer, New Layer, Duplicate Layer).
/// - The error line (a limit, a failed Apply or preview) in `status.bad.fg`, caption role.
///
/// The footer (Reset at the leading end; OK, Apply, Cancel in the platform's order, lead call 12)
/// is the side column's, drawn by the workspace.
pub struct FillPanel<'a> {
    id_salt: IdSalt,
    settings: &'a mut FillSettings,
    sampling: SamplingMode,
    error: Option<&'a str>,
    labels: &'a FillLabels,
}

impl<'a> FillPanel<'a> {
    /// The panel over `settings`, with the session's `sampling` option shown as picked.
    pub fn new(id_salt: impl AsIdSalt, settings: &'a mut FillSettings, sampling: SamplingMode, labels: &'a FillLabels) -> Self {
        Self { id_salt: IdSalt::new(id_salt), settings, sampling, error: None, labels }
    }

    pub fn error(mut self, error: Option<&'a str>) -> Self {
        self.error = error;
        self
    }

    pub fn show(self, ui: &mut Ui) -> FillPanelOutput {
        // pc: crates/ui-egui/src/dialogs/caf_ui/view.rs:118 `caf_props_panel`.
        let Self { id_salt, settings, sampling, error, labels } = self;
        let id = ui.make_persistent_id(id_salt);
        let numbers = LocaleNumbers::of(ui);
        let reset_label = DialogLabels::of(ui).reset;
        let mut pass = Pass::default();
        let shown = Stack::new(Gap::Row).show(ui, |ui| {
            // The overlay section starts closed; the state is kept in egui's memory.
            let open_key = id.with("overlay-open");
            let mut open = ui.data(|d| d.get_temp::<bool>(open_key)).unwrap_or(false);
            SectionHeader::new((id, "overlay"), labels.sampling_overlay.as_str()).open(&mut open).show(ui, |ui| {
                pass.changed |= check(ui, (id, "show"), &labels.show_sampling_area, &mut settings.show_overlay);
                slider(ui, (id, "opacity"), &labels.opacity, &mut settings.opacity, 0.0..=100.0, &numbers, &mut pass);
                caption_row(ui, &labels.color, |ui| {
                    let [r, g, b] = settings.color;
                    if ui.add(Swatch::new((id, "color"), labels.color.as_str(), Color32::from_rgb(r, g, b))).clicked() {
                        pass.pick_color = true;
                    }
                });
                let kinds = [(false, &labels.sampling_area), (true, &labels.excluded_area)];
                if let Some(excluded) =
                    caption_row(ui, &labels.indicates, |ui| choose(ui, (id, "indicates"), &labels.indicates, settings.excluded, &kinds))
                {
                    settings.excluded = excluded;
                    pass.changed = true;
                }
            });
            ui.data_mut(|d| d.insert_temp(open_key, open));

            let mut reset_sampling = false;
            SectionHeader::new((id, "sampling"), labels.sampling_options.as_str())
                .controls(|ui| {
                    reset_sampling =
                        ui.add(IconButton::new(Icon::Undo, reset_label.as_str()).size(ControlSize::Sm).id_salt((id, "reset-sampling"))).clicked();
                })
                .show(ui, |ui| {
                    let mut picked = sampling;
                    let modes = [
                        (SamplingMode::Auto, &labels.auto),
                        (SamplingMode::Rectangular, &labels.rectangular),
                        (SamplingMode::Custom, &labels.custom),
                    ];
                    let mut group = Segmented::new((id, "sampling-mode"), labels.sampling_options.as_str(), &mut picked);
                    for (mode, text) in modes {
                        group = group.item(mode, text.as_str());
                    }
                    group.show(ui);
                    if picked != sampling {
                        pass.sampling = Some(picked);
                    }
                    pass.changed |= check(ui, (id, "sample-all"), &labels.sample_all_layers, &mut settings.sample_all_layers);
                });

            if reset_sampling {
                pass.reset = Some(FillReset::Sampling);
            }
            let mut reset_fill = false;
            SectionHeader::new((id, "fill"), labels.fill_settings.as_str())
                .controls(|ui| {
                    reset_fill =
                        ui.add(IconButton::new(Icon::Undo, reset_label.as_str()).size(ControlSize::Sm).id_salt((id, "reset-fill"))).clicked();
                })
                .show(ui, |ui| {
                    let colors = [
                        (ColorAdaptation::None, &labels.none),
                        (ColorAdaptation::Default, &labels.default),
                        (ColorAdaptation::High, &labels.high),
                        (ColorAdaptation::VeryHigh, &labels.very_high),
                    ];
                    if let Some(next) = caption_row(ui, &labels.color_adaptation, |ui| {
                        choose(ui, (id, "color-adaptation"), &labels.color_adaptation, settings.color_adaptation, &colors)
                    }) {
                        settings.color_adaptation = next;
                        pass.changed = true;
                    }
                    let rotations = [
                        (RotationAdaptation::None, &labels.none),
                        (RotationAdaptation::Low, &labels.low),
                        (RotationAdaptation::Medium, &labels.medium),
                        (RotationAdaptation::High, &labels.high),
                        (RotationAdaptation::Full, &labels.full),
                    ];
                    if let Some(next) = caption_row(ui, &labels.rotation_adaptation, |ui| {
                        choose(ui, (id, "rotation"), &labels.rotation_adaptation, settings.rotation, &rotations)
                    }) {
                        settings.rotation = next;
                        pass.changed = true;
                    }
                    Row::new(Gap::Inline).show(ui, |ui| {
                        pass.changed |= check(ui, (id, "scale"), &labels.scale, &mut settings.scale);
                        pass.changed |= check(ui, (id, "mirror"), &labels.mirror, &mut settings.mirror);
                    });
                });

            if reset_fill {
                pass.reset = Some(FillReset::Fill);
            }
            SectionHeader::new((id, "output"), labels.output_settings.as_str()).show(ui, |ui| {
                let outputs = [
                    (FillOutputTo::CurrentLayer, &labels.current_layer),
                    (FillOutputTo::NewLayer, &labels.new_layer),
                    (FillOutputTo::DuplicateLayer, &labels.duplicate_layer),
                ];
                if let Some(next) =
                    caption_row(ui, &labels.output_to, |ui| choose(ui, (id, "output-to"), &labels.output_to, settings.output, &outputs))
                {
                    settings.output = next;
                    pass.changed = true;
                }
            });

            if let Some(error) = error {
                let tokens = Tokens::of(ui);
                let mut format = Text::format(&tokens, TypeRole::Caption, Content::Normal);
                format.color = FullWindowStyle::of(&tokens).error;
                ui.add(Label::new(LayoutJob::single_section(error.to_owned(), format)).wrap());
            }
        });
        FillPanelOutput {
            response: shown.response,
            changed: pass.changed,
            dragging: pass.dragging,
            sampling: pass.sampling,
            reset: pass.reset,
            pick_color: pass.pick_color,
        }
    }
}

/// What the controls of one pass add up to.
#[derive(Default)]
struct Pass {
    changed: bool,
    dragging: bool,
    sampling: Option<SamplingMode>,
    reset: Option<FillReset>,
    pick_color: bool,
}

const PERCENT: &str = "{n} %";

/// A 0 decimal slider row over `value`, written as the user edits it.
fn slider(ui: &mut Ui, id_salt: impl AsIdSalt, label: &str, value: &mut f64, range: RangeInclusive<f64>, numbers: &LocaleNumbers, pass: &mut Pass) {
    let shown = SliderRow::new(id_salt, label, *value, range, numbers).decimals(0).step(1.0).unit(Unit::new(PERCENT)).show(ui);
    pass.dragging |= shown.slider.response.dragged() || shown.field.response.dragged();
    for event in shown.edits() {
        pass.dragging |= matches!(event, EditEvent::Begin { .. } | EditEvent::Update { .. });
        if let Some(next) = event.value().copied()
            && next != *value
        {
            *value = next;
            pass.changed = true;
        }
    }
}

/// A check box; whether the user toggled it.
fn check(ui: &mut Ui, id_salt: impl AsIdSalt, label: &str, value: &mut bool) -> bool {
    Checkbox::bool(label, value).id_salt(id_salt).show(ui).response.changed()
}

/// `label` muted before the control `add` draws.
fn caption_row<R>(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    Row::new(Gap::Inline)
        .show(ui, |ui| {
            ui.add(Text::new(label).role(TypeRole::Label).content(Content::Muted));
            add(ui)
        })
        .inner
}

/// A select over `options`; the value the user picked this pass.
fn choose<T: Clone + PartialEq + Send + Sync + 'static>(
    ui: &mut Ui,
    id_salt: impl AsIdSalt,
    label: &str,
    current: T,
    options: &[(T, &String)],
) -> Option<T> {
    let mut value = current;
    let mut select = Select::new(id_salt, label, &mut value);
    for (option, text) in options {
        select = select.option(option.clone(), text.as_str());
    }
    select.show(ui).event.and_then(|event| event.value().cloned())
}
