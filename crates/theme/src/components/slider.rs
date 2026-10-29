//! Sliders: a flat track in the control edge colour, the part from the start to the value filled
//! in the accent colour, and a knob on the value. The thumb takes its edge from the `Surface`
//! primitive (an outline, or a bevel in Classic) and its shadow from `elevation.knob`; this
//! family holds the sizes, corners and colours.

use super::{
    BySize, ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic, State, States, THUMB_FIELDS, ThumbColors, ThumbPart,
    thumb_from_path, thumb_path,
};
use crate::color::Rgba;
use crate::tokens::{ColorToken, TokenRef};
use crate::values;

/// `sizes` with its standard size set to `md`, the small size keeping its step under it.
fn stepped(sizes: BySize<f32>, md: f32) -> BySize<f32> {
    BySize { sm: md - (sizes.md - sizes.sm), md }
}

/// Slider tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SliderTokens {
    /// Height of the control, the thumb's pointer target: the control height.
    pub height: BySize<f32>,
    /// Default length: [`values::SLIDER_WIDTH`] control heights. A justified layout stretches it.
    pub width: BySize<f32>,
    /// Height of the thumb, and its diameter while round: the icon size less
    /// [`values::SLIDER_THUMB_INSET`] (PhotoCraft's 12 at Compact).
    pub thumb: BySize<f32>,
    /// Width of the thumb: its height unless the theme sets another shape.
    pub thumb_width: BySize<f32>,
    /// Thickness of the track: [`values::SLIDER_TRACK`].
    pub track_height: f32,
    /// Thickness of a gradient track: [`values::SLIDER_GRADIENT_TRACK`]. It has square ends and
    /// no outline.
    pub gradient_height: f32,
    /// Corners of the track: [`values::SLIDER_TRACK_RADIUS`], clamped to half its height.
    pub track_radius: f32,
    /// Corners of the thumb, clamped to half its shorter side; 999 gives a round thumb.
    pub radius: f32,
    /// The track's fill: `line.control_edge`.
    pub track: Rgba,
    /// The filled part, from the start to the value: an accent state indicator (`accent.solid` in
    /// every state; `text.disabled` when disabled).
    pub fill: States<Rgba>,
    /// The thumb: a knob, a `surface.knob` face with a `line.knob_edge` outline in every
    /// enabled state; disabled, a face halfway to `surface.panel` with a `line.divider` outline.
    pub thumb_colors: ThumbColors,
    /// The ring round the thumb: clear at rest and when disabled, `accent.soft` while hovered or
    /// pressed.
    pub ring: States<Rgba>,
    /// Width of the ring: [`values::KNOB_RING`].
    pub ring_width: f32,
    /// From the thumb's edge to the ring's centre line: [`values::KNOB_RING_OFFSET`].
    pub ring_offset: f32,
    /// Height of the marks and ticks under the track ([`values::SLIDER_MARK`] at the Compact
    /// control height, scaled by the control height of the size and rounded to whole points).
    /// Their colour is `line.divider`. Not overridable.
    pub mark: BySize<f32>,
    /// How near a mark a drag has to come for the thumb to snap to it, in points:
    /// [`values::SLIDER_SNAP`]. Not overridable.
    pub snap: f32,
}

/// An overridable slider field. Paths: `radius`, `track`, `thumb_width`, `thumb_height`,
/// `fill.<state>`, `thumb.<part>.<state>`, `ring.<state>`, e.g. `slider.thumb.border.hover`. The
/// two thumb lengths set the standard size; the small size keeps its step under it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SliderField {
    /// `slider.radius`
    Radius,
    /// `slider.track`
    Track,
    /// `slider.thumb_width`
    ThumbWidth,
    /// `slider.thumb_height`
    ThumbHeight,
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

const FIELD_COUNT: usize = 4 + State::ALL.len() + THUMB_FIELDS.len() + State::ALL.len();

const FIELDS: [SliderField; FIELD_COUNT] = {
    let mut out = [SliderField::Radius; FIELD_COUNT];
    out[1] = SliderField::Track;
    out[2] = SliderField::ThumbWidth;
    out[3] = SliderField::ThumbHeight;
    let mut i = 0;
    while i < State::ALL.len() {
        out[4 + i] = SliderField::Fill(State::ALL[i]);
        i += 1;
    }
    let mut j = 0;
    while j < THUMB_FIELDS.len() {
        let (part, state) = THUMB_FIELDS[j];
        out[4 + State::ALL.len() + j] = SliderField::Thumb { part, state };
        j += 1;
    }
    let mut k = 0;
    while k < State::ALL.len() {
        out[4 + State::ALL.len() + THUMB_FIELDS.len() + k] = SliderField::Ring(State::ALL[k]);
        k += 1;
    }
    out
};

