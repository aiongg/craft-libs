//! The fields row under the bar: the selected marker's fields as [`PropertyRow`]s, the Color well
//! and its popover, and the Delete button, each a part of the editor's edit.

use std::any::Any;
use std::ops::RangeInclusive;

use craft_theme::{ControlSize, Rgba, Srgba};
use egui::{AsIdSalt, Id, Rect, Response, Ui, UiBuilder, pos2, vec2};

use super::{Edits, GradientEditor, GradientOp, Part, State, StopField, StopRef, locations, midpoint};
use crate::composites::PropertyRow;
use crate::composites::color::{ColorPicker, PickedColor, popover};
use crate::composites::gradient::stops::{self, StopKey};
use crate::composites::gradient::{GradientStops, StopRow, StopRows};
use crate::controls::number_field::FieldRule;
use crate::controls::{IconButton, NumberField, Swatch};
use crate::direction;
use crate::edit::ops::{self, ScalarOp};
use crate::edit::{self, EditEvent, EditPolicy, EditResponse, GestureId};
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

/// The Location field's run under way: the stops as it began and the stop's index in them. Every
/// value of the run moves that stop from there, as a drag edits the stops at its press, so a run
/// that passes a neighbour and comes back leaves the stops as they were.
#[derive(Clone, Debug)]
pub(super) struct LocationRun {
    run: GestureId,
    row: StopRow,
    index: usize,
    base: GradientStops,
}

/// What the number fields of a pass share.
struct Look {
    policy: EditPolicy,
    width: f32,
    enabled: bool,
}

