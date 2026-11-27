//! `List`: the layer list. A virtual column of keyed `ListRow`s in a `Scroll`, with the list's
//! own rules on top: picks, the lead row as the one Tab stop, keyboard moves, drag reorder with
//! its drop mark and edge scroll, touch drag-scroll and the rename lifecycle. The rules are
//! `list-rules`; the list applies them to the document cell.

use std::ops::Range;

use crate::fixture::LayerId;
use crate::list_rules::{Lead, ListOp, Pick, PressPlan};
use accesskit::{Action, Node, Role};
use craft_host::target::Os;
use craft_ui_rt::{
    AccessCx, BuildCx, Constraints, Element, Event, EventCx, Gesture, Handled, KeyInput, LayoutCx, NodeId, PaintCx, PointerInput, PointerKind, Pos2,
    Sense, State, Vec2, virtual_band,
};

use crate::doc::{DocCell, edit, renamable};
use crate::look::{Look, OVERSCAN_ROWS};
use crate::row::{ListRow, PART_NAME, RowProps};
use crate::scroll::{Extent, Scroll};

/// The hit part of the list's own area below the rows.
const PART_EMPTY: u64 = 100;

pub struct List {
    pub doc: DocCell,
    pub os: Os,
}

/// A press that went down on a row.
#[derive(Clone, Copy)]
struct Press {
    row: usize,
    plan: PressPlan,
    renames: bool,
}

pub struct ListLocal {
    offset: State<f32>,
    lead: State<Lead>,
    rename: State<Option<LayerId>>,
    band: State<Range<usize>>,
    /// The gap a running reorder drag would land in.
    drop: State<Option<usize>>,
    press: Option<Press>,
    /// The rows a reorder drag carries.
    drag: Option<Vec<usize>>,
    scroll_drag: bool,
    /// The pointer during a drag, list coordinates.
    pointer: Option<Pos2>,
    alt: bool,
    last_time: f64,
    row_h: f32,
}

impl List {
    fn selected(doc: &crate::fixture::Doc) -> Vec<usize> {
        doc.layers.iter().enumerate().filter(|(_, l)| doc.is_selected(l.id)).map(|(i, _)| i).collect()
    }

    fn apply(&self, l: &ListLocal, cx: &mut EventCx<'_>, op: ListOp) {
        let doc = self.doc.get(cx);
        match op {
            ListOp::Rename(row) => {
                if let Some(layer) = doc.layers.get(row).filter(|layer| renamable(layer)) {
                    l.rename.set(cx, Some(layer.id));
                }
            }
            ListOp::Pick(pick) => {
                let mut lead = l.lead.get(cx);
                match pick {
                    Pick::Replace(row) | Pick::Toggle(row) => {
                        lead.lead = Some(row);
                        lead.anchor = Some(row);
                    }
                    Pick::Range(row) => lead.lead = Some(row),
                    Pick::Clear => {}
                }
                l.lead.set_if_changed(cx, lead);
                if let Some(op) = crate::list_rules::doc_op(&doc, ListOp::Pick(pick)) {
                    edit(self.doc, cx, op);
                }
            }
            op => {
                if let Some(op) = crate::list_rules::doc_op(&doc, op) {
                    // The lead and anchor stay on their layers when rows move, as the list keys its rows by
                    // identity; a row that went away leaves them at its place.
                    let lead = l.lead.get(cx);
                    let id = |row: Option<usize>| row.and_then(|r| doc.layers.get(r)).map(|layer| layer.id);
                    let held = (id(lead.lead), id(lead.anchor));
                    edit(self.doc, cx, op);
                    let after = self.doc.get(cx);
                    let place = |held: Option<LayerId>, row: Option<usize>| held.and_then(|id| after.index_of(id)).or(row);
                    l.lead.set_if_changed(cx, Lead { lead: place(held.0, lead.lead), anchor: place(held.1, lead.anchor) });
                }
            }
        }
    }

    fn set_lead(&self, l: &ListLocal, cx: &mut EventCx<'_>, row: usize) {
        let mut lead = l.lead.get(cx);
        lead.lead = Some(row);
        l.lead.set_if_changed(cx, lead);
    }

