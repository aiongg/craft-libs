//! A menu row ([`MenuItem`](super::MenuItem)) as drawn: the list box row of a select's popup
//! (height, padding, text) with a mark column at the reading start (a check, or a dot for a
//! radio item), an icon column, the label with its mnemonic underlined, and at the reading end the
//! shortcut in [`Kbd`]'s text form or the submenu arrow. The active row has the accent fill and
//! draws everything in the active text colour; a submenu's row whose submenu is open has the hover
//! fill. Marks and the arrow are as large as the label's type, as PhotoCraft's ✔ and ⏵ glyphs are.

use std::sync::Arc;

use craft_theme::{ControlSize, Srgba, TypeRole};
use egui::accesskit::{HasPopup, Role, Toggled};
use egui::emath::GuiRounding as _;
use egui::text::LayoutJob;
use egui::{Color32, Galley, Pos2, Rect, Response, Shape, Stroke, Ui, pos2, vec2};

use super::item::ItemKind;
use super::mnemonic;
use crate::controls::{Kbd, ListboxStyle, icon_size};
use crate::direction;
use crate::icons::{self, Icon};
use crate::theme::{ToEgui, UiTokens, corner_radius, role_format};

/// A radio item's dot: its radius as a part of the glyph size, and the least radius (craft-menu's
/// ● at the list box's icon size).
const RADIO_DOT: f32 = 1.0 / 4.0;
const RADIO_DOT_MIN: f32 = 2.0;

/// A menu's sizes and colours, from [`Menu::style`](super::Menu::style).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct MenuStyle {
    /// The rows, separators and marks: list box rows (`listbox.*`) at the menu's size.
    pub rows: ListboxStyle,
    /// `menu.min_width`.
    pub min_width: f32,
    /// `menu.shortcut_gap`: least room between a label and its shortcut or arrow.
    pub shortcut_gap: f32,
    /// `menu.scroll_strip`: height of a scroll strip.
    pub scroll_strip: f32,
    /// `menu.arrow_base` and `menu.arrow_depth`: the submenu arrow's base and depth, as fractions
    /// of the label size.
    pub arrow_base: f32,
    pub arrow_depth: f32,
    /// `menu.active.fill` and `menu.active.text`: the active row.
    pub active_fill: Color32,
    pub active_text: Color32,
    /// `menu.open.fill`: a submenu's row while its submenu is open.
    pub open_fill: Color32,
    /// `menu.shortcut`: a shortcut's colour.
    pub shortcut: Color32,
    /// `kbd.label`: a shortcut's type role.
    pub shortcut_label: TypeRole,
}

impl MenuStyle {
    pub(crate) fn new(tokens: &UiTokens, size: ControlSize) -> Self {
        let m = &tokens.component.menu;
        Self {
            rows: ListboxStyle {
                radius: corner_radius(m.radius),
                active_fill: m.active_fill.to_egui(),
                active_text: m.active_text.to_egui(),
                ..crate::controls::Select::style(tokens, size).list
            },
            min_width: m.min_width,
            shortcut_gap: m.shortcut_gap,
            scroll_strip: m.scroll_strip,
            arrow_base: m.arrow_base,
            arrow_depth: m.arrow_depth,
            active_fill: m.active_fill.to_egui(),
            active_text: m.active_text.to_egui(),
            open_fill: m.open_fill.to_egui(),
            shortcut: m.shortcut.to_egui(),
            shortcut_label: tokens.component.kbd.label,
        }
    }
}

/// What a level's rows share: which columns they keep room for, and their width.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Columns {
    /// Some row has a check or radio mark.
    pub marks: bool,
    /// Some row has an icon.
    pub icons: bool,
    /// Every row's width: the widest row's, at least `menu.min_width`.
    pub width: f32,
}

/// One row's content, as measured and drawn.
pub(crate) struct RowContent<'r> {
    pub label: &'r str,
    /// Underlined in the label.
    pub mnemonic: Option<char>,
    pub icon: Option<Icon>,
    pub kind: ItemKind,
    pub shortcut: Option<&'r Kbd>,
    pub submenu: bool,
    /// A user label colour behind the row.
    pub tint: Option<Srgba>,
}

/// How a row is drawn this pass.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RowState {
    pub enabled: bool,
    /// Highlighted (pointer or keyboard).
    pub active: bool,
    /// Its submenu is open.
    pub open: bool,
}

/// The colours of a row or cell with label colour `tint`: the theme's tokens for PhotoCraft's
/// seven hues, else the same derivation for the colour.
pub(crate) fn tint_colors(tokens: &UiTokens, tint: Srgba) -> craft_theme::MenuTint {
    let tint = craft_theme::Rgba::from(tint);
    tokens.component.menu.tint_of(tint).unwrap_or_else(|| {
        craft_theme::MenuTint::derive(tint, tokens.component.popup.fill, &tokens.color, tokens.appearance.contrast == craft_theme::Contrast::High)
    })
}

/// Draws rows with `style` at `size`.
pub(crate) struct RowPainter<'s> {
    pub style: &'s MenuStyle,
    pub tokens: &'s UiTokens,
    pub size: ControlSize,
}

