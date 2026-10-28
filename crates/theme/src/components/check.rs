//! Check boxes, radio buttons and switches. Each has an indicator (a box, a circle, a track) that
//! takes its edge from the `Surface` primitive's sunken surface, and a mark that shows the state:
//! the check or dash, the radio dot, the switch thumb. This family holds their sizes and the
//! colours of the indicator and mark while off and while on (checked, mixed, selected).
//!
//! The mark and the off outline are the cues that identify the control and its state, so they
//! have the WCAG non-text minimum (3:1, 4.5:1 with high contrast) at rest: the mark against the
//! indicator's fill, and the off outline and the on fill against the control backgrounds
//! (`surface.panel` and `surface.raised`). Hover and pressed colours have no minimum.
//!
//! A disabled control that is on keeps an on look: a `text.disabled` fill with a
//! `state.disabled_fill` mark, the reverse of off's `state.disabled_fill` fill with a
//! `text.disabled` outline and thumb.

use super::{BySize, ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic, State, States};
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
    /// The check or dash, the radio dot, the switch thumb.
    Mark,
}

impl CheckPart {
    /// Every part, in path order.
    pub const ALL: [CheckPart; 3] = [CheckPart::Fill, CheckPart::Border, CheckPart::Mark];

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
    /// Check, dash, dot or thumb. Off, only a switch draws one (its thumb).
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
    /// Off: `text.faint` outline on `surface.sunken`; a switch's thumb in `text.faint`.
    pub off: CheckColors,
    /// On: `accent.*` fill and outline, a `text.on_accent` mark; disabled, a `text.disabled` fill
    /// and outline with a `state.disabled_fill` mark.
    pub on: CheckColors,
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

/// An overridable check field. Paths: `radius`, `<on|off>.<part>.<state>`, e.g.
/// `check.on.mark.rest`.
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
}

const FIELD_COUNT: usize = 1 + OnOff::ALL.len() * CheckPart::ALL.len() * State::ALL.len();

const FIELDS: [CheckField; FIELD_COUNT] = {
    let mut out = [CheckField::Radius; FIELD_COUNT];
    let per_set = CheckPart::ALL.len() * State::ALL.len();
    let mut i = 0;
    while i < FIELD_COUNT - 1 {
        let (set, rest) = (i / per_set, i % per_set);
        out[1 + i] =
            CheckField::Color { on_off: OnOff::ALL[set], part: CheckPart::ALL[rest / State::ALL.len()], state: State::ALL[rest % State::ALL.len()] };
        i += 1;
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
        // and the off thumb meet the non-text minimum wherever the control sits.
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
            off: CheckColors { fill: States::new(sunken, sunken, c.state.pressed, c.state.disabled_fill), border: outline, mark: outline },
            on: CheckColors { fill: accent, border: accent, mark: States::new(on_accent, on_accent, on_accent, c.state.disabled_fill) },
        }
    }

    fn path(f: CheckField) -> String {
        match f {
            CheckField::Radius => "radius".to_owned(),
            CheckField::Color { on_off, part, state } => format!("{}.{}.{}", on_off.name(), part.name(), state.name()),
        }
    }

    fn from_path(path: &str) -> Option<CheckField> {
        if path == "radius" {
            return Some(CheckField::Radius);
        }
        let mut segments = path.split('.');
        let (on_off, part, state) = (segments.next()?, segments.next()?, segments.next()?);
        if segments.next().is_some() {
            return None;
        }
        Some(CheckField::Color {
            on_off: OnOff::ALL.into_iter().find(|o| o.name() == on_off)?,
            part: CheckPart::ALL.into_iter().find(|p| p.name() == part)?,
            state: State::from_name(state)?,
        })
    }

    fn kind(f: CheckField) -> FieldKind {
        match f {
            CheckField::Radius => FieldKind::Length,
            CheckField::Color { .. } => FieldKind::Color,
        }
    }

    fn get(&self, f: CheckField) -> OverrideValue {
        match f {
            CheckField::Radius => OverrideValue::Length(self.radius),
            CheckField::Color { on_off, part, state } => OverrideValue::Color(self.colors(on_off).part(part).get(state)),
        }
    }

    fn set(&mut self, f: CheckField, v: OverrideValue) {
        match (f, v) {
            (CheckField::Radius, OverrideValue::Length(r)) => self.radius = r.min(values::CHECK_RADIUS_MAX),
            (CheckField::Color { on_off, part, state }, OverrideValue::Color(c)) => *self.colors_mut(on_off).part_mut(part).get_mut(state) = c,
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
        out.extend(OnOff::ALL.map(|on_off| pair(field(on_off, CheckPart::Mark), TokenRef::Component(field(on_off, CheckPart::Fill)))));
        out
    }
}
