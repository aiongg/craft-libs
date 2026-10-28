//! [`NumberField`]: a number in a sunken box, written and read in the user's region's digits
//! ([`LocaleNumbers`]), typed through [`behavior::Typing`](crate::behavior::Typing), scrubbed,
//! stepped and set on a pop-up slider.

use std::collections::VecDeque;
use std::ops::RangeInclusive;

use craft_theme::{ControlSize, TypeRole};
use egui::accesskit::{Action, ActionData, Role};
use egui::emath::GuiRounding as _;
use egui::text::CCursorRange;
use egui::text_edit::TextEditState;
use egui::{
    Align2, AsIdSalt, Color32, Context, CursorIcon, Event, EventFilter, FontId, Id, IdSalt, Key, Modifiers, PointerButton, Popup, PopupCloseBehavior,
    Rect, RectAlign, Response, Sense, Shape, Ui, pos2, vec2,
};

use super::slider::{Scale, Track, labelled, pointer, wheel, with_values};
use super::text_field::{BoxState, field_box};
use super::{IconButton, SIZE_NAMES, Slider, TextField, TextFieldStyle, TooltipExt as _, icon_size, size_arg};
use crate::behavior::{Codec, NotAValue, Typed, Typing, TypingLook};
use crate::edit::{self, CancelReason, EditEvent, EditPolicy, EditResponse, GestureHost, GestureId, UndoMarks};
use crate::icons::{self, Icon};
use crate::localized::{LocaleNumbers, Unit, first_field_id, incomplete, round_half_away, stepped};
use crate::primitives::{Content, Corner, Gap, Interactive, Row, Stack, StateColors, Surface, SurfaceKind, Text};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, margin_x};

/// A number field's colours beyond a text field's, from [`NumberField::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct NumberFieldStyle {
    /// The box, the text and the selection: [`TextField::style`], `field.number_width` wide.
    pub field: TextFieldStyle,
    /// The unit: `field.unit.rest` and `field.unit.disabled`.
    pub unit: Color32,
    pub unit_disabled: Color32,
    /// The scrub label or icon: `field.scrub.rest` and `field.scrub.disabled`.
    pub scrub: Color32,
    pub scrub_disabled: Color32,
}

/// The text a blank field shows: a mixed value, or one that isn't a finite number.
const BLANK: &str = "\u{2014}";

/// Steps a scrub moves per point of horizontal drag.
const STEPS_PER_POINT: f64 = 1.0;

/// An app's reader for text that isn't a plain number.
type Reader<'a> = dyn Fn(&str) -> Option<f64> + 'a;

/// The handle at the field's start that a drag scrubs the value with.
#[derive(Clone, Debug)]
enum Handle {
    Label(String),
    Icon(Icon),
}

/// A number the host owns, in a text field:
///
/// ```
/// use craft_ui::controls::NumberField;
/// use craft_ui::edit::EditEvent;
/// use craft_ui::localized::{LocaleNumbers, Unit};
/// # egui::__run_test_ui(|ui| { let mut width = 1920.0;
/// let numbers = LocaleNumbers::from_tag("de");
/// let edit = NumberField::new("width", "Width", width, &numbers)
///     .range(1.0..=30_000.0)
///     .unit(Unit::new("{n} px"))
///     .scrub_label("W")
///     .show(ui);
/// match edit.event {
///     Some(EditEvent::Update { value, .. } | EditEvent::Set { value, .. } | EditEvent::Commit { value, .. }) => width = value,
///     // Begin and Cancel go to the document's undo history (`edit::GestureHost`).
///     _ => {}
/// }
/// # });
/// ```
///
/// Every way of changing the number reports through the edit lifecycle ([`crate::edit`]), one
/// gesture at a time:
///
/// - **Typing** is one gesture that reports no updates (`-` and `1,` aren't numbers yet). Enter
///   and the focus leaving commit the text read back: [`LocaleNumbers::parse`] with the unit
///   taken off, then the app's [`or_parse`](NumberField::or_parse); rounded to the most decimals
///   shown (half away from zero) and clamped to the range. The text as the field shows it stands
///   for the value itself, decimals the field doesn't show included. Text that can't become a
///   number gets the invalid edge and the reason as tooltip as it is typed (`-` and `1,` don't).
///   Text that isn't a number commits nothing: it stays in the field, marked, until the user
///   fixes it or Esc restores the value. Esc during a run cancels it.
/// - **Scrubbing**: a horizontal drag on the [scrub label](NumberField::scrub_label) or
///   [icon](NumberField::scrub_icon) inside the field, or on a label outside it
///   ([`scrub_by`](NumberField::scrub_by)), moves a step per point; Shift ten steps, Alt a tenth
///   as fast. A drag is one gesture: Esc cancels it, a still pointer leaves the value exactly as
///   it was; once the gesture is cancelled (Esc, an undo, the host's own change) the drag does
///   nothing more until the release. A drag on the text selects it.
/// - **Keys and the wheel**: Up and Down step the focused field, as does the wheel over it;
///   Shift steps ten, Cmd/Ctrl a tenth where the decimals show it. A plain or fine step goes to
///   the next multiple of its size (by 1, 5.1 goes up to 6 and down to 5). Each frame's steps are
///   one [`Set`](EditEvent::Set); during a typing run they step from the typed value and the
///   run's commit carries the result. Text that isn't a number takes no step: an arrow ends the
///   run as Enter does (the text stays, marked) and the wheel scrolls the panel.
/// - **Pop-up slider** ([`popup_slider`](NumberField::popup_slider)): a ▾ at the field's end
///   opens a [`Slider`] below it; pressing the ▾ and dragging moves the value at once, as far as
///   the slider would, and closes it on release. A drag on either is one gesture.
/// - **Mixed** ([`mixed`](NumberField::mixed)): blank with a `—` placeholder; a typed number
///   commits; scrubs and steps start from the host's value.
///
/// A value that isn't a finite number shows blank too. The digits have tabular figures, so the
/// text keeps its width while scrubbing. The field keeps its left-to-right geometry right to left
/// ([`Unmirrored::Number`](crate::direction::Unmirrored::Number)): digits align right, the unit
/// after them, the scrub handle at the left.
///
/// AccessKit: a `SpinButton` named `label`, with its value (and the value text with its unit),
/// bounds and step, and Increment, Decrement and SetValue.
#[must_use = "show the field with `show`"]
pub struct NumberField<'a> {
    id_salt: IdSalt,
    label: String,
    value: f64,
    numbers: &'a LocaleNumbers,
    range: RangeInclusive<f64>,
    step: f64,
    decimals: RangeInclusive<usize>,
    unit: Unit,
    fallback: Option<Box<Reader<'a>>>,
    handle: Option<Handle>,
    scrub_by: Option<Response>,
    popup: Option<String>,
    mixed: bool,
    policy: EditPolicy,
    size: ControlSize,
    enabled: bool,
    invalid: bool,
}

