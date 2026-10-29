//! Check boxes, radio buttons and switches. Each has an indicator (a box, a circle, a track) that
//! takes its edge from the `Surface` primitive's sunken surface. A check box or radio button that
//! is on shows a mark (the check or dash, the radio dot); a switch shows its state by its track
//! and where its thumb sits, a knob that is the same while off and on. This family holds their
//! sizes and the colours of the indicator while off and while on (checked, mixed, selected), of
//! the mark, and of the switch thumb.
//!
//! The mark, the box and its state are the cues that identify the control, so they have the WCAG
//! non-text minimum (3:1, 4.5:1 with high contrast) at rest: the mark against the on fill, and
//! the box against the control backgrounds (`surface.panel` and `surface.raised`). The box is
//! identifiable by its fill or its outline, on or off. Where neither the off fill nor the off
//! outline reaches the minimum, the outline moves in lightness, just far enough; `text.faint`,
//! its source, stays. Hover and pressed colours have no minimum, and neither has the switch
//! thumb: where its face is close to the track, its outline or shadow gives the edge.
//!
//! A disabled control that is on keeps an on look: a `text.disabled` fill with a
//! `state.disabled_fill` mark, the reverse of off's `state.disabled_fill` fill with a
//! `text.disabled` outline.

use super::identify::identifiable_outline;
use super::{
    BySize, ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic, State, States, THUMB_FIELDS, ThumbColors, ThumbPart,
    thumb_from_path, thumb_path,
};
use crate::color::Rgba;
use crate::tokens::{ColorToken, TokenRef, TypeRole};
use crate::values;

/// One coloured part of a check box, radio button or switch indicator while off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CheckOffPart {
    /// The inside of the box, circle or track.
    Fill,
    /// The outline of the box, circle or track, over a flat surface's border.
    Border,
}

impl CheckOffPart {
    /// Every part, in path order.
    pub const ALL: [CheckOffPart; 2] = [CheckOffPart::Fill, CheckOffPart::Border];

    /// The path segment, e.g. `"border"`.
    pub const fn name(self) -> &'static str {
        match self {
            CheckOffPart::Fill => "fill",
            CheckOffPart::Border => "border",
        }
    }
}

/// One coloured part of a check box, radio button or switch indicator while on (checked, mixed
/// or selected).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CheckOnPart {
    /// The inside of the box, circle or track.
    Fill,
    /// The outline of the box, circle or track, over a flat surface's border.
    Border,
    /// The check or dash, the radio dot. A switch draws none.
    Mark,
}

impl CheckOnPart {
    /// Every part, in path order.
    pub const ALL: [CheckOnPart; 3] = [CheckOnPart::Fill, CheckOnPart::Border, CheckOnPart::Mark];

    /// The path segment, e.g. `"mark"`.
    pub const fn name(self) -> &'static str {
        match self {
            CheckOnPart::Fill => "fill",
            CheckOnPart::Border => "border",
            CheckOnPart::Mark => "mark",
        }
    }
}

/// The colours of the indicator while off, per state. Off has no mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CheckOffColors {
    /// Inside of the box, circle or track.
    pub fill: States<Rgba>,
    /// Outline of the box, circle or track.
    pub border: States<Rgba>,
}

impl CheckOffColors {
    /// The colours of `part`.
    pub fn part(&self, part: CheckOffPart) -> &States<Rgba> {
        match part {
            CheckOffPart::Fill => &self.fill,
            CheckOffPart::Border => &self.border,
        }
    }

    fn part_mut(&mut self, part: CheckOffPart) -> &mut States<Rgba> {
        match part {
            CheckOffPart::Fill => &mut self.fill,
            CheckOffPart::Border => &mut self.border,
        }
    }
}

/// The colours of the indicator and its mark while on, per state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CheckOnColors {
    /// Inside of the box, circle or track.
    pub fill: States<Rgba>,
    /// Outline of the box, circle or track.
    pub border: States<Rgba>,
    /// Check, dash or dot.
    pub mark: States<Rgba>,
}

impl CheckOnColors {
    /// The colours of `part`.
    pub fn part(&self, part: CheckOnPart) -> &States<Rgba> {
        match part {
            CheckOnPart::Fill => &self.fill,
            CheckOnPart::Border => &self.border,
            CheckOnPart::Mark => &self.mark,
        }
    }

    fn part_mut(&mut self, part: CheckOnPart) -> &mut States<Rgba> {
        match part {
            CheckOnPart::Fill => &mut self.fill,
            CheckOnPart::Border => &mut self.border,
            CheckOnPart::Mark => &mut self.mark,
        }
    }
}

