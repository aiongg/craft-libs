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
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::{Sides, Ui};

use super::super::model::{AdjustField, Layer, LayerKind, Mock, RectField};
use super::super::options::ALIGN;
use super::{Outcome, resolve};

#[derive(Clone, Debug)]
pub struct State {
    /// Whether a width or height edit keeps the layer's ratio.
    link: bool,
    /// The rectangle from before the running transform edit: every step of it starts there, so a
    /// linked width typed digit by digit keeps the ratio, and a cancel restores it.
    rect: Option<[f32; 4]>,
    /// The adjustment from before the running slider edit.
    adjust: Option<LayerKind>,
    /// The slider row being dragged and the unrounded value it reported: the host shows it back,
    /// since a different value would end the drag as an external change.
    drag: Option<(usize, f64)>,
}

impl Default for State {
    fn default() -> Self {
        Self { link: true, rect: None, adjust: None, drag: None }
    }
}

pub fn show(ui: &mut Ui, mock: &mut Mock, state: &mut State) {
    Panel::new("properties").title("Properties").show(ui, |ui| {
        let layer = match mock.doc.selected.as_slice() {
            [] => return drop(EmptyState::new("No layer selected").show(ui)),
            [id] => mock.doc.layer(*id).cloned(),
            many => return drop(EmptyState::new(format!("{} layers selected", many.len())).show(ui)),
        };
        let Some(layer) = layer else { return };
        // Each layer has its own fields, so a gesture or focus never carries over to another layer.
        ui.push_id(layer.id, |ui| {
            header(ui, &layer);
            match layer.kind {
                LayerKind::Adjustment { brightness, contrast, clip } => adjustment(ui, mock, &layer, [brightness, contrast], clip, state),
                _ => {
                    transform(ui, mock, &layer, state);
                    align(ui, mock, &layer);
                }
            }
        });
    });
}

fn header(ui: &mut Ui, layer: &Layer) {
    let (icon, kind) = match layer.kind {
        LayerKind::Pixel { .. } => (Glyph::Layers, "Pixel Layer"),
        LayerKind::Empty => (Glyph::NewLayer, "Pixel Layer"),
        LayerKind::Adjustment { .. } => (Glyph::AdjustmentLayer, "Brightness/Contrast"),
    };
    Sides::new().shrink_left().truncate().show(
        ui,
        |ui| {
            ui.add(Icon::new(icon));
            ui.add(Text::new(layer.name.as_str()).role(TypeRole::BodyStrong).truncate());
        },
        |ui| {
            ui.add(Text::new(kind).content(Content::Faint));
        },
    );
}

/// A pixel field of the Transform section, scrubbed by its row's label.
fn field(ui: &mut Ui, label: &RowLabel, id: &str, name: &str, value: f32, numbers: &LocaleNumbers, enabled: bool) -> EditResponse<f64> {
    let field = NumberField::new(id, name, f64::from(value), numbers)
        .range(-100_000.0..=100_000.0)
        .decimals(0..=0)
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
    let mut events: Vec<(RectField, EditEvent<f64>)> = Vec::new();
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
                        ui.add(IconButton::new(icon, "Link width and height").size(ControlSize::Sm).enabled(enabled).toggle(&mut state.link))
                    },
                    |ui, label| field(ui, label, "properties.h", "Height", h, &numbers, enabled),
                )
                .inner;
            let (c, d) = PropertyRow::pair(glyph("X"), glyph("Y"))
                .size(ControlSize::Sm)
                .reserve_link_slot()
                .show(
                    ui,
                    |ui, label| field(ui, label, "properties.x", "X", x, &numbers, enabled),
                    |ui, label| field(ui, label, "properties.y", "Y", y, &numbers, enabled),
                )
                .inner;
            for (which, response) in [(RectField::W, a), (RectField::H, b), (RectField::X, c), (RectField::Y, d)] {
                events.extend(response.event.map(|event| (which, event)));
            }
        });
    });
    for (which, event) in events {
        let current = mock.doc.layer(layer.id).map_or(layer.rect, |l| l.rect);
        let base = state.rect.unwrap_or(current);
        match resolve(&event, &current, &mut state.rect) {
            Some(Outcome::Apply { value, record }) => {
                mock.doc.set_rect_field(layer.id, base, which, value, state.link);
                if record {
                    mock.record(if matches!(which, RectField::X | RectField::Y) { "Move Layer" } else { "Transform" });
                }
            }
            Some(Outcome::Restore(rect)) => {
                if let Some(layer) = mock.doc.layer_mut(layer.id) {
                    layer.rect = rect;
                }
            }
            None => {}
        }
    }
}

fn align(ui: &mut Ui, mock: &mut Mock, layer: &Layer) {
    let mut clicked = None;
    SectionHeader::new("properties.align", "Align and Distribute").show(ui, |ui| {
        ui.horizontal(|ui| {
            let gap = Tokens::of(ui).space.gap.stack;
            for (at, (icon, name, how)) in ALIGN.into_iter().enumerate() {
                if at == 3 {
                    ui.add_space(gap);
                }
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

fn adjustment(ui: &mut Ui, mock: &mut Mock, layer: &Layer, [brightness, contrast]: [i32; 2], clip: bool, state: &mut State) {
    let numbers = LocaleNumbers::of(ui);
    let enabled = !layer.locked;
    let mut shown = clip;
    if Checkbox::bool("Clip to layer below", &mut shown).size(ControlSize::Sm).enabled(enabled).show(ui).response.changed() {
        mock.doc.set_adjustment(layer.id, AdjustField::Clip(shown));
        mock.record("Clip Layer");
    }
    let rows = [("Brightness", brightness, -150.0..=150.0), ("Contrast", contrast, -50.0..=100.0)];
    for (at, (name, value, range)) in rows.into_iter().enumerate() {
        let shown = state.drag.filter(|(row, _)| *row == at).map_or(f64::from(value), |(_, v)| v);
        let row = SliderRow::new(("properties", name), name, shown, range, &numbers).decimals(0).enabled(enabled).show(ui);
        for event in row.edits() {
            let current = mock.doc.layer(layer.id).map_or_else(|| layer.kind.clone(), |l| l.kind.clone());
            state.drag = match event {
                EditEvent::Update { value, .. } => Some((at, value)),
                _ => None,
            };
            match resolve(&event, &current, &mut state.adjust) {
                Some(Outcome::Apply { value, record }) => {
                    let value = value.round() as i32;
                    mock.doc.set_adjustment(layer.id, if at == 0 { AdjustField::Brightness(value) } else { AdjustField::Contrast(value) });
                    if record {
                        mock.record("Brightness/Contrast");
                    }
                }
                Some(Outcome::Restore(kind)) => {
                    if let Some(layer) = mock.doc.layer_mut(layer.id) {
                        layer.kind = kind;
                    }
                }
                None => {}
            }
        }
    }
}
