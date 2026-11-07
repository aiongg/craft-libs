//! `Select`: a closed field that opens a listbox of options and feeds the picked one to its binding (15.4, 15.5).
//!
//! The field is a leaf; its listbox is the component's own popup, shown from the `overlay` hook with the dropdown
//! preset. Focus moves into the listbox when it opens and returns to the field when it closes, so the list's own
//! keys (arrows, Home, End, type-ahead, Enter) drive the choice and Escape closes without a change.

use super::disclosure::DisclosureGlyph;
use super::words::ControlWords;
use crate::contract::{
    ActivateOn, Bind, BindMut, Collection, CollectionEvent, CollectionProps, CollectionResponse, Common, Component, ContainerProps, Cx, Edit as _,
    EditResponse, Focus as _, LeadOnOpen, MeasureCx, Overlay, OverlayUi, Overlays as _, PaintUi, Props as _, Remember as _, Role, SelectMode,
    ShowCollection as _, Sizing, Trigger, WordSetCx as _,
};
use crate::primitives::{PaintShape, StrokeWidth, TextRun};
use crate::runtime::env::Label;
use crate::runtime::focus::{FocusPolicy, Initial};
use crate::style::{ComponentStyle, IconLook, Ink, StyleCx, SurfaceLook, TextLook, TypeRole};
use craft_behavior::{DisplayOrder, Flow, ItemSource, Metrics, NavMove, Selection, SpanLayout, Value};
use craft_theme::{Rgba, Stateful, States};
use egui::{Response, Vec2};

#[cfg(feature = "testing")]
mod stories;

/// One choice of a `Select`: the value it stands for and the label it shows.
#[non_exhaustive]
pub struct SelectOption<T> {
    /// The value the binding holds while this option is picked.
    pub value: T,
    /// The text the option shows.
    pub label: Label,
}

impl<T> SelectOption<T> {
    /// An option standing for `value`, shown as `label`.
    pub fn new(value: T, label: impl Into<Label>) -> Self {
        SelectOption { value, label: label.into() }
    }
}

/// The style of a select: the field's look, the control metrics and the chevron.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct SelectStyle {
    #[style(surface)]
    surface: SurfaceLook,
    #[style(metrics)]
    metrics: Metrics,
    /// The chevron.
    #[style(icon)]
    chevron: IconLook,
}

/// The option last highlighted in the open listbox and how often the listbox has opened, remembered while the
/// select shows. Each opening gives the list a new id, so its lead starts on the picked option.
#[derive(Clone, Copy, Default)]
struct Highlight {
    at: Option<usize>,
    opened: u32,
}

/// A field that shows the picked option and opens a listbox of all of them.
#[derive(crate::contract::Component)]
#[component(overlay, probe(Select<'static, u8>))]
pub struct Select<'a, T: Value + 'static> {
    #[common]
    common: Common,
    value: Bind<'a, T>,
    options: &'a [SelectOption<T>],
}

impl<'a, T: Value + 'static> Select<'a, T> {
    /// A select named `label` over `value`, choosing among `options`.
    pub fn new(label: impl Into<Label>, value: Bind<'a, T>, options: &'a [SelectOption<T>]) -> Self {
        Select { common: Common::new(), value, options }.label(label)
    }

    fn position(&self, value: Option<&T>) -> Option<usize> {
        value.and_then(|value| self.options.iter().position(|option| option.value == *value))
    }
}

impl<T: Value + 'static> Component for Select<'_, T> {
    const ROLE: Role = Role::COMBO_BOX;
    type Style = SelectStyle;
    type Output = EditResponse<T>;

    fn style(cx: &StyleCx<'_>, _: SelectStyleKey<T>) -> SelectStyle {
        let control = cx.control();
        SelectStyle { surface: cx.field(), metrics: control.metrics(), chevron: IconLook::new(control.icon, Stateful::new(cx.ink(Ink::OnPanel))) }
    }

