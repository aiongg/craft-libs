//! Collections on craft-ui-rt (docs/contract-rt.md section 15.5): the rows view, the band, the
//! row facts cells, the member rows' sub-variants and the reveal by kept extents.
//!
//! - **Rows view.** A collection's rows lie in one `ScrollNode` (the scroll body containers use)
//!   under the collection's node, with one content node holding the band rows in the layout's
//!   coordinates. A wheel step or a glide moves the content node in layout: nothing runs. When the
//!   view leaves the band the last build covered, the scroll node's miss cell marks it and the
//!   collection's build reaches its rows again.
//! - **Band.** The rows the layout puts in the view grown by the scroll body's overscan on each
//!   scrolling edge, from the kept `ItemLayout` extents. The item fn is called for these rows only.
//! - **Band rows** (`MEMBERS = false`) are leaf nodes under the content node, each keyed by the
//!   `Id` the collection gives it (`item_node`), so a row keeps its node while its key holds,
//!   whatever its position.
//! - **Row facts.** What the collection knows of a row (its flags, its place in the set, its cells'
//!   columns, a held press) lives in one cell per row, which the row's run reads (tracked). The
//!   collection writes a cell only when the row's facts change, so a selection change runs the
//!   rows whose facts moved and keeps the others. Hover is never a row fact: a row reads its own
//!   `Hovered` fact in paint, so a hover swaps that row's paint variant and runs nothing.
//! - **Member rows** (`MEMBERS = true`) are parts of the collection's node. Its paint records one
//!   sub-list per member row and pointer look the row's paint read; ui-rt's paint shows each row's
//!   sub-list for the look of that row's part, so a hover over a member row repaints the
//!   collection's node from kept lists and runs nothing.
//! - **Reveal.** A row out of the view is brought in by the scroll node's glide, aimed from the kept
//!   extents; the collection runs only to move its lead, and builds the rows the glide brings into
//!   the band.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use craft_behavior::{ItemLayout, ScrollPos};
use craft_theme::Tokens;
use craft_ui_rt::{BuildCx, DisplayList, NodeId, Pos2, Rect, State, Vec2};

use super::node::{Placed, Pointer, ScopeNode};
use super::scroll::{ScrollKept, ScrollNode, overscan};
use super::ui::{Flow, Ui};
use crate::contract::Scroll;
use crate::primitives::Id;
use crate::runtime::env::ItemFacts;

/// The child keys of a collection's rows view under its node.
#[derive(Hash)]
enum Region {
    Rows,
    Content,
}

/// Where a collection's rows stand in one build.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RowsView {
    /// The rows' view, in window coordinates.
    pub(crate) rect: Rect,
    /// The scroll offset the rows view holds.
    pub(crate) offset: ScrollPos,
    /// The rect rows are built for, in the layout's content coordinates: the view at the offset
    /// and the overscan beyond each scrolling edge; the whole content when the rows do not scroll.
    pub(crate) band: Rect,
}

impl RowsView {
    /// The view of rows at `rect` scrolled to `offset`; `scrolls` is false for rows shown whole.
    pub(crate) fn new(rect: Rect, offset: ScrollPos, scrolls: bool) -> RowsView {
        let local = Rect::from_min_size(Pos2::new(offset.x as f32, offset.y as f32), rect.size());
        let band = if scrolls { overscan(local) } else { Rect::EVERYTHING };
        RowsView { rect, offset, band }
    }

    /// Where the content's origin lies on the window: the view's corner less the offset.
    pub(crate) fn content_origin(&self) -> Pos2 {
        self.rect.min - Vec2::new(self.offset.x as f32, self.offset.y as f32)
    }

    /// The rows of `layout` in the band, in display order.
    pub(crate) fn band_rows<L: ItemLayout>(&self, layout: &L, out: &mut Vec<usize>) {
        if self.band == Rect::EVERYTHING {
            let size = layout.content_size();
            layout.visible(ScrollPos::default(), Vec2::new(size.x as f32, size.y as f32).max(self.rect.size()), out);
            return;
        }
        let at = ScrollPos { x: f64::from(self.band.min.x.max(0.0)), y: f64::from(self.band.min.y.max(0.0)) };
        let size = Vec2::new(self.band.max.x - at.x as f32, self.band.max.y - at.y as f32);
        layout.visible(at, size, out);
    }

    /// Row `at`'s rect on the window in this build: its kept extent placed at the content's origin.
    /// A row's place in the content does not depend on the offset, so a wheel step keeps it.
    pub(crate) fn row_rect<L: ItemLayout>(&self, layout: &L, at: usize) -> Rect {
        layout.rect(at, ScrollPos::default()).translate(self.content_origin().to_vec2())
    }

