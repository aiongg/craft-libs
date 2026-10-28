//! Check boxes, radio buttons and switches. Each has an indicator (a box, a circle, a track) that
//! takes its edge from the `Surface` primitive's sunken surface. A check box or radio button that
//! is on shows a mark (the check or dash, the radio dot); a switch shows its state by its track
//! and where its thumb sits, a knob that is the same while off and on. This family holds their
//! sizes and the colours of the indicator while off and while on (checked, mixed, selected), of
//! the mark, and of the switch thumb.
//!
//! The mark, the on fill and the off outline are the cues that identify the control and its
//! state, so they have the WCAG non-text minimum (3:1, 4.5:1 with high contrast) at rest: the
//! mark against the on fill, and the off outline and the on fill against the control backgrounds
//! (`surface.panel` and `surface.raised`). Hover and pressed colours have no minimum, and neither
//! has the switch thumb: where its face is close to the track, its outline or shadow gives the
//! edge.
//!
//! A disabled control that is on keeps an on look: a `text.disabled` fill with a
//! `state.disabled_fill` mark, the reverse of off's `state.disabled_fill` fill with a
//! `text.disabled` outline.

use super::{
    BySize, ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic, State, States, THUMB_FIELDS, ThumbColors, ThumbPart,
    thumb_from_path, thumb_path,
};
use crate::color::Rgba;
use crate::tokens::{ColorToken, TokenRef, TypeRole};
use crate::values;

/// Whether a check box, radio button or switch is on (checked, mixed or selected) or off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OnOff {
    /// Unchecked, not selected.
    Off,
    /// Checked, mixed or selected.
    On,
}

impl OnOff {
    /// Both, in path order.
    pub const ALL: [OnOff; 2] = [OnOff::Off, OnOff::On];

    /// The path segment, e.g. `"on"`.
    pub const fn name(self) -> &'static str {
        match self {
            OnOff::Off => "off",
            OnOff::On => "on",
        }
    }
}

/// One coloured part of a check box, radio button or switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CheckPart {
    /// The inside of the box, circle or track.
    Fill,
    /// The outline of the box, circle or track, over a flat surface's border.
    Border,
    /// The check or dash, the radio dot; only while on.
    Mark,
}

impl CheckPart {
    /// Every part, in path order.
    pub const ALL: [CheckPart; 3] = [CheckPart::Fill, CheckPart::Border, CheckPart::Mark];

    /// The parts with a colour while `on_off`: off has no mark.
    pub const fn of(on_off: OnOff) -> &'static [CheckPart] {
        match on_off {
            OnOff::Off => &[CheckPart::Fill, CheckPart::Border],
            OnOff::On => &CheckPart::ALL,
        }
    }

    /// The path segment, e.g. `"mark"`.
    pub const fn name(self) -> &'static str {
        match self {
            CheckPart::Fill => "fill",
            CheckPart::Border => "border",
            CheckPart::Mark => "mark",
        }
    }
}

/// The colours of the indicator and mark while off or while on, per state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CheckColors {
    /// Inside of the box, circle or track.
    pub fill: States<Rgba>,
    /// Outline of the box, circle or track.
    pub border: States<Rgba>,
    /// Check, dash or dot. Off has no mark: clear, and no path.
    pub mark: States<Rgba>,
}

impl CheckColors {
    /// The colours of `part`.
    pub fn part(&self, part: CheckPart) -> &States<Rgba> {
        match part {
            CheckPart::Fill => &self.fill,
            CheckPart::Border => &self.border,
            CheckPart::Mark => &self.mark,
        }
    }

