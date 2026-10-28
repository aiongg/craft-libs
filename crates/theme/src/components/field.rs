//! Text fields: a sunken box holding one line of editable text, with an optional icon at each
//! end. The box takes its edge from the `Surface` primitive (a border, or a bevel in Classic);
//! this family holds its sizes, corners and the colours of the box, the text and the selection.
//!
//! The text, the placeholder, a number's unit and its scrub label are text on the field's fill
//! (4.5:1, 7:1 with high contrast), and so is the selected text on the selection. The focused and the invalid edge are the cues
//! that show the field's state (3:1, 4.5:1 with high contrast) on the field's fill. The resting
//! and hovered edges are design targets with no minimum. Each pair is measured at rest.

use super::{BySize, ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::tokens::{TokenRef, TypeRole};
use crate::values;

/// A part of a field that has one colour while enabled and one while disabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FieldPart {
    /// The inside of the box.
    Fill,
    /// The typed text.
    Text,
    /// The text shown while the field is empty.
    Placeholder,
    /// A number's unit, drawn after it.
    Unit,
    /// The label or icon at a number field's start that a drag scrubs the value with.
    Scrub,
}

impl FieldPart {
    /// Every part, in path order.
    pub const ALL: [FieldPart; 5] = [FieldPart::Fill, FieldPart::Text, FieldPart::Placeholder, FieldPart::Unit, FieldPart::Scrub];

    /// The path segment, e.g. `"placeholder"`.
    pub const fn name(self) -> &'static str {
        match self {
            FieldPart::Fill => "fill",
            FieldPart::Text => "text",
            FieldPart::Placeholder => "placeholder",
            FieldPart::Unit => "unit",
            FieldPart::Scrub => "scrub",
        }
    }
}

/// The state of a field's edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BorderState {
    /// Idle.
    Rest,
    /// Under the pointer.
    Hover,
    /// With keyboard focus, however it got there.
    Focused,
    /// Not interactive.
    Disabled,
}

impl BorderState {
    /// Every state, in path order.
    pub const ALL: [BorderState; 4] = [BorderState::Rest, BorderState::Hover, BorderState::Focused, BorderState::Disabled];

    /// The path segment, e.g. `"focused"`.
    pub const fn name(self) -> &'static str {
        match self {
            BorderState::Rest => "rest",
            BorderState::Hover => "hover",
            BorderState::Focused => "focused",
            BorderState::Disabled => "disabled",
        }
    }
}

/// A colour while enabled and while disabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Dimmed {
    /// Enabled, in every state.
    pub rest: Rgba,
    /// Disabled.
    pub disabled: Rgba,
}

impl Dimmed {
    /// The colour while enabled or not.
    pub fn get(&self, enabled: bool) -> Rgba {
        if enabled { self.rest } else { self.disabled }
    }

    fn get_mut(&mut self, disabled: bool) -> &mut Rgba {
        if disabled { &mut self.disabled } else { &mut self.rest }
    }
}

/// The colours of a field's edge, per [`BorderState`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BorderColors {
    /// Idle.
    pub rest: Rgba,
    /// Under the pointer.
    pub hover: Rgba,
    /// With keyboard focus.
    pub focused: Rgba,
    /// Not interactive.
    pub disabled: Rgba,
}

impl BorderColors {
    /// The colour in `state`.
    pub fn get(&self, state: BorderState) -> Rgba {
        match state {
            BorderState::Rest => self.rest,
            BorderState::Hover => self.hover,
            BorderState::Focused => self.focused,
            BorderState::Disabled => self.disabled,
        }
    }

    fn get_mut(&mut self, state: BorderState) -> &mut Rgba {
        match state {
            BorderState::Rest => &mut self.rest,
            BorderState::Hover => &mut self.hover,
            BorderState::Focused => &mut self.focused,
            BorderState::Disabled => &mut self.disabled,
        }
    }
}