impl RowPainter<'_> {
    /// The colours of a row with label colour `tint`: the theme's tokens for PhotoCraft's seven
    /// hues, else the same derivation for the colour.
    pub fn tint_colors(&self, tint: Srgba) -> craft_theme::MenuTint {
        tint_colors(self.tokens, tint)
    }

    fn label_galley(&self, ui: &Ui, row: &RowContent<'_>, color: Color32) -> Arc<Galley> {
        let format = role_format(self.tokens, self.style.rows.label, color);
        let mut job = LayoutJob::default();
        job.wrap.max_width = f32::INFINITY;
        mnemonic::append_underlined(&mut job, row.label, row.mnemonic, &format);
        ui.painter().layout_job(job)
    }

    fn shortcut_galley(&self, ui: &Ui, kbd: &Kbd, color: Color32) -> Arc<Galley> {
        let text = kbd.text_in(kbd.os_in(ui.ctx()), &crate::localized::Words::of(ui));
        ui.painter().layout_job(LayoutJob::single_section(text, role_format(self.tokens, self.style.shortcut_label, color)))
    }

    /// Side of the check mark, the radio dot and the submenu arrow: the label's type size.
    fn glyph(&self) -> f32 {
        self.tokens.font(self.style.rows.label).size
    }

    /// The width of the mark column, with its gap, when the level has one.
    fn marks_width(&self, cols: &Columns) -> f32 {
        if cols.marks { self.glyph() + self.style.rows.gap } else { 0.0 }
    }

    /// From a row's reading start to its label, inside the padding.
    fn label_offset(&self, cols: &Columns) -> f32 {
        let s = &self.style.rows;
        self.marks_width(cols) + if cols.icons { s.icon + s.gap } else { 0.0 }
    }

    /// Draws a mark or an arrow `icon` centred at `center_x`, as large as the label's type.
    /// The submenu arrow: a filled triangle centred on `center`, pointing to the reading end.
    fn submenu_arrow(&self, ui: &Ui, center: Pos2, color: Color32) {
        let side = self.glyph();
        // Unrotated, the triangle points down; a quarter turn points it right (left in RTL).
        let rect = Rect::from_center_size(center, vec2(side * self.style.arrow_base, side * self.style.arrow_depth));
        let quarter = std::f32::consts::TAU / 4.0;
        let turn = if direction::is_rtl(ui) { quarter } else { -quarter };
        ui.painter().add(Shape::rotated_triangle(rect, turn, color));
    }

    fn glyph_icon(&self, ui: &Ui, center_x: f32, y: f32, icon: Icon, color: Color32) {
        let side = self.glyph();
        let rect = Rect::from_center_size(pos2(center_x, y), vec2(side, side));
        icons::paint_in(ui, rect, icon, icons::active(ui.ctx()), side, icon_size(self.size).stroke(), color);
    }

    /// The width `row` needs in a level with `cols`' columns.
    pub fn width(&self, ui: &Ui, row: &RowContent<'_>, cols: &Columns) -> f32 {
        let s = &self.style.rows;
        let label = self.label_galley(ui, row, Color32::PLACEHOLDER).size().x;
        let trailing = match (row.submenu, row.shortcut) {
            (true, _) => self.glyph(),
            (false, Some(kbd)) => self.shortcut_galley(ui, kbd, Color32::PLACEHOLDER).size().x,
            (false, None) => 0.0,
        };
        let trailing = if trailing > 0.0 { self.style.shortcut_gap + trailing } else { 0.0 };
        2.0 * s.pad_x + self.label_offset(cols) + label + trailing
    }

    /// Paints `row` over `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect, row: &RowContent<'_>, cols: &Columns, state: RowState) {
        if !ui.is_rect_visible(rect) {
            return;
        }
        let (s, m) = (&self.style.rows, self.style);
        let ppp = ui.pixels_per_point();
        let active = state.active && state.enabled;
        let open = state.open && !state.active && state.enabled;
        if active {
            ui.painter().rect_filled(rect.round_to_pixels(ppp), s.radius, m.active_fill);
        } else if open {
            ui.painter().rect_filled(rect.round_to_pixels(ppp), s.radius, m.open_fill);
        }
        let tint = row.tint.filter(|_| state.enabled && !active && !open).map(|t| self.tint_colors(t));
        if let Some(tint) = tint {
            ui.painter().rect_filled(rect.round_to_pixels(ppp), s.radius, tint.fill.to_egui());
        }
        let (text, mark, shortcut) = match tint {
            _ if !state.enabled => (s.text_disabled, s.text_disabled, s.text_disabled),
            _ if active => (m.active_text, m.active_text, m.active_text),
            Some(tint) => (tint.text.to_egui(), tint.text.to_egui(), tint.shortcut.to_egui()),
            None => (s.text, s.mark, m.shortcut),
        };
        let forward = direction::forward(ui);
        let start = direction::start_x(ui, rect) + forward * s.pad_x;
        let end = direction::end_x(ui, rect) - forward * s.pad_x;
        let square = |center_x: f32| Rect::from_center_size(pos2(center_x, rect.center().y), vec2(s.icon, s.icon)).round_to_pixels(ppp);
        let mark_center = start + forward * self.glyph() / 2.0;
        match row.kind {
            ItemKind::Check(true) => self.glyph_icon(ui, mark_center, rect.center().y, Icon::Check, mark),
            ItemKind::Radio(true) => {
                ui.painter().circle_filled(pos2(mark_center, rect.center().y), (self.glyph() * RADIO_DOT).max(RADIO_DOT_MIN), mark);
            }
            _ => {}
        }
        if let Some(icon) = row.icon.filter(|_| cols.icons) {
            let icon_center = start + forward * (self.marks_width(cols) + s.icon / 2.0);
            icons::paint(ui, square(icon_center), icon, icon_size(self.size), text);
        }
        let galley = self.label_galley(ui, row, text);
        let x = start + forward * self.label_offset(cols);
        let x = if direction::is_rtl(ui) { x - galley.size().x } else { x };
        ui.painter().galley(pos2(x, rect.center().y - galley.size().y / 2.0).round_to_pixels(ppp), galley, text);
        if row.submenu {
            self.submenu_arrow(ui, pos2(end - forward * self.glyph() / 2.0, rect.center().y), text);
        } else if let Some(kbd) = row.shortcut {
            let galley = self.shortcut_galley(ui, kbd, shortcut);
            let x = if direction::is_rtl(ui) { end } else { end - galley.size().x };
            ui.painter().galley(pos2(x, rect.center().y - galley.size().y / 2.0).round_to_pixels(ppp), galley, shortcut);
        }
    }

    /// The width of `kbd` as a row shows it at its end.
    pub fn shortcut_width(&self, ui: &Ui, kbd: &Kbd) -> f32 {
        self.shortcut_galley(ui, kbd, Color32::PLACEHOLDER).size().x
    }

    /// Paints `text` in the shortcut's role and colour at the reading end of `rect`, ending
    /// `trailing` points before the row's padding and cut with an ellipsis to `room` points: a
    /// command's path before its shortcut in a palette row.
    pub fn detail(&self, ui: &Ui, rect: Rect, text: &str, trailing: f32, room: f32, state: RowState) {
        if text.is_empty() || room <= 0.0 || !ui.is_rect_visible(rect) {
            return;
        }
        let color = match (state.enabled, state.active) {
            (false, _) => self.style.rows.text_disabled,
            (true, true) => self.style.active_text,
            (true, false) => self.style.shortcut,
        };
        let mut job = LayoutJob::single_section(text.to_owned(), role_format(self.tokens, self.style.shortcut_label, color));
        job.wrap.max_width = room;
        job.wrap.max_rows = 1;
        let galley = ui.painter().layout_job(job);
        let forward = direction::forward(ui);
        let end = direction::end_x(ui, rect) - forward * (self.style.rows.pad_x + trailing);
        let x = if direction::is_rtl(ui) { end } else { end - galley.size().x };
        ui.painter().galley(pos2(x, rect.center().y - galley.size().y / 2.0).round_to_pixels(ui.pixels_per_point()), galley, color);
    }

    /// A separator: its line between `separator_pad` above and below.
    pub fn separator(&self, ui: &mut Ui, width: f32) {
        let s = &self.style.rows;
        let (rect, _) = ui.allocate_exact_size(vec2(width, 2.0 * s.separator_pad + 1.0), egui::Sense::hover());
        ui.painter().hline(rect.x_range(), rect.center().y.round_to_pixels(ui.pixels_per_point()), Stroke::new(1.0, s.separator));
    }
}

