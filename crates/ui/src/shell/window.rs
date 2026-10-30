//! The window level of the shell: the title slot as the window's title bar, the resize zones of
//! an undecorated window, the macOS drag flag, the modal drag overlay, the `raw_input_hook`
//! pieces and window restore. All of it reuses [`crate::title_bar`] and `craft-window`.

use std::marker::PhantomData;

#[cfg(feature = "window-frame")]
use std::cell::Cell;
#[cfg(feature = "window-frame")]
use std::rc::Rc;

#[cfg(feature = "window-frame")]
use egui::{Context, RawInput, Ui};

#[cfg(feature = "window-frame")]
use super::{AppShell, Slot, SlotFn, WindowKey, take_tab_toggle};
#[cfg(feature = "window-frame")]
use crate::title_bar::{Frame, ModalRects, TitleBar, release_after_os_resize, resize_zones_only};
#[cfg(feature = "window-frame")]
use crate::viewport::ViewportWindow;

/// The frame used when the host names none: the OS draws the title bar.
#[cfg(feature = "window-frame")]
static NATIVE: Frame = Frame::Native;

#[cfg(feature = "window-frame")]
type BarFn<'a> = Box<dyn FnOnce(&mut Ui, TitleBar<'a>) + 'a>;

/// What the host told the shell about the window's frame and restore. Empty without the
/// `window-frame` feature.
#[derive(Default)]
pub(super) struct Level<'a> {
    #[cfg(feature = "window-frame")]
    frame: Option<&'a Frame>,
    #[cfg(feature = "window-frame")]
    native_hit_test: bool,
    #[cfg(feature = "window-frame")]
    title_bar: Option<(Slot, BarFn<'a>)>,
    #[cfg(feature = "window-restore")]
    tracker: Option<&'a mut craft_window::restore::Tracker>,
    marker: PhantomData<&'a ()>,
}

#[cfg(feature = "window-frame")]
impl<'a> AppShell<'a> {
    /// The frame decision ([`craft_window::policy::decide_for`]) for this window. It sets up the
    /// [`title_bar`](Self::title_bar), the resize zones of an undecorated window and, on macOS,
    /// the window's `movable` flag. Without it the window counts as [`Frame::Native`].
    pub fn frame(mut self, frame: &'a Frame) -> Self {
        self.level.frame = Some(frame);
        self
    }

    /// The window procedure answers the hit test, edges included (Windows' `WM_NCHITTEST`), so
    /// the shell draws no resize zones. Off by default.
    pub fn native_hit_test(mut self, native: bool) -> Self {
        self.level.native_hit_test = native;
        self
    }

    /// The title slot as the window's title bar. `add` gets the [`TitleBar`] already set up with
    /// the [`frame`](Self::frame), the window key as its `id_salt` and the native hit-test flag;
    /// it adds its labels, title and contents and calls `show` or `show_titled`.
    ///
    /// When the slot is not shown (body-only mode without a reveal, or `visible(false)`) or the
    /// shell is [bare](Self::bare), the shell draws the resize zones of an undecorated window
    /// itself, so the window still resizes. In OS full screen there are none.
    pub fn title_bar(mut self, slot: Slot, add: impl FnOnce(&mut Ui, TitleBar<'a>) + 'a) -> Self {
        self.level.title_bar = Some((slot, Box::new(add)));
        self
    }

    /// Keeps the window's placement in the app's store: [`Tracker::frame`] runs on every pass of
    /// this shell, so call it from the window's own viewport. The tracker's key must be the
    /// window key. The app calls [`Tracker::flush`] on its exit path.
    ///
    /// The root window opens through [`craft_window::restore::install`]; a further window opens
    /// from [`craft_window::restore::cascade_window`] of the `MAIN` placement with
    /// `Tracker::new(store, key, defaults)`, all windows sharing one `Store`.
    ///
    /// [`Tracker`]: craft_window::restore::Tracker
    /// [`Tracker::frame`]: craft_window::restore::Tracker::frame
    /// [`Tracker::flush`]: craft_window::restore::Tracker::flush
    #[cfg(feature = "window-restore")]
    pub fn tracker(mut self, tracker: &'a mut craft_window::restore::Tracker) -> Self {
        self.level.tracker = Some(tracker);
        self
    }

    /// Call it last in the window's pass, after its dialogs and windows are drawn and after
    /// [`AppShell::show`] in the same pass (the title bar's gap check reads the current pass). While a modal
    /// is open (`modal_open`, or any app-modal [`ViewportWindow`] of the context) it keeps the
    /// title bar's free gap draggable ([`TitleBar::modal_drag_overlay`]). Dialogs of the window
    /// push their rects with `ModalRects::push(ctx, window, rect)`, with the window key as
    /// `window`.
    pub fn finish_frame(ctx: &Context, window: &WindowKey, modal_open: bool) {
        if modal_open || ViewportWindow::any_app_modal(ctx) {
            TitleBar::modal_drag_overlay(ctx, window, &ModalRects::take(ctx, window));
        }
    }

    /// For the app's `raw_input_hook`, for the window `raw` is the input of: ends a press the OS
    /// took for a resize or move while the window is undecorated ([`release_after_os_resize`]),
    /// and takes the Tab press that toggles the chrome ([`take_tab_toggle`]). Returns true when
    /// it took a Tab; the host then toggles the reveal or its flags. No Tab is taken while an
    /// app-modal window is open in any viewport ([`tab_toggles_chrome`](super::tab_toggles_chrome)).
    pub fn raw_input_hook(ctx: &Context, raw: &mut RawInput, frame: &Frame) -> bool {
        if matches!(frame, Frame::Integrated(_)) {
            release_after_os_resize(ctx, raw);
        }
        take_tab_toggle(ctx, raw)
    }
}

/// The title slot with its bar, and a flag that the bar was drawn on this pass.
#[cfg(feature = "window-frame")]
pub(super) fn bar_slot<'a>(shell: &mut AppShell<'a>) -> Option<Rc<Cell<bool>>> {
    let (slot, add) = shell.level.title_bar.take()?;
    let frame = shell.level.frame.unwrap_or(&NATIVE);
    let (window, native) = (shell.window.clone(), shell.level.native_hit_test);
    let drawn = Rc::new(Cell::new(false));
    let flag = Rc::clone(&drawn);
    let closure: SlotFn<'a> = Box::new(move |ui| {
        flag.set(true);
        add(ui, TitleBar::new(frame).id_salt(window).native_hit_test(native));
    });
    shell.title = Some((slot, closure));
    Some(drawn)
}

/// Start of a pass: the window's tracker reads the viewport.
#[cfg(feature = "window-restore")]
pub(super) fn track(shell: &mut AppShell<'_>, ctx: &egui::Context) {
    if let Some(tracker) = shell.level.tracker.as_deref_mut() {
        debug_assert_eq!(tracker.key(), shell.window.as_str(), "craft-ui: the Tracker's key is not the AppShell's window key");
        tracker.frame(ctx);
    }
}

/// End of a pass: the resize zones when no title bar drew them, and the macOS drag flag.
#[cfg(feature = "window-frame")]
pub(super) fn finish_pass(level: &Level<'_>, ui: &mut Ui, bar_drawn: bool, state: &mut super::State) {
    let Some(frame) = level.frame else { return };
    if !bar_drawn {
        resize_zones_only(ui, frame, level.native_hit_test);
    }
    #[cfg(target_os = "macos")]
    if matches!(frame, Frame::Inset(_)) {
        craft_window::title_bar::sync_movable(ui.ctx(), &mut state.movable_applied);
    }
    #[cfg(not(target_os = "macos"))]
    let _ = state;
}
