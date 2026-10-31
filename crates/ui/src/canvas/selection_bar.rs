//! [`SelectionBar`]: a small floating toolbar over a selection on the canvas (Photoshop's
//! Contextual Task Bar; WordCraft's mini toolbar over selected text).

#![expect(dead_code, unused_variables, unused_mut, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, Id, Pos2, Rect, Ui};

/// What [`SelectionBar::show`] returns.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SelectionBarResponse<R> {
    /// The body closure's result; `None` while the bar is hidden.
    pub inner: Option<R>,
    /// Where the bar was drawn.
    pub rect: Option<Rect>,
    /// The user dragged the grip: the bar stays at this screen position (relative to the
    /// canvas) until [`SelectionBar::unpin`]. The host may store it.
    pub pinned_at: Option<Pos2>,
    /// The user hid the bar from its ⋯ menu (Hide Bar); the host turns its preference off.
    pub hide_requested: bool,
    /// Reset Bar Position in the ⋯ menu: the bar follows the selection again; the host clears the
    /// position it stored.
    pub position_reset: bool,
}

/// A floating bar of the host's controls above a selection's screen rect.
///
/// ```no_run
/// use craft_ui::canvas::SelectionBar;
/// # fn demo(ui: &mut egui::Ui, canvas: egui::Rect, selection: Option<egui::Rect>) {
/// SelectionBar::new("selection-bar").show(ui, canvas, selection, |ui| {
///     let _ = ui.button("Select Subject");
/// });
/// # }
/// ```
///
/// - **Placement:** Photoshop's Contextual Task Bar: centred under `selection`, 14 pt below it;
///   above it (14 pt) when it would pass the canvas's bottom edge less 8 pt; at the canvas's
///   bottom less 8 pt when neither fits; x clamped 8 pt inside the canvas. Its width is the
///   content's, measured on the previous frame (420 pt before the first). [`SelectionBar::above`]
///   puts it 8 pt above the selection instead, flipping below when tight (Word's mini toolbar).
///   Hidden with no selection, and while a pointer button is down outside any egui area (a drag
///   on the canvas) unless the bar's grip is being dragged. A pinned bar stays where the user
///   dragged it, kept inside the canvas.
/// - **Look:** a popup surface with the toolbar's controls at the small control size, a grip at the
///   start and a ⋯ menu at the end (Hide Bar, Reset Bar Position). Themes change colours only.
/// - **Keyboard:** not in the Tab order of the window until the user moves there (F6 lists it
///   as a region while it shows, [`crate::focus::RegionFocus`]); Esc returns focus to the canvas.
/// - **AccessKit:** a `Toolbar` named by [`SelectionBarLabels::name`] ("Selection").
#[must_use = "show the bar with `show`"]
pub struct SelectionBar {
    id: Id,
    above: bool,
    pinned: Option<Pos2>,
    labels: Option<SelectionBarLabels>,
}

impl SelectionBar {
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        todo!()
    }

    /// Translated labels; default the installed words.
    pub fn labels(mut self, labels: SelectionBarLabels) -> Self {
        todo!()
    }

    /// Places the bar 8 pt above the selection, flipping below when tight.
    // wordcraft: crates/ui-egui/src/mini_toolbar.rs:12 `VGAP` (8 pt).
    pub fn above(mut self, above: bool) -> Self {
        todo!()
    }

    /// Keeps the bar at a stored position (relative to the canvas rect's top left).
    pub fn pinned(mut self, at: Option<Pos2>) -> Self {
        todo!()
    }

    /// Forgets the pinned position; the bar follows the selection again.
    pub fn unpin(ui: &Ui, id_salt: impl AsIdSalt) {
        todo!()
    }

    /// Draws the bar over `canvas` above `selection` (screen rects) with `add`'s controls.
    // designcraft: crates/ui-egui/src/taskbar.rs:18 `show` (placement :34-46, hidden while a
    // button is down off egui :22, `HEIGHT` 36 pt :11); wordcraft: crates/ui-egui/src/mini_toolbar.rs:166
    // `show`.
    pub fn show<R>(self, ui: &mut Ui, canvas: Rect, selection: Option<Rect>, add: impl FnOnce(&mut Ui) -> R) -> SelectionBarResponse<R> {
        todo!()
    }
}

crate::words! {
    /// The selection bar's labels, English by default.
    #[non_exhaustive]
    pub struct SelectionBarLabels {
        name: "selectionBar.name" = "Selection";
        grip: "selectionBar.grip" = "Move Bar";
        more: "selectionBar.more" = "More";
        hide: "selectionBar.hide" = "Hide Bar";
        reset_position: "selectionBar.resetPosition" = "Reset Bar Position";
    }
}
