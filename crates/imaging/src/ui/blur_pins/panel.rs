//! [`BlurPinsPanel`]: the selected pin's rows in the Blur Gallery dialog (task WS-BP2).

use egui::accesskit::Role;
use egui::{AsIdSalt, IdSalt, Response, Ui, UiBuilder};

use craft_theme::TypeRole;
use craft_ui::composites::SliderRow;
use craft_ui::edit::EditEvent;
use craft_ui::localized::{LocaleNumbers, Unit};
use craft_ui::primitives::{Content, Gap, Stack, Text};

use super::BlurPinsLabels;
use super::model::{BlurKind, BlurPins};

/// What [`BlurPinsPanel::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct BlurPinsPanelOutput {
    pub response: Response,
    /// The amount slider's gesture, with the pins after it.
    pub edit: Option<EditEvent<BlurPins>>,
}

/// The rows the Blur Gallery's filter dialog shows for its pins (in the dialog's body, above its
/// Preview check box): the selected pin's Blur (0..=500 px), or for Spin Blur Angle (0..=360°), a
/// `SliderRow`; "No pin selected" in `text.muted` without one; then the hints of the kind in the
/// caption role, `text.muted`, wrapped.
pub struct BlurPinsPanel<'a> {
    id_salt: IdSalt,
    pins: &'a mut BlurPins,
    labels: &'a BlurPinsLabels,
}

impl<'a> BlurPinsPanel<'a> {
    pub fn new(id_salt: impl AsIdSalt, pins: &'a mut BlurPins, labels: &'a BlurPinsLabels) -> Self {
        Self { id_salt: IdSalt::new(id_salt), pins, labels }
    }

    pub fn show(self, ui: &mut Ui) -> BlurPinsPanelOutput {
        // pc: crates/ui-egui/src/dialogs/blur_pins.rs:398 `body` (the slider, the hints).
        let Self { id_salt, pins, labels } = self;
        let id = ui.make_persistent_id(id_salt);
        let numbers = LocaleNumbers::of(ui);
        let kind = pins.kind;
        let mut edit = None;
        let builder = UiBuilder::new().scope_id(id).accessibility_role(Role::Group).accessibility_label(labels.pins.as_str());
        let group = ui.scope_builder(builder, |ui| {
            Stack::new(Gap::Stack).show(ui, |ui| {
                let selected = pins.selected.and_then(|index| pins.pins.get(index).map(|pin| (index, pin.amount)));
                match selected {
                    Some((index, amount)) => {
                        let (label, unit) = if kind == BlurKind::Spin { (&labels.blur_angle, "{n}°") } else { (&labels.blur, "{n} px") };
                        let row = SliderRow::new((id, "amount"), label.as_str(), amount, 0.0..=kind.max(), &numbers)
                            .decimals(0)
                            .step(1.0)
                            .unit(Unit::new(unit))
                            .show(ui);
                        for event in row.edits() {
                            if let Some(&value) = event.value() {
                                pins.pins[index].amount = value.clamp(0.0, kind.max());
                            }
                            edit = Some(event.map(|_| pins.clone()));
                        }
                    }
                    None => {
                        ui.add(Text::new(labels.no_pin.as_str()).role(TypeRole::Label).content(Content::Muted));
                    }
                }
                let hint = |ui: &mut Ui, text: &str| {
                    ui.add(Text::new(text).role(TypeRole::Caption).content(Content::Muted));
                };
                hint(ui, if kind == BlurKind::TiltShift { &labels.hint_tilt } else { &labels.hint_pins });
                match kind {
                    BlurKind::Iris => hint(ui, &labels.hint_iris),
                    BlurKind::Spin => hint(ui, &labels.hint_spin),
                    BlurKind::Field | BlurKind::TiltShift => {}
                }
            });
        });
        BlurPinsPanelOutput { response: group.response, edit }
    }
}
