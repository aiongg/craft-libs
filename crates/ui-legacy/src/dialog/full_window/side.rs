//! [`FullWindowSide`]: a side column of a full-window dialog, with its header, scrolling body
//! and the dialog's footer buttons.

use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::{AsIdSalt, Frame, IdSalt, Rect, ScrollArea, Stroke, Ui, UiBuilder, pos2};

use super::style::FullWindowStyle;
use super::{FullWindowFx, FullWindowLabels};
use crate::composites::PanelHeader;
use crate::dialog::{FooterButton, FooterButtonsResponse};
use crate::direction;
use crate::localized::SharedLabels as _;
use crate::patterns::panel::ScrollBarLabels;
use crate::theme::{Tokens, TokensOf as _};

type Leading<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// How wide a [`FullWindowSide`] is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SideWidth {
    /// The standard width: Liquify, Content-Aware Fill, Filter Gallery, Wide Angle.
    #[default]
    Standard,
    /// The wide width: Select and Mask's Properties.
    Wide,
    /// The narrow width: a column that carries only the footer buttons (Vanishing
    /// Point).
    Narrow,
}

/// The dialog's footer buttons in a side column: at its bottom (Liquify, Select and Mask,
/// Content-Aware Fill, Wide Angle) or its top (Filter Gallery), in the platform's order, with an
/// optional leading slot (a Reset icon button) at the other end of the row.
#[must_use = "pass the footer to `FullWindowSide::footer`"]
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
/// - **Look.** The panel surface fill, a divider line on the inner edge; the
///   optional [`PanelHeader`](crate::composites::PanelHeader) at the top (no menu); the body
///   padded by `space.pad.panel`.
/// - **Body.** Scrolls vertically when taller than the room; the footer stays in view.
///   [`scrolls(false)`](FullWindowSide::scrolls) leaves the scroll area out for a pane that
///   bounds itself.
/// - **Footer.** Placed with [`FullWindowFx::buttons`] in the full padded width, at the bottom or
///   top, `space.gap.inline` from the body; the leading slot at the reading start, laid out first, and the buttons in the rest of the
///   row (they overflow towards the leading slot when the rest is narrower than they are). Only one
///   column of a dialog has the footer.
/// - **Keyboard and AccessKit.** A `Group` named by [`name`](FullWindowSide::name), else the header
///   (else `fullWindow.side`); Tab goes
///   top to bottom, the footer buttons last (first for the top footer). The column takes no keys
///   of its own.
#[must_use = "show the column with `show`"]
pub struct FullWindowSide<'a> {
    id_salt: IdSalt,
    width: SideWidth,
    header: Option<String>,
    name: Option<String>,
    scrolls: bool,
    footer: Option<SideFooter<'a>>,
}

