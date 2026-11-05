//! The page preview.

use craft_ui::canvas::textures::{FakeThumbSource, Revision, ThumbRender, ThumbRequest, ThumbSource};
use craft_ui::direction::Direction;
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
    opened: bool,
}

fn preview() -> Harness<'static, App> {
    preview_in(Direction::Ltr, true)
}

fn preview_in(direction: Direction, opened: bool) -> Harness<'static, App> {
    HarnessBuilder::default()
        .with_step_dt(0.05)
        .with_size(egui::vec2(500.0, 500.0))
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            craft_ui::direction::apply(ctx, direction);
        })
        .build_ui_state(
            |ui, app: &mut App| {
                let out = PagePreview::new("preview", &app.pages, &app.shown, &mut app.at).opened(std::mem::take(&mut app.opened)).show(ui.ctx());
                app.steps += usize::from(out.stepped);
                app.closed |= out.closed;
            },
            App { pages: Pages(FakeThumbSource::new()), shown: vec![0, 2, 4], at: 0, steps: 0, closed: false, opened },
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

// The preview takes the keyboard only on the pass the app says it opened: a first pass without the flag leaves focus alone.
#[test]
fn the_keyboard_moves_in_when_the_app_says_opened() {
    let mut harness = preview_in(Direction::Ltr, false);
    harness.run();
    assert!(!harness.get_by_label("Close").accesskit_node().is_focused());
    harness.state_mut().opened = true;
    harness.run();
    assert!(harness.get_by_label("Close").accesskit_node().is_focused());
}

// Right to left, Previous sits right of Next and the arrow keys follow: → steps back, ← forward.
#[test]
fn right_to_left_swaps_the_buttons_and_the_keys() {
    let x = |harness: &Harness<'_, App>, label: &str| harness.get_by_label(label).accesskit_node().raw_bounds().expect("bounds").x0;
    let mut ltr = preview_in(Direction::Ltr, true);
    ltr.run();
    assert!(x(&ltr, "Previous page") < x(&ltr, "Next page"));
    let mut rtl = preview_in(Direction::Rtl, true);
    rtl.run();
    assert!(x(&rtl, "Previous page") > x(&rtl, "Next page"));
    rtl.state_mut().at = 1;
    rtl.key_press(Key::ArrowRight);
    rtl.run();
    assert_eq!(rtl.state().at, 0);
    rtl.key_press(Key::ArrowLeft);
    rtl.run();
    assert_eq!(rtl.state().at, 1);
}
