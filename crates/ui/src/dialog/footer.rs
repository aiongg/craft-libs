//! [`Footer`]: a dialog's button area, in the shapes dialogs use.

use egui::{Id, Sense, Ui, UiBuilder, vec2};

use super::buttons::{ButtonRole, Clicked, FooterButton, FooterButtons, FooterButtonsResponse, Platform};
use super::keys::EnterRule;
use super::preview::Preview;
use crate::composites::measure;
use crate::controls::Button;
use crate::direction;
use crate::localized::Labels as _;
use crate::theme::{Tokens, TokensOf};
use craft_theme::ControlSize;

crate::words! {
    /// The labels of the buttons a footer names itself, English by default; a footer the host gave
    /// no labels takes them from the installed [`Words`](crate::localized::Words).
    #[non_exhaustive]
    pub struct DialogLabels {
        ok: "dialog.ok" = "OK";
        cancel: "dialog.cancel" = "Cancel";
        apply: "dialog.apply" = "Apply";
        reset: "dialog.reset" = "Reset";
        /// A screen reader's description of the default button.
        default_hint: "dialog.defaultHint" = "Default";
        /// The accessible name of a dialog frame's close box.
        close: "dialog.close" = "Close";
    }
}

/// What holding Alt (Option on macOS) does to the Cancel button.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Reset {
    /// While Alt is held Cancel shows this label, and a click or Space on it answers
    /// [`Outcome::Reset`](super::Outcome::Reset): the dialog stays open. Esc stays Cancel.
    AltCancel(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    None,
    Standard,
    Side,
    Custom,
}

#[derive(Clone, Debug, Default)]
enum Label {
    #[default]
    Default,
    Text(String),
    Off,
}

impl Label {
    fn get(&self, default: &str) -> Option<String> {
        match self {
            Label::Default => Some(default.to_owned()),
            Label::Text(text) => Some(text.clone()),
            Label::Off => None,
        }
    }
}

type Enabled<'a> = Box<dyn FnOnce() -> bool + 'a>;
type Slot<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;
type Custom<'a> = Box<dyn FnOnce(&mut Ui, &mut FooterFx) + 'a>;

/// What the footer shows and the dialog needs from it, for one pass.
#[derive(Debug, Default)]
pub(super) struct FooterRun {
    pub clicked: Option<Clicked>,
    /// The Cancel button was showing the Reset label when it was clicked.
    pub reset: bool,
    /// The side column's extra button clicked, by its listed place.
    pub extra: Option<usize>,
    pub button_focused: bool,
    /// The enabled default button's id.
    pub default: Option<Id>,
}

/// The dialog's view for the footer.
#[derive(Clone, Copy, Debug)]
pub(super) struct FooterCx {
    pub id: Id,
    pub platform: Platform,
}

/// What a [`Footer::custom`] closure draws with.
pub struct FooterFx {
    cx: FooterCx,
    hint: String,
    run: FooterRun,
    rows: usize,
}

impl FooterFx {
    /// Shows `buttons` as a row of role buttons at the trailing edge of the room `ui` has, in the
    /// platform's order. Enter and Esc are still resolved by the dialog: Enter answers the
    /// [`Default`](ButtonRole::Default) button's outcome while it is enabled.
    pub fn buttons(&mut self, ui: &mut Ui, buttons: Vec<FooterButton>) -> FooterButtonsResponse {
        self.rows += 1;
        let row = FooterButtons::new((self.cx.id, self.rows), buttons).platform(self.cx.platform).default_hint(self.hint.clone()).show(ui);
        self.run.clicked = self.run.clicked.or(row.clicked);
        self.run.button_focused |= row.focused;
        self.run.default = self.run.default.or(row.default.as_ref().map(|r| r.id));
        row
    }
}

