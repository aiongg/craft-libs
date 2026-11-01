//! The outline bands of grouped rows and columns (N6.03).

use craft_ui::localized::{Labels as _, LocaleNumbers, fmt};
use craft_ui::theme::scale::Stroke as Line;
use craft_ui::theme::{StrokeExt as _, ToEgui as _, Tokens, TokensOf as _};
use egui::accesskit::Role;
use egui::{
    Align2, AsIdSalt, CornerRadius, FontId, IdSalt, Pos2, Rect, Response, Sense, Stroke, StrokeKind, TextStyle, Ui, Vec2, WidgetInfo, pos2, vec2,
};

use super::grid::{CellRange, CellRef, HEADER_MIN_ZOOM, SheetGeometry, column_letters};
use super::style::SheetStyle;

craft_ui::words! {
    /// The band buttons' accessible names, English by default (Excel's).
    #[non_exhaustive]
    pub struct OutlineBandLabels {
        level: "outline.level" = "Level {n}";
        collapse_rows: "outline.collapseRows" = "Hide Detail, rows {first}–{last}";
        expand_rows: "outline.expandRows" = "Show Detail, rows {first}–{last}";
        collapse_columns: "outline.collapseColumns" = "Hide Detail, columns {first}–{last}";
        expand_columns: "outline.expandColumns" = "Show Detail, columns {first}–{last}";
    }
}

/// The least sheet zoom a lane's width follows; below it lanes stay at this share of
/// `sheet.outline_lane` (GridCraft `outline.rs:44`). A proportion, not a size.
pub const LANE_MIN_ZOOM: f32 = 0.75;

/// A group of lines `start..=end` at `level` (1 = outermost), with its summary line after it
/// (below or right; before it when `summary_before`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutlineGroup {
    pub start: u32,
    pub end: u32,
    pub level: u8,
    pub collapsed: bool,
    pub summary_before: bool,
}

/// One direction's outline: its deepest level and its groups.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OutlineAxis {
    pub levels: u8,
    pub groups: Vec<OutlineGroup>,
}

/// What a band button does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutlineAction {
    /// Show levels 1…`level` (the corner buttons 1…n+1).
    Level { rows: bool, level: u8 },
    /// Collapse or expand a group (its − / + button).
    Toggle { rows: bool, group: OutlineGroup },
}

/// A lane's width at `zoom` (sheet zoom, 1.0 = 100 %): `lane × max(zoom, LANE_MIN_ZOOM)`,
/// rounded, `lane` being `sheet.outline_lane` ([`SheetStyle`](super::SheetStyle)).
// gridcraft: crates/ui-egui/src/outline.rs:44 `lane`.
pub fn lane(lane: f32, zoom: f32) -> f32 {
    (lane * zoom.max(LANE_MIN_ZOOM)).round()
}

/// Excel's outline bands (GridCraft): a strip left of the row headers and one above the column
/// headers, a lane per level; each expanded group a bracket along its lane from its first detail
/// line to its button, with a tick at the start; − / + on the summary line; the level buttons
/// 1…n+1 in the corner. Layout and hit tests are in logical coordinates and mirrored with the
/// grid. Buttons are Tab stops (level buttons first), Space presses; AccessKit `Button`s named by
/// [`OutlineBandLabels`] ("Level 2", "Hide Detail, rows 4–9", numbers through `LocaleNumbers`).
pub struct OutlineBand<'a> {
    id_salt: IdSalt,
    rows: &'a OutlineAxis,
    columns: &'a OutlineAxis,
    geometry: &'a SheetGeometry,
    zoom: f32,
    labels: Option<OutlineBandLabels>,
}

impl<'a> OutlineBand<'a> {
    pub fn new(id_salt: impl AsIdSalt, rows: &'a OutlineAxis, columns: &'a OutlineAxis, geometry: &'a SheetGeometry, zoom: f32) -> Self {
        Self { id_salt: IdSalt::new(id_salt), rows, columns, geometry, zoom, labels: None }
    }