    /// The widest option, so the field keeps its size whichever option is picked.
    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let widest = self.options.iter().map(|option| cx.layout(&TextRun::new(option.label.as_str())).size()).fold(Vec2::ZERO, Vec2::max);
        Vec2::new(widest.x + cx.gap() + cx.icon(), widest.y)
    }

    fn bind(&mut self) -> Option<BindMut<'_, T>> {
        Some(self.value.reborrow())
    }

    /// A click, Enter or Space toggles the listbox and Alt+Down opens it. While it is closed, the arrow keys, Home
    /// and End step through the options.
    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let list = cx.popup("list");
        let was_open = list.is_open(cx);
        list.trigger(Trigger::Click, cx);
        if list.is_open(cx) && !was_open {
            let held = cx.memory::<Highlight>();
            *held = Highlight { at: None, opened: held.opened + 1 };
        }
        if list.is_open(cx) || self.options.is_empty() {
            return;
        }
        let last = self.options.len() - 1;
        while let Some((step, _)) = cx.intents().take_move() {
            let at = self.position(cx.value());
            let to = match step {
                NavMove::Up | NavMove::Back => at.map_or(0, |at| at.saturating_sub(1)),
                NavMove::Down | NavMove::Forward => at.map_or(0, |at| (at + 1).min(last)),
                NavMove::Start => 0,
                NavMove::End => last,
                _ => continue,
            };
            if Some(to) != at {
                cx.set(self.options[to].value.clone());
            }
        }
    }

    /// The listbox: one row per option, the highlighted row selected. A pick comes back from the list as `Activate`
    /// (Enter) or a click on a row, and goes to the binding as one change.
    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        let popup = cx.popup("list");
        let labels: Vec<Label> = self.options.iter().map(|option| cx.env.resolve(&option.label)).collect();
        let Highlight { at, opened } = cx.recall::<Highlight>();
        let name = cx.label();
        let held = at.or_else(|| self.position(cx.value()));
        let overlay = Overlay::dropdown().focus(FocusPolicy::POPUP.initial(Initial::First));
        let events = ui.show(popup, overlay, |ui, _| {
            let selection = held.map_or_else(Selection::default, |at| Selection::new(vec![at], Some(at), Some(at)));
            let choices = Choices { labels: &labels };
            OptionList::new(Bind::given(selection))
                .lead_on_open(LeadOnOpen::First)
                .lead_follows_hover(true)
                .id_salt(opened)
                .label(name.clone())
                .show_items(ui, &choices, &mut |key, _| OptionRow::new(labels[*key].as_str()))
                .events
        });
        let mut picked = None;
        for event in events.into_iter().flatten() {
            match event {
                CollectionEvent::LeadMoved(at) => cx.memory::<Highlight>().at = Some(at),
                CollectionEvent::Activate(at) => picked = Some(at),
                _ => {}
            }
        }
        if let Some(at) = picked {
            cx.set(self.options[at].value.clone());
            popup.close(cx);
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let rtl = cx.env.direction().is_rtl();
        let (chevron, rest) = Flow::new(cx.content_rect, rtl).take_end(cx.style.chevron.side);
        let (_, text) = rest.take_end(cx.style.metrics.gap);
        let label = match self.position(cx.value()) {
            Some(at) => cx.env.resolve(&self.options[at].label),
            None if cx.value().is_none() => cx.env.resolve(&cx.words::<ControlWords>().mixed.clone()),
            None => Label::Empty,
        };
        let draw = ui.draw();
        draw.text(text.rect(), &TextRun::new(label.as_str()).elide());
        draw.stroke(
            &PaintShape::Path(DisclosureGlyph::new(1.0, cx.env.direction()).path(chevron)),
            StrokeWidth::Medium,
            cx.resolve(&cx.style.chevron.color),
        );
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<T> {
        cx.edit_response()
    }
}

/// The labels of a select's options as a collection source, keyed by position: the options are fixed while the
/// listbox is open.
struct Choices<'a> {
    labels: &'a [Label],
}

impl DisplayOrder<usize> for Choices<'_> {
    fn len(&self) -> usize {
        self.labels.len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.labels.len()).then_some(*key)
    }

    fn text(&self, at: usize) -> Option<std::borrow::Cow<'_, str>> {
        Some(std::borrow::Cow::Borrowed(self.labels[at].as_str()))
    }
}

