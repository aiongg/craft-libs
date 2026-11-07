//! `NewGuideDialog`: View > Guides > New Guide and Edit Selected Guides.

use super::layout::{GuideLength, GuideUnits};
use super::{
    ColorRequest, GUIDE_LIMIT, GuideColor, GuideColorChoice, GuideWords, Held, HostRequests, NEW_GUIDE_COLOR_KEY, ResetFooter, Shown, accepted,
    edited, run_form,
};
use crate::canvas::{Guide, GuideKey};
use crate::choice::Segmented;
use crate::contract::{Bind, Entry, NoValue, Outcome, Show as _, ShowContainer as _};
use crate::dialog::{Dialog, DialogId, DialogOutput, DialogResult, DialogStack, StackDialog, dialog_id};
use crate::form::{Field, FieldGroup};
use crate::notices::{Notice, NoticeLayout};
use crate::runtime::env::{Env, Label};
use crate::runtime::overlay::CloseReason;
use crate::runtime::words::CommonWords;
use crate::sections::{DialogRole, FooterButton};
use crate::text::NumberField;
use craft_behavior::{Axis, NumberCodec};
use craft_theme::Rgba;
use craft_theme::variant::Tone;
use egui::{Response, Ui};
use std::cell::Cell;
use std::sync::Arc;

/// The values of New Guide.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct NewGuideValues {
    /// The axis the guide runs along: `Axis::X` for a horizontal guide.
    pub along: Axis,
    /// The position in document pixels.
    pub position_px: f64,
    /// The guide's colour.
    pub color: GuideColorChoice,
}

impl NewGuideValues {
    /// A guide running `along` at `position_px` in `color`.
    pub fn new(along: Axis, position_px: f64, color: GuideColor) -> Self {
        NewGuideValues { along, position_px, color: GuideColorChoice::new(color) }
    }

    /// The values that edit `guide`; a guide with no colour of its own is drawn in `preference`.
    pub fn of_guide(guide: &Guide, preference: Rgba) -> Self {
        NewGuideValues::new(guide.along, guide.position, GuideColor::from_rgba(guide.color.unwrap_or(preference)))
    }
}

impl Default for NewGuideValues {
    /// A horizontal guide at 0 in Cyan.
    fn default() -> Self {
        NewGuideValues::new(Axis::X, 0.0, GuideColor::default())
    }
}

/// What New Guide reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct NewGuideOutput {
    /// The dialog's response.
    pub response: Response,
    /// How the dialog closed in this pass; `None` while it stays open.
    pub closed: Option<CloseReason>,
    /// The guide, the pass the dialog closed as OK: the edited guide with the new values, or a new guide with the
    /// pattern's key.
    pub result: Option<Guide>,
    /// The Custom colour was asked for: the app opens its colour picker and answers into the values.
    pub pick_color: Option<ColorRequest>,
}

impl Outcome for NewGuideOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// New Guide, and Edit Selected Guides when it edits a guide: an orientation, a position with a unit and a colour.
///
/// - Keys: Tab walks the fields, Enter is OK, Escape cancels. Holding Alt turns Cancel into Reset, which restores the
///   values from when the dialog opened and keeps the dialog open.
/// - Position: a number field in the ruler unit over the document's width (vertical guides) or height (horizontal),
///   so "50%", "2 cm" and "12 pt" work; text that is not a length is refused with the field's message.
/// - Colour: Photoshop's nine colours or Custom, which asks the app for a colour (`pick_color`).
/// - Edit: the edited guide keeps its other settings; a guide with no colour of its own keeps using the app's
///   preference unless the colour is changed here.
/// - Values: the `values` binding holds them; the app keeps the last guide's for the next open.
#[must_use = "show the dialog with `show`"]
#[derive(crate::contract::Pattern, crate::contract::Props)]
pub struct NewGuideDialog<'a> {
    units: GuideUnits,
    values: Held<'a, NewGuideValues>,
    edit: Option<(Guide, Rgba)>,
    /// The key of the guide a new guide makes.
    #[prop]
    key: GuideKey,
    /// The app's message when it refused the position: shown above the fields.
    #[prop]
    error: Option<Label>,
    /// Binds the open state to `open`.
    #[prop(name = "opened")]
    open: Option<Bind<'a, bool>>,
}

