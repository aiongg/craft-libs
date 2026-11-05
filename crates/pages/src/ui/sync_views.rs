//! [`SyncedViews`] (N3.22): two page views side by side that follow each other's zoom, layout,
//! rotation, current page and scroll, as PdfCraft's Compare Files shows the old file left and the new one right. The half
//! under the pointer leads (the new file at first). The pair draws the leading half first; the
//! other takes the leader's zoom, layout and scroll before it is drawn, in the same pass. The
//! halves and the divider are craft-ui's [`SplitViews`](craft_ui::canvas::SplitViews) with
//! [`SplitLayout::Two`](craft_ui::canvas::SplitLayout::Two).

use craft_ui::canvas::{SplitLayout, SplitViews};
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::Ui;

use super::view::{PageViewResponse, PageViewState};

/// Which half leads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lead {
    Left,
    #[default]
    Right,
}

/// The pair's state, kept by the app.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SyncedViewsState {
    pub lead: Lead,
    /// The leader's offset when the follower last copied it.
    last_offset: Option<(f64, f64)>,
    /// The leader's current page when the follower last copied it.
    last_current: Option<usize>,
}

/// The pair, built per pass: the app draws each half with its own [`PageView`](super::view::PageView)
/// inside `left` and `right`, and the pair keeps the states together.
///
/// ```ignore
/// SyncedViews::new("compare", &mut pair).show(ui, (&mut old_state, &mut new_state), |ui, state, half| match half {
///     Lead::Left => PageView::new("old", &mut old_pages, state).show(ui),
///     Lead::Right => PageView::new("new", &mut new_pages, state).show(ui),
/// });
/// ```
pub struct SyncedViews<'a> {
    id_salt: egui::IdSalt,
    state: &'a mut SyncedViewsState,
}

impl<'a> SyncedViews<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, state: &'a mut SyncedViewsState) -> Self {
        SyncedViews { id_salt: egui::IdSalt::new(id_salt), state }
    }

    /// Split the area in two halves (`SplitViews`, its divider and gap) and run `half` for each,
    /// the leader first.
    // pdfcraft: crates/ui-egui/src/workflows/compare_ui.rs:344 `compare_area`.
    pub fn show(
        self,
        ui: &mut Ui,
        states: (&mut PageViewState, &mut PageViewState),
        mut half: impl FnMut(&mut Ui, &mut PageViewState, Lead) -> PageViewResponse,
    ) -> (PageViewResponse, PageViewResponse) {
        let rect = ui.available_rect_before_wrap();
        let rects = SplitLayout::Two.rects(Tokens::of(ui).component.split.gap, rect);
        // `SplitLayout::Two` puts pane 0 (the main view) on the right.
        // The lead stays with the half a drag started in: it changes on a press, or on hover with
        // no button held.
        let may_switch = ui.input(|i| i.pointer.any_pressed() || !i.pointer.any_down());
        if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()).filter(|_| may_switch) {
            if rects[1].contains(pointer) {
                self.state.lead = Lead::Left;
            } else if rects[0].contains(pointer) {
                self.state.lead = Lead::Right;
            }
        }
        let lead = self.state.lead;
        let copied = (&mut self.state.last_offset, &mut self.state.last_current);
        let (left, right) = states;
        let (leader, follower) = match lead {
            Lead::Left => (left, right),
            Lead::Right => (right, left),
        };
        let active = match lead {
            Lead::Left => 1,
            Lead::Right => 0,
        };
        let mut responses: [Option<PageViewResponse>; 2] = [None, None];
        SplitViews::new(self.id_salt, SplitLayout::Two).active(active).first(active).show(ui, rect, |ui, pane, _| {
            let this = if pane == 1 { Lead::Left } else { Lead::Right };
            if this == lead {
                responses[pane] = Some(half(ui, leader, this));
            } else {
                follow(leader, follower, copied.0, copied.1);
                responses[pane] = Some(half(ui, follower, this));
            }
        });
        ui.advance_cursor_after_rect(rect);
        let [Some(right), Some(left)] = responses else { unreachable!("SplitViews draws every pane") };
        (left, right)
    }
}

/// The follower takes the leader's zoom, layout and rotation, its scroll offset once the leader
/// has moved since the last copy, and, when one page shows at a time, the leader's current page
/// once it has changed (a continuous view's current page follows from the offset). Returns
/// whether anything changed.
// pdfcraft: crates/ui-egui/src/workflows/compare_ui.rs:362 `compare_area`.
fn follow(
    leader: &PageViewState,
    follower: &mut PageViewState,
    last_offset: &mut Option<(f64, f64)>,
    last_current: &mut Option<usize>,
) -> bool {
    let before = follower.clone();
    follower.zoom = leader.zoom;
    follower.fit = leader.fit;
    follower.display = leader.display;
    follower.continuous = leader.continuous;
    follower.cover = leader.cover;
    follower.page_gaps = leader.page_gaps;
    follower.hide_white_space = leader.hide_white_space;
    follower.rotation = leader.rotation;
    if *last_offset != Some(leader.offset) {
        follower.offset = leader.offset;
        *last_offset = Some(leader.offset);
    }
    if *last_current != Some(leader.current) {
        if !leader.continuous {
            follower.go_to(leader.current);
        }
        *last_current = Some(leader.current);
    }
    *follower != before
}
