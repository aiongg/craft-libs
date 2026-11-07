//! `ListRow`: one row of a list, the item component of `List`, `FindingsList` and `EventLog`.
//!
//! In reading order a row has its toggle cells (in the columns the collection lays out), an indent and a
//! disclosure column for a tree, a clipping mark, an icon or thumbnails, the label with an optional detail line and
//! wrapped lines under it, and trailing items. The row mirrors in a right-to-left interface.
//!
//! Selection, the lead, hover, the drop mark, the drag ghost, the focus ring and the rename editor belong to the
//! collection that shows the row; the row draws the look the item step gives it and reports what its own parts do.

use super::toggle::{ToggleCell, ToggleFace, ToggleState};
use super::words::ListWords;
use crate::choice::Checkbox;
use crate::contract::{
    Access, Bind, ColumnAlign, ColumnSpec, Common, Component, Cx, ItemOutput, MeasureCx, Name, NoValue, Outcome, PaintUi, PartsCx, Pointer as _,
    Props as _, Role, RowState, Styled as _, WordSetCx as _,
};
use crate::controls::disclosure::Disclosure;
use crate::primitives::{ContentPainterCx, Draw, HAlign, IconSpec, PaintShape, RuleKind, Span, StrokeWidth, TextRun, Wrap};
use crate::runtime::env::{Label, WordTemplate};
use crate::style::{ComponentStyle, Flags, IconLook, Ink, StyleColor, StyleCx, SurfaceLook, TextLook, UiTokens};
use craft_behavior::{ColumnKey, Flow, IconId, Metrics, Mixed, NavShape, PartKey, PressPhase, Proposal, Roving, Sense};
use craft_theme::variant::Tone;
use craft_theme::{ControlSize, Rgba, RowStyle, Stateful, States, TypeRole, over};
use egui::{CornerRadius, Pos2, Rect, Response, Vec2};

/// The width of a thumbnail as a multiple of its height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThumbAspect(f32);

impl ThumbAspect {
    /// As wide as high.
    pub const SQUARE: Self = Self(1.0);

    /// `wide` parts of width to `high` parts of height; a zero part counts as a vanishing one.
    pub fn new(wide: u16, high: u16) -> Self {
        Self((f32::from(wide) / f32::from(high.max(1))).max(f32::EPSILON))
    }

    pub(crate) fn of(self, height: f32) -> f32 {
        self.0 * height
    }
}

/// The thumbnail setting of a list, which the host stores.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
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
        }
    }
}

/// One of a row's thumbnails.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
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
    #[non_exhaustive]
    Open {
        /// The state the click asks for.
        open: bool,
        /// The change reaches every descendant too (Alt held).
        all: bool,
    },
    /// The link glyph before an extra thumbnail was clicked: `linked` is the state it asks for.
    #[non_exhaustive]
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

impl Outcome for RowOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

impl ItemOutput<RowEvent> for RowOutput {
    fn into_event(self) -> Option<RowEvent> {
        self.event
    }

    fn label_rect(&self) -> Option<Rect> {
        Some(self.label)
    }
}

/// Paints into a thumbnail's rect through the content painter, which is clipped to it.
pub type ThumbPaint<'a> = Box<dyn FnOnce(&ContentPainterCx, Rect) + 'a>;

/// A thumbnail after the first: its painter and the link glyph before it.
pub(super) struct Thumb<'a> {
    pub paint: Option<ThumbPaint<'a>>,
    /// The state of the link glyph before an extra thumbnail and its name, a whole pattern with `{row}`.
    pub link: Option<(bool, Label)>,
}

/// A trailing item: a caption-role text as wide as it measures, and whether it drops out for room.
pub(super) struct Trailing {
    pub text: Label,
    pub optional: bool,
}

/// A text shown in one column of a row: the body role in the row's ink, on one elided line or wrapped.
pub(super) struct TextCell {
    pub column: ColumnKey,
    pub text: Label,
    pub wrap: bool,
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
    /// The row heads a group of rows: a wash under it, the title strong, the detail at the end of the same line and
    /// the disclosure of a group.
    #[style_key]
    #[prop(flag)]
    pub(super) heading: bool,
    /// Whether the row is its own AccessKit node; a face inside another component's part turns it off.
    #[prop]
    pub(super) node: bool,
    /// How strongly the label reads. A dimmed row wins over a muted one.
    #[style_key]
    #[prop]
    pub(super) ink: RowInk,
    /// The tone of the icon: the status colour for a severity, the row's ink for `Neutral`. A selected row's icon
    /// reads in the colour that reads on the selection.
    #[style_key]
    #[prop]
    pub(super) tone: Tone,
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
    /// The rename editor replaces the label; `state` sets it from the collection's row state.
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
    /// Each column's alignment and whether its figures are tabular, from `columns(..)`.
    pub(super) aligns: Vec<(ColumnKey, HAlign, bool)>,
    /// The columns that read in the muted look.
    pub(super) muted: Vec<ColumnKey>,
    pub(super) toggles: Vec<ToggleCell>,
    pub(super) texts: Vec<TextCell>,
    /// An icon before the label. A thumbnail takes its place.
    #[prop]
    pub(super) icon: Option<IconSpec<'a>>,
    pub(super) thumb: ThumbSize,
    pub(super) thumb_aspect: ThumbAspect,
    pub(super) first: Option<ThumbPaint<'a>>,
    pub(super) extras: Vec<Thumb<'a>>,
    /// The thumbnail the row's editing is aimed at: framed with corner brackets whether or not the row is selected.
    #[prop(name = "thumb_target")]
    pub(super) target: Option<ThumbTarget>,
    pub(super) trailing: Vec<Trailing>,
    /// The row's depth in a tree, which indents it; `None` for a row of a flat list.
    /// The row is at `depth` levels of a tree: indented by it, an `AccessKit` tree item.
    #[prop]
    pub(super) depth: Option<usize>,
    /// `Some(open)` for a tree row that has children, which shows the disclosure part.
    /// The row is a tree group with children, `open` or closed: it shows the disclosure part.
    #[prop]
    pub(super) group: Option<bool>,
    /// The row is an option of a listbox, not an item of a list.
    #[prop(flag)]
    pub(super) option: bool,
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
            heading: false,
            node: true,
            ink: RowInk::Normal,
            tone: Tone::Neutral,
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
            aligns: Vec::new(),
            muted: Vec::new(),
            toggles: Vec::new(),
            texts: Vec::new(),
            icon: None,
            thumb: ThumbSize::None,
            thumb_aspect: ThumbAspect::SQUARE,
            first: None,
            extras: Vec::new(),
            target: None,
            trailing: Vec::new(),
            depth: None,
            group: None,
            option: false,
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

