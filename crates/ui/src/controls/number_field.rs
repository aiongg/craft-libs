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
    Pos2, Rect, RectAlign, Response, Sense, Shape, Ui, ViewportCommand, pos2, vec2,
};

use super::field_wheel;
use super::slider::{PressTurn, Scale, Track, labelled, pointer, press_turn, with_values};
use super::text_field::{BoxState, field_box};
use super::{IconButton, Slider, TextField, TextFieldStyle, TooltipExt as _, icon_size};
use crate::behavior::{Codec, NotAValue, Typed, Typing, TypingLook, edit_menu};
use crate::edit::ops::{Bound, ScalarOp};
use crate::edit::{self, CancelReason, EditEvent, EditPolicy, EditResponse, Gesture, GestureHost, GestureId, PolicyArgs, UndoMarks};
use crate::icons::{self, Icon};
use crate::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt};
use crate::localized::{LocaleNumbers, Unit, first_field_id, incomplete, round_half_away, stepped};
use crate::primitives::{Content, Corner, Gap, Interactive, Row, Stack, StateColors, Surface, SurfaceKind, Text};
use crate::story::{Level, SizeArg, Story, StoryArgs, StoryChoice, variant_list};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, margin_x};
use crate::typed_story;

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
/// The keys and the pointer follow PhotoCraft's number fields:
///
/// - **Typing** is one gesture. A plain number ([`LocaleNumbers::parse`], with or without the
///   unit) applies as it is typed: an update for each keystroke that makes one, so the host
///   shows it at once. Arithmetic and the app's other readers
///   ([`or_parse`](NumberField::or_parse)) apply only on the commit, so `5/2*2` never applies
///   `5` on the way; `-` and `1,` aren't numbers yet and leave the value as it was. Enter
///   commits and leaves the field ([`keep_focus`](NumberField::keep_focus) keeps it there), and
///   the focus leaving commits: the text read back, rounded to the most decimals shown (half
///   away from zero) and clamped to the range. The text as the field shows it stands for the
///   value itself, decimals the field doesn't show included. Text that can't become a number
///   gets the invalid edge and the reason as tooltip as it is typed (`-` and `1,` don't). Text
///   that isn't a number commits the value from before the run: the text stays in the field,
///   marked, until the user fixes it or Esc restores the value. Esc cancels the run, leaves the
///   field and goes on to the host ([`EditResponse::escaped`]), as in a [`TextField`]. The host
///   needn't show each update as it was sent: it may answer one with a value of its own (snapped
///   or clamped), or keep its value until the commit (still [mixed](NumberField::mixed), or not
///   caught up yet); the text stays the user's. The run ends as moved by something else
///   ([`CancelReason::External`]) when the host's value changes to one the run didn't send, in
///   any pass but the one right after an update went out: a change in that pass is the host's
///   answer to the update.
/// - **Scrubbing**: a horizontal drag anywhere on a field without the focus, past egui's drag
///   threshold, scrubs it; a click without a drag gives it the focus with its text selected, so
///   typing replaces it. The [scrub label](NumberField::scrub_label) or
///   [icon](NumberField::scrub_icon) and a label outside the field
///   ([`scrub_by`](NumberField::scrub_by)) scrub with the focus in the field too; a drag on the
///   focused field's text selects it. [`scrub_text(false)`](NumberField::scrub_text) leaves the
///   text to the pointer (DesignCraft's fields). A scrub moves a step per point; Shift ten
///   steps, Alt a tenth as fast. A drag is one gesture: Esc cancels it, a still pointer leaves
///   the value exactly as it was; once the gesture is cancelled (Esc, an undo, the host's own
///   change) the drag does nothing more until the release.
/// - **Keys**: Up and Down step the focused field; Shift steps ten, Cmd/Ctrl a tenth where the
///   decimals show it. A plain or fine step goes to the next multiple of its size (by 1, 5.1
///   goes up to 6 and down to 5).
/// - **The wheel** steps a step a notch, ten with Shift. A focused field takes the wheel
///   wherever the pointer is, so no panel scrolls with it; a field without the focus takes a
///   wheel gesture that starts over it, while a gesture that started elsewhere (a panel being
///   scrolled) goes on scrolling over it, and one that leaves the field is the panel's from then
///   on. Alt+wheel, Ctrl/Cmd+wheel and a plain sideways scroll stay the view's. The focused
///   field's hand-off needs [`theme::install`](crate::theme::install).
/// - Each frame's steps (keys, wheel, screen reader) are one [`Set`](EditEvent::Set); during a
///   typing run they step from the typed value and the run's commit carries the result. Text
///   that isn't a number takes no step: an arrow ends the run as Enter does (the text stays,
///   marked) and the wheel is the panel's.
/// - **Pop-up slider** ([`popup_slider`](NumberField::popup_slider)): a ▾ at the field's end
///   opens a [`Slider`] below it; pressing the ▾ and dragging moves the value at once, as far as
///   the slider would, and closes it on release. A drag on either is one gesture.
/// - **Mixed** ([`mixed`](NumberField::mixed)): blank with a `—` placeholder; a typed number
///   commits; scrubs and steps start from the host's value.
///
/// A value that isn't a finite number shows blank too. The digits have tabular figures, so the
/// text keeps its width while scrubbing. The field keeps its left-to-right geometry right to left
/// ([`Unmirrored::Number`](crate::direction::Unmirrored::Number)): digits are centred between the
/// scrub handle and the unit, which sits at the box's end, the handle at the left.
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
    scrub_text: bool,
    next: Option<Box<NextStep<'a>>>,
    wrap: bool,
    popup: Option<String>,
    mixed: bool,
    policy: EditPolicy,
    size: ControlSize,
    enabled: bool,
    invalid: bool,
    keep_focus: bool,
    part: bool,
    edit_menu: bool,
    unit_menu: Option<UnitMenu<'a>>,
}

