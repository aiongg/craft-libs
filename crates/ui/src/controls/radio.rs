//! [`RadioGroup`] and [`RadioSet`]: one of several options, as round radio buttons in a radio
//! group.

use std::sync::Arc;

use craft_theme::{ControlSize, States, TypeRole};
use egui::accesskit::Role;
use egui::{AsIdSalt, Color32, CornerRadius, CursorIcon, Id, IdSalt, Rect, Response, Sense, Ui, UiBuilder, Vec2, vec2};

use super::button::content_color;
use super::check::{IndicatorColors, Labelled, Layout, Paint, box_shape, label_colors};
use super::checkbox::LONG_LABEL;
use super::{SIZE_NAMES, size_arg};
use crate::behavior::RadioSelection;
use crate::direction;
use crate::edit::{EditPolicy, EditResponse, Gesture};
use crate::primitives::{Content, Gap, Interactive, Row, Sensed, Stack, Text};
use crate::story::{Args, Level, Story};
use crate::theme::{Tokens, TokensOf, UiTokens};

/// A radio button's sizes and colours, from [`RadioGroup::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct RadioStyle {
    /// Height of one line: `check.height`.
    pub height: f32,
    /// Diameter of the circle: `check.box_size`.
    pub diameter: f32,
    /// Between the circle and the label: `check.box_gap`.
    pub gap: f32,
    /// `check.label`.
    pub label: TypeRole,
    /// Diameter of the dot: `check.dot`.
    pub dot: f32,
    /// Width of the outline, drawn inside the circle: `check.outline`.
    pub outline: f32,
    /// Not selected: `check.off.*`.
    pub off: IndicatorColors,
    /// Selected: `check.on.*`.
    pub on: IndicatorColors,
    /// The label: `text.muted`, `text.disabled`.
    pub text: States<Color32>,
}

impl RadioStyle {
    fn layout(&self) -> Layout {
        Layout { height: self.height, gap: self.gap, label: self.label, indicator: vec2(self.diameter, self.diameter) }
    }
}

/// Reports a sensed radio button with its settled state and paints its circle, dot and label.
fn paint(ui: &Ui, style: &RadioStyle, laid: &Labelled, sensed: Sensed, on: bool) -> Response {
    let mut response = sensed.checked(on).finish(ui);
    if response.enabled() {
        response = response.on_hover_cursor(CursorIcon::PointingHand);
    }
    if ui.is_rect_visible(response.rect) {
        let (rect, label) = laid.place(ui, response.rect);
        let paint = Paint::now(ui, if on { &style.on } else { &style.off }, &response);
        let round = CornerRadius::from(style.diameter / 2.0);
        ui.painter().add(box_shape(rect, round, style.outline, paint));
        if let Some(dot) = paint.mark {
            ui.painter().circle_filled(rect.center(), style.dot / 2.0, dot);
        }
        laid.paint_label(ui, label, content_color(ui, &style.text, &response, false));
    }
    response
}

/// One option of a [`RadioGroup`] or [`RadioSet`].
#[derive(Debug)]
struct Choice<T> {
    value: T,
    label: String,
    enabled: bool,
}

/// A list of radio buttons choosing one value, a radio group:
///
/// ```
/// # use craft_ui::controls::RadioGroup;
/// # #[derive(Clone, PartialEq)] enum Unit { Px, Mm, In }
/// # egui::__run_test_ui(|ui| { let mut unit = Unit::Px;
/// let edit = RadioGroup::new("units", "Units", &mut unit)
///     .item(Unit::Px, "Pixels")
///     .item(Unit::Mm, "Millimetres")
///     .item(Unit::In, "Inches")
///     .show(ui);
/// # });
/// ```
///
/// The options run top down from the reading start. AccessKit reports a `RadioGroup` named
/// `label` (not drawn: the form shows its own heading) and the number of options, with a
/// `RadioButton` per option, the selected one toggled after this frame's click or key, and its
/// position in the set. Keys and focus are [`RadioSelection`]'s: each enabled option a Tab stop,
/// the arrows move the focus without selecting, Enter, Space or a click selects. A change is one
/// [`Set`](crate::edit::EditEvent::Set) with the modifiers held. Options placed apart, each beside
/// its own field, belong in a [`RadioSet`].
#[derive(Debug)]
pub struct RadioGroup<'a, T> {
    id_salt: IdSalt,
    label: String,
    value: &'a mut T,
    items: Vec<Choice<T>>,
    size: ControlSize,
    enabled: bool,
}