impl<'a> NumberField<'a> {
    /// A field named `label` (its accessible name) showing the host's `value` in the region's
    /// digits; `id_salt` keeps its text, focus and gesture when the layout around it changes.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, value: f64, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            value,
            numbers,
            range: f64::NEG_INFINITY..=f64::INFINITY,
            step: 1.0,
            decimals: 0..=6,
            unit: Unit::default(),
            fallback: None,
            handle: None,
            scrub_by: None,
            popup: None,
            mixed: false,
            policy: EditPolicy::default(),
            size: ControlSize::Md,
            enabled: true,
            invalid: false,
        }
    }

    /// The values the field takes, read in ascending order; unbounded by default. A typed,
    /// scrubbed or stepped value is clamped to it; the host's value is shown as it is.
    pub fn range(mut self, range: RangeInclusive<f64>) -> Self {
        self.range = range;
        self
    }

    /// One key press, wheel notch or point of scrubbing; 0 turns stepping and scrubbing off.
    /// Default 1.
    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// The fewest and most digits shown after the decimal mark. Default `0..=6`.
    pub fn decimals(mut self, decimals: RangeInclusive<usize>) -> Self {
        self.decimals = decimals;
        self
    }

    /// The unit drawn around the number; typed text may include it.
    pub fn unit(mut self, unit: Unit) -> Self {
        self.unit = unit;
        self
    }

    /// A reader for text that isn't a plain number (arithmetic such as `5+5`, a timecode), tried
    /// after the locale's. Arrows after typed arithmetic step from its result.
    pub fn or_parse(mut self, parser: impl Fn(&str) -> Option<f64> + 'a) -> Self {
        self.fallback = Some(Box::new(parser));
        self
    }

    /// A short label at the field's start ("W", "X") that scrubs the value.
    pub fn scrub_label(mut self, text: impl Into<String>) -> Self {
        self.handle = Some(Handle::Label(text.into()));
        self
    }

    /// An icon at the field's start (an angle) that scrubs the value.
    pub fn scrub_icon(mut self, icon: Icon) -> Self {
        self.handle = Some(Handle::Icon(icon));
        self
    }

    /// Makes `label`, drawn before the field, scrub it: a scrubby label, as in a property row.
    /// The field adds drag and click sense to it, not focus (Tab goes to the field); a click on
    /// it focuses the field. A label whose field isn't drawn stays plain.
    pub fn scrub_by(mut self, label: &Response) -> Self {
        self.scrub_by = Some(label.clone());
        self
    }

    /// A ▾ at the field's end that opens a [`Slider`] over the range. `name` (translated by the
    /// app) is the ▾'s tooltip and accessible name.
    ///
    /// The slider needs a finite [`range`](NumberField::range): without one this is a debug
    /// assertion, and release builds draw no ▾.
    pub fn popup_slider(mut self, name: impl Into<String>) -> Self {
        self.popup = Some(name.into());
        self
    }

    /// The selected objects have different values: the field is blank with a `—` placeholder,
    /// and `value` is only where scrubs and steps start.
    pub fn mixed(mut self, mixed: bool) -> Self {
        self.mixed = mixed;
        self
    }

    /// How scrubs and drags report ([`EditPolicy`]); a typing run reports no updates whatever
    /// this says. The default reports every change at once and cancels on Esc.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
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

    /// Draws the invalid edge and reports the value invalid (the host rejects it).
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    pub fn style(tokens: &UiTokens, size: ControlSize) -> NumberFieldStyle {
        let f = &tokens.component.field;
        let mut field = TextField::style(tokens, size);
        field.width = f.number_width.get(size);
        NumberFieldStyle {
            field,
            unit: f.unit.rest.to_egui(),
            unit_disabled: f.unit.disabled.to_egui(),
            scrub: f.scrub.rest.to_egui(),
            scrub_disabled: f.scrub.disabled.to_egui(),
        }
    }

    /// Shows the field and returns this frame's edit. The response covers the box.
    pub fn show(mut self, ui: &mut Ui) -> EditResponse<f64> {
        let bounds = ordered(&self.range);
        if !(bounds.0.is_finite() && bounds.1.is_finite()) {
            debug_assert!(self.popup.is_none(), "a pop-up slider needs a finite range; set .range first");
            self.popup = None;
        }
        let tokens = Tokens::of(ui);
        let style = NumberField::style(&tokens, self.size);
        let enabled = self.enabled && ui.is_enabled();
        let id = Interactive::id_in(ui, self.id_salt);
        let host = (!self.mixed && self.value.is_finite()).then_some(self.value);
        let codec = Numbers { numbers: self.numbers, decimals: self.decimals.clone(), unit: &self.unit, fallback: self.fallback.as_deref(), bounds };
        let font = tokens.font(style.field.role);
        let parts = Parts::new(ui, &tokens, &style, &self, font.clone(), enabled);
        let stepping = enabled && self.step != 0.0 && self.step.is_finite();

        if enabled && !ui.is_sizing_pass() && ui.data_mut(|d| d.remove_temp::<bool>(first_field_id())).is_some() {
            ui.memory_mut(|m| m.request_focus(id));
            select_all(ui.ctx(), id, &codec.format(&host));
        }
        // Before `TextEdit`, which would move the caret on them; an input method's own while it
        // composes.
        let arrows = if stepping && ui.memory(|m| m.has_focus(id)) && !Typing::composing(ui, id) { take_arrows(ui) } else { Vec::new() };

        let width = if ui.layout().horizontal_justify() { ui.available_width() } else { style.field.width };
        let look = TypingLook {
            size: vec2(width, style.field.height),
            margin: margin_x(parts.start, parts.end),
            align: Align2::RIGHT_CENTER,
            font,
            text: if enabled { style.field.text } else { style.field.text_disabled },
            placeholder: if enabled { style.field.placeholder } else { style.field.placeholder_disabled },
            selection: style.field.selection,
            selected_text: style.field.selected_text,
            caret: style.field.caret,
        };
        let typing = Typing::with_codec(id, &self.label, host, codec.clone())
            .placeholder(BLANK)
            .policy(self.policy)
            .enabled(self.enabled)
            .invalid(self.invalid)
            .role(Role::SpinButton)
            .event_filter(EventFilter { vertical_arrows: true, ..EventFilter::default() })
            .tabular(true);
        // The box goes under the text, which `TextEdit` paints as it runs.
        let under = ui.painter().add(Shape::Noop);
        let mut typed = typing.show(ui, &look);
        let rect = typed.response().rect;
        let input = Input {
            id,
            base: if self.value.is_finite() { self.value } else { 0.0_f64.clamp(bounds.0, bounds.1) },
            step: self.step,
            modifiers: ui.input(|i| i.modifiers),
            codec,
        };

        if stepping {
            let mut sources = Vec::new();
            if let Some(handle) = parts.handle_rect(rect) {
                sources.push(ui.interact(handle, id.with("craft_ui::number_field::scrub"), Sense::CLICK | Sense::DRAG));
            }
            if let Some(label) = &self.scrub_by {
                sources.push(label.interact(Sense::CLICK | Sense::DRAG));
            }
            input.scrub(ui, &mut typed, &sources);
        }
        let popup_open = self.popup.is_some() && input.popup(ui, &mut typed, &self, rect, parts.popup_rect(rect), enabled);
        let response = typed.response().clone();
        let focused = enabled && response.has_focus();
        input.step(ui, &mut typed, &arrows, stepping, enabled, *self.decimals.end());

        let invalid = self.invalid || typed.rejected().is_some();
        report(ui.ctx(), &typed, bounds, &self.unit, stepping.then_some(self.step));
        ui.painter().set(under, Shape::Vec(field_box(ui, &style.field, response.rect, BoxState::of(&response, enabled, invalid))));
        parts.paint(ui, &self, rect, typed.text_rect(), !typed.text().is_empty());
        if let Some(reason) = typed.rejected().map(str::to_owned) {
            if focused && !popup_open {
                response.show_tooltip_text(reason.as_str());
            }
            let _ = response.clone().tooltip(reason);
        }
        typed.end_as(ui, |value| value.unwrap_or(self.value))
    }
}