/// A dialog's button area.
///
/// ```
/// # use craft_ui::dialog::{Footer, Reset};
/// let footer = Footer::standard().ok("Create").cancel("Close").apply("Apply").ok_enabled(true).reset(Reset::AltCancel("Reset".into()));
/// # let _ = footer;
/// ```
///
/// - [`standard`](Footer::standard): the row of role buttons at the trailing edge, in the
///   platform's order ([`ButtonRole::slot`]): OK and Cancel, plus Apply, up to two Alternates and
///   the Reset-on-Alt rule. [`leading`](Footer::leading) fills the start edge (a Preview check
///   box, Save and Load Settings) and [`trailing_extra`](Footer::trailing_extra) the room before
///   the buttons.
/// - [`side`](Footer::side): a column at the body's trailing side with OK above Cancel and the
///   [`extra`](Footer::extra) buttons under them, all as wide as the longest label and at least
///   `dialog.side_min`; the same order on every platform. [`below`](Footer::below) adds content
///   under the buttons.
/// - [`custom`](Footer::custom): the host draws the footer and places role buttons with
///   [`FooterFx::buttons`].
/// - [`none`](Footer::none): no row.
///
/// The buttons the host doesn't name take their labels from [`labels`](Footer::labels), or from the
/// installed [`Words`](crate::localized::Words) when none were given.
/// OK disabled ([`ok_enabled`](Footer::ok_enabled)) takes no click and no Enter.
#[must_use = "give the footer to `Dialog::footer`"]
pub struct Footer<'a> {
    kind: Kind,
    labels: DialogLabels,
    /// The host passed `labels`; otherwise they are read from the installed words when shown.
    labels_set: bool,
    ok: Label,
    cancel: Label,
    apply: Option<String>,
    alternates: Vec<String>,
    extras: Vec<String>,
    ok_enabled: bool,
    apply_enabled: bool,
    ok_when: Option<Enabled<'a>>,
    apply_when: Option<Enabled<'a>>,
    reset: Option<Reset>,
    enter: EnterRule,
    preview: Option<String>,
    leading: Option<Slot<'a>>,
    below: Option<Slot<'a>>,
    trailing_extra: Option<Slot<'a>>,
    custom: Option<Custom<'a>>,
}

impl<'a> Footer<'a> {
    fn of(kind: Kind) -> Self {
        Self {
            kind,
            labels: DialogLabels::default(),
            labels_set: false,
            ok: Label::Default,
            cancel: Label::Default,
            apply: None,
            alternates: Vec::new(),
            extras: Vec::new(),
            ok_enabled: true,
            apply_enabled: true,
            ok_when: None,
            apply_when: None,
            reset: None,
            enter: EnterRule::Default,
            preview: None,
            leading: None,
            below: None,
            trailing_extra: None,
            custom: None,
        }
    }

    /// No row: tool windows and blocking prompts.
    pub fn none() -> Self {
        Self::of(Kind::None)
    }

    /// The OK and Cancel row.
    pub fn standard() -> Self {
        Self::of(Kind::Standard)
    }

    /// A vertical column of buttons at the body's trailing side.
    pub fn side() -> Self {
        Self::of(Kind::Side)
    }

    /// A footer the host draws.
    pub fn custom(draw: impl FnOnce(&mut Ui, &mut FooterFx) + 'a) -> Self {
        Self { custom: Some(Box::new(draw)), ..Self::of(Kind::Custom) }
    }

    /// The translated labels of the buttons the host doesn't name.
    pub fn labels(mut self, labels: DialogLabels) -> Self {
        self.labels = labels;
        self.labels_set = true;
        self
    }

    /// The Default button's label.
    pub fn ok(mut self, label: impl Into<String>) -> Self {
        self.ok = Label::Text(label.into());
        self
    }

    /// No Default button (a job's progress shows Cancel only).
    pub fn without_ok(mut self) -> Self {
        self.ok = Label::Off;
        self
    }

    /// The Cancel button's label.
    pub fn cancel(mut self, label: impl Into<String>) -> Self {
        self.cancel = Label::Text(label.into());
        self
    }

    /// No Cancel button (an alert).
    pub fn without_cancel(mut self) -> Self {
        self.cancel = Label::Off;
        self
    }

    /// An Apply button: it answers [`Outcome::Apply`](super::Outcome::Apply) and the dialog
    /// stays open.
    pub fn apply(mut self, label: impl Into<String>) -> Self {
        self.apply = Some(label.into());
        self
    }

    /// An Alternate button ("Don't Save"); a footer holds two at most, in listed order.
    pub fn alternate(mut self, label: impl Into<String>) -> Self {
        debug_assert!(self.alternates.len() < 2, "craft-ui: a footer holds two Alternate buttons at most");
        self.alternates.push(label.into());
        self
    }

    /// A button of the [`side`](Footer::side) column under Cancel. It never closes the dialog:
    /// [`DialogResponse::extra`](super::DialogResponse::extra) reports it.
    pub fn extra(mut self, label: impl Into<String>) -> Self {
        self.extras.push(label.into());
        self
    }

    /// Whether OK takes input; Enter does nothing while it doesn't.
    pub fn ok_enabled(mut self, enabled: bool) -> Self {
        self.ok_enabled = enabled;
        self
    }

    pub fn apply_enabled(mut self, enabled: bool) -> Self {
        self.apply_enabled = enabled;
        self
    }

    /// Like [`ok_enabled`](Footer::ok_enabled), but decided by `enabled` once the body has run in
    /// the same pass, so an edit in the body enables OK at once. The body and `enabled` share the
    /// answer through a `Cell` the body sets.
    pub fn ok_enabled_with(mut self, enabled: impl FnOnce() -> bool + 'a) -> Self {
        self.ok_when = Some(Box::new(enabled));
        self
    }

