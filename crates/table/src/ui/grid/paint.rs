//! Painting and AccessKit nodes of the cell grid.

use craft_ui::theme::scale::Stroke as Line;
use craft_ui::theme::{StrokeExt as _, ToEgui as _, UiTokens};
use egui::accesskit::Role;
use egui::{Align2, Color32, FontId, Id, Painter, Pos2, Rect, Sense, Shape, Stroke, Ui, UiBuilder, Vec2, WidgetInfo};

use super::super::style::SheetStyle;
use super::{CellRange, CellRef, CellSelection, GridModel, GridState, HEADER_MIN_ZOOM, Pass, SheetGeometry, column_letters};

/// The colours and strokes the grid paints with, from the theme.
struct Look {
    background: Color32,
    header: Color32,
    header_text: Color32,
    header_selected: Color32,
    header_selected_text: Color32,
    header_whole: Color32,
    header_whole_text: Color32,
    gridline: Stroke,
    header_line: Stroke,
    accent: Stroke,
    fill: Color32,
    handle_fill: Color32,
    handle_stroke: Stroke,
    handle: f32,
    font: FontId,
}

impl Look {
    fn of(ui: &Ui, tokens: &UiTokens, style: &SheetStyle, zoom: f32) -> Self {
        let c = &tokens.color;
        let ctx = ui.ctx();
        let base = tokens.font(style.header_text);
        Self {
            background: c.surface.background.to_egui(),
            header: c.surface.chrome.to_egui(),
            header_text: c.text.muted.to_egui(),
            header_selected: c.state.selected.to_egui(),
            header_selected_text: c.text.on_selected.to_egui(),
            header_whole: c.accent.soft.to_egui(),
            header_whole_text: c.text.normal.to_egui(),
            gridline: Line::Hairline.egui(ctx, c.line.divider.to_egui()),
            header_line: Line::Hairline.egui(ctx, c.line.border.to_egui()),
            accent: Line::W2.egui(ctx, c.accent.solid.to_egui()),
            fill: style.selection_fill,
            handle_fill: style.fill_handle_fill,
            handle_stroke: Line::W1.egui(ctx, style.fill_handle_ring),
            handle: style.fill_handle,
            font: FontId::new(base.size * zoom.max(HEADER_MIN_ZOOM), base.family),
        }
    }
}

/// Lines `first..=last` shown over `from..to` on their axis.
#[derive(Clone, Copy)]
struct Span {
    first: u32,
    last: u32,
    from: f32,
    to: f32,
}

/// A frozen or scrolling block of cells: the part of the cells area a span of rows and of
/// columns share.
struct Pane {
    clip: Rect,
    rows: Span,
    columns: Span,
}

/// The frozen and scrolling spans of the rows and of the columns.
fn spans(geo: &SheetGeometry) -> (Vec<Span>, Vec<Span>) {
    let (frozen_rows, frozen_columns) = geo.layout.frozen;
    let (rows, columns) = geo.visible();
    let (cells, split) = (geo.cells(), geo.cells().min + geo.frozen_size);
    let row_spans = [
        (frozen_rows > 0).then(|| Span { first: 0, last: frozen_rows - 1, from: cells.top(), to: split.y }),
        (split.y < cells.bottom()).then_some(Span { first: rows.0, last: rows.1, from: split.y, to: cells.bottom() }),
    ];
    let column_spans = [
        (frozen_columns > 0).then(|| Span { first: 0, last: frozen_columns - 1, from: cells.left(), to: split.x }),
        (split.x < cells.right()).then_some(Span { first: columns.0, last: columns.1, from: split.x, to: cells.right() }),
    ];
    (row_spans.into_iter().flatten().collect(), column_spans.into_iter().flatten().collect())
}

fn panes(rows: &[Span], columns: &[Span]) -> Vec<Pane> {
    let mut panes = Vec::new();
    for &row in rows {
        for &column in columns {
            let clip = Rect::from_min_max(Pos2::new(column.from, row.from), Pos2::new(column.to, row.to));
            panes.push(Pane { clip, rows: row, columns: column });
        }
    }
    panes
}