/// The ▾'s size class: a small ghost icon button inside the field.
const POPUP_SIZE: ControlSize = ControlSize::Sm;

/// What the field's own input (scrubs, the ▾, steps) works with in a frame.
struct Input<'n> {
    id: Id,
    /// Where scrubs and steps start in a blank field.
    base: f64,
    step: f64,
    modifiers: Modifiers,
    codec: Numbers<'n>,
}

impl<'n> Input<'n> {
    /// Drags on the scrub `sources`: a step a point (Shift ten, Alt a tenth as fast); a click
    /// focuses the field.
    fn scrub(&self, ui: &Ui, typed: &mut Typed<Numbers<'n>>, sources: &[Response]) {
        let key = self.id.with("craft_ui::number_field::scrubbing");
        for drag in sources {
            if drag.hovered() || drag.dragged() {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
            }
            if drag.clicked() {
                ui.memory_mut(|m| m.request_focus(self.id));
                select_all(ui.ctx(), self.id, typed.text());
            }
            if drag.drag_started_by(PointerButton::Primary)
                && let Some((_, origin)) = pointer(ui, drag)
            {
                // A typing run ends as the focus leaves, its text read before the first step.
                ui.memory_mut(|m| m.surrender_focus(self.id));
                ui.data_mut(|d| d.insert_temp(key, Scrub { press: Press::default(), last_x: origin.x, steps: 0.0 }));
            }
            if drag.dragged_by(PointerButton::Primary)
                && let Some((pos, _)) = pointer(ui, drag)
                && let Some(mut scrub) = ui.data(|d| d.get_temp::<Scrub>(key))
                && scrub.press.under_way(typed)
            {
                let m = self.modifiers;
                let rate = if m.shift {
                    10.0
                } else if m.alt {
                    0.1
                } else {
                    1.0
                };
                scrub.steps += f64::from(pos.x - scrub.last_x) * STEPS_PER_POINT * rate;
                scrub.last_x = pos.x;
                let n = scrub.steps.trunc();
                scrub.steps -= n;
                let gesture = typed.gesture();
                if n != 0.0 {
                    let from = gesture.shown().unwrap_or(self.base);
                    gesture.update(Some(self.codec.settle(from + n * self.step)), m);
                }
                ui.data_mut(|d| d.insert_temp(key, scrub));
            }
            if drag.drag_stopped()
                && let Some(scrub) = ui.data_mut(|d| d.remove_temp::<Scrub>(key))
            {
                scrub.press.release(typed);
            }
        }
    }