    /// [`apply_enabled`](Footer::apply_enabled), decided after the body like
    /// [`ok_enabled_with`](Footer::ok_enabled_with).
    pub fn apply_enabled_with(mut self, enabled: impl FnOnce() -> bool + 'a) -> Self {
        self.apply_when = Some(Box::new(enabled));
        self
    }

    fn resolve_enabled(&mut self) {
        self.ok_enabled = self.ok_when.take().map_or(self.ok_enabled, |f| f());
        self.apply_enabled = self.apply_when.take().map_or(self.apply_enabled, |f| f());
    }

    /// Opts in to Alt turning Cancel into Reset.
    pub fn reset(mut self, reset: Reset) -> Self {
        self.reset = Some(reset);
        self
    }

    /// Who gets Enter.
    pub fn enter(mut self, rule: EnterRule) -> Self {
        self.enter = rule;
        self
    }

    /// Controls at the start edge of the [`standard`](Footer::standard) row.
    pub fn leading(mut self, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.leading = Some(Box::new(add));
        self
    }

    /// Content under the buttons of the [`side`](Footer::side) column (a Preview check box), at
    /// the column's start edge, a button gap below the last button.
    pub fn below(mut self, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.below = Some(Box::new(add));
        self
    }

    /// The Preview check box (the dialog's [`Preview`] flags, see
    /// [`DialogState::preview`](super::DialogState::preview)) at the start edge of the
    /// [`standard`](Footer::standard) row, before any [`leading`](Footer::leading) controls. A host
    /// that places the box in the body keeps its own [`Preview`] and draws it with
    /// [`Preview::checkbox`].
    pub fn preview(mut self, label: impl Into<String>) -> Self {
        self.preview = Some(label.into());
        self
    }

    /// Controls before the buttons, at the trailing edge of the [`standard`](Footer::standard) row
    /// (Color Table's Load and Save).
    pub fn trailing_extra(mut self, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.trailing_extra = Some(Box::new(add));
        self
    }

    pub(super) fn is_none(&self) -> bool {
        self.kind == Kind::None
    }

    pub(super) fn is_side(&self) -> bool {
        self.kind == Kind::Side
    }

    pub(super) fn enter_rule(&self) -> EnterRule {
        self.enter
    }

    fn resolve_labels(&mut self, ui: &Ui) {
        if !self.labels_set {
            self.labels = DialogLabels::of(ui);
        }
    }

    fn alt_reset(&self, ui: &Ui) -> Option<&str> {
        let Reset::AltCancel(label) = self.reset.as_ref()?;
        ui.input(|i| i.modifiers.alt).then_some(label.as_str())
    }

    fn run_of(&self, row: FooterButtonsResponse, reset: bool) -> FooterRun {
        FooterRun {
            clicked: row.clicked,
            reset: reset && row.clicked.is_some_and(|c| c.role == ButtonRole::Cancel),
            extra: None,
            button_focused: row.focused,
            default: row.default.map(|r| r.id),
        }
    }

    fn row_buttons(&self, reset: Option<&str>) -> Vec<FooterButton> {
        let mut buttons = Vec::new();
        if let Some(label) = self.ok.get(&self.labels.ok) {
            buttons.push(FooterButton::new(ButtonRole::Default, label).enabled(self.ok_enabled));
        }
        if let Some(label) = self.cancel.get(&self.labels.cancel) {
            buttons.push(FooterButton::new(ButtonRole::Cancel, reset.map_or(label, str::to_owned)));
        }
        if let Some(label) = &self.apply {
            buttons.push(FooterButton::new(ButtonRole::Apply, label.as_str()).enabled(self.apply_enabled));
        }
        buttons.extend(self.alternates.iter().map(|label| FooterButton::new(ButtonRole::Alternate, label.as_str())));
        buttons
    }

