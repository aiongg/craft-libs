//! [`SelectMaskPanel`]: Select and Mask's Properties panel (task WS-SM3).

use std::ops::RangeInclusive;

use craft_theme::TypeRole;
use craft_ui::composites::{SectionHeader, SliderRow};
use craft_ui::controls::{Button, Checkbox, Select, Swatch, ToggleButton, TooltipExt as _};
use craft_ui::dialog::ColorRequest;
use craft_ui::dialog::full_window::FullWindowStyle;
use craft_ui::direction;
use craft_ui::edit::EditEvent;
use craft_ui::localized::{Labels as _, LocaleNumbers, Unit};
use craft_ui::primitives::{Content, Text};
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::text::LayoutJob;
use egui::{AsIdSalt, IdSalt, Label, Response, Ui};

use crate::selection::ColorIndicates;
use crate::ui::mask_view::{MaskViewControls, MaskViewLabels, MaskViewState};

use super::SelectMaskLabels;
use super::edits::SelectMaskEdit;
use super::session::{OutputTo, RefineSettings, SelectMaskSettings};

/// The [`ColorRequest::key`] of the Overlay colour swatch; the app answers with
/// `SelectMaskWorkspace::set_color`.
pub const OVERLAY_COLOR_KEY: &str = "selectMask.overlayColor";

/// What [`SelectMaskPanel::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SelectMaskPanelOutput {
    pub response: Response,
    /// The refinement values changed (a session change: the history settles it, a preview is
    /// asked for).
    pub refine_changed: bool,
    /// The view or a view option changed (no undo step; a preview is asked for).
    pub view_changed: bool,
    /// A slider is being dragged.
    pub dragging: bool,
    /// Clear Selection or Invert pressed.
    pub edit: Option<SelectMaskEdit>,
    /// The Overlay colour swatch was pressed: the app opens its colour picker with this request
    /// (keyed [`OVERLAY_COLOR_KEY`]) and answers with `SelectMaskWorkspace::set_color`. Once.
    pub pick_color: Option<ColorRequest>,
}

/// The Properties panel in the Wide side column with the header "Properties", top to bottom:
///
/// - Remember Settings.
/// - **View Mode**: [`MaskViewControls`](crate::ui::mask_view::MaskViewControls) (the view, its
///   opacity, Show Edge, Show Original), then Real-Time Refinement and High Quality Preview; with
///   Overlay, Color (a swatch that asks the app through `ColorRequest`) and Indicates (Masked
///   Areas, Selected Areas).
/// - **Refine Mode**: Color Aware (on) and Object Aware, disabled with the tooltip "Object Aware
///   needs a segmentation model, which isn't available yet".
/// - **Edge Detection**: Radius (0..=250 px), Smart Radius.
/// - **Global Refinements**: Smooth (0..=100), Feather (0..=1000 px), Contrast (0..=100 %), Shift
///   Edge (−100..=100 %); buttons Clear Selection and Invert.
/// - **Output Settings**: Decontaminate Colors, Amount (0..=100 %, enabled with it), Output To
///   (the targets [`OutputTo::allowed`](super::OutputTo::allowed)).
/// - The error line (the limit message, or the preview's failure) in `status.bad.fg`, caption
///   role.
///
/// Each section is a collapsible `SectionHeader`. The footer (Reset at the leading end, OK and
/// Cancel) is the side column's, drawn by the workspace.
pub struct SelectMaskPanel<'a> {
    id_salt: IdSalt,
    settings: &'a mut SelectMaskSettings,
    refine: &'a mut RefineSettings,
    view: &'a mut MaskViewState,
    error: Option<&'a str>,
    labels: &'a SelectMaskLabels,
}

impl<'a> SelectMaskPanel<'a> {
    pub fn new(
        id_salt: impl AsIdSalt,
        settings: &'a mut SelectMaskSettings,
        refine: &'a mut RefineSettings,
        view: &'a mut MaskViewState,
        labels: &'a SelectMaskLabels,
    ) -> Self {
        Self { id_salt: IdSalt::new(id_salt), settings, refine, view, error: None, labels }
    }