/// Text-field tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FieldTokens {
    /// Height of the box: the control height.
    pub height: BySize<f32>,
    /// Default width: [`values::FIELD_WIDTH`] control heights. A justified layout stretches it.
    pub width: BySize<f32>,
    /// A number field's default width: [`values::NUMBER_FIELD_WIDTH`] control heights. A justified
    /// layout stretches it.
    pub number_width: BySize<f32>,
    /// Between the box's edge and its text or icon: `space.pad.control`.
    pub pad_x: f32,
    /// Icon size: the icon size of the control size.
    pub icon: BySize<f32>,
    /// Between an icon and the text: `space.gap.inline`.
    pub gap: f32,
    /// Corner radius of the box: `shape.radius.control`.
    pub radius: f32,
    /// Type role of the text and the placeholder.
    pub role: TypeRole,
    /// The box: `surface.sunken`; disabled, `state.disabled_fill`.
    pub fill: Dimmed,
    /// The edge: `line.control_edge`, `text.faint` under the pointer, `line.focus` while focused,
    /// `line.divider` disabled. A bevel keeps its own colours.
    pub border: BorderColors,
    /// The typed text: `text.normal`; disabled, `text.disabled`.
    pub text: Dimmed,
    /// The text shown while empty: `text.faint`; disabled, `text.disabled`.
    pub placeholder: Dimmed,
    /// A number's unit: `text.muted`; disabled, `text.disabled`.
    pub unit: Dimmed,
    /// A number field's scrub label or icon: `text.muted`; disabled, `text.disabled`.
    pub scrub: Dimmed,
    /// Behind selected text: `state.selected`.
    pub selection: Rgba,
    /// Selected text: `text.on_selected`.
    pub selected_text: Rgba,
    /// The text cursor: `text.normal`.
    pub caret: Rgba,
    /// The edge of a field whose value is invalid: `status.bad.fg`.
    pub invalid: Rgba,
}

impl FieldTokens {
    /// The colours of `part`.
    pub fn part(&self, part: FieldPart) -> &Dimmed {
        match part {
            FieldPart::Fill => &self.fill,
            FieldPart::Text => &self.text,
            FieldPart::Placeholder => &self.placeholder,
            FieldPart::Unit => &self.unit,
            FieldPart::Scrub => &self.scrub,
        }
    }

    fn part_mut(&mut self, part: FieldPart) -> &mut Dimmed {
        match part {
            FieldPart::Fill => &mut self.fill,
            FieldPart::Text => &mut self.text,
            FieldPart::Placeholder => &mut self.placeholder,
            FieldPart::Unit => &mut self.unit,
            FieldPart::Scrub => &mut self.scrub,
        }
    }

    fn color_mut(&mut self, f: FieldField) -> Option<&mut Rgba> {
        match f {
            FieldField::Radius => None,
            FieldField::Color { part, disabled } => Some(self.part_mut(part).get_mut(disabled)),
            FieldField::Border(state) => Some(self.border.get_mut(state)),
            FieldField::Selection => Some(&mut self.selection),
            FieldField::SelectedText => Some(&mut self.selected_text),
            FieldField::Caret => Some(&mut self.caret),
            FieldField::Invalid => Some(&mut self.invalid),
        }
    }
}

/// An overridable text-field token. Paths: `radius`,
/// `<fill|text|placeholder|unit|scrub>.<rest|disabled>`,
/// `border.<state>`, `selection.fill`, `selection.text`, `caret`, `invalid`, e.g.
/// `field.border.focused`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FieldField {
    /// `field.radius`
    Radius,
    /// `field.<part>.rest`, or `field.<part>.disabled` when `disabled`.
    Color {
        /// The part.
        part: FieldPart,
        /// The disabled colour rather than the enabled one.
        disabled: bool,
    },
    /// `field.border.<state>`
    Border(BorderState),
    /// `field.selection.fill`
    Selection,
    /// `field.selection.text`
    SelectedText,
    /// `field.caret`
    Caret,
    /// `field.invalid`
    Invalid,
}

const FIELD_COUNT: usize = 1 + FieldPart::ALL.len() * 2 + BorderState::ALL.len() + 4;

const FIELDS: [FieldField; FIELD_COUNT] = {
    let mut out = [FieldField::Radius; FIELD_COUNT];
    let mut i = 0;
    while i < FieldPart::ALL.len() * 2 {
        out[1 + i] = FieldField::Color { part: FieldPart::ALL[i / 2], disabled: i % 2 == 1 };
        i += 1;
    }
    let mut j = 0;
    while j < BorderState::ALL.len() {
        out[1 + FieldPart::ALL.len() * 2 + j] = FieldField::Border(BorderState::ALL[j]);
        j += 1;
    }
    let rest = FIELD_COUNT - 4;
    out[rest] = FieldField::Selection;
    out[rest + 1] = FieldField::SelectedText;
    out[rest + 2] = FieldField::Caret;
    out[rest + 3] = FieldField::Invalid;
    out
};

const fn enablement(disabled: bool) -> &'static str {
    if disabled { "disabled" } else { "rest" }
}

impl Family for FieldTokens {
    type Field = FieldField;
    const FIELDS: &'static [FieldField] = &FIELDS;

