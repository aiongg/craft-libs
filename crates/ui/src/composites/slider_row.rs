//! [`SliderRow`]: a caption and number field over a full-width slider.

use std::ops::RangeInclusive;

use craft_theme::{ControlSize, Srgba, TypeRole};
use egui::text::LayoutJob;
use egui::{AsIdSalt, Color32, IdSalt, Label, Layout, Rect, Response, Ui, UiBuilder, pos2, vec2};

use crate::controls::{NumberField, Slider, SliderScale};
use crate::direction;
use crate::edit::{EditEvent, EditPolicy, EditResponse};
use crate::localized::{LocaleNumbers, Unit};
use crate::primitives::Content;
use crate::story::{Level, Story, StoryArgs};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, role_format};
use crate::typed_story;

/// A slider row's sizes, type role and colour, from [`SliderRow::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SliderRowStyle {
    /// Width of the number field: `slider_row.field_width`.
    pub field_width: f32,
    /// Space after the row: `slider_row.gap`.
    pub gap: f32,
    /// The caption's type role: `label`.
    pub caption_role: TypeRole,
    /// The caption: `text.muted`.
    pub caption: Color32,
}

/// What a [`SliderRow`] reports in a frame: the slider's and the number field's edits. A slider
/// drag, a field scrub, a caption scrub and typing in the field are each one gesture of their
/// control, so each ends in one commit.
#[derive(Clone, Debug)]
#[must_use = "the host applies the edit events"]
pub struct SliderRowResponse {
    /// The slider's response.
    pub response: Response,
    pub slider: EditResponse<f64>,
    pub field: EditResponse<f64>,
}

impl SliderRowResponse {
    /// The frame's events in drawing order (the field's, then the slider's), for the host to apply
    /// one after the other.
    pub fn edits(self) -> impl Iterator<Item = EditEvent<f64>> {
        [self.field.event, self.slider.event].into_iter().flatten()
    }

    /// Whether the user submitted the field's value this frame.
    pub fn submitted(&self) -> bool {
        self.field.submitted
    }
}

/// A labelled slider row as in PhotoCraft's `slider_row` and `slider_row_prec`: on the first line
/// the caption at the reading start in `text.muted` and a [`NumberField`] at the reading end; on
/// the second the [`Slider`], the row's full width, optionally with a gradient track. The row
/// is followed by `slider_row.gap`.
///
/// ```
/// use craft_ui::composites::SliderRow;
/// use craft_ui::localized::{LocaleNumbers, Unit};
/// # egui::__run_test_ui(|ui| {
/// let numbers = LocaleNumbers::from_tag("en");
/// let mut opacity = 80.0;
/// for event in SliderRow::new("opacity", "Opacity", opacity, 0.0..=100.0, &numbers).unit(Unit::new("{n} %")).show(ui).edits() {
///     if let Some(value) = event.value() { opacity = *value; }
/// }
/// # });
/// ```
///
/// - **Number format:** without [`decimals`](SliderRow::decimals) the field shows up to two places
///   and steps by 0.01 when the range is 10 wide or less, and shows up to one place and steps by 1
///   otherwise. With `decimals(n)` it shows `n` places and steps by 10^-n.
/// - **Caption:** it scrubs the field across the row ([`NumberField::scrub_by`]) and names the
///   field and the slider (`labelled_by`). It is cut with an ellipsis before the field.
/// - **Edits:** the slider and the field report separately ([`SliderRowResponse`]); the host's
///   [`GestureHost`](crate::edit::GestureHost) orders them.
/// - **Keyboard:** the field and the slider are Tab stops, in that order.
#[must_use = "show the row with `show`"]
pub struct SliderRow<'a> {
    id_salt: IdSalt,
    label: String,
    value: f64,
    range: RangeInclusive<f64>,
    numbers: &'a LocaleNumbers,
    unit: Unit,
    decimals: Option<usize>,
    step: Option<f64>,
    track_colors: Option<Vec<Srgba>>,
    default: Option<f64>,
    wheel: Option<bool>,
    wheel_step: Option<f64>,
    scale: SliderScale,
    policy: EditPolicy,
    enabled: bool,
}