    /// The error line under the panel.
    pub fn error(mut self, error: Option<&'a str>) -> Self {
        self.error = error;
        self
    }

    pub fn show(self, ui: &mut Ui) -> SelectMaskPanelOutput {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:1540 `properties`.
        let Self { id_salt, settings, refine, view, error, labels } = self;
        let id = ui.make_persistent_id(id_salt);
        let numbers = LocaleNumbers::of(ui);
        let view_labels = MaskViewLabels::of(ui);
        let refine_before = refine.clone();
        let drag_key = id.with("dragging");
        let mut dragging = ui.data(|d| d.get_temp::<bool>(drag_key)).unwrap_or(false);
        let (mut view_changed, mut edit, mut pick_color) = (false, None, None);
        let response = ui
            .scope(|ui| {
                let _ = Checkbox::bool(labels.remember.as_str(), &mut settings.remember).id_salt((id, "remember")).show(ui);

                SectionHeader::new((id, "view"), labels.view_mode.as_str()).show(ui, |ui| {
                    let controls = MaskViewControls::new((id, "view-controls"), view).labels(view_labels).show(ui);
                    view_changed |= controls.changed.is_some();
                    let _ = Checkbox::bool(labels.real_time.as_str(), &mut settings.real_time).id_salt((id, "real-time")).show(ui);
                    let high_quality = settings.high_quality;
                    let _ = Checkbox::bool(labels.high_quality.as_str(), &mut settings.high_quality).id_salt((id, "high-quality")).show(ui);
                    view_changed |= settings.high_quality != high_quality;
                    if view.mode == crate::ui::mask_view::MaskViewMode::Overlay {
                        direction::horizontal(ui, |ui| {
                            ui.add(Text::new(labels.overlay_color.as_str()).role(TypeRole::Label).content(Content::Muted));
                            let color = view.overlay_color;
                            if ui.add(Swatch::new((id, "color"), labels.overlay_color.as_str(), color)).clicked() {
                                pick_color = Some(ColorRequest::new(OVERLAY_COLOR_KEY, color).title(labels.overlay_color.as_str()));
                            }
                            ui.add(Text::new(labels.indicates.as_str()).role(TypeRole::Label).content(Content::Muted));
                            let mut indicates = view.overlay_indicates;
                            let _ = Select::new((id, "indicates"), labels.indicates.as_str(), &mut indicates)
                                .option(ColorIndicates::MaskedAreas, labels.masked_areas.as_str())
                                .option(ColorIndicates::SelectedAreas, labels.selected_areas.as_str())
                                .show(ui);
                            if indicates != view.overlay_indicates {
                                view.overlay_indicates = indicates;
                                view_changed = true;
                            }
                        });
                    }
                });

                SectionHeader::new((id, "refine-mode"), labels.refine_mode.as_str()).show(ui, |ui| {
                    direction::horizontal(ui, |ui| {
                        // Color Aware is the one mode there is: pressing it changes nothing.
                        let mut color_aware = true;
                        ui.add(ToggleButton::new(labels.color_aware.as_str(), &mut color_aware).id_salt((id, "color-aware")));
                        let mut object_aware = false;
                        ui.add(ToggleButton::new(labels.object_aware.as_str(), &mut object_aware).id_salt((id, "object-aware")).enabled(false))
                            .tooltip(labels.object_aware_missing.as_str());
                    });
                });

                SectionHeader::new((id, "edge"), labels.edge_detection.as_str()).show(ui, |ui| {
                    slider(ui, (id, "radius"), &labels.radius, &mut refine.radius, 0.0..=250.0, "{n} px", true, &numbers, &mut dragging);
                    let _ = Checkbox::bool(labels.smart_radius.as_str(), &mut refine.smart_radius).id_salt((id, "smart-radius")).show(ui);
                });

                SectionHeader::new((id, "global"), labels.global_refinements.as_str()).show(ui, |ui| {
                    slider(ui, (id, "smooth"), &labels.smooth, &mut refine.smooth, 0.0..=100.0, "{n}", true, &numbers, &mut dragging);
                    slider(ui, (id, "feather"), &labels.feather, &mut refine.feather, 0.0..=1000.0, "{n} px", true, &numbers, &mut dragging);
                    slider(ui, (id, "contrast"), &labels.contrast, &mut refine.contrast, 0.0..=100.0, "{n}%", true, &numbers, &mut dragging);
                    slider(ui, (id, "shift"), &labels.shift_edge, &mut refine.shift_edge, -100.0..=100.0, "{n}%", true, &numbers, &mut dragging);
                    direction::horizontal(ui, |ui| {
                        if ui.add(Button::new(labels.clear_selection.as_str()).id_salt((id, "clear"))).clicked() {
                            edit = Some(SelectMaskEdit::Clear);
                        }
                        if ui.add(Button::new(labels.invert.as_str()).id_salt((id, "invert"))).clicked() {
                            edit = Some(SelectMaskEdit::Invert);
                        }
                    });
                });

                SectionHeader::new((id, "output"), labels.output_settings.as_str()).show(ui, |ui| {
                    let _ = Checkbox::bool(labels.decontaminate.as_str(), &mut refine.decontaminate).id_salt((id, "decontaminate")).show(ui);
                    let decontaminate = refine.decontaminate;
                    slider(ui, (id, "amount"), &labels.amount, &mut refine.amount, 0.0..=100.0, "{n}%", decontaminate, &numbers, &mut dragging);
                    refine.follow_decontaminate();
                    direction::horizontal(ui, |ui| {
                        ui.add(Text::new(labels.output_to.as_str()).role(TypeRole::Label).content(Content::Muted));
                        let mut select = Select::new((id, "output-to"), labels.output_to.as_str(), &mut refine.output);
                        for target in OutputTo::ALL.into_iter().filter(|target| target.allowed(decontaminate)) {
                            select = select.option(target, output_label(labels, target));
                        }
                        let _ = select.show(ui);
                    });
                });

                if let Some(error) = error {
                    let tokens = Tokens::of(ui);
                    let mut format = Text::format(&tokens, TypeRole::Caption, Content::Normal);
                    format.color = FullWindowStyle::of(&tokens).error;
                    drop(tokens);
                    ui.add(Label::new(LayoutJob::single_section(error.to_owned(), format)).wrap());
                }
            })
            .response;
        ui.data_mut(|d| d.insert_temp(drag_key, dragging));
        SelectMaskPanelOutput { response, refine_changed: *refine != refine_before, view_changed, dragging, edit, pick_color }
    }
}

