//! Where rows sit: the rows' rect and scroll range, which rows a pass builds, and rows on the screen.

use super::{ItemsHooks, ROW};
use crate::contract::{Collection, Cx, PartsCx};
use craft_behavior::{Flow, HitLevel, HitPriority, ItemLayout, ScrollPos, SelectionModel};
use craft_ui_rt::{Pos2, Rect};

impl<'s, 'r, C: Collection> ItemsHooks<'s, 'r, C> {
    /// The rect of the rows: the content rect below the header row.
    pub(super) fn below_header(&self, area: Rect) -> Rect {
        Rect::from_min_max(Pos2::new(area.min.x, area.min.y + self.header.height), area.max)
    }

    pub(super) fn rows_rect(&self, cx: &Cx<'_, C>) -> Rect {
        self.below_header(cx.core.content_rect)
    }

    /// Declares each member row's `ROW` part over its rect in the rows' view, with the member's sense: the
    /// collection's paint records one sub-list per row and look, and ui-rt shows the hovered row's sub-list from the
    /// kept lists (docs/contract-rt.md 15.5). Band rows are nodes of their own and declare no part here.
    pub(super) fn row_parts(&self, pcx: &mut PartsCx<'_, C>) {
        if !C::MEMBERS {
            return;
        }
        for &(hash, _, rect) in &self.row_parts {
            pcx.part(ROW.at(hash), rect).exact_hit().priority(HitPriority::new(HitLevel::Background, 1));
        }
    }

    /// Whether the scroll bar shows over the rows' rect `view`: the rows scroll and run past its height.
    pub(super) fn shows_bar(&self, view: Rect) -> bool {
        self.scrolls && self.content.y > view.height()
    }

    /// The rows' rect `view` less the scroll bar's track while the bar shows.
    pub(super) fn clear_of_bar(&self, view: Rect, thickness: f32, rtl: bool) -> Rect {
        if !self.shows_bar(view) {
            return view;
        }
        let track = bar_track(view, thickness, rtl);
        if rtl {
            Rect::from_min_max(Pos2::new(track.max.x, view.min.y), view.max)
        } else {
            Rect::from_min_max(view.min, Pos2::new(track.min.x, view.max.y))
        }
    }

    pub(super) fn max_offset(&self, view: f32) -> f64 {
        f64::from((self.content.y - view).max(0.0))
    }

    pub(super) fn page_rows(&self, layout: &C::ItemLayout, view: f32) -> usize {
        let row = if self.source.is_empty() { 0.0 } else { layout.rect(0, ScrollPos::default()).height() };
        if row > 0.0 { ((view / row) as usize).saturating_sub(1).max(1) } else { 1 }
    }

    /// `point` of the rows' coordinates on the screen.
    pub(super) fn on_screen(&self, point: Pos2) -> Pos2 {
        point
    }
}

impl<C: Collection> ItemsHooks<'_, '_, C> {
    /// The rows the build covers: the band's rows `shown`, then the rows a gesture holds while out of the band (the
    /// lead, the anchor, the renaming row and the grabbed row).
    pub(super) fn held_rows(&self, layout: &C::ItemLayout, shown: Vec<usize>) -> (Vec<usize>, Vec<usize>) {
        let mut order = shown.clone();
        let keep = [self.lead.key.as_ref(), self.selection.anchor(), self.renaming.key.as_ref(), self.drags.grabbed.as_ref()];
        // An item of a closed group has no rect: it is never built.
        for at in keep.into_iter().flatten().filter_map(|key| self.index_of(key)).filter(|&at| layout.shown(at)) {
            if !order.contains(&at) {
                order.push(at);
            }
        }
        (shown, order)
    }

    /// The rows since the last section row up to and including `at` (0 for a section row), continuing from `last`, the
    /// result for an earlier row: stripes start again after a section.
    pub(super) fn stripe_run(&self, last: Option<(usize, usize)>, at: usize) -> usize {
        match last {
            Some((from, run)) if from <= at => (from + 1..=at).fold(run, |run, row| if self.source.is_section(row) { 0 } else { run + 1 }),
            _ => (0..=at).rev().take_while(|&row| !self.source.is_section(row)).count(),
        }
    }

    /// The screen rect of row `at`: logical layout coordinates mirrored under right-to-left, then moved to the view.
    pub(super) fn screen(&self, layout: &C::ItemLayout, view: Rect, at: usize, offset: ScrollPos, rtl: bool) -> Rect {
        self.to_screen(layout, view, layout.rect(at, offset), rtl)
    }

    /// A rect of the layout's logical, viewport-relative coordinates on the screen.
    pub(super) fn to_screen(&self, layout: &C::ItemLayout, view: Rect, local: Rect, rtl: bool) -> Rect {
        let local = if rtl && layout.mirrors() { Flow::new(Rect::from_min_size(Pos2::ZERO, view.size()), true).to_physical(local) } else { local };
        local.translate(view.min.to_vec2())
    }

    /// The layout's grid view on the screen: sections placed in the view and mirrored under right-to-left.
    pub(super) fn grid_screen(&self, layout: &C::ItemLayout, grid: &craft_behavior::GridView, view: Rect, rtl: bool) -> craft_behavior::GridView {
        let mirrored = rtl && layout.mirrors();
        let edge = |x: f32| if mirrored { view.max.x - x } else { view.min.x + x };
        craft_behavior::GridView::new(grid.sections.iter().map(|section| {
            let edges = section.edges.iter().map(|&x| edge(x)).collect();
            craft_behavior::GridSection::new(self.to_screen(layout, view, section.rect, rtl), section.columns.clone(), edges)
        }))
    }
}

/// The scroll bar's track of `thickness` along the end edge of the rows' rect `view` (its left edge under right-to-left).
pub(super) fn bar_track(view: Rect, thickness: f32, rtl: bool) -> Rect {
    if rtl {
        Rect::from_min_max(view.min, Pos2::new(view.min.x + thickness, view.max.y))
    } else {
        Rect::from_min_max(Pos2::new(view.max.x - thickness, view.min.y), view.max)
    }
}
