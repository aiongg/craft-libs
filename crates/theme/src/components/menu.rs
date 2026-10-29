//! Menus: the popups of a menu bar, a context menu or a menu button. Their rows are list box rows
//! (`listbox.*`: height, padding, icon room, text, marks and separators); this family holds what a
//! menu adds: its least width, the gap before a row's shortcut or submenu arrow, the scroll strips
//! of a menu taller than the window, and the active row's colours. The active row (under the
//! pointer, or moved to with the keys) is filled with the accent, as in PhotoCraft's menus and
//! macOS: `accent.solid`. The menu keeps the keyboard focus, so that fill is the visible focus. A
//! submenu's row stays `state.hover` grey while its submenu is open (PhotoCraft's `widgets.open`).
//!
//! The active text (with its mark and shortcut) is text on the active fill, the open row's text
//! is text on the open fill, and the shortcuts are text on the popup card (4.5:1, 7:1 with high
//! contrast). Each pair is measured at rest.
//!
//! A row can carry a user label colour. PhotoCraft's seven ([`MenuHue`]) have tokens: a fill (the
//! hue over `popup.fill`) and the row's text and shortcut on it, derived per theme, mode and
//! contrast by [`MenuTint::derive`]. Other colours get the same derivation at paint time.

use super::{ComponentField, ContrastPair, Family, FieldKind, ListboxField, OverrideValue, PopupField, Semantic};
use crate::color::Rgba;
use crate::contrast::{contrast_ratio, ensure_contrast};
use crate::derive::over;
use crate::tokens::{ColorTokens, TokenRef};
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
    /// Corner radius of a row's fill: square (PhotoCraft's spectrum menu, in every theme):
    /// `menu.radius`.
    pub radius: f32,
    /// Width of a submenu arrow's base, as a fraction of the row's label size: `menu.arrow_base`.
    pub arrow_base: f32,
    /// A submenu arrow's depth from base to tip, as a fraction of the row's label size:
    /// `menu.arrow_depth`.
    pub arrow_depth: f32,
    /// Behind the active row: `accent.solid`.
    pub active_fill: Rgba,
    /// The active row's text, mark, shortcut and arrow: `text.on_accent`.
    pub active_text: Rgba,
    /// Behind a submenu's row while its submenu is open: `state.hover`.
    pub open_fill: Rgba,
    /// A row's shortcut: `text.muted`.
    pub shortcut: Rgba,
    /// The row colours for each of PhotoCraft's label hues, in [`MenuHue::ALL`] order:
    /// `menu.tint.<hue>.*`.
    pub tints: [MenuTint; MenuHue::ALL.len()],
}

/// PhotoCraft's label colours (`menu_tint`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MenuHue {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Violet,
    Gray,
}

impl MenuHue {
    /// Every hue, in path order.
    pub const ALL: [MenuHue; 7] = [MenuHue::Red, MenuHue::Orange, MenuHue::Yellow, MenuHue::Green, MenuHue::Blue, MenuHue::Violet, MenuHue::Gray];

    /// The path segment.
    pub const fn name(self) -> &'static str {
        match self {
            MenuHue::Red => "red",
            MenuHue::Orange => "orange",
            MenuHue::Yellow => "yellow",
            MenuHue::Green => "green",
            MenuHue::Blue => "blue",
            MenuHue::Violet => "violet",
            MenuHue::Gray => "gray",
        }
    }

    /// The label colour a row's tint is derived from.
    pub const fn color(self) -> Rgba {
        match self {
            MenuHue::Red => Rgba::rgb(190, 60, 60),
            MenuHue::Orange => Rgba::rgb(200, 120, 40),
            MenuHue::Yellow => Rgba::rgb(190, 170, 40),
            MenuHue::Green => Rgba::rgb(60, 150, 70),
            MenuHue::Blue => Rgba::rgb(50, 110, 200),
            MenuHue::Violet => Rgba::rgb(130, 80, 190),
            MenuHue::Gray => Rgba::rgb(120, 120, 120),
        }
    }

    /// The hue whose [`color`](Self::color) is `color`'s red, green and blue.
    pub fn of(color: Rgba) -> Option<MenuHue> {
        Self::ALL.into_iter().find(|h| h.color().0[..3] == color.0[..3])
    }

    fn from_name(name: &str) -> Option<MenuHue> {
        Self::ALL.into_iter().find(|h| h.name() == name)
    }
}

/// The colours of a row with a label colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuTint {
    /// Behind the row: the label colour over `surface.raised`, opaque.
    pub fill: Rgba,
    /// The label, icon, check or radio mark and submenu arrow.
    pub text: Rgba,
    /// The shortcut.
    pub shortcut: Rgba,
}

