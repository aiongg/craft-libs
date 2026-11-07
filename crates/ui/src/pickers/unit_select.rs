//! `UnitSelect`: a number field with a select of units beside it that picks the unit the field reads and writes.

use super::select::Select;
use crate::contract::{
    Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, Entry, MeasureCx, Outcome, PaintUi, Props as _, Role, WordSetCx as _,
};
use crate::runtime::env::{Label, UnitKind};
use crate::style::{ComponentStyle, StyleCx};
use crate::text::NumberField;
use craft_behavior::{EditEvent, Flow, Metrics, Mixed, Proposal, Sense};
use egui::{Response, Vec2};
use std::cell::Cell;
use std::ops::{Deref, RangeInclusive};

#[cfg(feature = "testing")]
mod stories;

/// The style of a unit select: the gap between the field and the select.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct UnitSelectStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// The output of a unit select: the number's edit, and the unit's edit when a unit was picked.
#[derive(Debug)]
#[non_exhaustive]
pub struct UnitSelectOutput {
    /// The edit of the number.
    pub edit: EditResponse<Option<f64>>,
    /// The unit the user picked this pass.
    pub unit: Option<EditEvent<UnitKind>>,
}

impl Deref for UnitSelectOutput {
    type Target = EditResponse<Option<f64>>;

    fn deref(&self) -> &Self::Target {
        &self.edit
    }
}

impl Outcome for UnitSelectOutput {
    type Value = Option<f64>;

    fn response(&self) -> &Response {
        &self.edit.response
    }

    #[cfg(feature = "testing")]
    fn story_facts(&self) -> crate::testing::StoryOutput {
        self.edit.story_facts()
    }
}

/// A number field with a select of units after it. The select picks the unit kind the field reads typed text in and
/// shows its number with; the number itself is the field's value, and the unit goes to its own binding, so the app
/// converts the number when the unit changes.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component]
pub struct UnitSelect<'a> {
    #[common]
    common: Common,
    value: Bind<'a, Option<f64>>,
    unit: Bind<'a, UnitKind>,
    units: &'a [Entry<UnitKind>],
    /// The range typed, stepped and scrubbed numbers stay in.
    #[prop]
    range: Option<RangeInclusive<f64>>,
    /// The unit select's accessible name.
    #[prop(into)]
    unit_label: Option<Label>,
    /// The unit child's event of this pass, kept from `paint` to `finish`.
    unit_event: Option<EditEvent<UnitKind>>,
    /// The width the select took when measured, which `paint` leaves it.
    unit_width: Cell<f32>,
}

impl<'a> UnitSelect<'a> {
    /// A number field named `label` over `value`, with a select of `units` over `unit` after it.
    pub fn new(label: impl Into<Label>, value: Bind<'a, Option<f64>>, unit: Bind<'a, UnitKind>, units: &'a [Entry<UnitKind>]) -> Self {
        UnitSelect { common: Common::new(), value, unit, units, range: None, unit_label: None, unit_event: None, unit_width: Cell::new(0.0) }
            .label(label)
    }

    fn number(&self, name: Label, value: Mixed<Option<f64>>, kind: Option<UnitKind>) -> NumberField<'static> {
        let field = NumberField::new(name, Bind::given(value));
        let field = match &self.range {
            Some(range) => field.range(range.clone()),
            None => field,
        };
        match kind {
            Some(kind) => field.unit_kind(kind),
            None => field,
        }
    }

    fn select_name(&self, group: &Label) -> Label {
        self.unit_label.clone().unwrap_or_else(|| group.clone())
    }
}

/// What the number field shows: the value, or mixed when the component's is.
fn shown_of(value: Option<&Option<f64>>) -> Mixed<Option<f64>> {
    value.map_or(Mixed::Mixed, |value| Mixed::Same(*value))
}

impl Component for UnitSelect<'_> {
    const ROLE: Role = Role::GROUP;
    const PARTS: bool = false;
    const STOPS: bool = false;
    const OVERLAYS: bool = false;
    type Style = UnitSelectStyle;
    type Output = UnitSelectOutput;

    fn style(cx: &StyleCx<'_>, _: UnitSelectStyleKey) -> UnitSelectStyle {
        UnitSelectStyle { metrics: Metrics::new().gap(cx.control().gap) }
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let group = cx.label();
        let mut number = self.number(group.clone(), shown_of(cx.value()), None);
        let mut select = Select::new(self.select_name(&group), Bind::<UnitKind>::given(Mixed::Mixed), self.units);
        let number = cx.child("number", &mut number, Proposal::Ideal);
        let select = cx.child("unit", &mut select, Proposal::Ideal);
        self.unit_width.set(select.x);
        Vec2::new(number.x + cx.gap() + select.x, number.y.max(select.y))
    }

    fn bind(&mut self) -> Option<BindMut<'_, Option<f64>>> {
        Some(self.value.reborrow())
    }

    /// The select is shown first, so the number follows the unit it shows after the pass; the number's edit is
    /// adopted as the component's own.
    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let group = cx.label();
        let flow = Flow::new(cx.rect, cx.flow.is_rtl());
        let (select_rect, rest) = flow.take_end(self.unit_width.get());
        let number_rect = rest.take_end(cx.style.metrics.gap).1.rect();
        let select = Select::new(self.select_name(&group), self.unit.reborrowed(), self.units);
        let unit = ui.child("unit", select_rect, select);
        let kind = match unit.shown.as_ref() {
            Some(Mixed::Same(kind)) => Some(**kind),
            _ => None,
        };
        self.unit_event = unit.event;
        let shown = shown_of(cx.value());
        let start = cx.value().and_then(|value| *value);
        let number = ui.child("number", number_rect, self.number(group, shown, kind));
        cx.adopt(&number, &start, |value, mine| *mine = *value);
    }

    fn finish(self, cx: Cx<'_, Self>) -> UnitSelectOutput {
        UnitSelectOutput { edit: cx.edit_response(), unit: self.unit_event }
    }
}
