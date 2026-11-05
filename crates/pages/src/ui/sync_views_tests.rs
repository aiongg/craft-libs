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
    harness.step();
    let app = harness.state();
    assert_eq!(app.left.zoom, 2.0);
    assert_eq!(app.left.offset, app.right.offset);
    assert!(app.right.offset.1 > 0.0);

    harness.hover_at(egui::pos2(100.0, 300.0));
    harness.run();
    assert_eq!(harness.state().pair.lead, Lead::Left);
    harness.state_mut().left.zoom = 1.0;
    harness.step();
    assert_eq!(harness.state().right.zoom, 1.0);
}

// The follower takes the leader's rotation along with its zoom, in the pass the leader changes it.
#[test]
fn the_follower_takes_the_leaders_rotation() {
    let mut harness = open();
    harness.hover_at(egui::pos2(700.0, 300.0));
    harness.run();
    harness.state_mut().right.rotation = 90;
    harness.step();
    assert_eq!(harness.state().left.rotation, 90);
}

// With one page showing, the follower turns to the leader's page when the leader turns.
#[test]
fn the_follower_turns_to_the_leaders_page() {
    let mut harness = open();
    harness.state_mut().left.continuous = false;
    harness.state_mut().right.continuous = false;
    harness.hover_at(egui::pos2(700.0, 300.0));
    harness.run();
    harness.state_mut().right.go_to(3);
    harness.step();
    assert_eq!(harness.state().left.current, 3);
}

// A drag that starts in one half and crosses into the other keeps the first half leading.
#[test]
fn a_drag_keeps_the_lead() {
    let mut harness = open();
    let (right, left) = (egui::pos2(700.0, 300.0), egui::pos2(100.0, 300.0));
    harness.hover_at(right);
    harness.run();
    harness.event(egui::Event::PointerButton { pos: right, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::NONE });
    harness.step();
    harness.event(egui::Event::PointerMoved(left));
    harness.step();
    assert_eq!(harness.state().pair.lead, Lead::Right, "held over the other half");
    harness.event(egui::Event::PointerButton { pos: left, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::NONE });
    harness.step();
    harness.event(egui::Event::PointerMoved(egui::pos2(101.0, 300.0)));
    harness.step();
    assert_eq!(harness.state().pair.lead, Lead::Left, "after the release hover decides");
}
