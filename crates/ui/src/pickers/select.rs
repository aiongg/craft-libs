//! `Select`: a closed field that opens a listbox of entries and feeds the picked one to its binding (15.4, 15.5).
//!
//! The field is a leaf that shows a `SelectFace`; its listbox is the component's own popup, shown from the `overlay`
//! hook on the dropdown preset. Focus stays on the field: the host hands the list keys to the listbox while it is
//! open, and Escape closes it without a change.

use super::face::SelectFace;
use super::words::PickerWords;
use crate::contract::{
    Access, Bind, BindMut, CollectionEvent, Common, Component, Cx, Edit as _, EditResponse, Entry, Focus as _, MeasureCx, Numeric, Overlay,
    OverlayUi, Overlays as _, PaintUi, PartsCx, Pointer as _, Props as _, Remember as _, Role, Show as _, ShowCollection as _, WordSetCx as _,
};
use crate::list::{List, ListRow};
use crate::runtime::env::Label;
use crate::runtime::focus::FocusPolicy;
use crate::style::{ComponentStyle, Ink, StyleCx, SurfaceLook, TextLook, TypeRole};
use crate::text::TextField;
use craft_behavior::nav::{StepResult, ValueAxes, ValueSteps};
use craft_behavior::{DisplayOrder, HitLevel, HitPriority, ItemSource, Metrics, NavMove, PartKey, Selection, Sense, Value, wheel_step};
use craft_theme::{Rgba, Stateful, States};
use egui::Vec2;
use std::borrow::Cow;
use std::hash::{DefaultHasher, Hash as _, Hasher as _};

#[cfg(feature = "testing")]
mod stories;

/// The part that shows the face over the whole field and opens the list on a click.
const FACE: PartKey = PartKey::new("face");

/// The style of a select: the field well and focus edge, which the face is shown over, and the text ink the face
/// reads as its host.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct SelectStyle {
    #[style(surface)]
    surface: SurfaceLook,
    /// No padding of its own: the face carries the field's.
    #[style(metrics)]
    metrics: Metrics,
    /// The face's text.
    #[style(text)]
    text: TextLook,
}

/// The fill of a row of a list: clear at rest, tinted under the pointer, while pressed and while selected.
pub(crate) fn row_look(cx: &StyleCx<'_>) -> SurfaceLook {
    let state = &cx.tokens.color.state;
    let clear = cx.color(Rgba::TRANSPARENT);
    let selected = cx.color(state.selected);
    let selected_hover = cx.color(state.selected_hover);
    let fill = Stateful::new(clear).off(States::new(clear, cx.color(state.hover), cx.color(state.pressed), clear)).on(States::new(
        selected,
        selected_hover,
        selected_hover,
        selected,
    ));
    SurfaceLook::new(fill)
}

/// What a select adds to an option's row: the host's content for an entry (a detail line, a trailing shortcut text, a
/// thumbnail painter). `Select::decorate` takes the closure.
pub struct RowDress<'a, T>(Box<DressFn<'a, T>>);

type DressFn<'a, T> = dyn Fn(&T, ListRow<'a>) -> ListRow<'a> + 'a;

