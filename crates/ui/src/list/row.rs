//! `ListRow`: one row of a list, the item component of `List`, `FindingsList` and `EventLog`.
//!
//! In reading order a row has its toggle cells (in the columns the collection lays out), an indent and a
//! disclosure column for a tree, a clipping mark, an icon or thumbnails, the label with an optional detail line and
//! wrapped lines under it, and trailing items. The row mirrors in a right-to-left interface.
//!
//! Selection, the lead, hover, the drop mark, the drag ghost, the focus ring and the rename editor belong to the
//! collection that shows the row; the row draws the look the item step gives it and reports what its own parts do.

use super::toggle::ToggleCell;
use super::words::ListWords;
use crate::contract::{Common, PaintUi};
use crate::primitives::{ContentPainterCx, IconSpec};
use crate::runtime::env::Label;
use craft_behavior::ColumnKey;
use egui::{Rect, Response, Vec2};

/// The thumbnail setting of a list, which the host stores.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ThumbSize {
    /// No thumbnail: rows are one control high.
    #[default]
    None,
    /// `size.thumb.sm`.
    Small,
    /// `size.thumb.md`.
    Medium,
    /// `size.thumb.lg`.
    Large,
    /// A side in points, for a thumbnail that follows a slider (a preset browser's list view).
    Px(u16),
}

impl ThumbSize {
    /// Every setting a user picks from.
    pub const ALL: [ThumbSize; 4] = [ThumbSize::None, ThumbSize::Small, ThumbSize::Medium, ThumbSize::Large];

    /// The setting's name in the language of `words`.
    pub fn label(self, words: &ListWords) -> Label {
        match self {
            ThumbSize::None => words.thumb_none.clone(),
            ThumbSize::Small => words.thumb_small.clone(),
            ThumbSize::Medium => words.thumb_medium.clone(),
            ThumbSize::Large => words.thumb_large.clone(),
            ThumbSize::Px(_) => words.thumb_custom.clone(),
        }
    }
}

/// One of a row's thumbnails.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThumbTarget {
    /// The first thumbnail ([`ListRow::thumb`]).
    First,
    /// A thumbnail after the first ([`ListRow::extra_thumb`]), counted from 0 in the order added.
    Extra(usize),
}

/// How strongly a row's label reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RowInk {
    /// `text.normal`.
    #[default]
    Normal,
    /// `text.muted`: a secondary row that is still on.
    Muted,
    /// `text.faint`: a hidden layer.
    Dimmed,
}

/// What a row reports besides what the collection reads from its pointer input.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RowEvent {
    /// The disclosure part of a group row was clicked: `open` is the state it asks for.
    Open {
        /// The state the click asks for.
        open: bool,
    },
    /// The link glyph before an extra thumbnail was clicked: `linked` is the state it asks for.
    LinkToggled {
        /// The extra thumbnail the glyph belongs to, counted from 0 in the order added.
        thumb: usize,
        /// The requested state.
        linked: bool,
    },
    /// A click landed on a thumbnail.
    ThumbClicked(ThumbTarget),
    /// A drag started on an extra thumbnail of a row with `thumb_drags`: the thumbnail is dragged, not the row.
    ThumbDrag(ThumbTarget),
}

/// What a row returns.
#[derive(Debug)]
#[non_exhaustive]
pub struct RowOutput {
    /// The row's response.
    pub response: Response,
    /// What the row's parts reported this pass.
    pub event: Option<RowEvent>,
    /// Where the label and detail are drawn, the full height of the row.
    pub label: Rect,
}

/// Paints into a thumbnail's rect through the content painter, which is clipped to it.
pub type ThumbPaint<'a> = Box<dyn FnOnce(&ContentPainterCx, Rect) + 'a>;

/// What a trailing item shows in the rect the row gives it: it shows its components with `PaintUi::child`.
pub type TrailingPaint<'a> = Box<dyn FnOnce(&mut PaintUi<'_>, Rect) + 'a>;

/// A thumbnail after the first: its painter and the link glyph before it.
pub(super) struct Thumb<'a> {
    pub paint: Option<ThumbPaint<'a>>,
    /// The state of the link glyph before an extra thumbnail and its name, a whole pattern with `{row}`.
    pub link: Option<(bool, Label)>,
}

