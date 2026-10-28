//! A menu row ([`MenuItem`](super::MenuItem)) as drawn: the list box row of a select's popup
//! (height, padding, text) with a mark column at the reading start (a check, or a dot for a
//! radio item), an icon column, the label with its mnemonic underlined, and at the reading end the
//! shortcut in [`Kbd`]'s text form or the submenu arrow. The active row has the accent fill and
//! draws everything in the active text colour.

use std::sync::Arc;

use craft_theme::{ControlSize, TypeRole};
use egui::accesskit::{HasPopup, Role, Toggled};
use egui::emath::GuiRounding as _;
use egui::text::LayoutJob;
use egui::{Color32, Galley, Rect, Response, Stroke, Ui, pos2, vec2};

use super::item::ItemKind;
use super::mnemonic;
use crate::controls::{Kbd, ListboxStyle, icon_size};
use crate::direction;
use crate::icons::{self, Icon};
use crate::theme::{ToEgui, UiTokens, role_format};

/// A radio item's dot: its radius as a part of the icon size, and the least radius (craft-menu's
/// ● at the list box's icon size).
const RADIO_DOT: f32 = 1.0 / 6.0;
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
    /// `menu.active.fill` and `menu.active.text`: the active row.
    pub active_fill: Color32,
    pub active_text: Color32,
    /// `menu.shortcut`: a shortcut's colour.
    pub shortcut: Color32,
    /// `kbd.label`: a shortcut's type role.
    pub shortcut_label: TypeRole,
}

impl MenuStyle {
    pub(crate) fn new(tokens: &UiTokens, size: ControlSize) -> Self {
        let m = &tokens.component.menu;
        Self {
            rows: crate::controls::Select::style(tokens, size).list,
            min_width: m.min_width,
            shortcut_gap: m.shortcut_gap,
            scroll_strip: m.scroll_strip,
            active_fill: m.active_fill.to_egui(),
            active_text: m.active_text.to_egui(),
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
}

/// How a row is drawn this pass.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RowState {
    pub enabled: bool,
    /// Highlighted (pointer or keyboard), or its submenu is open.
    pub active: bool,
}

/// Draws rows with `style` at `size`.
pub(crate) struct RowPainter<'s> {
    pub style: &'s MenuStyle,
    pub tokens: &'s UiTokens,
    pub size: ControlSize,
}

impl RowPainter<'_> {
    fn label_galley(&self, ui: &Ui, row: &RowContent<'_>, color: Color32) -> Arc<Galley> {
        let format = role_format(self.tokens, self.style.rows.label, color);
        let mut job = LayoutJob::default();
        job.wrap.max_width = f32::INFINITY;
        mnemonic::append_underlined(&mut job, row.label, row.mnemonic, &format);
        ui.painter().layout_job(job)
    }

    fn shortcut_galley(&self, ui: &Ui, kbd: &Kbd, color: Color32) -> Arc<Galley> {
        let text = kbd.text(kbd.os_in(ui.ctx()));
        ui.painter().layout_job(LayoutJob::single_section(text, role_format(self.tokens, self.style.shortcut_label, color)))
    }

    /// From a row's reading start to its label, inside the padding.
    fn label_offset(&self, cols: &Columns) -> f32 {
        let s = &self.style.rows;
        (s.icon + s.gap) * (f32::from(u8::from(cols.marks)) + f32::from(u8::from(cols.icons)))
    }

    /// The width `row` needs in a level with `cols`' columns.
    pub fn width(&self, ui: &Ui, row: &RowContent<'_>, cols: &Columns) -> f32 {
        let s = &self.style.rows;
        let label = self.label_galley(ui, row, Color32::PLACEHOLDER).size().x;
        let trailing = match (row.submenu, row.shortcut) {
            (true, _) => s.icon,
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
        if active {
            ui.painter().rect_filled(rect.round_to_pixels(ppp), s.radius, m.active_fill);
        }
        let (text, mark, shortcut) = match () {
            () if !state.enabled => (s.text_disabled, s.text_disabled, s.text_disabled),
            () if active => (m.active_text, m.active_text, m.active_text),
            () => (s.text, s.mark, m.shortcut),
        };
        let forward = direction::forward(ui);
        let start = direction::start_x(ui, rect) + forward * s.pad_x;
        let end = direction::end_x(ui, rect) - forward * s.pad_x;
        let square = |center_x: f32| Rect::from_center_size(pos2(center_x, rect.center().y), vec2(s.icon, s.icon)).round_to_pixels(ppp);
        let column = |n: f32| start + forward * (n * (s.icon + s.gap) + s.icon / 2.0);
        match row.kind {
            ItemKind::Check(true) => icons::paint(ui, square(column(0.0)), Icon::Check, icon_size(self.size), mark),
            ItemKind::Radio(true) => {
                ui.painter().circle_filled(pos2(column(0.0), rect.center().y), (s.icon * RADIO_DOT).max(RADIO_DOT_MIN), mark);
            }
            _ => {}
        }
        if let Some(icon) = row.icon.filter(|_| cols.icons) {
            icons::paint(ui, square(column(f32::from(u8::from(cols.marks)))), icon, icon_size(self.size), text);
        }
        let galley = self.label_galley(ui, row, text);
        let x = start + forward * self.label_offset(cols);
        let x = if direction::is_rtl(ui) { x - galley.size().x } else { x };
        ui.painter().galley(pos2(x, rect.center().y - galley.size().y / 2.0).round_to_pixels(ppp), galley, text);
        if row.submenu {
            icons::paint(ui, square(end - forward * s.icon / 2.0), Icon::ChevronRight, icon_size(self.size), text);
        } else if let Some(kbd) = row.shortcut {
            let galley = self.shortcut_galley(ui, kbd, shortcut);
            let x = if direction::is_rtl(ui) { end } else { end - galley.size().x };
            ui.painter().galley(pos2(x, rect.center().y - galley.size().y / 2.0).round_to_pixels(ppp), galley, shortcut);
        }
    }

    /// A separator: its line between `separator_pad` above and below.
    pub fn separator(&self, ui: &mut Ui, width: f32) {
        let s = &self.style.rows;
        let (rect, _) = ui.allocate_exact_size(vec2(width, 2.0 * s.separator_pad + 1.0), egui::Sense::hover());
        let x = rect.x_range().shrink(s.pad_x);
        ui.painter().hline(x, rect.center().y.round_to_pixels(ui.pixels_per_point()), Stroke::new(1.0, s.separator));
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