impl<'a, T> RowDress<'a, T> {
    /// The dress that runs `dress` over the row of each entry, given the entry's value.
    pub fn new(dress: impl Fn(&T, ListRow<'a>) -> ListRow<'a> + 'a) -> Self {
        RowDress(Box::new(dress))
    }

    fn apply(&self, value: &T, row: ListRow<'a>) -> ListRow<'a> {
        (self.0)(value, row)
    }
}

/// A field that shows the picked entry and opens a listbox of all of them. A click, Enter, Space or Alt+Down opens the
/// listbox with the picked entry active; Up, Down, Home, End and type-ahead move through it, Enter or a click picks,
/// Escape and Alt+Up close it. Closed, the arrow keys, Home, End and the wheel (while the field is focused or armed)
/// step the pick through the enabled entries. The field is as wide as its widest entry, so it keeps its width whichever
/// entry is picked.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(overlay, probe(Select<'static, u8>))]
pub struct Select<'a, T: Value + 'static> {
    #[common]
    common: Common,
    value: Bind<'a, T>,
    entries: &'a [Entry<T>],
    /// A filter field above the entries narrows them as the user types.
    #[prop(flag)]
    searchable: bool,
    /// Shown, muted, while the value is none of the entries.
    #[prop(into)]
    placeholder: Option<Label>,
    /// Adds the host's content to each entry's row.
    #[prop(func = "Fn(&T, ListRow<'a>) -> ListRow<'a>")]
    decorate: Option<RowDress<'a, T>>,
}

impl<'a, T: Value + 'static> Select<'a, T> {
    /// A select named `label` over `value`, choosing among `entries`.
    pub fn new(label: impl Into<Label>, value: Bind<'a, T>, entries: &'a [Entry<T>]) -> Self {
        Select { common: Common::new(), value, entries, searchable: false, placeholder: None, decorate: None }.label(label)
    }

    fn position(&self, value: Option<&T>) -> Option<usize> {
        value.and_then(|value| self.entries.iter().position(|entry| entry.value() == Some(value)))
    }

    /// Sets the value to the entry at `at` unless it is the value already.
    fn pick(&self, cx: &mut Cx<'_, Self>, at: usize) {
        if let Some(value) = self.entries[at].value()
            && self.position(cx.value()) != Some(at)
        {
            cx.set(value.clone());
        }
    }

    /// The positions of the entries that can be picked, in order.
    fn picks(&self) -> Vec<usize> {
        (0..self.entries.len()).filter(|&at| self.entries[at].is_enabled()).collect()
    }
}

/// The pick a move reaches: `picks` are the positions of the enabled entries and `from` the position of the current
/// value. From a value that is none of the picks, an arrow lands on the first.
fn step(picks: &[usize], from: Option<usize>, mv: NavMove) -> Option<usize> {
    let (first, last) = (*picks.first()?, *picks.last()?);
    let at = from.and_then(|from| picks.iter().position(|&pick| pick == from));
    match mv {
        NavMove::Up | NavMove::Back => Some(at.map_or(first, |at| picks[at.saturating_sub(1)])),
        NavMove::Down | NavMove::Forward => Some(at.map_or(first, |at| picks[(at + 1).min(picks.len() - 1)])),
        NavMove::Start => Some(first),
        NavMove::End => Some(last),
        _ => None,
    }
}

impl<T: Value + 'static> Component for Select<'_, T> {
    const ROLE: Role = Role::COMBO_BOX;
    type Style = SelectStyle;
    type Output = EditResponse<T>;

    fn style(cx: &StyleCx<'_>, _: SelectStyleKey<T>) -> SelectStyle {
        SelectStyle { surface: cx.field(), metrics: Metrics::new(), text: TextLook::new(TypeRole::Body, Stateful::new(cx.ink(Ink::OnPanel))) }
    }

    /// The widest entry's face, so the field keeps its size whichever entry is picked.
    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let widest = self
            .entries
            .iter()
            .filter(|entry| !entry.is_separator())
            .map(|entry| cx.face(SelectFace::new(cx.env.resolve(entry.label()))))
            .fold(Vec2::ZERO, Vec2::max);
        if widest == Vec2::ZERO { cx.face(SelectFace::new(Label::Empty)) } else { widest }
    }

