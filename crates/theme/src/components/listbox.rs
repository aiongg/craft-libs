//! List boxes: the rows of a select's or combo box's popup list, which menus share. A row shows
//! an option's text, with a check mark at its reading start when it is the chosen option. The
//! active row (under the pointer, or moved to with the keys) and the chosen row have the accent
//! fill with the text that sits on it, as PhotoCraft's dropdown and menu rows do. Group headers
//! and separators sit between rows. The list draws on the popup card (`popup.fill`).
//!
//! The text and the group headers are text on the popup card (4.5:1, 7:1 with high contrast), and
//! the active text is text on the active fill. The check mark of a menu row is the cue that shows
//! it is on (3:1, 4.5:1 with high contrast) on the popup card. A mark on the active or chosen row
//! follows that row's text colour, as a menu's marks follow its label, so it is measured with the
//! active text against the active fill, which is the stronger text minimum. The active fill must
//! be visibly different from the popup card (ratio 1.12, as a list row's hover fill); the
//! separator is a design target with no minimum. Each pair is measured at rest.

use super::{BySize, ComponentField, ContrastPair, Dimmed, Family, FieldKind, OverrideValue, PopupField, Semantic};
use crate::color::Rgba;
use crate::tokens::{TokenRef, TypeRole};
use crate::values;

/// List box tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ListboxTokens {
    /// Height of a row and of a group header: the control height.
    pub height: BySize<f32>,
    /// Between a row's edge and its contents: `space.pad.control` and [`values::LISTBOX_PAD_X`]
    /// more (PhotoCraft's 10 point button padding).
    pub pad_x: f32,
    /// The most a select's or combo box's list shows before it scrolls: [`values::LISTBOX_MAX_HEIGHT`]
    /// and, above Compact, the extra height of [`values::LISTBOX_MAX_ROWS`] rows.
    pub max_height: f32,
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
    /// Corner radius of the active row's fill: square (PhotoCraft's menu and dropdown rows).
    pub radius: f32,
    /// An option's text: `text.normal`; disabled, `text.disabled`.
    pub text: Dimmed,
    /// Behind the active row and the chosen option: `accent.solid`.
    pub active_fill: Rgba,
    /// The text and check mark of the active row and the chosen option: `text.on_accent`.
    pub active_text: Rgba,
    /// A menu row's check mark: `text.normal`; on the active row it is the active text.
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
            ListboxField::Mark => &mut self.mark,
            ListboxField::Group => &mut self.group,
            ListboxField::Separator => &mut self.separator,
        })
    }
}

/// An overridable list box token. Paths: `radius`, `text.rest`, `text.disabled`, `active.fill`,
/// `active.text`, `mark`, `group`, `separator`, e.g. `listbox.active.fill`.
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
        ListboxField::Mark,
        ListboxField::Group,
        ListboxField::Separator,
    ];

    fn defaults(s: &Semantic<'_>) -> ListboxTokens {
        let (c, size) = (s.color, s.size);
        ListboxTokens {
            height: BySize { sm: size.control.sm, md: size.control.md },
            pad_x: s.space.pad.control + values::LISTBOX_PAD_X,
            max_height: values::LISTBOX_MAX_HEIGHT + values::LISTBOX_MAX_ROWS * (size.control.md - values::MENU_BASE_CONTROL),
            gap: s.space.gap.inline,
            icon: BySize { sm: size.icon.sm, md: size.icon.md },
            label: TypeRole::Body,
            group_label: TypeRole::Caption,
            separator_pad: s.space.gap.inline,
            radius: 0.0,
            text: Dimmed { rest: c.text.normal, disabled: c.text.disabled },
            active_fill: c.accent.solid,
            active_text: c.text.on_accent,
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

    /// At rest: the text and the group header on the popup card, and the active text (which also
    /// colours the check mark on the active row) on the active fill (text); the check mark on the
    /// popup card (the cue of a checked menu row).
    fn pairs() -> Vec<ContrastPair> {
        let field = ComponentField::Listbox;
        let popup = TokenRef::Component(ComponentField::Popup(PopupField::Fill));
        let active = TokenRef::Component(field(ListboxField::ActiveFill));
        let pair = |fg, bg, minimum| ContrastPair { fg: TokenRef::Component(field(fg)), bg, minimum, alternate: None };
        vec![
            pair(ListboxField::ActiveFill, popup, values::LIST_HOVER_MIN),
            pair(ListboxField::Text { disabled: false }, popup, values::TEXT_MIN),
            pair(ListboxField::Group, popup, values::TEXT_MIN),
            pair(ListboxField::ActiveText, active, values::TEXT_MIN),
            pair(ListboxField::Mark, popup, values::NON_TEXT_MIN),
        ]
    }
}
