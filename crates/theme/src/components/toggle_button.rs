//! Toggle buttons. Off, a toggle button is a secondary button and takes `button.secondary`. This
//! family holds the colours of one that is on, in every state but disabled, and of one that is on
//! and disabled: `button.secondary.*.disabled` would make it look off.

use super::{ButtonPart, ComponentField, ContrastPair, Family, FieldKind, Identify, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::tokens::{ColorToken, TokenRef};
use crate::values;

/// Toggle-button tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ToggleButtonTokens {
    /// Fill while on: `accent.soft`, or where that fill does not show on the panel and on a
    /// popup and the outline does not either, the nearest lightness that does.
    pub on_fill: Rgba,
    /// Outline while on: `accent.solid`.
    pub on_border: Rgba,
    /// Label and icon while on: `accent.text`, moved to read on the fill.
    pub on_text: Rgba,
    /// Fill while on and disabled: `accent.soft`, the fill of a toggle button that is on.
    pub on_disabled_fill: Rgba,
    /// Outline while on and disabled: `line.divider`, as a disabled secondary button's.
    pub on_disabled_border: Rgba,
    /// Label and icon while on and disabled: `text.disabled`.
    pub on_disabled_text: Rgba,
}

impl ToggleButtonTokens {
    /// The look of a button that is on, shared with the icon button: `accent.soft` inside a
    /// 1 pt `accent.solid` outline, the label in `accent.text`. The state is identified by its
    /// fill or its outline: one of them reaches the non-text minimum on the panel and on a popup.
    pub(super) fn accent_look(s: &Semantic<'_>) -> ToggleButtonTokens {
        let c = s.color;
        let (on_fill, on_text) = Identify::on(&[c.surface.panel, c.surface.raised], s.high).edge(c.accent.solid).fill(c.accent.soft, c.accent.text);
        ToggleButtonTokens {
            on_fill,
            on_border: c.accent.solid,
            on_text,
            on_disabled_fill: c.accent.soft,
            on_disabled_border: c.line.divider,
            on_disabled_text: c.text.disabled,
        }
    }

    /// The colour of `part` while on.
    pub fn on(&self, part: ButtonPart) -> Rgba {
        match part {
            ButtonPart::Fill => self.on_fill,
            ButtonPart::Border => self.on_border,
            ButtonPart::Text => self.on_text,
        }
    }

    pub(super) fn on_mut(&mut self, part: ButtonPart) -> &mut Rgba {
        match part {
            ButtonPart::Fill => &mut self.on_fill,
            ButtonPart::Border => &mut self.on_border,
            ButtonPart::Text => &mut self.on_text,
        }
    }

    /// The colour of `part` while on and disabled.
    pub fn on_disabled(&self, part: ButtonPart) -> Rgba {
        match part {
            ButtonPart::Fill => self.on_disabled_fill,
            ButtonPart::Border => self.on_disabled_border,
            ButtonPart::Text => self.on_disabled_text,
        }
    }

    pub(super) fn on_disabled_mut(&mut self, part: ButtonPart) -> &mut Rgba {
        match part {
            ButtonPart::Fill => &mut self.on_disabled_fill,
            ButtonPart::Border => &mut self.on_disabled_border,
            ButtonPart::Text => &mut self.on_disabled_text,
        }
    }
}

/// An overridable toggle-button field: `toggle_button.on.<part>` or `toggle_button.on_disabled.<part>`,
/// e.g. `toggle_button.on.fill`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ToggleButtonField {
    /// A colour of a toggle button that is on.
    On(ButtonPart),
    /// A colour of a toggle button that is on and disabled.
    OnDisabled(ButtonPart),
}

const FIELDS: [ToggleButtonField; 2 * ButtonPart::ALL.len()] = {
    let mut out = [ToggleButtonField::On(ButtonPart::Fill); 2 * ButtonPart::ALL.len()];
    let mut i = 0;
    while i < ButtonPart::ALL.len() {
        out[i] = ToggleButtonField::On(ButtonPart::ALL[i]);
        out[ButtonPart::ALL.len() + i] = ToggleButtonField::OnDisabled(ButtonPart::ALL[i]);
        i += 1;
    }
    out
};

impl Family for ToggleButtonTokens {
    type Field = ToggleButtonField;
    const FIELDS: &'static [ToggleButtonField] = &FIELDS;

    fn defaults(s: &Semantic<'_>) -> ToggleButtonTokens {
        ToggleButtonTokens::accent_look(s)
    }

    fn path(f: ToggleButtonField) -> String {
        match f {
            ToggleButtonField::On(part) => format!("on.{}", part.name()),
            ToggleButtonField::OnDisabled(part) => format!("on_disabled.{}", part.name()),
        }
    }

    fn from_path(path: &str) -> Option<ToggleButtonField> {
        let part = |name: &str| ButtonPart::ALL.into_iter().find(|p| p.name() == name);
        match path.split_once('.')? {
            ("on", name) => part(name).map(ToggleButtonField::On),
            ("on_disabled", name) => part(name).map(ToggleButtonField::OnDisabled),
            _ => None,
        }
    }

    fn kind(_: ToggleButtonField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: ToggleButtonField) -> OverrideValue {
        OverrideValue::Color(match f {
            ToggleButtonField::On(part) => self.on(part),
            ToggleButtonField::OnDisabled(part) => self.on_disabled(part),
        })
    }

    fn set(&mut self, f: ToggleButtonField, v: OverrideValue) {
        // Mismatched kinds are rejected before this is called.
        if let OverrideValue::Color(c) = v {
            match f {
                ToggleButtonField::On(part) => *self.on_mut(part) = c,
                ToggleButtonField::OnDisabled(part) => *self.on_disabled_mut(part) = c,
            }
        }
    }

    /// The fill, or failing that the outline, shows on the panel and on a popup, and the text
    /// reads on the fill. The fill comes before its text: high contrast moves the fill first.
    /// Disabled colours have no pair: disabled text has no contrast minimum (WCAG 1.4.3 exempts
    /// inactive controls).
    fn pairs() -> Vec<ContrastPair> {
        let on = |part| TokenRef::Component(ComponentField::ToggleButton(ToggleButtonField::On(part)));
        let mut out = Vec::new();
        out.extend([ColorToken::SurfacePanel, ColorToken::SurfaceRaised].map(|bg| ContrastPair {
            fg: on(ButtonPart::Fill),
            bg: TokenRef::Color(bg),
            minimum: values::STATE_CUE_MIN,
            alternate: Some(on(ButtonPart::Border)),
        }));
        out.push(ContrastPair { fg: on(ButtonPart::Text), bg: on(ButtonPart::Fill), minimum: values::TEXT_MIN, alternate: None });
        out
    }
}
