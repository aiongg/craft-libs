//! [`ViewportWindow`]: an immediate viewport with an embedded fallback.

use egui::{
    Area, Context, CursorIcon, Frame, Id, Key, Modifiers, Order, Sense, Ui, UiKind, Vec2, ViewportBuilder, ViewportCommand, ViewportId, vec2,
};

use crate::dialog::{Backdrop, Body, Dialog, DialogState, Footer, Outcome, PlaceKey, Resize, ResizeMax};
use crate::popup_list;
use crate::theme::{ToEgui, current};

/// The size an embedded window opens at when the builder names none.
const DEFAULT_SIZE: Vec2 = vec2(480.0, 360.0);
/// The least size of an embedded window when the builder names none.
const MIN_SIZE: Vec2 = vec2(240.0, 160.0);

/// Where a window's body is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hosting {
    /// In its own OS window: the window supplies the title and the close box.
    Native,
    /// Inside the main window, in a [`Dialog`]: the dialog draws the title and the close box.
    Embedded,
}

/// What [`ViewportWindow::show`] reports for one pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ViewportResponse {
    /// The user asked to close the window: the OS close button or Alt+F4, the embedded close
    /// box, or Esc in an app-modal window. The window stays until the host stops showing it.
    pub close_requested: bool,
    /// Where the body was drawn this pass.
    pub hosting: Hosting,
}

/// A window in its own viewport, or an embedded dialog where viewports are embedded.
///
/// The app decorates the builder (`craft_window::decorate`); this crate does not depend on
/// `craft-window`. The host calls [`show`](ViewportWindow::show) on every pass the window should
/// exist and stops calling it to close the window.
///
/// - **Native.** An immediate viewport, so the body borrows the host's state directly. The body
///   fills the window with `surface.panel`.
/// - **Embedded.** Used when [`Context::embed_viewports`] is set or [`embedded`](Self::embedded)
///   forces it (web, tests). The body is drawn in a resizable [`Dialog`] sized from the builder's
///   inner and minimum inner size, remembered under the viewport id.
/// - **App-modal.** [`app_modal`](Self::app_modal) makes the main window ignore input while the
///   window is open. Native: a layer over the parent's content rect takes the pointer, shows the
///   not-allowed cursor, and a press on it focuses the window. Embedded: the dialog is modal.
///   Hosts ask [`blocks_main`](Self::blocks_main) before handling shortcuts. Esc is a close
///   request in an app-modal window, unless a popup is open.
///
/// ```
/// # use craft_ui::viewport::{Hosting, ViewportWindow};
/// # egui::__run_test_ctx(|ctx| {
/// let builder = egui::ViewportBuilder::default().with_inner_size([420.0, 300.0]);
/// let shown = ViewportWindow::new(egui::ViewportId::from_hash_of("settings"), builder)
///     .title("Settings")
///     .embedded(true)
///     .show(ctx, |ui, hosting| {
///         ui.label(format!("{hosting:?}"));
///     });
/// if shown.close_requested { /* stop showing it */ }
/// # });
/// ```
#[must_use = "show the window with `show`"]
pub struct ViewportWindow {
    id: ViewportId,
    builder: ViewportBuilder,
    title: Option<String>,
    modal: bool,
    scrim: bool,
    force_embedded: bool,
}

/// What the previous pass recorded about a window, to tell when it opens.
#[derive(Clone, Copy, Default)]
struct Seen {
    pass: u64,
    /// The window's own viewport, for [`ViewportWindow::refuse_close`]; none when it is embedded.
    child: Option<ViewportId>,
}

impl ViewportWindow {
    /// A window with this viewport id. `builder` carries the app's chrome decisions, the inner
    /// size and the minimum inner size.
    pub fn new(id: ViewportId, builder: ViewportBuilder) -> Self {
        Self { id, builder, title: None, modal: false, scrim: false, force_embedded: false }
    }

    /// The window's title; it replaces the builder's.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// The main window ignores input while this window is open. `scrim` fills the parent with
    /// `surface.scrim` (an embedded window always draws the dialog's own backdrop, the scrim when
    /// set).
    pub fn app_modal(mut self, scrim: bool) -> Self {
        self.modal = true;
        self.scrim = scrim;
        self
    }

    /// Draws the body in an embedded dialog even where viewports exist.
    pub fn embedded(mut self, force: bool) -> Self {
        self.force_embedded = force;
        self
    }

    /// Whether an app-modal window of this viewport was shown on this or the previous pass: the
    /// host skips its shortcuts and commands while it is. Ask in the viewport that shows the
    /// window; it is false in the app's other windows (see [`any_app_modal`](Self::any_app_modal)).
    pub fn blocks_main(ctx: &Context) -> bool {
        ctx.data(|d| d.get_temp::<Seen>(blocks_key(ctx))).is_some_and(|seen| seen.pass + 1 >= ctx.cumulative_pass_nr())
    }

