//! [`FullWindowDialog`]: a modal workspace that fills the window (Liquify, Filter Gallery).

use egui::accesskit::Role;
use egui::{Color32, Context, Id, Key, Modal, Modifiers, Order, Rect, Ui, UiBuilder, UiKind};

use super::buttons::{ButtonRole, FooterButton, FooterButtons, FooterButtonsResponse, Platform};
use super::dialog::{DialogResponse, InitialFocus, Outcome, is_enter_press};
use super::footer::{DialogLabels, FooterCx, FooterRun};
use super::keys::{EnterRule, KeyInput, resolve_keys};
use super::state::DialogState;
use crate::composites::PanelHeader;
use crate::edit;
use crate::popup_list;
use crate::theme::{ToEgui, current};

type TitleActions<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// What a [`FullWindowDialog`] body places its footer with.
pub struct FullWindowFx {
    cx: FooterCx,
    hint: String,
    run: FooterRun,
    rows: usize,
}

impl FullWindowFx {
    /// Shows `buttons` in the platform's order against the trailing edge of `rect` (Liquify: the
    /// bottom of its right panel; Filter Gallery: under its settings column). Enter and Esc are
    /// resolved by the dialog: Enter answers [`Outcome::Ok`] while the
    /// [`Default`](ButtonRole::Default) button is enabled.
    pub fn buttons(&mut self, ui: &mut Ui, rect: Rect, buttons: Vec<FooterButton>) -> FooterButtonsResponse {
        self.rows += 1;
        let mut row_ui = ui.new_child(UiBuilder::new().max_rect(rect));
        let row = FooterButtons::new((self.cx.id, self.rows), buttons).platform(self.cx.platform).default_hint(self.hint.clone()).show(&mut row_ui);
        self.run.clicked = self.run.clicked.or(row.clicked);
        self.run.button_focused |= row.focused;
        self.run.default = self.run.default.or(row.default.as_ref().map(|r| r.id));
        row
    }
}

/// A modal dialog over the whole window, or over the rect the host gives: Liquify, Filter Gallery,
/// Wide Angle, Content-Aware Fill.
///
/// - The surface is `surface.panel`. An optional title strip ([`title`](Self::title)) is
///   `size.panel_header` tall with `line.divider` under it; [`title_actions`](Self::title_actions)
///   adds controls at its trailing end (Liquify's zoom). The body gets the rest.
/// - The body places its own footer with [`FullWindowFx::buttons`]. The dialog is modal: other
///   input waits, and Esc and Enter follow [`resolve_keys`] ([`EnterRule::Body`] when the
///   workspace owns Enter).
/// - No close box, no placement or size memory, no drag. Apply keeps the dialog open
///   ([`Outcome::Apply`]).
/// - Local history is the host's `LocalHistory` in its fields. One at a time: each full-window
///   dialog is a pass of the host's frame.
///
/// ```
/// # use craft_ui::dialog::{ButtonRole, DialogState, FooterButton, FullWindowDialog};
/// # egui::__run_test_ctx(|ctx| {
/// let mut state = DialogState::new();
/// let shown = FullWindowDialog::new("liquify").title("Liquify").show(ctx, &mut state, &mut (), |ui, (), fx| {
///     let room = ui.max_rect();
///     fx.buttons(ui, room, vec![FooterButton::new(ButtonRole::Default, "OK"), FooterButton::new(ButtonRole::Cancel, "Cancel")]);
/// });
/// # let _ = shown;
/// # });
/// ```
#[must_use = "show the dialog with `show`"]
pub struct FullWindowDialog<'a> {
    id: Id,
    title: Option<String>,
    title_actions: Option<TitleActions<'a>>,
    rect: Option<Rect>,
    outer_gesture: bool,
    enter: EnterRule,
    initial_focus: InitialFocus,
    labels: DialogLabels,
}