impl<'a, T: Clone + PartialEq + Send + Sync + 'static> RadioGroup<'a, T> {
    /// A group named `label` (its accessible name) choosing `value`; `id_salt` keeps its focus
    /// and animations when the layout around it changes.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, value: &'a mut T) -> Self {
        Self { id_salt: IdSalt::new(id_salt), label: label.into(), value, items: Vec::new(), size: ControlSize::Md, enabled: true }
    }

    /// An option showing `label`, which selects `value`.
    pub fn item(mut self, value: T, label: impl Into<String>) -> Self {
        self.items.push(Choice { value, label: label.into(), enabled: true });
        self
    }

    /// Whether the last option added takes input; a disabled one takes no focus, so Tab and the
    /// arrow keys pass over it, and it draws `check.*.disabled`.
    pub fn item_enabled(mut self, enabled: bool) -> Self {
        if let Some(item) = self.items.last_mut() {
            item.enabled = enabled;
        }
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Whether the group takes input; a disabled group draws every option `check.*.disabled`.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Shows the group. The response covers every option; it is
    /// [changed](Response::changed) when the selection changed.
    pub fn show(self, ui: &mut Ui) -> EditResponse<T> {
        let tokens = Tokens::of(ui);
        let style = RadioGroup::style(&tokens, self.size);
        let enabled = self.enabled && ui.is_enabled();
        let builder = UiBuilder::new()
            .id_salt(self.id_salt)
            .layout(direction::column(ui))
            .accessibility_role(Role::RadioGroup)
            .accessibility_label(self.label.as_str());
        let inner = ui.scope_builder(builder, |group| {
            group.spacing_mut().item_spacing = Vec2::ZERO;
            let laid: Vec<_> = self.items.iter().map(|item| Labelled::layout(group, &tokens, &style.layout(), &item.label)).collect();
            let items = self.items.iter().enumerate().map(|(i, item)| (Interactive::id_in(group, i), enabled && item.enabled)).collect();
            let selected = self.items.iter().position(|item| item.value == *self.value);
            let mut radios = RadioSelection::new(items, selected);
            radios.report_size(group, group.unique_id());
            let sensed: Vec<_> = self
                .items
                .iter()
                .zip(&laid)
                .enumerate()
                .map(|(i, (item, laid))| {
                    Interactive::new(i, Role::RadioButton, item.label.as_str())
                        .position_in_set(i)
                        .state_layer(false)
                        .enabled(enabled && item.enabled)
                        .interact(group, laid.size)
                })
                .collect();
            radios.sensed(&sensed);
            let selected = radios.selected();
            let responses: Vec<_> = sensed
                .into_iter()
                .zip(&laid)
                .zip(&self.items)
                .enumerate()
                .map(|(i, ((sensed, laid), item))| laid.tooltip(paint(group, &style, laid, sensed, selected == Some(i)), &item.label))
                .collect();
            (responses, radios.changed().then_some(selected).flatten())
        });
        let (responses, chosen) = inner.inner;
        let rect = inner.response.rect;
        let mut response = match responses.into_iter().reduce(|all, r| all.union(r)) {
            Some(all) => all.with_new_rect(rect),
            None => ui.interact(Rect::from_min_size(rect.min, Vec2::ZERO), ui.make_persistent_id(self.id_salt), Sense::hover()),
        };
        let mut gesture = Gesture::load(ui, response.id, &*self.value, EditPolicy::default()).enabled(enabled);
        if let Some(i) = chosen {
            let value = self.items[i].value.clone();
            gesture.set(value.clone());
            *self.value = value;
            response.mark_changed();
        }
        EditResponse::new(response, gesture.finish())
    }
}

impl RadioGroup<'static, ()> {
    /// The sizes and colours of a radio button of `size`, in a [`RadioGroup`] or a [`RadioSet`].
    pub fn style(tokens: &UiTokens, size: ControlSize) -> RadioStyle {
        let c = &tokens.component.check;
        RadioStyle {
            height: c.height.get(size),
            diameter: c.box_size.get(size),
            gap: c.box_gap,
            label: c.label,
            dot: c.dot.get(size),
            outline: c.outline,
            off: IndicatorColors::from(&c.off),
            on: IndicatorColors::from(&c.on),
            text: label_colors(tokens, tokens.color.text.muted),
        }
    }
}

