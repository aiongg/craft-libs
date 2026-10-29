//! [`Checkbox`]: a box that is checked, unchecked or mixed.

use craft_theme::{ControlSize, States, TypeRole};
use egui::accesskit::Role;
use egui::{AsIdSalt, Color32, CornerRadius, CursorIcon, IdSalt, Rect, Shape, Stroke, Ui, vec2};

use super::button::content_color;
use super::check::{IndicatorColors, Labelled, Layout, Paint, box_shape, label_colors};
use super::{SIZE_NAMES, salt, size_arg};
use crate::edit::{EditPolicy, EditResponse, Gesture};
use crate::primitives::{Gap, Interactive, Stack};
use crate::story::{Args, Level, Story};
use crate::theme::{Tokens, TokensOf, UiTokens};

/// A check box's state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CheckState {
    /// Off.
    #[default]
    Unchecked,
    /// On.
    Checked,
    /// Neither: the option is on for some of the selected items and off for others.
    Mixed,
}

impl CheckState {
    /// The state after a click or Space: checked from unchecked or mixed, else unchecked.
    pub fn toggled(self) -> Self {
        match self {
            CheckState::Unchecked | CheckState::Mixed => CheckState::Checked,
            CheckState::Checked => CheckState::Unchecked,
        }
    }
}

impl From<bool> for CheckState {
    fn from(checked: bool) -> Self {
        if checked { CheckState::Checked } else { CheckState::Unchecked }
    }
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for bool {}
    impl Sealed for super::CheckState {}
}

/// A value a [`Checkbox`] edits: a [`CheckState`], or a `bool` for a box that is never mixed.
pub trait CheckValue: sealed::Sealed + Clone + PartialEq + Send + Sync + 'static {
    /// The state the box shows.
    fn state(&self) -> CheckState;
    /// The value after a click or Space.
    #[must_use]
    fn toggled(&self) -> Self;
}

impl CheckValue for CheckState {
    fn state(&self) -> CheckState {
        *self
    }

    fn toggled(&self) -> Self {
        CheckState::toggled(*self)
    }
}

impl CheckValue for bool {
    fn state(&self) -> CheckState {
        CheckState::from(*self)
    }

    fn toggled(&self) -> Self {
        !*self
    }
}

/// A check box's sizes and colours, from [`Checkbox::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CheckboxStyle {
    /// Height of one line: `check.height`.
    pub height: f32,
    /// Side of the box: `check.box_size`.
    pub side: f32,
    /// Between the box and the label: `check.box_gap`.
    pub gap: f32,
    /// `check.label`.
    pub label: TypeRole,
    /// The box's corners: `check.radius`.
    pub radius: CornerRadius,
    /// Stroke width of the check and the dash: `check.mark`.
    pub mark: f32,
    /// Width of the outline, drawn inside the box: `check.outline`.
    pub outline: f32,
    /// Unchecked: `check.off.*`.
    pub off: IndicatorColors,
    /// Checked or mixed: `check.on.*`.
    pub on: IndicatorColors,
    /// The label: `text.muted`, `text.disabled`.
    pub text: States<Color32>,
}

/// A check box with a label: `Checkbox::bool("Snap to grid", &mut snap).show(ui)`, or
/// [`new`](Checkbox::new) with a [`CheckState`] for a box that can be mixed.
///
/// A click anywhere on the box or label, or Space while it has focus, toggles it; a mixed box
/// becomes checked. Each change is one [`Set`](crate::edit::EditEvent::Set) with the modifiers
/// held. AccessKit reports a `CheckBox` named by the label, toggled true, false or mixed, after
/// this frame's click.
///
/// The box sits at the reading start, flat in every theme: the `check.off.*` colours with an
/// outline while unchecked, `check.on.*` with a check or a dash while checked or mixed. The
/// pointer is a hand over the box and the label, and pressing the label shows the box pressed. The label wraps, is cut or runs on as the `Ui`'s wrap mode says, with
/// the whole label in a tooltip when cut.
#[derive(Debug)]
pub struct Checkbox<'a, T: CheckValue = CheckState> {
    label: String,
    value: &'a mut T,
    size: ControlSize,
    enabled: bool,
    id_salt: Option<IdSalt>,
}

impl<'a> Checkbox<'a, CheckState> {
    /// A box showing `value`, which may be mixed.
    pub fn new(label: impl Into<String>, value: &'a mut CheckState) -> Self {
        Self::with(label, value)
    }
}

impl<'a> Checkbox<'a, bool> {
    /// A box that is checked or not.
    pub fn bool(label: impl Into<String>, value: &'a mut bool) -> Self {
        Self::with(label, value)
    }
}

