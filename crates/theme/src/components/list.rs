//! Lists: the rows of a list or tree panel (layers, history, channels). Sizes come from the Size
//! preset; the colours are the neutral state fills of the semantic tokens.
//!
//! The drop marker is the only cue that shows where a dragged row lands. It is drawn on its own
//! halo, as canvas overlays are, so it is measured against the halo alone (3:1, 4.5:1 with high
//! contrast) and reads on any row fill. Labels, dimmed labels and detail lines are text on the
//! panel and on the fills a row takes (4.5:1, 7:1 with high contrast); a selected row draws its
//! dimmed label and detail line in their own colours, `text.on_selected` moved toward the fill. The
//! ghost's label is text on the ghost's fill. The hover fill, half-opacity `state.hover`, is moved
//! until it is a ratio of 1.12 away from the panel (measured) and from the selected fill (kept by
//! the move), in every theme, mode and contrast. The selected fill, with and
//! without the pointer, is a state indicator: 3:1 (4.5:1 with high contrast) on the panel. The
//! ghost's fill is a design target with no minimum. Each pair is measured at rest.

use super::{ComponentField, ContrastPair, Family, FieldKind, Identify, OverrideValue, Semantic, SizeLeaf};
use crate::color::Oklch;
use crate::color::Rgba;
use crate::contrast::{contrast_ratio, ensure_contrast};
use crate::derive::{mix, over};
use crate::tokens::{ColorToken, ColorTokens, ThumbSizes, TokenRef, TypeRole};
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
    /// How far the second-thumb brackets stand outside the thumbnail: `list.bracket_outset`.
    pub bracket_outset: f32,
    /// Arm length of the second-thumb brackets: `list.bracket_arm`.
    pub bracket_arm: f32,
    /// Where the drag ghost starts from the pointer toward the reading end: `list.ghost_offset`.
    pub ghost_offset: [f32; 2],
    /// Type role of the label.
    pub label: TypeRole,
    /// Type role of the detail line under the label.
    pub detail_role: TypeRole,
    /// The label: `text.normal`.
    pub text: Rgba,
    /// The label of a dimmed row (a hidden layer): `text.faint`.
    pub dimmed: Rgba,
    /// The label of the selected row: `text.on_selected`, moved to read on the moved fills.
    pub selected_text: Rgba,
    /// The label of a dimmed row that is selected: `text.on_selected` moved toward the selected
    /// fill, readable on it and on the selected fill under the pointer.
    pub selected_dimmed: Rgba,
    /// The detail line: `text.faint`.
    pub detail: Rgba,
    /// The detail line of the selected row: as `selected_dimmed`.
    pub selected_detail: Rgba,
    /// Icons and the disclosure triangle: `text.muted`.
    pub icon: Rgba,
    /// Icons and the triangle under the pointer: `text.normal`.
    pub icon_hover: Rgba,
    /// Behind a row under the pointer: an opaque `state.hover` at [`values::LIST_HOVER_OPACITY`],
    /// moved to stay visible.
    pub hover: Rgba,
    /// Behind the selected row: `state.selected`, moved to reach the non-text minimum on the
    /// panel.
    pub selected: Rgba,
    /// Behind the selected row under the pointer: `state.selected_hover`, moved to reach the
    /// non-text minimum on the panel (and to keep its step from `selected` where the theme has one).
    /// Where the theme pins `selected`, the hover fill laid over it.
    pub selected_hover: Rgba,
    /// Dividers: `line.divider`.
    pub divider: Rgba,
    /// The frame of a thumbnail: `line.control_edge`.
    pub thumb_border: Rgba,
    /// The frame of the thumbnail of the selected row: `text.normal`.
    pub thumb_border_selected: Rgba,
    /// Corner radius of a thumbnail's frame: 0; the themes whose PhotoCraft kind is not a Pro
    /// kind pin its 5.
    pub thumb_radius: f32,
    /// Behind a layer-form row under the pointer: `list.hover` with `accent.solid` at
    /// [`values::LIST_LAYER_TINT_OPACITY`] laid over it.
    pub layer_hover: Rgba,
    /// Behind the selected layer-form row, with or without the pointer: `list.selected`; the themes
    /// whose PhotoCraft kind is not a Pro kind pin `state.hover` with a `layer_outline`.
    pub layer_selected: Rgba,
    /// Corner radius of the layer-form row's hover and selected fills and outline: `radius.popup`
    /// (PhotoCraft's `t.radius`) where the theme gives the selected row an outline, else `radius`.
    pub layer_radius: f32,
    /// An inside outline of the selected layer-form row, 1 pt: clear unless pinned
    /// (`line.control_edge` in the themes that pin `state.hover` as the fill).
    pub layer_outline: Rgba,
    /// The label of the selected layer-form row: `list.selected_text`.
    pub layer_selected_text: Rgba,
    /// The label of a dimmed selected layer-form row.
    pub layer_selected_dimmed: Rgba,
    /// The detail line of the selected layer-form row.
    pub layer_selected_detail: Rgba,
    /// The drop marker: `accent.solid`.
    pub drop: Rgba,
    /// The outline under the drop marker that separates it from the row fills: `surface.panel`.
    pub drop_halo: Rgba,
    /// The drag ghost's fill: `surface.raised`.
    pub ghost_fill: Rgba,
    /// The drag ghost's border: `accent.solid`.
    pub ghost_border: Rgba,
    /// The colours of each of PhotoCraft's label colours, in [`LabelColor::ALL`] order:
    /// `list.label.<color>.*`. Label colours are data colours, the same in every theme.
    pub labels: [LabelTint; LabelColor::ALL.len()],
}

