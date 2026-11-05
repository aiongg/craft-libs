//! [`FillPanel`]: the Content-Aware Fill panel (task WS-CA2).

use egui::text::LayoutJob;
use egui::{AsIdSalt, Color32, IdSalt, Label, Modifiers, Response, Ui};

use craft_theme::{ControlSize, Srgba, TypeRole};
use craft_ui::composites::{SectionHeader, SliderRow};
use craft_ui::controls::{Checkbox, IconButton, Segmented, Select, Swatch};
use craft_ui::dialog::full_window::FullWindowStyle;
use craft_ui::dialog::{ColorRequest, DialogLabels};
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, LocaleNumbers, Unit, UnitKind, Units};
use craft_ui::primitives::{Content, Gap, Row, Stack, Text};
use craft_ui::theme::{Tokens, TokensOf as _};

use super::session::{ColorAdaptation, FillOutputTo, FillReset, FillSettings, RotationAdaptation, SamplingMode};
use super::{FillLabels, OVERLAY_COLOR_KEY};

/// What [`FillPanel::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FillPanelOutput {
    /// The panel's response.
    pub response: Response,
    /// The last edit of a setting this pass, with the settings as they stand after it (written to
    /// the settings already): the tint and the preview follow. A slider's drag is
    /// `Begin`, `Update` and `Commit`; a check box or a select is one `Set`.
    pub edit: Option<EditEvent<FillSettings>>,
    /// A sampling option picked (the session changes: `FillSession::set_sampling`).
    pub sampling: Option<SamplingMode>,
    /// A section's Reset pressed.
    pub reset: Option<FillReset>,
    /// The Color swatch pressed: the app opens its color picker (key `fill.overlayColor`) and
    /// answers with `ContentAwareFillWorkspace::set_color`.
    pub pick_color: Option<ColorRequest>,
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
    labels: Option<FillLabels>,
}

impl<'a> FillPanel<'a> {
    /// The panel over `settings`, with the session's `sampling` option shown as picked.
    pub fn new(id_salt: impl AsIdSalt, settings: &'a mut FillSettings, sampling: SamplingMode) -> Self {
        Self { id_salt: IdSalt::new(id_salt), settings, sampling, error: None, labels: None }
    }

    /// The panel's words; [`FillLabels::of`] when not set.
    pub fn labels(mut self, labels: FillLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Sets an error message to show under the panel; `None` shows none.
    pub fn error(mut self, error: Option<&'a str>) -> Self {
        self.error = error;
        self
    }

    /// Draws the panel and returns its response and any setting changes.
    pub fn show(self, ui: &mut Ui) -> FillPanelOutput {
        // pc: crates/ui-egui/src/dialogs/caf_ui/view.rs:118 `caf_props_panel`.
        let Self { id_salt, settings, sampling, error, labels } = self;
        let labels = labels.unwrap_or_else(|| FillLabels::of(ui));
        let labels = &labels;
        let id = ui.make_persistent_id(id_salt);
        let numbers = LocaleNumbers::of(ui);
        let percent = Units::of(ui).unit(UnitKind::Percent);
        let reset_label = DialogLabels::of(ui).reset;
        let mut pass = Pass::default();
        let shown = Stack::new(Gap::Row).show(ui, |ui| {
            // The overlay section starts closed; the state is kept in egui's memory.
            let open_key = id.with("overlay-open");
            let mut open = ui.data(|d| d.get_temp::<bool>(open_key)).unwrap_or(false);
            SectionHeader::new((id, "overlay"), labels.sampling_overlay.as_str()).open(&mut open).show(ui, |ui| {
                if check(ui, (id, "show"), &labels.show_sampling_area, &mut settings.show_overlay) {
                    pass.set(settings);
                }
                opacity_slider(ui, (id, "opacity"), &labels.opacity, settings, &numbers, percent, &mut pass);
                caption_row(ui, &labels.color, |ui| {
                    let [r, g, b] = settings.color;
                    if ui.add(Swatch::new((id, "color"), labels.color.as_str(), Color32::from_rgb(r, g, b))).clicked() {
                        let request = ColorRequest::new(OVERLAY_COLOR_KEY, Srgba::from(Color32::from_rgb(r, g, b)));
                        pass.pick_color = Some(request.title(labels.color.as_str()));
                    }
                });
                let kinds = [(false, &labels.sampling_area), (true, &labels.excluded_area)];
                if let Some(excluded) =
                    caption_row(ui, &labels.indicates, |ui| choose(ui, (id, "indicates"), &labels.indicates, settings.excluded, &kinds))
                {
                    settings.excluded = excluded;
                    pass.set(settings);
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
                    if check(ui, (id, "sample-all"), &labels.sample_all_layers, &mut settings.sample_all_layers) {
                        pass.set(settings);
                    }
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
                        pass.set(settings);
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
                        pass.set(settings);
                    }
                    Row::new(Gap::Inline).show(ui, |ui| {
                        if check(ui, (id, "scale"), &labels.scale, &mut settings.scale) {
                            pass.set(settings);
                        }
                        if check(ui, (id, "mirror"), &labels.mirror, &mut settings.mirror) {
                            pass.set(settings);
                        }
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
                    pass.set(settings);
                }
            });

            if let Some(error) = error {
                let tokens = Tokens::of(ui);
                let mut format = Text::format(&tokens, TypeRole::Caption, Content::Normal);
                format.color = FullWindowStyle::of(&tokens).error;
                ui.add(Label::new(LayoutJob::single_section(error.to_owned(), format)).wrap());
            }
        });
        FillPanelOutput { response: shown.response, edit: pass.edit, sampling: pass.sampling, reset: pass.reset, pick_color: pass.pick_color }
    }
}

/// What the controls of one pass add up to.
#[derive(Default)]
struct Pass {
    edit: Option<EditEvent<FillSettings>>,
    sampling: Option<SamplingMode>,
    reset: Option<FillReset>,
    pick_color: Option<ColorRequest>,
}

impl Pass {
    /// A one-shot change of `settings` (already written).
    fn set(&mut self, settings: &FillSettings) {
        self.edit = Some(EditEvent::set(settings.clone(), Modifiers::NONE));
    }
}

/// The Opacity slider row (0..=100, no decimals) over `settings.opacity`, written as the user
/// edits it.
fn opacity_slider(
    ui: &mut Ui,
    id_salt: impl AsIdSalt,
    label: &str,
    settings: &mut FillSettings,
    numbers: &LocaleNumbers,
    unit: Unit,
    pass: &mut Pass,
) {
    let shown = SliderRow::new(id_salt, label, settings.opacity, 0.0..=100.0, numbers).decimals(0).step(1.0).unit(unit).show(ui);
    for event in shown.edits() {
        if let Some(next) = event.value().copied()
            && next != settings.opacity
        {
            settings.opacity = next;
        }
        pass.edit = Some(event.map(|_| settings.clone()));
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