/// Paints the grid: cells, gridlines, headers and selection; mirrored for a right-to-left sheet.
pub(super) fn paint<M: GridModel>(pass: &Pass<'_, M>, ui: &Ui, tokens: &UiTokens, state: &GridState, geo: &SheetGeometry) {
    let look = Look::of(ui, tokens, &pass.style, geo.layout.zoom);
    let painter = ui.painter().with_clip_rect(pass.rect.intersect(ui.clip_rect()));
    let first = painter.add(Shape::Noop);
    painter.rect_filled(geo.cells(), 0.0, look.background);
    let (rows, columns) = spans(geo);
    let panes = panes(&rows, &columns);
    for pane in &panes {
        let clipped = painter.with_clip_rect(pane.clip.intersect(painter.clip_rect()));
        paint_gridlines(&clipped, geo, pane, &look);
        paint_cells(pass, &clipped, geo, pane);
    }
    paint_selection(pass, &painter, geo, &state.selection, &look, state.editing);
    paint_headers(pass, &painter, geo, &state.selection, &look, (&rows, &columns));
    let last = painter.add(Shape::Noop);
    if pass.right_to_left {
        mirror(ui, &painter, first.0 + 1..last.0, geo);
    }
}

fn paint_gridlines(p: &Painter, geo: &SheetGeometry, pane: &Pane, look: &Look) {
    let clip = pane.clip;
    for row in pane.rows.first..=pane.rows.last {
        if geo.layout.rows.size(row) > 0.0 {
            let y = geo.y(row) + geo.layout.rows.size(row) as f32 * geo.layout.zoom;
            p.hline(clip.x_range(), y, look.gridline);
        }
    }
    for column in pane.columns.first..=pane.columns.last {
        if geo.layout.columns.size(column) > 0.0 {
            let x = geo.x(column) + geo.layout.columns.size(column) as f32 * geo.layout.zoom;
            p.vline(x, clip.y_range(), look.gridline);
        }
    }
}

/// The app paints every shown cell of the pane; a merged cell once, over its whole rect.
fn paint_cells<M: GridModel>(pass: &Pass<'_, M>, p: &Painter, geo: &SheetGeometry, pane: &Pane) {
    let area = CellRange::new(CellRef::new(pane.rows.first, pane.columns.first), CellRef::new(pane.rows.last, pane.columns.last));
    let merges: Vec<CellRange> = pass.model.merges().iter().copied().filter(|m| m.intersects(&area)).collect();
    for merge in &merges {
        pass.model.paint_cell(p, geo.range_rect(*merge), merge.start);
    }
    for row in pane.rows.first..=pane.rows.last {
        for column in pane.columns.first..=pane.columns.last {
            let cell = CellRef::new(row, column);
            let rect = geo.cell_rect(cell);
            if rect.width() > 0.0 && rect.height() > 0.0 && !merges.iter().any(|m| m.contains(cell)) {
                pass.model.paint_cell(p, rect, cell);
            }
        }
    }
}

/// `area` without `hole`: the strips above and below the hole, then what is left beside it, so a
/// translucent fill leaves the hole clear and the pieces never overlap.
fn rect_around(area: Rect, hole: Rect) -> Vec<Rect> {
    let hole = if hole.is_finite() { area.intersect(hole) } else { Rect::NOTHING };
    let parts = if hole.is_positive() {
        [
            Rect::from_min_max(area.min, Pos2::new(area.max.x, hole.min.y)),
            Rect::from_min_max(Pos2::new(area.min.x, hole.max.y), area.max),
            Rect::from_min_max(Pos2::new(area.min.x, hole.min.y), Pos2::new(hole.min.x, hole.max.y)),
            Rect::from_min_max(Pos2::new(hole.max.x, hole.min.y), Pos2::new(area.max.x, hole.max.y)),
        ]
    } else {
        [area, Rect::NOTHING, Rect::NOTHING, Rect::NOTHING]
    };
    parts.into_iter().filter(|r| r.is_positive() && r.is_finite()).collect()
}