/// PhotoCraft's layer label colours (`LabelColor`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LabelColor {
    Red,
    Orange,
    Yellow,
    Green,
    Seafoam,
    Blue,
    Indigo,
    Magenta,
    Fuchsia,
    Violet,
    Gray,
}

impl LabelColor {
    /// Every label colour, in path order.
    pub const ALL: [LabelColor; 11] = [
        LabelColor::Red,
        LabelColor::Orange,
        LabelColor::Yellow,
        LabelColor::Green,
        LabelColor::Seafoam,
        LabelColor::Blue,
        LabelColor::Indigo,
        LabelColor::Magenta,
        LabelColor::Fuchsia,
        LabelColor::Violet,
        LabelColor::Gray,
    ];

    /// The path segment.
    pub const fn name(self) -> &'static str {
        match self {
            LabelColor::Red => "red",
            LabelColor::Orange => "orange",
            LabelColor::Yellow => "yellow",
            LabelColor::Green => "green",
            LabelColor::Seafoam => "seafoam",
            LabelColor::Blue => "blue",
            LabelColor::Indigo => "indigo",
            LabelColor::Magenta => "magenta",
            LabelColor::Fuchsia => "fuchsia",
            LabelColor::Violet => "violet",
            LabelColor::Gray => "gray",
        }
    }

    fn from_name(name: &str) -> Option<LabelColor> {
        Self::ALL.into_iter().find(|c| c.name() == name)
    }

    /// PhotoCraft's colours for this label: the eye-cell fill and the swatch.
    const fn default_tint(self) -> LabelTint {
        let (fill, swatch) = match self {
            LabelColor::Red => (Rgba::rgb(0x9F, 0x2F, 0x30), Rgba::rgb(0xFC, 0x5D, 0x5B)),
            LabelColor::Orange => (Rgba::rgb(0x93, 0x4F, 0x0C), Rgba::rgb(0xF7, 0x97, 0x44)),
            LabelColor::Yellow => (Rgba::rgb(0x99, 0x78, 0x0C), Rgba::rgb(0xDC, 0xD6, 0x4B)),
            LabelColor::Green => (Rgba::rgb(0x4E, 0x71, 0x2E), Rgba::rgb(0x85, 0xDC, 0x6A)),
            LabelColor::Seafoam => (Rgba::rgb(0x0B, 0x54, 0x4F), Rgba::rgb(0x1C, 0x84, 0x88)),
            LabelColor::Blue => (Rgba::rgb(0x41, 0x5B, 0x87), Rgba::rgb(0x78, 0xAD, 0xF6)),
            LabelColor::Indigo => (Rgba::rgb(0x36, 0x34, 0x8E), Rgba::rgb(0x54, 0x4B, 0xE7)),
            LabelColor::Magenta => (Rgba::rgb(0x98, 0x1B, 0x51), Rgba::rgb(0xCC, 0x1A, 0x7E)),
            LabelColor::Fuchsia => (Rgba::rgb(0x71, 0x0F, 0x74), Rgba::rgb(0xB0, 0x14, 0xC0)),
            LabelColor::Violet => (Rgba::rgb(0x5D, 0x3F, 0x8E), Rgba::rgb(0x91, 0x76, 0xD5)),
            LabelColor::Gray => (Rgba::rgb(0x57, 0x57, 0x57), Rgba::rgb(0x9C, 0x9C, 0x9C)),
        };
        LabelTint { fill, icon: Rgba::rgb(0xE2, 0xE2, 0xE2), swatch }
    }
}

