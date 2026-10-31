//! [`Dialog`]: placement, size, stacking layer and outcome on top of [`DialogFrame`].

use egui::accesskit::Role;
use egui::{Color32, Context, CursorIcon, Id, Key, Modal, Modifiers, Order, Rect, ScrollArea, Ui, UiKind, Vec2};

use super::buttons::{ButtonRole, Platform};
use super::focus_scroll::scroll_focus_into_view;
use super::footer::DialogLabels;
use super::footer::{Footer, FooterCx, FooterRun};
use super::frame::{DialogFrame, DialogFrameResponse};
use super::history::{LocalHistory, Step, take_step};
use super::keys::{KeyInput, resolve_keys};
use super::pass::{self, Entry};
use super::placement::{PlaceKey, keep_on_screen, next_pin};
use super::size::Width;
use super::size::{Body, Resize, remembered_size, scroll_height};
use super::stack::{DialogId, DialogStack, dialog_id, older_above};
use super::state::DialogState;
use crate::edit::{self, OuterGesture};
use crate::localized::Labels as _;
use crate::popup_list;
use crate::theme::{ToEgui, current};

/// How a dialog was answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Outcome {
    /// The default button, Enter.
    Ok,
    /// Cancel, Esc, or the close box of a modal dialog.
    Cancel,
    /// An alternate button ("Don't Save").
    Alternate,
    /// Apply: the dialog stays open.
    Apply,
    /// Reset: the dialog stays open.
    Reset,
    /// The close box of a modeless dialog.
    Close,
    /// The button of this role, for a dialog that asks a question ([`Prompt`](super::Prompt)).
    Answer(ButtonRole),
}

impl Outcome {
    /// Whether the dialog is finished: it closes and the host takes it away.
    pub fn closes(self) -> bool {
        matches!(self, Outcome::Ok | Outcome::Cancel | Outcome::Close | Outcome::Alternate | Outcome::Answer(_))
    }
}

/// A plain press of Enter that is not a key repeat, as the fields count it.
pub(super) fn is_enter_press(event: &egui::Event) -> bool {
    matches!(event, egui::Event::Key { key: Key::Enter, pressed: true, repeat: false, modifiers, .. } if Modifiers { shift: false, ..*modifiers }.is_none())
}

/// Whether the keyboard focus is on a widget of `layer`.
fn focus_inside(ctx: &Context, layer: egui::LayerId) -> bool {
    ctx.memory(|m| m.focused()).and_then(|focused| ctx.read_response(focused)).is_some_and(|r| r.layer_id == layer)
}

/// What lies behind a modal dialog.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Backdrop {
    /// Nothing is drawn, as in Photoshop and PhotoCraft: previews are judged at true contrast. The
    /// window still takes no input.
    #[default]
    None,
    /// `surface.scrim`, for embedded viewport windows and blocking overlays with chrome.
    Scrim,
}

/// What takes the keyboard focus when a dialog opens; [`DialogResponse::initial_focus`] reports
/// when.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InitialFocus {
    /// The first enabled field, or the default button when there is none.
    #[default]
    Auto,
    /// The first enabled field.
    FirstField,
    /// The default button.
    DefaultButton,
    /// Nothing: the keyboard stays with the view (Select and Mask).
    None,
}

/// What [`Dialog::show`] reports for one pass.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct DialogResponse {
    /// How the dialog was answered this pass, if it was. For an outcome that
    /// [closes](Outcome::closes) the dialog, the state is closed again and focus returns to the
    /// control that had it when the dialog opened.
    pub outcome: Option<Outcome>,
    /// The dialog's frame this pass.
    pub rect: Rect,
    /// The outer gesture, while the dialog has one. `opened` is true on the pass it begins: the
    /// host passes it to `GestureHost::begin_outer` then, and commits or cancels it on the
    /// outcome.
    pub outer: Option<OuterGesture>,
    /// True on the pass the dialog opened.
    pub opened: bool,
    /// Set once, on the first laid-out pass that is not a pointer press: the host (or the footer)
    /// moves the focus now.
    pub initial_focus: Option<InitialFocus>,
    /// Which Alternate button was pressed (0 or 1, in listed order) when the outcome is
    /// [`Outcome::Alternate`].
    pub alternate: Option<usize>,
    /// The side column's [`extra`](Footer::extra) button pressed this pass; the dialog stays open.
    pub extra: Option<usize>,
    /// The step the dialog-local history took this pass (see [`Dialog::history`]); `fields` already
    /// hold the restored value.
    pub history: Option<Step>,
    /// Whether the dialog takes the keys this pass: it is the interactive top modal with no popup
    /// open, or a modeless dialog that holds the keyboard focus.
    pub active: bool,
}