impl ItemSource<usize> for Choices<'_> {}

/// The style of the option list: it has no look of its own.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct OptionListStyle {
    #[style(metrics)]
    metrics: Metrics,
}

// F8-MERGE: F8a. `OptionList` and `OptionRow` stand in for `list::List` and `list::ListRow` until the lists family
// completes them; `Select` then shows `List` and these two types go.
/// A single-selection listbox of option rows.
#[derive(crate::contract::Component)]
#[component(collection)]
pub struct OptionList<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    items: CollectionProps,
    selection: Bind<'a, Selection<usize>>,
}

impl<'a> OptionList<'a> {
    /// A listbox whose selected row is `selection`.
    pub fn new(selection: Bind<'a, Selection<usize>>) -> Self {
        OptionList {
            common: Common::new(),
            props: ContainerProps::new(),
            items: CollectionProps::new().select(SelectMode::Single).activate(ActivateOn::Click),
            selection,
        }
    }

    /// Where the lead starts.
    #[must_use]
    pub fn lead_on_open(mut self, at: LeadOnOpen) -> Self {
        self.items = self.items.lead_on_open(at);
        self
    }

    /// The lead follows the pointer over the rows.
    #[must_use]
    pub fn lead_follows_hover(mut self, on: bool) -> Self {
        self.items = self.items.lead_follows_hover(on);
        self
    }

    /// How the list sizes: by its rows (the default), or to fill the room it is given.
    #[must_use]
    pub fn sizing(mut self, sizing: Sizing) -> Self {
        self.props = self.props.sizing(sizing);
        self
    }
}

impl Component for OptionList<'_> {
    const ROLE: Role = Role::LIST_BOX;
    type Style = OptionListStyle;
    type Output = CollectionResponse<usize, ()>;

    fn style(_: &StyleCx<'_>, _: OptionListStyleKey) -> OptionListStyle {
        OptionListStyle::default()
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> Self::Output {
        cx.collection_response()
    }
}

impl Collection for OptionList<'_> {
    type Key = usize;
    type ItemLayout = SpanLayout;
    type Selection = Selection<usize>;
    type Item<'r> = OptionRow<'r>;
    type ItemEvent = ();

    fn items(&self) -> &CollectionProps {
        &self.items
    }

    fn selection(&mut self) -> Option<BindMut<'_, Selection<usize>>> {
        Some(self.selection.reborrow())
    }

    fn item_layout(&self, source: &dyn ItemSource<usize>, width: f32, cx: &StyleCx<'_>) -> SpanLayout {
        SpanLayout::new(source, width, cx.control().height)
    }
}

/// The style of an option row: hover and selection fills, the control's padding and the text colour.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct OptionRowStyle {
    #[style(surface)]
    surface: SurfaceLook,
    #[style(metrics)]
    metrics: Metrics,
    /// The option's label.
    #[style(text)]
    text: TextLook,
}

// F8-MERGE: F8a. The list family owns the row look with `ListRow`.
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

/// A row of the option list: its label. The list's item step handles the click and Enter that pick it.
#[derive(crate::contract::Component)]
pub struct OptionRow<'r> {
    #[common]
    common: Common,
    label: &'r str,
}

impl<'r> OptionRow<'r> {
    /// A row showing `label`.
    pub fn new(label: &'r str) -> Self {
        OptionRow { common: Common::new(), label }.label(label.to_owned())
    }
}

impl Component for OptionRow<'_> {
    const ROLE: Role = Role::LIST_BOX_OPTION;
    const OVERLAYS: bool = false;
    type Style = OptionRowStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: OptionRowStyleKey) -> OptionRowStyle {
        OptionRowStyle {
            surface: row_look(cx),
            metrics: cx.control().metrics(),
            text: TextLook::new(TypeRole::Label, Stateful::new(cx.ink(Ink::OnPanel))),
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        cx.layout(&TextRun::new(self.label)).size()
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        ui.draw().text(cx.content_rect, &TextRun::new(self.label).elide().color(cx.resolve(&cx.style.text.color)));
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}