/// The colours of one label colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LabelTint {
    /// Behind a toggle cell (the eye) of a row with this label.
    pub fill: Rgba,
    /// The toggle cell's icon on `fill`.
    pub icon: Rgba,
    /// The label's swatch, as in a colour menu.
    pub swatch: Rgba,
}

/// One colour of a [`LabelTint`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LabelPart {
    /// `fill`
    Fill,
    /// `icon`
    Icon,
    /// `swatch`
    Swatch,
}

impl LabelPart {
    /// Every part, in path order.
    pub const ALL: [LabelPart; 3] = [LabelPart::Fill, LabelPart::Icon, LabelPart::Swatch];

    /// The path segment.
    pub const fn name(self) -> &'static str {
        match self {
            LabelPart::Fill => "fill",
            LabelPart::Icon => "icon",
            LabelPart::Swatch => "swatch",
        }
    }
}

impl ListTokens {
    /// The colours of the label colour `color`.
    pub fn label(&self, color: LabelColor) -> LabelTint {
        self.labels[color as usize]
    }

    /// The height of a row with a thumbnail of side `thumb`: the thumbnail and its padding, at
    /// least `thumb_row_min`.
    pub fn thumb_row(&self, thumb: f32) -> f32 {
        (thumb + self.thumb_pad).max(self.thumb_row_min)
    }

    /// Where the theme pins the selected fill, the rest of the selected look follows it: the hover
    /// fill moved to stay visible beside it, the selected row under the pointer as the hover fill
    /// laid over it, and in high contrast the fills and the text moved as for an unpinned one.
    /// Each stays as pinned where the theme pins it too.
    pub(crate) fn follow(&mut self, color: &ColorTokens, high: bool, pinned: impl Fn(ListField) -> bool) {
        if pinned(ListField::Selected) {
            self.follow_selected(color, high, &pinned);
        }
        self.follow_layer(color, high, &pinned);
    }

    fn follow_selected(&mut self, color: &ColorTokens, high: bool, pinned: &impl Fn(ListField) -> bool) {
        if !pinned(ListField::Hover) {
            self.hover = visible_hover(self.hover, color.surface.panel, &[self.selected], color.text.normal, high);
        }
        let hover = if pinned(ListField::SelectedHover) { self.selected_hover } else { stack(self.hover, self.selected) };
        let look = SelectedLook::of(color, self.selected, hover, color.text.on_selected, None, high);
        let set = |f, slot: &mut Rgba, value| {
            if !pinned(f) {
                *slot = value;
            }
        };
        set(ListField::SelectedHover, &mut self.selected_hover, look.selected_hover);
        set(ListField::SelectedText, &mut self.selected_text, look.text);
        set(ListField::SelectedDimmed, &mut self.selected_dimmed, look.quiet);
        set(ListField::SelectedDetail, &mut self.selected_detail, look.quiet);
        self.selected = look.selected;
    }

