//! [`SyncedViews`] (N3.22): two page views side by side that follow each other's zoom, layout and
//! scroll, as PdfCraft's Compare Files shows the old file left and the new one right. The half
//! under the pointer leads (the new file at first). The pair draws the leading half first; the
//! other takes the leader's zoom, layout and scroll before it is drawn, in the same pass. The
//! halves and the divider are craft-ui's [`SplitViews`](craft_ui::canvas::SplitViews) with
//! [`SplitLayout::Two`](craft_ui::canvas::SplitLayout::Two).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

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
        half: impl FnMut(&mut Ui, &mut PageViewState, Lead) -> PageViewResponse,
    ) -> (PageViewResponse, PageViewResponse) {
        todo!()
    }
}