/// Check-box, radio-button and switch tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CheckTokens {
    /// Height of a check box, radio button or switch with its label: the control height.
    pub height: BySize<f32>,
    /// Side of the box, diameter of the circle, height of the switch track: the icon size.
    pub size: BySize<f32>,
    /// Between the indicator and the label.
    pub gap: f32,
    /// Label type role.
    pub label: TypeRole,
    /// Corner radius of the box: `shape.radius.control` or the theme's `check.radius`, at most
    /// [`CHECK_RADIUS_MAX`](values::CHECK_RADIUS_MAX).
    pub radius: f32,
    /// Stroke width of the check and the dash.
    pub mark: BySize<f32>,
    /// Diameter of the radio dot.
    pub dot: BySize<f32>,
    /// Width of the switch track.
    pub track: BySize<f32>,
    /// Between the switch track's edge and its thumb.
    pub thumb_inset: f32,
    /// Off: a `text.faint` outline on `surface.sunken`, the outline moved where neither shows.
    pub off: CheckOffColors,
    /// On: `accent.*` fill and outline, a `text.on_accent` mark; disabled, a `text.disabled` fill
    /// and outline with a `state.disabled_fill` mark.
    pub on: CheckOnColors,
    /// The switch thumb, off and on: a knob, a `surface.knob` face with a `line.knob_edge`
    /// outline in every enabled state; disabled, a face halfway to `surface.panel` with a
    /// `line.divider` outline.
    pub thumb: ThumbColors,
}

/// An overridable check field. Paths: `radius`, `off.<fill|border>.<state>`,
/// `on.<fill|border|mark>.<state>`, `thumb.<part>.<state>`, e.g. `check.on.mark.rest`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CheckField {
    /// `check.radius`
    Radius,
    /// `check.off.<part>.<state>`.
    Off {
        /// The part.
        part: CheckOffPart,
        /// The state.
        state: State,
    },
    /// `check.on.<part>.<state>`.
    On {
        /// The part.
        part: CheckOnPart,
        /// The state.
        state: State,
    },
    /// `check.thumb.<part>.<state>`: the switch thumb.
    Thumb {
        /// The part.
        part: ThumbPart,
        /// The state.
        state: State,
    },
}

const STATES: usize = State::ALL.len();
const OFF_COUNT: usize = CheckOffPart::ALL.len() * STATES;
const ON_COUNT: usize = CheckOnPart::ALL.len() * STATES;
const FIELD_COUNT: usize = 1 + OFF_COUNT + ON_COUNT + THUMB_FIELDS.len();

const FIELDS: [CheckField; FIELD_COUNT] = {
    let mut out = [CheckField::Radius; FIELD_COUNT];
    let mut i = 0;
    while i < OFF_COUNT {
        out[1 + i] = CheckField::Off { part: CheckOffPart::ALL[i / STATES], state: State::ALL[i % STATES] };
        i += 1;
    }
    let mut j = 0;
    while j < ON_COUNT {
        out[1 + OFF_COUNT + j] = CheckField::On { part: CheckOnPart::ALL[j / STATES], state: State::ALL[j % STATES] };
        j += 1;
    }
    let mut k = 0;
    while k < THUMB_FIELDS.len() {
        let (part, state) = THUMB_FIELDS[k];
        out[1 + OFF_COUNT + ON_COUNT + k] = CheckField::Thumb { part, state };
        k += 1;
    }
    out
};

impl Family for CheckTokens {
    type Field = CheckField;
    const FIELDS: &'static [CheckField] = &FIELDS;

    fn defaults(s: &Semantic<'_>) -> CheckTokens {
        let (c, size) = (s.color, s.size);
        let icon = BySize { sm: size.icon.sm, md: size.icon.md };
        // `text.faint` meets the text minimum on every surface and state fill, so the outline
        // meets the non-text minimum wherever the control sits; where a pinned `text.faint`
        // doesn't, and the fill doesn't either, the outline moves and `text.faint` stays.
        let sunken = c.surface.sunken;
        let rest_outline = identifiable_outline(c.text.faint, sunken, &[c.surface.panel, c.surface.raised], s.high);
        let outline = States::new(rest_outline, c.text.muted, c.text.muted, c.text.disabled);
        let accent = States::new(c.accent.solid, c.accent.hover, c.accent.pressed, c.text.disabled);
        let on_accent = c.text.on_accent;
        CheckTokens {
            height: BySize { sm: size.control.sm, md: size.control.md },
            size: icon,
            gap: s.space.gap.inline,
            label: TypeRole::Body,
            radius: s.shape.radius.control.min(values::CHECK_RADIUS_MAX),
            mark: BySize { sm: values::CHECK_MARK_SM, md: values::CHECK_MARK_MD },
            dot: BySize { sm: icon.sm * values::RADIO_DOT_RATIO, md: icon.md * values::RADIO_DOT_RATIO },
            track: BySize { sm: icon.sm * values::SWITCH_ASPECT, md: icon.md * values::SWITCH_ASPECT },
            thumb_inset: values::SWITCH_THUMB_INSET,
            off: CheckOffColors { fill: States::new(sunken, sunken, c.state.pressed, c.state.disabled_fill), border: outline },
            on: CheckOnColors { fill: accent, border: accent, mark: States::new(on_accent, on_accent, on_accent, c.state.disabled_fill) },
            thumb: ThumbColors::knob(c),
        }
    }

