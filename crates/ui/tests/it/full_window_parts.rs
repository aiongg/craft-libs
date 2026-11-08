//! The full-window parts' own logic: the view's fit and zoom rules, and the title text.

use craft_behavior::PixelSize;
use craft_ui::dialog::{FullWindowTitle, FullWindowView, FullWindowZoomKeys};
use craft_ui::runtime::env::Env;
use craft_ui::testing::{Fixture, trial};
use egui::{Pos2, Rect, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// The canvas region's size in points.
const CANVAS: Vec2 = Vec2::new(800.0, 600.0);

/// An image larger than the canvas, so a fitted view shows it reduced.
const IMAGE: PixelSize = PixelSize::new(4000, 3000);

/// Pixels per point of the tests' screen.
const SCALE: f32 = 1.0;

/// How far a pan moves the view's centre in image pixels.
const PAN: f64 = 10.0;

/// Zooms closer than this are the same zoom.
const SAME: f64 = 1e-6;

fn fitted() -> FullWindowView {
    FullWindowView::new().settled(Rect::from_min_size(Pos2::ZERO, CANVAS), IMAGE, SCALE)
}

#[trial]
fn a_zoom_command_ends_the_fit_and_fit_returns_to_it() {
    let rect = Rect::from_min_size(Pos2::ZERO, CANVAS);
    let mut view = fitted();
    let fit = view.device_zoom();
    assert!(view.is_fit() && fit < 1.0, "a new view fits an image larger than the canvas, reduced");
    assert!(!view.apply(FullWindowZoomKeys::Fit), "fitting a fitted view changes nothing");
    assert!(view.apply(FullWindowZoomKeys::In));
    assert!(!view.is_fit() && view.device_zoom() > fit);
    let kept = view.settled(rect, IMAGE, SCALE);
    assert!((kept.device_zoom() - view.device_zoom()).abs() < SAME, "a view that does not fit keeps its zoom when settled");
    assert!(view.apply(FullWindowZoomKeys::Fit));
    assert!((view.settled(rect, IMAGE, SCALE).device_zoom() - fit).abs() < SAME, "settling a fitted view fits again");
}

#[trial]
fn actual_pixels_is_one_hundred_percent_and_repeats_to_nothing() {
    let mut view = fitted();
    assert!(view.apply(FullWindowZoomKeys::ActualPixels));
    assert!((view.zoom_percent() - 100.0).abs() < SAME);
    assert!(!view.apply(FullWindowZoomKeys::ActualPixels), "the second command finds the view at 100 percent");
}

#[trial]
fn a_view_taken_back_from_the_canvas_ends_the_fit_only_when_it_moved() {
    let view = fitted();
    assert!(view.took(view.view()).is_fit(), "the canvas handing back the view it was given keeps the fit");
    let mut panned = view.view();
    panned.center.x += PAN;
    assert!(!view.took(panned).is_fit(), "a pan ends the fit");
}

#[trial]
fn the_title_drops_the_ellipsis_and_names_what_it_has() {
    let texts = Rc::new(RefCell::new(Vec::new()));
    let shown = texts.clone();
    let mut fixture = Fixture::new(move |ui| {
        let env = Env::of(ui);
        let mut texts = shown.borrow_mut();
        texts.clear();
        for title in [
            FullWindowTitle::new("Liquify…").subject("Layer 1").zoom(66.7),
            FullWindowTitle::new("Liquify…").zoom(66.7),
            FullWindowTitle::new("Wide Angle").subject("Layer 1"),
            FullWindowTitle::new("Filter Gallery…"),
        ] {
            texts.push(title.label(&env).as_str().to_owned());
        }
    });
    fixture.settle();
    assert_eq!(*texts.borrow(), ["Liquify (Layer 1, 67%)", "Liquify (67%)", "Wide Angle (Layer 1)", "Filter Gallery"]);
}

/// The canvas's navigation: the wheel, the middle button and the Hand tool it falls back to.
mod canvas_navigation {
    use craft_behavior::PixelSize;
    use craft_behavior::kurbo::Point;
    use craft_ui::canvas::{NavigationOptions, PixelRect, RasterSource, ToolPath};
    use craft_ui::contract::Bind;
    use craft_ui::dialog::{FullWindowCanvas, FullWindowView};
    use craft_ui::primitives::Revision;
    use craft_ui::testing::{Fixture, pointer, trial};
    use egui::{Color32, ColorImage, Pos2, Vec2};
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    /// Where the pointer rests: inside the canvas, off its centre.
    const OVER: Pos2 = Pos2::new(260.0, 170.0);

    /// How far a drag moves the pointer, in points.
    const DRAG: Vec2 = Vec2::new(30.0, 20.0);

    /// How far a document point may move under the pointer in a wheel zoom, in image pixels.
    const STILL: f64 = 0.5;

    /// An image larger than the canvas.
    struct Grey;

    impl RasterSource for Grey {
        fn size(&self) -> PixelSize {
            PixelSize::new(1600, 1200)
        }

        fn revision(&self) -> Revision {
            Revision::new(1)
        }

        fn damage(&self, _: Revision) -> Option<PixelRect> {
            Some(PixelRect::default())
        }

        fn read(&self, _: PixelRect, _: u32, out: &mut ColorImage) {
            out.pixels.fill(Color32::from_rgb(0x80, 0x80, 0x80));
        }
    }

    /// A canvas with no tool, the view the app owns and the image point under the pointer.
    struct Rig {
        fixture: Fixture,
        view: Rc<RefCell<FullWindowView>>,
        hover: Rc<Cell<Option<Point>>>,
    }

    fn rig(navigation: NavigationOptions) -> Rig {
        let view = Rc::new(RefCell::new(FullWindowView::new()));
        let hover = Rc::new(Cell::new(None));
        let (shown, seen) = (view.clone(), hover.clone());
        let mut fixture = Fixture::new(move |ui| {
            let mut view = shown.borrow_mut();
            seen.set(FullWindowCanvas::new(ToolPath::Freehand, Bind::bound(&mut *view), &Grey).navigation(navigation).show(ui).hover);
        });
        fixture.settle();
        Rig { fixture, view, hover }
    }

    impl Rig {
        fn zoom(&self) -> f64 {
            self.view.borrow().device_zoom()
        }

        fn centre(&self) -> Point {
            self.view.borrow().view().center
        }

        fn drag(&mut self, down: impl Fn(Pos2, bool) -> egui::Event, from: Pos2, by: Vec2) {
            self.fixture.frame(vec![pointer::moved(from)]);
            self.fixture.frame(vec![down(from, true)]);
            for step in 1..=4 {
                self.fixture.frame(vec![pointer::moved(from + by * (step as f32 / 4.0))]);
            }
            self.fixture.frame(vec![down(from + by, false)]);
            self.fixture.idle(1);
        }
    }

    fn primary(at: Pos2, pressed: bool) -> egui::Event {
        if pressed { pointer::press(at) } else { pointer::release(at) }
    }

    #[trial]
    fn the_wheel_zooms_about_the_pointer_by_the_navigation_options() {
        let mut plain = rig(NavigationOptions::default());
        let fitted = plain.zoom();
        plain.fixture.frame(vec![pointer::moved(OVER)]);
        plain.fixture.frame(vec![pointer::wheel_lines(1.0)]);
        assert_eq!(plain.zoom(), fitted, "the plain wheel does not zoom by default");

        let mut navigation = NavigationOptions::default();
        navigation.zoom_with_wheel = true;
        navigation.animated_zoom = false;
        let mut zooming = rig(navigation);
        let fitted = zooming.zoom();
        zooming.fixture.frame(vec![pointer::moved(OVER)]);
        let under = zooming.hover.get().expect("the pointer is over the image");
        zooming.fixture.frame(vec![pointer::wheel_lines(1.0)]);
        assert!(zooming.zoom() > fitted, "the wheel zooms in with Zoom with Scroll Wheel");
        assert!(!zooming.view.borrow().is_fit(), "a wheel zoom ends the fit");
        let still = zooming.hover.get().expect("the pointer is still over the image");
        assert!((still.x - under.x).abs() < STILL && (still.y - under.y).abs() < STILL, "the point under the pointer stays: {under:?} {still:?}");
    }

    #[trial]
    fn the_middle_button_pans_and_so_does_the_hand_tool_a_canvas_without_a_tool_falls_back_to() {
        let mut middle = rig(NavigationOptions::default());
        let start = middle.centre();
        middle.drag(pointer::middle, OVER, DRAG);
        let panned = middle.centre();
        assert!(panned.x < start.x - 1.0 && panned.y < start.y - 1.0, "the image follows the pointer: {start:?} {panned:?}");
        assert!(!middle.view.borrow().is_fit(), "a pan ends the fit");

        let mut hand = rig(NavigationOptions::default());
        let start = hand.centre();
        hand.drag(primary, OVER, DRAG);
        let panned = hand.centre();
        assert!(panned.x < start.x - 1.0 && panned.y < start.y - 1.0, "a plain drag without a tool pans: {start:?} {panned:?}");
    }
}
