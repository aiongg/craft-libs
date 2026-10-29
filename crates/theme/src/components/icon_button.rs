//! Square icon-only buttons. Off, an icon button is a ghost button and takes `button.ghost`;
//! this family holds its square size and the colours of a toggled-on icon button.

use super::{BySize, ComponentField, ContrastPair, Family, FieldKind, ITEM_FIELDS, ItemColors, ItemPart, OverrideValue, Semantic, State};
use crate::tokens::{ColorToken, TokenRef};
use crate::values;

/// Icon-button tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct IconButtonTokens {
    /// Side of the square: the control height.
    pub size: BySize<f32>,
    /// Icon size.
    pub icon: BySize<f32>,
    /// Diameter of the badge dot: [`values::ICON_BUTTON_BADGE_RATIO`] of the icon. Not
    /// overridable.
    pub badge: BySize<f32>,
    /// Colours while toggled on: `text.on_selected` on `state.selected`, the fill moved to reach
    /// the non-text minimum on the panel and on a popup, the text moved to read on it.
    pub selected: ItemColors,
}

/// An overridable icon-button field: `icon_button.selected.<part>.<state>`, e.g.
/// `icon_button.selected.fill.rest`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IconButtonField {
    /// A colour of the toggled-on look.
    Selected {
        /// The part.
        part: ItemPart,
        /// The state.
        state: State,
    },
}

const FIELDS: [IconButtonField; ITEM_FIELDS.len()] = {
    let mut out = [IconButtonField::Selected { part: ItemPart::Fill, state: State::Rest }; ITEM_FIELDS.len()];
    let mut i = 0;
    while i < ITEM_FIELDS.len() {
        out[i] = IconButtonField::Selected { part: ITEM_FIELDS[i].0, state: ITEM_FIELDS[i].1 };
        i += 1;
    }
    out
};

impl Family for IconButtonTokens {
    type Field = IconButtonField;
    const FIELDS: &'static [IconButtonField] = &FIELDS;

    fn defaults(s: &Semantic<'_>) -> IconButtonTokens {
        let size = s.size;
        IconButtonTokens {
            size: BySize { sm: size.control.sm, md: size.control.md },
            icon: BySize { sm: size.icon.sm, md: size.icon.md },
            badge: BySize { sm: size.icon.sm * values::ICON_BUTTON_BADGE_RATIO, md: size.icon.md * values::ICON_BUTTON_BADGE_RATIO },
            selected: ItemColors::selected(s.color, &[s.color.surface.panel, s.color.surface.raised], s.high),
        }
    }

    fn path(f: IconButtonField) -> String {
        let IconButtonField::Selected { part, state } = f;
        format!("selected.{}", super::item_path(part, state))
    }

    fn from_path(path: &str) -> Option<IconButtonField> {
        let (part, state) = super::item_from_path(path.strip_prefix("selected.")?)?;
        Some(IconButtonField::Selected { part, state })
    }

    fn kind(_: IconButtonField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: IconButtonField) -> OverrideValue {
        let IconButtonField::Selected { part, state } = f;
        OverrideValue::Color(self.selected.part(part).get(state))
    }

    fn set(&mut self, f: IconButtonField, v: OverrideValue) {
        let IconButtonField::Selected { part, state } = f;
        // Mismatched kinds are rejected before this is called.
        if let OverrideValue::Color(c) = v {
            *self.selected.get_mut(part, state) = c;
        }
    }

    /// The fill shows on the panel and on a popup, and comes before its text: high contrast moves
    /// the fill first.
    fn pairs() -> Vec<ContrastPair> {
        let field = |part, state| TokenRef::Component(ComponentField::IconButton(IconButtonField::Selected { part, state }));
        let fill = [ColorToken::SurfacePanel, ColorToken::SurfaceRaised].map(|bg| ContrastPair {
            fg: field(ItemPart::Fill, State::Rest),
            bg: TokenRef::Color(bg),
            minimum: values::NON_TEXT_MIN,
            alternate: None,
        });
        let text = [State::Rest, State::Hover, State::Pressed].map(|state| ContrastPair {
            fg: field(ItemPart::Text, state),
            bg: field(ItemPart::Fill, state),
            minimum: values::TEXT_MIN,
            alternate: None,
        });
        fill.into_iter().chain(text).collect()
    }
}