    /// The layer form follows `hover` and the fill, outline and text the theme pins for it. Where
    /// it pins none, the layer form takes the plain selected look as it is. Each stays as pinned
    /// where the theme pins it too.
    fn follow_layer(&mut self, color: &ColorTokens, high: bool, pinned: &impl Fn(ListField) -> bool) {
        if !pinned(ListField::LayerRadius) && self.layer_outline.a() == 0 {
            self.layer_radius = self.radius;
        }
        if !pinned(ListField::LayerHover) {
            self.layer_hover = layer_hover(self.hover, color, high);
        }
        let own = [ListField::LayerSelected, ListField::LayerOutline, ListField::LayerSelectedText].into_iter().any(pinned);
        if !own {
            let set = |f, slot: &mut Rgba, value| {
                if !pinned(f) {
                    *slot = value;
                }
            };
            set(ListField::LayerSelected, &mut self.layer_selected, self.selected);
            set(ListField::LayerSelectedText, &mut self.layer_selected_text, self.selected_text);
            set(ListField::LayerSelectedDimmed, &mut self.layer_selected_dimmed, self.selected_dimmed);
            set(ListField::LayerSelectedDetail, &mut self.layer_selected_detail, self.selected_detail);
            return;
        }
        let text = if pinned(ListField::LayerSelectedText) { self.layer_selected_text } else { color.text.on_selected };
        let edge = (self.layer_outline.a() > 0).then_some(self.layer_outline);
        let look = SelectedLook::of(color, self.layer_selected, self.layer_selected, text, edge, high);
        let set = |f, slot: &mut Rgba, value| {
            if !pinned(f) {
                *slot = value;
            }
        };
        set(ListField::LayerSelectedText, &mut self.layer_selected_text, look.text);
        set(ListField::LayerSelectedDimmed, &mut self.layer_selected_dimmed, look.quiet);
        set(ListField::LayerSelectedDetail, &mut self.layer_selected_detail, look.quiet);
        self.layer_selected = look.selected;
    }

    fn color_mut(&mut self, f: ListField) -> Option<&mut Rgba> {
        Some(match f {
            ListField::Radius | ListField::ThumbRadius | ListField::LayerRadius => return None,
            ListField::Label(color, part) => {
                let tint = &mut self.labels[color as usize];
                match part {
                    LabelPart::Fill => &mut tint.fill,
                    LabelPart::Icon => &mut tint.icon,
                    LabelPart::Swatch => &mut tint.swatch,
                }
            }
            ListField::Text => &mut self.text,
            ListField::Dimmed => &mut self.dimmed,
            ListField::SelectedText => &mut self.selected_text,
            ListField::SelectedDimmed => &mut self.selected_dimmed,
            ListField::Detail => &mut self.detail,
            ListField::SelectedDetail => &mut self.selected_detail,
            ListField::Icon => &mut self.icon,
            ListField::IconHover => &mut self.icon_hover,
            ListField::Hover => &mut self.hover,
            ListField::Selected => &mut self.selected,
            ListField::SelectedHover => &mut self.selected_hover,
            ListField::Divider => &mut self.divider,
            ListField::ThumbBorder => &mut self.thumb_border,
            ListField::ThumbBorderSelected => &mut self.thumb_border_selected,
            ListField::LayerHover => &mut self.layer_hover,
            ListField::LayerSelected => &mut self.layer_selected,
            ListField::LayerOutline => &mut self.layer_outline,
            ListField::LayerSelectedText => &mut self.layer_selected_text,
            ListField::LayerSelectedDimmed => &mut self.layer_selected_dimmed,
            ListField::LayerSelectedDetail => &mut self.layer_selected_detail,
            ListField::Drop => &mut self.drop,
            ListField::DropHalo => &mut self.drop_halo,
            ListField::GhostFill => &mut self.ghost_fill,
            ListField::GhostBorder => &mut self.ghost_border,
        })
    }
}

/// An overridable list token. Paths: `radius`, `text`, `text.dimmed`, `text.selected`,
/// `text.selected.dimmed`, `detail`, `detail.selected`, `icon`, `icon.hover`, `hover`,
/// `selected`, `selected.hover`, `divider`, `thumb.border`, `thumb.border.selected`, `thumb.radius`,
/// `layer.hover`, `layer.selected`, `layer.outline`, `layer.radius`, `layer.text.selected`,
/// `layer.text.selected.dimmed`, `layer.detail.selected`, `drop`, `drop.halo`, `ghost.fill`,
/// `ghost.border` and `label.<color>.<fill|icon|swatch>`, e.g. `list.selected.hover`.
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
    /// `list.text.selected.dimmed`
    SelectedDimmed,
    /// `list.detail`
    Detail,
    /// `list.detail.selected`
    SelectedDetail,
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
    /// `list.thumb.border.selected`
    ThumbBorderSelected,
    /// `list.thumb.radius`
    ThumbRadius,
    /// `list.layer.hover`
    LayerHover,
    /// `list.layer.selected`
    LayerSelected,
    /// `list.layer.outline`
    LayerOutline,
    /// `list.layer.radius`
    LayerRadius,
    /// `list.layer.text.selected`
    LayerSelectedText,
    /// `list.layer.text.selected.dimmed`
    LayerSelectedDimmed,
    /// `list.layer.detail.selected`
    LayerSelectedDetail,
    /// `list.drop`
    Drop,
    /// `list.drop.halo`
    DropHalo,
    /// `list.ghost.fill`
    GhostFill,
    /// `list.ghost.border`
    GhostBorder,
    /// `list.label.<color>.<fill|icon|swatch>`
    Label(LabelColor, LabelPart),
}

