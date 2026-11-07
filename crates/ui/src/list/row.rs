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
use crate::contract::{
    Access, Bind, Common, Component, Cx, Focus as _, ItemOutput, MeasureCx, Name, NoValue, Outcome, PaintUi, PartsCx, Pointer as _, Props as _, Role,
    Styled as _, WordSetCx as _,
};
use crate::controls::check::Checkbox;
use crate::controls::disclosure::Disclosure;
use crate::primitives::{ContentPainterCx, Draw, IconSpec, PaintShape, RuleKind, Span, StrokeWidth, TextRun, Wrap};
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, Flags, Ink, StyleColor, StyleCx, SurfaceLook, UiTokens};
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

    fn of(self, height: f32) -> f32 {
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
            toggles: Vec::new(),
            icon: None,
            thumb: ThumbSize::None,
            thumb_aspect: ThumbAspect::SQUARE,
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
        }
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
    text: Stateful<StyleColor>,
    icon: Stateful<StyleColor>,
    detail: Stateful<StyleColor>,
    thumb_edge: StyleColor,
    thumb_radius: CornerRadius,
    role: TypeRole,
    sizes: Sizes,
    dividers: bool,
}

impl ListRow<'_> {
    /// The height of a row with a thumbnail of `thumb` and, if `detail`, a detail line, for the item layout of a
    /// list of such rows.
    pub fn row_height(cx: &StyleCx<'_>, thumb: ThumbSize, detail: bool) -> f32 {
        Sizes::of(cx, false).height(thumb, detail)
    }
}

/// Where the parts of a row lie in its rect.
struct Placed {
    toggles: Vec<(usize, Rect)>,
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

    /// The offsets from the reading start of the row of the cells that lead and that trail the label band.
    fn bands(&self, cells_of: &[(ColumnKey, Rect)], rect: Rect, rtl: bool) -> (f32, f32, Vec<(usize, Rect, bool)>) {
        let (start, width) = if rtl { (rect.right(), rect.width()) } else { (rect.left(), rect.width()) };
        let offset = |x: f32| if rtl { start - x } else { x - start };
        let (mut lead, mut trail) = (0.0_f32, 0.0_f32);
        let mut cells = Vec::new();
        for (at, toggle) in self.toggles.iter().enumerate() {
            let Some((_, cell)) = cells_of.iter().find(|(column, _)| *column == toggle.column) else { continue };
            let (a, b) = (offset(cell.left()), offset(cell.right()));
            let (near, far) = (a.min(b), a.max(b));
            let leading = near + far <= width;
            if leading {
                lead = lead.max(far);
            } else {
                trail = trail.max(width - near);
            }
            cells.push((at, *cell, leading));
        }
        (lead, trail, cells)
    }

    fn place(&self, sizes: &Sizes, items: &[(f32, bool)], rect: Rect, rtl: bool) -> Placed {
        self.place_in(&self.cells, sizes, items, rect, rtl)
    }