impl MenuTint {
    /// The tint of `color` (its alpha scales the strength) for a menu on the popup card `base`
    /// with the text colours of `c`. It starts at [`values::MENU_TINT_ALPHA`] of the colour; `text.normal` and `text.muted`
    /// then move in OKLCH lightness until they reach the text minimum (4.5:1, 7:1 with `high`)
    /// on the fill. When they can't, the alpha drops by 0.01 until they do; at 0 the fill is the
    /// card itself. The check and radio marks and the arrow use the label colour, which has
    /// the text minimum, more than the 3:1 (4.5:1) that marks need.
    pub fn derive(color: Rgba, base: Rgba, c: &ColorTokens, high: bool) -> MenuTint {
        let min = if high { values::TEXT_MIN.1 } else { values::TEXT_MIN.0 };
        let start = f64::from(values::MENU_TINT_ALPHA) * f64::from(color.a()) / 255.0;
        let steps = (values::MENU_TINT_ALPHA * 100.0).round() as u32;
        let mut tint = MenuTint { fill: base, text: c.text.normal, shortcut: c.text.muted };
        for k in (0..=steps).rev() {
            let alpha = start * f64::from(k) / f64::from(steps);
            let fill = over(Rgba([color.0[0], color.0[1], color.0[2], (alpha * 255.0).round() as u8]), base);
            tint = MenuTint { fill, text: ensure_contrast(c.text.normal, fill, min), shortcut: ensure_contrast(c.text.muted, fill, min) };
            if contrast_ratio(tint.text, fill) >= min && contrast_ratio(tint.shortcut, fill) >= min {
                break;
            }
        }
        tint
    }
}

/// One colour of a [`MenuTint`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MenuTintPart {
    /// `fill`
    Fill,
    /// `text`
    Text,
    /// `shortcut`
    Shortcut,
}

impl MenuTintPart {
    /// Every part, in path order.
    pub const ALL: [MenuTintPart; 3] = [MenuTintPart::Fill, MenuTintPart::Text, MenuTintPart::Shortcut];

    /// The path segment.
    pub const fn name(self) -> &'static str {
        match self {
            MenuTintPart::Fill => "fill",
            MenuTintPart::Text => "text",
            MenuTintPart::Shortcut => "shortcut",
        }
    }
}

/// An overridable menu field. Paths: `radius`, `active.fill`, `active.text`, `open.fill`, `shortcut` and
/// `tint.<hue>.<fill|text|shortcut>`, e.g. `menu.active.fill`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MenuField {
    /// `menu.active.fill`
    ActiveFill,
    /// `menu.active.text`
    ActiveText,
    /// `menu.open.fill`
    OpenFill,
    /// `menu.shortcut`
    Shortcut,
    /// `menu.radius`
    Radius,
    /// `menu.tint.<hue>.<part>`
    Tint(MenuHue, MenuTintPart),
}

const FIELD_COUNT: usize = 5 + MenuHue::ALL.len() * MenuTintPart::ALL.len();

const FIELDS: [MenuField; FIELD_COUNT] = {
    let mut out = [MenuField::ActiveFill; FIELD_COUNT];
    out[1] = MenuField::ActiveText;
    out[2] = MenuField::OpenFill;
    out[3] = MenuField::Shortcut;
    out[4] = MenuField::Radius;
    let mut i = 5;
    while i < FIELD_COUNT {
        let n = i - 5;
        out[i] = MenuField::Tint(MenuHue::ALL[n / MenuTintPart::ALL.len()], MenuTintPart::ALL[n % MenuTintPart::ALL.len()]);
        i += 1;
    }
    out
};

impl MenuTokens {
    fn tint(&self, hue: MenuHue, part: MenuTintPart) -> Rgba {
        let tint = &self.tints[hue as usize];
        match part {
            MenuTintPart::Fill => tint.fill,
            MenuTintPart::Text => tint.text,
            MenuTintPart::Shortcut => tint.shortcut,
        }
    }

    /// The tokens of `color` when it is one of PhotoCraft's seven label colours.
    pub fn tint_of(&self, color: Rgba) -> Option<MenuTint> {
        MenuHue::of(color).map(|h| self.tints[h as usize])
    }
}