    /// Shows a [`standard`](Footer::standard) or [`custom`](Footer::custom) footer in the room
    /// `ui` has.
    pub(super) fn show_row(mut self, ui: &mut Ui, cx: FooterCx, preview: &mut Preview) -> FooterRun {
        self.resolve_labels(ui);
        if let Some(draw) = self.custom.take() {
            let mut fx = FooterFx { cx, hint: self.labels.default_hint.clone(), run: FooterRun::default(), rows: 0 };
            draw(ui, &mut fx);
            return fx.run;
        }
        self.resolve_enabled();
        let tokens = Tokens::of(ui);
        let gap = tokens.component.dialog.button_gap;
        let (_, rect) = ui.allocate_space(vec2(ui.available_width(), tokens.component.button.height.get(ControlSize::Md)));
        // The leading `Ui` is made first so that AccessKit lists it first; it is filled once the
        // trailing side's width is known.
        let mut lead = ui.new_child(UiBuilder::new().max_rect(rect));
        let reset = self.alt_reset(ui).map(str::to_owned);
        let buttons = FooterButtons::new((cx.id, "buttons"), self.row_buttons(reset.as_deref()))
            .platform(cx.platform)
            .default_hint(self.labels.default_hint.clone());
        let trailing_extra = self.trailing_extra.take();
        let buttons_id = ui.make_persistent_id((cx.id, "buttons"));
        let (row, used) = measure::trailing_row(ui, cx.id.with("trailing"), rect, Sense::hover(), gap, 0.0, |row| {
            if let Some(extra) = trailing_extra {
                extra(row);
            }
            buttons.place(row, buttons_id)
        });
        let (check, add) = (self.preview.take(), self.leading.take());
        if check.is_some() || add.is_some() {
            let start = direction::start_x(ui, rect);
            let end = if used.width() > 0.0 { direction::start_x(ui, used) - direction::forward(ui) * gap } else { direction::end_x(ui, rect) };
            if direction::forward(ui) * (end - start) >= 1.0 {
                let band = egui::Rect::from_x_y_ranges(start.min(end)..=start.max(end), rect.y_range());
                lead.scope_builder(UiBuilder::new().max_rect(band).layout(direction::row(ui)), |ui| {
                    if let Some(label) = check {
                        preview.checkbox(ui, label);
                    }
                    if let Some(add) = add {
                        add(ui);
                    }
                });
            }
        }
        self.run_of(row, reset.is_some())
    }

    /// Shows a [`side`](Footer::side) footer: `body` at the start side and the button column at
    /// the trailing side of the room `ui` has.
    pub(super) fn show_side(mut self, ui: &mut Ui, cx: FooterCx, body: impl FnOnce(&mut Ui)) -> FooterRun {
        self.resolve_labels(ui);
        let tokens = Tokens::of(ui);
        let t = tokens.component.dialog;
        let mut labels: Vec<(ButtonRole, String)> = Vec::new();
        labels.extend(self.ok.get(&self.labels.ok).map(|l| (ButtonRole::Default, l)));
        labels.extend(self.cancel.get(&self.labels.cancel).map(|l| (ButtonRole::Cancel, l)));
        let extras_from = labels.len();
        labels.extend(self.extras.iter().map(|l| (ButtonRole::Alternate, l.clone())));
        let width = labels.iter().fold(t.side_min, |w, (_, label)| w.max(Button::new(label.as_str()).natural_width(ui)));

        let room = ui.available_rect_before_wrap();
        let (start, end, forward) = (direction::start_x(ui, room), direction::end_x(ui, room), direction::forward(ui));
        let body_end = end - forward * (width + t.button_gap);
        let span = |a: f32, b: f32| a.min(b)..=a.max(b);
        let mut body_ui =
            ui.new_child(UiBuilder::new().max_rect(egui::Rect::from_x_y_ranges(span(start, body_end), room.y_range())).layout(direction::column(ui)));
        body_ui.set_min_width((body_end - start).abs());
        body(&mut body_ui);
        self.resolve_enabled();

        let mut column = ui.new_child(
            UiBuilder::new().max_rect(egui::Rect::from_x_y_ranges(span(end - forward * width, end), room.y_range())).layout(direction::column(ui)),
        );
        column.spacing_mut().item_spacing.y = t.button_gap;
        let mut run = FooterRun::default();
        for (index, (role, label)) in labels.iter().enumerate() {
            let variant = if *role == ButtonRole::Default { craft_theme::ButtonVariant::Primary } else { craft_theme::ButtonVariant::Secondary };
            let enabled = *role != ButtonRole::Default || self.ok_enabled;
            let response = column.add(Button::new(label.as_str()).variant(variant).enabled(enabled).min_width(width).id_salt((cx.id, "side", index)));
            run.button_focused |= response.has_focus();
            if enabled {
                crate::behavior::FieldTab::register_stop(&column, response.id);
            }
            if *role == ButtonRole::Default && enabled {
                run.default = Some(response.id);
                let hint = self.labels.default_hint.clone();
                column.ctx().accesskit_node_builder(response.id, |node| node.set_description(hint));
            }
            if response.clicked() {
                if index >= extras_from {
                    run.extra = Some(index - extras_from);
                } else {
                    run.clicked = Some(Clicked { index, role: *role, nth: 0 });
                }
            }
        }
        if let Some(below) = self.below.take() {
            below(&mut column);
        }
        ui.allocate_rect(body_ui.min_rect().union(column.min_rect()), Sense::hover());
        run
    }
}
