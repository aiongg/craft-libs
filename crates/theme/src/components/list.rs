//! Lists: the rows of a list or tree panel (layers, history, channels). Sizes come from the Size
//! preset; the colours are the neutral state fills of the semantic tokens.
//!
//! The drop marker is the only cue that shows where a dragged row lands. It is drawn on its own
//! halo, as canvas overlays are, so it is measured against the halo alone (3:1, 4.5:1 with high
//! contrast) and reads on any row fill. Labels are text on the panel and on the fills a row takes
//! (4.5:1, 7:1 with high contrast). The fills themselves are design targets with no minimum. Each
//! pair is measured at rest.

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::tokens::{ColorToken, ThumbSizes, TokenRef, TypeRole};
use crate::values;

/// List and list row tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ListTokens {
    /// Height of a row without a thumbnail: the standard control height.
    pub row_height: f32,
    /// Thumbnail sides for the three user settings.
    pub thumb: ThumbSizes,
    /// Added to a thumbnail's side for the row's height (PhotoCraft: thumbnail plus 8), at least
    /// `thumb_row_min`.
    pub thumb_pad: f32,
    /// Least height of a row with a thumbnail.
    pub thumb_row_min: f32,
    /// Between a row's edge and its contents: `space.pad.control`.
    pub pad_x: f32,
    /// Between the parts of a row: `space.gap.inline`.
    pub gap: f32,
    /// Indent of one tree level.
    pub indent: f32,
    /// Width of the disclosure column.
    pub disclosure: f32,
    /// Width of one toggle column (eye, lock).
    pub toggle_column: f32,
    /// Least width of the label: optional trailing items drop out, and the indent shrinks, before
    /// the label gets narrower.
    pub min_label: f32,
    /// Room kept at the trailing edge for an overlay scroll bar.
    pub trailing_pad: f32,
    /// Corner radius of the state fills: 0 in Compact, `shape.radius.control` in Comfortable.
    pub radius: f32,
    /// A divider under each row and at each toggle column's trailing edge.
    pub dividers: bool,
    /// Width of the drop marker line and outline.
    pub drop_width: f32,
    /// Width of the halo on each side of the drop marker.
    pub drop_halo_width: f32,
    /// Type role of the label.
    pub label: TypeRole,
    /// Type role of the detail line under the label.
    pub detail_role: TypeRole,
    /// The label: `text.normal`.
    pub text: Rgba,
    /// The label of a dimmed row (a hidden layer): `text.faint`.
    pub dimmed: Rgba,
    /// The label of the selected row: `text.on_selected`.
    pub selected_text: Rgba,
    /// The detail line: `text.faint`.
    pub detail: Rgba,
    /// Icons and the disclosure triangle: `text.muted`.
    pub icon: Rgba,
    /// Icons and the triangle under the pointer: `text.normal`.
    pub icon_hover: Rgba,
    /// Behind a row under the pointer: `state.hover`.
    pub hover: Rgba,
    /// Behind the selected row: `state.selected`.
    pub selected: Rgba,
    /// Behind the selected row under the pointer: `state.selected_hover`.
    pub selected_hover: Rgba,
    /// Dividers: `line.divider`.
    pub divider: Rgba,
    /// The frame of a thumbnail: `line.border`.
    pub thumb_border: Rgba,
    /// The drop marker: `accent.solid`.
    pub drop: Rgba,
    /// The outline under the drop marker that separates it from the row fills: `surface.panel`.
    pub drop_halo: Rgba,
    /// The drag ghost's fill: `surface.raised`.
    pub ghost_fill: Rgba,
    /// The drag ghost's border: `accent.solid`.
    pub ghost_border: Rgba,
}

impl ListTokens {
    /// The height of a row with a thumbnail of side `thumb`: the thumbnail and its padding, at
    /// least `thumb_row_min`.
    pub fn thumb_row(&self, thumb: f32) -> f32 {
        (thumb + self.thumb_pad).max(self.thumb_row_min)
    }

    fn color_mut(&mut self, f: ListField) -> Option<&mut Rgba> {
        Some(match f {
            ListField::Radius => return None,
            ListField::Text => &mut self.text,
            ListField::Dimmed => &mut self.dimmed,
            ListField::SelectedText => &mut self.selected_text,
            ListField::Detail => &mut self.detail,
            ListField::Icon => &mut self.icon,
            ListField::IconHover => &mut self.icon_hover,
            ListField::Hover => &mut self.hover,
            ListField::Selected => &mut self.selected,
            ListField::SelectedHover => &mut self.selected_hover,
            ListField::Divider => &mut self.divider,
            ListField::ThumbBorder => &mut self.thumb_border,
            ListField::Drop => &mut self.drop,
            ListField::DropHalo => &mut self.drop_halo,
            ListField::GhostFill => &mut self.ghost_fill,
            ListField::GhostBorder => &mut self.ghost_border,
        })
    }
}

/// An overridable list token. Paths: `radius`, `text`, `text.dimmed`, `text.selected`, `detail`,
/// `icon`, `icon.hover`, `hover`, `selected`, `selected.hover`, `divider`, `thumb.border`, `drop`,
/// `drop_halo`, `ghost.fill`, `ghost.border`, e.g. `list.selected.hover`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ListField {
    /// `list.radius`
    Radius,
    /// `list.text`
    Text,
    /// `list.text.dimmed`
    Dimmed,
    /// `list.text.selected`
    SelectedText,
    /// `list.detail`
    Detail,
    /// `list.icon`
    Icon,
    /// `list.icon.hover`
    IconHover,
    /// `list.hover`
    Hover,
    /// `list.selected`
    Selected,
    /// `list.selected.hover`
    SelectedHover,
    /// `list.divider`
    Divider,
    /// `list.thumb.border`
    ThumbBorder,
    /// `list.drop`
    Drop,
    /// `list.drop_halo`
    DropHalo,
    /// `list.ghost.fill`
    GhostFill,
    /// `list.ghost.border`
    GhostBorder,
}

