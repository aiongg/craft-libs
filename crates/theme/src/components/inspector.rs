//! Inspectors: the body of a Properties panel. The family holds the target header's row height and
//! icon, the body padding, and the colours of the header's name and kind text.

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::tokens::{ColorToken, TokenRef};
use crate::values;

/// Inspector tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct InspectorTokens {
    /// Height of the target header row: `inspector.header_height`.
    pub header_height: f32,
    /// Side of the kind icon in the header: `inspector.icon`.
    pub icon: f32,
    /// Padding round the body: `inspector.pad` (`space.pad.panel`).
    pub pad: f32,
    /// Least width left to the name before the kind text is dropped: `inspector.kind_min_name`.
    pub kind_min_name: f32,
    /// The header's name: `text.normal`.
    pub title: Rgba,
    /// The header's kind text: `text.faint`.
    pub kind: Rgba,
}

/// An overridable inspector field. Paths: `title`, `kind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum InspectorField {
    /// `inspector.title`
    Title,
    /// `inspector.kind`
    Kind,
}

impl Family for InspectorTokens {
    type Field = InspectorField;
    const FIELDS: &'static [InspectorField] = &[InspectorField::Title, InspectorField::Kind];

    /// PhotoCraft's properties header (24 high) at Compact; the row and icon follow the Size.
    fn defaults(s: &Semantic<'_>) -> InspectorTokens {
        InspectorTokens {
            header_height: s.set.inspector_header_height,
            icon: s.set.inspector_icon,
            pad: s.space.pad.panel,
            kind_min_name: values::INSPECTOR_KIND_MIN_NAME,
            title: s.color.text.normal,
            kind: s.color.text.faint,
        }
    }

    fn path(f: InspectorField) -> String {
        match f {
            InspectorField::Title => "title",
            InspectorField::Kind => "kind",
        }
        .to_owned()
    }

    fn from_path(path: &str) -> Option<InspectorField> {
        Some(match path {
            "title" => InspectorField::Title,
            "kind" => InspectorField::Kind,
            _ => return None,
        })
    }

    fn kind(_: InspectorField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: InspectorField) -> OverrideValue {
        match f {
            InspectorField::Title => OverrideValue::Color(self.title),
            InspectorField::Kind => OverrideValue::Color(self.kind),
        }
    }

    fn set(&mut self, f: InspectorField, v: OverrideValue) {
        if let OverrideValue::Color(c) = v {
            match f {
                InspectorField::Title => self.title = c,
                InspectorField::Kind => self.kind = c,
            }
        }
    }

    /// The name and the kind are text on the panel the host's dock draws.
    fn pairs() -> Vec<ContrastPair> {
        [InspectorField::Title, InspectorField::Kind]
            .map(|f| ContrastPair {
                fg: TokenRef::Component(ComponentField::Inspector(f)),
                bg: TokenRef::Color(ColorToken::SurfacePanel),
                minimum: values::TEXT_MIN,
                alternate: None,
            })
            .to_vec()
    }
}
