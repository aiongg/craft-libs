//! Group boxes: the outlined, titled group of a dialog or preferences page. The edge colour and
//! radius are overridable; the padding and gap come from the Size preset.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;

/// Group box tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GroupBoxTokens {
    /// The outline, a decorative edge with no contrast minimum: `group_box.edge`, `line.divider`.
    pub edge: Rgba,
    /// Corner radius: `group_box.radius`, `radius.panel`.
    pub radius: f32,
    /// Between the outline and the content: `group_box.padding`, `pad.panel`.
    pub padding: f32,
    /// Between two boxes: `group_box.gap`, `gap.stack`.
    pub gap: f32,
}

/// An overridable group box field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GroupBoxField {
    /// `group_box.edge`
    Edge,
    /// `group_box.radius`
    Radius,
}

impl Family for GroupBoxTokens {
    type Field = GroupBoxField;
    const FIELDS: &'static [GroupBoxField] = &[GroupBoxField::Edge, GroupBoxField::Radius];

    fn defaults(s: &Semantic<'_>) -> GroupBoxTokens {
        GroupBoxTokens { edge: s.color.line.divider, radius: s.shape.radius.panel, padding: s.space.pad.panel, gap: s.space.gap.stack }
    }

    fn path(f: GroupBoxField) -> String {
        match f {
            GroupBoxField::Edge => "edge",
            GroupBoxField::Radius => "radius",
        }
        .to_owned()
    }

    fn from_path(path: &str) -> Option<GroupBoxField> {
        Self::FIELDS.iter().copied().find(|f| Self::path(*f) == path)
    }

    fn kind(f: GroupBoxField) -> FieldKind {
        match f {
            GroupBoxField::Edge => FieldKind::Color,
            GroupBoxField::Radius => FieldKind::Length,
        }
    }

    fn get(&self, f: GroupBoxField) -> OverrideValue {
        match f {
            GroupBoxField::Edge => OverrideValue::Color(self.edge),
            GroupBoxField::Radius => OverrideValue::Length(self.radius),
        }
    }

    fn set(&mut self, f: GroupBoxField, v: OverrideValue) {
        match (f, v) {
            (GroupBoxField::Edge, OverrideValue::Color(c)) => self.edge = c,
            (GroupBoxField::Radius, OverrideValue::Length(r)) => self.radius = r,
            // Mismatched kinds are rejected before this is called.
            _ => {}
        }
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
