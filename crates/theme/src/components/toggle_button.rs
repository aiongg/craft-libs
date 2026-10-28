//! Toggle buttons. Off, a toggle button is a secondary button and takes `button.secondary`; on,
//! it holds `button.secondary.*.pressed`. This family holds the colours of a toggle button that
//! is on and disabled: `button.secondary.*.disabled` would make it look off.

use super::{ButtonPart, ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;

/// Toggle-button tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ToggleButtonTokens {
    /// Fill while on and disabled: `state.selected`, the fill of a toggle button that is on.
    pub on_disabled_fill: Rgba,
    /// Outline while on and disabled: `line.divider`, as a disabled secondary button's.
    pub on_disabled_border: Rgba,
    /// Label and icon while on and disabled: `text.disabled`.
    pub on_disabled_text: Rgba,
}

impl ToggleButtonTokens {
    /// The colour of `part` while on and disabled.
    pub fn on_disabled(&self, part: ButtonPart) -> Rgba {
        match part {
            ButtonPart::Fill => self.on_disabled_fill,
            ButtonPart::Border => self.on_disabled_border,
            ButtonPart::Text => self.on_disabled_text,
        }
    }

    fn on_disabled_mut(&mut self, part: ButtonPart) -> &mut Rgba {
        match part {
            ButtonPart::Fill => &mut self.on_disabled_fill,
            ButtonPart::Border => &mut self.on_disabled_border,
            ButtonPart::Text => &mut self.on_disabled_text,
        }
    }
}

/// An overridable toggle-button field: `toggle_button.on_disabled.<part>`, e.g.
/// `toggle_button.on_disabled.fill`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ToggleButtonField {
    /// A colour of a toggle button that is on and disabled.
    OnDisabled(ButtonPart),
}

const FIELDS: [ToggleButtonField; ButtonPart::ALL.len()] = {
    let mut out = [ToggleButtonField::OnDisabled(ButtonPart::Fill); ButtonPart::ALL.len()];
    let mut i = 0;
    while i < ButtonPart::ALL.len() {
        out[i] = ToggleButtonField::OnDisabled(ButtonPart::ALL[i]);
        i += 1;
    }
    out
};

impl Family for ToggleButtonTokens {
    type Field = ToggleButtonField;
    const FIELDS: &'static [ToggleButtonField] = &FIELDS;

    fn defaults(s: &Semantic<'_>) -> ToggleButtonTokens {
        let c = s.color;
        ToggleButtonTokens { on_disabled_fill: c.state.selected, on_disabled_border: c.line.divider, on_disabled_text: c.text.disabled }
    }

    fn path(f: ToggleButtonField) -> String {
        let ToggleButtonField::OnDisabled(part) = f;
        format!("on_disabled.{}", part.name())
    }

    fn from_path(path: &str) -> Option<ToggleButtonField> {
        let part = path.strip_prefix("on_disabled.")?;
        ButtonPart::ALL.into_iter().find(|p| p.name() == part).map(ToggleButtonField::OnDisabled)
    }

    fn kind(_: ToggleButtonField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: ToggleButtonField) -> OverrideValue {
        let ToggleButtonField::OnDisabled(part) = f;
        OverrideValue::Color(self.on_disabled(part))
    }

    fn set(&mut self, f: ToggleButtonField, v: OverrideValue) {
        let ToggleButtonField::OnDisabled(part) = f;
        // Mismatched kinds are rejected before this is called.
        if let OverrideValue::Color(c) = v {
            *self.on_disabled_mut(part) = c;
        }
    }

    /// None: disabled text has no contrast minimum (WCAG 1.4.3 exempts inactive controls).
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