/// The sides of `rect` that lie inside `clip`, as a stroke centred on the cell borders.
fn paint_outline(p: &Painter, rect: Rect, stroke: Stroke) {
    let clip = p.clip_rect();
    let (x, y) = (clip.x_range(), clip.y_range());
    let across = |from: f32, to: f32| (from.max(y.min), to.min(y.max));
    let along = |from: f32, to: f32| (from.max(x.min), to.min(x.max));
    let (top, bottom) = across(rect.top(), rect.bottom());
    let (left, right) = along(rect.left(), rect.right());
    for edge in [rect.left(), rect.right()] {
        if x.contains(edge) {
            p.vline(edge, top..=bottom, stroke);
        }
    }
    for edge in [rect.top(), rect.bottom()] {
        if y.contains(edge) {
            p.hline(left..=right, edge, stroke);
        }
    }
}

/// The selection over the cells: the ranges tinted with the active cell left clear, the current
/// range's outline and fill handle, the active cell's outline among several ranges.
fn paint_selection<M: GridModel>(pass: &Pass<'_, M>, p: &Painter, geo: &SheetGeometry, selection: &CellSelection, look: &Look, editing: bool) {
    let active = pass.space.merge_at(selection.active()).unwrap_or(CellRange::cell(selection.active()));
    let active_rect = geo.range_rect(active);
    let single = selection.ranges().len() == 1 && selection.is_single_cell(&pass.space);
    for &range in selection.ranges() {
        let rect = geo.range_rect(range);
        for clip in geo.range_panes(range) {
            let pane = p.with_clip_rect(clip.intersect(p.clip_rect()));
            if !single {
                for piece in rect_around(rect, active_rect) {
                    pane.rect_filled(piece.intersect(pane.clip_rect()), 0.0, look.fill);
                }
            }
            if range == selection.current() {
                paint_outline(&pane, rect, look.accent);
                if !editing {
                    let handle = Rect::from_center_size(rect.right_bottom(), Vec2::splat(look.handle));
                    pane.rect_filled(handle, 0.0, look.handle_fill);
                    pane.rect_stroke(handle, 0.0, look.handle_stroke, egui::StrokeKind::Inside);
                }
            }
        }
    }
    if selection.ranges().len() > 1 {
        for clip in geo.range_panes(active) {
            paint_outline(&p.with_clip_rect(clip.intersect(p.clip_rect())), active_rect, Stroke::new(look.accent.width / 2.0, look.accent.color));
        }
    }
}

/// Column and row headers, with the selected lines marked, and the select-all corner.
fn paint_headers<M: GridModel>(
    pass: &Pass<'_, M>,
    p: &Painter,
    geo: &SheetGeometry,
    selection: &CellSelection,
    look: &Look,
    (rows, columns): (&[Span], &[Span]),
) {
    let (cells, header) = (geo.cells(), geo.header_size());
    if header == Vec2::ZERO {
        return;
    }
    let top = Rect::from_min_max(Pos2::new(cells.left(), cells.top() - header.y), Pos2::new(cells.right(), cells.top()));
    let left = Rect::from_min_max(Pos2::new(cells.left() - header.x, cells.top()), Pos2::new(cells.left(), cells.bottom()));
    let corner = Rect::from_min_max(Pos2::new(cells.left() - header.x, cells.top() - header.y), cells.min);
    for area in [top, left, corner] {
        p.rect_filled(area, 0.0, look.header);
    }
    let (row_count, column_count) = (pass.space.row_count(), pass.space.column_count());
    for span in columns {
        let clip = p.with_clip_rect(Rect::from_x_y_ranges(span.from..=span.to, top.y_range()).intersect(p.clip_rect()));
        for column in span.first..=span.last {
            let rect = Rect::from_min_size(Pos2::new(geo.x(column), top.top()), Vec2::new(geo.cell_rect(CellRef::new(0, column)).width(), header.y));
            let shown = selection.ranges().iter().any(|r| r.columns().contains(&column));
            let whole = selection.ranges().iter().any(|r| r.start.row == 0 && r.end.row + 1 == row_count && r.columns().contains(&column));
            let label = pass.model.column_label(column);
            paint_header(&clip, rect, &label, (shown, whole), look);
            clip.vline(rect.right(), rect.y_range(), look.header_line);
        }
    }
    for span in rows {
        let clip = p.with_clip_rect(Rect::from_x_y_ranges(left.x_range(), span.from..=span.to).intersect(p.clip_rect()));
        for row in span.first..=span.last {
            let rect = Rect::from_min_size(Pos2::new(left.left(), geo.y(row)), Vec2::new(header.x, geo.cell_rect(CellRef::new(row, 0)).height()));
            let shown = selection.ranges().iter().any(|r| r.rows().contains(&row));
            let whole = selection.ranges().iter().any(|r| r.start.column == 0 && r.end.column + 1 == column_count && r.rows().contains(&row));
            paint_header(&clip, rect, &(row + 1).to_string(), (shown, whole), look);
            clip.hline(rect.x_range(), rect.bottom(), look.header_line);
        }
    }
    p.hline(corner.left()..=top.right(), cells.top(), look.header_line);
    p.vline(cells.left(), corner.top()..=left.bottom(), look.header_line);
    let (w, h) = (corner.width(), corner.height());
    let triangle = vec![
        Pos2::new(corner.right() - w * 0.2, corner.top() + h * 0.25),
        Pos2::new(corner.right() - w * 0.2, corner.bottom() - h * 0.15),
        Pos2::new(corner.left() + w * 0.35, corner.bottom() - h * 0.15),
    ];
    p.add(Shape::convex_polygon(triangle, look.header_text, Stroke::NONE));
}