    fn max_offset(&self, l: &ListLocal, cx: &mut EventCx<'_>) -> f32 {
        let len = self.doc.get(cx).layers.len();
        (len as f32 * l.row_h - cx.bounds().height()).max(0.0)
    }

    fn update_drop(&self, l: &ListLocal, cx: &mut impl craft_ui_rt::Writes) {
        let Some(p) = l.pointer else { return };
        let (len, offset) = {
            let store = cx.store_mut();
            (self.doc.with(store, |d| d.layers.len()), *store.peek(l.offset))
        };
        let insert = crate::list_rules::drop_insert(len, l.row_h as f32, (p.y + offset) as f32);
        l.drop.set_if_changed(cx, insert);
    }

    fn pointer(&self, l: &mut ListLocal, cx: &mut EventCx<'_>, p: &PointerInput) -> Handled {
        let at = cx.to_local(p.pos);
        match p.gesture {
            Gesture::Down { clicks } => {
                let primary = p.button == craft_ui_rt::PointerButton::Primary;
                let doc = self.doc.get(cx);
                let tagged = p.hit.and_then(|h| cx.tag_below(h.node)).map(|(_, tag)| tag - 1);
                let row = tagged.and_then(|id| doc.index_of(id));
                let Some(row) = row else {
                    if primary && let Some(pick) = crate::list_rules::press_empty(!doc.selection.is_empty()) {
                        self.apply(l, cx, ListOp::Pick(pick));
                    }
                    return Handled::Yes;
                };
                let held = doc.is_selected(doc.layers[row].id);
                if !primary {
                    if !held {
                        self.apply(l, cx, ListOp::Pick(Pick::Replace(row)));
                    }
                    self.set_lead(l, cx, row);
                    return Handled::Yes;
                }
                let plan = crate::list_rules::press_pick(row, held, p.mods, clicks);
                if let Some(pick) = plan.now {
                    self.apply(l, cx, ListOp::Pick(pick));
                }
                let in_name = p.hit.is_some_and(|h| h.part == PART_NAME);
                l.press = Some(Press { row, plan, renames: crate::list_rules::names(clicks, in_name) });
                self.set_lead(l, cx, row);
                Handled::Yes
            }
            Gesture::DragStart { long_pressed, .. } => {
                let Some(press) = l.press else { return Handled::Yes };
                l.pointer = Some(at);
                if p.kind == PointerKind::Touch && !long_pressed {
                    l.scroll_drag = true;
                    return Handled::Yes;
                }
                let doc = self.doc.get(cx);
                let held = doc.is_selected(doc.layers[press.row].id);
                l.drag = Some(if held { Self::selected(&doc) } else { vec![press.row] });
                l.alt = p.mods.alt;
                l.last_time = cx.time();
                cx.request_layout();
                self.update_drop(l, cx);
                Handled::Yes
            }
            Gesture::Drag { delta } => {
                l.pointer = Some(at);
                l.alt = p.mods.alt;
                if l.scroll_drag {
                    let max = self.max_offset(l, cx);
                    let offset = l.offset.get(cx);
                    l.offset.set_if_changed(cx, (offset - delta.y).clamp(0.0, max));
                } else if l.drag.is_some() {
                    self.update_drop(l, cx);
                }
                Handled::Yes
            }
            Gesture::Up { click, .. } => {
                if let Some(rows) = l.drag.take() {
                    let insert = l.drop.get(cx);
                    l.drop.set_if_changed(cx, None);
                    let len = self.doc.get(cx).layers.len();
                    let copy = p.mods.alt || l.alt;
                    if let Some(insert) = insert
                        && let Some((target, after)) = crate::list_rules::place(len, &rows, insert, copy)
                    {
                        self.apply(l, cx, ListOp::Move { rows, target, after, copy });
                    }
                    cx.request_layout();
                } else if let Some(press) = l.press.take()
                    && click
                {
                    if let Some(pick) = press.plan.on_click {
                        self.apply(l, cx, ListOp::Pick(pick));
                    }
                    if press.renames {
                        self.apply(l, cx, ListOp::Rename(press.row));
                    }
                }
                l.press = None;
                l.scroll_drag = false;
                l.pointer = None;
                Handled::Yes
            }
            Gesture::Cancel(_) => {
                l.press = None;
                l.drag = None;
                l.scroll_drag = false;
                l.pointer = None;
                l.drop.set_if_changed(cx, None);
                cx.request_layout();
                Handled::Yes
            }
            Gesture::LongPress => Handled::Yes,
        }
    }