    /// The rects of the row's columns, which the item context lists (`ItemCx::cells`): toggle cells and text cells
    /// draw in theirs and the rest of the row lays out in what they leave.
    #[must_use]
    pub fn cells(mut self, cells: &[(ColumnKey, Rect)]) -> Self {
        self.cells = cells.to_vec();
        self
    }

    /// The row state the collection gives the item: the rename editor, the stripe and the app's mark.
    #[must_use]
    pub fn state(mut self, row: RowState) -> Self {
        self.renaming = row.renaming;
        self.striped = row.striped;
        self.highlight |= row.emphasized;
        self
    }

    /// The columns' specs, for the alignment and the figures of the text cells: an end-aligned column's text ends
    /// at the cell's end (mirrored in a right-to-left interface) and a numeric column's figures are tabular.
    #[must_use]
    pub fn columns(mut self, specs: &[ColumnSpec]) -> Self {
        self.aligns = specs.iter().map(|spec| (spec.key.clone(), halign(spec.align), spec.tabular)).collect();
        self
    }

    /// The column `column` reads in the muted look: its text cell, or the label when it is the label column.
    #[must_use]
    pub fn cell_muted(mut self, column: ColumnKey) -> Self {
        self.muted.push(column);
        self
    }

    /// A toggle cell in the column `cell.column`; the collection flips it, the row shows it.
    #[must_use]
    pub fn toggle(mut self, cell: ToggleCell) -> Self {
        self.toggles.push(cell);
        self
    }

    /// `text` in the column `column` of `cells(..)`, in the body role and the row's ink on one line, elided. One call
    /// per text column; a column that `cells(..)` does not list (hidden, or a plain list) shows nothing. A press on
    /// it acts on the row.
    #[must_use]
    pub fn cell(mut self, column: ColumnKey, text: impl Into<Label>) -> Self {
        self.texts.push(TextCell { column, text: text.into(), wrap: false });
        self
    }

    /// Like `cell`, with the text wrapped to the column's width; the row is as high as its tallest wrapped cell.
    #[must_use]
    pub fn cell_lines(mut self, column: ColumnKey, text: impl Into<Label>) -> Self {
        self.texts.push(TextCell { column, text: text.into(), wrap: true });
        self
    }

    /// A thumbnail of `size` before the label, framed just outside; `paint` draws into the thumbnail's rect.
    /// `ThumbSize::None` draws none.
    #[must_use]
    pub fn thumb(mut self, size: ThumbSize, paint: impl FnOnce(&ContentPainterCx, Rect) + 'a) -> Self {
        self.thumb = size;
        self.thumb_aspect = ThumbAspect::SQUARE;
        self.first = Some(Box::new(paint));
        self
    }

    /// A thumbnail as high as `size` and `aspect` times as wide, for a gradient or other strip that is not square:
    /// its height sets the row's height as for any thumbnail. `ThumbSize::None` draws none.
    #[must_use]
    pub fn thumb_aspect(mut self, size: ThumbSize, aspect: ThumbAspect, paint: impl FnOnce(&ContentPainterCx, Rect) + 'a) -> Self {
        self = self.thumb(size, paint);
        self.thumb_aspect = aspect;
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

    /// A text at the trailing end, in the caption role and `text.muted`, as wide as it measures, that always shows.
    /// The first item added is the outermost.
    #[must_use]
    pub fn trailing(mut self, text: impl Into<Label>) -> Self {
        self.trailing.push(Trailing { text: text.into(), optional: false });
        self
    }

    /// A text at the trailing end that drops out when the label would get narrower than its least width with it.
    #[must_use]
    pub fn optional(mut self, text: impl Into<Label>) -> Self {
        self.trailing.push(Trailing { text: text.into(), optional: true });
        self
    }
}

const TOGGLE: PartKey = PartKey::new("toggle");
const TEXT_CELL: PartKey = PartKey::new("text-cell");
const TOGGLE_CELL: PartKey = PartKey::new("toggle-cell");
const LABEL_CELL: PartKey = PartKey::new("label-cell");
const DISCLOSURE: PartKey = PartKey::new("disclosure");
const LINK: PartKey = PartKey::new("link");
const THUMB: PartKey = PartKey::new("thumb");

/// How far the hover tint of the layer form tints toward the accent.
const LAYER_TINT: f32 = 0.12;
/// How far the stripe of a striped row tints toward the hover fill.
const STRIPE: f32 = 0.25;
/// The label's least width, in control heights, before the indent and the optional items give way.
const MIN_LABEL_HEIGHTS: f32 = 3.0;

/// The sizes a row takes from its control size and the tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Sizes {
    row: f32,
    icon: f32,
    pad: f32,
    gap: f32,
    trailing_pad: f32,
    min_label: f32,
    thumbs: [f32; 3],
    thumb_pad: f32,
    tight: f32,
    label_line: f32,
    detail_line: f32,
    /// The room a text cell keeps free at each side of its text.
    cell_inset: f32,
    /// The vertical room around a wrapped cell's text, in total.
    cell_pad: f32,
}

impl Sizes {
    fn of(cx: &StyleCx<'_>, strong: bool) -> Sizes {
        let (control, tokens) = (cx.control(), cx.tokens);
        let thumbs = &tokens.size.thumb;
        let role = if strong { TypeRole::BodyStrong } else { TypeRole::Body };
        Sizes {
            row: control.height,
            icon: control.icon,
            pad: control.pad_x,
            gap: control.gap,
            trailing_pad: tokens.space.pad.panel,
            min_label: MIN_LABEL_HEIGHTS * control.height,
            thumbs: [thumbs.sm, thumbs.md, thumbs.lg],
            thumb_pad: tokens.space.gap.stack,
            tight: tokens.space.gap.tight,
            label_line: tokens.typography.get(role).line_height,
            detail_line: tokens.typography.get(TypeRole::Caption).line_height,
            cell_inset: control.pad_x,
            cell_pad: control.gap,
        }
    }

    /// The width a text cell of `cell` width lays its text out in.
    fn cell_room(&self, cell: f32) -> f32 {
        (cell - 2.0 * self.cell_inset).max(0.0)
    }

    fn side(&self, thumb: ThumbSize) -> f32 {
        match thumb {
            ThumbSize::None => 0.0,
            ThumbSize::Small => self.thumbs[0],
            ThumbSize::Medium => self.thumbs[1],
            ThumbSize::Large => self.thumbs[2],
        }
    }

    /// The height of a row with a thumbnail of `thumb` and, if `detail`, a detail line: the control height, the
    /// thumbnail with its padding, or the two lines of text, whichever is tallest.
    fn height(&self, thumb: ThumbSize, detail: bool) -> f32 {
        let base = match self.side(thumb) {
            0.0 => self.row,
            side => (side + self.thumb_pad).max(self.row),
        };
        if detail { base.max(self.label_line + self.detail_line + self.gap) } else { base }
    }
}