impl<'a, T: Any + Send + Sync> GradientEditor<'a, T> {
    /// The selected marker's fields under the bar, pushed into `edits` after the bar (PhotoCraft's
    /// rows), each drawn from and mapped onto [`Edits::value`] as the parts before it leave it: a
    /// colour stop's Color well, its Opacity under [`StopRows::Combined`], an opacity stop's
    /// Opacity; a stop's Location and the Delete button; a midpoint's Midpoint. Returns the colour
    /// request (the bar's `requested` or a click on the well) and what the popover's picker
    /// reported.
    pub(super) fn fields_row(
        &self,
        ui: &mut Ui,
        id: Id,
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
            let stops = edits.value().clone();
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
                    (requested, picker) = self.well(ui, i, state, edits, enabled, requested);
                    if self.rules.rows == StopRows::Combined {
                        self.field(ui, StopField::Opacity, StopRow::Color, i, state, edits, &look);
                    }
                    self.location(ui, StopRow::Color, i, state, edits, &look);
                    self.delete(ui, StopRow::Color, edits, enabled);
                }
                Some(StopRef::Opacity(i)) => {
                    self.field(ui, StopField::Opacity, StopRow::Opacity, i, state, edits, &look);
                    self.location(ui, StopRow::Opacity, i, state, edits, &look);
                    self.delete(ui, StopRow::Opacity, edits, enabled);
                }
                Some(StopRef::Midpoint(row, i)) => self.field(ui, StopField::Midpoint, row, i, state, edits, &look),
                None => {}
            }
            if !well && let Some(anchor) = state.popover.take() {
                popover::close(ui.ctx(), anchor);
            }
            (requested, picker)
        })
        .inner
    }

    /// The Opacity or Midpoint field of stop `i` of `row`, drawn from and mapped onto
    /// [`Edits::value`].
    #[allow(clippy::too_many_arguments)]
    fn field(&self, ui: &mut Ui, field: StopField, row: StopRow, i: usize, state: &mut State, edits: &mut Edits, look: &Look) {
        let stops = edits.value();
        let (name, salt) = match field {
            StopField::Midpoint => (&self.labels.midpoint_field, "midpoint"),
            _ => (&self.labels.opacity, "opacity"),
        };
        let value = self.field_value(field, stops, row, i);
        let key = StopKey::of(stops, row, i);
        let edit = self.number(ui, (salt, row), name, value, field, look);
        let ops = self.field_ops(ui, field, key, &edit);
        self.push(edits, state, Part::Field, &edit, ops, |v, now| self.set_field(field, now, row, i, v).map_or_else(|| now.clone(), |(s, _)| s));
    }

    /// The operations `edit` of `field` reported, on the stop of `key`.
    fn field_ops(&self, ui: &Ui, field: StopField, key: Option<StopKey>, edit: &EditResponse<f64>) -> Vec<GradientOp> {
        let reported = ops::take::<ScalarOp>(ui.ctx(), edit.response.id);
        key.map_or_else(Vec::new, |key| reported.into_iter().map(|op| GradientOp::Field(field, key.clone(), op)).collect())
    }

    /// The range and the decimals of `field`, in percent.
    fn field_range(&self, field: StopField) -> (RangeInclusive<f64>, usize) {
        match field {
            StopField::Opacity => (0.0..=100.0, 0),
            StopField::Location => (0.0..=100.0, self.location_decimals),
            StopField::Midpoint => (percent(*self.rules.midpoint.start())..=percent(*self.rules.midpoint.end()), 0),
        }
    }

    /// How `field`'s operations apply ([`number_field::apply`]).
    pub(super) fn field_rule(&self, field: StopField) -> FieldRule {
        let (range, places) = self.field_range(field);
        FieldRule::new(&range, 1.0, places)
    }

    /// What `field` of stop `i` of `row` shows, in percent.
    pub(super) fn field_value(&self, field: StopField, stops: &GradientStops, row: StopRow, i: usize) -> f64 {
        percent(match (field, row) {
            (StopField::Opacity, StopRow::Color) => stops.color[i].color.a,
            (StopField::Opacity, StopRow::Opacity) => stops.opacity_stops()[i].opacity,
            (StopField::Location, _) => locations(stops, row)[i],
            (StopField::Midpoint, _) => midpoint(stops, row, i),
        })
    }

    /// `stops` with `field` of stop `i` of `row` at `v` percent, and the stop's index after it
    /// ([`stops::keeping`]).
    pub(super) fn set_field(&self, field: StopField, stops: &GradientStops, row: StopRow, i: usize, v: f64) -> Option<(GradientStops, usize)> {
        let v = (v / 100.0) as f32;
        match (field, row) {
            (StopField::Opacity, StopRow::Color) => {
                let mut out = stops.clone();
                out.color[i].color.a = v;
                Some((out, i))
            }
            (StopField::Opacity, StopRow::Opacity) => {
                let mut out = GradientStops { color: stops.color.clone(), opacity: stops.opacity_stops().into_owned() };
                out.opacity[i].opacity = v;
                Some((out, i))
            }
            (StopField::Location, _) => stops::move_to(stops, row, i, v, &self.rules),
            (StopField::Midpoint, _) => stops::set_midpoint(stops, row, i, v, &self.rules),
        }
    }

    /// The Color well of colour stop `i`: a click asks for its colour, or opens the colour
    /// popover with [`color_popover`](GradientEditor::color_popover), which a request from the
    /// bar opens too. Returns the colour request and what the popover's picker reported.
    fn well(
        &self,
        ui: &mut Ui,
        i: usize,
        state: &mut State,
        edits: &mut Edits,
        enabled: bool,
        requested: Option<usize>,
    ) -> (Option<usize>, PickerReport) {
        let side = Swatch::style(&Tokens::of(ui), ControlSize::Md).side;
        let color = self.stop_color(edits.value(), i);
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
                (shown, picker) = self.popup(ui, well, toggled, i, state, edits, enabled);
                shown
            })
        });
        (out, picker)
    }

    /// The colour popover on the bar, for an editor without the fields row
    /// ([`color_popover`](GradientEditor::color_popover)): a request opens it on the selected
    /// colour stop.
    pub(super) fn popover(
        &self,
        ui: &mut Ui,
        bar: Response,
        state: &mut State,
        edits: &mut Edits,
        enabled: bool,
        requested: Option<usize>,
    ) -> PickerReport {
        match state.selected {
            Some(StopRef::Color(i)) if i < edits.value().color.len() => {
                let toggled = requested.is_some() && state.popover != Some(bar.id);
                // The edit is pushed; the response is the bar's own.
                self.popup(ui, bar, toggled, i, state, edits, enabled).1
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
    /// editor's edit: its colour is an absolute operation on the stop ([`GradientOp::SetColor`]).
    /// Returns the anchor's response and what the picker reported.
    #[allow(clippy::too_many_arguments)]
    fn popup(
        &self,
        ui: &mut Ui,
        anchor: Response,
        toggled: bool,
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
        let key = StopKey::of(edits.value(), StopRow::Color, i);
        let color = self.stop_color(edits.value(), i);
        let shown = popover::popup(ui, anchor, toggled, &self.labels.color, color, self.numbers, enabled, Some(setup));
        state.popover = shown.open.then_some(anchor_id);
        // The picker is the part, not the anchor: when it isn't drawn its gesture is over.
        if let Some(picker) = shown.picker_id {
            let ops = match (&key, shown.edit.event.as_ref().and_then(EditEvent::value)) {
                (Some(at), Some(picked)) => vec![GradientOp::SetColor { at: at.clone(), color: picked.color }],
                _ => Vec::new(),
            };
            self.push(edits, state, Part::Picker(picker), &shown.edit, ops, |picked: PickedColor, now| self.with_color(now, i, picked.color));
        }
        (shown.edit.response, PickerReport { eyedropper_requested: shown.eyedropper_requested, picker_id: shown.picker_id })
    }

    /// Colour stop `i`'s colour as the well shows it: opaque in a split gradient.
    fn stop_color(&self, stops: &GradientStops, i: usize) -> Srgba {
        let color = stops.color[i].color;
        match self.rules.rows {
            StopRows::Split | StopRows::ColorOnly => Srgba { a: 1.0, ..color },
            StopRows::Combined => color,
        }
    }

    /// The Location field of stop `i` of `row`, drawn from and mapped onto [`Edits::value`]. Each
    /// value of a run moves the stop from where the run began ([`LocationRun`]); the selection
    /// follows the stop when it passes a neighbour.
    fn location(&self, ui: &mut Ui, row: StopRow, i: usize, state: &mut State, edits: &mut Edits, look: &Look) {
        let stops = &edits.value().clone();
        let value = self.field_value(StopField::Location, stops, row, i);
        let field = self.number(ui, ("location", row), &self.labels.location, value, StopField::Location, look);
        let event_run = field.event.as_ref().and_then(EditEvent::id);
        let run = state
            .location
            .take()
            .filter(|r| r.row == row && (Some(r.run) == event_run || edit::is_live(ui.ctx(), r.run)))
            .or_else(|| event_run.map(|run| LocationRun { run, row, index: i, base: stops.clone() }));
        let (base, from) = run.as_ref().map_or((stops, i), |r| (&r.base, r.index));
        let rules = &self.rules;
        let moved = |v: f64| stops::move_to(base, row, from, (v / 100.0) as f32, rules);
        // A run's values move the selection here; a Set's operations as they apply.
        if event_run.is_some()
            && let Some(v) = field.event.as_ref().and_then(EditEvent::value)
            && let Some((_, j)) = moved(*v)
        {
            state.selected = Some(StopRef::stop(row, j));
        }
        let ops = self.field_ops(ui, StopField::Location, StopKey::of(stops, row, i), &field);
        self.push(edits, state, Part::Field, &field, ops, |v, _| moved(v).map_or_else(|| stops.clone(), |(s, _)| s));
        state.location = run;
    }

    /// `field`'s number field in percent in a row labelled `name`, which scrubs it; `look.width`
    /// wide at the reading start of the control column.
    fn number(&self, ui: &mut Ui, salt: impl AsIdSalt, name: &str, value: f64, field: StopField, look: &Look) -> EditResponse<f64> {
        let (range, decimals) = self.field_range(field);
        PropertyRow::word(name)
            .scrubs()
            .show(ui, |ui, row| {
                at_start(ui, look.width, |ui| {
                    let field = NumberField::new(salt, name, value, self.numbers)
                        .range(range)
                        .decimals(decimals..=decimals)
                        .unit(Unit::new(&self.labels.percent))
                        .policy(look.policy)
                        .part(true)
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
    /// Its click takes its turn as the editor's input ([`Edits::inputs`]).
    fn delete(&self, ui: &mut Ui, row: StopRow, edits: &mut Edits, enabled: bool) {
        let tokens = Tokens::of(ui);
        let label = PropertyRow::style(&tokens, ControlSize::Md);
        let side = IconButton::style(&tokens, ControlSize::Md).side;
        let full = ui.available_rect_before_wrap();
        let indent = (label.word_width + label.label_gap).min((full.width() - side).max(0.0));
        let x = if direction::is_rtl(ui) { full.right() - indent - side } else { full.left() + indent };
        let rect = Rect::from_min_size(pos2(x, full.top()), vec2(side, side));
        let button =
            IconButton::new(Icon::Trash, self.labels.delete.as_str()).id_salt("delete").enabled(enabled && edits.value().len(row) > self.rules.min);
        if ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| ui.add(button)).inner.clicked() {
            edits.input(());
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
