//! Square icon-only buttons. Off, an icon button is a ghost button and takes `button.ghost`;
//! this family holds its square size and the colours of a toggled-on icon button: the look of a
//! toggle button that is on (`accent.soft` inside a 1 pt `accent.solid` outline, the glyph in
//! `accent.text`).

use super::{ButtonPart, BySize, ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic, ToggleButtonTokens};
use crate::tokens::{ColorToken, TokenRef};
use crate::values;

/// Icon-button tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct IconButtonTokens {
    /// Side of the square: the control height.
    pub size: BySize<f32>,
    /// Glyph size: the Size's icon less [`values::ICON_BUTTON_GLYPH_LESS`].
    pub icon: BySize<f32>,
    /// Diameter of the badge dot: [`values::ICON_BUTTON_BADGE_RATIO`] of the Size's icon. Not
    /// overridable.
    pub badge: BySize<f32>,
    /// Colours while toggled on (`on_*`) and while toggled on and disabled (`on_disabled_*`), the
    /// look of a toggle button that is on.
    pub selected: ToggleButtonTokens,
}

/// An overridable icon-button field: `icon_button.selected.<part>` or
/// `icon_button.selected_disabled.<part>`, e.g. `icon_button.selected.fill`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IconButtonField {
    /// A colour of an icon button that is toggled on.
    Selected(ButtonPart),
    /// A colour of an icon button that is toggled on and disabled.
    SelectedDisabled(ButtonPart),
}

const FIELDS: [IconButtonField; 2 * ButtonPart::ALL.len()] = {
    let mut out = [IconButtonField::Selected(ButtonPart::Fill); 2 * ButtonPart::ALL.len()];
    let mut i = 0;
    while i < ButtonPart::ALL.len() {
        out[i] = IconButtonField::Selected(ButtonPart::ALL[i]);
        out[ButtonPart::ALL.len() + i] = IconButtonField::SelectedDisabled(ButtonPart::ALL[i]);
        i += 1;
    }
    out
};

impl Family for IconButtonTokens {
    type Field = IconButtonField;
    const FIELDS: &'static [IconButtonField] = &FIELDS;

    fn defaults(s: &Semantic<'_>) -> IconButtonTokens {
        let size = s.size;
        let icon = BySize { sm: size.icon.sm - values::ICON_BUTTON_GLYPH_LESS.0, md: size.icon.md - values::ICON_BUTTON_GLYPH_LESS.1 };
        IconButtonTokens {
            size: BySize { sm: size.control.sm, md: size.control.md },
            icon,
            badge: BySize { sm: size.icon.sm * values::ICON_BUTTON_BADGE_RATIO, md: size.icon.md * values::ICON_BUTTON_BADGE_RATIO },
            selected: ToggleButtonTokens::accent_look(s),
        }
    }

    fn path(f: IconButtonField) -> String {
        match f {
            IconButtonField::Selected(part) => format!("selected.{}", part.name()),
            IconButtonField::SelectedDisabled(part) => format!("selected_disabled.{}", part.name()),
        }
    }

    fn from_path(path: &str) -> Option<IconButtonField> {
        let part = |name: &str| ButtonPart::ALL.into_iter().find(|p| p.name() == name);
        match path.split_once('.')? {
            ("selected", name) => part(name).map(IconButtonField::Selected),
            ("selected_disabled", name) => part(name).map(IconButtonField::SelectedDisabled),
            _ => None,
        }
    }

    fn kind(_: IconButtonField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: IconButtonField) -> OverrideValue {
        OverrideValue::Color(match f {
            IconButtonField::Selected(part) => self.selected.on(part),
            IconButtonField::SelectedDisabled(part) => self.selected.on_disabled(part),
        })
    }

    fn set(&mut self, f: IconButtonField, v: OverrideValue) {
        // Mismatched kinds are rejected before this is called.
        if let OverrideValue::Color(c) = v {
            match f {
                IconButtonField::Selected(part) => *self.selected.on_mut(part) = c,
                IconButtonField::SelectedDisabled(part) => *self.selected.on_disabled_mut(part) = c,
            }
        }
    }

    /// The fill, or failing that the outline, shows on the panel and on a popup, and the glyph
    /// reads on the fill. The fill comes before its glyph: high contrast moves the fill first.
    fn pairs() -> Vec<ContrastPair> {
        let field = |part| TokenRef::Component(ComponentField::IconButton(IconButtonField::Selected(part)));
        let fill = [ColorToken::SurfacePanel, ColorToken::SurfaceRaised].map(|bg| ContrastPair {
            fg: field(ButtonPart::Fill),
            bg: TokenRef::Color(bg),
            minimum: values::STATE_CUE_MIN,
            alternate: Some(field(ButtonPart::Border)),
        });
        let text = ContrastPair { fg: field(ButtonPart::Text), bg: field(ButtonPart::Fill), minimum: values::TEXT_MIN, alternate: None };
        fill.into_iter().chain([text]).collect()
    }
}
