//! Popovers: non-modal popups anchored to a control or a point, with an optional resize grip. The
//! card is the popup surface; this family holds the padding inside it, the grip and the room kept
//! to the window's edge. The grip's ticks are a pointer affordance for a function the keyboard
//! does not need, so they carry no contrast minimum.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::values;

/// Popover tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PopoverTokens {
    /// Between the card's edge and its contents: `space.gap.stack`.
    pub padding: f32,
    /// Side of the grip's square at the bottom trailing corner.
    pub grip_size: f32,
    /// Distance between the grip's ticks.
    pub grip_pitch: f32,
    /// The grip's ticks: `text.muted`.
    pub grip_tick: Rgba,
    /// Room kept to the window's edge when the popover grows or is clamped.
    pub screen_margin: f32,
}

/// An overridable popover field. Path: `grip.tick`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PopoverField {
    /// `popover.grip.tick`
    GripTick,
}

impl Family for PopoverTokens {
    type Field = PopoverField;
    const FIELDS: &'static [PopoverField] = &[PopoverField::GripTick];

    fn defaults(s: &Semantic<'_>) -> PopoverTokens {
        PopoverTokens {
            padding: s.space.gap.stack,
            grip_size: values::POPOVER_GRIP_SIZE,
            grip_pitch: values::POPOVER_GRIP_PITCH,
            grip_tick: s.color.text.muted,
            screen_margin: values::POPOVER_SCREEN_MARGIN,
        }
    }

    fn path(f: PopoverField) -> String {
        match f {
            PopoverField::GripTick => "grip.tick",
        }
        .to_owned()
    }

    fn from_path(path: &str) -> Option<PopoverField> {
        match path {
            "grip.tick" => Some(PopoverField::GripTick),
            _ => None,
        }
    }

    fn kind(_: PopoverField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: PopoverField) -> OverrideValue {
        match f {
            PopoverField::GripTick => OverrideValue::Color(self.grip_tick),
        }
    }

    fn set(&mut self, f: PopoverField, v: OverrideValue) {
        // Mismatched kinds are rejected before this is called.
        if let (PopoverField::GripTick, OverrideValue::Color(c)) = (f, v) {
            self.grip_tick = c;
        }
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
