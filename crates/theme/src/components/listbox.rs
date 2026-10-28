//! List boxes: the rows of a select's or combo box's popup list, which later menus and lists can
//! share. A row shows an option's text, with a check mark at its reading start when it is the
//! chosen option; the active row (under the pointer, or moved to with the keys) has a fill. Group
//! headers and separators sit between rows. The list draws on the popup surface
//! (`surface.raised`).
//!
//! The text, the chosen text and the group headers are text on the popup surface (4.5:1, 7:1 with
//! high contrast), and the active text is text on the active fill. The check mark is the cue that
//! shows the chosen option (3:1, 4.5:1 with high contrast) on the popup surface and on the active
//! fill. The active fill and the separator are design targets with no minimum. Each pair is
//! measured at rest.

use super::{BySize, ComponentField, ContrastPair, Dimmed, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::tokens::{ColorToken, TokenRef, TypeRole};
use crate::values;

/// List box tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ListboxTokens {
    /// Height of a row and of a group header: the control height.
    pub height: BySize<f32>,
    /// Between a row's edge and its contents: `space.pad.control`.
    pub pad_x: f32,
    /// Between the check mark, an icon and the text: `space.gap.inline`.
    pub gap: f32,
    /// Side of the check mark and of an option's icon: the icon size of the control size.
    pub icon: BySize<f32>,
    /// Type role of an option's text.
    pub label: TypeRole,
    /// Type role of a group header.
    pub group_label: TypeRole,
    /// Above and below a separator's line: `space.gap.inline`.
    pub separator_pad: f32,
    /// Corner radius of the active row's fill: `shape.radius.control`.
    pub radius: f32,
    /// An option's text: `text.normal`; disabled, `text.disabled`.
    pub text: Dimmed,
    /// Behind the active row: `state.hover`.
    pub active_fill: Rgba,
    /// The active row's text and check mark: `text.normal`.
    pub active_text: Rgba,
    /// The chosen option's text: `text.normal`.
    pub chosen_text: Rgba,
    /// The check mark at the chosen option: `text.normal`.
    pub mark: Rgba,
    /// A group header: `text.muted`.
    pub group: Rgba,
    /// A separator's line: `line.divider`.
    pub separator: Rgba,
}

impl ListboxTokens {
    fn color_mut(&mut self, f: ListboxField) -> Option<&mut Rgba> {
        Some(match f {
            ListboxField::Radius => return None,
            ListboxField::Text { disabled: false } => &mut self.text.rest,
            ListboxField::Text { disabled: true } => &mut self.text.disabled,
            ListboxField::ActiveFill => &mut self.active_fill,
            ListboxField::ActiveText => &mut self.active_text,
            ListboxField::ChosenText => &mut self.chosen_text,
            ListboxField::Mark => &mut self.mark,
            ListboxField::Group => &mut self.group,
            ListboxField::Separator => &mut self.separator,
        })
    }
}

/// An overridable list box token. Paths: `radius`, `text.rest`, `text.disabled`, `active.fill`,
/// `active.text`, `chosen.text`, `mark`, `group`, `separator`, e.g. `listbox.active.fill`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ListboxField {
    /// `listbox.radius`
    Radius,
    /// `listbox.text.rest`, or `listbox.text.disabled` when `disabled`.
    Text {
        /// The disabled colour rather than the enabled one.
        disabled: bool,
    },
    /// `listbox.active.fill`
    ActiveFill,
    /// `listbox.active.text`
    ActiveText,
    /// `listbox.chosen.text`
    ChosenText,
    /// `listbox.mark`
    Mark,
    /// `listbox.group`
    Group,
    /// `listbox.separator`
    Separator,
}