    pub fn labels(mut self, labels: OutlineBandLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The band's thickness for an outline `levels` deep: `(levels + 1)` lanes ([`lane`] of
    /// `style.outline_lane`) plus [`BAND_SLACK_LANES`] of a lane; 0 without an outline.
    // gridcraft: crates/ui-egui/src/outline.rs:50 `band` (`(levels + 1) × lane + 3`).
    pub fn thickness(levels: u8, zoom: f32, style: &super::SheetStyle) -> f32 {
        if levels == 0 {
            return 0.0;
        }
        let step = lane(style.outline_lane, zoom);
        (f32::from(levels) + 1.0 + BAND_SLACK_LANES) * step
    }

    // gridcraft: crates/ui-egui/src/outline.rs:88 `Button`, `:96` `Bracket`, `:105` `Layout`,
    // `:113` `bands` and the layout below.
    pub fn show(self, ui: &mut Ui) -> OutlineBandOutput {
        let Self { id_salt, rows, columns, geometry, zoom, labels } = self;
        let labels = labels.unwrap_or_else(|| OutlineBandLabels::of(ui));
        let numbers = LocaleNumbers::of(ui);
        let tokens = Tokens::of(ui);
        let style = SheetStyle::of(&tokens);
        let id = ui.make_persistent_id(id_salt);
        let frame = Frame::new(geometry, rows, columns, zoom, &style);
        let response = ui.interact(geometry.mirror_rect(frame.bounds()), id, Sense::hover());
        let mut action = None;
        if frame.row_band.width() <= 0.0 && frame.column_band.height() <= 0.0 {
            return OutlineBandOutput { response, action };
        }
        let layout = frame.layout();

        let colors = &tokens.color;
        let ppp = ui.ctx().pixels_per_point();
        let hairline = Line::Hairline.width(ppp);
        let edge = Line::Hairline.egui(ui.ctx(), colors.line.border.to_egui());
        let ink = Stroke::new(Line::W1.width(ppp), colors.text.muted.to_egui());
        let painter = ui.painter().clone();
        for band in [frame.row_band, frame.column_band] {
            if band.width() > 0.0 && band.height() > 0.0 {
                painter.rect_filled(geometry.mirror_rect(band), CornerRadius::ZERO, colors.surface.background.to_egui());
            }
        }
        let mirrored = |a: Pos2, b: Pos2| [geometry.mirror_pos(a), geometry.mirror_pos(b)];
        if frame.row_band.width() > 0.0 {
            let (x, band) = (frame.row_band.right() - hairline / 2.0, frame.row_band);
            painter.line_segment(mirrored(pos2(x, band.top()), pos2(x, band.bottom())), edge);
        }
        if frame.column_band.height() > 0.0 {
            let (y, band) = (frame.column_band.bottom() - hairline / 2.0, frame.column_band);
            painter.line_segment(mirrored(pos2(band.left(), y), pos2(band.right(), y)), edge);
        }
        for bracket in &layout.brackets {
            let clip = if bracket.line[0].x == bracket.line[1].x { frame.row_clip() } else { frame.column_clip() };
            let clipped = painter.with_clip_rect(geometry.mirror_rect(clip));
            clipped.line_segment(mirrored(bracket.line[0], bracket.line[1]), ink);
            if let Some(tick) = bracket.tick {
                clipped.line_segment(mirrored(tick[0], tick[1]), ink);
            }
        }

        let base = TextStyle::Small.resolve(ui.style());
        let font = FontId::new(base.size * zoom.max(HEADER_MIN_ZOOM), base.family);
        // Tab order: the level buttons first, then the group buttons.
        let mut buttons: Vec<&Button> = layout.buttons.iter().collect();
        buttons.sort_by_key(|button| matches!(button.action, OutlineAction::Toggle { .. }));
        for button in buttons {
            let clip = match button.action {
                OutlineAction::Toggle { rows: true, .. } => frame.row_clip(),
                OutlineAction::Toggle { rows: false, .. } => frame.column_clip(),
                OutlineAction::Level { .. } => frame.bounds(),
            };
            let visible = button.rect.intersect(clip);
            if visible.width() <= 0.0 || visible.height() <= 0.0 {
                continue;
            }
            let name = button.name(&labels, &numbers);
            let button_id = id.with(button.key());
            let screen = geometry.mirror_rect(visible);
            let pressed = ui.interact(screen, button_id, Sense::click());
            pressed.widget_info(|| WidgetInfo::labeled(Role::Button, true, &name));
            if pressed.clicked() && action.is_none() {
                action = Some(button.action);
            }
            let clipped = painter.with_clip_rect(geometry.mirror_rect(clip));
            let face = geometry.mirror_rect(button.rect);
            clipped.rect_filled(face, CornerRadius::ZERO, colors.surface.chrome.to_egui());
            clipped.rect_stroke(face, CornerRadius::ZERO, ink, StrokeKind::Inside);
            clipped.text(face.center(), Align2::CENTER_CENTER, button.action.glyph(), font.clone(), colors.text.muted.to_egui());
            if pressed.has_focus() {
                let focus = Line::W2.egui(ui.ctx(), colors.line.focus.to_egui());
                clipped.rect_stroke(face, CornerRadius::ZERO, focus, StrokeKind::Outside);
            }
        }
        OutlineBandOutput { response, action }
    }
}

/// The room after the last lane, as a share of one lane: GridCraft's 3 points over its 15-point
/// lane, kept as a proportion so it follows `sheet.outline_lane`.
pub const BAND_SLACK_LANES: f32 = 0.2;

/// What the bands did this pass.
#[must_use]
pub struct OutlineBandOutput {
    pub response: Response,
    pub action: Option<OutlineAction>,
}

/// The button's side as a share of a lane: GridCraft's lane less 4 points, of 15 (`outline.rs:197`).
const BUTTON_SHARE: f32 = 11.0 / 15.0;

/// The first lane's inset from the band's edge as a share of a lane: GridCraft's 2 of 15
/// (`outline.rs:203`).
const INSET_SHARE: f32 = 2.0 / 15.0;

impl OutlineAction {
    /// The button's face: the level digit, − or +.
    fn glyph(self) -> String {
        match self {
            OutlineAction::Level { level, .. } => level.to_string(),
            OutlineAction::Toggle { group, .. } => if group.collapsed { "+" } else { "\u{2212}" }.to_string(),
        }
    }
}

/// A button in the band.
pub(super) struct Button {
    pub(super) rect: Rect,
    pub(super) action: OutlineAction,
}

impl Button {
    /// What the button is called to a screen reader.
    fn name(&self, labels: &OutlineBandLabels, numbers: &LocaleNumbers) -> String {
        match self.action {
            OutlineAction::Level { level, .. } => fmt(&labels.level, &[("n", &numbers.format(f64::from(level), 0..=0))]),
            OutlineAction::Toggle { rows, group } => {
                let (word, first, last) = match (rows, group.collapsed) {
                    (true, false) => (
                        &labels.collapse_rows,
                        numbers.format(f64::from(group.start) + 1.0, 0..=0),
                        numbers.format(f64::from(group.end) + 1.0, 0..=0),
                    ),
                    (true, true) => {
                        (&labels.expand_rows, numbers.format(f64::from(group.start) + 1.0, 0..=0), numbers.format(f64::from(group.end) + 1.0, 0..=0))
                    }
                    (false, false) => (&labels.collapse_columns, column_letters(group.start), column_letters(group.end)),
                    (false, true) => (&labels.expand_columns, column_letters(group.start), column_letters(group.end)),
                };
                fmt(word, &[("first", &first), ("last", &last)])
            }
        }
    }

