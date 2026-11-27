//! The edge press of a collection that takes one (`Collection::edge_press`): with its modifiers held, the line between
//! two adjacent rows is a target of its own (Photoshop's Alt-click between two layers clips the upper one).

use super::{EDGE, ItemsHooks, ScrollPos};
use crate::contract::collection::CollectionEvent;
use crate::contract::{Collection, Cx, PartsCx};
use craft_behavior::hit::LINE_REACH;
use craft_behavior::{HitLevel, HitPriority, ItemLayout as _, Sense};
use craft_ui_rt::Rect;

impl<C: Collection> ItemsHooks<'_, '_, C> {
    /// Declares the band of each line between two shown rows that the edge press's test takes, while its modifiers
    /// are held: a part over the rows, with the test's cursor, that takes the press from both. The band reaches as
    /// far from the line as a line's hit does (`LINE_REACH`), across the row. The modifiers are a tracked read, so the
    /// collection runs once when they change and a pointer move runs nothing.
    pub(super) fn edge_parts(&mut self, comp: &C, pcx: &mut PartsCx<'_, C>) {
        self.edges.clear();
        let Some(rule) = comp.edge_press() else { return };
        if pcx.measure.with_ui(|ui| ui.ctx().modifiers()) != rule.mods() {
            return;
        }
        let Some(layout) = self.layout.clone() else { return };
        let view = self.below_header(pcx.content_rect);
        let rtl = pcx.flow.is_rtl();
        let offset = ScrollPos { x: self.offset_x, y: self.offset.clamp(0.0, self.max_offset(view.height())) };
        let mut rows = Vec::new();
        layout.visible(offset, view.size(), &mut rows);
        rows.sort_unstable();
        let usable = |at: usize| self.source.enabled(at) && layout.shown(at) && !self.source.section_stop(at);
        for pair in rows.windows(2) {
            let (above, below) = (pair[0], pair[1]);
            if below != above + 1 || !usable(above) || !usable(below) {
                continue;
            }
            let (above_key, below_key) = (self.source.key(above), self.source.key(below));
            let Some(cursor) = rule.test(&above_key, &below_key) else { continue };
            let rect = self.screen(&layout, view, below, offset, rtl);
            let line = rect.top();
            let band = Rect::from_x_y_ranges(rect.x_range(), (line - LINE_REACH)..=(line + LINE_REACH)).intersect(view);
            pcx.part(EDGE.at(self.edges.len() as u64), band)
                .sense(Sense::CLICK)
                .over()
                .priority(HitPriority::new(HitLevel::Edge, 0))
                .cursor(cursor)
                .pointer_only("The edge press is a pointer shortcut; the app's commands do the same from the keyboard");
            self.edges.push((above_key, below_key));
        }
    }

    /// Reports a click on a band declared this run as `CollectionEvent::EdgePressed`.
    pub(super) fn edge_step(&mut self, cx: &mut Cx<'_, C>) {
        for (at, (above, below)) in self.edges.clone().into_iter().enumerate() {
            if cx.part_activated(EDGE.at(at as u64)).is_some() {
                let mods = super::press_mods(cx.core.input());
                self.events.push(CollectionEvent::EdgePressed { above, below, mods });
            }
        }
    }
}
