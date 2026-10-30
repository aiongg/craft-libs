//! [`Dialog`]: placement, size, stacking layer and outcome on top of [`DialogFrame`].

use egui::accesskit::Role;
use egui::{Color32, Context, CursorIcon, Id, Key, Modal, Modifiers, Order, Rect, ScrollArea, Ui, UiKind, Vec2};

use super::frame::{DialogFrame, DialogFrameResponse};
use super::placement::{PlaceKey, keep_on_screen, next_pin};
use super::size::Width;
use super::size::{Body, Resize, remembered_size, scroll_height};
use super::state::DialogState;
use crate::edit::{self, OuterGesture};
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
}

impl Outcome {
    /// Whether the dialog is finished: it closes and the host takes it away.
    pub fn closes(self) -> bool {
        matches!(self, Outcome::Ok | Outcome::Cancel | Outcome::Close | Outcome::Alternate)
    }
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

#[derive(Clone, Copy, Debug)]
enum FooterKind {
    None,
}

/// A dialog's button area.
#[derive(Clone, Copy, Debug)]
#[must_use]
pub struct Footer {
    kind: FooterKind,
}

impl Footer {
    /// No row: tool windows and blocking prompts.
    pub fn none() -> Self {
        Self { kind: FooterKind::None }
    }
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
/// - **Placement.** The first open is centred, or at `first_place`, or at the offset the user
///   dragged it to (remembered under the [`PlaceKey`] in egui's persisted memory). After its first
///   laid-out pass its top-left stays put, so a body that grows extends down and right. The title
///   band drags; the offset is kept on screen by [`keep_on_screen`](super::keep_on_screen) and
///   the stored offset is never rewritten by it.
/// - **Size.** [`Width`] classes cap the content width at the window less margins;
///   [`resizable`](Dialog::resizable) adds a grip and remembers the size the user left it at.
///   [`Body::Scroll`] makes the body a scroll area that leaves room for the frame's other parts.
/// - **Modal** dialogs use egui's modal layer with no backdrop by default. A click outside does
///   nothing. Esc cancels when the dialog is the top modal and no popup is open. The close box is
///   Cancel. **Modeless** dialogs block nothing, return `Close` from the close box and raise when
///   pressed.
/// - **Edits.** With [`outer_gesture`](Dialog::outer_gesture) the body runs inside
///   [`edit::scope`], so its controls report only updates, and `outer` carries the gesture to
///   commit on Ok and cancel on Cancel.
#[must_use = "show the dialog with `show`"]
pub struct Dialog {
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
    footer: Footer,
    close_label: Option<String>,
    outer_gesture: bool,
    initial_focus: InitialFocus,
    roomy: bool,
}

impl Dialog {
    /// A modal dialog. `id` is the host's key for it: the id of its area.
    pub fn new(id: impl std::hash::Hash + std::fmt::Debug, title: impl Into<String>) -> Self {
        Self {
            id: Id::unique(("craft_ui::dialog", id)),
            title: title.into(),
            modal: true,
            backdrop: Backdrop::None,
            width: Width::Standard,
            resize: None,
            remember_size: None,
            body: Body::Fit,
            place: None,
            first_place: None,
            footer: Footer::none(),
            close_label: Some("Close".to_owned()),
            outer_gesture: false,
            initial_focus: InitialFocus::Auto,
            roomy: false,
        }
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

    /// Where the dialog first opens when it has no remembered place: an offset from the window's
    /// top-left (Blur Gallery: the right edge). `None` centres it.
    pub fn first_place(mut self, first: impl FnOnce(&Context) -> Option<Vec2> + 'static) -> Self {
        self.first_place = Some(Box::new(first));
        self
    }

    pub fn footer(mut self, footer: Footer) -> Self {
        self.footer = footer;
        self
    }

    /// The close box's accessible name (default "Close"); `None` removes the box.
    pub fn close_label(mut self, label: Option<String>) -> Self {
        self.close_label = label;
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

    /// Shows the dialog for this pass. `body` draws the controls over `fields`.
    pub fn show<H: Clone>(self, ctx: &Context, state: &mut DialogState<H>, fields: &mut H, body: impl FnOnce(&mut Ui, &mut H)) -> DialogResponse {
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
            footer,
            close_label,
            outer_gesture,
            initial_focus,
            roomy,
        } = self;
        let tokens = current(ctx);
        let t = tokens.component.dialog;
        let window = ctx.content_rect();
        let place = place.unwrap_or(PlaceKey(id));
        let opened = !state.is_open();
        let (pin_id, size_id, chrome_id, title_id) = (id.with("pin"), id.with("laid-out-size"), id.with("chrome"), id.with("title-height"));
        if opened {
            state.begin(ctx, fields.clone(), outer_gesture);
            ctx.data_mut(|d| {
                d.remove_temp::<Vec2>(pin_id);
                d.remove_temp::<Vec2>(size_id);
            });
        }
        let outer = state.outer();

        let pinned = ctx.data(|d| d.get_temp::<Vec2>(pin_id));
        let laid_out_size = ctx.data(|d| d.get_temp::<Vec2>(size_id)).unwrap_or(Vec2::ZERO);
        let title_height = ctx.data(|d| d.get_temp::<f32>(title_id)).unwrap_or(t.close_box);
        let chrome = ctx.data(|d| d.get_temp::<f32>(chrome_id)).unwrap_or(0.0);
        let start = pinned.or_else(|| place.remembered_place(ctx)).or_else(|| first_place.and_then(|first| first(ctx)));

        let mut area = Modal::default_area(id).role(Role::Dialog).accessible_name(title.as_str());
        if let Some(offset) = start {
            area = area.anchor(egui::Align2::LEFT_TOP, keep_on_screen(offset, laid_out_size, title_height, window.size(), t.keep_visible));
        }
        if !modal {
            area = area.order(Order::Middle).kind(UiKind::Window);
        }

        let pad = if roomy { t.pad_roomy } else { t.pad };
        let remembered = resize.and_then(|_| place.remembered_size(ctx));
        let resize_sizes = resize.map(|r| (r, r.max_size(&t, window.size())));
        let (sizing_out, frame_out) = (std::cell::Cell::new(false), std::cell::Cell::new(None::<DialogFrameResponse>));
        let draw = |ui: &mut Ui| {
            sizing_out.set(ui.is_sizing_pass());
            let mut frame = DialogFrame::new(id.with("frame"), title.as_str()).field_tab(true).roomy(roomy);
            if let Some(label) = &close_label {
                frame = frame.close_label(label.as_str());
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
                let inner = |ui: &mut Ui| body(ui, fields);
                let scoped = |ui: &mut Ui| match body_mode {
                    Body::Fit => inner(ui),
                    Body::Scroll { reserve } => {
                        let height = scroll_height(&t, window.height(), chrome, reserve);
                        ScrollArea::vertical().max_height(height).auto_shrink([false, true]).show(ui, inner);
                    }
                };
                if outer_gesture {
                    edit::scope(ui, scoped);
                } else {
                    scoped(ui);
                }
            };
            let shown = match footer.kind {
                FooterKind::None => frame.show(ui, draw_body),
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
            return DialogResponse { outcome: None, rect: response.rect, outer, opened, initial_focus: None };
        };

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

        let mut outcome = frame.close.as_ref().filter(|close| close.clicked()).map(|_| if modal { Outcome::Cancel } else { Outcome::Close });
        if outcome.is_none()
            && let Some((top, popup)) = modal_state
            && top
            && !popup
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            outcome = Some(Outcome::Cancel);
        }

        let focus = (!sizing && initial_focus != InitialFocus::None && state.focus_due() && !ctx.input(|i| i.pointer.any_down())).then(|| {
            state.take_focus_due();
            initial_focus
        });
        if outcome.is_some_and(Outcome::closes) {
            if let Some(focus) = state.return_focus() {
                ctx.memory_mut(|m| m.request_focus(focus));
            }
            state.close();
        }
        DialogResponse { outcome, rect: response.rect, outer, opened, initial_focus: focus }
    }
}
