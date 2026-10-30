//! The Properties panel: the selected layer's name and kind, its rectangle and alignment, or an
//! adjustment layer's values.

use craft_theme::{ControlSize, TypeRole};
use craft_ui::composites::{EmptyState, PropertyGroup, PropertyLabel, PropertyRow, RowLabel, SectionHeader, SliderRow};
use craft_ui::controls::{Checkbox, IconButton, NumberField};
use craft_ui::edit::{EditEvent, EditResponse};
use craft_ui::icons::Icon as Glyph;
use craft_ui::localized::{LocaleNumbers, Unit};
use craft_ui::patterns::Panel;
use craft_ui::primitives::{Content, Icon, Text};
use egui::Ui;

use super::super::model::{AdjustField, Layer, LayerKind, Mock, RectField};
use super::super::options::ALIGN;

#[derive(Clone, Debug, Default)]
pub struct State {
    /// Whether a width or height edit keeps the layer's ratio.
    link: bool,
}

/// The value a field or slider event hands the host and whether the edit is committed: a typed
/// entry or the end of a drag or scrub, as against a step of one that goes on.
fn taken(event: &EditEvent<f64>) -> Option<(f64, bool)> {
    match event {
        EditEvent::Update { value, .. } => Some((*value, false)),
        EditEvent::Commit { value, changed: true, .. } | EditEvent::Set { value, .. } => Some((*value, true)),
        _ => None,
    }
}

pub fn show(ui: &mut Ui, mock: &mut Mock, state: &mut State) {
    Panel::new("properties").title("Properties").show(ui, |ui| {
        let Some(layer) = mock.doc.selected_layer().cloned() else {
            EmptyState::new("No layer selected").show(ui);
            return;
        };
        header(ui, &layer);
        match layer.kind {
            LayerKind::Adjustment { brightness, contrast, clip } => adjustment(ui, mock, &layer, [brightness, contrast], clip),
            _ => {
                transform(ui, mock, &layer, state);
                align(ui, mock, &layer);
            }
        }
    });
}

fn header(ui: &mut Ui, layer: &Layer) {
    let (icon, kind) = match layer.kind {
        LayerKind::Pixel { .. } => (Glyph::Layers, "Pixel layer"),
        LayerKind::Empty => (Glyph::NewLayer, "Empty layer"),
        LayerKind::Adjustment { .. } => (Glyph::AdjustmentLayer, "Adjustment layer"),
    };
    ui.horizontal(|ui| {
        ui.add(Icon::new(icon));
        ui.add(Text::new(layer.name.as_str()).role(TypeRole::BodyStrong));
        ui.with_layout(craft_ui::direction::trailing(ui), |ui| {
            ui.add(Text::new(kind).content(Content::Faint));
        });
    });
}

/// A pixel field of the Transform section, scrubbed by its row's label.
fn field(ui: &mut Ui, label: &RowLabel, id: &str, name: &str, value: f32, numbers: &LocaleNumbers, enabled: bool) -> EditResponse<f64> {
    let field = NumberField::new(id, name, f64::from(value), numbers)
        .range(-100_000.0..=100_000.0)
        .decimals(0..=2)
        .unit(Unit::new("{n} px"))
        .size(ControlSize::Sm)
        .enabled(enabled);
    match label.scrub() {
        Some(label) => field.scrub_by(label),
        None => field,
    }
    .show(ui)
}

fn transform(ui: &mut Ui, mock: &mut Mock, layer: &Layer, state: &mut State) {
    let numbers = LocaleNumbers::of(ui);
    let enabled = !layer.locked;
    let [x, y, w, h] = layer.rect;
    let mut edits: Vec<(RectField, f64, bool)> = Vec::new();
    SectionHeader::new("properties.transform", "Transform").show(ui, |ui| {
        PropertyGroup::new("properties.transform").show(ui, |ui| {
            let glyph = |text: &str| PropertyLabel::glyph(text).scrubs();
            let (a, _, b) = PropertyRow::pair(glyph("W"), glyph("H"))
                .size(ControlSize::Sm)
                .show_linked(
                    ui,
                    |ui, label| field(ui, label, "properties.w", "Width", w, &numbers, enabled),
                    |ui| {
                        let icon = if state.link { Glyph::Link } else { Glyph::Unlink };
                        ui.add(IconButton::new(icon, "Constrain proportions").size(ControlSize::Sm).enabled(enabled).toggle(&mut state.link))
                    },
                    |ui, label| field(ui, label, "properties.h", "Height", h, &numbers, enabled),
                )
                .inner;
            let (c, d) = PropertyRow::pair(glyph("X"), glyph("Y"))
                .size(ControlSize::Sm)
                .show(
                    ui,
                    |ui, label| field(ui, label, "properties.x", "X", x, &numbers, enabled),
                    |ui, label| field(ui, label, "properties.y", "Y", y, &numbers, enabled),
                )
                .inner;
            for (which, response) in [(RectField::W, a), (RectField::H, b), (RectField::X, c), (RectField::Y, d)] {
                if let Some((value, commit)) = response.event.as_ref().and_then(taken) {
                    edits.push((which, value, commit));
                }
            }
        });
    });
    for (which, value, commit) in edits {
        mock.doc.set_rect_field(layer.id, which, value, state.link);
        if commit {
            mock.record(if matches!(which, RectField::X | RectField::Y) { "Move Layer" } else { "Transform" });
        }
    }
}

fn align(ui: &mut Ui, mock: &mut Mock, layer: &Layer) {
    let mut clicked = None;
    SectionHeader::new("properties.align", "Align and Distribute").show(ui, |ui| {
        ui.horizontal(|ui| {
            for (icon, name, how) in ALIGN {
                if ui.add(IconButton::new(icon, name).size(ControlSize::Sm).enabled(!layer.locked)).clicked() {
                    clicked = Some(how);
                }
            }
        });
    });
    if let Some(how) = clicked {
        mock.align_selected(how);
    }
}

fn adjustment(ui: &mut Ui, mock: &mut Mock, layer: &Layer, [brightness, contrast]: [i32; 2], clip: bool) {
    let numbers = LocaleNumbers::of(ui);
    let enabled = !layer.locked;
    let mut edits: Vec<(AdjustField, bool)> = Vec::new();
    let mut shown = clip;
    if Checkbox::bool("Clip to layer below", &mut shown).size(ControlSize::Sm).enabled(enabled).show(ui).response.changed() {
        edits.push((AdjustField::Clip(shown), true));
    }
    let rows = [("Brightness", brightness, -150.0..=150.0), ("Contrast", contrast, -50.0..=100.0)];
    for (at, (name, value, range)) in rows.into_iter().enumerate() {
        let row = SliderRow::new(("properties", name), name, value, range, &numbers).decimals(0).enabled(enabled).show(ui);
        for event in row.edits() {
            if let Some((value, commit)) = taken(&event) {
                let value = value.round() as i32;
                edits.push((if at == 0 { AdjustField::Brightness(value) } else { AdjustField::Contrast(value) }, commit));
            }
        }
    }
    for (change, commit) in edits {
        mock.doc.set_adjustment(layer.id, change);
        if commit {
            mock.record(if matches!(change, AdjustField::Clip(_)) { "Clip Layer" } else { "Brightness/Contrast" });
        }
    }
}
