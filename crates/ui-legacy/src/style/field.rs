//! Style of the `field` family.

use craft_theme::{BorderColors, BySize, Dimmed, FieldPart, Rgba, Tokens};

/// Control heights in a text field's default width.
const WIDTH_IN_CONTROLS: f32 = 8.0;
/// Control heights in a number field's default width.
const NUMBER_WIDTH_IN_CONTROLS: f32 = 3.0;

/// Colours and default widths of text and number fields.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldStyle {
    /// Default width of a text field; a justified layout stretches it.
    pub width: BySize<f32>,
    /// Default width of a number field; a justified layout stretches it.
    pub number_width: BySize<f32>,
    /// The box: `surface.sunken`; disabled, `state.disabled_fill`.
    pub fill: Dimmed,
    /// The edge: `line.control_edge` at rest and hovered, `line.focus` focused, `line.divider` disabled.
    pub border: BorderColors,
    /// The typed text.
    pub text: Dimmed,
    /// The text shown while empty.
    pub placeholder: Dimmed,
    /// A number's unit.
    pub unit: Dimmed,
    /// A number field's scrub label or icon.
    pub scrub: Dimmed,
    /// Behind selected text.
    pub selection: Rgba,
    /// Selected text.
    pub selected_text: Rgba,
    /// The text cursor.
    pub caret: Rgba,
    /// The edge of a field whose value is invalid.
    pub invalid: Rgba,
}

impl FieldStyle {
    /// The style for `tokens`.
    pub fn new(tokens: &Tokens) -> Self {
        let (c, size) = (&tokens.color, &tokens.size);
        let dimmed = |rest: Rgba| Dimmed { rest, disabled: c.text.disabled };
        FieldStyle {
            width: BySize { sm: WIDTH_IN_CONTROLS * size.control.sm, md: WIDTH_IN_CONTROLS * size.control.md },
            number_width: BySize { sm: NUMBER_WIDTH_IN_CONTROLS * size.control.sm, md: NUMBER_WIDTH_IN_CONTROLS * size.control.md },
            fill: Dimmed { rest: c.surface.sunken, disabled: c.state.disabled_fill },
            border: BorderColors { rest: c.line.control_edge, hover: c.line.control_edge, focused: c.line.focus, disabled: c.line.divider },
            text: dimmed(c.text.normal),
            placeholder: dimmed(c.text.faint),
            unit: dimmed(c.text.faint),
            scrub: dimmed(c.text.muted),
            selection: c.accent.soft,
            selected_text: c.text.normal,
            caret: c.text.normal,
            invalid: c.status.bad.fg,
        }
    }

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
}