/// A trailing item: its width in control heights, whether it drops out for room, and what it shows.
pub(super) struct Trailing<'a> {
    pub heights: f32,
    pub optional: bool,
    pub paint: Option<TrailingPaint<'a>>,
}

/// One row of a list.
#[derive(crate::contract::Component, crate::contract::Props)]
pub struct ListRow<'a> {
    #[common]
    pub(super) common: Common,
    /// The label in the strong weight (`body_strong`), for a row that heads the others.
    #[style_key]
    #[prop]
    pub(super) strong: bool,
    /// How strongly the label reads. A dimmed row wins over a muted one.
    #[style_key]
    #[prop]
    pub(super) ink: RowInk,
    /// The layer form, for lists of layers and channels: dividers under the row and at each toggle cell's edge,
    /// the accent tint on the hover and an outline on the selected row.
    #[style_key]
    #[prop]
    pub(super) layer: bool,
    /// The row of a striped list that is tinted: every second row.
    #[style_key]
    #[prop]
    pub(super) striped: bool,
    /// The label in italics.
    #[prop]
    pub(super) italic: bool,
    /// The hover look without the pointer (the canvas hovers the layer).
    #[prop]
    pub(super) highlight: bool,
    /// The row is clipped to the layer below: it is indented one step and shows the clipping mark before its icon
    /// or thumbnail.
    #[prop]
    pub(super) clipped: bool,
    /// A hairline along the row's bottom edge, in a plain row too.
    #[prop]
    pub(super) rule: bool,
    /// The rename editor replaces the label.
    #[prop]
    pub(super) renaming: bool,
    /// A press that starts a drag on an extra thumbnail drags the thumbnail instead of the row.
    #[prop]
    pub(super) thumb_drags: bool,
    /// The icon or thumbnails come before the disclosure column instead of after it.
    #[prop]
    pub(super) leading_first: bool,
    /// A second line under the label in the caption role and `text.faint`.
    #[prop(into)]
    pub(super) detail: Option<Label>,
    /// Secondary text under the detail line, wrapped to the label band's width. The row's height follows the
    /// wrapped lines.
    #[prop(into)]
    pub(super) lines: Option<Label>,
    /// What a screen reader reads as the row's name when it differs from the drawn text.
    #[prop(into)]
    pub(super) accessible_name: Option<String>,
    pub(super) spacer: bool,
    pub(super) cells: Vec<(ColumnKey, Rect)>,
    pub(super) toggles: Vec<ToggleCell>,
    pub(super) icon: Option<IconSpec<'a>>,
    pub(super) thumb: ThumbSize,
    pub(super) thumb_side: Option<Vec2>,
    pub(super) first: Option<ThumbPaint<'a>>,
    pub(super) extras: Vec<Thumb<'a>>,
    pub(super) target: Option<ThumbTarget>,
    pub(super) trailing: Vec<Trailing<'a>>,
    /// The row's depth in a tree, which indents it; `None` for a row of a flat list.
    pub(super) depth: Option<usize>,
    /// `Some(open)` for a tree row that has children, which shows the disclosure part.
    pub(super) group: Option<bool>,
    pub(super) event: Option<RowEvent>,
    pub(super) label_rect: Rect,
}

impl<'a> ListRow<'a> {
    /// A row showing `label`, which also names it for a screen reader.
    pub fn new(label: impl Into<Label>) -> Self {
        use crate::contract::Props as _;
        ListRow {
            common: Common::new(),
            strong: false,
            ink: RowInk::Normal,
            layer: false,
            striped: false,
            italic: false,
            highlight: false,
            clipped: false,
            rule: false,
            renaming: false,
            thumb_drags: false,
            leading_first: false,
            detail: None,
            lines: None,
            accessible_name: None,
            spacer: false,
            cells: Vec::new(),
            toggles: Vec::new(),
            icon: None,
            thumb: ThumbSize::None,
            thumb_side: None,
            first: None,
            extras: Vec::new(),
            target: None,
            trailing: Vec::new(),
            depth: None,
            group: None,
            event: None,
            label_rect: Rect::NOTHING,
        }
        .label(label)
    }