    /// The ▾ at `at` in the box `rect`, and its slider below the box: a press and drag on the ▾
    /// moves the value as far as the slider's thumb would; the slider's changes are the field's.
    /// Whether the slider is open.
    fn popup(&self, ui: &mut Ui, typed: &mut Typed<Numbers<'n>>, field: &NumberField<'_>, rect: Rect, at: Rect, enabled: bool) -> bool {
        let name = field.popup.as_deref().unwrap_or_default();
        let tokens = Tokens::of(ui);
        let popup_id = self.id.with("craft_ui::number_field::popup");
        let open = Popup::is_id_open(ui.ctx(), popup_id);
        let button = IconButton::style(&tokens, POPUP_SIZE);
        let arrow = Interactive::with_id(self.id.with("craft_ui::number_field::popup_button"), Role::Button, name)
            .sense(Sense::CLICK | Sense::DRAG)
            .colors(StateColors { fill: button.fill, border: None })
            .corner(Corner::Control)
            .enabled(enabled)
            .expanded(open)
            .interact_at(ui, at)
            .finish(ui);
        let color = super::button::content_color(ui, &button.text, &arrow, true);
        let icon = Rect::from_center_size(arrow.rect.center(), vec2(button.icon, button.icon)).round_to_pixels(ui.pixels_per_point());
        icons::paint(ui, icon, Icon::ChevronDown, icon_size(POPUP_SIZE), color);
        let arrow = arrow.tooltip(name);
        let (lo, hi) = self.codec.bounds;
        let scale = Scale::new(&(lo..=hi), (self.step > 0.0).then_some(self.step));
        let arrow_press = self.id.with("craft_ui::number_field::popup_press");
        if arrow.drag_started_by(PointerButton::Primary) {
            Popup::open_id(ui.ctx(), popup_id);
            ui.memory_mut(|m| m.surrender_focus(self.id));
            ui.data_mut(|d| d.insert_temp(arrow_press, Press::default()));
        }
        if arrow.dragged_by(PointerButton::Primary)
            && let Some((pos, origin)) = pointer(ui, &arrow)
            && let Some(mut press) = ui.data(|d| d.get_temp::<Press>(arrow_press))
            && press.under_way(typed)
        {
            let slider = Slider::style(&tokens, field.size);
            let track = Track::new(Rect::from_min_size(at.min, vec2(slider.width, slider.height)), &slider, scale);
            let gesture = typed.gesture();
            if let Some(start) = gesture.start().copied() {
                let dx = pos.x - origin.x;
                // A still pointer leaves the value as it was.
                let to = if dx == 0.0 {
                    start
                } else {
                    Some(self.codec.settle(scale.snap(scale.place(start.unwrap_or(self.base)) + track.value_per(dx))))
                };
                gesture.update(to, self.modifiers);
            }
            ui.data_mut(|d| d.insert_temp(arrow_press, press));
        }
        if arrow.drag_stopped() {
            Popup::close_id(ui.ctx(), popup_id);
            if let Some(press) = ui.data_mut(|d| d.remove_temp::<Press>(arrow_press)) {
                press.release(typed);
            }
        }
        let mut slider = Slider::new(self.id.with("craft_ui::number_field::slider"), name, typed.value().unwrap_or(self.base), lo..=hi)
            .size(field.size)
            .enabled(enabled);
        if self.step > 0.0 {
            slider = slider.step(self.step);
        }
        let popup = Popup::from_toggle_button_response(&arrow)
            .id(popup_id)
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .anchor(rect)
            .align(RectAlign::BOTTOM_END)
            .align_alternatives(&[RectAlign::TOP_END]);
        // Inside a scope the slider reports updates only, which the field's gesture carries: a
        // drag on it as one gesture, a tap or a key as one change.
        let Some(inner) = popup.show(|ui| edit::scope(ui, |ui| slider.show(ui)).inner) else { return false };
        let edit = inner.inner;
        let drag = &edit.response;
        let slider_press = self.id.with("craft_ui::number_field::slider_press");
        if drag.drag_started_by(PointerButton::Primary) {
            ui.data_mut(|d| d.insert_temp(slider_press, Press::default()));
        }
        let mut press = ui.data(|d| d.get_temp::<Press>(slider_press));
        let dragging = press.is_some() && (drag.dragged() || drag.drag_stopped());
        match edit.event.as_ref().and_then(EditEvent::value) {
            Some(&value) if typed.is_typing() || !dragging => typed.commit_with(ui, Some(self.codec.settle(value))),
            Some(&value) => {
                if let Some(press) = &mut press
                    && press.under_way(typed)
                {
                    typed.gesture().update(Some(self.codec.settle(value)), self.modifiers);
                }
            }
            None => {}
        }
        if let Some(press) = press {
            if drag.drag_stopped() {
                ui.data_mut(|d| d.remove::<Press>(slider_press));
                press.release(typed);
            } else {
                ui.data_mut(|d| d.insert_temp(slider_press, press));
            }
        }
        true
    }

