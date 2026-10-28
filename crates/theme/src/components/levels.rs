//! Levels editors: a histogram, rows of triangle thumbs under it and an output gradient bar. Sizes
//! scale with the Size preset; the thumb edge and ring colours are overridable. The thumbs' faces
//! (black, grey, white) are image tones, not tokens.

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic, State, States};
use crate::color::Rgba;
use crate::tokens::{ColorToken, TokenRef};
use crate::values;

/// Levels editor tokens. Sizes are in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct LevelsTokens {
    /// Least width of the editor: `levels.min_width`.
    pub min_width: f32,
    /// Greatest width of the editor: `levels.max_width`.
    pub max_width: f32,
    /// Height of a row of thumbs: `levels.row`.
    pub row: f32,
    /// Width of a triangle thumb: `levels.thumb_width`.
    pub thumb_width: f32,
    /// Height of a triangle thumb: `levels.thumb_height`.
    pub thumb_height: f32,
    /// Distance from a thumb's centre within which a press grabs it: `levels.hit`.
    pub hit: f32,
    /// Thickness of the output gradient bar: `levels.bar`.
    pub bar: f32,
    /// A thumb's 1 pt edge, which shows a black or white face on any panel: `text.muted`.
    pub thumb_edge: Rgba,
    /// The edge of a disabled thumb: `text.disabled`.
    pub disabled_edge: Rgba,
    /// The ring round a thumb, outside its edge: clear at rest and when disabled, `accent.soft`
    /// while hovered or pressed (the slider's ring). Its width is `slider.ring_width`.
    pub ring: States<Rgba>,
}

/// An overridable levels field. Paths: `thumb_edge`, `disabled_edge`, `ring.<state>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LevelsField {
    /// `levels.thumb_edge`
    ThumbEdge,
    /// `levels.disabled_edge`
    DisabledEdge,
    /// `levels.ring.<state>`
    Ring(State),
}

const FIELDS: [LevelsField; 2 + State::ALL.len()] = {
    let mut out = [LevelsField::ThumbEdge; 2 + State::ALL.len()];
    out[1] = LevelsField::DisabledEdge;
    let mut i = 0;
    while i < State::ALL.len() {
        out[2 + i] = LevelsField::Ring(State::ALL[i]);
        i += 1;
    }
    out
};

impl LevelsTokens {
    fn color(&self, f: LevelsField) -> Rgba {
        match f {
            LevelsField::ThumbEdge => self.thumb_edge,
            LevelsField::DisabledEdge => self.disabled_edge,
            LevelsField::Ring(state) => self.ring.get(state),
        }
    }

    fn color_mut(&mut self, f: LevelsField) -> &mut Rgba {
        match f {
            LevelsField::ThumbEdge => &mut self.thumb_edge,
            LevelsField::DisabledEdge => &mut self.disabled_edge,
            LevelsField::Ring(state) => self.ring.get_mut(state),
        }
    }
}

impl Family for LevelsTokens {
    type Field = LevelsField;
    const FIELDS: &'static [LevelsField] = &FIELDS;

    /// The Compact sizes (PhotoCraft's Levels), scaled by the control height of the size and
    /// rounded to whole points.
    fn defaults(s: &Semantic<'_>) -> LevelsTokens {
        let k = s.size.control.md / values::LEVELS_BASE_CONTROL;
        let scale = |w: f32| (w * k).round();
        let c = s.color;
        let (clear, soft) = (Rgba::TRANSPARENT, c.accent.soft);
        LevelsTokens {
            min_width: scale(values::LEVELS_MIN_WIDTH),
            max_width: scale(values::LEVELS_MAX_WIDTH),
            row: scale(values::LEVELS_ROW),
            thumb_width: scale(values::LEVELS_THUMB_WIDTH),
            thumb_height: scale(values::LEVELS_THUMB_HEIGHT),
            hit: scale(values::LEVELS_HIT),
            bar: scale(values::LEVELS_BAR),
            thumb_edge: c.text.muted,
            disabled_edge: c.text.disabled,
            ring: States::new(clear, soft, soft, clear),
        }
    }

    fn path(f: LevelsField) -> String {
        match f {
            LevelsField::ThumbEdge => "thumb_edge".to_owned(),
            LevelsField::DisabledEdge => "disabled_edge".to_owned(),
            LevelsField::Ring(state) => format!("ring.{}", state.name()),
        }
    }

    fn from_path(path: &str) -> Option<LevelsField> {
        match path {
            "thumb_edge" => Some(LevelsField::ThumbEdge),
            "disabled_edge" => Some(LevelsField::DisabledEdge),
            _ => State::from_name(path.strip_prefix("ring.")?).map(LevelsField::Ring),
        }
    }

    fn kind(_: LevelsField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: LevelsField) -> OverrideValue {
        OverrideValue::Color(self.color(f))
    }

    fn set(&mut self, f: LevelsField, v: OverrideValue) {
        if let OverrideValue::Color(c) = v {
            *self.color_mut(f) = c;
        }
    }

    /// The thumb edge on the panel and on a raised surface (a dialog; WCAG 1.4.11), 3:1 and 4.5:1
    /// with high contrast: it is what shows a black face on a dark surface and a white one on a
    /// light one. The faces are image tones and the ring a hover cue, so they have no minimum.
    fn pairs() -> Vec<ContrastPair> {
        let edge = TokenRef::Component(ComponentField::Levels(LevelsField::ThumbEdge));
        [ColorToken::SurfacePanel, ColorToken::SurfaceRaised]
            .map(|bg| ContrastPair { fg: edge, bg: TokenRef::Color(bg), minimum: values::NON_TEXT_MIN })
            .to_vec()
    }
}