    /// Whether an app-modal window of any viewport of the context was shown on its host's
    /// current or previous pass. A multi-window app skips its shortcuts and commands in every
    /// window while this is true, and shows its window-level dialog overlay.
    pub fn any_app_modal(ctx: &Context) -> bool {
        Self::modal_window(ctx).is_some()
    }

    /// Refuses a close request of any window of the app while an app-modal window is open:
    /// cancels the close and focuses the modal window. Returns whether it did. Call it from every
    /// viewport the app shows; the modal window's own close request is not refused.
    ///
    /// Every quit request reaches the app as a close request of its root window: the title-bar
    /// close button, the macOS Dock's Quit, and Cmd+Q through `craft_quit`'s app-menu Quit (it
    /// sends `ViewportCommand::Close` to the root). [`show`](Self::show) calls this each pass; an
    /// app on macOS also calls it in `eframe::App::logic` before `craft_quit::on_close_request`,
    /// which runs before `ui` and would take the quit over otherwise. It sees the `CancelClose`
    /// and leaves the window open.
    pub fn refuse_close(ctx: &Context) -> bool {
        let Some(child) = Self::modal_window(ctx) else { return false };
        if child == Some(ctx.viewport_id()) {
            return false;
        }
        if !ctx.input(|i| i.viewport().close_requested()) {
            return false;
        }
        ctx.send_viewport_cmd(ViewportCommand::CancelClose);
        if let Some(child) = child {
            ctx.send_viewport_cmd_to(child, ViewportCommand::Focus);
        }
        true
    }

    /// Tells the window module that the host closed the window with `id` (it stops calling
    /// [`show`](Self::show)): [`blocks_main`](Self::blocks_main) and
    /// [`refuse_close`](Self::refuse_close) stop at once instead of one pass later, and a
    /// reopening on the next pass counts as an opening.
    pub fn closed(ctx: &Context, id: ViewportId) {
        let key = Id::unique(("craft-ui-viewport", id));
        let blocks = blocks_key(ctx);
        let host = ctx.viewport_id();
        ctx.data_mut(|d| {
            d.remove_temp::<Seen>(key);
            d.remove_temp::<Seen>(blocks);
            d.get_temp_mut_or_default::<Modals>(modals_key()).0.retain(|m| m.host != host);
        });
    }

    /// An app-modal window that is open in any viewport: its own viewport, or none when it is
    /// embedded.
    fn modal_window(ctx: &Context) -> Option<Option<ViewportId>> {
        let open = ctx.data(|d| d.get_temp::<Modals>(modals_key())).unwrap_or_default();
        open.0.iter().find(|m| m.seen.pass + 1 >= ctx.cumulative_pass_nr_for(m.host)).map(|m| m.seen.child)
    }

    /// Shows the window for this pass. `body` draws the contents and learns where it is hosted.
    pub fn show(self, ctx: &Context, mut body: impl FnMut(&mut Ui, Hosting)) -> ViewportResponse {
        let ViewportWindow { id, mut builder, title, modal, scrim, force_embedded } = self;
        if let Some(title) = &title {
            builder = builder.with_title(title.clone());
        }
        let title = title.or_else(|| builder.title.clone()).unwrap_or_default();
        let pass = ctx.cumulative_pass_nr();
        let key = Id::unique(("craft-ui-viewport", id));
        let opened = !ctx.data(|d| d.get_temp::<Seen>(key)).is_some_and(|seen| seen.pass + 1 >= pass);
        let blocks = blocks_key(ctx);
        // A backend without an immediate renderer makes egui draw the viewport in its own window of
        // this context. That window sits under an app-modal layer, so the next pass uses the
        // embedded dialog instead (the pass that found out is discarded).
        let fallback = key.with("no-immediate-renderer");
        let embedded = force_embedded || ctx.embed_viewports() || ctx.data(|d| d.get_temp::<bool>(fallback)).unwrap_or(false);
        let seen = Seen { pass, child: (!embedded).then_some(id) };
        let host = ctx.viewport_id();
        ctx.data_mut(|d| {
            d.insert_temp(key, seen);
            if modal {
                d.insert_temp(blocks, seen);
                let open = d.get_temp_mut_or_default::<Modals>(modals_key());
                open.0.retain(|m| m.host != host);
                open.0.push(Open { host, seen });
            }
        });

        if modal {
            Self::refuse_close(ctx);
        }
        if embedded {
            return show_embedded(ctx, key, id, &builder, &title, modal, scrim, opened, &mut body);
        }

        if modal {
            block_parent(ctx, key, id, scrim, opened);
        }
        let hosting = std::cell::Cell::new(Hosting::Native);
        let close = ctx.show_viewport_immediate(id, builder, |ui, class| {
            let ctx = ui.ctx().clone();
            hosting.set(if class == egui::ViewportClass::EmbeddedWindow { Hosting::Embedded } else { Hosting::Native });
            if hosting.get() == Hosting::Embedded {
                ctx.data_mut(|d| d.insert_temp(fallback, true));
                ctx.request_discard("the backend has no immediate viewport renderer");
                return false;
            }
            let fill = current(&ctx).color.surface.panel.to_egui();
            egui::CentralPanel::default().frame(Frame::NONE.fill(fill)).show(ui, |ui| body(ui, hosting.get()));
            let escape = modal
                && !popup_list::any_open(&ctx)
                && !egui::Popup::is_any_open(&ctx)
                && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
            escape || ctx.input(|i| i.viewport().close_requested())
        });
        ViewportResponse { close_requested: close, hosting: hosting.get() }
    }
}

