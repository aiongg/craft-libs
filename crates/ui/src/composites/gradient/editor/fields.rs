//! The fields row under the bar: the selected marker's fields as [`PropertyRow`]s, the Color well
//! and its popover, and the Delete button, each a part of the editor's edit.

use std::any::Any;
use std::ops::RangeInclusive;

use craft_theme::{ControlSize, Rgba, Srgba};
use egui::{AsIdSalt, Id, Rect, Response, Ui, UiBuilder, pos2, vec2};

use super::{Edits, GradientEditor, State, StopRef, locations, midpoint};
use crate::composites::PropertyRow;
use crate::composites::color::{ColorPicker, PickedColor, popover};
use crate::composites::gradient::{GradientStops, StopRow, StopRows, stops};
use crate::controls::{IconButton, NumberField, Swatch};
use crate::direction;
use crate::edit::{EditEvent, EditPolicy, EditResponse};
use crate::icons::Icon;
use crate::localized::Unit;
use crate::theme::{Tokens, TokensOf};

const FIELDS: &str = "craft_ui::gradient_editor::fields";

/// What the colour popover's picker reported in a pass.
#[derive(Default)]
pub(super) struct PickerReport {
    pub eyedropper_requested: bool,
    pub picker_id: Option<Id>,
}

/// What the number fields of a pass share.
struct Look {
    policy: EditPolicy,
    width: f32,
    enabled: bool,
}