/// The look of a row.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct ListRowStyle {
    #[style(surface)]
    surface: SurfaceLook,
    #[style(metrics)]
    metrics: Metrics,
    /// The label.
    #[style(text)]
    text: TextLook,
    /// The row's icon.
    #[style(icon)]
    icon: IconLook,
    /// The detail line.
    #[style(text)]
    detail: TextLook,
    /// A text cell.
    #[style(text)]
    cell: TextLook,
    /// A text cell of a column that reads muted.
    #[style(text)]
    cell_muted: TextLook,
    thumb_edge: StyleColor,
    thumb_radius: CornerRadius,
    sizes: Sizes,
    dividers: bool,
}

/// What the row fill depends on.
#[derive(Clone, Copy)]
struct FillKey {
    striped: bool,
    layer: bool,
    heading: bool,
}

/// The fill of a row: a stripe or the heading wash at rest, the hover and pressed tints over it, the selection's
/// fills, and the lead row's hover tint (selected hover on a selected row). An emphasized row keeps its hover tint.
fn fill_of(cx: &StyleCx<'_>, key: FillKey) -> Stateful<StyleColor> {
    let state = &cx.tokens.color.state;
    let clear = Rgba::TRANSPARENT;
    let rest = if key.heading {
        state.hover
    } else if key.striped {
        state.hover.faded(STRIPE)
    } else {
        clear
    };
    let hover = if key.layer { over(cx.tokens.color.accent.solid.faded(LAYER_TINT), state.hover) } else { state.hover };
    let c = |rgba: Rgba| cx.color(rgba);
    let tinted = over(hover, rest);
    let selected_hover = if key.layer { state.selected } else { state.selected_hover };
    let mut fill = Stateful::new(c(rest)).off(States::new(c(rest), c(tinted), c(over(state.pressed, rest)), c(rest))).on(States::new(
        c(state.selected),
        c(selected_hover),
        c(selected_hover),
        c(state.selected),
    ));
    for rule in flag_rules() {
        let rgba = match rule.fill {
            RuleFill::Tinted => tinted,
            RuleFill::SelectedHover => selected_hover,
        };
        fill = fill.when(rule.flags, c(rgba));
    }
    fill
}

/// Which fill a flag rule paints.
#[derive(Clone, Copy)]
enum RuleFill {
    Tinted,
    SelectedHover,
}

/// A flag rule of a row: the fill it paints and whether its ink is the selection's paired ink.
struct FlagRule {
    flags: Flags,
    fill: RuleFill,
    on_selected: bool,
}

/// The flag rules of a row in priority order. Fill and ink both derive from this list, so an ink always pairs with
/// the fill under it: an emphasized row keeps its hover-like fill and base ink, the lead row on a selected row
/// paints the selected hover fill with `text.on_selected`, and the lead row alone keeps the hover tint.
fn flag_rules() -> [FlagRule; 3] {
    [
        FlagRule { flags: Flags::EMPHASIZED, fill: RuleFill::Tinted, on_selected: false },
        FlagRule { flags: Flags::LEAD | Flags::SELECTED, fill: RuleFill::SelectedHover, on_selected: true },
        FlagRule { flags: Flags::LEAD, fill: RuleFill::Tinted, on_selected: false },
    ]
}

/// The fill of a row of a list or a menu: the one surface, the active (lead) row included.
pub(crate) fn row_surface(cx: &StyleCx<'_>) -> Stateful<StyleColor> {
    fill_of(cx, FillKey { striped: false, layer: false, heading: false })
}

/// The ink of a row's text or icon: `base`, and the theme's paired `text.on_selected` on the selection fills.
pub(crate) fn row_ink(cx: &StyleCx<'_>, base: Rgba) -> Stateful<StyleColor> {
    row_ink_of(cx, cx.color(base))
}

/// `row_ink` for a base that is already a style colour (a tone).
fn row_ink_of(cx: &StyleCx<'_>, base: StyleColor) -> Stateful<StyleColor> {
    let on_selected = cx.color(cx.tokens.color.text.on_selected);
    let mut ink = Stateful::new(base).on(States::new(on_selected, on_selected, on_selected, on_selected));
    for rule in flag_rules() {
        ink = ink.when(rule.flags, if rule.on_selected { on_selected } else { base });
    }
    ink
}

/// The name a pattern with `{row}` gives for the row labelled `row`.
fn named(pattern: &Label, row: &str) -> String {
    WordTemplate::new(pattern.as_str()).fill(&[("row", row)])
}

fn halign(align: ColumnAlign) -> HAlign {
    match align {
        ColumnAlign::Start => HAlign::Start,
        ColumnAlign::End => HAlign::End,
    }
}

impl ListRow<'_> {
    /// The height of a row with a thumbnail of `thumb` and, if `detail`, a detail line, for the item layout of a
    /// list of such rows.
    pub fn row_height(cx: &StyleCx<'_>, thumb: ThumbSize, detail: bool) -> f32 {
        Sizes::of(cx, false).height(thumb, detail)
    }
}

/// The cells of a row that lead and trail its label band.
struct Bands {
    lead: f32,
    trail: f32,
    /// Index into `toggles`, the cell's rect, whether it leads.
    toggles: Vec<(usize, Rect, bool)>,
    /// Index into `texts`, the column's position in the row's cells, the cell's rect, whether it leads.
    texts: Vec<(usize, usize, Rect, bool)>,
}

/// Where the parts of a row lie in its rect.
struct Placed {
    toggles: Vec<(usize, Rect)>,
    /// The text cells by index into `texts`, with the column's position in the row's cells.
    texts: Vec<(usize, usize, Rect)>,
    dividers: Vec<f32>,
    disclosure: Option<Rect>,
    marker: Option<Rect>,
    icon: Option<Rect>,
    first: Option<Rect>,
    extras: Vec<(Rect, Option<Rect>)>,
    label: Rect,
    items: Vec<Option<Rect>>,
    min_width: f32,
}