impl<'a> FullWindowDialog<'a> {
    pub fn new(id: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            id: Id::unique(("craft_ui::dialog::full_window", id)),
            title: None,
            title_actions: None,
            rect: None,
            outer_gesture: false,
            enter: EnterRule::Default,
            initial_focus: InitialFocus::None,
            labels: DialogLabels::default(),
        }
    }

    /// The title strip's text, and the dialog's accessible name.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Controls at the trailing end of the title strip (a zoom field).
    pub fn title_actions(mut self, actions: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.title_actions = Some(Box::new(actions));
        self
    }

    /// The area the dialog covers; the window's content rect by default.
    pub fn rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    /// Runs the body in [`edit::scope`] under one outer gesture, as [`Dialog::outer_gesture`]
    /// does.
    ///
    /// [`Dialog::outer_gesture`]: super::Dialog::outer_gesture
    pub fn outer_gesture(mut self, outer: bool) -> Self {
        self.outer_gesture = outer;
        self
    }

    /// Whether the dialog answers Enter ([`EnterRule::Default`]) or the body owns it.
    pub fn enter(mut self, rule: EnterRule) -> Self {
        self.enter = rule;
        self
    }

    /// What takes the keyboard focus when the dialog opens: only [`InitialFocus::DefaultButton`]
    /// is acted on; the others leave the focus to the host. [`DialogResponse::initial_focus`]
    /// reports the pass either way. Nothing by default: a workspace keeps the keyboard on its
    /// view.
    pub fn initial_focus(mut self, focus: InitialFocus) -> Self {
        self.initial_focus = focus;
        self
    }

    /// The translated "default" hint the Default button is described with.
    pub fn labels(mut self, labels: DialogLabels) -> Self {
        self.labels = labels;
        self
    }

    /// Shows the dialog for this pass. `body` draws the workspace over `fields` and places the
    /// footer with the [`FullWindowFx`].
    pub fn show<H: Clone>(
        self,
        ctx: &Context,
        state: &mut DialogState<H>,
        fields: &mut H,
        body: impl FnOnce(&mut Ui, &mut H, &mut FullWindowFx),
    ) -> DialogResponse {
        let FullWindowDialog { id, title, title_actions, rect, outer_gesture, enter, initial_focus, labels } = self;
        let rect = rect.unwrap_or_else(|| ctx.content_rect());
        let opened = !state.is_open();
        if opened {
            state.begin(ctx, fields.clone(), outer_gesture);
        }
        let outer = state.outer();
        let platform = Platform::of(ctx);
        let mut fx =
            FullWindowFx { cx: FooterCx { id: id.with("footer"), platform }, hint: labels.default_hint.clone(), run: FooterRun::default(), rows: 0 };
        let panel = current(ctx).color.surface.panel.to_egui();
        let sizing = std::cell::Cell::new(false);

        let mut area = Modal::default_area(id).anchor(egui::Align2::LEFT_TOP, rect.min.to_vec2()).order(Order::Foreground).kind(UiKind::Modal);
        area = area.role(Role::Dialog);
        if let Some(title) = &title {
            area = area.accessible_name(title.as_str());
        }
        let shown = Modal::new(id).area(area).backdrop_color(Color32::TRANSPARENT).frame(egui::Frame::NONE).show(ctx, |ui| {
            sizing.set(ui.is_sizing_pass());
            ui.set_min_size(rect.size());
            ui.set_max_size(rect.size());
            ui.painter().rect_filled(ui.max_rect(), 0.0, panel);
            let inner = |ui: &mut Ui, fx: &mut FullWindowFx| {
                if let Some(title) = &title {
                    PanelHeader::new(id.with("title"), title.as_str()).menu(false).show(ui, |ui| {
                        if let Some(actions) = title_actions {
                            actions(ui);
                        }
                    });
                }
                body(ui, fields, fx);
            };
            if outer_gesture {
                edit::scope(ui, |ui| inner(ui, &mut fx));
            } else {
                inner(ui, &mut fx);
            }
            if let Some(outer) = outer {
                outer.keep(ui);
            }
        });
        let run = std::mem::take(&mut fx.run);

        let mut alternate = None;
        let mut outcome = run.clicked.map(|clicked| match clicked.role {
            ButtonRole::Default => Outcome::Ok,
            ButtonRole::Cancel => Outcome::Cancel,
            ButtonRole::Apply => Outcome::Apply,
            ButtonRole::Alternate => {
                alternate = Some(clicked.nth);
                Outcome::Alternate
            }
        });
        if outcome.is_none() {
            let (enter_pressed, escape) =
                ctx.input(|i| (!opened && i.events.iter().any(is_enter_press), i.key_pressed(Key::Escape) && i.modifiers.is_none()));
            outcome = resolve_keys(KeyInput {
                enter: enter_pressed,
                escape,
                modal: true,
                active: shown.is_top_modal,
                footer: true,
                popup_open: shown.any_popup_open || popup_list::any_open(ctx),
                button_focused: run.button_focused,
                default_enabled: run.default.is_some(),
                enter_rule: enter,
            });
            if outcome.is_some() {
                ctx.input_mut(|i| {
                    i.consume_key(Modifiers::NONE, Key::Escape);
                    i.events.retain(|e| !is_enter_press(e));
                });
            }
        }

        let focus = (!sizing.get() && state.focus_due() && !ctx.input(|i| i.pointer.any_down())).then(|| {
            state.take_focus_due();
            if initial_focus == InitialFocus::DefaultButton
                && let Some(target) = run.default
            {
                ctx.memory_mut(|m| m.request_focus(target));
            }
            initial_focus
        });
        if outcome.is_some_and(Outcome::closes) {
            if let Some(focus) = state.return_focus() {
                ctx.memory_mut(|m| m.request_focus(focus));
            }
            state.close();
        }
        DialogResponse { outcome, rect: shown.response.rect, outer, opened, initial_focus: focus, alternate, extra: None }
    }
}
