//! Document text spans: the styles of `DocRun` change the laid-out galley, and `Text::spans` shows them.

use craft_ui::contract::{Props as _, Show as _};
use craft_ui::display::Text;
use craft_ui::fonts::{Family, FontRef, I18nScripts, SystemFonts, UiFonts, deflated_default_definitions};
use craft_ui::list::ThumbAspect;
use craft_ui::primitives::{DocRun, DocText, TextLayout};
use craft_ui::style::DataColor;
use craft_ui::testing::{Fixture, trial};
use craft_ui::tile_grid::Tile;
use egui::Color32;
use std::cell::RefCell;
use std::rc::Rc;

const SIZE: f32 = 14.0;
const INDENT: f32 = 30.0;
const EPS: f32 = 0.01;
const INK: Color32 = Color32::from_rgb(200, 30, 30);

fn run(text: &str) -> DocRun<'_> {
    DocRun::new(text, FontRef::proportional(), SIZE, DataColor::new(INK))
}

fn egui_face(name: &str) -> egui::FontData {
    (**deflated_default_definitions().font_data.get(name).expect("an egui default face")).clone()
}

/// Installs a UI face and a semibold family of another face, so a bold run is measurably different.
fn install_semibold(fixture: &Fixture) {
    let _stack = UiFonts::new(&I18nScripts)
        .app_face("ui", egui_face("Ubuntu-Light"), &[FontRef::proportional()])
        .app_face("semibold", egui_face("Hack"), &[FontRef::named("semibold")])
        .family(Family::named("semibold").weight(600))
        .system_fonts(SystemFonts::Off)
        .install(fixture.ctx());
}

/// The layouts a content painter makes of each `runs` set, in order.
fn layouts(setup: impl FnOnce(&Fixture), sets: Vec<Vec<DocRun<'static>>>) -> Vec<TextLayout> {
    let found = Rc::new(RefCell::new(Vec::new()));
    let record = found.clone();
    let mut fixture = Fixture::new(move |ui| {
        let record = record.clone();
        let sets = sets.clone();
        Tile::new("Doc")
            .thumb(ThumbAspect::new(3, 2), move |cx, _| {
                let mut found = record.borrow_mut();
                if found.is_empty() {
                    found.extend(sets.iter().map(|runs| cx.layout(&DocText::new(runs))));
                }
            })
            .show(ui);
    });
    setup(&fixture);
    fixture.settle();
    drop(fixture);
    Rc::try_unwrap(found).map(RefCell::into_inner).unwrap_or_default()
}

/// A run's lines are in the galley's text formats, in the run's colour, and an indent shifts the run by its space.
#[trial]
fn run_styles_change_the_laid_out_galley() {
    let sets = vec![
        vec![run("Word")],
        vec![run("Word").underline()],
        vec![run("Word").strikethrough()],
        vec![run("Word").indent(INDENT)],
        vec![run("Word").bold()],
    ];
    let found = layouts(install_semibold, sets);
    assert_eq!(found.len(), 5, "every set laid out");
    let format = |at: usize| found[at].galley.job.sections[0].format.clone();
    assert_eq!(format(0).underline.width, 0.0);
    assert_eq!(format(0).strikethrough.width, 0.0);
    assert!(format(1).underline.width > 0.0 && format(1).underline.color == INK, "underline in the run's colour");
    assert!(format(2).strikethrough.width > 0.0 && format(2).strikethrough.color == INK, "strikethrough in the run's colour");
    let shift = found[3].size().x - found[0].size().x;
    assert!((shift - INDENT).abs() < EPS, "the first-line indent adds its space to the row: {shift}");
    assert_ne!(format(4).font_id.family, format(0).font_id.family, "bold takes the semibold family");
    assert!((found[4].size().x - found[0].size().x).abs() > EPS, "the bold face has its own advances");
}

/// `Text::spans` is named by its joined text, and a span's size and indent change the room the text takes.
#[trial]
fn text_spans_take_their_room_from_the_spans() {
    let size = Rc::new(RefCell::new(Vec::new()));
    let record = size.clone();
    let mut fixture = Fixture::new(move |ui| {
        let plain = Text::spans(&[run("Hello "), run("world")]).id_salt("plain").show(ui).rect;
        let indented = Text::spans(&[run("Hello ").indent(INDENT), run("world")]).id_salt("indented").show(ui).rect;
        *record.borrow_mut() = vec![plain, indented];
    });
    fixture.settle();
    assert!(fixture.by_label("Hello world").is_some(), "the spans' joined text is the accessible name");
    let rects = size.borrow().clone();
    assert!((rects[1].width() - rects[0].width() - INDENT).abs() < EPS, "the indent widens the text: {rects:?}");
    assert!((rects[1].height() - rects[0].height()).abs() < EPS, "and leaves its height");
}
