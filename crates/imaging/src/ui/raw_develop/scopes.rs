//! The docked and floating scope of the workspace.

use craft_ui::contract::{Bind, Pattern, Props};
use egui::Ui;

use crate::ui::scopes::{ScopeFrame, ScopePanelOutput, ScopePanelState};

/// The docked scope above the controls: a `ScopePanel` with Camera Raw's tone zones and typical histogram scale, its
/// context menu and a Float action in the section header.
#[must_use = "show the strip with `show`"]
#[derive(Pattern, Props)]
#[allow(dead_code)] // P3(dialogs-w9): dl.K67
pub struct RawScopeStrip<'a> {
    state: &'a mut ScopePanelState,
    frame: Option<&'a ScopeFrame<'a>>,
    /// Whether the scope floats; its Float action sets it.
    #[prop]
    floating: Option<Bind<'a, bool>>,
}

impl<'a> RawScopeStrip<'a> {
    /// The strip over `state` and the host's `frame`.
    pub fn new(state: &'a mut ScopePanelState, frame: Option<&'a ScopeFrame<'a>>) -> Self {
        let _ = (state, frame);
        todo!("dl.K67")
    }

    /// Draws the scope.
    pub fn show(self, ui: &mut Ui) -> ScopePanelOutput {
        let _ = (self, ui);
        todo!("dl.K67")
    }
}

/// The floating scope: a modeless child dialog of the workspace, resizable and remembered; its Dock action clears
/// `floating`.
#[must_use = "show the scope with `show`"]
#[derive(Pattern)]
#[allow(dead_code)] // P3(dialogs-w9): dl.K67
pub struct RawFloatingScope<'a> {
    state: &'a mut ScopePanelState,
    frame: Option<&'a ScopeFrame<'a>>,
    floating: Bind<'a, bool>,
}

impl<'a> RawFloatingScope<'a> {
    /// The floating scope over `state` and the host's `frame`.
    pub fn new(state: &'a mut ScopePanelState, frame: Option<&'a ScopeFrame<'a>>, floating: Bind<'a, bool>) -> Self {
        let _ = (state, frame, floating);
        todo!("dl.K67")
    }

    /// Draws the scope.
    pub fn show(self, ui: &mut Ui) -> ScopePanelOutput {
        let _ = (self, ui);
        todo!("dl.K67")
    }
}