    /// A stable key for the button's id.
    fn key(&self) -> (u8, bool, u32, u8) {
        match self.action {
            OutlineAction::Level { rows, level } => (0, rows, 0, level),
            OutlineAction::Toggle { rows, group } => (1, rows, group.start, group.level),
        }
    }
}

/// An expanded group's bracket: a line along its lane from the first detail line to its button,
/// with a tick at the start (none when the start is scrolled out of view).
struct Bracket {
    line: [Pos2; 2],
    tick: Option<[Pos2; 2]>,
}

#[derive(Default)]
pub(super) struct Layout {
    pub(super) buttons: Vec<Button>,
    brackets: Vec<Bracket>,
}

/// One pass's geometry, in logical coordinates: the bands and the two axes.
pub(super) struct Frame<'a> {
    geometry: &'a SheetGeometry,
    rows: &'a OutlineAxis,
    columns: &'a OutlineAxis,
    /// The lane's width, a button's side and the first lane's inset.
    step: f32,
    side: f32,
    inset: f32,
    row_band: Rect,
    column_band: Rect,
}

impl<'a> Frame<'a> {
    pub(super) fn new(geometry: &'a SheetGeometry, rows: &'a OutlineAxis, columns: &'a OutlineAxis, zoom: f32, style: &SheetStyle) -> Self {
        let step = lane(style.outline_lane, zoom);
        let band = Vec2::new(OutlineBand::thickness(rows.levels, zoom, style), OutlineBand::thickness(columns.levels, zoom, style));
        let cells = geometry.cells();
        let min = cells.min - geometry.header_size() - band;
        Self {
            geometry,
            rows,
            columns,
            step,
            side: step * BUTTON_SHARE,
            inset: step * INSET_SHARE,
            row_band: Rect::from_min_max(min, pos2(min.x + band.x, cells.bottom())),
            column_band: Rect::from_min_max(min, pos2(cells.right(), min.y + band.y)),
        }
    }