impl Family for ListboxTokens {
    type Field = ListboxField;
    const FIELDS: &'static [ListboxField] = &[
        ListboxField::Radius,
        ListboxField::Text { disabled: false },
        ListboxField::Text { disabled: true },
        ListboxField::ActiveFill,
        ListboxField::ActiveText,
        ListboxField::ChosenText,
        ListboxField::Mark,
        ListboxField::Group,
        ListboxField::Separator,
    ];

    fn defaults(s: &Semantic<'_>) -> ListboxTokens {
        let (c, size) = (s.color, s.size);
        ListboxTokens {
            height: BySize { sm: size.control.sm, md: size.control.md },
            pad_x: s.space.pad.control,
            gap: s.space.gap.inline,
            icon: BySize { sm: size.icon.sm, md: size.icon.md },
            label: TypeRole::Body,
            group_label: TypeRole::Caption,
            separator_pad: s.space.gap.inline,
            radius: s.shape.radius.control,
            text: Dimmed { rest: c.text.normal, disabled: c.text.disabled },
            active_fill: c.state.hover,
            active_text: c.text.normal,
            chosen_text: c.text.normal,
            mark: c.text.normal,
            group: c.text.muted,
            separator: c.line.divider,
        }
    }

    fn path(f: ListboxField) -> String {
        match f {
            ListboxField::Radius => "radius",
            ListboxField::Text { disabled: false } => "text.rest",
            ListboxField::Text { disabled: true } => "text.disabled",
            ListboxField::ActiveFill => "active.fill",
            ListboxField::ActiveText => "active.text",
            ListboxField::ChosenText => "chosen.text",
            ListboxField::Mark => "mark",
            ListboxField::Group => "group",
            ListboxField::Separator => "separator",
        }
        .to_owned()
    }

    fn from_path(path: &str) -> Option<ListboxField> {
        Self::FIELDS.iter().copied().find(|&f| Self::path(f) == path)
    }

    fn kind(f: ListboxField) -> FieldKind {
        match f {
            ListboxField::Radius => FieldKind::Length,
            _ => FieldKind::Color,
        }
    }

    fn get(&self, f: ListboxField) -> OverrideValue {
        match f {
            ListboxField::Radius => OverrideValue::Length(self.radius),
            ListboxField::Text { disabled } => OverrideValue::Color(self.text.get(!disabled)),
            ListboxField::ActiveFill => OverrideValue::Color(self.active_fill),
            ListboxField::ActiveText => OverrideValue::Color(self.active_text),
            ListboxField::ChosenText => OverrideValue::Color(self.chosen_text),
            ListboxField::Mark => OverrideValue::Color(self.mark),
            ListboxField::Group => OverrideValue::Color(self.group),
            ListboxField::Separator => OverrideValue::Color(self.separator),
        }
    }

    fn set(&mut self, f: ListboxField, v: OverrideValue) {
        match (f, v) {
            (ListboxField::Radius, OverrideValue::Length(r)) => self.radius = r,
            (f, OverrideValue::Color(c)) => {
                if let Some(slot) = self.color_mut(f) {
                    *slot = c;
                }
            }
            // Mismatched kinds are rejected before this is called.
            _ => {}
        }
    }

    /// At rest: the text, the chosen text and the group header on the popup surface, and the
    /// active text on the active fill (text); the check mark on the popup surface and on the
    /// active fill (the cue of the chosen option).
    fn pairs() -> Vec<ContrastPair> {
        let field = ComponentField::Listbox;
        let popup = TokenRef::Color(ColorToken::SurfaceRaised);
        let active = TokenRef::Component(field(ListboxField::ActiveFill));
        let pair = |fg, bg, minimum| ContrastPair { fg: field(fg), bg, minimum };
        vec![
            pair(ListboxField::Text { disabled: false }, popup, values::TEXT_MIN),
            pair(ListboxField::ChosenText, popup, values::TEXT_MIN),
            pair(ListboxField::Group, popup, values::TEXT_MIN),
            pair(ListboxField::ActiveText, active, values::TEXT_MIN),
            pair(ListboxField::Mark, popup, values::NON_TEXT_MIN),
            pair(ListboxField::Mark, active, values::NON_TEXT_MIN),
        ]
    }
}