impl ListField {
    const fn path(self) -> &'static str {
        match self {
            ListField::Radius => "radius",
            ListField::Text => "text",
            ListField::Dimmed => "text.dimmed",
            ListField::SelectedText => "text.selected",
            ListField::Detail => "detail",
            ListField::Icon => "icon",
            ListField::IconHover => "icon.hover",
            ListField::Hover => "hover",
            ListField::Selected => "selected",
            ListField::SelectedHover => "selected.hover",
            ListField::Divider => "divider",
            ListField::ThumbBorder => "thumb.border",
            ListField::Drop => "drop",
            ListField::DropHalo => "drop_halo",
            ListField::GhostFill => "ghost.fill",
            ListField::GhostBorder => "ghost.border",
        }
    }
}

impl Family for ListTokens {
    type Field = ListField;
    const FIELDS: &'static [ListField] = &[
        ListField::Radius,
        ListField::Text,
        ListField::Dimmed,
        ListField::SelectedText,
        ListField::Detail,
        ListField::Icon,
        ListField::IconHover,
        ListField::Hover,
        ListField::Selected,
        ListField::SelectedHover,
        ListField::Divider,
        ListField::ThumbBorder,
        ListField::Drop,
        ListField::DropHalo,
        ListField::GhostFill,
        ListField::GhostBorder,
    ];

    fn defaults(s: &Semantic<'_>) -> ListTokens {
        let (c, list) = (s.color, &s.size.list);
        ListTokens {
            row_height: s.size.control.md,
            thumb: list.thumb,
            thumb_pad: values::LIST_THUMB_PAD,
            thumb_row_min: list.thumb_row_min,
            pad_x: s.space.pad.control,
            gap: s.space.gap.inline,
            indent: list.indent,
            disclosure: list.disclosure,
            toggle_column: list.toggle_column,
            min_label: list.min_label,
            trailing_pad: list.trailing_pad,
            radius: if list.rounded { s.shape.radius.control } else { 0.0 },
            dividers: list.dividers,
            drop_width: values::LIST_DROP_WIDTH,
            drop_halo_width: values::LIST_DROP_HALO,
            label: TypeRole::Body,
            detail_role: TypeRole::Caption,
            text: c.text.normal,
            dimmed: c.text.faint,
            selected_text: c.text.on_selected,
            detail: c.text.faint,
            icon: c.text.muted,
            icon_hover: c.text.normal,
            hover: c.state.hover,
            selected: c.state.selected,
            selected_hover: c.state.selected_hover,
            divider: c.line.divider,
            thumb_border: c.line.border,
            drop: c.accent.solid,
            drop_halo: c.surface.panel,
            ghost_fill: c.surface.raised,
            ghost_border: c.accent.solid,
        }
    }

    fn path(f: ListField) -> String {
        f.path().to_owned()
    }

    fn from_path(path: &str) -> Option<ListField> {
        Self::FIELDS.iter().copied().find(|f| f.path() == path)
    }

    fn kind(f: ListField) -> FieldKind {
        match f {
            ListField::Radius => FieldKind::Length,
            _ => FieldKind::Color,
        }
    }

    fn get(&self, f: ListField) -> OverrideValue {
        match f {
            ListField::Radius => OverrideValue::Length(self.radius),
            f => {
                let mut copy = *self;
                OverrideValue::Color(copy.color_mut(f).map_or(Rgba::TRANSPARENT, |c| *c))
            }
        }
    }

    fn set(&mut self, f: ListField, v: OverrideValue) {
        match (f, v) {
            (ListField::Radius, OverrideValue::Length(r)) => self.radius = r,
            (f, OverrideValue::Color(c)) => {
                if let Some(slot) = self.color_mut(f) {
                    *slot = c;
                }
            }
            // Mismatched kinds are rejected before this is called.
            _ => {}
        }
    }

    fn pairs() -> Vec<ContrastPair> {
        let field = ComponentField::List;
        let panel = TokenRef::Color(ColorToken::SurfacePanel);
        let raised = TokenRef::Color(ColorToken::SurfaceRaised);
        let component = |f| TokenRef::Component(field(f));
        let pair = |fg, bg, minimum| ContrastPair { fg: TokenRef::Component(field(fg)), bg, minimum };
        vec![
            pair(ListField::Text, panel, values::TEXT_MIN),
            pair(ListField::Text, component(ListField::Hover), values::TEXT_MIN),
            pair(ListField::Text, raised, values::TEXT_MIN),
            pair(ListField::Dimmed, panel, values::TEXT_MIN),
            pair(ListField::Detail, panel, values::TEXT_MIN),
            pair(ListField::SelectedText, component(ListField::Selected), values::TEXT_MIN),
            pair(ListField::SelectedText, component(ListField::SelectedHover), values::TEXT_MIN),
            pair(ListField::Drop, component(ListField::DropHalo), values::NON_TEXT_MIN),
        ]
    }
}