impl ListRow<'_> {
    /// Whether the detail takes a second line: a heading shows it at the end of the label's line.
    fn detail_line(&self) -> bool {
        self.detail.is_some() && !self.heading
    }

    /// Whether the leading part holds thumbnails; without a first thumbnail or a thumbnail side none are placed.
    fn shows_thumbs(&self, sizes: &Sizes) -> bool {
        self.first.is_some() && sizes.side(self.thumb) > 0.0
    }

    /// The width of the leading part: every thumbnail and what is between them, or the icon; `None` for a row with
    /// neither.
    fn leading(&self, sizes: &Sizes) -> Option<f32> {
        let side = sizes.side(self.thumb);
        if self.shows_thumbs(sizes) {
            let first = self.thumb_aspect.of(side);
            let between = |extra: &Thumb<'_>| if extra.link.is_some() { sizes.icon } else { sizes.gap };
            Some(first + self.extras.iter().map(|extra| first + between(extra)).sum::<f32>())
        } else {
            self.icon.as_ref().map(|_| sizes.icon)
        }
    }

    /// The thumbnail width and height of this row.
    fn thumb_size(&self, sizes: &Sizes) -> Vec2 {
        let side = sizes.side(self.thumb);
        Vec2::new(self.thumb_aspect.of(side), side)
    }

    /// The levels of indent: a leaf inside a group indents one more level, past its parent's disclosure column.
    fn steps(&self) -> usize {
        match (self.depth, self.group) {
            (Some(depth), None) if depth > 0 => depth + 1,
            (Some(depth), _) => depth,
            (None, _) => 0,
        }
    }

    /// The widths of the trailing items, as `text_width` measures their texts.
    fn item_widths(&self, text_width: &dyn Fn(&TextRun<'_>) -> f32) -> Vec<(f32, bool)> {
        self.trailing.iter().map(|item| (text_width(&trailing_run(item)), item.optional)).collect()
    }

    /// The indent a row asks for before it gives way to the label's least width.
    fn indent_full(&self, sizes: &Sizes) -> f32 {
        (self.steps() + usize::from(self.clipped)) as f32 * sizes.icon
    }

    /// The column no text or toggle cell claims, which holds the label.
    fn label_column<'c>(&self, cells_of: &'c [(ColumnKey, Rect)]) -> Option<&'c ColumnKey> {
        let claimed = |key: &ColumnKey| self.toggles.iter().any(|toggle| toggle.column == *key) || self.texts.iter().any(|text| text.column == *key);
        cells_of.iter().map(|(key, _)| key).find(|key| !claimed(key))
    }

    /// The offsets from the reading start of the row of the label column's near edge and of what lies past its
    /// far edge, and the toggle cells and text cells with whether each leads. A cell leads when it displays before
    /// the label column. A row without columns keeps the whole width for the label band; a row whose columns are all
    /// claimed has an empty label band and every cell leads.
    fn bands(&self, cells_of: &[(ColumnKey, Rect)], rect: Rect, rtl: bool) -> Bands {
        let start = if rtl { rect.right() } else { rect.left() };
        let span = |cell: Rect| {
            let (a, b) = (if rtl { start - cell.left() } else { cell.left() - start }, if rtl { start - cell.right() } else { cell.right() - start });
            (a.min(b), a.max(b))
        };
        let label_at = self.label_column(cells_of).and_then(|label| cells_of.iter().position(|(key, _)| key == label));
        let (lead, trail) = match label_at {
            Some(at) => {
                let (near, far) = span(cells_of[at].1);
                (near, (rect.width() - far).max(0.0))
            }
            None if cells_of.is_empty() => (0.0, 0.0),
            None => {
                let end = cells_of.iter().map(|(_, cell)| span(*cell).1).fold(0.0, f32::max);
                (end, (rect.width() - end).max(0.0))
            }
        };
        let leads = |position: usize| label_at.is_none_or(|at| position < at);
        let mut toggles = Vec::new();
        for (at, toggle) in self.toggles.iter().enumerate() {
            let Some(position) = cells_of.iter().position(|(column, _)| *column == toggle.column) else { continue };
            toggles.push((at, cells_of[position].1, leads(position)));
        }
        let mut texts = Vec::new();
        for (at, text) in self.texts.iter().enumerate() {
            let Some(position) = cells_of.iter().position(|(column, _)| *column == text.column) else { continue };
            texts.push((at, position, cells_of[position].1, leads(position)));
        }
        Bands { lead, trail, toggles, texts }
    }

    /// The alignment of a column's text and whether its figures are tabular.
    fn align_of(&self, column: &ColumnKey) -> (HAlign, bool) {
        self.aligns.iter().find(|(key, ..)| key == column).map_or((HAlign::Start, false), |(_, align, numeric)| (*align, *numeric))
    }

    /// The look of the text cell of `column`.
    fn cell_look<'s>(&self, style: &'s ListRowStyle, column: &ColumnKey) -> &'s TextLook {
        if self.muted.contains(column) { &style.cell_muted } else { &style.cell }
    }

    fn place(&self, sizes: &Sizes, items: &[(f32, bool)], rect: Rect, rtl: bool) -> Placed {
        self.place_in(&self.cells, sizes, items, rect, rtl)
    }

    fn place_in(&self, cells_of: &[(ColumnKey, Rect)], sizes: &Sizes, items: &[(f32, bool)], rect: Rect, rtl: bool) -> Placed {
        let Bands { lead, trail, toggles: cells, texts } = self.bands(cells_of, rect, rtl);
        let flow = Flow::new(rect, rtl);
        let band_width = (rect.width() - lead - trail).max(0.0);
        let band = flow.span(lead, band_width);
        let inner = Flow::new(band, rtl);
        let leading = self.leading(sizes);
        let leading_width = leading.map_or(0.0, |side| side + sizes.gap);
        let disclosure = if self.group.is_some() { sizes.icon } else { 0.0 };
        let marker = if self.clipped { sizes.icon } else { 0.0 };
        let column = disclosure + marker;
        let base = sizes.pad;
        let indent_full = self.indent_full(sizes);
        let end = band_width - sizes.trailing_pad;
        let required: f32 = items.iter().filter(|(_, optional)| !optional).map(|(width, _)| width + sizes.gap).sum();
        let mut used = required;
        let mut kept: Vec<bool> = items.iter().map(|(_, optional)| !optional).collect();
        for (at, (width, optional)) in items.iter().enumerate() {
            if *optional && end - (used + width + sizes.gap) - (base + indent_full + column + leading_width) >= sizes.min_label {
                kept[at] = true;
                used += width + sizes.gap;
            }
        }
        let label_end = end - used;
        let overflow = (base + column + leading_width + sizes.min_label - label_end).max(0.0);
        let label_end = label_end + overflow;
        let indent = indent_full.min((label_end - base - column - leading_width - sizes.min_label).max(0.0));
        let mut from_end = 0.0;
        let items = items
            .iter()
            .zip(&kept)
            .map(|((width, _), kept)| {
                kept.then(|| {
                    let start = end - from_end - width + overflow;
                    from_end += width + sizes.gap;
                    inner.span(start, *width)
                })
            })
            .collect();
        let column_start = base + indent + if self.leading_first { leading_width } else { 0.0 };
        let leading_start = base + indent + if self.leading_first { 0.0 } else { column };
        let label_start = base + indent + column + leading_width;
        let disclosure_rect = (disclosure > 0.0).then(|| inner.span(column_start, disclosure));
        let marker_rect = (marker > 0.0).then(|| inner.span(column_start + disclosure, marker));
        let mut at = leading_start;
        let (mut icon, mut first, mut extras) = (None, None, Vec::new());
        match leading {
            Some(_) if self.shows_thumbs(sizes) => {
                let size = self.thumb_size(sizes);
                let place = |x: f32, width: f32| Rect::from_center_size(inner.span(x, width).center(), Vec2::new(width, size.y));
                first = Some(place(at, size.x));
                at += size.x;
                for extra in &self.extras {
                    let link = extra.link.is_some().then(|| inner.span(at, sizes.icon));
                    at += if link.is_some() { sizes.icon } else { sizes.gap };
                    extras.push((place(at, size.x), link));
                    at += size.x;
                }
            }
            Some(width) => icon = Some(inner.span(at, width)),
            None => {}
        }
        let toggles = cells.iter().map(|(at, cell, _)| (*at, *cell)).collect();
        let texts = texts.iter().map(|(at, position, cell, _)| (*at, *position, *cell)).collect();
        let dividers = cells.iter().filter(|(_, _, leading)| *leading).map(|(_, cell, _)| if rtl { cell.left() } else { cell.right() }).collect();
        let min_width = lead + trail + base + column + leading_width + sizes.min_label + required + sizes.trailing_pad;
        Placed {
            toggles,
            texts,
            dividers,
            disclosure: disclosure_rect,
            marker: marker_rect,
            icon,
            first,
            extras,
            label: inner.span(label_start, (label_end - label_start).max(0.0)),
            items,
            min_width,
        }
    }
}

