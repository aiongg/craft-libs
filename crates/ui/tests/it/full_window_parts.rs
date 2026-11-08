//! The full-window parts' own logic: the view's fit and zoom rules, and the title text.

use craft_behavior::PixelSize;
use craft_behavior::kurbo::Size;
use craft_ui::canvas::ViewCommands;
use craft_ui::contract::{Bind, Role, ShowContainer as _};
use craft_ui::dialog::{FullWindowDialog, FullWindowTitle, FullWindowView, FullWindowZoomBar};
use craft_ui::runtime::env::Env;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Pos2, Rect, Vec2};
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
    assert!(!view.apply(ViewCommands::FitOnScreen), "fitting a fitted view changes nothing");
    assert!(view.apply(ViewCommands::ZoomIn));
    assert!(!view.is_fit() && view.device_zoom() > fit);
    let kept = view.settled(rect, IMAGE, SCALE);
    assert!((kept.device_zoom() - view.device_zoom()).abs() < SAME, "a view that does not fit keeps its zoom when settled");
    assert!(view.apply(ViewCommands::FitOnScreen));
    assert!((view.settled(rect, IMAGE, SCALE).device_zoom() - fit).abs() < SAME, "settling a fitted view fits again");
}

#[trial]
fn actual_pixels_is_one_hundred_percent_and_repeats_to_nothing() {
    let mut view = fitted();
    assert!(view.apply(ViewCommands::ActualPixels));
    assert!((view.zoom_percent() - 100.0).abs() < SAME);
    assert!(!view.apply(ViewCommands::ActualPixels), "the second command finds the view at 100 percent");
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
    assert_eq!(*texts.borrow(), ["Liquify (Layer 1, 66.7%)", "Liquify (66.7%)", "Wide Angle (Layer 1)", "Filter Gallery"]);
}

// Zoom bar and view commands (fwzoom).

/// The legacy zoom bar's image, in pixels, and the canvas region it fits in.
const BAR_IMAGE: PixelSize = PixelSize::new(200, 100);
const BAR_CANVAS: Vec2 = Vec2::new(500.0, 250.0);

/// A fixture that shows a full-window dialog holding a zoom bar bound to the returned view.
fn dialog_with_zoom_bar() -> (Fixture, Rc<RefCell<FullWindowView>>) {
    let view = Rc::new(RefCell::new(FullWindowView::new().settled(Rect::from_min_size(Pos2::ZERO, BAR_CANVAS), BAR_IMAGE, SCALE)));
    let shared = view.clone();
    let fixture = Fixture::new(move |ui| {
        let mut open = true;
        FullWindowDialog::new("Liquify").opened(Bind::bound(&mut open)).show(ui, |ui, _| {
            FullWindowZoomBar::new(Bind::bound(&mut shared.borrow_mut())).show(ui);
        });
    });
    (fixture, view)
}

#[trial]
fn the_zoom_bar_steps_the_ladder_and_lists_an_off_menu_zoom() {
    let (mut fixture, view) = dialog_with_zoom_bar();
    fixture.settle();
    let fitted = view.borrow().zoom_percent();
    assert!(view.borrow().is_fit());

    let zoom_in = fixture.by_label("Zoom In").expect("the Zoom In button is named by its command");
    fixture.click(&zoom_in).settle();
    let stepped = view.borrow().zoom_percent();
    assert!(!view.borrow().is_fit() && stepped > fitted, "Zoom In steps up from the fit: {stepped} after {fitted}");

    let zoom_out = fixture.by_label("Zoom Out").expect("the Zoom Out button is named by its command");
    fixture.click(&zoom_out).settle();
    assert!(view.borrow().zoom_percent() < stepped, "Zoom Out steps back down");

    // 300 % is not on the menu: the select lists it as itself, between 200 % and 400 %, after Fit in View.
    view.borrow_mut().set_device_zoom(3.0);
    fixture.settle();
    let select = fixture.by_role(Role::COMBO_BOX).into_iter().next().expect("the zoom select is shown");
    fixture.click(&select).settle();
    let options: Vec<String> =
        fixture.by_role(Role::LIST_BOX_OPTION).iter().filter_map(|option| fixture.tree().name_of(option.node).map(str::to_owned)).collect();
    let at = |text: &str| options.iter().position(|option| option == text).unwrap_or_else(|| panic!("{text} in {options:?}"));
    assert!(at("200%") < at("300%") && at("300%") < at("400%"), "{options:?}");
    assert_eq!(options[0], "Fit in View");
}