/// Gives a row its AccessKit role, its mnemonic, its checked state, its shortcut, and for a
/// submenu row, its popup and whether it is open. Its name (the label without a mark) and its
/// enabled state come from [`Interactive`](crate::primitives::Interactive).
pub(crate) fn access(r: &Response, row: &RowContent<'_>, open: bool) {
    let os = row.shortcut.map(|kbd| kbd.os_in(&r.ctx));
    r.ctx.accesskit_node_builder(r.id, |n| {
        if let Some(m) = row.mnemonic {
            n.set_access_key(m.to_string());
        }
        match row.kind.checked() {
            Some(true) => n.set_toggled(Toggled::True),
            Some(false) => n.set_toggled(Toggled::False),
            None => n.clear_toggled(),
        }
        if let (Some(kbd), Some(os)) = (row.shortcut, os) {
            n.set_keyboard_shortcut(kbd.spoken(os));
        }
        if row.submenu {
            n.set_has_popup(HasPopup::Menu);
            n.set_expanded(open);
        }
    });
}

/// The AccessKit role of a row of `kind`.
pub(crate) fn role(kind: ItemKind) -> Role {
    match kind {
        ItemKind::Plain => Role::MenuItem,
        ItemKind::Check(_) => Role::MenuItemCheckBox,
        ItemKind::Radio(_) => Role::MenuItemRadio,
    }
}
