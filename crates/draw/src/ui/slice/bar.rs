//! [`SliceBar`]: the Slice tools' options bar cells (task SH-T9).

use std::sync::Arc;

use egui::{AsIdSalt, IdSalt, Ui};

use super::{SliceCommand, SliceLabels, SliceMode, SliceOrigin, SliceToolSettings};
use craft_ui::canvas::Constraint;
use craft_ui::canvas::marquee::fields::ConstraintFields;
use craft_ui::controls::{Button, Checkbox, ControlSize};
use craft_ui::edit::EditEvent;
use craft_ui::localized::SharedLabels as _;
use craft_ui::localized::units::LengthUnit;
use craft_ui::patterns::OptionsBar;

/// What [`SliceBar::show`] reports.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
#[must_use = "the app stores the style and runs the command"]
pub struct SliceBarOutput {
    /// The Style, Width or Height changed (`ConstraintFieldsOutput::event`): tool settings, no
    /// undo step. A style pick resets the numbers (Fixed Size 64 × 64, else 1 × 1).
    pub style: Option<EditEvent<Constraint>>,
    /// A unit picked from a Fixed Size field's menu.
    pub unit_picked: Option<LengthUnit>,
    /// A button: Slices From Guides (Slice); Promote, Divide…, Delete, Slice Options… and the Hide
    /// Auto Slices check box (Select).
    pub command: Option<SliceCommand>,
}

/// The cells a Slice tool adds to the options bar.
///
/// - Slice: the kernel's `ConstraintFields` (Style, Width, swap, Height) over
///   [`SliceToolSettings::style`], then "Slices From Guides".
/// - Select: Promote (enabled for a selected auto or layer slice), Divide…, Delete (enabled for a
///   selected user slice), "Hide Auto Slices" check box, "Slice Options…" (Divide… and Slice
///   Options… for any selected slice).
/// - [`locked`](Self::locked): every button is greyed (View › Lock Slices).
/// - Keyboard: each cell is a Tab stop in the bar's order.
pub struct SliceBar<'a> {
    id_salt: IdSalt,
    mode: SliceMode,
    settings: &'a SliceToolSettings,
    selected: Option<SliceOrigin>,
    locked: bool,
    labels: Option<SliceLabels>,
}

impl<'a> SliceBar<'a> {
    /// The bar of `mode` with `settings`.
    pub fn new(id_salt: impl AsIdSalt, mode: SliceMode, settings: &'a SliceToolSettings) -> Self {
        SliceBar { id_salt: IdSalt::new(id_salt), mode, settings, selected: None, locked: false, labels: None }
    }

    /// The selected slice's origin, or `None` without a selection: enables Promote, Divide…,
    /// Delete and Slice Options….
    pub fn selected(mut self, origin: Option<SliceOrigin>) -> Self {
        self.selected = origin;
        self
    }

    /// Whether the slices are locked: the buttons are greyed (PhotoCraft's `!locked`).
    pub fn locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: SliceLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Adds the cells to the bar's row `ui`.
    pub fn show(self, ui: &mut Ui) -> SliceBarOutput {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:404-471 `options_bar` (Style :414-417 with the
        // 64 × 64 / 1 × 1 reset, the swap button :433, Slices From Guides :440, Promote :447,
        // Divide… :450, Delete :453, Hide Auto Slices :460, Slice Options… :462).
        // new: the Style cells are `craft_ui::canvas::ConstraintFields` (K13).
        let Self { id_salt, mode, settings, selected, locked, labels } = self;
        let labels = labels.map_or_else(|| SliceLabels::shared(ui), Arc::new);
        let mut out = SliceBarOutput::default();
        let button = |ui: &mut Ui, text: &str, enabled: bool| ui.add(Button::new(text).size(ControlSize::Sm).enabled(enabled && !locked)).clicked();
        ui.push_id(id_salt, |ui| match mode {
            SliceMode::Slice => {
                let fields = ConstraintFields::new("style", &settings.style).show(ui);
                out.style = fields.event;
                out.unit_picked = fields.unit_picked;
                OptionsBar::divider(ui);
                if button(ui, &labels.from_guides, true) {
                    out.command = Some(SliceCommand::FromGuides);
                }
            }
            SliceMode::Select => {
                // PhotoCraft enables Promote and Divide… for any selection, Delete for a stored
                // slice; promoting a user slice does nothing, and a layer slice is not deleted by
                // the bar or the key (SH-T4).
                let promotable = matches!(selected, Some(SliceOrigin::Auto | SliceOrigin::Layer));
                if button(ui, &labels.promote, promotable) {
                    out.command = Some(SliceCommand::Promote);
                }
                if button(ui, &labels.divide, selected.is_some()) {
                    out.command = Some(SliceCommand::Divide);
                }
                if button(ui, &labels.delete, selected == Some(SliceOrigin::User)) {
                    out.command = Some(SliceCommand::Delete);
                }
                OptionsBar::divider(ui);
                let mut hide = settings.hide_auto;
                if Checkbox::bool(labels.hide_auto.as_str(), &mut hide).size(ControlSize::Sm).show(ui).response.changed() {
                    out.command = Some(SliceCommand::HideAuto(hide));
                }
                OptionsBar::divider(ui);
                if button(ui, &labels.options, selected.is_some()) {
                    out.command = Some(SliceCommand::Options);
                }
            }
        });
        out
    }
}