    fn part_mut(&mut self, part: CheckPart) -> &mut States<Rgba> {
        match part {
            CheckPart::Fill => &mut self.fill,
            CheckPart::Border => &mut self.border,
            CheckPart::Mark => &mut self.mark,
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
    /// Off: `text.faint` outline on `surface.sunken`.
    pub off: CheckColors,
    /// On: `accent.*` fill and outline, a `text.on_accent` mark; disabled, a `text.disabled` fill
    /// and outline with a `state.disabled_fill` mark.
    pub on: CheckColors,
    /// The switch thumb, off and on: a knob, a `surface.knob` face with a `line.knob_edge`
    /// outline in every enabled state; disabled, a face halfway to `surface.panel` with a
    /// `line.divider` outline.
    pub thumb: ThumbColors,
}

impl CheckTokens {
    /// The colours while `on_off`.
    pub fn colors(&self, on_off: OnOff) -> &CheckColors {
        match on_off {
            OnOff::Off => &self.off,
            OnOff::On => &self.on,
        }
    }

    fn colors_mut(&mut self, on_off: OnOff) -> &mut CheckColors {
        match on_off {
            OnOff::Off => &mut self.off,
            OnOff::On => &mut self.on,
        }
    }
}

/// An overridable check field. Paths: `radius`, `<on|off>.<part>.<state>` (no `off.mark`),
/// `thumb.<part>.<state>`, e.g. `check.on.mark.rest`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CheckField {
    /// `check.radius`
    Radius,
    /// `check.<on|off>.<part>.<state>`.
    Color {
        /// On or off.
        on_off: OnOff,
        /// The part.
        part: CheckPart,
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

const COLOR_COUNT: usize = (CheckPart::of(OnOff::Off).len() + CheckPart::of(OnOff::On).len()) * State::ALL.len();
const FIELD_COUNT: usize = 1 + COLOR_COUNT + THUMB_FIELDS.len();

const FIELDS: [CheckField; FIELD_COUNT] = {
    let mut out = [CheckField::Radius; FIELD_COUNT];
    let mut i = 1;
    let mut set = 0;
    while set < OnOff::ALL.len() {
        let parts = CheckPart::of(OnOff::ALL[set]);
        let mut j = 0;
        while j < parts.len() * State::ALL.len() {
            out[i] = CheckField::Color { on_off: OnOff::ALL[set], part: parts[j / State::ALL.len()], state: State::ALL[j % State::ALL.len()] };
            i += 1;
            j += 1;
        }
        set += 1;
    }
    let mut k = 0;
    while k < THUMB_FIELDS.len() {
        let (part, state) = THUMB_FIELDS[k];
        out[i + k] = CheckField::Thumb { part, state };
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
        // meets the non-text minimum wherever the control sits.
        let outline = States::new(c.text.faint, c.text.muted, c.text.muted, c.text.disabled);
        let sunken = c.surface.sunken;
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
            off: CheckColors {
                fill: States::new(sunken, sunken, c.state.pressed, c.state.disabled_fill),
                border: outline,
                mark: States::new(Rgba::TRANSPARENT, Rgba::TRANSPARENT, Rgba::TRANSPARENT, Rgba::TRANSPARENT),
            },
            on: CheckColors { fill: accent, border: accent, mark: States::new(on_accent, on_accent, on_accent, c.state.disabled_fill) },
            thumb: ThumbColors::knob(c),
        }
    }

    fn path(f: CheckField) -> String {
        match f {
            CheckField::Radius => "radius".to_owned(),
            CheckField::Color { on_off, part, state } => format!("{}.{}.{}", on_off.name(), part.name(), state.name()),
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
        let on_off = OnOff::ALL.into_iter().find(|o| o.name() == on_off)?;
        let part = CheckPart::of(on_off).iter().copied().find(|p| p.name() == part)?;
        Some(CheckField::Color { on_off, part, state: State::from_name(state)? })
    }

    fn kind(f: CheckField) -> FieldKind {
        match f {
            CheckField::Radius => FieldKind::Length,
            CheckField::Color { .. } | CheckField::Thumb { .. } => FieldKind::Color,
        }
    }

    fn get(&self, f: CheckField) -> OverrideValue {
        match f {
            CheckField::Radius => OverrideValue::Length(self.radius),
            CheckField::Color { on_off, part, state } => OverrideValue::Color(self.colors(on_off).part(part).get(state)),
            CheckField::Thumb { part, state } => OverrideValue::Color(self.thumb.part(part).get(state)),
        }
    }

    fn set(&mut self, f: CheckField, v: OverrideValue) {
        match (f, v) {
            (CheckField::Radius, OverrideValue::Length(r)) => self.radius = r.min(values::CHECK_RADIUS_MAX),
            (CheckField::Color { on_off, part, state }, OverrideValue::Color(c)) if CheckPart::of(on_off).contains(&part) => {
                *self.colors_mut(on_off).part_mut(part).get_mut(state) = c
            }
            (CheckField::Thumb { part, state }, OverrideValue::Color(c)) => *self.thumb.part_mut(part).get_mut(state) = c,
            // Mismatched kinds are rejected before this is called.
            _ => {}
        }
    }

    /// At rest only. The on fill comes first: high contrast moves it before the mark drawn on it.
    fn pairs() -> Vec<ContrastPair> {
        let field = |on_off, part| ComponentField::Check(CheckField::Color { on_off, part, state: State::Rest });
        let backgrounds = [ColorToken::SurfacePanel, ColorToken::SurfaceRaised].map(TokenRef::Color);
        let pair = |fg, bg| ContrastPair { fg, bg, minimum: values::NON_TEXT_MIN };
        let mut out: Vec<_> = backgrounds.map(|bg| pair(field(OnOff::On, CheckPart::Fill), bg)).into();
        out.extend(backgrounds.map(|bg| pair(field(OnOff::Off, CheckPart::Border), bg)));
        out.push(pair(field(OnOff::On, CheckPart::Mark), TokenRef::Component(field(OnOff::On, CheckPart::Fill))));
        out
    }
}