    /// This frame's `arrows`, wheel notches and screen-reader requests, as one change: during a
    /// drag, none. Text that isn't a number takes no step: the typing run ends as on Enter, the
    /// text stays rejected, and the wheel is the panel's.
    fn step(&self, ui: &Ui, typed: &mut Typed<Numbers<'n>>, arrows: &[(Modifiers, f64)], stepping: bool, enabled: bool, places: usize) {
        let response = typed.response().clone();
        let dragging = typed.gesture().is_active() && !typed.is_typing();
        let focused = enabled && response.has_focus();
        let reads = typed.reads_as_value();
        let mut to = None;
        if stepping {
            let notches = wheel(ui, self.id, 1.0, 10.0, focused && reads && response.contains_pointer(), dragging);
            let requests = ui.input(|i| {
                i.num_accesskit_action_requests(self.id, Action::Increment) as f64
                    - i.num_accesskit_action_requests(self.id, Action::Decrement) as f64
            });
            let steps = !arrows.is_empty() || notches != 0.0 || requests != 0.0;
            if steps && !reads {
                typed.commit();
            } else if steps {
                let from = typed.value().unwrap_or(self.base);
                let v = arrows.iter().fold(from, |v, &(modifiers, n)| stepped(v, self.step, places, modifiers, n));
                to = Some([notches, requests].into_iter().filter(|n| *n != 0.0).fold(v, |v, n| stepped(v, self.step, places, Modifiers::NONE, n)));
            }
        }
        if enabled {
            let set = ui.input(|i| {
                i.accesskit_action_requests(self.id, Action::SetValue)
                    .filter_map(|r| match r.data {
                        Some(ActionData::NumericValue(v)) if v.is_finite() => Some(v),
                        _ => None,
                    })
                    .last()
            });
            to = set.or(to);
        }
        if let Some(v) = to
            && !dragging
        {
            let v = self.codec.settle(v);
            typed.commit_with(ui, Some(v));
            if focused {
                select_all(ui.ctx(), self.id, &self.codec.format(&Some(v)));
            }
        }
    }
}

/// A press on a scrub handle, the ▾ or the pop-up slider, until its release.
#[derive(Clone, Copy, Debug, Default)]
struct Press {
    /// The gesture the press began; none before it begins.
    gesture: Option<GestureId>,
}

impl Press {
    /// Begins the press's gesture, once and when no typing run is under way (a run the press
    /// ended commits first); whether that gesture is under way. One that has ended (a cancel)
    /// leaves the press with nothing to do until its release.
    fn under_way(&mut self, typed: &mut Typed<Numbers<'_>>) -> bool {
        if self.gesture.is_none() && !typed.is_typing() {
            let gesture = typed.gesture();
            gesture.begin();
            self.gesture = gesture.id();
        }
        self.gesture.is_some() && typed.gesture().id() == self.gesture
    }

    /// Commits the press's gesture on release, while it is under way.
    fn release(self, typed: &mut Typed<Numbers<'_>>) {
        if self.gesture.is_some() && typed.gesture().id() == self.gesture {
            typed.gesture().commit();
        }
    }
}

/// A scrub under way.
#[derive(Clone, Copy, Debug, Default)]
struct Scrub {
    press: Press,
    /// The pointer's x when the steps were last counted (the press, at first: the points moved
    /// before egui starts the drag count too).
    last_x: f32,
    /// Steps moved and not yet taken.
    steps: f64,
}

/// `range` in ascending order, a bound that is no number left open.
fn ordered(range: &RangeInclusive<f64>) -> (f64, f64) {
    let lo = if range.start().is_nan() { f64::NEG_INFINITY } else { *range.start() };
    let hi = if range.end().is_nan() { f64::INFINITY } else { *range.end() };
    if lo <= hi { (lo, hi) } else { (hi, lo) }
}

/// How a number field writes its value and reads typed text: `None` is a blank field (mixed
/// values, or a host value that isn't a finite number).
#[derive(Clone)]
struct Numbers<'n> {
    numbers: &'n LocaleNumbers,
    decimals: RangeInclusive<usize>,
    unit: &'n Unit,
    fallback: Option<&'n Reader<'n>>,
    bounds: (f64, f64),
}

impl Numbers<'_> {
    /// `value` rounded to the most decimals shown and clamped to the range.
    fn settle(&self, value: f64) -> f64 {
        round_half_away(value, *self.decimals.end()).clamp(self.bounds.0, self.bounds.1)
    }
}

impl Codec for Numbers<'_> {
    type Value = Option<f64>;

    fn format(&self, value: &Option<f64>) -> String {
        value.filter(|v| v.is_finite()).map_or_else(String::new, |v| self.numbers.format(v, self.decimals.clone()))
    }

    fn parse(&self, text: &str, start: &Option<f64>) -> Result<Option<f64>, NotAValue> {
        let bare = self.unit.strip(text);
        // The text as shown stands for the value itself, with the decimals it doesn't show.
        if start.is_some() && bare == self.format(start) {
            return Ok(*start);
        }
        // A mixed field committed blank changes nothing.
        if start.is_none() && bare.is_empty() {
            return Ok(None);
        }
        let read = self.numbers.parse(bare);
        match read.as_ref().ok().copied().or_else(|| self.fallback.and_then(|f| f(text))).filter(|v| v.is_finite()) {
            Some(v) => Ok(Some(self.settle(v))),
            None => {
                let error = read.err().unwrap_or(craft_i18n::NumberError::TooLarge);
                Err(NotAValue { reason: self.numbers.describe(error), partial: incomplete(bare, error) })
            }
        }
    }
}

/// Where a field's parts go: the room at each end of the text and the handle's width.
struct Parts {
    pad: f32,
    gap: f32,
    /// Width of the scrub label or icon; 0 without one.
    handle: f32,
    prefix: f32,
    suffix: f32,
    /// The ▾ button's side and its inset from the box's edge; 0 without one.
    popup: f32,
    popup_inset: f32,
    /// Room before and after the text.
    start: f32,
    end: f32,
    font: FontId,
    /// The handle's and the unit's colours, enabled or not.
    scrub: Color32,
    unit: Color32,
}