impl<'a, T: Any + Send + Sync> GradientEditor<'a, T> {
    /// The selected marker's fields under the bar, drawn from `stops` (the stops the bar leaves)
    /// and pushed into `edits` after the bar (PhotoCraft's rows): a colour stop's Color well, its
    /// Opacity under [`StopRows::Combined`], an opacity stop's Opacity; a stop's Location and the
    /// Delete button; a midpoint's Midpoint. Returns the colour request (the bar's `requested`
    /// or a click on the well) and what the popover's picker reported.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn fields_row(
        &self,
        ui: &mut Ui,
        id: Id,
        stops: &GradientStops,
        state: &mut State,
        edits: &mut Edits,
        enabled: bool,
        requested: Option<usize>,
    ) -> (Option<usize>, PickerReport) {
        let builder = UiBuilder::new().scope_id(id.with(FIELDS)).layout(direction::column(ui));
        ui.scope_builder(builder, |ui| {
            let unit = Unit::new(&self.labels.percent);
            let look = Look {
                policy: EditPolicy::default().cancellable(self.policy.cancellable).cost(self.policy.cost),
                width: NumberField::fit_width(ui, ControlSize::Md, self.numbers, 0.0..=100.0, self.location_decimals, &unit),
                enabled,
            };
            let selected = state.selected.filter(|m| match *m {
                StopRef::Midpoint(row, i) => i + 1 < stops.len(row),
                m => m.index() < stops.len(m.row()),
            });
            let mut requested = requested;
            let mut picker = PickerReport::default();
            let mut well = false;
            match selected {
                Some(StopRef::Color(i)) => {
                    well = true;
                    (requested, picker) = self.well(ui, stops, i, state, edits, enabled, requested);
                    if self.rules.rows == StopRows::Combined {
                        let color = stops.color[i].color;
                        let field = self.number(ui, ("opacity", StopRow::Color), &self.labels.opacity, percent(color.a), 0.0..=100.0, 0, &look);
                        edits.typing(ui.ctx(), &field, |v| {
                            let mut out = stops.clone();
                            out.color[i].color.a = (v / 100.0) as f32;
                            out
                        });
                    }
                    self.location(ui, StopRow::Color, i, stops, state, edits, &look);
                    self.delete(ui, StopRow::Color, stops, edits, enabled);
                }
                Some(StopRef::Opacity(i)) => {
                    let opacity = stops.opacity_stops()[i].opacity;
                    let field = self.number(ui, ("opacity", StopRow::Opacity), &self.labels.opacity, percent(opacity), 0.0..=100.0, 0, &look);
                    edits.typing(ui.ctx(), &field, |v| {
                        let mut out = GradientStops { color: stops.color.clone(), opacity: stops.opacity_stops().into_owned() };
                        out.opacity[i].opacity = (v / 100.0) as f32;
                        out
                    });
                    self.location(ui, StopRow::Opacity, i, stops, state, edits, &look);
                    self.delete(ui, StopRow::Opacity, stops, edits, enabled);
                }
                Some(StopRef::Midpoint(row, i)) => {
                    let range = percent(*self.rules.midpoint.start())..=percent(*self.rules.midpoint.end());
                    let label = &self.labels.midpoint_field;
                    let field = self.number(ui, ("midpoint", row), label, percent(midpoint(stops, row, i)), range, 0, &look);
                    let rules = &self.rules;
                    edits.typing(ui.ctx(), &field, |v| {
                        stops::set_midpoint(stops, row, i, (v / 100.0) as f32, rules).map_or_else(|| stops.clone(), |(s, _)| s)
                    });
                }
                None => {}
            }
            if !well && let Some(anchor) = state.popover.take() {
                popover::close(ui.ctx(), anchor);
            }
            (requested, picker)
        })
        .inner
    }

    /// The Color well of colour stop `i`: a click asks for its colour, or opens the colour
    /// popover with [`color_popover`](GradientEditor::color_popover), which a request from the
    /// bar opens too. Returns the colour request and what the popover's picker reported.
    #[allow(clippy::too_many_arguments)]
    fn well(
        &self,
        ui: &mut Ui,
        stops: &GradientStops,
        i: usize,
        state: &mut State,
        edits: &mut Edits,
        enabled: bool,
        requested: Option<usize>,
    ) -> (Option<usize>, PickerReport) {
        let side = Swatch::style(&Tokens::of(ui), ControlSize::Md).side;
        let color = self.stop_color(stops, i);
        let mut out = requested;
        let mut picker = PickerReport::default();
        PropertyRow::word(&self.labels.color).show(ui, |ui, _| {
            at_start(ui, side, |ui| {
                let well = ui.add(Swatch::new("color", self.labels.color.as_str(), Rgba::from(color)).enabled(enabled));
                if !self.color_popover {
                    if well.clicked() {
                        out = Some(i);
                    }
                    return well;
                }
                let toggled = well.clicked() || (requested.is_some() && state.popover != Some(well.id));
                let shown;
                (shown, picker) = self.popup(ui, well, toggled, stops, i, state, edits, enabled);
                shown
            })
        });
        (out, picker)
    }

    /// The colour popover on the bar, for an editor without the fields row
    /// ([`color_popover`](GradientEditor::color_popover)): a request opens it on the selected
    /// colour stop.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn popover(
        &self,
        ui: &mut Ui,
        bar: Response,
        stops: &GradientStops,
        state: &mut State,
        edits: &mut Edits,
        enabled: bool,
        requested: Option<usize>,
    ) -> PickerReport {
        match state.selected {
            Some(StopRef::Color(i)) if i < stops.color.len() => {
                let toggled = requested.is_some() && state.popover != Some(bar.id);
                // The edit is pushed; the response is the bar's own.
                self.popup(ui, bar, toggled, stops, i, state, edits, enabled).1
            }
            _ => {
                if let Some(anchor) = state.popover.take() {
                    popover::close(ui.ctx(), anchor);
                }
                PickerReport::default()
            }
        }
    }

    /// The colour popover of colour stop `i` under `anchor`, which `toggled` opens or closes, with
    /// the host's [`color_picker`](GradientEditor::color_picker) setup. Its picker is a part of the
    /// editor's edit, mapped to the stop. Returns the anchor's response and what the picker
    /// reported.
    #[allow(clippy::too_many_arguments)]
    fn popup(
        &self,
        ui: &mut Ui,
        anchor: Response,
        toggled: bool,
        stops: &GradientStops,
        i: usize,
        state: &mut State,
        edits: &mut Edits,
        enabled: bool,
    ) -> (Response, PickerReport) {
        let rows = self.rules.rows;
        let combined = rows == StopRows::Combined;
        let host = self.picker.take();
        let setup: Box<dyn FnOnce(ColorPicker<'a>) -> ColorPicker<'a> + 'a> = Box::new(move |picker| {
            let picker = picker.alpha(combined);
            match host {
                Some(host) => host(picker),
                None => picker,
            }
        });
        let anchor_id = anchor.id;
        let shown = popover::popup(ui, anchor, toggled, &self.labels.color, self.stop_color(stops, i), self.numbers, enabled, Some(setup));
        state.popover = shown.open.then_some(anchor_id);
        // The picker is the part, not the anchor: when it isn't drawn its gesture is over.
        if let Some(picker) = shown.picker_id {
            edits.push_as(ui.ctx(), picker, shown.edit.event.as_ref(), |picked: PickedColor| {
                let mut out = stops.clone();
                let stop = &mut out.color[i].color;
                // A split gradient's colour stops keep their own alpha, which nothing shows.
                *stop = if combined { picked.color } else { Srgba { a: stop.a, ..picked.color } };
                out
            });
        }
        (shown.edit.response, PickerReport { eyedropper_requested: shown.eyedropper_requested, picker_id: shown.picker_id })
    }

    /// Colour stop `i`'s colour as the well shows it: opaque in a split gradient.
    fn stop_color(&self, stops: &GradientStops, i: usize) -> Srgba {
        let color = stops.color[i].color;
        match self.rules.rows {
            StopRows::Split => Srgba { a: 1.0, ..color },
            StopRows::Combined => color,
        }
    }

    /// The Location field of stop `i` of `row`. The selection follows the stop when it passes a
    /// neighbour.
    #[allow(clippy::too_many_arguments)]
    fn location(&self, ui: &mut Ui, row: StopRow, i: usize, stops: &GradientStops, state: &mut State, edits: &mut Edits, look: &Look) {
        let at = locations(stops, row)[i];
        let field = self.number(ui, ("location", row), &self.labels.location, percent(at), 0.0..=100.0, self.location_decimals, look);
        let rules = &self.rules;
        let moved = |v: f64| stops::move_to(stops, row, i, (v / 100.0) as f32, rules);
        if let Some(v) = field.event.as_ref().and_then(EditEvent::value)
            && let Some((_, j)) = moved(*v)
        {
            state.selected = Some(StopRef::stop(row, j));
        }
        edits.typing(ui.ctx(), &field, |v| moved(v).map_or_else(|| stops.clone(), |(s, _)| s));
    }

    /// A number field in percent in a row labelled `name`, which scrubs it; `look.width` wide at
    /// the reading start of the control column.
    #[allow(clippy::too_many_arguments)]
    fn number(
        &self,
        ui: &mut Ui,
        salt: impl AsIdSalt,
        name: &str,
        value: f64,
        range: RangeInclusive<f64>,
        decimals: usize,
        look: &Look,
    ) -> EditResponse<f64> {
        PropertyRow::word(name)
            .scrubs()
            .show(ui, |ui, row| {
                at_start(ui, look.width, |ui| {
                    let field = NumberField::new(salt, name, value, self.numbers)
                        .range(range)
                        .decimals(decimals..=decimals)
                        .unit(Unit::new(&self.labels.percent))
                        .policy(look.policy)
                        .enabled(look.enabled);
                    match row.scrub() {
                        Some(label) => field.scrub_by(label),
                        None => field,
                    }
                    .show(ui)
                })
            })
            .inner
    }

    /// The Delete button under the fields, at the reading start of the control column
    /// (PhotoCraft); it removes the selected stop of `row`, and is disabled at the row's minimum.
    /// Its click waits for a typing run as a press does ([`Edits::clicks`]).
    fn delete(&self, ui: &mut Ui, row: StopRow, stops: &GradientStops, edits: &mut Edits, enabled: bool) {
        let tokens = Tokens::of(ui);
        let label = PropertyRow::style(&tokens, ControlSize::Md);
        let side = IconButton::style(&tokens, ControlSize::Md).side;
        let full = ui.available_rect_before_wrap();
        let indent = (label.word_width + label.label_gap).min((full.width() - side).max(0.0));
        let x = if direction::is_rtl(ui) { full.right() - indent - side } else { full.left() + indent };
        let rect = Rect::from_min_size(pos2(x, full.top()), vec2(side, side));
        let button = IconButton::new(Icon::Trash, self.labels.delete.as_str()).id_salt("delete").enabled(enabled && stops.len(row) > self.rules.min);
        if ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| ui.add(button)).inner.clicked() {
            edits.click(());
        }
    }
}

/// A 0..=1 value in percent, without the binary fraction an `f32` adds (0.3 is 30, not
/// 30.0000012).
fn percent(v: f32) -> f64 {
    (f64::from(v) * 1e6).round() / 1e4
}

/// `add` in a column `width` wide at the reading start of the `Ui`.
fn at_start<R>(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let full = ui.available_rect_before_wrap();
    let width = width.min(full.width());
    let x = if direction::is_rtl(ui) { full.right() - width } else { full.left() };
    let rect = Rect::from_min_size(pos2(x, full.top()), vec2(width, full.height()));
    ui.scope_builder(UiBuilder::new().max_rect(rect).layout(egui::Layout::top_down_justified(direction::start_align(ui))), add).inner
}
