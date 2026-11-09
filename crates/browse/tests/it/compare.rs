//! `CompareTile`, the compare arrangements and the caption.

use std::cell::RefCell;
use std::rc::Rc;

use craft_browse::ui::compare::{ArrangeSpace, CompareCaption, CompareImage, ComparePair, CompareTile, RenderState, survey_cell, survey_columns};
use craft_ui::contract::Bind;
use craft_ui::patterns::form_dialogs::PreviewView;
use craft_ui::primitives::textures::Revision;
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{ColorImage, Rect, Vec2, pos2};

const PICTURE: [u32; 2] = [480, 320];
const PICTURE_GRAY: u8 = 128;
/// Where the pan drag starts, right of the pane's centre, and how far it goes.
const PAN_START: f32 = 40.0;
const PAN_BY: f32 = 24.0;
const SPACE: ArrangeSpace = ArrangeSpace::new(8.0, 4.0);

/// A displacement or extent of the trial's room, named so a trial's geometry reads as inputs.
fn offset(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}

fn image() -> CompareImage<'static> {
    CompareImage::Picture(
        1,
        Revision::of(&0u8),
        PICTURE,
        Box::new(|| ColorImage::filled([PICTURE[0] as usize, PICTURE[1] as usize], egui::Color32::from_gray(PICTURE_GRAY))),
    )
}

/// What a fixture of one or two tiles reports.
#[derive(Default)]
struct Seen {
    view: PreviewView,
    clicked: [bool; 2],
    double_clicked: [bool; 2],
}

fn pair(width: f32, state: fn() -> RenderState) -> (Fixture, Rc<RefCell<Seen>>) {
    let seen = Rc::new(RefCell::new(Seen { view: PreviewView::fitting(), ..Seen::default() }));
    let shown = seen.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut seen = shown.borrow_mut();
        let area = Rect::from_min_size(ui.max_rect().min, offset(width, 240.0));
        let [left, right] = ComparePair::new(SPACE).rects(area);
        let mut view = seen.view.clone();
        for (at, (name, rect)) in [("Select", left), ("Candidate", right)].into_iter().enumerate() {
            let out = CompareTile::new(name, Bind::bound(&mut view), image())
                .caption(CompareCaption::new(name, "IMG_0001.jpg").rating(Some(3)))
                .state(state())
                .place(ui, rect);
            seen.clicked[at] |= out.clicked;
            seen.double_clicked[at] |= out.double_clicked;
        }
        seen.view = view;
    });
    fixture.room(offset(width, 240.0));
    fixture.settle();
    (fixture, seen)
}

fn ready() -> RenderState {
    RenderState::Ready
}

/// Two panes over one view stay in step: a drag through the first pans the view the second draws, once a zoom
/// leaves the picture larger than the pane.
#[trial]
fn compare_tiles_share_zoom_and_pan() {
    let (mut fixture, seen) = pair(640.0, ready);
    assert!(seen.borrow().view.fit, "the pair starts fitted");
    let select = fixture.by_label("Select").expect("the Select pane is shown");
    fixture.click(&select);
    fixture.click(&select);
    fixture.settle();
    let zoomed = seen.borrow().view.clone();
    assert!(!zoomed.fit, "a double click shows 1:1");
    // The drag starts away from the double click, so it is a first press and not a third.
    let from = select.center() + offset(PAN_START, 0.0);
    fixture.frame(vec![pointer::moved(from)]);
    fixture.frame(vec![pointer::press(from)]);
    for step in 1..=4 {
        fixture.frame(vec![pointer::moved(from + offset(PAN_BY * step as f32 / 4.0, 0.0))]);
    }
    fixture.frame(vec![pointer::release(from + offset(PAN_BY, 0.0))]);
    fixture.settle();
    let panned = seen.borrow().view.clone();
    assert_ne!(panned.transform.offset, zoomed.transform.offset, "a drag pans the shared view");
    assert_eq!(fixture.by_label("Candidate").map(|pane| pane.rect.size()), fixture.by_label("Select").map(|pane| pane.rect.size()));
}

/// The file name leaves the caption when the tile is too narrow for it.
#[trial]
fn the_caption_drops_the_name_when_narrow() {
    let named = |fixture: &Fixture| fixture.texts().iter().any(|run| run.text == "IMG_0001.jpg");
    let (wide, _) = pair(960.0, ready);
    assert!(named(&wide), "a wide tile shows the name");
    let (narrow, _) = pair(220.0, ready);
    assert!(!named(&narrow), "a narrow tile drops it");
}

/// Survey columns maximise the tile for 3:2 pictures.
#[trial]
fn survey_columns_maximise_the_tile() {
    const ASPECT: f32 = 1.5;
    let wide = Rect::from_min_size(pos2(0.0, 0.0), offset(900.0, 300.0));
    let tall = Rect::from_min_size(pos2(0.0, 0.0), offset(300.0, 900.0));
    assert_eq!(survey_columns(1, wide, ASPECT), 1);
    assert!(survey_columns(6, wide, ASPECT) > survey_columns(6, tall, ASPECT), "a wide area takes more columns than a tall one");
    assert_eq!(survey_columns(0, wide, ASPECT), 1, "no tiles still gives one column");
}

/// A two-row survey puts tile 0 at the top left, and at the top right in a right-to-left interface.
#[trial]
fn survey_cells_mirror_in_a_right_to_left_interface() {
    const ASPECT: f32 = 1.5;
    let area = Rect::from_min_size(pos2(10.0, 20.0), offset(400.0, 600.0));
    assert_eq!(survey_columns(2, area, ASPECT), 1, "a tall area stacks two tiles in two rows");
    let (first, second) = (survey_cell(2, 0, area, ASPECT, false), survey_cell(2, 1, area, ASPECT, false));
    assert_eq!((first.min, second.min.y > first.min.y), (area.min, true), "tile 0 is at the top, tile 1 below it");
    let mirrored = survey_cell(2, 0, area, ASPECT, true);
    assert_eq!(mirrored.max.x, area.max.x, "tile 0 sits at the right edge when reading right to left");
    assert_eq!(mirrored.min.y, area.min.y);
    let wide = Rect::from_min_size(pos2(0.0, 0.0), offset(900.0, 300.0));
    let columns = survey_columns(4, wide, ASPECT);
    assert!(columns > 1);
    assert_eq!(survey_cell(4, 0, wide, ASPECT, true).max.x, wide.max.x);
    assert!(
        survey_cell(4, 1, wide, ASPECT, true).max.x < survey_cell(4, 0, wide, ASPECT, true).min.x + 0.01,
        "tile 1 follows tile 0 toward the left"
    );
}