impl Component for ListRow<'_> {
    const ROLE: Role = Role::LIST_ITEM;
    const OVERLAYS: bool = false;
    const ALT_ROLES: &'static [Role] = &[Role::TREE_ITEM, Role::LIST_BOX_OPTION];
    type Style = ListRowStyle;
    type Output = RowOutput;

    fn style(cx: &StyleCx<'_>, key: ListRowStyleKey) -> ListRowStyle {
        let tokens = cx.tokens;
        let colors = &tokens.color;
        let text = &colors.text;
        let clear = Rgba::TRANSPARENT;
        let c = |rgba: Rgba| cx.color(rgba);
        let fill = fill_of(cx, FillKey { striped: key.striped, layer: key.layer, heading: key.heading });
        let rounded = key.layer || tokens.shape.rows == RowStyle::Rounded;
        let radius = if rounded { UiTokens::corner(tokens.shape.radius.control) } else { CornerRadius::ZERO };
        let mut surface = SurfaceLook::new(fill).radius(radius);
        if key.layer {
            surface = surface.border(Stateful::new(c(clear)).when(Flags::SELECTED, c(colors.accent.solid)), tokens.stroke.thin);
        }
        let ink = |base: Rgba| row_ink(cx, base);
        let base = match key.ink {
            RowInk::Normal => text.normal,
            RowInk::Muted => text.muted,
            RowInk::Dimmed => text.faint,
        };
        let weight = key.strong || key.heading;
        let sizes = Sizes::of(cx, weight);
        let muted = if key.ink == RowInk::Dimmed { text.faint } else { text.muted };
        let role = if weight { TypeRole::BodyStrong } else { TypeRole::Body };
        ListRowStyle {
            surface,
            metrics: Metrics::new().min_size(Vec2::new(0.0, sizes.row)).gap(sizes.gap),
            text: TextLook::new(role, ink(base)),
            icon: IconLook::new(
                sizes.icon,
                match key.tone {
                    Tone::Neutral => ink(base),
                    tone => row_ink_of(cx, cx.tone(tone)),
                },
            ),
            detail: TextLook::new(TypeRole::Caption, ink(text.faint)),
            cell: TextLook::new(role, ink(base)),
            cell_muted: TextLook::new(role, ink(muted)),
            thumb_edge: c(colors.line.border),
            thumb_radius: UiTokens::corner(tokens.shape.radius.sm),
            sizes,
            dividers: tokens.shape.rows == RowStyle::Divided,
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let sizes = &cx.style.sizes;
        if self.spacer {
            return Vec2::ZERO;
        }
        let height = sizes.height(self.thumb, self.detail_line());
        let rtl = cx.env.direction().is_rtl();
        let room = cx.room.unwrap_or(0.0);
        let columns = self.toggles.iter().map(|toggle| &toggle.column).chain(self.texts.iter().map(|text| &text.column));
        let toggles: f32 = columns.filter_map(|key| self.cells.iter().find(|(column, _)| column == key)).map(|(_, cell)| cell.width()).sum();
        let widths = self.item_widths(&|run| cx.layout_at(run, f32::INFINITY).size().x);
        let probe = self.place_in(&[], sizes, &widths, Rect::from_min_size(Pos2::ZERO, Vec2::new(room, height)), rtl);
        let probe_min = probe.min_width + toggles;
        let band = cx.room.map_or(f32::INFINITY, |_| (probe.label.width() - toggles).max(0.0));
        let wrapped = self.lines.as_ref().map_or(0.0, |text| {
            let run = TextRun::new(text.as_str()).role(cx.style.detail.role).wrap(Wrap::Wrap);
            sizes.gap + cx.layout_at(&run, band).size().y
        });
        let detail = if self.detail_line() { sizes.detail_line } else { 0.0 };
        let text = sizes.label_line + detail + wrapped;
        let height = if self.lines.is_some() { height.max(text + sizes.gap) } else { height };
        let height = self
            .texts
            .iter()
            .filter(|cell| cell.wrap)
            .filter_map(|cell| self.cells.iter().find(|(column, _)| *column == cell.column).map(|(_, rect)| (cell, rect)))
            .fold(height, |height, (cell, rect)| {
                let run = TextRun::new(cell.text.as_str()).role(cx.style.cell.role).wrap(Wrap::Wrap);
                height.max(cx.layout_at(&run, sizes.cell_room(rect.width())).size().y + sizes.cell_pad)
            });
        if cx.proposal == Proposal::Min {
            return Vec2::new(probe_min, height);
        }
        let label = cx.layout(&TextRun::new(cx.resolved.label).role(cx.style.text.role)).size().x;
        Vec2::new(probe_min - sizes.min_label + label.max(sizes.min_label) + self.indent_full(sizes), height)
    }

    fn sense(&self) -> Sense {
        if self.spacer { Sense::empty() } else { Sense::CLICK | Sense::FOCUS }
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        if self.spacer {
            return;
        }
        let widths = self.item_widths(&|run| cx.layout_at(run, f32::INFINITY).size().x);
        let placed = self.place(&cx.style.sizes, &widths, rect, cx.flow.is_rtl());
        for (at, cell) in &placed.toggles {
            let toggle = &self.toggles[*at];
            let mut flags = Flags::empty();
            flags.set(Flags::CHECKED, toggle.state == ToggleState::On);
            flags.set(Flags::MIXED, toggle.state == ToggleState::Mixed);
            let shape = if matches!(toggle.face, ToggleFace::Check) {
                Rect::from_center_size(cell.center(), Vec2::splat(cx.style.sizes.icon))
            } else {
                *cell
            };
            let part = cx.part(TOGGLE.at(*at as u64), shape).flags(flags);
            let _ = match &toggle.tooltip {
                Some(tip) => part.tooltip(tip.clone()),
                None => part,
            };
        }
        if let Some(disclosure) = placed.disclosure {
            cx.part(DISCLOSURE, disclosure).sense(Sense::CLICK).pointer_only("Left and Right open and close the row");
        }
        let mut focusable = false;
        for (at, (_, link)) in placed.extras.iter().enumerate() {
            if let Some(link) = link {
                cx.part(LINK.at(at as u64), *link).sense(Sense::CLICK | Sense::FOCUS).focusable();
                focusable = true;
            }
        }
        if focusable {
            cx.roving(Roving::new(NavShape::Flat));
        }
        let thumbs = placed.first.into_iter().chain(placed.extras.iter().map(|(thumb, _)| *thumb));
        for (at, thumb) in thumbs.enumerate() {
            let sense = if self.thumb_drags && at > 0 { Sense::CLICK | Sense::DRAG } else { Sense::CLICK };
            cx.part(THUMB.at(at as u64), thumb).sense(sense).pointer_only("A thumbnail is a pointer target; the keys act on the row");
        }
    }

    fn flags(&self) -> Flags {
        if self.highlight { Flags::EMPHASIZED } else { Flags::empty() }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if self.spacer {
            return;
        }
        if let Some(open) = self.group
            && cx.part_activated(DISCLOSURE).is_some()
        {
            self.event = Some(RowEvent::Open { open: !open, all: cx.part(DISCLOSURE).press.mods.alt });
        }
        for (at, thumb) in self.extras.iter().enumerate() {
            let part = LINK.at(at as u64);
            if let Some((linked, _)) = &thumb.link
                && cx.part_activated(part).is_some()
            {
                self.event = Some(RowEvent::LinkToggled { thumb: at, linked: !linked });
            }
        }
        for at in 0..=self.extras.len() {
            let Some(result) = cx.part_if_declared(THUMB.at(at as u64)) else { continue };
            let target = if at == 0 { ThumbTarget::First } else { ThumbTarget::Extra(at - 1) };
            let phase = result.press.phase;
            if matches!(phase, PressPhase::DragStart { .. }) && self.thumb_drags && at > 0 {
                self.event = Some(RowEvent::ThumbDrag(target));
            } else if cx.part_activated(THUMB.at(at as u64)).is_some() {
                self.event = Some(RowEvent::ThumbClicked(target));
            }
        }
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        if !self.node {
            access.role(Role::None);
            return;
        }
        if self.spacer {
            access.name(Name::Decorative);
            return;
        }
        let words = cx.words::<ListWords>();
        let label = cx.resolved.label;
        let name = match (&self.accessible_name, &self.detail) {
            (Some(name), _) => Name::Text(name.clone()),
            (None, Some(detail)) => Name::Joined(vec![label.to_owned(), detail.as_str().to_owned()]),
            (None, None) => Name::FromLabel,
        };
        access.name(name);
        let description = self.common.description.clone();
        match (self.clipped, description) {
            (true, Some(text)) => access.description(format!("{}, {text}", words.clipped.as_str())),
            (true, None) => access.description(words.clipped.as_str()),
            (false, Some(text)) => access.description(text),
            (false, None) => access,
        };
        if self.depth.is_some() {
            access.role(Role::TREE_ITEM);
        } else if self.option {
            access.role(Role::LIST_BOX_OPTION);
        }
        if let Some(open) = self.group {
            access.expanded(open);
            let pattern = if open { &words.collapse } else { &words.expand };
            access.node(DISCLOSURE, Role::BUTTON).name = Name::Text(named(pattern, label));
        }
        let selected =
            cx.resolved.selected == Some(true) || cx.env.facts_of(cx.resolved.id).is_some_and(|facts| facts.flags.contains(Flags::SELECTED));
        // The item step adds the row index; the column index is the cell's position among the row's columns.
        let column_of = |key: &ColumnKey| self.cells.iter().position(|(column, _)| column == key);
        if let Some(column) = self.label_column(&self.cells) {
            let node = access.node(LABEL_CELL, Role::CELL);
            node.name = Name::Text(label.to_owned());
            node.grid.column_index = column_of(column);
            node.flags.set(Flags::SELECTED, selected);
        }
        for (at, toggle) in self.toggles.iter().enumerate() {
            let cell = access.node(TOGGLE_CELL.at(at as u64), Role::CELL);
            cell.name = Name::Text(label.to_owned());
            cell.grid.column_index = column_of(&toggle.column);
            cell.flags.set(Flags::SELECTED, selected);
            let role = if toggle.check_box { Role::CHECK_BOX } else { Role::BUTTON };
            let node = access.node(TOGGLE.at(at as u64), role);
            node.name = Name::Text(named(&toggle.name, label));
            node.under(TOGGLE_CELL.at(at as u64));
        }
        for text in &self.texts {
            let Some(position) = self.cells.iter().position(|(column, _)| *column == text.column) else { continue };
            let node = access.node(TEXT_CELL.at(position as u64), Role::CELL);
            node.name = Name::Text(text.text.as_str().to_owned());
            node.grid.column_index = Some(position);
            node.flags.set(Flags::SELECTED, selected);
        }
        for (at, thumb) in self.extras.iter().enumerate() {
            if let Some((_, name)) = &thumb.link {
                access.node(LINK.at(at as u64), Role::BUTTON).name = Name::Text(named(name, label));
            }
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        if self.spacer {
            return;
        }
        let style = cx.style;
        let sizes = &style.sizes;
        let rtl = cx.env.direction().is_rtl();
        let rect = cx.rect;
        let widths = self.item_widths(&|run| ui.draw().layout(run, f32::INFINITY).size().x);
        let placed = self.place(sizes, &widths, rect, rtl);
        self.label_rect = placed.label;
        {
            let draw = ui.draw();
            if self.rule || (self.layer && style.dividers) {
                draw.rule(RuleKind::Divider, Pos2::new(rect.left(), rect.bottom()), Pos2::new(rect.right(), rect.bottom()));
            }
            if self.layer && style.dividers {
                for x in &placed.dividers {
                    draw.rule(RuleKind::Divider, Pos2::new(*x, rect.top()), Pos2::new(*x, rect.bottom()));
                }
            }
        }
        self.paint_toggles(ui, cx, &placed);
        let ink = cx.resolve(&style.text.color);
        if let Some(open) = self.group
            && placed.disclosure.is_some()
        {
            ui.face(DISCLOSURE, Disclosure::new(Bind::given(open)).node(false).label(cx.resolved.label.to_owned()));
        }
        {
            let draw = ui.draw();
            if let Some(slot) = placed.marker {
                draw_clip_arrow(&draw, slot, rtl, ink);
            }
            if let Some(slot) = placed.icon
                && let Some(icon) = self.icon.as_mut()
            {
                icon.color.get_or_insert(cx.resolve(&style.icon.color));
                draw.icon(Rect::from_center_size(slot.center(), Vec2::splat(sizes.icon)), icon);
            }
        }
        self.paint_thumbs(ui, cx, &placed, ink);
        self.paint_text(&ui.draw(), cx, placed.label);
        self.paint_cells(&ui.draw(), cx, &placed);
        for (item, slot) in self.trailing.iter().zip(placed.items) {
            if let Some(slot) = slot {
                ui.draw().text(slot, &trailing_run(item));
            }
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> RowOutput {
        RowOutput { response: cx.response.clone(), event: self.event, label: self.label_rect }
    }
}

impl ListRow<'_> {
    fn paint_toggles(&self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>, placed: &Placed) {
        let icon = cx.style.sizes.icon;
        for (at, cell) in &placed.toggles {
            let toggle = &self.toggles[*at];
            let part = TOGGLE.at(*at as u64);
            if let Some(fill) = toggle.fill {
                ui.draw().data_fill(&PaintShape::Rect(*cell, CornerRadius::ZERO), fill);
            }
            match &toggle.face {
                ToggleFace::Check => {
                    let value = match toggle.state {
                        ToggleState::Mixed => Mixed::Mixed,
                        state => Mixed::Same(state == ToggleState::On),
                    };
                    ui.face(part, Checkbox::new(Bind::given(value)).size(ControlSize::Sm));
                }
                ToggleFace::Icons { on, off } => {
                    let shown = match toggle.state {
                        ToggleState::On | ToggleState::Mixed => Some(on),
                        ToggleState::Off if toggle.always_visible || cx.part(part).hovered => Some(off),
                        ToggleState::Off => None,
                    };
                    if let Some(id) = shown {
                        let ink = if toggle.fill.is_some() { Ink::OnDark } else { Ink::OnPanel };
                        ui.draw().icon(Rect::from_center_size(cell.center(), Vec2::splat(icon)), &IconSpec::svg(id.clone()).ink(ink));
                    }
                }
            }
        }
    }

    fn paint_thumbs(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>, placed: &Placed, ink: StyleColor) {
        let style = cx.style;
        let outset = style.sizes.tight;
        let arm = style.sizes.gap;
        let rects: Vec<Rect> = placed.first.into_iter().chain(placed.extras.iter().map(|(thumb, _)| *thumb)).collect();
        let mut paints = self.first.take().into_iter().map(Some).chain(self.extras.iter_mut().map(|extra| extra.paint.take()));
        for (at, thumb) in rects.iter().enumerate() {
            if let Some(Some(paint)) = paints.next() {
                let _ = ui.child(("thumb", at), *thumb, RowThumb::new(paint));
            }
            let draw = ui.draw();
            let shape = PaintShape::Rect(*thumb, style.thumb_radius);
            draw.stroke(&shape, StrokeWidth::Thin, style.thumb_edge);
            let target = if at == 0 { ThumbTarget::First } else { ThumbTarget::Extra(at - 1) };
            if self.target == Some(target) {
                let r = thumb.expand(outset);
                for (corner, dx, dy) in
                    [(r.left_top(), 1.0, 1.0), (r.right_top(), -1.0, 1.0), (r.right_bottom(), -1.0, -1.0), (r.left_bottom(), 1.0, -1.0)]
                {
                    let path = vec![corner + Vec2::new(arm * dx, 0.0), corner, corner + Vec2::new(0.0, arm * dy)];
                    draw.stroke(&PaintShape::Path(path), StrokeWidth::Medium, ink);
                }
            }
        }
        for (at, (_, link)) in placed.extras.iter().enumerate() {
            let (Some(slot), Some((linked, _))) = (link, &self.extras[at].link) else { continue };
            let id = IconId::new(if *linked { "link" } else { "unlink" });
            let side = Rect::from_center_size(slot.center(), Vec2::splat(style.sizes.icon));
            ui.draw().icon(side, &IconSpec::svg(id));
        }
    }

    fn paint_text(&self, draw: &Draw<'_>, cx: &Cx<'_, Self>, label: Rect) {
        let style = cx.style;
        let sizes = &style.sizes;
        let rtl = cx.env.direction().is_rtl();
        let wrapped = self.lines.as_ref().map(|text| {
            let run = TextRun::new(text.as_str()).role(style.detail.role).color(cx.resolve(&style.detail.color)).wrap(Wrap::Wrap);
            draw.layout(&run, label.width())
        });
        let detail = if self.detail_line() { sizes.detail_line } else { 0.0 };
        let lines = wrapped.as_ref().map_or(0.0, |layout| sizes.gap + layout.size().y);
        let block = sizes.label_line + detail + lines;
        let mut top = label.top() + ((label.height() - block) / 2.0).max(0.0);
        // A heading's detail sits at the end of the label's line and the title is cut to what it leaves.
        let beside = match (&self.detail, self.heading) {
            (Some(detail), true) => {
                let run = TextRun::new(detail.as_str()).role(style.detail.role).color(cx.resolve(&style.detail.color)).elide();
                let shown = draw.layout(&run, label.width());
                let x = if rtl { label.left() } else { label.right() - shown.size().x };
                draw.galley(Pos2::new(x, top + (sizes.label_line - shown.size().y) / 2.0), &shown);
                shown.size().x + sizes.gap
            }
            _ => 0.0,
        };
        if !self.renaming {
            let muted = self.label_column(&self.cells).is_some_and(|column| self.muted.contains(column));
            let look = if muted { &style.cell_muted } else { &style.text };
            let mut span = Span::new(cx.resolved.label);
            span.italic = self.italic;
            let run = TextRun::spans(vec![span]).role(look.role).color(cx.resolve(&look.color)).elide();
            draw.text(Rect::from_min_size(Pos2::new(label.left(), top), Vec2::new((label.width() - beside).max(0.0), sizes.label_line)), &run);
        }
        top += sizes.label_line;
        if let Some(detail) = self.detail.as_ref().filter(|_| !self.heading) {
            let run = TextRun::new(detail.as_str()).role(style.detail.role).color(cx.resolve(&style.detail.color)).elide();
            draw.text(Rect::from_min_size(Pos2::new(label.left(), top), Vec2::new(label.width(), sizes.detail_line)), &run);
            top += sizes.detail_line;
        }
        if let Some(layout) = wrapped {
            let x = if rtl { label.right() - layout.size().x } else { label.left() };
            draw.galley(Pos2::new(x, top + sizes.gap), &layout);
        }
    }
}

impl ListRow<'_> {
    /// The text cells, inset and centred on the row's height, each aligned as its column says.
    fn paint_cells(&self, draw: &Draw<'_>, cx: &Cx<'_, Self>, placed: &Placed) {
        let sizes = &cx.style.sizes;
        let rtl = cx.env.direction().is_rtl();
        for (at, _, cell) in &placed.texts {
            let text = &self.texts[*at];
            let (align, numeric) = self.align_of(&text.column);
            let look = self.cell_look(cx.style, &text.column);
            let cell = Rect::from_x_y_ranges(cell.x_range(), cx.rect.y_range());
            let room = Flow::new(cell, rtl).span(sizes.cell_inset, sizes.cell_room(cell.width()));
            let mut span = Span::new(text.text.as_str());
            span.tabular = numeric;
            let run = TextRun::spans(vec![span]).role(look.role).color(cx.resolve(&look.color)).align(align);
            let run = if text.wrap { run.wrap(Wrap::Wrap) } else { run.elide() };
            let layout = draw.layout(&run, room.width());
            let top =
                if text.wrap { room.top() + ((room.height() - layout.size().y) / 2.0).max(0.0) } else { room.center().y - sizes.label_line / 2.0 };
            let free = (room.width() - layout.size().x).max(0.0);
            let inset = match align {
                HAlign::Start => 0.0,
                HAlign::Center => free / 2.0,
                HAlign::End => free,
            };
            let x = if rtl { room.right() - layout.size().x - inset } else { room.left() + inset };
            draw.galley(Pos2::new(x, top), &layout);
        }
    }
}

/// The run of a trailing item.
fn trailing_run(item: &Trailing) -> TextRun<'_> {
    TextRun::spans(vec![Span::new(item.text.as_str()).muted()]).role(TypeRole::Caption)
}

