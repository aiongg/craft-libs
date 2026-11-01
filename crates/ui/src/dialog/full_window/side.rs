//! [`FullWindowSide`]: a side column of a full-window dialog, with its header, scrolling body
//! and the dialog's footer buttons.

use egui::{IdSalt, Rect, Ui};

use super::FullWindowFx;
use crate::dialog::{FooterButton, FooterButtonsResponse};
use egui::AsIdSalt;

type Leading<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// How wide a [`FullWindowSide`] is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SideWidth {
    /// `full_window.side.width` (300): Liquify, Content-Aware Fill, Filter Gallery, Wide Angle.
    #[default]
    Standard,
    /// `full_window.side.width_wide` (350): Select and Mask's Properties.
    Wide,
}

/// The dialog's footer buttons in a side column: at its bottom (Liquify, Select and Mask,
/// Content-Aware Fill, Wide Angle) or its top (Filter Gallery), in the platform's order, with an
/// optional leading slot (a Reset icon button) at the other end of the row.
#[must_use = "pass the footer to `FullWindowSide::footer`"]
#[expect(dead_code, reason = "skeleton: K15 places the footer")]
pub struct SideFooter<'a> {
    pub(super) buttons: Vec<FooterButton>,
    pub(super) top: bool,
    pub(super) leading: Option<Leading<'a>>,
}

impl<'a> SideFooter<'a> {
    /// The buttons at the bottom of the column, under its scrolling body.
    pub fn bottom(buttons: Vec<FooterButton>) -> Self {
        Self { buttons, top: false, leading: None }
    }

    /// The buttons at the top of the column, over its body (Filter Gallery).
    pub fn top(buttons: Vec<FooterButton>) -> Self {
        Self { buttons, top: true, leading: None }
    }

    /// Draws `add` at the reading start of the footer row, the buttons keeping the trailing end:
    /// Select and Mask's and Content-Aware Fill's Reset.
    pub fn leading(mut self, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.leading = Some(Box::new(add));
        self
    }
}

/// What [`FullWindowSide::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
pub struct FullWindowSideOutput {
    /// The footer row's response, when the column has the footer. The dialog also resolves its
    /// clicks into its [`Outcome`](crate::dialog::Outcome).
    pub footer: Option<FooterButtonsResponse>,
    /// The whole column.
    pub rect: Rect,
}

/// A side column of a [`FullWindowDialog`](super::FullWindowDialog): Liquify's and Select and
/// Mask's Properties, Content-Aware Fill's Preview and settings panels, Filter Gallery's
/// settings.
///
/// ```ignore
/// let side = FullWindowSide::new("sam-props")
///     .width(SideWidth::Wide)
///     .header(&labels.properties)
///     .footer(SideFooter::bottom(vec![FooterButton::new(ButtonRole::Default, ok), FooterButton::new(ButtonRole::Cancel, cancel)])
///         .leading(|ui| reset_button(ui)))
///     .show(ui, fx, |ui| properties(ui, session));
/// ```
///
/// - **Place.** An `egui::Panel` at the end edge of what is left of the dialog's body (the right
///   edge, the left in a right-to-left interface); a second column shown after it stacks inwards
///   beside it. Call it after the options bar and the tool strip, before the canvas.
/// - **Look.** `full_window.side.fill`, a 1 pt `full_window.side.divider` on the inner edge; the
///   optional [`PanelHeader`](crate::composites::PanelHeader) at the top (no menu); the body
///   padded by `full_window.side.pad_x` and `pad_y`.
/// - **Body.** Scrolls vertically when taller than the room; the footer stays in view.
/// - **Footer.** Placed with [`FullWindowFx::buttons`] in the full padded width, at the bottom or
///   top, `full_window.footer.gap` from the body; the leading slot at the reading start. Only one
///   column of a dialog has the footer.
/// - **Keyboard and AccessKit.** A `Group` named by the header (or `fullWindow.side`); Tab goes
///   top to bottom, the footer buttons last (first for the top footer). The column takes no keys
///   of its own.
#[must_use = "show the column with `show`"]
pub struct FullWindowSide<'a> {
    id_salt: IdSalt,
    width: SideWidth,
    header: Option<String>,
    footer: Option<SideFooter<'a>>,
}

impl<'a> FullWindowSide<'a> {
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id_salt: IdSalt::new(id_salt), width: SideWidth::Standard, header: None, footer: None }
    }

    /// Standard (default) or Wide.
    pub fn width(mut self, width: SideWidth) -> Self {
        self.width = width;
        self
    }

    /// A [`PanelHeader`](crate::composites::PanelHeader) at the top with this title (translated),
    /// also the column's accessible name: "Properties", "Preview", "Content-Aware Fill".
    pub fn header(mut self, title: impl Into<String>) -> Self {
        self.header = Some(title.into());
        self
    }

    /// The dialog's footer buttons in this column.
    pub fn footer(mut self, footer: SideFooter<'a>) -> Self {
        self.footer = Some(footer);
        self
    }

    /// Draws the column in `ui` (the dialog's body), its body with `body`, and the footer with
    /// `fx`.
    #[expect(clippy::todo, reason = "skeleton: K15")]
    pub fn show(self, ui: &mut Ui, fx: &mut FullWindowFx, body: impl FnOnce(&mut Ui)) -> FullWindowSideOutput {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:991-992 (the right panel, `RIGHT_W` on
        // `t.dock`), :1070 (the footer row at the bottom, 14 pt in), gallery_ui.rs:756-770 (the
        // footer at the top of the right column, 12 × 10 padding), select_mask_ui.rs:1886 (the
        // footer at the bottom, 10 pt in).
        let _ = (self.id_salt, self.width, self.header, self.footer, ui, fx, body);
        todo!()
    }
}
