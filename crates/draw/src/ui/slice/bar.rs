//! [`SliceBar`]: the Slice tools' options bar cells (task SH-T9).

use std::sync::Arc;

use egui::{AsIdSalt, IdSalt, Ui};

use super::{SliceCommand, SliceLabels, SliceMode, SliceOrigin, SliceToolSettings};
use craft_ui::canvas::Constraint;
use craft_ui::canvas::marquee::fields::ConstraintFields;
use craft_ui::controls::{Button, Checkbox};
use craft_ui::edit::EditEvent;
use craft_ui::localized::SharedLabels as _;
use craft_ui::localized::units::{LengthUnit, UnitContext};
use craft_ui::patterns::{OptionsBar, OptionsBarSlots};

/// What [`SliceBar`] reports into [`SliceBar::output`].
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
#[must_use = "the app stores the style and runs the command"]
pub struct SliceBarOutput {
    /// The Style, Width or Height changed (`ConstraintFieldsOutput::event`): tool settings, no
    /// undo step. A style pick resets the numbers (Fixed Size 64 × 64, else 1 × 1).
    pub style: Option<EditEvent<Constraint>>,
    /// A unit picked from a Fixed Size field's menu; reported only with
    /// [`SliceBar::size_units`], the app's ruler units.
    pub unit_picked: Option<LengthUnit>,
    /// A button: Slices From Guides (Slice); Promote, Divide…, Delete, Slice Options… and the Hide
    /// Auto Slices check box (Select).
    pub command: Option<SliceCommand>,
}

/// The Fixed Size unit, the units menu's list and the document's two extents.
type SizeUnits<'a> = (LengthUnit, &'a [LengthUnit], UnitContext, UnitContext);

/// The cells a Slice tool adds to the options bar.
///
/// - Slice: the kernel's `ConstraintFields` (Style, Width, swap, Height) over
///   [`SliceToolSettings::style`], then "Slices From Guides" (greyed
///   [`without a document`](Self::no_document)). Fixed Size shows in the units of
///   [`size_units`](Self::size_units) and reports a pick in [`SliceBarOutput::unit_picked`].
/// - Select: Promote (enabled for a selected auto or layer slice), Divide…, Delete (enabled for a
///   selected stored slice: user or layer), "Hide Auto Slices" check box, "Slice Options…"
///   (Divide… and Slice Options… for any selected slice).
/// - [`locked`](Self::locked): every button is greyed (View › Lock Slices).
/// - `add_to` fills the bar's body slot (the tools have no Commit or Cancel); the results go to
///   the [`SliceBarOutput`] of [`output`](Self::output).
/// - Keyboard: each cell is a Tab stop in the bar's order.
pub struct SliceBar<'a> {
    id_salt: IdSalt,
    mode: SliceMode,
    settings: &'a SliceToolSettings,
    selected: Option<SliceOrigin>,
    locked: bool,
    no_document: bool,
    size_units: Option<SizeUnits<'a>>,
    labels: Option<SliceLabels>,
    output: Option<&'a mut SliceBarOutput>,
}

impl<'a> SliceBar<'a> {
    /// The bar of `mode` with `settings`.
    pub fn new(id_salt: impl AsIdSalt, mode: SliceMode, settings: &'a SliceToolSettings) -> Self {
        SliceBar {
            id_salt: IdSalt::new(id_salt),
            mode,
            settings,
            selected: None,
            locked: false,
            no_document: false,
            size_units: None,
            labels: None,
            output: None,
        }
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

    /// Whether no document is open: Slices From Guides is greyed (PhotoCraft's `has_doc`).
    pub fn no_document(mut self, no_document: bool) -> Self {
        self.no_document = no_document;
        self
    }

    /// Shows Fixed Size in `unit`, with `units` in the units menu (the app's ruler units list),
    /// converting through `width` and `height` (the document's resolution); passed to
    /// `ConstraintFields::size_units`. Without it Fixed Size is in pixels with no menu.
    pub fn size_units(mut self, unit: LengthUnit, units: &'a [LengthUnit], width: UnitContext, height: UnitContext) -> Self {
        self.size_units = Some((unit, units, width, height));
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: SliceLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Where the bar reports its results.
    pub fn output(mut self, output: &'a mut SliceBarOutput) -> Self {
        self.output = Some(output);
        self
    }

    /// Fills the bar's body slot.
    pub fn add_to(self, slots: &mut OptionsBarSlots<'a>) {
        slots.body(move |ui| self.cells(ui));
    }

    fn cells(self, ui: &mut Ui) {
        // pc: crates/ui-egui/src/tools/slice_ui.rs:404-471 `options_bar` (Style :414-417 with the
        // 64 × 64 / 1 × 1 reset, the swap button :433, Slices From Guides :440 greyed without a
        // document, Promote :447, Divide… :450, Delete :453, Hide Auto Slices :460, Slice
        // Options… :462).
        // new: the Style cells are `craft_ui::canvas::ConstraintFields` (K13).
        let Self { id_salt, mode, settings, selected, locked, no_document, size_units, labels, output } = self;
        let labels = labels.map_or_else(|| SliceLabels::shared(ui), Arc::new);
        let mut out = SliceBarOutput::default();
        let button =
            |ui: &mut Ui, text: &str, enabled: bool| ui.add(Button::new(text).size(OptionsBar::CONTROL_SIZE).enabled(enabled && !locked)).clicked();
        ui.push_id(id_salt, |ui| match mode {
            SliceMode::Slice => {
                let mut fields = ConstraintFields::new("style", &settings.style);
                if let Some((unit, units, width, height)) = size_units {
                    fields = fields.size_units(unit, units, width, height);
                }
                let fields = fields.show(ui);
                out.style = fields.event;
                out.unit_picked = fields.unit_picked;
                OptionsBar::divider(ui);
                if button(ui, &labels.from_guides, !no_document) {
                    out.command = Some(SliceCommand::FromGuides);
                }
            }
            SliceMode::Select => {
                // PhotoCraft enables Promote and Divide… for any selection, Delete for a stored
                // slice; promoting a user slice does nothing. Delete takes user and layer slices
                // alike, as the key does (Photoshop deletes layer-based slices).
                let promotable = matches!(selected, Some(SliceOrigin::Auto | SliceOrigin::Layer));
                if button(ui, &labels.promote, promotable) {
                    out.command = Some(SliceCommand::Promote);
                }
                if button(ui, &labels.divide, selected.is_some()) {
                    out.command = Some(SliceCommand::Divide);
                }
                if button(ui, &labels.delete, selected.is_some_and(SliceOrigin::stored)) {
                    out.command = Some(SliceCommand::Delete);
                }
                OptionsBar::divider(ui);
                let mut hide = settings.hide_auto;
                if Checkbox::bool(labels.hide_auto.as_str(), &mut hide).size(OptionsBar::CONTROL_SIZE).show(ui).response.changed() {
                    out.command = Some(SliceCommand::HideAuto(hide));
                }
                OptionsBar::divider(ui);
                if button(ui, &labels.options, selected.is_some()) {
                    out.command = Some(SliceCommand::Options);
                }
            }
        });
        if let Some(output) = output {
            output.style = out.style.or(output.style.take());
            output.unit_picked = out.unit_picked.or(output.unit_picked);
            output.command = out.command.or(output.command);
        }
    }
}