impl Parts {
    fn new(ui: &Ui, tokens: &UiTokens, style: &NumberFieldStyle, field: &NumberField<'_>, font: FontId, enabled: bool) -> Self {
        let f = &style.field;
        let width = |text: &str| {
            if text.is_empty() {
                0.0
            } else {
                ui.fonts_mut(|fonts| fonts.layout_no_wrap(text.to_owned(), font.clone(), Color32::PLACEHOLDER).size().x)
            }
        };
        let handle = match &field.handle {
            Some(Handle::Label(text)) => width(text),
            Some(Handle::Icon(_)) => f.icon,
            None => 0.0,
        };
        let (prefix, suffix) = (width(field.unit.prefix()), width(field.unit.suffix()));
        let (popup, popup_inset) = match field.popup {
            Some(_) => {
                let side = IconButton::style(tokens, POPUP_SIZE).side;
                (side, ((f.height - side) / 2.0).max(0.0))
            }
            None => (0.0, 0.0),
        };
        let start = f.pad_x + if handle > 0.0 { handle + f.gap } else { 0.0 } + prefix;
        let end = suffix + if popup > 0.0 { popup_inset + popup + f.gap } else { f.pad_x };
        let (scrub, unit) = if enabled { (style.scrub, style.unit) } else { (style.scrub_disabled, style.unit_disabled) };
        Self { pad: f.pad_x, gap: f.gap, handle, prefix, suffix, popup, popup_inset, start, end, font, scrub, unit }
    }

    /// The scrub handle's hit area: from the box's start to halfway into the gap after it.
    fn handle_rect(&self, rect: Rect) -> Option<Rect> {
        (self.handle > 0.0).then(|| Rect::from_min_max(rect.min, pos2(rect.left() + self.pad + self.handle + self.gap / 2.0, rect.bottom())))
    }

    fn popup_rect(&self, rect: Rect) -> Rect {
        let x = rect.right() - self.popup_inset - self.popup / 2.0;
        Rect::from_center_size(pos2(x, rect.center().y), vec2(self.popup, self.popup))
    }

    /// The scrub handle in `rect`, and the unit beside the `text` while there is text.
    fn paint(&self, ui: &Ui, field: &NumberField<'_>, rect: Rect, text: Rect, has_text: bool) {
        let painter = ui.painter();
        let y = rect.center().y;
        match &field.handle {
            Some(Handle::Label(label)) => {
                painter.text(pos2(rect.left() + self.pad, y), Align2::LEFT_CENTER, label, self.font.clone(), self.scrub);
            }
            Some(Handle::Icon(icon)) => {
                let at = Rect::from_min_size(pos2(rect.left() + self.pad, y - self.handle / 2.0), vec2(self.handle, self.handle));
                icons::paint(ui, at.round_to_pixels(ui.pixels_per_point()), *icon, icon_size(field.size), self.scrub);
            }
            None => {}
        }
        if !has_text {
            return;
        }
        let y = text.center().y;
        if self.prefix > 0.0 {
            painter.text(pos2(text.left(), y), Align2::RIGHT_CENTER, field.unit.prefix(), self.font.clone(), self.unit);
        }
        if self.suffix > 0.0 {
            painter.text(pos2(text.right(), y), Align2::LEFT_CENTER, field.unit.suffix(), self.font.clone(), self.unit);
        }
    }
}

/// The arrow-key steps of this frame, taken from the input: their modifiers and the steps up
/// (down when negative).
fn take_arrows(ui: &Ui) -> Vec<(Modifiers, f64)> {
    let mut steps = Vec::new();
    ui.input_mut(|i| {
        i.events.retain(|event| match event {
            Event::Key { key: key @ (Key::ArrowUp | Key::ArrowDown), pressed: true, modifiers, .. } => {
                steps.push((*modifiers, if *key == Key::ArrowUp { 1.0 } else { -1.0 }));
                false
            }
            _ => true,
        });
    });
    steps
}

/// Selects all of `text` in the text edit `id` from the next pass, so typing replaces it.
fn select_all(ctx: &Context, id: Id, text: &str) {
    let mut state = TextEditState::load(ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::select_all_str(text)));
    state.store(ctx, id);
}

