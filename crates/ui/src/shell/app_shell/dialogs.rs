//! The window's dialog host: one [`DialogHost`] per [`WindowKey`], shared by the shell that runs it and the bodies that
//! open dialogs on it.
//!
//! `Slot` needs `Clone` and belongs to one container id, and the host is neither `Clone` nor per container, so the
//! shell keeps the hosts in a context service keyed by window. [`WindowDialogs`] is the shared handle to one of them.

use crate::contract::BodyCx;
use crate::dialog::DialogHost;
use crate::runtime::egui::memory;
use crate::runtime::env::WindowKey;
use egui::{Context, Ui};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

/// The hosts of the context's windows.
#[derive(Default)]
struct Hosts(HashMap<WindowKey, WindowDialogs>);

/// A window's dialog host, as a handle that clones cheaply. The `AppShell` of the window runs the host after its body
/// in each pass, so a dialog opened in any pass shows over the window from that pass on and its answer waits for
/// `take` until the app reads it.
///
/// A dialog that is being run does not reach the host through this handle: it opens children with the host methods of
/// its own (`DialogHost::open_child`).
#[derive(Clone, Default)]
pub struct WindowDialogs(Arc<Mutex<DialogHost>>);

impl WindowDialogs {
    /// The host of `window`, created on first use.
    fn of(ctx: &Context, window: &WindowKey) -> WindowDialogs {
        memory::with::<Hosts, _>(ctx, |hosts| hosts.0.entry(window.clone()).or_default().clone())
    }

    /// The host of `window` when a body has asked for it.
    pub(super) fn existing(ctx: &Context, window: &WindowKey) -> Option<WindowDialogs> {
        memory::with::<Hosts, _>(ctx, |hosts| hosts.0.get(window).cloned())
    }

    /// Runs `f` on the host.
    pub fn with<R>(&self, f: impl FnOnce(&mut DialogHost) -> R) -> R {
        f(&mut self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// Runs the open dialogs for this pass.
    pub(super) fn run(&self, ui: &mut Ui) {
        self.with(|host| host.run(ui));
    }
}

impl BodyCx<'_> {
    /// The dialog host of the window this body is in; `None` where no shell scope is set. Only an `AppShell` runs the
    /// host. Two windows (two [`WindowKey`]s) have two hosts. A dialog opened here shows over the window after the body in
    /// the same pass.
    pub fn dialogs(&self) -> Option<WindowDialogs> {
        let window = self.shell()?;
        Some(WindowDialogs::of(self.memory.ctx(), window.window()))
    }
}