/// Radio buttons choosing one value, each placed where the layout needs it (an option followed
/// by its own field), as one radio group:
///
/// ```
/// # use craft_ui::controls::RadioSet;
/// # #[derive(Clone, PartialEq)] enum Crop { Original, Fixed }
/// # egui::__run_test_ui(|ui| { let (mut crop, mut ratio) = (Crop::Original, 1.5);
/// let mut set = RadioSet::new("crop", "Crop", &mut crop).item(Crop::Original).item(Crop::Fixed).begin(ui);
/// set.radio(ui, 0, "Original ratio");
/// ui.horizontal(|ui| {
///     set.radio(ui, 1, "Fixed ratio");
///     ui.add_enabled(crop == Crop::Fixed, egui::DragValue::new(&mut ratio));
/// });
/// let edit = set.end(ui);
/// # });
/// ```
///
/// [`begin`](RadioSet::begin) takes this frame's keys and click and applies the change to the
/// value before any option is drawn, so every option, and the code between them, sees the new
/// selection in the frame of the click. Each option's id comes from the set's `id_salt`, not
/// from where it sits. Keys and focus are [`RadioSelection`]'s, as in a [`RadioGroup`]: each
/// enabled option a Tab stop, the arrows move the focus (to whatever control sits that way)
/// without selecting, Enter, Space or a click selects. AccessKit reports a `RadioGroup` named
/// `label`, with the number of options, where `begin` runs, with every option a `RadioButton`
/// inside it, at its position in the set. [`end`](PreparedRadioSet::end) reports a change as one
/// [`Set`](crate::edit::EditEvent::Set) with the modifiers held.
#[derive(Debug)]
pub struct RadioSet<'a, T> {
    id_salt: IdSalt,
    label: String,
    value: &'a mut T,
    items: Vec<Choice<T>>,
    size: ControlSize,
    enabled: bool,
}

impl<'a, T: Clone + PartialEq + Send + Sync + 'static> RadioSet<'a, T> {
    /// A set named `label` (its accessible name) choosing `value`; `id_salt` gives the options
    /// their ids, so their focus and animations stay wherever they are placed.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, value: &'a mut T) -> Self {
        Self { id_salt: IdSalt::new(id_salt), label: label.into(), value, items: Vec::new(), size: ControlSize::Md, enabled: true }
    }

    /// An option selecting `value`; options are numbered from 0 in the order they are added.
    pub fn item(mut self, value: T) -> Self {
        self.items.push(Choice { value, label: String::new(), enabled: true });
        self
    }

    /// Whether the last option added takes input; a disabled one takes no focus, so Tab and the
    /// arrow keys pass over it, and it draws `check.*.disabled`.
    pub fn item_enabled(mut self, enabled: bool) -> Self {
        if let Some(item) = self.items.last_mut() {
            item.enabled = enabled;
        }
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Whether the set takes input; a disabled set draws every option `check.*.disabled`.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Applies this frame's keys and click to the value, and adds the group's AccessKit node
    /// to `ui`. Draw each option with [`radio`](PreparedRadioSet::radio), then call
    /// [`end`](PreparedRadioSet::end).
    pub fn begin(self, ui: &mut Ui) -> PreparedRadioSet<T> {
        let tokens = Tokens::of(ui);
        let style = RadioGroup::style(&tokens, self.size);
        let id = ui.make_persistent_id(self.id_salt);
        // Only the group's AccessKit node: it takes no room, and the options join it as children.
        drop(ui.new_child(UiBuilder::new().scope_id(id).accessibility_role(Role::RadioGroup).accessibility_label(self.label.as_str())));
        let enabled = self.enabled && ui.is_enabled();
        let items: Vec<_> = self.items.iter().enumerate().map(|(i, item)| (id.with(i), enabled && item.enabled)).collect();
        let selected = self.items.iter().position(|item| item.value == *self.value);
        let mut radios = RadioSelection::new(items.clone(), selected);
        radios.report_size(ui, id);
        radios.click_from_input(ui);
        let mut gesture = Gesture::load(ui, id, &*self.value, EditPolicy::default()).enabled(enabled);
        let chosen = radios.selected().filter(|_| radios.changed());
        if let Some(i) = chosen {
            let value = self.items[i].value.clone();
            gesture.set(value.clone());
            *self.value = value;
        }
        PreparedRadioSet { id, items, radios, tokens, style, gesture, responses: Vec::new(), changed: chosen.is_some() }
    }
}

