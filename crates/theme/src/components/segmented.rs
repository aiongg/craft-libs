//! Segmented controls: a sunken track holding a row of segments, one of them selected. The track
//! takes its edge from the `Surface` primitive; this family holds its fill, the segments' sizes
//! and the colours of plain and selected segments.

use super::{BySize, ComponentField, ContrastPair, Family, FieldKind, ITEM_FIELDS, ItemColors, ItemPart, OverrideValue, Semantic, State, States};
use crate::color::Rgba;
use crate::tokens::{TokenRef, TypeRole};
use crate::values;

/// Segmented-control tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SegmentedTokens {
    /// Height of the track: the control height.
    pub height: BySize<f32>,
    /// Horizontal padding at the start and end of a segment.
    pub pad_x: BySize<f32>,
    /// Icon size in icon segments.
    pub icon: BySize<f32>,
    /// Between the track's edge and its segments.
    pub inset: f32,
    /// Label type role.
    pub label: TypeRole,
    /// The track's fill: `surface.sunken`.
    pub track: Rgba,
    /// Segments that are not selected: transparent on the track until hovered.
    pub segment: ItemColors,
    /// The selected segment: `text.on_selected` on `state.selected`.
    pub selected: ItemColors,
}

/// An overridable segmented-control field. Paths: `track`, `segment.<part>.<state>`,
/// `selected.<part>.<state>`, e.g. `segmented.selected.fill.rest`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SegmentedField {
    /// `segmented.track`
    Track,
    /// A colour of segments that are not selected.
    Segment {
        /// The part.
        part: ItemPart,
        /// The state.
        state: State,
    },
    /// A colour of the selected segment.
    Selected {
        /// The part.
        part: ItemPart,
        /// The state.
        state: State,
    },
}

const FIELD_COUNT: usize = 1 + 2 * ITEM_FIELDS.len();

const FIELDS: [SegmentedField; FIELD_COUNT] = {
    let mut out = [SegmentedField::Track; FIELD_COUNT];
    let mut i = 0;
    while i < ITEM_FIELDS.len() {
        let (part, state) = ITEM_FIELDS[i];
        out[1 + i] = SegmentedField::Segment { part, state };
        out[1 + ITEM_FIELDS.len() + i] = SegmentedField::Selected { part, state };
        i += 1;
    }
    out
};

impl SegmentedTokens {
    fn colors_mut(&mut self, f: SegmentedField) -> Option<&mut Rgba> {
        match f {
            SegmentedField::Track => Some(&mut self.track),
            SegmentedField::Segment { part, state } => Some(self.segment.get_mut(part, state)),
            SegmentedField::Selected { part, state } => Some(self.selected.get_mut(part, state)),
        }
    }
}

impl Family for SegmentedTokens {
    type Field = SegmentedField;
    const FIELDS: &'static [SegmentedField] = &FIELDS;

    fn defaults(s: &Semantic<'_>) -> SegmentedTokens {
        let (c, size, space) = (s.color, s.size, s.space);
        let clear = Rgba::TRANSPARENT;
        SegmentedTokens {
            height: BySize { sm: size.control.sm, md: size.control.md },
            pad_x: BySize { sm: space.pad.control, md: space.pad.control + space.gap.inline },
            icon: BySize { sm: size.icon.sm, md: size.icon.md },
            inset: values::SEGMENTED_INSET,
            label: TypeRole::Label,
            track: c.surface.sunken,
            segment: ItemColors {
                fill: States::new(clear, c.state.hover, c.state.pressed, clear),
                text: States::new(c.text.normal, c.text.normal, c.text.normal, c.text.disabled),
            },
            selected: ItemColors::selected(c),
        }
    }

    fn path(f: SegmentedField) -> String {
        match f {
            SegmentedField::Track => "track".to_owned(),
            SegmentedField::Segment { part, state } => format!("segment.{}", super::item_path(part, state)),
            SegmentedField::Selected { part, state } => format!("selected.{}", super::item_path(part, state)),
        }
    }

    fn from_path(path: &str) -> Option<SegmentedField> {
        if path == "track" {
            return Some(SegmentedField::Track);
        }
        let (set, rest) = path.split_once('.')?;
        let (part, state) = super::item_from_path(rest)?;
        match set {
            "segment" => Some(SegmentedField::Segment { part, state }),
            "selected" => Some(SegmentedField::Selected { part, state }),
            _ => None,
        }
    }

    fn kind(_: SegmentedField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: SegmentedField) -> OverrideValue {
        OverrideValue::Color(match f {
            SegmentedField::Track => self.track,
            SegmentedField::Segment { part, state } => self.segment.part(part).get(state),
            SegmentedField::Selected { part, state } => self.selected.part(part).get(state),
        })
    }

    fn set(&mut self, f: SegmentedField, v: OverrideValue) {
        // Mismatched kinds are rejected before this is called.
        if let (OverrideValue::Color(c), Some(slot)) = (v, self.colors_mut(f)) {
            *slot = c;
        }
    }

    fn pairs() -> Vec<ContrastPair> {
        use SegmentedField as F;
        let field = |f| ComponentField::Segmented(f);
        let mut out = Vec::new();
        // A resting segment is transparent: its text sits on the track.
        out.push(ContrastPair {
            fg: TokenRef::Component(field(F::Segment { part: ItemPart::Text, state: State::Rest })),
            bg: TokenRef::Component(field(F::Track)),
            minimum: values::TEXT_MIN,
        });
        for state in [State::Hover, State::Pressed] {
            out.push(ContrastPair {
                fg: TokenRef::Component(field(F::Segment { part: ItemPart::Text, state })),
                bg: TokenRef::Component(field(F::Segment { part: ItemPart::Fill, state })),
                minimum: values::TEXT_MIN,
            });
        }
        for state in [State::Rest, State::Hover, State::Pressed] {
            out.push(ContrastPair {
                fg: TokenRef::Component(field(F::Selected { part: ItemPart::Text, state })),
                bg: TokenRef::Component(field(F::Selected { part: ItemPart::Fill, state })),
                minimum: values::TEXT_MIN,
            });
        }
        out
    }
}