impl<'a> NewGuideDialog<'a> {
    /// New Guide over `values`, for a document measured in `units`.
    pub fn new(units: GuideUnits, values: Bind<'a, NewGuideValues>) -> Self {
        NewGuideDialog { units, values: Held::new(values), edit: None, key: GuideKey::new(0), error: None, open: None }
    }

    /// Edit Selected Guides for `guide`, which is drawn in `preference` when it has no colour of its own: the
    /// `values` start as [`NewGuideValues::of_guide`].
    pub fn edit(units: GuideUnits, guide: Guide, preference: Rgba, values: Bind<'a, NewGuideValues>) -> Self {
        NewGuideDialog { edit: Some((guide, preference)), ..NewGuideDialog::new(units, values) }
    }

    /// Shows the dialog.
    pub fn show(self, ui: &mut Ui) -> NewGuideOutput {
        let NewGuideDialog { units, values, edit, key, error, open } = self;
        let env = Env::of(ui);
        let words = env.words::<GuideWords>();
        let common = env.words::<CommonWords>();
        let number_units = env.units();
        let title = if edit.is_some() { words.edit_title.clone() } else { words.new_guide_title.clone() };
        let reset = ResetFooter::new(ui, None, true);
        let accept = FooterButton::new(common.ok.clone(), DialogRole::Accept);
        let mut dialog = Dialog::new(title.clone()).close_on_enter(true).outer_gesture(true).buttons(reset.buttons(accept, false));
        if let Some(open) = open {
            dialog = dialog.opened(open);
        }
        let orientations = [Entry::new(Axis::X, words.horizontal.clone()), Entry::new(Axis::Y, words.vertical.clone())];
        let fallback = edit.as_ref().map_or_else(NewGuideValues::default, |(guide, preference)| NewGuideValues::of_guide(guide, *preference));
        let asks = Cell::new(false);
        let shown = run_form(ui, dialog, values, fallback, Arc::from(title.as_str()), &reset, |ui, _, values| {
            if let Some(message) = &error {
                Notice::new(Tone::Bad, message.clone()).layout(NoticeLayout::Inline).show(ui);
            }
            FieldGroup::new(title.clone()).show(ui, |ui, _| {
                Field::new(words.orientation.clone()).show(ui, |ui, _| {
                    let along = values.along;
                    if let Some(picked) = edited(&Segmented::new(words.orientation.clone(), Bind::given(along), &orientations).show(ui)) {
                        values.along = picked;
                    }
                });
                position_row(ui, &words, &number_units, &units, values);
                if values.color.row(ui, &words) {
                    asks.set(true);
                }
            });
            true
        });
        let Shown { output, values } = shown;
        let DialogOutput { response, closed, .. } = output;
        let result = accepted(closed).then(|| guide_of(&values, edit.as_ref(), key));
        let pick_color = asks.get().then(|| values.color.request(NEW_GUIDE_COLOR_KEY, words.picker_title.clone()));
        NewGuideOutput { response, closed, result, pick_color }
    }
}

/// The guide the values make: the edited guide with the values applied, or a new guide with `key`. An edited guide
/// with no colour of its own stays so while the colour is still the preference it opened with.
fn guide_of(values: &NewGuideValues, edit: Option<&(Guide, Rgba)>, key: GuideKey) -> Guide {
    let (base, keeps_preference) = match edit {
        Some((guide, preference)) => (guide.clone(), guide.color.is_none() && values.color.color == GuideColor::from_rgba(*preference)),
        None => (Guide::new(key, values.along, values.position_px), false),
    };
    let color = if keeps_preference { None } else { Some(values.color.color.rgba()) };
    Guide { along: values.along, position: values.position_px, color, ..base }
}