/// The down-right clipping mark: a stem down from the top, then a leg toward the reading end with an arrowhead.
fn draw_clip_arrow(draw: &Draw<'_>, slot: Rect, rtl: bool, ink: StyleColor) {
    let side = slot.width().min(slot.height());
    let square = Rect::from_center_size(slot.center(), Vec2::splat(side));
    let at = |x: f32, y: f32| Pos2::new(if rtl { square.right() - x * side } else { square.left() + x * side }, square.top() + y * side);
    draw.stroke(&PaintShape::Path(vec![at(0.2, 0.2), at(0.2, 0.7), at(0.8, 0.7)]), StrokeWidth::Medium, ink);
    draw.stroke(&PaintShape::Path(vec![at(0.6, 0.45), at(0.85, 0.7), at(0.6, 0.95)]), StrokeWidth::Medium, ink);
}

/// The style of a thumbnail: it has no look of its own.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
pub(crate) struct RowThumbStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// The picture of a row's thumbnail, painted by the host through the content painter, which is clipped to the
/// thumbnail's rect.
#[derive(crate::contract::Component)]
#[component(content_painter, sample = RowThumb::sample)]
pub(crate) struct RowThumb<'a> {
    #[common]
    common: Common,
    paint: Option<ThumbPaint<'a>>,
}