    /// Both bands.
    fn bounds(&self) -> Rect {
        self.row_band.union(self.column_band)
    }

    /// Brackets and group buttons stay out of the corner, where the level buttons are.
    fn row_clip(&self) -> Rect {
        Rect::from_min_max(pos2(self.row_band.left(), self.geometry.cells().top()), self.row_band.max)
    }

    fn column_clip(&self) -> Rect {
        Rect::from_min_max(pos2(self.geometry.cells().left(), self.column_band.top()), self.column_band.max)
    }

    /// Where `line` is on its axis, clipped to its pane: `None` when hidden or off screen.
    fn extent(&self, rows: bool, line: u32) -> Option<(f32, f32)> {
        let cell = CellRef::new(if rows { line } else { 0 }, if rows { 0 } else { line });
        let (rect, pane) = (self.geometry.cell_rect(cell), *self.geometry.range_panes(CellRange::cell(cell)).first()?);
        let (a, b, top, bottom) =
            if rows { (rect.top(), rect.bottom(), pane.top(), pane.bottom()) } else { (rect.left(), rect.right(), pane.left(), pane.right()) };
        let (from, to) = (a.max(top), b.min(bottom));
        (b > a && to > from).then_some((from, to))
    }

    fn start(&self, rows: bool, line: u32) -> f32 {
        if rows { self.geometry.y(line) } else { self.geometry.x(line) }
    }

    fn end(&self, rows: bool, line: u32) -> f32 {
        let cell = self.geometry.cell_rect(CellRef::new(if rows { line } else { 0 }, if rows { 0 } else { line }));
        if rows { cell.bottom() } else { cell.right() }
    }

    /// The first on-screen line of `start..=end`: `start` itself, else the lines from the first
    /// scrolling line on, so a long group isn't scanned from its far-off start.
    fn first_visible(&self, rows: bool, start: u32, end: u32) -> Option<(u32, (f32, f32))> {
        if let Some(extent) = self.extent(rows, start) {
            return Some((start, extent));
        }
        let ((first_row, _), (first_column, _)) = self.geometry.visible();
        (start.max(if rows { first_row } else { first_column })..=end).find_map(|line| self.extent(rows, line).map(|extent| (line, extent)))
    }