    fn defaults(s: &Semantic<'_>) -> FieldTokens {
        let (c, size) = (s.color, s.size);
        FieldTokens {
            height: BySize { sm: size.control.sm, md: size.control.md },
            width: BySize { sm: values::FIELD_WIDTH * size.control.sm, md: values::FIELD_WIDTH * size.control.md },
            number_width: BySize { sm: values::NUMBER_FIELD_WIDTH * size.control.sm, md: values::NUMBER_FIELD_WIDTH * size.control.md },
            pad_x: s.space.pad.control,
            icon: BySize { sm: size.icon.sm, md: size.icon.md },
            gap: s.space.gap.inline,
            radius: s.shape.radius.control,
            role: TypeRole::Body,
            fill: Dimmed { rest: c.surface.sunken, disabled: c.state.disabled_fill },
            // `text.faint` is the next neutral above `line.control_edge` that isn't `text.disabled`.
            border: BorderColors { rest: c.line.control_edge, hover: c.text.faint, focused: c.line.focus, disabled: c.line.divider },
            text: Dimmed { rest: c.text.normal, disabled: c.text.disabled },
            placeholder: Dimmed { rest: c.text.faint, disabled: c.text.disabled },
            unit: Dimmed { rest: c.text.muted, disabled: c.text.disabled },
            scrub: Dimmed { rest: c.text.muted, disabled: c.text.disabled },
            selection: c.state.selected,
            selected_text: c.text.on_selected,
            caret: c.text.normal,
            invalid: c.status.bad.fg,
        }
    }

    fn path(f: FieldField) -> String {
        match f {
            FieldField::Radius => "radius".to_owned(),
            FieldField::Color { part, disabled } => format!("{}.{}", part.name(), enablement(disabled)),
            FieldField::Border(state) => format!("border.{}", state.name()),
            FieldField::Selection => "selection.fill".to_owned(),
            FieldField::SelectedText => "selection.text".to_owned(),
            FieldField::Caret => "caret".to_owned(),
            FieldField::Invalid => "invalid".to_owned(),
        }
    }

    fn from_path(path: &str) -> Option<FieldField> {
        FIELDS.iter().copied().find(|&f| Self::path(f) == path)
    }

    fn kind(f: FieldField) -> FieldKind {
        match f {
            FieldField::Radius => FieldKind::Length,
            _ => FieldKind::Color,
        }
    }

    fn get(&self, f: FieldField) -> OverrideValue {
        match f {
            FieldField::Radius => OverrideValue::Length(self.radius),
            FieldField::Color { part, disabled } => OverrideValue::Color(self.part(part).get(!disabled)),
            FieldField::Border(state) => OverrideValue::Color(self.border.get(state)),
            FieldField::Selection => OverrideValue::Color(self.selection),
            FieldField::SelectedText => OverrideValue::Color(self.selected_text),
            FieldField::Caret => OverrideValue::Color(self.caret),
            FieldField::Invalid => OverrideValue::Color(self.invalid),
        }
    }

    fn set(&mut self, f: FieldField, v: OverrideValue) {
        match (f, v) {
            (FieldField::Radius, OverrideValue::Length(r)) => self.radius = r,
            (f, OverrideValue::Color(c)) => {
                if let Some(slot) = self.color_mut(f) {
                    *slot = c;
                }
            }
            // Mismatched kinds are rejected before this is called.
            _ => {}
        }
    }

    /// At rest, on the field's fill: the text, the placeholder, the unit and the scrub label
    /// (text), the focused and invalid edges (state cues); and the selected text on the selection.
    fn pairs() -> Vec<ContrastPair> {
        let field = ComponentField::Field;
        let fill = TokenRef::Component(field(FieldField::Color { part: FieldPart::Fill, disabled: false }));
        let pair = |fg, bg, minimum| ContrastPair { fg: field(fg), bg, minimum };
        vec![
            pair(FieldField::Color { part: FieldPart::Text, disabled: false }, fill, values::TEXT_MIN),
            pair(FieldField::Color { part: FieldPart::Placeholder, disabled: false }, fill, values::TEXT_MIN),
            pair(FieldField::Color { part: FieldPart::Unit, disabled: false }, fill, values::TEXT_MIN),
            pair(FieldField::Color { part: FieldPart::Scrub, disabled: false }, fill, values::TEXT_MIN),
            pair(FieldField::Border(BorderState::Focused), fill, values::NON_TEXT_MIN),
            pair(FieldField::Invalid, fill, values::NON_TEXT_MIN),
            pair(FieldField::SelectedText, TokenRef::Component(field(FieldField::Selection)), values::TEXT_MIN),
        ]
    }
}