fn position_row(ui: &mut Ui, words: &GuideWords, numbers: &crate::runtime::env::Units, units: &GuideUnits, values: &mut NewGuideValues) {
    let ruler = units.ruler;
    let extent = if values.along == Axis::Y { units.width_px } else { units.height_px };
    let shown = units.from_pixels(values.position_px, ruler, extent).unwrap_or(0.0);
    let high = units.from_pixels(GUIDE_LIMIT, ruler, extent).filter(|v| v.is_finite()).unwrap_or(GUIDE_LIMIT).abs();
    let places = *ruler.decimals().end();
    Field::new(words.position.clone()).show(ui, |ui, _| {
        let codec = NumberCodec::new(units.reader(numbers, extent), places);
        let field = NumberField::new(words.position.clone(), Bind::given(Some(shown)))
            .codec(codec)
            .range(-high..=high)
            .step(ruler.step())
            .unit(numbers.symbol(ruler));
        if let Some(Some(typed)) = edited(&field.show(ui).edit) {
            let scale = 10f64.powi(i32::try_from(places).unwrap_or(0));
            let changed = (typed * scale).round() != (shown * scale).round();
            if let (true, Some(px)) = (changed, units.to_pixels(GuideLength::new(typed, ruler), extent)) {
                values.position_px = px.clamp(-GUIDE_LIMIT, GUIDE_LIMIT);
            }
        }
    });
}

/// A hosted New Guide: the `DialogHost` owns the values, and the app's colour answer goes into them.
pub struct HostedNewGuideDialog {
    id: DialogId,
    units: GuideUnits,
    edit: Option<(Guide, Rgba)>,
    key: GuideKey,
    error: Option<Label>,
    values: NewGuideValues,
    requests: HostRequests,
}

impl HostedNewGuideDialog {
    /// New Guide for a document measured in `units`, opening with `values`; the guide made has `key`.
    pub fn new(units: GuideUnits, key: GuideKey, values: NewGuideValues) -> Self {
        HostedNewGuideDialog { id: dialog_id("guides.newGuide"), units, edit: None, key, error: None, values, requests: HostRequests::new() }
    }

    /// Edit Selected Guides for `guide`, whose colour defaults to `preference`.
    pub fn edit(units: GuideUnits, guide: Guide, preference: Rgba) -> Self {
        let values = NewGuideValues::of_guide(&guide, preference);
        HostedNewGuideDialog { edit: Some((guide, preference)), ..HostedNewGuideDialog::new(units, GuideKey::new(0), values) }
    }

    /// The app's message for a position it refused: the dialog reopens with it above the fields.
    #[must_use]
    pub fn error(mut self, message: impl Into<Label>) -> Self {
        self.error = Some(message.into());
        self
    }

    /// The values the dialog shows.
    pub fn values(&self) -> &NewGuideValues {
        &self.values
    }

    /// The colour request, once.
    pub fn pick_color(&mut self) -> Option<ColorRequest> {
        self.requests.take_color()
    }

    /// The colour picker's answer.
    pub fn set_color(&mut self, key: &str, color: craft_theme::Srgba) {
        if key == NEW_GUIDE_COLOR_KEY {
            self.values.color.answer(color.to_rgba());
        }
    }
}

impl StackDialog for HostedNewGuideDialog {
    type Result = Guide;

    fn id(&self) -> DialogId {
        self.id
    }

    fn show(&mut self, ui: &mut Ui, _: &DialogStack) -> Option<DialogResult<Guide>> {
        let bound = Bind::bound(&mut self.values);
        let mut dialog = match &self.edit {
            Some((guide, preference)) => NewGuideDialog::edit(self.units, guide.clone(), *preference, bound),
            None => NewGuideDialog::new(self.units, bound).key(self.key),
        };
        if let Some(error) = &self.error {
            dialog = dialog.error(error.clone());
        }
        let shown = dialog.show(ui);
        if let Some(request) = shown.pick_color {
            self.requests.request_color(request);
        }
        match (shown.result, shown.closed) {
            (Some(guide), _) => Some(DialogResult::Ok(guide)),
            (None, Some(_)) => Some(DialogResult::Cancel),
            (None, None) => None,
        }
    }

    fn key(&self) -> Option<&str> {
        Some("guides.newGuide")
    }
}