/// The rows of [`NumberField::unit_menu`] and where its pick goes.
struct UnitMenu<'a> {
    entries: &'a [MenuEntry<usize>],
    picked: &'a mut Option<usize>,
}

impl<'a> NumberField<'a> {
    /// A field named `label` (its accessible name) showing the host's `value` in the region's
    /// digits; `id_salt` keeps its text, focus and gesture when the layout around it changes.
    /// `value` may be any number that converts to `f64` (`f32`, `i32`).
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, value: impl Into<f64>, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            value: value.into(),
            numbers,
            range: f64::NEG_INFINITY..=f64::INFINITY,
            step: 1.0,
            decimals: 0..=6,
            unit: Unit::default(),
            fallback: None,
            handle: None,
            scrub_by: None,
            scrub_text: true,
            next: None,
            wrap: false,
            popup: None,
            mixed: false,
            policy: EditPolicy::default(),
            size: ControlSize::Md,
            enabled: true,
            invalid: false,
            keep_focus: false,
            part: false,
            edit_menu: true,
            unit_menu: None,
        }
    }

    /// The values the field takes, read in ascending order; unbounded by default. A typed,
    /// scrubbed or stepped value is clamped to it; the host's value is shown as it is.
    pub fn range<V: Into<f64> + Copy>(mut self, range: RangeInclusive<V>) -> Self {
        self.range = (*range.start()).into()..=(*range.end()).into();
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

    /// Up and Down, the wheel and screen-reader increments step to `next(value, direction)`
    /// (direction is 1 or -1; one call per notch, modifiers do not scale it) instead of by
    /// [`step`](NumberField::step): zoom levels (12.5, 25, 50, 100 ...). The result is clamped to
    /// the range and rounded to the decimals, and a frame's steps are one
    /// [`Set`](EditEvent::Set). A scrub is still continuous.
    pub fn steps_with(mut self, next: impl Fn(f64, i32) -> f64 + 'a) -> Self {
        self.next = Some(Box::new(next));
        self
    }

    /// While the pointer scrubs in a viewport that is full screen, the pointer wraps from one
    /// horizontal edge of the screen to the other and the scrub goes on from there; the jump
    /// adds nothing to the value. Where the platform does not move the pointer on request
    /// (Wayland without a pointer lock) the scrub stops at the edge. Off by default, and
    /// ignored when the viewport is not full screen.
    pub fn scrub_wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
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

    /// Whether a drag on a field without the focus scrubs it (PhotoCraft); on by default. Off, a
    /// press in the text places the caret and a drag selects, as in a text field (DesignCraft),
    /// and only the scrub handles scrub.
    pub fn scrub_text(mut self, scrub: bool) -> Self {
        self.scrub_text = scrub;
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
    /// and `value` is only where scrubs and steps start. The host may keep the field mixed until
    /// a gesture commits: while it stays mixed, a typing run's or a scrub's updates count as
    /// applied, so the gesture goes on. Turning it on or giving it another value during a gesture
    /// is a change like any other (see Typing on [`NumberField`]).
    pub fn mixed(mut self, mixed: bool) -> Self {
        self.mixed = mixed;
        self
    }

    /// How the field reports ([`EditPolicy`]); a typing run's text that applies only on the
    /// commit (arithmetic) holds its updates whatever this says. The default reports every
    /// change at once and cancels on Esc.
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

    /// Whether the field keeps the focus after Enter commits; off by default.
    pub fn keep_focus(mut self, keep: bool) -> Self {
        self.keep_focus = keep;
        self
    }

    /// Whether a secondary click, or the Menu key or Shift+F10 while the field has the focus,
    /// opens the [`EditMenu`](crate::behavior::EditMenu) (Undo, Cut, Copy, Paste, Delete,
    /// Select All) on the field's text; on by default.
    pub fn edit_menu(mut self, on: bool) -> Self {
        self.edit_menu = on;
        self
    }

    /// A secondary click on the field opens a [`ContextMenu`] of `entries` (the units to pick
    /// from; each row's command is the host's index of that unit), in place of the
    /// [edit menu](NumberField::edit_menu). A pick is written to `picked` (otherwise left
    /// alone); the field does not change the value or the [`unit`](NumberField::unit): the host
    /// applies the unit, as PhotoCraft's options bar does. Mark the current unit with
    /// [`MenuItem::radio`](crate::menu::MenuItem::radio). The Menu key does not open it.
    pub fn unit_menu(mut self, entries: &'a [MenuEntry<usize>], picked: &'a mut Option<usize>) -> Self {
        self.unit_menu = Some(UnitMenu { entries, picked });
        self
    }

    /// Whether the field is a part of a composite reporting through one edit: a press on the
    /// scrub sources, the ▾ or the pop-up slider takes its turn from the edit order
    /// (`slider::press_turn`). While another control's typing run has the focus (a sibling
    /// field's), the press asks it to end and waits, and runs from the host's value after the
    /// run's ending event.
    pub(crate) fn part(mut self, part: bool) -> Self {
        self.part = part;
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

    /// The width of a field of `size` that shows every value of `range` with `decimals` digits
    /// after the decimal mark and its `unit` whole: the box's padding, the unit and the widest
    /// number at either end of the range, in tabular figures. Columns of fields with different
    /// ranges take the widest of their widths.
    pub fn fit_width(ui: &Ui, size: ControlSize, numbers: &LocaleNumbers, range: RangeInclusive<f64>, decimals: usize, unit: &Unit) -> f32 {
        let tokens = Tokens::of(ui);
        let style = NumberField::style(&tokens, size);
        let font = tokens.font(style.field.role);
        let width = |text: String, tabular: bool| {
            let mut job = egui::text::LayoutJob::simple_singleline(text, font.clone(), Color32::PLACEHOLDER);
            if tabular {
                for section in &mut job.sections {
                    section.format.font_features.push(crate::theme::TABULAR_FIGURES, 1);
                }
            }
            ui.fonts_mut(|f| f.layout_job(job)).size().x
        };
        let number = [*range.start(), *range.end()].map(|v| width(numbers.format(v, decimals..=decimals), true)).into_iter().fold(0.0, f32::max);
        let unit = width(unit.prefix().to_owned(), false) + width(unit.suffix().to_owned(), false);
        (2.0 * style.field.pad_x + number + unit).ceil()
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
        let id = ui.make_persistent_id(self.id_salt);
        let real = (!self.mixed && self.value.is_finite()).then_some(self.value);
        let host = Follow::host(ui, id, real);
        let codec = Numbers { numbers: self.numbers, decimals: self.decimals.clone(), unit: &self.unit, fallback: self.fallback.as_deref(), bounds };
        let font = tokens.font(style.field.role);
        let parts = Parts::new(ui, &tokens, &style, &self, font.clone(), enabled);
        let stepping = enabled && self.step != 0.0 && self.step.is_finite();

        if enabled && !ui.is_sizing_pass() && ui.data_mut(|d| d.remove_temp::<bool>(first_field_id())).is_some() {
            ui.memory_mut(|m| m.request_focus(id));
            select_all(ui.ctx(), id, &codec.format(&host));
        }
        // The edit menu goes first: it takes its keys (↓ moves in it) and applies a chosen command
        // before the arrows below or the field read the input. Over the text of a field without
        // the focus, a secondary press belongs to the scrub (it covers the text): it gives the
        // field the focus with its text selected, and the menu opens at the pointer.
        let edit_menu = self.edit_menu && self.unit_menu.is_none();
        let menu = if edit_menu && enabled {
            let scrub_text = ui.ctx().read_response(scrub_text_id(id)).filter(|_| stepping && self.scrub_text && !ui.memory(|m| m.has_focus(id)));
            let open_at = scrub_text.filter(Response::secondary_clicked).and_then(|r| r.interact_pointer_pos());
            if open_at.is_some() {
                ui.memory_mut(|m| m.request_focus(id));
                select_all(ui.ctx(), id, &codec.format(&host));
            }
            Some(edit_menu::show(ui, id, false, open_at))
        } else {
            None
        };
        // Before `TextEdit`, which would move the caret on them; an input method's own while it
        // composes.
        let arrows = if stepping && ui.memory(|m| m.has_focus(id)) && !Typing::composing(ui, id) { take_arrows(ui) } else { Vec::new() };

        let width = if ui.layout().horizontal_justify() { ui.available_width() } else { style.field.width };
        let look = TypingLook {
            size: vec2(width, style.field.height),
            margin: margin_x(parts.start, parts.end),
            align: Align2::CENTER_CENTER,
            font,
            text: if enabled { style.field.text } else { style.field.text_disabled },
            placeholder: if enabled { style.field.placeholder } else { style.field.placeholder_disabled },
            selection: style.field.selection,
            selected_text: style.field.selected_text,
            caret: style.field.caret,
        };
        let mut typing = Typing::with_codec(id, &self.label, host, codec.clone())
            .placeholder(BLANK)
            .policy(self.policy)
            .enabled(self.enabled)
            .invalid(self.invalid)
            .keep_focus(self.keep_focus)
            .role(Role::SpinButton)
            .edit_menu(edit_menu)
            .event_filter(EventFilter { vertical_arrows: true, ..EventFilter::default() })
            .tabular(true);
        if let Some(menu) = menu {
            typing = typing.edit_menu_shown(menu);
        }
        // The box goes under the text, which `TextEdit` paints as it runs.
        let under = ui.painter().add(Shape::Noop);
        let mut typed = typing.show(ui, &look);
        let rect = typed.response().rect;
        // The field's own typing run having the focus is no reason to wait: its scrub ends it.
        let own = typed.gesture().id();
        let rule = FieldRule::new(&self.range, self.step, *self.decimals.end());
        let input = Input {
            id,
            base: if self.value.is_finite() { self.value } else { rule.blank() },
            step: self.step,
            next: self.next.as_deref(),
            wrap: self.wrap,
            rule,
            modifiers: ui.input(|i| i.modifiers),
            codec,
            part: self.part,
            own,
        };

        let mut secondary = None;
        if stepping {
            let mut sources = Vec::new();
            if let Some(handle) = parts.handle_rect(rect) {
                sources.push(ui.interact(handle, id.with("craft_ui::number_field::scrub"), Sense::CLICK | Sense::DRAG));
            }
            if let Some(label) = &self.scrub_by {
                sources.push(label.interact(Sense::CLICK | Sense::DRAG));
            }
            // Over the text of a field without the focus, where `TextEdit` would take the press.
            if self.scrub_text && !typed.response().has_focus() {
                let text = ui.interact(rect, scrub_text_id(id), Sense::CLICK | Sense::DRAG);
                sources.push(text);
            }
            secondary = sources.iter().find(|s| s.secondary_clicked()).and_then(|s| s.interact_pointer_pos());
            input.scrub(ui, &mut typed, &sources);
        }
        let popup_open = self.popup.is_some() && input.popup(ui, &mut typed, &self, rect, parts.popup_rect(rect), enabled);
        let response = typed.response().clone();
        if let Some(UnitMenu { entries, picked }) = self.unit_menu.take().filter(|_| enabled) {
            let at = secondary.or_else(|| response.secondary_clicked().then(|| response.interact_pointer_pos()).flatten());
            let menu = ContextMenu::new(entries).size(self.size).own_gestures(false).open_at(at.map(OpenAt::Pointer)).show(ui, &response);
            if menu.chosen.is_some() {
                *picked = menu.chosen;
            }
        }
        let focused = enabled && response.has_focus();
        let stepped = input.step(ui, &mut typed, &arrows, stepping, enabled);

        let invalid = self.invalid || typed.rejected().is_some();
        report(ui.ctx(), &typed, bounds, &self.unit, stepping.then_some(self.step));
        ui.painter().set(under, Shape::Vec(field_box(ui, &style.field, response.rect, BoxState::of(&response, enabled, invalid))));
        parts.paint(ui, &self, rect, typed.text_rect(), !typed.text().is_empty());
        if let Some(reason) = typed.rejected().map(str::to_owned) {
            if focused && !popup_open {
                response.show_tooltip(reason.as_str());
            }
            let _ = response.clone().tooltip(reason);
        }
        let (gesture, typing) = (typed.gesture().id(), typed.is_typing());
        let edit = typed.end_as(ui, |value| value.unwrap_or(self.value));
        Follow::store(ui, id, gesture, typing, real, edit.event.as_ref());
        if self.part {
            // Any other value the field sends (typed, scrubbed, set on the pop-up slider) is
            // absolute.
            let set = || edit.event.as_ref().and_then(EditEvent::value).filter(|v| v.is_finite()).map(|v| vec![ScalarOp::Set(*v)]);
            if let Some(ops) = stepped.or_else(set) {
                edit::ops::report(ui.ctx(), id, ops);
            }
        }
        edit
    }
}

/// How the field reads its host during one of its own gestures, kept from the last pass: the
/// host doesn't always show the value an update sent (Typing on [`NumberField`]).
#[derive(Clone, Copy, Debug)]
struct Follow {
    gesture: GestureId,
    /// The host's value on the last pass: `None` mixed or not a finite number.
    seen: Option<f64>,
    /// The gesture is a typing run.
    typing: bool,
    /// An update went out on the last pass with a value the host didn't show: its answer comes on
    /// this pass.
    sent: bool,
}

impl Follow {
    fn key(id: Id) -> Id {
        id.with("craft_ui::number_field::follow")
    }

    /// The host value the field gives its gesture this pass for the host's `real` one. While the
    /// host is consistent with the gesture, that is the gesture's last reported value: the host
    /// shows it or stays mixed, or, in a typing run, keeps its value from the last pass or answers
    /// the update the last pass sent. Otherwise it is `real`, and the gesture cancels as moved by
    /// something else.
    fn host(ui: &Ui, id: Id, real: Option<f64>) -> Option<f64> {
        let Some((gesture, reported)) = Gesture::<Option<f64>>::reported(ui, id) else { return real };
        let Some(follow) = ui.data(|d| d.get_temp::<Self>(Self::key(id))).filter(|f| f.gesture == gesture) else { return real };
        let unmoved = real == follow.seen;
        let consistent = real == reported || (unmoved && (real.is_none() || follow.typing)) || (follow.typing && follow.sent);
        if consistent { reported } else { real }
    }

    /// Keeps what the next pass needs while `gesture` goes on: `real` as seen, and whether this
    /// pass's `event` sent an update the host's value doesn't show yet.
    fn store(ui: &Ui, id: Id, gesture: Option<GestureId>, typing: bool, real: Option<f64>, event: Option<&EditEvent<f64>>) {
        let ended = matches!(event, Some(EditEvent::Commit { .. } | EditEvent::Cancel { .. }));
        let follow = gesture.filter(|_| !ended).map(|gesture| {
            let sent = matches!(event, Some(EditEvent::Update { value, .. }) if Some(*value) != real);
            Self { gesture, seen: real, typing, sent }
        });
        ui.data_mut(|d| match follow {
            Some(follow) => {
                d.insert_temp(Self::key(id), follow);
            }
            None => d.remove::<Self>(Self::key(id)),
        });
    }
}

/// The step function of [`NumberField::steps_with`].
type NextStep<'a> = dyn Fn(f64, i32) -> f64 + 'a;

/// The ▾'s size class: a small ghost icon button inside the field.
const POPUP_SIZE: ControlSize = ControlSize::Sm;

/// What the field's own input (scrubs, the ▾, steps) works with in a frame.
struct Input<'n> {
    id: Id,
    /// Where scrubs and steps start in a blank field.
    base: f64,
    step: f64,
    next: Option<&'n NextStep<'n>>,
    wrap: bool,
    rule: FieldRule,
    modifiers: Modifiers,
    codec: Numbers<'n>,
    /// The field is a part of a composite ([`NumberField::part`]).
    part: bool,
    /// The field's own gesture.
    own: Option<GestureId>,
}

impl<'n> Input<'n> {
    /// When the press `started` in this pass on one of the field's press targets (`key`) runs:
    /// now, after a wait, or not yet.
    fn turn(&self, ui: &Ui, key: &str, started: Option<Pos2>) -> PressTurn {
        press_turn(ui, self.id.with(key), self.part, self.own, started)
    }

    /// Drags on the scrub `sources`: a step a point (Shift ten, Alt a tenth as fast); a click
    /// focuses the field.
    fn scrub(&self, ui: &Ui, typed: &mut Typed<Numbers<'n>>, sources: &[Response]) {
        let key = self.id.with("craft_ui::number_field::scrubbing");
        let started = sources.iter().find_map(|drag| drag.drag_started_by(PointerButton::Primary).then(|| pointer(ui, drag)).flatten());
        let origin = match self.turn(ui, "craft_ui::number_field::scrub_press", started.map(|(_, origin)| origin)) {
            PressTurn::Now => started.map(|(_, origin)| origin),
            // A press released while it waited does nothing.
            PressTurn::Held(at) => sources.iter().any(|drag| drag.dragged_by(PointerButton::Primary)).then_some(at),
            PressTurn::Waits => None,
        };
        if let Some(origin) = origin {
            // A typing run ends as the focus leaves, its text read before the first step.
            ui.memory_mut(|m| m.surrender_focus(self.id));
            ui.data_mut(|d| d.insert_temp(key, Scrub { press: Press::default(), last_x: origin.x, steps: 0.0, last: None, warp: None }));
        }
        for drag in sources {
            if drag.hovered() || drag.dragged() {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
            }
            if drag.clicked() {
                ui.memory_mut(|m| m.request_focus(self.id));
                select_all(ui.ctx(), self.id, typed.text());
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
                let edges = (self.wrap && ui.input(|i| i.viewport().fullscreen == Some(true))).then(|| {
                    let screen = ui.ctx().content_rect();
                    (screen.left(), screen.right())
                });
                let travel = travel(scrub.last_x, pos.x, scrub.warp, edges);
                if let Some(x) = travel.send {
                    ui.ctx().send_viewport_cmd(ViewportCommand::CursorPosition(pos2(x, pos.y)));
                }
                scrub.steps += f64::from(travel.delta) * STEPS_PER_POINT * rate;
                scrub.last_x = travel.last_x;
                scrub.warp = travel.warp;
                let n = scrub.steps.trunc();
                scrub.steps -= n;
                let gesture = typed.gesture();
                if n != 0.0 {
                    // A gesture the host ended carries on from the host's value, which lacks the
                    // scrub's steps when the host reverted them: the scrub goes on from the value
                    // it set last.
                    let from = match scrub.last {
                        Some((id, value)) if gesture.id() != Some(id) => value,
                        _ => *gesture.shown(),
                    };
                    let to = Some(self.codec.settle(from.unwrap_or(self.base) + n * self.step));
                    gesture.update(to, m);
                    scrub.last = gesture.id().map(|id| (id, to));
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
        let started = arrow.drag_started_by(PointerButton::Primary).then(|| pointer(ui, &arrow).map(|(_, origin)| origin)).flatten();
        let runs = match self.turn(ui, "craft_ui::number_field::popup_turn", started) {
            PressTurn::Now => started.is_some(),
            PressTurn::Held(_) => arrow.dragged_by(PointerButton::Primary),
            PressTurn::Waits => false,
        };
        if runs {
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
            if let Some(anchor) = gesture.anchor().copied() {
                let dx = pos.x - origin.x;
                // A still pointer leaves the value as it was.
                let to = if dx == 0.0 {
                    anchor
                } else {
                    Some(self.codec.settle(scale.snap(scale.place(anchor.unwrap_or(self.base)) + track.value_per(dx))))
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
            .enabled(enabled)
            .part(self.part);
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
        // The field reports the slider's value as its own.
        let _ = edit::ops::take::<ScalarOp>(ui.ctx(), edit.response.id);
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
    /// text stays rejected, and the wheel is the panel's. The operations the change applied: in a
    /// typing run, the typed value first.
    fn step(&self, ui: &Ui, typed: &mut Typed<Numbers<'n>>, arrows: &[(Modifiers, f64)], stepping: bool, enabled: bool) -> Option<Vec<ScalarOp>> {
        let response = typed.response().clone();
        let dragging = typed.gesture().is_active() && !typed.is_typing();
        let focused = enabled && response.has_focus();
        let reads = typed.reads_as_value();
        let mut op = None;
        if stepping {
            let notches = field_wheel::notches(ui, self.id, focused, response.rect, enabled && reads && !dragging, 10.0);
            let requests = ui.input(|i| {
                i.num_accesskit_action_requests(self.id, Action::Increment) as f64
                    - i.num_accesskit_action_requests(self.id, Action::Decrement) as f64
            });
            let steps = !arrows.is_empty() || notches != 0.0 || requests != 0.0;
            if steps && !reads {
                typed.commit();
            } else if steps {
                let others = [notches, requests].into_iter().filter(|n| *n != 0.0).map(|n| (Modifiers::NONE, n));
                op = Some(ScalarOp::Presses(arrows.iter().copied().chain(others).collect()));
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
            op = set.map(ScalarOp::Set).or(op);
        }
        let op = op.filter(|_| !dragging)?;
        let from = typed.value().unwrap_or(self.base);
        let ops = if typed.is_typing() { vec![ScalarOp::Set(from), op] } else { vec![op] };
        let v = ops.iter().fold(from, |v, op| match (op, self.next) {
            (ScalarOp::Presses(presses), Some(next)) => next_stop(next, v, presses, &self.rule),
            _ => apply(op, v, &self.rule),
        });
        // Composites replay a field's operations; its own stops are not theirs to repeat.
        let ops = if self.next.is_some() { vec![ScalarOp::Set(v)] } else { ops };
        typed.commit_with(ui, Some(v));
        if focused {
            select_all(ui.ctx(), self.id, &self.codec.format(&Some(v)));
        }
        Some(ops)
    }
}

/// A press on a scrub handle, the ▾ or the pop-up slider, until its release.
#[derive(Clone, Copy, Debug, Default)]
struct Press {
    /// The input the press began ([`Gesture::input`](edit::Gesture::input): the same when a
    /// gesture the host ended carries it on); none before it begins.
    input: Option<GestureId>,
}

impl Press {
    /// Begins the press's gesture, once and when no typing run is under way (a run the press
    /// ended commits first); whether that gesture is under way. One that has ended (a cancel)
    /// leaves the press with nothing to do until its release.
    fn under_way(&mut self, typed: &mut Typed<Numbers<'_>>) -> bool {
        if self.input.is_none() && !typed.is_typing() {
            let gesture = typed.gesture();
            gesture.begin();
            self.input = gesture.input();
        }
        self.input.is_some() && typed.gesture().input() == self.input
    }

    /// Commits the press's gesture on release, while it is under way.
    fn release(self, typed: &mut Typed<Numbers<'_>>) {
        if self.input.is_some() && typed.gesture().input() == self.input {
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
    /// The gesture of the scrub's last change and the value it set.
    last: Option<(GestureId, Option<f64>)>,
    /// Where the pointer was asked to go at the screen's edge, until it is seen there.
    warp: Option<f32>,
}

/// How far from the screen's edge the pointer counts as at it, and how far inside the opposite
/// edge it is put.
const WRAP_EDGE: f32 = 2.0;

/// What one pointer position adds to a scrub.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Travel {
    /// Points moved, not counting a warp.
    pub(super) delta: f32,
    /// The x to measure the next move from.
    pub(super) last_x: f32,
    /// The x the pointer was asked to go to and has not been seen at.
    pub(super) warp: Option<f32>,
    /// The x to move the pointer to now.
    pub(super) send: Option<f32>,
}

/// The points the pointer moved from `last_x` to `x`. With the horizontal `edges` of the screen,
/// a move that reaches one asks for the pointer at the other (`send`), once: the request stays
/// pending until the pointer is seen nearer the target than it was, and the position that
/// then shows up there is not a move. A pointer that stays where it was (a platform that does not
/// move it) waits; one that moves elsewhere is an ordinary move, and the request is not sent
/// again.
pub(super) fn travel(last_x: f32, x: f32, warp: Option<f32>, edges: Option<(f32, f32)>) -> Travel {
    let mut now = Travel { delta: x - last_x, last_x: x, warp: None, send: None };
    if let Some(target) = warp {
        if x == last_x {
            return Travel { delta: 0.0, warp, ..now };
        }
        if (x - target).abs() < (x - last_x).abs() {
            return Travel { delta: 0.0, ..now };
        }
        return Travel { warp, ..now };
    }
    if let Some((left, right)) = edges {
        let target = if now.delta > 0.0 && x >= right - WRAP_EDGE {
            Some(left + 2.0 * WRAP_EDGE)
        } else if now.delta < 0.0 && x <= left + WRAP_EDGE {
            Some(right - 2.0 * WRAP_EDGE)
        } else {
            None
        };
        (now.warp, now.send) = (target, target);
    }
    now
}

/// `range` in ascending order, a bound that is no number left open.
fn ordered(range: &RangeInclusive<f64>) -> (f64, f64) {
    let lo = if range.start().is_nan() { f64::NEG_INFINITY } else { *range.start() };
    let hi = if range.end().is_nan() { f64::INFINITY } else { *range.end() };
    if lo <= hi { (lo, hi) } else { (hi, lo) }
}

/// How a number field's value takes an operation ([`apply`]): what a composite that draws the
/// field as a part builds from the same range, step and decimals to replay the field's operations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FieldRule {
    /// One step ([`NumberField::step`]); 0 or not a number: none.
    pub step: f64,
    /// The most decimals shown, which a value is rounded to.
    pub places: usize,
    /// The range in ascending order, a bound that is no number left open.
    pub bounds: (f64, f64),
}

impl FieldRule {
    /// The rule of a field with `range`, `step` and at most `places` decimals.
    pub fn new(range: &RangeInclusive<f64>, step: f64, places: usize) -> Self {
        Self { step, places, bounds: ordered(range) }
    }

    /// Where a blank field (mixed values) steps from: 0, kept in the range.
    pub fn blank(&self) -> f64 {
        0.0_f64.clamp(self.bounds.0, self.bounds.1)
    }

    /// `value` rounded to the decimals and clamped to the range.
    pub fn settle(&self, value: f64) -> f64 {
        round_half_away(value, self.places).clamp(self.bounds.0, self.bounds.1)
    }
}

/// The value `op` gives a field at `from` under `rule`, settled ([`FieldRule::settle`]). Presses
/// step as the arrow keys do ([`stepped`](crate::localized::stepped)): Shift ten steps, Ctrl/Cmd
/// a tenth; a field without a step leaves `from`. A move to a bound that is open leaves `from`.
pub(crate) fn apply(op: &ScalarOp, from: f64, rule: &FieldRule) -> f64 {
    let stepping = rule.step != 0.0 && rule.step.is_finite();
    let to = match op {
        ScalarOp::Presses(presses) if stepping => presses.iter().fold(from, |v, &(modifiers, n)| stepped(v, rule.step, rule.places, modifiers, n)),
        ScalarOp::Presses(_) => from,
        ScalarOp::By(delta) => from + delta,
        ScalarOp::Set(v) => *v,
        ScalarOp::ToBound(bound) => Some(if *bound == Bound::Low { rule.bounds.0 } else { rule.bounds.1 }).filter(|b| b.is_finite()).unwrap_or(from),
    };
    rule.settle(to)
}

/// The value `presses` reach from `from` by `next`, one call for each notch (a press of `n`
/// notches calls it `n` times, at least once, up when positive), settled under `rule`.
fn next_stop(next: &NextStep<'_>, from: f64, presses: &[(Modifiers, f64)], rule: &FieldRule) -> f64 {
    let to = presses.iter().fold(from, |v, &(_, n)| {
        let direction = if n < 0.0 { -1 } else { 1 };
        (0..n.abs().round().max(1.0) as usize).fold(v, |v, _| rule.settle(next(v, direction)))
    });
    rule.settle(to)
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

    /// A plain number in the region's digits, with or without the unit; not what only the app's
    /// reader reads (arithmetic).
    fn applies_as_typed(&self, text: &str) -> bool {
        self.numbers.parse(self.unit.strip(text)).is_ok()
    }

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
    /// From the box's end to the suffix's end: the suffix sits at the box's end, not beside the
    /// number, as PhotoCraft's does.
    unit_inset: f32,
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
        let unit_inset = if popup > 0.0 { popup_inset + popup + f.gap } else { f.pad_x };
        let end = suffix + unit_inset;
        let (scrub, unit) = if enabled { (style.scrub, style.unit) } else { (style.scrub_disabled, style.unit_disabled) };
        Self { pad: f.pad_x, gap: f.gap, handle, prefix, suffix, popup, popup_inset, unit_inset, start, end, font, scrub, unit }
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
            painter.text(pos2(rect.right() - self.unit_inset, y), Align2::RIGHT_CENTER, field.unit.suffix(), self.font.clone(), self.unit);
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

/// The id of the scrub over the text of the field `id`.
fn scrub_text_id(id: Id) -> Id {
    id.with("craft_ui::number_field::scrub_text")
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

#[derive(StoryChoice, Clone, Copy, Debug, Default, PartialEq)]
enum LocaleArg {
    #[default]
    En,
    De,
    #[choice(name = "ar-EG")]
    ArEg,
}

impl LocaleArg {
    fn numbers(self) -> LocaleNumbers {
        LocaleNumbers::from_tag(match self {
            LocaleArg::En => "en",
            LocaleArg::De => "de",
            LocaleArg::ArEg => "ar-EG",
        })
    }
}

#[derive(StoryArgs, Debug, Default, PartialEq)]
struct LocaleSizeArgs {
    size: SizeArg,
    locale: LocaleArg,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct StatesArgs {
    size: SizeArg,
    locale: LocaleArg,
    #[arg(name = "edit menu")]
    edit_menu: bool,
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "controls/number-field/states",
            Level::Controls,
            "Number field",
            "States",
            StatesArgs { size: SizeArg::Md, locale: LocaleArg::En, edit_menu: true },
            show_states
        ),
        typed_story!("controls/number-field/scrub", Level::Controls, "Number field", "Scrub", LocaleSizeArgs::default(), show_scrub),
        typed_story!(
            "controls/number-field/right-to-left",
            Level::Controls,
            "Number field",
            "Right to left",
            LocaleSizeArgs::default(),
            show_right_to_left
        ),
        typed_story!("controls/number-field/unit-menu", Level::Controls, "Number field", "Unit menu", LocaleSizeArgs::default(), show_unit_menu),
        typed_story!("controls/number-field/event-log", Level::Controls, "Number field", "Event log", PolicyArgs::default(), show_event_log),
    ]
}

/// Plain, with a unit, mixed, invalid, with a pop-up slider, and disabled.
fn show_states(ui: &mut Ui, args: &mut StatesArgs) {
    let size = args.size.control();
    let numbers = args.locale.numbers();
    let sum = |text: &str| text.split_once('+').and_then(|(a, b)| Some(a.trim().parse::<f64>().ok()? + b.trim().parse::<f64>().ok()?));
    let edit_menu = args.edit_menu;
    let mixed_key = ui.scope_id().with("number-field-mixed");
    let is_mixed = ui.data(|d| d.get_temp::<bool>(mixed_key)).unwrap_or(true);
    with_values(ui, "number-field-states", [42.0, 12.5, 30.0, 7.0, 100.0, 100.0], |ui, values| {
        variant_list(ui, "number-field-states", |form| {
            let [plain, unit, mixed, invalid, opacity, zoom] = values;
            labelled(form, "Plain", &format!("{}", *plain), |ui| {
                let edit = NumberField::new("number-field-plain", "Plain", *plain, &numbers).size(size).edit_menu(edit_menu).or_parse(sum).show(ui);
                super::slider::store(&edit, plain)
            });
            labelled(form, "With unit", &format!("{}", *unit), |ui| {
                let edit = NumberField::new("number-field-unit", "Gap", *unit, &numbers)
                    .range(0.0..=100.0)
                    .step(0.5)
                    .decimals(0..=2)
                    .unit(Unit::new("{n} mm"))
                    .size(size)
                    .show(ui);
                super::slider::store(&edit, unit)
            });
            labelled(form, "Mixed", &format!("{}", *mixed), |ui| {
                let edit =
                    NumberField::new("number-field-mixed", "Size", *mixed, &numbers).unit(Unit::new("{n} pt")).mixed(is_mixed).size(size).show(ui);
                if edit.event.as_ref().is_some_and(|e| matches!(e, EditEvent::Set { .. } | EditEvent::Commit { changed: true, .. })) {
                    ui.data_mut(|d| d.insert_temp(mixed_key, false));
                }
                super::slider::store(&edit, mixed)
            });
            labelled(form, "Invalid", &format!("{}", *invalid), |ui| {
                let edit =
                    NumberField::new("number-field-invalid", "Columns", *invalid, &numbers).range(1.0..=12.0).invalid(true).size(size).show(ui);
                super::slider::store(&edit, invalid)
            });
            labelled(form, "Pop-up slider", &format!("{}", *opacity), |ui| {
                let edit = NumberField::new("number-field-popup", "Opacity", *opacity, &numbers)
                    .range(0.0..=100.0)
                    .unit(Unit::new("{n}%"))
                    .popup_slider("Opacity slider")
                    .size(size)
                    .show(ui);
                super::slider::store(&edit, opacity)
            });
            labelled(form, "Zoom stops", &format!("{}", *zoom), |ui| {
                let edit = NumberField::new("number-field-zoom", "Zoom", *zoom, &numbers)
                    .range(0.1..=12_800.0)
                    .decimals(0..=1)
                    .unit(Unit::new("{n}%"))
                    .steps_with(zoom_step)
                    .edit_menu(edit_menu)
                    .size(size)
                    .show(ui);
                super::slider::store(&edit, zoom)
            });
            labelled(form, "Disabled", &format!("{}", 64.0), |ui| {
                NumberField::new("number-field-disabled", "Disabled", 64.0, &numbers)
                    .unit(Unit::new("{n} px"))
                    .scrub_label("W")
                    .enabled(false)
                    .size(size)
                    .show(ui)
                    .response
            });
            labelled(form, "Not a number", &f64::NAN.to_string(), |ui| {
                NumberField::new("number-field-nan", "Not a number", f64::NAN, &numbers).size(size).show(ui).response
            });
        });
    });
}

/// PhotoCraft's zoom runs from 0.1 % to 12800 %.
const ZOOM_STOPS: [f64; 11] = [12.5, 25.0, 50.0, 100.0, 200.0, 400.0, 800.0, 1600.0, 3200.0, 6400.0, 12_800.0];

/// The next zoom stop above `value` for a positive `notches`, the previous one below it otherwise.
fn zoom_step(value: f64, notches: i32) -> f64 {
    let mut stops = ZOOM_STOPS.into_iter();
    match notches > 0 {
        true => stops.find(|s| *s > value).unwrap_or(value),
        false => stops.rfind(|s| *s < value).unwrap_or(value),
    }
}

/// Scrub labels and icons inside the field, and a label before it.
/// A width whose unit a secondary click picks.
fn show_unit_menu(ui: &mut Ui, args: &mut LocaleSizeArgs) {
    const UNITS: [&str; 3] = ["{n} px", "{n} mm", "{n} in"];
    let size = args.size.control();
    let numbers = args.locale.numbers();
    let state_id = ui.scope_id().with("number-field-unit-menu");
    let (mut value, mut unit) = ui.data(|d| d.get_temp::<(f64, usize)>(state_id)).unwrap_or((120.0, 0));
    let entries: Vec<MenuEntry<usize>> =
        UNITS.iter().enumerate().map(|(i, u)| MenuItem::new(u.replace("{n} ", ""), i).radio(i == unit).into()).collect();
    let mut picked = None;
    variant_list(ui, "number-field-unit-menu", |form| {
        labelled(form, "Width", &format!("{value} {unit}"), |ui| {
            let edit = NumberField::new("number-field-unit", "Width", value, &numbers)
                .range(0.0..=10_000.0)
                .unit(Unit::new(UNITS[unit]))
                .unit_menu(&entries, &mut picked)
                .size(size)
                .show(ui);
            super::slider::store(&edit, &mut value)
        });
    });
    if let Some(picked) = picked {
        unit = picked;
    }
    ui.data_mut(|d| d.insert_temp(state_id, (value, unit)));
}

fn show_scrub(ui: &mut Ui, args: &mut LocaleSizeArgs) {
    let size = args.size.control();
    let numbers = args.locale.numbers();
    with_values(ui, "number-field-scrub", [1920.0, 1080.0, 15.0, 4.0, 50.0], |ui, values| {
        Stack::new(Gap::Stack).show(ui, |ui| {
            let [w, h, angle, radius, wrapped] = values;
            variant_list(ui, "number-field-scrub", |form| {
            for (salt, letter, value) in [("number-field-w", "W", w), ("number-field-h", "H", h)] {
                labelled(form, letter, &format!("{}", *value), |ui| {
                    let edit = NumberField::new(salt, letter, *value, &numbers)
                        .range(1.0..=30_000.0)
                        .unit(Unit::new("{n} px"))
                        .scrub_label(letter)
                        .size(size)
                        .show(ui);
                    super::slider::store(&edit, value)
                });
            }
            labelled(form, "Angle", &format!("{}", *angle), |ui| {
                let edit = NumberField::new("number-field-angle", "Angle", *angle, &numbers)
                    .range(-180.0..=180.0)
                    .decimals(0..=1)
                    .unit(Unit::new("{n}°"))
                    .scrub_icon(Icon::RotateCw)
                    .size(size)
                    .show(ui);
                super::slider::store(&edit, angle)
            });
            labelled(form, "Wrap", &format!("{}", *wrapped), |ui| {
                let edit = NumberField::new("number-field-wrap", "Wrapping scrub", *wrapped, &numbers)
                    .range(0.0..=100.0)
                    .unit(Unit::new("{n}%"))
                    .scrub_label("%")
                    .scrub_wrap(true)
                    .size(size)
                    .show(ui);
                super::slider::store(&edit, wrapped)
            });
            form.row("Radius", |ui, label| {
                let field = NumberField::new("number-field-radius", "Radius", *radius, &numbers).range(0.0..=500.0).unit(Unit::new("{n} px")).size(size);
                let edit = match label.scrub() {
                    Some(label) => field.scrub_by(label),
                    None => field,
                }
                .show(ui);
                super::slider::store(&edit, radius)
            });
            });
            ui.add(
                Text::new(
                    "Drag a field or its label sideways: a step a point, Shift ten, Alt a tenth as fast. Esc puts the value back. A click edits; the wheel over a field steps it. The Wrap field carries the pointer to the opposite screen edge in a full-screen window.",
                )
                .content(Content::Muted),
            );
        });
    });
}

/// A labelled row right to left: the label mirrors, the field keeps its geometry.
fn show_right_to_left(ui: &mut Ui, args: &mut LocaleSizeArgs) {
    let size = args.size.control();
    let numbers = args.locale.numbers();
    with_values(ui, "number-field-rtl", [12.5], |ui, values| {
        Stack::new(Gap::Stack).show(ui, |ui| {
            let note = "With the direction set to right to left the label moves to the right. The field keeps its geometry: digits stay centred, the unit stays at the box's end and the scrub label stays at the left.";
            ui.add(Text::new(note).content(Content::Muted));
            let value = &mut values[0];
            variant_list(ui, "number-field-rtl", |form| labelled(form, "Width", &format!("{}", *value), |ui| {
                let edit = NumberField::new("number-field-rtl", "Width", *value, &numbers).decimals(0..=2).unit(Unit::new("{n} cm")).scrub_label("W").size(size).show(ui);
                super::slider::store(&edit, value)
            }));
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
fn show_event_log(ui: &mut Ui, args: &mut PolicyArgs) {
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