impl<'a, T: CheckValue> Checkbox<'a, T> {
    fn with(label: impl Into<String>, value: &'a mut T) -> Self {
        Self { label: label.into(), value, size: ControlSize::Md, enabled: true, id_salt: None }
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Whether it takes input; a disabled box draws `check.*.disabled`.
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

    /// Shows the box; the response is [changed](egui::Response::changed) on a toggle.
    pub fn show(self, ui: &mut Ui) -> EditResponse<T> {
        let tokens = Tokens::of(ui);
        let style = Checkbox::style(&tokens, self.size);
        let layout = Layout { height: style.height, gap: style.gap, label: style.label, indicator: vec2(style.side, style.side) };
        let laid = Labelled::layout(ui, &tokens, &layout, &self.label);
        let sensed = Interactive::salted(salt(ui, self.id_salt), Role::CheckBox, self.label.as_str())
            .state_layer(false)
            .enabled(self.enabled)
            .interact(ui, laid.size);
        let mut gesture = Gesture::load(ui, sensed.response().id, &*self.value, EditPolicy::default()).enabled(sensed.response().enabled());
        let clicked = sensed.response().clicked();
        if clicked {
            let next = self.value.toggled();
            gesture.set(next.clone());
            *self.value = next;
        }
        let state = self.value.state();
        let sensed = match state {
            CheckState::Mixed => sensed.mixed(),
            _ => sensed.checked(state == CheckState::Checked),
        };
        let mut response = sensed.finish(ui);
        if response.enabled() {
            response = response.on_hover_cursor(CursorIcon::PointingHand);
        }
        if clicked {
            response.mark_changed();
        }
        if ui.is_rect_visible(response.rect) {
            let (rect, label) = laid.place(ui, response.rect);
            let on = state != CheckState::Unchecked;
            let paint = Paint::now(ui, if on { &style.on } else { &style.off }, &response);
            ui.painter().add(box_shape(rect, style.radius, style.outline, paint));
            if let Some(color) = paint.mark {
                ui.painter().extend(mark(rect, state, style.mark, color));
            }
            laid.paint_label(ui, label, content_color(ui, &style.text, &response, false));
        }
        let response = laid.tooltip(response, &self.label);
        EditResponse::new(response, gesture.finish())
    }
}

impl Checkbox<'static, CheckState> {
    pub fn style(tokens: &UiTokens, size: ControlSize) -> CheckboxStyle {
        let c = &tokens.component.check;
        CheckboxStyle {
            height: c.height.get(size),
            side: c.box_size.get(size),
            gap: c.box_gap,
            label: c.label,
            radius: CornerRadius::from(c.radius),
            mark: c.mark.get(size),
            outline: c.outline,
            off: IndicatorColors::from(&c.off),
            on: IndicatorColors::from(&c.on),
            text: label_colors(tokens, tokens.color.text.muted),
        }
    }
}

/// The check or dash in the box `rect`; none when unchecked. The check is PhotoCraft's: two
/// segments through points laid out on a 14 point box and scaled to `rect`.
fn mark(rect: Rect, state: CheckState, width: f32, color: Color32) -> Vec<Shape> {
    let k = rect.width() / 14.0;
    let stroke = Stroke::new(width, color);
    match state {
        CheckState::Unchecked => Vec::new(),
        CheckState::Checked => {
            let a = rect.left_center() + vec2(3.0, 0.5) * k;
            let b = rect.center_bottom() + vec2(-1.0, -3.5) * k;
            let c = rect.right_top() + vec2(-3.0, 3.5) * k;
            vec![Shape::line_segment([a, b], stroke), Shape::line_segment([b, c], stroke)]
        }
        CheckState::Mixed => {
            let y = rect.center().y;
            vec![Shape::line_segment([egui::pos2(rect.left() + 0.26 * rect.width(), y), egui::pos2(rect.left() + 0.74 * rect.width(), y)], stroke)]
        }
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Checkbox"];

/// A label long enough to wrap in a narrow panel.
pub(crate) const LONG_LABEL: &str = "Show the transform controls on the selected layer whenever the move tool is active";

pub(crate) fn stories() -> Vec<Story> {
    vec![Story {
        id: "controls/checkbox/states",
        level: Level::Controls,
        component: "Checkbox",
        name: "States",
        args: || Args::new().with_choice("size", SIZE_NAMES, 1).with_text("label", "Snap to grid"),
        show,
    }]
}

/// Unchecked, checked and mixed boxes, enabled and disabled, then a long label. The enabled ones toggle.
fn show(ui: &mut Ui, args: &mut Args) {
    let size = size_arg(args.choice("size"));
    let label = args.text("label").to_owned();
    let state_id = ui.scope_id().with("checkbox-states");
    let initial = [CheckState::Unchecked, CheckState::Checked, CheckState::Mixed];
    let (mut states, mut long) = ui.data(|d| d.get_temp::<([CheckState; 3], bool)>(state_id).unwrap_or((initial, true)));
    Stack::new(Gap::Stack).show(ui, |ui| {
        for (i, state) in states.iter_mut().enumerate() {
            let _ = Checkbox::new(format!("{label} ({})", name(initial[i])), state).size(size).id_salt(("checkbox", i)).show(ui);
        }
        for (i, state) in initial.into_iter().enumerate() {
            let mut state = state;
            let _ =
                Checkbox::new(format!("Disabled ({})", name(state)), &mut state).size(size).enabled(false).id_salt(("checkbox-disabled", i)).show(ui);
        }
        let _ = Checkbox::bool(LONG_LABEL, &mut long).size(size).id_salt("checkbox-long").show(ui);
    });
    ui.data_mut(|d| d.insert_temp(state_id, (states, long)));
}

fn name(state: CheckState) -> &'static str {
    match state {
        CheckState::Unchecked => "unchecked",
        CheckState::Checked => "checked",
        CheckState::Mixed => "mixed",
    }
}