    /// Empty room in a list: a row that takes its height but draws nothing, senses nothing and has no AccessKit
    /// node. Mark its position `ItemSource::is_section` so keys and clicks skip it.
    pub fn spacer() -> Self {
        let mut row = Self::new(Label::Empty);
        row.spacer = true;
        row
    }

    /// The rects of the row's columns, which the item context lists (`ItemCx::cells`): toggle cells draw in theirs
    /// and the rest of the row lays out in what they leave.
    #[must_use]
    pub fn cells(mut self, cells: &[(ColumnKey, Rect)]) -> Self {
        self.cells = cells.to_vec();
        self
    }

    /// A toggle cell in the column `cell.column`; the collection flips it, the row shows it.
    #[must_use]
    pub fn toggle(mut self, cell: ToggleCell) -> Self {
        self.toggles.push(cell);
        self
    }

    /// An icon before the label. A thumbnail takes its place.
    #[must_use]
    pub fn icon(mut self, icon: IconSpec<'a>) -> Self {
        self.icon = Some(icon);
        self
    }

    /// A thumbnail of `size` before the label, framed just outside; `paint` draws into the thumbnail's rect.
    /// `ThumbSize::None` draws none.
    #[must_use]
    pub fn thumb(mut self, size: ThumbSize, paint: impl FnOnce(&ContentPainterCx, Rect) + 'a) -> Self {
        self.thumb = size;
        self.thumb_side = None;
        self.first = Some(Box::new(paint));
        self
    }

    /// A thumbnail `side` wide and high, for a gradient or other strip that is not square: its height sets the
    /// row's height as for any thumbnail.
    #[must_use]
    pub fn thumb_sized(mut self, side: Vec2, paint: impl FnOnce(&ContentPainterCx, Rect) + 'a) -> Self {
        self = self.thumb(ThumbSize::Px(side.y.round().clamp(0.0, f32::from(u16::MAX)) as u16), paint);
        self.thumb_side = Some(side);
        self
    }

    /// A thumbnail after the first and any earlier extra ones, of the same size and frame (a layer's mask). Extra
    /// thumbnails never drop out for width: the label is cut first. Nothing is drawn without a first thumbnail.
    #[must_use]
    pub fn extra_thumb(mut self, paint: impl FnOnce(&ContentPainterCx, Rect) + 'a) -> Self {
        self.extras.push(Thumb { paint: Some(Box::new(paint)), link: None });
        self
    }

    /// A link glyph before the extra thumbnail added last: linked when `linked`. A click reports
    /// [`RowEvent::LinkToggled`]. `name` is its name for a screen reader, a whole pattern with `{row}` for the
    /// row's label. Does nothing before an [`extra_thumb`](ListRow::extra_thumb).
    #[must_use]
    pub fn thumb_link(mut self, linked: bool, name: impl Into<Label>) -> Self {
        if let Some(thumb) = self.extras.last_mut() {
            thumb.link = Some((linked, name.into()));
        }
        self
    }

    /// The thumbnail the row's editing is aimed at: framed with corner brackets whether or not the row is selected.
    #[must_use]
    pub fn thumb_target(mut self, target: ThumbTarget) -> Self {
        self.target = Some(target);
        self
    }

    /// An item at the trailing end, `heights` control heights wide, that always shows. The first item added is the
    /// outermost.
    #[must_use]
    pub fn trailing(mut self, heights: f32, show: impl FnOnce(&mut PaintUi<'_>, Rect) + 'a) -> Self {
        self.trailing.push(Trailing { heights, optional: false, paint: Some(Box::new(show)) });
        self
    }

    /// An item at the trailing end that drops out when the label would get narrower than its least width with it.
    #[must_use]
    pub fn optional(mut self, heights: f32, show: impl FnOnce(&mut PaintUi<'_>, Rect) + 'a) -> Self {
        self.trailing.push(Trailing { heights, optional: true, paint: Some(Box::new(show)) });
        self
    }

    /// The row is at `depth` levels of a tree: indented by it, an `AccessKit` tree item.
    #[must_use]
    pub fn depth(mut self, depth: usize) -> Self {
        self.depth = Some(depth);
        self
    }

    /// The row is a tree group with children, `open` or closed: it shows the disclosure part.
    #[must_use]
    pub fn group(mut self, open: bool) -> Self {
        self.group = Some(open);
        self
    }
}