/// A [`RadioSet`] whose value is settled for this frame: draw each option with
/// [`radio`](PreparedRadioSet::radio), then [`end`](PreparedRadioSet::end) it.
#[derive(Debug)]
#[must_use = "a radio set reports its change in `end`"]
pub struct PreparedRadioSet<T> {
    id: Id,
    /// Each option's id and whether it takes input.
    items: Vec<(Id, bool)>,
    radios: RadioSelection,
    tokens: Arc<UiTokens>,
    style: RadioStyle,
    gesture: Gesture<T>,
    responses: Vec<Response>,
    changed: bool,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> PreparedRadioSet<T> {
    /// Draws option `index` (in the order of [`item`](RadioSet::item)) showing `label`, at the
    /// next place in `ui`. Draw each option once per frame.
    ///
    /// # Panics
    ///
    /// If the set has no option `index`.
    pub fn radio(&mut self, ui: &mut Ui, index: usize, label: &str) -> Response {
        self.radio_named(ui, index, label, label)
    }

    /// Draws option `index` showing `label`, as [`radio`](PreparedRadioSet::radio), with
    /// `name` as its accessible name: a short label ("H:") for an option named in full ("Hue").
    ///
    /// # Panics
    ///
    /// If the set has no option `index`.
    pub fn radio_named(&mut self, ui: &mut Ui, index: usize, label: &str, name: &str) -> Response {
        let (id, enabled) = self.items[index];
        let selected = self.radios.selected() == Some(index);
        let (tokens, style) = (&self.tokens, &self.style);
        let response = ui
            .scope_builder(UiBuilder::new().accessibility_parent(self.id), |ui| {
                let laid = Labelled::layout(ui, tokens, &style.layout(), label);
                let sensed = Interactive::with_id(id, Role::RadioButton, name)
                    .position_in_set(index)
                    .state_layer(false)
                    .enabled(enabled)
                    .interact(ui, laid.size);
                laid.tooltip(paint(ui, style, &laid, sensed, selected), label)
            })
            .inner;
        self.responses.push(response.clone());
        response
    }

    /// Ends the frame of the set. The response covers every option drawn; it is
    /// [changed](Response::changed) when the selection changed.
    pub fn end(self, ui: &mut Ui) -> EditResponse<T> {
        let mut response = match self.responses.into_iter().reduce(|all, r| all.union(r)) {
            Some(all) => all,
            None => ui.interact(Rect::from_min_size(ui.cursor().min, Vec2::ZERO), self.id.with("options"), Sense::hover()),
        };
        if self.changed {
            response.mark_changed();
        }
        EditResponse::new(response, self.gesture.finish())
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Radio"];

const OPTIONS: [&str; 5] = ["Pixels", "Millimetres", "Inches", "Points", "Picas"];

pub(crate) fn stories() -> Vec<Story> {
    let args = || Args::new().with_choice("size", SIZE_NAMES, 1).with_bool("disabled", false);
    vec![
        Story::new("controls/radio/states", Level::Controls, "Radio", "States", args, show_states),
        Story::new("controls/radio/groups", Level::Controls, "Radio", "Groups", args, show_groups),
    ]
}

/// A radio set with an option followed by its field and a long label; a disabled option; a
/// disabled set, selected and not.
fn show_states(ui: &mut Ui, args: &mut Args) {
    let size = size_arg(args.choice("size"));
    let enabled = !args.bool("disabled");
    let state_id = ui.scope_id().with("radio-states");
    let mut choice = ui.data(|d| d.get_temp::<usize>(state_id).unwrap_or(0));
    Stack::new(Gap::Stack).show(ui, |ui| {
        let mut set = RadioSet::new("radio-zoom", "Zoom", &mut choice).size(size).enabled(enabled);
        for i in 0..4 {
            set = set.item(i);
        }
        let mut set = set.item_enabled(false).begin(ui);
        let _ = set.radio(ui, 0, "Fit to window");
        Row::new(Gap::Inline).show(ui, |ui| {
            let _ = set.radio(ui, 1, "Actual size");
            ui.add(Text::new("100%").content(Content::Muted));
        });
        let _ = set.radio(ui, 2, LONG_LABEL);
        let _ = set.radio(ui, 3, "Print size (no printer)");
        let _ = set.end(ui);

        let mut fixed = 0;
        let mut set = RadioSet::new("radio-disabled", "Disabled", &mut fixed).item(0).item(1).size(size).enabled(false).begin(ui);
        let _ = set.radio(ui, 0, "Disabled (selected)");
        let _ = set.radio(ui, 1, "Disabled");
        let _ = set.end(ui);
    });
    ui.data_mut(|d| d.insert_temp(state_id, choice));
}

/// Groups of three and five options side by side; the five have a disabled option.
fn show_groups(ui: &mut Ui, args: &mut Args) {
    let size = size_arg(args.choice("size"));
    let enabled = !args.bool("disabled");
    let state_id = ui.scope_id().with("radio-groups");
    let mut choices = ui.data(|d| d.get_temp::<[usize; 2]>(state_id).unwrap_or([0, 2]));
    Stack::new(Gap::Stack).show(ui, |ui| {
        Row::new(Gap::Section).show(ui, |ui| {
            for (n, choice) in [3, 5].into_iter().zip(choices.iter_mut()) {
                let mut group = RadioGroup::new(("radio-group", n), format!("Units ({n})"), choice).size(size).enabled(enabled);
                for (i, label) in OPTIONS.iter().take(n).enumerate() {
                    group = group.item(i, *label).item_enabled(i != 3);
                }
                let _ = group.show(ui);
            }
        });
        ui.add(Text::new(format!("Selected: {}, {}", OPTIONS[choices[0]], OPTIONS[choices[1]])).content(Content::Muted));
    });
    ui.data_mut(|d| d.insert_temp(state_id, choices));
}