impl ListField {
    const fn path(self) -> &'static str {
        match self {
            ListField::Label(..) => "label",
            ListField::Radius => "radius",
            ListField::Text => "text",
            ListField::Dimmed => "text.dimmed",
            ListField::SelectedText => "text.selected",
            ListField::SelectedDimmed => "text.selected.dimmed",
            ListField::Detail => "detail",
            ListField::SelectedDetail => "detail.selected",
            ListField::Icon => "icon",
            ListField::IconHover => "icon.hover",
            ListField::Hover => "hover",
            ListField::Selected => "selected",
            ListField::SelectedHover => "selected.hover",
            ListField::Divider => "divider",
            ListField::ThumbBorder => "thumb.border",
            ListField::ThumbBorderSelected => "thumb.border.selected",
            ListField::ThumbRadius => "thumb.radius",
            ListField::LayerHover => "layer.hover",
            ListField::LayerSelected => "layer.selected",
            ListField::LayerOutline => "layer.outline",
            ListField::LayerRadius => "layer.radius",
            ListField::LayerSelectedText => "layer.text.selected",
            ListField::LayerSelectedDimmed => "layer.text.selected.dimmed",
            ListField::LayerSelectedDetail => "layer.detail.selected",
            ListField::Drop => "drop",
            ListField::DropHalo => "drop.halo",
            ListField::GhostFill => "ghost.fill",
            ListField::GhostBorder => "ghost.border",
        }
    }
}

const BASE_FIELDS: [ListField; 27] = [
    ListField::Radius,
    ListField::Text,
    ListField::Dimmed,
    ListField::SelectedText,
    ListField::SelectedDimmed,
    ListField::Detail,
    ListField::SelectedDetail,
    ListField::Icon,
    ListField::IconHover,
    ListField::Hover,
    ListField::Selected,
    ListField::SelectedHover,
    ListField::Divider,
    ListField::ThumbBorder,
    ListField::ThumbBorderSelected,
    ListField::ThumbRadius,
    ListField::LayerHover,
    ListField::LayerSelected,
    ListField::LayerOutline,
    ListField::LayerRadius,
    ListField::LayerSelectedText,
    ListField::LayerSelectedDimmed,
    ListField::LayerSelectedDetail,
    ListField::Drop,
    ListField::DropHalo,
    ListField::GhostFill,
    ListField::GhostBorder,
];

const FIELD_COUNT: usize = BASE_FIELDS.len() + LabelColor::ALL.len() * LabelPart::ALL.len();

const FIELDS: [ListField; FIELD_COUNT] = {
    let mut out = [ListField::Radius; FIELD_COUNT];
    let mut i = 0;
    while i < BASE_FIELDS.len() {
        out[i] = BASE_FIELDS[i];
        i += 1;
    }
    while i < FIELD_COUNT {
        let n = i - BASE_FIELDS.len();
        out[i] = ListField::Label(LabelColor::ALL[n / LabelPart::ALL.len()], LabelPart::ALL[n % LabelPart::ALL.len()]);
        i += 1;
    }
    out
};