impl RowThumb<'static> {
    #[cfg(feature = "testing")]
    fn sample() -> Self {
        RowThumb::new(Box::new(|_, _| {}))
    }
}

impl<'a> RowThumb<'a> {
    pub(crate) fn new(paint: ThumbPaint<'a>) -> Self {
        RowThumb { common: Common::new(), paint: Some(paint) }
    }
}

impl Component for RowThumb<'_> {
    const ROLE: Role = Role::None;
    const PARTS: bool = false;
    const STOPS: bool = false;
    const OVERLAYS: bool = false;
    type Style = RowThumbStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: RowThumbStyleKey) -> RowThumbStyle {
        RowThumbStyle::default()
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let painter = ui.content_painter(cx);
        if let Some(paint) = self.paint.take() {
            paint(&painter, cx.rect);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

/// The width of a character of a trailing text under fixed sizes.
#[cfg(feature = "testing")]
const FIXED_ADVANCE: f32 = 10.0;

/// Where the label, the trailing items and the disclosure of `row` lie in a rect of `width` under fixed sizes, where
/// each character of a trailing text is `FIXED_ADVANCE` wide: the geometry trials read it.
#[cfg(feature = "testing")]
pub(crate) fn placement_under_fixed_sizes(row: &ListRow<'_>, width: f32, rtl: bool) -> RowPlacement {
    let sizes = Sizes {
        row: 20.0,
        icon: 16.0,
        pad: 6.0,
        gap: 4.0,
        trailing_pad: 8.0,
        min_label: 60.0,
        thumbs: [24.0, 32.0, 48.0],
        thumb_pad: 4.0,
        tight: 2.0,
        label_line: 16.0,
        detail_line: 12.0,
        cell_inset: 6.0,
        cell_pad: 4.0,
    };
    let items = row.item_widths(&|run| FIXED_ADVANCE * run.full_text().chars().count() as f32);
    let placed = row.place(&sizes, &items, Rect::from_min_size(Pos2::ZERO, Vec2::new(width, sizes.row)), rtl);
    RowPlacement { label: placed.label, items: placed.items, disclosure: placed.disclosure, min_label: sizes.min_label }
}

/// The indent steps of `row`.
#[cfg(feature = "testing")]
pub(crate) fn indent_steps(row: &ListRow<'_>) -> usize {
    row.steps()
}

/// A row's parts placed by [`placement_under_fixed_sizes`].
#[cfg(feature = "testing")]
#[non_exhaustive]
pub struct RowPlacement {
    /// The label and detail.
    pub label: Rect,
    /// The trailing items, `None` for one that dropped out.
    pub items: Vec<Option<Rect>>,
    /// The disclosure column of a group row.
    pub disclosure: Option<Rect>,
    /// The label's least width under the fixed sizes.
    pub min_label: f32,
}