fn paint_header(p: &Painter, rect: Rect, text: &str, (shown, whole): (bool, bool), look: &Look) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let (fill, color) = match (shown, whole) {
        (_, true) => (Some(look.header_whole), look.header_whole_text),
        (true, false) => (Some(look.header_selected), look.header_selected_text),
        (false, false) => (None, look.header_text),
    };
    if let Some(fill) = fill {
        p.rect_filled(rect, 0.0, fill);
    }
    p.text(rect.center(), Align2::CENTER_CENTER, text, look.font.clone(), color);
}

/// Reflects the shapes `range` of the painter's layer across the grid's centre line.
fn mirror(ui: &Ui, painter: &Painter, range: std::ops::Range<usize>, geo: &SheetGeometry) {
    let axis = geo.layout.rect.left() + geo.layout.rect.right();
    ui.ctx().graphics_mut(|graphics| {
        let list = graphics.entry(painter.layer_id());
        for index in range {
            list.mutate_shape(egui::layers::ShapeIdx(index), |clipped| {
                clipped.clip_rect = geo.mirror_rect(clipped.clip_rect);
                mirror_shape(&mut clipped.shape, axis);
            });
        }
    });
}

/// Reflects one shape across the vertical line `x = axis / 2`; text and images keep their
/// orientation and move.
fn mirror_shape(shape: &mut Shape, axis: f32) {
    let flip = |p: &mut Pos2| p.x = axis - p.x;
    match shape {
        Shape::Noop | Shape::Callback(_) => {}
        Shape::Vec(shapes) => shapes.iter_mut().for_each(|s| mirror_shape(s, axis)),
        Shape::Circle(c) => flip(&mut c.center),
        Shape::Ellipse(e) => flip(&mut e.center),
        Shape::LineSegment { points, .. } => points.iter_mut().for_each(flip),
        Shape::Path(path) => {
            path.points.iter_mut().for_each(flip);
            path.points.reverse();
        }
        Shape::Rect(r) => {
            r.rect = Rect::from_min_max(Pos2::new(axis - r.rect.right(), r.rect.top()), Pos2::new(axis - r.rect.left(), r.rect.bottom()));
            let c = r.corner_radius;
            r.corner_radius = egui::CornerRadius { nw: c.ne, ne: c.nw, sw: c.se, se: c.sw };
        }
        Shape::QuadraticBezier(b) => b.points.iter_mut().for_each(flip),
        Shape::CubicBezier(b) => b.points.iter_mut().for_each(flip),
        _ => {
            let bounds = shape.visual_bounding_rect();
            if bounds.is_finite() {
                shape.translate(Vec2::new(axis - bounds.right() - bounds.left(), 0.0));
            }
        }
    }
}