impl Family for ListTokens {
    type Field = ListField;
    const FIELDS: &'static [ListField] = &FIELDS;
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ListTokens;
        "row_height" 0.0..=1024.0 => row_height;
        "thumb.sm" 0.0..=96.0 => thumb.sm;
        "thumb.md" 0.0..=96.0 => thumb.md;
        "thumb.lg" 0.0..=96.0 => thumb.lg;
        "thumb_pad" 0.0..=64.0 => thumb_pad;
        "thumb_row_min" 0.0..=1024.0 => thumb_row_min;
        "pad_x" 0.0..=32.0 => pad_x;
        "gap" 0.0..=32.0 => gap;
        "indent" 0.0..=1024.0 => indent;
        "disclosure" 0.0..=1024.0 => disclosure;
        "toggle_column" 0.0..=1024.0 => toggle_column;
        "min_label" 0.0..=1024.0 => min_label;
        "trailing_pad" 0.0..=64.0 => trailing_pad;
        "drop_width" 0.0..=1024.0 => drop_width;
        "drop_halo_width" 0.0..=16.0 => drop_halo_width;
        "bracket_outset" 0.0..=64.0 => bracket_outset;
        "bracket_arm" 0.0..=1024.0 => bracket_arm;
        "ghost_offset.0" -64.0..=64.0 => ghost_offset[0];
        "ghost_offset.1" -64.0..=64.0 => ghost_offset[1];
    };

    fn defaults(s: &Semantic<'_>) -> ListTokens {
        let (c, list) = (s.color, &s.size.list);
        let look = SelectedLook::of(c, c.state.selected, c.state.selected_hover, c.text.on_selected, None, s.high);
        let selected = look.selected;
        let hover = visible_hover(half(c.state.hover, values::LIST_HOVER_OPACITY), c.surface.panel, &[selected], c.text.normal, s.high);
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
            bracket_outset: values::LIST_BRACKET_OUTSET,
            bracket_arm: values::LIST_BRACKET_ARM,
            ghost_offset: values::LIST_GHOST_OFFSET,
            label: TypeRole::Body,
            detail_role: TypeRole::Caption,
            text: c.text.normal,
            dimmed: c.text.faint,
            selected_text: look.text,
            selected_dimmed: look.quiet,
            detail: c.text.faint,
            selected_detail: look.quiet,
            icon: c.text.muted,
            icon_hover: c.text.normal,
            hover,
            selected,
            selected_hover: look.selected_hover,
            divider: c.line.divider,
            thumb_border: c.line.control_edge,
            thumb_border_selected: c.text.normal,
            thumb_radius: 0.0,
            layer_hover: layer_hover(hover, c, s.high),
            layer_selected: selected,
            layer_outline: Rgba::TRANSPARENT,
            layer_radius: s.shape.radius.popup,
            layer_selected_text: look.text,
            layer_selected_dimmed: look.quiet,
            layer_selected_detail: look.quiet,
            drop: c.accent.solid,
            drop_halo: c.surface.panel,
            ghost_fill: c.surface.raised,
            ghost_border: c.accent.solid,
            labels: LabelColor::ALL.map(LabelColor::default_tint),
        }
    }

    fn path(f: ListField) -> String {
        match f {
            ListField::Label(color, part) => format!("label.{}.{}", color.name(), part.name()),
            f => f.path().to_owned(),
        }
    }

    fn from_path(path: &str) -> Option<ListField> {
        if let Some(rest) = path.strip_prefix("label.") {
            let (color, part) = rest.split_once('.')?;
            return Some(ListField::Label(LabelColor::from_name(color)?, LabelPart::ALL.into_iter().find(|p| p.name() == part)?));
        }
        Self::FIELDS.iter().copied().find(|f| f.path() == path)
    }

    fn kind(f: ListField) -> FieldKind {
        match f {
            ListField::Radius | ListField::ThumbRadius | ListField::LayerRadius => FieldKind::Length,
            _ => FieldKind::Color,
        }
    }

    fn get(&self, f: ListField) -> OverrideValue {
        match f {
            ListField::Radius => OverrideValue::Length(self.radius),
            ListField::ThumbRadius => OverrideValue::Length(self.thumb_radius),
            ListField::LayerRadius => OverrideValue::Length(self.layer_radius),
            f => {
                let mut copy = *self;
                OverrideValue::Color(copy.color_mut(f).map_or(Rgba::TRANSPARENT, |c| *c))
            }
        }
    }

    fn set(&mut self, f: ListField, v: OverrideValue) {
        match (f, v) {
            (ListField::Radius, OverrideValue::Length(r)) => self.radius = r,
            (ListField::ThumbRadius, OverrideValue::Length(r)) => self.thumb_radius = r,
            (ListField::LayerRadius, OverrideValue::Length(r)) => self.layer_radius = r,
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
        let component = |f| TokenRef::Component(field(f));
        let pair = |fg, bg, minimum| ContrastPair { fg: TokenRef::Component(field(fg)), bg, minimum, alternate: None };
        vec![
            // A selected row shows on the panel, also while hovered, and hovering it shows.
            pair(ListField::Selected, panel, values::STATE_CUE_MIN),
            pair(ListField::SelectedHover, panel, values::STATE_CUE_MIN),
            pair(ListField::Hover, panel, values::LIST_HOVER_MIN),
            pair(ListField::Text, panel, values::TEXT_MIN),
            pair(ListField::Text, component(ListField::Hover), values::TEXT_MIN),
            pair(ListField::Text, component(ListField::GhostFill), values::TEXT_MIN),
            pair(ListField::Dimmed, panel, values::TEXT_MIN),
            pair(ListField::Detail, panel, values::TEXT_MIN),
            pair(ListField::SelectedText, component(ListField::Selected), values::TEXT_MIN),
            pair(ListField::SelectedText, component(ListField::SelectedHover), values::TEXT_MIN),
            pair(ListField::SelectedDimmed, component(ListField::Selected), values::TEXT_MIN),
            pair(ListField::SelectedDimmed, component(ListField::SelectedHover), values::TEXT_MIN),
            pair(ListField::SelectedDetail, component(ListField::Selected), values::TEXT_MIN),
            pair(ListField::SelectedDetail, component(ListField::SelectedHover), values::TEXT_MIN),
            // The layer form: hovering shows, the selected row shows by its fill or its outline.
            pair(ListField::LayerHover, panel, values::LIST_HOVER_MIN),
            ContrastPair { alternate: Some(component(ListField::LayerOutline)), ..pair(ListField::LayerSelected, panel, values::STATE_CUE_MIN) },
            pair(ListField::Text, component(ListField::LayerHover), values::TEXT_MIN),
            pair(ListField::LayerSelectedText, component(ListField::LayerSelected), values::TEXT_MIN),
            pair(ListField::LayerSelectedDimmed, component(ListField::LayerSelected), values::TEXT_MIN),
            pair(ListField::LayerSelectedDetail, component(ListField::LayerSelected), values::TEXT_MIN),
            pair(ListField::Drop, component(ListField::DropHalo), values::NON_TEXT_MIN),
        ]
    }
}

