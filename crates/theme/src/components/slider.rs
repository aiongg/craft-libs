//! Sliders: a sunken track, the part from the start to the value filled in the accent colour, and
//! a knob on the value. The track and thumb take their edges from the `Surface` primitive (a
//! border, or a bevel in Classic) and the thumb its shadow from `elevation.knob`; this family
//! holds their sizes, corners and colours.

use super::{
    BySize, ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic, State, States, THUMB_FIELDS, ThumbColors, ThumbPart,
    thumb_from_path, thumb_path,
};
use crate::color::Rgba;
use crate::tokens::TokenRef;
use crate::values;

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
    /// The thumb: a knob, a `surface.knob` face with a `line.knob_edge` outline in every
    /// enabled state; disabled, a face halfway to `surface.panel` with a `line.divider` outline.
    pub thumb_colors: ThumbColors,
    /// The ring round the thumb: clear at rest and when disabled, `accent.soft` while hovered or
    /// pressed.
    pub ring: States<Rgba>,
    /// Width of the ring: [`values::KNOB_RING`].
    pub ring_width: f32,
    /// Height of the marks and ticks under the track ([`values::SLIDER_MARK`] at the Compact
    /// control height, scaled by the control height of the size and rounded to whole points).
    /// Their colour is `line.divider`. Not overridable.
    pub mark: BySize<f32>,
    /// How near a mark a drag has to come for the thumb to snap to it, in points:
    /// [`values::SLIDER_SNAP`]. Not overridable.
    pub snap: f32,
}

/// An overridable slider field. Paths: `radius`, `track`, `fill.<state>`,
/// `thumb.<part>.<state>`, `ring.<state>`, e.g. `slider.thumb.border.hover`.
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
    /// `slider.ring.<state>`
    Ring(State),
}

const FIELD_COUNT: usize = 2 + State::ALL.len() + THUMB_FIELDS.len() + State::ALL.len();

const FIELDS: [SliderField; FIELD_COUNT] = {
    let mut out = [SliderField::Radius; FIELD_COUNT];
    out[1] = SliderField::Track;
    let mut i = 0;
    while i < State::ALL.len() {
        out[2 + i] = SliderField::Fill(State::ALL[i]);
        i += 1;
    }
    let mut j = 0;
    while j < THUMB_FIELDS.len() {
        let (part, state) = THUMB_FIELDS[j];
        out[2 + State::ALL.len() + j] = SliderField::Thumb { part, state };
        j += 1;
    }
    let mut k = 0;
    while k < State::ALL.len() {
        out[2 + State::ALL.len() + THUMB_FIELDS.len() + k] = SliderField::Ring(State::ALL[k]);
        k += 1;
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
            SliderField::Ring(state) => Some(self.ring.get_mut(state)),
        }
    }
}

impl Family for SliderTokens {
    type Field = SliderField;
    const FIELDS: &'static [SliderField] = &FIELDS;

    fn defaults(s: &Semantic<'_>) -> SliderTokens {
        let (c, size) = (s.color, s.size);
        let (clear, soft) = (Rgba::TRANSPARENT, c.accent.soft);
        SliderTokens {
            height: BySize { sm: size.control.sm, md: size.control.md },
            width: BySize { sm: values::SLIDER_WIDTH * size.control.sm, md: values::SLIDER_WIDTH * size.control.md },
            thumb: BySize { sm: size.icon.sm, md: size.icon.md },
            track_height: values::SLIDER_TRACK,
            radius: s.shape.radius.pill,
            track: c.surface.sunken,
            fill: States::new(c.accent.solid, c.accent.hover, c.accent.pressed, c.text.disabled),
            thumb_colors: ThumbColors::knob(c),
            ring: States::new(clear, soft, soft, clear),
            ring_width: values::KNOB_RING,
            mark: BySize {
                sm: (values::SLIDER_MARK * size.control.sm / values::SLIDER_BASE_CONTROL).round(),
                md: (values::SLIDER_MARK * size.control.md / values::SLIDER_BASE_CONTROL).round(),
            },
            snap: values::SLIDER_SNAP,
        }
    }

    fn path(f: SliderField) -> String {
        match f {
            SliderField::Radius => "radius".to_owned(),
            SliderField::Track => "track".to_owned(),
            SliderField::Fill(state) => format!("fill.{}", state.name()),
            SliderField::Thumb { part, state } => format!("thumb.{}", thumb_path(part, state)),
            SliderField::Ring(state) => format!("ring.{}", state.name()),
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
        if let Some(state) = path.strip_prefix("ring.") {
            return State::from_name(state).map(SliderField::Ring);
        }
        let (part, state) = thumb_from_path(path.strip_prefix("thumb.")?)?;
        Some(SliderField::Thumb { part, state })
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
            SliderField::Ring(state) => OverrideValue::Color(self.ring.get(state)),
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
    /// with high contrast. The thumb is neither text nor a state indicator, so it has no minimum:
    /// where its face is close to the fill under it, its outline or shadow gives the edge.
    fn pairs() -> Vec<ContrastPair> {
        let field = |f| ComponentField::Slider(f);
        [State::Rest, State::Hover, State::Pressed]
            .into_iter()
            .map(|state| ContrastPair {
                fg: TokenRef::Component(field(SliderField::Fill(state))),
                bg: TokenRef::Component(field(SliderField::Track)),
                minimum: values::NON_TEXT_MIN,
                alternate: None,
            })
            .collect()
    }
}
