//! [`ToolPill`]: a floating pill of tools over the canvas (FigCraft's bottom toolbar), for apps
//! whose tools don't sit in a rail.

#![expect(dead_code, unused_variables, unused_mut, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::hash::Hash;

use egui::{AsIdSalt, Id, Rect, Ui};

use crate::patterns::ToolSlot;

/// Where the pill sits on the canvas.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PillPlace {
    #[default]
    BottomCenter,
    TopCenter,
}

/// What [`ToolPill::show`] returns.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ToolPillResponse<T> {
    /// The tool to make current: a clicked button, or a tool chosen from a group's flyout.
    pub select: Option<T>,
    pub rect: Rect,
}

/// The pill: the same [`ToolSlot`]s a [`crate::patterns::ToolRail`] takes, laid out in a row in a
/// rounded popup surface over the canvas, each slot one button with a flyout for its group (the
/// rail's [`crate::patterns::FlyoutButton`]).
///
/// ```no_run
/// use craft_ui::canvas::ToolPill;
/// # fn demo(ui: &mut egui::Ui, canvas: egui::Rect, slots: &[craft_ui::patterns::ToolSlot<&'static str>], current: &'static str) {
/// if let Some(tool) = ToolPill::new("tools").show(ui, canvas, slots, &current).select { /* switch tool */ }
/// # }
/// ```
///
/// - **Placement:** centred at the canvas's bottom (or top) edge, inset by the canvas overlay inset;
///   it never covers the scroll bars. The canvas keeps the pointer everywhere else.
/// - **Keyboard:** a `Toolbar` region for F6 ([`crate::focus::RegionFocus`]); arrows move between
///   buttons, Enter or Space selects, Down or Alt+Down opens a group's flyout. Tool letters are
///   the app's keymap and need nothing here.
/// - **AccessKit:** a `Toolbar` named "Tools"; the current tool's button is pressed.
#[must_use = "show the pill with `show`"]
pub struct ToolPill {
    id: Id,
    place: PillPlace,
}

impl ToolPill {
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        todo!()
    }

    pub fn place(mut self, place: PillPlace) -> Self {
        todo!()
    }

    /// Draws the pill over `canvas` with `slots`, `current` shown pressed.
    // figcraft: crates/ui-egui/src/toolbar.rs:489 `pill` (shadow, panel, border at the canvas's
    // bottom centre; registered as the toolbar navigation region).
    pub fn show<T: Clone + PartialEq + Hash + Send + Sync + 'static>(
        self,
        ui: &mut Ui,
        canvas: Rect,
        slots: &[ToolSlot<T>],
        current: &T,
    ) -> ToolPillResponse<T> {
        todo!()
    }
}