#[trial]
fn view_chords_zoom_the_dialogs_view() {
    let (mut fixture, view) = dialog_with_zoom_bar();
    fixture.settle();
    let fitted = view.borrow().zoom_percent();

    fixture.key(Key::Num1, Modifiers::COMMAND).settle();
    assert!((view.borrow().zoom_percent() - 100.0).abs() < SAME && !view.borrow().is_fit(), "Cmd+1 shows 100 percent");
    fixture.key(Key::Equals, Modifiers::COMMAND).settle();
    assert!(view.borrow().zoom_percent() > 100.0, "Cmd+= zooms the dialog's view in");
    fixture.key(Key::Minus, Modifiers::COMMAND).settle();
    assert!((view.borrow().zoom_percent() - 100.0).abs() < SAME, "in and out are inverse steps from a rung");
    fixture.key(Key::Num0, Modifiers::COMMAND).settle();
    assert!(view.borrow().is_fit() && (view.borrow().zoom_percent() - fitted).abs() < SAME, "Cmd+0 fits again");
}

/// An image of one grey.
struct Grey;

impl craft_ui::canvas::RasterSource for Grey {
    fn size(&self) -> PixelSize {
        BAR_IMAGE
    }

    fn revision(&self) -> craft_ui::primitives::Revision {
        craft_ui::primitives::Revision::new(1)
    }

    fn damage(&self, _: craft_ui::primitives::Revision) -> Option<craft_ui::canvas::PixelRect> {
        None
    }

    fn read(&self, _: craft_ui::canvas::PixelRect, _: u32, out: &mut egui::ColorImage) {
        out.pixels.fill(egui::Color32::from_rgb(0x80, 0x80, 0x80));
    }
}

#[trial]
fn view_chords_zoom_the_dialogs_view_while_its_canvas_has_the_focus() {
    let view = Rc::new(RefCell::new(FullWindowView::new()));
    let shared = view.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut open = true;
        FullWindowDialog::new("Liquify").opened(Bind::bound(&mut open)).show(ui, |ui, _| {
            FullWindowZoomBar::new(Bind::bound(&mut shared.borrow_mut())).show(ui);
            craft_ui::dialog::FullWindowCanvas::new(Bind::bound(&mut shared.borrow_mut()), &Grey).show(ui);
        });
    });
    fixture.settle();
    let canvas = fixture.by_role(Role::CANVAS).into_iter().next().expect("the dialog shows its canvas");
    fixture.click(&canvas).settle();
    assert_eq!(craft_ui::testing::access::focused_key(fixture.tree()), canvas.node, "a click focuses the canvas");
    let fitted = view.borrow().zoom_percent();

    fixture.key(Key::Equals, Modifiers::COMMAND).settle();
    assert!(view.borrow().zoom_percent() > fitted && !view.borrow().is_fit(), "Cmd+= zooms the dialog's view in");
    fixture.key(Key::Num0, Modifiers::COMMAND).settle();
    assert!(view.borrow().is_fit() && (view.borrow().zoom_percent() - fitted).abs() < SAME, "Cmd+0 fits again");
}

#[trial]
fn fit_on_screen_refits_in_the_same_pass_and_magnifies_a_small_image() {
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(1000.0));
    let mut view = FullWindowView::new().settled(rect, PixelSize::new(100, 100), SCALE);
    assert!((view.zoom_percent() - 400.0).abs() < SAME, "a small image fits magnified, up to 400 percent");
    assert!(view.apply(ViewCommands::ActualPixels));
    assert!(view.apply(ViewCommands::ZoomIn));
    assert!(view.apply(ViewCommands::FitOnScreen));
    assert!(view.is_fit() && (view.zoom_percent() - 400.0).abs() < SAME, "the zoom read right after the command is the fitted one");
    assert!(!view.apply(ViewCommands::FillScreen) && !view.apply(ViewCommands::RotateClockwise), "only the zoom commands act");
    assert_eq!(view.size(), Size::new(100.0, 100.0));
}