    fn path(f: CheckField) -> String {
        match f {
            CheckField::Radius => "radius".to_owned(),
            CheckField::Off { part, state } => format!("off.{}.{}", part.name(), state.name()),
            CheckField::On { part, state } => format!("on.{}.{}", part.name(), state.name()),
            CheckField::Thumb { part, state } => format!("thumb.{}", thumb_path(part, state)),
        }
    }

    fn from_path(path: &str) -> Option<CheckField> {
        if path == "radius" {
            return Some(CheckField::Radius);
        }
        if let Some(rest) = path.strip_prefix("thumb.") {
            let (part, state) = thumb_from_path(rest)?;
            return Some(CheckField::Thumb { part, state });
        }
        let mut segments = path.split('.');
        let (on_off, part, state) = (segments.next()?, segments.next()?, segments.next()?);
        if segments.next().is_some() {
            return None;
        }
        let state = State::from_name(state)?;
        match on_off {
            "off" => Some(CheckField::Off { part: CheckOffPart::ALL.into_iter().find(|p| p.name() == part)?, state }),
            "on" => Some(CheckField::On { part: CheckOnPart::ALL.into_iter().find(|p| p.name() == part)?, state }),
            _ => None,
        }
    }

    fn kind(f: CheckField) -> FieldKind {
        match f {
            CheckField::Radius => FieldKind::Length,
            CheckField::Off { .. } | CheckField::On { .. } | CheckField::Thumb { .. } => FieldKind::Color,
        }
    }

    fn get(&self, f: CheckField) -> OverrideValue {
        match f {
            CheckField::Radius => OverrideValue::Length(self.radius),
            CheckField::Off { part, state } => OverrideValue::Color(self.off.part(part).get(state)),
            CheckField::On { part, state } => OverrideValue::Color(self.on.part(part).get(state)),
            CheckField::Thumb { part, state } => OverrideValue::Color(self.thumb.part(part).get(state)),
        }
    }

    fn set(&mut self, f: CheckField, v: OverrideValue) {
        match (f, v) {
            (CheckField::Radius, OverrideValue::Length(r)) => self.radius = r.min(values::CHECK_RADIUS_MAX),
            (CheckField::Off { part, state }, OverrideValue::Color(c)) => *self.off.part_mut(part).get_mut(state) = c,
            (CheckField::On { part, state }, OverrideValue::Color(c)) => *self.on.part_mut(part).get_mut(state) = c,
            (CheckField::Thumb { part, state }, OverrideValue::Color(c)) => *self.thumb.part_mut(part).get_mut(state) = c,
            // Mismatched kinds are rejected before this is called.
            _ => {}
        }
    }

    /// At rest only. The on fill comes first: high contrast moves it before the mark drawn on it.
    /// A box is identifiable by its fill or its outline, on or off.
    fn pairs() -> Vec<ContrastPair> {
        let field = |f| TokenRef::Component(ComponentField::Check(f));
        let on = |part| field(CheckField::On { part, state: State::Rest });
        let off = |part| field(CheckField::Off { part, state: State::Rest });
        let backgrounds = [ColorToken::SurfacePanel, ColorToken::SurfaceRaised].map(TokenRef::Color);
        let min = values::NON_TEXT_MIN;
        let mut out: Vec<_> =
            backgrounds.map(|bg| ContrastPair { fg: on(CheckOnPart::Fill), bg, minimum: min, alternate: Some(on(CheckOnPart::Border)) }).into();
        out.extend(backgrounds.map(|bg| ContrastPair { fg: off(CheckOffPart::Border), bg, minimum: min, alternate: Some(off(CheckOffPart::Fill)) }));
        out.push(ContrastPair { fg: on(CheckOnPart::Mark), bg: on(CheckOnPart::Fill), minimum: min, alternate: None });
        out
    }
}