impl<'a> SliderRow<'a> {
    /// A row named `label` (the caption and the accessible name) showing the host's `value` in
    /// `range`; `id_salt` keeps the gestures and focus when the layout around it changes.
    pub fn new(
        id_salt: impl AsIdSalt,
        label: impl Into<String>,
        value: impl Into<f64>,
        range: RangeInclusive<f64>,
        numbers: &'a LocaleNumbers,
    ) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            value: value.into(),
            range,
            numbers,
            unit: Unit::default(),
            decimals: None,
            step: None,
            track_colors: None,
            default: None,
            wheel: None,
            wheel_step: None,
            scale: SliderScale::Linear,
            policy: EditPolicy::default(),
            enabled: true,
        }
    }

    /// The unit drawn in the field (`Unit::new("{n} %")`).
    pub fn unit(mut self, unit: Unit) -> Self {
        self.unit = unit;
        self
    }

    /// Places shown in the field, which also set its step (PhotoCraft's `slider_row_prec`).
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = Some(decimals);
        self
    }

    /// The step of the row: drags, taps, keys and the wheel on the slider move the value to its
    /// multiples from the range's start ([`Slider::step`]), and the field steps by it and
    /// moves typed values onto it ([`NumberField::snap_typed`]). Every value the row reports is
    /// on the step, so a host that stores it as reported keeps the gesture under way. The step
    /// replaces the one that [`decimals`](SliderRow::decimals) or the range sets; the places the
    /// field shows stay as they are.
    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(step).filter(|s| s.is_finite() && *s > 0.0);
        self
    }

    /// Draws the slider's track as a gradient through `stops` ([`Slider::track_colors`]).
    pub fn track_colors(mut self, stops: impl IntoIterator<Item = impl Into<Srgba>>) -> Self {
        self.track_colors = Some(stops.into_iter().map(Into::into).collect());
        self
    }

    /// The value a double-click on the slider's track or thumb sets ([`Slider::default_value`];
    /// PhotoCraft `RowGestures::reset`).
    pub fn default_value(mut self, value: impl Into<f64>) -> Self {
        self.default = Some(value.into());
        self
    }

    /// Whether the wheel over the slider steps it without the slider having the focus, ten steps
    /// with Shift ([`Slider::wheel_unfocused`]; PhotoCraft `RowGestures::wheel_step`, for
    /// dialogs). The step is the slider's key step, a hundredth of the range. Off by default:
    /// the wheel then steps the focused slider only. `wheel(false)` is off for the focused slider
    /// too ([`Slider::wheel`]): the slider never takes the wheel.
    pub fn wheel(mut self, on: bool) -> Self {
        self.wheel = Some(on);
        self
    }

    /// How far one wheel notch moves the value (Shift: ten notches), in the range's units, in
    /// place of the slider's key step, a hundredth of the range (PhotoCraft's non-default
    /// `RowGestures::wheel_step`). It sets the step only; [`wheel`](SliderRow::wheel) lets the
    /// wheel step the unfocused slider, and the focused slider is always stepped by it. The
    /// field's own wheel steps and the keys keep their steps.
    pub fn wheel_step(mut self, step: f64) -> Self {
        self.wheel_step = Some(step);
        self
    }

    /// A curved track: the thumb at fraction f of the track is `start + (end - start) * f^k`, so
    /// small values get fine steps (PhotoCraft `RowGestures::curve`). The field, typing, scrubbing
    /// and steps stay linear. `k` must be positive.
    pub fn curve(mut self, k: f64) -> Self {
        self.scale = SliderScale::Power(k);
        self
    }

    /// The edit policy of the slider and the field.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The style of a slider row in `tokens`.
    pub fn style(tokens: &UiTokens) -> SliderRowStyle {
        let t = &tokens.component.slider_row;
        SliderRowStyle { field_width: t.field_width, gap: t.gap, caption_role: TypeRole::Label, caption: Content::Muted.color(tokens).to_egui() }
    }

    /// Shows the row across the available width.
    pub fn show(self, ui: &mut Ui) -> SliderRowResponse {
        let tokens = Tokens::of(ui);
        let style = SliderRow::style(&tokens);
        let field_height = NumberField::style(&tokens, ControlSize::Md).field.height;
        let id = ui.make_persistent_id(self.id_salt);
        let rect = ui.available_rect_before_wrap();
        let width = rect.width();
        let rtl = direction::is_rtl(ui);

        let (_, line) = ui.allocate_space(vec2(width, field_height));
        let field_width = style.field_width.min(width);
        let (field_rect, caption_rect) = if rtl {
            let field = Rect::from_min_size(line.min, vec2(field_width, line.height()));
            (field, Rect::from_min_max(pos2(field.right() + ui.spacing().item_spacing.x, line.top()), line.max))
        } else {
            let field = Rect::from_min_size(pos2(line.right() - field_width, line.top()), vec2(field_width, line.height()));
            (field, Rect::from_min_max(line.min, pos2(field.left() - ui.spacing().item_spacing.x, line.bottom())))
        };

        // The caption is drawn first: the field scrubs by it, and AccessKit lists it first.
        let job = LayoutJob::single_section(self.label.clone(), role_format(&tokens, style.caption_role, style.caption));
        let caption = ui
            .scope_builder(UiBuilder::new().max_rect(caption_rect).layout(direction::row(ui)), |ui| {
                ui.add(Label::new(job).truncate().selectable(false))
            })
            .inner;

        let (places, step) = match self.decimals {
            Some(n) => (n..=n, 10f64.powi(-i32::try_from(n.min(6)).unwrap_or(6))),
            None if (self.range.end() - self.range.start()).abs() <= 10.0 => (0..=2, 0.01),
            None => (0..=1, 1.0),
        };
        let step = self.step.unwrap_or(step);
        let field = NumberField::new(id.with("field"), self.label.as_str(), self.value, self.numbers)
            .range(self.range.clone())
            .step(step)
            .snap_typed(self.step.is_some())
            .decimals(places)
            .unit(self.unit)
            .scrub_by(&caption)
            .policy(self.policy)
            .enabled(self.enabled);
        let layout = Layout::left_to_right(egui::Align::Center).with_main_justify(true);
        let field = field.show(&mut ui.new_child(UiBuilder::new().max_rect(field_rect).layout(layout)));

        let mut slider = Slider::new(id.with("slider"), self.label.as_str(), self.value, self.range.clone())
            .width(width)
            .wheel(self.wheel != Some(false))
            .wheel_unfocused(self.wheel == Some(true))
            .scale(self.scale)
            .policy(self.policy)
            .enabled(self.enabled);
        if let Some(step) = self.step {
            slider = slider.step(step);
        }
        if let Some(step) = self.wheel_step {
            slider = slider.wheel_step(step);
        }
        if let Some(value) = self.default {
            slider = slider.default_value(value);
        }
        if let Some(stops) = self.track_colors {
            slider = slider.track_colors(stops);
        }
        let slider = slider.show(ui);

        let ctx = ui.ctx().clone();
        for control in [field.response.id, slider.response.id] {
            if ctx.has_accesskit_node(control) {
                ctx.accesskit_node_builder(control, |node| {
                    // The node's own label would win over `labelled_by`.
                    node.clear_label();
                    node.push_labelled_by(caption.id.accesskit_id());
                });
            }
        }
        ui.add_space(style.gap);
        SliderRowResponse { response: slider.response.clone(), slider, field }
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["SliderRow"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "composites/slider-row/adjustments",
            Level::Composites,
            "SliderRow",
            "Adjustments",
            AdjustmentsArgs { disabled: false, width: 260.0 },
            show_adjustments
        ),
        typed_story!("composites/slider-row/wheel-step", Level::Composites, "SliderRow", "Wheel step", WheelStepArgs { step: 5.0 }, show_wheel_step),
    ]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct WheelStepArgs {
    #[arg(range = 0.5..=20.0)]
    step: f32,
}

