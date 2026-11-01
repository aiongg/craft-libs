//! The page preview.

use craft_ui::canvas::textures::{FakeThumbSource, Revision, ThumbRender, ThumbRequest, ThumbSource};
use craft_ui::theme::{Base, install};
use egui::Key;
use egui::accesskit::Role;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};
use kurbo::Size;

use super::preview::*;
use super::view::PageSource;

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
        5
    }

    fn page_size(&self, _page: usize) -> Size {
        Size::new(612.0, 792.0)
    }
}

struct App {
    pages: Pages,
    shown: Vec<usize>,
    at: usize,
    steps: usize,
    closed: bool,
}

fn preview() -> Harness<'static, App> {
    HarnessBuilder::default()
        .with_step_dt(0.05)
        .with_size(egui::vec2(500.0, 500.0))
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app: &mut App| {
                let out = PagePreview::new("preview", &app.pages, &app.shown, &mut app.at).show(ui.ctx());
                app.steps += usize::from(out.stepped);
                app.closed |= out.closed;
            },
            App { pages: Pages(FakeThumbSource::new()), shown: vec![0, 2, 4], at: 0, steps: 0, closed: false },
        )
}

// Right arrow from page 0 of [0, 2, 4] shows 2 and reports stepped; at the end next is disabled; Esc reports closed.
#[test]
fn arrows_step_and_esc_closes() {
    let mut harness = preview();
    harness.run();
    harness.get_by_role_and_label(Role::Dialog, "Page 1 of 5");
    assert!(harness.get_by_label("Previous page").accesskit_node().is_disabled());

    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!((harness.state().at, harness.state().steps), (1, 1));
    harness.get_by_role_and_label(Role::Dialog, "Page 3 of 5");

    harness.key_press(Key::PageDown);
    harness.run();
    assert_eq!(harness.state().at, 2);
    assert!(harness.get_by_label("Next page").accesskit_node().is_disabled());
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!((harness.state().at, harness.state().steps), (2, 2), "the end does not step");

    assert!(!harness.state().closed);
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().closed);
}