fn output_label(labels: &SelectMaskLabels, target: OutputTo) -> &str {
    match target {
        OutputTo::Selection => &labels.output_selection,
        OutputTo::LayerMask => &labels.output_layer_mask,
        OutputTo::NewLayer => &labels.output_new_layer,
        OutputTo::NewLayerWithMask => &labels.output_new_layer_with_mask,
        OutputTo::NewDocument => &labels.output_new_document,
        OutputTo::NewDocumentWithMask => &labels.output_new_document_with_mask,
    }
}

/// One slider row: the events set `value`; `dragging` follows the gesture (true from its first
/// event until its commit, set or cancel), so a drag that pauses is still one drag.
#[expect(clippy::too_many_arguments, reason = "a private helper over one panel's rows")]
fn slider(
    ui: &mut Ui,
    id_salt: impl AsIdSalt,
    label: &str,
    value: &mut f64,
    range: RangeInclusive<f64>,
    unit: &str,
    enabled: bool,
    numbers: &LocaleNumbers,
    dragging: &mut bool,
) {
    let row = SliderRow::new(id_salt, label, *value, range, numbers).unit(Unit::new(unit)).enabled(enabled).show(ui);
    for event in row.edits() {
        *dragging = matches!(event, EditEvent::Begin { .. } | EditEvent::Update { .. });
        if let Some(next) = event.value() {
            *value = *next;
        }
    }
}
