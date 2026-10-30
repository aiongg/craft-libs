//! [`PreferencesWindow`]: the Preferences body in its own window.

use egui::{Context, Id, Vec2, ViewportBuilder, ViewportId, vec2};

use super::{PreferencesBody, PreferencesHost, PreferencesOutput};
use crate::dialog::Platform;
#[cfg(feature = "window-restore")]
use crate::viewport::Hosting;
use crate::viewport::ViewportWindow;

#[cfg(feature = "window-restore")]
mod remember;
#[cfg(feature = "window-restore")]
pub use remember::Remember;

/// The size the window opens at the first time.
pub const DEFAULT_SIZE: Vec2 = vec2(1040.0, 800.0);
/// The least size of the window.
pub const MIN_SIZE: Vec2 = vec2(560.0, 400.0);

type Decorate = Box<dyn Fn(ViewportBuilder) -> ViewportBuilder>;

/// The Preferences window: [`PreferencesBody`] in a [`ViewportWindow`] that is app-modal, so the
/// body is one code path for the OS window and for the embedded dialog.
///
/// The app keeps one value and calls [`show`](PreferencesWindow::show) on every pass the window
/// should exist, from the viewport that owns it; it stops calling to close the window (and
/// does so when `show` reports `closed`).
///
/// - **Modal.** While it is open [`blocks_main`](PreferencesWindow::blocks_main) is true: the app
///   skips its shortcuts and UI commands. The main window takes no pointer input and shows the
///   not-allowed cursor, a press on it focuses this window, and every quit request is refused
///   (see [`ViewportWindow::refuse_close`]).
/// - **Close.** The close button, Alt+F4 and Esc cancel like the Cancel button, except while a
///   nested dialog is open: that dialog takes input alone.
/// - **Nested dialogs.** [`PreferencesHost::nested`] draws them inside this window's viewport.
/// - **Chrome.** [`builder`](PreferencesWindow::builder) takes `craft_window::decorate`.
/// - **Size and position.** With the `window-restore` feature,
///   [`remember`](PreferencesWindow::remember) keeps them per app.
///
/// # Quit on macOS
///
/// `craft_quit::on_close_request` runs before `ui` and would take a Cmd+Q or Dock Quit over while
/// the window is open. The app asks [`refuse_close`](PreferencesWindow::refuse_close) first, in
/// `eframe::App::logic`:
///
/// ```
/// struct MyApp;
///
/// impl eframe::App for MyApp {
///     fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
///         if craft_ui::preferences::PreferencesWindow::refuse_close(ctx) {
///             return;
///         }
///         craft_quit::on_close_request(ctx);
///     }
///
///     fn ui(&mut self, _ui: &mut egui::Ui, _frame: &mut eframe::Frame) {}
/// }
/// ```
#[must_use = "show the window with `show`"]
pub struct PreferencesWindow {
    salt: Id,
    embedded: bool,
    platform: Option<Platform>,
    decorate: Option<Decorate>,
    #[cfg(feature = "window-restore")]
    remember: Option<Remember>,
}

impl PreferencesWindow {
    /// A window. `id_salt` names its viewport, its body's remembered page and its embedded
    /// placement.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            salt: Id::unique(("craft_ui::preferences::window", id_salt)),
            embedded: false,
            platform: None,
            decorate: None,
            #[cfg(feature = "window-restore")]
            remember: None,
        }
    }

    /// Draws the body in an embedded dialog even where viewports exist (web, tests).
    pub fn embedded(mut self, embedded: bool) -> Self {
        self.embedded = embedded;
        self
    }

    /// The button order to follow; the context's platform by default.
    pub fn platform(mut self, platform: Platform) -> Self {
        self.platform = Some(platform);
        self
    }

    /// Remembers the window's size and position per app; see [`Remember`].
    #[cfg(feature = "window-restore")]
    pub fn remember(mut self, remember: Remember) -> Self {
        self.remember = Some(remember);
        self
    }

    /// Changes the viewport builder before the window is created: the app passes its frame
    /// decision (`|b| craft_window::decorate(b, &frame)`).
    pub fn builder(mut self, decorate: impl Fn(ViewportBuilder) -> ViewportBuilder + 'static) -> Self {
        self.decorate = Some(Box::new(decorate));
        self
    }

    /// Whether the window is open: the app skips its shortcuts and UI commands while it is. Ask in
    /// the viewport that shows the window.
    pub fn blocks_main(ctx: &Context) -> bool {
        ViewportWindow::blocks_main(ctx)
    }

    /// Refuses a quit request while the window is open; see [`ViewportWindow::refuse_close`].
    pub fn refuse_close(ctx: &Context) -> bool {
        ViewportWindow::refuse_close(ctx)
    }

    /// The viewport id of the window, for commands to it.
    pub fn viewport_id(&self) -> ViewportId {
        ViewportId(self.salt)
    }

    /// The viewport builder: title, the default and least size, resizable.
    fn base(&self, title: &str) -> ViewportBuilder {
        ViewportBuilder::default().with_title(title).with_inner_size(DEFAULT_SIZE).with_min_inner_size(MIN_SIZE).with_resizable(true)
    }

    /// Shows the window for this pass. `closed` in the output tells the app to stop showing it.
    pub fn show<H: PreferencesHost + ?Sized>(&mut self, ctx: &Context, host: &mut H) -> PreferencesOutput {
        let title = host.texts().title;
        let builder = self.base(&title);
        #[cfg(feature = "window-restore")]
        let builder = match &mut self.remember {
            Some(remember) => remember.apply(builder, ctx.zoom_factor()),
            None => builder,
        };
        let builder = match &self.decorate {
            Some(decorate) => decorate(builder),
            None => builder,
        };
        let force = self.embedded;
        let embedded = force || ctx.embed_viewports();
        let close_key = self.salt.with("close-requested");
        let mut out = PreferencesOutput::default();
        // A close request is seen after the body ran (the embedded close box, Esc): the body cancels
        // on the next pass.
        let cancel = ctx.data_mut(|d| d.remove_temp::<bool>(close_key)).unwrap_or(false);
        let (salt, viewport, platform) = (self.salt, self.viewport_id(), self.platform);
        #[cfg(feature = "window-restore")]
        let remember = &mut self.remember;
        let shown = ViewportWindow::new(viewport, builder).title(title).app_modal(embedded).embedded(force).show(
            ctx,
            |ui, #[cfg_attr(not(feature = "window-restore"), allow(unused_variables))] hosting| {
                // Only an OS window has its own viewport state to track.
                #[cfg(feature = "window-restore")]
                if let (Some(remember), Hosting::Native) = (remember.as_mut(), hosting) {
                    remember.track(ui.ctx());
                }
                let mut body = PreferencesBody::new(salt).cancel_requested(cancel);
                if let Some(platform) = platform {
                    body = body.platform(platform);
                }
                out = body.show(ui, host);
            },
        );
        if out.closed.is_some() {
            ViewportWindow::closed(ctx, viewport);
            #[cfg(feature = "window-restore")]
            if let Some(remember) = &mut self.remember {
                remember.close();
            }
        } else if shown.close_requested && !host.nested_open() {
            ctx.data_mut(|d| d.insert_temp(close_key, true));
            ctx.request_repaint();
        }
        out
    }
}
