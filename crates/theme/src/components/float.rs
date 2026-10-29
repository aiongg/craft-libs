//! Floating windows: the frame and title bar of a panel group that floats over the workspace,
//! which a floating `Panel` shares. Corners are square and a shadow lifts the window.
//!
//! The title is text on the title bar; the ▾ and × at rest are the only mark of their buttons and
//! the focus stroke the only cue for keyboard focus, so each is measured as a non-text mark (3:1,
//! 4.5:1): the glyphs and the focus stroke on the title bar.
//! The fill, border and dim are design targets with no minimum.

use super::{ComponentField, ContrastPair, DockField, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::scale::Elevation;
use crate::tokens::TokenRef;
use crate::values;

/// Floating window tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FloatTokens {
    /// Width of the border.
    pub border_width: f32,
    /// Corner radius of the fill, the border and the shadow: square by default.
    pub radius: f32,
    /// The shadow: `elevation.floating`.
    pub shadow: Elevation,
    /// Behind the contents: `surface.panel`.
    pub fill: Rgba,
    /// The outline: `line.border`.
    pub border: Rgba,
    /// Over a window that a modal one blocks: `surface.scrim`.
    pub blocked: Rgba,
    /// Behind the title: `surface.background`, the tab strip's.
    pub title: Rgba,
    /// The title and the ▾ and × under the pointer or with focus: `text.normal`.
    pub title_text: Rgba,
    /// The ▾ and × at rest: `text.muted`.
    pub title_muted: Rgba,
    /// The diagonal resize grip: `accent.solid`.
    pub grip: Rgba,
    /// A keyboard focus stroke on the title bar and tabs: `line.focus`.
    pub focus: Rgba,
}

impl FloatTokens {
    fn color_mut(&mut self, f: FloatField) -> Option<&mut Rgba> {
        Some(match f {
            FloatField::Radius | FloatField::Shadow => return None,
            FloatField::Fill => &mut self.fill,
            FloatField::Border => &mut self.border,
            FloatField::Blocked => &mut self.blocked,
            FloatField::Title => &mut self.title,
            FloatField::TitleText => &mut self.title_text,
            FloatField::TitleMuted => &mut self.title_muted,
            FloatField::Grip => &mut self.grip,
            FloatField::Focus => &mut self.focus,
        })
    }
}

/// An overridable floating window token. Paths: `radius`, `shadow`, `fill`, `border`, `blocked`,
/// `title`, `title.text`, `title.muted`, `grip`, `focus`, e.g. `float.title.text`. `shadow` takes
/// `{"level": 0 to 3}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FloatField {
    /// `float.radius`
    Radius,
    /// `float.shadow`
    Shadow,
    /// `float.fill`
    Fill,
    /// `float.border`
    Border,
    /// `float.blocked`
    Blocked,
    /// `float.title`
    Title,
    /// `float.title.text`
    TitleText,
    /// `float.title.muted`
    TitleMuted,
    /// `float.grip`
    Grip,
    /// `float.focus`
    Focus,
}

impl FloatField {
    const fn path(self) -> &'static str {
        match self {
            FloatField::Radius => "radius",
            FloatField::Shadow => "shadow",
            FloatField::Fill => "fill",
            FloatField::Border => "border",
            FloatField::Blocked => "blocked",
            FloatField::Title => "title",
            FloatField::TitleText => "title.text",
            FloatField::TitleMuted => "title.muted",
            FloatField::Grip => "grip",
            FloatField::Focus => "focus",
        }
    }
}

impl Family for FloatTokens {
    type Field = FloatField;
    const FIELDS: &'static [FloatField] = &[
        FloatField::Radius,
        FloatField::Shadow,
        FloatField::Fill,
        FloatField::Border,
        FloatField::Blocked,
        FloatField::Title,
        FloatField::TitleText,
        FloatField::TitleMuted,
        FloatField::Grip,
        FloatField::Focus,
    ];

    fn defaults(s: &Semantic<'_>) -> FloatTokens {
        let c = s.color;
        FloatTokens {
            border_width: values::FLOAT_BORDER_WIDTH,
            radius: values::FLOAT_RADIUS,
            shadow: s.elevation.floating,
            fill: c.surface.panel,
            border: c.line.border,
            blocked: c.surface.scrim,
            title: c.surface.background,
            title_text: c.text.normal,
            title_muted: c.text.muted,
            grip: c.accent.solid,
            focus: c.line.focus,
        }
    }

    fn path(f: FloatField) -> String {
        f.path().to_owned()
    }

    fn from_path(path: &str) -> Option<FloatField> {
        Self::FIELDS.iter().copied().find(|f| f.path() == path)
    }

    fn kind(f: FloatField) -> FieldKind {
        match f {
            FloatField::Radius => FieldKind::Length,
            FloatField::Shadow => FieldKind::Elevation,
            _ => FieldKind::Color,
        }
    }

    fn get(&self, f: FloatField) -> OverrideValue {
        match f {
            FloatField::Radius => OverrideValue::Length(self.radius),
            FloatField::Shadow => OverrideValue::Elevation(self.shadow),
            f => {
                let mut copy = *self;
                OverrideValue::Color(copy.color_mut(f).map_or(Rgba::TRANSPARENT, |c| *c))
            }
        }
    }

    fn set(&mut self, f: FloatField, v: OverrideValue) {
        match (f, v) {
            (FloatField::Radius, OverrideValue::Length(r)) => self.radius = r,
            (FloatField::Shadow, OverrideValue::Elevation(e)) => self.shadow = e,
            (f, OverrideValue::Color(c)) => {
                if let Some(slot) = self.color_mut(f) {
                    *slot = c;
                }
            }
            // Mismatched kinds are rejected before this is called.
            _ => {}
        }
    }

    fn pairs() -> Vec<ContrastPair> {
        let component = |f| TokenRef::Component(ComponentField::Float(f));
        let pair = |fg, bg, minimum| ContrastPair { fg: component(fg), bg: component(bg), minimum, alternate: None };
        let (text, mark) = (values::TEXT_MIN, values::NON_TEXT_MIN);
        vec![
            pair(FloatField::TitleText, FloatField::Title, text),
            pair(FloatField::TitleMuted, FloatField::Title, mark),
            pair(FloatField::Focus, FloatField::Title, mark),
            ContrastPair {
                fg: component(FloatField::Focus),
                bg: TokenRef::Component(ComponentField::Dock(DockField::Background)),
                minimum: mark,
                alternate: None,
            },
        ]
    }
}