type FirstPlace = Box<dyn FnOnce(&Context) -> Option<Vec2>>;

/// A modal or modeless dialog: the frame, where it sits, how big it is and how it is answered.
///
/// ```
/// # use craft_ui::dialog::{Dialog, DialogState, Outcome};
/// # egui::__run_test_ctx(|ctx| {
/// let mut state = DialogState::new();
/// let mut name = String::from("Untitled");
/// let shown = Dialog::new("new-document", "New Document").show(ctx, &mut state, &mut name, |ui, name| {
///     ui.text_edit_singleline(name);
/// });
/// if shown.outcome == Some(Outcome::Cancel) { /* revert */ }
/// # });
/// ```
///
/// - **State stays with the host.** The fields are `&mut H` and the dialog never stores them;
///   `state` holds the chrome only (the outer gesture, the snapshot for Reset).
/// - **Placement.** The first open is centred (over the window, or over `center_over`'s rect), or at `first_place`, or at the offset the user
///   dragged it to (remembered under the [`PlaceKey`] in egui's persisted memory). After its first
///   laid-out pass its top-left stays put, so a body that grows extends down and right. The title
///   band drags; the offset is kept on screen by [`keep_on_screen`](super::keep_on_screen) and
///   the stored offset is never rewritten by it.
/// - **Size.** [`Width`] classes cap the content width at the window less margins;
///   [`resizable`](Dialog::resizable) adds a grip and remembers the size the user left it at.
///   [`Body::Scroll`] makes the body a scroll area that leaves room for the frame's other parts.
/// - **Modal** dialogs use egui's modal layer with no backdrop by default. A click outside does
///   nothing. Enter and Esc follow [`resolve_keys`]: Esc cancels when the dialog is the top modal and no popup is open. The close box is
///   Cancel. **Modeless** dialogs block nothing, return `Close` from the close box and raise when
///   pressed.
/// - **Edits.** With [`outer_gesture`](Dialog::outer_gesture) the body runs inside
///   [`edit::scope`], so its controls report only updates, and `outer` carries the gesture to
///   commit on Ok and cancel on Cancel.
#[must_use = "show the dialog with `show`"]
pub struct Dialog<'a> {
    id: Id,
    title: String,
    modal: bool,
    backdrop: Backdrop,
    width: Width,
    resize: Option<Resize>,
    remember_size: Option<bool>,
    body: Body,
    place: Option<PlaceKey>,
    first_place: Option<FirstPlace>,
    center_over: Option<Rect>,
    footer: Footer<'a>,
    /// `None`: the installed words' "Close"; `Some(None)`: no close box.
    close_label: Option<Option<String>>,
    outer_gesture: bool,
    initial_focus: InitialFocus,
    roomy: bool,
    history: bool,
    stack: Option<&'a DialogStack>,
    role: Role,
    description: Option<String>,
    answers: bool,
}

impl<'a> Dialog<'a> {
    /// A modal dialog. `id` is the host's key for it: the id of its area. [`dialog_id`] gives the
    /// [`DialogId`] a [`DialogStack`] knows it by.
    pub fn new(id: impl std::hash::Hash + std::fmt::Debug, title: impl Into<String>) -> Self {
        Self::with_id(dialog_id(id), title)
    }