    fn place_in(&self, cells_of: &[(ColumnKey, Rect)], sizes: &Sizes, items: &[(f32, bool)], rect: Rect, rtl: bool) -> Placed {
        let (lead, trail, cells) = self.bands(cells_of, rect, rtl);
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
        let dividers = cells.iter().filter(|(_, _, leading)| *leading).map(|(_, cell, _)| if rtl { cell.left() } else { cell.right() }).collect();
        let min_width = lead + trail + base + column + leading_width + sizes.min_label + required + sizes.trailing_pad;
        Placed {
            toggles,
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
    const ALT_ROLES: &'static [Role] = &[Role::TREE_ITEM];
    type Style = ListRowStyle;
    type Output = RowOutput;

    fn style(cx: &StyleCx<'_>, key: ListRowStyleKey) -> ListRowStyle {
        let tokens = cx.tokens;
        let colors = &tokens.color;
        let (state, text) = (&colors.state, &colors.text);
        let clear = Rgba::TRANSPARENT;
        let stripe = if key.striped { state.hover.faded(STRIPE) } else { clear };
        let hover = if key.layer { over(colors.accent.solid.faded(LAYER_TINT), state.hover) } else { state.hover };
        let c = |rgba: Rgba| cx.color(rgba);
        let selected_hover = if key.layer { state.selected } else { state.selected_hover };
        let fill = Stateful::new(c(stripe))
            .off(States::new(c(stripe), c(over(hover, stripe)), c(over(state.pressed, stripe)), c(stripe)))
            .on(States::new(c(state.selected), c(selected_hover), c(selected_hover), c(state.selected)))
            .when(Flags::EMPHASIZED, c(over(hover, stripe)));
        let rounded = key.layer || tokens.shape.rows == RowStyle::Rounded;
        let radius = if rounded { UiTokens::corner(tokens.shape.radius.control) } else { CornerRadius::ZERO };
        let mut surface = SurfaceLook::new(fill).radius(radius);
        if key.layer {
            surface = surface.border(Stateful::new(c(clear)).when(Flags::SELECTED, c(colors.accent.solid)), tokens.stroke.thin);
        }
        let on_selected = c(text.on_selected);
        let ink = |base: Rgba| Stateful::new(c(base)).on(States::new(on_selected, on_selected, on_selected, on_selected));
        let base = match key.ink {
            RowInk::Normal => text.normal,
            RowInk::Muted => text.muted,
            RowInk::Dimmed => text.faint,
        };
        let sizes = Sizes::of(cx, key.strong);
        ListRowStyle {
            surface,
            metrics: Metrics::new().min_size(Vec2::new(0.0, sizes.row)).gap(sizes.gap),
            text: ink(base),
            icon: match key.tone {
                Tone::Neutral => ink(base),
                tone => Stateful::new(cx.tone(tone)).on(States::new(on_selected, on_selected, on_selected, on_selected)),
            },
            detail: ink(text.faint),
            thumb_edge: c(colors.line.border),
            thumb_radius: UiTokens::corner(tokens.shape.radius.sm),
            role: if key.strong { TypeRole::BodyStrong } else { TypeRole::Body },
            sizes,
            dividers: tokens.shape.rows == RowStyle::Divided,
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let sizes = &cx.style.sizes;
        if self.spacer {
            return Vec2::ZERO;
        }
        let height = sizes.height(self.thumb, self.detail.is_some());
        let rtl = cx.env.direction().is_rtl();
        let room = cx.room.unwrap_or(0.0);
        let toggles: f32 = self
            .toggles
            .iter()
            .filter_map(|toggle| self.cells.iter().find(|(column, _)| *column == toggle.column))
            .map(|(_, cell)| cell.width())
            .sum();
        let widths = self.item_widths(&|run| cx.layout_at(run, f32::INFINITY).size().x);
        let probe = self.place_in(&[], sizes, &widths, Rect::from_min_size(Pos2::ZERO, Vec2::new(room, height)), rtl);
        let probe_min = probe.min_width + toggles;
        let band = cx.room.map_or(f32::INFINITY, |_| (probe.label.width() - toggles).max(0.0));
        let wrapped = self.lines.as_ref().map_or(0.0, |text| {
            let run = TextRun::new(text.as_str()).role(TypeRole::Caption).wrap(Wrap::Wrap);
            sizes.gap + cx.layout_at(&run, band).size().y
        });
        let detail = if self.detail.is_some() { sizes.detail_line } else { 0.0 };
        let text = sizes.label_line + detail + wrapped;
        let height = if self.lines.is_some() { height.max(text + sizes.gap) } else { height };
        if cx.proposal == Proposal::Min {
            return Vec2::new(probe_min, height);
        }
        let label = cx.layout(&TextRun::new(cx.resolved.label).role(cx.style.role)).size().x;
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
        let by_key = cx.activation().filter(|activation| activation.by_key).and(cx.active_part());
        if let Some(open) = self.group
            && cx.part(DISCLOSURE).press.phase.is_click()
        {
            self.event = Some(RowEvent::Open { open: !open, all: cx.part(DISCLOSURE).press.mods.alt });
        }
        for (at, thumb) in self.extras.iter().enumerate() {
            let part = LINK.at(at as u64);
            if let Some((linked, _)) = &thumb.link
                && let Some(result) = cx.part_if_declared(part)
                && (result.press.phase.is_click() || by_key == Some(part))
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
            } else if phase.is_click() {
                self.event = Some(RowEvent::ThumbClicked(target));
            }
        }
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
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
        }
        if let Some(open) = self.group {
            access.expanded(open);
            let pattern = if open { &words.collapse } else { &words.expand };
            access.node(DISCLOSURE, Role::BUTTON).name = Name::Text(pattern.as_str().replace("{row}", label));
        }
        for (at, toggle) in self.toggles.iter().enumerate() {
            let role = if toggle.check_box { Role::CHECK_BOX } else { Role::BUTTON };
            access.node(TOGGLE.at(at as u64), role).name = Name::Text(toggle.name.as_str().replace("{row}", label));
        }
        for (at, thumb) in self.extras.iter().enumerate() {
            if let Some((_, name)) = &thumb.link {
                access.node(LINK.at(at as u64), Role::BUTTON).name = Name::Text(name.as_str().replace("{row}", label));
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
        let ink = cx.resolve(&style.text);
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
                icon.color.get_or_insert(cx.resolve(&style.icon));
                draw.icon(Rect::from_center_size(slot.center(), Vec2::splat(sizes.icon)), icon);
            }
        }
        self.paint_thumbs(ui, cx, &placed, ink);
        self.paint_text(&ui.draw(), cx, placed.label);
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
        let wrapped = self.lines.as_ref().map(|text| {
            let run = TextRun::new(text.as_str()).role(TypeRole::Caption).color(cx.resolve(&style.detail)).wrap(Wrap::Wrap);
            draw.layout(&run, label.width())
        });
        let detail = if self.detail.is_some() { sizes.detail_line } else { 0.0 };
        let lines = wrapped.as_ref().map_or(0.0, |layout| sizes.gap + layout.size().y);
        let block = sizes.label_line + detail + lines;
        let mut top = label.top() + ((label.height() - block) / 2.0).max(0.0);
        if !self.renaming {
            let mut span = Span::new(cx.resolved.label);
            span.italic = self.italic;
            let run = TextRun::spans(vec![span]).role(style.role).color(cx.resolve(&style.text)).elide();
            draw.text(Rect::from_min_size(Pos2::new(label.left(), top), Vec2::new(label.width(), sizes.label_line)), &run);
        }
        top += sizes.label_line;
        if let Some(detail) = &self.detail {
            let run = TextRun::new(detail.as_str()).role(TypeRole::Caption).color(cx.resolve(&style.detail)).elide();
            draw.text(Rect::from_min_size(Pos2::new(label.left(), top), Vec2::new(label.width(), sizes.detail_line)), &run);
            top += sizes.detail_line;
        }
        if let Some(layout) = wrapped {
            let x = if cx.env.direction().is_rtl() { label.right() - layout.size().x } else { label.left() };
            draw.galley(Pos2::new(x, top + sizes.gap), &layout);
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
struct RowThumbStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// The picture of a row's thumbnail, painted by the host through the content painter, which is clipped to the
/// thumbnail's rect.
#[derive(crate::contract::Component)]
#[component(content_painter, exempt(case = "face-only", reason = "A part of `ListRow`, shown by its stories."))]
struct RowThumb<'a> {
    #[common]
    common: Common,
    paint: Option<ThumbPaint<'a>>,
}

impl<'a> RowThumb<'a> {
    fn new(paint: ThumbPaint<'a>) -> Self {
        RowThumb { common: Common::new(), paint: Some(paint) }
    }
}

impl Component for RowThumb<'_> {
    const ROLE: Role = Role::None;
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
const FIXED_ADVANCE: f32 = 10.0;

/// Where the label, the trailing items and the disclosure of `row` lie in a rect of `width` under fixed sizes, where
/// each character of a trailing text is `FIXED_ADVANCE` wide: the geometry trials read it.
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
    };
    let items = row.item_widths(&|run| FIXED_ADVANCE * run.full_text().chars().count() as f32);
    let placed = row.place(&sizes, &items, Rect::from_min_size(Pos2::ZERO, Vec2::new(width, sizes.row)), rtl);
    RowPlacement { label: placed.label, items: placed.items, disclosure: placed.disclosure, min_label: sizes.min_label }
}

/// The indent steps of `row`.
pub(crate) fn indent_steps(row: &ListRow<'_>) -> usize {
    row.steps()
}

/// A row's parts placed by [`placement_under_fixed_sizes`].
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
