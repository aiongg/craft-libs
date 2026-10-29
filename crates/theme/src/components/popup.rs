//! Popups: the card that holds a menu, a select's list or a combo box's suggestions. It is the
//! theme's card (`surface.panel`) with a one-point edge and a shadow below it. The shadow's offset
//! and blur are `elevation.popup`; its colour is black at the alpha of the theme's mode, which a
//! theme pins to give its popups a shadow of their own (Classic has none).
//!
//! The fill, the edge and the shadow are design targets with no contrast minimum; what sits on the
//! fill (rows, text, shortcuts) is measured by the families that draw it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;

/// Popup tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PopupTokens {
    /// Behind the contents: `surface.panel`.
    pub fill: Rgba,
    /// The one-point outline: `line.border`.
    pub border: Rgba,
    /// The shadow's colour: black at `elevation.popup`'s alpha.
    pub shadow: Rgba,
}

/// An overridable popup field. Paths: `fill`, `border`, `shadow`, e.g. `popup.shadow`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PopupField {
    /// `popup.fill`
    Fill,
    /// `popup.border`
    Border,
    /// `popup.shadow`
    Shadow,
}

impl Family for PopupTokens {
    type Field = PopupField;
    const FIELDS: &'static [PopupField] = &[PopupField::Fill, PopupField::Border, PopupField::Shadow];

    fn defaults(s: &Semantic<'_>) -> PopupTokens {
        let c = s.color;
        PopupTokens { fill: c.surface.panel, border: c.line.border, shadow: Rgba([0, 0, 0, (s.elevation.popup.alpha * 255.0).round() as u8]) }
    }

    fn path(f: PopupField) -> String {
        match f {
            PopupField::Fill => "fill",
            PopupField::Border => "border",
            PopupField::Shadow => "shadow",
        }
        .to_owned()
    }

    fn from_path(path: &str) -> Option<PopupField> {
        Some(match path {
            "fill" => PopupField::Fill,
            "border" => PopupField::Border,
            "shadow" => PopupField::Shadow,
            _ => return None,
        })
    }

    fn kind(_: PopupField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: PopupField) -> OverrideValue {
        OverrideValue::Color(match f {
            PopupField::Fill => self.fill,
            PopupField::Border => self.border,
            PopupField::Shadow => self.shadow,
        })
    }

    fn set(&mut self, f: PopupField, v: OverrideValue) {
        let OverrideValue::Color(c) = v else {
            // Mismatched kinds are rejected before this is called.
            return;
        };
        match f {
            PopupField::Fill => self.fill = c,
            PopupField::Border => self.border = c,
            PopupField::Shadow => self.shadow = c,
        }
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
