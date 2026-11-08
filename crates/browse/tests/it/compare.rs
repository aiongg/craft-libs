//! `CompareTile`, the compare arrangements and the caption.

use std::cell::RefCell;
use std::rc::Rc;

use craft_browse::ui::compare::{ArrangeSpace, CompareCaption, CompareImage, ComparePair, CompareTile, RenderState, survey_columns};
use craft_ui::contract::Bind;
use craft_ui::patterns::form_dialogs::PreviewView;
use craft_ui::primitives::textures::Revision;
use craft_ui::testing::{Fixture, trial};
use egui::{ColorImage, Rect, pos2, vec2};

const PICTURE: [u32; 2] = [480, 320];
const SPACE: ArrangeSpace = ArrangeSpace::new(8.0, 4.0);

fn image() -> CompareImage<'static> {
    CompareImage::Picture(
        1,
        Revision::of(&0u8),
        PICTURE,
        Box::new(|| ColorImage::filled([PICTURE[0] as usize, PICTURE[1] as usize], egui::Color32::GRAY)),
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
        let area = Rect::from_min_size(ui.max_rect().min, vec2(width, 240.0));
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
    fixture.room(vec2(width, 240.0));
    fixture.settle();
    (fixture, seen)
}

fn ready() -> RenderState {
    RenderState::Ready
}

/// Two panes over one view stay in step: a zoom through the first is the view the second draws.
#[trial]
fn compare_tiles_share_zoom_and_pan() {
    let (mut fixture, seen) = pair(640.0, ready);
    let fitted = seen.borrow().view.clone();
    assert!(fitted.fit, "the pair starts fitted");
    let select = fixture.by_label("Select").expect("the Select pane is shown");
    fixture.drag(&select, vec2(24.0, 0.0));
    fixture.settle();
    let moved = seen.borrow().view.clone();
    assert!(!moved.fit, "a drag leaves fit");
    assert_eq!(fixture.by_label("Candidate").map(|pane| pane.rect.size()), fixture.by_label("Select").map(|pane| pane.rect.size()));
}

/// A double click toggles Fit and 1:1.
#[trial]
fn double_click_toggles_fit_and_one_to_one() {
    let fitted = PreviewView::fitting();
    let actual = CompareTile::toggle_zoom(&fitted);
    assert!(!actual.fit);
    assert!((actual.transform.scale - 1.0).abs() < f32::EPSILON, "one device pixel per image pixel");
    assert!(CompareTile::toggle_zoom(&actual).fit, "the next double click fits again");

    let (mut fixture, seen) = pair(640.0, ready);
    let select = fixture.by_label("Select").expect("the Select pane is shown");
    fixture.click(&select);
    fixture.click(&select);
    fixture.settle();
    assert!(seen.borrow().double_clicked[0], "the pane reports the double click");
    assert!(!seen.borrow().view.fit, "a double click on a fitted pane shows 1:1");
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

/// An overdue render and a failed one say so; a render still within the host's wait shows nothing.
#[trial]
fn the_pills_follow_the_render_state() {
    let said = |state: fn() -> RenderState, text: &str| {
        let (fixture, _) = pair(640.0, state);
        fixture.texts().iter().any(|run| run.text.contains(text))
    };
    assert!(said(|| RenderState::Pending(true), "Rendering"));
    assert!(said(|| RenderState::Failed("The renderer stopped".into()), "The renderer stopped"));
    assert!(!said(|| RenderState::Pending(false), "Rendering"));
    assert!(!said(ready, "Rendering"));
}

/// Survey columns maximise the tile for 3:2 pictures.
#[trial]
fn survey_columns_maximise_the_tile() {
    const ASPECT: f32 = 1.5;
    let wide = Rect::from_min_size(pos2(0.0, 0.0), vec2(900.0, 300.0));
    let tall = Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 900.0));
    assert_eq!(survey_columns(1, wide, ASPECT), 1);
    assert!(survey_columns(6, wide, ASPECT) > survey_columns(6, tall, ASPECT), "a wide area takes more columns than a tall one");
    assert_eq!(survey_columns(0, wide, ASPECT), 1, "no tiles still gives one column");
}

/// The two panes are equal and mirror in a right-to-left interface.
#[trial]
fn compare_panes_are_equal_and_mirror() {
    let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 200.0));
    let [select, candidate] = ComparePair::new(SPACE).rects(area);
    assert_eq!(select.size(), candidate.size());
    assert!(select.right() < candidate.left(), "the Select pane is at the reading start");
    let [select_rtl, candidate_rtl] = ComparePair::new(SPACE).rtl(true).rects(area);
    assert_eq!(select_rtl, candidate);
    assert_eq!(candidate_rtl, select);
}
