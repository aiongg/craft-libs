//! Sliders: a sunken track, the part from the start to the value filled in the accent colour, and
//! a raised thumb on the value. The track and thumb take their edges from the `Surface` primitive
//! (a border, or a bevel in Classic); this family holds their sizes, corners and colours.

use super::{BySize, ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic, State, States};
use crate::color::Rgba;
use crate::tokens::TokenRef;
use crate::values;

/// One coloured part of a slider's thumb.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ThumbPart {
    /// The face.
    Fill,
    /// The outline; a bevel keeps its own colours.
    Border,
}

impl ThumbPart {
    /// Every part, in path order.
    pub const ALL: [ThumbPart; 2] = [ThumbPart::Fill, ThumbPart::Border];

    /// The path segment, e.g. `"fill"`.
    pub const fn name(self) -> &'static str {
        match self {
            ThumbPart::Fill => "fill",
            ThumbPart::Border => "border",
        }
    }
}

/// The colours of a slider's thumb, per state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ThumbColors {
    /// Face.
    pub fill: States<Rgba>,
    /// Outline.
    pub border: States<Rgba>,
}

impl ThumbColors {
    /// The colours of `part`.
    pub fn part(&self, part: ThumbPart) -> &States<Rgba> {
        match part {
            ThumbPart::Fill => &self.fill,
            ThumbPart::Border => &self.border,
        }
    }

    fn part_mut(&mut self, part: ThumbPart) -> &mut States<Rgba> {
        match part {
            ThumbPart::Fill => &mut self.fill,
            ThumbPart::Border => &mut self.border,
        }
    }
}

/// Slider tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SliderTokens {
    /// Height of the control, the thumb's pointer target: the control height.
    pub height: BySize<f32>,
    /// Default length: [`values::SLIDER_WIDTH`] control heights. A justified layout stretches it.
    pub width: BySize<f32>,
    /// Diameter (or side) of the thumb: the icon size.
    pub thumb: BySize<f32>,
    /// Thickness of the track.
    pub track_height: f32,
    /// Corners of the track and the thumb, each clamped to half its height; 999 gives a pill
    /// track and a round thumb.
    pub radius: f32,
    /// The track's fill: `surface.sunken`.
    pub track: Rgba,
    /// The filled part, from the start to the value: an accent state indicator (`accent.solid`,
    /// `accent.hover`, `accent.pressed`; `text.disabled` when disabled).
    pub fill: States<Rgba>,
    /// The thumb: a raised face (`surface.raised`, then `state.hover` and `state.pressed`) with a
    /// `line.control_edge` outline.
    pub thumb_colors: ThumbColors,
}

/// An overridable slider field. Paths: `radius`, `track`, `fill.<state>`,
/// `thumb.<part>.<state>`, e.g. `slider.thumb.border.hover`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SliderField {
    /// `slider.radius`
    Radius,
    /// `slider.track`
    Track,
    /// `slider.fill.<state>`
    Fill(State),
    /// `slider.thumb.<part>.<state>`
    Thumb {
        /// The part.
        part: ThumbPart,
        /// The state.
        state: State,
    },
}

const FIELD_COUNT: usize = 2 + State::ALL.len() + ThumbPart::ALL.len() * State::ALL.len();

const FIELDS: [SliderField; FIELD_COUNT] = {
    let mut out = [SliderField::Radius; FIELD_COUNT];
    out[1] = SliderField::Track;
    let mut i = 0;
    while i < State::ALL.len() {
        out[2 + i] = SliderField::Fill(State::ALL[i]);
        i += 1;
    }
    let mut j = 0;
    while j < ThumbPart::ALL.len() * State::ALL.len() {
        out[2 + State::ALL.len() + j] = SliderField::Thumb { part: ThumbPart::ALL[j / State::ALL.len()], state: State::ALL[j % State::ALL.len()] };
        j += 1;
    }
    out
};