    fn point(rows: bool, along: f32, across: f32) -> Pos2 {
        if rows { pos2(across, along) } else { pos2(along, across) }
    }

    fn square(rows: bool, along: f32, across: f32, side: f32) -> Rect {
        Rect::from_center_size(Self::point(rows, along, across), vec2(side, side))
    }

    /// Lane `level`'s centre across (1-based).
    fn lane(&self, rows: bool, level: u8) -> f32 {
        let origin = if rows { self.row_band.left() } else { self.column_band.top() };
        origin + self.inset + (f32::from(level) - 0.5) * self.step
    }

    pub(super) fn layout(&self) -> Layout {
        let mut out = Layout::default();
        let cells = self.geometry.cells();
        let header = self.geometry.header_size();
        // The level buttons sit along the heading strip beside the corner, or the band without headings.
        let corner = Vec2::new(
            if header.x > 0.0 { header.x } else { self.row_band.width() },
            if header.y > 0.0 { header.y } else { self.column_band.height() },
        );
        for (rows, axis) in [(true, self.rows), (false, self.columns)] {
            if axis.levels == 0 {
                continue;
            }
            let level_along = if rows { cells.top() - corner.y / 2.0 } else { cells.left() - corner.x / 2.0 };
            for level in 1..=axis.levels + 1 {
                let rect = Self::square(rows, level_along, self.lane(rows, level), self.side);
                out.buttons.push(Button { rect, action: OutlineAction::Level { rows, level } });
            }
            let (lo, hi) = if rows { (cells.top(), cells.bottom()) } else { (cells.left(), cells.right()) };
            for group in &axis.groups {
                let across = self.lane(rows, group.level);
                let summary = if group.summary_before { group.start.saturating_sub(1) } else { group.end.saturating_add(1) };
                let has_summary = !(group.start..=group.end).contains(&summary);
                let button = has_summary.then(|| self.extent(rows, summary).map(|(a, b)| (a + b) / 2.0)).flatten();
                if let Some(mid) = button {
                    out.buttons
                        .push(Button { rect: Self::square(rows, mid, across, self.side), action: OutlineAction::Toggle { rows, group: *group } });
                }
                if !group.collapsed {
                    self.bracket(&mut out, rows, group, button, across, (lo, hi));
                }
            }
        }
        out
    }

    /// The bracket of one group's detail over the part that is on screen, and its tick.
    fn bracket(&self, out: &mut Layout, rows: bool, group: &OutlineGroup, button: Option<f32>, across: f32, (_, hi): (f32, f32)) {
        let Some((first_line, (from, _))) = self.first_visible(rows, group.start, group.end) else { return };
        let tick_length = self.side / 2.0;
        if group.summary_before {
            // From the button on to the last detail line, with the tick there.
            let from = button.map_or(from, |mid| mid + self.side / 2.0);
            let end = self.end(rows, group.end);
            let to = end.min(hi);
            if to <= from {
                return;
            }
            let tick = (end <= hi).then(|| [Self::point(rows, to, across), Self::point(rows, to, across + tick_length)]);
            out.brackets.push(Bracket { line: [Self::point(rows, from, across), Self::point(rows, to, across)], tick });
            return;
        }
        let to = match button {
            Some(mid) => mid - self.side / 2.0,
            None => self.start(rows, group.end.saturating_add(1)).min(hi),
        };
        if to <= from {
            return;
        }
        let start_shown = first_line == group.start && (from - self.start(rows, group.start)).abs() < self.step / 2.0;
        let tick = start_shown.then(|| [Self::point(rows, from, across), Self::point(rows, from, across + tick_length)]);
        out.brackets.push(Bracket { line: [Self::point(rows, from, across), Self::point(rows, to, across)], tick });
    }
}