    fn bind(&mut self) -> Option<BindMut<'_, T>> {
        Some(self.value.reborrow())
    }

    fn parts(&mut self, rect: egui::Rect, cx: &mut PartsCx<'_, Self>) {
        cx.part(FACE, rect)
            .sense(Sense::CLICK | Sense::HOVER)
            .priority(HitPriority::new(HitLevel::Body, 0))
            .pointer_only("Enter, Space and Alt+Down open the list")
            .toggles("list");
    }

    /// Enter and Space toggle the listbox. While it is closed, the arrow keys, Home, End and the wheel step the pick
    /// through the enabled entries, and AccessKit sets it.
    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let list = cx.popup("list");
        if cx.activation().is_some() {
            if list.is_open(cx) {
                list.close(cx);
            } else {
                list.open(cx);
            }
        }
        let picks = self.picks();
        if list.is_open(cx) || picks.is_empty() {
            return;
        }
        let held = self.position(cx.value());
        if let Some(at) = cx.intents().take_set_value() {
            let to = (at.round().max(0.0) as usize).min(picks.len() - 1);
            self.pick(cx, picks[to]);
        }
        while let Some((mv, _)) = cx.intents().take_move() {
            if let Some(to) = step(&picks, self.position(cx.value()), mv) {
                self.pick(cx, to);
            }
        }
        if let Some(wheel) = cx.take_wheel() {
            let at = held.and_then(|held| picks.iter().position(|&pick| pick == held)).unwrap_or(0);
            let steps = ValueSteps::new(at as f64, 0.0, (picks.len() - 1) as f64, 1.0, 1.0);
            if let StepResult::Set(next) = wheel_step(&steps, ValueAxes::Both, &wheel, cx.env.direction().is_rtl()) {
                self.pick(cx, picks[(next.round().max(0.0) as usize).min(picks.len() - 1)]);
            }
        }
    }

    /// The picked entry's place among the entries that can be picked, shown as its label, so AccessKit steps and sets it.
    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let picks = self.picks();
        let range = 0.0..=picks.len().saturating_sub(1) as f64;
        let held = self.position(cx.value());
        let numeric = match held.and_then(|held| picks.iter().position(|&pick| pick == held)) {
            Some(at) => Numeric::new(at as f64, range).text(cx.env.resolve(self.entries[picks[at]].label()).as_str()),
            None => Numeric::empty(range),
        };
        access.numeric(numeric.step(1.0));
    }

    /// The listbox: one option row per entry, a spacer for a separator, the picked entry's row selected. A pick comes
    /// back from the list as Enter or a click on a row, and goes to the binding as one change.
    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        let popup = cx.popup("list");
        let entries = self.entries;
        let labels: Vec<Label> = entries.iter().map(|entry| cx.env.resolve(entry.label())).collect();
        let name = cx.label();
        let words = cx.words::<PickerWords>();
        let held = self.position(cx.value());
        let (searchable, dress) = (self.searchable, self.decorate.as_ref());
        // A filter field needs the key focus, so its popup takes focus in; the others leave it on the field.
        let overlay = if searchable { Overlay::dropdown().focus(FocusPolicy::POPUP) } else { Overlay::dropdown() };
        let events = ui.show(popup, overlay, |ui, body| {
            let query = if searchable {
                let query = body.memory::<Query>();
                TextField::search(search_name(&words, &name), Bind::bound(&mut query.0)).show(ui);
                query.0.trim().to_lowercase()
            } else {
                String::new()
            };
            let shown = matching(entries, &labels, &query);
            let rows = Rows { entries, labels: &labels, shown: &shown };
            let selection = held.filter(|at| shown.contains(at)).map_or_else(Selection::default, |at| Selection::new(vec![at], Some(at), Some(at)));
            let stamp = {
                let mut hasher = DefaultHasher::new();
                query.hash(&mut hasher);
                hasher.finish()
            };
            List::options(name.clone())
                .empty(words.no_matches.clone())
                .rows_stamp(stamp)
                .selection(Bind::given(selection))
                .show_items(ui, &rows, &mut |at, _| {
                    let entry = &entries[*at];
                    let Some(value) = entry.value() else {
                        return ListRow::spacer();
                    };
                    let row = ListRow::new(labels[*at].clone()).option().enabled(entry.is_enabled());
                    let row = match entry.icon_spec() {
                        Some(icon) => row.icon(icon.clone()),
                        None => row,
                    };
                    match dress {
                        Some(dress) => dress.apply(value, row),
                        None => row,
                    }
                })
                .events
        });
        let picked = events.into_iter().flatten().find_map(|event| match event {
            CollectionEvent::Activate(at) => Some(at),
            _ => None,
        });
        if let Some(value) = picked.and_then(|at| entries.get(at)).and_then(Entry::value) {
            cx.set(value.clone());
            popup.close(cx);
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let (text, muted) = match self.position(cx.value()) {
            Some(at) => (cx.env.resolve(self.entries[at].label()), false),
            None if cx.value().is_none() => (cx.env.resolve(&cx.words::<PickerWords>().mixed.clone()), true),
            None => (self.placeholder.as_ref().map_or(Label::Empty, |text| cx.env.resolve(text)), true),
        };
        let face = SelectFace::new(text);
        ui.face(FACE, if muted { face.placeholder() } else { face });
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<T> {
        cx.edit_response()
    }
}

/// The filter text of a searchable select's open list.
#[derive(Clone, Default)]
struct Query(String);

/// The filter field's accessible name: the select's own name leads, so the field reads as part of it.
fn search_name(words: &PickerWords, select: &Label) -> Label {
    if select.as_str().is_empty() { words.search.clone() } else { Label::from(format!("{} {}", select.as_str(), words.search.as_str())) }
}

/// The positions of the entries whose label holds `query` (lowercase), in order; every entry, separators included,
/// while the query is empty.
fn matching<T>(entries: &[Entry<T>], labels: &[Label], query: &str) -> Vec<usize> {
    if query.is_empty() {
        return (0..entries.len()).collect();
    }
    (0..entries.len()).filter(|&at| !entries[at].is_separator() && labels[at].as_str().to_lowercase().contains(query)).collect()
}

/// The entries that match the filter as a collection source, keyed by entry position: the entries are fixed while the
/// list is open.
struct Rows<'a, T> {
    entries: &'a [Entry<T>],
    labels: &'a [Label],
    shown: &'a [usize],
}

impl<T> DisplayOrder<usize> for Rows<'_, T> {
    fn len(&self) -> usize {
        self.shown.len()
    }

    fn key(&self, at: usize) -> usize {
        self.shown[at]
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        self.shown.iter().position(|shown| shown == key)
    }

    fn is_section(&self, at: usize) -> bool {
        self.entries[self.shown[at]].is_separator()
    }

    fn enabled(&self, at: usize) -> bool {
        self.entries[self.shown[at]].is_enabled()
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        let key = self.shown[at];
        (!self.entries[key].is_separator()).then(|| Cow::Borrowed(self.labels[key].as_str()))
    }
}

impl<T> ItemSource<usize> for Rows<'_, T> {}