impl Family for MenuTokens {
    type Field = MenuField;
    const FIELDS: &'static [MenuField] = &FIELDS;

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
            radius: 0.0,
            arrow_base: values::MENU_ARROW_BASE,
            arrow_depth: values::MENU_ARROW_DEPTH,
            active_fill: c.accent.solid,
            active_text: c.text.on_accent,
            open_fill: c.state.hover,
            shortcut: c.text.muted,
            tints: MenuHue::ALL.map(|h| MenuTint::derive(h.color(), c.surface.panel, c, s.high)),
        }
    }

    fn path(f: MenuField) -> String {
        match f {
            MenuField::ActiveFill => "active.fill",
            MenuField::ActiveText => "active.text",
            MenuField::OpenFill => "open.fill",
            MenuField::Shortcut => "shortcut",
            MenuField::Radius => "radius",
            MenuField::Tint(hue, part) => return format!("tint.{}.{}", hue.name(), part.name()),
        }
        .to_owned()
    }

    fn from_path(path: &str) -> Option<MenuField> {
        if let Some(rest) = path.strip_prefix("tint.") {
            let (hue, part) = rest.split_once('.')?;
            return Some(MenuField::Tint(MenuHue::from_name(hue)?, MenuTintPart::ALL.into_iter().find(|p| p.name() == part)?));
        }
        Self::FIELDS.iter().copied().find(|&f| Self::path(f) == path)
    }

    fn kind(f: MenuField) -> FieldKind {
        match f {
            MenuField::Radius => FieldKind::Length,
            _ => FieldKind::Color,
        }
    }

    fn get(&self, f: MenuField) -> OverrideValue {
        OverrideValue::Color(match f {
            MenuField::Radius => return OverrideValue::Length(self.radius),
            MenuField::ActiveFill => self.active_fill,
            MenuField::ActiveText => self.active_text,
            MenuField::OpenFill => self.open_fill,
            MenuField::Shortcut => self.shortcut,
            MenuField::Tint(hue, part) => self.tint(hue, part),
        })
    }

    fn set(&mut self, f: MenuField, v: OverrideValue) {
        if let (MenuField::Radius, OverrideValue::Length(r)) = (f, v) {
            self.radius = r;
            return;
        }
        let OverrideValue::Color(c) = v else {
            // Mismatched kinds are rejected before this is called.
            return;
        };
        match f {
            MenuField::Radius => {}
            MenuField::ActiveFill => self.active_fill = c,
            MenuField::ActiveText => self.active_text = c,
            MenuField::OpenFill => self.open_fill = c,
            MenuField::Shortcut => self.shortcut = c,
            MenuField::Tint(hue, part) => {
                let tint = &mut self.tints[hue as usize];
                match part {
                    MenuTintPart::Fill => tint.fill = c,
                    MenuTintPart::Text => tint.text = c,
                    MenuTintPart::Shortcut => tint.shortcut = c,
                }
            }
        }
    }

    /// At rest: the active text on the active fill, the row text on the open fill, and the
    /// shortcuts on the popup card.
    fn pairs() -> Vec<ContrastPair> {
        let field = ComponentField::Menu;
        let tints = MenuHue::ALL.into_iter().flat_map(|hue| {
            [MenuTintPart::Text, MenuTintPart::Shortcut].map(|part| ContrastPair {
                fg: TokenRef::Component(field(MenuField::Tint(hue, part))),
                bg: TokenRef::Component(field(MenuField::Tint(hue, MenuTintPart::Fill))),
                minimum: values::TEXT_MIN,
                alternate: None,
            })
        });
        let mut pairs = vec![
            // The active row shows against the card; its text is fitted to the fill after it.
            ContrastPair {
                fg: TokenRef::Component(field(MenuField::ActiveFill)),
                bg: TokenRef::Component(ComponentField::Popup(PopupField::Fill)),
                minimum: values::ACTIVE_ROW_MIN,
                alternate: None,
            },
            ContrastPair {
                fg: TokenRef::Component(field(MenuField::ActiveText)),
                bg: TokenRef::Component(field(MenuField::ActiveFill)),
                minimum: values::TEXT_MIN,
                alternate: None,
            },
            ContrastPair {
                fg: TokenRef::Component(ComponentField::Listbox(ListboxField::Text { disabled: false })),
                bg: TokenRef::Component(field(MenuField::OpenFill)),
                minimum: values::TEXT_MIN,
                alternate: None,
            },
            ContrastPair {
                fg: TokenRef::Component(field(MenuField::Shortcut)),
                bg: TokenRef::Component(ComponentField::Popup(PopupField::Fill)),
                minimum: values::TEXT_MIN,
                alternate: None,
            },
        ];
        pairs.extend(tints);
        pairs
    }
}
