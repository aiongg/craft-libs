//! Segmented controls: separate pills side by side (PhotoCraft's `pill_tab`), one of them
//! selected. This family holds the pills' sizes and the colours of plain and selected pills.

use super::list::half;
use super::{
    BySize, ComponentField, ContrastPair, Family, FieldKind, ITEM_FIELDS, Identify, ItemColors, ItemPart, OverrideValue, Semantic, SizeLeaf, State,
    States,
};
use crate::color::Rgba;
use crate::tokens::{ColorToken, TokenRef, TypeRole};
use crate::values;

/// Segmented-control tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SegmentedTokens {
    /// Height of a pill: the control height.
    pub height: BySize<f32>,
    /// Horizontal padding at the start and end of a pill.
    pub pad_x: BySize<f32>,
    /// Icon size in icon pills.
    pub icon: BySize<f32>,
    /// Between pills.
    pub gap: f32,
    /// Label type role.
    pub label: TypeRole,
    /// Pills that are not selected: transparent until hovered, then `state.hover` at
    /// [`values::SEGMENT_HOVER_OPACITY`]; `text.muted`.
    pub segment: ItemColors,
    /// The selected pill: `text.normal` on `state.hover`. In high contrast the fill or its edge
    /// reaches the non-text minimum on the panel and on a popup.
    pub selected: ItemColors,
    /// The 1 pt outline inside the selected pill: `line.control_edge`; `line.divider` when
    /// disabled.
    pub selected_border: States<Rgba>,
}

/// An overridable segmented-control field. Paths: `segment.<part>.<state>`,
/// `selected.<part>.<state>`, `selected.border.<state>`, e.g. `segmented.selected.fill.rest`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SegmentedField {
    /// A colour of pills that are not selected.
    Segment {
        /// The part.
        part: ItemPart,
        /// The state.
        state: State,
    },
    /// A colour of the selected pill.
    Selected {
        /// The part.
        part: ItemPart,
        /// The state.
        state: State,
    },
    /// The outline of the selected pill.
    SelectedBorder(State),
}

const FIELD_COUNT: usize = 2 * ITEM_FIELDS.len() + State::ALL.len();

const FIELDS: [SegmentedField; FIELD_COUNT] = {
    let mut out = [SegmentedField::SelectedBorder(State::Rest); FIELD_COUNT];
    let mut i = 0;
    while i < ITEM_FIELDS.len() {
        let (part, state) = ITEM_FIELDS[i];
        out[i] = SegmentedField::Segment { part, state };
        out[ITEM_FIELDS.len() + i] = SegmentedField::Selected { part, state };
        i += 1;
    }
    let mut i = 0;
    while i < State::ALL.len() {
        out[2 * ITEM_FIELDS.len() + i] = SegmentedField::SelectedBorder(State::ALL[i]);
        i += 1;
    }
    out
};

impl SegmentedTokens {
    fn colors_mut(&mut self, f: SegmentedField) -> Option<&mut Rgba> {
        match f {
            SegmentedField::Segment { part, state } => Some(self.segment.get_mut(part, state)),
            SegmentedField::Selected { part, state } => Some(self.selected.get_mut(part, state)),
            SegmentedField::SelectedBorder(state) => Some(self.selected_border.get_mut(state)),
        }
    }
}

impl Family for SegmentedTokens {
    type Field = SegmentedField;
    const FIELDS: &'static [SegmentedField] = &FIELDS;
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { SegmentedTokens;
        "height.sm" 0.0..=1024.0 => height.sm;
        "height.md" 0.0..=1024.0 => height.md;
        "pad_x.sm" 0.0..=64.0 => pad_x.sm;
        "pad_x.md" 0.0..=64.0 => pad_x.md;
        "icon.sm" 0.0..=1024.0 => icon.sm;
        "icon.md" 0.0..=1024.0 => icon.md;
        "gap" 0.0..=64.0 => gap;
    };

    fn defaults(s: &Semantic<'_>) -> SegmentedTokens {
        let (c, size, space) = (s.color, s.size, s.space);
        let clear = Rgba::TRANSPARENT;
        let hover = half(c.state.hover, values::SEGMENT_HOVER_OPACITY);
        let edge = c.line.control_edge;
        let (fill, text) = Identify::on(&[c.surface.panel, c.surface.raised], s.high).edge(edge).fill(c.state.hover, c.text.normal);
        SegmentedTokens {
            height: BySize { sm: size.control.sm, md: size.control.md },
            pad_x: BySize { sm: space.pad.control, md: space.pad.control + space.gap.inline },
            icon: BySize { sm: size.icon.sm, md: size.icon.md },
            gap: space.gap.inline + values::SEGMENTED_GAP_OVER_INLINE,
            label: TypeRole::Label,
            segment: ItemColors {
                fill: States::new(clear, hover, hover, clear),
                text: States::new(c.text.muted, c.text.muted, c.text.muted, c.text.disabled),
            },
            selected: ItemColors { fill: States::new(fill, fill, fill, c.state.disabled_fill), text: States::new(text, text, text, c.text.disabled) },
            selected_border: States::new(edge, edge, edge, c.line.divider),
        }
    }

    fn path(f: SegmentedField) -> String {
        match f {
            SegmentedField::Segment { part, state } => format!("segment.{}", super::item_path(part, state)),
            SegmentedField::Selected { part, state } => format!("selected.{}", super::item_path(part, state)),
            SegmentedField::SelectedBorder(state) => format!("selected.border.{}", state.name()),
        }
    }

    fn from_path(path: &str) -> Option<SegmentedField> {
        if let Some(state) = path.strip_prefix("selected.border.") {
            return State::from_name(state).map(SegmentedField::SelectedBorder);
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
            SegmentedField::Segment { part, state } => self.segment.part(part).get(state),
            SegmentedField::Selected { part, state } => self.selected.part(part).get(state),
            SegmentedField::SelectedBorder(state) => self.selected_border.get(state),
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
        let field = |f| TokenRef::Component(ComponentField::Segmented(f));
        let surfaces = [ColorToken::SurfacePanel, ColorToken::SurfaceRaised].map(TokenRef::Color);
        let mut out = Vec::new();
        // A resting pill is transparent: its text sits on the panel or on a popup.
        for bg in surfaces {
            out.push(ContrastPair {
                fg: field(F::Segment { part: ItemPart::Text, state: State::Rest }),
                bg,
                minimum: values::TEXT_MIN,
                alternate: None,
            });
        }
        out.push(ContrastPair {
            fg: field(F::Segment { part: ItemPart::Text, state: State::Hover }),
            bg: field(F::Segment { part: ItemPart::Fill, state: State::Hover }),
            minimum: values::TEXT_MIN,
            alternate: None,
        });
        // The selected pill shows by its fill or its outline; the fill comes before its text:
        // high contrast moves the fill first.
        for bg in surfaces {
            out.push(ContrastPair {
                fg: field(F::Selected { part: ItemPart::Fill, state: State::Rest }),
                bg,
                minimum: values::STATE_CUE_MIN,
                alternate: Some(field(F::SelectedBorder(State::Rest))),
            });
        }
        out.push(ContrastPair {
            fg: field(F::Selected { part: ItemPart::Text, state: State::Rest }),
            bg: field(F::Selected { part: ItemPart::Fill, state: State::Rest }),
            minimum: values::TEXT_MIN,
            alternate: None,
        });
        out
    }
}