/// The hover fill of a list row: `state.hover`, moved along OKLCH lightness to the nearest step
/// that is at least `values::LIST_HOVER_MIN` (WCAG ratio) away from `rest` and from every fill in
/// `beside` (translucent fills over `rest`), and that `text` still reads on. When no step satisfies
/// all of them, the nearest step that keeps its distance from the others wins.
pub(super) fn visible_hover(hover: Rgba, rest: Rgba, beside: &[Rgba], text: Rgba, high: bool) -> Rgba {
    let min = if high { values::LIST_HOVER_MIN.1 } else { values::LIST_HOVER_MIN.0 };
    let text_min = if high { values::TEXT_MIN.1 } else { values::TEXT_MIN.0 };
    let fills: Vec<Rgba> = std::iter::once(rest).chain(beside.iter().map(|&f| over(f, rest))).collect();
    let distinct = |c: Rgba| fills.iter().all(|&f| contrast_ratio(over(c, rest), f) >= min);
    if distinct(hover) {
        return hover;
    }
    let start = hover.to_oklch();
    let alpha = hover.a();
    let at = |l: f64| Oklch { l, ..start }.to_rgba().with_alpha(alpha);
    // For each fill and direction, the step nearest that fill that clears `min` against it.
    let mut candidates: Vec<Rgba> = Vec::new();
    for &fill in &fills {
        let l0 = over(fill, rest).to_oklch().l;
        for target in [0.0, 1.0] {
            let (mut lo, mut hi) = (l0, target);
            if contrast_ratio(over(at(hi), rest), fill) < min {
                continue;
            }
            for _ in 0..40 {
                let mid = 0.5 * (lo + hi);
                if contrast_ratio(over(at(mid), rest), fill) >= min {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            candidates.push(at(hi));
        }
    }
    candidates.retain(|&c| distinct(c));
    let nearest =
        |pool: &mut dyn Iterator<Item = Rgba>| pool.min_by(|a, b| (a.to_oklch().l - start.l).abs().total_cmp(&(b.to_oklch().l - start.l).abs()));
    let readable = |c: &Rgba| contrast_ratio(text, over(*c, rest)) >= text_min;
    nearest(&mut candidates.iter().copied().filter(readable)).or_else(|| nearest(&mut candidates.iter().copied())).unwrap_or(hover)
}

/// `text` moved until it reads (the text minimum) on every fill, over `rest`.
fn readable_on(text: Rgba, fills: &[Rgba], rest: Rgba, high: bool) -> Rgba {
    let min = if high { values::TEXT_MIN.1 } else { values::TEXT_MIN.0 };
    let mut text = text;
    // Moving away from one fill can move toward the other only when they sit either side of the
    // text; a second round settles that case.
    for _ in 0..2 {
        for &fill in fills {
            text = ensure_contrast(text, over(fill, rest), min);
        }
    }
    text
}

/// The selected row's text moved toward the selected fill (a hidden layer's name, a detail line
/// on the selected row), then as far back as text needs to read on both selected fills.
fn quiet_on_selected(text: Rgba, fills: &[Rgba; 2], rest: Rgba) -> Rgba {
    let quiet = mix(text, over(fills[0], rest), values::LIST_SELECTED_QUIET);
    readable_on(quiet, fills, rest, false)
}

/// `hover` with `accent.solid` at [`values::LIST_LAYER_TINT_OPACITY`] laid over it (PhotoCraft
/// paints the half-opacity hover, then the accent tint), moved to stay visible on the panel. It is
/// never drawn beside the selected row's fill, so it keeps no step from it. Where the tint would
/// leave the text unreadable on it (a light accent over a dark hover in high contrast), `hover`
/// stays without it.
fn layer_hover(hover: Rgba, color: &ColorTokens, high: bool) -> Rgba {
    let tinted = stack(color.accent.solid.faded(values::LIST_LAYER_TINT_OPACITY), hover);
    let moved = visible_hover(tinted, color.surface.panel, &[], color.text.normal, high);
    let min = if high { values::TEXT_MIN.1 } else { values::TEXT_MIN.0 };
    if contrast_ratio(color.text.normal, over(moved, color.surface.panel)) >= min { moved } else { hover }
}

/// An opaque `state.hover` at `opacity`; a translucent one is already a partial overlay.
pub(super) fn half(hover: Rgba, opacity: f32) -> Rgba {
    if hover.a() == 255 { hover.faded(opacity) } else { hover }
}

/// `top` painted over `bottom`, both possibly translucent.
fn stack(top: Rgba, bottom: Rgba) -> Rgba {
    let (at, ab) = (f64::from(top.a()) / 255.0, f64::from(bottom.a()) / 255.0);
    let a = at + ab * (1.0 - at);
    if a <= 0.0 {
        return Rgba::TRANSPARENT;
    }
    let ch = |i: usize| ((f64::from(top.0[i]) * at + f64::from(bottom.0[i]) * ab * (1.0 - at)) / a).round() as u8;
    Rgba([ch(0), ch(1), ch(2), (a * 255.0).round() as u8])
}

/// The selected row's fills and text for the fills `selected` and `selected_hover`: unchanged in
/// standard contrast, moved to identify the state on the panel in high contrast.
pub(super) struct SelectedLook {
    pub selected: Rgba,
    pub selected_hover: Rgba,
    pub text: Rgba,
    pub quiet: Rgba,
}

impl SelectedLook {
    /// `text` is the label on the unmoved fills; `edge` an outline that identifies the selected row
    /// as well as its fill.
    pub(super) fn of(c: &ColorTokens, selected: Rgba, selected_hover: Rgba, text: Rgba, edge: Option<Rgba>, high: bool) -> SelectedLook {
        let panel = [c.surface.panel];
        let min_step = if high { values::LIST_HOVER_MIN.1 } else { values::LIST_HOVER_MIN.0 };
        let identify = || edge.map_or_else(|| Identify::on(&panel, high), |e| Identify::on(&panel, high).edge(e));
        let (moved, text_on) = identify().fill(selected, text);
        // A theme whose selected row has a distinct hover step keeps one.
        let stepped = contrast_ratio(over(selected_hover, c.surface.panel), over(selected, c.surface.panel)) >= min_step;
        let apart: &[Rgba] = if stepped { &[moved] } else { &[] };
        let (moved_hover, text_on) = identify().apart(apart).with(&[moved]).fill(selected_hover, text_on);
        let fills = [moved, moved_hover];
        // Text that already reads on the unmoved fills stays as it is, pinned or not.
        let text = if fills == [selected, selected_hover] { text } else { readable_on(text_on, &fills, c.surface.panel, high) };
        let quiet = quiet_on_selected(text, &fills, c.surface.panel);
        SelectedLook { selected: moved, selected_hover: moved_hover, text, quiet }
    }
}