impl<'a> FullWindowSide<'a> {
    /// A side region keyed by `id_salt` with the Standard width, no header and no footer; the body scrolls.
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id_salt: IdSalt::new(id_salt), width: SideWidth::Standard, header: None, name: None, scrolls: true, footer: None }
    }

    /// Standard (default), Wide or Narrow.
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

    /// The column's accessible name (translated), when the header is not it or there is no header:
    /// two columns of one dialog need two names.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Whether the body scrolls (default `true`). A pane that bounds itself (a preview that fits
    /// its room) passes `false` and gets the padded room as its `max_rect` with no scroll area.
    pub fn scrolls(mut self, scrolls: bool) -> Self {
        self.scrolls = scrolls;
        self
    }

    /// The dialog's footer buttons in this column.
    pub fn footer(mut self, footer: SideFooter<'a>) -> Self {
        self.footer = Some(footer);
        self
    }

    /// Draws the column in `ui` (the dialog's body), its body with `body`, and the footer with
    /// `fx`.
    pub fn show(self, ui: &mut Ui, fx: &mut FullWindowFx, body: impl FnOnce(&mut Ui)) -> FullWindowSideOutput {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:991-992 (the right panel, `RIGHT_W` on
        // `t.dock`), :1070 (the footer row at the bottom, 14 pt in), gallery_ui.rs:756-770 (the
        // footer at the top of the right column, 12 × 10 padding), select_mask_ui.rs:1886 (the
        // footer at the bottom, 10 pt in).
        let FullWindowSide { id_salt, width, header, name, scrolls, footer } = self;
        let tokens = Tokens::of(ui);
        let style = FullWindowStyle::of(&tokens);
        let button_height = tokens.size.control.md;
        let id = ui.make_persistent_id(id_salt);
        let name = name.or_else(|| header.clone()).unwrap_or_else(|| FullWindowLabels::shared(ui).side.clone());
        let scroll_bar_label = ScrollBarLabels::name(ui);
        let width = match width {
            SideWidth::Standard => style.side_width,
            SideWidth::Wide => style.side_width_wide,
            SideWidth::Narrow => style.side_width_narrow,
        };
        // The column is at the left in a right-to-left interface, so its inner edge is the right one.
        let inner_start = direction::is_rtl(ui);

        let mut footer_response = None;
        let shown = direction::end_panel(ui, id.with("panel"))
            .exact_size(width)
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::NONE.fill(style.side_fill))
            .show(ui, |ui| {
                let group = UiBuilder::new().id_salt("column").accessibility_role(Role::Group).accessibility_label(name.as_str());
                ui.scope_builder(group, |ui| {
                    let room = ui.max_rect();
                    if let Some(title) = &header {
                        PanelHeader::new(id.with("header"), title.as_str()).menu(false).show(ui, |_| {});
                    }
                    let (left, right) = (room.left() + style.side_pad.x, room.right() - style.side_pad.x);
                    let row_at = |y: f32| Rect::from_min_max(pos2(left, y), pos2(right, y + button_height));
                    let mut top = ui.cursor().min.y + style.side_pad.y;
                    let mut bottom = room.bottom() - style.side_pad.y;
                    let mut footer_rect = None;
                    if let Some(footer) = &footer {
                        if footer.top {
                            footer_rect = Some(row_at(top));
                            top += button_height + style.footer_gap;
                        } else {
                            let row = row_at(bottom - button_height);
                            bottom = row.top() - style.footer_gap;
                            footer_rect = Some(row);
                        }
                    }
                    // The top footer comes first in the focus order, the bottom one last.
                    let place = |ui: &mut Ui, fx: &mut FullWindowFx, footer: SideFooter<'a>, row: Rect| {
                        let SideFooter { buttons, leading, .. } = footer;
                        // The leading slot is laid out first (AccessKit lists it first); the buttons
                        // get what is left of the row.
                        let mut room = row;
                        if let Some(leading) = leading {
                            let mut lead = ui.new_child(UiBuilder::new().max_rect(row).layout(direction::row(ui)));
                            leading(&mut lead);
                            let used = lead.min_rect();
                            if used.width() > 0.0 {
                                let from = direction::end_x(ui, used) + direction::forward(ui) * style.footer_gap;
                                let to = direction::end_x(ui, row);
                                room = Rect::from_x_y_ranges(from.min(to)..=from.max(to), row.y_range());
                            }
                        }
                        fx.buttons(ui, room, buttons)
                    };
                    let (mut top_footer, mut bottom_footer) = (None, None);
                    if let Some(f) = footer {
                        if f.top { top_footer = Some(f) } else { bottom_footer = Some(f) }
                    }
                    if let (Some(f), Some(row)) = (top_footer.take(), footer_rect) {
                        footer_response = Some(place(ui, fx, f, row));
                    }
                    let area = Rect::from_min_max(pos2(left, top), pos2(right, bottom.max(top)));
                    let mut content = ui.new_child(UiBuilder::new().max_rect(area));
                    if scrolls {
                        ScrollArea::vertical()
                            .id_salt(id.with("body"))
                            .auto_shrink([false, false])
                            .vertical_scroll_bar_label(&scroll_bar_label)
                            .show(&mut content, body);
                    } else {
                        body(&mut content);
                    }
                    if let (Some(f), Some(row)) = (bottom_footer, footer_rect) {
                        footer_response = Some(place(ui, fx, f, row));
                    }
                });
            });
        let rect = shown.response.rect;
        let x = if inner_start { rect.right() - 0.5 } else { rect.left() + 0.5 }.round_to_pixel_center(ui.pixels_per_point());
        ui.painter().line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], Stroke::new(style.side_divider.width, style.side_divider.color));
        FullWindowSideOutput { footer: footer_response, rect }
    }
}