    fn key(&self, l: &mut ListLocal, cx: &mut EventCx<'_>, k: &KeyInput) -> Handled {
        if !k.down {
            return Handled::No;
        }
        let doc = self.doc.get(cx);
        let view = cx.bounds().height();
        let page = crate::list_rules::page_rows(view as f32, l.row_h as f32);
        let mut lead = l.lead.get(cx);
        let selected = Self::selected(&doc);
        let result = crate::list_rules::key_press(self.os, k.key, k.mods, doc.layers.len(), page, &mut lead, &selected);
        if !result.handled {
            return Handled::No;
        }
        l.lead.set_if_changed(cx, lead);
        if let Some(to) = result.reveal {
            let offset = l.offset.get(cx);
            let top = to as f32 * l.row_h;
            let target = crate::list_rules::reveal(offset as f32, view as f32, top as f32, l.row_h as f32);
            let max = self.max_offset(l, cx);
            l.offset.set_if_changed(cx, target.clamp(0.0, max));
        }
        for op in result.ops {
            self.apply(l, cx, op);
        }
        Handled::Yes
    }
}

impl Element for List {
    type Output = ();
    type Local = ListLocal;

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, cx: &mut BuildCx<'_>) -> ListLocal {
        ListLocal {
            offset: cx.state(0.0),
            lead: cx.state(Lead::default()),
            rename: cx.state(None),
            band: cx.state(0..0),
            drop: cx.state(None),
            press: None,
            drag: None,
            scroll_drag: false,
            pointer: None,
            alt: false,
            last_time: 0.0,
            row_h: 0.0,
        }
    }

    fn build(&self, l: &mut ListLocal, cx: &mut BuildCx<'_>) {
        let look = Look::of(&cx.tokens());
        let doc = self.doc.get(cx);
        // A list without a lead takes the document's active layer's row.
        let mut lead = l.lead.get(cx);
        if lead.lead.is_none_or(|row| row >= doc.layers.len())
            && let Some(row) = doc.active.and_then(|a| doc.index_of(a))
        {
            lead.lead = Some(row);
            lead.anchor = lead.anchor.or(Some(row));
            l.lead.set_if_changed(cx, lead);
        }
        let rows = Rows { doc: self.doc, lead: l.lead, rename: l.rename, band: l.band, offset: l.offset, list: cx.node() };
        let content = doc.layers.len() as f32 * look.row_height;
        cx.child(Scroll::new(l.offset, content, doc.layers.len(), move |_, extent, cx| {
            cx.child(rows.with_extent(extent));
        }));
    }

    fn layout(&self, l: &mut ListLocal, cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        let look = Look::of(&cx.tokens());
        l.row_h = look.row_height;
        let size = Vec2::new(if c.max.x.is_finite() { c.max.x } else { c.min.x }, if c.max.y.is_finite() { c.max.y } else { c.min.y });
        if l.drag.is_some()
            && let Some(p) = l.pointer
        {
            // Edge scroll runs on frame time: reading it asks for the next frame while it is read.
            let time = cx.env().time.get(cx);
            let dt = (time - l.last_time).clamp(0.0, 0.1);
            l.last_time = time;
            let step = crate::list_rules::edge_step(size.y, l.row_h, p.y, dt as f32);
            if step != 0.0 {
                let len = self.doc.get(cx).layers.len();
                let max = (len as f32 * l.row_h - size.y).max(0.0);
                let offset = l.offset.get(cx);
                l.offset.set_if_changed(cx, (offset + step).clamp(0.0, max));
                self.update_drop(l, cx);
            }
        }
        let child = cx.child_at(0);
        cx.layout_child(child, Constraints::tight(size));
        cx.place_child(child, Vec2::ZERO);
        size
    }

    fn relayout_boundary(&self) -> bool {
        true
    }

    fn paint(&self, l: &ListLocal, cx: &mut PaintCx<'_>) {
        let bounds = cx.bounds();
        cx.hit(bounds, Sense::CLICK | Sense::DRAG, PART_EMPTY);
        let Some(insert) = l.drop.get(cx) else { return };
        let look = Look::of(&cx.tokens());
        let offset = l.offset.get(cx);
        let y = insert as f32 * look.row_height - offset;
        let (band, w) = (look.thick / 2.0, bounds.width());
        let halo = crate::geo::rect(-band - look.thin, y - band - look.thin, w + band + look.thin, y + band + look.thin);
        let mark = crate::geo::rect(-band, y - band, w + band, y + band);
        cx.over(|cx| {
            cx.fill_rect(halo, look.panel);
            cx.fill_rect(mark, look.accent);
        });
    }

    fn event(&self, l: &mut ListLocal, cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        match ev {
            Event::Pointer(p) => self.pointer(l, cx, p),
            Event::Key(k) => self.key(l, cx, k),
            _ => Handled::No,
        }
    }

    fn access(&self, l: &ListLocal, cx: &mut AccessCx<'_>) -> Option<Node> {
        let mut node = Node::new(Role::List);
        node.set_label("Layers");
        let offset = l.offset.get(cx);
        let len = self.doc.get(cx).layers.len();
        let max = (len as f32 * l.row_h - cx.size().y).max(0.0);
        node.set_scroll_y(f64::from(offset));
        node.set_scroll_y_min(0.0);
        node.set_scroll_y_max(f64::from(max));
        node.add_action(Action::Focus);
        if offset > 0.0 {
            node.add_action(Action::ScrollUp);
        }
        if offset < max {
            node.add_action(Action::ScrollDown);
        }
        Some(node)
    }

    fn focusable(&self) -> bool {
        true
    }
}