    pub(super) fn with_id(id: DialogId, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            modal: true,
            backdrop: Backdrop::None,
            width: Width::Standard,
            resize: None,
            remember_size: None,
            body: Body::Fit,
            place: None,
            first_place: None,
            center_over: None,
            footer: Footer::none(),
            close_label: None,
            outer_gesture: false,
            initial_focus: InitialFocus::Auto,
            roomy: false,
            history: false,
            stack: None,
            role: Role::Dialog,
            description: None,
            answers: false,
        }
    }

    /// The id a [`DialogStack`] knows this dialog by.
    pub fn id(&self) -> DialogId {
        self.id
    }

    /// Makes this dialog take part in `stack`: while an entry above it is open, the whole dialog
    /// is disabled, and it comes to the front when it becomes the top. Modal dialogs only; the
    /// host pushes the id before the first `show` and removes it on the closing outcome.
    pub fn stack(mut self, stack: &'a DialogStack) -> Self {
        self.stack = Some(stack);
        self
    }

    pub(super) fn role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }

    pub(super) fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Footer buttons and Enter answer [`Outcome::Answer`] with the button's role.
    pub(super) fn answers(mut self) -> Self {
        self.answers = true;
        self
    }

    /// A tool window: no modal layer, no backdrop, no input block. Its body edits are the app's
    /// normal edits.
    pub fn modeless(mut self) -> Self {
        self.modal = false;
        self
    }

    pub fn backdrop(mut self, backdrop: Backdrop) -> Self {
        self.backdrop = backdrop;
        self
    }

    /// The content width class; `Standard` by default.
    pub fn width(mut self, width: Width) -> Self {
        self.width = width;
        self
    }

    /// Adds a grip. The contents fill the area it gives, and the width class no longer applies.
    pub fn resizable(mut self, resize: Resize) -> Self {
        self.resize = Some(resize);
        self
    }

    /// Whether a resizable dialog reopens at the size it was left at; on by default.
    pub fn remember_size(mut self, remember: bool) -> Self {
        self.remember_size = Some(remember);
        self
    }

    pub fn body(mut self, body: Body) -> Self {
        self.body = body;
        self
    }

    /// What position and size are remembered under: per command for command dialogs, per kind
    /// otherwise. The dialog's own id by default.
    pub fn place_key(mut self, key: PlaceKey) -> Self {
        self.place = Some(key);
        self
    }

    /// Centres the first open over `host` (a rect in screen coordinates: a region of the window
    /// the dialog belongs to) instead of over the window. The dialog is measured first and placed
    /// by its own size, so the centre is exact. A remembered place and `first_place` come first.
    /// The dialog stays modal to the whole window.
    pub fn center_over(mut self, host: Rect) -> Self {
        self.center_over = Some(host);
        self
    }

    /// Where the dialog first opens when it has no remembered place: an offset from the window's
    /// top-left (Blur Gallery: the right edge). `None` centres it.
    pub fn first_place(mut self, first: impl FnOnce(&Context) -> Option<Vec2> + 'static) -> Self {
        self.first_place = Some(Box::new(first));
        self
    }

    /// The button area; [`Footer::none`] by default. See [`Footer`] for the shapes and
    /// [`resolve_keys`] for Enter and Esc.
    pub fn footer(mut self, footer: Footer<'a>) -> Self {
        self.footer = footer;
        self
    }

    /// The close box's accessible name (default the installed words' "Close"); `None` removes the box.
    pub fn close_label(mut self, label: Option<String>) -> Self {
        self.close_label = Some(label);
        self
    }

    /// Whether the dialog holds an [`OuterGesture`] from open to answer.
    pub fn outer_gesture(mut self, outer: bool) -> Self {
        self.outer_gesture = outer;
        self
    }

    pub fn initial_focus(mut self, focus: InitialFocus) -> Self {
        self.initial_focus = focus;
        self
    }

    /// The larger frame margin, `dialog.pad_roomy`.
    pub fn roomy(mut self, roomy: bool) -> Self {
        self.roomy = roomy;
        self
    }

    /// Keeps a [`LocalHistory`] of the fields: Cmd/Ctrl+Z and Shift+Cmd/Ctrl+Z step them while the
    /// dialog is the top modal (or has the focus, when modeless) and no text field is editing, and
    /// the keys never reach the document. A drag or typing run is one step.
    /// [`DialogResponse::history`] reports a step so an edit preview re-applies the restored
    /// fields as an update. The history covers the whole of `H`: the host keeps chrome that
    /// should not be undone outside it.
    pub fn history(mut self, history: bool) -> Self {
        self.history = history;
        self
    }

    /// Shows the dialog for this pass. `body` draws the controls over `fields`.
    pub fn show<H: Clone + PartialEq>(
        self,
        ctx: &Context,
        state: &mut DialogState<H>,
        fields: &mut H,
        body: impl FnOnce(&mut Ui, &mut H),
    ) -> DialogResponse {
        self.show_with_below(ctx, state, fields, body, |_, _| {})
    }

    /// [`show`](Dialog::show) with `below` drawn under the body. With [`Body::Scroll`] the region
    /// stays put while the body scrolls (PhotoCraft's filter preview under its parameters); its
    /// height is measured from the pass before and the scroll area gets the room that is left. It
    /// runs inside the same [`outer_gesture`](Dialog::outer_gesture) scope as `body`.
    pub fn show_with_below<H: Clone + PartialEq>(
        self,
        ctx: &Context,
        state: &mut DialogState<H>,
        fields: &mut H,
        body: impl FnOnce(&mut Ui, &mut H),
        below: impl FnOnce(&mut Ui, &mut H),
    ) -> DialogResponse {
        let Dialog {
            id,
            title,
            modal,
            backdrop,
            width,
            resize,
            remember_size: remember,
            body: body_mode,
            place,
            first_place,
            center_over,
            footer,
            close_label,
            outer_gesture,
            initial_focus,
            roomy,
            history,
            stack,
            role,
            description,
            answers,
        } = self;
        let tokens = current(ctx);
        let t = tokens.component.dialog;
        let window = ctx.content_rect();
        let place = place.unwrap_or(PlaceKey(id));
        let opened = !state.is_open();
        let (pin_id, size_id, chrome_id, title_id) = (id.with("pin"), id.with("laid-out-size"), id.with("chrome"), id.with("title-height"));
        let measured_id = id.with("measured-size");
        let below_id = id.with("below-height");
        let inside_id = id.with("focus-inside");
        if opened {
            state.begin(ctx, fields.clone(), outer_gesture);
            state.history = history.then(|| LocalHistory::new(fields));
            ctx.data_mut(|d| {
                d.remove_temp::<Vec2>(pin_id);
                d.remove_temp::<Vec2>(size_id);
                d.remove_temp::<Vec2>(measured_id);
            });
        }
        let outer = state.outer();

        let pinned = ctx.data(|d| d.get_temp::<Vec2>(pin_id));
        let measured = ctx.data(|d| d.get_temp::<Vec2>(measured_id));
        let laid_out_size = ctx.data(|d| d.get_temp::<Vec2>(size_id)).or(measured).unwrap_or(Vec2::ZERO);
        let title_height = ctx.data(|d| d.get_temp::<f32>(title_id)).unwrap_or(t.close_box);
        let chrome = ctx.data(|d| d.get_temp::<f32>(chrome_id)).unwrap_or(0.0);
        let below_used = ctx.data(|d| d.get_temp::<f32>(below_id)).unwrap_or(0.0);
        let below_now = std::cell::Cell::new(0.0_f32);
        let centred = || Some(center_offset(center_over?, measured?, window));
        let start = pinned.or_else(|| place.remembered_place(ctx)).or_else(|| first_place.and_then(|first| first(ctx))).or_else(centred);

        let blocked = modal && stack.is_some_and(|s| s.blocks(id));
        let mut area = Modal::default_area(id).role(role).accessible_name(title.as_str());
        if let Some(offset) = start {
            area = area.anchor(egui::Align2::LEFT_TOP, keep_on_screen(offset, laid_out_size, title_height, window.size(), t.keep_visible));
        }
        if !modal {
            area = area.order(Order::Middle).kind(UiKind::Window);
        }

        let pad = if roomy { t.pad_roomy } else { t.pad };
        let remembered = resize.and_then(|_| place.remembered_size(ctx));
        let resize_sizes = resize.map(|r| (r, r.max_size(&t, window.size())));
        let footer_close = footer.host_close_label().map(str::to_owned);
        let (has_footer, enter_rule) = (!footer.is_none(), footer.enter_rule());
        let run_out = std::cell::RefCell::new(FooterRun::default());
        let (sizing_out, frame_out) = (std::cell::Cell::new(false), std::cell::Cell::new(None::<DialogFrameResponse>));
        let draw = |ui: &mut Ui| {
            sizing_out.set(ui.is_sizing_pass());
            if blocked {
                ui.disable();
            }
            let mut frame = DialogFrame::new(id.with("frame"), title.as_str()).field_tab(true).roomy(roomy);
            match &close_label {
                None => frame = frame.close_label(footer_close.clone().unwrap_or_else(|| DialogLabels::of(ui).close)),
                Some(Some(label)) => frame = frame.close_label(label.as_str()),
                Some(None) => {}
            }
            frame = match resize_sizes {
                Some((r, max)) => frame.resize(
                    egui::Resize::default()
                        .id(id.with("resize"))
                        .with_stroke(false)
                        .default_size(remembered.unwrap_or(r.default).min(max))
                        .min_size(r.min)
                        .max_size(max),
                ),
                None => {
                    let (min, max) = width.range(&t, window.width() - 2.0 * pad);
                    frame.width(min, max)
                }
            };
            let draw_body = |ui: &mut Ui| {
                let scoped = |ui: &mut Ui| {
                    let mut height = scroll_height(&t, window.height(), chrome, below_used);
                    if let Some((_, max)) = resize_sizes {
                        height = height.min((max.y - (chrome - 2.0 * pad) - below_used).max(t.scroll_min));
                    }
                    ScrollArea::vertical().max_height(height).auto_shrink([body_mode == Body::Fit, true]).show(ui, |ui| {
                        body(ui, fields);
                        scroll_focus_into_view(ui);
                    });
                    let top = ui.min_rect().bottom();
                    below(ui, fields);
                    below_now.set(ui.min_rect().bottom() - top);
                };
                if outer_gesture {
                    edit::scope(ui, scoped);
                } else {
                    scoped(ui);
                }
            };
            let cx = FooterCx { id: id.with("footer"), platform: Platform::of(ui.ctx()) };
            let shown = if !has_footer {
                frame.show(ui, draw_body)
            } else if footer.is_side() {
                frame.show(ui, |ui| {
                    run_out.replace(footer.show_side(ui, cx, draw_body));
                })
            } else {
                frame.show_footer(ui, draw_body, |ui| {
                    run_out.replace(footer.show_row(ui, cx, state.preview_mut()));
                })
            };
            if let Some(outer) = outer {
                outer.keep(ui);
            }
            frame_out.set(Some(shown));
        };

        let (modal_state, response) = if modal {
            let color = match backdrop {
                Backdrop::None => Color32::TRANSPARENT,
                Backdrop::Scrim => tokens.color.surface.scrim.to_egui(),
            };
            let shown = Modal::new(id).area(area).backdrop_color(color).frame(egui::Frame::NONE).show(ctx, draw);
            (Some((shown.is_top_modal, shown.any_popup_open)), shown.response)
        } else {
            (None, area.show(ctx, draw).response)
        };
        let sizing = sizing_out.get();
        let Some(frame) = frame_out.take() else {
            return DialogResponse {
                outcome: None,
                rect: response.rect,
                outer,
                opened,
                initial_focus: None,
                alternate: None,
                extra: None,
                history: None,
                active: false,
            };
        };

        // The sizing pass is discarded, but its size places the pass that replaces it.
        ctx.data_mut(|d| d.insert_temp(measured_id, response.rect.size()));
        pass::record(ctx, Entry { id, layer: response.layer_id, rect: response.rect, modal });
        ctx.accesskit_node_builder(response.layer_id.id, |node| {
            if modal {
                node.set_modal();
            }
            if let Some(description) = description {
                node.set_description(description);
            }
        });
        if let Some(stack) = stack
            && modal
            && stack.is_top(id)
            && older_above(ctx, response.layer_id, stack.ids().filter(|other| *other != id).filter_map(|other| pass::layer_of(ctx, other)))
        {
            ctx.move_to_top(response.layer_id);
            ctx.request_repaint();
        }

        // The scroll area was sized with the region's height from the pass before; run the pass
        // again when it has changed.
        let below_height = below_now.get();
        ctx.data_mut(|d| d.insert_temp(below_id, below_height));
        if (below_height - below_used).abs() > 0.5 {
            ctx.request_discard("dialog region below the body changed height");
        }
        ctx.data_mut(|d| {
            d.insert_temp(chrome_id, frame.chrome);
            d.insert_temp(title_id, frame.title_height);
        });
        if !sizing {
            if let (Some((r, max)), Some(given)) = (resize_sizes, Some(frame.content_size).filter(|s| s.is_finite()))
                && remember.unwrap_or(true)
                && let Some(kept) = remembered_size(given, remembered.unwrap_or(r.default), max)
                && remembered != Some(kept)
            {
                place.remember_size(ctx, kept);
            }
            let drag = frame.drag.drag_delta();
            let laid_out = response.rect.min - window.min;
            if let Some(pin) = next_pin(pinned, laid_out, drag, response.rect.size(), frame.title_height, window.size(), t.keep_visible) {
                ctx.data_mut(|d| d.insert_temp(pin_id, pin));
                if drag != Vec2::ZERO {
                    place.remember_place(ctx, pin);
                }
            }
            ctx.data_mut(|d| d.insert_temp(size_id, response.rect.size()));
        }
        if frame.drag.dragged() {
            ctx.set_cursor_icon(CursorIcon::Grabbing);
        } else if frame.drag.hovered() {
            ctx.set_cursor_icon(CursorIcon::Grab);
        }

        let run = run_out.take();
        // Esc and a field's Enter take the focus away within the pass, so the keys go by where
        // the focus was when the pass began.
        let inside_before = ctx.data(|d| d.get_temp::<bool>(inside_id)).unwrap_or(false);
        let inside_now = focus_inside(ctx, response.layer_id);
        ctx.data_mut(|d| d.insert_temp(inside_id, inside_now));
        let (active, popup_open) = match modal_state {
            Some((top, popup)) => (top && !blocked, popup || popup_list::any_open(ctx)),
            None => (inside_before, popup_list::any_open(ctx)),
        };
        let stepped = history.then(|| local_history(ctx, state, fields, active)).flatten();
        let reset_shown = run.reset;
        let mut alternate = None;
        let mut outcome = frame.close.as_ref().filter(|close| close.clicked()).map(|_| if modal { Outcome::Cancel } else { Outcome::Close });
        if outcome.is_none()
            && let Some(clicked) = run.clicked
        {
            outcome = Some(match clicked.role {
                role if answers => Outcome::Answer(role),
                ButtonRole::Default => Outcome::Ok,
                ButtonRole::Cancel if reset_shown => Outcome::Reset,
                ButtonRole::Cancel if modal => Outcome::Cancel,
                ButtonRole::Cancel => Outcome::Close,
                ButtonRole::Apply => Outcome::Apply,
                ButtonRole::Alternate => {
                    alternate = Some(clicked.nth);
                    Outcome::Alternate
                }
            });
        }
        if outcome.is_none() {
            let (enter, escape) =
                ctx.input(|i| (!opened && i.events.iter().any(is_enter_press), i.key_pressed(Key::Escape) && i.modifiers.is_none()));
            outcome = resolve_keys(KeyInput {
                enter,
                escape,
                modal,
                active,
                footer: has_footer,
                popup_open,
                button_focused: run.button_focused,
                default_enabled: run.default.is_some(),
                enter_rule,
            });
            if answers && outcome == Some(Outcome::Ok) {
                outcome = Some(Outcome::Answer(ButtonRole::Default));
            }
            if outcome.is_some() {
                ctx.input_mut(|i| {
                    i.consume_key(Modifiers::NONE, Key::Escape);
                    i.events.retain(|event| !is_enter_press(event));
                });
            }
        }

        let focus =
            (!sizing && !blocked && initial_focus != InitialFocus::None && state.focus_due() && !ctx.input(|i| i.pointer.any_down())).then(|| {
                state.take_focus_due();
                let target = match initial_focus {
                    InitialFocus::Auto => frame.first_field.or(run.default),
                    InitialFocus::FirstField => frame.first_field,
                    InitialFocus::DefaultButton => run.default,
                    InitialFocus::None => None,
                };
                if let Some(target) = target {
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
        DialogResponse {
            outcome,
            rect: response.rect,
            outer,
            opened,
            initial_focus: focus,
            alternate,
            extra: run.extra,
            history: stepped,
            active: active && !popup_open,
        }
    }
}

/// Records the fields as a history step, or takes the step Cmd/Ctrl+Z asks for.
/// The offset from the window's top-left that centres a dialog of `size` over `host`.
fn center_offset(host: Rect, size: Vec2, window: Rect) -> Vec2 {
    host.center().to_vec2() - size / 2.0 - window.min.to_vec2()
}

fn local_history<H: Clone + PartialEq>(ctx: &Context, state: &mut DialogState<H>, fields: &mut H, active: bool) -> Option<Step> {
    let history = state.history.get_or_insert_with(|| LocalHistory::new(fields));
    let step = (active && !ctx.text_edit_focused()).then(|| take_step(ctx)).flatten();
    let stepped = step.filter(|&step| history.step(fields, step));
    if stepped.is_none() {
        history.settle(fields, ctx.input(|i| i.pointer.any_down()) || edit::focused_gesture(ctx).is_some());
    } else {
        ctx.request_repaint();
    }
    stepped
}
