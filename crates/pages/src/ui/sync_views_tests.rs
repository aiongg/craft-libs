//! Synced side-by-side views.
#![allow(clippy::expect_used, reason = "tests")]

use craft_ui::canvas::textures::{FakeThumbSource, Revision, ThumbRender, ThumbRequest, ThumbSource};
use craft_ui::theme::{Base, install};
use egui::vec2;
use egui_kittest::{Harness, HarnessBuilder};
use kurbo::Size;

use super::sync_views::*;
use super::view::{PageSource, PageView, PageViewState};
use crate::layout::Fit;

struct Pages(FakeThumbSource<usize>);

impl ThumbSource<usize> for Pages {
    fn revision(&self, key: &usize, request: &ThumbRequest) -> Revision {
        self.0.revision(key, request)
    }

    fn render(&self, key: &usize, request: &ThumbRequest) -> ThumbRender {
        self.0.render(key, request)
    }
}

impl PageSource for Pages {
    fn page_count(&self) -> usize {
        8
    }

    fn page_size(&self, _page: usize) -> Size {
        Size::new(612.0, 792.0)
    }
}

struct App {
    pages: Pages,
    pair: SyncedViewsState,
    left: PageViewState,
    right: PageViewState,
}

fn open() -> Harness<'static, App> {
    let still = |state: &mut PageViewState| state.fit = Fit::None;
    let mut app = App {
        pages: Pages(FakeThumbSource::new().aspect(612, 792)),
        pair: SyncedViewsState::default(),
        left: PageViewState::default(),
        right: PageViewState::default(),
    };
    still(&mut app.left);
    still(&mut app.right);
    let mut harness =
        HarnessBuilder::default().with_size(vec2(900.0, 600.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App| {
                let pages = &app.pages;
                SyncedViews::new("compare", &mut app.pair)
                    .show(ui, (&mut app.left, &mut app.right), |ui, state, half| PageView::new(("pane", half == Lead::Left), pages, state).show(ui));
            },
            app,
        );
    harness.run();
    harness
}

// Zooming the right half under the pointer sets the left half's zoom before drawing and its offset after.
#[test]
fn the_follower_takes_the_leaders_zoom_and_scroll() {
    let mut harness = open();
    harness.hover_at(egui::pos2(700.0, 300.0));
    harness.run();
    assert_eq!(harness.state().pair.lead, Lead::Right);
    let app = harness.state_mut();
    app.right.zoom = 2.0;
    app.right.offset = (0.0, 400.0);
    harness.run_steps(3);
    let app = harness.state();
    assert_eq!(app.left.zoom, 2.0);
    assert_eq!(app.left.offset, app.right.offset);
    assert!(app.right.offset.1 > 0.0);

    harness.hover_at(egui::pos2(100.0, 300.0));
    harness.run();
    assert_eq!(harness.state().pair.lead, Lead::Left);
    harness.state_mut().left.zoom = 1.0;
    harness.run_steps(3);
    assert_eq!(harness.state().right.zoom, 1.0);
}
