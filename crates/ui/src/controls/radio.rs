//! [`Radio`] and [`RadioGroup`]: one of several options, as round radio buttons.

use craft_theme::{ControlSize, States, TypeRole};
use egui::accesskit::Role;
use egui::{AsIdSalt, Color32, CornerRadius, IdSalt, Rect, Response, Sense, Ui, UiBuilder, Vec2, vec2};

use super::button::content_color;
use super::check::{IndicatorColors, Labelled, Layout, Paint, indicator_shapes, label_colors};
use super::checkbox::LONG_LABEL;
use super::{SIZE_NAMES, salt, size_arg};
use crate::behavior::RadioSelection;
use crate::direction;
use crate::edit::{EditPolicy, EditResponse, Gesture};
use crate::primitives::{Content, Gap, Interactive, Row, Sensed, Stack, Surface, SurfaceKind, SurfaceStyle, Text};
use crate::story::{Args, Level, Story};
use crate::theme::{Tokens, TokensOf, UiTokens};

/// A radio button's sizes and colours, from [`Radio::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct RadioStyle {
    /// Height of one line: `check.height`.
    pub height: f32,
    /// Diameter of the circle: `check.size`.
    pub diameter: f32,
    /// Between the circle and the label: `check.gap`.
    pub gap: f32,
    /// `check.label`.
    pub label: TypeRole,
    /// Diameter of the dot: `check.dot`.
    pub dot: f32,
    /// The sunken surface the circle draws on, for its edge (a border, or a bevel in bevel
    /// themes); the fill is the circle's own.
    pub surface: SurfaceStyle,
    /// Not selected: `check.off.*`.
    pub off: IndicatorColors,
    /// Selected: `check.on.*`.
    pub on: IndicatorColors,
    /// The label: `text.normal`, `text.disabled`.
    pub text: States<Color32>,
}

impl RadioStyle {
    fn layout(&self) -> Layout {
        Layout { height: self.height, gap: self.gap, label: self.label, indicator: vec2(self.diameter, self.diameter) }
    }
}

/// One radio button with a label, for a layout of its own (an option followed by its field);
/// it selects `value` in `current`: `Radio::new("Pixels", &mut unit, Unit::Px).show(ui)`.
///
/// Each radio button is its own Tab stop, and one drawn before the radio button a click selects
/// shows the change from the next frame. A list of options belongs in a [`RadioGroup`]: one Tab
/// stop with arrow keys, every option settled in the click's frame.
///
/// A click on the circle or label, or Space while it has focus, selects it; a change is one
/// [`Set`](crate::edit::EditEvent::Set) with the modifiers held. AccessKit reports a
/// `RadioButton` named by the label, toggled when selected, after this frame's click. The circle
/// sits at the reading start on a sunken [`Surface`]: `check.off.*` with an outline, or
/// `check.on.*` with a dot when selected.
#[derive(Debug)]
pub struct Radio<'a, T> {
    label: String,
    current: &'a mut T,
    value: T,
    size: ControlSize,
    enabled: bool,
    id_salt: Option<IdSalt>,
}

impl<'a, T: Clone + PartialEq + Send + Sync + 'static> Radio<'a, T> {
    pub fn new(label: impl Into<String>, current: &'a mut T, value: T) -> Self {
        Self { label: label.into(), current, value, size: ControlSize::Md, enabled: true, id_salt: None }
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Whether it takes input; a disabled radio button draws `check.*.disabled`.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Keeps focus and animations when the layout around it changes; else the `Ui`'s next
    /// automatic id.
    pub fn id_salt(mut self, id_salt: impl AsIdSalt) -> Self {
        self.id_salt = Some(IdSalt::new(id_salt));
        self
    }

    /// Shows the radio button; the response is [changed](Response::changed) when a click
    /// selects it.
    pub fn show(self, ui: &mut Ui) -> EditResponse<T> {
        let tokens = Tokens::of(ui);
        let style = Radio::style(&tokens, self.size);
        let laid = Labelled::layout(ui, &tokens, &style.layout(), &self.label);
        let sensed = Interactive::new(salt(ui, self.id_salt), Role::RadioButton, self.label.as_str())
            .state_layer(false)
            .enabled(self.enabled)
            .interact(ui, laid.size);
        let mut gesture = Gesture::load(ui, sensed.response().id, &*self.current, EditPolicy::default());
        let changed = sensed.response().clicked() && *self.current != self.value;
        if changed {
            gesture.set(self.value.clone());
            *self.current = self.value.clone();
        }
        let mut response = paint(ui, &style, &laid, sensed, *self.current == self.value);
        if changed {
            response.mark_changed();
        }
        EditResponse::new(laid.tooltip(response, &self.label), gesture.finish())
    }
}

impl Radio<'static, ()> {
    pub fn style(tokens: &UiTokens, size: ControlSize) -> RadioStyle {
        let c = &tokens.component.check;
        RadioStyle {
            height: c.height.get(size),
            diameter: c.size.get(size),
            gap: c.gap,
            label: c.label,
            dot: c.dot.get(size),
            surface: Surface::style(tokens, SurfaceKind::Sunken),
            off: IndicatorColors::from(&c.off),
            on: IndicatorColors::from(&c.on),
            text: label_colors(tokens),
        }
    }
}