impl SliderTokens {
    fn color_mut(&mut self, f: SliderField) -> Option<&mut Rgba> {
        match f {
            SliderField::Radius => None,
            SliderField::Track => Some(&mut self.track),
            SliderField::Fill(state) => Some(self.fill.get_mut(state)),
            SliderField::Thumb { part, state } => Some(self.thumb_colors.part_mut(part).get_mut(state)),
        }
    }
}

impl Family for SliderTokens {
    type Field = SliderField;
    const FIELDS: &'static [SliderField] = &FIELDS;

    fn defaults(s: &Semantic<'_>) -> SliderTokens {
        let (c, size) = (s.color, s.size);
        let edge = c.line.control_edge;
        SliderTokens {
            height: BySize { sm: size.control.sm, md: size.control.md },
            width: BySize { sm: values::SLIDER_WIDTH * size.control.sm, md: values::SLIDER_WIDTH * size.control.md },
            thumb: BySize { sm: size.icon.sm, md: size.icon.md },
            track_height: values::SLIDER_TRACK,
            radius: values::SLIDER_RADIUS.px(),
            track: c.surface.sunken,
            fill: States::new(c.accent.solid, c.accent.hover, c.accent.pressed, c.text.disabled),
            thumb_colors: ThumbColors {
                fill: States::new(c.surface.raised, c.state.hover, c.state.pressed, c.state.disabled_fill),
                border: States::new(edge, edge, edge, c.line.divider),
            },
        }
    }

    fn path(f: SliderField) -> String {
        match f {
            SliderField::Radius => "radius".to_owned(),
            SliderField::Track => "track".to_owned(),
            SliderField::Fill(state) => format!("fill.{}", state.name()),
            SliderField::Thumb { part, state } => format!("thumb.{}.{}", part.name(), state.name()),
        }
    }

    fn from_path(path: &str) -> Option<SliderField> {
        match path {
            "radius" => return Some(SliderField::Radius),
            "track" => return Some(SliderField::Track),
            _ => {}
        }
        if let Some(state) = path.strip_prefix("fill.") {
            return State::from_name(state).map(SliderField::Fill);
        }
        let (part, state) = path.strip_prefix("thumb.")?.split_once('.')?;
        Some(SliderField::Thumb { part: ThumbPart::ALL.into_iter().find(|p| p.name() == part)?, state: State::from_name(state)? })
    }

    fn kind(f: SliderField) -> FieldKind {
        match f {
            SliderField::Radius => FieldKind::Length,
            _ => FieldKind::Color,
        }
    }

    fn get(&self, f: SliderField) -> OverrideValue {
        match f {
            SliderField::Radius => OverrideValue::Length(self.radius),
            SliderField::Track => OverrideValue::Color(self.track),
            SliderField::Fill(state) => OverrideValue::Color(self.fill.get(state)),
            SliderField::Thumb { part, state } => OverrideValue::Color(self.thumb_colors.part(part).get(state)),
        }
    }

    fn set(&mut self, f: SliderField, v: OverrideValue) {
        match (f, v) {
            (SliderField::Radius, OverrideValue::Length(r)) => self.radius = r,
            (f, OverrideValue::Color(c)) => {
                if let Some(slot) = self.color_mut(f) {
                    *slot = c;
                }
            }
            // Mismatched kinds are rejected before this is called.
            _ => {}
        }
    }

    /// The filled part against the track: an accent state indicator (WCAG 1.4.11), 3:1 and 4.5:1
    /// with high contrast. The thumb's outline is a control edge, a design target.
    fn pairs() -> Vec<ContrastPair> {
        let field = |f| ComponentField::Slider(f);
        [State::Rest, State::Hover, State::Pressed]
            .into_iter()
            .map(|state| ContrastPair {
                fg: field(SliderField::Fill(state)),
                bg: TokenRef::Component(field(SliderField::Track)),
                minimum: values::NON_TEXT_MIN,
            })
            .collect()
    }
}
