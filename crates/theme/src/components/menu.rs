//! Menus: the popups of a menu bar, a context menu or a menu button. Their rows are list box rows
//! (`listbox.*`: height, padding, icon room, text, marks and separators); this family holds what a
//! menu adds: its least width, the gap before a row's shortcut or submenu arrow, the scroll strips
//! of a menu taller than the window, and the active row's colours. The active row (under the
//! pointer, or moved to with the keys) is filled with the accent, as in PhotoCraft Pro and macOS:
//! `accent.strong`, the accent fill that carries text. The menu
//! keeps the keyboard focus, so that fill is the visible focus.
//!
//! The active text (with its mark and shortcut) is text on the active fill, and the shortcuts are
//! text on the popup surface (4.5:1, 7:1 with high contrast). Each pair is measured at rest.

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::tokens::{ColorToken, TokenRef};
use crate::values;

/// Menu tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct MenuTokens {
    /// The narrowest a menu is; it widens to its rows: `menu.min_width`.
    pub min_width: f32,
    /// Least room between a row's label and its shortcut or submenu arrow: `menu.shortcut_gap`.
    pub shortcut_gap: f32,
    /// Height of the ▲ / ▼ strip at either end of a menu that scrolls: `menu.scroll_strip`.
    pub scroll_strip: f32,
    /// Behind the active row: `accent.strong`.
    pub active_fill: Rgba,
    /// The active row's text, mark, shortcut and arrow: `text.on_strong`.
    pub active_text: Rgba,
    /// A row's shortcut: `text.muted`.
    pub shortcut: Rgba,
}

/// An overridable menu field. Paths: `active.fill`, `active.text`, `shortcut`, e.g.
/// `menu.active.fill`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MenuField {
    /// `menu.active.fill`
    ActiveFill,
    /// `menu.active.text`
    ActiveText,
    /// `menu.shortcut`
    Shortcut,
}

impl Family for MenuTokens {
    type Field = MenuField;
    const FIELDS: &'static [MenuField] = &[MenuField::ActiveFill, MenuField::ActiveText, MenuField::Shortcut];

    /// The Compact sizes (PhotoCraft's), scaled by the control height of the size and rounded to
    /// whole points.
    fn defaults(s: &Semantic<'_>) -> MenuTokens {
        let k = s.size.control.md / values::MENU_BASE_CONTROL;
        let scale = |w: f32| (w * k).round();
        let c = s.color;
        MenuTokens {
            min_width: scale(values::MENU_MIN_WIDTH),
            shortcut_gap: scale(values::MENU_SHORTCUT_GAP),
            scroll_strip: scale(values::MENU_SCROLL_STRIP),
            active_fill: c.accent.strong,
            active_text: c.text.on_strong,
            shortcut: c.text.muted,
        }
    }

    fn path(f: MenuField) -> String {
        match f {
            MenuField::ActiveFill => "active.fill",
            MenuField::ActiveText => "active.text",
            MenuField::Shortcut => "shortcut",
        }
        .to_owned()
    }

    fn from_path(path: &str) -> Option<MenuField> {
        Self::FIELDS.iter().copied().find(|&f| Self::path(f) == path)
    }

    fn kind(_: MenuField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: MenuField) -> OverrideValue {
        OverrideValue::Color(match f {
            MenuField::ActiveFill => self.active_fill,
            MenuField::ActiveText => self.active_text,
            MenuField::Shortcut => self.shortcut,
        })
    }

    fn set(&mut self, f: MenuField, v: OverrideValue) {
        let OverrideValue::Color(c) = v else {
            // Mismatched kinds are rejected before this is called.
            return;
        };
        match f {
            MenuField::ActiveFill => self.active_fill = c,
            MenuField::ActiveText => self.active_text = c,
            MenuField::Shortcut => self.shortcut = c,
        }
    }

    /// At rest: the active text on the active fill, and the shortcuts on the popup surface.
    fn pairs() -> Vec<ContrastPair> {
        let field = ComponentField::Menu;
        vec![
            ContrastPair {
                fg: TokenRef::Component(field(MenuField::ActiveText)),
                bg: TokenRef::Component(field(MenuField::ActiveFill)),
                minimum: values::TEXT_MIN,
            },
            ContrastPair {
                fg: TokenRef::Component(field(MenuField::Shortcut)),
                bg: TokenRef::Color(ColorToken::SurfaceRaised),
                minimum: values::TEXT_MIN,
            },
        ]
    }
}