/// Reports a sensed radio button with its settled state and paints its circle, dot and label.
fn paint(ui: &Ui, style: &RadioStyle, laid: &Labelled, sensed: Sensed, on: bool) -> Response {
    let response = sensed.checked(on).finish(ui);
    if ui.is_rect_visible(response.rect) {
        let (rect, label) = laid.place(ui, response.rect);
        let paint = Paint::now(ui, if on { &style.on } else { &style.off }, &response);
        let round = CornerRadius::from(style.diameter / 2.0);
        ui.painter().extend(indicator_shapes(ui, &style.surface, rect, round, paint));
        if on {
            ui.painter().circle_filled(rect.center(), style.dot / 2.0, paint.mark);
        }
        laid.paint_label(ui, label, content_color(ui, &style.text, &response, false));
    }
    response
}

/// A list of radio buttons choosing one value, a WAI-ARIA radio group:
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
/// `label` (not drawn: the form shows its own heading) with a `RadioButton` per option, the
/// selected one toggled, after this frame's click or key. Keys and focus are
/// [`RadioSelection`]'s: one Tab stop, arrows (mirrored in a right-to-left interface), Home and
/// End. A change is one [`Set`](crate::edit::EditEvent::Set) with the modifiers held.
#[derive(Debug)]
pub struct RadioGroup<'a, T> {
    id_salt: IdSalt,
    label: String,
    value: &'a mut T,
    items: Vec<(T, String)>,
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
        self.items.push((value, label.into()));
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Shows the group. The response covers every option; it is
    /// [changed](Response::changed) when the selection changed.
    pub fn show(self, ui: &mut Ui) -> EditResponse<T> {
        let tokens = Tokens::of(ui);
        let style = Radio::style(&tokens, self.size);
        let enabled = self.enabled && ui.is_enabled();
        let builder = UiBuilder::new()
            .id_salt(self.id_salt)
            .layout(direction::column(ui))
            .accessibility_role(Role::RadioGroup)
            .accessibility_label(self.label.as_str());
        let inner = ui.scope_builder(builder, |group| {
            group.spacing_mut().item_spacing = Vec2::ZERO;
            let laid: Vec<_> = self.items.iter().map(|(_, label)| Labelled::layout(group, &tokens, &style.layout(), label)).collect();
            let ids = (0..self.items.len()).map(|i| Interactive::id_in(group, i)).collect();
            let selected = self.items.iter().position(|(value, _)| value == &*self.value);
            let mut radios = RadioSelection::begin(group, ids, selected, enabled);
            let sensed: Vec<_> = self
                .items
                .iter()
                .zip(&laid)
                .enumerate()
                .map(|(i, ((_, label), laid))| {
                    Interactive::new(i, Role::RadioButton, label.as_str())
                        .sense(radios.sense(i))
                        .state_layer(false)
                        .enabled(enabled)
                        .interact(group, laid.size)
                })
                .collect();
            radios.sensed(group, &sensed);
            let selected = radios.selected();
            let responses: Vec<_> = sensed
                .into_iter()
                .zip(&laid)
                .zip(&self.items)
                .enumerate()
                .map(|(i, ((sensed, laid), (_, label)))| laid.tooltip(paint(group, &style, laid, sensed, selected == Some(i)), label))
                .collect();
            (responses, radios.changed().then_some(selected).flatten())
        });
        let (responses, chosen) = inner.inner;
        let rect = inner.response.rect;
        let mut response = match responses.into_iter().reduce(|all, r| all.union(r)) {
            Some(all) => all.with_new_rect(rect),
            None => ui.interact(Rect::from_min_size(rect.min, Vec2::ZERO), ui.make_persistent_id(self.id_salt), Sense::hover()),
        };
        let mut gesture = Gesture::load(ui, response.id, &*self.value, EditPolicy::default());
        if let Some(i) = chosen {
            let value = self.items[i].0.clone();
            gesture.set(value.clone());
            *self.value = value;
            response.mark_changed();
        }
        EditResponse::new(response, gesture.finish())
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Radio"];

const OPTIONS: [&str; 5] = ["Pixels", "Millimetres", "Inches", "Points", "Picas"];

pub(crate) fn stories() -> Vec<Story> {
    let args = || Args::new().with_choice("size", SIZE_NAMES, 1).with_bool("disabled", false);
    vec![
        Story { id: "controls/radio/states", level: Level::Controls, component: "Radio", name: "States", args, show: show_states },
        Story { id: "controls/radio/groups", level: Level::Controls, component: "Radio", name: "Groups", args, show: show_groups },
    ]
}

/// Single radio buttons: selected and not, enabled and disabled, a long label.
fn show_states(ui: &mut Ui, args: &mut Args) {
    let size = size_arg(args.choice("size"));
    let enabled = !args.bool("disabled");
    let state_id = ui.scope_id().with("radio-states");
    let mut choice = ui.data(|d| d.get_temp::<usize>(state_id).unwrap_or(0));
    Stack::new(Gap::Stack).show(ui, |ui| {
        for (i, label) in ["Fit to window", "Actual size", LONG_LABEL].into_iter().enumerate() {
            let _ = Radio::new(label, &mut choice, i).size(size).enabled(enabled).id_salt(("radio", i)).show(ui);
        }
        for (i, label) in ["Disabled (selected)", "Disabled"].into_iter().enumerate() {
            let mut fixed = 0;
            let _ = Radio::new(label, &mut fixed, i).size(size).enabled(false).id_salt(("radio-disabled", i)).show(ui);
        }
    });
    ui.data_mut(|d| d.insert_temp(state_id, choice));
}

/// Groups of three and five options side by side.
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
                    group = group.item(i, *label);
                }
                let _ = group.show(ui);
            }
        });
        ui.add(Text::new(format!("Selected: {}, {}", OPTIONS[choices[0]], OPTIONS[choices[1]])).content(Content::Muted));
    });
    ui.data_mut(|d| d.insert_temp(state_id, choices));
}