impl SliderTokens {
    fn color_mut(&mut self, f: SliderField) -> Option<&mut Rgba> {
        match f {
            SliderField::Radius | SliderField::ThumbWidth | SliderField::ThumbHeight => None,
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
        let thumb = BySize { sm: size.icon.sm - values::SLIDER_THUMB_INSET.0, md: size.icon.md - values::SLIDER_THUMB_INSET.1 };
        let height = BySize { sm: size.control.sm - values::SLIDER_HEIGHT_INSET.0, md: size.control.md - values::SLIDER_HEIGHT_INSET.1 };
        SliderTokens {
            height,
            width: BySize { sm: values::SLIDER_WIDTH * size.control.sm, md: values::SLIDER_WIDTH * size.control.md },
            thumb,
            thumb_width: thumb,
            track_height: values::SLIDER_TRACK,
            gradient_height: values::SLIDER_GRADIENT_TRACK,
            track_radius: values::SLIDER_TRACK_RADIUS,
            radius: s.shape.radius.pill,
            track: c.line.control_edge,
            fill: States::new(c.accent.solid, c.accent.solid, c.accent.solid, c.text.disabled),
            thumb_colors: ThumbColors::knob(c),
            ring: States::new(clear, soft, soft, clear),
            ring_width: values::KNOB_RING,
            ring_offset: values::KNOB_RING_OFFSET,
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
            SliderField::ThumbWidth => "thumb_width".to_owned(),
            SliderField::ThumbHeight => "thumb_height".to_owned(),
            SliderField::Fill(state) => format!("fill.{}", state.name()),
            SliderField::Thumb { part, state } => format!("thumb.{}", thumb_path(part, state)),
            SliderField::Ring(state) => format!("ring.{}", state.name()),
        }
    }

    fn from_path(path: &str) -> Option<SliderField> {
        match path {
            "radius" => return Some(SliderField::Radius),
            "track" => return Some(SliderField::Track),
            "thumb_width" => return Some(SliderField::ThumbWidth),
            "thumb_height" => return Some(SliderField::ThumbHeight),
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
            SliderField::Radius | SliderField::ThumbWidth | SliderField::ThumbHeight => FieldKind::Length,
            _ => FieldKind::Color,
        }
    }

    fn get(&self, f: SliderField) -> OverrideValue {
        match f {
            SliderField::Radius => OverrideValue::Length(self.radius),
            SliderField::ThumbWidth => OverrideValue::Length(self.thumb_width.md),
            SliderField::ThumbHeight => OverrideValue::Length(self.thumb.md),
            SliderField::Track => OverrideValue::Color(self.track),
            SliderField::Fill(state) => OverrideValue::Color(self.fill.get(state)),
            SliderField::Thumb { part, state } => OverrideValue::Color(self.thumb_colors.part(part).get(state)),
            SliderField::Ring(state) => OverrideValue::Color(self.ring.get(state)),
        }
    }

    fn set(&mut self, f: SliderField, v: OverrideValue) {
        match (f, v) {
            (SliderField::Radius, OverrideValue::Length(r)) => self.radius = r,
            (SliderField::ThumbWidth, OverrideValue::Length(w)) => self.thumb_width = stepped(self.thumb_width, w),
            (SliderField::ThumbHeight, OverrideValue::Length(h)) => self.thumb = stepped(self.thumb, h),
            (f, OverrideValue::Color(c)) => {
                if let Some(slot) = self.color_mut(f) {
                    *slot = c;
                }
            }
            // Mismatched kinds are rejected before this is called.
            _ => {}
        }
    }

    /// The track against the panel or a popup, and the filled part against the track: the
    /// control edge minimum ([`values::CONTROL_EDGE_MIN`]), 3:1 with high contrast and listed,
    /// never warned, in Standard, where the accent fill sits on PhotoCraft's `line.control_edge`
    /// track. The thumb is neither text nor a state indicator, so it has no minimum: where its
    /// face is close to the fill under it, its outline or shadow gives the edge.
    ///
    /// The track comes first: high contrast moves it before the fill is fitted to it.
    fn pairs() -> Vec<ContrastPair> {
        let field = |f| ComponentField::Slider(f);
        let tracks = [ColorToken::SurfacePanel, ColorToken::SurfaceRaised].map(|surface| ContrastPair {
            fg: TokenRef::Component(field(SliderField::Track)),
            bg: TokenRef::Color(surface),
            minimum: values::CONTROL_EDGE_MIN,
            alternate: None,
        });
        let fills = [State::Rest, State::Hover, State::Pressed].map(|state| ContrastPair {
            fg: TokenRef::Component(field(SliderField::Fill(state))),
            bg: TokenRef::Component(field(SliderField::Track)),
            minimum: values::CONTROL_EDGE_MIN,
            alternate: None,
        });
        tracks.into_iter().chain(fills).collect()
    }
}