/// A row whose wheel notch over the slider moves the value by `step`, the unfocused slider
/// included.
fn show_wheel_step(ui: &mut Ui, args: &mut WheelStepArgs) {
    let state_id = ui.scope_id().with("slider-row-wheel-step");
    let mut value = ui.data(|d| d.get_temp::<f64>(state_id)).unwrap_or(50.0);
    let numbers = LocaleNumbers::from_tag("en");
    ui.allocate_ui(vec2(260.0, 0.0), |ui| {
        ui.set_max_width(260.0);
        let row = SliderRow::new("wheel-step", "Opacity", value, 0.0..=1000.0, &numbers).wheel(true).wheel_step(f64::from(args.step));
        for event in row.show(ui).edits() {
            if let EditEvent::Update { value: v, .. } | EditEvent::Set { value: v, .. } | EditEvent::Commit { value: v, .. } = event {
                value = v;
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(state_id, value));
}

#[derive(StoryArgs, Debug, PartialEq)]
pub(super) struct AdjustmentsArgs {
    pub(super) disabled: bool,
    #[arg(range = 120.0..=400.0)]
    pub(super) width: f32,
}

/// Opacity in percent, Exposure with two places, a gradient Hue row and an Amount row, in a
/// panel `width` wide; `disabled` disables the Amount row.
pub(super) fn show_adjustments(ui: &mut Ui, args: &mut AdjustmentsArgs) {
    let (disabled, width) = (args.disabled, args.width.min(ui.available_width()));
    let state_id = ui.scope_id().with("slider-row-adjustments");
    let mut values = ui.data(|d| d.get_temp::<[f64; 4]>(state_id)).unwrap_or([80.0, 0.5, 210.0, 35.0]);
    let numbers = LocaleNumbers::from_tag("en");
    let hue = [
        Srgba::rgb(0.9, 0.2, 0.2),
        Srgba::rgb(0.9, 0.9, 0.2),
        Srgba::rgb(0.2, 0.9, 0.2),
        Srgba::rgb(0.2, 0.9, 0.9),
        Srgba::rgb(0.2, 0.2, 0.9),
        Srgba::rgb(0.9, 0.2, 0.9),
        Srgba::rgb(0.9, 0.2, 0.2),
    ];
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        let rows = [
            SliderRow::new("opacity", "Opacity", values[0], 0.0..=100.0, &numbers).unit(Unit::new("{n} %")),
            SliderRow::new("exposure", "Exposure", values[1], -20.0..=20.0, &numbers).decimals(2),
            SliderRow::new("hue", "Hue", values[2], 0.0..=360.0, &numbers).unit(Unit::new("{n}°")).track_colors(hue),
            SliderRow::new("amount", "Amount", values[3], 0.0..=100.0, &numbers).enabled(!disabled),
        ];
        for (value, row) in values.iter_mut().zip(rows) {
            for event in row.show(ui).edits() {
                if let EditEvent::Update { value: v, .. } | EditEvent::Set { value: v, .. } | EditEvent::Commit { value: v, .. } = event {
                    *value = v;
                }
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(state_id, values));
}