/// The spin button's numbers for AccessKit: value, bounds, step (`None` without stepping), the
/// value text with its unit, and the requests it takes.
fn report(ctx: &Context, typed: &Typed<Numbers<'_>>, (lo, hi): (f64, f64), unit: &Unit, step: Option<f64>) {
    let (value, text) = (*typed.value(), typed.text());
    ctx.accesskit_node_builder(typed.response().id, |node| {
        match value {
            Some(v) => node.set_numeric_value(v),
            None => node.clear_numeric_value(),
        }
        if lo.is_finite() {
            node.set_min_numeric_value(lo);
        }
        if hi.is_finite() {
            node.set_max_numeric_value(hi);
        }
        if let Some(step) = step {
            node.set_numeric_value_step(step);
            if value.is_none_or(|v| v < hi) {
                node.add_action(Action::Increment);
            }
            if value.is_none_or(|v| v > lo) {
                node.add_action(Action::Decrement);
            }
        }
        if !text.is_empty() {
            node.set_value(format!("{}{text}{}", unit.prefix(), unit.suffix()));
        }
    });
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Number field"];

const LOCALES: &[&str] = &["en", "de", "ar-EG"];

pub(crate) fn stories() -> Vec<Story> {
    let args = || Args::new().with_choice("size", SIZE_NAMES, 1).with_choice("locale", LOCALES, 0);
    let story = |id, name, show| Story { id, level: Level::Controls, component: "Number field", name, args, show };
    vec![
        story("controls/number-field/states", "States", show_states),
        story("controls/number-field/scrub", "Scrub", show_scrub),
        story("controls/number-field/right-to-left", "Right to left", show_right_to_left),
        Story {
            id: "controls/number-field/event-log",
            level: Level::Controls,
            component: "Number field",
            name: "Event log",
            args: || Args::new().with_choice("update rate", edit::RATES, 0).with_bool("cancellable", true).with_bool("on release only", false),
            show: show_event_log,
        },
    ]
}

fn numbers(args: &Args) -> LocaleNumbers {
    LocaleNumbers::from_tag(args.choice("locale"))
}

/// A value kept between frames, and the value text a row shows beside its field.
fn row(ui: &mut Ui, label: &str, value: f64, field: impl FnOnce(&mut Ui)) {
    labelled(ui, label, &format!("{value}"), field);
}

/// Plain, with a unit, mixed, invalid, with a pop-up slider, and disabled.
fn show_states(ui: &mut Ui, args: &mut Args) {
    let size = size_arg(args.choice("size"));
    let numbers = numbers(args);
    let sum = |text: &str| text.split_once('+').and_then(|(a, b)| Some(a.trim().parse::<f64>().ok()? + b.trim().parse::<f64>().ok()?));
    with_values(ui, "number-field-states", [42.0, 12.5, 30.0, 7.0, 100.0], |ui, values| {
        Stack::new(Gap::Stack).show(ui, |ui| {
            let [plain, unit, mixed, invalid, opacity] = values;
            row(ui, "Plain", *plain, |ui| {
                let edit = NumberField::new("number-field-plain", "Plain", *plain, &numbers).size(size).or_parse(sum).show(ui);
                super::slider::apply(&edit, plain);
            });
            row(ui, "With unit", *unit, |ui| {
                let edit = NumberField::new("number-field-unit", "Gap", *unit, &numbers)
                    .range(0.0..=100.0)
                    .step(0.5)
                    .decimals(0..=2)
                    .unit(Unit::new("{n} mm"))
                    .size(size)
                    .show(ui);
                super::slider::apply(&edit, unit);
            });
            let mixed_key = ui.scope_id().with("number-field-mixed");
            let is_mixed = ui.data(|d| d.get_temp::<bool>(mixed_key)).unwrap_or(true);
            row(ui, "Mixed", *mixed, |ui| {
                let edit =
                    NumberField::new("number-field-mixed", "Size", *mixed, &numbers).unit(Unit::new("{n} pt")).mixed(is_mixed).size(size).show(ui);
                if edit
                    .event
                    .as_ref()
                    .is_some_and(|e| matches!(e, EditEvent::Set { .. } | EditEvent::Commit { changed: true, .. } | EditEvent::Update { .. }))
                {
                    ui.data_mut(|d| d.insert_temp(mixed_key, false));
                }
                super::slider::apply(&edit, mixed);
            });
            row(ui, "Invalid", *invalid, |ui| {
                let edit =
                    NumberField::new("number-field-invalid", "Columns", *invalid, &numbers).range(1.0..=12.0).invalid(true).size(size).show(ui);
                super::slider::apply(&edit, invalid);
            });
            row(ui, "Pop-up slider", *opacity, |ui| {
                let edit = NumberField::new("number-field-popup", "Opacity", *opacity, &numbers)
                    .range(0.0..=100.0)
                    .unit(Unit::new("{n}%"))
                    .popup_slider("Opacity slider")
                    .size(size)
                    .show(ui);
                super::slider::apply(&edit, opacity);
            });
            row(ui, "Disabled", 64.0, |ui| {
                let _ = NumberField::new("number-field-disabled", "Disabled", 64.0, &numbers)
                    .unit(Unit::new("{n} px"))
                    .scrub_label("W")
                    .enabled(false)
                    .size(size)
                    .show(ui);
            });
            row(ui, "Not a number", f64::NAN, |ui| {
                let _ = NumberField::new("number-field-nan", "Not a number", f64::NAN, &numbers).size(size).show(ui);
            });
        });
    });
}

/// Scrub labels and icons inside the field, and a label before it.
fn show_scrub(ui: &mut Ui, args: &mut Args) {
    let size = size_arg(args.choice("size"));
    let numbers = numbers(args);
    with_values(ui, "number-field-scrub", [1920.0, 1080.0, 15.0, 4.0], |ui, values| {
        Stack::new(Gap::Stack).show(ui, |ui| {
            let [w, h, angle, radius] = values;
            for (salt, letter, value) in [("number-field-w", "W", w), ("number-field-h", "H", h)] {
                row(ui, letter, *value, |ui| {
                    let edit = NumberField::new(salt, letter, *value, &numbers)
                        .range(1.0..=30_000.0)
                        .unit(Unit::new("{n} px"))
                        .scrub_label(letter)
                        .size(size)
                        .show(ui);
                    super::slider::apply(&edit, value);
                });
            }
            row(ui, "Angle", *angle, |ui| {
                let edit = NumberField::new("number-field-angle", "Angle", *angle, &numbers)
                    .range(-180.0..=180.0)
                    .decimals(0..=1)
                    .unit(Unit::new("{n}°"))
                    .scrub_icon(Icon::RotateCw)
                    .size(size)
                    .show(ui);
                super::slider::apply(&edit, angle);
            });
            Row::new(Gap::Inline).show(ui, |ui| {
                let label = ui.add(Text::new("Radius"));
                let edit = NumberField::new("number-field-radius", "Radius", *radius, &numbers)
                    .range(0.0..=500.0)
                    .unit(Unit::new("{n} px"))
                    .scrub_by(&label)
                    .size(size)
                    .show(ui);
                super::slider::apply(&edit, radius);
            });
            ui.add(
                Text::new("Drag a label sideways: a step a point, Shift ten, Alt a tenth as fast. Esc puts the value back.").content(Content::Muted),
            );
        });
    });
}

/// A labelled row right to left: the label mirrors, the field keeps its geometry.
fn show_right_to_left(ui: &mut Ui, args: &mut Args) {
    let size = size_arg(args.choice("size"));
    let numbers = numbers(args);
    with_values(ui, "number-field-rtl", [12.5], |ui, values| {
        Stack::new(Gap::Stack).show(ui, |ui| {
            let note = "With the direction set to right to left the label moves to the right. The field keeps its geometry: digits align right, the unit follows them and the scrub label stays at the left.";
            ui.add(Text::new(note).content(Content::Muted));
            let value = &mut values[0];
            row(ui, "Width", *value, |ui| {
                let edit = NumberField::new("number-field-rtl", "Width", *value, &numbers).decimals(0..=2).unit(Unit::new("{n} cm")).scrub_label("W").size(size).show(ui);
                super::slider::apply(&edit, value);
            });
        });
    });
}

/// Events kept in the event log.
const LOG_LEN: usize = 20;

/// The event log's document: one opacity, shown by a slider and a number field, and an undo
/// history of whole snapshots.
#[derive(Clone, Debug)]
struct LogDoc {
    opacity: f64,
    undo: Vec<f64>,
}

impl UndoMarks for LogDoc {
    type Mark = usize;

    fn undo_mark(&self) -> usize {
        self.undo.len()
    }

    fn revert_to(&mut self, mark: usize) {
        if let Some(&opacity) = self.undo.get(mark) {
            self.opacity = opacity;
            self.undo.truncate(mark);
        }
    }

    fn squash_since(&mut self, mark: usize) {
        self.undo.truncate(mark + 1);
    }
}

#[derive(Clone, Debug)]
struct LogState {
    doc: LogDoc,
    host: GestureHost<LogDoc>,
    field: Option<Id>,
    /// Newest first.
    log: VecDeque<String>,
}

impl Default for LogState {
    fn default() -> Self {
        Self { doc: LogDoc { opacity: 80.0, undo: Vec::new() }, host: GestureHost::new(), field: None, log: VecDeque::new() }
    }
}

impl LogState {
    fn log(&mut self, line: String) {
        self.log.push_front(line);
        self.log.truncate(LOG_LEN);
    }

    fn handle(&mut self, ui: &Ui, control: &str, event: Option<EditEvent<f64>>) {
        let Some(event) = event else { return };
        self.log(format!("{control}: {}", edit::describe(&event)));
        self.host.handle(ui.ctx(), &mut self.doc, event, |doc, value| {
            doc.undo.push(doc.opacity);
            doc.opacity = value;
        });
    }

    /// Undo outside the field: during a gesture it cancels the gesture instead of undoing a step.
    fn undo(&mut self, ui: &Ui) {
        match self.host.active() {
            Some(id) => {
                self.host.cancel(&mut self.doc, id);
                edit::cancel_all(ui.ctx(), CancelReason::UndoDuringGesture);
                self.log(format!("undo: cancel_all({:?})", CancelReason::UndoDuringGesture));
            }
            None => {
                if let Some(opacity) = self.doc.undo.pop() {
                    self.doc.opacity = opacity;
                    self.log("undo: one step".to_owned());
                }
            }
        }
    }
}

/// The inspector pattern: a slider and a number field showing one value under a
/// [`GestureHost`], their events newest first and the undo-step count. Cmd/Ctrl+Z outside the
/// field undoes a step, or cancels the gesture under way.
fn show_event_log(ui: &mut Ui, args: &mut Args) {
    let policy = edit::policy(args);
    let numbers = LocaleNumbers::from_tag("en");
    let state_id = ui.scope_id().with("number-field-event-log");
    let mut state = ui.data(|d| d.get_temp::<LogState>(state_id)).unwrap_or_default();
    let in_field = state.field.is_some_and(|id| ui.memory(|m| m.has_focus(id)));
    if !in_field && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Z)) {
        state.undo(ui);
    }
    Stack::new(Gap::Stack).show(ui, |ui| {
        let opacity = state.doc.opacity;
        Row::new(Gap::Inline).show(ui, |ui| {
            let label = ui.add(Text::new("Opacity"));
            let edit = Slider::new("number-field-log-slider", "Opacity", opacity, 0.0..=100.0).step(1.0).policy(policy).show(ui);
            state.handle(ui, "slider", edit.event);
            let edit = NumberField::new("number-field-log", "Opacity", opacity, &numbers)
                .range(0.0..=100.0)
                .unit(Unit::new("{n}%"))
                .scrub_by(&label)
                .popup_slider("Opacity slider")
                .policy(policy)
                .show(ui);
            state.field = Some(edit.response.id);
            state.handle(ui, "field", edit.event);
        });
        if let Some(event) = state.host.end_frame(ui.ctx(), &mut state.doc) {
            let line = format!("host: {}", edit::describe(&event));
            state.log(line);
        }
        ui.add(Text::new(format!("{opacity}, {} undo steps", state.doc.undo.len())).content(Content::Muted));
        Surface::new(SurfaceKind::Sunken).show(ui, |ui| {
            ui.set_min_width(LOG_WIDTH);
            if state.log.is_empty() {
                ui.add(Text::new("No events yet").content(Content::Faint));
            }
            for line in &state.log {
                ui.add(Text::new(line.as_str()).role(TypeRole::Mono));
            }
        });
    });
    ui.data_mut(|d| d.insert_temp(state_id, state));
}

const LOG_WIDTH: f32 = 420.0;