/// The AccessKit nodes of what is shown: the headers and rows of cells. They are created before
/// the grid's own widget so the grid, on top of them, takes the pointer and the focus.
pub(super) fn describe_cells<M: GridModel>(pass: &Pass<'_, M>, ui: &mut Ui, grid: Id, geo: &SheetGeometry) {
    let enabled = ui.is_enabled();
    let (rows, columns) = spans(geo);
    let header = geo.header_size();
    if header.y > 0.0 {
        let strip = Rect::from_min_size(Pos2::new(geo.cells().left(), geo.cells().top() - header.y), Vec2::new(geo.cells().width(), header.y));
        let row = row_ui(ui, grid, ("column headers", 0), strip);
        for span in &columns {
            for column in span.first..=span.last {
                let width = geo.cell_rect(CellRef::new(0, column)).width();
                let rect = Rect::from_min_size(Pos2::new(geo.x(column), strip.top()), Vec2::new(width, header.y));
                node(
                    &row,
                    geo.mirror_rect(rect),
                    grid.with(("column header", column)),
                    Role::ColumnHeader,
                    &pass.model.column_label(column),
                    enabled,
                );
            }
        }
    }
    for span in &rows {
        for r in span.first..=span.last {
            let height = geo.cell_rect(CellRef::new(r, 0)).height();
            if height <= 0.0 {
                continue;
            }
            let top = geo.y(r);
            let strip = Rect::from_min_size(Pos2::new(geo.cells().left() - header.x, top), Vec2::new(geo.cells().width() + header.x, height));
            let row = row_ui(ui, grid, ("row", r), strip);
            if header.x > 0.0 {
                let rect = Rect::from_min_size(strip.min, Vec2::new(header.x, height));
                node(&row, geo.mirror_rect(rect), grid.with(("row header", r)), Role::RowHeader, &(r + 1).to_string(), enabled);
            }
            for column_span in &columns {
                for c in column_span.first..=column_span.last {
                    describe_cell(
                        pass,
                        &row,
                        grid,
                        geo,
                        CellRef::new(r, c),
                        Rect::from_x_y_ranges(column_span.from..=column_span.to, span.from..=span.to),
                    );
                }
            }
        }
    }
}

fn describe_cell<M: GridModel>(pass: &Pass<'_, M>, row: &Ui, grid: Id, geo: &SheetGeometry, cell: CellRef, pane: Rect) {
    let rect = geo.cell_rect(cell);
    if rect.width() <= 0.0 {
        return;
    }
    let id = cell_id(grid, cell);
    let label = format!("{}{}", column_letters(cell.column), cell.row + 1);
    node(row, geo.mirror_rect(rect.intersect(pane)), id, Role::GridCell, &label, row.is_enabled());
    row.ctx().accesskit_node_builder(id, |n| {
        n.set_row_index(cell.row as usize);
        n.set_column_index(cell.column as usize);
        n.set_value(pass.model.cell_text(cell));
    });
}

/// The grid's own node: its size, and the active cell as the focused descendant when that cell
/// was described this pass (`described` is the geometry [`describe_cells`] ran on).
pub(super) fn describe_grid<M: GridModel>(pass: &Pass<'_, M>, ui: &Ui, grid: Id, state: &GridState, described: &SheetGeometry) {
    let active = state.selection.active();
    let (rows, columns) = spans(described);
    let shown = rows.iter().any(|r| (r.first..=r.last).contains(&active.row)) && columns.iter().any(|c| (c.first..=c.last).contains(&active.column));
    let visible = shown && described.cell_rect(active).width() > 0.0 && described.cell_rect(active).height() > 0.0;
    ui.ctx().accesskit_node_builder(grid, |n| {
        n.set_row_count(pass.space.row_count() as usize);
        n.set_column_count(pass.space.column_count() as usize);
        if visible {
            n.set_active_descendant(cell_id(grid, active).accesskit_id());
        }
    });
}

fn cell_id(grid: Id, cell: CellRef) -> Id {
    grid.with(("cell", cell.row, cell.column))
}

/// A child ui for an AccessKit row of the grid; it allocates nothing.
fn row_ui(ui: &mut Ui, grid: Id, salt: (&'static str, u32), rect: Rect) -> Ui {
    ui.new_child(UiBuilder::new().id_salt(salt).accessibility_parent(grid).accessibility_role(Role::Row).max_rect(rect))
}

/// A widget in `ui` that only names itself to AccessKit.
fn node(ui: &Ui, rect: Rect, id: Id, role: Role, label: &str, enabled: bool) {
    let response = ui.interact(rect, id, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(role, enabled, label));
}