/// The canvas's navigation: the wheel, the middle button and the Hand tool it falls back to.
mod canvas_navigation {
    use craft_behavior::PixelSize;
    use craft_behavior::kurbo::Point;
    use craft_ui::canvas::{NavigationOptions, PixelRect, RasterSource};
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

    /// A drag far past the image's edge, in points.
    const PAST: Vec2 = Vec2::new(3000.0, 3000.0);

    /// How close the image's corner stops to the canvas's corner, in points.
    const EDGE: f32 = 0.01;

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
            seen.set(FullWindowCanvas::new(Bind::bound(&mut *view), &Grey).navigation(navigation).show(ui).hover);
        });
        fixture.settle();
        Rig { fixture, view, hover }
    }

    impl Rig {
        fn zoom(&self) -> f64 {
            self.view.borrow().device_zoom()
        }

        /// Zooms to 100 %, where the image is larger than the canvas, and lets the canvas settle the view.
        fn zoom_in(&mut self) {
            self.view.borrow_mut().set_device_zoom(1.0);
            self.fixture.frame(Vec::new());
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

    /// The pointer rests here for a reading of the image point away from the wheel's anchor.
    const ELSEWHERE: Pos2 = Pos2::new(60.0, 40.0);

    #[trial]
    fn a_kept_view_holds_what_the_canvas_zoomed_to_for_the_next_pass() {
        let mut navigation = NavigationOptions::default();
        navigation.zoom_with_wheel = true;
        navigation.animated_zoom = false;
        let hover = Rc::new(Cell::new(None));
        let seen = hover.clone();
        let mut fixture = Fixture::new(move |ui| {
            seen.set(FullWindowCanvas::new(Bind::kept(), &Grey).navigation(navigation).show(ui).hover);
        });
        fixture.settle();
        fixture.frame(vec![pointer::moved(ELSEWHERE)]);
        let before = hover.get().expect("the pointer is over the image");
        fixture.frame(vec![pointer::moved(OVER)]);
        fixture.frame(vec![pointer::wheel_lines(1.0)]);
        fixture.frame(vec![pointer::moved(ELSEWHERE)]);
        let after = hover.get().expect("the pointer is over the image");
        assert!((after.x - before.x).abs() > STILL, "the next pass shows the zoomed view, not a fresh fitted one: {before:?} {after:?}");
    }

    #[trial]
    fn the_middle_button_pans_up_to_the_image_edge_and_so_does_the_hand_tool_a_canvas_without_a_tool_falls_back_to() {
        let mut middle = rig(NavigationOptions::default());
        middle.zoom_in();
        let start = middle.centre();
        middle.drag(pointer::middle, OVER, DRAG);
        let panned = middle.centre();
        assert!(panned.x < start.x - 1.0 && panned.y < start.y - 1.0, "the image follows the pointer: {start:?} {panned:?}");
        assert!(!middle.view.borrow().is_fit(), "a pan ends the fit");
        middle.drag(pointer::middle, OVER, PAST);
        let view = middle.view.borrow().view();
        let corner = view.to_screen(Point::ZERO);
        assert!((corner - view.rect.min).length() < EDGE, "with Overscroll off a drag past the edge stops there: {corner:?} {:?}", view.rect);

        let mut hand = rig(NavigationOptions::default());
        hand.zoom_in();
        let start = hand.centre();
        hand.drag(primary, OVER, DRAG);
        let panned = hand.centre();
        assert!(panned.x < start.x - 1.0 && panned.y < start.y - 1.0, "a plain drag without a tool pans: {start:?} {panned:?}");
    }

    #[trial]
    fn a_canvas_takes_its_room_so_what_follows_lies_past_it() {
        let rects = Rc::new(Cell::new(None));
        let seen = rects.clone();
        let mut view = FullWindowView::new();
        let mut fixture = Fixture::new(move |ui| {
            let canvas = FullWindowCanvas::new(Bind::bound(&mut view), &Grey).show(ui).response.rect;
            seen.set(Some((canvas, ui.available_rect_before_wrap())));
        });
        fixture.settle();
        let (canvas, rest) = rects.get().expect("the canvas was shown");
        assert!(rest.min.y >= canvas.max.y || rest.min.x >= canvas.max.x, "the room after the canvas starts past it: {canvas:?} {rest:?}");
    }
}

mod sides {
    //! The side columns of a full-window dialog and the dialog's buttons among them.

    use craft_i18n::Direction;
    use craft_ui::buttons::Button;
    use craft_ui::choice::Checkbox;
    use craft_ui::contract::{Bind, Props as _, Show as _, ShowContainer as _};
    use craft_ui::dialog::{FullWindowDialog, FullWindowSide, SideFooter};
    use craft_ui::patterns::form_dialogs::ResetFooter;
    use craft_ui::runtime::focus::Initial;
    use craft_ui::runtime::overlay::CloseReason;
    use craft_ui::sections::{DialogRole, FooterButton};
    use craft_ui::testing::{Fixture, trial};
    use craft_ui::text::TextField;
    use egui::{Key, Modifiers, Rect};
    use std::cell::RefCell;
    use std::rc::Rc;

    /// What the Liquify-shaped dialog showed and reported.
    struct Seen {
        closed: Option<CloseReason>,
        picked: Option<usize>,
        body: Rect,
        side: Rect,
        leading: Rect,
        dialog: Rect,
    }

    /// Liquify's shape: the canvas in the body, a Properties column at the end edge, and OK and Cancel (Reset while
    /// Alt is held) in the side columns' footer after a leading Restore All button.
    fn liquify(direction: Direction) -> (Fixture, Rc<RefCell<Seen>>) {
        let seen = Rc::new(RefCell::new(Seen {
            closed: None,
            picked: None,
            body: Rect::NOTHING,
            side: Rect::NOTHING,
            leading: Rect::NOTHING,
            dialog: Rect::NOTHING,
        }));
        let shared = seen.clone();
        let (mut open, mut mesh, mut name) = (true, false, String::new());
        let mut fixture = Fixture::new(move |ui| {
            let footer = ResetFooter::new(ui, None, true);
            let mut seen = shared.borrow_mut();
            let (mut side, mut leading) = (Rect::NOTHING, Rect::NOTHING);
            let shown = FullWindowDialog::new("Liquify")
                .opened(Bind::bound(&mut open))
                .initial_focus(Initial::First)
                .buttons(footer.buttons(FooterButton::new("OK", DialogRole::Accept), true))
                .sides(|ui, _| {
                    side = FullWindowSide::new("Properties")
                        .show(ui, |ui, _| {
                            // Two stops in the column: a field (the first stop) and a check box. Enter on either reaches OK.
                            TextField::new("Brush Name", Bind::bound(&mut name)).show(ui);
                            Checkbox::new(Bind::bound(&mut mesh)).label("Show the Mesh, the Pins and the Guides of the Brush Tool").show(ui);
                        })
                        .output
                        .response
                        .rect;
                })
                .side_footer(SideFooter::bottom().leading(|ui| leading = Button::new("Restore All").show(ui).rect))
                .show(ui, |_, cx| cx.rect);
            seen.body = shown.inner.unwrap_or(Rect::NOTHING);
            seen.dialog = shown.output.response.rect;
            (seen.side, seen.leading) = (side, leading);
            seen.closed = seen.closed.or(shown.output.closed);
            seen.picked = seen.picked.or(shown.output.picked);
        });
        fixture.direction(direction);
        (fixture, seen)
    }

    #[trial]
    fn enter_activates_ok_and_cancel_cancels_in_the_side_columns_footer() {
        let (mut fixture, seen) = liquify(Direction::Ltr);
        fixture.settle().key(Key::Enter, Modifiers::NONE).settle();
        assert_eq!((seen.borrow().closed, seen.borrow().picked), (Some(CloseReason::Activated), Some(0)), "Enter activates OK");

        let (mut fixture, seen) = liquify(Direction::Ltr);
        let field = fixture.settle().focused();
        fixture.key(Key::Tab, Modifiers::NONE).settle();
        assert!(fixture.focused().is_some() && fixture.focused() != field, "Tab moves focus from the field to the check box");
        fixture.key(Key::Enter, Modifiers::NONE).settle();
        assert_eq!(
            (seen.borrow().closed, seen.borrow().picked),
            (Some(CloseReason::Activated), Some(0)),
            "Enter on the focused check box activates OK, and Space is its toggle"
        );

        let (mut fixture, seen) = liquify(Direction::Ltr);
        fixture.settle();
        let cancel = fixture.by_label("Cancel").expect("Cancel is the dialog's second button");
        fixture.click(&cancel).settle();
        assert_eq!(seen.borrow().closed, Some(CloseReason::Requested), "Cancel closes the dialog unanswered");
    }

    #[trial]
    fn alt_turns_cancel_into_reset_in_the_side_columns_footer() {
        let (mut fixture, seen) = liquify(Direction::Ltr);
        fixture.settle().hold(Modifiers::ALT).settle();
        assert!(fixture.by_label("Cancel").is_none(), "Alt hides Cancel");
        let reset = fixture.by_label("Reset").expect("Alt turns Cancel into Reset");
        fixture.click(&reset).settle();
        assert_eq!(seen.borrow().closed, None, "Reset keeps the dialog open");
        assert_eq!(seen.borrow().picked, Some(1), "the pick names Reset's place");
    }

    #[trial]
    fn the_leading_slot_and_the_buttons_share_the_footer_row_inside_the_side_columns() {
        for direction in [Direction::Ltr, Direction::Rtl] {
            let (mut fixture, seen) = liquify(direction);
            fixture.settle();
            let (ok, cancel) = (fixture.by_label("OK").expect("OK").rect, fixture.by_label("Cancel").expect("Cancel").rect);
            let seen = seen.borrow();
            let (leading, side, body) = (seen.leading, seen.side, seen.body);
            let buttons = ok.union(cancel);
            assert!(!leading.intersects(buttons), "{direction:?}: Restore All {leading:?} over the buttons {buttons:?}");
            let rtl = direction == Direction::Rtl;
            let leading_first = if rtl { leading.min.x >= buttons.max.x } else { leading.max.x <= buttons.min.x };
            assert!(leading_first, "{direction:?}: Restore All at the reading start of the row");
            let past_body = |rect: Rect| if rtl { rect.max.x <= body.min.x } else { rect.min.x >= body.max.x };
            assert!(past_body(side) && past_body(leading.union(buttons)), "{direction:?}: the column and its footer lie past the canvas");
            assert!(leading.min.y >= side.max.y && buttons.min.y >= side.max.y, "{direction:?}: the footer lies under the column");
            let row = leading.union(buttons);
            let (start, end) = if rtl {
                (row.max.x.round() <= side.max.x.round(), row.min.x.round() >= seen.dialog.min.x.round())
            } else {
                (row.min.x.round() >= side.min.x.round(), row.max.x.round() <= seen.dialog.max.x.round())
            };
            assert!(
                start && end,
                "{direction:?}: the footer row {row:?} starts with the column {side:?} and stays inside the dialog {:?}",
                seen.dialog
            );
        }
    }
}

/// An overlay drawn through a frame.
mod mapped_overlay {
    use craft_behavior::PixelSize;
    use craft_behavior::geometry::quad::Homography;
    use craft_behavior::kurbo::{Point, Rect};
    use craft_ui::canvas::{PixelRect, RasterSource};
    use craft_ui::contract::Bind;
    use craft_ui::dialog::{FullWindowCanvas, FullWindowView, RasterOverlay};
    use craft_ui::primitives::Revision;
    use craft_ui::testing::{Frames, trial};
    use egui::epaint::Mesh;
    use egui::{Color32, ColorImage, Pos2, Shape};

    /// The overlay's and the document's size.
    const IMAGE: PixelSize = PixelSize::new(40, 40);

    /// The mesh's cells a side.
    const CELLS: u32 = 4;

    /// How far a mesh corner may lie from the corner it should reach, in points.
    const NEAR: f32 = 0.01;

    /// An image of one colour.
    struct Flat;

    impl RasterSource for Flat {
        fn size(&self) -> PixelSize {
            IMAGE
        }

        fn revision(&self) -> Revision {
            Revision::new(1)
        }

        fn damage(&self, _: Revision) -> Option<PixelRect> {
            None
        }

        fn read(&self, _: PixelRect, _: u32, out: &mut ColorImage) {
            out.pixels.fill(Color32::from_rgb(0xff, 0xff, 0xff));
        }
    }

    /// The document's corners, clockwise from the top left.
    fn document() -> [Point; 4] {
        let (w, h) = (f64::from(IMAGE.width), f64::from(IMAGE.height));
        [Point::new(0.0, 0.0), Point::new(w, 0.0), Point::new(w, h), Point::new(0.0, h)]
    }

    /// The meshes a settled pass paints with the overlay `make` gives over `Flat`.
    fn meshes(make: impl Fn() -> RasterOverlay<'static>) -> Vec<Mesh> {
        let mut view = FullWindowView::new();
        let mut frames = Frames::new();
        let mut shapes = Vec::new();
        for _ in 0..2 {
            shapes = frames.frame_shapes(|ui| {
                FullWindowCanvas::new(Bind::bound(&mut view), &Flat).overlay(make()).show(ui);
            });
        }
        shapes
            .into_iter()
            .filter_map(|shape| match shape {
                Shape::Mesh(mesh) => Some(std::sync::Arc::unwrap_or_clone(mesh)),
                _ => None,
            })
            .collect()
    }

    fn corners(mesh: &Mesh, cells: u32) -> [Pos2; 4] {
        let row = (cells + 1) as usize;
        let v = |i: usize| mesh.vertices[i].pos;
        [v(0), v(cells as usize), v(row * row - 1), v(row * cells as usize)]
    }

    #[trial]
    fn a_mapped_overlay_lands_its_corners_on_the_frames_corners_as_the_flat_one_does() {
        let frame = Homography::unit_to_quad(document()).expect("a box");
        let flat = meshes(|| RasterOverlay::new(&Flat));
        let mapped = meshes(|| RasterOverlay::new(&Flat).mapped(frame, CELLS));
        let flat = flat.last().expect("the flat overlay's quad");
        let mapped = mapped.iter().find(|mesh| mesh.vertices.len() == ((CELLS + 1) * (CELLS + 1)) as usize).expect("the mapped mesh");
        for (a, b) in corners(flat, 1).iter().zip(corners(mapped, CELLS)) {
            assert!(a.distance(b) < NEAR, "the mesh corner {b:?} is the flat quad's corner {a:?}");
        }
        let half = meshes(|| RasterOverlay::new(&Flat).mapped(frame, CELLS).placed(Rect::new(0.0, 0.0, 0.5, 0.5)));
        let half = half.iter().find(|mesh| mesh.vertices.len() == ((CELLS + 1) * (CELLS + 1)) as usize).expect("the placed mesh");
        let (whole, part) = (corners(mapped, CELLS), corners(half, CELLS));
        assert!(part[0].distance(whole[0]) < NEAR && (part[2] - whole[0]).length() < (whole[2] - whole[0]).length() * 0.51);
    }

    #[trial]
    fn cells_whose_centre_lies_outside_the_frame_are_left_out() {
        let frame = Homography::unit_to_quad(document()).expect("a box");
        let overlay = || RasterOverlay::new(&Flat).mapped(frame, CELLS).placed(Rect::new(0.5, 0.5, 1.5, 1.5));
        let shown = meshes(overlay);
        let mesh = shown.iter().find(|mesh| mesh.vertices.len() == ((CELLS + 1) * (CELLS + 1)) as usize).expect("the mapped mesh");
        let cells = u64::from(CELLS * CELLS);
        assert_eq!(mesh.indices.len() as u64, cells / 4 * 6, "only the cells over the frame (a quarter) have triangles");
    }
}
