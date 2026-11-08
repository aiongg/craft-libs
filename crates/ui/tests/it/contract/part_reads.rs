//! What component code reads inside a pass: the state of a part declared next pass, the pass clock and a text
//! layout in `update`; and an AccessKit click on a scroll bar track.

use super::kit::PlainStyle;
use craft_behavior::kurbo::{Point, Rect as DocRect};
use craft_behavior::{CanvasView, PartKey, PressPhase, Sense};
use craft_ui::canvas::{Canvas, CanvasScrollbars};
use craft_ui::contract::{
    Animate as _, Bind, Common, Component, Cx, PaintUi, PartsCx, Pointer as _, Role, Show as _, ShowContainer as _, Styled as _,
};
use craft_ui::fonts::FontRef;
use craft_ui::primitives::{DocRun, DocText};
use craft_ui::style::{DataColor, Stateful, States, StyleCx};
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::{Fixture, Frames, trial};
use egui::{Color32, Pos2, Rect, Response, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const BODY: PartKey = PartKey::new("body");
const ADDED: PartKey = PartKey::new("added");
const SIZE: f32 = 14.0;
const TEXT: &str = "abcdefghijklmnopqrst";

/// What the lab component read.
#[derive(Default)]
struct Log {
    /// The idle value `paint` resolved for the part that was not declared yet.
    undeclared: Vec<f32>,
    /// The pass clock `update` and `paint` read, per pass.
    clock: Vec<(f64, f64)>,
    /// The character under the press, found from a layout made in `update`.
    character: Option<usize>,
}

/// A leaf that adds a part when it is clicked, reads the clock and lays text out in `update`.
#[derive(craft_ui::contract::Component)]
struct Lab<'a> {
    #[common]
    common: Common,
    added: &'a Cell<bool>,
    log: &'a RefCell<Log>,
    update_clock: f64,
}

impl Component for Lab<'_> {
    const ROLE: Role = Role::None;
    type Style = PlainStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: LabStyleKey) -> PlainStyle {
        PlainStyle::min(Vec2::new(200.0, 30.0))
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        cx.part(BODY, rect).sense(Sense::CLICK);
        if self.added.get() {
            cx.part(ADDED, rect.shrink(4.0)).sense(Sense::HOVER);
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.update_clock = cx.now();
        match cx.part(BODY).press.phase {
            PressPhase::Release { click: true, .. } => self.added.set(true),
            PressPhase::Down { start, .. } => {
                let runs = [DocRun::new(TEXT, FontRef::proportional(), SIZE, DataColor::new(Color32::BLACK))];
                let layout = cx.layout_doc(&DocText::new(&runs));
                let at = start.origin - cx.rect.min;
                self.log.borrow_mut().character = Some(layout.galley.cursor_from_pos(at).index.0);
            }
            _ => {}
        }
    }

    fn paint(&mut self, _: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let look = Stateful::new(0.0_f32).off(States::new(1.0, 2.0, 3.0, 4.0));
        let mut log = self.log.borrow_mut();
        log.clock.push((self.update_clock, cx.now()));
        if cx.part_if_declared(ADDED).is_none() {
            log.undeclared.push(cx.resolve_part(ADDED, &look));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn area() -> Rect {
    Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(200.0, 30.0))
}

#[trial]
fn a_part_added_by_a_click_paints_idle_in_the_same_pass() {
    let (added, log) = (Cell::new(false), RefCell::new(Log::default()));
    let mut frames = Frames::new();
    let mut show = |ui: &mut egui::Ui| {
        Lab { common: Common::new(), added: &added, log: &log, update_clock: 0.0 }.show_at(ui, area());
    };
    let at = area().center();
    frames.frame(vec![moved(at)], &mut show);
    frames.frame(vec![press(at)], &mut show);
    log.borrow_mut().undeclared.clear();
    frames.frame(vec![release(at)], &mut show);
    assert!(added.get(), "the click added the part");
    let log = log.borrow();
    let reads = &log.undeclared;
    assert!(!reads.is_empty() && reads.iter().all(|read| *read == 1.0), "the part declared next pass paints its rest look: {reads:?}");
}

#[trial]
fn the_pass_clock_follows_the_fixtures_manual_time_in_update_and_paint() {
    let (added, log) = (Cell::new(false), RefCell::new(Log::default()));
    let mut frames = Frames::new();
    let mut show = |ui: &mut egui::Ui| {
        Lab { common: Common::new(), added: &added, log: &log, update_clock: 0.0 }.show_at(ui, area());
    };
    frames.frame(vec![], &mut show);
    let before = *log.borrow().clock.last().expect("a pass painted");
    frames.advance(2.5);
    frames.frame(vec![], &mut show);
    let after = *log.borrow().clock.last().expect("a pass painted");
    assert!((after.0 - before.0 - 2.5).abs() < 1e-6, "update reads the advanced time: {before:?} -> {after:?}");
    assert!((after.1 - before.1 - 2.5).abs() < 1e-6, "paint reads the advanced time: {before:?} -> {after:?}");
}

#[trial]
fn update_lays_text_out_and_finds_the_character_under_a_press() {
    let (added, log) = (Cell::new(false), RefCell::new(Log::default()));
    let mut frames = Frames::new();
    let mut show = |ui: &mut egui::Ui| {
        Lab { common: Common::new(), added: &added, log: &log, update_clock: 0.0 }.show_at(ui, area());
    };
    let at = area().min + Vec2::new(40.0, 5.0);
    frames.frame(vec![moved(at)], &mut show);
    frames.frame(vec![press(at)], &mut show);
    let character = log.borrow().character.expect("update laid the text out at the press");
    assert!(character > 0 && character < TEXT.len(), "a press 40 points into the text lands inside it: {character}");
}

/// The view's rect in the fixture, in points.
const VIEW: Rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(400.0, 300.0));
const ZOOM: f64 = 2.0;

#[trial]
fn an_accesskit_click_on_a_scroll_bar_track_pages_toward_its_centre() {
    let view = Rc::new(RefCell::new(CanvasView::new(VIEW, Point::new(0.0, 0.0), ZOOM)));
    let edited = Rc::clone(&view);
    let document = DocRect::new(0.0, 0.0, 800.0, 600.0);
    let mut fixture = Fixture::new(move |ui| {
        let view = *edited.borrow();
        Canvas::new("Document").view(view).show(ui, |ui, body| {
            let _ = CanvasScrollbars::new("Scroll bars", Bind::bound(&mut *edited.borrow_mut()), document).show_at(ui, body.rect);
        });
    });
    fixture.settle();
    let bar = fixture.by_label("Horizontal scroll bar").expect("the bar has a node");
    fixture.access(AccessActionKind::Click, &bar);
    let page = f64::from(VIEW.width()) / ZOOM;
    let moved = view.borrow().center.x;
    assert!((moved - page).abs() < 1e-6, "paged {moved}, a screenful is {page}");
}