/// The rows in view and a few beyond, keyed by layer.
#[derive(Clone, Copy)]
struct Rows {
    doc: DocCell,
    lead: State<Lead>,
    rename: State<Option<LayerId>>,
    band: State<Range<usize>>,
    offset: State<f32>,
    list: NodeId,
}

struct RowsElement {
    cells: Rows,
    extent: State<Extent>,
}

impl Rows {
    fn with_extent(self, extent: State<Extent>) -> RowsElement {
        RowsElement { cells: self, extent }
    }
}

impl Element for RowsElement {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn build(&self, _: &mut (), cx: &mut BuildCx<'_>) {
        let r = self.cells;
        let doc = r.doc.get(cx);
        let lead = r.lead.get(cx).lead;
        let renaming = r.rename.get(cx);
        let n = doc.layers.len();
        let band = r.band.get(cx);
        let band = band.start.min(n)..band.end.min(n);
        let next_renamable = |from: usize, step: isize| {
            let mut i = from as isize + step;
            while i >= 0 && (i as usize) < n {
                let layer = &doc.layers[i as usize];
                if renamable(layer) {
                    return Some(layer.id);
                }
                i += step;
            }
            None
        };
        for index in band {
            let layer = &doc.layers[index];
            let props = RowProps {
                layer: layer.clone(),
                index,
                count: n,
                selected: doc.is_selected(layer.id),
                active: doc.active == Some(layer.id),
                lead: lead == Some(index),
                renaming: renaming == Some(layer.id),
                list: r.list,
                prev_rename: next_renamable(index, -1),
                next_rename: next_renamable(index, 1),
            };
            cx.keyed(layer.id, |cx| {
                cx.child(ListRow::new(props, r.doc, r.rename));
            });
        }
    }

    fn layout(&self, _: &mut (), cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        let r = self.cells;
        let look = Look::of(&cx.tokens());
        let n = r.doc.with(cx, |d| d.layers.len());
        let h = look.row_height;
        let size = c.constrain(Vec2::new(c.max.x, n as f32 * h));
        let offset = r.offset.get(cx);
        let view = self.extent.get(cx).view;
        let built = r.band.get(cx);
        let built = built.start.min(n)..built.end.min(n);
        for (i, index) in built.clone().enumerate() {
            let child = cx.child_at(i);
            cx.layout_child(child, Constraints::tight(Vec2::new(size.x, h)));
            cx.place_child(child, Vec2::new(0.0, index as f32 * h));
        }
        let want = virtual_band(offset, view, h, n, OVERSCAN_ROWS, built.clone());
        if want != built {
            r.band.set(cx, want);
        }
        size
    }

    fn relayout_boundary(&self) -> bool {
        true
    }
}