    /// Where the scroll node glides to show row `at` whole (`Flow::reveal`, window coordinates of
    /// the content position that comes to the view's start), from the kept extents; `None` when
    /// the row is already in view.
    pub(crate) fn reveal<L: ItemLayout>(&self, layout: &L, at: usize) -> Option<Pos2> {
        let row = layout.rect(at, ScrollPos::default());
        let view = Rect::from_min_size(Pos2::new(self.offset.x as f32, self.offset.y as f32), self.rect.size());
        if view.contains_rect(row) {
            return None;
        }
        // The least move that shows the row: its start at the view's start when it lies before the
        // view, its end at the view's end when it lies after.
        let mut to = view.min;
        if row.min.y < view.min.y || row.height() > view.height() {
            to.y = row.min.y;
        } else if row.max.y > view.max.y {
            to.y = row.max.y - view.height();
        }
        if row.min.x < view.min.x || row.width() > view.width() {
            to.x = row.min.x;
        } else if row.max.x > view.max.x {
            to.x = row.max.x - view.width();
        }
        Some(self.content_origin() + to.to_vec2())
    }

    /// Whether row `at` lies in the band the last build covered: a reveal of a row outside it builds
    /// the rows again once the glide brings it in.
    pub(crate) fn in_band<L: ItemLayout>(&self, layout: &L, at: usize) -> bool {
        self.band.intersects(layout.rect(at, ScrollPos::default()))
    }
}

/// The offset the rows view under the collection node `collection` holds, read without reaching
/// it: the read step lays the rows out at it. Zero before the first build.
pub(crate) fn kept_offset(cx: &BuildCx<'_>, collection: Option<NodeId>) -> ScrollPos {
    let Some(node) = collection else { return ScrollPos::default() };
    cx.peek::<ScrollNode>(node, Region::Rows).map_or(ScrollPos::default(), |p| {
        let offset = p.local.offset;
        ScrollPos { x: f64::from(offset.x), y: f64::from(offset.y) }
    })
}

/// The offset the collection's rows view holds, read from the `Ui` of the collection's own build
/// (its steps run in the collection node's build, as a leaf's measure does in `show_leaf`). A
/// measuring run reads zero: the collection's size does not depend on its offset.
pub(crate) fn rows_offset(ui: &Ui<'_, '_>) -> ScrollPos {
    if ui.flow.measuring.is_some() {
        return ScrollPos::default();
    }
    kept_offset(ui.cx, Some(ui.cx.node()))
}

/// Builds the band rows of a collection into its rows view: `build` shows each row at its
/// [`RowsView::row_rect`] on the `Ui` it is given, whose flow is the content node's. `extent` is the
/// layout's whole content size. A measuring run builds no row and returns `None`.
///
/// With `scroll` the view is the scroll body's: the content node moves by its offset in layout.
/// Without it the rows are shown whole in one content node.
pub(crate) fn rows_view<R>(
    ui: &mut Ui<'_, '_>,
    view: RowsView,
    scroll: Option<Scroll>,
    extent: Vec2,
    build: impl FnOnce(&mut Ui<'_, '_>, &RowFacts) -> R,
) -> Option<R> {
    if ui.flow.measuring.is_some() {
        return None;
    }
    let (id, tokens) = (ui.flow.id, ui.flow.tokens.clone());
    let Ui { cx, flow, .. } = ui;
    let (out, node) = match scroll {
        Some(scroll) => cx.keyed(Region::Rows, |cx| {
            cx.inline(ScrollNode { scroll }, |sk: &mut ScrollKept, _, cx| {
                sk.subscribe(cx);
                let at = RowsView { offset: ScrollPos { x: f64::from(sk.offset.x), y: f64::from(sk.offset.y) }, ..view };
                let (out, mut inner, content) = content_node(cx, id, at, extent, &tokens, build);
                sk.finish_build(cx, view.rect, content, &mut inner, scroll, &tokens);
                (out, cx.node())
            })
        }),
        None => {
            let (out, _, content) = content_node(cx, id, view, extent, &tokens, build);
            (out, content)
        }
    };
    let placed = Placed { node, offset: view.rect.min - flow.origin, size: view.rect.size() };
    match flow.placed.iter_mut().find(|p| p.node == node) {
        Some(p) => *p = placed,
        None => flow.placed.push(placed),
    }
    Some(out)
}