/// The app-modal windows of the context, one per hosting viewport.
#[derive(Clone, Default)]
struct Modals(Vec<Open>);

#[derive(Clone, Copy)]
struct Open {
    host: ViewportId,
    seen: Seen,
}

fn modals_key() -> Id {
    Id::unique("craft-ui-viewport-modals")
}

fn blocks_key(ctx: &Context) -> Id {
    Id::unique("craft-ui-viewport-blocks").with(ctx.viewport_id())
}

/// The layer over the parent window of a native app-modal window: it takes every press and drag,
/// shows the not-allowed cursor, and a press (or the parent gaining OS focus) focuses the child.
fn block_parent(ctx: &Context, key: Id, child: ViewportId, scrim: bool, opened: bool) {
    if opened {
        // Keyboard focus is shared by all viewports: a focused field of the parent would keep
        // receiving the child's typing.
        ctx.memory_mut(|m| {
            if let Some(focused) = m.focused() {
                m.surrender_focus(focused);
            }
        });
    }
    let focused = ctx.input(|i| i.focused);
    let was_focused = ctx.data_mut(|d| {
        let was = d.get_temp::<bool>(key.with("parent-focused")).unwrap_or(focused);
        d.insert_temp(key.with("parent-focused"), focused);
        was
    });
    if focused && !was_focused && !opened {
        ctx.send_viewport_cmd_to(child, ViewportCommand::Focus);
    }
    let rect = ctx.content_rect();
    let fill = scrim.then(|| current(ctx).color.surface.scrim.to_egui());
    let layer = Area::new(key.with("block"))
        .order(Order::Foreground)
        .fixed_pos(rect.min)
        .kind(UiKind::Modal)
        .sense(Sense::click_and_drag())
        .interactable(true);
    layer.show(ctx, |ui| {
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        if let Some(fill) = fill {
            ui.painter().rect_filled(rect, 0.0, fill);
        }
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::NotAllowed);
        }
        // A press of any length: a click that ends in the frame it began is not "down" at its end.
        if response.hovered() && ui.input(|i| i.pointer.any_pressed()) {
            ui.ctx().send_viewport_cmd_to(child, ViewportCommand::Focus);
        }
    });
}

/// The body in a resizable dialog sized from the builder.
#[allow(clippy::too_many_arguments)]
fn show_embedded(
    ctx: &Context,
    key: Id,
    id: ViewportId,
    builder: &ViewportBuilder,
    title: &str,
    modal: bool,
    scrim: bool,
    opened: bool,
    body: &mut impl FnMut(&mut Ui, Hosting),
) -> ViewportResponse {
    let state_key = key.with("dialog-state");
    let mut state = ctx.data(|d| d.get_temp::<DialogState>(state_key)).unwrap_or_default();
    if opened {
        state.close();
    }
    let min = builder.min_inner_size.unwrap_or(MIN_SIZE);
    let default = builder.inner_size.unwrap_or(DEFAULT_SIZE).max(min);
    let mut dialog = Dialog::new(("craft-ui-viewport", id), title)
        .resizable(Resize { default, min, max: ResizeMax::Window })
        .body(Body::Fit)
        .footer(Footer::none())
        .place_key(PlaceKey::new(id));
    dialog = if modal { dialog.backdrop(if scrim { Backdrop::Scrim } else { Backdrop::None }) } else { dialog.modeless() };
    let shown = dialog.show(ctx, &mut state, &mut (), |ui, ()| body(ui, Hosting::Embedded));
    ctx.data_mut(|d| d.insert_temp(state_key, state));
    ViewportResponse { close_requested: matches!(shown.outcome, Some(Outcome::Cancel | Outcome::Close)), hosting: Hosting::Embedded }
}