/// The content node: the band rows placed in content coordinates, the facts cells provided to them.
fn content_node<R>(
    cx: &mut BuildCx<'_>,
    id: Id,
    view: RowsView,
    extent: Vec2,
    tokens: &Arc<Tokens>,
    build: impl FnOnce(&mut Ui<'_, '_>, &RowFacts) -> R,
) -> (R, Flow, NodeId) {
    cx.keyed(Region::Content, |cx| {
        cx.inline(ScopeNode { key: 0 }, |ck, _, cx| {
            let origin = view.content_origin();
            let region = Rect::from_min_size(origin, extent.max(view.rect.size()));
            let band = if view.band == Rect::EVERYTHING { None } else { Some(view.band.translate(origin.to_vec2())) };
            let mut flow = Flow::new(id, origin, region, 0.0, tokens.clone()).in_view(band);
            let facts = RowFacts::of(cx);
            let mut ui = Ui::new(cx, flow);
            let out = build(&mut ui, &facts);
            let Ui { cx, flow: built, .. } = ui;
            flow = built;
            facts.drop_unshown();
            // The content is as large as the whole layout, though only the band is built: the scroll
            // body's bars, fades and clamp read the extent, and it records the band it covered.
            flow.extent = Rect::from_min_size(origin, extent);
            flow.limited = band.is_some();
            if ck.placed != flow.placed || ck.size != extent {
                ck.placed = flow.placed.clone();
                ck.size = extent;
                cx.request_layout();
            }
            (out, flow, cx.node())
        })
    })
}

/// The facts cells of a collection's band rows, kept on its content node and provided to the rows
/// under it. A row reads its own cell ([`row_facts`]); the collection writes a cell only when that
/// row's facts change.
#[derive(Clone, Default)]
pub(crate) struct RowFacts(Rc<RefCell<RowFactsMap>>);

#[derive(Default)]
struct RowFactsMap {
    cells: HashMap<Id, State<Arc<ItemFacts>>>,
    /// The rows given facts in the build that runs now.
    shown: Vec<Id>,
}

impl RowFacts {
    /// The content node's record, created at its first build and provided to the rows under it.
    fn of(cx: &mut BuildCx<'_>) -> RowFacts {
        let facts = cx.slot("row facts", RowFacts::default);
        let facts = cx.peek_state(facts).clone();
        facts.0.borrow_mut().shown.clear();
        cx.provide(facts.clone());
        facts
    }

    /// Sets row `id`'s facts before the row is shown: a change marks the row, which then runs in
    /// this build; the same facts mark nothing.
    pub(crate) fn put(&self, cx: &mut BuildCx<'_>, id: Id, facts: ItemFacts) {
        let mut map = self.0.borrow_mut();
        map.shown.push(id);
        match map.cells.get(&id).copied() {
            Some(cell) => {
                if **cx.peek_state(cell) != facts {
                    cell.set(cx, Arc::new(facts));
                }
            }
            None => {
                let cell = cx.slot(("row facts", id), || Arc::new(facts.clone()));
                if **cx.peek_state(cell) != facts {
                    cell.set(cx, Arc::new(facts));
                }
                map.cells.insert(id, cell);
            }
        }
    }

    /// Forgets the rows this build did not give facts: rows that left the band or the source. Their
    /// cells are the content node's slots and go with it; a row that comes back finds its slot again.
    fn drop_unshown(&self) {
        let mut map = self.0.borrow_mut();
        let RowFactsMap { cells, shown } = &mut *map;
        cells.retain(|id, _| shown.contains(id));
    }
}

/// The facts the collection gave the row `id` in the build that runs now, read so that the row's
/// node builds again when they change (the row steps' `EnvView::facts_of` on rt). `None` outside
/// a collection's rows.
pub(crate) fn row_facts(cx: &mut BuildCx<'_>, id: Id) -> Option<Arc<ItemFacts>> {
    let facts = cx.provided::<RowFacts>()?;
    let cell = facts.0.borrow().cells.get(&id).copied()?;
    Some(cell.get(cx))
}

/// One member row's recorded paint: its sub-list per pointer look its paint read (`Rest` always,
/// `Hover` and `Press` when read), keyed by the row's part in the collection's node.
#[derive(Clone, Debug)]
pub(crate) struct MemberLooks {
    /// The ui-rt part id of the row's `(ROW, row)` part.
    pub(crate) part: u64,
    pub(crate) looks: smallvec::SmallVec<[(Pointer, DisplayList); 1]>,
}

impl MemberLooks {
    /// The sub-list for `look`, falling back to `Rest` when the row's paint did not read it.
    pub(crate) fn list(&self, look: Pointer) -> Option<&DisplayList> {
        self.looks.iter().find(|(l, _)| *l == look).or_else(|| self.looks.first()).map(|(_, list)| list)
    }
}

/// The member rows' lists to paint for the pointer's place: each row shows its `Rest` list except
/// the row whose part `hovered` names (`Hover`, or `Press` when `pressed`). The collection node's
/// paint calls this with the hovered part of its node from ui-rt's hit record.
pub(crate) fn member_lists(rows: &[MemberLooks], hovered: Option<u64>, pressed: bool) -> impl Iterator<Item = &DisplayList> {
    rows.iter().filter_map(move |row| {
        let look = match hovered {
            Some(part) if part == row.part && pressed => Pointer::Press,
            Some(part) if part == row.part => Pointer::Hover,
            _ => Pointer::Rest,
        };
        row.list(look)
    })
}
